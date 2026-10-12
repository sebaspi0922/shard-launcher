// Interacciones básicas: cuenta offline, guardar ajustes de instancia y
// navegación por el raíl.

import 'package:flutter_test/flutter_test.dart';
import 'package:shard/ui/accounts/account_dialogs.dart';
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
