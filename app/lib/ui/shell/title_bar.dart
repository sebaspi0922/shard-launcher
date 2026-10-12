// Barra de título propia: arrastre, doble clic para maximizar, actividad en
// curso y botones de ventana.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../core/window.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';

const kTitleBarHeight = 44.0;

/// Zona que mueve la ventana al arrastrar y la maximiza con doble clic.
class WindowDragArea extends ConsumerWidget {
  const WindowDragArea({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final w = ref.watch(windowControlsProvider);
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onPanStart: (_) => w.startDragging(),
      onDoubleTap: w.toggleMaximize,
      child: child,
    );
  }
}

class TitleBar extends ConsumerWidget {
  const TitleBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final w = ref.watch(windowControlsProvider);
    final l = AppLocalizations.of(context);
    return SizedBox(
      height: kTitleBarHeight,
      child: Row(
        children: [
          const Expanded(
            child: WindowDragArea(
              child: Center(child: _ActivityPill()),
            ),
          ),
          _WindowButton(
            icon: Symbols.remove_rounded,
            tooltip: l.windowMinimize,
            onTap: w.minimize,
          ),
          ValueListenableBuilder<bool>(
            valueListenable: w.maximized,
            builder: (_, max, _) => _WindowButton(
              icon: max ? Symbols.filter_none_rounded : Symbols.crop_square_rounded,
              iconSize: max ? 14 : 16,
              tooltip: max ? l.windowRestore : l.windowMaximize,
              onTap: w.toggleMaximize,
            ),
          ),
          _WindowButton(
            icon: Symbols.close_rounded,
            tooltip: l.windowClose,
            onTap: w.requestClose,
            danger: true,
          ),
          const SizedBox(width: Space.x1),
        ],
      ),
    );
  }
}

/// Píldora central con el lanzamiento o la partida en curso.
class _ActivityPill extends ConsumerWidget {
  const _ActivityPill();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final active = ref.watch(activeLaunchProvider);
    if (active == null) return const SizedBox.shrink();
    final instance = ref.watch(instanceByIdProvider(active.key));
    final s = active.value;
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final label = switch (s.phase) {
      LaunchPhase.preparing => l.launchPreparing,
      LaunchPhase.downloading => l.launchDownloading(s.done, s.total),
      LaunchPhase.starting => l.launchStarting,
      _ => l.launchRunning,
    };
    final running = s.phase == LaunchPhase.running;
    return Material(
      color: c.surface2,
      shape: StadiumBorder(side: BorderSide(color: c.border)),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => context.go('/instances/${active.key}'),
        child: SizedBox(
          height: 28,
          child: Stack(
            children: [
              if (s.progress != null)
                Positioned.fill(
                  child: FractionallySizedBox(
                    alignment: Alignment.centerLeft,
                    widthFactor: s.progress,
                    child: ColoredBox(color: c.accent.withValues(alpha: 0.18)),
                  ),
                ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (running)
                      Container(
                        width: 7,
                        height: 7,
                        decoration: BoxDecoration(
                          color: c.success,
                          shape: BoxShape.circle,
                        ),
                      )
                    else
                      const SizedBox.square(
                        dimension: 12,
                        child: CircularProgressIndicator(strokeWidth: 1.6),
                      ),
                    const SizedBox(width: Space.x2),
                    Text(
                      instance?.name ?? active.key,
                      style: context.text.labelMedium!.copyWith(color: c.text),
                    ),
                    const SizedBox(width: Space.x2),
                    Text(
                      label,
                      style: context.text.labelMedium!.copyWith(
                        color: running ? c.success : c.textSecondary,
                        fontFeatures: const [FontFeature.tabularFigures()],
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
  }
}

class _WindowButton extends StatefulWidget {
  const _WindowButton({
    required this.icon,
    required this.tooltip,
    required this.onTap,
    this.danger = false,
    this.iconSize = 18,
  });

  final IconData icon;
  final String tooltip;
  final Future<void> Function() onTap;
  final bool danger;
  final double iconSize;

  @override
  State<_WindowButton> createState() => _WindowButtonState();
}

class _WindowButtonState extends State<_WindowButton> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final bg = !_hover
        ? Colors.transparent
        : widget.danger
            ? const Color(0xFFE5484D)
            : c.surface2;
    return Tooltip(
      message: widget.tooltip,
      child: MouseRegion(
        onEnter: (_) => setState(() => _hover = true),
        onExit: (_) => setState(() => _hover = false),
        child: GestureDetector(
          onTap: widget.onTap,
          child: AnimatedContainer(
            duration: Motion.fast,
            width: 40,
            height: 30,
            margin: const EdgeInsets.symmetric(horizontal: 2),
            decoration: BoxDecoration(
              color: bg,
              borderRadius: BorderRadius.circular(Radii.control),
            ),
            child: Icon(
              widget.icon,
              size: widget.iconSize,
              color: _hover && widget.danger ? Colors.white : c.textSecondary,
            ),
          ),
        ),
      ),
    );
  }
}
