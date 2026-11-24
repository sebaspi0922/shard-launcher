// Fondo «Grafito»: cuántos fotogramas pide y cuándo deja de pedirlos.

import 'dart:ui' show AppLifecycleState;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shard/ui/widgets/atmosphere.dart';

var _frames = 0;
var _counting = false;

/// Fotogramas dibujados en 1 s. `pump` solo dibuja si alguien pidió un
/// fotograma, así que se cuenta lo que pide el fondo.
Future<int> framesInOneSecond(WidgetTester t) async {
  if (!_counting) {
    t.binding.addPersistentFrameCallback((_) => _frames++);
    _counting = true;
  }
  await t.pump();
  _frames = 0;
  for (var i = 0; i < 60; i++) {
    await t.pump(const Duration(microseconds: 16667));
  }
  return _frames;
}

void lifecycle(WidgetTester t, List<AppLifecycleState> states) {
  for (final s in states) {
    t.binding.handleAppLifecycleStateChanged(s);
  }
}

void main() {
  testWidgets('animado va a 60 fotogramas por segundo', (t) async {
    await t.pumpWidget(const Atmosphere());
    expect(await framesInOneSecond(t), 60);
  });

  testWidgets('quieto (modo rendimiento) no pide fotogramas', (t) async {
    await t.pumpWidget(const Atmosphere(animate: false));
    expect(await framesInOneSecond(t), 0);
  });

  testWidgets('con la ventana oculta se para y vuelve al mostrarla', (t) async {
    await t.pumpWidget(const Atmosphere());
    lifecycle(t, [AppLifecycleState.inactive, AppLifecycleState.hidden]);
    expect(await framesInOneSecond(t), 0);

    lifecycle(t, [AppLifecycleState.inactive, AppLifecycleState.resumed]);
    expect(await framesInOneSecond(t), 60);
  });

  testWidgets('sin foco sigue un rato, luego se para y vuelve con el foco',
      (t) async {
    await t.pumpWidget(const Atmosphere());
    lifecycle(t, [AppLifecycleState.inactive]);
    expect(await framesInOneSecond(t), 60);

    await t.pump(const Duration(seconds: 30));
    expect(await framesInOneSecond(t), 0);

    lifecycle(t, [AppLifecycleState.resumed]);
    expect(await framesInOneSecond(t), 60);
  });
}
