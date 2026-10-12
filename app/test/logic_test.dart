// Lógica sin UI: niveles de log, formato y máquina de estados de «Jugar».

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:lcore_app/core/bridge.dart';
import 'package:lcore_app/core/format.dart';
import 'package:lcore_app/core/launch.dart';
import 'package:lcore_app/core/logs.dart';
import 'package:lcore_app/core/providers.dart';
import 'package:lcore_app/l10n/app_localizations_es.dart';
import 'package:lcore_app/ui/theme/accent.dart';

import 'support/fake_bridge.dart';

void main() {
  group('niveles de log', () {
    test('formato de Minecraft con hilo', () {
      expect(parseLogLevel('[12:00:01] [Render thread/WARN]: x'), LogLevel.warn);
      expect(parseLogLevel('[12:00:01] [main/ERROR]: x'), LogLevel.error);
      expect(parseLogLevel('[12:00:01] [main/INFO]: x'), LogLevel.info);
      expect(parseLogLevel('[12:00:01 DEBUG]: x'), LogLevel.debug);
    });

    test('las continuaciones heredan el nivel', () {
      expect(
        parseLogLevel('\tat foo.Bar(Bar.java:1)', previous: LogLevel.error),
        LogLevel.error,
      );
      expect(parseLogLevel('Caused by: x', previous: LogLevel.warn), LogLevel.warn);
    });

    test('el anillo se acota', () {
      final b = LogBuffer();
      b.addAll([
        for (var i = 0; i < kLogCapacity + 10; i++)
          LogLineView(text: 'l$i', stderr: false),
      ]);
      expect(b.length, kLogCapacity);
      expect(b.dropped, 10);
      expect(b.lines.first.text, 'l10');
    });
  });

  group('formato', () {
    final l = AppLocalizationsEs();
    test('tiempo jugado', () {
      expect(formatPlaytime(l, BigInt.zero), '0 min');
      expect(formatPlaytime(l, BigInt.from(45 * 60)), '45 min');
      expect(formatPlaytime(l, BigInt.from(3 * 3600 + 5 * 60)), '3 h 5 min');
    });
    test('tiempo relativo', () {
      final now = DateTime(2026, 10, 11, 12);
      int ago(Duration d) => now.subtract(d).millisecondsSinceEpoch ~/ 1000;
      expect(formatRelative(l, null, now: now), 'Nunca');
      expect(formatRelative(l, ago(const Duration(hours: 3)), now: now), 'hace 3 horas');
      expect(formatRelative(l, ago(const Duration(days: 1)), now: now), 'ayer');
    });
    test('memoria', () {
      expect(formatMemory(l, 4096), '4 GB');
      expect(formatMemory(l, 4608), '4,5 GB');
    });
  });

  test('acento desde píxeles', () async {
    final c = await accentFromPixels(List.filled(500, 0xFF2E7D32));
    expect(c, isNotNull);
  });

  group('lanzamiento', () {
    late FakeBridge bridge;
    late ProviderContainer container;
    const id = 'survival-amigos';

    setUp(() {
      bridge = FakeBridge();
      container = ProviderContainer(
        retry: (_, _) => null,
        overrides: [bridgeProvider.overrideWithValue(bridge)],
      );
      container.listen(launchProvider, (_, _) {});
    });
    tearDown(() => container.dispose());

    LaunchState state() => container.read(launchStateProvider(id));
    Future<void> flush() => Future<void>.delayed(Duration.zero);

    test('preparando → descargando → iniciando → en juego → inactivo', () async {
      bridge.launchCompleter = Completer();
      final f = container.read(launchProvider.notifier).launch(id);
      expect(state().phase, LaunchPhase.preparing);

      bridge.emit(UiEvent.progress(
        taskId: BigInt.one,
        stage: 'assets',
        done: BigInt.from(10),
        total: BigInt.from(40),
        bytes: false,
      ));
      await flush();
      expect(state().phase, LaunchPhase.downloading);
      expect(state().done, 10);
      expect(state().progress, 0.25);

      bridge.emit(UiEvent.gameStarted(gameId: BigInt.two, instanceId: id));
      await flush();
      expect(state().phase, LaunchPhase.starting);

      bridge.launchCompleter!.complete(GameView(
        gameId: BigInt.two,
        instanceId: id,
        startedAt: 0,
      ));
      await f;
      expect(state().phase, LaunchPhase.starting);

      bridge.emit(UiEvent.gameLog(gameId: BigInt.two, lines: const [
        LogLineView(text: '[x] [Render thread/INFO]: Sound engine started', stderr: false),
      ]));
      await flush();
      expect(state().phase, LaunchPhase.running);

      bridge.emit(UiEvent.gameExited(
        gameId: BigInt.two,
        instanceId: id,
        code: 0,
        secondsPlayed: BigInt.from(60),
      ));
      await flush();
      expect(state().phase, LaunchPhase.idle);
    });

    test('un error del núcleo deja «Reintentar»', () async {
      bridge.launchError = const BridgeError(kind: 'java_required', message: 'x');
      await container.read(launchProvider.notifier).launch(id);
      expect(state().phase, LaunchPhase.error);
      expect((state().failure as LaunchError).error, isA<BridgeError>());
      container.read(launchProvider.notifier).dismiss(id);
      expect(state().phase, LaunchPhase.idle);
    });

    test('salida con código distinto de 0 es un fallo', () async {
      await container.read(launchProvider.notifier).launch(id);
      bridge.emit(UiEvent.gameExited(
        gameId: BigInt.one,
        instanceId: id,
        code: 1,
        secondsPlayed: BigInt.zero,
      ));
      await flush();
      expect(state().phase, LaunchPhase.error);
      expect((state().failure as GameCrash).code, 1);
    });
  });
}
