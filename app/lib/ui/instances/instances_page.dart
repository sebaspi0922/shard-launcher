// Biblioteca de instancias: rejilla o lista, búsqueda y creación.

import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/play_button.dart';
import 'create_instance_dialog.dart';
import 'trash_dialog.dart';

class InstancesViewMode extends Notifier<bool> {
  @override
  bool build() => true;

  void set(bool grid) => state = grid;
}

/// `true` = rejilla, `false` = lista.
final instancesGridProvider = NotifierProvider<InstancesViewMode, bool>(InstancesViewMode.new);

class InstancesPage extends ConsumerStatefulWidget {
  const InstancesPage({super.key});

  @override
  ConsumerState<InstancesPage> createState() => _InstancesPageState();
}

/// Spike GPUI (medición): con `SHARD_SPIKE_TIMING=1` imprime la hora del
/// primer fotograma con instancias pintadas.
bool _firstFrameLogged = false;
void _logFirstFrame() {
  if (_firstFrameLogged || Platform.environment['SHARD_SPIKE_TIMING'] != '1') return;
  _firstFrameLogged = true;
  WidgetsBinding.instance.addPostFrameCallback((_) {
    stdout.writeln('SHARD_FIRST_FRAME unix_ms=${DateTime.now().millisecondsSinceEpoch}');
  });
}

class _InstancesPageState extends ConsumerState<InstancesPage> {
  final _search = TextEditingController();

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  bool _matches(AppLocalizations l, InstanceView i, String q) {
    if (q.isEmpty) return true;
    final hay = '${i.name} ${i.mc} ${loaderName(l, i.loader)}'.toLowerCase();
    return q.split(RegExp(r'\s+')).every(hay.contains);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(instancesProvider);
    final all = ref.watch(recentInstancesProvider);
    final grid = ref.watch(instancesGridProvider);
    final q = _search.text.trim().toLowerCase();
    final list = all.where((i) => _matches(l, i, q)).toList();
    if (async is AsyncData && all.isNotEmpty) _logFirstFrame();

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(l.instancesTitle, style: context.text.headlineMedium),
                  const SizedBox(height: Space.x1),
                  Text(
                    l.instancesCount(all.length),
                    style: context.text.bodyMedium!.copyWith(
                      color: c.textSecondary,
                    ),
                  ),
                ],
              ),
              const Spacer(),
              SizedBox(
                width: 300,
                child: TextField(
                  controller: _search,
                  onChanged: (_) => setState(() {}),
                  decoration: InputDecoration(
                    hintText: l.instancesSearch,
                    prefixIcon: const Icon(Symbols.search_rounded, size: 18),
                    suffixIcon: q.isEmpty
                        ? null
                        : IconButton(
                            iconSize: 16,
                            onPressed: () => setState(_search.clear),
                            icon: const Icon(Symbols.close_rounded),
                          ),
                  ),
                ),
              ),
              const SizedBox(width: Space.x3),
              SegmentedButton<bool>(
                showSelectedIcon: false,
                segments: [
                  ButtonSegment(
                    value: true,
                    tooltip: l.viewGrid,
                    icon: const Icon(Symbols.grid_view_rounded, size: 18),
                  ),
                  ButtonSegment(
                    value: false,
                    tooltip: l.viewList,
                    icon: const Icon(Symbols.view_agenda_rounded, size: 18),
                  ),
                ],
                selected: {grid},
                onSelectionChanged: (s) =>
                    ref.read(instancesGridProvider.notifier).set(s.first),
              ),
              const SizedBox(width: Space.x3),
              IconButton.outlined(
                tooltip: l.trashTitle,
                onPressed: () => showTrash(context),
                icon: const Icon(Symbols.delete_rounded, size: 20),
              ),
              const SizedBox(width: Space.x3),
              FilledButton.icon(
                onPressed: () => showCreateInstance(context),
                icon: const Icon(Symbols.add_rounded, size: 20),
                label: Text(l.instancesNew),
              ),
            ],
          ),
          const SizedBox(height: Space.x6),
          Expanded(
            child: switch (async) {
              AsyncError(:final error) => Center(child: ErrorNote(error)),
              AsyncData() when all.isEmpty => EmptyState(
                  icon: Symbols.deployed_code_rounded,
                  title: l.instancesEmptyTitle,
                  body: l.instancesEmptyBody,
                  action: FilledButton.icon(
                    onPressed: () => showCreateInstance(context),
                    icon: const Icon(Symbols.add_rounded, size: 20),
                    label: Text(l.instancesNew),
                  ),
                ),
              AsyncData() when list.isEmpty => Center(
                  child: Text(
                    l.instancesNoResults(_search.text.trim()),
                    style: context.text.bodyLarge!.copyWith(
                      color: c.textSecondary,
                    ),
                  ),
                ),
              AsyncData() => grid
                  ? GridView.builder(
                      padding: const EdgeInsets.only(bottom: Space.x8),
                      gridDelegate:
                          const SliverGridDelegateWithMaxCrossAxisExtent(
                        maxCrossAxisExtent: 300,
                        mainAxisExtent: 232,
                        crossAxisSpacing: Space.x4,
                        mainAxisSpacing: Space.x4,
                      ),
                      itemCount: list.length,
                      itemBuilder: (_, i) => InstanceCard(instance: list[i]),
                    )
                  : ListView.separated(
                      padding: const EdgeInsets.only(bottom: Space.x8),
                      itemCount: list.length,
                      separatorBuilder: (_, _) =>
                          const SizedBox(height: Space.x2),
                      itemBuilder: (_, i) => InstanceRow(instance: list[i]),
                    ),
              _ => const Center(child: CircularProgressIndicator()),
            },
          ),
        ],
      ),
    );
  }
}

/// Tarjeta de la rejilla.
class InstanceCard extends ConsumerStatefulWidget {
  const InstanceCard({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<InstanceCard> createState() => _InstanceCardState();
}

class _InstanceCardState extends ConsumerState<InstanceCard> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final i = widget.instance;
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final launch = ref.watch(launchStateProvider(i.id));
    final showPlay = _hover || launch.phase != LaunchPhase.idle;

    return MouseRegion(
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: AnimatedContainer(
        duration: Motion.fast,
        decoration: BoxDecoration(
          color: c.card(_hover ? c.surface2 : c.surface1),
          borderRadius: BorderRadius.circular(Radii.card + 2),
          border: Border.all(color: _hover ? c.borderStrong : c.border),
        ),
        child: Material(
          type: MaterialType.transparency,
          borderRadius: BorderRadius.circular(Radii.card + 2),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: () => context.go('/instances/${i.id}'),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SizedBox(
                  height: 104,
                  child: Stack(
                    clipBehavior: Clip.none,
                    fit: StackFit.expand,
                    children: [
                      BlockLandscape(seed: i.id, block: 12),
                      Positioned(
                        top: Space.x3,
                        right: Space.x3,
                        child: AnimatedOpacity(
                          duration: Motion.fast,
                          opacity: showPlay ? 1 : 0,
                          child: PlayButton(
                            instanceId: i.id,
                            size: PlayButtonSize.small,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                Transform.translate(
                  offset: const Offset(Space.x4, -24),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: InstanceIcon(id: i.id, name: i.name, size: 48),
                  ),
                ),
                Expanded(
                  child: Transform.translate(
                    offset: const Offset(0, -16),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: Space.x4),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            i.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.titleMedium,
                          ),
                          const SizedBox(height: 3),
                          Row(
                            children: [
                              Container(
                                width: 7,
                                height: 7,
                                decoration: BoxDecoration(
                                  color: loaderColor(i.loader),
                                  borderRadius: BorderRadius.circular(2),
                                ),
                              ),
                              const SizedBox(width: 6),
                              Text(
                                versionLine(l, i.mc, i.loader),
                                style: context.text.bodySmall,
                              ),
                            ],
                          ),
                          const Spacer(),
                          Row(
                            children: [
                              Icon(Symbols.schedule_rounded, size: 14, color: c.textMuted),
                              const SizedBox(width: 4),
                              Text(
                                formatPlaytime(l, i.playtimeSeconds),
                                style: context.text.bodySmall!.copyWith(
                                  color: c.textMuted,
                                ),
                              ),
                              const Spacer(),
                              Text(
                                formatRelative(l, i.lastPlayed),
                                style: context.text.bodySmall!.copyWith(
                                  color: c.textMuted,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
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

/// Fila de la vista de lista.
class InstanceRow extends ConsumerWidget {
  const InstanceRow({super.key, required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final i = instance;
    final l = AppLocalizations.of(context);
    final c = context.colors;
    TextStyle muted = context.text.bodySmall!.copyWith(color: c.textMuted);
    return Material(
      color: c.card(c.surface1),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(color: c.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => context.go('/instances/${i.id}'),
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.x4,
            vertical: Space.x3,
          ),
          child: Row(
            children: [
              InstanceIcon(id: i.id, name: i.name, size: 40),
              const SizedBox(width: Space.x4),
              Expanded(
                flex: 3,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(i.name, style: context.text.titleSmall),
                    const SizedBox(height: 2),
                    Text(
                      versionLine(l, i.mc, i.loader),
                      style: context.text.bodySmall,
                    ),
                  ],
                ),
              ),
              Expanded(
                flex: 2,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(l.instancePlaytime, style: muted),
                    Text(
                      formatPlaytime(l, i.playtimeSeconds),
                      style: context.text.bodyMedium,
                    ),
                  ],
                ),
              ),
              Expanded(
                flex: 2,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(l.instanceLastPlayed, style: muted),
                    Text(
                      formatRelative(l, i.lastPlayed),
                      style: context.text.bodyMedium,
                    ),
                  ],
                ),
              ),
              PlayButton(instanceId: i.id, size: PlayButtonSize.small),
            ],
          ),
        ),
      ),
    );
  }
}
