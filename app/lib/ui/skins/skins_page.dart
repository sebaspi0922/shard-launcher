// Skins: visor 3D de la skin activa, cambiarla (PNG o URL, clásica o
// delgada), restablecerla y elegir capa. Solo cuentas de Microsoft.

import 'package:file_selector/file_selector.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/providers.dart';
import '../../core/skins.dart';
import '../../l10n/app_localizations.dart';
import '../accounts/account_dialogs.dart';
import '../accounts/account_panel.dart';
import '../theme/tokens.dart';
import '../widgets/avatar.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';
import 'skin_mesh.dart';
import 'skin_viewer.dart';

class SkinsPage extends ConsumerWidget {
  const SkinsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final account = ref.watch(activeAccountProvider);
    final loading = ref.watch(accountsProvider).isLoading;

    final Widget body;
    if (account == null) {
      body = loading
          ? const Center(child: CircularProgressIndicator())
          : EmptyState(
              icon: Symbols.checkroom_rounded,
              title: l.skinsNoAccountTitle,
              body: l.skinsNoAccountBody,
              action: FilledButton.icon(
                onPressed: () => showAccountPanel(context),
                icon: const Icon(Symbols.person_add_rounded, size: 18),
                label: Text(l.skinsAddAccount),
              ),
            );
    } else if (account.shard) {
      body = EmptyState(
        icon: Symbols.checkroom_rounded,
        title: l.accountShard,
        body: l.shardSkinsBody,
      );
    } else if (!account.microsoft) {
      body = EmptyState(
        icon: Symbols.no_accounts_rounded,
        title: l.skinsOfflineTitle,
        body: l.skinsOfflineBody(account.name),
        action: FilledButton.icon(
          onPressed: () => showMicrosoftLogin(context),
          icon: const Icon(Symbols.login_rounded, size: 18),
          label: Text(l.accountsAddMicrosoft),
        ),
      );
    } else {
      body = _SkinsBody(account: account);
    }

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, Space.x6),
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
                    Text(l.skinsTitle, style: context.text.headlineMedium),
                    const SizedBox(height: Space.x1),
                    Text(
                      l.skinsSubtitle,
                      style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                    ),
                  ],
                ),
              ),
              if (account != null)
                InkWell(
                  borderRadius: BorderRadius.circular(Radii.pill),
                  onTap: () => showAccountPanel(context),
                  child: Container(
                    padding: const EdgeInsets.fromLTRB(6, 6, 14, 6),
                    decoration: BoxDecoration(
                      color: c.surface2,
                      borderRadius: BorderRadius.circular(Radii.pill),
                      border: Border.all(color: c.border),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        PlayerAvatar(account: account, size: 26),
                        const SizedBox(width: Space.x2),
                        Text(account.name, style: context.text.labelLarge),
                        const SizedBox(width: Space.x1),
                        Icon(Symbols.expand_more_rounded, size: 18, color: c.textMuted),
                      ],
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: Space.x6),
          Expanded(child: body),
        ],
      ),
    );
  }
}

class _SkinsBody extends ConsumerStatefulWidget {
  const _SkinsBody({required this.account});

  final AccountView account;

  @override
  ConsumerState<_SkinsBody> createState() => _SkinsBodyState();
}

enum _Source { file, url }

class _SkinsBodyState extends ConsumerState<_SkinsBody> {
  _Source _source = _Source.file;
  String? _file;
  final _url = TextEditingController();

  /// Modelo elegido; `null` = el de la skin actual.
  bool? _slim;
  bool _busy = false;
  String? _capeBusy;

  @override
  void dispose() {
    _url.dispose();
    super.dispose();
  }

  ProfileController get _profile =>
      ref.read(profileProvider(widget.account.id).notifier);

  Future<void> _pickFile() async {
    final l = AppLocalizations.of(context);
    final f = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(label: l.skinsPngFiles, extensions: const ['png']),
      ],
    );
    if (f != null && mounted) setState(() => _file = f.path);
  }

  Future<void> _apply(ProfileView profile, SkinTexture? preview) async {
    final l = AppLocalizations.of(context);
    final current = profile.activeSkin;
    final slim = _slim ?? current?.slim ?? false;
    setState(() => _busy = true);
    try {
      if (_source == _Source.file && _file != null) {
        if (preview == null || !isValidSkinSize(preview)) {
          showToast(context, l.skinsInvalidPng, error: true);
          return;
        }
        await _profile.uploadSkin(_file!, slim: slim);
      } else if (_source == _Source.url && _url.text.trim().isNotEmpty) {
        await _profile.setSkinUrl(_url.text.trim(), slim: slim);
      } else if (current != null) {
        // Solo cambia el modelo: la misma textura con otros brazos.
        await _profile.setSkinUrl(current.url, slim: slim);
      }
      if (!mounted) return;
      setState(() {
        _file = null;
        _url.clear();
        _slim = null;
      });
      showToast(context, l.skinsApplied);
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _reset() async {
    final l = AppLocalizations.of(context);
    final ok = await confirm(
      context,
      title: l.skinsResetTitle,
      body: l.skinsResetBody,
      action: l.skinsReset,
    );
    if (!ok || !mounted) return;
    setState(() => _busy = true);
    try {
      await _profile.resetSkin();
      if (mounted) showToast(context, l.skinsResetDone);
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _cape(String? id) async {
    final l = AppLocalizations.of(context);
    setState(() => _capeBusy = id ?? '');
    try {
      if (id == null) {
        await _profile.hideCape();
      } else {
        await _profile.showCape(id);
      }
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _capeBusy = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final async = ref.watch(profileProvider(widget.account.id));
    final profile = async.value;
    if (profile == null) {
      return async.hasError
          ? Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 480),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    ErrorNote(async.error!),
                    const SizedBox(height: Space.x4),
                    OutlinedButton.icon(
                      onPressed: () =>
                          ref.invalidate(profileProvider(widget.account.id)),
                      icon: const Icon(Symbols.refresh_rounded, size: 18),
                      label: Text(l.retry),
                    ),
                  ],
                ),
              ),
            )
          : const Center(child: CircularProgressIndicator());
    }

    final skin = profile.activeSkin;
    final cape = profile.activeCape;
    final skinTex =
        skin == null ? null : ref.watch(skinTextureProvider(skin.url));
    final capeTex = cape == null ? null : ref.watch(skinTextureProvider(cape.url)).value;
    final preview = _source == _Source.file && _file != null
        ? ref.watch(localSkinTextureProvider(_file!))
        : null;
    final slim = _slim ?? skin?.slim ?? false;
    final previewTex = preview?.value;
    final showPreview = previewTex != null && isValidSkinSize(previewTex);
    final shown = showPreview ? previewTex : skinTex?.value;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          flex: 11,
          child: _ViewerPanel(
            texture: shown,
            cape: capeTex,
            slim: slim,
            preview: showPreview,
            loading: shown == null && (skinTex?.isLoading ?? false),
          ),
        ),
        const SizedBox(width: Space.x5),
        Expanded(
          flex: 9,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _ChangeSkinCard(
                  source: _source,
                  file: _file,
                  url: _url,
                  slim: slim,
                  busy: _busy,
                  previewError: preview != null &&
                      !preview.isLoading &&
                      (previewTex == null || !isValidSkinSize(previewTex)),
                  canApply: !_busy &&
                      ((_source == _Source.file && showPreview) ||
                          (_source == _Source.url && _url.text.trim().isNotEmpty) ||
                          (skin != null && slim != skin.slim)),
                  onSource: (s) => setState(() => _source = s),
                  onPick: _pickFile,
                  onClearFile: () => setState(() => _file = null),
                  onUrlChanged: () => setState(() {}),
                  onSlim: (v) => setState(() => _slim = v),
                  onApply: () => _apply(profile, previewTex),
                  onReset: _reset,
                ),
                const SizedBox(height: Space.x5),
                _CapesCard(
                  capes: profile.capes,
                  busy: _capeBusy,
                  onSelect: _cape,
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _ViewerPanel extends StatelessWidget {
  const _ViewerPanel({
    required this.texture,
    required this.cape,
    required this.slim,
    required this.preview,
    required this.loading,
  });

  final SkinTexture? texture;
  final SkinTexture? cape;
  final bool slim;
  final bool preview;
  final bool loading;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final t = texture;
    return Panel(
      padding: const EdgeInsets.all(Space.x3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: SkinStage(
              child: Stack(
                children: [
                  Positioned.fill(
                    child: t == null
                        ? Center(
                            child: loading
                                ? const CircularProgressIndicator()
                                : Column(
                                    mainAxisSize: MainAxisSize.min,
                                    children: [
                                      Icon(Symbols.image_not_supported_rounded,
                                          size: 32, color: c.textMuted),
                                      const SizedBox(height: Space.x2),
                                      Text(l.skinsTextureMissing,
                                          style: context.text.bodySmall),
                                    ],
                                  ),
                          )
                        : Padding(
                            padding: const EdgeInsets.fromLTRB(0, Space.x10, 0, Space.x8),
                            child: SkinViewer(
                              key: ValueKey(t),
                              skin: t,
                              cape: cape,
                              slim: slim,
                            ),
                          ),
                  ),
                  Positioned(
                    left: Space.x3,
                    top: Space.x3,
                    child: Row(
                      children: [
                        if (preview) ...[
                          ColorBadge(l.skinsPreview, color: c.accent, icon: Symbols.visibility_rounded),
                          const SizedBox(width: Space.x2),
                        ],
                        Tag(slim ? l.skinsSlim : l.skinsClassic,
                            icon: Symbols.accessibility_new_rounded),
                      ],
                    ),
                  ),
                  if (t != null)
                    Positioned(
                      left: 0,
                      right: 0,
                      bottom: Space.x3,
                      child: Text(
                        l.skinsDragHint,
                        textAlign: TextAlign.center,
                        style: context.text.labelSmall!.copyWith(color: c.textMuted),
                      ),
                    ),
                ],
              ),
            ),
          ),
          if (t != null) ...[
            const SizedBox(width: Space.x3),
            SizedBox(
              width: 132,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: SkinStage(
                      label: l.skinsFront,
                      child: Padding(
                        padding: const EdgeInsets.fromLTRB(8, 22, 8, 8),
                        child: SkinViewer(
                          skin: t,
                          slim: slim,
                          camera: const SkinCamera(),
                          interactive: false,
                          shadow: false,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: Space.x3),
                  Expanded(
                    child: SkinStage(
                      label: l.skinsBack,
                      child: Padding(
                        padding: const EdgeInsets.fromLTRB(8, 22, 8, 8),
                        child: SkinViewer(
                          skin: t,
                          cape: cape,
                          slim: slim,
                          camera: const SkinCamera(yaw: 3.14159265),
                          interactive: false,
                          shadow: false,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _ChangeSkinCard extends StatelessWidget {
  const _ChangeSkinCard({
    required this.source,
    required this.file,
    required this.url,
    required this.slim,
    required this.busy,
    required this.previewError,
    required this.canApply,
    required this.onSource,
    required this.onPick,
    required this.onClearFile,
    required this.onUrlChanged,
    required this.onSlim,
    required this.onApply,
    required this.onReset,
  });

  final _Source source;
  final String? file;
  final TextEditingController url;
  final bool slim;
  final bool busy;
  final bool previewError;
  final bool canApply;
  final ValueChanged<_Source> onSource;
  final VoidCallback onPick;
  final VoidCallback onClearFile;
  final VoidCallback onUrlChanged;
  final ValueChanged<bool> onSlim;
  final VoidCallback onApply;
  final VoidCallback onReset;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final name = file?.split(RegExp(r'[/\\]')).last;
    return Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SectionTitle(l.skinsChange),
          SegmentedButton<_Source>(
            showSelectedIcon: false,
            segments: [
              ButtonSegment(
                value: _Source.file,
                icon: const Icon(Symbols.upload_file_rounded, size: 18),
                label: Text(l.skinsFromFile),
              ),
              ButtonSegment(
                value: _Source.url,
                icon: const Icon(Symbols.link_rounded, size: 18),
                label: Text(l.skinsFromUrl),
              ),
            ],
            selected: {source},
            onSelectionChanged: (s) => onSource(s.first),
          ),
          const SizedBox(height: Space.x4),
          if (source == _Source.file)
            InkWell(
              borderRadius: BorderRadius.circular(Radii.control),
              onTap: busy ? null : onPick,
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.x4,
                  vertical: Space.x4,
                ),
                decoration: BoxDecoration(
                  color: c.backgroundDeep.withValues(alpha: 0.35),
                  borderRadius: BorderRadius.circular(Radii.control),
                  border: Border.all(
                    color: previewError
                        ? c.danger.withValues(alpha: 0.6)
                        : c.borderStrong,
                  ),
                ),
                child: Row(
                  children: [
                    Icon(
                      name == null ? Symbols.add_photo_alternate_rounded : Symbols.image_rounded,
                      color: name == null ? c.textMuted : c.accent,
                    ),
                    const SizedBox(width: Space.x3),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            name ?? l.skinsPickPng,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.titleSmall,
                          ),
                          const SizedBox(height: 2),
                          Text(
                            previewError ? l.skinsInvalidPng : l.skinsPngHint,
                            style: context.text.bodySmall!.copyWith(
                              color: previewError ? c.danger : null,
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (name != null)
                      IconButton(
                        tooltip: l.remove,
                        iconSize: 18,
                        onPressed: onClearFile,
                        icon: const Icon(Symbols.close_rounded),
                      ),
                  ],
                ),
              ),
            )
          else
            TextField(
              controller: url,
              onChanged: (_) => onUrlChanged(),
              decoration: InputDecoration(
                labelText: l.skinsUrl,
                hintText: 'https://…/skin.png',
                helperText: l.skinsUrlHint,
                prefixIcon: const Icon(Symbols.link_rounded, size: 18),
              ),
            ),
          const SizedBox(height: Space.x5),
          Text(l.skinsModel, style: context.text.titleSmall),
          const SizedBox(height: Space.x2),
          Row(
            children: [
              Expanded(
                child: _ModelOption(
                  label: l.skinsClassic,
                  detail: l.skinsClassicDesc,
                  selected: !slim,
                  onTap: () => onSlim(false),
                ),
              ),
              const SizedBox(width: Space.x3),
              Expanded(
                child: _ModelOption(
                  label: l.skinsSlim,
                  detail: l.skinsSlimDesc,
                  selected: slim,
                  onTap: () => onSlim(true),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.x5),
          Row(
            children: [
              TextButton.icon(
                onPressed: busy ? null : onReset,
                icon: const Icon(Symbols.restart_alt_rounded, size: 18),
                label: Text(l.skinsReset),
              ),
              const Spacer(),
              FilledButton.icon(
                onPressed: canApply ? onApply : null,
                icon: busy
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Symbols.check_rounded, size: 18),
                label: Text(l.skinsApply),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _ModelOption extends StatelessWidget {
  const _ModelOption({
    required this.label,
    required this.detail,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final String detail;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Material(
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
        child: Padding(
          padding: const EdgeInsets.all(Space.x3),
          child: Row(
            children: [
              Icon(
                selected
                    ? Symbols.radio_button_checked_rounded
                    : Symbols.radio_button_unchecked_rounded,
                size: 18,
                color: selected ? c.accent : c.textMuted,
              ),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(label, style: context.text.titleSmall),
                    Text(detail, style: context.text.bodySmall),
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

class _CapesCard extends ConsumerWidget {
  const _CapesCard({
    required this.capes,
    required this.busy,
    required this.onSelect,
  });

  final List<CapeView> capes;

  /// Capa que se está aplicando (`''` = quitar capa).
  final String? busy;
  final ValueChanged<String?> onSelect;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final none = !capes.any((c) => c.active);
    return Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SectionTitle(
            l.skinsCapes,
            trailing: Text(
              l.skinsCapesCount(capes.length),
              style: context.text.bodySmall,
            ),
          ),
          if (capes.isEmpty)
            Text(l.skinsNoCapes, style: context.text.bodySmall)
          else
            Wrap(
              spacing: Space.x3,
              runSpacing: Space.x3,
              children: [
                _CapeTile(
                  label: l.skinsNoCape,
                  selected: none,
                  busy: busy == '',
                  onTap: busy != null || none ? null : () => onSelect(null),
                  child: Icon(
                    Symbols.block_rounded,
                    color: context.colors.textMuted,
                  ),
                ),
                for (final cape in capes)
                  _CapeTile(
                    label: cape.alias ?? l.skinsCape,
                    selected: cape.active,
                    busy: busy == cape.id,
                    onTap: busy != null || cape.active
                        ? null
                        : () => onSelect(cape.id),
                    child: switch (ref.watch(skinTextureProvider(cape.url))) {
                      AsyncData(:final value?) => Padding(
                          padding: const EdgeInsets.all(10),
                          child: CapePreview(cape: value),
                        ),
                      AsyncLoading() => const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      _ => Icon(
                          Symbols.flag_rounded,
                          color: context.colors.textMuted,
                        ),
                    },
                  ),
              ],
            ),
        ],
      ),
    );
  }
}

class _CapeTile extends StatelessWidget {
  const _CapeTile({
    required this.label,
    required this.selected,
    required this.busy,
    required this.onTap,
    required this.child,
  });

  final String label;
  final bool selected;
  final bool busy;
  final VoidCallback? onTap;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    // Toda la ficha (también el nombre) responde al clic.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: SizedBox(
        width: 92,
        child: Column(
          children: [
            Material(
              color: selected ? c.accentContainer.withValues(alpha: 0.6) : c.surface1,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(Radii.card),
                side: BorderSide(
                  color: selected ? c.accent : c.border,
                  width: selected ? 2 : 1,
                ),
              ),
              child: InkWell(
                borderRadius: BorderRadius.circular(Radii.card),
                onTap: onTap,
                child: SizedBox(
                  width: 92,
                  height: 112,
                  child: Center(
                    child: busy
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : child,
                  ),
                ),
              ),
            ),
            const SizedBox(height: Space.x2),
            Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: context.text.labelMedium!.copyWith(
                color: selected ? c.text : c.textSecondary,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
