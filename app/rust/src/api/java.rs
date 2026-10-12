//! Instalaciones de Java: detectadas y gestionadas por el launcher.

use lcore::java::{JavaInstall, JavaSource};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct JavaView {
    /// Ejecutable `bin/java`.
    pub path: String,
    pub major: u32,
    pub version: String,
    pub vendor: Option<String>,
    /// `override` (elegida a mano), `managed` (instalada por la app) o `system`.
    pub source: String,
}

impl From<JavaInstall> for JavaView {
    fn from(j: JavaInstall) -> Self {
        Self {
            path: j.path.display().to_string(),
            major: j.major,
            version: j.version,
            vendor: j.vendor,
            source: match j.source {
                JavaSource::Override => "override",
                JavaSource::Managed => "managed",
                JavaSource::System => "system",
            }
            .into(),
        }
    }
}

/// Javas encontradas (elegidas a mano, gestionadas y del sistema).
pub async fn list_java() -> Result<Vec<JavaView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_java()
            .await?
            .into_iter()
            .map(JavaView::from)
            .collect())
    })
    .await
}

/// Instala un JRE de esa versión mayor. El progreso llega por eventos
/// (etapa `java`).
pub async fn install_java(major: u32) -> Result<JavaView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.install_java(major).await?.into()) }).await
}
