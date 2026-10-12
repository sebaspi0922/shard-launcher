// Skins: perfil de la cuenta de Microsoft y texturas decodificadas a píxeles
// (el visor dibuja píxel a píxel, sin filtrado).

import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'bridge.dart';
import 'providers.dart';

/// Textura en ARGB (skin 64×64 / 64×32 o capa 64×32; las HD se muestrean).
class SkinTexture {
  SkinTexture(this.width, this.height, this.argb);

  final int width;
  final int height;
  final Uint32List argb;

  /// Factor respecto a la plantilla de 64 de ancho.
  int get scale => (width ~/ 64).clamp(1, 16);

  /// Plantilla antigua (64×32): sin capas en cuerpo y extremidades, y las
  /// extremidades izquierdas copian las derechas.
  bool get legacy => height * 2 == width;

  /// Píxel en coordenadas de la plantilla de 64 (0 = transparente).
  int at(int u, int v) {
    final x = u * scale;
    final y = v * scale;
    if (x < 0 || y < 0 || x >= width || y >= height) return 0;
    return argb[y * width + x];
  }

  /// Decodifica PNG (u otro formato) a píxeles.
  static Future<SkinTexture?> decode(Uint8List bytes) async {
    try {
      final codec = await ui.instantiateImageCodec(bytes);
      final frame = await codec.getNextFrame();
      final img = frame.image;
      final data = await img.toByteData(format: ui.ImageByteFormat.rawRgba);
      final w = img.width;
      final h = img.height;
      img.dispose();
      codec.dispose();
      if (data == null) return null;
      final out = Uint32List(w * h);
      for (var i = 0; i < out.length; i++) {
        final o = i * 4;
        out[i] = (data.getUint8(o + 3) << 24) |
            (data.getUint8(o) << 16) |
            (data.getUint8(o + 1) << 8) |
            data.getUint8(o + 2);
      }
      return SkinTexture(w, h, out);
    } catch (_) {
      return null;
    }
  }
}

/// Medidas válidas de una skin para subirla.
bool isValidSkinSize(SkinTexture t) =>
    t.width == 64 && (t.height == 64 || t.height == 32);

/// Las texturas de Mojang llegan por `http://`; la caché solo admite `https://`.
String secureTextureUrl(String url) =>
    url.startsWith('http://') ? 'https://${url.substring(7)}' : url;

/// Textura remota (skin o capa), descargada una vez al disco.
final skinTextureProvider = FutureProvider.family<SkinTexture?, String>(
  (ref, url) async {
    final path =
        await ref.watch(bridgeProvider).cachedImage(secureTextureUrl(url));
    if (path == null) return null;
    return SkinTexture.decode(await File(path).readAsBytes());
  },
);

/// Textura de un archivo local (vista previa antes de subir).
final localSkinTextureProvider =
    FutureProvider.autoDispose.family<SkinTexture?, String>(
  (ref, path) async {
    final f = File(path);
    if (!await f.exists() || await f.length() > 2 * 1024 * 1024) return null;
    return SkinTexture.decode(await f.readAsBytes());
  },
);

/// Perfil (skins y capas) de una cuenta de Microsoft, por id de cuenta.
class ProfileController extends AsyncNotifier<ProfileView> {
  ProfileController(this.accountId);

  final String accountId;

  LauncherBridge get _bridge => ref.read(bridgeProvider);

  @override
  Future<ProfileView> build() =>
      ref.watch(bridgeProvider).getProfile(account: accountId);

  Future<void> _run(Future<ProfileView> Function() call) async {
    final p = await call();
    if (ref.mounted) state = AsyncData(p);
    // La cara del avatar sale de la cuenta guardada: refrescarla.
    await ref.read(accountsProvider.notifier).refresh();
  }

  Future<void> uploadSkin(String path, {required bool slim}) =>
      _run(() => _bridge.uploadSkin(path, account: accountId, slim: slim));

  Future<void> setSkinUrl(String url, {required bool slim}) =>
      _run(() => _bridge.setSkinUrl(url, account: accountId, slim: slim));

  Future<void> resetSkin() => _run(() => _bridge.resetSkin(account: accountId));

  Future<void> showCape(String id) async {
    final p = await _bridge.showCape(id, account: accountId);
    if (ref.mounted) state = AsyncData(p);
  }

  Future<void> hideCape() async {
    final p = await _bridge.hideCape(account: accountId);
    if (ref.mounted) state = AsyncData(p);
  }
}

final profileProvider =
    AsyncNotifierProvider.family<ProfileController, ProfileView, String>(
  ProfileController.new,
);

extension ProfileViewX on ProfileView {
  SkinView? get activeSkin {
    for (final s in skins) {
      if (s.state == 'ACTIVE') return s;
    }
    return skins.isEmpty ? null : skins.first;
  }

  CapeView? get activeCape {
    for (final c in capes) {
      if (c.active) return c;
    }
    return null;
  }
}
