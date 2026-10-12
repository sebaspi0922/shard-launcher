// Aviso flotante cuando el juego se cierra mal: «Diagnosticar» abre Doctor.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/crash_notice.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import 'doctor_dialog.dart';

class CrashBanner extends ConsumerWidget {
  const CrashBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final notice = ref.watch(crashNoticeProvider);
    final instance =
        notice == null ? null : ref.watch(instanceByIdProvider(notice.instanceId));
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final visible = notice != null;

    return AnimatedSlide(
      duration: Motion.medium,
      curve: Motion.curve,
      offset: visible ? Offset.zero : const Offset(0, 1.4),
      child: AnimatedOpacity(
        duration: Motion.fast,
        opacity: visible ? 1 : 0,
        child: !visible
            ? const SizedBox.shrink()
            : Container(
                width: 400,
                padding: const EdgeInsets.fromLTRB(Space.x4, Space.x4, Space.x2, Space.x3),
                decoration: BoxDecoration(
                  color: Color.alphaBlend(
                    c.danger.withValues(alpha: 0.10),
                    Color.alphaBlend(c.surface3, c.background),
                  ),
                  borderRadius: BorderRadius.circular(Radii.card),
                  border: Border.all(color: c.danger.withValues(alpha: 0.45)),
                  boxShadow: [
                    BoxShadow(
                      color: Colors.black.withValues(alpha: 0.45),
                      blurRadius: 28,
                      offset: const Offset(0, 10),
                    ),
                  ],
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Container(
                      width: 36,
                      height: 36,
                      decoration: BoxDecoration(
                        color: c.danger.withValues(alpha: 0.16),
                        borderRadius: BorderRadius.circular(Radii.control),
                      ),
                      child: Icon(Symbols.heart_broken_rounded, size: 20, color: c.danger),
                    ),
                    const SizedBox(width: Space.x3),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(l.crashBannerTitle, style: context.text.titleSmall),
                          const SizedBox(height: 2),
                          Text(
                            [
                              instance?.name ?? notice.instanceId,
                              if (notice.code != null) l.crashBannerCode(notice.code!),
                            ].join(' · '),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: context.text.bodySmall,
                          ),
                          const SizedBox(height: Space.x3),
                          Row(
                            children: [
                              FilledButton.icon(
                                style: FilledButton.styleFrom(
                                  minimumSize: const Size(0, 34),
                                  padding: const EdgeInsets.symmetric(horizontal: 14),
                                ),
                                onPressed: instance == null
                                    ? null
                                    : () {
                                        ref.read(crashNoticeProvider.notifier).dismiss();
                                        showDoctor(context, instance);
                                      },
                                icon: const Icon(Symbols.stethoscope_rounded, size: 18),
                                label: Text(l.doctorDiagnose),
                              ),
                              const SizedBox(width: Space.x2),
                              TextButton(
                                style: TextButton.styleFrom(
                                  minimumSize: const Size(0, 34),
                                ),
                                onPressed: () {
                                  ref.read(crashNoticeProvider.notifier).dismiss();
                                  context.go('/instances/${notice.instanceId}?tab=logs');
                                },
                                child: Text(l.launchSeeLogs),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                    IconButton(
                      tooltip: l.close,
                      visualDensity: VisualDensity.compact,
                      iconSize: 18,
                      onPressed: () => ref.read(crashNoticeProvider.notifier).dismiss(),
                      icon: const Icon(Symbols.close_rounded),
                    ),
                  ],
                ),
              ),
      ),
    );
  }
}
