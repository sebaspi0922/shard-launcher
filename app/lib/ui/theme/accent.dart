// Color de acento: preset elegido o extraído de una imagen de fondo.

import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_color_utilities/material_color_utilities.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/ui_prefs.dart';

/// Presets propios del acento.
const kAccentPresets = <Color>[
  Color(0xFF35D69B), // jade (por defecto)
  Color(0xFF4FA8FF), // azul glaciar
  Color(0xFF9B7BFF), // amatista
  Color(0xFFFF6F91), // coral
  Color(0xFFFFA94D), // ámbar
  Color(0xFF3CC8E0), // turquesa
  Color(0xFFE5C04A), // oro viejo
  Color(0xFFE879C6), // orquídea
];

const kDefaultAccent = Color(0xFF35D69B);

/// Acento del paisaje generado (cuando el acento es automático).
const kLandscapeAccent = Color(0xFFFFA94D);

/// Acento extraído de una imagen de fondo local.
final imageAccentProvider = FutureProvider.family<Color?, String>((ref, path) async {
  final file = File(path);
  if (!await file.exists()) return null;
  return accentFromImageBytes(await file.readAsBytes());
});

/// Acento actual: el preset elegido o, en automático, el del fondo.
final accentProvider = Provider<Color>((ref) {
  final p = ref.watch(uiPrefsProvider);
  final preset = Color(p.accentPreset);
  if (!p.accentAuto) return preset;
  return switch (p.background) {
    BackgroundKind.image when p.backgroundPath != null =>
      ref.watch(imageAccentProvider(p.backgroundPath!)).value ?? preset,
    BackgroundKind.landscape => kLandscapeAccent,
    _ => kDefaultAccent,
  };
});

/// Color dominante «bonito» de una lista de píxeles ARGB (cuantizador + puntuación
/// de material_color_utilities). `null` si la imagen no tiene color útil.
Future<Color?> accentFromPixels(List<int> argb) async {
  if (argb.isEmpty) return null;
  final result = await QuantizerCelebi().quantize(argb, 128);
  final ranked = Score.score(result.colorToCount, fallbackColorARGB: 0);
  final best = ranked.first;
  return best == 0 ? null : Color(best);
}

/// Igual que [accentFromPixels], desde los bytes de una imagen (PNG, JPEG…).
Future<Color?> accentFromImageBytes(Uint8List bytes) async {
  final codec = await ui.instantiateImageCodec(bytes, targetWidth: 96);
  final frame = await codec.getNextFrame();
  final data = await frame.image.toByteData(format: ui.ImageByteFormat.rawRgba);
  frame.image.dispose();
  if (data == null) return null;
  final pixels = <int>[];
  for (var i = 0; i + 3 < data.lengthInBytes; i += 4) {
    final a = data.getUint8(i + 3);
    if (a < 128) continue;
    pixels.add(
      (a << 24) |
          (data.getUint8(i) << 16) |
          (data.getUint8(i + 1) << 8) |
          data.getUint8(i + 2),
    );
  }
  return accentFromPixels(pixels);
}
