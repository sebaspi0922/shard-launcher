//! Directorios de datos y layout en disco (docs/arquitectura.md §4).

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

/// Identificador de la app: directorio de datos, servicios del keyring.
/// Provisional hasta decidir el nombre (D2).
pub const APP_ID: &str = "lcore";

/// Variable de entorno que fija el directorio de datos.
pub const DATA_DIR_ENV: &str = "LCORE_DATA_DIR";

#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `LCORE_DATA_DIR` o el directorio de datos del SO.
    pub fn default_root() -> Result<PathBuf> {
        if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
        directories::ProjectDirs::from("", "", APP_ID)
            .map(|d| d.data_dir().to_path_buf())
            .ok_or_else(|| Error::InvalidPath("no se encontró el directorio de usuario".into()))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.root.join("accounts.json")
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("cache").join("http")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.root.join("meta")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn natives_dir(&self) -> PathBuf {
        self.root.join("natives")
    }

    pub fn java_dir(&self) -> PathBuf {
        self.root.join("java")
    }

    pub fn installers_dir(&self) -> PathBuf {
        self.root.join("installers")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, id: &str) -> Result<PathBuf> {
        Ok(self.instances_dir().join(single_segment(id)?))
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn trash_dir(&self) -> PathBuf {
        self.root.join("trash")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
}

/// Acepta solo un nombre de un segmento (sin separadores, `.` ni `..`).
pub fn single_segment(name: &str) -> Result<&str> {
    let mut comps = Path::new(name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) if !name.contains(['/', '\\']) => Ok(name),
        _ => Err(Error::InvalidPath(format!("nombre no válido: {name:?}"))),
    }
}

/// Une `rel` a `base` rechazando rutas absolutas y `..` (protección zip-slip).
pub fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    let mut out = base.to_path_buf();
    let mut any = false;
    for comp in rel_path.components() {
        match comp {
            Component::Normal(c) => {
                out.push(c);
                any = true;
            }
            Component::CurDir => {}
            _ => return Err(Error::InvalidPath(format!("ruta insegura: {rel:?}"))),
        }
    }
    if !any || rel.contains('\\') {
        return Err(Error::InvalidPath(format!("ruta insegura: {rel:?}")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_segment_rechaza_rutas() {
        assert!(single_segment("mi-instancia").is_ok());
        for bad in ["", ".", "..", "a/b", "/abs", "a\\b"] {
            assert!(single_segment(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn safe_join_rechaza_escapes() {
        let base = Path::new("/base");
        assert_eq!(
            safe_join(base, "mods/a.jar").unwrap(),
            PathBuf::from("/base/mods/a.jar")
        );
        assert_eq!(safe_join(base, "./x").unwrap(), PathBuf::from("/base/x"));
        for bad in ["../x", "a/../../x", "/etc/passwd", "", ".", "a\\..\\b"] {
            assert!(safe_join(base, bad).is_err(), "{bad:?}");
        }
    }
}
