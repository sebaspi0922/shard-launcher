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

/// Radios del estilo activo (los fija `buildTheme`).
abstract final class Radii {
  static double control = 8;
  static double card = 12;
  static double panel = 16;
  static const double pill = 999;
}

abstract final class Motion {
  static const fast = Duration(milliseconds: 150);
  static const medium = Duration(milliseconds: 220);
  static const curve = Curves.easeOutCubic;

  /// Cambio de sección.
  static const page = Duration(milliseconds: 260);

  /// Constante de tiempo con que la lista persigue a la rueda del ratón: en
  /// este tiempo recorre ~63 % de lo que falta (~95 % en el triple).
  static const wheel = Duration(milliseconds: 70);

  /// Salida exponencial suave (diálogos, secciones, acordeones).
  static const emphasized = Cubic(0.16, 1, 0.3, 1);
}

/// Colores de la app (dependen del estilo y del acento).
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
    this.panelBase,
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

  /// Base casi opaca bajo tarjetas y paneles, para que un fondo con dibujo no
  /// estorbe al texto. `null` = superficies translúcidas (efecto cristal).
  final Color? panelBase;

  /// Color de una tarjeta o panel apoyado directamente sobre el fondo.
  Color card(Color surface) =>
      panelBase == null ? surface : Color.alphaBlend(surface, panelBase!);

  /// Escala completa a partir de un color semilla.
  factory AppColors.fromSeed(Color seed) {
    final hct = Hct.fromInt(seed.toARGB32());
    final tint = TonalPalette.of(hct.hue, 10);
    Color t(int tone) => Color(tint.get(tone));
    final a = AccentSet.fromSeed(seed);
    return AppColors(
      background: t(6),
      backgroundDeep: t(4),
      surface1: const Color(0x0FFFFFFF),
      surface2: const Color(0x17FFFFFF),
      surface3: const Color(0x21FFFFFF),
      border: const Color(0x14FFFFFF),
      borderStrong: const Color(0x26FFFFFF),
      text: t(96),
      textSecondary: t(78),
      textMuted: t(60),
      accent: a.accent,
      accentHover: a.hover,
      onAccent: a.onAccent,
      accentContainer: a.container,
      onAccentContainer: a.onContainer,
      glow: a.glow,
      success: const Color(0xFF4ADE80),
      warning: const Color(0xFFFBBF24),
      danger: const Color(0xFFF87171),
      info: const Color(0xFF7DD3FC),
    );
  }

  /// Gris neutro y plano, con acento blanco de tinta casi negra.
  factory AppColors.graphite([AccentSet? accent]) {
    final a =
        accent ??
        const AccentSet(
          accent: Color(0xFFFFFFFF),
          hover: Color(0xFFE6E6E6),
          onAccent: Color(0xFF0D0D0D),
          container: Color(0x29FFFFFF),
          onContainer: Color(0xFFFFFFFF),
          glow: Color(0xFFAEBCFF),
        );
    return AppColors(
      background: const Color(0xFF212121),
      backgroundDeep: const Color(0xFF171717),
      surface1: const Color(0x0DFFFFFF),
      surface2: const Color(0x14FFFFFF),
      surface3: const Color(0x1CFFFFFF),
      border: const Color(0x1FFFFFFF),
      borderStrong: const Color(0x33FFFFFF),
      text: const Color(0xFFECECEC),
      textSecondary: const Color(0xFFB4B4B4),
      textMuted: const Color(0xFF8F8F8F),
      accent: a.accent,
      accentHover: a.hover,
      onAccent: a.onAccent,
      accentContainer: a.container,
      onAccentContainer: a.onContainer,
      glow: a.glow,
      success: const Color(0xFF10A37F),
      warning: const Color(0xFFE0B04A),
      danger: const Color(0xFFF87171),
      info: const Color(0xFF8FD4FF),
      panelBase: const Color(0xEB212121),
    );
  }

  /// Negro profundo con dorado como único acento.
  factory AppColors.gold([AccentSet? accent]) {
    final a =
        accent ??
        const AccentSet(
          accent: Color(0xFFF1C40F),
          hover: Color(0xFFFAC814),
          onAccent: Color(0xFF121212),
          container: Color(0x2EF1C40F),
          onContainer: Color(0xFFF8E187),
          glow: Color(0xFFF1C40F),
        );
    return AppColors(
      background: const Color(0xFF000000),
      backgroundDeep: const Color(0xFF000000),
      surface1: const Color(0x0DFFFFFF),
      surface2: const Color(0x14FFFFFF),
      surface3: const Color(0x1FFFFFFF),
      border: const Color(0x24FFFFFF),
      borderStrong: const Color(0x38FFFFFF),
      text: const Color(0xFFE5E5E5),
      textSecondary: const Color(0xFFA8A8A8),
      textMuted: const Color(0xFF767676),
      accent: a.accent,
      accentHover: a.hover,
      onAccent: a.onAccent,
      accentContainer: a.container,
      onAccentContainer: a.onContainer,
      glow: a.glow,
      success: const Color(0xFF21C45D),
      warning: const Color(0xFFF39C12),
      danger: const Color(0xFFE74C3C),
      info: const Color(0xFF7DD3FC),
      panelBase: const Color(0xF0000000),
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
      panelBase: Color.lerp(panelBase, other.panelBase, t),
    );
  }
}

/// Colores del acento y sus variantes.
@immutable
class AccentSet {
  const AccentSet({
    required this.accent,
    required this.hover,
    required this.onAccent,
    required this.container,
    required this.onContainer,
    required this.glow,
  });

  /// Tonos derivados de un color semilla.
  factory AccentSet.fromSeed(Color seed) {
    final hct = Hct.fromInt(seed.toARGB32());
    final p = TonalPalette.of(hct.hue, hct.chroma.clamp(40, 90).toDouble());
    final soft = TonalPalette.of(hct.hue, 24);
    Color t(TonalPalette p, int tone) => Color(p.get(tone));
    return AccentSet(
      accent: t(p, 74),
      hover: t(p, 82),
      onAccent: t(p, 12),
      container: t(soft, 26),
      onContainer: t(p, 90),
      glow: t(p, 60),
    );
  }

  final Color accent;
  final Color hover;
  final Color onAccent;
  final Color container;
  final Color onContainer;
  final Color glow;
}

extension AppThemeX on BuildContext {
  AppColors get colors => Theme.of(this).extension<AppColors>()!;
  TextTheme get text => Theme.of(this).textTheme;
}

/// Interpolación de dobles para animaciones propias.
double lerpD(double a, double b, double t) => lerpDouble(a, b, t)!;
