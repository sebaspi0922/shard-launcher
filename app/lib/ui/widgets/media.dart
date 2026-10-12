// Imágenes: iconos remotos con caché en disco, imágenes locales con respaldo
// procedural y pestañas en forma de píldora.

import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/content_search.dart';
import '../theme/tokens.dart';
import 'art.dart';

/// Imagen de un archivo local; si no existe o falla, `fallback`.
class LocalImage extends StatelessWidget {
  const LocalImage({
    super.key,
    required this.path,
    required this.fallback,
    this.fit = BoxFit.cover,
    this.cacheWidth,
  });

  final String? path;
  final Widget fallback;
  final BoxFit fit;

  /// Ancho de decodificación (miniaturas: menos memoria).
  final int? cacheWidth;

  @override
  Widget build(BuildContext context) {
    final p = path;
    if (p == null || !File(p).existsSync()) return fallback;
    return Image.file(
      File(p),
      fit: fit,
      cacheWidth: cacheWidth,
      filterQuality: FilterQuality.medium,
      gaplessPlayback: true,
      errorBuilder: (_, _, _) => fallback,
    );
  }
}

/// Icono de un proyecto de Modrinth/CurseForge. Se descarga una vez al disco;
/// mientras tanto (o si no hay), `fallback` o, por defecto, un icono generado
/// con sus iniciales.
class ProjectIcon extends ConsumerWidget {
  const ProjectIcon({
    super.key,
    required this.url,
    required this.seed,
    required this.title,
    this.size = 56,
    this.fallback,
  });

  final String? url;
  final String seed;
  final String title;
  final double size;

  /// Lo que se ve sin icono (por defecto, las iniciales de `title`).
  final Widget? fallback;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fallback =
        this.fallback ?? InstanceIcon(id: seed, name: title, size: size);
    final u = url;
    if (u == null || u.isEmpty) return fallback;
    final path = ref.watch(cachedImageProvider(u)).value;
    if (path == null) return fallback;
    return ClipRRect(
      borderRadius: BorderRadius.circular(size * 0.26),
      child: SizedBox.square(
        dimension: size,
        child: ColoredBox(
          color: context.colors.surface2,
          child: LocalImage(
            path: path,
            fallback: fallback,
            cacheWidth: (size * 2).round(),
          ),
        ),
      ),
    );
  }
}

/// Grupo de píldoras seleccionables con contador opcional.
class PillTabs<T> extends StatelessWidget {
  const PillTabs({
    super.key,
    required this.items,
    required this.selected,
    required this.onSelect,
  });

  final List<({T value, String label, int? count, IconData? icon})> items;
  final T selected;
  final ValueChanged<T> onSelect;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Wrap(
      spacing: Space.x2,
      runSpacing: Space.x2,
      children: [
        for (final it in items)
          _Pill(
            label: it.label,
            count: it.count,
            icon: it.icon,
            selected: it.value == selected,
            onTap: () => onSelect(it.value),
            colors: c,
          ),
      ],
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({
    required this.label,
    required this.count,
    required this.icon,
    required this.selected,
    required this.onTap,
    required this.colors,
  });

  final String label;
  final int? count;
  final IconData? icon;
  final bool selected;
  final VoidCallback onTap;
  final AppColors colors;

  @override
  Widget build(BuildContext context) {
    final c = colors;
    final fg = selected ? c.onAccentContainer : c.textSecondary;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(Radii.pill),
        child: AnimatedContainer(
          duration: Motion.fast,
          curve: Motion.curve,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          decoration: BoxDecoration(
            color: selected ? c.accentContainer : c.surface1,
            borderRadius: BorderRadius.circular(Radii.pill),
            border: Border.all(
              color: selected ? c.accent.withValues(alpha: 0.45) : c.border,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (icon != null) ...[
                Icon(icon, size: 16, color: selected ? c.accent : c.textMuted),
                const SizedBox(width: 6),
              ],
              Text(
                label,
                style: context.text.labelLarge!.copyWith(
                  color: fg,
                  fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
                ),
              ),
              if (count != null) ...[
                const SizedBox(width: 6),
                Text(
                  '$count',
                  style: context.text.labelMedium!.copyWith(
                    color: selected ? c.accent : c.textMuted,
                    fontFeatures: const [FontFeature.tabularFigures()],
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Pequeña etiqueta de color (estado, origen, aviso).
class ColorBadge extends StatelessWidget {
  const ColorBadge(this.label, {super.key, required this.color, this.icon});

  final String label;
  final Color color;
  final IconData? icon;

  @override
  Widget build(BuildContext context) => Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(Radii.pill),
          border: Border.all(color: color.withValues(alpha: 0.3)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 13, color: color),
              const SizedBox(width: 4),
            ],
            Text(
              label,
              style: context.text.labelSmall!.copyWith(
                color: color,
                letterSpacing: 0.2,
              ),
            ),
          ],
        ),
      );
}
