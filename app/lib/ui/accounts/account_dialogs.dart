// Alta de cuentas: Microsoft (código de dispositivo) y offline.

import 'dart:async';

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

Future<void> showMicrosoftLogin(BuildContext context) async {
  final account = await showDialog<AccountView>(
    context: context,
    barrierDismissible: false,
    builder: (_) => const MicrosoftLoginDialog(),
  );
  if (account != null && context.mounted) {
    showToast(context, AppLocalizations.of(context).msSignedIn(account.name));
  }
}

Future<void> showOfflineAccount(BuildContext context) async {
  final account = await showDialog<AccountView>(
    context: context,
    builder: (_) => const OfflineAccountDialog(),
  );
  if (account != null && context.mounted) {
    showToast(context, AppLocalizations.of(context).accountAdded(account.name));
  }
}

class MicrosoftLoginDialog extends ConsumerStatefulWidget {
  const MicrosoftLoginDialog({super.key});

  @override
  ConsumerState<MicrosoftLoginDialog> createState() =>
      _MicrosoftLoginDialogState();
}

class _MicrosoftLoginDialogState extends ConsumerState<MicrosoftLoginDialog> {
  late final LauncherBridge _bridge = ref.read(bridgeProvider);
  DeviceCodeView? _code;
  DateTime? _expiresAt;
  Duration _left = Duration.zero;
  Object? _error;
  bool _expired = false;
  bool _finished = false;
  Timer? _tick;

  /// Intento actual; los resultados de intentos viejos se ignoran.
  int _attempt = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_begin());
  }

  Future<void> _begin() async {
    final attempt = ++_attempt;
    setState(() {
      _code = null;
      _error = null;
      _expired = false;
    });
    try {
      final code = await _bridge.beginMicrosoftLogin();
      if (!mounted || attempt != _attempt) return;
      setState(() {
        _code = code;
        _expiresAt = DateTime.now().add(Duration(seconds: code.expiresIn.toInt()));
        _left = Duration(seconds: code.expiresIn.toInt());
      });
      _tick?.cancel();
      _tick = Timer.periodic(const Duration(seconds: 1), (_) => _onTick());
      final account = await _bridge.completeMicrosoftLogin();
      if (!mounted || attempt != _attempt) return;
      _finished = true;
      _tick?.cancel();
      await ref.read(accountsProvider.notifier).refresh();
      if (mounted) Navigator.of(context).pop(account);
    } catch (e) {
      if (!mounted || attempt != _attempt || isCancelled(e)) return;
      _tick?.cancel();
      setState(() {
        if (e is BridgeError && e.kind == 'auth_device_code_expired') {
          _expired = true;
        } else {
          _error = e;
        }
      });
    }
  }

  void _onTick() {
    final at = _expiresAt;
    if (at == null || !mounted) return;
    final left = at.difference(DateTime.now());
    setState(() {
      _left = left.isNegative ? Duration.zero : left;
      if (left.isNegative) {
        _expired = true;
        _tick?.cancel();
      }
    });
  }

  void _cancel() {
    _attempt++;
    _bridge.cancelMicrosoftLogin();
    Navigator.of(context).pop();
  }

  @override
  void dispose() {
    _tick?.cancel();
    if (!_finished) _bridge.cancelMicrosoftLogin();
    super.dispose();
  }

  String _mmss(Duration d) {
    final m = d.inMinutes;
    final s = (d.inSeconds % 60).toString().padLeft(2, '0');
    return '$m:$s';
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final c = context.colors;
    final code = _code;

    Widget body;
    if (_error != null) {
      body = ErrorNote(_error!);
    } else if (_expired) {
      body = Row(
        children: [
          Icon(Symbols.timer_off_rounded, color: c.warning, size: 20),
          const SizedBox(width: Space.x2),
          Expanded(child: Text(l.msExpired)),
        ],
      );
    } else if (code == null) {
      body = Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.x6),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const SizedBox.square(
              dimension: 18,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            const SizedBox(width: Space.x3),
            Text(l.msRequesting),
          ],
        ),
      );
    } else {
      final total = code.expiresIn.toInt();
      body = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(l.msStep),
          const SizedBox(height: Space.x4),
          Container(
            padding: const EdgeInsets.symmetric(vertical: Space.x5),
            decoration: BoxDecoration(
              color: c.backgroundDeep.withValues(alpha: 0.6),
              borderRadius: BorderRadius.circular(Radii.card),
              border: Border.all(color: c.accent.withValues(alpha: 0.35)),
            ),
            child: Column(
              children: [
                SelectableText(
                  code.userCode,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                    fontFamily: kMonoFamily,
                    fontSize: 34,
                    fontWeight: FontWeight.w500,
                    letterSpacing: 6,
                    color: c.text,
                  ),
                ),
                const SizedBox(height: Space.x1),
                Text(
                  code.verificationUri.replaceFirst(RegExp(r'^https?://'), ''),
                  style: context.text.bodySmall,
                ),
              ],
            ),
          ),
          const SizedBox(height: Space.x4),
          Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  icon: const Icon(Symbols.content_copy_rounded, size: 18),
                  label: Text(l.msCopyCode),
                  onPressed: () => copyText(context, code.userCode, l.copied),
                ),
              ),
              const SizedBox(width: Space.x2),
              Expanded(
                child: FilledButton.icon(
                  icon: const Icon(Symbols.open_in_new_rounded, size: 18),
                  label: Text(l.msOpenBrowser),
                  onPressed: () => launchUrl(
                    Uri.parse(code.verificationUri),
                    mode: LaunchMode.externalApplication,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.x5),
          Row(
            children: [
              const SizedBox.square(
                dimension: 14,
                child: CircularProgressIndicator(strokeWidth: 1.8),
              ),
              const SizedBox(width: Space.x2),
              Expanded(
                child: Text(l.msWaiting, style: context.text.bodySmall),
              ),
              Text(
                l.msExpiresIn(_mmss(_left)),
                style: context.text.bodySmall!.copyWith(
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
            ],
          ),
          const SizedBox(height: Space.x2),
          ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: LinearProgressIndicator(
              value: total == 0 ? 0 : _left.inSeconds / total,
              minHeight: 3,
            ),
          ),
        ],
      );
    }

    return AppDialog(
      title: l.msTitle,
      icon: Symbols.account_circle_rounded,
      width: 460,
      actions: [
        TextButton(onPressed: _cancel, child: Text(l.cancel)),
        if (_error != null || _expired)
          FilledButton(
            onPressed: _begin,
            child: Text(_expired ? l.msRequestNew : l.retry),
          ),
      ],
      child: body,
    );
  }
}

final _offlineName = RegExp(r'^[A-Za-z0-9_]{3,16}$');

class OfflineAccountDialog extends ConsumerStatefulWidget {
  const OfflineAccountDialog({super.key});

  @override
  ConsumerState<OfflineAccountDialog> createState() =>
      _OfflineAccountDialogState();
}

class _OfflineAccountDialogState extends ConsumerState<OfflineAccountDialog> {
  final _name = TextEditingController();
  bool _busy = false;
  bool _touched = false;
  Object? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  bool get _valid => _offlineName.hasMatch(_name.text.trim());

  Future<void> _submit() async {
    setState(() => _touched = true);
    if (!_valid || _busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final a = await ref
          .read(accountsProvider.notifier)
          .addOffline(_name.text.trim());
      if (mounted) Navigator.of(context).pop(a);
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
    return AppDialog(
      title: l.offlineTitle,
      icon: Symbols.person_rounded,
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l.cancel),
        ),
        FilledButton(
          onPressed: _busy ? null : _submit,
          child: Text(l.add),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.offlineBody),
          const SizedBox(height: Space.x4),
          TextField(
            controller: _name,
            autofocus: true,
            maxLength: 16,
            onChanged: (_) => setState(() {}),
            onSubmitted: (_) => _submit(),
            decoration: InputDecoration(
              labelText: l.offlineNameLabel,
              helperText: l.offlineNameHint,
              counterText: '',
              errorText: _touched && !_valid ? l.offlineNameInvalid : null,
              prefixIcon: const Icon(Symbols.badge_rounded, size: 18),
            ),
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
