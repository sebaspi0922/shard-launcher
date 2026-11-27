// Referencias de paridad sacadas de la app Flutter: para cada escena, la
// captura (igual que un golden) y las cajas de los elementos clave en JSON.
//
// No vive en app/test para no tocar la app: tools/paridad.sh --refs lo copia
// allí, lo corre con `flutter test` y lo borra. Escribe en
// tools/paridad/ref/<escena>.{png,json}.
//
// Las escenas usan el mismo FakeBridge y el mismo arranque que
// test/goldens_test.dart; las que repiten un golden deben dar el mismo PNG.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shard/app.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/crash_notice.dart';
import 'package:shard/core/launch.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/core/content_search.dart';
import 'package:shard/core/tasks.dart';
import 'package:shard/ui/content/content_page.dart';
import 'package:shard/ui/content/modpack_dialog.dart';
import 'package:shard/ui/content/project_sheet.dart';
import 'package:shard/ui/servers/server_dialog.dart';
import 'package:shard/ui/servers/server_widgets.dart';
import 'package:shard/ui/servers/servers_page.dart';
import 'package:shard/ui/accounts/account_dialogs.dart';
import 'package:shard/ui/console/console_view.dart';
import 'package:shard/ui/instances/instance_settings_tab.dart';
import 'package:shard/ui/widgets/memory_slider.dart';
import 'package:shard/ui/accounts/account_panel.dart';
import 'package:shard/core/updates.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/ui/instances/change_version_dialog.dart';
import 'package:shard/ui/instances/create_instance_dialog.dart';
import 'package:shard/ui/instances/trash_dialog.dart';
import 'package:shard/ui/doctor/crash_banner.dart';
import 'package:shard/ui/doctor/doctor_dialog.dart';
import 'package:shard/ui/first_run/first_run_page.dart';
import 'package:shard/ui/settings/settings_page.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/ui/home/home_page.dart';
import 'package:shard/ui/shell/backdrop.dart';
import 'package:shard/ui/shell/update_banner.dart';
import 'package:shard/ui/instances/content_tab.dart';
import 'package:shard/ui/instances/backups_tab.dart';
import 'package:shard/ui/instances/instances_page.dart';
import 'package:shard/ui/shell/title_bar.dart';
import 'package:shard/ui/skins/skin_viewer.dart';
import 'package:shard/ui/skins/skins_page.dart';
import 'package:shard/ui/widgets/art.dart';
import 'package:shard/ui/widgets/avatar.dart';
import 'package:shard/ui/widgets/common.dart';
import 'package:shard/ui/widgets/media.dart';
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

/// Cajas del selector de versión y loader (create_instance_dialog.dart) dentro
/// de `dialog`: lista, filas visibles, interruptor de snapshots, opciones de
/// loader y su versión.
void _versionBoxes(WidgetTester tester, Finder dialog, Map<String, Rect> boxes) {
  Finder inDialog(Finder f) => find.descendant(of: dialog, matching: f);
  final list = inDialog(find.byType(ListView));
  _put(tester, boxes, 'versions', find.ancestor(of: list, matching: find.byType(Container)));
  final rows = find.descendant(of: list, matching: find.byType(InkWell));
  for (var i = 0; i < rows.evaluate().length; i++) {
    _put(tester, boxes, 'version-$i', rows.at(i));
  }
  _put(tester, boxes, 'snapshots', inDialog(find.byType(Switch)));
  final loaders = inDialog(find.byType(LoaderOption));
  for (var i = 0; i < loaders.evaluate().length; i++) {
    _put(tester, boxes, 'loader-$i', loaders.at(i));
  }
  _put(tester, boxes, 'loader-version', inDialog(find.byType(LoaderVersionField)));
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

/// Cajas del detalle de una instancia (fase 3): raíl, cabecera y sus
/// botones, pestañas y, si está abierto, el menú «Más opciones». `extra`
/// añade las de la pestaña o el diálogo de la escena. Los nombres son los
/// `accessible-id` de Slint (`tab-2-btn` = tercera `tab`, sin el ancho).
Map<String, Rect> _detailBoxes(WidgetTester tester, {required String name, Map<String, Finder> extra = const {}}) {
  final boxes = <String, Rect>{};
  void put(String key, Finder f) => _put(tester, boxes, key, f);
  final header = _named('_Header');
  put('rail', _named('_NavRail'));
  put('titlebar', find.byType(TitleBar));
  put('header', header);
  put('header-icon', find.descendant(of: header, matching: find.byType(InstanceIcon)));
  put('txt-name', find.descendant(of: header, matching: find.text(name)));
  put('back-btn', find.descendant(of: header, matching: find.byType(TextButton)));
  final icons = find.descendant(of: header, matching: find.byType(IconButton));
  if (icons.evaluate().length >= 2) {
    put('folder', icons.at(0));
    put('more', icons.at(1));
  }
  put('header-play', find.descendant(of: header, matching: find.byType(PlayButton)));
  final tabs = _named('_TabItem');
  for (var i = 0; i < tabs.evaluate().length; i++) {
    put('tab-$i-btn', tabs.at(i));
  }
  put('menu', _named('_MenuPanel'));
  for (final e in extra.entries) {
    put(e.key, e.value);
  }
  return boxes;
}

// F3 copias
/// Cajas de la pestaña Copias (backups_tab.dart): resumen, «Crear copia»,
/// filas con sus botones y el panel de copias automáticas.
Map<String, Finder> _backupsBoxes() {
  final tab = find.byType(BackupsTab);
  Finder inTab(Finder f) => find.descendant(of: tab, matching: f);
  final rows = inTab(_named('_BackupRow'));
  final auto = inTab(_named('_AutoSettings'));
  final pills = find.descendant(of: auto, matching: _named('_Pill'));
  final boxes = <String, Finder>{
    'txt-summary': inTab(find.textContaining('copias ·')),
    'create-play': inTab(find.byType(FilledButton)),
    'list': find.ancestor(of: inTab(find.byType(ListView)), matching: find.byType(Panel)).first,
    'auto': find.descendant(of: auto, matching: find.byType(Panel)),
    'auto-switch': find.descendant(of: auto, matching: find.byType(Switch)),
    'keep': find.descendant(of: auto, matching: find.byType(Slider)),
    'txt-keep': find.descendant(of: auto, matching: find.text('Copias automáticas que se conservan')),
    'txt-note': find.descendant(of: auto, matching: find.text('Las copias fijadas nunca se borran solas.')),
  };
  for (var i = 0; i < 5; i++) {
    boxes['interval-$i-btn'] = pills.at(i);
  }
  for (var i = 0; i < 4; i++) {
    final row = rows.at(i);
    boxes['backup-$i'] = row;
    boxes['pin-$i'] = find.descendant(of: row, matching: find.byType(IconButton)).first;
    boxes['restore-$i-btn'] = find.descendant(of: row, matching: find.byType(OutlinedButton));
    boxes['dots-$i'] = find.descendant(of: row, matching: find.byType(IconButton)).last;
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

/// Primer elemento cuyo tipo empieza por `prefix` (genéricos: `SelectButton<String?>`).
Finder _typeStarts(String prefix) =>
    find.byWidgetPredicate((w) => w.runtimeType.toString().startsWith(prefix));

/// Cajas de Contenido (fase 4). `card-i-play`: botón «Instalar» o insignia
/// «Instalado» de la tarjeta (anclados a la derecha).
Map<String, Rect> _contentBoxes(WidgetTester tester, {required String title, required String subtitle, String? results}) {
  final boxes = <String, Rect>{};
  Finder inPage(Finder f) => find.descendant(of: find.byType(ContentPage), matching: f);
  void put(String name, Finder f) => _put(tester, boxes, name, f);
  put('rail', _named('_NavRail'));
  put('txt-title', inPage(find.text(title)));
  put('txt-subtitle', inPage(find.text(subtitle)));
  put('source', inPage(_typeStarts('SegmentedButton')));
  put('import', inPage(find.byType(OutlinedButton)));
  put('target', inPage(_named('_TargetBanner')));
  put('search', inPage(find.byType(TextField)));
  final selects = inPage(_typeStarts('SelectButton'));
  for (var i = 0; i < selects.evaluate().length; i++) {
    put('filter-$i', selects.at(i));
  }
  final pills = inPage(_named('_Pill'));
  for (var i = 0; i < pills.evaluate().length; i++) {
    put('kind-$i', pills.at(i));
  }
  if (results != null) put('txt-results', inPage(find.text(results)));
  put('empty', inPage(find.byType(EmptyState)));
  put('empty-action', find.descendant(of: inPage(find.byType(EmptyState)), matching: find.byType(FilledButton)));
  final cards = inPage(find.byType(ProjectCard));
  for (var i = 0; i < cards.evaluate().length; i++) {
    final card = cards.at(i);
    put('card-$i', card);
    put('card-$i-icon', find.descendant(of: card, matching: find.byType(ProjectIcon)));
    final btn = find.descendant(of: card, matching: find.byType(FilledButton));
    put('card-$i-play', btn.evaluate().isNotEmpty ? btn : find.descendant(of: card, matching: find.byType(ColorBadge)));
  }
  return boxes;
}

/// Cajas de Servidores (fase 4).
Map<String, Rect> _serversBoxes(WidgetTester tester, {required String title, required String subtitle, required String add}) {
  final boxes = <String, Rect>{};
  Finder inPage(Finder f) => find.descendant(of: find.byType(ServersPage), matching: f);
  void put(String name, Finder f) => _put(tester, boxes, name, f);
  put('rail', _named('_NavRail'));
  put('txt-title', inPage(find.text(title)));
  put('txt-subtitle', inPage(find.text(subtitle)));
  put('import', inPage(find.byType(OutlinedButton)));
  put('add', find.ancestor(of: inPage(find.text(add)), matching: find.byType(FilledButton)));
  final cards = inPage(_named('_ServerCard'));
  for (var i = 0; i < cards.evaluate().length; i++) {
    final card = cards.at(i);
    put('server-$i', card);
    put('server-$i-icon', find.descendant(of: card, matching: find.byType(ServerFavicon)));
    put('server-$i-play', find.descendant(of: card, matching: find.byType(FilledButton)));
  }
  return boxes;
}

/// Desplazamiento vertical de la página de Ajustes (la escena de Slint lo
/// copia para ver la misma zona).
double? _settingsScroll(WidgetTester tester) {
  final f = find.descendant(of: find.byType(SettingsPage), matching: find.byType(Scrollable));
  if (f.evaluate().isEmpty) return null;
  return tester.state<ScrollableState>(f.first).position.pixels;
}

/// Cajas de Ajustes: grupos (título y panel), botones segmentados,
/// interruptores, deslizadores, tarjetas de estilo y fondo, campo y filas de
/// Java.
Map<String, Rect> _settingsBoxes(WidgetTester tester, {required String title}) {
  final boxes = <String, Rect>{};
  Finder inPage(Finder f) => find.descendant(of: find.byType(SettingsPage), matching: f);
  void put(String name, Finder f) => _put(tester, boxes, name, f);
  // Solo lo que se ve, numerado en orden (Slint no expone lo que queda
  // fuera de la ventana).
  bool shown(Rect r) => r.top < 800 && r.bottom > 44;
  void all(String kind, Finder f) {
    var n = 0;
    for (var i = 0; i < f.evaluate().length; i++) {
      final r = tester.getRect(f.at(i));
      if (!r.isEmpty && shown(r)) boxes['$kind-${n++}'] = r;
    }
  }

  put('rail', _named('_NavRail'));
  put('txt-title', inPage(find.text(title)));
  all('group', inPage(find.byType(SettingsGroup)));
  all('panel', inPage(find.byType(Panel)));
  all('seg', inPage(find.byType(SegmentedButton<String>)));
  all('switch', inPage(find.byType(Switch)));
  all('slider', inPage(find.byType(Slider)));
  all('option', inPage(_named('_BackgroundOption')));
  all('swatch', inPage(_named('_Swatch')));
  all('java', inPage(_named('_JavaRow')));
  all('install', inPage(_named('_InstallButton')));
  put('field', inPage(find.byType(TextField)));
  boxes.removeWhere((name, r) => name != 'rail' && !shown(r));
  return boxes;
}

/// Cajas del asistente de primer arranque.
Map<String, Rect> _firstRunBoxes(WidgetTester tester, {required String title}) {
  final boxes = <String, Rect>{};
  Finder inPage(Finder f) => find.descendant(of: find.byType(FirstRunPage), matching: f);
  void put(String name, Finder f) => _put(tester, boxes, name, f);
  void all(String kind, Finder f) {
    for (var i = 0; i < f.evaluate().length; i++) {
      put('$kind-$i', f.at(i));
    }
  }

  put('steps', inPage(_named('_StepRail')));
  put('wizard', find.ancestor(of: inPage(_named('_StepRail')), matching: find.byType(Container)).first);
  put('txt-title', inPage(find.text(title)));
  all('step', inPage(_named('_StepItem')));
  all('choice', inPage(_named('_ChoiceCard')));
  all('feature', inPage(_named('_Feature')));
  all('source', inPage(_named('_SourceCard')));
  all('summary-row', inPage(_named('_SummaryRow')));
  all('account', find.ancestor(of: inPage(find.byType(PlayerAvatar)), matching: find.byType(Panel)));
  put('filled', inPage(find.byType(FilledButton)));
  final texts = inPage(find.byType(TextButton));
  for (var i = 0; i < texts.evaluate().length; i++) {
    put('text-btn-$i', texts.at(i));
  }
  return boxes;
}

Future<void> _capture(WidgetTester tester, String scene, Map<String, Rect> boxes) async {
  try {
    await expectLater(find.byType(LauncherApp), matchesGoldenFile('$_goldenOut/$scene.png'));
  } finally {
    debugDisableShadows = true;
  }
  final json = {
    'atmosphere_s': _atmosphereSeconds(tester),
    'scroll_y': _settingsScroll(tester),
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

  // Fase 3: detalle de instancia. Cada pestaña o diálogo añade sus escenas
  // debajo de su marca (así las ramas no chocan).
  // F3 menú
  testWidgets('instance_menu_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=worlds');
    await tester.tap(find.byTooltip('Más opciones'));
    await settle(tester);
    await _capture(tester, 'instance_menu_es', _detailBoxes(tester, name: 'Survival con amigos'));
  });
  // F3 contenido
  testWidgets('instance_content_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=content');
    final tab = find.byType(ContentTab);
    Finder inTab(Finder f) => find.descendant(of: tab, matching: f);
    final pills = inTab(_named('_Pill'));
    final banners = inTab(_named('_Banner'));
    final rows = inTab(_named('_ContentRow'));
    Finder inRow(int i, Finder f) => find.descendant(of: rows.at(i), matching: f);
    await _capture(tester, 'instance_content_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      for (var i = 0; i < pills.evaluate().length; i++) 'filter-$i-btn': pills.at(i),
      'content-search': inTab(find.byType(TextField)),
      'content-refresh': inTab(find.byTooltip('Buscar actualizaciones')),
      'content-add-play': inTab(find.byType(FilledButton)),
      for (var i = 0; i < banners.evaluate().length; i++) 'banner-$i': banners.at(i),
      'txt-banner-0': inTab(find.text('2 actualizaciones disponibles')),
      'update-all-play': inTab(find.widgetWithText(TextButton, 'Actualizar todo')),
      'identify-play': inTab(find.widgetWithText(TextButton, 'Identificar')),
      'content-list': inTab(find.byType(Panel)),
      // Las filas que se ven (la lista sigue debajo del borde de la ventana).
      for (var i = 0; i < 5 && i < rows.evaluate().length; i++) ...{
        'content-row-$i': rows.at(i),
        'content-icon-$i': inRow(i, find.byType(ProjectIcon)),
        'txt-content-name-$i': inRow(i, find.byType(Text)).first,
        'content-switch-$i': inRow(i, find.byType(Switch)),
        'content-remove-$i': inRow(i, find.byType(IconButton)),
      },
      'content-update-play': inTab(find.byType(OutlinedButton)),
    }));
  });
  // F3 mundos
  testWidgets('instance_worlds_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=worlds');
    final detail = _named('_WorldDetail');
    Finder inDetail(Finder f) => find.descendant(of: detail, matching: f);
    final cards = _named('_WorldCard');
    final tags = inDetail(find.byType(Tag));
    final segs = inDetail(find.byType(SegmentedButton<String>));
    final switches = inDetail(find.byType(Switch));
    await _capture(tester, 'instance_worlds_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      for (var i = 0; i < cards.evaluate().length; i++) ...{
        'world-$i': cards.at(i),
        'world-icon-$i': find.descendant(of: cards.at(i), matching: _named('WorldIcon')),
        'txt-world-$i': find.descendant(of: cards.at(i), matching: find.byType(Text)).first,
      },
      'world-detail': detail,
      'world-big-icon': inDetail(_named('WorldIcon')),
      'txt-world-name': inDetail(find.text('Mundo de verano')),
      'rename': inDetail(find.byTooltip('Cambiar nombre')),
      'world-folder': inDetail(find.byTooltip('Abrir carpeta')),
      for (var i = 0; i < tags.evaluate().length; i++) 'world-tag-$i-btn': tags.at(i),
      'txt-world-game': inDetail(find.text('Partida')),
      if (segs.evaluate().length == 2) ...{'mode': segs.at(0), 'difficulty': segs.at(1)},
      // Los interruptores de hardcore y trucos (los de datapacks quedan fuera).
      for (var i = 0; i < 2 && i < switches.evaluate().length; i++) 'switch-$i': switches.at(i),
      'txt-datapacks': inDetail(find.text('Datapacks')),
      'add-datapacks-btn': inDetail(find.widgetWithText(TextButton, 'Añadir datapacks')),
    }));
  });
  // F3 capturas
  testWidgets('instance_screenshots_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=screenshots');
    final thumbs = _named('_Thumb');
    await _capture(tester, 'instance_screenshots_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      'txt-shots-count': find.text('9 capturas'),
      'shots-folder-btn': find.widgetWithText(TextButton, 'Abrir carpeta'),
      for (var i = 0; i < thumbs.evaluate().length; i++) 'shot-$i': thumbs.at(i),
    }));
  });
  // F3 copias
  testWidgets('instance_backups_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=backups');
    await _capture(tester, 'instance_backups_es',
        _detailBoxes(tester, name: 'Survival con amigos', extra: _backupsBoxes()));
  });

  testWidgets('backup_restore_es', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(tester, location: '/instances/survival-amigos?tab=backups', bridge: bridge);
    final i = c.read(instanceByIdProvider('survival-amigos'))!;
    showDialog<void>(
      context: pageContext(tester),
      builder: (_) => RestoreBackupDialog(instance: i, backup: bridge.backups[1]),
    );
    await settle(tester);
    final dialog = find.byType(RestoreBackupDialog);
    Finder inDialog(Finder f) => find.descendant(of: dialog, matching: f);
    final modes = inDialog(_named('_ModeOption'));
    await _capture(tester, 'backup_restore_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      'dialog': inDialog(find.byType(Material)),
      'txt-dialog-title': inDialog(find.text('Restaurar copia')),
      'txt-from': inDialog(find.textContaining('Copia «')),
      'mode-0': modes.at(0),
      'mode-1': modes.at(1),
      'warn': inDialog(find.byWidgetPredicate(
          (w) => w is Container && w.decoration is BoxDecoration && (w.decoration! as BoxDecoration).border != null)),
      'safety': inDialog(find.byType(CheckboxListTile)),
      'cancel-btn': inDialog(find.byType(TextButton)),
      'filled-play': inDialog(find.byType(FilledButton)),
    }));
  });
  // F3 registros
  testWidgets('instance_logs_es', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(tester, location: '/instances/survival-amigos?tab=logs', bridge: bridge);
    c.read(launchProvider.notifier).debugSet(
          'survival-amigos',
          LaunchState(phase: LaunchPhase.running, gameId: BigInt.one),
        );
    bridge.emit(UiEvent.gameStarted(gameId: BigInt.one, instanceId: 'survival-amigos'));
    bridge.emit(UiEvent.gameLog(gameId: BigInt.one, lines: _sampleLog));
    await settle(tester);
    Finder inView(Finder f) => find.descendant(of: find.byType(ConsoleView), matching: f);
    final chips = inView(_named('_LevelChip'));
    final tools = inView(_named('_ToolButton'));
    final lines = inView(_named('_LogLine'));
    await _capture(tester, 'instance_logs_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      'filter': inView(find.byType(TextField)),
      for (var i = 0; i < 4; i++) 'chip-$i': chips.at(i),
      'txt-lines': inView(find.text('21 líneas')),
      for (var i = 0; i < 3; i++) 'tool-$i': tools.at(i),
      'console': find.ancestor(of: inView(find.byType(Scrollbar)), matching: find.byType(Container)).first,
      'line-0': lines.at(0),
      'line-5': lines.at(5),
      'line-10': lines.at(10),
      'line-20': lines.at(20),
    }));
  });
  // F3 ajustes
  testWidgets('instance_settings_es', (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=settings');
    Finder inTab(Finder f) => find.descendant(of: find.byType(InstanceSettingsTab), matching: f);
    final sections = inTab(_named('_Section'));
    final fields = inTab(find.byType(TextField));
    final switches = inTab(_named('_GlobalSwitch'));
    await _capture(tester, 'instance_settings_es', _detailBoxes(tester, name: 'Survival con amigos', extra: {
      for (var i = 0; i < 4; i++) 'section-$i': sections.at(i),
      'name-field': fields.at(0),
      'jvm-field': fields.at(1),
      'width-field': fields.at(2),
      'height-field': fields.at(3),
      for (var i = 0; i < 2; i++) 'global-switch-$i': switches.at(i),
      'memory': inTab(find.byType(MemorySlider)),
      'fullscreen': inTab(find.byType(Switch)).last,
      'txt-resolution': inTab(find.text('Vacío = tamaño por defecto del juego.')),
    }));
  });
  // F3 crear instancia
  testWidgets('create_instance_es', (tester) async {
    await pumpApp(tester, location: '/instances');
    showCreateInstance(pageContext(tester));
    await settle(tester);
    await tester.tap(find.text('Fabric'));
    await settle(tester);
    final boxes = _dialogBoxes<CreateInstanceDialog>(tester, title: 'Nueva instancia');
    _versionBoxes(tester, find.byType(CreateInstanceDialog), boxes);
    await _capture(tester, 'create_instance_es', boxes);
  });
  // F3 cambiar versión
  testWidgets('change_version_es', (tester) async {
    final c = await pumpApp(tester, location: '/instances/survival-amigos?tab=settings');
    showChangeVersion(pageContext(tester), c.read(instanceByIdProvider('survival-amigos'))!);
    await settle(tester);
    await tester.tap(find.text('NeoForge'));
    await settle(tester);
    final boxes = _detailBoxes(tester, name: 'Survival con amigos');
    boxes.addAll(_dialogBoxes<ChangeVersionDialog>(tester, title: 'Cambiar versión de «Survival con amigos»'));
    final dialog = find.byType(ChangeVersionDialog);
    _versionBoxes(tester, dialog, boxes);
    final tags = find.descendant(of: dialog, matching: find.byType(Tag));
    for (var i = 0; i < tags.evaluate().length; i++) {
      _put(tester, boxes, 'version-tag-$i-btn', tags.at(i));
    }
    _put(tester, boxes, 'warning', find.descendant(of: dialog, matching: find.byWidgetPredicate(
        (w) => w is Container && w.padding == const EdgeInsets.fromLTRB(12, 8, 12, 8))));
    _put(tester, boxes, 'backup', find.descendant(of: dialog, matching: find.byType(Checkbox)));
    await _capture(tester, 'change_version_es', boxes);
  });
  // F3 papelera
  testWidgets('trash_en', (tester) async {
    await pumpApp(tester, location: '/instances', bridge: FakeBridge(language: 'en'));
    showTrash(pageContext(tester));
    await settle(tester);
    final boxes = _dialogBoxes<TrashDialog>(tester, title: 'Trash');
    final dialog = find.byType(TrashDialog);
    final rows = find.descendant(of: dialog, matching: find.byType(Panel));
    for (var i = 0; i < rows.evaluate().length; i++) {
      final row = rows.at(i);
      _put(tester, boxes, 'trash-row-$i', row);
      _put(tester, boxes, 'trash-icon-$i', find.descendant(of: row, matching: find.byType(InstanceIcon)));
      _put(tester, boxes, 'trash-left-$i-btn', find.descendant(of: row, matching: find.byType(Tag)));
      _put(tester, boxes, 'trash-purge-$i', find.descendant(of: row, matching: find.byType(IconButton)));
      _put(tester, boxes, 'trash-restore-$i-btn', find.descendant(of: row, matching: find.byType(OutlinedButton)));
    }
    _put(tester, boxes, 'txt-trash-hint', find.descendant(of: dialog, matching: find.textContaining('30 days')));
    await _capture(tester, 'trash_en', boxes);
  // Fase 4: contenido y servidores (mismas escenas que los goldens).
  const contentEs = (title: 'Contenido', subtitle: 'Mods, paquetes de recursos, shaders, datapacks y modpacks.');

  testWidgets('content_search_es', (tester) async {
    await pumpApp(tester, location: '/content');
    await _capture(tester, 'content_search_es',
        _contentBoxes(tester, title: contentEs.title, subtitle: contentEs.subtitle, results: '6 resultados'));
  });

  testWidgets('content_target_es', (tester) async {
    await pumpApp(tester, location: '/content?instance=survival-amigos&kind=mod');
    await _capture(tester, 'content_target_es',
        _contentBoxes(tester, title: contentEs.title, subtitle: contentEs.subtitle, results: '6 resultados'));
  });

  testWidgets('content_curseforge_en', (tester) async {
    final c = await pumpApp(tester, location: '/content', bridge: FakeBridge(language: 'en'));
    final n = c.read(contentSearchProvider.notifier);
    n.setFilters(c.read(contentSearchProvider).filters.copyWith(source: 'curseforge'));
    await settle(tester);
    await _capture(tester, 'content_curseforge_en',
        _contentBoxes(tester, title: 'Content', subtitle: 'Mods, resource packs, shaders, datapacks and modpacks.'));
  });

  testWidgets('project_sheet_es', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/content?instance=survival-amigos&kind=mod', bridge: bridge);
    showProjectSheet(pageContext(tester), bridge.searchResults.first);
    await settle(tester);
    final boxes = <String, Rect>{};
    void put(String name, Finder f) => _put(tester, boxes, name, f);
    final sheet = find.byType(ProjectSheet);
    Finder inSheet(Finder f) => find.descendant(of: sheet, matching: f);
    put('rail', _named('_NavRail'));
    put('sheet', sheet);
    put('sheet-icon', inSheet(find.byType(ProjectIcon)));
    put('txt-sheet-title', inSheet(find.text('Sodium')));
    put('sheet-select', inSheet(_typeStarts('SelectButton')));
    put('txt-installed', inSheet(find.text('Ya está instalado en esta instancia (0.6.13+mc1.21.4).')));
    put('txt-versions', inSheet(find.text('Versiones compatibles')));
    put('latest-play', inSheet(find.byType(FilledButton)));
    final rows = inSheet(_named('_VersionRow'));
    for (var i = 0; i < rows.evaluate().length; i++) {
      put('version-$i', rows.at(i));
      put('version-$i-play', find.descendant(of: rows.at(i), matching: find.byType(OutlinedButton)));
    }
    await _capture(tester, 'project_sheet_es', boxes);
  });

  testWidgets('modpack_installing_es', (tester) async {
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
          TaskProgress(id: BigInt.from(9), stage: 'modpack', done: 87, total: 212, bytes: false),
        );
    await settle(tester);
    await _capture(tester, 'modpack_installing_es',
        _dialogBoxes<ModpackDialog>(tester, title: 'Instalar «Fabulously Optimized»'));
  });

  testWidgets('servers_es', (tester) async {
    await pumpApp(tester, location: '/servers');
    await _capture(tester, 'servers_es',
        _serversBoxes(tester, title: 'Servidores', subtitle: '4 servidores · 3 en línea', add: 'Añadir servidor'));
  });

  testWidgets('server_dialog_en', (tester) async {
    await pumpApp(tester, location: '/servers', bridge: FakeBridge(language: 'en'));
    showServerDialog(pageContext(tester));
    await settle(tester);
    // `code` y `outlined` caen en el icono y el botón del desplegable: van como select-*.
    final boxes = _dialogBoxes<ServerDialog>(tester, title: 'Add server')
      ..remove('code')
      ..remove('outlined');
    final selects = find.descendant(of: find.byType(ServerDialog), matching: _typeStarts('SelectButton'));
    for (var i = 0; i < selects.evaluate().length; i++) {
      _put(tester, boxes, 'select-$i', selects.at(i));
    }
    await _capture(tester, 'server_dialog_en', boxes);
  });

  // Fase 5: Ajustes, primer arranque y Doctor. Mismos pasos que sus goldens.
  testWidgets('settings_es', (tester) async {
    await pumpApp(tester, location: '/settings');
    await _capture(tester, 'settings_es', _settingsBoxes(tester, title: 'Ajustes'));
  });

  testWidgets('settings_appearance_es', (tester) async {
    await pumpApp(tester, location: '/settings');
    await tester.drag(find.text('General'), const Offset(0, -300));
    await settle(tester);
    await _capture(tester, 'settings_appearance_es', _settingsBoxes(tester, title: 'Ajustes'));
  });

  testWidgets('settings_java_es', (tester) async {
    await pumpApp(tester, location: '/settings');
    await tester.drag(find.text('General'), const Offset(0, -900));
    await settle(tester);
    await _capture(tester, 'settings_java_es', _settingsBoxes(tester, title: 'Ajustes'));
  });

  testWidgets('settings_integrations_es', (tester) async {
    await pumpApp(tester, location: '/settings', bridge: FakeBridge()..curseforgeKey = true);
    await tester.drag(find.text('General'), const Offset(0, -2400));
    await settle(tester);
    await _capture(tester, 'settings_integrations_es', _settingsBoxes(tester, title: 'Ajustes'));
  });

  testWidgets('settings_update_en', (tester) async {
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
    await _capture(tester, 'settings_update_en', _settingsBoxes(tester, title: 'Settings'));
  });

  testWidgets('first_run_welcome_es', (tester) async {
    await pumpApp(tester, location: '/welcome', bridge: FakeBridge(instances: [], accounts: [], language: null));
    await _capture(tester, 'first_run_welcome_es', _firstRunBoxes(tester, title: 'Welcome to Shard'));
  });

  testWidgets('first_run_import_es', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: FakeBridge(instances: [], accounts: []),
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.import)],
    );
    await tester.tap(find.byIcon(Symbols.deployed_code_rounded).first);
    await settle(tester);
    await _capture(tester, 'first_run_import_es', _firstRunBoxes(tester, title: 'Trae lo que ya tienes'));
  });

  // Sin regenerar: Sebas decidió quitar la cuenta Shard y Flutter aún tiene
  // su tarjeta. La referencia es el golden first_run_account_en (Microsoft y
  // offline) y las cajas choice-* se midieron en él (ver el JSON).
  testWidgets('first_run_account_en', skip: true, (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: FakeBridge(instances: [], language: 'en'),
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.account)],
    );
    await _capture(tester, 'first_run_account_en', _firstRunBoxes(tester, title: 'Add your account'));
  });

  testWidgets('first_run_done_es', (tester) async {
    await pumpApp(
      tester,
      location: '/welcome',
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.done)],
    );
    await _capture(tester, 'first_run_done_es', _firstRunBoxes(tester, title: 'Todo listo'));
  });

  // El golden abre Doctor sobre el detalle de la instancia (fase 3); aquí va
  // sobre Instancias para no depender de esa pantalla. Mismo diálogo.
  testWidgets('doctor_es', (tester) async {
    final c = await pumpApp(tester, location: '/instances');
    showDoctor(pageContext(tester), c.read(instanceByIdProvider('survival-amigos'))!);
    await settle(tester);
    final boxes = <String, Rect>{};
    final dialog = find.byType(DoctorDialog);
    Finder inDialog(Finder f) => find.descendant(of: dialog, matching: f);
    void put(String name, Finder f) => _put(tester, boxes, name, f);
    put('rail', _named('_NavRail'));
    put('dialog', inDialog(find.byType(Material)));
    put('txt-dialog-title', inDialog(find.text('Doctor · Survival con amigos')));
    put('excerpt', inDialog(_named('_CrashExcerpt')));
    // Solo las tarjetas que asoman en la zona con scroll (Slint no expone
    // lo recortado): acaba 92 px por encima del borde del diálogo.
    final bottom = tester.getRect(inDialog(find.byType(Material)).first).bottom - 92;
    final cards = inDialog(_named('_DiagnosisCard'));
    for (var i = 0; i < cards.evaluate().length; i++) {
      if (tester.getRect(cards.at(i)).top < bottom) put('diag-$i', cards.at(i));
    }
    put('filled', inDialog(find.widgetWithText(FilledButton, 'Cerrar')));
    put('cancel', inDialog(find.widgetWithText(TextButton, 'Volver a analizar')));
    await _capture(tester, 'doctor_es', boxes);
  });
}

/// `sampleLog` de test/goldens_test.dart (escena de registros).
final _sampleLog = [
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
