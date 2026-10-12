// Ajustes globales: general, juego, descargas y acerca de.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';
import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/errors.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../instances/instance_settings_tab.dart' show splitArgs;
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/memory_slider.dart';
import 'curseforge_key_dialog.dart';

class SettingsPage extends ConsumerStatefulWidget {
  const SettingsPage({super.key});

  @override
  ConsumerState<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends ConsumerState<SettingsPage> {
  /// Cambios aún sin enviar (deslizadores y texto se guardan con retardo).
  SettingsView? _draft;
  Timer? _debounce;
  final _jvm = TextEditingController();
  bool _jvmLoaded = false;

  @override
  void dispose() {
    _debounce?.cancel();
    if (_draft != null) unawaited(_flush());
    _jvm.dispose();
    super.dispose();
  }

  Future<void> _flush() async {
    final draft = _draft;
    _draft = null;
    if (draft == null) return;
    try {
      await ref.read(settingsProvider.notifier).save(draft);
    } catch (_) {
      if (mounted) {
        showToast(
          context,
          AppLocalizations.of(context).settingsSaveFailed,
          error: true,
        );
      }
    }
  }

  void _change(SettingsView next, {bool debounce = false}) {
    setState(() => _draft = next);
    _debounce?.cancel();
    if (debounce) {
      _debounce = Timer(const Duration(milliseconds: 500), _flush);
    } else {
      unawaited(_flush());
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final async = ref.watch(settingsProvider);
    final saved = async.value;
    if (saved == null) {
      return async.hasError
          ? Center(child: ErrorNote(async.error!))
          : const Center(child: CircularProgressIndicator());
    }
    final s = _draft ?? saved;
    if (!_jvmLoaded) {
      _jvm.text = s.defaultJvmArgs.join(' ');
      _jvmLoaded = true;
    }
    final total = ref.watch(systemMemoryProvider);
    final bridge = ref.watch(bridgeProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, Space.x8),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 860),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(l.settingsTitle, style: context.text.headlineMedium),
              const SizedBox(height: Space.x6),
              _Group(
                title: l.settingsGeneral,
                icon: Symbols.tune_rounded,
                children: [
                  SettingRow(
                    title: l.settingsLanguage,
                    description: l.settingsLanguageDesc,
                    control: SegmentedButton<String>(
                      showSelectedIcon: false,
                      segments: [
                        ButtonSegment(value: 'system', label: Text(l.languageSystem)),
                        const ButtonSegment(value: 'es', label: Text('Español')),
                        const ButtonSegment(value: 'en', label: Text('English')),
                      ],
                      selected: {s.language ?? 'system'},
                      onSelectionChanged: (v) => _change(
                        s.copyWith(
                          language: () => v.first == 'system' ? null : v.first,
                        ),
                      ),
                    ),
                  ),
                  const Divider(),
                  SettingRow(
                    title: l.settingsClose,
                    description: l.settingsCloseDesc,
                    control: SegmentedButton<String>(
                      showSelectedIcon: false,
                      segments: [
                        ButtonSegment(value: 'ask', label: Text(l.closeAsk)),
                        ButtonSegment(value: 'tray', label: Text(l.closeTray)),
                        ButtonSegment(value: 'quit', label: Text(l.closeQuit)),
                      ],
                      selected: {s.closeBehavior},
                      onSelectionChanged: (v) =>
                          _change(s.copyWith(closeBehavior: v.first)),
                    ),
                  ),
                  const Divider(),
                  SettingRow(
                    title: l.settingsMultiple,
                    description: l.settingsMultipleDesc,
                    control: Switch(
                      value: s.allowMultipleGames,
                      onChanged: (v) =>
                          _change(s.copyWith(allowMultipleGames: v)),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.x5),
              _Group(
                title: l.settingsGame,
                icon: Symbols.sports_esports_rounded,
                children: [
                  SettingRow(
                    title: l.settingsMemory,
                    description: l.settingsMemoryDesc(formatMemory(l, total)),
                    controlWidth: 400,
                    control: MemorySlider(
                      valueMb: s.defaultMemoryMb,
                      totalMb: total,
                      onChanged: (v) => _change(
                        s.copyWith(defaultMemoryMb: v),
                        debounce: true,
                      ),
                    ),
                  ),
                  const Divider(),
                  SettingRow(
                    title: l.settingsJvm,
                    description: l.settingsJvmDesc,
                    controlWidth: 400,
                    control: TextField(
                      controller: _jvm,
                      style: TextStyle(
                        fontFamily: kMonoFamily,
                        fontSize: 12.5,
                        color: context.colors.text,
                      ),
                      decoration: const InputDecoration(hintText: '-XX:+UseG1GC'),
                      onChanged: (v) => _change(
                        s.copyWith(defaultJvmArgs: splitArgs(v)),
                        debounce: true,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.x5),
              _Group(
                title: l.settingsDownloads,
                icon: Symbols.download_rounded,
                children: [
                  SettingRow(
                    title: l.settingsConcurrency,
                    description: l.settingsConcurrencyDesc,
                    controlWidth: 400,
                    control: Row(
                      children: [
                        Expanded(
                          child: Slider(
                            min: 1,
                            max: 32,
                            divisions: 31,
                            value: s.maxConcurrentDownloads.clamp(1, 32).toDouble(),
                            label: '${s.maxConcurrentDownloads}',
                            onChanged: (v) => _change(
                              s.copyWith(maxConcurrentDownloads: v.round()),
                              debounce: true,
                            ),
                          ),
                        ),
                        const SizedBox(width: Space.x3),
                        Container(
                          width: 76,
                          padding: const EdgeInsets.symmetric(vertical: 8),
                          alignment: Alignment.center,
                          decoration: BoxDecoration(
                            color: context.colors.surface2,
                            borderRadius: BorderRadius.circular(Radii.control),
                            border: Border.all(color: context.colors.border),
                          ),
                          child: Text(
                            '${s.maxConcurrentDownloads}',
                            style: context.text.titleSmall,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.x5),
              _Group(
                title: l.settingsIntegrations,
                icon: Symbols.hub_rounded,
                children: [
                  const _CurseforgeKeyRow(),
                  const Divider(),
                  SettingRow(
                    title: l.settingsDiscord,
                    description: l.settingsDiscordDesc,
                    control: Switch(
                      value: s.discordRpc,
                      onChanged: (v) => _change(s.copyWith(discordRpc: v)),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.x5),
              _Group(
                title: l.settingsAbout,
                icon: Symbols.info_rounded,
                children: [
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: Space.x3),
                    child: Row(
                      children: [
                        const BrandMark(size: 40),
                        const SizedBox(width: Space.x4),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(kAppName, style: context.text.titleLarge),
                              Text(
                                l.settingsAboutBody(bridge.coreVersion),
                                style: context.text.bodySmall,
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                  const Divider(),
                  SettingRow(
                    title: l.settingsDataDir,
                    description: bridge.dataDir,
                    control: IconButton(
                      tooltip: l.copy,
                      onPressed: () => copyText(context, bridge.dataDir, l.copied),
                      icon: const Icon(Symbols.content_copy_rounded, size: 18),
                    ),
                  ),
                  const Divider(),
                  SettingRow(
                    title: l.settingsMsa,
                    control: Tag(
                      s.msaClientIdSet ? l.msaConfigured : l.msaNotConfigured,
                      dot: s.msaClientIdSet
                          ? context.colors.success
                          : context.colors.warning,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Group extends StatelessWidget {
  const _Group({
    required this.title,
    required this.icon,
    required this.children,
  });

  final String title;
  final IconData icon;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: Space.x1, bottom: Space.x2),
          child: Row(
            children: [
              Icon(icon, size: 18, color: c.accent),
              const SizedBox(width: Space.x2),
              Text(title, style: context.text.titleMedium),
            ],
          ),
        ),
        Panel(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.x5,
            vertical: Space.x2,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: children,
          ),
        ),
      ],
    );
  }
}

class _CurseforgeKeyRow extends ConsumerWidget {
  const _CurseforgeKeyRow();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final has = ref.watch(curseforgeKeyProvider).value ?? false;
    return SettingRow(
      title: l.settingsCurseforge,
      description: l.settingsCurseforgeDesc,
      control: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Tag(
            has ? l.cfKeySet : l.cfKeyNotSet,
            dot: has ? c.success : c.textMuted,
          ),
          const SizedBox(width: Space.x3),
          if (has)
            TextButton(
              onPressed: () async {
                final ok = await confirm(
                  context,
                  title: l.cfKeyRemoveTitle,
                  body: l.cfKeyRemoveBody,
                  action: l.remove,
                  danger: true,
                );
                if (!ok) return;
                try {
                  await ref.read(curseforgeKeyProvider.notifier).clear();
                } catch (e) {
                  if (context.mounted) showToast(context, errorText(l, e), error: true);
                }
              },
              child: Text(l.remove),
            ),
          OutlinedButton.icon(
            onPressed: () => showCurseforgeKeyDialog(context, ref),
            icon: const Icon(Symbols.key_rounded, size: 18),
            label: Text(has ? l.cfKeyChange : l.cfKeyAdd),
          ),
        ],
      ),
    );
  }
}
