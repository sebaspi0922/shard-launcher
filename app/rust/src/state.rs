//! Estado global del puente: un runtime de tokio y un único `Core` por proceso.

use std::future::Future;
use std::sync::OnceLock;

use lcore::Core;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

use crate::api::types::BridgeError;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static CORE: OnceCell<Core> = OnceCell::const_new();

pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("lcore")
            .build()
            .expect("no se pudo crear el runtime de tokio")
    })
}

/// Ejecuta `fut` en el runtime de tokio (lcore lo necesita: reqwest, fs, procesos).
pub async fn on_rt<F, T>(fut: F) -> Result<T, BridgeError>
where
    F: Future<Output = Result<T, BridgeError>> + Send + 'static,
    T: Send + 'static,
{
    runtime()
        .spawn(fut)
        .await
        .map_err(|e| BridgeError::internal(format!("tarea interrumpida: {e}")))?
}

pub async fn init_core(data_dir: Option<String>) -> Result<Core, BridgeError> {
    CORE.get_or_try_init(|| async move {
        let opts = lcore::Options {
            data_dir: data_dir.map(Into::into),
            ..lcore::Options::default()
        };
        Core::open(opts).await.map_err(BridgeError::from)
    })
    .await
    .cloned()
}

pub fn core() -> Result<Core, BridgeError> {
    CORE.get()
        .cloned()
        .ok_or_else(|| BridgeError::internal("el núcleo no está iniciado".into()))
}
