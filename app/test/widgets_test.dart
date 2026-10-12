// Interacciones básicas: cuenta offline, guardar ajustes de instancia y
// navegación por el raíl.

import 'package:flutter_test/flutter_test.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:shard/ui/accounts/account_dialogs.dart';
import 'package:shard/ui/home/home_page.dart';
import 'package:shard/ui/instances/instances_page.dart';
import 'package:shard/ui/settings/settings_page.dart';
import 'package:shard/ui/widgets/common.dart';
import 'package:shard/ui/widgets/side_tooltip.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';

/// Botón del raíl por su etiqueta (usa un tooltip lateral propio).
Finder findRail(String label) =>
    find.byWidgetPredicate((w) => w is SideTooltip && w.message == label);

void main() {
  tearDown(() => debugDisableShadows = true);

  testWidgets('cuenta offline: valida y añade', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/', bridge: bridge);
    showOfflineAccount(pageContext(tester));
    await settle(tester);

    await tester.enterText(find.byType(TextField), 'a!');
    await tester.tap(find.text('Añadir'));
    await settle(tester);
    expect(find.textContaining('Usa de 3 a 16'), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'Alex_99');
    await tester.tap(find.text('Añadir'));
    await settle(tester);
    expect(bridge.accounts.any((a) => a.name == 'Alex_99'), isTrue);
    expect(find.byType(OfflineAccountDialog), findsNothing);
    debugDisableShadows = true;
  });

  testWidgets('ajustes de instancia: guardar cambios', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(
      tester,
      location: '/instances/vanilla-puro?tab=settings',
      bridge: bridge,
    );
    expect(find.text('Hay cambios sin guardar'), findsNothing);
    await tester.enterText(find.widgetWithText(TextField, 'Vanilla puro'), 'Vanilla 2');
    await settle(tester);
    expect(find.text('Hay cambios sin guardar'), findsOneWidget);
    await tester.tap(find.text('Guardar'));
    await settle(tester);
    expect(bridge.instances.firstWhere((i) => i.id == 'vanilla-puro').name, 'Vanilla 2');
    expect(find.text('Hay cambios sin guardar'), findsNothing);
    debugDisableShadows = true;
  });

  testWidgets('el raíl navega entre secciones', (tester) async {
    await pumpApp(tester, location: '/');
    await tester.tap(findRail('Instancias'));
    await settle(tester);
    expect(find.byType(InstancesPage), findsOneWidget);
    await tester.tap(findRail('Ajustes'));
    await settle(tester);
    expect(find.byType(SettingsPage), findsOneWidget);
    debugDisableShadows = true;
  });

  testWidgets('al cambiar de sección las páginas no se superponen', (tester) async {
    await pumpApp(tester, location: '/');
    await tester.tap(findRail('Instancias'));
    // Opacidad efectiva de una página (producto de sus FadeTransition).
    double opacity(Type page) {
      final finder = find.byType(page);
      if (finder.evaluate().isEmpty) return 0;
      return tester
          .widgetList<FadeTransition>(
            find.ancestor(of: finder, matching: find.byType(FadeTransition)),
          )
          .fold(1.0, (v, f) => v * f.opacity.value);
    }

    for (var ms = 0; ms <= 300; ms += 20) {
      await tester.pump(const Duration(milliseconds: 20));
      final home = opacity(HomePage), list = opacity(InstancesPage);
      expect(home > 0.01 && list > 0.01, isFalse, reason: 'a los $ms ms: $home / $list');
    }
    await settle(tester);
    expect(opacity(InstancesPage), 1);
    debugDisableShadows = true;
  });

  testWidgets('en modo rendimiento el cambio de sección es instantáneo', (tester) async {
    await pumpApp(
      tester,
      location: '/',
      bridge: FakeBridge()..uiPrefs = '{"first_run_done":true,"performance":true}',
    );
    await tester.tap(findRail('Instancias'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 16));
    expect(find.byType(HomePage), findsNothing);
    expect(find.byType(InstancesPage), findsOneWidget);
    debugDisableShadows = true;
  });

  testWidgets('contenido: desactivar y actualizar todo', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(
      tester,
      location: '/instances/survival-amigos?tab=content',
      bridge: bridge,
    );
    await tester.tap(find.byType(Switch).first);
    await settle(tester);
    expect(bridge.content['survival-amigos']!.first.enabled, isFalse);

    await tester.tap(find.text('Actualizar todo'));
    await settle(tester);
    expect(bridge.updates, isEmpty);
    expect(find.text('Actualizar todo'), findsNothing);
    debugDisableShadows = true;
  });

  /// Imágenes de iconos de ejemplo en pantalla.
  Finder fixtureIcons() => find.byWidgetPredicate((w) {
        if (w is! Image) return false;
        final p = w.image;
        final inner = p is ResizeImage ? p.imageProvider : p;
        return inner is FileImage && inner.file.path.contains('fixtures/icons/');
      });

  /// Iconos de tipo (el puzzle de los mods) en las filas de la lista.
  Finder kindIcons() => find.descendant(
        of: find.byType(ListView),
        matching: find.byIcon(Symbols.extension_rounded),
      );

  testWidgets('contenido: cada archivo conocido muestra el icono de su proyecto',
      (tester) async {
    await pumpApp(tester, location: '/instances/survival-amigos?tab=content');
    // Fabric API, Sodium, Iris, Lithium y JourneyMap (las filas visibles).
    expect(fixtureIcons(), findsAtLeastNWidgets(5));
    expect(kindIcons(), findsNothing);
    debugDisableShadows = true;
  });

  testWidgets('contenido: sin red los iconos no estorban y queda el del tipo',
      (tester) async {
    final bridge = FakeBridge()..contentIconsFail = true;
    await pumpApp(
      tester,
      location: '/instances/survival-amigos?tab=content',
      bridge: bridge,
    );
    expect(find.text('Sodium'), findsOneWidget);
    expect(fixtureIcons(), findsNothing);
    expect(kindIcons(), findsAtLeastNWidgets(5));
    debugDisableShadows = true;
  });

  testWidgets('copias: restaurar solo mundos con copia previa', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(
      tester,
      location: '/instances/survival-amigos?tab=backups',
      bridge: bridge,
    );
    final before = bridge.backups.length;
    await tester.tap(find.text('Restaurar').at(1));
    await settle(tester);
    await tester.tap(find.text('Solo los mundos'));
    await settle(tester);
    await tester.tap(find.text('Restaurar mundos'));
    await settle(tester);
    expect(bridge.restored, ['backup-3:worlds']);
    expect(bridge.backups.length, before + 1);
    expect(bridge.backups.first.label, 'Antes de restaurar');
    debugDisableShadows = true;
  });

  testWidgets('servidores: entrar lanza con el servidor', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/servers', bridge: bridge);
    await tester.tap(find.text('Entrar').first);
    await settle(tester);
    expect(bridge.lastLaunch?.id, 'survival-amigos');
    expect(bridge.lastLaunch?.server, 'mc.amigos.example');
    debugDisableShadows = true;
  });

  testWidgets('contenido: instalar en la instancia de destino', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(
      tester,
      location: '/content?instance=survival-amigos&kind=mod',
      bridge: bridge,
    );
    final card = find.ancestor(
      of: find.textContaining('Entity Culling', findRichText: true),
      matching: find.byType(InkWell),
    );
    await tester.tap(
      find.descendant(of: card.first, matching: find.text('Instalar')),
    );
    await settle(tester);
    expect(bridge.installed, ['survival-amigos:entityculling']);
    debugDisableShadows = true;
  });

  testWidgets('ajustes: guardar la clave de CurseForge', (tester) async {
    final bridge = FakeBridge();
    await pumpApp(tester, location: '/settings', bridge: bridge);
    await tester.scrollUntilVisible(
      find.text('Añadir clave'),
      200,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.tap(find.text('Añadir clave'));
    await settle(tester);
    await tester.enterText(
      find.descendant(of: find.byType(AppDialog), matching: find.byType(TextField)),
      'clave-secreta',
    );
    await tester.pump();
    await tester.tap(find.text('Guardar'));
    await settle(tester);
    expect(bridge.curseforgeKey, isTrue);
    expect(find.text('Configurada'), findsOneWidget);
    expect(find.text('clave-secreta'), findsNothing);
    debugDisableShadows = true;
  });
}
