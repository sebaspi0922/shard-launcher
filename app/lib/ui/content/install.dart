// Instalación de un proyecto en una instancia con aviso del resultado.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/errors.dart';
import '../../core/instance_data.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../widgets/common.dart';

/// Instala `project` (y sus dependencias) y avisa. `true` si fue bien.
Future<bool> installAndReport(
  BuildContext context,
  WidgetRef ref, {
  required String instanceId,
  required ProjectView project,
  String? version,
  String? world,
}) async {
  final l = AppLocalizations.of(context);
  final instance = ref.read(instanceByIdProvider(instanceId));
  try {
    final out = await ref.read(bridgeProvider).installContent(
          instanceId,
          project.source,
          project.id,
          version: version,
          world: world,
        );
    ref
      ..invalidate(instanceContentProvider(instanceId))
      ..invalidate(contentUpdatesProvider(instanceId));
    if (world != null) {
      ref.invalidate(datapacksProvider((instance: instanceId, world: world)));
    }
    if (!context.mounted) return true;
    final deps = out.installed.where((i) => i.dependency && !i.alreadyPresent).length;
    final name = instance?.name ?? instanceId;
    if (out.skipped.isNotEmpty) {
      showToast(
        context,
        l.installPartial(project.title, out.skipped.length),
        error: true,
      );
    } else if (deps > 0) {
      showToast(context, l.installedWithDeps(project.title, name, deps));
    } else {
      showToast(context, l.installedIn(project.title, name));
    }
    return true;
  } catch (e) {
    if (context.mounted) showToast(context, errorText(l, e), error: true);
    return false;
  }
}
