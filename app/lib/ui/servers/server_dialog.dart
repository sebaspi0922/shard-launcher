// Añadir o editar un servidor: nombre, dirección, instancia y cuenta.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_symbols_icons/symbols.dart';
import 'package:material_ui/material_ui.dart';

import '../../core/bridge.dart';
import '../../core/format.dart';
import '../../core/providers.dart';
import '../../core/servers_state.dart';
import '../../l10n/app_localizations.dart';
import '../theme/tokens.dart';
import '../widgets/art.dart';
import '../widgets/avatar.dart';
import '../widgets/common.dart';
import '../widgets/select_button.dart';

Future<void> showServerDialog(BuildContext context, {ServerView? server}) =>
    showDialog<void>(
      context: context,
      builder: (_) => ServerDialog(server: server),
    );

/// `host`, `host:puerto` o un nombre con puntos y guiones.
final _addressRe = RegExp(r'^[A-Za-z0-9.\-_\[\]:]+$');

class ServerDialog extends ConsumerStatefulWidget {
  const ServerDialog({super.key, this.server});

  final ServerView? server;

  @override
  ConsumerState<ServerDialog> createState() => _ServerDialogState();
}

class _ServerDialogState extends ConsumerState<ServerDialog> {
  late final _name = TextEditingController(text: widget.server?.name ?? '');
  late final _address = TextEditingController(text: widget.server?.address ?? '');
  late String? _instance =
      widget.server?.instanceId ?? ref.read(homeInstanceProvider)?.id;
  late String? _account = widget.server?.accountId;
  bool _busy = false;
  bool _touched = false;
  Object? _error;

  @override
  void dispose() {
    _name.dispose();
    _address.dispose();
    super.dispose();
  }

  bool get _addressOk {
    final a = _address.text.trim();
    return a.isNotEmpty && _addressRe.hasMatch(a);
  }

  Future<void> _save() async {
    setState(() => _touched = true);
    if (!_addressOk) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    final ctrl = ref.read(serversProvider.notifier);
    try {
      final s = widget.server;
      if (s == null) {
        await ctrl.add(
          name: _name.text.trim(),
          address: _address.text.trim(),
          instanceId: _instance,
          accountId: _account,
        );
      } else {
        await ctrl.edit(
          id: s.id,
          name: _name.text.trim(),
          address: _address.text.trim(),
          instanceId: _instance,
          accountId: _account,
        );
      }
      ref.invalidate(serverStatusProvider);
      if (mounted) Navigator.pop(context);
    } catch (e) {
      if (mounted) {
        setState(() {
          _busy = false;
          _error = e;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final instances = ref.watch(recentInstancesProvider);
    final accounts = ref.watch(accountsProvider).value ?? const <AccountView>[];
    final editing = widget.server != null;
    return AppDialog(
      title: editing ? l.serverEdit : l.serverAdd,
      icon: Symbols.dns_rounded,
      width: 480,
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : _save,
          child: Text(editing ? l.save : l.serverAdd),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _address,
            autofocus: !editing,
            onChanged: (_) => setState(() {}),
            onSubmitted: (_) => _save(),
            decoration: InputDecoration(
              labelText: l.serverAddress,
              hintText: 'play.example.net:25565',
              prefixIcon: const Icon(Symbols.language_rounded, size: 18),
              errorText: _touched && !_addressOk ? l.serverAddressInvalid : null,
            ),
          ),
          const SizedBox(height: Space.x3),
          TextField(
            controller: _name,
            onSubmitted: (_) => _save(),
            decoration: InputDecoration(
              labelText: l.serverName,
              hintText: l.serverNameHint,
            ),
          ),
          const SizedBox(height: Space.x4),
          Text(l.serverInstance, style: context.text.labelLarge),
          const SizedBox(height: Space.x2),
          SelectButton<String?>(
            width: double.infinity,
            value: _instance,
            options: [
              SelectOption(null, l.serverInstanceNone),
              for (final i in instances)
                SelectOption(
                  i.id,
                  i.name,
                  subtitle: versionLine(l, i.mc, i.loader),
                  leading: InstanceIcon(id: i.id, name: i.name, size: 24),
                ),
            ],
            onChanged: _busy ? null : (v) => setState(() => _instance = v),
          ),
          const SizedBox(height: Space.x3),
          Text(l.serverAccount, style: context.text.labelLarge),
          const SizedBox(height: Space.x2),
          SelectButton<String?>(
            width: double.infinity,
            value: _account,
            options: [
              SelectOption(null, l.serverAccountActive),
              for (final a in accounts)
                SelectOption(
                  a.id,
                  a.name,
                  subtitle: a.shard
                      ? l.accountShard
                      : a.microsoft
                      ? l.accountMicrosoft
                      : l.accountOffline,
                  leading: PlayerAvatar(account: a, size: 22),
                ),
            ],
            onChanged: _busy ? null : (v) => setState(() => _account = v),
          ),
          if (_error != null) ...[
            const SizedBox(height: Space.x3),
            ErrorNote(_error!),
          ],
        ],
      ),
    );
  }
}
