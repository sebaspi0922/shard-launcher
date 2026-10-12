// Tooltip a la derecha del elemento y centrado con él (el de Material solo
// sabe ponerse encima o debajo).

import 'dart:async';

import 'package:material_ui/material_ui.dart';

import '../theme/tokens.dart';

class SideTooltip extends StatefulWidget {
  const SideTooltip({
    super.key,
    required this.message,
    required this.child,
    this.gap = Space.x2,
  });

  final String message;
  final Widget child;

  /// Separación entre el borde derecho del elemento y el tooltip.
  final double gap;

  @override
  State<SideTooltip> createState() => _SideTooltipState();
}

class _SideTooltipState extends State<SideTooltip>
    with SingleTickerProviderStateMixin {
  final _link = LayerLink();
  final _portal = OverlayPortalController();
  late final _anim = AnimationController(vsync: this, duration: Motion.fast);
  Timer? _wait;

  void _show() {
    _wait?.cancel();
    final delay = TooltipTheme.of(context).waitDuration ?? Duration.zero;
    _wait = Timer(delay, () {
      if (!mounted) return;
      _portal.show();
      _anim.forward();
    });
  }

  void _hide() {
    _wait?.cancel();
    _anim.reverse().whenComplete(() {
      if (mounted && _anim.isDismissed) _portal.hide();
    });
  }

  @override
  void dispose() {
    _wait?.cancel();
    _anim.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = TooltipTheme.of(context);
    return CompositedTransformTarget(
      link: _link,
      child: OverlayPortal(
        controller: _portal,
        overlayChildBuilder: (_) => Align(
          alignment: Alignment.topLeft,
          child: CompositedTransformFollower(
            link: _link,
            targetAnchor: Alignment.centerRight,
            followerAnchor: Alignment.centerLeft,
            offset: Offset(widget.gap, 0),
            child: IgnorePointer(
              child: FadeTransition(
                opacity: CurvedAnimation(parent: _anim, curve: Motion.curve),
                child: Semantics(
                  tooltip: widget.message,
                  child: Container(
                    padding: theme.padding,
                    decoration: theme.decoration,
                    child: Text(widget.message, style: theme.textStyle),
                  ),
                ),
              ),
            ),
          ),
        ),
        child: MouseRegion(
          onEnter: (_) => _show(),
          onExit: (_) => _hide(),
          child: Listener(
            onPointerDown: (_) => _hide(),
            child: widget.child,
          ),
        ),
      ),
    );
  }
}
