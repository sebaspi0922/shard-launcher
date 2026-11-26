//! Ajustes contra el núcleo y el sistema (sustituye a `api/settings.rs`,
//! `api/java.rs`, `api/ui_prefs.rs` y a los plugins `launch_at_startup` y
//! `file_selector` de Flutter). Cada acción es una tarea de tokio que entrega
//! su resultado con `settings::post`.

use std::fs;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};

use crate::backend::Backend;
use crate::errors::Failure;
use crate::settings::{self, Java, SettingsActions, Values};

/// Extensiones de fondo admitidas y tamaño máximo (como el bridge).
const BACKGROUND_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MAX_BACKGROUND_BYTES: u64 = 64 * 1024 * 1024;
const BACKGROUNDS_DIR: &str = "backgrounds";

#[derive(Clone)]
pub struct CoreSettings {
    pub backend: Backend,
    pub lang: String,
    /// Guardados de uno en uno, siempre con los últimos valores: así una
    /// escritura vieja no puede terminar después de una nueva.
    writer: Arc<tokio::sync::Mutex<()>>,
    pending: Arc<Mutex<Option<(u64, Values)>>>,
}

impl CoreSettings {
    pub fn new(backend: Backend, lang: &str) -> Self {
        Self { backend, lang: lang.to_owned(), writer: Arc::default(), pending: Arc::default() }
    }
}

thread_local! {
    /// En X11 y Wayland el texto copiado vive mientras viva quien lo copió.
    static CLIPBOARD: std::cell::RefCell<Option<arboard::Clipboard>> = const { std::cell::RefCell::new(None) };
}

fn invalid(lang: &str, message: String) -> Failure {
    Failure::new(lang, &lcore::Error::InvalidInput(message))
}

/// Copia una imagen elegida a `<datos>/backgrounds/` (borra el fondo
/// anterior) y devuelve la ruta de la copia (`copy_background` del bridge).
pub fn copy_background(src: &Path, dir: &Path) -> Result<PathBuf, String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| BACKGROUND_EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| format!("formato de imagen no admitido: {}", src.display()))?;
    let meta = fs::metadata(src).map_err(|e| format!("{}: {e}", src.display()))?;
    if !meta.is_file() || meta.len() > MAX_BACKGROUND_BYTES {
        return Err(format!("imagen no válida o demasiado grande: {}", src.display()));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    // Nombre nuevo aunque el reloj no haya avanzado (Windows va a saltos de
    // ~15 ms): así la ruta cambia y la UI vuelve a leer la imagen.
    let (mut n, mut dest) = (0u32, dir.join(format!("fondo-{stamp}.{ext}")));
    while dest.exists() {
        n += 1;
        dest = dir.join(format!("fondo-{stamp}-{n}.{ext}"));
    }
    let tmp = dir.join(format!(".fondo-{stamp}-{n}.tmp"));
    fs::copy(src, &tmp).map_err(|e| format!("{}: {e}", src.display()))?;
    fs::rename(&tmp, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    // Solo se conserva el fondo actual.
    if let Ok(entries) = fs::read_dir(dir) {
        for p in entries.flatten().map(|e| e.path()) {
            if p != dest && p.is_file() {
                let _ = fs::remove_file(&p);
            }
        }
    }
    Ok(dest)
}

/// Autoarranque con la sesión: `.desktop` en `~/.config/autostart` (Linux),
/// registro (Windows) o LaunchAgent (macOS), con `--background`. En una
/// AppImage el ejecutable cambia en cada montaje y en Flatpak la ruta interna
/// no existe fuera: se usa `$APPIMAGE` o `flatpak run <id>`.
fn autostart() -> Result<auto_launch::AutoLaunch, String> {
    let path = match (std::env::var("FLATPAK_ID"), std::env::var("APPIMAGE")) {
        (Ok(id), _) => format!("flatpak run {id}"),
        (_, Ok(appimage)) => appimage,
        _ => std::env::current_exe().map_err(|e| e.to_string())?.display().to_string(),
    };
    // Exec del .desktop y valor del registro de Windows (`ruta args`): una
    // ruta con espacios (C:\Program Files\…) va entre comillas.
    let path = if cfg!(any(target_os = "linux", windows)) && path.contains(' ') && !path.starts_with("flatpak ") {
        format!("\"{path}\"")
    } else {
        path
    };
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Shard")
        .set_app_path(&path)
        .set_args(&["--background"])
        .build()
        .map_err(|e| e.to_string())
}

impl SettingsActions for CoreSettings {
    fn load(&self, req: u64) {
        let core = self.backend.core.clone();
        self.backend.rt.spawn(async move {
            let v = Values::from_core(&core.settings().await);
            settings::post(move |s| s.loaded(req, Ok(v)));
        });
    }

    fn save(&self, values: &Values, seq: u64) {
        *self.pending.lock().expect("ajustes pendientes") = Some((seq, values.clone()));
        let (core, lang, writer, pending) = (self.backend.core.clone(), self.lang.clone(), self.writer.clone(), self.pending.clone());
        self.backend.rt.spawn(async move {
            let _turn = writer.lock().await;
            let Some((seq, v)) = pending.lock().expect("ajustes pendientes").take() else { return };
            let r = core.update_settings(|s| v.apply(s)).await.map(|s| Values::from_core(&s)).map_err(|e| Failure::new(&lang, &e));
            settings::post(move |s| s.saved(seq, r));
        });
    }

    fn list_java(&self) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let r = core.list_java().await.map(|l| l.into_iter().map(Java::from).collect()).map_err(|e| Failure::new(&lang, &e));
            settings::post(move |s| s.java_listed(r));
        });
    }

    fn install_java(&self, major: u32) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let r = core.install_java(major).await.map(Java::from).map_err(|e| Failure::new(&lang, &e));
            settings::post(move |s| s.java_installed(major, r));
        });
    }

    fn autostart_supported(&self) -> bool {
        auto_launch::AutoLaunch::is_support()
    }

    fn load_autostart(&self) {
        self.backend.rt.spawn_blocking(|| {
            let on = autostart().and_then(|a| a.is_enabled().map_err(|e| e.to_string())).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "no se pudo leer el autoarranque");
                false
            });
            settings::post(move |s| s.autostart_loaded(on));
        });
    }

    fn set_autostart(&self, on: bool) {
        let lang = self.lang.clone();
        self.backend.rt.spawn_blocking(move || {
            let r = autostart()
                .and_then(|a| {
                    if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())?;
                    a.is_enabled().map_err(|e| e.to_string())
                })
                .map_err(|e| invalid(&lang, format!("autoarranque: {e}")));
            settings::post(move |s| s.autostart_set(r));
        });
    }

    fn save_prefs(&self, prefs: &crate::prefs::UiPrefs) {
        // En el hilo de la UI: es un archivo pequeño y así las escrituras no
        // se cruzan ni llegan en desorden.
        if let Err(e) = crate::prefs::save(self.backend.core.paths().root(), prefs) {
            tracing::warn!(error = %e, "no se pudo guardar ui.json");
        }
    }

    fn pick_background(&self) {
        let dir = self.backend.core.paths().root().join(BACKGROUNDS_DIR);
        let lang = self.lang.clone();
        let label = crate::i18n::text(&lang, "appearanceImages");
        self.backend.rt.spawn(async move {
            let picked = rfd::AsyncFileDialog::new().add_filter(label, &BACKGROUND_EXTENSIONS).pick_file().await;
            let Some(file) = picked else {
                settings::post(|s| s.background_imported(Ok(None)));
                return;
            };
            let src = file.path().to_owned();
            let r = tokio::task::spawn_blocking(move || copy_background(&src, &dir))
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
                .map(Some)
                .map_err(|m| invalid(&lang, m));
            settings::post(move |s| s.background_imported(r));
        });
    }

    fn image_accent(&self, path: &Path) {
        self.backend.image_accent(path.to_owned());
    }

    fn copy_text(&self, text: &str) -> bool {
        CLIPBOARD.with(|c| {
            let mut c = c.borrow_mut();
            if c.is_none() {
                *c = arboard::Clipboard::new().map_err(|e| tracing::warn!(error = %e, "no hay portapapeles")).ok();
            }
            let Some(cb) = c.as_mut() else { return false };
            cb.set_text(text.to_owned()).map_err(|e| tracing::warn!(error = %e, "no se pudo copiar al portapapeles")).is_ok()
        })
    }

    fn check_update(&self) {
        let b = self.backend.clone();
        crate::backend::post(|c| c.update_checking());
        self.backend.rt.spawn(async move { b.check_update(true).await });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copia_el_fondo_y_borra_el_anterior() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("foto.PNG");
        fs::write(&src, b"x").unwrap();
        let out = dir.path().join("backgrounds");
        let first = copy_background(&src, &out).unwrap();
        let second = copy_background(&src, &out).unwrap();
        assert_ne!(first, second, "cada fondo tiene una ruta nueva");
        assert!(second.exists() && !first.exists(), "solo queda el último fondo");
        assert_eq!(second.extension().unwrap(), "png");
        assert!(copy_background(&dir.path().join("a.gif"), &out).is_err());
    }
}
