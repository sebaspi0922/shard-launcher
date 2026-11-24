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
import 'package:shard/core/launch.dart';
import 'package:shard/ui/instances/instances_page.dart';
import 'package:shard/ui/shell/title_bar.dart';
import 'package:shard/ui/widgets/art.dart';
import 'package:shard/ui/widgets/common.dart';
import 'package:shard/ui/widgets/play_button.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';

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
}
