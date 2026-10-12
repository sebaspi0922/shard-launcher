// Panel de cuentas que se abre desde el avatar del raíl.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/app_info.dart';
import '../../core/bridge.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/avatar.dart';
import '../widgets/common.dart';
import 'account_dialogs.dart';

/// Abre el panel anclado abajo a la izquierda, junto al raíl.
Future<void> showAccountPanel(BuildContext context) => showGeneralDialog(
      context: context,
      barrierDismissible: true,
      barrierLabel: AppLocalizations.of(context).accountsTitle,
      barrierColor: Colors.black.withValues(alpha: 0.25),
      transitionDuration: Motion.medium,
      pageBuilder: (_, _, _) => const Align(
        alignment: Alignment.bottomLeft,
        child: Padding(
          padding: EdgeInsets.only(left: 84, bottom: Space.x3),
          child: AccountPanel(),
        ),
      ),
      transitionBuilder: (_, anim, _, child) {
        final a = CurvedAnimation(parent: anim, curve: Motion.curve);
        return FadeTransition(
          opacity: a,
          child: SlideTransition(
            position: Tween(begin: const Offset(-0.04, 0.02), end: Offset.zero)
                .animate(a),
            child: child,
          ),
        );
      },
    );

class AccountPanel extends ConsumerWidget {
  const AccountPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final accounts = ref.watch(accountsProvider);
    final list = accounts.value ?? const <AccountView>[];
    final active = ref.watch(activeAccountProvider);

    return Material(
      color: Color.alphaBlend(c.surface3, c.background),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(Radii.panel),
        side: BorderSide(color: c.borderStrong),
      ),
      elevation: 12,
      shadowColor: Colors.black,
      child: SizedBox(
        width: 340,
        child: Padding(
          padding: const EdgeInsets.all(Space.x3),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(
                  Space.x2,
                  Space.x1,
                  Space.x2,
                  Space.x3,
                ),
                child: Text(l.accountsTitle, style: context.text.titleMedium),
              ),
              if (accounts.isLoading && list.isEmpty)
                const Padding(
                  padding: EdgeInsets.all(Space.x5),
                  child: Center(child: CircularProgressIndicator()),
                )
              else if (list.isEmpty)
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: Space.x2,
                    vertical: Space.x3,
                  ),
                  child: Text(
                    l.accountsEmpty,
                    style: context.text.bodyMedium!.copyWith(
                      color: c.textSecondary,
                    ),
                  ),
                )
              else
                for (final a in list)
                  _AccountTile(account: a, active: a.id == active?.id),
              const SizedBox(height: Space.x2),
              const Divider(),
              const SizedBox(height: Space.x2),
              _ActionTile(
                icon: Symbols.add_circle_rounded,
                label: l.accountsAddMicrosoft,
                onTap: () {
                  final nav = Navigator.of(context)..pop();
                  showMicrosoftLogin(nav.context);
                },
              ),
              _ActionTile(
                icon: Symbols.person_add_rounded,
                label: l.accountsAddOffline,
                onTap: () {
                  final nav = Navigator.of(context)..pop();
                  showOfflineAccount(nav.context);
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AccountTile extends ConsumerWidget {
  const _AccountTile({required this.account, required this.active});

  final AccountView account;
  final bool active;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    return Padding(
      padding: const EdgeInsets.only(bottom: 2),
      child: Material(
        color: active ? c.accentContainer.withValues(alpha: 0.55) : Colors.transparent,
        borderRadius: BorderRadius.circular(Radii.control + 2),
        child: InkWell(
          borderRadius: BorderRadius.circular(Radii.control + 2),
          onTap: active
              ? null
              : () async {
                  try {
                    await ref.read(accountsProvider.notifier).setActive(account.id);
                  } catch (e) {
                    if (context.mounted) {
                      showToast(context, l.errGeneric, error: true);
                    }
                  }
                },
          child: Padding(
            padding: const EdgeInsets.all(Space.x2),
            child: Row(
              children: [
                PlayerAvatar(account: account, size: 36),
                const SizedBox(width: Space.x3),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        account.name,
                        style: context.text.titleSmall,
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: 2),
                      Row(
                        children: [
                          Icon(
                            account.microsoft
                                ? Symbols.verified_rounded
                                : Symbols.wifi_off_rounded,
                            size: 13,
                            color: account.microsoft ? c.info : c.textMuted,
                          ),
                          const SizedBox(width: 4),
                          Text(
                            account.microsoft ? l.accountMicrosoft : l.accountOffline,
                            style: context.text.bodySmall,
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                if (active)
                  Padding(
                    padding: const EdgeInsets.only(right: Space.x1),
                    child: Icon(
                      Symbols.check_circle_rounded,
                      fill: 1,
                      color: c.accent,
                      size: 20,
                    ),
                  ),
                IconButton(
                  tooltip: l.remove,
                  iconSize: 18,
                  visualDensity: VisualDensity.compact,
                  icon: const Icon(Symbols.delete_rounded),
                  onPressed: () => _remove(context, ref),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Future<void> _remove(BuildContext context, WidgetRef ref) async {
    final l = AppLocalizations.of(context);
    final ok = await confirm(
      context,
      title: l.accountRemoveTitle(account.name),
      body: l.accountRemoveBody(kAppName),
      action: l.remove,
      danger: true,
    );
    if (!ok) return;
    try {
      await ref.read(accountsProvider.notifier).remove(account.id);
      if (context.mounted) showToast(context, l.accountRemoved(account.name));
    } catch (e) {
      if (context.mounted) showToast(context, l.errGeneric, error: true);
    }
  }
}

class _ActionTile extends StatelessWidget {
  const _ActionTile({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.colors;
    return InkWell(
      borderRadius: BorderRadius.circular(Radii.control),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.x2, vertical: 10),
        child: Row(
          children: [
            Icon(icon, size: 20, color: c.accent),
            const SizedBox(width: Space.x3),
            Text(label, style: context.text.bodyMedium),
          ],
        ),
      ),
    );
  }
}
