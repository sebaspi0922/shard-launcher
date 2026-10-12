// Ficha de un proyecto (panel lateral): descripción, destino de la
// instalación y versiones compatibles.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/content_search.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../instances/content_tab.dart' show contentKindColor, contentKindIcon;
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';
import '../widgets/select_button.dart';
import 'content_page.dart' show ContentTarget;
import 'install.dart';
import 'modpack_dialog.dart';

/// Abre la ficha deslizando desde la derecha.
Future<void> showProjectSheet(
  BuildContext context,
  ProjectView project, {
  ContentTarget? target,
}) =>
    showGeneralDialog<void>(
      context: context,
      barrierDismissible: true,
      barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
      barrierColor: Colors.black.withValues(alpha: 0.45),
      transitionDuration: Motion.medium,
      pageBuilder: (_, _, _) => Align(
        alignment: Alignment.centerRight,
        child: ProjectSheet(project: project, target: target),
      ),
      transitionBuilder: (_, anim, _, child) => SlideTransition(
        position: Tween(begin: const Offset(0.25, 0), end: Offset.zero)
            .chain(CurveTween(curve: Motion.curve))
            .animate(anim),
        child: FadeTransition(opacity: anim, child: child),
      ),
    );

class ProjectSheet extends ConsumerStatefulWidget {
  const ProjectSheet({super.key, required this.project, this.target});

  final ProjectView project;
  final ContentTarget? target;

  @override
  ConsumerState<ProjectSheet> createState() => _ProjectSheetState();
}

class _ProjectSheetState extends ConsumerState<ProjectSheet> {
  String? _instanceId;
  String? _world;

  /// Versión que se está instalando (`''` = la última compatible).
  String? _installing;

  String get _kind => widget.project.kind ?? 'mod';
  bool get _isPack => _kind == 'modpack';

  @override
  void initState() {
    super.initState();
    _instanceId = widget.target?.instanceId ?? ref.read(homeInstanceProvider)?.id;
    _world = widget.target?.world;
  }

  Future<void> _install(String? version) async {
    final id = _instanceId;
    if (id == null) return;
    setState(() => _installing = version ?? '');
    final ok = await installAndReport(
      context,
      ref,
      instanceId: id,
      project: widget.project,
      version: version,
      world: _kind == 'datapack' ? _world : null,
    );
    if (!mounted) return;
    setState(() => _installing = null);
    if (ok) Navigator.pop(context);
  }

  /// El diálogo del modpack se abre encima; al cerrarlo se cierra la ficha.
  Future<void> _installPack(String url) async {
    await showModpackInstall(context, source: url, name: widget.project.title);
    if (mounted) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final p = widget.project;
    final kc = contentKindColor(_kind);
    final instance = _instanceId == null ? null : ref.watch(instanceByIdProvider(_instanceId!));
    final search = ref.watch(contentSearchProvider).filters;

    final VersionsQuery query = _isPack
        ? (source: p.source, project: p.id, mc: search.mc, loader: search.loader)
        : (
            source: p.source,
            project: p.id,
            mc: instance?.mc,
            loader: kindUsesLoader(_kind) && instance != null ? instance.loader : null,
          );
    final versions = ref.watch(projectVersionsProvider(query));
    final installedHere = !_isPack && instance != null
        ? (ref.watch(instanceContentProvider(instance.id)).value ?? const <InstalledContentView>[])
            .where((x) => x.projectId == p.id)
            .firstOrNull
        : null;

    return Material(
      color: Color.alphaBlend(c.surface2, c.background),
      elevation: 0,
      shape: Border(left: BorderSide(color: c.borderStrong)),
      child: SizedBox(
        width: 580,
        height: double.infinity,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.x6, Space.x6, Space.x4, 0),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  ProjectIcon(url: p.iconUrl, seed: p.id, title: p.title, size: 76),
                  const SizedBox(width: Space.x4),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(p.title, style: context.text.headlineSmall),
                        if (p.author != null)
                          Text(l.byAuthor(p.author!), style: context.text.bodySmall),
                        const SizedBox(height: Space.x2),
                        Wrap(
                          spacing: Space.x2,
                          runSpacing: Space.x1,
                          children: [
                            ColorBadge(
                              contentKindSingular(l, _kind),
                              color: kc,
                              icon: contentKindIcon(_kind),
                            ),
                            Tag(formatCompact(l, p.downloads), icon: Symbols.download_rounded),
                            Tag(formatCompact(l, p.follows), icon: Symbols.favorite_rounded),
                            Tag(p.source == 'curseforge' ? 'CurseForge' : 'Modrinth'),
                          ],
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    tooltip: l.close,
                    onPressed: () => Navigator.pop(context),
                    icon: const Icon(Symbols.close_rounded),
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.x6, Space.x4, Space.x6, 0),
              child: Text(
                p.description,
                style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
              ),
            ),
            if (p.categories.isNotEmpty)
              Padding(
                padding: const EdgeInsets.fromLTRB(Space.x6, Space.x3, Space.x6, 0),
                child: Wrap(
                  spacing: Space.x1,
                  runSpacing: Space.x1,
                  children: [for (final cat in p.categories) Tag(cat)],
                ),
              ),
            const SizedBox(height: Space.x5),
            const Divider(),
            if (!_isPack)
              Padding(
                padding: const EdgeInsets.fromLTRB(Space.x6, Space.x4, Space.x6, 0),
                child: _Destination(
                  kind: _kind,
                  instanceId: _instanceId,
                  world: _world,
                  onInstance: (id) => setState(() {
                    _instanceId = id;
                    _world = null;
                  }),
                  onWorld: (w) => setState(() => _world = w),
                ),
              ),
            if (installedHere != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(Space.x6, Space.x3, Space.x6, 0),
                child: Row(
                  children: [
                    Icon(Symbols.check_circle_rounded, size: 16, color: c.success),
                    const SizedBox(width: Space.x2),
                    Expanded(
                      child: Text(
                        l.contentAlreadyInstalled(installedHere.versionNumber ?? installedHere.file),
                        style: context.text.bodySmall!.copyWith(color: c.success),
                      ),
                    ),
                  ],
                ),
              ),
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.x6, Space.x5, Space.x6, Space.x2),
              child: Row(
                children: [
                  Text(l.versionsCompatible, style: context.text.titleMedium),
                  const SizedBox(width: Space.x2),
                  if (versions.value != null) Tag('${versions.value!.length}'),
                  const Spacer(),
                  if (!_isPack)
                    FilledButton.icon(
                      onPressed: _installing != null ||
                              instance == null ||
                              (_kind == 'datapack' && _world == null) ||
                              (versions.value?.isEmpty ?? true)
                          ? null
                          : () => _install(null),
                      icon: _installing == ''
                          ? const SizedBox.square(
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Symbols.download_rounded, size: 18),
                      label: Text(l.installLatest),
                    ),
                ],
              ),
            ),
            Expanded(
              child: switch (versions) {
                AsyncError(:final error) => Padding(
                    padding: const EdgeInsets.all(Space.x6),
                    child: Align(alignment: Alignment.topCenter, child: ErrorNote(error)),
                  ),
                AsyncData(:final value) when value.isEmpty => Padding(
                    padding: const EdgeInsets.all(Space.x6),
                    child: Text(
                      instance == null && !_isPack
                          ? l.chooseInstanceFirst
                          : l.versionsNoneCompatible,
                      style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                    ),
                  ),
                AsyncData(:final value) => ListView.separated(
                    padding: const EdgeInsets.fromLTRB(Space.x4, 0, Space.x4, Space.x6),
                    itemCount: value.length,
                    separatorBuilder: (_, _) => const SizedBox(height: Space.x1),
                    itemBuilder: (context, i) => _VersionRow(
                      version: value[i],
                      busy: _installing == value[i].id,
                      enabled: _installing == null &&
                          (_isPack ||
                              (instance != null &&
                                  (_kind != 'datapack' || _world != null))),
                      isPack: _isPack,
                      onInstall: () {
                        final v = value[i];
                        if (_isPack) {
                          final url = v.fileUrl;
                          if (url == null) return;
                          _installPack(url);
                        } else {
                          _install(v.id);
                        }
                      },
                    ),
                  ),
                _ => const Center(child: CircularProgressIndicator()),
              },
            ),
          ],
        ),
      ),
    );
  }
}

String contentKindSingular(AppLocalizations l, String kind) => switch (kind) {
      'mod' => l.kindMod,
      'resourcepack' => l.kindResourcepack,
      'shader' => l.kindShader,
      'datapack' => l.kindDatapack,
      'modpack' => l.kindModpack,
      _ => kind,
    };

class _Destination extends ConsumerWidget {
  const _Destination({
    required this.kind,
    required this.instanceId,
    required this.world,
    required this.onInstance,
    required this.onWorld,
  });

  final String kind;
  final String? instanceId;
  final String? world;
  final ValueChanged<String> onInstance;
  final ValueChanged<String> onWorld;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final instances = ref.watch(recentInstancesProvider);
    final worlds = kind == 'datapack' && instanceId != null
        ? ref.watch(worldsProvider(instanceId!)).value ?? const <WorldView>[]
        : const <WorldView>[];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l.installIn, style: context.text.labelLarge),
        const SizedBox(height: Space.x2),
        SelectButton<String?>(
          width: double.infinity,
          value: instanceId,
          options: [
            if (instanceId == null) SelectOption(null, l.chooseInstance),
            for (final i in instances)
              SelectOption(
                i.id,
                i.name,
                subtitle: versionLine(l, i.mc, i.loader),
                leading: InstanceIcon(id: i.id, name: i.name, size: 24),
              ),
          ],
          onChanged: (v) {
            if (v != null) onInstance(v);
          },
        ),
        if (kind == 'datapack') ...[
          const SizedBox(height: Space.x3),
          Text(l.installInWorld, style: context.text.labelLarge),
          const SizedBox(height: Space.x2),
          if (worlds.isEmpty)
            Text(
              l.worldsNone,
              style: context.text.bodySmall,
            )
          else
            SelectButton<String?>(
              width: double.infinity,
              icon: Symbols.public_rounded,
              value: world,
              options: [
                if (world == null) SelectOption(null, l.chooseWorld),
                for (final w in worlds) SelectOption(w.folder, w.name),
              ],
              onChanged: (v) {
                if (v != null) onWorld(v);
              },
            ),
        ],
      ],
    );
  }
}

class _VersionRow extends StatelessWidget {
  const _VersionRow({
    required this.version,
    required this.busy,
    required this.enabled,
    required this.isPack,
    required this.onInstall,
  });

  final ContentVersionView version;
  final bool busy;
  final bool enabled;
  final bool isPack;
  final VoidCallback onInstall;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final v = version;
    final (label, color) = switch (v.release) {
      'beta' => (l.releaseBeta, c.warning),
      'alpha' => (l.releaseAlpha, c.danger),
      _ => (l.releaseStable, c.success),
    };
    final date = DateTime.tryParse(v.datePublished);
    final mc = v.mcVersions.length > 3
        ? '${v.mcVersions.first} – ${v.mcVersions.last}'
        : v.mcVersions.join(', ');
    final loaders = v.loaders.map((x) => loaderName(l, x)).join(', ');
    return Container(
      padding: const EdgeInsets.fromLTRB(Space.x3, Space.x2, Space.x2, Space.x2),
      decoration: BoxDecoration(
        color: c.surface1,
        borderRadius: BorderRadius.circular(Radii.control),
        border: Border.all(color: c.border),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        v.versionNumber,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.text.titleSmall,
                      ),
                    ),
                    const SizedBox(width: Space.x2),
                    ColorBadge(label, color: color),
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  [
                    if (mc.isNotEmpty) mc,
                    if (loaders.isNotEmpty) loaders,
                    if (date != null) formatDate(l, date),
                    if (v.fileSize > BigInt.zero) formatBytes(l, v.fileSize),
                  ].join('  ·  '),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.text.bodySmall!.copyWith(color: c.textMuted),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.x2),
          OutlinedButton(
            style: OutlinedButton.styleFrom(
              minimumSize: const Size(0, 32),
              padding: const EdgeInsets.symmetric(horizontal: 12),
            ),
            onPressed: enabled ? onInstall : null,
            child: busy
                ? const SizedBox.square(
                    dimension: 14,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(isPack ? l.modpackInstall : l.install),
          ),
        ],
      ),
    );
  }
}
