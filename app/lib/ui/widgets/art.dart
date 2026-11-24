// Arte generado: paisajes de bloques por instancia, iconos y la marca de la
// app. Todo procedural (sin imágenes de terceros).

import 'dart:math' as math;
import 'dart:ui' as ui;

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
class BlockLandscape extends LeafRenderObjectWidget {
  const BlockLandscape({super.key, required this.seed, this.block = 14});

  final String seed;
  final double block;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderLandscape(seed, block, MediaQuery.devicePixelRatioOf(context));

  @override
  void updateRenderObject(BuildContext context, RenderObject renderObject) =>
      (renderObject as _RenderLandscape)
        ..seed = seed
        ..block = block
        ..dpr = MediaQuery.devicePixelRatioOf(context);
}

/// Pinta el paisaje una vez a una imagen y después solo dibuja la imagen.
/// Impeller no guarda el raster de lo que no cambia: sin esto, los cientos de
/// rectángulos se rasterizaban de nuevo en cada fotograma del fondo animado.
class _RenderLandscape extends RenderBox {
  _RenderLandscape(this._seed, this._block, this._dpr);

  String _seed;
  set seed(String v) {
    if (v == _seed) return;
    _seed = v;
    _drop();
  }

  double _block;
  set block(double v) {
    if (v == _block) return;
    _block = v;
    _drop();
  }

  double _dpr;
  set dpr(double v) {
    if (v == _dpr) return;
    _dpr = v;
    _drop();
  }

  ui.Image? _image;
  Size? _imageSize;

  void _drop() {
    _image?.dispose();
    _image = null;
    markNeedsPaint();
  }

  @override
  bool get sizedByParent => true;

  @override
  Size computeDryLayout(BoxConstraints constraints) => constraints.biggest;

  @override
  void paint(PaintingContext context, Offset offset) {
    if (size.isEmpty) return;
    if (_image == null || _imageSize != size) {
      _image?.dispose();
      final recorder = ui.PictureRecorder();
      _LandscapePainter(_seed, _block).paint(Canvas(recorder)..scale(_dpr), size);
      final picture = recorder.endRecording();
      _image = picture.toImageSync(
        (size.width * _dpr).ceil(),
        (size.height * _dpr).ceil(),
      );
      picture.dispose();
      _imageSize = size;
    }
    context.canvas.drawImageRect(
      _image!,
      Offset.zero & size * _dpr,
      offset & size,
      Paint(),
    );
  }

  @override
  void dispose() {
    _image?.dispose();
    super.dispose();
  }
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

class _LandscapePainter {
  _LandscapePainter(this.seed, this.block);

  final String seed;
  final double block;

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
        painter: BrandMarkPainter(context.colors.accent),
      );
}

/// Pinta la marca: una S hecha con dos fragmentos inclinados y desplazados,
/// en cuatro tonos del acento (también se usa para el icono de la bandeja).
class BrandMarkPainter extends CustomPainter {
  const BrandMarkPainter(this.accent);

  final Color accent;

  // Diseño en una caja de 256 (docs/logo/opciones/4-monograma.svg).
  static const _topLight = [Offset(206, 30), Offset(98, 30), Offset(158, 122)];
  static const _topDark = [Offset(98, 30), Offset(50, 122), Offset(158, 122)];
  static const _bottomLight = [Offset(50, 226), Offset(98, 134), Offset(158, 226)];
  static const _bottomDark = [Offset(158, 226), Offset(206, 134), Offset(98, 134)];

  @override
  void paint(Canvas canvas, Size size) {
    final hct = Hct.fromInt(accent.toARGB32());
    Color t(double tone) => Color(Hct.from(hct.hue, hct.chroma, tone).toInt());
    // Algo más grande que el diseño para aprovechar la caja.
    final k = size.shortestSide / 256 * 1.14;
    canvas
      ..save()
      ..translate(size.width / 2, size.height / 2)
      ..scale(k)
      ..translate(-128, -128);
    void tri(List<Offset> p, Color color) => canvas.drawPath(
          Path()..addPolygon(p, true),
          Paint()
            ..color = color
            ..isAntiAlias = true,
        );
    tri(_topLight, t(88));
    tri(_topDark, t(72));
    tri(_bottomLight, t(56));
    tri(_bottomDark, t(42));
    canvas.restore();
  }

  @override
  bool shouldRepaint(BrandMarkPainter old) => old.accent != accent;
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
