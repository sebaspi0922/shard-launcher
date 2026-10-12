// Arranque de la app en tests: fuentes reales, ventana de 1280×800 y puente
// falso.

import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:lcore_app/app.dart';
import 'package:lcore_app/core/format.dart';
import 'package:lcore_app/core/providers.dart';
import 'package:material_ui/material_ui.dart';

import 'fake_bridge.dart';

bool _fontsLoaded = false;

/// Carga todas las fuentes del FontManifest (Inter, JetBrains Mono, iconos).
Future<void> loadAppFonts() async {
  if (_fontsLoaded) return;
  final manifest = json.decode(
    await rootBundle.loadString('FontManifest.json'),
  ) as List<dynamic>;
  for (final entry in manifest.cast<Map<String, dynamic>>()) {
    final loader = FontLoader(entry['family'] as String);
    for (final font in (entry['fonts'] as List).cast<Map<String, dynamic>>()) {
      loader.addFont(rootBundle.load(font['asset'] as String));
    }
    await loader.load();
  }
  _fontsLoaded = true;
}

/// Monta la app en `location` y devuelve el contenedor de Riverpod.
Future<ProviderContainer> pumpApp(
  WidgetTester tester, {
  required String location,
  FakeBridge? bridge,
  Size size = const Size(1280, 800),
}) async {
  await loadAppFonts();
  appClock = () => kTestNow;
  debugDisableShadows = false;
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  final b = bridge ?? FakeBridge();
  await tester.pumpWidget(
    ProviderScope(
      retry: (_, _) => null,
      overrides: [
        bridgeProvider.overrideWithValue(b),
        initialLocationProvider.overrideWithValue(location),
      ],
      child: const LauncherApp(),
    ),
  );
  await settle(tester);
  return ProviderScope.containerOf(tester.element(find.byType(LauncherApp)));
}

/// Deja pasar microtareas y transiciones (sin esperar a animaciones infinitas).
Future<void> settle(WidgetTester tester) async {
  for (var i = 0; i < 6; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  await tester.pump(const Duration(milliseconds: 400));
}

/// Contexto bajo el navegador, para abrir diálogos en tests.
BuildContext pageContext(WidgetTester tester) =>
    tester.element(find.byType(Scaffold).first);
