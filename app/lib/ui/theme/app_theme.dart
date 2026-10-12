// Tema oscuro de la app a partir de los tokens.

import 'package:material_ui/material_ui.dart';

import 'styles.dart';
import 'tokens.dart';

const kFontFamily = 'Inter';
const kMonoFamily = 'JetBrainsMono';

/// Tema del estilo con su acento o, si se da, con [accent].
ThemeData buildTheme(AppStyle style, [Color? accent]) {
  final spec = style.spec;
  Radii.control = spec.control;
  Radii.card = spec.card;
  Radii.panel = spec.panel;
  final c = spec.palette(accent);
  final scheme = ColorScheme(
    brightness: Brightness.dark,
    primary: c.accent,
    onPrimary: c.onAccent,
    primaryContainer: c.accentContainer,
    onPrimaryContainer: c.onAccentContainer,
    secondary: c.accent,
    onSecondary: c.onAccent,
    surface: c.background,
    onSurface: c.text,
    onSurfaceVariant: c.textSecondary,
    surfaceContainerLowest: c.backgroundDeep,
    surfaceContainerLow: Color.alphaBlend(c.surface1, c.background),
    surfaceContainer: Color.alphaBlend(c.surface2, c.background),
    surfaceContainerHigh: Color.alphaBlend(c.surface3, c.background),
    surfaceContainerHighest:
        Color.alphaBlend(c.surface3, Color.alphaBlend(c.surface2, c.background)),
    outline: c.borderStrong,
    outlineVariant: c.border,
    error: c.danger,
    onError: const Color(0xFF2B0A0A),
  );

  TextStyle s(double size, FontWeight w, {double h = 1.35, double ls = 0}) =>
      TextStyle(
        fontFamily: kFontFamily,
        fontSize: size,
        fontWeight: w,
        height: h,
        letterSpacing: ls,
        color: c.text,
      );

  final text = TextTheme(
    displaySmall: s(34, FontWeight.w700, h: 1.15, ls: -0.8),
    headlineMedium: s(26, FontWeight.w700, h: 1.2, ls: -0.5),
    headlineSmall: s(22, FontWeight.w600, h: 1.25, ls: -0.3),
    titleLarge: s(18, FontWeight.w600, ls: -0.2),
    titleMedium: s(15, FontWeight.w600, ls: -0.1),
    titleSmall: s(13, FontWeight.w600),
    bodyLarge: s(15, FontWeight.w400, h: 1.5),
    bodyMedium: s(13.5, FontWeight.w400, h: 1.5),
    bodySmall: s(12, FontWeight.w400, h: 1.45).copyWith(color: c.textSecondary),
    labelLarge: s(13.5, FontWeight.w600),
    labelMedium: s(12, FontWeight.w500),
    labelSmall: s(11, FontWeight.w600, ls: 0.4),
  );

  final controlShape = RoundedRectangleBorder(
    borderRadius: BorderRadius.circular(Radii.control),
  );
  const controlPadding = EdgeInsets.symmetric(horizontal: 16, vertical: 12);

  OutlineInputBorder inputBorder(Color color, [double width = 1]) =>
      OutlineInputBorder(
        borderRadius: BorderRadius.circular(Radii.control),
        borderSide: BorderSide(color: color, width: width),
      );

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    fontFamily: kFontFamily,
    textTheme: text,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: c.background,
    splashFactory: InkSparkle.splashFactory,
    visualDensity: VisualDensity.standard,
    extensions: [c],
    dividerTheme: DividerThemeData(color: c.border, thickness: 1, space: 1),
    iconTheme: IconThemeData(color: c.textSecondary, size: 20),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 350),
      decoration: BoxDecoration(
        color: Color.alphaBlend(c.surface3, c.backgroundDeep),
        borderRadius: BorderRadius.circular(Radii.control),
        border: Border.all(color: c.borderStrong),
      ),
      textStyle: text.labelMedium!.copyWith(color: c.text),
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: c.accent,
        foregroundColor: c.onAccent,
        disabledBackgroundColor: c.surface2,
        disabledForegroundColor: c.textMuted,
        textStyle: text.labelLarge,
        padding: controlPadding,
        minimumSize: const Size(0, 40),
        shape: controlShape,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: c.text,
        backgroundColor: c.surface1,
        side: BorderSide(color: c.borderStrong),
        textStyle: text.labelLarge,
        padding: controlPadding,
        minimumSize: const Size(0, 40),
        shape: controlShape,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: c.textSecondary,
        textStyle: text.labelLarge,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        minimumSize: const Size(0, 36),
        shape: controlShape,
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        foregroundColor: c.textSecondary,
        shape: controlShape,
      ),
    ),
    inputDecorationTheme: InputDecorationThemeData(
      filled: true,
      fillColor: c.surface1,
      isDense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      hintStyle: text.bodyMedium!.copyWith(color: c.textMuted),
      labelStyle: text.bodyMedium!.copyWith(color: c.textSecondary),
      floatingLabelStyle: text.bodySmall!.copyWith(color: c.accent),
      helperStyle: text.bodySmall,
      errorStyle: text.bodySmall!.copyWith(color: c.danger),
      prefixIconColor: c.textMuted,
      suffixIconColor: c.textMuted,
      border: inputBorder(c.border),
      enabledBorder: inputBorder(c.border),
      focusedBorder: inputBorder(c.accent, 1.5),
      errorBorder: inputBorder(c.danger),
      focusedErrorBorder: inputBorder(c.danger, 1.5),
    ),
    sliderTheme: SliderThemeData(
      activeTrackColor: c.accent,
      inactiveTrackColor: c.surface3,
      thumbColor: c.accent,
      overlayColor: c.accent.withValues(alpha: 0.12),
      trackHeight: 4,
      valueIndicatorColor: Color.alphaBlend(c.surface3, c.backgroundDeep),
      valueIndicatorTextStyle: text.labelMedium,
      showValueIndicator: ShowValueIndicator.onDrag,
      tickMarkShape: SliderTickMarkShape.noTickMark,
    ),
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? c.onAccent : c.textSecondary,
      ),
      trackColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? c.accent : c.surface2,
      ),
      trackOutlineColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? c.accent : c.borderStrong,
      ),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? c.accent : Colors.transparent,
      ),
      checkColor: WidgetStatePropertyAll(c.onAccent),
      side: BorderSide(color: c.borderStrong, width: 1.5),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
    ),
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: ButtonStyle(
        backgroundColor: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected) ? c.accentContainer : c.surface1,
        ),
        foregroundColor: WidgetStateProperty.resolveWith(
          (s) => s.contains(WidgetState.selected)
              ? c.onAccentContainer
              : c.textSecondary,
        ),
        side: WidgetStatePropertyAll(BorderSide(color: c.border)),
        shape: WidgetStatePropertyAll(controlShape),
        textStyle: WidgetStatePropertyAll(text.labelMedium),
        visualDensity: VisualDensity.compact,
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: Color.alphaBlend(c.surface2, c.background),
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.panel),
        side: BorderSide(color: c.borderStrong),
      ),
      titleTextStyle: text.titleLarge,
      contentTextStyle: text.bodyMedium!.copyWith(color: c.textSecondary),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: Color.alphaBlend(c.surface3, c.backgroundDeep),
      contentTextStyle: text.bodyMedium,
      actionTextColor: c.accent,
      width: 420,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(color: c.borderStrong),
      ),
    ),
    menuTheme: MenuThemeData(
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(
          Color.alphaBlend(c.surface3, c.background),
        ),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        side: WidgetStatePropertyAll(BorderSide(color: c.borderStrong)),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.card)),
        ),
        padding: const WidgetStatePropertyAll(EdgeInsets.all(6)),
      ),
    ),
    menuButtonTheme: MenuButtonThemeData(
      style: MenuItemButton.styleFrom(
        foregroundColor: c.text,
        iconColor: c.textSecondary,
        textStyle: text.bodyMedium,
        minimumSize: const Size(180, 38),
        shape: controlShape,
      ),
    ),
    popupMenuTheme: PopupMenuThemeData(
      color: Color.alphaBlend(c.surface3, c.background),
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(color: c.borderStrong),
      ),
      textStyle: text.bodyMedium,
    ),
    scrollbarTheme: ScrollbarThemeData(
      thumbColor: WidgetStatePropertyAll(c.borderStrong),
      radius: const Radius.circular(8),
      thickness: const WidgetStatePropertyAll(6),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(
      color: c.accent,
      linearTrackColor: c.surface2,
      circularTrackColor: Colors.transparent,
    ),
    listTileTheme: ListTileThemeData(
      shape: controlShape,
      iconColor: c.textSecondary,
      textColor: c.text,
    ),
  );
}
