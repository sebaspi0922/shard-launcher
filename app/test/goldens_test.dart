// Capturas de las pantallas clave a 1280×800 con fuentes reales.
// Regenerar: tools/flutter.sh test --update-goldens test/goldens_test.dart

import 'dart:async';

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shard/app.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/launch.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/ui/accounts/account_dialogs.dart';
import 'package:shard/ui/accounts/account_panel.dart';
import 'package:shard/ui/instances/create_instance_dialog.dart';
import 'package:shard/core/content_search.dart';
import 'package:shard/core/tasks.dart';
import 'package:shard/ui/content/modpack_dialog.dart';
import 'package:shard/ui/content/project_sheet.dart';
import 'package:shard/ui/instances/backups_tab.dart';
import 'package:shard/ui/instances/instances_page.dart';
import 'package:shard/ui/servers/server_dialog.dart';
import 'package:material_ui/material_ui.dart' show SingleChildScrollView, showDialog;
import 'package:material_symbols_icons/symbols.dart';
import 'package:shard/core/crash_notice.dart';
import 'package:shard/core/updates.dart';
import 'package:shard/ui/doctor/doctor_dialog.dart';
import 'package:shard/ui/first_run/first_run_page.dart';
import 'package:shard/ui/instances/change_version_dialog.dart';
import 'package:shard/ui/instances/trash_dialog.dart';
import 'package:shard/ui/shell/backdrop.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';
import 'support/updates.dart';

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

  testWidgets('detalle: contenido', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=content');
    await expectGolden('instance_content_es');
  });

  testWidgets('detalle: mundos', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=worlds');
    await expectGolden('instance_worlds_es');
  });

  testWidgets('detalle: capturas', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=screenshots');
    await expectGolden('instance_screenshots_es');
  });

  testWidgets('detalle: copias', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=backups');
    await expectGolden('instance_backups_es');
  });

  testWidgets('restaurar copia', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(
      tester,
      location: '/instances/survival-amigos?tab=backups',
      bridge: bridge,
    );
    final i = c.read(instanceByIdProvider('survival-amigos'))!;
    showDialog<void>(
      context: pageContext(tester),
      builder: (_) => RestoreBackupDialog(instance: i, backup: bridge.backups[1]),
    );
    await settle(tester);
    await expectGolden('backup_restore_es');
  });

  testWidgets('contenido: búsqueda', (tester) async {
    await pumpApp(tester, location: '/content');
    await expectGolden('content_search_es');
  });

  testWidgets('contenido: desde una instancia', (tester) async {
    await pumpApp(
      tester,
      location: '/content?instance=survival-amigos&kind=mod',
    );
    await expectGolden('content_target_es');
  });

  testWidgets('contenido: CurseForge sin clave (en)', (tester) async {
    final c = await pumpApp(
      tester,
      location: '/content',
      bridge: FakeBridge(language: 'en'),
    );
    final n = c.read(contentSearchProvider.notifier);
    n.setFilters(c.read(contentSearchProvider).filters.copyWith(source: 'curseforge'));
    await settle(tester);
    await expectGolden('content_curseforge_en');
  });

  testWidgets('ficha de proyecto', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(
      tester,
      location: '/content?instance=survival-amigos&kind=mod',
      bridge: bridge,
    );
    showProjectSheet(pageContext(tester), bridge.searchResults.first);
    await settle(tester);
    await expectGolden('project_sheet_es');
  });

  testWidgets('modpack instalándose', (tester) async {
    final bridge = FakeBridge()..modpackCompleter = Completer();
    final c = await pumpApp(tester, location: '/content', bridge: bridge);
    showModpackInstall(
      pageContext(tester),
      source: 'https://cdn.modrinth.com/data/fo/pack.mrpack',
      name: 'Fabulously Optimized',
    );
    await settle(tester);
    await tester.tap(find.text('Instalar modpack').last);
    c.read(tasksProvider.notifier).debugSet(
          TaskProgress(
            id: BigInt.from(9),
            stage: 'modpack',
            done: 87,
            total: 212,
            bytes: false,
          ),
        );
    await settle(tester);
    await expectGolden('modpack_installing_es');
  });

  testWidgets('servidores', (tester) async {
    await pumpApp(tester, location: '/servers');
    await expectGolden('servers_es');
  });

  testWidgets('añadir servidor (en)', (tester) async {
    await pumpApp(tester, location: '/servers', bridge: FakeBridge(language: 'en'));
    showServerDialog(pageContext(tester));
    await settle(tester);
    await expectGolden('server_dialog_en');
  });

  testWidgets('ajustes', (tester) async {
    await pumpApp(tester, location: '/settings');
    await expectGolden('settings_es');
  });

  testWidgets('ajustes: integraciones', (tester) async {
    await pumpApp(tester, location: '/settings', bridge: FakeBridge()..curseforgeKey = true);
    await tester.drag(find.text('General'), const Offset(0, -2400));
    await settle(tester);
    await expectGolden('settings_integrations_es');
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

  round3();
}


void round3() {
  testWidgets('skins', (tester) async {
    await pumpApp(tester, location: '/skins');
    await expectGolden('skins_es');
  });

  testWidgets('skins: cuenta offline (en)', (tester) async {
    await pumpApp(
      tester,
      location: '/skins',
      bridge: FakeBridge(
        language: 'en',
        accounts: const [
          AccountView(id: 'o1', name: 'Pruebas_01', microsoft: false, active: true),
        ],
      ),
    );
    await expectGolden('skins_offline_en');
  });

  testWidgets('doctor', (tester) async {
    final c = await pumpApp(tester, location: '/instances/survival-amigos?tab=content');
    showDoctor(pageContext(tester), c.read(instanceByIdProvider('survival-amigos'))!);
    await settle(tester);
    await expectGolden('doctor_es');
  });

  testWidgets('aviso de cierre inesperado', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(crashNoticeProvider.notifier).debugShow(
          CrashNotice(gameId: BigInt.two, instanceId: 'create-aeronautica', code: 1),
        );
    await settle(tester);
    await expectGolden('crash_banner_es');
  });

  testWidgets('aviso de actualización (AppImage)', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(updateProvider.notifier).debugSet(
          const UpdateState(status: UpdateStatus.available, info: appImageUpdate),
        );
    await settle(tester);
    await expectGolden('update_banner_es');
  });

  testWidgets('ajustes con actualización manual (Flatpak, en)', (tester) async {
    final c = await pumpApp(
      tester,
      location: '/settings',
      bridge: FakeBridge(language: 'en')..updatePackage = 'flatpak',
    );
    c.read(updateProvider.notifier).debugSet(
          const UpdateState(status: UpdateStatus.available, info: flatpakUpdate, dismissed: true),
        );
    await settle(tester);
    await tester.dragUntilVisible(
      find.text('Check for updates automatically'),
      find.byType(SingleChildScrollView).first,
      const Offset(0, -300),
    );
    await settle(tester);
    await expectGolden('settings_update_en');
  });

  testWidgets('menú de la instancia', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=worlds');
    await tester.tap(find.byTooltip('Más opciones'));
    await settle(tester);
    await expectGolden('instance_menu_es');
  });

  testWidgets('cambiar versión', (tester) async {
    final c = await pumpApp(tester, location: '/instances/survival-amigos?tab=settings');
    showChangeVersion(pageContext(tester), c.read(instanceByIdProvider('survival-amigos'))!);
    await settle(tester);
    await tester.tap(find.text('NeoForge'));
    await settle(tester);
    await expectGolden('change_version_es');
  });

  testWidgets('papelera (en)', (tester) async {
    await pumpApp(tester, location: '/instances', bridge: FakeBridge(language: 'en'));
    showTrash(pageContext(tester));
    await settle(tester);
    await expectGolden('trash_en');
  });

  testWidgets('ajustes: apariencia', (tester) async {
    await pumpApp(tester, location: '/settings');
    await tester.drag(find.text('General'), const Offset(0, -300));
    await settle(tester);
    await expectGolden('settings_appearance_es');
  });

  testWidgets('ajustes: java', (tester) async {
    await pumpApp(tester, location: '/settings');
    await tester.drag(find.text('General'), const Offset(0, -900));
    await settle(tester);
    await expectGolden('settings_java_es');
  });

  for (final style in ['graphite', 'sebaspi', 'landscape', 'aurora']) {
    testWidgets('estilo $style', (tester) async {
      await pumpApp(
        tester,
        location: '/',
        bridge: FakeBridge()
          ..uiPrefs = '{"first_run_done":true,"style":"$style","blur":10,"dim":0.5}',
      );
      // Atlas de glifos y grano del fondo (se crean de forma asíncrona).
      await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 200)));
      await tester.pump(const Duration(milliseconds: 100));
      await expectGolden('home_style_${style}_es');
    });
  }

  // Para la web: el fondo se anima allí, detrás de esta captura transparente.
  testWidgets('web: inicio sin fondo', (tester) async {
    debugHideBackdrop = true;
    try {
      await pumpApp(
        tester,
        location: '/',
        bridge: FakeBridge()..uiPrefs = '{"first_run_done":true,"style":"graphite"}',
      );
      await expectGolden('web_home_graphite_es');
    } finally {
      debugHideBackdrop = false;
    }
  });

  testWidgets('primer arranque: bienvenida', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: FakeBridge(instances: [], accounts: [], language: null),
    );
    await expectGolden('first_run_welcome_es');
  });

  testWidgets('primer arranque: importar', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: FakeBridge(instances: [], accounts: []),
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.import)],
    );
    await tester.tap(find.byIcon(Symbols.deployed_code_rounded).first);
    await settle(tester);
    await expectGolden('first_run_import_es');
  });

  testWidgets('primer arranque: cuenta (en)', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: FakeBridge(instances: [], language: 'en'),
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.account)],
    );
    await expectGolden('first_run_account_en');
  });

  testWidgets('primer arranque: listo', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.done)],
    );
    await expectGolden('first_run_done_es');
  });
}

final sampleLog = [
  for (final (i, (text, err)) in [
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
  ].indexed)
    LogLineView(text: text, stderr: err, seq: BigInt.from(i)),
];
