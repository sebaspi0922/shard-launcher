// Piezas comunes de servidores: favicon, señal de ping y botón «Entrar».

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/launch.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';

/// Favicon del servidor o un bloque generado a partir de la dirección.
class ServerFavicon extends ConsumerWidget {
  const ServerFavicon({
    super.key,
    required this.server,
    this.status,
    this.size = 48,
  });

  final ServerView server;
  final ServerStatusView? status;
  final double size;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final b64 = status?.faviconBase64;
    final bytes = b64 == null ? null : ref.watch(faviconProvider(b64));
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(size * 0.22),
        border: Border.all(color: Colors.white.withValues(alpha: 0.14)),
      ),
      clipBehavior: Clip.antiAlias,
      child: bytes != null
          ? Image.memory(
              bytes,
              fit: BoxFit.cover,
              filterQuality: FilterQuality.none,
              gaplessPlayback: true,
              errorBuilder: (_, _, _) =>
                  BlockLandscape(seed: server.address, block: size / 6),
            )
          : BlockLandscape(seed: server.address, block: size / 6),
    );
  }
}

/// Barras de señal según la latencia.
class PingBars extends StatelessWidget {
  const PingBars({super.key, required this.status});

  final ServerStatusView? status;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final s = status;
    final ms = s?.latencyMs.toInt() ?? 0;
    final online = s?.online ?? false;
    final level = !online
        ? 0
        : ms < 60
            ? 4
            : ms < 120
                ? 3
                : ms < 250
                    ? 2
                    : 1;
    final color = !online
        ? c.textMuted
        : level >= 3
            ? c.success
            : level == 2
                ? c.warning
                : c.danger;
    return Row(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        for (var i = 0; i < 4; i++) ...[
          if (i > 0) const SizedBox(width: 2),
          Container(
            width: 3.5,
            height: 5.0 + i * 3,
            decoration: BoxDecoration(
              color: i < level ? color : c.surface3,
              borderRadius: BorderRadius.circular(1),
            ),
          ),
        ],
      ],
    );
  }
}

/// Lanza la instancia del servidor entrando directamente a él.
Future<void> joinServer(WidgetRef ref, ServerView s) async {
  final id = s.instanceId;
  if (id == null) return;
  await ref
      .read(launchProvider.notifier)
      .launch(id, server: s.address, account: s.accountId);
}

/// Botón «Entrar» con el estado del lanzamiento de su instancia.
class JoinButton extends ConsumerWidget {
  const JoinButton({super.key, required this.server, this.compact = false});

  final ServerView server;
  final bool compact;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final id = server.instanceId;
    final instance = id == null ? null : ref.watch(instanceByIdProvider(id));
    final launch = id == null ? LaunchState.idle : ref.watch(launchStateProvider(id));
    final running = launch.phase == LaunchPhase.running;
    final tooltip = instance == null
        ? l.serverNoInstance
        : running
            ? l.serverAlreadyRunning(instance.name)
            : l.serverJoinWith(instance.name);
    final onPressed = instance == null || launch.busy || running
        ? null
        : () => joinServer(ref, server);
    final child = launch.busy
        ? SizedBox.square(
            dimension: 16,
            child: CircularProgressIndicator(strokeWidth: 2, color: c.onAccentContainer),
          )
        : Icon(
            running ? Symbols.sports_esports_rounded : Symbols.login_rounded,
            size: 18,
          );
    if (compact) {
      return IconButton.filledTonal(
        tooltip: tooltip,
        style: IconButton.styleFrom(
          backgroundColor: c.accentContainer,
          foregroundColor: c.onAccentContainer,
        ),
        onPressed: onPressed,
        icon: child,
      );
    }
    return Tooltip(
      message: tooltip,
      child: FilledButton.icon(
        onPressed: onPressed,
        icon: child,
        label: Text(running ? l.launchRunning : l.serverJoin),
      ),
    );
  }
}

/// «3 / 20» jugadores o «Sin conexión».
String serverPlayers(AppLocalizations l, ServerStatusView? s) {
  if (s == null) return '…';
  if (!s.online) return l.serverOffline;
  String n(int v) => v >= 10000 ? formatCompact(l, BigInt.from(v)) : '$v';
  return l.serverPlayersText(n(s.playersOnline), n(s.playersMax));
}
