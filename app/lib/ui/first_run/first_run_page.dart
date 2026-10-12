// Asistente de primer arranque: bienvenida e idioma → importar (Roxy y
// .minecraft, simulando antes) → añadir cuenta → listo. Se puede saltar.

import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';
import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../core/tasks.dart';
import '../../core/ui_prefs.dart';
import '../../l10n/app_localizations.dart';
import '../accounts/account_dialogs.dart';
import '../shell/backdrop.dart';
import '../shell/title_bar.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/avatar.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

enum FirstRunStep { welcome, import, account, done }

/// Paso inicial (los tests abren uno concreto).
final firstRunInitialStepProvider =
    Provider<FirstRunStep>((ref) => FirstRunStep.welcome);

final importSourcesProvider = FutureProvider.autoDispose<ImportSourcesView>(
  (ref) => ref.watch(bridgeProvider).detectImportSources(),
);

final roxyPlanProvider = FutureProvider.autoDispose<RoxyPlanView>(
  (ref) => ref.watch(bridgeProvider).planRoxyImport(),
);

final minecraftPlanProvider = FutureProvider.autoDispose<MinecraftPlanView>(
  (ref) => ref.watch(bridgeProvider).planMinecraftImport(),
);

/// ¿Hay que mostrar el asistente? Explícitamente pendiente, o sin marcar y
/// sin instancias ni cuentas.
bool needsFirstRun(
  UiPrefs prefs, {
  required bool hasInstances,
  required bool hasAccounts,
}) {
  if (prefs.firstRunDone == true) return false;
  if (prefs.firstRunDone == false) return true;
  return !hasInstances && !hasAccounts;
}

class FirstRunPage extends ConsumerStatefulWidget {
  const FirstRunPage({super.key});

  @override
  ConsumerState<FirstRunPage> createState() => _FirstRunPageState();
}

class _FirstRunPageState extends ConsumerState<FirstRunPage> {
  late FirstRunStep _step = ref.read(firstRunInitialStepProvider);

  void _go(FirstRunStep s) => setState(() => _step = s);

  Future<void> _finish() async {
    await ref
        .read(uiPrefsProvider.notifier)
        .update((p) => p.copyWith(firstRunDone: true));
    if (mounted) context.go('/');
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Stack(
        fit: StackFit.expand,
        children: [
          const AppBackdrop(),
          Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const TitleBar(),
              Expanded(
                child: Center(
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(
                      Space.x8,
                      0,
                      Space.x8,
                      Space.x10,
                    ),
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(
                        maxWidth: 980,
                        maxHeight: 620,
                      ),
                      child: Container(
                        clipBehavior: Clip.antiAlias,
                        decoration: BoxDecoration(
                          color: Color.alphaBlend(
                            c.surface1,
                            c.backgroundDeep.withValues(alpha: 0.82),
                          ),
                          borderRadius: BorderRadius.circular(Radii.panel + 4),
                          border: Border.all(color: c.borderStrong),
                          boxShadow: [
                            BoxShadow(
                              color: Colors.black.withValues(alpha: 0.4),
                              blurRadius: 48,
                              offset: const Offset(0, 18),
                            ),
                          ],
                        ),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            _StepRail(step: _step, onSkip: _finish),
                            Expanded(
                              child: Padding(
                                padding: const EdgeInsets.fromLTRB(
                                  Space.x10,
                                  Space.x8,
                                  Space.x10,
                                  Space.x6,
                                ),
                                child: AnimatedSwitcher(
                                  duration: Motion.medium,
                                  switchInCurve: Motion.curve,
                                  transitionBuilder: (child, anim) =>
                                      FadeTransition(
                                    opacity: anim,
                                    child: SlideTransition(
                                      position: Tween(
                                        begin: const Offset(0.03, 0),
                                        end: Offset.zero,
                                      ).animate(anim),
                                      child: child,
                                    ),
                                  ),
                                  child: KeyedSubtree(
                                    key: ValueKey(_step),
                                    child: switch (_step) {
                                      FirstRunStep.welcome => _WelcomeStep(
                                          onNext: () => _go(FirstRunStep.import),
                                        ),
                                      FirstRunStep.import => _ImportStep(
                                          onBack: () => _go(FirstRunStep.welcome),
                                          onNext: () => _go(FirstRunStep.account),
                                        ),
                                      FirstRunStep.account => _AccountStep(
                                          onBack: () => _go(FirstRunStep.import),
                                          onNext: () => _go(FirstRunStep.done),
                                        ),
                                      FirstRunStep.done => _DoneStep(onFinish: _finish),
                                    },
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
          // Saltar siempre a mano, arriba a la izquierda (lejos de los
          // botones de ventana).
          Positioned(
            left: Space.x4,
            top: 6,
            child: TextButton.icon(
              style: TextButton.styleFrom(
                foregroundColor: c.textSecondary,
                minimumSize: const Size(0, 32),
              ),
              onPressed: _finish,
              icon: const Icon(Symbols.skip_next_rounded, size: 18),
              label: Text(l.firstRunSkip),
            ),
          ),
        ],
      ),
    );
  }
}

class _StepRail extends StatelessWidget {
  const _StepRail({required this.step, required this.onSkip});

  final FirstRunStep step;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final labels = [
      l.firstRunStepWelcome,
      l.firstRunStepImport,
      l.firstRunStepAccount,
      l.firstRunStepDone,
    ];
    return Container(
      width: 240,
      padding: const EdgeInsets.fromLTRB(Space.x6, Space.x8, Space.x6, Space.x6),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.18),
        border: Border(right: BorderSide(color: c.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const BrandMark(size: 30),
              const SizedBox(width: Space.x3),
              Text(kAppName, style: context.text.titleLarge),
            ],
          ),
          const SizedBox(height: Space.x8),
          for (final (i, label) in labels.indexed) ...[
            _StepItem(
              index: i,
              label: label,
              state: i < step.index
                  ? _StepState.done
                  : i == step.index
                      ? _StepState.current
                      : _StepState.todo,
            ),
            if (i < labels.length - 1)
              Container(
                margin: const EdgeInsets.only(left: 13),
                width: 2,
                height: 22,
                color: i < step.index ? c.accent.withValues(alpha: 0.6) : c.border,
              ),
          ],
          const Spacer(),
          Text(
            l.firstRunRailHint,
            style: context.text.bodySmall!.copyWith(color: c.textMuted),
          ),
        ],
      ),
    );
  }
}

enum _StepState { done, current, todo }

class _StepItem extends StatelessWidget {
  const _StepItem({
    required this.index,
    required this.label,
    required this.state,
  });

  final int index;
  final String label;
  final _StepState state;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final active = state != _StepState.todo;
    return Row(
      children: [
        AnimatedContainer(
          duration: Motion.medium,
          width: 28,
          height: 28,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: switch (state) {
              _StepState.done => c.accent,
              _StepState.current => c.accentContainer,
              _StepState.todo => c.surface2,
            },
            border: Border.all(
              color: state == _StepState.current ? c.accent : c.border,
              width: state == _StepState.current ? 2 : 1,
            ),
          ),
          child: state == _StepState.done
              ? Icon(Symbols.check_rounded, size: 16, color: c.onAccent, weight: 700)
              : Text(
                  '${index + 1}',
                  style: context.text.labelLarge!.copyWith(
                    color: active ? c.onAccentContainer : c.textMuted,
                  ),
                ),
        ),
        const SizedBox(width: Space.x3),
        Text(
          label,
          style: context.text.labelLarge!.copyWith(
            color: active ? c.text : c.textMuted,
            fontWeight: state == _StepState.current ? FontWeight.w600 : FontWeight.w500,
          ),
        ),
      ],
    );
  }
}

/// Cabecera común de cada paso.
class _StepHeader extends StatelessWidget {
  const _StepHeader({required this.title, required this.body, this.eyebrow});

  final String title;
  final String body;
  final String? eyebrow;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (eyebrow != null) ...[
          Text(
            eyebrow!.toUpperCase(),
            style: context.text.labelSmall!.copyWith(
              color: c.accent,
              letterSpacing: 1.2,
            ),
          ),
          const SizedBox(height: Space.x2),
        ],
        Text(title, style: context.text.headlineMedium),
        const SizedBox(height: Space.x2),
        Text(
          body,
          style: context.text.bodyMedium!.copyWith(color: c.textSecondary, height: 1.5),
        ),
      ],
    );
  }
}

/// Barra de acciones de cada paso.
class _StepActions extends StatelessWidget {
  const _StepActions({this.back, required this.primary, this.secondary});

  final VoidCallback? back;
  final Widget primary;
  final Widget? secondary;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Row(
      children: [
        if (back != null)
          TextButton.icon(
            onPressed: back,
            icon: const Icon(Symbols.arrow_back_rounded, size: 18),
            label: Text(l.back),
          ),
        const Spacer(),
        if (secondary != null) ...[secondary!, const SizedBox(width: Space.x2)],
        primary,
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// 1. Bienvenida e idioma
// ---------------------------------------------------------------------------

class _WelcomeStep extends ConsumerWidget {
  const _WelcomeStep({required this.onNext});

  final VoidCallback onNext;

  Future<void> _setLanguage(WidgetRef ref, String? lang) async {
    final s = ref.read(settingsProvider).value;
    if (s == null) return;
    await ref.read(settingsProvider.notifier).save(s.copyWith(language: () => lang));
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final lang = ref.watch(settingsProvider).value?.language;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _StepHeader(
          eyebrow: l.firstRunStepWelcome,
          title: l.firstRunWelcomeTitle(kAppName),
          body: l.firstRunWelcomeBody,
        ),
        const SizedBox(height: Space.x8),
        Text(l.settingsLanguage, style: context.text.titleSmall),
        const SizedBox(height: Space.x3),
        Row(
          children: [
            for (final (code, label, detail) in [
              (null, l.languageSystem, l.firstRunLanguageSystem),
              ('es', 'Español', 'España · Latinoamérica'),
              ('en', 'English', 'United Kingdom · United States'),
            ]) ...[
              Expanded(
                child: _ChoiceCard(
                  icon: code == null ? Symbols.computer_rounded : Symbols.translate_rounded,
                  title: label,
                  body: detail,
                  selected: lang == code,
                  onTap: () => _setLanguage(ref, code),
                ),
              ),
              if (code != 'en') const SizedBox(width: Space.x3),
            ],
          ],
        ),
        const SizedBox(height: Space.x8),
        Text(l.firstRunHighlights, style: context.text.titleSmall),
        const SizedBox(height: Space.x3),
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final (i, (icon, title, body)) in [
              (Symbols.deployed_code_rounded, l.firstRunFeatureInstances, l.firstRunFeatureInstancesBody),
              (Symbols.extension_rounded, l.firstRunFeatureContent, l.firstRunFeatureContentBody),
              (Symbols.stethoscope_rounded, l.firstRunFeatureDoctor, l.firstRunFeatureDoctorBody),
            ].indexed) ...[
              if (i > 0) const SizedBox(width: Space.x5),
              Expanded(child: _Feature(icon: icon, title: title, body: body)),
            ],
          ],
        ),
        const Spacer(),
        _StepActions(
          primary: FilledButton.icon(
            onPressed: onNext,
            icon: const Icon(Symbols.arrow_forward_rounded, size: 18),
            label: Text(l.firstRunStart),
          ),
        ),
      ],
    );
  }
}

class _Feature extends StatelessWidget {
  const _Feature({required this.icon, required this.title, required this.body});

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20, color: c.accent),
        const SizedBox(width: Space.x3),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: context.text.labelLarge),
              const SizedBox(height: 2),
              Text(body, style: context.text.bodySmall!.copyWith(height: 1.4)),
            ],
          ),
        ),
      ],
    );
  }
}

class _ChoiceCard extends StatelessWidget {
  const _ChoiceCard({
    required this.icon,
    required this.title,
    required this.body,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String body;
  final bool selected;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Material(
      color: selected ? c.accentContainer.withValues(alpha: 0.55) : c.surface1,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        side: BorderSide(
          color: selected ? c.accent : c.border,
          width: selected ? 2 : 1,
        ),
      ),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(Radii.card),
        child: Padding(
          padding: const EdgeInsets.all(Space.x4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(icon, size: 22, color: selected ? c.accent : c.textSecondary),
                  const Spacer(),
                  if (selected)
                    Icon(Symbols.check_circle_rounded, size: 18, color: c.accent, fill: 1),
                ],
              ),
              const SizedBox(height: Space.x3),
              Text(title, style: context.text.titleSmall),
              const SizedBox(height: 2),
              Text(
                body,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: context.text.bodySmall,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// 2. Importar
// ---------------------------------------------------------------------------

enum _ImportPhase { choose, running, done }

class _ImportStep extends ConsumerStatefulWidget {
  const _ImportStep({required this.onBack, required this.onNext});

  final VoidCallback onBack;
  final VoidCallback onNext;

  @override
  ConsumerState<_ImportStep> createState() => _ImportStepState();
}

class _ImportStepState extends ConsumerState<_ImportStep> {
  bool _roxy = true;
  bool _minecraft = false;
  _ImportPhase _phase = _ImportPhase.choose;

  /// Origen que se está importando ahora (`roxy` o `minecraft`).
  String? _current;
  int _imported = 0;
  final List<Object> _errors = [];

  Future<void> _run(ImportSourcesView sources) async {
    final bridge = ref.read(bridgeProvider);
    setState(() {
      _phase = _ImportPhase.running;
      _imported = 0;
      _errors.clear();
    });
    if (_roxy && sources.roxy != null) {
      setState(() => _current = 'roxy');
      try {
        final r = await bridge.importRoxy();
        _imported += r.imported.length;
      } catch (e) {
        _errors.add(e);
      }
    }
    if (_minecraft && sources.minecraft != null && mounted) {
      setState(() => _current = 'minecraft');
      try {
        final id = await bridge.importMinecraft();
        if (id != null) _imported++;
      } catch (e) {
        _errors.add(e);
      }
    }
    if (!mounted) return;
    await ref.read(instancesProvider.notifier).refresh();
    if (mounted) {
      setState(() {
        _current = null;
        _phase = _ImportPhase.done;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(importSourcesProvider);
    final sources = async.value;
    final none = sources != null && sources.roxy == null && sources.minecraft == null;
    final selected = sources != null &&
        ((_roxy && sources.roxy != null) || (_minecraft && sources.minecraft != null));
    final running = _phase == _ImportPhase.running;
    final progress = ref.watch(stageProgressProvider('import'));

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _StepHeader(
          eyebrow: l.firstRunStepImport,
          title: l.firstRunImportTitle,
          body: l.firstRunImportBody,
        ),
        const SizedBox(height: Space.x6),
        Expanded(
          child: switch (async) {
            AsyncError(:final error) => Align(
                alignment: Alignment.topCenter,
                child: ErrorNote(error),
              ),
            AsyncData() when none => EmptyState(
                icon: Symbols.travel_explore_rounded,
                title: l.firstRunImportNoneTitle,
                body: l.firstRunImportNoneBody,
              ),
            AsyncData() => SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (sources!.roxy != null)
                      _SourceCard(
                        icon: Symbols.rocket_launch_rounded,
                        title: l.firstRunRoxy,
                        path: sources.roxy!,
                        selected: _roxy,
                        enabled: _phase == _ImportPhase.choose,
                        status: _status(l, 'roxy', _roxy),
                        onChanged: (v) => setState(() => _roxy = v),
                        child: const _RoxyPlan(),
                      ),
                    if (sources.roxy != null && sources.minecraft != null)
                      const SizedBox(height: Space.x3),
                    if (sources.minecraft != null)
                      _SourceCard(
                        icon: Symbols.deployed_code_rounded,
                        title: l.firstRunMinecraft,
                        path: sources.minecraft!,
                        selected: _minecraft,
                        enabled: _phase == _ImportPhase.choose,
                        status: _status(l, 'minecraft', _minecraft),
                        onChanged: (v) => setState(() => _minecraft = v),
                        child: const _MinecraftPlan(),
                      ),
                  ],
                ),
              ),
            _ => const Center(child: CircularProgressIndicator()),
          },
        ),
        if (running) ...[
          const SizedBox(height: Space.x4),
          Row(
            children: [
              Expanded(
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: LinearProgressIndicator(
                    minHeight: 6,
                    value: progress?.fraction,
                  ),
                ),
              ),
              const SizedBox(width: Space.x3),
              Text(
                progress == null || progress.total == 0
                    ? l.firstRunImporting
                    : l.firstRunImportingFiles(progress.done, progress.total),
                style: context.text.bodySmall,
              ),
            ],
          ),
        ],
        if (_phase == _ImportPhase.done) ...[
          const SizedBox(height: Space.x4),
          if (_errors.isEmpty)
            Row(
              children: [
                Icon(Symbols.check_circle_rounded, color: c.success, size: 20, fill: 1),
                const SizedBox(width: Space.x2),
                Text(l.firstRunImported(_imported), style: context.text.titleSmall),
              ],
            )
          else
            ErrorNote(_errors.first),
        ],
        const SizedBox(height: Space.x5),
        _StepActions(
          back: running ? null : widget.onBack,
          secondary: _phase == _ImportPhase.choose && !none
              ? TextButton(onPressed: widget.onNext, child: Text(l.firstRunNotNow))
              : null,
          primary: _phase == _ImportPhase.choose && selected
              ? FilledButton.icon(
                  onPressed: () => _run(sources),
                  icon: const Icon(Symbols.download_rounded, size: 18),
                  label: Text(l.firstRunImportAction),
                )
              : FilledButton.icon(
                  onPressed: running ? null : widget.onNext,
                  icon: const Icon(Symbols.arrow_forward_rounded, size: 18),
                  label: Text(l.next),
                ),
        ),
      ],
    );
  }

  String? _status(AppLocalizations l, String source, bool chosen) {
    if (_phase == _ImportPhase.choose || !chosen) return null;
    if (_current == source) return l.firstRunImporting;
    return _phase == _ImportPhase.done || _current != null ? l.firstRunSourceDone : null;
  }
}

class _SourceCard extends StatelessWidget {
  const _SourceCard({
    required this.icon,
    required this.title,
    required this.path,
    required this.selected,
    required this.enabled,
    required this.status,
    required this.onChanged,
    required this.child,
  });

  final IconData icon;
  final String title;
  final String path;
  final bool selected;
  final bool enabled;
  final String? status;
  final ValueChanged<bool> onChanged;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return AnimatedContainer(
      duration: Motion.fast,
      decoration: BoxDecoration(
        color: selected ? c.surface2 : c.surface1,
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(
          color: selected ? c.accent.withValues(alpha: 0.55) : c.border,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          InkWell(
            borderRadius: const BorderRadius.vertical(top: Radius.circular(Radii.card)),
            onTap: enabled ? () => onChanged(!selected) : null,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(Space.x4, Space.x3, Space.x3, Space.x3),
              child: Row(
                children: [
                  Container(
                    width: 36,
                    height: 36,
                    decoration: BoxDecoration(
                      color: c.accentContainer.withValues(alpha: 0.6),
                      borderRadius: BorderRadius.circular(Radii.control),
                    ),
                    child: Icon(icon, size: 20, color: c.accent),
                  ),
                  const SizedBox(width: Space.x3),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(title, style: context.text.titleSmall),
                        Text(
                          path,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontFamily: kMonoFamily,
                            fontSize: 11.5,
                            color: c.textMuted,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (status != null) ...[
                    ColorBadge(status!, color: c.accent),
                    const SizedBox(width: Space.x2),
                  ],
                  Checkbox(
                    value: selected,
                    onChanged: enabled ? (v) => onChanged(v ?? false) : null,
                  ),
                ],
              ),
            ),
          ),
          if (selected) ...[
            Divider(height: 1, color: c.border),
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.x4, Space.x3, Space.x4, Space.x4),
              child: child,
            ),
          ],
        ],
      ),
    );
  }
}

class _RoxyPlan extends ConsumerWidget {
  const _RoxyPlan();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return switch (ref.watch(roxyPlanProvider)) {
      AsyncError(:final error) => ErrorNote(error),
      AsyncData(value: final p) => () {
          final todo = p.instances.where((i) => i.skippedReason == null).toList();
          final bytes = todo.fold<BigInt>(BigInt.zero, (a, i) => a + i.bytes);
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                l.firstRunRoxySummary(
                  todo.length,
                  formatBytes(l, bytes),
                  formatBytes(l, p.sharedBytes),
                ),
                style: context.text.bodySmall!.copyWith(color: c.textSecondary),
              ),
              const SizedBox(height: Space.x3),
              for (final i in p.instances)
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Opacity(
                    opacity: i.skippedReason == null ? 1 : 0.5,
                    child: Row(
                      children: [
                        InstanceIcon(id: i.originalId, name: i.name, size: 26),
                        const SizedBox(width: Space.x3),
                        Expanded(
                          child: Text(
                            i.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.bodyMedium,
                          ),
                        ),
                        Text(
                          versionLine(l, i.mc, i.loader),
                          style: context.text.bodySmall,
                        ),
                        const SizedBox(width: Space.x4),
                        SizedBox(
                          width: 150,
                          child: Text(
                            i.skippedReason == null
                                ? [
                                    formatBytes(l, i.bytes),
                                    if (i.backups > 0) l.firstRunBackups(i.backups),
                                  ].join(' · ')
                                : l.firstRunSkipped(i.skippedReason!),
                            textAlign: TextAlign.end,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.bodySmall!.copyWith(
                              fontFeatures: const [FontFeature.tabularFigures()],
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              for (final w in p.warnings)
                Text('· $w', style: context.text.bodySmall!.copyWith(color: c.warning)),
            ],
          );
        }(),
      _ => const LinearProgressIndicator(minHeight: 2),
    };
  }
}

class _MinecraftPlan extends ConsumerWidget {
  const _MinecraftPlan();

  String _section(AppLocalizations l, String s) => switch (s) {
        'saves' => l.sectionSaves,
        'mods' => l.sectionMods,
        'config' => l.sectionConfig,
        'resourcepacks' => l.sectionResourcepacks,
        'shaderpacks' => l.sectionShaderpacks,
        'screenshots' => l.sectionScreenshots,
        'options' => l.sectionOptions,
        'servers' => l.sectionServers,
        _ => s,
      };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return switch (ref.watch(minecraftPlanProvider)) {
      AsyncError(:final error) => ErrorNote(error),
      AsyncData(value: final p) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              l.firstRunMinecraftSummary(
                p.name,
                versionLine(l, p.mc, p.loader),
                formatBytes(l, p.bytes),
              ),
              style: context.text.bodySmall!.copyWith(color: c.textSecondary),
            ),
            const SizedBox(height: Space.x3),
            Wrap(
              spacing: Space.x2,
              runSpacing: Space.x2,
              children: [
                for (final s in p.sections.where((s) => s.present))
                  Tag(
                    '${_section(l, s.section)} · ${formatBytes(l, s.bytes)}',
                    icon: Symbols.check_rounded,
                  ),
              ],
            ),
            for (final w in p.warnings)
              Padding(
                padding: const EdgeInsets.only(top: Space.x2),
                child: Text('· $w', style: context.text.bodySmall!.copyWith(color: c.warning)),
              ),
          ],
        ),
      _ => const LinearProgressIndicator(minHeight: 2),
    };
  }
}

// ---------------------------------------------------------------------------
// 3. Cuenta
// ---------------------------------------------------------------------------

class _AccountStep extends ConsumerWidget {
  const _AccountStep({required this.onBack, required this.onNext});

  final VoidCallback onBack;
  final VoidCallback onNext;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final accounts = ref.watch(accountsProvider).value ?? const <AccountView>[];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _StepHeader(
          eyebrow: l.firstRunStepAccount,
          title: l.firstRunAccountTitle,
          body: l.firstRunAccountBody,
        ),
        const SizedBox(height: Space.x6),
        IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(
                child: _ChoiceCard(
                  icon: Symbols.verified_user_rounded,
                  title: l.accountsAddMicrosoft,
                  body: l.firstRunMicrosoftDesc,
                  selected: false,
                  onTap: () => showMicrosoftLogin(context),
                ),
              ),
              const SizedBox(width: Space.x3),
              Expanded(
                child: _ChoiceCard(
                  icon: Symbols.person_rounded,
                  title: l.accountsAddOffline,
                  body: l.firstRunOfflineDesc,
                  selected: false,
                  onTap: () => showOfflineAccount(context),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: Space.x5),
        if (accounts.isNotEmpty) ...[
          Text(l.firstRunAccountsAdded, style: context.text.titleSmall),
          const SizedBox(height: Space.x2),
          Expanded(
            child: ListView(
              children: [
                for (final a in accounts)
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.x2),
                    child: Panel(
                      padding: const EdgeInsets.all(Space.x3),
                      child: Row(
                        children: [
                          PlayerAvatar(account: a, size: 32),
                          const SizedBox(width: Space.x3),
                          Expanded(child: Text(a.name, style: context.text.titleSmall)),
                          ColorBadge(
                            a.microsoft ? l.accountMicrosoft : l.accountOffline,
                            color: a.microsoft ? c.accent : c.textSecondary,
                          ),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ] else
          const Spacer(),
        _StepActions(
          back: onBack,
          secondary: accounts.isEmpty
              ? TextButton(onPressed: onNext, child: Text(l.firstRunNotNow))
              : null,
          primary: FilledButton.icon(
            onPressed: onNext,
            icon: const Icon(Symbols.arrow_forward_rounded, size: 18),
            label: Text(l.next),
          ),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// 4. Listo
// ---------------------------------------------------------------------------

class _DoneStep extends ConsumerWidget {
  const _DoneStep({required this.onFinish});

  final Future<void> Function() onFinish;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final instances = ref.watch(instancesProvider).value?.length ?? 0;
    final account = ref.watch(activeAccountProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _StepHeader(
          eyebrow: l.firstRunStepDone,
          title: l.firstRunDoneTitle,
          body: l.firstRunDoneBody,
        ),
        const SizedBox(height: Space.x6),
        Panel(
          child: Column(
            children: [
              _SummaryRow(
                icon: Symbols.deployed_code_rounded,
                label: l.navInstances,
                value: instances == 0 ? l.firstRunNoInstances : l.instancesCount(instances),
              ),
              Divider(color: c.border),
              _SummaryRow(
                icon: Symbols.person_rounded,
                label: l.firstRunAccount,
                value: account?.name ?? l.accountNone,
              ),
              Divider(color: c.border),
              _SummaryRow(
                icon: Symbols.palette_rounded,
                label: l.settingsAppearance,
                value: l.firstRunAppearanceHint,
              ),
            ],
          ),
        ),
        const Spacer(),
        _StepActions(
          primary: FilledButton.icon(
            onPressed: () => unawaited(onFinish()),
            icon: const Icon(Symbols.rocket_launch_rounded, size: 18),
            label: Text(l.firstRunFinish(kAppName)),
          ),
        ),
      ],
    );
  }
}

class _SummaryRow extends StatelessWidget {
  const _SummaryRow({
    required this.icon,
    required this.label,
    required this.value,
  });

  final IconData icon;
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.x2),
      child: Row(
        children: [
          Icon(icon, size: 20, color: c.accent),
          const SizedBox(width: Space.x3),
          Text(label, style: context.text.titleSmall),
          const Spacer(),
          Text(value, style: context.text.bodyMedium!.copyWith(color: c.textSecondary)),
        ],
      ),
    );
  }
}
