import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'app.dart';
import 'core/app_info.dart';
import 'core/bridge.dart';
import 'core/desktop.dart';
import 'core/providers.dart';
import 'core/ui_prefs.dart';
import 'core/window.dart';
import 'src/rust/api/app.dart';
import 'src/rust/api/instances.dart';
import 'src/rust/frb_generated.dart';
import 'ui/first_run/first_run_page.dart';
import 'ui/shell/tray_controller.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  // `SHARD_DATA_DIR` abre otra carpeta de datos (pruebas sin tocar la real).
  final dataDir = Platform.environment['SHARD_DATA_DIR'];
  final root = await start(dataDir: dataDir?.isEmpty ?? true ? null : dataDir);

  // Prueba de humo del puente: `SHARD_SMOKE=1` lista las instancias y sale.
  if (Platform.environment['SHARD_SMOKE'] == '1') {
    final list = await listInstances();
    stdout.writeln('shard ${coreVersion()} en $root');
    for (final i in list) {
      stdout.writeln('${i.id} ${i.mc} ${i.loader} ${i.playtimeSeconds}s');
    }
    exit(0);
  }

  final bridge = RustBridge(root);

  // Primer arranque: se decide antes de pintar para no mostrar Inicio un instante.
  final prefs = UiPrefs.parse(bridge.getUiPrefs());
  var location = '/';
  if (prefs.firstRunDone != true) {
    final instances = await bridge.listInstances().catchError((_) => <InstanceView>[]);
    final accounts = await bridge.listAccounts().catchError((_) => <AccountView>[]);
    if (needsFirstRun(
      prefs,
      hasInstances: instances.isNotEmpty,
      hasAccounts: accounts.isNotEmpty,
    )) {
      location = '/welcome';
    }
  }
  // Spike GPUI (medición): `SHARD_START_ROUTE=/instances` abre otra pantalla.
  final startRoute = Platform.environment['SHARD_START_ROUTE'];
  if (location == '/' && startRoute != null && startRoute.isNotEmpty) {
    location = startRoute;
  }

  // Autoarranque con la sesión: empieza oculta si hay bandeja.
  final tray = DesktopTrayHost();
  final startHidden =
      args.contains(kBackgroundArg) && tray.supported && location == '/';

  await windowManager.ensureInitialized();
  const options = WindowOptions(
    size: Size(kDesignWidth, kDesignHeight),
    minimumSize: Size(960, 600),
    center: true,
    title: kAppName,
    titleBarStyle: TitleBarStyle.hidden,
    backgroundColor: Color(0xFF0B0D0C),
  );
  await windowManager.waitUntilReadyToShow(options, () async {
    if (startHidden) return;
    await windowManager.show();
    await windowManager.focus();
  });
  await windowManager.setPreventClose(true);

  final container = ProviderContainer(
    retry: (_, _) => null,
    overrides: [
      bridgeProvider.overrideWithValue(bridge),
      windowControlsProvider.overrideWithValue(DesktopWindowControls()),
      trayHostProvider.overrideWithValue(tray),
      autostartProvider.overrideWithValue(DesktopAutostart()),
      startHiddenProvider.overrideWithValue(startHidden),
      initialLocationProvider.overrideWithValue(location),
    ],
  );
  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const LauncherApp(),
    ),
  );
  // Con la ventana oculta puede que no se pinte nada: el icono de la bandeja
  // se crea ya, sin esperar al primer fotograma.
  if (startHidden) container.read(trayControllerProvider);
}
