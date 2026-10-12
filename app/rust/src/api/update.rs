//! Actualizaciones de la app (D8). La última consulta con versión nueva se
//! guarda aquí para que `install_update` no dependa de lo que mande Dart.

use std::sync::Mutex;

use lcore::update::{Package, UpdateInfo};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

static PENDING: Mutex<Option<UpdateInfo>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct UpdateView {
    pub version: String,
    pub current: String,
    pub pub_date: Option<String>,
    pub notes: Option<String>,
    pub notes_url: Option<String>,
    /// Se descarga e instala desde la app (AppImage).
    pub can_install: bool,
    /// Archivo del paquete o página de descargas.
    pub download_url: String,
}

/// Paquete de esta instalación: `appimage`, `flatpak`, `arch` o `unpackaged`.
#[flutter_rust_bridge::frb(sync)]
pub fn update_package() -> String {
    match Package::detect() {
        Package::AppImage { .. } => "appimage",
        Package::Flatpak => "flatpak",
        Package::Arch => "arch",
        Package::Unpackaged => "unpackaged",
    }
    .into()
}

/// `None` si `current` ya es la última versión.
pub async fn check_update(current: String) -> Result<Option<UpdateView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let info = core.check_update(&current, &Package::detect()).await?;
        let view = info.as_ref().map(|i| UpdateView {
            version: i.version.clone(),
            current: i.current.clone(),
            pub_date: i.pub_date.clone(),
            notes: i.notes.clone(),
            notes_url: i.notes_url.clone(),
            can_install: i.can_install(),
            download_url: i.download_url().to_owned(),
        });
        *PENDING.lock().expect("candado de actualización") = info;
        Ok(view)
    })
    .await
}

/// Instala la versión de la última consulta. Devuelve la ruta del AppImage,
/// que hay que relanzar para usarla.
pub async fn install_update() -> Result<String, BridgeError> {
    let core = core()?;
    let info = PENDING
        .lock()
        .expect("candado de actualización")
        .clone()
        .ok_or_else(|| BridgeError::internal("no hay ninguna actualización pendiente".into()))?;
    on_rt(async move {
        let path = core.install_update(&info).await?;
        Ok(path.to_string_lossy().into_owned())
    })
    .await
}
