// Contenido: buscador de Modrinth y CurseForge con filtros, scroll infinito,
// ficha del proyecto e instalación en una instancia (o modpack nuevo).

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/content_search.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../instances/content_tab.dart' show contentKindIcon, contentKindLabel;
import '../settings/curseforge_key_dialog.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';
import '../widgets/select_button.dart';
import 'install.dart';
import 'modpack_dialog.dart';
import 'project_sheet.dart';

String sortLabel(AppLocalizations l, String s) => switch (s) {
      'downloads' => l.sortDownloads,
      'follows' => l.sortFollows,
      'newest' => l.sortNewest,
      'updated' => l.sortUpdated,
      _ => l.sortRelevance,
    };

/// Destino de la instalación elegido al entrar desde una instancia.
class ContentTarget {
  const ContentTarget({required this.instanceId, this.world});

  final String instanceId;
  final String? world;
}

class ContentPage extends ConsumerStatefulWidget {
  const ContentPage({super.key, this.instanceId, this.kind, this.world});

  final String? instanceId;
  final String? kind;
  final String? world;

  @override
  ConsumerState<ContentPage> createState() => _ContentPageState();
}

class _ContentPageState extends ConsumerState<ContentPage> {
  final _search = TextEditingController();
  final _scroll = ScrollController();

  @override
  void initState() {
    super.initState();
    _search.text = ref.read(contentSearchProvider).filters.text;
    _scroll.addListener(_onScroll);
    WidgetsBinding.instance.addPostFrameCallback((_) => _configure());
  }

  @override
  void didUpdateWidget(ContentPage old) {
    super.didUpdateWidget(old);
    if (old.instanceId != widget.instanceId ||
        old.kind != widget.kind ||
        old.world != widget.world) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _configure());
    }
  }

  @override
  void dispose() {
    _search.dispose();
    _scroll.dispose();
    super.dispose();
  }

  /// Ajusta los filtros a la instancia de destino (versión, loader y tipo).
  void _configure() {
    if (!mounted) return;
    final ctrl = ref.read(contentSearchProvider.notifier);
    final current = ref.read(contentSearchProvider).filters;
    final kind = kContentKinds.contains(widget.kind) ? widget.kind! : null;
    final target = widget.instanceId == null
        ? null
        : ref.read(instanceByIdProvider(widget.instanceId!));
    if (target == null && kind == null) {
      ctrl.ensureLoaded();
      return;
    }
    ctrl.setFilters(
      current.copyWith(
        kind: kind,
        mc: target == null ? null : () => target.mc,
        loader: target == null
            ? null
            : () => kFilterLoaders.contains(target.loader) ? target.loader : null,
      ),
    );
  }

  void _onScroll() {
    final p = _scroll.position;
    if (p.extentAfter < 600) {
      unawaited(ref.read(contentSearchProvider.notifier).loadMore());
    }
  }

  void _set(SearchFilters f, {bool debounce = false}) {
    if (_scroll.hasClients) _scroll.jumpTo(0);
    ref.read(contentSearchProvider.notifier).setFilters(f, debounce: debounce);
  }

  ContentTarget? get _target => widget.instanceId == null
      ? null
      : ContentTarget(instanceId: widget.instanceId!, world: widget.world);

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final state = ref.watch(contentSearchProvider);
    final f = state.filters;
    final versions = ref.watch(releaseVersionsProvider).value ?? const <String>[];
    final target =
        widget.instanceId == null ? null : ref.watch(instanceByIdProvider(widget.instanceId!));
    final cfKey = ref.watch(curseforgeKeyProvider).value ?? false;
    final needsKey = f.source == 'curseforge' && !cfKey;

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(l.contentTitle, style: context.text.headlineMedium),
                    const SizedBox(height: Space.x1),
                    Text(
                      l.contentSubtitle,
                      style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                    ),
                  ],
                ),
              ),
              SegmentedButton<String>(
                showSelectedIcon: false,
                segments: const [
                  ButtonSegment(value: 'modrinth', label: Text('Modrinth')),
                  ButtonSegment(value: 'curseforge', label: Text('CurseForge')),
                ],
                selected: {f.source},
                onSelectionChanged: (v) => _set(f.copyWith(source: v.first)),
              ),
              const SizedBox(width: Space.x3),
              OutlinedButton.icon(
                onPressed: () => showModpackInstall(context),
                icon: const Icon(Symbols.upload_file_rounded, size: 18),
                label: Text(l.modpackImport),
              ),
            ],
          ),
          if (target != null) ...[
            const SizedBox(height: Space.x4),
            _TargetBanner(instance: target, world: widget.world),
          ],
          const SizedBox(height: Space.x4),
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _search,
                  onChanged: (v) => _set(f.copyWith(text: v), debounce: true),
                  decoration: InputDecoration(
                    hintText: l.contentSearchHint(contentKindLabel(l, f.kind).toLowerCase()),
                    prefixIcon: const Icon(Symbols.search_rounded, size: 18),
                    suffixIcon: _search.text.isEmpty
                        ? null
                        : IconButton(
                            iconSize: 16,
                            onPressed: () {
                              _search.clear();
                              _set(f.copyWith(text: ''));
                            },
                            icon: const Icon(Symbols.close_rounded),
                          ),
                  ),
                ),
              ),
              const SizedBox(width: Space.x3),
              SelectButton<String?>(
                icon: Symbols.deployed_code_rounded,
                prefix: l.filterVersion,
                value: f.mc,
                options: [
                  SelectOption(null, l.filterAny),
                  if (f.mc != null && !versions.contains(f.mc)) SelectOption(f.mc, f.mc!),
                  for (final v in versions) SelectOption(v, v),
                ],
                onChanged: (v) => _set(f.copyWith(mc: () => v)),
              ),
              if (kindUsesLoader(f.kind)) ...[
                const SizedBox(width: Space.x2),
                SelectButton<String?>(
                  icon: Symbols.memory_rounded,
                  prefix: l.filterLoader,
                  value: f.loader,
                  options: [
                    SelectOption(null, l.filterAny),
                    for (final lo in kFilterLoaders)
                      SelectOption(lo, loaderName(l, lo)),
                  ],
                  onChanged: (v) => _set(f.copyWith(loader: () => v)),
                ),
              ],
              const SizedBox(width: Space.x2),
              SelectButton<String>(
                icon: Symbols.sort_rounded,
                value: f.sort,
                options: [
                  for (final s in kSortOrders) SelectOption(s, sortLabel(l, s)),
                ],
                onChanged: (v) => _set(f.copyWith(sort: v)),
              ),
            ],
          ),
          const SizedBox(height: Space.x3),
          Row(
            children: [
              PillTabs<String>(
                items: [
                  for (final k in kContentKinds)
                    (
                      value: k,
                      label: contentKindLabel(l, k),
                      count: null,
                      icon: contentKindIcon(k),
                    ),
                ],
                selected: f.kind,
                onSelect: (k) => _set(f.copyWith(kind: k)),
              ),
              const Spacer(),
              if (state.started && state.error == null && !needsKey)
                Text(
                  l.contentResults(state.total),
                  style: context.text.bodySmall!.copyWith(color: c.textMuted),
                ),
            ],
          ),
          const SizedBox(height: Space.x4),
          Expanded(
            child: needsKey
                ? EmptyState(
                    icon: Symbols.key_rounded,
                    title: l.cfKeyNeededTitle,
                    body: l.cfKeyNeededBody,
                    action: FilledButton.icon(
                      onPressed: () async {
                        if (await showCurseforgeKeyDialog(context, ref)) {
                          unawaited(ref.read(contentSearchProvider.notifier).retry());
                        }
                      },
                      icon: const Icon(Symbols.key_rounded, size: 18),
                      label: Text(l.cfKeyAdd),
                    ),
                  )
                : _Results(
                    state: state,
                    scroll: _scroll,
                    target: _target,
                  ),
          ),
        ],
      ),
    );
  }
}

class _TargetBanner extends ConsumerWidget {
  const _TargetBanner({required this.instance, this.world});

  final InstanceView instance;
  final String? world;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Container(
      padding: const EdgeInsets.fromLTRB(Space.x3, Space.x2, Space.x2, Space.x2),
      decoration: BoxDecoration(
        color: c.accentContainer.withValues(alpha: 0.4),
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: c.accent.withValues(alpha: 0.35)),
      ),
      child: Row(
        children: [
          InstanceIcon(id: instance.id, name: instance.name, size: 32),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(text: l.contentTargetPrefix),
                  TextSpan(
                    text: instance.name,
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                  TextSpan(
                    text: '  ·  ${versionLine(l, instance.mc, instance.loader)}',
                    style: TextStyle(color: c.textSecondary),
                  ),
                  if (world != null)
                    TextSpan(
                      text: '  ·  ${l.contentTargetWorld(world!)}',
                      style: TextStyle(color: c.textSecondary),
                    ),
                ],
              ),
              style: context.text.bodyMedium,
            ),
          ),
          TextButton.icon(
            onPressed: () => context.go('/instances/${instance.id}?tab=content'),
            icon: const Icon(Symbols.arrow_back_rounded, size: 18),
            label: Text(l.contentBackToInstance),
          ),
          IconButton(
            tooltip: l.contentClearTarget,
            onPressed: () => context.go('/content'),
            icon: const Icon(Symbols.close_rounded, size: 18),
          ),
        ],
      ),
    );
  }
}

class _Results extends ConsumerWidget {
  const _Results({required this.state, required this.scroll, this.target});

  final SearchState state;
  final ScrollController scroll;
  final ContentTarget? target;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    if (state.items.isEmpty) {
      if (state.error != null) {
        return Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                ErrorNote(state.error!),
                const SizedBox(height: Space.x4),
                OutlinedButton(
                  onPressed: () => ref.read(contentSearchProvider.notifier).retry(),
                  child: Text(l.retry),
                ),
              ],
            ),
          ),
        );
      }
      if (state.loading || !state.started) {
        return const Center(child: CircularProgressIndicator());
      }
      return EmptyState(
        icon: Symbols.search_off_rounded,
        title: l.contentNoResultsTitle,
        body: l.contentNoResultsBody,
      );
    }
    final installed = target == null
        ? const <String>{}
        : {
            for (final x in ref.watch(instanceContentProvider(target!.instanceId)).value ??
                const <InstalledContentView>[])
              if (x.projectId != null) x.projectId!,
          };
    return CustomScrollView(
      controller: scroll,
      slivers: [
        SliverGrid(
          gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: 640,
            mainAxisExtent: 148,
            mainAxisSpacing: Space.x3,
            crossAxisSpacing: Space.x3,
          ),
          delegate: SliverChildBuilderDelegate(
            (context, i) {
              final p = state.items[i];
              return ProjectCard(
                project: p,
                target: target,
                installed: installed.contains(p.id),
              );
            },
            childCount: state.items.length,
          ),
        ),
        SliverToBoxAdapter(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: Space.x6),
            child: Center(
              child: state.loading
                  ? const SizedBox.square(
                      dimension: 24,
                      child: CircularProgressIndicator(strokeWidth: 2.5),
                    )
                  : state.error != null
                      ? TextButton(
                          onPressed: () =>
                              ref.read(contentSearchProvider.notifier).loadMore(),
                          child: Text(l.retry),
                        )
                      : !state.hasMore
                          ? Text(
                              l.contentEnd,
                              style: context.text.bodySmall!.copyWith(color: c.textMuted),
                            )
                          : const SizedBox.shrink(),
            ),
          ),
        ),
      ],
    );
  }
}

class ProjectCard extends ConsumerStatefulWidget {
  const ProjectCard({
    super.key,
    required this.project,
    required this.target,
    required this.installed,
  });

  final ProjectView project;
  final ContentTarget? target;
  final bool installed;

  @override
  ConsumerState<ProjectCard> createState() => _ProjectCardState();
}

class _ProjectCardState extends ConsumerState<ProjectCard> {
  bool _busy = false;

  /// Con destino, «Instalar» instala ya la versión compatible más nueva; sin
  /// él (o si es un modpack) abre la ficha para elegir.
  Future<void> _install() async {
    final p = widget.project;
    final t = widget.target;
    final kind = p.kind ?? 'mod';
    if (t == null || kind == 'modpack' || (kind == 'datapack' && t.world == null)) {
      await showProjectSheet(context, p, target: t);
      return;
    }
    setState(() => _busy = true);
    await installAndReport(
      context,
      ref,
      instanceId: t.instanceId,
      project: p,
      world: t.world,
    );
    if (mounted) setState(() => _busy = false);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final p = widget.project;
    final isPack = p.kind == 'modpack';
    return Material(
      color: c.surface1,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(color: c.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => showProjectSheet(context, p, target: widget.target),
        child: Padding(
          padding: const EdgeInsets.all(Space.x4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              ProjectIcon(url: p.iconUrl, seed: p.id, title: p.title, size: 64),
              const SizedBox(width: Space.x4),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text.rich(
                      TextSpan(
                        children: [
                          TextSpan(text: p.title, style: context.text.titleMedium),
                          if (p.author != null)
                            TextSpan(
                              text: '  ${l.byAuthor(p.author!)}',
                              style: context.text.bodySmall!.copyWith(color: c.textMuted),
                            ),
                        ],
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: Space.x1),
                    Text(
                      p.description,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: context.text.bodySmall,
                    ),
                    const Spacer(),
                    Row(
                      children: [
                        Icon(Symbols.download_rounded, size: 15, color: c.textMuted),
                        const SizedBox(width: 4),
                        Text(
                          formatCompact(l, p.downloads),
                          style: context.text.labelMedium!.copyWith(color: c.textSecondary),
                        ),
                        const SizedBox(width: Space.x3),
                        Icon(Symbols.favorite_rounded, size: 14, color: c.textMuted),
                        const SizedBox(width: 4),
                        Text(
                          formatCompact(l, p.follows),
                          style: context.text.labelMedium!.copyWith(color: c.textSecondary),
                        ),
                        const SizedBox(width: Space.x3),
                        Expanded(
                          child: Row(
                            children: [
                              for (final cat in p.categories.take(2)) ...[
                                Flexible(child: _Category(cat)),
                                const SizedBox(width: Space.x1),
                              ],
                            ],
                          ),
                        ),
                        const SizedBox(width: Space.x2),
                        if (widget.installed)
                          ColorBadge(
                            l.contentInstalled,
                            color: c.success,
                            icon: Symbols.check_rounded,
                          )
                        else
                          FilledButton.tonalIcon(
                            style: FilledButton.styleFrom(
                              backgroundColor: c.accentContainer,
                              foregroundColor: c.onAccentContainer,
                              minimumSize: const Size(0, 32),
                              padding: const EdgeInsets.symmetric(horizontal: 12),
                            ),
                            onPressed: _busy ? null : _install,
                            icon: _busy
                                ? const SizedBox.square(
                                    dimension: 14,
                                    child: CircularProgressIndicator(strokeWidth: 2),
                                  )
                                : Icon(
                                    isPack ? Symbols.add_box_rounded : Symbols.download_rounded,
                                    size: 16,
                                  ),
                            label: Text(isPack ? l.modpackInstall : l.install),
                          ),
                      ],
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

class _Category extends StatelessWidget {
  const _Category(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
      decoration: BoxDecoration(
        color: c.surface2,
        borderRadius: BorderRadius.circular(Radii.pill),
      ),
      child: Text(
        label,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: context.text.labelSmall!.copyWith(color: c.textSecondary),
      ),
    );
  }
}
