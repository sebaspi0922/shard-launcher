//! Cliente HTTP único: User-Agent propio, timeouts, reintentos con backoff y
//! caché de JSON con validación (ETag / Last-Modified) para trabajar sin red.

use std::path::PathBuf;
use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::fsutil::{now_secs, read_json, write_json};
use crate::{Error, Result};

pub const USER_AGENT: &str = concat!(
    "Shard/",
    env!("CARGO_PKG_VERSION"),
    " (personal Minecraft launcher)"
);

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// Espera antes del intento `attempt` (1 = primer reintento), con jitter.
    fn delay(&self, attempt: u32) -> Duration {
        let exp = self
            .base_delay
            .saturating_mul(1 << attempt.saturating_sub(1).min(16));
        let capped = exp.min(self.max_delay);
        let jitter = capped.mul_f64(fastrand::f64() * 0.25);
        capped + jitter
    }
}

#[derive(Debug, Clone)]
pub struct Http {
    client: reqwest::Client,
    cache_dir: PathBuf,
    offline: bool,
    retry: RetryPolicy,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    fetched_at: i64,
    body: String,
}

impl Http {
    pub fn new(cache_dir: PathBuf, offline: bool) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            client,
            cache_dir,
            offline,
            retry: RetryPolicy::default(),
        })
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    /// Envía la petición que construye `build`, reintentando errores de
    /// conexión, 429 y 5xx. Devuelve la última respuesta sea cual sea su estado.
    pub async fn send(
        &self,
        build: impl Fn(&reqwest::Client) -> RequestBuilder,
    ) -> Result<Response> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let last = attempt >= self.retry.max_attempts;
            match build(&self.client).send().await {
                Ok(resp) if !last && is_retryable(resp.status()) => {
                    let wait = retry_after(&resp).unwrap_or_else(|| self.retry.delay(attempt));
                    tracing::debug!(url = %resp.url(), status = %resp.status(), ?wait, "reintento");
                    tokio::time::sleep(wait.min(self.retry.max_delay * 3)).await;
                }
                Ok(resp) => return Ok(resp),
                Err(e) if !last && (e.is_connect() || e.is_timeout() || e.is_request()) => {
                    let wait = self.retry.delay(attempt);
                    tracing::debug!(error = %e, ?wait, "reintento por error de red");
                    tokio::time::sleep(wait).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// GET de un JSON con reintentos, sin caché.
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        if self.offline {
            return Err(Error::Offline { url: url.into() });
        }
        let resp = self.send(|c| c.get(url)).await?;
        let resp = check_status(resp)?;
        let text = resp.text().await?;
        serde_json::from_str(&text).map_err(|e| Error::json(url, e))
    }

    /// GET de un JSON con caché en disco. Si la copia tiene menos de `ttl`, no
    /// se pide; si no, se revalida. Sin red, se usa la copia aunque esté vieja.
    pub async fn get_json_cached<T: DeserializeOwned>(
        &self,
        url: &str,
        ttl: Duration,
    ) -> Result<T> {
        let path = self.cache_path(url);
        let cached: Option<CacheEntry> = read_json(&path).await.ok().flatten();
        let parse = |body: &str| serde_json::from_str::<T>(body).map_err(|e| Error::json(url, e));

        if let Some(entry) = &cached {
            let age = now_secs().saturating_sub(entry.fetched_at);
            if self.offline || (age >= 0 && (age as u64) < ttl.as_secs()) {
                return parse(&entry.body);
            }
        } else if self.offline {
            return Err(Error::Offline { url: url.into() });
        }

        let result = self
            .send(|c| {
                let mut req = c.get(url);
                if let Some(entry) = &cached {
                    if let Some(etag) = &entry.etag {
                        req = req.header(header::IF_NONE_MATCH, etag);
                    }
                    if let Some(lm) = &entry.last_modified {
                        req = req.header(header::IF_MODIFIED_SINCE, lm);
                    }
                }
                req
            })
            .await;

        let resp = match result {
            Ok(resp) => resp,
            Err(e) => {
                // Sin red: la copia vieja es mejor que nada.
                if let Some(entry) = &cached {
                    tracing::warn!(%url, error = %e, "uso la copia en caché");
                    return parse(&entry.body);
                }
                return Err(e);
            }
        };

        if resp.status() == StatusCode::NOT_MODIFIED
            && let Some(mut entry) = cached
        {
            entry.fetched_at = now_secs();
            let value = parse(&entry.body)?;
            let _ = write_json(&path, &entry).await;
            return Ok(value);
        }

        let resp = match check_status(resp) {
            Ok(resp) => resp,
            Err(e)
                if cached.is_some()
                    && matches!(e, Error::HttpStatus { status, .. } if status >= 500) =>
            {
                tracing::warn!(%url, error = %e, "uso la copia en caché");
                return parse(&cached.unwrap().body);
            }
            Err(e) => return Err(e),
        };
        let etag = header_string(&resp, header::ETAG);
        let last_modified = header_string(&resp, header::LAST_MODIFIED);
        let body = resp.text().await?;
        let value = parse(&body)?;
        let entry = CacheEntry {
            url: url.into(),
            etag,
            last_modified,
            fetched_at: now_secs(),
            body,
        };
        if let Err(e) = write_json(&path, &entry).await {
            tracing::warn!(%url, error = %e, "no se pudo guardar en caché");
        }
        Ok(value)
    }

    fn cache_path(&self, url: &str) -> PathBuf {
        let hash = hex::encode(Sha256::digest(url.as_bytes()));
        self.cache_dir.join(format!("{}.json", &hash[..32]))
    }
}

fn is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_after(resp: &Response) -> Option<Duration> {
    resp.headers()
        .get(header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

fn header_string(resp: &Response, name: header::HeaderName) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

/// Convierte un estado no exitoso en `Error::HttpStatus`.
pub fn check_status(resp: Response) -> Result<Response> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        Err(Error::HttpStatus {
            status: status.as_u16(),
            url: resp.url().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header as h, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fast() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
        }
    }

    #[tokio::test]
    async fn reintenta_5xx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/x"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/x"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[1]"))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().into(), false)
            .unwrap()
            .with_retry(fast());
        let v: Vec<u32> = http.get_json(&format!("{}/x", server.uri())).await.unwrap();
        assert_eq!(v, vec![1]);
    }

    #[tokio::test]
    async fn no_reintenta_4xx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().into(), false)
            .unwrap()
            .with_retry(fast());
        let err = http
            .get_json::<serde_json::Value>(&format!("{}/x", server.uri()))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::HttpStatus { status: 404, .. }));
    }

    #[tokio::test]
    async fn cache_revalida_con_etag_y_sirve_sin_red() {
        let server = MockServer::start().await;
        let url = format!("{}/m.json", server.uri());
        Mock::given(method("GET"))
            .and(h("if-none-match", "\"v1\""))
            .respond_with(ResponseTemplate::new(304))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("etag", "\"v1\"")
                    .set_body_string(r#"{"a":1}"#),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().into(), false)
            .unwrap()
            .with_retry(fast());
        let v: serde_json::Value = http.get_json_cached(&url, Duration::ZERO).await.unwrap();
        assert_eq!(v["a"], 1);
        // TTL cero: revalida y recibe 304.
        let v: serde_json::Value = http.get_json_cached(&url, Duration::ZERO).await.unwrap();
        assert_eq!(v["a"], 1);

        let offline = Http::new(dir.path().into(), true).unwrap();
        let v: serde_json::Value = offline.get_json_cached(&url, Duration::ZERO).await.unwrap();
        assert_eq!(v["a"], 1);
        let err = offline
            .get_json_cached::<serde_json::Value>(&format!("{}/otro", server.uri()), Duration::ZERO)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Offline { .. }));
    }
}
