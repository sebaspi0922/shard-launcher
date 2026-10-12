// Botón «Jugar» con los estados del lanzamiento.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/launch.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';

enum PlayButtonSize { large, medium, small }

class PlayButton extends ConsumerWidget {
  const PlayButton({
    super.key,
    required this.instanceId,
    this.size = PlayButtonSize.medium,
    this.expand = false,
  });

  final String instanceId;
  final PlayButtonSize size;
  final bool expand;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(launchStateProvider(instanceId));
    final l = AppLocalizations.of(context);
    final c = context.colors;

    final (height, minWidth, font, icon) = switch (size) {
      PlayButtonSize.large => (56.0, 220.0, 17.0, 24.0),
      PlayButtonSize.medium => (44.0, 150.0, 14.5, 20.0),
      PlayButtonSize.small => (34.0, 96.0, 13.0, 18.0),
    };

    final label = switch (s.phase) {
      LaunchPhase.idle => l.play,
      LaunchPhase.preparing => l.launchPreparing,
      LaunchPhase.downloading => l.launchDownloading(s.done, s.total),
      LaunchPhase.starting => l.launchStarting,
      LaunchPhase.running => l.launchRunning,
      LaunchPhase.error => l.retry,
    };

    final (bg, fg, border) = switch (s.phase) {
      LaunchPhase.idle => (c.accent, c.onAccent, Colors.transparent),
      LaunchPhase.running => (
          c.success.withValues(alpha: 0.14),
          c.success,
          c.success.withValues(alpha: 0.4),
        ),
      LaunchPhase.error => (
          c.danger.withValues(alpha: 0.12),
          c.danger,
          c.danger.withValues(alpha: 0.45),
        ),
      _ => (c.accentContainer, c.onAccentContainer, c.accent.withValues(alpha: 0.35)),
    };

    final leading = switch (s.phase) {
      LaunchPhase.idle => Icon(Symbols.play_arrow_rounded, fill: 1, size: icon, color: fg),
      LaunchPhase.running => Container(
          width: icon * 0.42,
          height: icon * 0.42,
          decoration: BoxDecoration(
            color: fg,
            shape: BoxShape.circle,
            boxShadow: [BoxShadow(color: fg.withValues(alpha: 0.6), blurRadius: 8)],
          ),
        ),
      LaunchPhase.error => Icon(Symbols.refresh_rounded, size: icon, color: fg),
      LaunchPhase.downloading => Icon(Symbols.download_rounded, size: icon, color: fg),
      _ => SizedBox.square(
          dimension: icon * 0.8,
          child: CircularProgressIndicator(strokeWidth: 2.2, color: fg),
        ),
    };

    final enabled = s.phase == LaunchPhase.idle || s.phase == LaunchPhase.error;
    final radius = BorderRadius.circular(size == PlayButtonSize.large ? 14 : Radii.control);

    final button = AnimatedContainer(
      duration: Motion.medium,
      curve: Motion.curve,
      height: height,
      constraints: BoxConstraints(minWidth: minWidth),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: radius,
        border: Border.all(color: border),
        boxShadow: s.phase == LaunchPhase.idle && size != PlayButtonSize.small
            ? [
                BoxShadow(
                  color: c.accent.withValues(alpha: 0.35),
                  blurRadius: 24,
                  offset: const Offset(0, 6),
                ),
              ]
            : null,
      ),
      child: Material(
        type: MaterialType.transparency,
        borderRadius: radius,
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: enabled
              ? () => ref.read(launchProvider.notifier).launch(instanceId)
              : null,
          child: Stack(
            alignment: Alignment.center,
            children: [
              if (s.phase == LaunchPhase.downloading && s.progress != null)
                Positioned.fill(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: FractionallySizedBox(
                      widthFactor: s.progress,
                      heightFactor: 1,
                      child: ColoredBox(color: c.accent.withValues(alpha: 0.28)),
                    ),
                  ),
                ),
              Padding(
                padding: EdgeInsets.symmetric(horizontal: height * 0.45),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    leading,
                    SizedBox(width: height * 0.2),
                    Flexible(
                      child: Text(
                        label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: font,
                          fontWeight: FontWeight.w700,
                          color: fg,
                          letterSpacing: size == PlayButtonSize.large ? 0.2 : 0,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
    if (expand) return SizedBox(width: double.infinity, child: button);
    return IntrinsicWidth(child: button);
  }
}
