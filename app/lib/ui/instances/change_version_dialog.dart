// Cambiar la versión de Minecraft y el loader de una instancia, con copia de
// seguridad previa opcional.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import 'create_instance_dialog.dart';

Future<void> showChangeVersion(BuildContext context, InstanceView instance) async {
  final changed = await showDialog<InstanceView>(
    context: context,
    builder: (_) => ChangeVersionDialog(instance: instance),
  );
  if (changed != null && context.mounted) {
    final l = AppLocalizations.of(context);
    showToast(context, l.changeVersionDone(versionLine(l, changed.mc, changed.loader)));
  }
}

class ChangeVersionDialog extends ConsumerStatefulWidget {
  const ChangeVersionDialog({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<ChangeVersionDialog> createState() =>
      _ChangeVersionDialogState();
}

class _ChangeVersionDialogState extends ConsumerState<ChangeVersionDialog> {
  final _search = TextEditingController();
  bool _snapshots = false;
  late String? _mc = widget.instance.mc;
  late String _loader = kCreatableLoaders.contains(widget.instance.loader)
      ? widget.instance.loader
      : 'vanilla';
  late String? _loaderVersion = widget.instance.loaderVersion;
  bool _backup = true;
  bool _busy = false;
  Object? _error;

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  bool get _changed =>
      _mc != widget.instance.mc ||
      _loader != widget.instance.loader ||
      (_loader != 'vanilla' && _loaderVersion != widget.instance.loaderVersion);

  Future<void> _apply() async {
    final mc = _mc;
    if (mc == null || _busy) return;
    final l = AppLocalizations.of(context);
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final i = widget.instance;
      if (_backup) {
        await ref.read(backupsProvider(i.id).notifier).create(
              label: l.changeVersionBackupLabel(versionLine(l, i.mc, i.loader)),
            );
      }
      final changed = await ref.read(instancesProvider.notifier).changeVersion(
            id: i.id,
            mc: mc,
            loader: _loader,
            loaderVersion: _loader == 'vanilla' ? null : _loaderVersion,
          );
      if (mounted) Navigator.of(context).pop(changed);
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
    final c = context.colors;
    final versions = ref.watch(versionsProvider(_snapshots));
    final i = widget.instance;

    return AppDialog(
      title: l.changeVersionTitle(i.name),
      icon: Symbols.swap_vert_rounded,
      width: 720,
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: Text(l.cancel),
        ),
        FilledButton.icon(
          onPressed: _busy || _mc == null || !_changed ? null : _apply,
          icon: _busy
              ? const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Symbols.check_rounded, size: 18),
          label: Text(l.changeVersionApply),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Tag(
                l.changeVersionCurrent(
                  i.loaderVersion == null
                      ? versionLine(l, i.mc, i.loader)
                      : '${versionLine(l, i.mc, i.loader)} ${i.loaderVersion}',
                ),
                dot: loaderColor(i.loader),
              ),
              if (_changed && _mc != null) ...[
                const SizedBox(width: Space.x2),
                Icon(Symbols.arrow_forward_rounded, size: 16, color: c.textMuted),
                const SizedBox(width: Space.x2),
                Tag(
                  versionLine(l, _mc!, _loader),
                  dot: loaderColor(_loader),
                  strong: true,
                ),
              ],
            ],
          ),
          const SizedBox(height: Space.x4),
          SizedBox(
            height: 300,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  flex: 5,
                  child: VersionPicker(
                    versions: versions,
                    search: _search,
                    snapshots: _snapshots,
                    selected: _mc,
                    onSnapshots: (v) => setState(() => _snapshots = v),
                    onSelect: (id) => setState(() {
                      _mc = id;
                      _loaderVersion = null;
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
                        LoaderOption(
                          loader: loader,
                          selected: loader == _loader,
                          onTap: () => setState(() {
                            _loader = loader;
                            _loaderVersion = null;
                          }),
                        ),
                      const Spacer(),
                      if (_loader != 'vanilla' && _mc != null)
                        LoaderVersionField(
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
          const SizedBox(height: Space.x4),
          Container(
            padding: const EdgeInsets.fromLTRB(Space.x3, Space.x2, Space.x3, Space.x2),
            decoration: BoxDecoration(
              color: c.warning.withValues(alpha: 0.08),
              borderRadius: BorderRadius.circular(Radii.control),
              border: Border.all(color: c.warning.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Icon(Symbols.warning_rounded, size: 18, color: c.warning),
                const SizedBox(width: Space.x3),
                Expanded(
                  child: Text(l.changeVersionWarning, style: context.text.bodySmall),
                ),
                const SizedBox(width: Space.x3),
                InkWell(
                  borderRadius: BorderRadius.circular(6),
                  onTap: _busy ? null : () => setState(() => _backup = !_backup),
                  child: Row(
                    children: [
                      Checkbox(
                        value: _backup,
                        onChanged: _busy ? null : (v) => setState(() => _backup = v ?? false),
                      ),
                      Text(l.changeVersionBackup, style: context.text.labelLarge),
                      const SizedBox(width: Space.x2),
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
