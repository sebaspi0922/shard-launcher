// Interacciones básicas: cuenta offline, guardar ajustes de instancia y
// navegación por el raíl.

import 'package:flutter_test/flutter_test.dart';
import 'package:lcore_app/ui/accounts/account_dialogs.dart';
import 'package:lcore_app/ui/instances/instances_page.dart';
import 'package:lcore_app/ui/settings/settings_page.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_bridge.dart';
import 'support/harness.dart';

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
    await tester.tap(find.byTooltip('Instancias'));
    await settle(tester);
    expect(find.byType(InstancesPage), findsOneWidget);
    await tester.tap(find.byTooltip('Ajustes'));
    await settle(tester);
    expect(find.byType(SettingsPage), findsOneWidget);
    debugDisableShadows = true;
  });
}
