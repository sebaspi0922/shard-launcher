//! Mundos, datapacks y capturas de una instancia (research/06-lanzamiento.md §8).
//!
//! `saves/<carpeta>/level.dat` es NBT con gzip; se lee y se reescribe como
//! `fastnbt::Value` para conservar las etiquetas que no se tocan. Los campos
//! cambian de sitio entre versiones, así que la lectura es tolerante: lo que
//! falta es `None`.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fastnbt::Value;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::Serialize;

use crate::launcher::Core;
use crate::paths::single_segment;
use crate::{Error, Result};

const LEVEL_DAT: &str = "level.dat";
/// Subcarpeta de `datapacks/` donde viven los desactivados.
const DISABLED_DIR: &str = ".disabled";
/// Tope de `level.dat` descomprimido (los reales pesan unos KB).
const MAX_LEVEL_BYTES: u64 = 64 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl GameMode {
    pub fn id(self) -> i32 {
        match self {
            GameMode::Survival => 0,
            GameMode::Creative => 1,
            GameMode::Adventure => 2,
            GameMode::Spectator => 3,
        }
    }

    pub fn from_id(id: i64) -> Option<Self> {
        Some(match id {
            0 => GameMode::Survival,
            1 => GameMode::Creative,
            2 => GameMode::Adventure,
            3 => GameMode::Spectator,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            GameMode::Survival => "survival",
            GameMode::Creative => "creative",
            GameMode::Adventure => "adventure",
            GameMode::Spectator => "spectator",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "survival" => GameMode::Survival,
            "creative" => GameMode::Creative,
            "adventure" => GameMode::Adventure,
            "spectator" => GameMode::Spectator,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn id(self) -> i8 {
        match self {
            Difficulty::Peaceful => 0,
            Difficulty::Easy => 1,
            Difficulty::Normal => 2,
            Difficulty::Hard => 3,
        }
    }

    pub fn from_id(id: i64) -> Option<Self> {
        Some(match id {
            0 => Difficulty::Peaceful,
            1 => Difficulty::Easy,
            2 => Difficulty::Normal,
            3 => Difficulty::Hard,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "peaceful",
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "peaceful" => Difficulty::Peaceful,
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            "hard" => Difficulty::Hard,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldInfo {
    /// Carpeta dentro de `saves/` (identifica el mundo).
    pub folder: String,
    /// `LevelName`; la carpeta si falta.
    pub name: String,
    pub game_mode: Option<GameMode>,
    pub difficulty: Option<Difficulty>,
    pub hardcore: Option<bool>,
    /// `allowCommands`.
    pub cheats: Option<bool>,
    pub last_played_ms: Option<i64>,
    pub version_name: Option<String>,
    pub data_version: Option<i32>,
    pub size_bytes: u64,
    pub icon: Option<PathBuf>,
    /// El `level.dat` no se pudo leer; el resto de campos queda vacío.
    pub error: Option<String>,
}

/// Cambios sobre `level.dat`; `None` = no tocar.
#[derive(Debug, Clone, Default)]
pub struct WorldPatch {
    pub game_mode: Option<GameMode>,
    pub difficulty: Option<Difficulty>,
    pub hardcore: Option<bool>,
    pub allow_commands: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatapackInfo {
    /// Nombre del archivo `.zip` o de la carpeta.
    pub name: String,
    pub is_dir: bool,
    pub enabled: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotInfo {
    pub file: String,
    pub path: PathBuf,
    pub size: u64,
    /// Segundos Unix de la última modificación.
    pub modified: Option<i64>,
}

// ---------------------------------------------------------------------------
// API sobre `Core`
// ---------------------------------------------------------------------------

impl Core {
    /// Mundos de la instancia, del último jugado al más antiguo.
    pub async fn list_worlds(&self, instance_id: &str) -> Result<Vec<WorldInfo>> {
        let saves = self.saves_dir(instance_id).await?;
        blocking(move || {
            let rd = match fs::read_dir(&saves) {
                Ok(rd) => rd,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(Error::io(&saves, e)),
            };
            let mut out = Vec::new();
            for entry in rd {
                let entry = entry.map_err(|e| Error::io(&saves, e))?;
                let Some(folder) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let dir = entry.path();
                if dir.is_dir() && dir.join(LEVEL_DAT).is_file() {
                    out.push(world_info(&folder, &dir));
                }
            }
            out.sort_by(|a, b| {
                b.last_played_ms
                    .cmp(&a.last_played_ms)
                    .then_with(|| a.folder.cmp(&b.folder))
            });
            Ok(out)
        })
        .await
    }

    /// Cambia `Data.LevelName` (la carpeta no se renombra).
    pub async fn rename_world(
        &self,
        instance_id: &str,
        folder: &str,
        new_name: &str,
    ) -> Result<WorldInfo> {
        let new_name = new_name.trim().to_owned();
        if new_name.is_empty() {
            return Err(Error::InvalidInput(
                "el nombre del mundo no puede estar vacío".into(),
            ));
        }
        self.modify_level(instance_id, folder, move |data| {
            data.insert("LevelName".into(), Value::String(new_name));
            Ok(())
        })
        .await
    }

    /// Modo de juego, dificultad, hardcore y trucos del mundo.
    pub async fn update_world(
        &self,
        instance_id: &str,
        folder: &str,
        patch: WorldPatch,
    ) -> Result<WorldInfo> {
        self.modify_level(instance_id, folder, move |data| {
            apply_patch(data, &patch);
            Ok(())
        })
        .await
    }

    /// Borra el mundo para siempre (no pasa por la papelera).
    pub async fn delete_world(&self, instance_id: &str, folder: &str) -> Result<()> {
        let dir = self.world_dir(instance_id, folder).await?;
        self.ensure_instance_stopped(instance_id).await?;
        let meta = tokio::fs::symlink_metadata(&dir)
            .await
            .map_err(|e| Error::io(&dir, e))?;
        if meta.file_type().is_symlink() {
            tokio::fs::remove_file(&dir)
                .await
                .map_err(|e| Error::io(&dir, e))
        } else {
            tokio::fs::remove_dir_all(&dir)
                .await
                .map_err(|e| Error::io(&dir, e))
        }
    }

    /// Datapacks activos y desactivados del mundo, ordenados por nombre.
    pub async fn list_datapacks(
        &self,
        instance_id: &str,
        world: &str,
    ) -> Result<Vec<DatapackInfo>> {
        let packs = self.world_dir(instance_id, world).await?.join("datapacks");
        blocking(move || {
            let mut out = list_packs(&packs, true)?;
            out.extend(list_packs(&packs.join(DISABLED_DIR), false)?);
            out.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then(b.enabled.cmp(&a.enabled))
            });
            Ok(out)
        })
        .await
    }

    /// Activa o desactiva un datapack moviéndolo desde/hacia `datapacks/.disabled/`.
    pub async fn set_datapack_enabled(
        &self,
        instance_id: &str,
        world: &str,
        name: &str,
        enabled: bool,
    ) -> Result<()> {
        let packs = self.world_dir(instance_id, world).await?.join("datapacks");
        self.ensure_instance_stopped(instance_id).await?;
        let name = single_segment(name)?.to_owned();
        if name.starts_with('.') {
            return Err(Error::InvalidPath(format!("nombre no válido: {name:?}")));
        }
        blocking(move || {
            let disabled = packs.join(DISABLED_DIR);
            let (from, to) = if enabled {
                (disabled.join(&name), packs.join(&name))
            } else {
                (packs.join(&name), disabled.join(&name))
            };
            if exists(&to) && !exists(&from) {
                return Ok(()); // ya estaba en el estado pedido
            }
            if !exists(&from) {
                return Err(no_datapack(&name));
            }
            if exists(&to) {
                return Err(Error::InvalidInput(format!(
                    "ya existe un datapack llamado {name} en el destino"
                )));
            }
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            fs::rename(&from, &to).map_err(|e| Error::io(&from, e))
        })
        .await
    }

    /// Borra un datapack (activo o desactivado).
    pub async fn remove_datapack(&self, instance_id: &str, world: &str, name: &str) -> Result<()> {
        let packs = self.world_dir(instance_id, world).await?.join("datapacks");
        self.ensure_instance_stopped(instance_id).await?;
        let name = single_segment(name)?.to_owned();
        if name.starts_with('.') {
            return Err(Error::InvalidPath(format!("nombre no válido: {name:?}")));
        }
        blocking(move || {
            for dir in [packs.clone(), packs.join(DISABLED_DIR)] {
                let path = dir.join(&name);
                match fs::symlink_metadata(&path) {
                    Ok(m) if m.is_dir() => {
                        return fs::remove_dir_all(&path).map_err(|e| Error::io(&path, e));
                    }
                    Ok(_) => return fs::remove_file(&path).map_err(|e| Error::io(&path, e)),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => return Err(Error::io(&path, e)),
                }
            }
            Err(no_datapack(&name))
        })
        .await
    }

    /// Capturas de `screenshots/`, las más nuevas primero.
    pub async fn list_screenshots(&self, instance_id: &str) -> Result<Vec<ScreenshotInfo>> {
        let dir = self.screenshots_dir(instance_id).await?;
        blocking(move || {
            let rd = match fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(Error::io(&dir, e)),
            };
            let mut out = Vec::new();
            for entry in rd {
                let entry = entry.map_err(|e| Error::io(&dir, e))?;
                let Some(file) = entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if !is_image(&file) {
                    continue;
                }
                let Ok(meta) = fs::metadata(entry.path()) else {
                    continue;
                };
                if !meta.is_file() {
                    continue;
                }
                out.push(ScreenshotInfo {
                    file,
                    path: entry.path(),
                    size: meta.len(),
                    modified: epoch_secs(&meta),
                });
            }
            out.sort_by(|a, b| {
                b.modified
                    .cmp(&a.modified)
                    .then_with(|| b.file.cmp(&a.file))
            });
            Ok(out)
        })
        .await
    }

    pub async fn delete_screenshot(&self, instance_id: &str, file: &str) -> Result<()> {
        let dir = self.screenshots_dir(instance_id).await?;
        let path = dir.join(single_segment(file)?);
        if !is_image(file) {
            return Err(Error::InvalidInput(format!("{file} no es una captura")));
        }
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Err(Error::InvalidInput(format!("no existe la captura {file}")))
            }
            Err(e) => Err(Error::io(&path, e)),
        }
    }

    // -- rutas -------------------------------------------------------------

    async fn saves_dir(&self, instance_id: &str) -> Result<PathBuf> {
        let instance = self.get_instance(instance_id).await?;
        Ok(self.instance_game_dir(&instance.id)?.join("saves"))
    }

    async fn screenshots_dir(&self, instance_id: &str) -> Result<PathBuf> {
        let instance = self.get_instance(instance_id).await?;
        Ok(self.instance_game_dir(&instance.id)?.join("screenshots"))
    }

    async fn world_dir(&self, instance_id: &str, folder: &str) -> Result<PathBuf> {
        let dir = self
            .saves_dir(instance_id)
            .await?
            .join(single_segment(folder)?);
        if !tokio::fs::metadata(&dir)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
        {
            return Err(Error::InvalidInput(format!("no existe el mundo {folder}")));
        }
        Ok(dir)
    }

    /// Lee `level.dat`, aplica `f` sobre `Data` y lo guarda (con copia en `level.dat_old`).
    async fn modify_level(
        &self,
        instance_id: &str,
        folder: &str,
        f: impl FnOnce(&mut HashMap<String, Value>) -> Result<()> + Send + 'static,
    ) -> Result<WorldInfo> {
        let dir = self.world_dir(instance_id, folder).await?;
        self.ensure_instance_stopped(instance_id).await?;
        let folder = folder.to_owned();
        blocking(move || {
            let path = dir.join(LEVEL_DAT);
            if !path.is_file() {
                return Err(Error::InvalidInput(format!(
                    "{folder} no es un mundo (falta level.dat)"
                )));
            }
            let mut root = read_level(&path)?;
            f(data_mut(&mut root, &path)?)?;
            write_level(&path, &root)?;
            Ok(world_info(&folder, &dir))
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// NBT
// ---------------------------------------------------------------------------

/// Lee un NBT con gzip (o sin comprimir, si no empieza por la firma gzip).
pub fn read_level(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
    let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        GzDecoder::new(bytes.as_slice())
            .take(MAX_LEVEL_BYTES)
            .read_to_end(&mut out)
            .map_err(|e| Error::io(path, e))?;
        out
    } else {
        bytes
    };
    fastnbt::from_bytes::<Value>(&raw).map_err(|e| bad_level(path, e))
}

/// Escribe el NBT con gzip: antes copia el actual a `level.dat_old` y después
/// escribe un temporal y lo renombra.
pub fn write_level(path: &Path, value: &Value) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;
    let raw = fastnbt::to_bytes(value).map_err(|e| bad_level(path, e))?;
    if path.exists() {
        let old = dir.join(format!("{LEVEL_DAT}_old"));
        fs::copy(path, &old).map_err(|e| Error::io(&old, e))?;
    }
    let tmp = dir.join(format!(".{LEVEL_DAT}.{:08x}.tmp", fastrand::u32(..)));
    let result = (|| {
        let file = fs::File::create(&tmp).map_err(|e| Error::io(&tmp, e))?;
        let mut gz = GzEncoder::new(file, Compression::default());
        gz.write_all(&raw).map_err(|e| Error::io(&tmp, e))?;
        let file = gz.finish().map_err(|e| Error::io(&tmp, e))?;
        file.sync_all().map_err(|e| Error::io(&tmp, e))?;
        drop(file);
        fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn bad_level(path: &Path, e: impl std::fmt::Display) -> Error {
    Error::InvalidInput(format!("{}: level.dat no válido: {e}", path.display()))
}

fn data_mut<'a>(root: &'a mut Value, path: &Path) -> Result<&'a mut HashMap<String, Value>> {
    match root {
        Value::Compound(top) => match top.get_mut("Data") {
            Some(Value::Compound(data)) => Ok(data),
            _ => Err(bad_level(path, "no hay compuesto Data")),
        },
        _ => Err(bad_level(path, "la raíz no es un compuesto")),
    }
}

fn data_ref(root: &Value) -> Option<&HashMap<String, Value>> {
    match root {
        Value::Compound(top) => match top.get("Data") {
            Some(Value::Compound(data)) => Some(data),
            _ => None,
        },
        _ => None,
    }
}

fn compound<'a>(map: &'a HashMap<String, Value>, key: &str) -> Option<&'a HashMap<String, Value>> {
    match map.get(key) {
        Some(Value::Compound(c)) => Some(c),
        _ => None,
    }
}

fn int_of(v: &Value) -> Option<i64> {
    match v {
        Value::Byte(_) | Value::Short(_) | Value::Int(_) | Value::Long(_) => v.as_i64(),
        _ => None,
    }
}

fn bool_of(v: &Value) -> Option<bool> {
    int_of(v).map(|n| n != 0)
}

fn mode_of(v: &Value) -> Option<GameMode> {
    match v {
        Value::String(s) => GameMode::parse(s),
        other => int_of(other).and_then(GameMode::from_id),
    }
}

fn difficulty_of(v: &Value) -> Option<Difficulty> {
    match v {
        Value::String(s) => Difficulty::parse(s),
        other => int_of(other).and_then(Difficulty::from_id),
    }
}

/// Campos que ya existen conservan su tipo (número o texto); los nuevos usan el formato clásico.
fn apply_patch(data: &mut HashMap<String, Value>, patch: &WorldPatch) {
    if let Some(mode) = patch.game_mode {
        let value = match data.get("GameType") {
            Some(Value::String(_)) => Value::String(mode.as_str().into()),
            _ => Value::Int(mode.id()),
        };
        data.insert("GameType".into(), value);
    }
    if let Some(difficulty) = patch.difficulty {
        let typed = |old: Option<&Value>| match old {
            Some(Value::String(_)) => Value::String(difficulty.as_str().into()),
            _ => Value::Byte(difficulty.id()),
        };
        let mut done = false;
        if let Some(Value::Compound(settings)) = data.get_mut("difficulty_settings") {
            let value = match settings.get("difficulty") {
                Some(Value::String(_)) | None => Value::String(difficulty.as_str().into()),
                old => typed(old),
            };
            settings.insert("difficulty".into(), value);
            done = true;
        }
        if !done || data.contains_key("Difficulty") {
            let value = typed(data.get("Difficulty"));
            data.insert("Difficulty".into(), value);
        }
    }
    if let Some(hardcore) = patch.hardcore {
        let flag = Value::Byte(i8::from(hardcore));
        let mut done = false;
        if let Some(Value::Compound(settings)) = data.get_mut("difficulty_settings")
            && settings.contains_key("hardcore")
        {
            settings.insert("hardcore".into(), flag.clone());
            done = true;
        }
        if !done || data.contains_key("hardcore") {
            data.insert("hardcore".into(), flag);
        }
    }
    if let Some(cheats) = patch.allow_commands {
        data.insert("allowCommands".into(), Value::Byte(i8::from(cheats)));
    }
}

fn world_info(folder: &str, dir: &Path) -> WorldInfo {
    let icon = dir.join("icon.png");
    let mut info = WorldInfo {
        folder: folder.to_owned(),
        name: folder.to_owned(),
        game_mode: None,
        difficulty: None,
        hardcore: None,
        cheats: None,
        last_played_ms: None,
        version_name: None,
        data_version: None,
        size_bytes: dir_size(dir),
        icon: icon.is_file().then_some(icon),
        error: None,
    };
    let root = match read_level(&dir.join(LEVEL_DAT)) {
        Ok(root) => root,
        Err(e) => {
            info.error = Some(e.to_string());
            return info;
        }
    };
    let Some(data) = data_ref(&root) else {
        info.error = Some("level.dat no tiene el compuesto Data".into());
        return info;
    };
    if let Some(Value::String(name)) = data.get("LevelName") {
        info.name = name.clone();
    }
    info.game_mode = data.get("GameType").and_then(mode_of);
    let settings = compound(data, "difficulty_settings");
    info.difficulty = settings
        .and_then(|s| s.get("difficulty"))
        .or_else(|| data.get("Difficulty"))
        .and_then(difficulty_of);
    info.hardcore = settings
        .and_then(|s| s.get("hardcore"))
        .or_else(|| data.get("hardcore"))
        .and_then(bool_of);
    info.cheats = data.get("allowCommands").and_then(bool_of);
    info.last_played_ms = data.get("LastPlayed").and_then(int_of);
    info.data_version = data
        .get("DataVersion")
        .and_then(int_of)
        .and_then(|n| i32::try_from(n).ok());
    info.version_name = compound(data, "Version")
        .and_then(|v| v.get("Name"))
        .and_then(|n| match n {
            Value::String(s) => Some(s.clone()),
            _ => None,
        });
    info
}

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::io(".", io::Error::other(format!("tarea interrumpida: {e}"))))?
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn no_datapack(name: &str) -> Error {
    Error::InvalidInput(format!("no existe el datapack {name}"))
}

fn epoch_secs(meta: &fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

fn is_image(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg"].iter().any(|e| lower.ends_with(e))
}

/// Tamaño total de un directorio sin seguir enlaces simbólicos.
fn dir_size(dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) if m.is_file() => m.len(),
            _ => 0,
        })
        .sum()
}

fn list_packs(dir: &Path, enabled: bool) -> Result<Vec<DatapackInfo>> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut out = Vec::new();
    for entry in rd {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::metadata(entry.path()) else {
            continue;
        };
        let is_dir = meta.is_dir();
        if !is_dir && !(meta.is_file() && name.to_ascii_lowercase().ends_with(".zip")) {
            continue;
        }
        out.push(DatapackInfo {
            size_bytes: if is_dir {
                dir_size(&entry.path())
            } else {
                meta.len()
            },
            name,
            is_dir,
            enabled,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Pruebas
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use crate::instance::{GameVersion, LoaderKind, NewInstance};
    use crate::launch::GameInfo;
    use crate::secrets::MemoryStore;
    use fastnbt::{ByteArray, IntArray, LongArray, nbt};
    use std::sync::Arc;

    async fn core(dir: &Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap()
    }

    async fn instance(core: &Core) -> String {
        core.create_instance(NewInstance {
            name: "Pruebas".into(),
            game: GameVersion {
                mc: "1.21".into(),
                loader: LoaderKind::Vanilla,
                loader_version: None,
            },
        })
        .await
        .unwrap()
        .id
    }

    /// `level.dat` clásico (1.16 – 1.21) con etiquetas que no se deben perder.
    fn classic_level(name: &str, last_played: i64) -> Value {
        nbt!({
            "Data": {
                "LevelName": name,
                "GameType": 0i32,
                "Difficulty": 2i8,
                "hardcore": 0i8,
                "allowCommands": 0i8,
                "LastPlayed": last_played,
                "DataVersion": 3953i32,
                "Version": { "Name": "1.21", "Id": 3953i32, "Snapshot": 0i8 },
                "SpawnX": 10i32,
                "RandomSeed": 123456789i64,
                "BorderSize": 59999968.0f64,
                "DayTime": 6000i64,
                "WanderingTraderSpawnChance": 25.5f32,
                "GameRules": { "keepInventory": "true", "doFireTick": "false" },
                "ServerBrands": ["vanilla", "fabric"],
                "Player": {
                    "Pos": [1.5f64, 64.0f64, -3.25f64],
                    "Inventory": [
                        { "id": "minecraft:stone", "Count": 3i8, "Slot": 0i8 },
                        { "id": "minecraft:dirt", "Count": 1i8, "Slot": 1i8 }
                    ],
                    "UUID": IntArray::new(vec![1, 2, 3, 4])
                },
                "DataPacks": { "Enabled": ["vanilla"], "Disabled": [] },
                "Blob": ByteArray::new(vec![1, -2, 3]),
                "Longs": LongArray::new(vec![9_000_000_000i64, -1])
            },
            "Extra": "fuera de Data"
        })
    }

    fn put_level(dir: &Path, root: &Value) {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(LEVEL_DAT);
        // Sin copia previa que ensucie el directorio.
        let raw = fastnbt::to_bytes(root).unwrap();
        let mut gz = GzEncoder::new(fs::File::create(&path).unwrap(), Compression::default());
        gz.write_all(&raw).unwrap();
        gz.finish().unwrap();
    }

    #[tokio::test]
    async fn lista_mundos_ordenados_y_tolerantes() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let saves = core.instance_game_dir(&id).unwrap().join("saves");
        put_level(&saves.join("viejo"), &classic_level("Mundo viejo", 1_000));
        put_level(&saves.join("nuevo"), &classic_level("Mundo nuevo", 9_000));
        fs::write(saves.join("nuevo/icon.png"), "png").unwrap();
        fs::write(saves.join("nuevo/region.mca"), vec![0u8; 100]).unwrap();
        // Formato moderno: dificultad en `difficulty_settings`, modo como texto, sin Version.
        put_level(
            &saves.join("moderno"),
            &nbt!({ "Data": {
                "LevelName": "Moderno",
                "GameType": "creative",
                "difficulty_settings": { "difficulty": "hard", "hardcore": 1i8, "locked": 0i8 },
                "allowCommands": 1i8,
                "LastPlayed": 5_000i64
            }}),
        );
        // Mínimo: casi todo falta.
        put_level(&saves.join("vacio"), &nbt!({ "Data": {} }));
        // Roto y sin level.dat.
        fs::create_dir_all(saves.join("roto")).unwrap();
        fs::write(saves.join("roto/level.dat"), "basura").unwrap();
        fs::create_dir_all(saves.join("no-es-mundo")).unwrap();

        let list = core.list_worlds(&id).await.unwrap();
        let folders: Vec<_> = list.iter().map(|w| w.folder.as_str()).collect();
        assert_eq!(folders, ["nuevo", "moderno", "viejo", "roto", "vacio"]);
        let nuevo = &list[0];
        assert_eq!(nuevo.name, "Mundo nuevo");
        assert_eq!(nuevo.game_mode, Some(GameMode::Survival));
        assert_eq!(nuevo.difficulty, Some(Difficulty::Normal));
        assert_eq!(nuevo.hardcore, Some(false));
        assert_eq!(nuevo.cheats, Some(false));
        assert_eq!(nuevo.last_played_ms, Some(9_000));
        assert_eq!(nuevo.version_name.as_deref(), Some("1.21"));
        assert_eq!(nuevo.data_version, Some(3953));
        assert!(nuevo.size_bytes >= 100 + 3);
        assert!(nuevo.icon.as_ref().is_some_and(|p| p.ends_with("icon.png")));
        let moderno = &list[1];
        assert_eq!(moderno.game_mode, Some(GameMode::Creative));
        assert_eq!(moderno.difficulty, Some(Difficulty::Hard));
        assert_eq!(moderno.hardcore, Some(true));
        assert_eq!(moderno.cheats, Some(true));
        assert_eq!(moderno.version_name, None);
        let roto = &list[3];
        assert!(roto.error.is_some() && roto.name == "roto");
        let vacio = &list[4];
        assert_eq!(vacio.name, "vacio");
        assert!(vacio.game_mode.is_none() && vacio.difficulty.is_none() && vacio.error.is_none());
    }

    #[tokio::test]
    async fn renombrar_y_modificar_conserva_etiquetas() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let dir = core.instance_game_dir(&id).unwrap().join("saves/m");
        let original = classic_level("Original", 42);
        put_level(&dir, &original);

        let info = core
            .rename_world(&id, "m", "  Nuevo nombre ")
            .await
            .unwrap();
        assert_eq!(info.name, "Nuevo nombre");
        assert_eq!(info.folder, "m");
        assert!(dir.join("level.dat_old").is_file());
        // Solo cambió LevelName.
        let mut expected = original.clone();
        if let Value::Compound(top) = &mut expected
            && let Some(Value::Compound(data)) = top.get_mut("Data")
        {
            data.insert("LevelName".into(), Value::String("Nuevo nombre".into()));
        }
        assert_eq!(read_level(&dir.join(LEVEL_DAT)).unwrap(), expected);
        // `level.dat_old` es el anterior, legible.
        assert_eq!(read_level(&dir.join("level.dat_old")).unwrap(), original);

        let info = core
            .update_world(
                &id,
                "m",
                WorldPatch {
                    game_mode: Some(GameMode::Creative),
                    difficulty: Some(Difficulty::Hard),
                    hardcore: Some(true),
                    allow_commands: Some(true),
                },
            )
            .await
            .unwrap();
        assert_eq!(info.game_mode, Some(GameMode::Creative));
        assert_eq!(info.difficulty, Some(Difficulty::Hard));
        assert_eq!(info.hardcore, Some(true));
        assert_eq!(info.cheats, Some(true));
        let after = read_level(&dir.join(LEVEL_DAT)).unwrap();
        let (Value::Compound(top), Value::Compound(exp_top)) = (&after, &expected) else {
            panic!()
        };
        assert_eq!(top.get("Extra"), exp_top.get("Extra"));
        let (Some(Value::Compound(d)), Some(Value::Compound(e))) =
            (top.get("Data"), exp_top.get("Data"))
        else {
            panic!()
        };
        // Tipos clásicos: GameType int, Difficulty/hardcore/allowCommands byte.
        assert_eq!(d.get("GameType"), Some(&Value::Int(1)));
        assert_eq!(d.get("Difficulty"), Some(&Value::Byte(3)));
        assert_eq!(d.get("hardcore"), Some(&Value::Byte(1)));
        assert_eq!(d.get("allowCommands"), Some(&Value::Byte(1)));
        // Todo lo demás, intacto (tags desconocidos incluidos).
        for key in e.keys() {
            if [
                "GameType",
                "Difficulty",
                "hardcore",
                "allowCommands",
                "LevelName",
            ]
            .contains(&key.as_str())
            {
                continue;
            }
            assert_eq!(d.get(key), e.get(key), "cambió {key}");
        }
        assert_eq!(d.len(), e.len());
        // Ningún temporal suelto.
        let names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|x| x.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");

        assert!(core.rename_world(&id, "m", "   ").await.is_err());
        fs::create_dir_all(dir.parent().unwrap().join("sin-level")).unwrap();
        let e = core.rename_world(&id, "sin-level", "x").await.unwrap_err();
        assert!(e.to_string().contains("no es un mundo"), "{e}");
        assert!(core.rename_world(&id, "../m", "x").await.is_err());
        assert!(core.rename_world(&id, "no-existe", "x").await.is_err());
    }

    #[tokio::test]
    async fn parche_respeta_formato_moderno() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let dir = core.instance_game_dir(&id).unwrap().join("saves/mod");
        put_level(
            &dir,
            &nbt!({ "Data": {
                "LevelName": "M",
                "GameType": "survival",
                "difficulty_settings": { "difficulty": "easy", "hardcore": 0i8, "locked": 1i8 }
            }}),
        );
        let info = core
            .update_world(
                &id,
                "mod",
                WorldPatch {
                    game_mode: Some(GameMode::Spectator),
                    difficulty: Some(Difficulty::Peaceful),
                    hardcore: Some(true),
                    allow_commands: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(info.game_mode, Some(GameMode::Spectator));
        assert_eq!(info.difficulty, Some(Difficulty::Peaceful));
        assert_eq!(info.hardcore, Some(true));
        let root = read_level(&dir.join(LEVEL_DAT)).unwrap();
        let data = data_ref(&root).unwrap();
        assert_eq!(
            data.get("GameType"),
            Some(&Value::String("spectator".into()))
        );
        assert!(!data.contains_key("Difficulty") && !data.contains_key("hardcore"));
        let settings = compound(data, "difficulty_settings").unwrap();
        assert_eq!(
            settings.get("difficulty"),
            Some(&Value::String("peaceful".into()))
        );
        assert_eq!(settings.get("hardcore"), Some(&Value::Byte(1)));
        assert_eq!(settings.get("locked"), Some(&Value::Byte(1)));
    }

    #[tokio::test]
    async fn no_modifica_con_el_juego_en_marcha_y_borra_con_permiso() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let saves = core.instance_game_dir(&id).unwrap().join("saves");
        put_level(&saves.join("m"), &classic_level("M", 1));
        fs::create_dir_all(saves.join("m/datapacks")).unwrap();
        fs::write(saves.join("m/datapacks/a.zip"), "zip").unwrap();
        core.inner.games.lock().await.insert(
            1,
            GameInfo {
                game_id: 1,
                instance_id: id.clone(),
                pid: None,
                started_at: 0,
            },
        );
        let before = fs::read(saves.join("m/level.dat")).unwrap();
        assert!(core.rename_world(&id, "m", "X").await.is_err());
        assert!(
            core.update_world(&id, "m", WorldPatch::default())
                .await
                .is_err()
        );
        assert!(core.delete_world(&id, "m").await.is_err());
        assert!(
            core.set_datapack_enabled(&id, "m", "a.zip", false)
                .await
                .is_err()
        );
        assert!(core.remove_datapack(&id, "m", "a.zip").await.is_err());
        assert_eq!(fs::read(saves.join("m/level.dat")).unwrap(), before);
        assert!(saves.join("m/datapacks/a.zip").exists());
        // Listar sí se puede.
        assert_eq!(core.list_worlds(&id).await.unwrap().len(), 1);

        core.inner.games.lock().await.clear();
        core.delete_world(&id, "m").await.unwrap();
        assert!(!saves.join("m").exists());
        assert!(core.list_worlds(&id).await.unwrap().is_empty());
        assert!(core.delete_world(&id, "m").await.is_err());
    }

    #[tokio::test]
    async fn datapacks_activar_desactivar_borrar() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let world = core.instance_game_dir(&id).unwrap().join("saves/m");
        put_level(&world, &classic_level("M", 1));
        let packs = world.join("datapacks");
        fs::create_dir_all(packs.join("carpeta/data")).unwrap();
        fs::write(packs.join("carpeta/pack.mcmeta"), "{}").unwrap();
        fs::write(packs.join("a.zip"), "12345").unwrap();
        fs::write(packs.join("notas.txt"), "no es un pack").unwrap();
        fs::create_dir_all(packs.join(".disabled")).unwrap();
        fs::write(packs.join(".disabled/b.zip"), "1").unwrap();

        let list = core.list_datapacks(&id, "m").await.unwrap();
        let view: Vec<_> = list
            .iter()
            .map(|p| (p.name.as_str(), p.enabled, p.is_dir))
            .collect();
        assert_eq!(
            view,
            [
                ("a.zip", true, false),
                ("b.zip", false, false),
                ("carpeta", true, true)
            ]
        );
        assert_eq!(list[0].size_bytes, 5);

        core.set_datapack_enabled(&id, "m", "a.zip", false)
            .await
            .unwrap();
        core.set_datapack_enabled(&id, "m", "b.zip", true)
            .await
            .unwrap();
        core.set_datapack_enabled(&id, "m", "carpeta", false)
            .await
            .unwrap();
        // Repetir no falla.
        core.set_datapack_enabled(&id, "m", "b.zip", true)
            .await
            .unwrap();
        assert!(packs.join(".disabled/a.zip").exists());
        assert!(packs.join("b.zip").exists());
        assert!(packs.join(".disabled/carpeta/pack.mcmeta").exists());
        assert!(
            core.set_datapack_enabled(&id, "m", "fantasma.zip", true)
                .await
                .is_err()
        );
        assert!(
            core.set_datapack_enabled(&id, "m", "../level.dat", true)
                .await
                .is_err()
        );
        assert!(
            core.set_datapack_enabled(&id, "m", ".disabled", true)
                .await
                .is_err()
        );

        core.remove_datapack(&id, "m", "a.zip").await.unwrap(); // desactivado
        core.remove_datapack(&id, "m", "b.zip").await.unwrap(); // activo
        core.remove_datapack(&id, "m", "carpeta").await.unwrap(); // carpeta desactivada
        assert!(core.list_datapacks(&id, "m").await.unwrap().is_empty());
        assert!(core.remove_datapack(&id, "m", "a.zip").await.is_err());
        assert!(core.list_datapacks(&id, "nada").await.is_err());
    }

    #[tokio::test]
    async fn capturas_ordenadas_y_borrado() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        assert!(core.list_screenshots(&id).await.unwrap().is_empty());
        let dir = core.instance_game_dir(&id).unwrap().join("screenshots");
        fs::create_dir_all(&dir).unwrap();
        for (name, secs) in [
            ("a.png", 1_000_000u64),
            ("b.png", 3_000_000),
            ("c.jpg", 2_000_000),
        ] {
            let path = dir.join(name);
            fs::write(&path, "img").unwrap();
            let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(time)
                .unwrap();
        }
        fs::write(dir.join("notas.txt"), "no").unwrap();
        let list = core.list_screenshots(&id).await.unwrap();
        let names: Vec<_> = list.iter().map(|s| s.file.as_str()).collect();
        assert_eq!(names, ["b.png", "c.jpg", "a.png"]);
        assert_eq!(list[0].size, 3);
        assert_eq!(list[0].modified, Some(3_000_000));
        assert!(list[0].path.ends_with("screenshots/b.png"));

        core.delete_screenshot(&id, "b.png").await.unwrap();
        assert!(!dir.join("b.png").exists());
        assert!(core.delete_screenshot(&id, "b.png").await.is_err());
        assert!(core.delete_screenshot(&id, "../x.png").await.is_err());
        assert!(core.delete_screenshot(&id, "notas.txt").await.is_err());
        assert!(dir.join("notas.txt").exists());
    }

    #[test]
    fn nbt_ida_y_vuelta_directa() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(LEVEL_DAT);
        let root = classic_level("Ida y vuelta", 7);
        write_level(&path, &root).unwrap();
        assert_eq!(read_level(&path).unwrap(), root);
        // Sin comprimir también se lee.
        let plain = tmp.path().join("plano.dat");
        fs::write(&plain, fastnbt::to_bytes(&root).unwrap()).unwrap();
        assert_eq!(read_level(&plain).unwrap(), root);
        assert!(read_level(&tmp.path().join("no-existe")).is_err());
    }
}
