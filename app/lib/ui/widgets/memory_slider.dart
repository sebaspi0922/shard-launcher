// Deslizador de memoria acotado por la RAM del equipo, con aviso.

import 'dart:math' as math;

import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/format.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';

const kMemoryStep = 512;
const kMemoryMin = 1024;

class MemorySlider extends StatelessWidget {
  const MemorySlider({
    super.key,
    required this.valueMb,
    required this.totalMb,
    required this.onChanged,
  });

  final int valueMb;
  final int totalMb;
  final ValueChanged<int> onChanged;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    // Tope: la RAM total redondeada a pasos (y nunca por debajo del mínimo).
    final max = math.max(kMemoryMin + kMemoryStep, totalMb ~/ kMemoryStep * kMemoryStep);
    final value = valueMb.clamp(kMemoryMin, max);
    final warn = totalMb > 0 && value > totalMb * 0.75;
    final divisions = (max - kMemoryMin) ~/ kMemoryStep;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: SliderTheme(
                data: SliderTheme.of(context).copyWith(
                  activeTrackColor: warn ? c.warning : null,
                  thumbColor: warn ? c.warning : null,
                ),
                child: Slider(
                  min: kMemoryMin.toDouble(),
                  max: max.toDouble(),
                  divisions: divisions,
                  value: value.toDouble(),
                  label: formatMemory(l, value),
                  onChanged: (v) => onChanged(v.round()),
                ),
              ),
            ),
            const SizedBox(width: Space.x3),
            Container(
              width: 76,
              padding: const EdgeInsets.symmetric(vertical: 8),
              alignment: Alignment.center,
              decoration: BoxDecoration(
                color: c.surface2,
                borderRadius: BorderRadius.circular(Radii.control),
                border: Border.all(color: c.border),
              ),
              child: Text(
                formatMemory(l, value),
                style: context.text.titleSmall!.copyWith(
                  color: warn ? c.warning : c.text,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ),
          ],
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 24),
          child: Row(
            children: [
              Text(formatMemory(l, kMemoryMin), style: context.text.bodySmall),
              const Spacer(),
              Text(formatMemory(l, max), style: context.text.bodySmall),
            ],
          ),
        ),
        if (warn) ...[
          const SizedBox(height: Space.x3),
          Row(
            children: [
              Icon(Symbols.warning_rounded, size: 16, color: c.warning),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Text(
                  l.isMemoryWarning(formatMemory(l, totalMb)),
                  style: context.text.bodySmall!.copyWith(color: c.warning),
                ),
              ),
            ],
          ),
        ],
      ],
    );
  }
}
