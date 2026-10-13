// Rueda del ratón animada (AppScrollBehavior).

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shard/ui/widgets/smooth_scroll.dart';

Widget _host(Widget child, {bool disableAnimations = false}) => MaterialApp(
  scrollBehavior: const AppScrollBehavior(),
  home: MediaQuery(
    data: MediaQueryData(
      size: const Size(400, 400),
      disableAnimations: disableAnimations,
    ),
    child: child,
  ),
);

Widget _list(ScrollController c, {double height = 400}) => SizedBox(
  height: height,
  child: ListView(
    controller: c,
    children: [for (var i = 0; i < 100; i++) const SizedBox(height: 50)],
  ),
);

Future<void> _wheel(WidgetTester t, Offset at, double dy) async {
  final p = TestPointer(1, PointerDeviceKind.mouse);
  await t.sendEventToBinding(p.hover(at));
  await t.sendEventToBinding(p.scroll(Offset(0, dy)));
}

void main() {
  testWidgets('cada paso se desliza y los pasos seguidos se suman', (t) async {
    final c = ScrollController();
    await t.pumpWidget(_host(_list(c)));
    await _wheel(t, const Offset(200, 200), 100);
    await t.pump();
    await t.pump(const Duration(milliseconds: 60));
    expect(c.offset, greaterThan(0));
    expect(c.offset, lessThan(100));
    await _wheel(t, const Offset(200, 200), 100);
    await t.pumpAndSettle();
    expect(c.offset, 200);
  });

  testWidgets('la rueda de giro libre no congela la lista', (t) async {
    // G502 en modo libre: un paso cada ~5 ms, más rápido que los fotogramas.
    final c = ScrollController();
    await t.pumpWidget(_host(_list(c)));
    final seen = <double>[];
    for (var i = 0; i < 30; i++) {
      await _wheel(t, const Offset(200, 200), 79.5);
      await t.pump(const Duration(milliseconds: 5));
      seen.add(c.offset);
    }
    // Avanza durante el giro, no solo al final.
    expect(seen[10], greaterThan(0));
    expect(seen[29], greaterThan(seen[20]));
    await t.pumpAndSettle();
    expect(c.offset, closeTo(30 * 79.5, 0.01));
  });

  testWidgets('en el borde la rueda pasa a la lista padre', (t) async {
    final outer = ScrollController();
    final inner = ScrollController();
    await t.pumpWidget(
      _host(
        ListView(
          controller: outer,
          children: [_list(inner, height: 200), const SizedBox(height: 2000)],
        ),
      ),
    );
    // Hacia arriba: la interior ya está arriba del todo, así que se mueve la
    // exterior... que también lo está; nada se mueve.
    await _wheel(t, const Offset(200, 100), -50);
    await t.pumpAndSettle();
    expect(inner.offset, 0);
    expect(outer.offset, 0);
    // Hacia abajo se mueve solo la interior.
    await _wheel(t, const Offset(200, 100), 100);
    await t.pumpAndSettle();
    expect(inner.offset, 100);
    expect(outer.offset, 0);
    // Con la interior al final, la rueda mueve la exterior.
    inner.jumpTo(inner.position.maxScrollExtent);
    await t.pump();
    await _wheel(t, const Offset(200, 100), 100);
    await t.pumpAndSettle();
    expect(outer.offset, 100);
  });

  testWidgets('con las animaciones desactivadas salta al instante', (t) async {
    final c = ScrollController();
    await t.pumpWidget(_host(_list(c), disableAnimations: true));
    await _wheel(t, const Offset(200, 200), 100);
    await t.pump();
    expect(c.offset, 100);
  });
}
