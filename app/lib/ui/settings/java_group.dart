// Ajustes › Java: instalaciones detectadas e instalar una versión.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../core/tasks.dart';
import '../../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/common.dart';
import '../widgets/media.dart';
import 'settings_page.dart';

/// Versiones mayores que se ofrecen para instalar (las que pide Minecraft).
const kInstallableJava = [8, 17, 21, 25];

final javaListProvider = FutureProvider.autoDispose<List<JavaView>>(
  (ref) => ref.watch(bridgeProvider).listJava(),
);

/// Versión mayor que se está instalando, si hay alguna.
class JavaInstallController extends Notifier<int?> {
  @override
  int? build() => null;

  Future<JavaView> install(int major) async {
    state = major;
    try {
      final j = await ref.read(bridgeProvider).installJava(major);
      ref.invalidate(javaListProvider);
      return j;
    } finally {
      state = null;
    }
  }
}

final javaInstallProvider =
    NotifierProvider<JavaInstallController, int?>(JavaInstallController.new);

class JavaGroup extends ConsumerWidget {
  const JavaGroup({super.key});

  Future<void> _install(BuildContext context, WidgetRef ref, int major) async {
    final l = AppLocalizations.of(context);
    try {
      final j = await ref.read(javaInstallProvider.notifier).install(major);
      if (context.mounted) showToast(context, l.javaInstalled(j.major, j.version));
    } catch (e) {
      if (context.mounted) showToast(context, errorText(l, e), error: true);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final async = ref.watch(javaListProvider);
    final installing = ref.watch(javaInstallProvider);
    final progress = ref.watch(stageProgressProvider('java'));
    final list = async.value ?? const <JavaView>[];
    final managed = {for (final j in list) if (j.source == 'managed') j.major};

    return SettingsGroup(
      title: l.settingsJava,
      icon: Symbols.coffee_rounded,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: Space.x3, bottom: Space.x2),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(l.javaDetected, style: context.text.titleSmall),
                    const SizedBox(height: 2),
                    Text(l.javaDetectedDesc, style: context.text.bodySmall),
                  ],
                ),
              ),
              IconButton(
                tooltip: l.javaRescan,
                onPressed: async.isLoading ? null : () => ref.invalidate(javaListProvider),
                icon: const Icon(Symbols.refresh_rounded, size: 18),
              ),
            ],
          ),
        ),
        switch (async) {
          AsyncError(:final error) => Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.x2),
              child: ErrorNote(error),
            ),
          AsyncData() when list.isEmpty => Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.x3),
              child: Text(l.javaNone, style: context.text.bodySmall),
            ),
          AsyncData() => Column(
              children: [
                for (final j in list) _JavaRow(java: j),
              ],
            ),
          _ => const Padding(
              padding: EdgeInsets.symmetric(vertical: Space.x4),
              child: LinearProgressIndicator(minHeight: 2),
            ),
        },
        const Divider(),
        SettingRow(
          title: l.javaInstall,
          description: installing == null
              ? l.javaInstallDesc
              : progress?.total == null || progress!.total == 0
                  ? l.javaInstalling(installing)
                  : l.javaInstallingProgress(
                      installing,
                      formatBytes(l, BigInt.from(progress.done)),
                      formatBytes(l, BigInt.from(progress.total)),
                    ),
          control: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              for (final major in kInstallableJava) ...[
                const SizedBox(width: Space.x2),
                _InstallButton(
                  major: major,
                  installed: managed.contains(major),
                  installing: installing == major,
                  fraction: installing == major ? progress?.fraction : null,
                  onPressed: installing != null
                      ? null
                      : () => _install(context, ref, major),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

class _JavaRow extends StatelessWidget {
  const _JavaRow({required this.java});

  final JavaView java;

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final j = java;
    final (label, color) = switch (j.source) {
      'managed' => (l.javaManaged, c.accent),
      'override' => (l.javaOverride, c.warning),
      _ => (l.javaSystem, c.info),
    };
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          Container(
            width: 44,
            height: 36,
            alignment: Alignment.center,
            decoration: BoxDecoration(
              color: c.surface2,
              borderRadius: BorderRadius.circular(Radii.control),
              border: Border.all(color: c.border),
            ),
            child: Text('${j.major}', style: context.text.titleMedium),
          ),
          const SizedBox(width: Space.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      j.vendor == null ? j.version : '${j.version} · ${j.vendor}',
                      style: context.text.bodyMedium,
                    ),
                    const SizedBox(width: Space.x2),
                    ColorBadge(label, color: color),
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  j.path,
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
        ],
      ),
    );
  }
}

class _InstallButton extends StatelessWidget {
  const _InstallButton({
    required this.major,
    required this.installed,
    required this.installing,
    required this.fraction,
    required this.onPressed,
  });

  final int major;
  final bool installed;
  final bool installing;
  final double? fraction;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    final label = Text('Java $major');
    if (installing) {
      return OutlinedButton.icon(
        onPressed: null,
        icon: SizedBox.square(
          dimension: 14,
          child: CircularProgressIndicator(strokeWidth: 2, value: fraction),
        ),
        label: label,
      );
    }
    if (installed) {
      return OutlinedButton.icon(
        onPressed: onPressed,
        icon: Icon(Symbols.check_rounded, size: 16, color: c.success),
        label: label,
      );
    }
    return OutlinedButton.icon(
      onPressed: onPressed,
      icon: const Icon(Symbols.download_rounded, size: 16),
      label: label,
    );
  }
}
