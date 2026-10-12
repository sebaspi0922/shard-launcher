// Arte generado: paisajes de bloques por instancia, iconos y la marca de la
// app. Todo procedural (sin imágenes de terceros).

import 'dart:math' as math;

import 'package:material_color_utilities/material_color_utilities.dart';
import 'package:material_ui/material_ui.dart';

import '../theme/tokens.dart';

/// Hash FNV-1a estable (String.hashCode no lo es entre ejecuciones).
int stableHash(String s) {
  var h = 0x811c9dc5;
  for (final unit in s.codeUnits) {
    h ^= unit;
    h = (h * 0x01000193) & 0xffffffff;
  }
  return h;
}

/// Matiz propio de una instancia (0–360).
double hueFor(String id) => (stableHash(id) % 360).toDouble();

Color _tone(double hue, double chroma, double tone) =>
    Color(Hct.from(hue, chroma, tone).toInt());

/// Paisaje de colinas escalonadas como bloques, con cielo y astro.
class BlockLandscape extends StatelessWidget {
  const BlockLandscape({super.key, required this.seed, this.block = 14});

  final String seed;
  final double block;

  @override
  Widget build(BuildContext context) => CustomPaint(
        painter: _LandscapePainter(seed, block),
        child: const SizedBox.expand(),
      );
}

/// Bioma del paisaje: matiz, croma y tono de la tierra y de su capa superior.
typedef _Biome = ({double hue, double chroma, double tone, double topHue, double topChroma, double topTone});

const _biomes = <_Biome>[
  // pradera
  (hue: 135, chroma: 34, tone: 34, topHue: 128, topChroma: 52, topTone: 52),
  // desierto
  (hue: 80, chroma: 30, tone: 60, topHue: 85, topChroma: 34, topTone: 74),
  // nieve
  (hue: 230, chroma: 10, tone: 70, topHue: 230, topChroma: 6, topTone: 94),
  // mesa
  (hue: 40, chroma: 46, tone: 42, topHue: 55, topChroma: 40, topTone: 58),
  // cerezos
  (hue: 140, chroma: 26, tone: 32, topHue: 350, topChroma: 40, topTone: 78),
  // bosque oscuro
  (hue: 150, chroma: 28, tone: 22, topHue: 145, topChroma: 40, topTone: 36),
];

class _LandscapePainter extends CustomPainter {
  _LandscapePainter(this.seed, this.block);

  final String seed;
  final double block;

  @override
  void paint(Canvas canvas, Size size) {
    final h = stableHash(seed);
    final skyHue = (h % 360).toDouble();
    final biome = _biomes[(h >> 9) % _biomes.length];
    final rnd = math.Random(h);
    final rect = Offset.zero & size;

    // Cielo: degradado vertical del matiz propio, tono de atardecer.
    final skyTop = _tone(skyHue, 36, 18);
    final skyBottom = _tone(skyHue + 25, 44, 46);
    canvas.drawRect(
      rect,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [skyTop, skyBottom],
        ).createShader(rect),
    );

    // Astro cuadrado con halo.
    final sun = Offset(
      size.width * (0.6 + rnd.nextDouble() * 0.28),
      size.height * (0.2 + rnd.nextDouble() * 0.12),
    );
    final r = math.max(5.0, math.min(18.0, size.shortestSide * 0.045));
    canvas.drawCircle(
      sun,
      r * 5,
      Paint()
        ..shader = RadialGradient(colors: [
          _tone(skyHue + 40, 40, 85).withValues(alpha: 0.32),
          _tone(skyHue + 40, 40, 85).withValues(alpha: 0),
        ]).createShader(Rect.fromCircle(center: sun, radius: r * 5)),
    );
    canvas.drawRect(
      Rect.fromCenter(center: sun, width: r * 2, height: r * 2),
      Paint()..color = _tone(skyHue + 40, 24, 90).withValues(alpha: 0.9),
    );
    canvas.drawRect(
      Rect.fromCenter(center: sun, width: r * 1.2, height: r * 1.2),
      Paint()..color = _tone(skyHue + 40, 10, 98),
    );

    // Tres capas de colinas: las lejanas se funden con la niebla del cielo.
    final layers = [
      (base: 0.56, amp: 0.14, fog: 0.5),
      (base: 0.70, amp: 0.12, fog: 0.22),
      (base: 0.84, amp: 0.08, fog: 0.0),
    ];
    final cols = (size.width / block).ceil() + 1;
    for (final (i, layer) in layers.indexed) {
      final shade = 1 - (2 - i) * 0.12;
      Color fogged(Color c) => Color.lerp(c, skyBottom, layer.fog)!;
      final body = Paint()
        ..color = fogged(
          _tone(biome.hue, biome.chroma, biome.tone * shade),
        );
      final top = Paint()
        ..color = fogged(
          _tone(biome.topHue, biome.topChroma, biome.topTone * shade),
        );
      var y = layer.base + (rnd.nextDouble() - 0.5) * layer.amp;
      var run = 0;
      for (var c = 0; c < cols; c++) {
        // Mesetas de varios bloques con escalones de uno o dos.
        if (run <= 0) {
          y += (rnd.nextInt(5) - 2) * (block / size.height);
          y = y.clamp(layer.base - layer.amp, layer.base + layer.amp);
          run = 1 + rnd.nextInt(3);
        }
        run--;
        final py = (y * size.height / block).round() * block;
        final x = c * block;
        canvas.drawRect(Rect.fromLTRB(x, py, x + block + 0.5, size.height), body);
        canvas.drawRect(Rect.fromLTWH(x, py, block + 0.5, block * 0.34), top);
      }
    }

    // Viñeta inferior para que el texto encima se lea.
    canvas.drawRect(
      rect,
      Paint()
        ..shader = LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Colors.transparent, Colors.black.withValues(alpha: 0.45)],
          stops: const [0.45, 1],
        ).createShader(rect),
    );
  }

  @override
  bool shouldRepaint(_LandscapePainter old) =>
      old.seed != seed || old.block != block;
}

/// Icono cuadrado de instancia: degradado propio con sus iniciales.
class InstanceIcon extends StatelessWidget {
  const InstanceIcon({
    super.key,
    required this.id,
    required this.name,
    this.size = 44,
  });

  final String id;
  final String name;
  final double size;

  String get _initials {
    final words = name.trim().split(RegExp(r'\s+')).where((w) => w.isNotEmpty);
    final letters = words.take(2).map((w) => w.characters.first.toUpperCase());
    final s = letters.join();
    return s.isEmpty ? '?' : s;
  }

  @override
  Widget build(BuildContext context) {
    final hue = hueFor(id);
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(size * 0.26),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [_tone(hue + 25, 45, 62), _tone(hue - 15, 45, 34)],
        ),
        border: Border.all(color: Colors.white.withValues(alpha: 0.16)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.35),
            blurRadius: size * 0.3,
            offset: Offset(0, size * 0.08),
          ),
        ],
      ),
      child: Text(
        _initials,
        style: TextStyle(
          fontSize: size * 0.36,
          fontWeight: FontWeight.w700,
          color: Colors.white.withValues(alpha: 0.95),
          letterSpacing: -0.5,
          height: 1,
        ),
      ),
    );
  }
}

/// Marca de la app: un cubo isométrico con las caras en tonos del acento.
class BrandMark extends StatelessWidget {
  const BrandMark({super.key, this.size = 32});

  final double size;

  @override
  Widget build(BuildContext context) => CustomPaint(
        size: Size.square(size),
        painter: _CubePainter(context.colors.accent),
      );
}

class _CubePainter extends CustomPainter {
  _CubePainter(this.accent);

  final Color accent;

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;
    final hct = Hct.fromInt(accent.toARGB32());
    Color t(double tone) => Color(Hct.from(hct.hue, hct.chroma, tone).toInt());
    final top = Path()
      ..moveTo(w * 0.5, h * 0.06)
      ..lineTo(w * 0.92, h * 0.29)
      ..lineTo(w * 0.5, h * 0.52)
      ..lineTo(w * 0.08, h * 0.29)
      ..close();
    final left = Path()
      ..moveTo(w * 0.08, h * 0.29)
      ..lineTo(w * 0.5, h * 0.52)
      ..lineTo(w * 0.5, h * 0.96)
      ..lineTo(w * 0.08, h * 0.73)
      ..close();
    final right = Path()
      ..moveTo(w * 0.92, h * 0.29)
      ..lineTo(w * 0.92, h * 0.73)
      ..lineTo(w * 0.5, h * 0.96)
      ..lineTo(w * 0.5, h * 0.52)
      ..close();
    canvas.drawPath(top, Paint()..color = t(86));
    canvas.drawPath(left, Paint()..color = t(66));
    canvas.drawPath(right, Paint()..color = t(46));
    // Hueco en la cara superior: un bloque «encajado».
    final inner = Path()
      ..moveTo(w * 0.5, h * 0.19)
      ..lineTo(w * 0.69, h * 0.29)
      ..lineTo(w * 0.5, h * 0.39)
      ..lineTo(w * 0.31, h * 0.29)
      ..close();
    canvas.drawPath(inner, Paint()..color = t(56));
  }

  @override
  bool shouldRepaint(_CubePainter old) => old.accent != accent;
}

/// Fondo de la app: base casi negra con halos suaves del acento.
class Backdrop extends StatelessWidget {
  const Backdrop({super.key});

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return DecoratedBox(
      decoration: BoxDecoration(color: c.backgroundDeep),
      child: Stack(
        fit: StackFit.expand,
        children: [
          DecoratedBox(
            decoration: BoxDecoration(
              gradient: RadialGradient(
                center: const Alignment(-0.85, -1.1),
                radius: 1.25,
                colors: [c.glow.withValues(alpha: 0.20), Colors.transparent],
              ),
            ),
          ),
          DecoratedBox(
            decoration: BoxDecoration(
              gradient: RadialGradient(
                center: const Alignment(1.15, 1.2),
                radius: 1.1,
                colors: [
                  HSLColor.fromColor(c.glow)
                      .withHue((HSLColor.fromColor(c.glow).hue + 50) % 360)
                      .toColor()
                      .withValues(alpha: 0.10),
                  Colors.transparent,
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
