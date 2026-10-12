// Diálogo para guardar la clave de API de CurseForge. La clave va al keyring
// del sistema y nunca se vuelve a mostrar.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/errors.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../widgets/common.dart';

/// Pide y guarda la clave. `true` si se guardó.
Future<bool> showCurseforgeKeyDialog(BuildContext context, WidgetRef ref) async {
  final l = AppLocalizations.of(context);
  final key = await showTextPrompt(
    context,
    title: l.cfKeyTitle,
    label: l.cfKeyLabel,
    helper: l.cfKeyHelper,
    action: l.save,
    obscure: true,
    icon: Symbols.key_rounded,
  );
  if (key == null || key.isEmpty || !context.mounted) return false;
  try {
    await ref.read(curseforgeKeyProvider.notifier).set(key);
    if (context.mounted) showToast(context, l.cfKeySaved);
    return true;
  } catch (e) {
    if (context.mounted) showToast(context, errorText(l, e), error: true);
    return false;
  }
}
