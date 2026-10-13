// Rueda del ratón con desplazamiento animado en todas las listas de la app.
//
// Flutter mueve la lista de golpe con cada paso de la rueda. Aquí cada paso
// solo mueve un destino, y la lista lo persigue fotograma a fotograma
// recorriendo una parte fija de lo que le falta: frena sola al llegar. Como
// los pasos no reinician ninguna animación, una rueda de giro libre (varios
// pasos por fotograma) se suma sin trabas y al soltarla la lista se desliza
// hasta pararse. El touchpad (eventos de pan/zoom) ya es continuo y no se
// toca. Con «reducir movimiento» o el modo rendimiento se vuelve al salto.

import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../theme/tokens.dart';

class AppScrollBehavior extends MaterialScrollBehavior {
  const AppScrollBehavior();

  // Se engancha en los dos puntos porque muchas listas internas (menús,
  // campos de texto, pestañas) desactivan con `copyWith` la barra o el
  // indicador de sobredesplazamiento, y quedarían fuera.
  @override
  Widget buildOverscrollIndicator(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) => _wrap(
    context,
    super.buildOverscrollIndicator(context, child, details),
    details,
  );

  @override
  Widget buildScrollbar(
    BuildContext context,
    Widget child,
    ScrollableDetails details,
  ) {
    final bar = super.buildScrollbar(context, child, details);
    return child is _SmoothWheel ? bar : _wrap(context, bar, details);
  }

  Widget _wrap(BuildContext context, Widget child, ScrollableDetails details) {
    final controller = details.controller;
    if (controller == null) return child;
    return _SmoothWheel(
      scrollable: context,
      controller: controller,
      direction: details.direction,
      flipModifiers: pointerAxisModifiers,
      animate: !(MediaQuery.maybeDisableAnimationsOf(context) ?? false),
      child: child,
    );
  }
}

/// Desplazamiento en curso de cada lista.
final _active = Expando<_WheelScrollActivity>();

class _SmoothWheel extends SingleChildRenderObjectWidget {
  const _SmoothWheel({
    required this.scrollable,
    required this.controller,
    required this.direction,
    required this.flipModifiers,
    required this.animate,
    required super.child,
  });

  /// Contexto del `Scrollable` envuelto, para elegir su posición en el
  /// controlador (un controlador puede servir a varias listas).
  final BuildContext scrollable;
  final ScrollController controller;
  final AxisDirection direction;
  final Set<LogicalKeyboardKey> flipModifiers;
  final bool animate;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderWheelFirst(_plan);

  @override
  void updateRenderObject(BuildContext context, _RenderWheelFirst render) {
    render.plan = _plan;
  }

  /// Qué hacer con este paso de la rueda, o `null` si esta lista no se movería
  /// (en el borde, sin contenido, física que no acepta desplazamiento).
  VoidCallback? _plan(PointerScrollEvent event) {
    final position = controller.positions
        .where((p) => p.context.storageContext == scrollable)
        .firstOrNull;
    if (position == null ||
        !position.hasContentDimensions ||
        !position.physics.shouldAcceptUserOffset(position)) {
      return null;
    }

    // Mismo cálculo que el Scrollable: Mayús cambia de eje con la rueda.
    final flip =
        event.kind == PointerDeviceKind.mouse &&
        HardwareKeyboard.instance.logicalKeysPressed.any(
          flipModifiers.contains,
        );
    final axis = flip
        ? flipAxis(axisDirectionToAxis(direction))
        : axisDirectionToAxis(direction);
    var delta = axis == Axis.vertical
        ? event.scrollDelta.dy
        : event.scrollDelta.dx;
    if (axisDirectionIsReversed(direction)) delta = -delta;
    if (delta == 0) return null;

    double clamp(double v) =>
        v.clamp(position.minScrollExtent, position.maxScrollExtent);

    // Salto de siempre: sin animaciones, touchpad que llega como evento de
    // rueda, o posiciones coordinadas (NestedScrollView) con lógica propia.
    if (!animate ||
        event.kind != PointerDeviceKind.mouse ||
        position is! ScrollPositionWithSingleContext) {
      if (clamp(position.pixels + delta) == position.pixels) return null;
      return () => position.pointerScroll(delta);
    }

    // Si otra cosa interrumpió el desplazamiento (barra, teclado, arrastre),
    // la actividad ya no está viva y el paso parte de donde esté la lista.
    final running = _active[position];
    final pending = running != null && running.alive ? running : null;
    final target = clamp((pending?.target ?? position.pixels) + delta);
    if (pending == null && target == position.pixels) return null;
    return () {
      // Si ya va hacia el borde, el paso se consume sin pasar a la lista padre.
      if (pending != null) {
        pending.target = target;
      } else {
        final activity = _WheelScrollActivity(position, target);
        _active[position] = activity;
        position.beginActivity(activity);
      }
    };
  }
}

class _WheelEntry extends BoxHitTestEntry {
  _WheelEntry(super.target, super.localPosition);

  /// Ninguna otra lista animada debajo en este hit test.
  bool innermost = true;
}

/// Entra en el hit test antes que sus hijos, así que todas estas capas reciben
/// la rueda antes que los `Scrollable` que envuelven. La capa más interna
/// decide: elige la primera lista, de dentro afuera, que se movería, y se
/// queda con el evento antes de que lo pida el propio `Scrollable`.
class _RenderWheelFirst extends RenderProxyBox {
  _RenderWheelFirst(this.plan);

  VoidCallback? Function(PointerScrollEvent) plan;

  static int _added = 0;
  static PointerEvent? _event;
  static final _chain = <_RenderWheelFirst>[];

  @override
  bool hitTest(BoxHitTestResult result, {required Offset position}) {
    if (!size.contains(position)) return false;
    final entry = _WheelEntry(this, position);
    result.add(entry);
    final before = ++_added;
    final hit = hitTestChildren(result, position: position);
    entry.innermost = _added == before;
    return hit;
  }

  @override
  void handleEvent(PointerEvent event, BoxHitTestEntry entry) {
    if (event is! PointerScrollEvent) return;
    // Cada capa recibe una copia transformada; la original identifica el paso.
    final original = event.original ?? event;
    if (!identical(original, _event)) {
      _event = original;
      _chain.clear();
    }
    _chain.add(this);
    if (entry is! _WheelEntry || !entry.innermost) return;

    for (final layer in _chain.reversed) {
      final action = layer.plan(event);
      if (action == null) continue;
      GestureBinding.instance.pointerSignalResolver.register(event, (e) {
        action();
        (e as PointerScrollEvent).respond(allowPlatformDefault: false);
      });
      break;
    }
    _chain.clear();
    _event = null;
  }
}

/// Persigue [target] con un acercamiento exponencial: en cada fotograma recorre
/// la misma fracción de lo que falta (según el tiempo transcurrido, así que no
/// depende de la tasa de refresco). Cambiar el destino no reinicia nada.
class _WheelScrollActivity extends ScrollActivity {
  _WheelScrollActivity(ScrollPositionWithSingleContext position, this.target)
    : _position = position,
      super(position) {
    _ticker = position.context.vsync.createTicker(_tick)..start();
  }

  final ScrollPositionWithSingleContext _position;
  late final Ticker _ticker;
  Duration _last = Duration.zero;
  double _velocity = 0;
  bool alive = true;
  double target;

  void _tick(Duration elapsed) {
    final dt = (elapsed - _last).inMicroseconds / 1e6;
    _last = elapsed;
    if (dt <= 0) return;
    final p = _position;
    // El contenido puede encoger mientras tanto (listas que se filtran).
    target = target.clamp(p.minScrollExtent, p.maxScrollExtent);
    final remaining = target - p.pixels;
    if (remaining.abs() < 0.5) {
      delegate.setPixels(target);
      delegate.goBallistic(0);
      return;
    }
    final step =
        remaining * (1 - math.exp(-dt / (Motion.wheel.inMicroseconds / 1e6)));
    _velocity = step / dt;
    // Algo nos frenó (un borde que no se ve en las medidas): se para.
    if (delegate.setPixels(p.pixels + step) != 0) delegate.goBallistic(0);
  }

  @override
  bool get shouldIgnorePointer => false;

  @override
  bool get isScrolling => true;

  @override
  double get velocity => _velocity;

  @override
  void dispose() {
    alive = false;
    _ticker.dispose();
    super.dispose();
  }
}
