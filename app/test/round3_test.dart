// Ronda 3: skins, Doctor, versión y papelera, Java, apariencia, primer
// arranque y bandeja.

import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/crash_notice.dart';
import 'package:shard/core/desktop.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/core/skins.dart';
import 'package:shard/core/ui_prefs.dart';
import 'package:shard/core/window.dart';
import 'package:shard/ui/doctor/doctor_dialog.dart';
import 'package:shard/ui/first_run/first_run_page.dart';
import 'package:shard/ui/home/home_page.dart';
import 'package:shard/ui/instances/change_version_dialog.dart';
import 'package:shard/ui/instances/trash_dialog.dart';
import 'package:shard/ui/shell/tray_controller.dart';
import 'package:shard/ui/skins/skin_mesh.dart';
import 'package:shard/ui/theme/accent.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';
import 'support/sample_skin.dart';

/// `pumpApp` activa las sombras; el framework comprueba que vuelvan a su
/// valor antes de los `tearDown`.
void testUi(String name, Future<void> Function(WidgetTester) body) =>
    testWidgets(name, (tester) async {
      try {
        await body(tester);
      } finally {
        debugDisableShadows = true;
      }
    });

void main() {
  group('preferencias de la interfaz', () {
    test('ida y vuelta por JSON', () {
      const p = UiPrefs(
        accentAuto: true,
        accentPreset: 0xFF4FA8FF,
        background: BackgroundKind.image,
        backgroundPath: '/datos/backgrounds/fondo.png',
        blur: 20,
        dim: 0.3,
        performance: true,
        firstRunDone: true,
      );
      final back = UiPrefs.parse(jsonEncode(p.toJson()));
      expect(back.accentAuto, isTrue);
      expect(back.accentPreset, 0xFF4FA8FF);
      expect(back.background, BackgroundKind.image);
      expect(back.backgroundPath, '/datos/backgrounds/fondo.png');
      expect(back.blur, 20);
      expect(back.dim, closeTo(0.3, 1e-9));
      expect(back.performance, isTrue);
      expect(back.firstRunDone, isTrue);
    });

    test('valores dañados o fuera de rango', () {
      expect(UiPrefs.parse('no es json').background, BackgroundKind.aurora);
      final p = UiPrefs.parse('{"blur": 999, "dim": -1, "background": "x"}');
      expect(p.blur, 40);
      expect(p.dim, 0);
      expect(p.background, BackgroundKind.aurora);
      expect(p.firstRunDone, isNull);
    });

    test('cuándo se muestra el asistente', () {
      const unset = UiPrefs();
      expect(needsFirstRun(unset, hasInstances: false, hasAccounts: false), isTrue);
      expect(needsFirstRun(unset, hasInstances: true, hasAccounts: false), isFalse);
      expect(
        needsFirstRun(const UiPrefs(firstRunDone: false), hasInstances: true, hasAccounts: true),
        isTrue,
      );
      expect(
        needsFirstRun(const UiPrefs(firstRunDone: true), hasInstances: false, hasAccounts: false),
        isFalse,
      );
    });

    test('el acento sigue al preset o al fondo', () {
      final bridge = FakeBridge()..uiPrefs = '{"accent":"preset","accent_preset":${0xFF9B7BFF}}';
      final c = ProviderContainer(overrides: [bridgeProvider.overrideWithValue(bridge)]);
      addTearDown(c.dispose);
      expect(c.read(accentProvider), const Color(0xFF9B7BFF));
      c.read(uiPrefsProvider.notifier).update(
            (p) => p.copyWith(accentAuto: true, background: BackgroundKind.landscape),
          );
      expect(c.read(accentProvider), kLandscapeAccent);
      expect(jsonDecode(bridge.uiPrefs)['accent'], 'auto');
    });
  });

  group('skin', () {
    test('textura: plantilla antigua y fuera de rango', () {
      final t = SkinTexture(64, 32, sampleCape().argb);
      expect(t.legacy, isTrue);
      expect(t.at(-1, 0), 0);
      expect(t.at(64, 0), 0);
      expect(isValidSkinSize(t), isTrue);
      expect(isValidSkinSize(SkinTexture(32, 32, sampleCape().argb)), isFalse);
    });

    test('URL de textura segura', () {
      expect(
        secureTextureUrl('http://textures.minecraft.net/texture/abc'),
        'https://textures.minecraft.net/texture/abc',
      );
      expect(secureTextureUrl('https://x.y/z.png'), 'https://x.y/z.png');
    });

    test('de frente solo se ven las caras frontales', () {
      final skin = sampleSkin();
      const size = Size(400, 600);
      final front = buildSkinMesh(
        skin: skin,
        slim: true,
        camera: const SkinCamera(),
        size: size,
      );
      final turned = buildSkinMesh(
        skin: skin,
        slim: true,
        camera: const SkinCamera(yaw: -0.6, pitch: 0.2),
        size: size,
      );
      // Cara 8×8 + cuerpo 8×12 + brazos 2×(3×12) + piernas 2×(4×12),
      // más lo opaco de las capas exteriores.
      expect(front.quads, greaterThanOrEqualTo(64 + 96 + 72 + 96));
      // Girado se ven también laterales y la parte de arriba.
      expect(turned.quads, greaterThan(front.quads));
      // Con capa, de espaldas se dibuja también la capa.
      final back = buildSkinMesh(
        skin: skin,
        cape: sampleCape(),
        slim: true,
        camera: const SkinCamera(yaw: 3.14159),
        size: size,
      );
      expect(back.quads, greaterThan(front.quads));
    });
  });

  test('aviso de cierre: una sola vez por partida', () async {
    final bridge = FakeBridge();
    final c = ProviderContainer(overrides: [bridgeProvider.overrideWithValue(bridge)]);
    addTearDown(c.dispose);
    c.listen(crashNoticeProvider, (_, _) {});
    bridge.emit(UiEvent.gameCrashed(
      gameId: BigInt.one,
      instanceId: 'survival-amigos',
      crashReport: '/x/crash.txt',
    ));
    await Future<void>.delayed(Duration.zero);
    expect(c.read(crashNoticeProvider)?.instanceId, 'survival-amigos');
    c.read(crashNoticeProvider.notifier).dismiss();
    bridge.emit(UiEvent.gameExited(
      gameId: BigInt.one,
      instanceId: 'survival-amigos',
      code: 1,
      secondsPlayed: BigInt.from(30),
    ));
    await Future<void>.delayed(Duration.zero);
    expect(c.read(crashNoticeProvider), isNull);
    // Salida limpia: sin aviso.
    bridge.emit(UiEvent.gameExited(
      gameId: BigInt.two,
      instanceId: 'vanilla-puro',
      code: 0,
      secondsPlayed: BigInt.from(30),
    ));
    await Future<void>.delayed(Duration.zero);
    expect(c.read(crashNoticeProvider), isNull);
  });

  testUi('Doctor aplica un arreglo y vuelve a analizar', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(tester, location: '/', bridge: bridge);
    showDoctor(pageContext(tester), c.read(instanceByIdProvider('survival-amigos'))!);
    await settle(tester);
    expect(find.text('Falta una dependencia de un mod'), findsOneWidget);
    await tester.tap(find.text('Aplicar arreglo').first);
    await settle(tester);
    expect(bridge.appliedFixes, ['mod_dependency']);
    expect(find.text('Falta una dependencia de un mod'), findsNothing);
  });

  testUi('cambiar versión hace copia antes', (tester) async {
    final bridge = FakeBridge();
    final c = await pumpApp(tester, location: '/instances', bridge: bridge);
    showChangeVersion(pageContext(tester), c.read(instanceByIdProvider('vanilla-puro'))!);
    await settle(tester);
    await tester.tap(find.text('1.21.1'));
    await settle(tester);
    await tester.tap(find.widgetWithText(FilledButton, 'Cambiar versión'));
    await settle(tester);
    expect(bridge.versionChanges.single.$2, '1.21.1');
    expect(bridge.backups.first.label, contains('1.21.4'));
  });

  testUi('papelera: recuperar', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/instances', bridge: bridge);
    showTrash(pageContext(tester));
    await settle(tester);
    await tester.tap(find.text('Recuperar').first);
    await settle(tester);
    expect(bridge.instances.any((i) => i.id == 'pruebas-viejas'), isTrue);
    expect(bridge.trash.length, 1);
  });

  testUi('skins: elegir capa', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/skins', bridge: bridge);
    await tester.tap(find.text('Aniversario'));
    await settle(tester);
    expect(bridge.profile.capes.firstWhere((c) => c.id == 'c2').active, isTrue);
    await tester.tap(find.text('Sin capa'));
    await settle(tester);
    expect(bridge.profile.capes.any((c) => c.active), isFalse);
  });

  testUi('skins: cambiar solo el modelo reaplica la textura', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/skins', bridge: bridge);
    await tester.tap(find.text('Clásica').last);
    await settle(tester);
    await tester.tap(find.text('Aplicar skin'));
    await settle(tester);
    expect(bridge.skinCalls.single, startsWith('url:http://textures.minecraft.net/'));
    expect(bridge.skinCalls.single, endsWith(':false'));
  });

  testUi('apariencia: elegir el paisaje se guarda', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/settings', bridge: bridge);
    await tester.drag(find.text('General'), const Offset(0, -300));
    await settle(tester);
    await tester.tap(find.text('Paisaje'));
    await settle(tester);
    expect(jsonDecode(bridge.uiPrefs)['background'], 'landscape');
  });

  testUi('Java: instalar una versión', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/settings', bridge: bridge);
    await tester.drag(find.text('General'), const Offset(0, -900));
    await settle(tester);
    await tester.tap(find.text('Java 25'));
    await settle(tester);
    expect(bridge.javas.any((j) => j.major == 25), isTrue);
  });

  testUi('primer arranque: saltar marca el asistente como hecho', (tester) async {
    final bridge = FakeBridge(instances: [], accounts: [])..uiPrefs = '{}';
    await pumpApp(tester, location: '/welcome', bridge: bridge);
    await tester.tap(find.text('Saltar asistente'));
    await settle(tester);
    expect(jsonDecode(bridge.uiPrefs)['first_run_done'], isTrue);
    expect(find.byType(HomePage), findsOneWidget);
  });

  testUi('primer arranque: importar Roxy y .minecraft', (tester) async {
    final bridge = FakeBridge(instances: [], accounts: []);
    await pumpApp(
      tester,
      location: '/welcome',
      bridge: bridge,
      overrides: [firstRunInitialStepProvider.overrideWithValue(FirstRunStep.import)],
    );
    await tester.tap(find.text('Launcher oficial (.minecraft)'));
    await settle(tester);
    await tester.tap(find.text('Importar').last);
    await settle(tester);
    expect(bridge.imported, ['roxy', 'minecraft']);
    expect(find.text('3 instancias importadas'), findsOneWidget);
  });

  testUi('cerrar con «bandeja» oculta la ventana y deja el icono', (tester) async {
    final bridge = FakeBridge()..closeBehavior = 'tray';
    final window = NoopWindowControls();
    final tray = NoopTrayHost();
    await pumpApp(
      tester,
      location: '/',
      bridge: bridge,
      overrides: [
        windowControlsProvider.overrideWithValue(window),
        trayHostProvider.overrideWithValue(tray),
      ],
    );
    // El icono se pinta en otro hilo: dejarlo terminar fuera del reloj falso.
    await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 200)));
    await settle(tester);
    await tester.runAsync(window.requestClose);
    await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 100)));
    await settle(tester);
    expect(window.hidden, isTrue);
    expect(tray.shown?.playLabel, 'Jugar «Survival con amigos»');
    tray.shown!.onOpen();
    await settle(tester);
    expect(window.hidden, isFalse);
  });

  testUi('renderTrayIcon devuelve un PNG', (tester) async {
    final png = await tester.runAsync(() => renderTrayIcon(kDefaultAccent));
    expect(png, isNotNull);
    expect(png!.sublist(1, 4), [0x50, 0x4E, 0x47]); // «PNG»
  });

  test('autoarranque inerte en tests', () async {
    final a = NoopAutostart();
    await a.setEnabled(true);
    expect(await a.isEnabled(), isTrue);
  });
}
