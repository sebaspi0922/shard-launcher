// Máquina de estados del botón «Jugar» por instancia:
// Preparando → Descargando x/y → Iniciando → En juego → (inactivo | error).

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'logs.dart';
import 'providers.dart';

enum LaunchPhase { idle, preparing, downloading, starting, running, error }

/// Motivo de un fallo de lanzamiento o de partida.
sealed class LaunchFailure {
  const LaunchFailure();
}

/// El núcleo rechazó el lanzamiento (`BridgeError` u otra excepción).
class LaunchError extends LaunchFailure {
  const LaunchError(this.error);
  final Object error;
}

/// El juego se cerró con un crash report o un código distinto de 0.
class GameCrash extends LaunchFailure {
  const GameCrash({this.code, this.crashReport});
  final int? code;
  final String? crashReport;
}

class LaunchState {
  const LaunchState({
    this.phase = LaunchPhase.idle,
    this.done = 0,
    this.total = 0,
    this.stage,
    this.gameId,
    this.failure,
  });

  final LaunchPhase phase;
  final int done;
  final int total;
  final String? stage;
  final BigInt? gameId;
  final LaunchFailure? failure;

  static const idle = LaunchState();

  bool get busy =>
      phase == LaunchPhase.preparing ||
      phase == LaunchPhase.downloading ||
      phase == LaunchPhase.starting;

  double? get progress => total > 0 ? (done / total).clamp(0, 1) : null;
}

/// Líneas que indican que la ventana del juego ya está en pie.
final _readyLine = RegExp(
  r'Sound engine started|OpenAL initialized|Backend library: LWJGL|'
  r'Created: \d+x\d+|Reloading ResourceManager',
);

/// Tras este tiempo en «Iniciando» se da el juego por abierto.
const _startingTimeout = Duration(seconds: 20);

/// Etapas de progreso que emite la preparación de un lanzamiento (versión,
/// librerías, assets y Java). Las de contenido, modpacks o backups no cuentan.
const kLaunchStages = {'libraries', 'assets', 'java'};

class LaunchController extends Notifier<Map<String, LaunchState>> {
  /// Instancias preparándose, por orden de lanzamiento.
  final List<String> _pending = [];

  /// Tarea de progreso → instancia que la reclamó.
  final Map<BigInt, String> _taskOwner = {};
  final Map<BigInt, String> _games = {};
  final Map<String, Timer> _timers = {};

  @override
  Map<String, LaunchState> build() {
    listenEvents(ref, _onEvent);
    ref.onDispose(() {
      for (final t in _timers.values) {
        t.cancel();
      }
    });
    unawaited(_restore());
    return const {};
  }

  LaunchState of(String id) => state[id] ?? LaunchState.idle;

  void _set(String id, LaunchState s) {
    state = {...state, id: s};
  }

  /// Recupera las partidas que siguen abiertas al arrancar la app.
  Future<void> _restore() async {
    try {
      final games = await ref.read(bridgeProvider).runningGames();
      if (!ref.mounted) return;
      for (final g in games) {
        _games[g.gameId] = g.instanceId;
        ref.read(logHubProvider).track(g.gameId, g.instanceId);
        _set(
          g.instanceId,
          LaunchState(phase: LaunchPhase.running, gameId: g.gameId),
        );
      }
    } catch (_) {
      // Sin partidas que recuperar.
    }
  }

  /// Lanza la instancia. `server` entra directo a ese servidor; `account`
  /// fuerza una cuenta (si no, la activa).
  Future<void> launch(
    String instanceId, {
    String? server,
    String? account,
  }) async {
    if (of(instanceId).busy || of(instanceId).phase == LaunchPhase.running) {
      return;
    }
    _pending.add(instanceId);
    _set(instanceId, const LaunchState(phase: LaunchPhase.preparing));
    try {
      final accounts = await ref.read(accountsProvider.future);
      if (!ref.mounted) return;
      final acc = accounts.any((a) => !a.shard && a.id == account)
          ? account
          : ref.read(activeAccountProvider)?.id;
      if (acc == null) throw const BridgeError(kind: 'account_not_found', message: '');
      final game = await ref
          .read(bridgeProvider)
          .launchInstance(instanceId, account: acc, server: server);
      if (!ref.mounted) return;
      _games[game.gameId] = instanceId;
      // GameStarted puede haber llegado antes; no retroceder de «En juego».
      if (of(instanceId).phase != LaunchPhase.running) {
        _starting(instanceId, game.gameId);
      }
    } catch (e) {
      if (!ref.mounted) return;
      _set(
        instanceId,
        LaunchState(phase: LaunchPhase.error, failure: LaunchError(e)),
      );
    } finally {
      _pending.remove(instanceId);
      _taskOwner.removeWhere((_, owner) => owner == instanceId);
    }
  }

  /// Instancia a la que pertenece una tarea de progreso, si es de un
  /// lanzamiento. Una tarea nueva de las etapas de lanzamiento la reclama la
  /// primera instancia en preparación que no tenga ya otra tarea activa (cada
  /// lanzamiento descarga por pasos); si todas tienen una, la más antigua.
  String? _ownerOf(BigInt taskId, String stage) {
    final known = _taskOwner[taskId];
    if (known != null) return known;
    if (!kLaunchStages.contains(stage)) return null;
    final candidates = _pending.where((id) {
      final p = of(id).phase;
      return p == LaunchPhase.preparing || p == LaunchPhase.downloading;
    }).toList();
    if (candidates.isEmpty) return null;
    final busy = _taskOwner.values.toSet();
    final owner = candidates.firstWhere(
      (id) => !busy.contains(id),
      orElse: () => candidates.first,
    );
    _taskOwner[taskId] = owner;
    return owner;
  }

  /// Vuelve a inactivo (p. ej. al cerrar el aviso de error).
  void dismiss(String instanceId) {
    if (of(instanceId).phase == LaunchPhase.error) {
      _set(instanceId, LaunchState.idle);
    }
  }

  void _starting(String id, BigInt gameId) {
    _set(id, LaunchState(phase: LaunchPhase.starting, gameId: gameId));
    _timers.remove(id)?.cancel();
    _timers[id] = Timer(_startingTimeout, () => _running(id));
  }

  void _running(String id) {
    _timers.remove(id)?.cancel();
    final s = of(id);
    if (s.phase == LaunchPhase.starting) {
      _set(id, LaunchState(phase: LaunchPhase.running, gameId: s.gameId));
    }
  }

  void _onEvent(UiEvent e) {
    switch (e) {
      case UiEvent_Progress(:final taskId, :final done, :final total, :final stage):
        final id = _ownerOf(taskId, stage);
        if (id == null) return;
        final s = of(id);
        if (s.phase == LaunchPhase.preparing ||
            s.phase == LaunchPhase.downloading) {
          _set(
            id,
            LaunchState(
              phase: LaunchPhase.downloading,
              done: done.toInt(),
              total: total.toInt(),
              stage: stage,
            ),
          );
        }
      case UiEvent_TaskFinished(:final taskId):
        // Al terminar una descarga se vuelve a «Preparando» hasta la siguiente.
        final id = _taskOwner.remove(taskId);
        if (id != null && of(id).phase == LaunchPhase.downloading) {
          _set(id, const LaunchState(phase: LaunchPhase.preparing));
        }
      case UiEvent_GameStarted(:final gameId, :final instanceId):
        _games[gameId] = instanceId;
        if (of(instanceId).phase != LaunchPhase.running) {
          _starting(instanceId, gameId);
        }
      case UiEvent_GameLog(:final gameId, :final lines):
        final id = _games[gameId];
        if (id == null || of(id).phase != LaunchPhase.starting) return;
        if (lines.any((l) => _readyLine.hasMatch(l.text))) _running(id);
      case UiEvent_GameCrashed(:final gameId, :final instanceId, :final crashReport):
        _games.remove(gameId);
        _timers.remove(instanceId)?.cancel();
        _set(
          instanceId,
          LaunchState(
            phase: LaunchPhase.error,
            failure: GameCrash(crashReport: crashReport),
          ),
        );
      case UiEvent_GameExited(:final gameId, :final instanceId, :final code):
        _games.remove(gameId);
        _timers.remove(instanceId)?.cancel();
        if (of(instanceId).phase == LaunchPhase.error) return;
        final failed = code != null && code != 0;
        _set(
          instanceId,
          failed
              ? LaunchState(
                  phase: LaunchPhase.error,
                  failure: GameCrash(code: code),
                )
              : LaunchState.idle,
        );
      case UiEvent_AccountsChanged():
        break;
    }
  }

  /// Solo para tests y goldens: fija el estado de una instancia.
  void debugSet(String instanceId, LaunchState s) => _set(instanceId, s);
}

final launchProvider =
    NotifierProvider<LaunchController, Map<String, LaunchState>>(
  LaunchController.new,
);

final launchStateProvider = Provider.family<LaunchState, String>(
  (ref, id) => ref.watch(launchProvider)[id] ?? LaunchState.idle,
);

/// Lanzamiento en curso más relevante (para la píldora de actividad).
final activeLaunchProvider = Provider<MapEntry<String, LaunchState>?>((ref) {
  MapEntry<String, LaunchState>? best;
  for (final e in ref.watch(launchProvider).entries) {
    if (e.value.busy) return e;
    if (e.value.phase == LaunchPhase.running) best ??= e;
  }
  return best;
});
