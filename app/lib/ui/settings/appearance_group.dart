// Ajustes › Apariencia: estilo, fondo, acento, desenfoque, oscurecido y modo
// rendimiento. Se guardan en `ui.json`.

import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/errors.dart';
import '../../core/providers.dart';
import '../../core/ui_prefs.dart';
import '../../l10n/app_localizations.dart';
import '../shell/backdrop.dart';
import '../theme/accent.dart';
import '../theme/styles.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/ascii_sphere.dart';
import '../widgets/atmosphere.dart';
import '../widgets/common.dart';
import 'settings_page.dart';

class AppearanceGroup extends ConsumerWidget {
  const AppearanceGroup({super.key});

  Future<void> _pickImage(BuildContext context, WidgetRef ref) async {
    final l = AppLocalizations.of(context);
    final f = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: l.appearanceImages,
          extensions: const ['png', 'jpg', 'jpeg', 'webp'],
        ),
      ],
    );
    if (f == null) return;
    try {
      final copy = await ref.read(bridgeProvider).importBackground(f.path);
      await ref.read(uiPrefsProvider.notifier).update(
            (p) => p.copyWith(
              background: BackgroundKind.image,
              backgroundPath: () => copy,
            ),
          );
    } catch (e) {
      if (context.mounted) showToast(context, errorText(l, e), error: true);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final p = ref.watch(uiPrefsProvider);
    final prefs = ref.read(uiPrefsProvider.notifier);
    final hasImage = p.backgroundPath != null && File(p.backgroundPath!).existsSync();
    final image = p.usesImage && hasImage;
    // El desenfoque y el oscurecido solo afectan a imágenes (paisaje o propia).
    final tunable = image || p.style == AppStyle.landscape;

    String styleName(AppStyle s) => switch (s) {
          AppStyle.graphite => l.styleGraphite,
          AppStyle.sebaspi => l.styleSebaspi,
          AppStyle.landscape => l.appearanceLandscape,
          AppStyle.aurora => l.appearanceAurora,
        };

    return SettingsGroup(
      title: l.settingsAppearance,
      icon: Symbols.palette_rounded,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: Space.x3),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.appearanceStyle, style: context.text.titleSmall),
              const SizedBox(height: 2),
              Text(l.appearanceStyleDesc, style: context.text.bodySmall),
              const SizedBox(height: Space.x3),
              Row(
                children: [
                  for (final s in AppStyle.values) ...[
                    if (s != AppStyle.values.first) const SizedBox(width: Space.x3),
                    Expanded(
                      child: _BackgroundOption(
                        label: styleName(s),
                        selected: p.style == s,
                        onTap: () => prefs.update((p) => p.copyWith(style: s)),
                        child: StylePreview(style: s),
                      ),
                    ),
                  ],
                ],
              ),
            ],
          ),
        ),
        const Divider(),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: Space.x3),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l.appearanceBackground, style: context.text.titleSmall),
              const SizedBox(height: 2),
              Text(l.appearanceBackgroundDesc, style: context.text.bodySmall),
              const SizedBox(height: Space.x3),
              Row(
                children: [
                  Expanded(
                    child: _BackgroundOption(
                      label: l.appearanceBackgroundStyle,
                      selected: !image,
                      onTap: () => prefs.update(
                        (p) => p.copyWith(background: BackgroundKind.style),
                      ),
                      child: StylePreview(style: p.style),
                    ),
                  ),
                  const SizedBox(width: Space.x3),
                  Expanded(
                    child: _BackgroundOption(
                      label: hasImage ? l.appearanceImage : l.appearancePickImage,
                      selected: image,
                      onTap: () => hasImage
                          ? prefs.update(
                              (p) => p.copyWith(background: BackgroundKind.image),
                            )
                          : _pickImage(context, ref),
                      trailing: hasImage
                          ? IconButton(
                              tooltip: l.appearanceChangeImage,
                              iconSize: 16,
                              visualDensity: VisualDensity.compact,
                              onPressed: () => _pickImage(context, ref),
                              icon: const Icon(Symbols.edit_rounded),
                            )
                          : null,
                      child: hasImage
                          ? Image.file(
                              File(p.backgroundPath!),
                              fit: BoxFit.cover,
                              cacheWidth: 360,
                              errorBuilder: (_, _, _) => const SizedBox(),
                            )
                          : ColoredBox(
                              color: c.surface1,
                              child: Center(
                                child: Icon(
                                  Symbols.add_photo_alternate_rounded,
                                  color: c.textMuted,
                                  size: 28,
                                ),
                              ),
                            ),
                    ),
                  ),
                  // Mismo ancho de tarjeta que los estilos.
                  const SizedBox(width: Space.x3),
                  const Spacer(),
                  const SizedBox(width: Space.x3),
                  const Spacer(),
                ],
              ),
            ],
          ),
        ),
        const Divider(),
        SettingRow(
          title: l.appearanceAccent,
          description: switch (p.accent) {
            AccentMode.style => l.appearanceAccentStyleDesc,
            AccentMode.auto => l.appearanceAccentAutoDesc,
            AccentMode.preset => l.appearanceAccentDesc,
          },
          control: Wrap(
            spacing: Space.x2,
            runSpacing: Space.x2,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              _ModeChip(
                icon: Symbols.style_rounded,
                label: l.appearanceAccentStyle,
                selected: p.accent == AccentMode.style,
                onTap: () => prefs.update((p) => p.copyWith(accent: AccentMode.style)),
              ),
              if (image)
                _ModeChip(
                  icon: Symbols.auto_awesome_rounded,
                  label: l.appearanceAccentAuto,
                  selected: p.accent == AccentMode.auto,
                  onTap: () => prefs.update((p) => p.copyWith(accent: AccentMode.auto)),
                ),
              const SizedBox(width: Space.x1),
              for (final color in kAccentPresets)
                _Swatch(
                  color: color,
                  selected: p.accent == AccentMode.preset &&
                      p.accentPreset == color.toARGB32(),
                  onTap: () => prefs.update(
                    (p) => p.copyWith(
                      accent: AccentMode.preset,
                      accentPreset: color.toARGB32(),
                    ),
                  ),
                ),
            ],
          ),
        ),
        const Divider(),
        SettingRow(
          title: l.appearanceBlur,
          description: l.appearanceBlurDesc,
          controlWidth: 320,
          control: _PercentSlider(
            value: p.blur / 40,
            enabled: tunable && !p.performance,
            onChanged: (v) => prefs.update(
              (p) => p.copyWith(blur: v * 40),
              debounce: true,
            ),
          ),
        ),
        const Divider(),
        SettingRow(
          title: l.appearanceDim,
          description: l.appearanceDimDesc,
          controlWidth: 320,
          control: _PercentSlider(
            value: p.dim / 0.85,
            enabled: tunable,
            onChanged: (v) => prefs.update(
              (p) => p.copyWith(dim: v * 0.85),
              debounce: true,
            ),
          ),
        ),
        const Divider(),
        SettingRow(
          title: l.appearancePerformance,
          description: l.appearancePerformanceDesc,
          control: Switch(
            value: p.performance,
            onChanged: (v) => prefs.update((p) => p.copyWith(performance: v)),
          ),
        ),
      ],
    );
  }
}

/// Miniatura de un estilo: su fondo (quieto) con sus colores, a escala.
class StylePreview extends StatelessWidget {
  const StylePreview({super.key, required this.style});

  final AppStyle style;

  @override
  Widget build(BuildContext context) {
    final colors = style.spec.palette(null);
    final Widget art = switch (style) {
      AppStyle.graphite => Atmosphere(background: colors.background, animate: false),
      AppStyle.sebaspi => AsciiSphere(color: colors.accent, animate: false),
      AppStyle.landscape => const BlockLandscape(seed: kBackdropLandscapeSeed, block: 26),
      AppStyle.aurora => const Backdrop(),
    };
    return Theme(
      data: Theme.of(context).copyWith(extensions: [colors]),
      child: IgnorePointer(
        child: FittedBox(
          fit: BoxFit.cover,
          clipBehavior: Clip.hardEdge,
          child: SizedBox(
            width: 1280,
            height: 800,
            child: Stack(
              fit: StackFit.expand,
              children: [
                art,
                // Un botón del acento del estilo, para verlo de un vistazo.
                Positioned(
                  left: 96,
                  bottom: 96,
                  child: Container(
                    width: 300,
                    height: 84,
                    decoration: BoxDecoration(
                      color: colors.accent,
                      borderRadius: BorderRadius.circular(style.spec.control * 3),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _BackgroundOption extends StatelessWidget {
  const _BackgroundOption({
    required this.label,
    required this.selected,
    required this.onTap,
    required this.child,
    this.trailing,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;
  final Widget child;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    // El nombre también elige el fondo.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Material(
            color: Colors.transparent,
            child: InkWell(
              onTap: onTap,
              borderRadius: BorderRadius.circular(Radii.card),
              child: AnimatedContainer(
                duration: Motion.fast,
                height: 92,
                padding: const EdgeInsets.all(3),
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(Radii.card),
                  border: Border.all(
                    color: selected ? c.accent : c.border,
                    width: selected ? 2 : 1,
                  ),
                ),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(Radii.card - 3),
                  child: child,
                ),
              ),
            ),
          ),
          const SizedBox(height: Space.x2),
          Row(
            children: [
              if (selected) ...[
                Icon(Symbols.check_circle_rounded, size: 16, color: c.accent, fill: 1),
                const SizedBox(width: 6),
              ],
              Expanded(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.text.labelLarge!.copyWith(
                    color: selected ? c.text : c.textSecondary,
                  ),
                ),
              ),
              ?trailing,
            ],
          ),
        ],
      ),
    );
  }
}

class _Swatch extends StatelessWidget {
  const _Swatch({
    required this.color,
    required this.selected,
    required this.onTap,
  });

  final Color color;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Padding(
      padding: const EdgeInsets.only(left: 6),
      child: InkWell(
        onTap: onTap,
        customBorder: const CircleBorder(),
        child: AnimatedContainer(
          duration: Motion.fast,
          width: 26,
          height: 26,
          decoration: BoxDecoration(
            color: color,
            shape: BoxShape.circle,
            border: Border.all(
              color: selected ? c.text : Colors.black.withValues(alpha: 0.25),
              width: selected ? 2 : 1,
            ),
          ),
          child: selected
              ? const Icon(Symbols.check_rounded, size: 14, color: Color(0xFF101010), weight: 700)
              : null,
        ),
      ),
    );
  }
}

class _ModeChip extends StatelessWidget {
  const _ModeChip({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(Radii.pill),
      child: AnimatedContainer(
        duration: Motion.fast,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 5),
        decoration: BoxDecoration(
          color: selected ? c.accentContainer : c.surface1,
          borderRadius: BorderRadius.circular(Radii.pill),
          border: Border.all(
            color: selected ? c.accent.withValues(alpha: 0.6) : c.border,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 15,
              color: selected ? c.accent : c.textMuted,
            ),
            const SizedBox(width: 6),
            Text(
              label,
              style: context.text.labelLarge!.copyWith(
                color: selected ? c.onAccentContainer : c.textSecondary,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _PercentSlider extends StatelessWidget {
  const _PercentSlider({
    required this.value,
    required this.enabled,
    required this.onChanged,
  });

  final double value;
  final bool enabled;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final v = value.clamp(0.0, 1.0);
    return Row(
      children: [
        Expanded(
          child: Slider(
            value: v,
            onChanged: enabled ? onChanged : null,
          ),
        ),
        SizedBox(
          width: 48,
          child: Text(
            '${(v * 100).round()} %',
            textAlign: TextAlign.end,
            style: context.text.labelLarge!.copyWith(
              color: enabled ? c.text : c.textMuted,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ),
      ],
    );
  }
}
