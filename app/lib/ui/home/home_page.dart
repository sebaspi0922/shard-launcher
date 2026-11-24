// Inicio: tarjeta grande de «Jugar», instancias recientes y sesión.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../accounts/account_dialogs.dart';
import '../accounts/account_panel.dart';
import '../instances/create_instance_dialog.dart';
import '../servers/server_dialog.dart';
import '../servers/server_widgets.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/avatar.dart';
import '../widgets/common.dart';
import '../widgets/launch_feedback.dart';
import '../widgets/play_button.dart';

class HomePage extends ConsumerWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final account = ref.watch(activeAccountProvider);
    final instances = ref.watch(instancesProvider);
    final home = ref.watch(homeInstanceProvider);
    final recent = ref.watch(recentInstancesProvider)
        .where((i) => i.id != home?.id)
        .take(3)
        .toList();

    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.x8, Space.x1, Space.x8, Space.x8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            account == null ? l.homeGreetingAnon : l.homeGreeting(account.name),
            style: context.text.headlineMedium,
          ),
          const SizedBox(height: Space.x1),
          Text(
            l.homeSubtitle,
            style: context.text.bodyLarge!.copyWith(color: c.textSecondary),
          ),
          const SizedBox(height: Space.x6),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: switch (instances) {
                    AsyncError(:final error) => Center(child: ErrorNote(error)),
                    AsyncData() when home == null => const _NoInstancesCard(),
                    AsyncData() => Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Expanded(child: _HeroCard(instance: home!)),
                          if (recent.isNotEmpty) ...[
                            const SizedBox(height: Space.x6),
                            SectionTitle(
                              l.homeRecent,
                              trailing: TextButton(
                                onPressed: () => context.go('/instances'),
                                child: Text(l.homeSeeAll),
                              ),
                            ),
                            Row(
                              children: [
                                for (final (i, inst) in recent.indexed) ...[
                                  if (i > 0) const SizedBox(width: Space.x4),
                                  Expanded(child: _RecentCard(instance: inst)),
                                ],
                                for (var i = recent.length; i < 3; i++) ...[
                                  const SizedBox(width: Space.x4),
                                  const Expanded(child: SizedBox()),
                                ],
                              ],
                            ),
                          ],
                        ],
                      ),
                    _ => const Center(child: CircularProgressIndicator()),
                  },
                ),
                const SizedBox(width: Space.x6),
                const SizedBox(
                  width: 296,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _SessionCard(),
                      SizedBox(height: Space.x4),
                      _RunningCard(),
                      SizedBox(height: Space.x4),
                      _QuickServersCard(),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _HeroCard extends ConsumerWidget {
  const _HeroCard({required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final launch = ref.watch(launchStateProvider(instance.id));
    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(Radii.panel + 4),
        border: Border.all(color: c.borderStrong),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.35),
            blurRadius: 30,
            offset: const Offset(0, 12),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.panel + 3),
        child: Stack(
          fit: StackFit.expand,
          children: [
            BlockLandscape(seed: instance.id, block: 18),
            // Velo lateral para que el texto se lea sobre el paisaje.
            DecoratedBox(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors: [
                    Colors.black.withValues(alpha: 0.6),
                    Colors.black.withValues(alpha: 0.0),
                  ],
                  stops: const [0.0, 0.75],
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(Space.x8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      InstanceIcon(id: instance.id, name: instance.name, size: 40),
                      const SizedBox(width: Space.x3),
                      Text(
                        l.homeContinue.toUpperCase(),
                        style: context.text.labelSmall!.copyWith(
                          color: c.accent,
                          letterSpacing: 1.6,
                        ),
                      ),
                    ],
                  ),
                  const Spacer(),
                  Text(
                    instance.name,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: context.text.displaySmall!.copyWith(
                      shadows: [
                        Shadow(
                          color: Colors.black.withValues(alpha: 0.4),
                          blurRadius: 12,
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: Space.x3),
                  Wrap(
                    spacing: Space.x2,
                    runSpacing: Space.x2,
                    children: [
                      Tag(
                        versionLine(l, instance.mc, instance.loader),
                        dot: loaderColor(instance.loader),
                        strong: true,
                      ),
                      Tag(
                        formatPlaytime(l, instance.playtimeSeconds),
                        icon: Symbols.schedule_rounded,
                        strong: true,
                      ),
                      Tag(
                        instance.lastPlayed == null
                            ? l.homeNeverPlayed
                            : formatRelative(l, instance.lastPlayed),
                        icon: Symbols.history_rounded,
                        strong: true,
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.x6),
                  Row(
                    children: [
                      PlayButton(
                        instanceId: instance.id,
                        size: PlayButtonSize.large,
                      ),
                      const SizedBox(width: Space.x3),
                      const _InstancePicker(),
                    ],
                  ),
                  if (launch.phase == LaunchPhase.error) ...[
                    const SizedBox(height: Space.x4),
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 520),
                      child: LaunchFailureNote(instanceId: instance.id),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _InstancePicker extends ConsumerWidget {
  const _InstancePicker();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final list = ref.watch(recentInstancesProvider);
    final current = ref.watch(homeInstanceProvider);
    return MenuAnchor(
      alignmentOffset: const Offset(0, 6),
      menuChildren: [
        for (final i in list)
          MenuItemButton(
            leadingIcon: InstanceIcon(id: i.id, name: i.name, size: 26),
            trailingIcon: i.id == current?.id
                ? Icon(Symbols.check_rounded, size: 18, color: c.accent)
                : null,
            onPressed: () => ref.read(homeSelectionProvider.notifier).select(i.id),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Space.x2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(i.name, style: context.text.bodyMedium),
                  Text(
                    versionLine(l, i.mc, i.loader),
                    style: context.text.bodySmall,
                  ),
                ],
              ),
            ),
          ),
      ],
      builder: (context, controller, _) => SizedBox(
        height: 56,
        child: OutlinedButton.icon(
          style: OutlinedButton.styleFrom(
            backgroundColor: Colors.black.withValues(alpha: 0.3),
            side: BorderSide(color: Colors.white.withValues(alpha: 0.18)),
            shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
            padding: const EdgeInsets.symmetric(horizontal: Space.x5),
          ),
          onPressed: () =>
              controller.isOpen ? controller.close() : controller.open(),
          icon: const Icon(Symbols.swap_horiz_rounded, size: 20),
          label: Text(l.homeChangeInstance),
        ),
      ),
    );
  }
}

class _RecentCard extends ConsumerWidget {
  const _RecentCard({required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Material(
      color: c.card(c.surface1),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(color: c.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => context.go('/instances/${instance.id}'),
        child: Padding(
          padding: const EdgeInsets.all(Space.x3),
          child: Row(
            children: [
              InstanceIcon(id: instance.id, name: instance.name, size: 44),
              const SizedBox(width: Space.x3),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      instance.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: context.text.titleSmall,
                    ),
                    const SizedBox(height: 2),
                    Text(
                      versionLine(l, instance.mc, instance.loader),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: context.text.bodySmall,
                    ),
                    Text(
                      formatRelative(l, instance.lastPlayed),
                      maxLines: 1,
                      style: context.text.bodySmall!.copyWith(color: c.textMuted),
                    ),
                  ],
                ),
              ),
              _MiniPlay(instanceId: instance.id),
            ],
          ),
        ),
      ),
    );
  }
}

/// Botón redondo de jugar para tarjetas pequeñas.
class _MiniPlay extends ConsumerWidget {
  const _MiniPlay({required this.instanceId});

  final String instanceId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final s = ref.watch(launchStateProvider(instanceId));
    final c = context.colors;
    final l = AppLocalizations.of(context);
    if (s.busy) {
      return const SizedBox.square(
        dimension: 36,
        child: Padding(
          padding: EdgeInsets.all(9),
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      );
    }
    final running = s.phase == LaunchPhase.running;
    return IconButton.filledTonal(
      tooltip: running ? l.launchRunning : l.play,
      style: IconButton.styleFrom(
        backgroundColor: running ? c.success.withValues(alpha: 0.15) : c.accentContainer,
        foregroundColor: running ? c.success : c.onAccentContainer,
      ),
      onPressed: running
          ? null
          : () => ref.read(launchProvider.notifier).launch(instanceId),
      icon: Icon(
        running ? Symbols.sports_esports_rounded : Symbols.play_arrow_rounded,
        fill: 1,
        size: 20,
      ),
    );
  }
}

class _NoInstancesCard extends StatelessWidget {
  const _NoInstancesCard();

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Panel(
      radius: Radii.panel + 4,
      child: EmptyState(
        icon: Symbols.deployed_code_rounded,
        title: l.homeNoInstancesTitle,
        body: l.homeNoInstancesBody,
        action: FilledButton.icon(
          onPressed: () => showCreateInstance(context),
          icon: const Icon(Symbols.add_rounded, size: 20),
          label: Text(l.instancesNew),
        ),
      ),
    );
  }
}

class _SessionCard extends ConsumerWidget {
  const _SessionCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final account = ref.watch(activeAccountProvider);
    return Panel(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l.homeSession.toUpperCase(),
            style: context.text.labelSmall!.copyWith(
              color: c.textMuted,
              letterSpacing: 1.2,
            ),
          ),
          const SizedBox(height: Space.x3),
          if (account == null) ...[
            Text(l.accountNoneHint, style: context.text.bodyMedium),
            const SizedBox(height: Space.x3),
            FilledButton.icon(
              onPressed: () => showMicrosoftLogin(context),
              icon: const Icon(Symbols.add_circle_rounded, size: 18),
              label: Text(l.accountsAddMicrosoft),
            ),
            const SizedBox(height: Space.x2),
            OutlinedButton(
              onPressed: () => showShardAccount(context),
              child: Text(l.accountShard),
            ),
            const SizedBox(height: Space.x2),
            OutlinedButton(
              onPressed: () => showOfflineAccount(context),
              child: Text(l.accountsAddOffline),
            ),
          ] else
            Row(
              children: [
                PlayerAvatar(account: account, size: 44),
                const SizedBox(width: Space.x3),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(account.name, style: context.text.titleMedium),
                      const SizedBox(height: 2),
                      Row(
                        children: [
                          Icon(
                            account.microsoft
                                ? Symbols.verified_rounded
                                : Symbols.wifi_off_rounded,
                            size: 14,
                            color: account.microsoft ? c.info : c.textMuted,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            account.shard
                                ? l.accountShard
                                : account.microsoft
                                ? l.accountMicrosoft
                                : l.accountOffline,
                            style: context.text.bodySmall,
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                TextButton(
                  onPressed: () => showAccountPanel(context),
                  child: Text(l.homeChangeAccount),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

class _RunningCard extends ConsumerWidget {
  const _RunningCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final running = ref
        .watch(launchProvider)
        .entries
        .where((e) => e.value.phase == LaunchPhase.running)
        .map((e) => e.key)
        .toList();
    return Panel(
      padding: const EdgeInsets.all(Space.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l.homeRunningNow.toUpperCase(),
            style: context.text.labelSmall!.copyWith(
              color: c.textMuted,
              letterSpacing: 1.2,
            ),
          ),
          const SizedBox(height: Space.x3),
          if (running.isEmpty)
            Row(
              children: [
                Icon(Symbols.bedtime_rounded, size: 18, color: c.textMuted),
                const SizedBox(width: Space.x2),
                Text(
                  l.homeNothingRunning,
                  style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
                ),
              ],
            )
          else
            for (final id in running)
              _RunningRow(instanceId: id),
        ],
      ),
    );
  }
}

class _RunningRow extends ConsumerWidget {
  const _RunningRow({required this.instanceId});

  final String instanceId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final i = ref.watch(instanceByIdProvider(instanceId));
    final c = context.colors;
    final l = AppLocalizations.of(context);
    return InkWell(
      borderRadius: BorderRadius.circular(Radii.control),
      onTap: () => context.go('/instances/$instanceId'),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.x1),
        child: Row(
          children: [
            InstanceIcon(id: instanceId, name: i?.name ?? instanceId, size: 32),
            const SizedBox(width: Space.x3),
            Expanded(
              child: Text(
                i?.name ?? instanceId,
                style: context.text.titleSmall,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            Container(
              width: 7,
              height: 7,
              decoration: BoxDecoration(color: c.success, shape: BoxShape.circle),
            ),
            const SizedBox(width: 6),
            Text(
              l.launchRunning,
              style: context.text.labelMedium!.copyWith(color: c.success),
            ),
          ],
        ),
      ),
    );
  }
}

class _QuickServersCard extends ConsumerWidget {
  const _QuickServersCard();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final servers = (ref.watch(serversProvider).value ?? const <ServerView>[])
        .take(3)
        .toList();
    final status = ref.watch(serverStatusProvider).value ?? const {};
    return Panel(
      padding: const EdgeInsets.fromLTRB(Space.x4, Space.x3, Space.x3, Space.x3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Text(
                l.homeQuickServers.toUpperCase(),
                style: context.text.labelSmall!.copyWith(
                  color: c.textMuted,
                  letterSpacing: 1.2,
                ),
              ),
              const Spacer(),
              TextButton(
                style: TextButton.styleFrom(
                  minimumSize: const Size(0, 28),
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                ),
                onPressed: () => context.go('/servers'),
                child: Text(l.homeSeeAllServers),
              ),
            ],
          ),
          const SizedBox(height: Space.x1),
          if (servers.isEmpty) ...[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(Symbols.dns_rounded, size: 18, color: c.textMuted),
                const SizedBox(width: Space.x2),
                Expanded(
                  child: Text(l.homeQuickServersEmpty, style: context.text.bodySmall),
                ),
              ],
            ),
            const SizedBox(height: Space.x3),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                style: OutlinedButton.styleFrom(minimumSize: const Size(0, 34)),
                onPressed: () => showServerDialog(context),
                icon: const Icon(Symbols.add_rounded, size: 18),
                label: Text(l.serverAdd),
              ),
            ),
          ] else
            for (final s in servers)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: Space.x1),
                child: Row(
                  children: [
                    ServerFavicon(server: s, status: status[s.address], size: 36),
                    const SizedBox(width: Space.x3),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            s.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.titleSmall,
                          ),
                          const SizedBox(height: 2),
                          Row(
                            children: [
                              PingBars(status: status[s.address]),
                              const SizedBox(width: 6),
                              Flexible(
                                child: Text(
                                  status[s.address]?.online ?? false
                                      ? '${serverPlayers(l, status[s.address])} · ${l.serverPing(status[s.address]!.latencyMs.toInt())}'
                                      : serverPlayers(l, status[s.address]),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: context.text.bodySmall,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                    JoinButton(server: s, compact: true),
                  ],
                ),
              ),
        ],
      ),
    );
  }
}
