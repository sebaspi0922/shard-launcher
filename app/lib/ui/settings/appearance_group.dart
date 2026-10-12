// Ajustes › Apariencia: acento, fondo, desenfoque, oscurecido y modo
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
import '../theme/tokens.dart';
import '../widgets/art.dart';
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
    final blurEnabled = !p.performance && p.background != BackgroundKind.aurora;

    return SettingsGroup(
      title: l.settingsAppearance,
      icon: Symbols.palette_rounded,
      children: [
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
                      label: l.appearanceAurora,
                      selected: p.background == BackgroundKind.aurora,
                      onTap: () => prefs.update(
                        (p) => p.copyWith(background: BackgroundKind.aurora),
                      ),
                      child: const Backdrop(),
                    ),
                  ),
                  const SizedBox(width: Space.x3),
                  Expanded(
                    child: _BackgroundOption(
                      label: l.appearanceLandscape,
                      selected: p.background == BackgroundKind.landscape,
                      onTap: () => prefs.update(
                        (p) => p.copyWith(background: BackgroundKind.landscape),
                      ),
                      child: const BlockLandscape(seed: kBackdropLandscapeSeed, block: 9),
                    ),
                  ),
                  const SizedBox(width: Space.x3),
                  Expanded(
                    child: _BackgroundOption(
                      label: hasImage ? l.appearanceImage : l.appearancePickImage,
                      selected: p.background == BackgroundKind.image && hasImage,
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
                ],
              ),
            ],
          ),
        ),
        const Divider(),
        SettingRow(
          title: l.appearanceAccent,
          description: p.accentAuto ? l.appearanceAccentAutoDesc : l.appearanceAccentDesc,
          control: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              _AutoChip(
                selected: p.accentAuto,
                onTap: () => prefs.update((p) => p.copyWith(accentAuto: true)),
              ),
              const SizedBox(width: Space.x3),
              for (final color in kAccentPresets)
                _Swatch(
                  color: color,
                  selected: !p.accentAuto && p.accentPreset == color.toARGB32(),
                  onTap: () => prefs.update(
                    (p) => p.copyWith(
                      accentAuto: false,
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
            enabled: blurEnabled,
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
            enabled: p.background != BackgroundKind.aurora,
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

class _AutoChip extends StatelessWidget {
  const _AutoChip({required this.selected, required this.onTap});

  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
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
              Symbols.auto_awesome_rounded,
              size: 15,
              color: selected ? c.accent : c.textMuted,
            ),
            const SizedBox(width: 6),
            Text(
              l.appearanceAccentAuto,
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
