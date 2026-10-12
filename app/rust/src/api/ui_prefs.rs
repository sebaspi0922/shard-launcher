//! Preferencias propias de la interfaz (apariencia, primer arranque…) en
//! `<datos>/ui.json`. Son de la app, no del núcleo: no tocan `settings.json`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

const UI_FILE: &str = "ui.json";
const BACKGROUNDS_DIR: &str = "backgrounds";
/// Tamaño máximo del archivo de preferencias y de una imagen de fondo.
const MAX_PREFS_BYTES: u64 = 256 * 1024;
const MAX_BACKGROUND_BYTES: u64 = 40 * 1024 * 1024;
const BACKGROUND_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

fn io_err(path: &Path, e: std::io::Error) -> BridgeError {
    BridgeError {
        kind: "io".into(),
        message: format!("{}: {e}", path.display()),
    }
}

fn invalid(message: String) -> BridgeError {
    BridgeError {
        kind: "invalid_input".into(),
        message,
    }
}

/// Escribe con un temporal y `rename` para no dejar el archivo a medias.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), BridgeError> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes).map_err(|e| io_err(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io_err(path, e))
}

fn prefs_path() -> Result<PathBuf, BridgeError> {
    Ok(core()?.paths().root().join(UI_FILE))
}

/// Contenido de `ui.json` (un objeto JSON); `{}` si no existe o no se puede leer.
#[flutter_rust_bridge::frb(sync)]
pub fn get_ui_prefs() -> String {
    let Ok(path) = prefs_path() else {
        return "{}".into();
    };
    let ok = fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_PREFS_BYTES);
    if !ok {
        return "{}".into();
    }
    match fs::read_to_string(&path) {
        Ok(text) if serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&text).is_ok() => text,
        _ => "{}".into(),
    }
}

/// Guarda `ui.json`. Debe ser un objeto JSON.
pub async fn set_ui_prefs(json: String) -> Result<(), BridgeError> {
    let path = prefs_path()?;
    on_rt(async move {
        let value: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&json)
            .map_err(|e| invalid(format!("preferencias no válidas: {e}")))?;
        let text = serde_json::to_string_pretty(&value)
            .map_err(|e| BridgeError::internal(e.to_string()))?;
        if text.len() as u64 > MAX_PREFS_BYTES {
            return Err(invalid("preferencias demasiado grandes".into()));
        }
        tokio::task::spawn_blocking(move || write_atomic(&path, text.as_bytes()))
            .await
            .map_err(|e| BridgeError::internal(format!("tarea interrumpida: {e}")))?
    })
    .await
}

/// Copia una imagen elegida por el usuario a `<datos>/backgrounds/` (borrando
/// el fondo anterior) y devuelve la ruta de la copia.
pub async fn import_background(path: String) -> Result<String, BridgeError> {
    let dir = core()?.paths().root().join(BACKGROUNDS_DIR);
    on_rt(async move {
        tokio::task::spawn_blocking(move || copy_background(Path::new(&path), &dir))
            .await
            .map_err(|e| BridgeError::internal(format!("tarea interrumpida: {e}")))?
    })
    .await
}

fn copy_background(src: &Path, dir: &Path) -> Result<String, BridgeError> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| BACKGROUND_EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| invalid(format!("formato de imagen no admitido: {}", src.display())))?;
    let meta = fs::metadata(src).map_err(|e| io_err(src, e))?;
    if !meta.is_file() || meta.len() > MAX_BACKGROUND_BYTES {
        return Err(invalid(format!("imagen no válida o demasiado grande: {}", src.display())));
    }
    fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest = dir.join(format!("fondo-{stamp}.{ext}"));
    let tmp = dir.join(format!(".fondo-{stamp}.tmp"));
    fs::copy(src, &tmp).map_err(|e| io_err(src, e))?;
    fs::rename(&tmp, &dest).map_err(|e| io_err(&dest, e))?;
    // Solo se conserva el fondo actual.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p != dest && p.is_file() {
                let _ = fs::remove_file(&p);
            }
        }
    }
    Ok(dest.display().to_string())
}
