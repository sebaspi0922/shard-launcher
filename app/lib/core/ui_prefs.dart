// Preferencias propias de la interfaz (`<datos>/ui.json`): apariencia y
// primer arranque. Se leen de forma síncrona al arrancar (sin parpadeos) y se
// guardan en segundo plano.

import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'providers.dart';

/// Fondo de la app.
enum BackgroundKind {
  /// Halos suaves del acento (por defecto).
  aurora,

  /// Paisaje de bloques generado.
  landscape,

  /// Imagen elegida por el usuario.
  image,
}

class UiPrefs {
  const UiPrefs({
    this.accentAuto = false,
    this.accentPreset = 0xFF35D69B,
    this.background = BackgroundKind.aurora,
    this.backgroundPath,
    this.blur = 12,
    this.dim = 0.45,
    this.performance = false,
    this.firstRunDone,
  });

  /// `true` = acento extraído del fondo.
  final bool accentAuto;

  /// Color ARGB del preset elegido.
  final int accentPreset;
  final BackgroundKind background;

  /// Copia de la imagen de fondo dentro de la carpeta de datos.
  final String? backgroundPath;

  /// Desenfoque del fondo (0–40).
  final double blur;

  /// Oscurecido del fondo (0–0,85).
  final double dim;

  /// Sin desenfoque ni efectos de fondo.
  final bool performance;

  /// `null` = nunca se marcó (instalación nueva o anterior al asistente).
  final bool? firstRunDone;

  static const defaults = UiPrefs();

  factory UiPrefs.fromJson(Map<String, dynamic> j) {
    double number(String k, double def, double min, double max) {
      final v = j[k];
      return v is num ? v.toDouble().clamp(min, max) : def;
    }

    final bg = j['background'];
    final firstRun = j['first_run_done'];
    final preset = j['accent_preset'];
    return UiPrefs(
      accentAuto: j['accent'] == 'auto',
      accentPreset: preset is int ? preset : defaults.accentPreset,
      background: BackgroundKind.values.firstWhere(
        (b) => b.name == bg,
        orElse: () => BackgroundKind.aurora,
      ),
      backgroundPath: j['background_path'] is String ? j['background_path'] as String : null,
      blur: number('blur', defaults.blur, 0, 40),
      dim: number('dim', defaults.dim, 0, 0.85),
      performance: j['performance'] == true,
      firstRunDone: firstRun is bool ? firstRun : null,
    );
  }

  factory UiPrefs.parse(String text) {
    try {
      final v = jsonDecode(text);
      if (v is Map<String, dynamic>) return UiPrefs.fromJson(v);
    } catch (_) {
      // Archivo dañado: valores por defecto.
    }
    return defaults;
  }

  Map<String, dynamic> toJson() => {
        'accent': accentAuto ? 'auto' : 'preset',
        'accent_preset': accentPreset,
        'background': background.name,
        'background_path': ?backgroundPath,
        'blur': blur,
        'dim': dim,
        'performance': performance,
        'first_run_done': ?firstRunDone,
      };

  UiPrefs copyWith({
    bool? accentAuto,
    int? accentPreset,
    BackgroundKind? background,
    String? Function()? backgroundPath,
    double? blur,
    double? dim,
    bool? performance,
    bool? firstRunDone,
  }) =>
      UiPrefs(
        accentAuto: accentAuto ?? this.accentAuto,
        accentPreset: accentPreset ?? this.accentPreset,
        background: background ?? this.background,
        backgroundPath:
            backgroundPath != null ? backgroundPath() : this.backgroundPath,
        blur: blur ?? this.blur,
        dim: dim ?? this.dim,
        performance: performance ?? this.performance,
        firstRunDone: firstRunDone ?? this.firstRunDone,
      );
}

class UiPrefsController extends Notifier<UiPrefs> {
  Timer? _debounce;

  @override
  UiPrefs build() {
    ref.onDispose(() => _debounce?.cancel());
    return UiPrefs.parse(ref.watch(bridgeProvider).getUiPrefs());
  }

  /// Cambia y guarda. `debounce` para deslizadores (se guarda al soltar).
  Future<void> update(
    UiPrefs Function(UiPrefs p) change, {
    bool debounce = false,
  }) async {
    state = change(state);
    _debounce?.cancel();
    if (debounce) {
      _debounce = Timer(const Duration(milliseconds: 400), _save);
    } else {
      await _save();
    }
  }

  Future<void> _save() =>
      ref.read(bridgeProvider).setUiPrefs(jsonEncode(state.toJson()));
}

final uiPrefsProvider =
    NotifierProvider<UiPrefsController, UiPrefs>(UiPrefsController.new);
