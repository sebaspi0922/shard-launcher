// Desplegable compacto con el estilo de la app (botón + menú).

import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../theme/tokens.dart';

class SelectOption<T> {
  const SelectOption(this.value, this.label, {this.leading, this.subtitle});

  final T value;
  final String label;
  final Widget? leading;
  final String? subtitle;
}

class SelectButton<T> extends StatelessWidget {
  const SelectButton({
    super.key,
    required this.options,
    required this.value,
    required this.onChanged,
    this.icon,
    this.prefix,
    this.width,
    this.menuMaxHeight = 360,
  });

  final List<SelectOption<T>> options;
  final T value;
  final ValueChanged<T>? onChanged;
  final IconData? icon;

  /// Texto antes del valor («Versión: 1.21.4»).
  final String? prefix;
  final double? width;
  final double menuMaxHeight;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final current = options.where((o) => o.value == value).firstOrNull;
    return MenuAnchor(
      alignmentOffset: const Offset(0, 4),
      style: MenuStyle(
        maximumSize: WidgetStatePropertyAll(Size(480, menuMaxHeight)),
      ),
      menuChildren: [
        for (final o in options)
          MenuItemButton(
            leadingIcon: o.leading,
            trailingIcon: o.value == value
                ? Icon(Symbols.check_rounded, size: 18, color: c.accent)
                : null,
            onPressed: onChanged == null ? null : () => onChanged!(o.value),
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: (width ?? 160) - 60),
              child: o.subtitle == null
                  ? Text(o.label)
                  : Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(o.label, style: context.text.bodyMedium),
                        Text(o.subtitle!, style: context.text.bodySmall),
                      ],
                    ),
            ),
          ),
      ],
      builder: (context, controller, _) {
        final button = OutlinedButton(
          style: OutlinedButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 12),
          ),
          onPressed: onChanged == null
              ? null
              : () => controller.isOpen ? controller.close() : controller.open(),
          child: Row(
            mainAxisSize: width == null ? MainAxisSize.min : MainAxisSize.max,
            children: [
              if (current?.leading != null) ...[
                current!.leading!,
                const SizedBox(width: Space.x2),
              ] else if (icon != null) ...[
                Icon(icon, size: 18, color: c.textSecondary),
                const SizedBox(width: Space.x2),
              ],
              if (prefix != null)
                Text(
                  '$prefix  ',
                  style: context.text.labelLarge!.copyWith(
                    color: c.textMuted,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              Flexible(
                fit: width == null ? FlexFit.loose : FlexFit.tight,
                child: Text(
                  current?.label ?? '—',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(width: Space.x1),
              Icon(Symbols.expand_more_rounded, size: 18, color: c.textMuted),
            ],
          ),
        );
        return width == null ? button : SizedBox(width: width, child: button);
      },
    );
  }
}
