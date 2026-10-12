//! Arreglos que Doctor puede proponer y aplicar sobre una instancia.

use std::path::Component;

use serde::{Deserialize, Serialize};

use crate::launcher::Core;
use crate::paths::{safe_join, single_segment};
use crate::{Error, Result};

/// Archivos de la raíz del juego que se pueden borrar para que se regeneren.
const REGENERABLE_FILES: [&str; 4] = [
    "options.txt",
    "servers.dat",
    "optionsof.txt",
    "optionsshaders.txt",
];

/// Mínimo de memoria que acepta `SetMemory`.
const MIN_MEMORY_MB: u32 = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FixAction {
    /// Renombra `mods/<file_name>` a `<file_name>.disabled`.
    DisableMod {
        file_name: String,
    },
    SetMemory {
        mb: u32,
    },
    InstallJava {
        major: u32,
    },
    RemoveJvmArg {
        arg: String,
    },
    ClearJvmArgs,
    /// Borra un archivo regenerable del directorio del juego.
    DeleteInstanceFile {
        relative_path: String,
    },
    ReinstallLoader,
    /// Borra `saves/*/session.lock`.
    ClearWorldLocks,
}

impl FixAction {
    /// Descripción corta para mostrar al usuario antes de aplicar el arreglo.
    pub fn describe(&self) -> String {
        match self {
            FixAction::DisableMod { file_name } => format!("Desactivar el mod {file_name}"),
            FixAction::SetMemory { mb } => format!("Asignar {mb} MB de memoria"),
            FixAction::InstallJava { major } => format!("Instalar Java {major}"),
            FixAction::RemoveJvmArg { arg } => format!("Quitar el argumento JVM {arg}"),
            FixAction::ClearJvmArgs => "Vaciar los argumentos JVM personalizados".into(),
            FixAction::DeleteInstanceFile { relative_path } => {
                format!("Borrar {relative_path} (se regenera al iniciar)")
            }
            FixAction::ReinstallLoader => "Reinstalar el loader".into(),
            FixAction::ClearWorldLocks => "Borrar los bloqueos de mundo (session.lock)".into(),
        }
    }
}

/// Comprueba que `relative_path` sea un archivo que Doctor puede borrar y
/// devuelve sus componentes normalizados.
fn allowed_instance_file(relative_path: &str) -> Result<Vec<String>> {
    let comps: Vec<String> = std::path::Path::new(relative_path)
        .components()
        .filter_map(|c| match c {
            Component::Normal(c) => Some(c.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let allowed = match comps.as_slice() {
        [one] => REGENERABLE_FILES.contains(&one.as_str()),
        [first, _, ..] => first == "config",
        [] => false,
    };
    if allowed {
        Ok(comps)
    } else {
        Err(Error::InvalidPath(format!(
            "Doctor no puede borrar {relative_path:?}: solo {} o rutas dentro de config/",
            REGENERABLE_FILES.join(", ")
        )))
    }
}

impl Core {
    /// Aplica un arreglo y devuelve un mensaje para el usuario.
    pub async fn apply_fix(&self, instance_id: &str, fix: &FixAction) -> Result<String> {
        let instance = self.get_instance(instance_id).await?;
        let game_dir = self.instance_game_dir(instance_id)?;
        match fix {
            FixAction::DisableMod { file_name } => {
                let name = single_segment(file_name)?;
                if name.to_ascii_lowercase().ends_with(".disabled") {
                    return Err(Error::InvalidInput(format!("{name} ya está desactivado")));
                }
                let from = game_dir.join("mods").join(name);
                if !tokio::fs::try_exists(&from).await.unwrap_or(false) {
                    return Err(Error::InvalidInput(format!(
                        "el mod {name} no existe en esta instancia"
                    )));
                }
                let to = game_dir.join("mods").join(format!("{name}.disabled"));
                if tokio::fs::try_exists(&to).await.unwrap_or(false) {
                    return Err(Error::InvalidInput(format!("ya existe {name}.disabled")));
                }
                tokio::fs::rename(&from, &to)
                    .await
                    .map_err(|e| Error::io(&from, e))?;
                Ok(format!(
                    "Mod {name} desactivado; puedes reactivarlo quitando «.disabled»."
                ))
            }
            FixAction::SetMemory { mb } => {
                if *mb < MIN_MEMORY_MB {
                    return Err(Error::InvalidInput(format!(
                        "la memoria mínima es {MIN_MEMORY_MB} MB"
                    )));
                }
                let mb = *mb;
                self.update_instance(instance_id, |i| i.java.memory_mb = Some(mb))
                    .await?;
                Ok(format!("Memoria asignada a esta instancia: {mb} MB."))
            }
            FixAction::RemoveJvmArg { arg } => {
                let mut args = match &instance.java.jvm_args {
                    Some(a) => a.clone(),
                    None => self.settings().await.default_jvm_args,
                };
                let before = args.len();
                args.retain(|a| a != arg);
                if args.len() == before {
                    return Err(Error::InvalidInput(format!(
                        "el argumento {arg:?} no está en los argumentos JVM de esta instancia"
                    )));
                }
                // Si venían de los ajustes globales, la instancia pasa a tener su propia lista.
                self.update_instance(instance_id, |i| i.java.jvm_args = Some(args))
                    .await?;
                Ok(format!(
                    "Se quitó {arg:?} de los argumentos JVM de esta instancia."
                ))
            }
            FixAction::ClearJvmArgs => {
                self.update_instance(instance_id, |i| i.java.jvm_args = Some(Vec::new()))
                    .await?;
                Ok("Se vaciaron los argumentos JVM personalizados de esta instancia.".into())
            }
            FixAction::DeleteInstanceFile { relative_path } => {
                let comps = allowed_instance_file(relative_path)?;
                let path = safe_join(&game_dir, &comps.join("/"))?;
                let meta = match tokio::fs::symlink_metadata(&path).await {
                    Ok(m) => m,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Err(Error::InvalidInput(format!("{relative_path} no existe")));
                    }
                    Err(e) => return Err(Error::io(&path, e)),
                };
                if meta.is_dir() {
                    tokio::fs::remove_dir_all(&path).await
                } else {
                    tokio::fs::remove_file(&path).await
                }
                .map_err(|e| Error::io(&path, e))?;
                Ok(format!(
                    "Se borró {relative_path}; el juego lo regenerará al iniciar."
                ))
            }
            FixAction::ClearWorldLocks => {
                let saves = game_dir.join("saves");
                let mut entries = match tokio::fs::read_dir(&saves).await {
                    Ok(e) => e,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        return Ok("No hay mundos; no había bloqueos que borrar.".into());
                    }
                    Err(e) => return Err(Error::io(&saves, e)),
                };
                let mut removed = 0usize;
                while let Some(entry) = entries
                    .next_entry()
                    .await
                    .map_err(|e| Error::io(&saves, e))?
                {
                    let Ok(kind) = entry.file_type().await else {
                        continue;
                    };
                    if !kind.is_dir() {
                        continue;
                    }
                    let lock = entry.path().join("session.lock");
                    match tokio::fs::remove_file(&lock).await {
                        Ok(()) => removed += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(Error::io(&lock, e)),
                    }
                }
                Ok(format!(
                    "Se borraron {removed} archivo(s) de bloqueo de mundo."
                ))
            }
            FixAction::InstallJava { major } => {
                let java = self.ensure_java(*major).await?;
                Ok(format!(
                    "Java {} lista en {}.",
                    java.major,
                    java.path.display()
                ))
            }
            FixAction::ReinstallLoader => {
                let removed = match self.installed_loader_id(&instance.game).await {
                    Some(id) => {
                        let dir = self
                            .paths()
                            .versions_dir()
                            .join(crate::paths::single_segment(&id)?);
                        match tokio::fs::remove_dir_all(&dir).await {
                            Ok(()) => true,
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                            Err(e) => return Err(Error::io(&dir, e)),
                        }
                    }
                    None => false,
                };
                Ok(if removed {
                    format!(
                        "Se borró la instalación del loader de «{}»; se reinstalará al lanzar. Mods, mundos y ajustes no se tocan.",
                        instance.name
                    )
                } else {
                    format!(
                        "No había instalación del loader que borrar para «{}»; se comprobará al lanzar.",
                        instance.name
                    )
                })
            }
        }
    }
}
