// Doctor: diagnósticos de la última partida por gravedad, con su explicación,
// el culpable y el arreglo automático si lo hay.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';

/// Informe de Doctor de una instancia (se recalcula al invalidarlo).
final doctorReportProvider =
    FutureProvider.autoDispose.family<DoctorReportView, String>(
  (ref, id) => ref.watch(bridgeProvider).diagnose(id),
);

Future<void> showDoctor(BuildContext context, InstanceView instance) =>
    showDialog<void>(
      context: context,
      builder: (_) => DoctorDialog(instance: instance),
    );

Color severityColor(AppColors c, String severity) => switch (severity) {
      'critical' => c.danger,
      'warning' => c.warning,
      _ => c.info,
    };

IconData categoryIcon(String category) => switch (category) {
      'java' => Symbols.coffee_rounded,
      'memory' => Symbols.memory_rounded,
      'jvm_args' => Symbols.terminal_rounded,
      'mod_conflict' => Symbols.extension_off_rounded,
      'missing_dependency' => Symbols.link_off_rounded,
      'corrupted_file' => Symbols.broken_image_rounded,
      'graphics_driver' => Symbols.monitor_rounded,
      'loader' => Symbols.build_rounded,
      'account' => Symbols.person_rounded,
      'network' => Symbols.wifi_off_rounded,
      'system' => Symbols.computer_rounded,
      'world_corruption' => Symbols.public_off_rounded,
      _ => Symbols.help_rounded,
    };

class DoctorDialog extends ConsumerStatefulWidget {
  const DoctorDialog({super.key, required this.instance});

  final InstanceView instance;

  @override
  ConsumerState<DoctorDialog> createState() => _DoctorDialogState();
}

class _DoctorDialogState extends ConsumerState<DoctorDialog> {
  String? _applying;

  Future<void> _apply(DiagnosisView d) async {
    final l = AppLocalizations.of(context);
    setState(() => _applying = d.id);
    try {
      final msg = await ref
          .read(bridgeProvider)
          .applyFix(widget.instance.id, d.id);
      // Memoria, JVM o loader cambian la instancia.
      await ref.read(instancesProvider.notifier).refresh();
      ref.invalidate(doctorReportProvider(widget.instance.id));
      if (mounted) showToast(context, msg.isEmpty ? l.doctorFixApplied : msg);
    } catch (e) {
      if (mounted) showToast(context, errorText(l, e), error: true);
    } finally {
      if (mounted) setState(() => _applying = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final async = ref.watch(doctorReportProvider(widget.instance.id));
    final report = async.value;

    final Widget body;
    if (report == null) {
      body = async.hasError
          ? ErrorNote(async.error!)
          : SizedBox(
              height: 220,
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const CircularProgressIndicator(),
                    const SizedBox(height: Space.x4),
                    Text(l.doctorAnalyzing, style: context.text.bodySmall),
                  ],
                ),
              ),
            );
    } else if (report.diagnoses.isEmpty) {
      body = SizedBox(
        height: 260,
        child: EmptyState(
          icon: Symbols.verified_rounded,
          title: l.doctorHealthyTitle,
          body: l.doctorHealthyBody,
        ),
      );
    } else {
      int count(String s) => report.diagnoses.where((d) => d.severity == s).length;
      body = ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Wrap(
                spacing: Space.x2,
                children: [
                  if (count('critical') > 0)
                    ColorBadge(
                      l.doctorCritical(count('critical')),
                      color: c.danger,
                      icon: Symbols.error_rounded,
                    ),
                  if (count('warning') > 0)
                    ColorBadge(
                      l.doctorWarnings(count('warning')),
                      color: c.warning,
                      icon: Symbols.warning_rounded,
                    ),
                  if (count('info') > 0)
                    ColorBadge(
                      l.doctorInfo(count('info')),
                      color: c.info,
                      icon: Symbols.info_rounded,
                    ),
                ],
              ),
              if (report.crashExcerpt != null) ...[
                const SizedBox(height: Space.x4),
                _CrashExcerpt(
                  text: report.crashExcerpt!,
                  path: report.crashReportPath,
                ),
              ],
              const SizedBox(height: Space.x4),
              for (final d in report.diagnoses) ...[
                _DiagnosisCard(
                  diagnosis: d,
                  applying: _applying == d.id,
                  onApply: _applying == null ? () => _apply(d) : null,
                ),
                const SizedBox(height: Space.x3),
              ],
            ],
          ),
        ),
      );
    }

    return AppDialog(
      title: l.doctorTitle(widget.instance.name),
      icon: Symbols.stethoscope_rounded,
      width: 720,
      actions: [
        TextButton.icon(
          onPressed: async.isLoading
              ? null
              : () => ref.invalidate(doctorReportProvider(widget.instance.id)),
          icon: const Icon(Symbols.refresh_rounded, size: 18),
          label: Text(l.doctorRerun),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.close),
        ),
      ],
      child: body,
    );
  }
}

class _CrashExcerpt extends StatelessWidget {
  const _CrashExcerpt({required this.text, required this.path});

  final String text;
  final String? path;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Container(
      padding: const EdgeInsets.all(Space.x3),
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(Radii.control),
        border: Border.all(color: c.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Symbols.description_rounded, size: 16, color: c.textMuted),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Text(l.doctorCrashExcerpt, style: context.text.labelMedium),
              ),
              if (path != null)
                TextButton(
                  style: TextButton.styleFrom(
                    minimumSize: const Size(0, 28),
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                  ),
                  onPressed: () => launchUrl(Uri.file(path!)),
                  child: Text(l.doctorOpenReport),
                ),
            ],
          ),
          const SizedBox(height: Space.x2),
          SelectableText(
            text,
            minLines: 1,
            maxLines: 6,
            style: TextStyle(
              fontFamily: kMonoFamily,
              fontSize: 12,
              height: 1.45,
              color: c.textSecondary,
            ),
          ),
        ],
      ),
    );
  }
}

class _DiagnosisCard extends StatelessWidget {
  const _DiagnosisCard({
    required this.diagnosis,
    required this.applying,
    required this.onApply,
  });

  final DiagnosisView diagnosis;
  final bool applying;
  final VoidCallback? onApply;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final d = diagnosis;
    final color = severityColor(c, d.severity);
    return Container(
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: c.surface1,
        borderRadius: BorderRadius.circular(Radii.card),
        border: Border.all(color: c.border),
      ),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Container(width: 4, color: color),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(Space.x4),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Container(
                      width: 36,
                      height: 36,
                      decoration: BoxDecoration(
                        color: color.withValues(alpha: 0.14),
                        borderRadius: BorderRadius.circular(Radii.control),
                      ),
                      child: Icon(categoryIcon(d.category), size: 20, color: color),
                    ),
                    const SizedBox(width: Space.x3),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Expanded(
                                child: Text(d.title, style: context.text.titleSmall),
                              ),
                              const SizedBox(width: Space.x2),
                              Text(
                                l.doctorConfidence(d.confidence),
                                style: context.text.labelSmall!.copyWith(
                                  color: c.textMuted,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: Space.x1),
                          Text(
                            d.explanation,
                            style: context.text.bodySmall!.copyWith(
                              color: c.textSecondary,
                              height: 1.45,
                            ),
                          ),
                          if (d.culprit != null) ...[
                            const SizedBox(height: Space.x2),
                            Row(
                              children: [
                                Text(
                                  l.doctorCulprit,
                                  style: context.text.labelSmall!.copyWith(
                                    color: c.textMuted,
                                  ),
                                ),
                                const SizedBox(width: Space.x2),
                                Flexible(
                                  child: Container(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 8,
                                      vertical: 3,
                                    ),
                                    decoration: BoxDecoration(
                                      color: Colors.black.withValues(alpha: 0.3),
                                      borderRadius: BorderRadius.circular(6),
                                    ),
                                    child: Text(
                                      d.culprit!,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                      style: TextStyle(
                                        fontFamily: kMonoFamily,
                                        fontSize: 11.5,
                                        color: c.text,
                                      ),
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ],
                          if (d.fix != null) ...[
                            const SizedBox(height: Space.x3),
                            Container(
                              padding: const EdgeInsets.fromLTRB(
                                Space.x3,
                                Space.x2,
                                Space.x2,
                                Space.x2,
                              ),
                              decoration: BoxDecoration(
                                color: c.accentContainer.withValues(alpha: 0.35),
                                borderRadius: BorderRadius.circular(Radii.control),
                                border: Border.all(
                                  color: c.accent.withValues(alpha: 0.25),
                                ),
                              ),
                              child: Row(
                                children: [
                                  Icon(Symbols.auto_fix_high_rounded,
                                      size: 18, color: c.accent),
                                  const SizedBox(width: Space.x2),
                                  Expanded(
                                    child: Text(
                                      d.fix!,
                                      style: context.text.bodySmall!.copyWith(
                                        color: c.text,
                                      ),
                                    ),
                                  ),
                                  const SizedBox(width: Space.x2),
                                  FilledButton.tonalIcon(
                                    onPressed: onApply,
                                    icon: applying
                                        ? const SizedBox.square(
                                            dimension: 14,
                                            child: CircularProgressIndicator(
                                              strokeWidth: 2,
                                            ),
                                          )
                                        : const Icon(Symbols.build_rounded, size: 16),
                                    label: Text(l.doctorApplyFix),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
