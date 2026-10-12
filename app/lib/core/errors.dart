// Traducción de errores del núcleo por su `kind` estable.

import '../l10n/app_localizations.dart';
import 'bridge.dart';

/// Mensaje legible para el usuario.
String errorText(AppLocalizations l, Object error) {
  if (error is! BridgeError) return l.errGeneric;
  return switch (error.kind) {
    'auth_missing_client_id' => l.errAuthMissingClientId,
    'auth_app_not_approved' => l.errAuthAppNotApproved,
    'auth_not_entitled' => l.errAuthNotEntitled,
    'auth_no_profile' => l.errAuthNoProfile,
    'auth_xbox' => l.errAuthXbox,
    'auth_declined' => l.errAuthDeclined,
    'auth_device_code_expired' => l.errAuthExpired,
    'auth_session_expired' || 'auth_refresh_token_missing' =>
      l.errAuthSessionExpired,
    'auth_invalid_offline_name' => l.offlineNameInvalid,
    'auth_msa' || 'auth_unexpected' || 'auth_offline_account' =>
      l.errAuthGeneric,
    'network' || 'http_status' => l.errNetwork,
    'offline' => l.errOffline,
    'java_required' => l.errJavaRequired,
    'game_already_running' => l.errGameAlreadyRunning,
    'instance_not_found' => l.errInstanceNotFound,
    'account_not_found' => l.errAccountNotFound,
    'version_not_found' => l.errVersionNotFound,
    'hash_mismatch' => l.errHashMismatch,
    'io' || 'invalid_path' => l.errIo,
    'invalid_input' => l.errInvalidInput,
    _ => l.errGeneric,
  };
}

/// Detalle técnico (mensaje del núcleo) para mostrar aparte, si aporta algo.
String? errorDetail(Object error) {
  if (error is BridgeError) {
    return error.message.isEmpty ? null : '${error.kind}: ${error.message}';
  }
  return error.toString();
}

/// Errores esperables en los que el detalle técnico no aporta.
bool isCancelled(Object error) =>
    error is BridgeError && error.kind == 'cancelled';
