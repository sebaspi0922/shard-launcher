//! Inicio de sesión Yggdrasil y preparación de authlib-injector.

use std::path::Path;
use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::auth::{AuthError, join};
use crate::fsutil::write_atomic;
use crate::net::{Http, check_status};
use crate::{Error, Result};

const ARTIFACT_LATEST: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";
const ARTIFACT_PINNED: &str = "https://authlib-injector.yushi.moe/artifact/56.json";
const INJECTOR_VERSION: &str = "1.2.8";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Credentials {
    pub access_token: String,
    pub client_token: String,
    pub selected_profile: Profile,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Profile {
    pub id: Uuid,
    pub name: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
    #[serde(rename = "errorMessage")]
    error_message: String,
}

fn unexpected(detail: &str) -> Error {
    AuthError::Unexpected {
        service: "Yggdrasil",
        detail: detail.into(),
    }
    .into()
}

pub(crate) fn normalize_base(base: &str) -> Result<String> {
    let url = Url::parse(base.trim()).map_err(|_| unexpected("URL base no válida"))?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(unexpected(
            "la URL base debe usar HTTPS y no llevar credenciales, consulta ni fragmento",
        ));
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

pub(crate) async fn authenticate(
    http: &Http,
    base: &str,
    username: &str,
    password: &str,
) -> Result<Credentials> {
    let url = join(base, "authserver/authenticate");
    if http.is_offline() {
        return Err(Error::Offline { url });
    }
    let body = serde_json::json!({
        "agent": {"name": "Minecraft", "version": 1},
        "username": username,
        "password": password,
    });
    let response = http.send(|client| client.post(&url).json(&body)).await?;
    if response.status().as_u16() == 403 {
        let bytes = response.bytes().await?;
        if serde_json::from_slice::<ErrorResponse>(&bytes).is_ok_and(|error| {
            let message = error.error_message.to_ascii_lowercase();
            error.error == "ForbiddenOperationException"
                && (message.contains("credential")
                    || message.contains("invalid username or password"))
        }) {
            return Err(AuthError::InvalidCredentials.into());
        }
        return Err(unexpected("el servidor rechazó el inicio de sesión"));
    }
    if response.status().as_u16() == 401 {
        return Err(unexpected("el servidor rechazó el inicio de sesión"));
    }
    let bytes = check_status(response)?.bytes().await?;
    parse_credentials(&bytes)
}

fn parse_credentials(bytes: &[u8]) -> Result<Credentials> {
    let credentials: Credentials = serde_json::from_slice(bytes).map_err(|_| {
        unexpected("la respuesta no contiene tokens y un perfil seleccionado válidos")
    })?;
    if credentials.access_token.is_empty()
        || credentials.client_token.is_empty()
        || credentials.selected_profile.name.trim().is_empty()
        || credentials.selected_profile.id.is_nil()
    {
        return Err(unexpected("el servidor devolvió una sesión incompleta"));
    }
    Ok(credentials)
}

pub(crate) async fn validate(http: &Http, base: &str, credentials: &Credentials) -> Result<bool> {
    let url = join(base, "authserver/validate");
    if http.is_offline() {
        return Err(Error::Offline { url });
    }
    let body = serde_json::json!({
        "accessToken": credentials.access_token,
        "clientToken": credentials.client_token,
    });
    let response = http.send(|client| client.post(&url).json(&body)).await?;
    match response.status().as_u16() {
        204 => Ok(true),
        401 | 403 => Ok(false),
        status if (200..300).contains(&status) => Err(unexpected(
            "validate respondió con éxito pero sin el estado 204 esperado",
        )),
        _ => check_status(response).map(|_| true),
    }
}

pub(crate) async fn refresh(
    http: &Http,
    base: &str,
    credentials: &Credentials,
) -> Result<Credentials> {
    let url = join(base, "authserver/refresh");
    if http.is_offline() {
        return Err(Error::Offline { url });
    }
    let body = serde_json::json!({
        "accessToken": credentials.access_token,
        "clientToken": credentials.client_token,
    });
    let response = http.send(|client| client.post(&url).json(&body)).await?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(AuthError::SessionExpired.into());
    }
    let bytes = check_status(response)?.bytes().await?;
    parse_credentials(&bytes)
}

fn prefetched(bytes: &[u8]) -> Result<String> {
    let metadata: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| unexpected("metadatos JSON no válidos"))?;
    if !metadata.is_object() {
        return Err(unexpected("los metadatos deben ser un objeto JSON"));
    }
    Ok(STANDARD.encode(bytes))
}

pub(crate) async fn launch_args(http: &Http, base: &str, cache: &Path) -> Result<[String; 2]> {
    if http.is_offline() {
        return Err(Error::Offline { url: base.into() });
    }
    let response = check_status(
        http.send(|client| client.get(base).header("Accept", "application/json"))
            .await?,
    )?;
    let metadata = prefetched(&response.bytes().await?)?;
    let jar = cache.join(INJECTOR_VERSION).join("authlib-injector.jar");
    ensure_injector(http, &jar).await?;
    Ok(injector_args(&jar, base, &metadata))
}

fn injector_args(jar: &Path, base: &str, metadata: &str) -> [String; 2] {
    [
        format!("-javaagent:{}={base}", jar.display()),
        format!("-Dauthlibinjector.yggdrasil.prefetched={metadata}"),
    ]
}

#[derive(Deserialize)]
struct Artifact {
    version: String,
    download_url: String,
    checksums: Checksums,
}

#[derive(Deserialize)]
struct Checksums {
    sha1: Option<String>,
    sha256: Option<String>,
}

impl Checksums {
    fn matches(&self, bytes: &[u8]) -> bool {
        (self.sha256.is_some() || self.sha1.is_some())
            && self.sha256.as_ref().is_none_or(|expected| {
                hex::encode(Sha256::digest(bytes)).eq_ignore_ascii_case(expected)
            })
            && self.sha1.as_ref().is_none_or(|expected| {
                hex::encode(Sha1::digest(bytes)).eq_ignore_ascii_case(expected)
            })
    }
}

async fn ensure_injector(http: &Http, jar: &Path) -> Result<()> {
    let ttl = Duration::from_secs(24 * 60 * 60);
    let mut artifact: Artifact = http.get_json_cached(ARTIFACT_LATEST, ttl).await?;
    if artifact.version != INJECTOR_VERSION {
        artifact = http.get_json_cached(ARTIFACT_PINNED, ttl).await?;
    }
    if artifact.version != INJECTOR_VERSION || !artifact.download_url.starts_with("https://") {
        return Err(unexpected("manifiesto de authlib-injector no válido"));
    }
    if let Ok(bytes) = tokio::fs::read(jar).await
        && artifact.checksums.matches(&bytes)
    {
        return Ok(());
    }
    let response = check_status(
        http.send(|client| client.get(&artifact.download_url))
            .await?,
    )?;
    let bytes = response.bytes().await?;
    if !artifact.checksums.matches(&bytes) {
        return Err(unexpected(
            "la suma de authlib-injector no coincide con el manifiesto",
        ));
    }
    write_atomic(jar, &bytes).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_preserves_json_bytes() {
        assert_eq!(prefetched(br#"{"meta":{}}"#).unwrap(), "eyJtZXRhIjp7fX0=");
        assert!(prefetched(b"[]").is_err());
        assert!(prefetched(b"<html>").is_err());
    }

    #[test]
    fn exact_injector_arguments() {
        assert_eq!(
            injector_args(
                Path::new("/cache with spaces/authlib-injector.jar"),
                &normalize_base("https://example.org/api///").unwrap(),
                "e30=",
            ),
            [
                "-javaagent:/cache with spaces/authlib-injector.jar=https://example.org/api",
                "-Dauthlibinjector.yggdrasil.prefetched=e30=",
            ],
        );
    }

    #[test]
    fn verifies_every_published_checksum() {
        let mut sums = Checksums {
            sha1: None,
            sha256: Some(hex::encode(Sha256::digest(b"jar"))),
        };
        assert!(sums.matches(b"jar"));
        assert!(!sums.matches(b"corrupt"));
        sums.sha1 = Some(hex::encode(Sha1::digest(b"jar")));
        assert!(sums.matches(b"jar"));
        sums.sha1 = Some("invalid".into());
        assert!(!sums.matches(b"jar"));
    }
}
