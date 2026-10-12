// Visor de la skin: modelo 3D girable con el ratón y vistas planas
// (frente/espalda) para las miniaturas.


import 'package:material_ui/material_ui.dart';

import '../../core/skins.dart';
import '../theme/tokens.dart';
import 'skin_mesh.dart';

/// Ángulo inicial: tres cuartos, algo desde arriba.
const kDefaultSkinCamera = SkinCamera(yaw: -0.5, pitch: 0.16);

class SkinViewer extends StatefulWidget {
  const SkinViewer({
    super.key,
    required this.skin,
    this.cape,
    required this.slim,
    this.camera = kDefaultSkinCamera,
    this.interactive = true,
    this.shadow = true,
  });

  final SkinTexture skin;
  final SkinTexture? cape;
  final bool slim;
  final SkinCamera camera;
  final bool interactive;
  final bool shadow;

  @override
  State<SkinViewer> createState() => _SkinViewerState();
}

class _SkinViewerState extends State<SkinViewer> {
  late double _yaw = widget.camera.yaw;
  late double _pitch = widget.camera.pitch;

  void _reset() => setState(() {
        _yaw = widget.camera.yaw;
        _pitch = widget.camera.pitch;
      });

  @override
  Widget build(BuildContext context) {
    final paint = CustomPaint(
      painter: _SkinPainter(
        skin: widget.skin,
        cape: widget.cape,
        slim: widget.slim,
        camera: SkinCamera(yaw: _yaw, pitch: _pitch),
        shadow: widget.shadow,
      ),
      child: const SizedBox.expand(),
    );
    if (!widget.interactive) return paint;
    return MouseRegion(
      cursor: SystemMouseCursors.grab,
      child: GestureDetector(
        onDoubleTap: _reset,
        onPanUpdate: (d) => setState(() {
          _yaw += d.delta.dx * 0.012;
          _pitch = (_pitch + d.delta.dy * 0.008).clamp(-0.6, 0.8);
        }),
        child: paint,
      ),
    );
  }
}

class _SkinPainter extends CustomPainter {
  _SkinPainter({
    required this.skin,
    required this.cape,
    required this.slim,
    required this.camera,
    required this.shadow,
  });

  final SkinTexture skin;
  final SkinTexture? cape;
  final bool slim;
  final SkinCamera camera;
  final bool shadow;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;
    if (shadow) {
      // Sombra elíptica bajo los pies.
      final scale = skinScale(size);
      final feet = Offset(size.width / 2, size.height / 2 + 16 * scale);
      final rect = Rect.fromCenter(
        center: feet,
        width: 15 * scale,
        height: 3.6 * scale,
      );
      canvas.drawOval(
        rect,
        Paint()
          ..shader = RadialGradient(
            colors: [
              Colors.black.withValues(alpha: 0.45),
              Colors.black.withValues(alpha: 0),
            ],
          ).createShader(rect),
      );
    }
    final mesh = buildSkinMesh(
      skin: skin,
      cape: cape,
      slim: slim,
      camera: camera,
      size: size,
    );
    final v = mesh.vertices;
    if (v == null) return;
    canvas.drawVertices(
      v,
      BlendMode.dst,
      Paint()
        ..isAntiAlias = false
        ..filterQuality = FilterQuality.none,
    );
    v.dispose();
  }

  @override
  bool shouldRepaint(_SkinPainter old) =>
      old.skin != skin ||
      old.cape != cape ||
      old.slim != slim ||
      old.camera.yaw != camera.yaw ||
      old.camera.pitch != camera.pitch;
}

/// Miniatura del frente de una capa, pixelada.
class CapePreview extends StatelessWidget {
  const CapePreview({super.key, required this.cape});

  final SkinTexture cape;

  @override
  Widget build(BuildContext context) => AspectRatio(
        aspectRatio: 10 / 16,
        child: CustomPaint(painter: _CapePainter(cape)),
      );
}

class _CapePainter extends CustomPainter {
  _CapePainter(this.cape);

  final SkinTexture cape;

  @override
  void paint(Canvas canvas, Size size) {
    final px = size.width / 10;
    final paint = Paint()..isAntiAlias = false;
    for (final (x, y, argb) in capeFront(cape)) {
      if (argb >>> 24 == 0) continue;
      paint.color = Color(argb | 0xFF000000);
      canvas.drawRect(
        Rect.fromLTWH(x * px, y * px, px + 0.5, px + 0.5),
        paint,
      );
    }
  }

  @override
  bool shouldRepaint(_CapePainter old) => old.cape != cape;
}

/// Marco con fondo de «estudio» para el visor.
class SkinStage extends StatelessWidget {
  const SkinStage({super.key, required this.child, this.label});

  final Widget child;
  final String? label;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: c.border),
        gradient: RadialGradient(
          center: const Alignment(0, -0.2),
          radius: 0.9,
          colors: [
            c.accent.withValues(alpha: 0.16),
            c.surface1,
          ],
        ),
      ),
      child: Stack(
        children: [
          Positioned.fill(child: child),
          if (label != null)
            Positioned(
              left: Space.x3,
              top: Space.x2,
              child: Text(
                label!,
                style: context.text.labelSmall!.copyWith(color: c.textMuted),
              ),
            ),
        ],
      ),
    );
  }
}
