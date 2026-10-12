// Servidores: lista editable y reordenable con estado en vivo (MOTD,
// jugadores, ping y favicon), instancia por servidor y «Entrar».

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import 'server_dialog.dart';
import 'server_widgets.dart';

class ServersPage extends ConsumerWidget {
  const ServersPage({super.key});

  Future<void> _import(BuildContext context, WidgetRef ref, InstanceView i) async {
    final l = AppLocalizations.of(context);
    try {
      final n = await ref.read(serversProvider.notifier).importFrom(i.id);
      if (context.mounted) {
        showToast(context, n == 0 ? l.serversImportNone(i.name) : l.serversImported(n, i.name));
      }
    } catch (e) {
      if (context.mounted) showToast(context, errorText(l, e), error: true);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(serversProvider);
    final servers = async.value ?? const <ServerView>[];
    final status = ref.watch(serverStatusProvider);
    final instances = ref.watch(recentInstancesProvider);

    final importMenu = MenuAnchor(
      alignmentOffset: const Offset(0, 6),
      menuChildren: [
        for (final i in instances)
          MenuItemButton(
            leadingIcon: InstanceIcon(id: i.id, name: i.name, size: 24),
            onPressed: () => _import(context, ref, i),
            child: Text(i.name),
          ),
      ],
      builder: (context, controller, _) => OutlinedButton.icon(
        onPressed: instances.isEmpty
            ? null
            : () => controller.isOpen ? controller.close() : controller.open(),
        icon: const Icon(Symbols.download_rounded, size: 18),
        label: Text(l.serversImport),
      ),
    );

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
                    Text(l.serversTitle, style: context.text.headlineMedium),
                    const SizedBox(height: Space.x1),
                    Text(
                      servers.isEmpty
                          ? l.serversSubtitle
                          : l.serversCount(
                              servers.length,
                              status.value?.values.where((s) => s.online).length ?? 0,
                            ),
                      style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                    ),
                  ],
                ),
              ),
              IconButton(
                tooltip: l.serversRefresh,
                onPressed: status.isLoading
                    ? null
                    : () => ref.invalidate(serverStatusProvider),
                icon: status.isLoading
                    ? const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Symbols.refresh_rounded, size: 20),
              ),
              const SizedBox(width: Space.x2),
              importMenu,
              const SizedBox(width: Space.x3),
              FilledButton.icon(
                onPressed: () => showServerDialog(context),
                icon: const Icon(Symbols.add_rounded, size: 20),
                label: Text(l.serverAdd),
              ),
            ],
          ),
          const SizedBox(height: Space.x6),
          Expanded(
            child: servers.isEmpty
                ? switch (async) {
                    AsyncError(:final error) => Center(child: ErrorNote(error)),
                    AsyncLoading() => const Center(child: CircularProgressIndicator()),
                    _ => EmptyState(
                        icon: Symbols.dns_rounded,
                        title: l.serversEmptyTitle,
                        body: l.serversEmptyBody,
                        action: FilledButton.icon(
                          onPressed: () => showServerDialog(context),
                          icon: const Icon(Symbols.add_rounded, size: 20),
                          label: Text(l.serverAdd),
                        ),
                      ),
                  }
                : ReorderableListView.builder(
                    buildDefaultDragHandles: false,
                    padding: const EdgeInsets.only(bottom: Space.x8),
                    itemCount: servers.length,
                    proxyDecorator: (child, _, _) => Material(
                      color: Colors.transparent,
                      elevation: 8,
                      shadowColor: Colors.black54,
                      borderRadius: BorderRadius.circular(Radii.card),
                      child: child,
                    ),
                    onReorderItem: (from, to) {
                      ref
                          .read(serversProvider.notifier)
                          .move(servers[from].id, to)
                          .catchError((Object e) {
                        if (context.mounted) {
                          showToast(context, errorText(l, e), error: true);
                        }
                      });
                    },
                    itemBuilder: (context, i) {
                      final s = servers[i];
                      return Padding(
                        key: ValueKey(s.id),
                        padding: const EdgeInsets.only(bottom: Space.x3),
                        child: _ServerCard(
                          index: i,
                          server: s,
                          status: status.value?[s.address],
                          pinging: status.isLoading,
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _ServerCard extends ConsumerWidget {
  const _ServerCard({
    required this.index,
    required this.server,
    required this.status,
    required this.pinging,
  });

  final int index;
  final ServerView server;
  final ServerStatusView? status;
  final bool pinging;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final s = server;
    final st = status;
    final instance = s.instanceId == null ? null : ref.watch(instanceByIdProvider(s.instanceId!));
    final online = st?.online ?? false;
    final motd = st == null
        ? (pinging ? l.serverPinging : '')
        : online
            ? st.motd
            : l.serverUnreachable;

    return Container(
      decoration: BoxDecoration(
        color: c.surface1,
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: c.border),
      ),
      padding: const EdgeInsets.fromLTRB(Space.x2, Space.x3, Space.x3, Space.x3),
      child: Row(
        children: [
          ReorderableDragStartListener(
            index: index,
            child: MouseRegion(
              cursor: SystemMouseCursors.grab,
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 12),
                child: Icon(Symbols.drag_indicator_rounded, size: 20, color: c.textMuted),
              ),
            ),
          ),
          const SizedBox(width: Space.x1),
          ServerFavicon(server: s, status: st, size: 56),
          const SizedBox(width: Space.x4),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Flexible(
                      child: Text(
                        s.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: context.text.titleMedium,
                      ),
                    ),
                    const SizedBox(width: Space.x2),
                    Container(
                      width: 8,
                      height: 8,
                      decoration: BoxDecoration(
                        color: st == null
                            ? c.textMuted
                            : online
                                ? c.success
                                : c.danger,
                        shape: BoxShape.circle,
                      ),
                    ),
                    if (online && st!.versionName.isNotEmpty) ...[
                      const SizedBox(width: Space.x2),
                      Flexible(
                        child: Text(
                          st.versionName,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.text.bodySmall!.copyWith(color: c.textMuted),
                        ),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  s.address,
                  style: context.text.bodySmall!.copyWith(
                    fontFamily: kMonoFamily,
                    fontSize: 11.5,
                    color: c.textSecondary,
                  ),
                ),
                if (motd.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    motd,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.text.bodySmall!.copyWith(
                      color: online ? c.textSecondary : c.textMuted,
                    ),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(width: Space.x4),
          SizedBox(
            width: 150,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                Tooltip(
                  message: st?.sample.join('\n') ?? '',
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Symbols.group_rounded, size: 16, color: c.textMuted),
                      const SizedBox(width: 4),
                      Flexible(
                        child: Text(
                          serverPlayers(l, st),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.text.labelLarge!.copyWith(
                            color: online ? c.text : c.textMuted,
                            fontFeatures: const [FontFeature.tabularFigures()],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: Space.x1),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    PingBars(status: st),
                    const SizedBox(width: 6),
                    Text(
                      online ? l.serverPing(st!.latencyMs.toInt()) : '—',
                      style: context.text.bodySmall!.copyWith(color: c.textMuted),
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.x4),
          SizedBox(
            width: 190,
            child: instance == null
                ? Text(
                    l.serverNoInstance,
                    style: context.text.bodySmall!.copyWith(color: c.textMuted),
                  )
                : Row(
                    children: [
                      InstanceIcon(id: instance.id, name: instance.name, size: 28),
                      const SizedBox(width: Space.x2),
                      Expanded(
                        child: Text(
                          instance.name,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: context.text.bodySmall!.copyWith(color: c.textSecondary),
                        ),
                      ),
                    ],
                  ),
          ),
          const SizedBox(width: Space.x3),
          JoinButton(server: s),
          const SizedBox(width: Space.x1),
          MenuAnchor(
            alignmentOffset: const Offset(-140, 4),
            menuChildren: [
              MenuItemButton(
                leadingIcon: const Icon(Symbols.edit_rounded, size: 18),
                onPressed: () => showServerDialog(context, server: s),
                child: Text(l.serverEdit),
              ),
              MenuItemButton(
                leadingIcon: const Icon(Symbols.content_copy_rounded, size: 18),
                onPressed: () => copyText(context, s.address, l.copied),
                child: Text(l.serverCopyAddress),
              ),
              MenuItemButton(
                leadingIcon: Icon(Symbols.delete_rounded, size: 18, color: c.danger),
                onPressed: () async {
                  final ok = await confirm(
                    context,
                    title: l.serverRemoveTitle(s.name),
                    body: l.serverRemoveBody,
                    action: l.remove,
                    danger: true,
                  );
                  if (!ok) return;
                  try {
                    await ref.read(serversProvider.notifier).remove(s.id);
                  } catch (e) {
                    if (context.mounted) showToast(context, errorText(l, e), error: true);
                  }
                },
                child: Text(l.remove, style: TextStyle(color: c.danger)),
              ),
            ],
            builder: (context, controller, _) => IconButton(
              tooltip: l.instanceMore,
              onPressed: () => controller.isOpen ? controller.close() : controller.open(),
              icon: const Icon(Symbols.more_vert_rounded, size: 19),
            ),
          ),
        ],
      ),
    );
  }
}
