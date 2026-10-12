// Fondo de la app según Ajustes › Apariencia: halos del acento, paisaje
// generado o una imagen propia, con desenfoque y oscurecido.

import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/ui_prefs.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';

/// Semilla del paisaje de fondo (elegida por cómo queda).
const kBackdropLandscapeSeed = 'shard-ocaso';

class AppBackdrop extends ConsumerWidget {
  const AppBackdrop({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final p = ref.watch(uiPrefsProvider);
    final c = context.colors;
    final Widget art;
    switch (p.background) {
      case BackgroundKind.aurora:
        return const RepaintBoundary(child: Backdrop());
      case BackgroundKind.landscape:
        art = const BlockLandscape(seed: kBackdropLandscapeSeed, block: 26);
      case BackgroundKind.image:
        final path = p.backgroundPath;
        if (path == null || !File(path).existsSync()) {
          return const RepaintBoundary(child: Backdrop());
        }
        art = Image.file(
          File(path),
          fit: BoxFit.cover,
          gaplessPlayback: true,
          filterQuality: FilterQuality.medium,
          errorBuilder: (_, _, _) => const Backdrop(),
        );
    }
    final blur = p.performance ? 0.0 : p.blur;
    return RepaintBoundary(
      child: Stack(
        fit: StackFit.expand,
        children: [
          ColoredBox(color: c.backgroundDeep),
          if (blur > 0.5)
            ImageFiltered(
              imageFilter: ui.ImageFilter.blur(
                sigmaX: blur,
                sigmaY: blur,
                tileMode: TileMode.clamp,
              ),
              child: art,
            )
          else
            art,
          // Oscurecido con el tinte del fondo base y algo más a la izquierda,
          // donde está el raíl.
          DecoratedBox(
            decoration: BoxDecoration(
              gradient: LinearGradient(
                colors: [
                  c.backgroundDeep.withValues(alpha: (p.dim + 0.15).clamp(0, 0.95)),
                  c.backgroundDeep.withValues(alpha: p.dim),
                ],
                stops: const [0, 0.45],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
