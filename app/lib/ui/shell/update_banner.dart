// Aviso flotante de versión nueva: instalar y reiniciar (AppImage) o abrir la
// descarga (Flatpak, pacman).

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/tasks.dart';
import '../../core/updates.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';

class UpdateBanner extends ConsumerWidget {
  const UpdateBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(updateProvider);
    final info = s.info;
    final visible = info != null &&
        !s.dismissed &&
        switch (s.status) {
          UpdateStatus.available ||
          UpdateStatus.installing ||
          UpdateStatus.ready ||
          UpdateStatus.failed =>
            true,
          _ => false,
        };
    final c = context.colors;

    return AnimatedSlide(
      duration: Motion.medium,
      curve: Motion.curve,
      offset: visible ? Offset.zero : const Offset(0, 1.4),
      child: AnimatedOpacity(
        duration: Motion.fast,
        opacity: visible ? 1 : 0,
        child: !visible
            ? const SizedBox.shrink()
            : Container(
                width: 400,
                padding: const EdgeInsets.fromLTRB(Space.x4, Space.x4, Space.x2, Space.x3),
                decoration: BoxDecoration(
                  color: Color.alphaBlend(
                    c.accent.withValues(alpha: 0.08),
                    Color.alphaBlend(c.surface3, c.background),
                  ),
                  borderRadius: BorderRadius.circular(Radii.card),
                  border: Border.all(color: c.accent.withValues(alpha: 0.40)),
                  boxShadow: [
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.45),
                      blurRadius: 28,
                      offset: const Offset(0, 10),
                    ),
                  ],
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Container(
                      width: 36,
                      height: 36,
                      decoration: BoxDecoration(
                        color: c.accent.withValues(alpha: 0.16),
                        borderRadius: BorderRadius.circular(Radii.control),
                      ),
                      child: Icon(Symbols.system_update_alt_rounded, size: 20, color: c.accent),
                    ),
                    const SizedBox(width: Space.x3),
                    Expanded(child: UpdateDetails(state: s, info: info)),
                    IconButton(
                      tooltip: AppLocalizations.of(context).close,
                      visualDensity: VisualDensity.compact,
                      iconSize: 18,
                      onPressed: () => ref.read(updateProvider.notifier).dismiss(),
                      icon: const Icon(Symbols.close_rounded),
                    ),
                  ],
                ),
              ),
      ),
    );
  }
}

/// Título, estado y acciones de una versión nueva (aviso y Ajustes).
class UpdateDetails extends ConsumerWidget {
  const UpdateDetails({super.key, required this.state, required this.info});

  final UpdateState state;
  final UpdateView info;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final ctrl = ref.read(updateProvider.notifier);
    final progress = ref.watch(stageProgressProvider('update'));
    final notesUrl = info.notesUrl;
    const buttonSize = Size(0, 34);

    final status = switch (state.status) {
      UpdateStatus.installing => l.updateInstalling,
      UpdateStatus.ready => l.updateReady,
      UpdateStatus.failed when state.error != null =>
        '${l.updateFailed}: ${errorText(l, state.error!)}',
      _ when !info.canInstall => l.updateManualHint,
      _ => info.notes ?? l.updateAvailableBody(info.current),
    };

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(l.updateAvailableTitle(info.version), style: context.text.titleSmall),
        const SizedBox(height: 2),
        Text(
          status,
          maxLines: 3,
          overflow: TextOverflow.ellipsis,
          style: context.text.bodySmall?.copyWith(
            color: state.status == UpdateStatus.failed ? c.danger : null,
          ),
        ),
        if (state.status == UpdateStatus.installing) ...[
          const SizedBox(height: Space.x3),
          ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: LinearProgressIndicator(minHeight: 4, value: progress?.fraction),
          ),
        ] else ...[
          const SizedBox(height: Space.x3),
          Wrap(
            spacing: Space.x2,
            runSpacing: Space.x2,
            children: [
              if (state.status == UpdateStatus.ready)
                FilledButton.icon(
                  style: FilledButton.styleFrom(minimumSize: buttonSize),
                  onPressed: ctrl.restart,
                  icon: const Icon(Symbols.restart_alt_rounded, size: 18),
                  label: Text(l.updateRestart),
                )
              else if (info.canInstall)
                FilledButton.icon(
                  style: FilledButton.styleFrom(minimumSize: buttonSize),
                  onPressed: ctrl.install,
                  icon: const Icon(Symbols.download_rounded, size: 18),
                  label: Text(l.updateInstall),
                )
              else
                FilledButton.icon(
                  style: FilledButton.styleFrom(minimumSize: buttonSize),
                  onPressed: () => launchUrl(Uri.parse(info.downloadUrl)),
                  icon: const Icon(Symbols.open_in_new_rounded, size: 18),
                  label: Text(l.updateDownload),
                ),
              if (notesUrl != null)
                TextButton(
                  style: TextButton.styleFrom(minimumSize: buttonSize),
                  onPressed: () => launchUrl(Uri.parse(notesUrl)),
                  child: Text(l.updateNotes),
                ),
            ],
          ),
        ],
      ],
    );
  }
}
