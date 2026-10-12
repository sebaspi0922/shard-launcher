// Tokens del sistema visual: colores derivados del acento, espaciado y radios.

import 'dart:ui' show lerpDouble;

import 'package:material_color_utilities/material_color_utilities.dart';
import 'package:material_ui/material_ui.dart';

/// Espaciado en múltiplos de 4.
abstract final class Space {
  static const double x1 = 4;
  static const double x2 = 8;
  static const double x3 = 12;
  static const double x4 = 16;
  static const double x5 = 20;
  static const double x6 = 24;
  static const double x8 = 32;
  static const double x10 = 40;
}

abstract final class Radii {
  static const double control = 8;
  static const double card = 12;
  static const double panel = 16;
  static const double pill = 999;
}

abstract final class Motion {
  static const fast = Duration(milliseconds: 150);
  static const medium = Duration(milliseconds: 220);
  static const curve = Curves.easeOutCubic;
}

/// Colores de la app. Todo sale del matiz del acento.
@immutable
class AppColors extends ThemeExtension<AppColors> {
  const AppColors({
    required this.background,
    required this.backgroundDeep,
    required this.surface1,
    required this.surface2,
    required this.surface3,
    required this.border,
    required this.borderStrong,
    required this.text,
    required this.textSecondary,
    required this.textMuted,
    required this.accent,
    required this.accentHover,
    required this.onAccent,
    required this.accentContainer,
    required this.onAccentContainer,
    required this.glow,
    required this.success,
    required this.warning,
    required this.danger,
    required this.info,
  });

  final Color background;
  final Color backgroundDeep;

  /// Superficies en capas (1 = la más baja), translúcidas sobre el fondo.
  final Color surface1;
  final Color surface2;
  final Color surface3;
  final Color border;
  final Color borderStrong;
  final Color text;
  final Color textSecondary;
  final Color textMuted;
  final Color accent;
  final Color accentHover;
  final Color onAccent;
  final Color accentContainer;
  final Color onAccentContainer;
  final Color glow;
  final Color success;
  final Color warning;
  final Color danger;
  final Color info;

  /// Escala completa a partir de un color semilla.
  factory AppColors.fromSeed(Color seed) {
    final hct = Hct.fromInt(seed.toARGB32());
    final hue = hct.hue;
    final accent = TonalPalette.of(hue, hct.chroma.clamp(40, 90).toDouble());
    final tint = TonalPalette.of(hue, 10);
    final soft = TonalPalette.of(hue, 24);
    Color t(TonalPalette p, int tone) => Color(p.get(tone));
    return AppColors(
      background: t(tint, 6),
      backgroundDeep: t(tint, 4),
      surface1: Color(0x0FFFFFFF),
      surface2: Color(0x17FFFFFF),
      surface3: Color(0x21FFFFFF),
      border: Color(0x14FFFFFF),
      borderStrong: Color(0x26FFFFFF),
      text: t(tint, 96),
      textSecondary: t(tint, 78),
      textMuted: t(tint, 60),
      accent: t(accent, 74),
      accentHover: t(accent, 82),
      onAccent: t(accent, 12),
      accentContainer: t(soft, 26),
      onAccentContainer: t(accent, 90),
      glow: t(accent, 60),
      success: const Color(0xFF4ADE80),
      warning: const Color(0xFFFBBF24),
      danger: const Color(0xFFF87171),
      info: const Color(0xFF7DD3FC),
    );
  }

  @override
  AppColors copyWith() => this;

  @override
  AppColors lerp(AppColors? other, double t) {
    if (other == null) return this;
    Color c(Color a, Color b) => Color.lerp(a, b, t)!;
    return AppColors(
      background: c(background, other.background),
      backgroundDeep: c(backgroundDeep, other.backgroundDeep),
      surface1: c(surface1, other.surface1),
      surface2: c(surface2, other.surface2),
      surface3: c(surface3, other.surface3),
      border: c(border, other.border),
      borderStrong: c(borderStrong, other.borderStrong),
      text: c(text, other.text),
      textSecondary: c(textSecondary, other.textSecondary),
      textMuted: c(textMuted, other.textMuted),
      accent: c(accent, other.accent),
      accentHover: c(accentHover, other.accentHover),
      onAccent: c(onAccent, other.onAccent),
      accentContainer: c(accentContainer, other.accentContainer),
      onAccentContainer: c(onAccentContainer, other.onAccentContainer),
      glow: c(glow, other.glow),
      success: c(success, other.success),
      warning: c(warning, other.warning),
      danger: c(danger, other.danger),
      info: c(info, other.info),
    );
  }
}

extension AppThemeX on BuildContext {
  AppColors get colors => Theme.of(this).extension<AppColors>()!;
  TextTheme get text => Theme.of(this).textTheme;
}

/// Interpolación de dobles para animaciones propias.
double lerpD(double a, double b, double t) => lerpDouble(a, b, t)!;
