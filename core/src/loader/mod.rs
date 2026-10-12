//! Loaders: de `GameVersion` (mc + loader + versión) al id de versión instalado
//! en `versions/<id>/<id>.json`, listo para `resolve_version`.

pub mod fabric;
pub mod forge;

use serde::Serialize;

use crate::instance::{GameVersion, LoaderKind};
use crate::launcher::Core;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

impl Core {
    /// Versiones del loader disponibles para una versión de Minecraft (la más nueva primero).
    pub async fn loader_versions(
        &self,
        loader: LoaderKind,
        mc: &str,
    ) -> Result<Vec<LoaderVersion>> {
        match loader {
            LoaderKind::Fabric | LoaderKind::Quilt => {
                fabric::versions(self, fabric::Flavor::from(loader), mc).await
            }
            LoaderKind::Forge | LoaderKind::NeoForge => {
                forge::versions(self, forge::Flavor::from(loader), mc).await
            }
            LoaderKind::Vanilla => Ok(Vec::new()),
            other => Err(unsupported(other)),
        }
    }

    /// Instala (si hace falta) el perfil del loader y devuelve el id de versión a lanzar.
    pub async fn prepare_loader(&self, game: &GameVersion) -> Result<String> {
        match game.loader {
            LoaderKind::Vanilla => Ok(game.mc.clone()),
            LoaderKind::Fabric | LoaderKind::Quilt => {
                let flavor = fabric::Flavor::from(game.loader);
                let version = match &game.loader_version {
                    Some(v) => v.clone(),
                    None => fabric::latest_stable(self, flavor, &game.mc).await?,
                };
                fabric::install_profile(self, flavor, &game.mc, &version).await
            }
            LoaderKind::Forge | LoaderKind::NeoForge => {
                let flavor = forge::Flavor::from(game.loader);
                let version = match &game.loader_version {
                    Some(v) => v.clone(),
                    None => forge::latest_stable(self, flavor, &game.mc).await?,
                };
                if let Some(id) = forge::installed_id(self, flavor, &game.mc, &version).await? {
                    return Ok(id);
                }
                // El instalador corre con cualquier Java reciente; se prefiere la del juego.
                let major = self
                    .vanilla_version(&game.mc)
                    .await
                    .map(|v| v.java_major())
                    .unwrap_or(21)
                    .max(17);
                let java = self.ensure_java(major).await?;
                forge::install(self, flavor, &game.mc, &version, &java.path).await
            }
            // Las versiones importadas tal cual: el id está en `loader_version`.
            LoaderKind::Custom => game
                .loader_version
                .clone()
                .ok_or_else(|| Error::InvalidInput("versión custom sin id".into())),
            other => Err(unsupported(other)),
        }
    }
}

impl Core {
    /// Id de versión que dejó instalado el loader (para reinstalarlo), si se conoce.
    pub async fn installed_loader_id(&self, game: &GameVersion) -> Option<String> {
        let version = game.loader_version.as_deref()?;
        match game.loader {
            LoaderKind::Fabric | LoaderKind::Quilt => {
                Some(fabric::Flavor::from(game.loader).version_id(&game.mc, version))
            }
            LoaderKind::Forge | LoaderKind::NeoForge => {
                forge::installed_id(self, forge::Flavor::from(game.loader), &game.mc, version)
                    .await
                    .ok()
                    .flatten()
            }
            _ => None,
        }
    }
}

fn unsupported(loader: LoaderKind) -> Error {
    Error::InvalidInput(format!(
        "el loader {} aún no está soportado",
        loader.as_str()
    ))
}
