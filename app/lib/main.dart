import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'app.dart';
import 'core/app_info.dart';
import 'core/bridge.dart';
import 'core/providers.dart';
import 'core/window.dart';
import 'src/rust/api/app.dart';
import 'src/rust/api/instances.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  // `SHARD_DATA_DIR` abre otra carpeta de datos (pruebas sin tocar la real).
  final dataDir = Platform.environment['SHARD_DATA_DIR'];
  final root = await start(dataDir: dataDir?.isEmpty ?? true ? null : dataDir);

  // Prueba de humo del puente: `LCORE_SMOKE=1` lista las instancias y sale.
  if (Platform.environment['LCORE_SMOKE'] == '1') {
    final list = await listInstances();
    stdout.writeln('lcore ${coreVersion()} en $root');
    for (final i in list) {
      stdout.writeln('${i.id} ${i.mc} ${i.loader} ${i.playtimeSeconds}s');
    }
    exit(0);
  }

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
    await windowManager.show();
    await windowManager.focus();
  });
  await windowManager.setPreventClose(true);

  runApp(
    ProviderScope(
      retry: (_, _) => null,
      overrides: [
        bridgeProvider.overrideWithValue(RustBridge(root)),
        windowControlsProvider.overrideWithValue(DesktopWindowControls()),
      ],
      child: const LauncherApp(),
    ),
  );
}
