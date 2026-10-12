// Pestaña «Capturas»: galería de miniaturas con visor, abrir y borrar.

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

class ScreenshotsTab extends ConsumerWidget {
  const ScreenshotsTab({super.key, required this.instance});

  final InstanceView instance;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(screenshotsProvider(instance.id));
    final shots = async.value ?? const <ScreenshotView>[];
    if (shots.isEmpty) {
      return switch (async) {
        AsyncError(:final error) => Center(child: ErrorNote(error)),
        AsyncLoading() => const Center(child: CircularProgressIndicator()),
        _ => EmptyState(
            icon: Symbols.photo_library_rounded,
            title: l.screenshotsEmptyTitle,
            body: l.screenshotsEmptyBody,
          ),
      };
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Text(
              l.screenshotsCount(shots.length),
              style: context.text.bodyMedium!.copyWith(color: c.textSecondary),
            ),
            const Spacer(),
            TextButton.icon(
              onPressed: () =>
                  launchUrl(Uri.file('${instance.gameDir}/screenshots')),
              icon: const Icon(Symbols.folder_open_rounded, size: 18),
              label: Text(l.openFolder),
            ),
          ],
        ),
        const SizedBox(height: Space.x2),
        Expanded(
          child: GridView.builder(
            gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
              maxCrossAxisExtent: 300,
              childAspectRatio: 16 / 10,
              mainAxisSpacing: Space.x3,
              crossAxisSpacing: Space.x3,
            ),
            itemCount: shots.length,
            itemBuilder: (context, i) => _Thumb(
              shot: shots[i],
              onOpen: () => showScreenshotViewer(context, instance.id, shots, i),
            ),
          ),
        ),
      ],
    );
  }
}

class _Thumb extends StatefulWidget {
  const _Thumb({required this.shot, required this.onOpen});

  final ScreenshotView shot;
  final VoidCallback onOpen;

  @override
  State<_Thumb> createState() => _ThumbState();
}

class _ThumbState extends State<_Thumb> {
  bool _hover = false;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final s = widget.shot;
    return MouseRegion(
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        onTap: widget.onOpen,
        child: AnimatedContainer(
          duration: Motion.fast,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(Radii.card),
            border: Border.all(
              color: _hover ? c.accent.withValues(alpha: 0.6) : c.border,
              width: _hover ? 1.5 : 1,
            ),
          ),
          clipBehavior: Clip.antiAlias,
          child: Stack(
            fit: StackFit.expand,
            children: [
              LocalImage(
                path: s.path,
                cacheWidth: 600,
                fallback: BlockLandscape(seed: s.file, block: 12),
              ),
              Positioned(
                left: 0,
                right: 0,
                bottom: 0,
                child: Container(
                  padding: const EdgeInsets.fromLTRB(10, 18, 10, 8),
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      begin: Alignment.topCenter,
                      end: Alignment.bottomCenter,
                      colors: [
                        Colors.transparent,
                        Colors.black.withValues(alpha: 0.7),
                      ],
                    ),
                  ),
                  child: Text(
                    s.modified == null
                        ? s.file
                        : formatDateTime(l, s.modified!),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.text.labelMedium!.copyWith(color: Colors.white),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Visor a pantalla completa con flechas, abrir y borrar.
Future<void> showScreenshotViewer(
  BuildContext context,
  String instanceId,
  List<ScreenshotView> shots,
  int index,
) =>
    showDialog<void>(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.85),
      builder: (_) => _Viewer(instanceId: instanceId, shots: shots, index: index),
    );

class _Viewer extends ConsumerStatefulWidget {
  const _Viewer({
    required this.instanceId,
    required this.shots,
    required this.index,
  });

  final String instanceId;
  final List<ScreenshotView> shots;
  final int index;

  @override
  ConsumerState<_Viewer> createState() => _ViewerState();
}

class _ViewerState extends ConsumerState<_Viewer> {
  late List<ScreenshotView> _shots = widget.shots;
  late int _i = widget.index;
  final _focus = FocusNode();

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  void _go(int delta) {
    final n = _i + delta;
    if (n >= 0 && n < _shots.length) setState(() => _i = n);
  }

  Future<void> _delete() async {
    final l = AppLocalizations.of(context);
    final shot = _shots[_i];
    final ok = await confirm(
      context,
      title: l.screenshotDeleteTitle,
      body: l.screenshotDeleteBody(shot.file),
      action: l.delete,
      danger: true,
    );
    if (!ok || !mounted) return;
    try {
      await ref.read(bridgeProvider).deleteScreenshot(widget.instanceId, shot.file);
      ref.invalidate(screenshotsProvider(widget.instanceId));
      if (!mounted) return;
      final next = [..._shots]..removeAt(_i);
      if (next.isEmpty) {
        Navigator.pop(context);
        return;
      }
      setState(() {
        _shots = next;
        _i = _i.clamp(0, next.length - 1);
      });
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final s = _shots[_i];
    return KeyboardListener(
      focusNode: _focus,
      autofocus: true,
      onKeyEvent: (e) {
        if (e is! KeyDownEvent) return;
        if (e.logicalKey == LogicalKeyboardKey.arrowLeft) _go(-1);
        if (e.logicalKey == LogicalKeyboardKey.arrowRight) _go(1);
      },
      child: Padding(
        padding: const EdgeInsets.all(Space.x8),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    '${s.file}   ·   ${_i + 1} / ${_shots.length}',
                    style: context.text.titleSmall!.copyWith(color: Colors.white),
                  ),
                ),
                _ViewerButton(
                  icon: Symbols.open_in_new_rounded,
                  tooltip: l.screenshotOpen,
                  onPressed: () => launchUrl(Uri.file(s.path)),
                ),
                _ViewerButton(
                  icon: Symbols.delete_rounded,
                  tooltip: l.delete,
                  onPressed: _delete,
                ),
                _ViewerButton(
                  icon: Symbols.close_rounded,
                  tooltip: l.close,
                  onPressed: () => Navigator.pop(context),
                ),
              ],
            ),
            const SizedBox(height: Space.x4),
            Expanded(
              child: Row(
                children: [
                  _ViewerButton(
                    icon: Symbols.chevron_left_rounded,
                    tooltip: l.previous,
                    onPressed: _i > 0 ? () => _go(-1) : null,
                  ),
                  const SizedBox(width: Space.x3),
                  Expanded(
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(Radii.card),
                      child: LocalImage(
                        path: s.path,
                        fit: BoxFit.contain,
                        fallback: AspectRatio(
                          aspectRatio: 16 / 9,
                          child: BlockLandscape(seed: s.file, block: 22),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: Space.x3),
                  _ViewerButton(
                    icon: Symbols.chevron_right_rounded,
                    tooltip: l.next,
                    onPressed: _i < _shots.length - 1 ? () => _go(1) : null,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ViewerButton extends StatelessWidget {
  const _ViewerButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) => IconButton(
        tooltip: tooltip,
        style: IconButton.styleFrom(
          backgroundColor: Colors.white.withValues(alpha: 0.08),
          foregroundColor: Colors.white,
          disabledForegroundColor: Colors.white24,
          fixedSize: const Size.square(44),
        ),
        onPressed: onPressed,
        icon: Icon(icon, size: 22),
      );
}
