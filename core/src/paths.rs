//! Directorios de datos y layout en disco (docs/arquitectura.md §4).

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

/// Identificador de la app: directorio de datos, servicios del keyring.
/// Nombre de la app: Shard (D2).
pub const APP_ID: &str = "shard";

/// Variable de entorno que fija el directorio de datos.
pub const DATA_DIR_ENV: &str = "SHARD_DATA_DIR";

/// Directorio de datos del usuario en el sistema (`~/.local/share` en Linux).
///
/// Dentro de Flatpak, `XDG_DATA_HOME` apunta a `~/.var/app/<id>/data`; se usa el
/// del sistema para compartir los datos con la versión nativa (el manifiesto da
/// acceso a esas carpetas).
pub fn host_data_dir() -> Option<PathBuf> {
    host_data_dir_with(
        in_flatpak(),
        std::env::var_os("HOST_XDG_DATA_HOME"),
        directories::BaseDirs::new()
            .map(|d| (d.home_dir().to_path_buf(), d.data_dir().to_path_buf())),
    )
}

fn in_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists()
}

fn host_data_dir_with(
    flatpak: bool,
    host_xdg: Option<std::ffi::OsString>,
    dirs: Option<(PathBuf, PathBuf)>,
) -> Option<PathBuf> {
    let (home, data) = dirs?;
    if !flatpak {
        return Some(data);
    }
    match host_xdg.filter(|v| !v.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => Some(home.join(".local/share")),
    }
}

#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `SHARD_DATA_DIR` o el directorio de datos del SO.
    pub fn default_root() -> Result<PathBuf> {
        if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
        host_data_dir()
            .map(|d| d.join(APP_ID))
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
    fn datos_del_sistema_dentro_de_flatpak() {
        let dirs = Some((
            PathBuf::from("/home/u"),
            PathBuf::from("/home/u/.var/app/dev.shard.Shard/data"),
        ));
        assert_eq!(
            host_data_dir_with(true, None, dirs.clone()),
            Some(PathBuf::from("/home/u/.local/share"))
        );
        assert_eq!(
            host_data_dir_with(true, Some("/datos".into()), dirs.clone()),
            Some(PathBuf::from("/datos"))
        );
        assert_eq!(
            host_data_dir_with(false, None, dirs),
            Some(PathBuf::from("/home/u/.var/app/dev.shard.Shard/data"))
        );
    }

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
