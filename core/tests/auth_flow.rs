//! Autenticación SISU sin login real y cuentas Shard contra un servidor simulado.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lcore::auth::{Account, AccountKind, AuthError, Endpoints, UserType};
use lcore::net::RetryPolicy;
use lcore::secrets::{MemoryStore, SecretStore};
use lcore::{Core, Error, Options};
use minecraft_auth::java::JavaAuthManager;
use minecraft_auth::msa::MsaToken;
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
    Env {
        server,
        core,
        secrets,
        _dir: dir,
    }
}

fn msa_account(name: &str) -> Account {
    Account {
        id: UUID.into(),
        name: name.into(),
        kind: AccountKind::Msa,
        skin_url: None,
        added_at: 1,
        last_used: None,
    }
}

async fn insert_account(env: &Env, account: Account) {
    let mut file = env.core.accounts().await.unwrap();
    file.schema = 1;
    file.accounts.retain(|a| a.id != account.id);
    file.active.get_or_insert_with(|| account.id.clone());
    file.accounts.push(account);
    let json = serde_json::to_vec_pretty(&file).unwrap();
    tokio::fs::write(env.core.paths().accounts_file(), json)
        .await
        .unwrap();
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Sesión completa con token y perfil válidos en caché: `launch_session()` no
/// necesita red. Las claves del dispositivo las genera el propio crate.
async fn cached_sisu_session(name: &str, mc_token: &str) -> String {
    let manager = JavaAuthManager::builder(reqwest::Client::new()).login_msa_token(MsaToken {
        expire_time_ms: now_ms() + 3_600_000,
        access_token: "MSA_ACCESS_DE_PRUEBA".into(),
        refresh_token: Some("MSA_REFRESH_DE_PRUEBA".into()),
    });
    let mut json = manager.to_json().await.unwrap();
    json["minecraft_token"] = json!({
        "expire_time_ms": now_ms() + 3_600_000,
        "token_type": "Bearer",
        "access_token": mc_token,
    });
    json["minecraft_profile"] = json!({
        "id": uuid::Uuid::parse_str(UUID).unwrap(),
        "name": name,
    });
    json.to_string()
}

#[tokio::test]
async fn sisu_lanza_con_launch_session_y_guarda_solo_en_el_keyring() {
    let env = env().await;
    insert_account(&env, msa_account("NombreAnterior")).await;
    let saved = cached_sisu_session("Jugador", "MC_TOKEN_1").await;
    env.secrets.set("shard.sisu", UUID, &saved).unwrap();

    // No hay client ID en settings: la sesión SISU tampoco lo necesita.
    let session = env.core.session(None).await.unwrap();
    assert_eq!(session.uuid, UUID);
    assert_eq!(session.name, "Jugador");
    assert_eq!(session.access_token.expose(), "MC_TOKEN_1");
    assert_eq!(session.user_type, UserType::Msa);
    assert!(!format!("{session:?}").contains("MC_TOKEN_1"));

    let stored = env.secrets.get("shard.sisu", UUID).unwrap().unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["device_type"], "Win32");
    assert_eq!(stored["minecraft_token"]["access_token"], "MC_TOKEN_1");
    assert!(stored.get("device_key_pair").is_some());

    let file = env.core.accounts().await.unwrap();
    assert_eq!(file.accounts[0].name, "Jugador");
    assert!(file.accounts[0].last_used.is_some());
    let on_disk = tokio::fs::read_to_string(env.core.paths().accounts_file())
        .await
        .unwrap();
    for secret in [
        "MSA_REFRESH_DE_PRUEBA",
        "MSA_ACCESS_DE_PRUEBA",
        "MC_TOKEN_1",
        "device_key_pair",
    ] {
        assert!(!on_disk.contains(secret), "{secret} en accounts.json");
    }
}

#[tokio::test]
async fn sesion_del_antiguo_client_id_pide_relogin_sin_borrar_la_cuenta() {
    let env = env().await;
    insert_account(&env, msa_account("Jugador")).await;
    env.secrets.set("shard.msa", UUID, "PRISM_REFRESH").unwrap();
    env.secrets
        .set(
            "shard.mc",
            UUID,
            r#"{"token":"LEGACY_MC","expires_at":9999999999}"#,
        )
        .unwrap();

    for error in [
        env.core.session(None).await.unwrap_err(),
        env.core.refresh_account(UUID).await.unwrap_err(),
    ] {
        assert!(
            matches!(error, Error::Auth(AuthError::LegacyMsaSession)),
            "{error:?}"
        );
        assert_eq!(error.kind(), "auth_session_expired");
        assert!(error.to_string().contains("vuelve a iniciar sesión"));
    }
    assert_eq!(env.core.accounts().await.unwrap().accounts.len(), 1);
    assert_eq!(
        env.secrets.get("shard.msa", UUID).unwrap().as_deref(),
        Some("PRISM_REFRESH")
    );
    assert!(env.secrets.get("shard.mc", UUID).unwrap().is_some());
    assert!(env.secrets.get("shard.sisu", UUID).unwrap().is_none());
}

#[tokio::test]
async fn perfil_premium_usa_el_token_de_launch_session() {
    let env = env().await;
    insert_account(&env, msa_account("NombreAnterior")).await;
    let saved = cached_sisu_session("Jugador", "MC_TOKEN_1").await;
    env.secrets.set("shard.sisu", UUID, &saved).unwrap();
    Mock::given(method("GET"))
        .and(path("/minecraft/profile"))
        .and(header("authorization", "Bearer MC_TOKEN_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": UUID,
            "name": "Jugador",
            "skins": [{
                "id": "s1",
                "state": "ACTIVE",
                "url": "http://textures/s1",
                "variant": "CLASSIC"
            }],
            "capes": []
        })))
        .expect(1)
        .mount(&env.server)
        .await;

    let account = env.core.refresh_account(UUID).await.unwrap();
    assert_eq!(account.name, "Jugador");
    assert_eq!(account.skin_url.as_deref(), Some("http://textures/s1"));
}

#[tokio::test]
async fn cuentas_offline_y_borrado_de_todos_los_secretos_microsoft() {
    let env = env().await;
    insert_account(&env, msa_account("Jugador")).await;
    let saved = cached_sisu_session("Jugador", "MC_TOKEN_1").await;
    env.secrets.set("shard.sisu", UUID, &saved).unwrap();
    env.secrets.set("shard.msa", UUID, "PRISM_REFRESH").unwrap();
    env.secrets.set("shard.mc", UUID, "LEGACY_MC").unwrap();

    let offline = env.core.add_offline_account("Steve").await.unwrap();
    assert_eq!(offline.id, "5627dd98e6be3c21b8a8e92344183641");
    assert!(env.core.add_offline_account("Steve").await.is_err());
    assert!(env.core.add_offline_account("x").await.is_err());

    env.core.set_active_account(&offline.id).await.unwrap();
    let session = env.core.session(None).await.unwrap();
    assert_eq!(session.user_type, UserType::Legacy);
    assert_eq!(session.uuid, offline.id);
    assert!(env.core.profile(None).await.is_err());

    env.core.remove_account(UUID).await.unwrap();
    for service in ["shard.msa", "shard.mc", "shard.sisu"] {
        assert_eq!(env.secrets.get(service, UUID).unwrap(), None);
    }
    let file = env.core.accounts().await.unwrap();
    assert_eq!(file.accounts.len(), 1);
    assert_eq!(file.active.as_deref(), Some(offline.id.as_str()));
}

#[tokio::test]
async fn shard_account_stores_tokens_and_launches_as_selected_profile() {
    let env = env().await;
    let base = format!("{}/yggdrasil/", env.server.uri());
    env.core
        .update_settings(|settings| settings.yggdrasil_base_url = Some(base))
        .await
        .unwrap();
    let offline = env.core.add_offline_account("Steve").await.unwrap();
    Mock::given(method("POST"))
        .and(path("/yggdrasil/authserver/authenticate"))
        .and(wiremock::matchers::body_json(json!({
            "agent": {"name": "Minecraft", "version": 1},
            "username": "test@example.org",
            "password": "SHARD_PASSWORD",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessToken": "SHARD_ACCESS",
            "clientToken": "SHARD_CLIENT",
            "selectedProfile": {"id": offline.id, "name": "Steve"},
        })))
        .expect(2)
        .mount(&env.server)
        .await;

    let login = || {
        env.core
            .add_shard_account("test@example.org", "SHARD_PASSWORD")
    };
    let shard = login().await.unwrap();
    assert_eq!(shard.kind, AccountKind::Shard);
    assert_ne!(shard.id, offline.id);
    assert_eq!(login().await.unwrap().id, shard.id);
    assert_eq!(env.core.accounts().await.unwrap().accounts.len(), 2);

    let stored = env
        .secrets
        .get("shard.yggdrasil", &shard.id)
        .unwrap()
        .unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["credentials"]["accessToken"], "SHARD_ACCESS");
    assert_eq!(stored["credentials"]["clientToken"], "SHARD_CLIENT");
    assert!(!stored.to_string().contains("SHARD_PASSWORD"));
    let accounts = tokio::fs::read_to_string(env.core.paths().accounts_file())
        .await
        .unwrap();
    for secret in ["SHARD_PASSWORD", "SHARD_ACCESS", "SHARD_CLIENT"] {
        assert!(!accounts.contains(secret));
    }

    env.core.set_active_account(&shard.id).await.unwrap();
    Mock::given(method("POST"))
        .and(path("/yggdrasil/authserver/validate"))
        .and(wiremock::matchers::body_json(json!({
            "accessToken": "SHARD_ACCESS",
            "clientToken": "SHARD_CLIENT",
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&env.server)
        .await;
    let session = env.core.session(None).await.unwrap();
    assert_eq!(session.uuid, offline.id);
    assert_eq!(session.name, "Steve");
    assert_eq!(session.user_type, UserType::Mojang);
    assert_eq!(session.access_token.expose(), "SHARD_ACCESS");
    assert_eq!(
        env.core.session(Some(&offline.id)).await.unwrap().user_type,
        UserType::Legacy
    );
    env.core.remove_account(&shard.id).await.unwrap();
    assert!(
        env.secrets
            .get("shard.yggdrasil", &shard.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(env.core.accounts().await.unwrap().active, Some(offline.id));
}

#[tokio::test]
async fn shard_login_rejects_failed_or_incomplete_response() {
    let env = env().await;
    env.core
        .update_settings(|settings| settings.yggdrasil_base_url = Some(env.server.uri()))
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/authserver/authenticate"))
        .respond_with(ResponseTemplate::new(403))
        .up_to_n_times(1)
        .mount(&env.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/authserver/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessToken": "SHARD_ACCESS",
            "clientToken": "SHARD_CLIENT",
        })))
        .mount(&env.server)
        .await;
    for _ in 0..2 {
        let err = env
            .core
            .add_shard_account("Steve", "password")
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "auth_unexpected");
    }
    assert!(env.core.accounts().await.unwrap().accounts.is_empty());
}

#[tokio::test]
async fn shard_login_maps_invalid_credentials() {
    let env = env().await;
    env.core
        .update_settings(|settings| settings.yggdrasil_base_url = Some(env.server.uri()))
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/authserver/authenticate"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid credentials. Invalid username or password."
        })))
        .expect(1)
        .mount(&env.server)
        .await;

    let err = env
        .core
        .add_shard_account("Steve", "wrong password")
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Auth(AuthError::InvalidCredentials)),
        "{err:?}"
    );
    assert_eq!(err.kind(), "auth_invalid_credentials");
    assert!(env.core.accounts().await.unwrap().accounts.is_empty());
}

#[tokio::test]
async fn shard_login_maps_unreachable_server_to_network() {
    let env = env().await;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    env.core
        .update_settings(|settings| settings.yggdrasil_base_url = Some(base))
        .await
        .unwrap();

    let err = env
        .core
        .add_shard_account("Steve", "password")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Network(_)), "{err:?}");
    assert_eq!(err.kind(), "network");
    assert!(env.core.accounts().await.unwrap().accounts.is_empty());
}

async fn shard_account_for_refresh(env: &Env) -> lcore::auth::Account {
    env.core
        .update_settings(|settings| settings.yggdrasil_base_url = Some(env.server.uri()))
        .await
        .unwrap();
    Mock::given(method("POST"))
        .and(path("/authserver/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessToken": "SHARD_ACCESS",
            "clientToken": "SHARD_CLIENT",
            "selectedProfile": {"id": UUID, "name": "JugadorShard"},
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    env.core
        .add_shard_account("Steve", "password")
        .await
        .unwrap()
}

#[tokio::test]
async fn shard_session_refreshes_and_stores_rejected_token() {
    let env = env().await;
    let shard = shard_account_for_refresh(&env).await;
    env.server.reset().await;
    Mock::given(method("POST"))
        .and(path("/authserver/validate"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid token."
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/authserver/refresh"))
        .and(wiremock::matchers::body_json(json!({
            "accessToken": "SHARD_ACCESS",
            "clientToken": "SHARD_CLIENT",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessToken": "SHARD_ACCESS_2",
            "clientToken": "SHARD_CLIENT",
            "selectedProfile": {"id": UUID, "name": "JugadorShard"},
        })))
        .expect(1)
        .mount(&env.server)
        .await;

    let session = env.core.session(Some(&shard.id)).await.unwrap();
    assert_eq!(session.access_token.expose(), "SHARD_ACCESS_2");
    let stored = env
        .secrets
        .get("shard.yggdrasil", &shard.id)
        .unwrap()
        .unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["credentials"]["accessToken"], "SHARD_ACCESS_2");

    env.server.reset().await;
    Mock::given(method("POST"))
        .and(path("/authserver/validate"))
        .and(body_string_contains("SHARD_ACCESS_2"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&env.server)
        .await;
    let session = env.core.session(Some(&shard.id)).await.unwrap();
    assert_eq!(session.access_token.expose(), "SHARD_ACCESS_2");
}

#[tokio::test]
async fn shard_session_with_rejected_refresh_is_expired() {
    let env = env().await;
    let shard = shard_account_for_refresh(&env).await;
    env.server.reset().await;
    Mock::given(method("POST"))
        .and(path("/authserver/validate"))
        .and(body_string_contains("SHARD_ACCESS"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid token."
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/authserver/refresh"))
        .and(body_string_contains("SHARD_ACCESS"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "ForbiddenOperationException",
            "errorMessage": "Invalid token."
        })))
        .expect(1)
        .mount(&env.server)
        .await;

    let err = env.core.session(Some(&shard.id)).await.unwrap_err();
    assert!(
        matches!(err, Error::Auth(AuthError::SessionExpired)),
        "{err:?}"
    );
    assert_eq!(err.kind(), "auth_session_expired");
}
