// Estilos de serie: paleta, forma, acento y fondo de cada uno.

import 'package:material_ui/material_ui.dart';

import 'tokens.dart';

enum AppStyle {
  /// Gris neutro y plano, acento blanco y anillos que derivan (por defecto).
  graphite,

  /// Negro con dorado y una esfera ASCII que gira.
  sebaspi,

  /// Paisaje de bloques al atardecer.
  landscape,

  /// Halos suaves del acento.
  aurora;

  static AppStyle parse(Object? name) => values.firstWhere(
        (s) => s.name == name,
        orElse: () => graphite,
      );
}

/// Lo que define un estilo.
@immutable
class StyleSpec {
  const StyleSpec({
    required this.accent,
    required this.control,
    required this.card,
    required this.panel,
    required this.palette,
  });

  /// Acento propio del estilo.
  final Color accent;

  /// Radios.
  final double control;
  final double card;
  final double panel;

  /// Colores; `accent` es `null` cuando se usa el acento del estilo.
  final AppColors Function(Color? accent) palette;
}

const _jade = Color(0xFF35D69B);
const _amber = Color(0xFFFFA94D);

final kStyles = <AppStyle, StyleSpec>{
  AppStyle.graphite: StyleSpec(
    accent: const Color(0xFFFFFFFF),
    control: 8,
    card: 8,
    panel: 10,
    palette: (a) =>
        AppColors.graphite(a == null ? null : AccentSet.fromSeed(a)),
  ),
  AppStyle.sebaspi: StyleSpec(
    accent: const Color(0xFFF1C40F),
    control: 8,
    card: 12,
    panel: 16,
    palette: (a) => AppColors.gold(a == null ? null : AccentSet.fromSeed(a)),
  ),
  AppStyle.landscape: StyleSpec(
    accent: _amber,
    control: 8,
    card: 12,
    panel: 16,
    palette: (a) => AppColors.fromSeed(a ?? _amber),
  ),
  AppStyle.aurora: StyleSpec(
    accent: _jade,
    control: 8,
    card: 12,
    panel: 16,
    palette: (a) => AppColors.fromSeed(a ?? _jade),
  ),
};

extension AppStyleSpec on AppStyle {
  StyleSpec get spec => kStyles[this]!;
}
