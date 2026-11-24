// Acento que Flutter saca de unas imágenes (accent.dart), para comparar con
// ui/src/accent.rs. Como parity_ref_test.dart, se copia a app/test para
// correrlo:
//   cp tools/paridad/flutter/accent_ref_test.dart app/test/_accent_ref_test.dart
//   (cd app && flutter test test/_accent_ref_test.dart); rm app/test/_accent_ref_test.dart
// Imprime «archivo 0xAARRGGBB» por imagen.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:shard/ui/theme/accent.dart';

const images = [
  'test/fixtures/icons/complementary-reimagined.png',
  'test/fixtures/icons/fabulously-optimized.png',
  'test/fixtures/icons/terralith.png',
  'test/fixtures/icons/journeymap.png',
  'test/goldens/home_style_landscape_es.png',
  'test/goldens/home_style_aurora_es.png',
];

void main() {
  testWidgets('acento de imágenes', (tester) async {
    for (final path in images) {
      final bytes = File(path).readAsBytesSync();
      final c = await tester.runAsync(() => accentFromImageBytes(bytes));
      // ignore: avoid_print
      print('ACCENT $path 0x${c?.toARGB32().toRadixString(16).toUpperCase()}');
    }
  });
}
