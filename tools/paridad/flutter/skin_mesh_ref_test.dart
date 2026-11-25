// Referencias del visor de skins para el port a Rust (ui/src/skin_mesh.rs):
// para cada caso, cuántos cuadriláteros dibuja `buildSkinMesh` y la imagen
// que pinta `SkinViewer` a 1× sobre fondo transparente. El test de Rust
// `skin_mesh::tests::igual_que_flutter` compara las dos cosas.
//
// No vive en app/test para no tocar la app. Regenerar (desde la raíz):
//   cp tools/paridad/flutter/skin_mesh_ref_test.dart app/test/_skin_mesh_ref_test.dart
//   (cd app && flutter test test/_skin_mesh_ref_test.dart)
//   rm app/test/_skin_mesh_ref_test.dart
// Escribe ui/tests/data/skin_mesh_ref.json y ui/tests/data/skin_<caso>.png.

import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shard/core/skins.dart';
import 'package:shard/ui/skins/skin_mesh.dart';
import 'package:shard/ui/skins/skin_viewer.dart';

import 'support/sample_skin.dart';

// Relativo al test (golden) y a app/ (JSON).
const _goldenOut = '../../ui/tests/data';
const _jsonOut = '../ui/tests/data';

class _Case {
  const _Case(this.name, this.w, this.h, this.camera,
      {this.slim = true, this.cape = false, this.legacy = false, this.shadow = false});
  final String name;
  final double w;
  final double h;
  final SkinCamera camera;
  final bool slim;
  final bool cape;
  final bool legacy;
  final bool shadow;
}

const _cases = [
  _Case('frente', 400, 600, SkinCamera()),
  _Case('defecto', 446, 551, kDefaultSkinCamera, cape: true, shadow: true),
  _Case('espalda', 116, 275.5, SkinCamera(yaw: 3.14159265), cape: true),
  _Case('antigua', 300, 300, SkinCamera(yaw: 0.7, pitch: -0.3), slim: false, legacy: true),
];

SkinTexture _skin(_Case c) {
  final s = sampleSkin();
  return c.legacy ? SkinTexture(64, 32, s.argb.sublist(0, 64 * 32)) : s;
}

void main() {
  setUpAll(() => autoUpdateGoldenFiles = true);
  final out = <Map<String, Object>>[];

  for (final c in _cases) {
    testWidgets(c.name, (tester) async {
      tester.view.physicalSize = const Size(800, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      final skin = _skin(c);
      final cape = c.cape ? sampleCape() : null;
      final mesh = buildSkinMesh(
        skin: skin,
        cape: cape,
        slim: c.slim,
        camera: c.camera,
        size: Size(c.w, c.h),
      );
      await tester.pumpWidget(Directionality(
        textDirection: TextDirection.ltr,
        child: Align(
          alignment: Alignment.topLeft,
          child: RepaintBoundary(
            key: const ValueKey('viewer'),
            child: SizedBox(
              width: c.w,
              height: c.h,
              child: SkinViewer(
                skin: skin,
                cape: cape,
                slim: c.slim,
                camera: c.camera,
                interactive: false,
                shadow: c.shadow,
              ),
            ),
          ),
        ),
      ));
      await expectLater(find.byKey(const ValueKey('viewer')), matchesGoldenFile('$_goldenOut/skin_${c.name}.png'));
      out.add({
        'name': c.name,
        'w': c.w,
        'h': c.h,
        'yaw': c.camera.yaw,
        'pitch': c.camera.pitch,
        'slim': c.slim,
        'cape': c.cape,
        'legacy': c.legacy,
        'shadow': c.shadow,
        'quads': mesh.quads,
      });
    });
  }

  tearDownAll(() {
    File('$_jsonOut/skin_mesh_ref.json')
        .writeAsStringSync('${const JsonEncoder.withIndent('  ').convert({'cases': out})}\n');
  });
}
