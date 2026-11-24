//! Texto de un error del núcleo para la UI (app/lib/core/errors.dart): el
//! tipo (`Error::kind`) decide la clave del ARB; el mensaje va de detalle.

/// Clave del ARB para un `kind` de `lcore::Error`.
pub fn key(kind: &str) -> &'static str {
    match kind {
        "auth_missing_client_id" => "errAuthMissingClientId",
        "auth_app_not_approved" => "errAuthAppNotApproved",
        "auth_not_entitled" => "errAuthNotEntitled",
        "auth_no_profile" => "errAuthNoProfile",
        "auth_xbox" => "errAuthXbox",
        "auth_declined" => "errAuthDeclined",
        "auth_device_code_expired" => "errAuthExpired",
        "auth_session_expired" | "auth_refresh_token_missing" => "errAuthSessionExpired",
        "auth_invalid_offline_name" => "offlineNameInvalid",
        "auth_msa" | "auth_unexpected" | "auth_offline_account" => "errAuthGeneric",
        "network" | "http_status" => "errNetwork",
        "offline" => "errOffline",
        "java_required" => "errJavaRequired",
        "game_already_running" => "errGameAlreadyRunning",
        "instance_not_found" => "errInstanceNotFound",
        "account_not_found" => "errAccountNotFound",
        "version_not_found" => "errVersionNotFound",
        "hash_mismatch" => "errHashMismatch",
        "io" | "invalid_path" => "errIo",
        "invalid_input" => "errInvalidInput",
        "curseforge_key_missing" => "errCurseforgeKey",
        "secrets" => "errSecrets",
        "update_rejected" => "errUpdateRejected",
        _ => "errGeneric",
    }
}

/// (texto, detalle) de un error del núcleo.
pub fn describe(lang: &str, e: &lcore::Error) -> (String, String) {
    (crate::i18n::text(lang, key(e.kind())), format!("{}: {e}", e.kind()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn todas_las_claves_existen_en_los_arb() {
        let kinds = [
            "auth_missing_client_id", "auth_app_not_approved", "auth_not_entitled", "auth_no_profile", "auth_xbox",
            "auth_declined", "auth_device_code_expired", "auth_session_expired", "auth_invalid_offline_name", "auth_msa",
            "network", "offline", "java_required", "game_already_running", "instance_not_found", "account_not_found",
            "version_not_found", "hash_mismatch", "io", "invalid_input", "curseforge_key_missing", "secrets",
            "update_rejected", "otro",
        ];
        for lang in ["es", "en"] {
            for k in kinds {
                let key = super::key(k);
                assert_ne!(crate::i18n::text(lang, key), key, "{lang}: falta {key}");
            }
        }
    }
}
