// Papelera de instancias: recuperar o borrar para siempre. El núcleo vacía
// solo lo que lleva más de 30 días.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';

/// Días que una instancia pasa en la papelera antes de borrarse sola.
const kTrashRetentionDays = 30;

Future<void> showTrash(BuildContext context) => showDialog<void>(
      context: context,
      builder: (_) => const TrashDialog(),
    );

class TrashDialog extends ConsumerStatefulWidget {
  const TrashDialog({super.key});

  @override
  ConsumerState<TrashDialog> createState() => _TrashDialogState();
}

class _TrashDialogState extends ConsumerState<TrashDialog> {
  String? _busy;

  Future<void> _restore(TrashedView t) async {
    final l = AppLocalizations.of(context);
    setState(() => _busy = t.trashId);
    try {
      final i = await ref.read(instancesProvider.notifier).restore(t.trashId);
      if (!mounted) return;
      showToast(context, l.trashRestored(i.name));
      Navigator.pop(context);
      context.go('/instances/${i.id}');
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _busy = null);
    }
  }

  Future<void> _purge(List<TrashedView> items) async {
    final l = AppLocalizations.of(context);
    final ok = await confirm(
      context,
      title: items.length == 1
          ? l.trashPurgeTitle(items.first.name)
          : l.trashEmptyTitle(items.length),
      body: l.trashPurgeBody,
      action: l.trashPurge,
      danger: true,
    );
    if (!ok || !mounted) return;
    setState(() => _busy = items.length == 1 ? items.first.trashId : '*');
    try {
      for (final t in items) {
        await ref.read(instancesProvider.notifier).purge(t.trashId);
      }
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _busy = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final async = ref.watch(trashProvider);
    final items = async.value;

    return AppDialog(
      title: l.trashTitle,
      icon: Symbols.delete_rounded,
      width: 620,
      actions: [
        if (items != null && items.isNotEmpty)
          TextButton.icon(
            style: TextButton.styleFrom(foregroundColor: context.colors.danger),
            onPressed: _busy != null ? null : () => _purge(items),
            icon: const Icon(Symbols.delete_forever_rounded, size: 18),
            label: Text(l.trashEmptyAll),
          ),
        FilledButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.close),
        ),
      ],
      child: switch (async) {
        AsyncError(:final error) => ErrorNote(error),
        AsyncData(value: final list) when list.isEmpty => SizedBox(
            height: 240,
            child: EmptyState(
              icon: Symbols.delete_sweep_rounded,
              title: l.trashEmptyStateTitle,
              body: l.trashEmptyStateBody(kTrashRetentionDays),
            ),
          ),
        AsyncData(value: final list) => Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                l.trashHint(kTrashRetentionDays),
                style: context.text.bodySmall,
              ),
              const SizedBox(height: Space.x4),
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 420),
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: list.length,
                  separatorBuilder: (_, _) => const SizedBox(height: Space.x2),
                  itemBuilder: (context, i) => _TrashRow(
                    item: list[i],
                    busy: _busy == list[i].trashId || _busy == '*',
                    enabled: _busy == null,
                    onRestore: () => _restore(list[i]),
                    onPurge: () => _purge([list[i]]),
                  ),
                ),
              ),
            ],
          ),
        _ => const SizedBox(
            height: 200,
            child: Center(child: CircularProgressIndicator()),
          ),
      },
    );
  }
}

class _TrashRow extends StatelessWidget {
  const _TrashRow({
    required this.item,
    required this.busy,
    required this.enabled,
    required this.onRestore,
    required this.onPurge,
  });

  final TrashedView item;
  final bool busy;
  final bool enabled;
  final VoidCallback onRestore;
  final VoidCallback onPurge;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final t = item;
    final deleted = DateTime.fromMillisecondsSinceEpoch(t.deletedAt * 1000);
    final left = kTrashRetentionDays - appClock().difference(deleted).inDays;
    return Panel(
      padding: const EdgeInsets.all(Space.x3),
      child: Row(
        children: [
          Opacity(
            opacity: 0.75,
            child: InstanceIcon(id: t.id, name: t.name, size: 44),
          ),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  t.name,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.text.titleSmall,
                ),
                const SizedBox(height: 2),
                Text(
                  [
                    versionLine(l, t.mc, t.loader),
                    l.trashDeletedAgo(formatRelative(l, t.deletedAt)),
                    formatPlaytime(l, t.playtimeSeconds),
                  ].join(' · '),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.text.bodySmall,
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.x2),
          Tag(
            l.trashDaysLeft(left.clamp(0, kTrashRetentionDays)),
            icon: Symbols.hourglass_bottom_rounded,
          ),
          const SizedBox(width: Space.x2),
          IconButton(
            tooltip: l.trashPurge,
            onPressed: enabled ? onPurge : null,
            icon: Icon(Symbols.delete_forever_rounded, size: 20, color: c.danger),
          ),
          const SizedBox(width: Space.x1),
          OutlinedButton.icon(
            onPressed: enabled ? onRestore : null,
            icon: busy
                ? const SizedBox.square(
                    dimension: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Symbols.restore_from_trash_rounded, size: 18),
            label: Text(l.trashRestore),
          ),
        ],
      ),
    );
  }
}
