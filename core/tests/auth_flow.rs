//! Cadena de login completa contra un servidor simulado (wiremock).

use std::sync::Arc;
use std::time::Duration;

use lcore::auth::{AccountKind, AuthError, Endpoints, UserType};
use lcore::net::RetryPolicy;
use lcore::secrets::{MemoryStore, SecretStore};
use lcore::{Core, Error, Options};
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CLIENT_ID: &str = "00000000-0000-0000-0000-000000000001";
const UUID: &str = "11111111222233334444555555555555";

struct Env {
    server: MockServer,
    core: Core,
    secrets: Arc<MemoryStore>,
    _dir: tempfile::TempDir,
}

async fn env() -> Env {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let secrets = Arc::new(MemoryStore::default());
    let core = Core::open(Options {
        data_dir: Some(dir.path().to_path_buf()),
        secrets: Some(secrets.clone()),
        endpoints: Endpoints::all(&server.uri()),
        retry: Some(RetryPolicy {
            max_attempts: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(2),
        }),
        ..Options::default()
    })
    .await
    .unwrap();
    core.update_settings(|s| s.msa_client_id = Some(CLIENT_ID.into()))
        .await
        .unwrap();
    Env {
        server,
        core,
        secrets,
        _dir: dir,
    }
}

async fn mount_device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/devicecode"))
        .and(body_string_contains("XboxLive.signin+offline_access"))
        .and(body_string_contains(CLIENT_ID))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "DEVCODE",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://www.microsoft.com/link",
            "expires_in": 900,
            "interval": 0,
            "message": "..."
        })))
        .mount(server)
        .await;
}

/// Token endpoint: dos respuestas «pendiente» y luego éxito.
async fn mount_token_after_pending(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/token"))
        .and(body_string_contains("device_code=DEVCODE"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending",
            "error_description": "AADSTS70016: pending\r\nTrace ID: x"
        })))
        .up_to_n_times(2)
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/token"))
        .and(body_string_contains("device_code=DEVCODE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token_type": "Bearer",
            "access_token": "MSA_ACCESS_1",
            "refresh_token": "MSA_REFRESH_1",
            "expires_in": 3600
        })))
        .mount(server)
        .await;
}

async fn mount_xbox(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/user/authenticate"))
        .and(body_string_contains("d=MSA_ACCESS_"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "XBL_TOKEN",
            "DisplayClaims": { "xui": [ { "uhs": "UHS1" } ] }
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/xsts/authorize"))
        .and(body_string_contains("XBL_TOKEN"))
        .and(body_string_contains("rp://api.minecraftservices.com/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "XSTS_TOKEN",
            "DisplayClaims": { "xui": [ { "uhs": "UHS1" } ] }
        })))
        .mount(server)
        .await;
}

async fn mount_minecraft(server: &MockServer, mc_token: &str) {
    Mock::given(method("POST"))
        .and(path("/authentication/login_with_xbox"))
        .and(body_string_contains("XBL3.0 x=UHS1;XSTS_TOKEN"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "username": "irrelevante",
            "access_token": mc_token,
            "token_type": "Bearer",
            "expires_in": 86400
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/minecraft/profile"))
        .and(header("authorization", format!("Bearer {mc_token}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": UUID,
            "name": "Jugador",
            "skins": [ { "id": "s1", "state": "ACTIVE", "url": "http://textures/s1", "variant": "CLASSIC" } ],
            "capes": []
        })))
        .mount(server)
        .await;
}

async fn login(env: &Env) -> lcore::auth::Account {
    let code = env.core.begin_msa_login().await.unwrap();
    assert_eq!(code.user_code, "ABCD-EFGH");
    env.core.complete_msa_login(&code).await.unwrap()
}

#[tokio::test]
async fn login_completo_guarda_cuenta_sin_tokens_en_disco() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    mount_minecraft(&env.server, "MC_TOKEN_1").await;

    let account = login(&env).await;
    assert_eq!(account.id, UUID);
    assert_eq!(account.name, "Jugador");
    assert_eq!(account.kind, AccountKind::Msa);
    assert_eq!(account.skin_url.as_deref(), Some("http://textures/s1"));

    let file = env.core.accounts().await.unwrap();
    assert_eq!(file.active.as_deref(), Some(UUID));

    // Los secretos están en el almacén, no en accounts.json.
    assert_eq!(
        env.secrets.get("shard.msa", UUID).unwrap().as_deref(),
        Some("MSA_REFRESH_1")
    );
    let on_disk = std::fs::read_to_string(env.core.paths().accounts_file()).unwrap();
    for secret in ["MSA_REFRESH_1", "MSA_ACCESS_1", "MC_TOKEN_1", "XSTS_TOKEN"] {
        assert!(!on_disk.contains(secret), "{secret} en accounts.json");
    }

    // La sesión usa el token en caché sin volver a la red.
    let session = env.core.session(None).await.unwrap();
    assert_eq!(session.access_token.expose(), "MC_TOKEN_1");
    assert_eq!(session.user_type, UserType::Msa);
    assert!(!format!("{session:?}").contains("MC_TOKEN_1"));
}

#[tokio::test]
async fn refresco_rota_el_refresh_token() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    mount_minecraft(&env.server, "MC_TOKEN_1").await;
    login(&env).await;

    env.server.reset().await;
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=MSA_REFRESH_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "MSA_ACCESS_2",
            "refresh_token": "MSA_REFRESH_2",
            "expires_in": 3600
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    mount_xbox(&env.server).await;
    mount_minecraft(&env.server, "MC_TOKEN_2").await;

    let account = env.core.refresh_account(UUID).await.unwrap();
    assert_eq!(account.name, "Jugador");
    assert_eq!(
        env.secrets.get("shard.msa", UUID).unwrap().as_deref(),
        Some("MSA_REFRESH_2")
    );
    let session = env.core.session(Some("Jugador")).await.unwrap();
    assert_eq!(session.access_token.expose(), "MC_TOKEN_2");
}

#[tokio::test]
async fn refresh_token_revocado_pide_login() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    mount_minecraft(&env.server, "MC_TOKEN_1").await;
    login(&env).await;

    env.server.reset().await;
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "AADSTS70000: revoked"
        })))
        .mount(&env.server)
        .await;
    let err = env.core.refresh_account(UUID).await.unwrap_err();
    assert!(
        matches!(err, Error::Auth(AuthError::SessionExpired)),
        "{err:?}"
    );
    assert_eq!(err.kind(), "auth_session_expired");
}

#[tokio::test]
async fn app_no_aprobada_da_error_claro() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    Mock::given(method("POST"))
        .and(path("/authentication/login_with_xbox"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "path": "/authentication/login_with_xbox",
            "errorMessage": "Invalid app registration, see https://aka.ms/AppRegInfo for more information"
        })))
        .mount(&env.server)
        .await;

    let code = env.core.begin_msa_login().await.unwrap();
    let err = env.core.complete_msa_login(&code).await.unwrap_err();
    assert_eq!(err.kind(), "auth_app_not_approved");
    assert!(env.core.accounts().await.unwrap().accounts.is_empty());
}

#[tokio::test]
async fn xerr_sin_perfil_de_xbox() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    Mock::given(method("POST"))
        .and(path("/user/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Token": "XBL_TOKEN",
            "DisplayClaims": { "xui": [ { "uhs": "UHS1" } ] }
        })))
        .mount(&env.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/xsts/authorize"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "Identity": "0", "XErr": 2148916233u64, "Message": "", "Redirect": "https://start.ui.xboxlive.com/CreateAccount"
        })))
        .mount(&env.server)
        .await;

    let code = env.core.begin_msa_login().await.unwrap();
    let err = env.core.complete_msa_login(&code).await.unwrap_err();
    match err {
        Error::Auth(AuthError::Xbox { xerr, message }) => {
            assert_eq!(xerr, 2148916233);
            assert!(message.contains("perfil de Xbox"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn sin_juego_y_sin_perfil() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    Mock::given(method("POST"))
        .and(path("/authentication/login_with_xbox"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "MC", "expires_in": 86400
        })))
        .mount(&env.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/minecraft/profile"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&env.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/entitlements/mcstore"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "items": [] })))
        .mount(&env.server)
        .await;

    let code = env.core.begin_msa_login().await.unwrap();
    let err = env.core.complete_msa_login(&code).await.unwrap_err();
    assert_eq!(err.kind(), "auth_not_entitled");
}

#[tokio::test]
async fn login_rechazado_y_sin_client_id() {
    let env = env().await;
    mount_device_code(&env.server).await;
    Mock::given(method("POST"))
        .and(path("/consumers/oauth2/v2.0/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_declined", "error_description": "x"
        })))
        .mount(&env.server)
        .await;
    let code = env.core.begin_msa_login().await.unwrap();
    let err = env.core.complete_msa_login(&code).await.unwrap_err();
    assert_eq!(err.kind(), "auth_declined");

    // Sin client ID (y sin la variable de entorno) no se contacta a nadie.
    if std::env::var_os(lcore::settings::MSA_CLIENT_ID_ENV).is_none() {
        env.core
            .update_settings(|s| s.msa_client_id = None)
            .await
            .unwrap();
        let err = env.core.begin_msa_login().await.unwrap_err();
        assert_eq!(err.kind(), "auth_missing_client_id");
    }
}

#[tokio::test]
async fn cuentas_offline_y_borrado_de_secretos() {
    let env = env().await;
    mount_device_code(&env.server).await;
    mount_token_after_pending(&env.server).await;
    mount_xbox(&env.server).await;
    mount_minecraft(&env.server, "MC_TOKEN_1").await;
    login(&env).await;

    let offline = env.core.add_offline_account("Steve").await.unwrap();
    assert_eq!(offline.id, "5627dd98e6be3c21b8a8e92344183641");
    assert!(env.core.add_offline_account("Steve").await.is_err());
    assert!(env.core.add_offline_account("x").await.is_err());

    // La activa sigue siendo la primera.
    let s = env.core.session(None).await.unwrap();
    assert_eq!(s.name, "Jugador");
    env.core.set_active_account(&offline.id).await.unwrap();
    let s = env.core.session(None).await.unwrap();
    assert_eq!(s.user_type, UserType::Legacy);
    assert_eq!(s.uuid, offline.id);
    assert!(env.core.profile(None).await.is_err());

    env.core.remove_account(UUID).await.unwrap();
    assert_eq!(env.secrets.get("shard.msa", UUID).unwrap(), None);
    assert_eq!(env.secrets.get("shard.mc", UUID).unwrap(), None);
    let file = env.core.accounts().await.unwrap();
    assert_eq!(file.accounts.len(), 1);
    assert_eq!(file.active.as_deref(), Some(offline.id.as_str()));
}
