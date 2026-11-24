// Fondo «Grafito»: círculos de contorno fino y puntos pastel que derivan sin
// prisa, cada uno con su propio factor (parallax). Va y vuelve en 18 s de forma
// lineal y se mueve desde el primer fotograma.

import 'dart:async';

import 'package:material_ui/material_ui.dart';

/// Una forma: posición base (fracción del tamaño + píxeles) más la deriva.
class _Shape {
  const _Shape({
    required this.x,
    required this.y,
    required this.size,
    required this.color,
    this.fx = 0,
    this.fy = 0,
    this.sx = 0,
    this.lx = 0,
    this.sy = 0,
    this.ly = 0,
    this.radius,
    this.outline = false,
  });

  /// Píxeles desde el borde izquierdo/superior.
  final double x;
  final double y;

  /// Fracción del ancho/alto que se suma a [x]/[y].
  final double fx;
  final double fy;

  /// Cuánto le afectan el vaivén y la subida en cada eje.
  final double sx;
  final double lx;
  final double sy;
  final double ly;
  final double size;
  final Color color;

  /// `null` = círculo.
  final double? radius;
  final bool outline;
}

const _shapes = <_Shape>[
  _Shape(x: -70, y: -40, sx: 0.4, ly: 1, size: 340, color: Color(0x66AEBCFF), outline: true),
  _Shape(x: 70, y: 0, fy: 0.38, sx: -0.3, ly: 1, size: 220, color: Color(0x55C4B6FF), outline: true),
  _Shape(x: -210, fx: 1, y: -250, fy: 1, sx: 1, ly: -1, size: 260, color: Color(0x508FD4FF), outline: true),
  _Shape(x: 0, fx: 0.55, y: 18, lx: -0.4, sy: 0.2, size: 92, color: Color(0x38FFFFFF), outline: true),
  _Shape(x: 36, y: -120, fy: 1, lx: 0.2, sy: 0.3, size: 14, radius: 3, color: Color(0xFF9EB0FF)),
  _Shape(x: 0, fx: 0.22, y: 64, sx: 1, ly: -0.3, size: 8, color: Color(0xFFD2C6FF)),
  _Shape(x: -120, fx: 1, y: 0, fy: 0.22, sx: -0.5, ly: 1, size: 10, color: Color(0xFF8FD4FF)),
  _Shape(x: 0, fx: 0.4, y: -70, fy: 1, lx: 0.5, sy: -0.2, size: 6, radius: 1, color: Color(0x88FFFFFF)),
  _Shape(x: 180, y: 0, fy: 0.62, sx: -0.6, size: 18, color: Color(0xFFAEBCFF), outline: true),
  _Shape(x: 0, fx: 0.72, y: 120, sx: 0.5, ly: 0.4, size: 150, color: Color(0x30FFFFFF), outline: true),
  _Shape(x: 0, fx: 0.12, y: 0, fy: 0.18, lx: -0.3, sy: 0.4, size: 7, color: Color(0xFFC4B6FF)),
  _Shape(x: -64, fx: 1, y: 0, fy: 0.72, sx: 0.2, ly: -0.3, size: 11, radius: 2, color: Color(0xAA8FD4FF)),
];

/// Duración de un vaivén completo (ida y vuelta).
const _period = Duration(seconds: 18);

class Atmosphere extends StatefulWidget {
  const Atmosphere({
    super.key,
    this.background = const Color(0xFF212121),
    this.animate = true,
  });

  final Color background;

  /// `false` = formas quietas (modo rendimiento).
  final bool animate;

  @override
  State<Atmosphere> createState() => _AtmosphereState();
}

class _AtmosphereState extends State<Atmosphere>
    with SingleTickerProviderStateMixin, WidgetsBindingObserver {
  /// Sin foco más de esto, el fondo se para. Oculta o minimizada no hace
  /// falta: Flutter ya no pide fotogramas.
  static const _idleAfter = Duration(seconds: 30);

  late final _clock = AnimationController(vsync: this, duration: _period);
  Timer? _idle;
  bool _away = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _sync();
  }

  @override
  void didUpdateWidget(Atmosphere old) {
    super.didUpdateWidget(old);
    _sync();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _idle?.cancel();
    if (state == AppLifecycleState.inactive) {
      _idle = Timer(_idleAfter, () {
        _away = true;
        _sync();
      });
    } else if (state == AppLifecycleState.resumed && _away) {
      _away = false;
      _sync();
    }
  }

  /// Con «reducir movimiento», en modo rendimiento o tras un rato sin foco las
  /// formas se quedan quietas.
  void _sync() {
    final run =
        widget.animate && !_away && !MediaQuery.disableAnimationsOf(context);
    if (!run) {
      _clock.stop();
    } else if (!_clock.isAnimating) {
      _clock.repeat();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _idle?.cancel();
    _clock.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => RepaintBoundary(
        child: CustomPaint(
          painter: _AtmospherePainter(_clock, widget.background),
          child: const SizedBox.expand(),
        ),
      );
}

class _AtmospherePainter extends CustomPainter {
  _AtmospherePainter(this.clock, this.background) : super(repaint: clock);

  final Animation<double> clock;
  final Color background;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = background);
    // Ping-pong lineal: 0 → 1 en 9 s y de vuelta.
    final phase = clock.value * 2;
    final swing = phase < 1 ? phase : 2 - phase;
    final sway = -28 + 62 * swing;
    final lift = 30 - 72 * swing;
    final stroke = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..isAntiAlias = true;
    final fill = Paint()..isAntiAlias = true;
    for (final s in _shapes) {
      final left = s.x + s.fx * size.width + s.sx * sway + s.lx * lift;
      final top = s.y + s.fy * size.height + s.sy * sway + s.ly * lift;
      final rect = Rect.fromLTWH(left, top, s.size, s.size);
      if (s.outline) {
        // El trazo va por dentro del borde, como en el original.
        canvas.drawOval(rect.deflate(0.5), stroke..color = s.color);
      } else if (s.radius == null) {
        canvas.drawOval(rect, fill..color = s.color);
      } else {
        canvas.drawRRect(
          RRect.fromRectAndRadius(rect, Radius.circular(s.radius!)),
          fill..color = s.color,
        );
      }
    }
  }

  @override
  bool shouldRepaint(_AtmospherePainter old) => old.background != background;
}
