// Consola de una instancia: líneas coloreadas por nivel, filtro, seguir al
// final y copiar.

import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/logs.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';

/// Grupos de nivel que se pueden ocultar desde la barra.
enum _LevelGroup { error, warn, info, debug }

_LevelGroup _group(LogLevel l) => switch (l) {
      LogLevel.error => _LevelGroup.error,
      LogLevel.warn => _LevelGroup.warn,
      LogLevel.debug || LogLevel.trace => _LevelGroup.debug,
      _ => _LevelGroup.info,
    };

class ConsoleView extends ConsumerStatefulWidget {
  const ConsoleView({super.key, required this.instanceId});

  final String instanceId;

  @override
  ConsumerState<ConsoleView> createState() => _ConsoleViewState();
}

class _ConsoleViewState extends ConsumerState<ConsoleView> {
  final _scroll = ScrollController();
  final _filter = TextEditingController();
  final Set<_LevelGroup> _hidden = {};
  bool _follow = true;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
  }

  @override
  void dispose() {
    _scroll.dispose();
    _filter.dispose();
    super.dispose();
  }

  /// Si el usuario sube, se deja de seguir; al volver abajo, se reanuda.
  void _onScroll() {
    if (!_scroll.hasClients) return;
    final pos = _scroll.position;
    if (pos.userScrollDirection == ScrollDirection.idle) return;
    final atEnd = pos.pixels >= pos.maxScrollExtent - 24;
    if (atEnd != _follow) setState(() => _follow = atEnd);
  }

  void _toEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scroll.hasClients) _scroll.jumpTo(_scroll.position.maxScrollExtent);
    });
  }

  List<LogEntry> _visible(List<LogEntry> all) {
    final q = _filter.text.trim().toLowerCase();
    if (q.isEmpty && _hidden.isEmpty) return all;
    return [
      for (final e in all)
        if (!_hidden.contains(_group(e.level)) &&
            (q.isEmpty || e.text.toLowerCase().contains(q)))
          e,
    ];
  }

  Color _color(AppColors c, LogLevel level) => switch (level) {
        LogLevel.error => c.danger,
        LogLevel.warn => c.warning,
        LogLevel.info => c.textSecondary,
        LogLevel.debug || LogLevel.trace => c.textMuted,
        LogLevel.other => c.textSecondary,
      };

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final buffer = ref.watch(logBufferProvider(widget.instanceId));

    return ListenableBuilder(
      listenable: buffer,
      builder: (context, _) {
        final all = buffer.lines;
        final lines = _visible(all);
        if (_follow) _toEnd();
        final counts = <_LevelGroup, int>{};
        for (final e in all) {
          counts.update(_group(e.level), (v) => v + 1, ifAbsent: () => 1);
        }

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                SizedBox(
                  width: 260,
                  child: TextField(
                    controller: _filter,
                    onChanged: (_) => setState(() {}),
                    decoration: InputDecoration(
                      hintText: l.consoleFilter,
                      prefixIcon: const Icon(Symbols.filter_list_rounded, size: 18),
                    ),
                  ),
                ),
                const SizedBox(width: Space.x3),
                for (final g in _LevelGroup.values) ...[
                  _LevelChip(
                    label: switch (g) {
                      _LevelGroup.error => l.logLevelError,
                      _LevelGroup.warn => l.logLevelWarn,
                      _LevelGroup.info => l.logLevelInfo,
                      _LevelGroup.debug => l.logLevelDebug,
                    },
                    count: counts[g] ?? 0,
                    color: switch (g) {
                      _LevelGroup.error => c.danger,
                      _LevelGroup.warn => c.warning,
                      _LevelGroup.info => c.textSecondary,
                      _LevelGroup.debug => c.textMuted,
                    },
                    selected: !_hidden.contains(g),
                    onTap: () => setState(() {
                      if (!_hidden.remove(g)) _hidden.add(g);
                    }),
                  ),
                  const SizedBox(width: Space.x2),
                ],
                const Spacer(),
                Text(l.consoleLines(all.length), style: context.text.bodySmall),
                const SizedBox(width: Space.x3),
                _ToolButton(
                  icon: Symbols.vertical_align_bottom_rounded,
                  tooltip: l.consoleFollow,
                  active: _follow,
                  onPressed: () {
                    setState(() => _follow = !_follow);
                    if (_follow) _toEnd();
                  },
                ),
                _ToolButton(
                  icon: Symbols.content_copy_rounded,
                  tooltip: l.consoleCopy,
                  onPressed: lines.isEmpty
                      ? null
                      : () => copyText(
                            context,
                            lines.map((e) => e.text).join('\n'),
                            l.consoleCopied(lines.length),
                          ),
                ),
                _ToolButton(
                  icon: Symbols.delete_sweep_rounded,
                  tooltip: l.consoleClear,
                  onPressed: all.isEmpty ? null : buffer.clear,
                ),
              ],
            ),
            const SizedBox(height: Space.x3),
            Expanded(
              child: Container(
                decoration: BoxDecoration(
                  color: c.backgroundDeep.withValues(alpha: 0.75),
                  borderRadius: BorderRadius.circular(Radii.card),
                  border: Border.all(color: c.border),
                ),
                clipBehavior: Clip.antiAlias,
                child: all.isEmpty
                    ? EmptyState(
                        icon: Symbols.terminal_rounded,
                        title: l.consoleEmptyTitle,
                        body: l.consoleEmptyBody,
                      )
                    : lines.isEmpty
                        ? Center(
                            child: Text(
                              l.consoleNoMatches,
                              style: context.text.bodyMedium!.copyWith(
                                color: c.textSecondary,
                              ),
                            ),
                          )
                        : Scrollbar(
                            controller: _scroll,
                            child: ListView.builder(
                              controller: _scroll,
                              padding: const EdgeInsets.symmetric(vertical: Space.x3),
                              itemCount: lines.length,
                              itemBuilder: (context, i) {
                                final e = lines[i];
                                return _LogLine(
                                  number: i + 1,
                                  text: e.text,
                                  color: _color(c, e.level),
                                  marker: e.level == LogLevel.error
                                      ? c.danger
                                      : e.level == LogLevel.warn
                                          ? c.warning
                                          : null,
                                );
                              },
                            ),
                          ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _LogLine extends StatelessWidget {
  const _LogLine({
    required this.number,
    required this.text,
    required this.color,
    this.marker,
  });

  final int number;
  final String text;
  final Color color;
  final Color? marker;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    const style = TextStyle(fontFamily: kMonoFamily, fontSize: 12, height: 1.55);
    return Container(
      decoration: BoxDecoration(
        color: marker?.withValues(alpha: 0.06),
        border: Border(
          left: BorderSide(color: marker ?? Colors.transparent, width: 2),
        ),
      ),
      padding: const EdgeInsets.only(right: Space.x4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 52,
            child: Text(
              '$number',
              textAlign: TextAlign.right,
              style: style.copyWith(color: c.textMuted.withValues(alpha: 0.6)),
            ),
          ),
          const SizedBox(width: Space.x4),
          Expanded(
            child: Text(text, style: style.copyWith(color: color)),
          ),
        ],
      ),
    );
  }
}

class _LevelChip extends StatelessWidget {
  const _LevelChip({
    required this.label,
    required this.count,
    required this.color,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final int count;
  final Color color;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(Radii.pill),
      child: AnimatedContainer(
        duration: Motion.fast,
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          color: selected ? c.surface2 : Colors.transparent,
          borderRadius: BorderRadius.circular(Radii.pill),
          border: Border.all(color: selected ? c.borderStrong : c.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 7,
              height: 7,
              decoration: BoxDecoration(
                color: selected ? color : Colors.transparent,
                shape: BoxShape.circle,
                border: Border.all(color: color, width: 1.2),
              ),
            ),
            const SizedBox(width: 6),
            Text(
              label,
              style: context.text.labelMedium!.copyWith(
                color: selected ? c.text : c.textMuted,
              ),
            ),
            const SizedBox(width: 6),
            Text(
              '$count',
              style: context.text.labelMedium!.copyWith(
                color: c.textMuted,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ToolButton extends StatelessWidget {
  const _ToolButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.active = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return IconButton(
      tooltip: tooltip,
      isSelected: active,
      style: IconButton.styleFrom(
        backgroundColor: active ? c.accentContainer : null,
        foregroundColor: active ? c.onAccentContainer : c.textSecondary,
      ),
      onPressed: onPressed,
      icon: Icon(icon, size: 19),
    );
  }
}
