// Aviso global cuando una partida se cierra mal (crash report o código de
// salida distinto de 0), con acceso directo a Doctor.

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

class CrashNotice {
  const CrashNotice({
    required this.gameId,
    required this.instanceId,
    this.code,
    this.crashReport,
  });

  final BigInt gameId;
  final String instanceId;
  final int? code;
  final String? crashReport;
}

class CrashNoticeController extends Notifier<CrashNotice?> {
  /// Partidas ya avisadas (llegan `GameCrashed` y `GameExited` de la misma).
  final Set<BigInt> _seen = {};

  @override
  CrashNotice? build() {
    listenEvents(ref, (e) {
      switch (e) {
        case UiEvent_GameCrashed(:final gameId, :final instanceId, :final crashReport):
          _show(CrashNotice(
            gameId: gameId,
            instanceId: instanceId,
            crashReport: crashReport,
          ));
        case UiEvent_GameExited(:final gameId, :final instanceId, :final code)
            when code != null && code != 0:
          _show(CrashNotice(gameId: gameId, instanceId: instanceId, code: code));
        case UiEvent_GameStarted(:final instanceId):
          // Una partida nueva de la misma instancia deja el aviso viejo sin sentido.
          if (state?.instanceId == instanceId) state = null;
        default:
          break;
      }
    });
    return null;
  }

  void _show(CrashNotice n) {
    if (!_seen.add(n.gameId)) return;
    state = n;
  }

  void dismiss() => state = null;

  /// Solo para tests y goldens.
  void debugShow(CrashNotice n) => state = n;
}

final crashNoticeProvider =
    NotifierProvider<CrashNoticeController, CrashNotice?>(
  CrashNoticeController.new,
);
