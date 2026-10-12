// Pestaña «Mundos»: lista con detalles y, a la derecha, el mundo elegido con
// su modo, dificultad, hardcore, trucos, datapacks y borrado.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

String gameModeLabel(AppLocalizations l, String? mode) => switch (mode) {
      'survival' => l.modeSurvival,
      'creative' => l.modeCreative,
      'adventure' => l.modeAdventure,
      'spectator' => l.modeSpectator,
      _ => l.unknownValue,
    };

String difficultyLabel(AppLocalizations l, String? d) => switch (d) {
      'peaceful' => l.diffPeaceful,
      'easy' => l.diffEasy,
      'normal' => l.diffNormal,
      'hard' => l.diffHard,
      _ => l.unknownValue,
    };

IconData gameModeIcon(String? mode) => switch (mode) {
      'creative' => Symbols.brush_rounded,
      'adventure' => Symbols.explore_rounded,
      'spectator' => Symbols.visibility_rounded,
      _ => Symbols.swords_rounded,
    };

/// Icono del mundo (`icon.png`) o un paisaje generado.
class WorldIcon extends StatelessWidget {
  const WorldIcon({super.key, required this.world, this.size = 52});

  final WorldView world;
  final double size;

  @override
  Widget build(BuildContext context) => Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(size * 0.2),
          border: Border.all(color: Colors.white.withValues(alpha: 0.14)),
        ),
        clipBehavior: Clip.antiAlias,
        child: LocalImage(
          path: world.icon,
          cacheWidth: (size * 2).round(),
          fallback: BlockLandscape(seed: world.folder, block: size / 8),
        ),
      );
}

class WorldsTab extends ConsumerStatefulWidget {
  const WorldsTab({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<WorldsTab> createState() => _WorldsTabState();
}

class _WorldsTabState extends ConsumerState<WorldsTab> {
  String? _selected;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final id = widget.instance.id;
    final async = ref.watch(worldsProvider(id));
    final worlds = async.value ?? const <WorldView>[];
    if (worlds.isEmpty) {
      return switch (async) {
        AsyncError(:final error) => Center(child: ErrorNote(error)),
        AsyncLoading() => const Center(child: CircularProgressIndicator()),
        _ => EmptyState(
            icon: Symbols.public_rounded,
            title: l.worldsEmptyTitle,
            body: l.worldsEmptyBody,
          ),
      };
    }
    final current = worlds.firstWhere(
      (w) => w.folder == _selected,
      orElse: () => worlds.first,
    );
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 380,
          child: ListView.separated(
            itemCount: worlds.length,
            separatorBuilder: (_, _) => const SizedBox(height: Space.x2),
            itemBuilder: (context, i) {
              final w = worlds[i];
              return _WorldCard(
                world: w,
                selected: w.folder == current.folder,
                onTap: () => setState(() => _selected = w.folder),
              );
            },
          ),
        ),
        const SizedBox(width: Space.x5),
        Expanded(
          child: _WorldDetail(
            key: ValueKey(current.folder),
            instance: widget.instance,
            world: current,
          ),
        ),
      ],
    );
  }
}

class _WorldCard extends StatelessWidget {
  const _WorldCard({
    required this.world,
    required this.selected,
    required this.onTap,
  });

  final WorldView world;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final w = world;
    final last = w.lastPlayedMs == null
        ? l.lastPlayedNever
        : formatRelative(l, w.lastPlayedMs! ~/ 1000);
    return Material(
      color: selected ? c.accentContainer.withValues(alpha: 0.45) : c.surface1,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(
          color: selected ? c.accent.withValues(alpha: 0.5) : c.border,
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(Space.x3),
          child: Row(
            children: [
              WorldIcon(world: w),
              const SizedBox(width: Space.x3),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            w.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.titleSmall,
                          ),
                        ),
                        if (w.hardcore == true) ...[
                          const SizedBox(width: Space.x2),
                          Icon(Symbols.favorite_rounded, fill: 1, size: 14, color: c.danger),
                        ],
                      ],
                    ),
                    const SizedBox(height: 2),
                    Text(
                      w.error != null
                          ? l.worldUnreadable
                          : '${gameModeLabel(l, w.gameMode)} · ${difficultyLabel(l, w.difficulty)}',
                      style: context.text.bodySmall!.copyWith(
                        color: w.error != null ? c.warning : null,
                      ),
                    ),
                    Text(
                      '$last · ${formatBytes(l, w.sizeBytes)}',
                      style: context.text.bodySmall!.copyWith(color: c.textMuted),
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

class _WorldDetail extends ConsumerWidget {
  const _WorldDetail({super.key, required this.instance, required this.world});

  final InstanceView instance;
  final WorldView world;

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
    final w = world;
    final ctrl = ref.read(worldsProvider(instance.id).notifier);
    final running =
        ref.watch(launchStateProvider(instance.id)).phase == LaunchPhase.running;
    final editable = !running && w.error == null;
    final folderPath = '${instance.gameDir}/saves/${w.folder}';

    return Panel(
      padding: EdgeInsets.zero,
      child: ListView(
        padding: const EdgeInsets.all(Space.x5),
        children: [
          Row(
            children: [
              WorldIcon(world: w, size: 72),
              const SizedBox(width: Space.x4),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            w.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.headlineSmall,
                          ),
                        ),
                        const SizedBox(width: Space.x1),
                        IconButton(
                          tooltip: l.worldRename,
                          onPressed: editable
                              ? () => _rename(context, ref)
                              : null,
                          icon: const Icon(Symbols.edit_rounded, size: 18),
                        ),
                      ],
                    ),
                    Wrap(
                      spacing: Space.x2,
                      runSpacing: Space.x1,
                      children: [
                        Tag(w.folder, icon: Symbols.folder_rounded),
                        if (w.versionName != null)
                          Tag(w.versionName!, icon: Symbols.deployed_code_rounded),
                        Tag(formatBytes(l, w.sizeBytes), icon: Symbols.hard_drive_rounded),
                      ],
                    ),
                  ],
                ),
              ),
              IconButton(
                tooltip: l.openFolder,
                onPressed: () => launchUrl(Uri.file(folderPath)),
                icon: const Icon(Symbols.folder_open_rounded, size: 20),
              ),
            ],
          ),
          if (running) ...[
            const SizedBox(height: Space.x4),
            _Note(icon: Symbols.lock_rounded, text: l.worldLockedRunning),
          ],
          if (w.error != null) ...[
            const SizedBox(height: Space.x4),
            ErrorNote(BridgeError(kind: 'io', message: w.error!)),
          ],
          const SizedBox(height: Space.x5),
          Text(l.worldGame, style: context.text.titleMedium),
          const SizedBox(height: Space.x1),
          SettingRow(
            title: l.worldMode,
            control: SegmentedButton<String>(
              showSelectedIcon: false,
              segments: [
                for (final m in ['survival', 'creative', 'adventure', 'spectator'])
                  ButtonSegment(
                    value: m,
                    icon: Icon(gameModeIcon(m), size: 16),
                    label: Text(gameModeLabel(l, m)),
                  ),
              ],
              selected: {if (w.gameMode != null) w.gameMode!},
              emptySelectionAllowed: true,
              onSelectionChanged: editable
                  ? (v) => _run(
                        context,
                        () => ctrl.patch(w.folder, gameMode: v.first),
                      )
                  : null,
            ),
          ),
          const Divider(),
          SettingRow(
            title: l.worldDifficulty,
            control: SegmentedButton<String>(
              showSelectedIcon: false,
              segments: [
                for (final d in ['peaceful', 'easy', 'normal', 'hard'])
                  ButtonSegment(value: d, label: Text(difficultyLabel(l, d))),
              ],
              selected: {if (w.difficulty != null) w.difficulty!},
              emptySelectionAllowed: true,
              onSelectionChanged: editable
                  ? (v) => _run(
                        context,
                        () => ctrl.patch(w.folder, difficulty: v.first),
                      )
                  : null,
            ),
          ),
          const Divider(),
          SettingRow(
            title: l.worldHardcore,
            description: l.worldHardcoreDesc,
            control: Switch(
              value: w.hardcore ?? false,
              onChanged: editable
                  ? (v) => _run(context, () => ctrl.patch(w.folder, hardcore: v))
                  : null,
            ),
          ),
          const Divider(),
          SettingRow(
            title: l.worldCheats,
            description: l.worldCheatsDesc,
            control: Switch(
              value: w.cheats ?? false,
              onChanged: editable
                  ? (v) => _run(context, () => ctrl.patch(w.folder, cheats: v))
                  : null,
            ),
          ),
          const SizedBox(height: Space.x6),
          _Datapacks(instance: instance, world: w, editable: !running),
          const SizedBox(height: Space.x6),
          Container(
            padding: const EdgeInsets.all(Space.x4),
            decoration: BoxDecoration(
              color: c.danger.withValues(alpha: 0.06),
              borderRadius: BorderRadius.circular(Radii.card),
              border: Border.all(color: c.danger.withValues(alpha: 0.25)),
            ),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(l.worldDelete, style: context.text.titleSmall),
                      const SizedBox(height: 2),
                      Text(l.worldDeleteDesc, style: context.text.bodySmall),
                    ],
                  ),
                ),
                const SizedBox(width: Space.x4),
                OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(
                    foregroundColor: c.danger,
                    side: BorderSide(color: c.danger.withValues(alpha: 0.5)),
                  ),
                  onPressed: running ? null : () => _delete(context, ref),
                  icon: const Icon(Symbols.delete_forever_rounded, size: 18),
                  label: Text(l.delete),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _rename(BuildContext context, WidgetRef ref) async {
    final l = AppLocalizations.of(context);
    final name = await showTextPrompt(
      context,
      title: l.worldRename,
      label: l.worldName,
      initial: world.name,
      action: l.save,
    );
    if (name == null || name.trim().isEmpty || !context.mounted) return;
    await _run(
      context,
      () => ref.read(worldsProvider(instance.id).notifier).rename(world.folder, name),
    );
  }

  Future<void> _delete(BuildContext context, WidgetRef ref) async {
    final l = AppLocalizations.of(context);
    final ok = await confirm(
      context,
      title: l.worldDeleteTitle(world.name),
      body: l.worldDeleteBody,
      action: l.worldDeleteAction,
      danger: true,
    );
    if (!ok || !context.mounted) return;
    await _run(context, () async {
      await ref.read(worldsProvider(instance.id).notifier).delete(world.folder);
      if (context.mounted) showToast(context, l.worldDeleted(world.name));
    });
  }
}

class _Datapacks extends ConsumerWidget {
  const _Datapacks({
    required this.instance,
    required this.world,
    required this.editable,
  });

  final InstanceView instance;
  final WorldView world;
  final bool editable;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final key = (instance: instance.id, world: world.folder);
    final async = ref.watch(datapacksProvider(key));
    final packs = async.value ?? const <DatapackView>[];
    final bridge = ref.read(bridgeProvider);

    Future<void> run(Future<void> Function() f) async {
      try {
        await f();
      } catch (e) {
        if (context.mounted) showToast(context, errorText(l, e), error: true);
      }
      ref.invalidate(datapacksProvider(key));
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(l.kindDatapacks, style: context.text.titleMedium),
            const SizedBox(width: Space.x2),
            if (packs.isNotEmpty) Tag('${packs.length}'),
            const Spacer(),
            TextButton.icon(
              onPressed: () => context.go(
                Uri(
                  path: '/content',
                  queryParameters: {
                    'instance': instance.id,
                    'kind': 'datapack',
                    'world': world.folder,
                  },
                ).toString(),
              ),
              icon: const Icon(Symbols.add_rounded, size: 18),
              label: Text(l.datapacksAdd),
            ),
          ],
        ),
        const SizedBox(height: Space.x2),
        if (async.isLoading && packs.isEmpty)
          const Padding(
            padding: EdgeInsets.all(Space.x4),
            child: Center(child: CircularProgressIndicator()),
          )
        else if (packs.isEmpty)
          Container(
            padding: const EdgeInsets.all(Space.x4),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(Radii.card),
              border: Border.all(color: c.border),
            ),
            child: Text(
              l.datapacksEmpty,
              style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
            ),
          )
        else
          Container(
            decoration: BoxDecoration(
              color: c.card(c.surface1),
              borderRadius: BorderRadius.circular(Radii.card),
              border: Border.all(color: c.border),
            ),
            child: Column(
              children: [
                for (final (i, d) in packs.indexed) ...[
                  if (i > 0) Divider(indent: Space.x4, endIndent: Space.x4, color: c.border),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(Space.x4, 6, Space.x2, 6),
                    child: Row(
                      children: [
                        Icon(
                          d.isDir ? Symbols.folder_rounded : Symbols.data_object_rounded,
                          size: 18,
                          color: d.enabled ? const Color(0xFF86EFAC) : c.textMuted,
                        ),
                        const SizedBox(width: Space.x3),
                        Expanded(
                          child: Text(
                            d.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.bodyMedium!.copyWith(
                              color: d.enabled ? c.text : c.textMuted,
                            ),
                          ),
                        ),
                        if (!d.isDir)
                          Text(
                            formatBytes(l, d.sizeBytes),
                            style: context.text.bodySmall!.copyWith(color: c.textMuted),
                          ),
                        const SizedBox(width: Space.x3),
                        Switch(
                          value: d.enabled,
                          onChanged: editable
                              ? (v) => run(
                                    () => bridge.setDatapackEnabled(
                                      instance.id,
                                      world.folder,
                                      d.name,
                                      v,
                                    ),
                                  )
                              : null,
                        ),
                        IconButton(
                          tooltip: l.remove,
                          onPressed: editable
                              ? () async {
                                  final ok = await confirm(
                                    context,
                                    title: l.contentRemoveTitle(d.name),
                                    body: l.datapackRemoveBody,
                                    action: l.remove,
                                    danger: true,
                                  );
                                  if (ok) {
                                    await run(
                                      () => bridge.removeDatapack(
                                        instance.id,
                                        world.folder,
                                        d.name,
                                      ),
                                    );
                                  }
                                }
                              : null,
                          icon: const Icon(Symbols.delete_rounded, size: 18),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
      ],
    );
  }
}

class _Note extends StatelessWidget {
  const _Note({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Container(
      padding: const EdgeInsets.all(Space.x3),
      decoration: BoxDecoration(
        color: c.info.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(Radii.control),
        border: Border.all(color: c.info.withValues(alpha: 0.3)),
      ),
      child: Row(
        children: [
          Icon(icon, size: 18, color: c.info),
          const SizedBox(width: Space.x2),
          Expanded(child: Text(text, style: context.text.bodyMedium)),
        ],
      ),
    );
  }
}
