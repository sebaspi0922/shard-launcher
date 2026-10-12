// Capturas de las pantallas clave a 1280×800 con fuentes reales.
// Regenerar: tools/flutter.sh test --update-goldens test/goldens_test.dart

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:lcore_app/app.dart';
import 'package:lcore_app/core/bridge.dart';
import 'package:lcore_app/core/launch.dart';
import 'package:lcore_app/ui/accounts/account_dialogs.dart';
import 'package:lcore_app/ui/accounts/account_panel.dart';
import 'package:lcore_app/ui/instances/create_instance_dialog.dart';
import 'package:lcore_app/ui/instances/instances_page.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';

/// Compara con el golden y devuelve las sombras a su valor de test (el
/// framework comprueba esa variable antes de los tearDown).
Future<void> expectGolden(String name) async {
  try {
    await expectLater(
      find.byType(LauncherApp),
      matchesGoldenFile('goldens/$name.png'),
    );
  } finally {
    debugDisableShadows = true;
  }
}

void main() {
  testWidgets('inicio', (tester) async {
    await pumpApp(tester, location: '/');
    await expectGolden('home_es');
  });

  testWidgets('inicio descargando', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          const LaunchState(
            phase: LaunchPhase.downloading,
            done: 1834,
            total: 3920,
            stage: 'assets',
          ),
        );
    await settle(tester);
    await expectGolden('home_downloading_es');
  });

  testWidgets('inicio con error', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          const LaunchState(
            phase: LaunchPhase.error,
            failure: LaunchError(
              BridgeError(
                kind: 'java_required',
                message: 'hace falta Java 21: instálala con `lcore java install 21`',
              ),
            ),
          ),
        );
    await settle(tester);
    await expectGolden('home_error_es');
  });

  testWidgets('inicio vacío (en)', (tester) async {
    await pumpApp(
      tester,
      location: '/',
      bridge: FakeBridge(instances: [], accounts: [], language: 'en'),
    );
    await expectGolden('home_empty_en');
  });

  testWidgets('instancias en rejilla', (tester) async {
    final c = await pumpApp(tester, location: '/instances');
    c.read(launchProvider.notifier).debugSet(
          'create-aeronautica',
          const LaunchState(phase: LaunchPhase.running),
        );
    await settle(tester);
    await expectGolden('instances_grid_es');
  });

  testWidgets('instancias en lista (en)', (tester) async {
    final c = await pumpApp(
      tester,
      location: '/instances',
      bridge: FakeBridge(language: 'en'),
    );
    c.read(instancesGridProvider.notifier).set(false);
    await settle(tester);
    await expectGolden('instances_list_en');
  });

  testWidgets('detalle: ajustes', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=settings');
    await expectGolden('instance_settings_es');
  });

  testWidgets('detalle: registros', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(
      tester,
      location: '/instances/survival-amigos?tab=logs',
      bridge: bridge,
    );
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          LaunchState(phase: LaunchPhase.running, gameId: BigInt.one),
        );
    bridge.emit(
      UiEvent.gameStarted(gameId: BigInt.one, instanceId: 'survival-amigos'),
    );
    bridge.emit(UiEvent.gameLog(gameId: BigInt.one, lines: sampleLog));
    await settle(tester);
    await expectGolden('instance_logs_es');
  });

  testWidgets('ajustes', (tester) async {
    await pumpApp(tester, location: '/settings');
    await expectGolden('settings_es');
  });

  testWidgets('panel de cuentas', (tester) async {
    await pumpApp(tester, location: '/');
    showAccountPanel(pageContext(tester));
    await settle(tester);
    await expectGolden('accounts_panel_es');
  });

  testWidgets('login de Microsoft', (tester) async {
    await pumpApp(tester, location: '/');
    showMicrosoftLogin(pageContext(tester));
    await settle(tester);
    await expectGolden('ms_login_es');
  });

  testWidgets('cuenta offline (en)', (tester) async {
    await pumpApp(tester, location: '/', bridge: FakeBridge(language: 'en'));
    showOfflineAccount(pageContext(tester));
    await settle(tester);
    await expectGolden('offline_account_en');
  });

  testWidgets('crear instancia', (tester) async {
    await pumpApp(tester, location: '/instances');
    showCreateInstance(pageContext(tester));
    await settle(tester);
    await tester.tap(find.text('Fabric'));
    await settle(tester);
    await expectGolden('create_instance_es');
  });
}

final sampleLog = [
  for (final (text, err) in [
    ('[18:02:11] [main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.16.10', false),
    ('[18:02:11] [main/INFO]: Loading 87 mods:', false),
    ('\t- fabric-api 0.119.2+1.21.4', false),
    ('\t- sodium 0.6.13+mc1.21.4', false),
    ('\t- iris 1.8.8+mc1.21.4', false),
    ('[18:02:12] [main/WARN]: Mod resolution found an incompatible version of cloth-config', false),
    ('[18:02:14] [Render thread/INFO]: Setting user: NadiaK', false),
    ('[18:02:15] [Render thread/INFO]: Backend library: LWJGL version 3.3.3-snapshot', false),
    ('[18:02:16] [Render thread/DEBUG]: Created GL debug context', false),
    ('[18:02:18] [Render thread/INFO]: Reloading ResourceManager: vanilla, fabric, sodium', false),
    ('[18:02:19] [Worker-Main-4/ERROR]: Failed to load texture minecraft:textures/entity/cape.png', true),
    ('java.io.FileNotFoundException: minecraft:textures/entity/cape.png', true),
    ('\tat net.minecraft.class_3300.method_14486(class_3300.java:41)', true),
    ('\tat net.minecraft.class_1049.method_4545(class_1049.java:64)', true),
    ('[18:02:21] [Render thread/INFO]: Sound engine started', false),
    ('[18:02:21] [Render thread/INFO]: Created: 1024x512x4 minecraft:textures/atlas/blocks.png-atlas', false),
    ('[18:02:24] [Server thread/INFO]: Preparing level "Mundo de verano"', false),
    ('[18:02:26] [Server thread/INFO]: Preparing spawn area: 83%', false),
    ('[18:02:27] [Server thread/WARN]: Can\'t keep up! Is the server overloaded? Running 2140ms or 42 ticks behind', false),
    ('[18:02:28] [Render thread/INFO]: Loaded 1290 advancements', false),
    ('[18:02:29] [Server thread/INFO]: NadiaK joined the game', false),
  ])
    LogLineView(text: text, stderr: err),
];
