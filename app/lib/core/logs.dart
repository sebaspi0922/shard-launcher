// Registros de juego por instancia, alimentados por los eventos `GameLog`.
// Cada instancia guarda las últimas líneas de su partida más reciente.

import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

/// Igual que el anillo del núcleo (`logs::DEFAULT_LOG_CAPACITY`).
const kLogCapacity = 5000;

enum LogLevel { trace, debug, info, warn, error, other }

class LogEntry {
  const LogEntry(this.text, this.level, {this.stderr = false});
  final String text;
  final LogLevel level;
  final bool stderr;
}

// `[12:34:56] [Render thread/INFO]`, `[12:34:56 WARN]`, `[main/ERROR]`…
final _levelRe = RegExp(
  r'^\s*(?:\[[^\]]*\]\s*)?\[(?:[^\]/]*/)?(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL|SEVERE)\]',
);
final _bareLevelRe = RegExp(
  r'\b(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL|SEVERE)\]?:',
);
final _continuationRe = RegExp(r'^(\s+at |\s*Caused by: |\s*\.\.\. \d+ more|\t)');

LogLevel _parseName(String s) => switch (s) {
      'TRACE' => LogLevel.trace,
      'DEBUG' => LogLevel.debug,
      'INFO' => LogLevel.info,
      'WARN' || 'WARNING' => LogLevel.warn,
      'ERROR' || 'FATAL' || 'SEVERE' => LogLevel.error,
      _ => LogLevel.other,
    };

/// Nivel de una línea. Las continuaciones (stack traces) heredan el anterior.
LogLevel parseLogLevel(String line, {LogLevel? previous, bool stderr = false}) {
  final m = _levelRe.firstMatch(line) ?? _bareLevelRe.firstMatch(line);
  if (m != null) return _parseName(m.group(1)!);
  if (previous != null && _continuationRe.hasMatch(line)) return previous;
  if (line.contains('Exception') || line.contains('Error:')) {
    return LogLevel.error;
  }
  return stderr ? LogLevel.warn : LogLevel.other;
}

/// Líneas de una instancia (anillo acotado).
class LogBuffer extends ChangeNotifier {
  final ListQueue<LogEntry> _lines = ListQueue();
  BigInt? gameId;

  /// Líneas descartadas por el anillo desde el inicio de la partida.
  int dropped = 0;

  int get length => _lines.length;
  List<LogEntry> get lines => List.unmodifiable(_lines);

  void reset(BigInt? game) {
    gameId = game;
    _lines.clear();
    dropped = 0;
    notifyListeners();
  }

  void addAll(Iterable<LogLineView> views) {
    var prev = _lines.isEmpty ? null : _lines.last.level;
    for (final v in views) {
      final level = parseLogLevel(v.text, previous: prev, stderr: v.stderr);
      _lines.add(LogEntry(v.text, level, stderr: v.stderr));
      prev = level;
      if (_lines.length > kLogCapacity) {
        _lines.removeFirst();
        dropped++;
      }
    }
    notifyListeners();
  }

  void clear() => reset(gameId);
}

/// Reparte los `GameLog` entre los búferes de cada instancia.
class LogHub {
  LogHub(Ref ref) {
    listenEvents(ref, _onEvent);
  }

  final Map<String, LogBuffer> _buffers = {};
  final Map<BigInt, String> _games = {};

  LogBuffer of(String instanceId) =>
      _buffers.putIfAbsent(instanceId, LogBuffer.new);

  /// Asocia una partida que ya estaba abierta al arrancar la app.
  void track(BigInt gameId, String instanceId) {
    _games[gameId] = instanceId;
    of(instanceId).gameId ??= gameId;
  }

  void _onEvent(UiEvent e) {
    switch (e) {
      case UiEvent_GameStarted(:final gameId, :final instanceId):
        _games[gameId] = instanceId;
        of(instanceId).reset(gameId);
      case UiEvent_GameLog(:final gameId, :final lines):
        final id = _games[gameId];
        if (id != null) of(id).addAll(lines);
      default:
        break;
    }
  }

  void dispose() {
    for (final b in _buffers.values) {
      b.dispose();
    }
  }
}

final logHubProvider = Provider<LogHub>((ref) {
  final hub = LogHub(ref);
  ref.onDispose(hub.dispose);
  return hub;
});

final logBufferProvider = Provider.family<LogBuffer, String>(
  (ref, id) => ref.watch(logHubProvider).of(id),
);
