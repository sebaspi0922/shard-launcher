// Pestaña «Copias»: copias de seguridad con crear, fijar, etiquetar, restaurar
// (completa o solo mundos) y borrar, y los ajustes de las automáticas.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/launch.dart';
import '../../core/tasks.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

String backupTitle(AppLocalizations l, BackupView b) =>
    b.label ?? (b.automatic ? l.backupAutomatic : l.backupManual);

class BackupsTab extends ConsumerWidget {
  const BackupsTab({super.key, required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(backupsProvider(instance.id));
    final list = async.value ?? const <BackupView>[];
    final running =
        ref.watch(launchStateProvider(instance.id)).phase == LaunchPhase.running;
    final total = list.fold<BigInt>(BigInt.zero, (a, b) => a + b.sizeBytes);

    final Widget body;
    if (list.isEmpty) {
      body = switch (async) {
        AsyncError(:final error) => Center(child: ErrorNote(error)),
        AsyncLoading() => const Center(child: CircularProgressIndicator()),
        _ => Panel(
            child: EmptyState(
              icon: Symbols.cloud_sync_rounded,
              title: l.backupsEmptyTitle,
              body: l.backupsEmptyBody,
            ),
          ),
      };
    } else {
      body = Align(
        alignment: Alignment.topCenter,
        child: Panel(
          padding: EdgeInsets.zero,
          child: ListView.separated(
            shrinkWrap: true,
            padding: const EdgeInsets.symmetric(vertical: Space.x1),
            itemCount: list.length,
            separatorBuilder: (_, _) =>
                Divider(indent: Space.x4, endIndent: Space.x4, color: c.border),
            itemBuilder: (context, i) => _BackupRow(
              instance: instance,
              backup: list[i],
              locked: running,
            ),
          ),
        ),
      );
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Text(
                    list.isEmpty
                        ? l.backupsTitle
                        : l.backupsSummary(list.length, formatBytes(l, total)),
                    style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                  ),
                  const Spacer(),
                  Tooltip(
                    message: running ? l.closeGameFirst : '',
                    child: FilledButton.icon(
                      onPressed: running
                          ? null
                          : () => showCreateBackup(context, instance.id),
                      icon: const Icon(Symbols.add_rounded, size: 20),
                      label: Text(l.backupCreate),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.x3),
              Expanded(child: body),
            ],
          ),
        ),
        const SizedBox(width: Space.x5),
        SizedBox(width: 330, child: _AutoSettings(instanceId: instance.id)),
      ],
    );
  }
}

class _BackupRow extends ConsumerWidget {
  const _BackupRow({
    required this.instance,
    required this.backup,
    required this.locked,
  });

  final InstanceView instance;
  final BackupView backup;
  final bool locked;

  Future<void> _run(BuildContext context, Future<void> Function() f) async {
    try {
      await f();
    } catch (e) {
      if (context.mounted) {
        showToast(context, errorText(AppLocalizations.of(context), e), error: true);
      }
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final b = backup;
    final ctrl = ref.read(backupsProvider(instance.id).notifier);
    final color = b.automatic ? c.info : c.accent;
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x4, 10, Space.x2, 10),
      child: Row(
        children: [
          Container(
            width: 42,
            height: 42,
            decoration: BoxDecoration(
              color: color.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: color.withValues(alpha: 0.25)),
            ),
            child: Icon(
              b.automatic ? Symbols.schedule_rounded : Symbols.inventory_2_rounded,
              size: 20,
              color: color,
            ),
          ),
          const SizedBox(width: Space.x4),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        backupTitle(l, b),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.text.titleSmall,
                      ),
                    ),
                    const SizedBox(width: Space.x2),
                    if (b.automatic && b.label != null)
                      ColorBadge(l.backupAutoBadge, color: c.info),
                    if (b.pinned) ...[
                      const SizedBox(width: Space.x1),
                      ColorBadge(l.backupPinnedBadge, color: c.warning),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  '${formatDateTime(l, b.createdAt)}  ·  ${formatRelative(l, b.createdAt)}  ·  ${formatBytes(l, b.sizeBytes)}',
                  style: context.text.bodySmall!.copyWith(color: c.textMuted),
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: b.pinned ? l.backupUnpin : l.backupPin,
            isSelected: b.pinned,
            style: IconButton.styleFrom(
              foregroundColor: b.pinned ? c.warning : c.textSecondary,
            ),
            onPressed: () => _run(context, () => ctrl.setPinned(b.id, !b.pinned)),
            icon: Icon(Symbols.push_pin_rounded, fill: b.pinned ? 1 : 0, size: 19),
          ),
          const SizedBox(width: Space.x1),
          Tooltip(
            message: locked ? l.closeGameFirst : '',
            child: OutlinedButton.icon(
              style: OutlinedButton.styleFrom(minimumSize: const Size(0, 34)),
              onPressed: locked
                  ? null
                  : () => showRestoreBackup(context, instance, b),
              icon: const Icon(Symbols.settings_backup_restore_rounded, size: 18),
              label: Text(l.backupRestore),
            ),
          ),
          const SizedBox(width: Space.x1),
          MenuAnchor(
            alignmentOffset: const Offset(-140, 4),
            menuChildren: [
              MenuItemButton(
                leadingIcon: const Icon(Symbols.label_rounded, size: 18),
                onPressed: () async {
                  final label = await showTextPrompt(
                    context,
                    title: l.backupLabelTitle,
                    label: l.backupLabel,
                    initial: b.label ?? '',
                    action: l.save,
                    allowEmpty: true,
                  );
                  if (label != null && context.mounted) {
                    await _run(
                      context,
                      () => ctrl.setLabel(b.id, label.isEmpty ? null : label),
                    );
                  }
                },
                child: Text(l.backupLabelTitle),
              ),
              MenuItemButton(
                leadingIcon: Icon(Symbols.delete_rounded, size: 18, color: c.danger),
                onPressed: () async {
                  final ok = await confirm(
                    context,
                    title: l.backupDeleteTitle,
                    body: l.backupDeleteBody(backupTitle(l, b)),
                    action: l.delete,
                    danger: true,
                  );
                  if (ok && context.mounted) {
                    await _run(context, () => ctrl.delete(b.id));
                  }
                },
                child: Text(l.delete, style: TextStyle(color: c.danger)),
              ),
            ],
            builder: (context, controller, _) => IconButton(
              tooltip: l.instanceMore,
              onPressed: () =>
                  controller.isOpen ? controller.close() : controller.open(),
              icon: const Icon(Symbols.more_vert_rounded, size: 19),
            ),
          ),
        ],
      ),
    );
  }
}

class _AutoSettings extends ConsumerStatefulWidget {
  const _AutoSettings({required this.instanceId});

  final String instanceId;

  @override
  ConsumerState<_AutoSettings> createState() => _AutoSettingsState();
}

class _AutoSettingsState extends ConsumerState<_AutoSettings> {
  BackupSettingsView? _draft;
  Timer? _debounce;

  @override
  void dispose() {
    _debounce?.cancel();
    if (_draft != null) unawaited(_flush());
    super.dispose();
  }

  Future<void> _flush() async {
    final d = _draft;
    _draft = null;
    if (d == null) return;
    try {
      await ref.read(backupSettingsProvider(widget.instanceId).notifier).save(d);
    } catch (e) {
      if (mounted) {
        showToast(context, errorText(AppLocalizations.of(context), e), error: true);
      }
    }
  }

  void _change(BackupSettingsView v, {bool debounce = false}) {
    setState(() => _draft = v);
    _debounce?.cancel();
    _debounce = Timer(Duration(milliseconds: debounce ? 450 : 0), _flush);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final saved = ref.watch(backupSettingsProvider(widget.instanceId)).value;
    final s = _draft ?? saved;
    if (s == null) return const SizedBox.shrink();
    const intervals = [6, 12, 24, 72, 168];

    BackupSettingsView next({bool? auto, int? hours, int? max}) =>
        BackupSettingsView(
          auto: auto ?? s.auto,
          intervalHours: hours ?? s.intervalHours,
          maxCount: max ?? s.maxCount,
        );

    return Panel(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(Symbols.autorenew_rounded, size: 18, color: c.accent),
              const SizedBox(width: Space.x2),
              Expanded(child: Text(l.backupAutoTitle, style: context.text.titleSmall)),
              Switch(
                value: s.auto,
                onChanged: (v) => _change(next(auto: v)),
              ),
            ],
          ),
          const SizedBox(height: Space.x1),
          Text(l.backupAutoDesc, style: context.text.bodySmall),
          AnimatedOpacity(
            duration: Motion.fast,
            opacity: s.auto ? 1 : 0.45,
            child: IgnorePointer(
              ignoring: !s.auto,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const SizedBox(height: Space.x5),
                  Text(l.backupInterval, style: context.text.labelLarge),
                  const SizedBox(height: Space.x2),
                  PillTabs<int>(
                    items: [
                      for (final h in {...intervals, s.intervalHours})
                        (
                          value: h,
                          label: _intervalLabel(l, h),
                          count: null,
                          icon: null,
                        ),
                    ],
                    selected: s.intervalHours,
                    onSelect: (h) => _change(next(hours: h)),
                  ),
                  const SizedBox(height: Space.x5),
                  Row(
                    children: [
                      Expanded(
                        child: Text(l.backupKeep, style: context.text.labelLarge),
                      ),
                      Text(
                        l.backupKeepCount(s.maxCount),
                        style: context.text.titleSmall!.copyWith(color: c.accent),
                      ),
                    ],
                  ),
                  Slider(
                    min: 1,
                    max: 20,
                    divisions: 19,
                    value: s.maxCount.clamp(1, 20).toDouble(),
                    label: '${s.maxCount}',
                    onChanged: (v) => _change(next(max: v.round()), debounce: true),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: Space.x2),
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Symbols.push_pin_rounded, size: 16, color: c.textMuted),
              const SizedBox(width: Space.x2),
              Expanded(child: Text(l.backupPinnedNote, style: context.text.bodySmall)),
            ],
          ),
        ],
      ),
    );
  }

  String _intervalLabel(AppLocalizations l, int h) => switch (h) {
        24 => l.backupDaily,
        168 => l.backupWeekly,
        72 => l.backupEveryDays(3),
        _ => l.backupEveryHours(h),
      };
}

// ---------------------------------------------------------------------------
// Diálogos
// ---------------------------------------------------------------------------

/// Barra de progreso de la tarea en curso de una etapa (`backup`, `restore`).
class StageProgress extends ConsumerWidget {
  const StageProgress({super.key, required this.stages, required this.label});

  final List<String> stages;
  final String label;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    TaskProgress? t;
    for (final s in stages) {
      t ??= ref.watch(stageProgressProvider(s));
    }
    final f = t?.fraction;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(child: Text(label, style: context.text.bodyMedium)),
            if (t != null && t.total > 0)
              Text(
                t.bytes
                    ? '${formatBytes(l, BigInt.from(t.done))} / ${formatBytes(l, BigInt.from(t.total))}'
                    : '${t.done} / ${t.total}',
                style: context.text.bodySmall!.copyWith(color: c.textMuted),
              ),
          ],
        ),
        const SizedBox(height: Space.x2),
        ClipRRect(
          borderRadius: BorderRadius.circular(4),
          child: LinearProgressIndicator(value: f, minHeight: 6),
        ),
      ],
    );
  }
}

Future<void> showCreateBackup(BuildContext context, String instanceId) =>
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => _CreateBackupDialog(instanceId: instanceId),
    );

class _CreateBackupDialog extends ConsumerStatefulWidget {
  const _CreateBackupDialog({required this.instanceId});

  final String instanceId;

  @override
  ConsumerState<_CreateBackupDialog> createState() => _CreateBackupDialogState();
}

class _CreateBackupDialogState extends ConsumerState<_CreateBackupDialog> {
  final _label = TextEditingController();
  bool _pinned = false;
  bool _busy = false;
  Object? _error;

  @override
  void dispose() {
    _label.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    final l = AppLocalizations.of(context);
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final label = _label.text.trim();
      await ref.read(backupsProvider(widget.instanceId).notifier).create(
            label: label.isEmpty ? null : label,
            pinned: _pinned,
          );
      if (!mounted) return;
      Navigator.pop(context);
      showToast(context, l.backupCreated);
    } catch (e) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = e;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return AppDialog(
      title: l.backupCreateTitle,
      icon: Symbols.cloud_upload_rounded,
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : _create,
          child: Text(l.backupCreate),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.backupCreateBody),
          const SizedBox(height: Space.x4),
          TextField(
            controller: _label,
            enabled: !_busy,
            autofocus: true,
            onSubmitted: (_) => _busy ? null : _create(),
            decoration: InputDecoration(
              labelText: l.backupLabelOptional,
              hintText: l.backupLabelHint,
            ),
          ),
          const SizedBox(height: Space.x2),
          CheckboxListTile(
            value: _pinned,
            onChanged: _busy ? null : (v) => setState(() => _pinned = v ?? false),
            contentPadding: EdgeInsets.zero,
            controlAffinity: ListTileControlAffinity.leading,
            title: Text(l.backupPinOnCreate, style: context.text.bodyMedium),
            subtitle: Text(l.backupPinnedNote, style: context.text.bodySmall),
          ),
          if (_busy) ...[
            const SizedBox(height: Space.x3),
            StageProgress(stages: const ['backup'], label: l.backupCreating),
          ],
          if (_error != null) ...[
            const SizedBox(height: Space.x3),
            ErrorNote(_error!),
          ],
        ],
      ),
    );
  }
}

Future<void> showRestoreBackup(
  BuildContext context,
  InstanceView instance,
  BackupView backup,
) =>
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => RestoreBackupDialog(instance: instance, backup: backup),
    );

class RestoreBackupDialog extends ConsumerStatefulWidget {
  const RestoreBackupDialog({
    super.key,
    required this.instance,
    required this.backup,
  });

  final InstanceView instance;
  final BackupView backup;

  @override
  ConsumerState<RestoreBackupDialog> createState() => _RestoreBackupDialogState();
}

class _RestoreBackupDialogState extends ConsumerState<RestoreBackupDialog> {
  bool _worldsOnly = false;
  bool _safety = true;

  /// `null` = sin empezar; `backup` = copia previa; `restore` = restaurando.
  String? _step;
  Object? _error;

  Future<void> _restore() async {
    final l = AppLocalizations.of(context);
    final ctrl = ref.read(backupsProvider(widget.instance.id).notifier);
    setState(() => _error = null);
    try {
      if (_safety) {
        setState(() => _step = 'backup');
        await ctrl.create(label: l.backupSafetyLabel);
      }
      setState(() => _step = 'restore');
      await ctrl.restore(widget.backup.id, worldsOnly: _worldsOnly);
      if (!mounted) return;
      Navigator.pop(context);
      showToast(context, l.backupRestored(backupTitle(l, widget.backup)));
    } catch (e) {
      if (mounted) {
        setState(() {
          _step = null;
          _error = e;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final b = widget.backup;
    final busy = _step != null;
    return AppDialog(
      title: l.backupRestoreTitle,
      icon: Symbols.settings_backup_restore_rounded,
      width: 520,
      actions: [
        TextButton(
          onPressed: busy ? null : () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: c.danger,
            foregroundColor: const Color(0xFF2B0A0A),
          ),
          onPressed: busy ? null : _restore,
          child: Text(_worldsOnly ? l.backupRestoreWorlds : l.backupRestoreFull),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l.backupRestoreFrom(backupTitle(l, b), formatDateTime(l, b.createdAt)),
          ),
          const SizedBox(height: Space.x4),
          _ModeOption(
            selected: !_worldsOnly,
            icon: Symbols.select_all_rounded,
            title: l.backupModeFull,
            body: l.backupModeFullDesc,
            onTap: busy ? null : () => setState(() => _worldsOnly = false),
          ),
          const SizedBox(height: Space.x2),
          _ModeOption(
            selected: _worldsOnly,
            icon: Symbols.public_rounded,
            title: l.backupModeWorlds,
            body: l.backupModeWorldsDesc,
            onTap: busy ? null : () => setState(() => _worldsOnly = true),
          ),
          const SizedBox(height: Space.x4),
          Container(
            padding: const EdgeInsets.all(Space.x3),
            decoration: BoxDecoration(
              color: c.warning.withValues(alpha: 0.08),
              borderRadius: BorderRadius.circular(Radii.control),
              border: Border.all(color: c.warning.withValues(alpha: 0.35)),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(Symbols.warning_rounded, size: 18, color: c.warning),
                const SizedBox(width: Space.x2),
                Expanded(
                  child: Text(
                    _worldsOnly ? l.backupWarnWorlds : l.backupWarnFull,
                    style: context.text.bodyMedium!.copyWith(color: c.text),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: Space.x2),
          CheckboxListTile(
            value: _safety,
            onChanged: busy ? null : (v) => setState(() => _safety = v ?? true),
            contentPadding: EdgeInsets.zero,
            controlAffinity: ListTileControlAffinity.leading,
            title: Text(l.backupSafety, style: context.text.bodyMedium),
          ),
          if (busy) ...[
            const SizedBox(height: Space.x2),
            StageProgress(
              stages: [_step!],
              label: _step == 'backup' ? l.backupCreating : l.backupRestoring,
            ),
          ],
          if (_error != null) ...[
            const SizedBox(height: Space.x3),
            ErrorNote(_error!),
          ],
        ],
      ),
    );
  }
}

class _ModeOption extends StatelessWidget {
  const _ModeOption({
    required this.selected,
    required this.icon,
    required this.title,
    required this.body,
    required this.onTap,
  });

  final bool selected;
  final IconData icon;
  final String title;
  final String body;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Material(
      color: selected ? c.accentContainer.withValues(alpha: 0.5) : c.surface1,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(
          color: selected ? c.accent.withValues(alpha: 0.6) : c.border,
          width: selected ? 1.5 : 1,
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(Space.x3),
          child: Row(
            children: [
              Icon(icon, size: 22, color: selected ? c.accent : c.textSecondary),
              const SizedBox(width: Space.x3),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: context.text.titleSmall),
                    const SizedBox(height: 2),
                    Text(body, style: context.text.bodySmall),
                  ],
                ),
              ),
              Icon(
                selected
                    ? Symbols.radio_button_checked_rounded
                    : Symbols.radio_button_unchecked_rounded,
                size: 20,
                color: selected ? c.accent : c.textMuted,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
