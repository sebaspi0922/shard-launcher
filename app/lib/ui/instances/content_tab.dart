// Pestaña «Contenido» de una instancia: mods, paquetes de recursos y shaders
// con activar/desactivar, quitar, actualizar e identificar.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

/// Filtro de la lista por tipo.
enum _Filter { all, mod, resourcepack, shader }

/// Icono propio de cada tipo de contenido.
IconData contentKindIcon(String kind) => switch (kind) {
      'mod' => Symbols.extension_rounded,
      'resourcepack' => Symbols.palette_rounded,
      'shader' => Symbols.wb_sunny_rounded,
      'datapack' => Symbols.data_object_rounded,
      'modpack' => Symbols.inventory_2_rounded,
      _ => Symbols.draft_rounded,
    };

/// Color de acento de cada tipo de contenido.
Color contentKindColor(String kind) => switch (kind) {
      'mod' => const Color(0xFF7DD3FC),
      'resourcepack' => const Color(0xFFF0ABFC),
      'shader' => const Color(0xFFFCD34D),
      'datapack' => const Color(0xFF86EFAC),
      'modpack' => const Color(0xFFFDA4AF),
      _ => const Color(0xFFA0A0A0),
    };

/// Nombre visible de un tipo de contenido (plural).
String contentKindLabel(AppLocalizations l, String kind) => switch (kind) {
      'mod' => l.kindMods,
      'resourcepack' => l.kindResourcepacks,
      'shader' => l.kindShaders,
      'datapack' => l.kindDatapacks,
      'modpack' => l.kindModpacks,
      _ => kind,
    };

class ContentTab extends ConsumerStatefulWidget {
  const ContentTab({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<ContentTab> createState() => _ContentTabState();
}

class _ContentTabState extends ConsumerState<ContentTab> {
  _Filter _filter = _Filter.all;
  final _search = TextEditingController();
  bool _updatingAll = false;
  bool _identifying = false;
  final Set<String> _updating = {};

  String get _id => widget.instance.id;

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  bool _matches(InstalledContentView c) {
    if (c.kind == 'datapack') return false;
    if (_filter != _Filter.all && c.kind != _filter.name) return false;
    final q = _search.text.trim().toLowerCase();
    if (q.isEmpty) return true;
    return '${c.projectName ?? ''} ${c.file}'.toLowerCase().contains(q);
  }

  Future<void> _run(Future<void> Function() action) async {
    try {
      await action();
    } catch (e) {
      if (mounted) {
        showToast(context, errorText(AppLocalizations.of(context), e), error: true);
      }
    }
  }

  Future<void> _updateOne(ContentUpdateView u) async {
    final l = AppLocalizations.of(context);
    final key = contentKey(u.kind, u.file);
    setState(() => _updating.add(key));
    await _run(() async {
      final v = await ref
          .read(instanceContentProvider(_id).notifier)
          .updateOne(u.kind, u.file);
      if (mounted && v != null) {
        showToast(context, l.contentUpdated(u.projectName ?? u.file, v));
      }
    });
    if (mounted) setState(() => _updating.remove(key));
  }

  Future<void> _updateAll(List<ContentUpdateView> updates) async {
    final l = AppLocalizations.of(context);
    setState(() => _updatingAll = true);
    final (ok, errors) = await ref
        .read(instanceContentProvider(_id).notifier)
        .updateAll(updates);
    if (!mounted) return;
    setState(() => _updatingAll = false);
    if (errors.isEmpty) {
      showToast(context, l.contentUpdatedAll(ok));
    } else {
      showToast(context, l.contentUpdatedSome(ok, errors.length), error: true);
    }
  }

  Future<void> _identify() async {
    final l = AppLocalizations.of(context);
    setState(() => _identifying = true);
    await _run(() async {
      final r = await ref.read(instanceContentProvider(_id).notifier).identify();
      if (mounted) {
        showToast(
          context,
          l.contentIdentified(r.identified.length, r.unmatched.length),
        );
      }
    });
    if (mounted) setState(() => _identifying = false);
  }

  Future<void> _remove(InstalledContentView item) async {
    final l = AppLocalizations.of(context);
    final name = item.projectName ?? item.file;
    final ok = await confirm(
      context,
      title: l.contentRemoveTitle(name),
      body: l.contentRemoveBody,
      action: l.remove,
      danger: true,
    );
    if (!ok || !mounted) return;
    await _run(() async {
      await ref.read(instanceContentProvider(_id).notifier).remove(item);
      if (mounted) showToast(context, l.contentRemoved(name));
    });
  }

  void _add() {
    final kind = _filter == _Filter.all ? 'mod' : _filter.name;
    context.go('/content?instance=$_id&kind=$kind');
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(instanceContentProvider(_id));
    final updatesAsync = ref.watch(contentUpdatesProvider(_id));
    final updates = {
      for (final u in updatesAsync.value ?? const <ContentUpdateView>[])
        contentKey(u.kind, u.file): u,
    };

    final all = (async.value ?? const <InstalledContentView>[])
        .where((x) => x.kind != 'datapack')
        .toList();
    int count(_Filter f) =>
        f == _Filter.all ? all.length : all.where((x) => x.kind == f.name).length;
    final visible = all.where(_matches).toList();
    final unknown = all.where((x) => x.source == 'unknown').length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            PillTabs<_Filter>(
              items: [
                for (final f in _Filter.values)
                  (
                    value: f,
                    label: f == _Filter.all ? l.filterAll : contentKindLabel(l, f.name),
                    count: count(f),
                    icon: null,
                  ),
              ],
              selected: _filter,
              onSelect: (f) => setState(() => _filter = f),
            ),
            const Spacer(),
            SizedBox(
              width: 230,
              child: TextField(
                controller: _search,
                onChanged: (_) => setState(() {}),
                decoration: InputDecoration(
                  hintText: l.contentFilterHint,
                  prefixIcon: const Icon(Symbols.search_rounded, size: 18),
                ),
              ),
            ),
            const SizedBox(width: Space.x2),
            IconButton(
              tooltip: l.contentCheckUpdates,
              onPressed: updatesAsync.isLoading
                  ? null
                  : () => ref.invalidate(contentUpdatesProvider(_id)),
              icon: updatesAsync.isLoading
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Symbols.refresh_rounded, size: 20),
            ),
            const SizedBox(width: Space.x2),
            FilledButton.icon(
              onPressed: _add,
              icon: const Icon(Symbols.add_rounded, size: 20),
              label: Text(l.contentAdd),
            ),
          ],
        ),
        if (updates.isNotEmpty || unknown > 0) ...[
          const SizedBox(height: Space.x3),
          Row(
            children: [
              if (updates.isNotEmpty)
                Expanded(
                  child: _Banner(
                    icon: Symbols.upgrade_rounded,
                    color: c.accent,
                    text: l.contentUpdatesAvailable(updates.length),
                    action: _updatingAll ? null : () => _updateAll(updates.values.toList()),
                    actionLabel: l.contentUpdateAll,
                    busy: _updatingAll,
                  ),
                ),
              if (updates.isNotEmpty && unknown > 0) const SizedBox(width: Space.x3),
              if (unknown > 0)
                Expanded(
                  child: _Banner(
                    icon: Symbols.help_rounded,
                    color: c.warning,
                    text: l.contentUnknownFiles(unknown),
                    action: _identifying ? null : _identify,
                    actionLabel: l.contentIdentify,
                    busy: _identifying,
                  ),
                ),
            ],
          ),
        ],
        const SizedBox(height: Space.x3),
        Expanded(
          child: switch (async) {
            AsyncError(:final error) when all.isEmpty =>
              Center(child: ErrorNote(error)),
            AsyncLoading() when all.isEmpty =>
              const Center(child: CircularProgressIndicator()),
            _ when all.isEmpty => EmptyState(
                icon: Symbols.extension_rounded,
                title: l.contentEmptyTitle,
                body: l.contentEmptyBody,
                action: FilledButton.icon(
                  onPressed: _add,
                  icon: const Icon(Symbols.travel_explore_rounded, size: 20),
                  label: Text(l.contentBrowse),
                ),
              ),
            _ when visible.isEmpty => Center(
                child: Text(
                  l.contentNoMatches,
                  style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                ),
              ),
            _ => Align(
                alignment: Alignment.topCenter,
                child: Panel(
                  padding: EdgeInsets.zero,
                  child: ListView.separated(
                    shrinkWrap: true,
                    padding: const EdgeInsets.symmetric(vertical: Space.x1),
                    itemCount: visible.length,
                    separatorBuilder: (_, _) => Divider(
                      indent: Space.x4,
                      endIndent: Space.x4,
                      color: c.border,
                    ),
                    itemBuilder: (context, i) {
                      final item = visible[i];
                      final key = contentKey(item.kind, item.file);
                      final u = updates[key];
                      return _ContentRow(
                        item: item,
                        update: u,
                        updating: _updating.contains(key) || _updatingAll,
                        onToggle: (v) => _run(
                          () => ref
                              .read(instanceContentProvider(_id).notifier)
                              .setEnabled(item, v),
                        ),
                        onUpdate: u == null ? null : () => _updateOne(u),
                        onRemove: () => _remove(item),
                      );
                    },
                  ),
                ),
              ),
          },
        ),
      ],
    );
  }
}

class _Banner extends StatelessWidget {
  const _Banner({
    required this.icon,
    required this.color,
    required this.text,
    required this.action,
    required this.actionLabel,
    this.busy = false,
  });

  final IconData icon;
  final Color color;
  final String text;
  final VoidCallback? action;
  final String actionLabel;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.fromLTRB(Space.x4, Space.x2, Space.x2, Space.x2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: color.withValues(alpha: 0.28)),
      ),
      child: Row(
        children: [
          Icon(icon, size: 18, color: color),
          const SizedBox(width: Space.x3),
          Expanded(child: Text(text, style: context.text.bodyMedium)),
          TextButton(
            style: TextButton.styleFrom(foregroundColor: color),
            onPressed: action,
            child: busy
                ? SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2, color: color),
                  )
                : Text(actionLabel),
          ),
        ],
      ),
    );
  }
}

class _ContentRow extends StatelessWidget {
  const _ContentRow({
    required this.item,
    required this.update,
    required this.updating,
    required this.onToggle,
    required this.onUpdate,
    required this.onRemove,
  });

  final InstalledContentView item;
  final ContentUpdateView? update;
  final bool updating;
  final ValueChanged<bool> onToggle;
  final VoidCallback? onUpdate;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final kc = contentKindColor(item.kind);
    final name = item.projectName ?? item.file;
    final details = [
      if (item.projectName != null) item.file,
      if (item.versionNumber != null) item.versionNumber!,
      if (!item.isDir) formatBytes(l, item.size),
    ].join('  ·  ');
    final (sourceLabel, sourceColor) = switch (item.source) {
      'modrinth' => ('Modrinth', const Color(0xFF4ADE80)),
      'curseforge' => ('CurseForge', const Color(0xFFF59E0B)),
      _ => (l.sourceUnknown, c.textMuted),
    };
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: Space.x4, vertical: 10),
      child: Row(
        children: [
          AnimatedOpacity(
            duration: Motion.fast,
            opacity: item.enabled ? 1 : 0.45,
            child: Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: kc.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: kc.withValues(alpha: 0.25)),
              ),
              child: Icon(contentKindIcon(item.kind), size: 20, color: kc),
            ),
          ),
          const SizedBox(width: Space.x4),
          Expanded(
            child: AnimatedOpacity(
              duration: Motion.fast,
              opacity: item.enabled ? 1 : 0.55,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Flexible(
                        child: Text(
                          name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.text.titleSmall,
                        ),
                      ),
                      const SizedBox(width: Space.x2),
                      ColorBadge(sourceLabel, color: sourceColor),
                      if (!item.enabled) ...[
                        const SizedBox(width: Space.x1),
                        ColorBadge(l.contentDisabled, color: c.textMuted),
                      ],
                    ],
                  ),
                  const SizedBox(height: 2),
                  Text(
                    details,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.text.bodySmall!.copyWith(color: c.textMuted),
                  ),
                ],
              ),
            ),
          ),
          if (update != null) ...[
            const SizedBox(width: Space.x3),
            Tooltip(
              message: l.contentUpdateTo(update!.latestVersion),
              child: OutlinedButton.icon(
                style: OutlinedButton.styleFrom(
                  foregroundColor: c.accent,
                  side: BorderSide(color: c.accent.withValues(alpha: 0.4)),
                  backgroundColor: c.accent.withValues(alpha: 0.08),
                  minimumSize: const Size(0, 32),
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                ),
                onPressed: updating ? null : onUpdate,
                icon: updating
                    ? const SizedBox.square(
                        dimension: 14,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Symbols.arrow_upward_rounded, size: 16),
                label: Text(update!.latestVersion, maxLines: 1),
              ),
            ),
          ],
          const SizedBox(width: Space.x4),
          Tooltip(
            message: item.enabled ? l.contentDisable : l.contentEnable,
            child: Switch(value: item.enabled, onChanged: onToggle),
          ),
          const SizedBox(width: Space.x2),
          IconButton(
            tooltip: l.remove,
            onPressed: onRemove,
            icon: const Icon(Symbols.delete_rounded, size: 19),
          ),
        ],
      ),
    );
  }
}
