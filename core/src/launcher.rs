//! Fachada `Core`: la usan la CLI y el puente de Flutter (D7).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock, broadcast};

use crate::auth::Endpoints;
use crate::events::{Event, EventBus};
use crate::fsutil::{read_json, write_json};
use crate::launch::GameInfo;
use crate::net::{Http, RetryPolicy};
use crate::paths::Paths;
use crate::secrets::{KeyringStore, SecretStore};
use crate::settings::Settings;
use crate::{Error, Result};

#[derive(Default)]
pub struct Options {
    /// Directorio de datos; `None` = `Paths::default_root()`.
    pub data_dir: Option<PathBuf>,
    /// No hacer peticiones de red (solo caché).
    pub offline: bool,
    /// Almacén de secretos; `None` = keyring del sistema.
    pub secrets: Option<Arc<dyn SecretStore>>,
    pub endpoints: Endpoints,
    pub retry: Option<RetryPolicy>,
}

#[derive(Clone)]
pub struct Core {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub paths: Paths,
    pub settings: RwLock<Settings>,
    pub http: Http,
    pub events: EventBus,
    pub secrets: Arc<dyn SecretStore>,
    pub endpoints: Endpoints,
    /// Serializa las lecturas-modificaciones de `accounts.json`.
    pub accounts_lock: Mutex<()>,
    /// Serializa las lecturas-modificaciones de `instance.json`.
    pub instances_lock: Mutex<()>,
    /// Partidas en marcha por `game_id`.
    pub games: Mutex<HashMap<u64, GameInfo>>,
}

impl Core {
    pub async fn open(opts: Options) -> Result<Self> {
        let root = match opts.data_dir {
            Some(dir) => dir,
            None => Paths::default_root()?,
        };
        tokio::fs::create_dir_all(&root)
            .await
            .map_err(|e| Error::io(&root, e))?;
        let paths = Paths::new(root);
        let settings = read_json::<Settings>(&paths.settings_file())
            .await?
            .unwrap_or_default();
        let mut http = Http::new(paths.http_cache_dir(), opts.offline)?;
        if let Some(retry) = opts.retry {
            http = http.with_retry(retry);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                paths,
                settings: RwLock::new(settings),
                http,
                events: EventBus::new(),
                secrets: opts.secrets.unwrap_or_else(|| Arc::new(KeyringStore)),
                endpoints: opts.endpoints,
                accounts_lock: Mutex::new(()),
                instances_lock: Mutex::new(()),
                games: Mutex::new(HashMap::new()),
            }),
        })
    }

    pub fn paths(&self) -> &Paths {
        &self.inner.paths
    }

    pub fn http(&self) -> &Http {
        &self.inner.http
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    pub async fn settings(&self) -> Settings {
        self.inner.settings.read().await.clone()
    }

    /// Modifica los ajustes y los guarda.
    pub async fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let mut guard = self.inner.settings.write().await;
        let mut next = guard.clone();
        f(&mut next);
        write_json(&self.inner.paths.settings_file(), &next).await?;
        *guard = next.clone();
        Ok(next)
    }

    pub(crate) async fn secret_get(&self, service: String, user: String) -> Result<Option<String>> {
        let store = self.inner.secrets.clone();
        blocking(move || store.get(&service, &user)).await
    }

    pub(crate) async fn secret_set(
        &self,
        service: String,
        user: String,
        value: String,
    ) -> Result<()> {
        let store = self.inner.secrets.clone();
        blocking(move || store.set(&service, &user, &value)).await
    }

    pub(crate) async fn secret_delete(&self, service: String, user: String) -> Result<()> {
        let store = self.inner.secrets.clone();
        blocking(move || store.delete(&service, &user)).await
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Secrets(format!("tarea interrumpida: {e}")))?
}
