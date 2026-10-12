// Componentes comunes: paneles, chips, estados vacíos, avisos y diálogos.

import 'package:flutter/services.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/errors.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';

/// Superficie translúcida con borde fino.
class Panel extends StatelessWidget {
  const Panel({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(Space.x5),
    this.level = 1,
    this.radius = Radii.card,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final int level;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Container(
      padding: padding,
      decoration: BoxDecoration(
        color: switch (level) {
          1 => c.surface1,
          2 => c.surface2,
          _ => c.surface3,
        },
        borderRadius: BorderRadius.circular(radius),
        border: Border.all(color: c.border),
      ),
      child: child,
    );
  }
}

/// Título de sección con acción opcional a la derecha.
class SectionTitle extends StatelessWidget {
  const SectionTitle(this.title, {super.key, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.only(bottom: Space.x3),
        child: Row(
          children: [
            Text(title, style: context.text.titleMedium),
            const Spacer(),
            ?trailing,
          ],
        ),
      );
}

/// Píldora pequeña con icono o punto de color.
class Tag extends StatelessWidget {
  const Tag(this.label, {super.key, this.icon, this.dot, this.strong = false});

  final String label;
  final IconData? icon;
  final Color? dot;
  final bool strong;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      decoration: BoxDecoration(
        color: strong ? Colors.black.withValues(alpha: 0.35) : c.surface2,
        borderRadius: BorderRadius.circular(Radii.pill),
        border: Border.all(color: c.border),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (dot != null) ...[
            Container(
              width: 7,
              height: 7,
              decoration: BoxDecoration(color: dot, shape: BoxShape.circle),
            ),
            const SizedBox(width: 6),
          ],
          if (icon != null) ...[
            Icon(icon, size: 14, color: c.textSecondary),
            const SizedBox(width: 5),
          ],
          Text(
            label,
            style: context.text.labelMedium!.copyWith(
              color: strong ? c.text : c.textSecondary,
            ),
          ),
        ],
      ),
    );
  }
}

/// Color identificativo de cada loader.
Color loaderColor(String loader) => switch (loader) {
      'vanilla' => const Color(0xFF6CCB5F),
      'fabric' => const Color(0xFFDBC59C),
      'quilt' => const Color(0xFFB28CF5),
      'forge' => const Color(0xFF8FA6C9),
      'neoforge' => const Color(0xFFF0954A),
      'labymod' => const Color(0xFF5BB0F0),
      _ => const Color(0xFFA0A0A0),
    };

/// Estado vacío centrado con icono, texto y acción.
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    required this.body,
    this.action,
  });

  final IconData icon;
  final String title;
  final String body;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 380),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 64,
              height: 64,
              decoration: BoxDecoration(
                color: c.accentContainer.withValues(alpha: 0.6),
                borderRadius: BorderRadius.circular(Radii.panel),
                border: Border.all(color: c.accent.withValues(alpha: 0.25)),
              ),
              child: Icon(icon, size: 30, color: c.accent),
            ),
            const SizedBox(height: Space.x5),
            Text(
              title,
              textAlign: TextAlign.center,
              style: context.text.titleLarge,
            ),
            const SizedBox(height: Space.x2),
            Text(
              body,
              textAlign: TextAlign.center,
              style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
            ),
            if (action != null) ...[const SizedBox(height: Space.x5), action!],
          ],
        ),
      ),
    );
  }
}

/// Aviso de error en línea con detalle técnico desplegable.
class ErrorNote extends StatefulWidget {
  const ErrorNote(this.error, {super.key});

  final Object error;

  @override
  State<ErrorNote> createState() => _ErrorNoteState();
}

class _ErrorNoteState extends State<ErrorNote> {
  bool _open = false;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final l = AppLocalizations.of(context);
    final detail = errorDetail(widget.error);
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(Space.x3),
      decoration: BoxDecoration(
        color: c.danger.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(Radii.control),
        border: Border.all(color: c.danger.withValues(alpha: 0.35)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Symbols.error_rounded, size: 18, color: c.danger),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Text(
                  errorText(l, widget.error),
                  style: context.text.bodyMedium!.copyWith(color: c.text),
                ),
              ),
              if (detail != null)
                InkWell(
                  borderRadius: BorderRadius.circular(6),
                  onTap: () => setState(() => _open = !_open),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 6),
                    child: Text(
                      l.details,
                      style: context.text.labelMedium!.copyWith(
                        color: c.textSecondary,
                        decoration: TextDecoration.underline,
                      ),
                    ),
                  ),
                ),
            ],
          ),
          if (_open && detail != null) ...[
            const SizedBox(height: Space.x2),
            SelectableText(
              detail,
              style: context.text.bodySmall!.copyWith(
                fontFamily: 'JetBrainsMono',
                fontSize: 11.5,
              ),
            ),
          ],
        ],
      ),
    );
  }
}

/// Aviso breve abajo.
void showToast(BuildContext context, String message, {bool error = false}) {
  final c = context.colors;
  ScaffoldMessenger.maybeOf(context)
    ?..hideCurrentSnackBar()
    ..showSnackBar(
      SnackBar(
        content: Row(
          children: [
            Icon(
              error ? Symbols.error_rounded : Symbols.check_circle_rounded,
              color: error ? c.danger : c.success,
              size: 18,
            ),
            const SizedBox(width: Space.x3),
            Expanded(child: Text(message)),
          ],
        ),
        duration: Duration(seconds: error ? 7 : 4),
      ),
    );
}

/// Diálogo con el estilo de la app.
class AppDialog extends StatelessWidget {
  const AppDialog({
    super.key,
    required this.title,
    required this.child,
    this.actions = const [],
    this.width = 440,
    this.icon,
  });

  final String title;
  final Widget child;
  final List<Widget> actions;
  final double width;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Dialog(
      insetPadding: const EdgeInsets.all(Space.x6),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(
            Space.x6,
            Space.x6,
            Space.x6,
            Space.x5,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  if (icon != null) ...[
                    Icon(icon, color: c.accent, size: 22),
                    const SizedBox(width: Space.x3),
                  ],
                  Expanded(
                    child: Text(title, style: context.text.titleLarge),
                  ),
                ],
              ),
              const SizedBox(height: Space.x4),
              Flexible(child: child),
              if (actions.isNotEmpty) ...[
                const SizedBox(height: Space.x6),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    for (final (i, a) in actions.indexed) ...[
                      if (i > 0) const SizedBox(width: Space.x2),
                      a,
                    ],
                  ],
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Confirmación simple. Devuelve `true` si se acepta.
Future<bool> confirm(
  BuildContext context, {
  required String title,
  required String body,
  required String action,
  bool danger = false,
}) async {
  final l = AppLocalizations.of(context);
  final ok = await showDialog<bool>(
    context: context,
    builder: (context) {
      final c = context.colors;
      return AppDialog(
        title: title,
        width: 420,
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(l.cancel),
          ),
          FilledButton(
            style: danger
                ? FilledButton.styleFrom(
                    backgroundColor: c.danger,
                    foregroundColor: const Color(0xFF2B0A0A),
                  )
                : null,
            onPressed: () => Navigator.pop(context, true),
            child: Text(action),
          ),
        ],
        child: Text(body),
      );
    },
  );
  return ok ?? false;
}

/// Copia al portapapeles y avisa.
Future<void> copyText(BuildContext context, String text, String done) async {
  await Clipboard.setData(ClipboardData(text: text));
  if (context.mounted) showToast(context, done);
}

/// Fila de ajuste: etiqueta y descripción a la izquierda, control a la derecha.
class SettingRow extends StatelessWidget {
  const SettingRow({
    super.key,
    required this.title,
    this.description,
    required this.control,
    this.controlWidth,
  });

  final String title;
  final String? description;
  final Widget control;
  final double? controlWidth;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.x3),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: context.text.titleSmall),
                if (description != null) ...[
                  const SizedBox(height: 2),
                  Text(description!, style: context.text.bodySmall),
                ],
              ],
            ),
          ),
          const SizedBox(width: Space.x6),
          if (controlWidth != null)
            SizedBox(width: controlWidth, child: control)
          else
            control,
        ],
      ),
    );
  }
}
