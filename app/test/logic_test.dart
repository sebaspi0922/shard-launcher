// Lógica sin UI: niveles de log, formato y máquina de estados de «Jugar».

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/format.dart';
import 'package:shard/core/launch.dart';
import 'package:shard/core/logs.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/l10n/app_localizations_es.dart';
import 'package:shard/ui/theme/accent.dart';

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
          LogLineView(text: 'l$i', stderr: false, seq: BigInt.from(i)),
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

      bridge.emit(UiEvent.gameLog(gameId: BigInt.two, lines: [
        LogLineView(
          text: '[x] [Render thread/INFO]: Sound engine started',
          stderr: false,
          seq: BigInt.zero,
        ),
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

  group('progreso del lanzamiento', () {
    late FakeBridge bridge;
    late ProviderContainer container;

    setUp(() {
      bridge = FakeBridge()..launchCompleter = Completer();
      container = ProviderContainer(
        retry: (_, _) => null,
        overrides: [bridgeProvider.overrideWithValue(bridge)],
      );
      container.listen(launchProvider, (_, _) {});
    });
    tearDown(() => container.dispose());

    LaunchState state(String id) => container.read(launchStateProvider(id));
    Future<void> flush() => Future<void>.delayed(Duration.zero);
    UiEvent progress(int task, String stage, int done) => UiEvent.progress(
          taskId: BigInt.from(task),
          stage: stage,
          done: BigInt.from(done),
          total: BigInt.from(100),
          bytes: false,
        );

    test('otras tareas (contenido, backup) no cuentan como descarga', () async {
      unawaited(container.read(launchProvider.notifier).launch('vanilla-puro'));
      bridge.emit(progress(7, 'backup', 50));
      bridge.emit(progress(8, 'content', 30));
      await flush();
      expect(state('vanilla-puro').phase, LaunchPhase.preparing);
      bridge.emit(progress(9, 'assets', 40));
      await flush();
      expect(state('vanilla-puro').phase, LaunchPhase.downloading);
      expect(state('vanilla-puro').done, 40);
      // El fin de una tarea ajena no devuelve a «Preparando».
      bridge.emit(UiEvent.taskFinished(taskId: BigInt.from(7), ok: true));
      await flush();
      expect(state('vanilla-puro').phase, LaunchPhase.downloading);
      bridge.emit(UiEvent.taskFinished(taskId: BigInt.from(9), ok: true));
      await flush();
      expect(state('vanilla-puro').phase, LaunchPhase.preparing);
    });

    test('dos lanzamientos a la vez reparten sus tareas', () async {
      unawaited(container.read(launchProvider.notifier).launch('vanilla-puro'));
      unawaited(container.read(launchProvider.notifier).launch('quilt-pruebas'));
      bridge.emit(progress(1, 'libraries', 10));
      bridge.emit(progress(2, 'libraries', 70));
      bridge.emit(progress(1, 'libraries', 20));
      await flush();
      expect(state('vanilla-puro').done, 20);
      expect(state('quilt-pruebas').done, 70);
    });

    const legacy = AccountView(
      id: 'legacy',
      name: 'Antigua',
      microsoft: false,
      shard: true,
      active: true,
    );

    test('un servidor con cuenta antigua usa una cuenta admitida', () async {
      bridge.accounts = [legacy, sampleAccounts().last];
      bridge.launchCompleter = null;
      await container
          .read(launchProvider.notifier)
          .launch('vanilla-puro', server: 'mc.example.net', account: legacy.id);
      expect(bridge.lastLaunch?.account, 'a2');
    });

    test('una cuenta antigua como única cuenta impide el lanzamiento', () async {
      bridge.accounts = [legacy];
      bridge.launchCompleter = null;
      await container.read(launchProvider.notifier).launch('vanilla-puro');
      expect(state('vanilla-puro').phase, LaunchPhase.error);
      expect(bridge.lastLaunch, isNull);
    });

    test('entrar a un servidor pasa la dirección y la cuenta', () async {
      bridge.launchCompleter = null;
      await container
          .read(launchProvider.notifier)
          .launch('vanilla-puro', server: 'mc.example.net', account: 'a2');
      expect(bridge.lastLaunch?.server, 'mc.example.net');
      expect(bridge.lastLaunch?.account, 'a2');
    });
  });

  group('histórico de registros', () {
    LogLineView line(int seq) =>
        LogLineView(text: 'l$seq', stderr: false, seq: BigInt.from(seq));

    test('se descartan duplicados y se detectan huecos', () {
      final b = LogBuffer()..addAll([line(0), line(1), line(2)]);
      b.addAll([line(2), line(3)]);
      expect(b.lines.map((e) => e.text), ['l0', 'l1', 'l2', 'l3']);
      expect(b.hasGap([line(4)]), isFalse);
      expect(b.hasGap([line(6)]), isTrue);
    });

    test('la consola abierta tarde recibe el histórico', () async {
      final bridge = FakeBridge();
      final game = BigInt.from(5);
      bridge.latestGames['survival-amigos'] = game;
      bridge.backlogs[game] = [for (var i = 0; i < 3; i++) line(i)];
      final container = ProviderContainer(
        overrides: [bridgeProvider.overrideWithValue(bridge)],
      );
      addTearDown(container.dispose);
      final buffer = container.read(logBufferProvider('survival-amigos'));
      expect(buffer.length, 3);
      expect(buffer.gameId, game);

      // Llegan líneas en vivo, algunas repetidas con el histórico.
      bridge.emit(UiEvent.gameLog(gameId: game, lines: [line(2), line(3)]));
      await Future<void>.delayed(Duration.zero);
      expect(buffer.length, 4);

      // Un hueco (se perdieron eventos) se rellena desde el histórico.
      bridge.backlogs[game] = [for (var i = 0; i < 8; i++) line(i)];
      bridge.emit(UiEvent.gameLog(gameId: game, lines: [line(7)]));
      await Future<void>.delayed(Duration.zero);
      expect(buffer.length, 8);
      expect(buffer.lines.last.text, 'l7');
    });
  });
}
