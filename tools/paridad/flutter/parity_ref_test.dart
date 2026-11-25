// Referencias de paridad sacadas de la app Flutter: para cada escena, la
// captura (igual que un golden) y las cajas de los elementos clave en JSON.
//
// No vive en app/test para no tocar la app: tools/paridad.sh --refs lo copia
// allí, lo corre con `flutter test` y lo borra. Escribe en
// tools/paridad/ref/<escena>.{png,json}.
//
// Las escenas usan el mismo FakeBridge y el mismo arranque que
// test/goldens_test.dart; las que repiten un golden deben dar el mismo PNG.

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shard/app.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/crash_notice.dart';
import 'package:shard/core/launch.dart';
import 'package:shard/ui/accounts/account_dialogs.dart';
import 'package:shard/ui/accounts/account_panel.dart';
import 'package:shard/core/updates.dart';
import 'package:shard/ui/doctor/crash_banner.dart';
import 'package:shard/ui/home/home_page.dart';
import 'package:shard/ui/shell/backdrop.dart';
import 'package:shard/ui/shell/update_banner.dart';
import 'package:shard/ui/instances/instances_page.dart';
import 'package:shard/ui/shell/title_bar.dart';
import 'package:shard/ui/skins/skin_viewer.dart';
import 'package:shard/ui/skins/skins_page.dart';
import 'package:shard/ui/widgets/art.dart';
import 'package:shard/ui/widgets/common.dart';
import 'package:shard/ui/widgets/play_button.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';
import 'support/updates.dart';

// Relativo al test (golden) y a app/ (JSON).
const _goldenOut = '../../tools/paridad/ref';
const _jsonOut = '../tools/paridad/ref';

Finder _named(String type) =>
    find.byWidgetPredicate((w) => w.runtimeType.toString() == type);

Finder _inPage(Finder f) =>
    find.descendant(of: find.byType(InstancesPage), matching: f);

/// Cajas de la pantalla Instancias. Los nombres que empiezan por `txt-` son
/// textos: se comparan por posición y alto (el ancho de un texto depende del
/// motor de texto, no del layout).
Map<String, Rect> _instancesBoxes(WidgetTester tester, {required String title, required String count}) {
  final boxes = <String, Rect>{};
  void put(String name, Finder f) {
    if (f.evaluate().isEmpty) return;
    final r = tester.getRect(f.first);
    if (!r.isEmpty) boxes[name] = r;
  }

  put('rail', _named('_NavRail'));
  put('titlebar', find.byType(TitleBar));
  put('pill', _named('_ActivityPill'));
  put('txt-title', _inPage(find.text(title)));
  put('txt-count', _inPage(find.text(count)));
  put('search', _inPage(find.byType(TextField)));
  put('view-toggle', _inPage(find.byType(SegmentedButton<bool>)));
  put('trash', _inPage(find.byWidgetPredicate(
      (w) => w is IconButton && (w.tooltip == 'Papelera' || w.tooltip == 'Trash'))));
  put('new', _inPage(find.byType(FilledButton)));
  put('empty', _inPage(find.byType(EmptyState)));
  for (final (kind, type) in [('card', InstanceCard), ('row', InstanceRow)]) {
    final items = _inPage(find.byType(type));
    for (var i = 0; i < items.evaluate().length; i++) {
      final item = items.at(i);
      boxes['$kind-$i'] = tester.getRect(item);
      final icon = find.descendant(of: item, matching: find.byType(InstanceIcon));
      if (icon.evaluate().isNotEmpty) boxes['$kind-$i-icon'] = tester.getRect(icon);
      final play = find.descendant(of: item, matching: find.byType(PlayButton));
      if (play.evaluate().isNotEmpty) boxes['$kind-$i-play'] = tester.getRect(play);
    }
  }
  return boxes;
}

/// Pone en `boxes` la caja del primer elemento de `f` (si existe y no está vacía).
void _put(WidgetTester tester, Map<String, Rect> boxes, String name, Finder f) {
  if (f.evaluate().isEmpty) return;
  final r = tester.getRect(f.first);
  if (!r.isEmpty) boxes[name] = r;
}

/// Cajas de un diálogo `AppDialog` (common.dart) de tipo `T`: panel, título y
/// botones por tipo.
Map<String, Rect> _dialogBoxes<T>(WidgetTester tester, {required String title}) {
  final boxes = <String, Rect>{};
  final dialog = find.byType(T);
  Finder inDialog(Finder f) => find.descendant(of: dialog, matching: f);
  _put(tester, boxes, 'rail', _named('_NavRail'));
  _put(tester, boxes, 'dialog', inDialog(find.byType(Material)));
  _put(tester, boxes, 'txt-dialog-title', inDialog(find.text(title)));
  _put(tester, boxes, 'field', inDialog(find.byType(TextField)));
  final fields = inDialog(find.byType(TextField));
  if (fields.evaluate().length > 1) _put(tester, boxes, 'field-2', fields.at(1));
  _put(tester, boxes, 'code', inDialog(find.byWidgetPredicate(
      (w) => w is Container && w.decoration is BoxDecoration && (w.decoration! as BoxDecoration).border != null)));
  _put(tester, boxes, 'outlined', inDialog(find.byType(OutlinedButton)));
  _put(tester, boxes, 'filled', inDialog(find.byType(FilledButton)));
  _put(tester, boxes, 'cancel', inDialog(find.byType(TextButton)));
  _put(tester, boxes, 'progress', inDialog(find.byType(LinearProgressIndicator)));
  return boxes;
}

/// Cajas de la pantalla Skins.
Map<String, Rect> _skinsBoxes(WidgetTester tester, {required String title, required String account, String? reset}) {
  final boxes = <String, Rect>{};
  Finder inPage(Finder f) => find.descendant(of: find.byType(SkinsPage), matching: f);
  void put(String name, Finder f) => _put(tester, boxes, name, f);
  put('rail', _named('_NavRail'));
  put('txt-title', inPage(find.text(title)));
  put('chip', find.ancestor(of: inPage(find.text(account)), matching: find.byType(InkWell)));
  put('empty', inPage(find.byType(EmptyState)));
  put('empty-action', find.descendant(of: inPage(find.byType(EmptyState)), matching: find.byType(FilledButton)));
  put('viewer', inPage(_named('_ViewerPanel')));
  final stages = inPage(find.byType(SkinStage));
  for (var i = 0; i < stages.evaluate().length; i++) {
    put('stage-$i', stages.at(i));
  }
  put('change', inPage(_named('_ChangeSkinCard')));
  put('source', inPage(_named('SegmentedButton<_Source>')));
  final models = inPage(_named('_ModelOption'));
  for (var i = 0; i < models.evaluate().length; i++) {
    put('model-$i', models.at(i));
  }
  if (reset != null) put('reset', find.ancestor(of: inPage(find.text(reset)), matching: find.byType(TextButton)));
  put('apply', find.descendant(of: inPage(_named('_ChangeSkinCard')), matching: find.byType(FilledButton)));
  put('capes', inPage(_named('_CapesCard')));
  final capes = inPage(_named('_CapeTile'));
  for (var i = 0; i < capes.evaluate().length; i++) {
    put('cape-$i', capes.at(i));
  }
  return boxes;
}

Finder _inHome(Finder f) =>
    find.descendant(of: find.byType(HomePage), matching: f);

/// Cajas de Inicio y de los avisos flotantes. `*-btn`: botón anclado a la
/// izquierda tan ancho como su texto (x, y y alto).
Map<String, Rect> _homeBoxes(WidgetTester tester, {required Map<String, String> texts}) {
  final boxes = <String, Rect>{};
  void put(String name, Finder f) {
    if (f.evaluate().isEmpty) return;
    final r = tester.getRect(f.first);
    if (!r.isEmpty) boxes[name] = r;
  }

  put('rail', _named('_NavRail'));
  put('titlebar', find.byType(TitleBar));
  put('pill', _named('_ActivityPill'));
  for (final e in texts.entries) {
    put('txt-${e.key}', _inHome(find.text(e.value)));
  }
  final hero = _named('_HeroCard');
  put('hero', hero);
  put('hero-icon', find.descendant(of: hero, matching: find.byType(InstanceIcon)));
  put('hero-play-btn', find.descendant(of: hero, matching: find.byType(PlayButton)));
  put('hero-picker-btn', find.descendant(of: hero, matching: _named('_InstancePicker')));
  put('hero-failure', find.descendant(of: hero, matching: _named('LaunchFailureNote')));
  put('no-instances', _named('_NoInstancesCard'));
  put('session', _named('_SessionCard'));
  put('running', _named('_RunningCard'));
  put('servers', _named('_QuickServersCard'));
  final recent = _named('_RecentCard');
  for (var i = 0; i < recent.evaluate().length; i++) {
    boxes['recent-$i'] = tester.getRect(recent.at(i));
    final icon = find.descendant(of: recent.at(i), matching: find.byType(InstanceIcon));
    if (icon.evaluate().isNotEmpty) boxes['recent-$i-icon'] = tester.getRect(icon);
  }
  for (final (name, type) in [('update-banner', UpdateBanner), ('crash-banner', CrashBanner)]) {
    final f = find.byType(type);
    if (f.evaluate().isNotEmpty) {
      final r = tester.getRect(f);
      if (r.height > 1) boxes[name] = r;
    }
  }
  return boxes;
}

/// Segundos del giro de la esfera ASCII en la captura (estilo sebaspi).
double? _sphereSeconds(WidgetTester tester) {
  for (final w in tester.widgetList<CustomPaint>(find.byType(CustomPaint))) {
    final p = w.painter;
    if (p != null && p.runtimeType.toString() == '_SpherePainter') {
      final d = p as dynamic;
      return (d.base() as double) + (d.time.value as double);
    }
  }
  return null;
}

/// Segundos del reloj del fondo animado en la captura (la escena de Slint
/// pone el fondo quieto en ese mismo instante).
double? _atmosphereSeconds(WidgetTester tester) {
  for (final w in tester.widgetList<CustomPaint>(find.byType(CustomPaint))) {
    final p = w.painter;
    if (p != null && p.runtimeType.toString() == '_AtmospherePainter') {
      return ((p as dynamic).clock as Animation<double>).value * 18;
    }
  }
  return null;
}

Future<void> _capture(WidgetTester tester, String scene, Map<String, Rect> boxes) async {
  try {
    await expectLater(find.byType(LauncherApp), matchesGoldenFile('$_goldenOut/$scene.png'));
  } finally {
    debugDisableShadows = true;
  }
  final json = {
    'atmosphere_s': _atmosphereSeconds(tester),
    'sphere_s': _sphereSeconds(tester),
    'boxes': {
      for (final e in boxes.entries)
        e.key: [e.value.left, e.value.top, e.value.width, e.value.height]
            .map((v) => double.parse(v.toStringAsFixed(2)))
            .toList(),
    },
  };
  File('$_jsonOut/$scene.json').writeAsStringSync('${const JsonEncoder.withIndent('  ').convert(json)}\n');
}

void main() {
  setUpAll(() => autoUpdateGoldenFiles = true);

  testWidgets('instances_grid_es', (tester) async {
    final c = await pumpApp(tester, location: '/instances');
    c.read(launchProvider.notifier).debugSet(
          'create-aeronautica',
          const LaunchState(phase: LaunchPhase.running),
        );
    await settle(tester);
    await _capture(tester, 'instances_grid_es',
        _instancesBoxes(tester, title: 'Instancias', count: '5 instancias'));
  });

  testWidgets('instances_list_en', (tester) async {
    final c = await pumpApp(tester, location: '/instances', bridge: FakeBridge(language: 'en'));
    c.read(instancesGridProvider.notifier).set(false);
    await settle(tester);
    await _capture(tester, 'instances_list_en',
        _instancesBoxes(tester, title: 'Instances', count: '5 instances'));
  });

  testWidgets('instances_empty_es', (tester) async {
    await pumpApp(tester, location: '/instances', bridge: FakeBridge(instances: []));
    await _capture(tester, 'instances_empty_es',
        _instancesBoxes(tester, title: 'Instancias', count: 'Ninguna instancia'));
  });

  testWidgets('instances_search_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    await tester.enterText(_inPage(find.byType(TextField)), 'sur');
    await settle(tester);
    await _capture(tester, 'instances_search_es',
        _instancesBoxes(tester, title: 'Instancias', count: '5 instancias'));
  });

  testWidgets('instances_noresults_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    await tester.enterText(_inPage(find.byType(TextField)), 'zzz');
    await settle(tester);
    await _capture(tester, 'instances_noresults_es',
        _instancesBoxes(tester, title: 'Instancias', count: '5 instancias'));
  });

  // Fase 2: cuentas y skins. Los goldens de cuentas abren el panel y los
  // diálogos sobre Inicio; aquí van sobre Instancias para no depender de la
  // pantalla de Inicio (fase 1). Mismo panel y diálogos.
  testWidgets('accounts_panel_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    showAccountPanel(pageContext(tester));
    await settle(tester);
    final boxes = <String, Rect>{};
    void put(String name, Finder f) => _put(tester, boxes, name, f);
    put('rail', _named('_NavRail'));
    put('panel', find.byType(AccountPanel));
    put('txt-panel-title', find.descendant(of: find.byType(AccountPanel), matching: find.text('Cuentas')));
    final tiles = _named('_AccountTile');
    for (var i = 0; i < tiles.evaluate().length; i++) {
      put('account-$i', tiles.at(i));
    }
    final actions = _named('_ActionTile');
    for (var i = 0; i < actions.evaluate().length; i++) {
      put('action-$i', actions.at(i));
    }
    await _capture(tester, 'accounts_panel_es', boxes);
  });

  testWidgets('ms_login_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    showMicrosoftLogin(pageContext(tester));
    await settle(tester);
    await _capture(tester, 'ms_login_es',
        _dialogBoxes<MicrosoftLoginDialog>(tester, title: 'Iniciar sesión con Microsoft'));
  });

  testWidgets('offline_account_en', (tester) async {
    await pumpApp(tester, location: '/instances', bridge: FakeBridge(language: 'en'));
    showOfflineAccount(pageContext(tester));
    await settle(tester);
    await _capture(tester, 'offline_account_en',
        _dialogBoxes<OfflineAccountDialog>(tester, title: 'Offline account'));
  });

  testWidgets('shard_account_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    showShardAccount(pageContext(tester));
    await settle(tester);
    await _capture(tester, 'shard_account_es',
        _dialogBoxes<AccountCredentialsDialog>(tester, title: 'Cuenta Shard'));
  });

  testWidgets('skins_es', (tester) async {
    await pumpApp(tester, location: '/skins');
    await _capture(tester, 'skins_es', _skinsBoxes(tester, title: 'Skins', account: 'NadiaK', reset: 'Restablecer'));
  });

  testWidgets('skins_offline_en', (tester) async {
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
    await _capture(tester, 'skins_offline_en', _skinsBoxes(tester, title: 'Skins', account: 'Pruebas_01'));
  });

  const homeEs = {'greeting': 'Hola, NadiaK', 'subtitle': '¿A qué jugamos hoy?', 'hero-name': 'Survival con amigos', 'recent': 'Recientes'};

  testWidgets('home_es', (tester) async {
    await pumpApp(tester, location: '/');
    await _capture(tester, 'home_es', _homeBoxes(tester, texts: homeEs));
  });

  testWidgets('home_downloading_es', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          const LaunchState(phase: LaunchPhase.downloading, done: 1834, total: 3920, stage: 'assets'),
        );
    await settle(tester);
    await _capture(tester, 'home_downloading_es', _homeBoxes(tester, texts: homeEs));
  });

  testWidgets('home_error_es', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          const LaunchState(
            phase: LaunchPhase.error,
            failure: LaunchError(
              BridgeError(kind: 'java_required', message: 'hace falta Java 21: instálala con `lcore java install 21`'),
            ),
          ),
        );
    await settle(tester);
    await _capture(tester, 'home_error_es', _homeBoxes(tester, texts: homeEs));
  });

  testWidgets('home_empty_en', (tester) async {
    await pumpApp(tester, location: '/', bridge: FakeBridge(instances: [], accounts: [], language: 'en'));
    await _capture(tester, 'home_empty_en',
        _homeBoxes(tester, texts: {'greeting': 'Welcome', 'subtitle': 'What are we playing today?'}));
  });

  for (final style in ['graphite', 'sebaspi', 'landscape', 'aurora']) {
    testWidgets('home_style_${style}_es', (tester) async {
      await pumpApp(
        tester,
        location: '/',
        bridge: FakeBridge()..uiPrefs = '{"first_run_done":true,"style":"$style","blur":10,"dim":0.5}',
      );
      await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 200)));
      await tester.pump(const Duration(milliseconds: 100));
      await _capture(tester, 'home_style_${style}_es', _homeBoxes(tester, texts: homeEs));
    });
  }

  testWidgets('web_home_graphite_es', (tester) async {
    debugHideBackdrop = true;
    try {
      await pumpApp(tester, location: '/', bridge: FakeBridge()..uiPrefs = '{"first_run_done":true,"style":"graphite"}');
      await _capture(tester, 'web_home_graphite_es', _homeBoxes(tester, texts: homeEs));
    } finally {
      debugHideBackdrop = false;
    }
  });

  testWidgets('crash_banner_es', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(crashNoticeProvider.notifier).debugShow(
          CrashNotice(gameId: BigInt.two, instanceId: 'create-aeronautica', code: 1),
        );
    await settle(tester);
    await _capture(tester, 'crash_banner_es', _homeBoxes(tester, texts: homeEs));
  });

  testWidgets('update_banner_es', (tester) async {
    final c = await pumpApp(tester, location: '/');
    c.read(updateProvider.notifier).debugSet(const UpdateState(status: UpdateStatus.available, info: appImageUpdate));
    await settle(tester);
    await _capture(tester, 'update_banner_es', _homeBoxes(tester, texts: homeEs));
  });
}
