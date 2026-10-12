// Avatar de cuenta: la cara de la skin (con su capa) o un avatar generado.

import 'dart:ui' as ui;

import 'package:material_color_utilities/material_color_utilities.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../theme/tokens.dart';
import 'art.dart';

class PlayerAvatar extends StatelessWidget {
  const PlayerAvatar({super.key, required this.account, this.size = 40});

  final AccountView? account;
  final double size;

  @override
  Widget build(BuildContext context) {
    final a = account;
    final radius = BorderRadius.circular(size * 0.24);
    Widget child;
    if (a == null) {
      child = Container(
        color: context.colors.surface2,
        alignment: Alignment.center,
        child: Icon(
          Symbols.person_rounded,
          size: size * 0.55,
          color: context.colors.textMuted,
        ),
      );
    } else if (a.skinUrl != null) {
      child = SkinFace(
        url: a.skinUrl!,
        fallback: _GeneratedFace(name: a.name, size: size),
      );
    } else {
      child = _GeneratedFace(name: a.name, size: size);
    }
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        borderRadius: radius,
        border: Border.all(color: Colors.white.withValues(alpha: 0.14)),
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: child,
      ),
    );
  }
}

/// Cara pixelada generada a partir del nombre (para cuentas sin skin).
class _GeneratedFace extends StatelessWidget {
  const _GeneratedFace({required this.name, required this.size});

  final String name;
  final double size;

  @override
  Widget build(BuildContext context) =>
      CustomPaint(painter: _FacePainter(name), size: Size.square(size));
}

class _FacePainter extends CustomPainter {
  _FacePainter(this.name);

  final String name;

  @override
  void paint(Canvas canvas, Size size) {
    final h = stableHash(name.toLowerCase());
    final hue = (h % 360).toDouble();
    Color t(double hueShift, double chroma, double tone) =>
        Color(Hct.from(hue + hueShift, chroma, tone).toInt());
    final cell = size.width / 8;
    final skin = t(40, 24, 72);
    final skinShade = t(40, 24, 64);
    final hair = t(0, 40, 30);
    final eye = t(180, 30, 92);
    final pupil = t(200, 50, 30);
    // Mapa 8×8: 0 piel, 1 sombra, 2 pelo, 3 ojo blanco, 4 pupila, 5 boca.
    final rows = [
      '22222222',
      '22222222',
      '20000002',
      '00000000',
      '03400430',
      '00011000',
      '00155100',
      '10000001',
    ];
    final paints = [skin, skinShade, hair, eye, pupil, t(10, 40, 40)];
    for (var y = 0; y < 8; y++) {
      for (var x = 0; x < 8; x++) {
        final v = int.parse(rows[y][x]);
        canvas.drawRect(
          Rect.fromLTWH(x * cell, y * cell, cell + 0.5, cell + 0.5),
          Paint()..color = paints[v],
        );
      }
    }
  }

  @override
  bool shouldRepaint(_FacePainter old) => old.name != name;
}

/// Recorta la cara (8×8 en 8,8) y la capa (40,8) de una skin remota.
class SkinFace extends StatefulWidget {
  const SkinFace({super.key, required this.url, required this.fallback});

  final String url;
  final Widget fallback;

  @override
  State<SkinFace> createState() => _SkinFaceState();
}

class _SkinFaceState extends State<SkinFace> {
  ui.Image? _image;
  ImageStream? _stream;
  late final ImageStreamListener _listener = ImageStreamListener(
    (info, _) {
      if (mounted) setState(() => _image = info.image);
    },
    onError: (_, _) {},
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _resolve();
  }

  @override
  void didUpdateWidget(SkinFace old) {
    super.didUpdateWidget(old);
    if (old.url != widget.url) _resolve();
  }

  void _resolve() {
    final stream = NetworkImage(widget.url)
        .resolve(createLocalImageConfiguration(context));
    if (stream.key == _stream?.key) return;
    _stream?.removeListener(_listener);
    _stream = stream..addListener(_listener);
  }

  @override
  void dispose() {
    _stream?.removeListener(_listener);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final img = _image;
    if (img == null) return widget.fallback;
    return CustomPaint(painter: _SkinFacePainter(img), child: const SizedBox.expand());
  }
}

class _SkinFacePainter extends CustomPainter {
  _SkinFacePainter(this.image);

  final ui.Image image;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..filterQuality = FilterQuality.none;
    final scale = image.width / 64;
    final dst = Offset.zero & size;
    Rect src(double x, double y) =>
        Rect.fromLTWH(x * scale, y * scale, 8 * scale, 8 * scale);
    canvas.drawImageRect(image, src(8, 8), dst, paint);
    canvas.drawImageRect(image, src(40, 8), dst, paint);
  }

  @override
  bool shouldRepaint(_SkinFacePainter old) => old.image != image;
}
