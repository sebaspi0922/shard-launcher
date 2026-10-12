// Detalle de una instancia: cabecera con acciones y pestañas.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/app_info.dart';
import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../console/console_view.dart';
import '../doctor/doctor_dialog.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/launch_feedback.dart';
import '../widgets/play_button.dart';
import 'backups_tab.dart';
import 'change_version_dialog.dart';
import 'content_tab.dart';
import 'instance_settings_tab.dart';
import 'screenshots_tab.dart';
import 'worlds_tab.dart';

enum InstanceTab { content, worlds, screenshots, backups, logs, settings }

InstanceTab parseTab(String? s) => InstanceTab.values.firstWhere(
      (t) => t.name == s,
      orElse: () => InstanceTab.settings,
    );

class InstanceDetailPage extends ConsumerWidget {
  const InstanceDetailPage({super.key, required this.id, required this.tab});

  final String id;
  final InstanceTab tab;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final instances = ref.watch(instancesProvider);
    final i = ref.watch(instanceByIdProvider(id));
    ref.watch(instanceDataRefresherProvider);
    if (i == null) {
      return instances.isLoading
          ? const Center(child: CircularProgressIndicator())
          : EmptyState(
              icon: Symbols.help_rounded,
              title: l.instanceNotFound,
              body: '',
              action: OutlinedButton(
                onPressed: () => context.go('/instances'),
                child: Text(l.back),
              ),
            );
    }
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, 0, Space.x8, Space.x6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _Header(instance: i),
          const SizedBox(height: Space.x2),
          LaunchFailureNote(instanceId: i.id),
          const SizedBox(height: Space.x2),
          _TabBar(
            selected: tab,
            onSelect: (t) => context.go('/instances/$id?tab=${t.name}'),
          ),
          const SizedBox(height: Space.x4),
          Expanded(
            child: switch (tab) {
              InstanceTab.settings => InstanceSettingsTab(
                  key: ValueKey('settings-${i.id}'),
                  instance: i,
                ),
              InstanceTab.logs => ConsoleView(instanceId: i.id),
              InstanceTab.content => ContentTab(instance: i),
              InstanceTab.worlds => WorldsTab(instance: i),
              InstanceTab.screenshots => ScreenshotsTab(instance: i),
              InstanceTab.backups => BackupsTab(instance: i),
            },
          ),
        ],
      ),
    );
  }
}

class _Header extends ConsumerWidget {
  const _Header({required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final i = instance;
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Container(
      height: 168,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(Radii.panel),
        border: Border.all(color: c.borderStrong),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.panel - 1),
        child: Stack(
          fit: StackFit.expand,
          children: [
            BlockLandscape(seed: i.id, block: 16),
            DecoratedBox(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors: [
                    Colors.black.withValues(alpha: 0.72),
                    Colors.black.withValues(alpha: 0.15),
                  ],
                ),
              ),
            ),
            Positioned(
              top: Space.x3,
              left: Space.x3,
              child: TextButton.icon(
                style: TextButton.styleFrom(
                  foregroundColor: c.text,
                  backgroundColor: Colors.black.withValues(alpha: 0.25),
                  minimumSize: const Size(0, 32),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                ),
                onPressed: () => context.go('/instances'),
                icon: const Icon(Symbols.arrow_back_rounded, size: 18),
                label: Text(l.navInstances),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.x6,
                Space.x6,
                Space.x6,
                Space.x5,
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  InstanceIcon(id: i.id, name: i.name, size: 64),
                  const SizedBox(width: Space.x5),
                  Expanded(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.end,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          i.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.text.headlineMedium,
                        ),
                        const SizedBox(height: Space.x2),
                        Wrap(
                          spacing: Space.x2,
                          children: [
                            Tag(
                              i.loaderVersion == null
                                  ? versionLine(l, i.mc, i.loader)
                                  : '${versionLine(l, i.mc, i.loader)} ${i.loaderVersion}',
                              dot: loaderColor(i.loader),
                              strong: true,
                            ),
                            Tag(
                              formatPlaytime(l, i.playtimeSeconds),
                              icon: Symbols.schedule_rounded,
                              strong: true,
                            ),
                            Tag(
                              formatRelative(l, i.lastPlayed),
                              icon: Symbols.history_rounded,
                              strong: true,
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: Space.x4),
                  _HeaderIconButton(
                    icon: Symbols.folder_open_rounded,
                    tooltip: l.instanceOpenFolder,
                    onPressed: () => launchUrl(Uri.file(i.gameDir)),
                  ),
                  const SizedBox(width: Space.x2),
                  _MoreMenu(instance: i),
                  const SizedBox(width: Space.x3),
                  PlayButton(instanceId: i.id),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _HeaderIconButton extends StatelessWidget {
  const _HeaderIconButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => IconButton(
        tooltip: tooltip,
        style: IconButton.styleFrom(
          fixedSize: const Size.square(44),
          backgroundColor: Colors.black.withValues(alpha: 0.3),
          foregroundColor: context.colors.text,
          side: BorderSide(color: Colors.white.withValues(alpha: 0.16)),
        ),
        onPressed: onPressed,
        icon: Icon(icon, size: 20),
      );
}

class _MoreMenu extends ConsumerWidget {
  const _MoreMenu({required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return MenuAnchor(
      alignmentOffset: const Offset(0, 6),
      menuChildren: [
        MenuItemButton(
          leadingIcon: const Icon(Symbols.swap_vert_rounded, size: 18),
          onPressed: () => showChangeVersion(context, instance),
          child: Text(l.instanceChangeVersion),
        ),
        MenuItemButton(
          leadingIcon: const Icon(Symbols.stethoscope_rounded, size: 18),
          onPressed: () => showDoctor(context, instance),
          child: Text(l.doctorDiagnose),
        ),
        MenuItemButton(
          leadingIcon: const Icon(Symbols.folder_open_rounded, size: 18),
          onPressed: () => launchUrl(Uri.file(instance.gameDir)),
          child: Text(l.instanceOpenFolder),
        ),
        const Divider(height: 9),
        MenuItemButton(
          leadingIcon: Icon(Symbols.delete_rounded, size: 18, color: c.danger),
          onPressed: () => _delete(context, ref),
          child: Text(l.instanceDelete, style: TextStyle(color: c.danger)),
        ),
      ],
      builder: (context, controller, _) => _HeaderIconButton(
        icon: Symbols.more_horiz_rounded,
        tooltip: l.instanceMore,
        onPressed: () => controller.isOpen ? controller.close() : controller.open(),
      ),
    );
  }

  Future<void> _delete(BuildContext context, WidgetRef ref) async {
    final l = AppLocalizations.of(context);
    final ok = await confirm(
      context,
      title: l.instanceDeleteTitle(instance.name),
      body: l.instanceDeleteBody(kAppName),
      action: l.delete,
      danger: true,
    );
    if (!ok || !context.mounted) return;
    try {
      await ref.read(instancesProvider.notifier).delete(instance.id);
      if (context.mounted) {
        showToast(context, l.instanceDeleted(instance.name));
        context.go('/instances');
      }
    } catch (e) {
      if (context.mounted) showToast(context, l.errGeneric, error: true);
    }
  }
}

class _TabBar extends StatelessWidget {
  const _TabBar({required this.selected, required this.onSelect});

  final InstanceTab selected;
  final ValueChanged<InstanceTab> onSelect;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    String label(InstanceTab t) => switch (t) {
          InstanceTab.content => l.tabContent,
          InstanceTab.worlds => l.tabWorlds,
          InstanceTab.screenshots => l.tabScreenshots,
          InstanceTab.backups => l.tabBackups,
          InstanceTab.logs => l.tabLogs,
          InstanceTab.settings => l.tabSettings,
        };
    IconData icon(InstanceTab t) => switch (t) {
          InstanceTab.content => Symbols.extension_rounded,
          InstanceTab.worlds => Symbols.public_rounded,
          InstanceTab.screenshots => Symbols.photo_library_rounded,
          InstanceTab.backups => Symbols.cloud_sync_rounded,
          InstanceTab.logs => Symbols.terminal_rounded,
          InstanceTab.settings => Symbols.tune_rounded,
        };
    return Container(
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: c.border)),
      ),
      child: Row(
        children: [
          for (final t in InstanceTab.values)
            _TabItem(
              label: label(t),
              icon: icon(t),
              selected: t == selected,
              onTap: () => onSelect(t),
            ),
        ],
      ),
    );
  }
}

class _TabItem extends StatelessWidget {
  const _TabItem({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final color = selected ? c.text : c.textMuted;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.vertical(top: Radius.circular(Radii.control)),
      child: Container(
        padding: const EdgeInsets.fromLTRB(Space.x3, Space.x3, Space.x4, Space.x3),
        decoration: BoxDecoration(
          border: Border(
            bottom: BorderSide(
              color: selected ? c.accent : Colors.transparent,
              width: 2,
            ),
          ),
        ),
        child: Row(
          children: [
            Icon(icon, size: 18, color: selected ? c.accent : color),
            const SizedBox(width: Space.x2),
            Text(
              label,
              style: context.text.labelLarge!.copyWith(
                color: color,
                fontWeight: selected ? FontWeight.w600 : FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
