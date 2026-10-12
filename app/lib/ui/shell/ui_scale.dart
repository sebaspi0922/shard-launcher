// Escala toda la UI con la ventana (0,8×–1,5× respecto a 1280×800).

import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';

double uiScaleFor(Size size) => math
    .min(size.width / kDesignWidth, size.height / kDesignHeight)
    .clamp(0.8, 1.5);

class UiScale extends StatelessWidget {
  const UiScale({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final mq = MediaQuery.of(context);
    final s = uiScaleFor(mq.size);
    if ((s - 1).abs() < 0.01) return child;
    final logical = mq.size / s;
    return FittedBox(
      fit: BoxFit.fill,
      alignment: Alignment.topLeft,
      child: SizedBox.fromSize(
        size: logical,
        child: MediaQuery(
          data: mq.copyWith(
            size: logical,
            padding: mq.padding / s,
            viewPadding: mq.viewPadding / s,
            viewInsets: mq.viewInsets / s,
          ),
          child: child,
        ),
      ),
    );
  }
}
