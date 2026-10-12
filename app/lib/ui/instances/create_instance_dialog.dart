// Diálogo para crear una instancia: nombre, versión y loader.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';

/// Versiones de Minecraft (con o sin snapshots).
final versionsProvider = FutureProvider.family<List<VersionView>, bool>(
  (ref, snapshots) =>
      ref.watch(bridgeProvider).listVersions(snapshots: snapshots),
);

/// Versiones de un loader para una versión de Minecraft.
final loaderVersionsProvider =
    FutureProvider.family<List<String>, (String, String)>(
  (ref, key) => ref.watch(bridgeProvider).listLoaderVersions(key.$1, key.$2),
);

Future<void> showCreateInstance(BuildContext context) async {
  final created = await showDialog<InstanceView>(
    context: context,
    builder: (_) => const CreateInstanceDialog(),
  );
  if (created != null && context.mounted) {
    showToast(context, AppLocalizations.of(context).createDone(created.name));
    context.go('/instances/${created.id}');
  }
}

class CreateInstanceDialog extends ConsumerStatefulWidget {
  const CreateInstanceDialog({super.key});

  @override
  ConsumerState<CreateInstanceDialog> createState() =>
      _CreateInstanceDialogState();
}

class _CreateInstanceDialogState extends ConsumerState<CreateInstanceDialog> {
  final _name = TextEditingController();
  final _search = TextEditingController();
  bool _nameEdited = false;
  bool _snapshots = false;
  String? _mc;
  String _loader = 'vanilla';
  String? _loaderVersion;
  bool _busy = false;
  bool _touched = false;
  bool _pickPending = false;
  Object? _error;

  @override
  void dispose() {
    _name.dispose();
    _search.dispose();
    super.dispose();
  }

  void _suggestName(AppLocalizations l) {
    if (_nameEdited || _mc == null) return;
    _name.text = _loader == 'vanilla'
        ? 'Minecraft $_mc'
        : '${loaderName(l, _loader)} $_mc';
  }

  Future<void> _create() async {
    setState(() => _touched = true);
    final mc = _mc;
    if (_name.text.trim().isEmpty || mc == null || _busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final i = await ref.read(instancesProvider.notifier).create(
            name: _name.text.trim(),
            mc: mc,
            loader: _loader,
            loaderVersion: _loader == 'vanilla' ? null : _loaderVersion,
          );
      if (mounted) Navigator.of(context).pop(i);
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
    final versions = ref.watch(versionsProvider(_snapshots));

    // Elegir la última release al cargar (tras el frame: cambia el nombre).
    final list = versions.value;
    if (_mc == null && list != null && list.isNotEmpty && !_pickPending) {
      _pickPending = true;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _pickPending = false;
        if (!mounted || _mc != null) return;
        setState(() {
          _mc = list
              .firstWhere((v) => v.kind == 'release', orElse: () => list.first)
              .id;
          _suggestName(l);
        });
      });
    }

    return AppDialog(
      title: l.createTitle,
      icon: Symbols.add_box_rounded,
      width: 720,
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l.cancel),
        ),
        FilledButton.icon(
          onPressed: _busy || _mc == null ? null : _create,
          icon: _busy
              ? const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Symbols.add_rounded, size: 18),
          label: Text(l.create),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _name,
            onChanged: (_) => setState(() => _nameEdited = true),
            decoration: InputDecoration(
              labelText: l.createName,
              hintText: l.createNameHint,
              errorText: _touched && _name.text.trim().isEmpty
                  ? l.createNameRequired
                  : null,
            ),
          ),
          const SizedBox(height: Space.x5),
          SizedBox(
            height: 330,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  flex: 5,
                  child: _VersionPicker(
                    versions: versions,
                    search: _search,
                    snapshots: _snapshots,
                    selected: _mc,
                    onSnapshots: (v) => setState(() {
                      _snapshots = v;
                      _mc = null;
                    }),
                    onSelect: (id) => setState(() {
                      _mc = id;
                      _loaderVersion = null;
                      _suggestName(l);
                    }),
                    onSearch: () => setState(() {}),
                  ),
                ),
                const SizedBox(width: Space.x5),
                Expanded(
                  flex: 4,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text(l.createLoader, style: context.text.titleSmall),
                      const SizedBox(height: Space.x2),
                      for (final loader in kCreatableLoaders)
                        _LoaderOption(
                          loader: loader,
                          selected: loader == _loader,
                          onTap: () => setState(() {
                            _loader = loader;
                            _loaderVersion = null;
                            _suggestName(l);
                          }),
                        ),
                      const Spacer(),
                      if (_loader != 'vanilla' && _mc != null)
                        _LoaderVersionField(
                          loader: _loader,
                          mc: _mc!,
                          value: _loaderVersion,
                          onChanged: (v) => setState(() => _loaderVersion = v),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.x4),
            ErrorNote(_error!),
          ],
          if (versions.hasError) ...[
            const SizedBox(height: Space.x4),
            ErrorNote(versions.error!),
          ],
        ],
      ),
    );
  }
}

class _VersionPicker extends StatelessWidget {
  const _VersionPicker({
    required this.versions,
    required this.search,
    required this.snapshots,
    required this.selected,
    required this.onSnapshots,
    required this.onSelect,
    required this.onSearch,
  });

  final AsyncValue<List<VersionView>> versions;
  final TextEditingController search;
  final bool snapshots;
  final String? selected;
  final ValueChanged<bool> onSnapshots;
  final ValueChanged<String> onSelect;
  final VoidCallback onSearch;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final q = search.text.trim().toLowerCase();
    final all = versions.value ?? const <VersionView>[];
    final list = q.isEmpty ? all : all.where((v) => v.id.toLowerCase().contains(q)).toList();
    final latest = all.where((v) => v.kind == 'release').firstOrNull?.id;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(l.createVersion, style: context.text.titleSmall),
            const Spacer(),
            Text(l.createSnapshots, style: context.text.bodySmall),
            const SizedBox(width: Space.x1),
            Transform.scale(
              scale: 0.8,
              child: Switch(value: snapshots, onChanged: onSnapshots),
            ),
          ],
        ),
        const SizedBox(height: Space.x1),
        TextField(
          controller: search,
          onChanged: (_) => onSearch(),
          decoration: InputDecoration(
            hintText: l.createSearchVersion,
            prefixIcon: const Icon(Symbols.search_rounded, size: 18),
          ),
        ),
        const SizedBox(height: Space.x2),
        Expanded(
          child: Container(
            decoration: BoxDecoration(
              color: c.backgroundDeep.withValues(alpha: 0.4),
              borderRadius: BorderRadius.circular(Radii.control),
              border: Border.all(color: c.border),
            ),
            child: versions.isLoading && all.isEmpty
                ? const Center(child: CircularProgressIndicator())
                : list.isEmpty
                    ? Center(
                        child: Text(
                          l.createNoVersions,
                          style: context.text.bodySmall,
                        ),
                      )
                    : ListView.builder(
                        padding: const EdgeInsets.all(Space.x1),
                        itemCount: list.length,
                        itemExtent: 36,
                        itemBuilder: (context, i) {
                          final v = list[i];
                          final sel = v.id == selected;
                          return Material(
                            color: sel ? c.accentContainer : Colors.transparent,
                            borderRadius: BorderRadius.circular(6),
                            child: InkWell(
                              borderRadius: BorderRadius.circular(6),
                              onTap: () => onSelect(v.id),
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: Space.x3,
                                ),
                                child: Row(
                                  children: [
                                    Text(
                                      v.id,
                                      style: context.text.bodyMedium!.copyWith(
                                        color: sel ? c.onAccentContainer : c.text,
                                        fontWeight:
                                            sel ? FontWeight.w600 : FontWeight.w400,
                                      ),
                                    ),
                                    const SizedBox(width: Space.x2),
                                    if (v.id == latest) Tag(l.createLatestRelease),
                                    const Spacer(),
                                    if (v.kind != 'release')
                                      Text(
                                        v.kind,
                                        style: context.text.bodySmall!.copyWith(
                                          color: c.warning,
                                        ),
                                      ),
                                    if (sel)
                                      Icon(
                                        Symbols.check_rounded,
                                        size: 18,
                                        color: c.onAccentContainer,
                                      ),
                                  ],
                                ),
                              ),
                            ),
                          );
                        },
                      ),
          ),
        ),
      ],
    );
  }
}

class _LoaderOption extends StatelessWidget {
  const _LoaderOption({
    required this.loader,
    required this.selected,
    required this.onTap,
  });

  final String loader;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Material(
        color: selected ? c.accentContainer.withValues(alpha: 0.7) : c.surface1,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(Radii.control),
          side: BorderSide(
            color: selected ? c.accent.withValues(alpha: 0.6) : c.border,
          ),
        ),
        child: InkWell(
          borderRadius: BorderRadius.circular(Radii.control),
          onTap: onTap,
          child: SizedBox(
            height: 38,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Space.x3),
              child: Row(
                children: [
                  Container(
                    width: 9,
                    height: 9,
                    decoration: BoxDecoration(
                      color: loaderColor(loader),
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                  const SizedBox(width: Space.x3),
                  Text(
                    loaderName(l, loader),
                    style: context.text.bodyMedium!.copyWith(
                      fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                    ),
                  ),
                  const Spacer(),
                  if (selected)
                    Icon(Symbols.radio_button_checked_rounded, size: 18, color: c.accent)
                  else
                    Icon(Symbols.radio_button_unchecked_rounded, size: 18, color: c.textMuted),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _LoaderVersionField extends ConsumerWidget {
  const _LoaderVersionField({
    required this.loader,
    required this.mc,
    required this.value,
    required this.onChanged,
  });

  final String loader;
  final String mc;
  final String? value;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final versions = ref.watch(loaderVersionsProvider((loader, mc)));
    return switch (versions) {
      AsyncData(:final value) when value.isEmpty => Text(
          l.createNoLoaderVersions(loaderName(l, loader), mc),
          style: context.text.bodySmall!.copyWith(color: context.colors.warning),
        ),
      AsyncData(value: final list) => DropdownButtonFormField<String?>(
          initialValue: value,
          isExpanded: true,
          decoration: InputDecoration(labelText: l.createLoaderVersion),
          borderRadius: BorderRadius.circular(Radii.control),
          dropdownColor: Color.alphaBlend(context.colors.surface3, context.colors.background),
          menuMaxHeight: 320,
          items: [
            DropdownMenuItem(value: null, child: Text(l.createLoaderLatest)),
            for (final v in list) DropdownMenuItem(value: v, child: Text(v)),
          ],
          onChanged: onChanged,
        ),
      AsyncError(:final error) => ErrorNote(error),
      _ => const LinearProgressIndicator(minHeight: 2),
    };
  }
}
