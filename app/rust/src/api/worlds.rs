//! Mundos, datapacks y capturas de una instancia.

use lcore::world::{Difficulty, GameMode, WorldInfo, WorldPatch};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct WorldView {
    /// Carpeta dentro de `saves/` (identifica el mundo).
    pub folder: String,
    pub name: String,
    /// `survival`, `creative`, `adventure` o `spectator`.
    pub game_mode: Option<String>,
    /// `peaceful`, `easy`, `normal` o `hard`.
    pub difficulty: Option<String>,
    pub hardcore: Option<bool>,
    pub cheats: Option<bool>,
    pub last_played_ms: Option<i64>,
    pub version_name: Option<String>,
    pub size_bytes: u64,
    /// Ruta de `icon.png`, si existe.
    pub icon: Option<String>,
    /// El `level.dat` no se pudo leer.
    pub error: Option<String>,
}

impl From<WorldInfo> for WorldView {
    fn from(w: WorldInfo) -> Self {
        Self {
            folder: w.folder,
            name: w.name,
            game_mode: w.game_mode.map(|m| m.as_str().into()),
            difficulty: w.difficulty.map(|d| d.as_str().into()),
            hardcore: w.hardcore,
            cheats: w.cheats,
            last_played_ms: w.last_played_ms,
            version_name: w.version_name,
            size_bytes: w.size_bytes,
            icon: w.icon.map(|p| p.display().to_string()),
            error: w.error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatapackView {
    pub name: String,
    pub is_dir: bool,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ScreenshotView {
    pub file: String,
    pub path: String,
    pub size: u64,
    /// Segundos Unix de la última modificación.
    pub modified: Option<i64>,
}

fn invalid(message: String) -> BridgeError {
    BridgeError {
        kind: "invalid_input".into(),
        message,
    }
}

pub async fn list_worlds(instance_id: String) -> Result<Vec<WorldView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_worlds(&instance_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

/// Cambia el nombre visible (la carpeta no se renombra).
pub async fn rename_world(
    instance_id: String,
    folder: String,
    name: String,
) -> Result<WorldView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.rename_world(&instance_id, &folder, &name).await?.into()) }).await
}

/// Modo, dificultad, hardcore y trucos; `None` = no tocar.
pub async fn update_world(
    instance_id: String,
    folder: String,
    game_mode: Option<String>,
    difficulty: Option<String>,
    hardcore: Option<bool>,
    cheats: Option<bool>,
) -> Result<WorldView, BridgeError> {
    let core = core()?;
    let patch = WorldPatch {
        game_mode: game_mode
            .map(|m| GameMode::parse(&m).ok_or_else(|| invalid(format!("modo desconocido: {m}"))))
            .transpose()?,
        difficulty: difficulty
            .map(|d| {
                Difficulty::parse(&d).ok_or_else(|| invalid(format!("dificultad desconocida: {d}")))
            })
            .transpose()?,
        hardcore,
        allow_commands: cheats,
    };
    on_rt(async move { Ok(core.update_world(&instance_id, &folder, patch).await?.into()) }).await
}

/// Borra el mundo para siempre.
pub async fn delete_world(instance_id: String, folder: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.delete_world(&instance_id, &folder).await?) }).await
}

pub async fn list_datapacks(
    instance_id: String,
    world: String,
) -> Result<Vec<DatapackView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_datapacks(&instance_id, &world)
            .await?
            .into_iter()
            .map(|d| DatapackView {
                name: d.name,
                is_dir: d.is_dir,
                enabled: d.enabled,
                size_bytes: d.size_bytes,
            })
            .collect())
    })
    .await
}

pub async fn set_datapack_enabled(
    instance_id: String,
    world: String,
    name: String,
    enabled: bool,
) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .set_datapack_enabled(&instance_id, &world, &name, enabled)
            .await?)
    })
    .await
}

pub async fn remove_datapack(
    instance_id: String,
    world: String,
    name: String,
) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.remove_datapack(&instance_id, &world, &name).await?) }).await
}

pub async fn list_screenshots(instance_id: String) -> Result<Vec<ScreenshotView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_screenshots(&instance_id)
            .await?
            .into_iter()
            .map(|s| ScreenshotView {
                file: s.file,
                path: s.path.display().to_string(),
                size: s.size,
                modified: s.modified,
            })
            .collect())
    })
    .await
}

pub async fn delete_screenshot(instance_id: String, file: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.delete_screenshot(&instance_id, &file).await?) }).await
}
