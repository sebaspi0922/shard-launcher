// Instalar un modpack (desde la búsqueda, un archivo o una URL) en una
// instancia nueva, con progreso.

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../instances/backups_tab.dart' show StageProgress;
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';

/// Con `source` (ruta o URL) instala directamente; sin él deja elegir.
Future<void> showModpackInstall(
  BuildContext context, {
  String? source,
  String? name,
}) =>
    showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => ModpackDialog(source: source, name: name),
    );

enum _From { file, url }

class ModpackDialog extends ConsumerStatefulWidget {
  const ModpackDialog({super.key, this.source, this.name});

  final String? source;
  final String? name;

  @override
  ConsumerState<ModpackDialog> createState() => _ModpackDialogState();
}

class _ModpackDialogState extends ConsumerState<ModpackDialog> {
  _From _from = _From.file;
  String? _file;
  final _url = TextEditingController();
  late final _name = TextEditingController(text: widget.name ?? '');
  bool _busy = false;
  Object? _error;
  ModpackInstallView? _done;

  bool get _fixed => widget.source != null;

  String? get _source => _fixed
      ? widget.source
      : switch (_from) {
          _From.file => _file,
          _From.url => _url.text.trim().isEmpty ? null : _url.text.trim(),
        };

  @override
  void dispose() {
    _url.dispose();
    _name.dispose();
    super.dispose();
  }

  Future<void> _pick() async {
    final l = AppLocalizations.of(context);
    final f = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(label: l.modpackFiles, extensions: const ['mrpack', 'zip']),
      ],
    );
    if (f != null && mounted) setState(() => _file = f.path);
  }

  Future<void> _install() async {
    final source = _source;
    if (source == null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final name = _name.text.trim();
      final r = await ref
          .read(bridgeProvider)
          .installModpack(source, name: name.isEmpty ? null : name);
      await ref.read(instancesProvider.notifier).refresh();
      if (mounted) setState(() => _done = r);
    } catch (e) {
      if (mounted) setState(() => _error = e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final done = _done;
    if (done != null) {
      final i = done.instance;
      return AppDialog(
        title: l.modpackDoneTitle,
        icon: Symbols.check_circle_rounded,
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(l.close),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(context);
              context.go('/instances/${i.id}?tab=content');
            },
            child: Text(l.modpackOpenInstance),
          ),
        ],
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                InstanceIcon(id: i.id, name: i.name, size: 48),
                const SizedBox(width: Space.x3),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(i.name, style: context.text.titleMedium),
                      Text(versionLine(l, i.mc, i.loader), style: context.text.bodySmall),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: Space.x3),
            Text(l.modpackDoneBody(done.filesDownloaded, done.filesTotal)),
            if (done.warnings.isNotEmpty) ...[
              const SizedBox(height: Space.x3),
              for (final w in done.warnings.take(4))
                Text('• $w', style: context.text.bodySmall!.copyWith(color: c.warning)),
            ],
          ],
        ),
      );
    }

    return AppDialog(
      title: _fixed ? l.modpackInstallTitle(widget.name ?? '') : l.modpackImportTitle,
      icon: Symbols.inventory_2_rounded,
      width: 500,
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: _busy || _source == null ? null : _install,
          child: Text(l.modpackInstall),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.modpackBody),
          const SizedBox(height: Space.x4),
          if (!_fixed) ...[
            SegmentedButton<_From>(
              showSelectedIcon: false,
              segments: [
                ButtonSegment(
                  value: _From.file,
                  icon: const Icon(Symbols.folder_open_rounded, size: 16),
                  label: Text(l.modpackFromFile),
                ),
                ButtonSegment(
                  value: _From.url,
                  icon: const Icon(Symbols.link_rounded, size: 16),
                  label: Text(l.modpackFromUrl),
                ),
              ],
              selected: {_from},
              onSelectionChanged: _busy ? null : (v) => setState(() => _from = v.first),
            ),
            const SizedBox(height: Space.x3),
            if (_from == _From.file)
              OutlinedButton.icon(
                style: OutlinedButton.styleFrom(alignment: Alignment.centerLeft),
                onPressed: _busy ? null : _pick,
                icon: const Icon(Symbols.upload_file_rounded, size: 18),
                label: Text(
                  _file ?? l.modpackChooseFile,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              )
            else
              TextField(
                controller: _url,
                enabled: !_busy,
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(
                  labelText: 'URL',
                  hintText: 'https://cdn.modrinth.com/…/pack.mrpack',
                  prefixIcon: Icon(Symbols.link_rounded, size: 18),
                ),
              ),
            const SizedBox(height: Space.x3),
          ],
          TextField(
            controller: _name,
            enabled: !_busy,
            decoration: InputDecoration(
              labelText: l.modpackName,
              hintText: l.modpackNameHint,
            ),
          ),
          if (_busy) ...[
            const SizedBox(height: Space.x4),
            StageProgress(stages: const ['modpack'], label: l.modpackInstalling),
            const SizedBox(height: Space.x2),
            Text(l.modpackInstallingHint, style: context.text.bodySmall),
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
