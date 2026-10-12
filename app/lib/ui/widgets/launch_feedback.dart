// Aviso del último fallo de lanzamiento o de partida de una instancia.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/errors.dart';
import '../../core/launch.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

class LaunchFailureNote extends ConsumerWidget {
  const LaunchFailureNote({
    super.key,
    required this.instanceId,
    this.showLogsLink = true,
  });

  final String instanceId;
  final bool showLogsLink;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(launchStateProvider(instanceId));
    final failure = s.failure;
    if (s.phase != LaunchPhase.error || failure == null) {
      return const SizedBox.shrink();
    }
    final l = AppLocalizations.of(context);
    final c = context.colors;

    final (title, detail, crashed) = switch (failure) {
      LaunchError(:final error) => (
          l.launchFailed,
          errorText(l, error),
          false,
        ),
      GameCrash(:final code, :final crashReport) => (
          l.launchCrashed,
          [
            if (code != null) l.launchExitCode(code),
            if (crashReport != null) l.launchCrashReport(crashReport),
          ].join('\n'),
          true,
        ),
    };
    final tech = failure is LaunchError ? errorDetail(failure.error) : null;

    return Container(
      padding: const EdgeInsets.fromLTRB(Space.x4, Space.x3, Space.x2, Space.x3),
      decoration: BoxDecoration(
        color: Color.alphaBlend(
          c.danger.withValues(alpha: 0.14),
          Colors.black.withValues(alpha: 0.55),
        ),
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: c.danger.withValues(alpha: 0.45)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 1),
            child: Icon(Symbols.error_rounded, color: c.danger, size: 20),
          ),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: context.text.titleSmall),
                if (detail.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    detail,
                    style: context.text.bodySmall!.copyWith(color: c.textSecondary),
                  ),
                ],
                if (tech != null) ...[
                  const SizedBox(height: Space.x1),
                  Text(
                    tech,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: context.text.bodySmall!.copyWith(
                      fontFamily: kMonoFamily,
                      fontSize: 11,
                      color: c.textMuted,
                    ),
                  ),
                ],
                if (crashed && showLogsLink) ...[
                  const SizedBox(height: Space.x2),
                  TextButton.icon(
                    style: TextButton.styleFrom(
                      foregroundColor: c.text,
                      padding: const EdgeInsets.symmetric(horizontal: 8),
                      minimumSize: const Size(0, 30),
                    ),
                    onPressed: () => context.go('/instances/$instanceId?tab=logs'),
                    icon: const Icon(Symbols.terminal_rounded, size: 16),
                    label: Text(l.launchSeeLogs),
                  ),
                ],
              ],
            ),
          ),
          IconButton(
            tooltip: l.close,
            visualDensity: VisualDensity.compact,
            iconSize: 18,
            onPressed: () => ref.read(launchProvider.notifier).dismiss(instanceId),
            icon: const Icon(Symbols.close_rounded),
          ),
        ],
      ),
    );
  }
}
