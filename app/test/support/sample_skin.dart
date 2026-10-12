// Skin y capa de muestra generadas en código (diseño propio) para tests y
// goldens: el visor recibe píxeles, sin decodificar imágenes.

import 'dart:typed_data';

import 'package:shard/core/skins.dart';

int _hash(int x, int y) {
  var h = (x * 374761393 + y * 668265263) & 0x7fffffff;
  h = ((h ^ (h >> 13)) * 1274126177) & 0x7fffffff;
  return h;
}

/// Color con algo de ruido para que parezca tela.
int _tone(int argb, int x, int y, {double amount = 0.07}) {
  final k = 1 + ((_hash(x, y) % 1000) / 1000 - 0.5) * 2 * amount;
  int ch(int shift) => (((argb >> shift) & 0xFF) * k).round().clamp(0, 255);
  return (argb & 0xFF000000) | (ch(16) << 16) | (ch(8) << 8) | ch(0);
}

class _Canvas {
  _Canvas(this.w, this.h) : px = Uint32List(w * h);
  final int w;
  final int h;
  final Uint32List px;

  void set(int x, int y, int argb, {bool noise = true}) {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    px[y * w + x] = noise ? _tone(argb, x, y) : argb;
  }

  void fill(int x, int y, int fw, int fh, int argb, {bool noise = true}) {
    for (var j = 0; j < fh; j++) {
      for (var i = 0; i < fw; i++) {
        set(x + i, y + j, argb, noise: noise);
      }
    }
  }

  /// Toda la UV de una caja (las seis caras).
  void box(int u, int v, int bw, int bh, int d, int argb) {
    fill(u + d, v, bw * 2, d, argb);
    fill(u, v + d, 2 * d + 2 * bw, bh, argb);
  }

  /// Franja horizontal en las cuatro caras laterales de una caja.
  void band(int u, int v, int bw, int d, int fromRow, int rows, int argb) {
    fill(u, v + d + fromRow, 2 * d + 2 * bw, rows, argb);
  }
}

const _skin = 0xFFE0AC86;
const _skinShade = 0xFFC98F6B;
const _hair = 0xFF3B2618;
const _hoodie = 0xFF2E8C83;
const _hoodieDark = 0xFF226860;
const _beanie = 0xFFD9573F;
const _pants = 0xFF2D3550;
const _shoe = 0xFF2A2A2E;
const _sole = 0xFFE8E4DA;

/// Skin 64×64 de brazos delgados: gorro rojo, sudadera turquesa y vaqueros.
SkinTexture sampleSkin() {
  final c = _Canvas(64, 64);
  // Cabeza.
  c.box(0, 0, 8, 8, 8, _skin);
  c.fill(8, 0, 8, 8, _hair);
  c.band(0, 0, 8, 8, 0, 2, _hair);
  c.fill(24, 8, 8, 6, _hair); // nuca
  c.fill(0, 8, 2, 5, _hair); // patilla derecha
  c.fill(22, 8, 2, 5, _hair); // patilla izquierda
  // Cara (frente en 8,8).
  for (final x in [1, 5]) {
    c.set(8 + x, 12, 0xFFFFFFFF, noise: false);
    c.set(9 + x, 12, 0xFF1F3A5F, noise: false);
  }
  c.fill(10, 11, 1, 1, _hair);
  c.fill(13, 11, 1, 1, _hair);
  c.fill(11, 14, 2, 1, 0xFF9C4A3C, noise: false);
  c.fill(10, 13, 4, 1, _skinShade);
  // Gorro (capa del sombrero en 32,0).
  c.fill(40, 0, 8, 8, _beanie);
  c.band(32, 0, 8, 8, 0, 3, _beanie);
  c.band(32, 0, 8, 8, 2, 1, 0xFFB8402D);
  c.set(43, 0, 0xFFF2E6D8, noise: false);
  c.set(44, 0, 0xFFF2E6D8, noise: false);
  // Cuerpo.
  c.box(16, 16, 8, 12, 4, _hoodie);
  c.fill(23, 20, 2, 12, _hoodieDark); // cremallera
  c.fill(21, 28, 6, 3, _hoodieDark); // bolsillo
  c.fill(22, 20, 4, 1, 0xFFF2E6D8); // cordones
  // Brazos (delgados: 3 de ancho).
  c.box(40, 16, 3, 12, 4, _hoodie);
  c.band(40, 16, 3, 4, 9, 3, _skin);
  c.fill(47, 16, 3, 4, _skin); // mano por abajo
  c.box(32, 48, 3, 12, 4, _hoodie);
  c.band(32, 48, 3, 4, 9, 3, _skin);
  c.fill(39, 48, 3, 4, _skin);
  // Piernas.
  c.box(0, 16, 4, 12, 4, _pants);
  c.band(0, 16, 4, 4, 9, 3, _shoe);
  c.band(0, 16, 4, 4, 11, 1, _sole);
  c.box(16, 48, 4, 12, 4, _pants);
  c.band(16, 48, 4, 4, 9, 3, _shoe);
  c.band(16, 48, 4, 4, 11, 1, _sole);
  // Capucha caída sobre la espalda (capa exterior del cuerpo).
  c.fill(32, 36, 8, 3, _hoodieDark);
  return SkinTexture(64, 64, c.px);
}

/// Capa 64×32: violeta con un rombo dorado.
SkinTexture sampleCape({int color = 0xFF4B2D7F, int emblem = 0xFFE5C04A}) {
  final c = _Canvas(64, 32);
  c.box(0, 0, 10, 16, 1, color);
  for (var y = 0; y < 16; y++) {
    for (var x = 0; x < 10; x++) {
      final dx = (x - 4.5).abs();
      final dy = (y - 7.5).abs();
      if (dx / 3.5 + dy / 5.5 <= 1) c.set(1 + x, 1 + y, emblem);
      if (y == 0 || y == 15) c.set(1 + x, 1 + y, emblem);
    }
  }
  return SkinTexture(64, 32, c.px);
}
