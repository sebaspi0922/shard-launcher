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

class LaunchController extends Notifier<Map<String, LaunchState>> {
  /// Instancia cuyo lanzamiento está en curso (recibe el progreso).
  String? _pending;
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

  Future<void> launch(String instanceId) async {
    if (of(instanceId).busy || of(instanceId).phase == LaunchPhase.running) {
      return;
    }
    _pending = instanceId;
    _set(instanceId, const LaunchState(phase: LaunchPhase.preparing));
    final account = ref.read(activeAccountProvider)?.id;
    try {
      final game = await ref
          .read(bridgeProvider)
          .launchInstance(instanceId, account: account);
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
      if (_pending == instanceId) _pending = null;
    }
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
      case UiEvent_Progress(:final done, :final total, :final stage):
        final id = _pending;
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
      case UiEvent_TaskFinished():
        // Al terminar una descarga se vuelve a «Preparando» hasta la siguiente.
        final id = _pending;
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
