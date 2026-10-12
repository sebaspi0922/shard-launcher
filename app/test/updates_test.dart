// Actualizaciones: estados del controlador. Las goldens del aviso y de
// Ajustes están en `goldens_test.dart`.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shard/core/bridge.dart';
import 'package:shard/core/providers.dart';
import 'package:shard/core/updates.dart';

import 'support/fake_bridge.dart';
import 'support/updates.dart';

ProviderContainer _container(FakeBridge bridge) {
  final c = ProviderContainer(overrides: [bridgeProvider.overrideWithValue(bridge)]);
  addTearDown(c.dispose);
  c.listen(updateProvider, (_, _) {});
  return c;
}

void main() {
  test('consulta, instala y queda listo para reiniciar', () async {
    final bridge = FakeBridge()..update = appImageUpdate;
    final c = _container(bridge);
    final ctrl = c.read(updateProvider.notifier);

    await ctrl.check();
    expect(c.read(updateProvider).status, UpdateStatus.available);
    await ctrl.install();
    final s = c.read(updateProvider);
    expect(s.status, UpdateStatus.ready);
    expect(s.path, '/tmp/Shard.AppImage');
  });

  test('sin versión nueva y fallos de instalación', () async {
    final bridge = FakeBridge();
    final c = _container(bridge);
    final ctrl = c.read(updateProvider.notifier);

    await ctrl.check();
    expect(c.read(updateProvider).status, UpdateStatus.upToDate);

    bridge
      ..update = appImageUpdate
      ..installedPath = null;
    await ctrl.check();
    await ctrl.install();
    final s = c.read(updateProvider);
    expect(s.status, UpdateStatus.failed);
    expect((s.error as BridgeError).kind, 'update_rejected');
    // Se puede reintentar: la versión sigue a mano.
    expect(s.info?.version, '0.2.0.r40');
  });

  test('una consulta automática que falla no se muestra', () async {
    final bridge = FakeBridge()..checkError = const BridgeError(kind: 'network', message: 'x');
    final c = _container(bridge);
    final ctrl = c.read(updateProvider.notifier);

    await ctrl.check(manual: false);
    expect(c.read(updateProvider).status, UpdateStatus.idle);
    await ctrl.check();
    expect(c.read(updateProvider).status, UpdateStatus.failed);
  });

  test('un aviso cerrado no vuelve por la misma versión', () async {
    final bridge = FakeBridge()..update = appImageUpdate;
    final c = _container(bridge);
    final ctrl = c.read(updateProvider.notifier);

    await ctrl.check(manual: false);
    ctrl.dismiss();
    await ctrl.check(manual: false);
    expect(c.read(updateProvider).dismissed, isTrue);
    // Otra versión sí avisa.
    bridge.update = const UpdateView(
      version: '0.3.0',
      current: '0.1.0.r27',
      canInstall: true,
      downloadUrl: 'x',
    );
    await ctrl.check(manual: false);
    expect(c.read(updateProvider).dismissed, isFalse);
  });
}
