// Fondo «sebaspi»: esfera de caracteres dorados sobre negro que gira despacio,
// con un halo tenue y grano fino.

import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:material_ui/material_ui.dart';

/// Rampa de densidad, de vacío a lleno.
const _ramp = ' .:-=+*#%@';

/// Segundos por vuelta.
const _turn = 90.0;

/// Fotogramas por segundo del giro (no necesita más).
const _fps = 15;

class AsciiSphere extends StatefulWidget {
  const AsciiSphere({
    super.key,
    this.color = const Color(0xFFF1C40F),
    this.animate = true,
  });

  final Color color;
  final bool animate;

  @override
  State<AsciiSphere> createState() => _AsciiSphereState();
}

class _AsciiSphereState extends State<AsciiSphere>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker = createTicker(_tick);
  final _time = ValueNotifier<double>(0);
  Duration _last = Duration.zero;
  _Glyphs? _glyphs;
  ui.Image? _grain;
  double? _dpr;

  void _tick(Duration elapsed) {
    if (elapsed - _last < const Duration(microseconds: 1000000 ~/ _fps)) return;
    _last = elapsed;
    _time.value = elapsed.inMicroseconds / 1e6;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final dpr = MediaQuery.devicePixelRatioOf(context);
    if (dpr != _dpr) {
      _dpr = dpr;
      _Glyphs.build(dpr).then((g) {
        if (!mounted) return g.image.dispose();
        setState(() {
          _glyphs?.image.dispose();
          _glyphs = g;
        });
      });
    }
    if (_grain == null) {
      _makeGrain().then((img) {
        if (!mounted) return img.dispose();
        setState(() => _grain = img);
      });
    }
    _syncTicker();
  }

  @override
  void didUpdateWidget(AsciiSphere old) {
    super.didUpdateWidget(old);
    _syncTicker();
  }

  void _syncTicker() {
    final run = widget.animate && !MediaQuery.disableAnimationsOf(context);
    if (run && !_ticker.isActive) {
      _last = Duration.zero;
      _ticker.start();
    } else if (!run && _ticker.isActive) {
      // El giro seguirá donde se quedó: el reloj del ticker vuelve a 0.
      _ticker.stop();
      _base += _time.value;
      _time.value = 0;
    }
  }

  /// Segundos acumulados de giros anteriores.
  double _base = 0;

  @override
  void dispose() {
    _ticker.dispose();
    _time.dispose();
    _glyphs?.image.dispose();
    _grain?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => RepaintBoundary(
        child: CustomPaint(
          painter: _SpherePainter(
            time: _time,
            base: () => _base,
            glyphs: _glyphs,
            grain: _grain,
            color: widget.color,
          ),
          child: const SizedBox.expand(),
        ),
      );
}

/// Atlas con un glifo blanco por carácter de la rampa.
class _Glyphs {
  _Glyphs(this.image, this.cell, this.dpr);

  final ui.Image image;

  /// Tamaño lógico de cada celda.
  final Size cell;
  final double dpr;

  static Future<_Glyphs> build(double dpr) async {
    final painters = [
      for (final ch in _ramp.split(''))
        TextPainter(
          text: TextSpan(
            text: ch,
            style: const TextStyle(
              fontFamily: 'JetBrainsMono',
              fontSize: 12,
              height: 1.15,
              color: Color(0xFFFFFFFF),
            ),
          ),
          textDirection: TextDirection.ltr,
        )..layout(),
    ];
    final cell = Size(
      painters.map((p) => p.width).reduce(math.max),
      painters.map((p) => p.height).reduce(math.max),
    );
    final rec = ui.PictureRecorder();
    final canvas = Canvas(rec)..scale(dpr);
    for (var i = 0; i < painters.length; i++) {
      painters[i].paint(canvas, Offset(i * cell.width, 0));
      painters[i].dispose();
    }
    final image = await rec.endRecording().toImage(
          (cell.width * _ramp.length * dpr).ceil(),
          (cell.height * dpr).ceil(),
        );
    return _Glyphs(image, cell, dpr);
  }
}

/// Ruido monocromo para el grano (se repite en mosaico).
Future<ui.Image> _makeGrain() {
  const n = 128;
  final rnd = math.Random(7);
  final px = Uint8List(n * n * 4);
  for (var i = 0; i < n * n; i++) {
    final v = rnd.nextInt(256);
    px[i * 4] = v;
    px[i * 4 + 1] = v;
    px[i * 4 + 2] = v;
    px[i * 4 + 3] = 255;
  }
  final done = Completer<ui.Image>();
  ui.decodeImageFromPixels(px, n, n, ui.PixelFormat.rgba8888, done.complete);
  return done.future;
}

class _SpherePainter extends CustomPainter {
  _SpherePainter({
    required this.time,
    required this.base,
    required this.glyphs,
    required this.grain,
    required this.color,
  }) : super(repaint: time);

  final ValueListenable<double> time;
  final double Function() base;
  final _Glyphs? glyphs;
  final ui.Image? grain;
  final Color color;

  // Luz desde arriba a la izquierda, hacia el espectador.
  static const _lx = -0.55, _ly = -0.5, _lz = 0.67;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = const Color(0xFF000000));
    final r = math.min(size.height * 0.40, size.width * 0.30);
    final center = Offset(size.width * 0.70, size.height * 0.53);

    // Halo dorado muy suave detrás de la esfera.
    canvas.drawCircle(
      center,
      r * 1.6,
      Paint()
        ..shader = ui.Gradient.radial(center, r * 1.6, [
          color.withValues(alpha: 0.10),
          color.withValues(alpha: 0),
        ]),
    );

    final g = glyphs;
    if (g != null) _paintSphere(canvas, center, r, g);

    if (grain != null) {
      canvas.drawRect(
        Offset.zero & size,
        Paint()
          ..shader = ImageShader(
            grain!,
            TileMode.repeated,
            TileMode.repeated,
            Matrix4.identity().storage,
          )
          ..color = const Color(0x0AFFFFFF)
          ..blendMode = BlendMode.overlay,
      );
    }
  }

  void _paintSphere(Canvas canvas, Offset center, double r, _Glyphs g) {
    final cw = g.cell.width, ch = g.cell.height;
    final theta = (base() + time.value) * 2 * math.pi / _turn;
    final cols = (2 * r / cw).floor();
    final rows = (2 * r / ch).floor();
    final left = center.dx - cols * cw / 2;
    final top = center.dy - rows * ch / 2;
    final transforms = <RSTransform>[];
    final rects = <Rect>[];
    final colors = <Color>[];
    final inv = 1 / g.dpr;
    for (var row = 0; row < rows; row++) {
      final y = top + (row + 0.5) * ch;
      final ny = (y - center.dy) / r;
      for (var col = 0; col < cols; col++) {
        final x = left + (col + 0.5) * cw;
        final nx = (x - center.dx) / r;
        final d = nx * nx + ny * ny;
        if (d >= 1) continue;
        final nz = math.sqrt(1 - d);
        final lon = math.atan2(nx, nz) + theta;
        final lat = math.asin(ny);
        final diffuse = math.max(0.0, nx * _lx + ny * _ly + nz * _lz);
        // Manchas que giran con la esfera y franjas de latitud.
        final tex = 0.5 +
            0.35 * math.sin(lon * 3) * math.cos(lat * 2) +
            0.15 * math.sin(lon * 7 + lat * 5);
        final band = math.sin(lat * 16).abs() < 0.14 ? 0.55 : 1.0;
        final v = ((0.12 + 0.88 * diffuse) * (0.4 + 0.6 * tex) * band)
            .clamp(0.0, 1.0);
        final idx = (v * (_ramp.length - 0.01)).floor();
        if (idx == 0) continue;
        transforms.add(RSTransform(inv, 0, x - cw / 2, y - ch / 2));
        rects.add(Rect.fromLTWH(
          idx * cw * g.dpr,
          0,
          cw * g.dpr,
          ch * g.dpr,
        ));
        colors.add(color.withValues(alpha: 0.16 + 0.42 * v));
      }
    }
    canvas.drawAtlas(
      g.image,
      transforms,
      rects,
      colors,
      BlendMode.modulate,
      null,
      Paint()..filterQuality = FilterQuality.medium,
    );
  }

  @override
  bool shouldRepaint(_SpherePainter old) =>
      old.glyphs != glyphs || old.grain != grain || old.color != color;
}
