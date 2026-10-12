//! Instancias: `instances/<id>/{instance.json, content.json, icon.png, logs/, game/}`
//! (docs/arquitectura.md §4). Al borrar van a `trash/<id>__<epoch>/` durante 30 días.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fsutil::{now_secs, read_json, write_json};
use crate::launcher::Core;
use crate::paths::single_segment;
use crate::{Error, Result};

pub const INSTANCE_SCHEMA: u32 = 1;
pub const TRASH_RETENTION_SECS: i64 = 30 * 24 * 3600;
const TRASH_SEPARATOR: &str = "__";
/// Carpeta, dentro de la instancia en la papelera, con sus backups.
const TRASH_BACKUPS: &str = ".backups";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    LabyMod,
    Custom,
}

impl LoaderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LoaderKind::Vanilla => "vanilla",
            LoaderKind::Fabric => "fabric",
            LoaderKind::Quilt => "quilt",
            LoaderKind::Forge => "forge",
            LoaderKind::NeoForge => "neoforge",
            LoaderKind::LabyMod => "labymod",
            LoaderKind::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "vanilla" => LoaderKind::Vanilla,
            "fabric" => LoaderKind::Fabric,
            "quilt" => LoaderKind::Quilt,
            "forge" => LoaderKind::Forge,
            "neoforge" => LoaderKind::NeoForge,
            "labymod" => LoaderKind::LabyMod,
            "custom" => LoaderKind::Custom,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameVersion {
    pub mc: String,
    pub loader: LoaderKind,
    #[serde(default)]
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JavaSettings {
    /// Java concreta para esta instancia; `None` = automática.
    pub path: Option<PathBuf>,
    /// `None` = valor global.
    pub memory_mb: Option<u32>,
    /// `None` = argumentos globales; `Some(vec![])` = ninguno.
    pub jvm_args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSettings {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupSettings {
    pub auto: bool,
    pub interval_hours: Option<u32>,
    pub max_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedFrom {
    /// `roxy`, `dot-minecraft`, `prism`…
    pub kind: String,
    pub original_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub game: GameVersion,
    pub created_at: i64,
    #[serde(default)]
    pub last_played: Option<i64>,
    #[serde(default)]
    pub playtime_seconds: u64,
    #[serde(default)]
    pub java: JavaSettings,
    #[serde(default)]
    pub window: WindowSettings,
    #[serde(default)]
    pub backups: BackupSettings,
    #[serde(default)]
    pub dev_console: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_from: Option<ImportedFrom>,
}

/// Datos para crear una instancia.
#[derive(Debug, Clone)]
pub struct NewInstance {
    pub name: String,
    pub game: GameVersion,
}

/// Instancia en la papelera.
#[derive(Debug, Clone, Serialize)]
pub struct TrashedInstance {
    /// Nombre de la carpeta en `trash/`.
    pub trash_id: String,
    pub instance: Instance,
    pub deleted_at: i64,
}

/// Nombre → identificador de carpeta: minúsculas ASCII y guiones.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = deunicode_basic(c);
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let mut slug: String = out.trim_matches('-').chars().take(48).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "instancia".into()
    } else {
        slug
    }
}

/// Quita tildes de las letras latinas más comunes.
fn deunicode_basic(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
        'ñ' | 'Ñ' => 'n',
        'ç' | 'Ç' => 'c',
        other => other,
    }
}

impl Instance {
    /// Memoria efectiva en MB.
    pub fn memory_mb(&self, default_mb: u32) -> u32 {
        self.java.memory_mb.unwrap_or(default_mb)
    }
}

impl Core {
    pub fn instance_dir(&self, id: &str) -> Result<PathBuf> {
        self.paths().instance_dir(id)
    }

    /// Directorio del juego (`--gameDir`).
    pub fn instance_game_dir(&self, id: &str) -> Result<PathBuf> {
        Ok(self.instance_dir(id)?.join("game"))
    }

    pub async fn list_instances(&self) -> Result<Vec<Instance>> {
        let dir = self.paths().instances_dir();
        let mut out = Vec::new();
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(Error::io(&dir, e)),
        };
        while let Some(entry) = entries.next_entry().await.map_err(|e| Error::io(&dir, e))? {
            let path = entry.path().join("instance.json");
            match read_json::<Instance>(&path).await {
                Ok(Some(i)) => out.push(i),
                Ok(None) => {}
                Err(e) => tracing::warn!(error = %e, "instancia ilegible"),
            }
        }
        out.sort_by(|a, b| {
            b.last_played
                .cmp(&a.last_played)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out)
    }

    pub async fn get_instance(&self, id: &str) -> Result<Instance> {
        read_json(&self.instance_dir(id)?.join("instance.json"))
            .await?
            .ok_or_else(|| Error::InstanceNotFound(id.to_owned()))
    }

    /// Busca por id exacto, nombre (sin distinguir mayúsculas) o prefijo único de id.
    pub async fn find_instance(&self, query: &str) -> Result<Instance> {
        if single_segment(query).is_ok()
            && let Ok(i) = self.get_instance(query).await
        {
            return Ok(i);
        }
        let all = self.list_instances().await?;
        let by_name: Vec<_> = all
            .iter()
            .filter(|i| i.name.eq_ignore_ascii_case(query))
            .collect();
        if let [one] = by_name.as_slice() {
            return Ok((*one).clone());
        }
        let by_prefix: Vec<_> = all.iter().filter(|i| i.id.starts_with(query)).collect();
        match by_prefix.as_slice() {
            [one] => Ok((*one).clone()),
            _ => Err(Error::InstanceNotFound(query.to_owned())),
        }
    }

    /// Id libre a partir de un nombre (`slug`, `slug-2`…).
    /// Id libre a partir de un nombre (`slug`, `slug-2`…). Un id está ocupado si
    /// existe su instancia o su carpeta de backups (para no heredar backups ajenos).
    pub async fn unique_instance_id(&self, name: &str) -> Result<String> {
        self.free_id(&slugify(name), 1).await
    }

    async fn free_id(&self, base: &str, mut n: u32) -> Result<String> {
        loop {
            let candidate = if n <= 1 {
                base.to_owned()
            } else {
                format!("{base}-{n}")
            };
            let taken = tokio::fs::try_exists(self.instance_dir(&candidate)?)
                .await
                .unwrap_or(true)
                || tokio::fs::try_exists(self.paths().backups_dir().join(&candidate))
                    .await
                    .unwrap_or(true);
            if !taken {
                return Ok(candidate);
            }
            n = n.max(1) + 1;
        }
    }

    /// Reserva un id creando su carpeta (atómico: dos creaciones a la vez no comparten carpeta).
    async fn reserve_instance_id(&self, name: &str) -> Result<String> {
        let instances = self.paths().instances_dir();
        tokio::fs::create_dir_all(&instances)
            .await
            .map_err(|e| Error::io(&instances, e))?;
        let base = slugify(name);
        let mut n = 1;
        loop {
            let candidate = self.free_id(&base, n).await?;
            let dir = self.instance_dir(&candidate)?;
            match tokio::fs::create_dir(&dir).await {
                Ok(()) => return Ok(candidate),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    n = candidate
                        .rsplit_once('-')
                        .and_then(|(_, k)| k.parse::<u32>().ok())
                        .unwrap_or(1)
                        + 1;
                }
                Err(e) => return Err(Error::io(&dir, e)),
            }
        }
    }

    pub async fn create_instance(&self, new: NewInstance) -> Result<Instance> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(Error::InvalidInput("el nombre no puede estar vacío".into()));
        }
        let id = self.reserve_instance_id(name).await?;
        let instance = Instance {
            schema: INSTANCE_SCHEMA,
            id: id.clone(),
            name: name.to_owned(),
            game: new.game,
            created_at: now_secs(),
            last_played: None,
            playtime_seconds: 0,
            java: JavaSettings::default(),
            window: WindowSettings::default(),
            backups: BackupSettings::default(),
            dev_console: false,
            imported_from: None,
        };
        let game_dir = self.instance_game_dir(&id)?;
        tokio::fs::create_dir_all(&game_dir)
            .await
            .map_err(|e| Error::io(&game_dir, e))?;
        self.save_instance(&instance).await?;
        Ok(instance)
    }

    pub async fn save_instance(&self, instance: &Instance) -> Result<()> {
        write_json(
            &self.instance_dir(&instance.id)?.join("instance.json"),
            instance,
        )
        .await
    }

    /// Lee, modifica y guarda una instancia.
    pub async fn update_instance(
        &self,
        id: &str,
        f: impl FnOnce(&mut Instance),
    ) -> Result<Instance> {
        let _guard = self.inner.instances_lock.lock().await;
        let mut instance = self.get_instance(id).await?;
        f(&mut instance);
        instance.id = id.to_owned();
        instance.schema = INSTANCE_SCHEMA;
        self.save_instance(&instance).await?;
        Ok(instance)
    }

    /// Suma una sesión de juego al contador.
    pub async fn record_play_session(&self, id: &str, seconds: u64) -> Result<Instance> {
        self.update_instance(id, |i| {
            i.playtime_seconds += seconds;
            i.last_played = Some(now_secs());
        })
        .await
    }

    /// Manda la instancia a la papelera.
    /// Sus backups van con ella (`trash/<trash_id>/.backups`).
    pub async fn delete_instance(&self, id: &str) -> Result<TrashedInstance> {
        let instance = self.get_instance(id).await?;
        self.ensure_instance_stopped(id).await?;
        let now = now_secs();
        let trash_id = format!("{id}{TRASH_SEPARATOR}{now}");
        let trash = self.paths().trash_dir();
        tokio::fs::create_dir_all(&trash)
            .await
            .map_err(|e| Error::io(&trash, e))?;
        let from = self.instance_dir(id)?;
        let to = trash.join(&trash_id);
        tokio::fs::rename(&from, &to)
            .await
            .map_err(|e| Error::io(&from, e))?;
        let backups = self.paths().backups_dir().join(single_segment(id)?);
        if tokio::fs::try_exists(&backups).await.unwrap_or(false) {
            tokio::fs::rename(&backups, to.join(TRASH_BACKUPS))
                .await
                .map_err(|e| Error::io(&backups, e))?;
        }
        Ok(TrashedInstance {
            trash_id,
            instance,
            deleted_at: now,
        })
    }

    pub async fn list_trash(&self) -> Result<Vec<TrashedInstance>> {
        let dir = self.paths().trash_dir();
        let mut out = Vec::new();
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(Error::io(&dir, e)),
        };
        while let Some(entry) = entries.next_entry().await.map_err(|e| Error::io(&dir, e))? {
            let trash_id = entry.file_name().to_string_lossy().into_owned();
            let Some(deleted_at) = trash_id
                .rsplit_once(TRASH_SEPARATOR)
                .and_then(|(_, t)| t.parse::<i64>().ok())
            else {
                continue;
            };
            if let Ok(Some(instance)) =
                read_json::<Instance>(&entry.path().join("instance.json")).await
            {
                out.push(TrashedInstance {
                    trash_id,
                    instance,
                    deleted_at,
                });
            }
        }
        out.sort_by_key(|t| std::cmp::Reverse(t.deleted_at));
        Ok(out)
    }

    /// Recupera una instancia de la papelera (con otro id si el original está ocupado).
    pub async fn restore_instance(&self, trash_id: &str) -> Result<Instance> {
        let from = self.paths().trash_dir().join(single_segment(trash_id)?);
        let mut instance: Instance = read_json(&from.join("instance.json"))
            .await?
            .ok_or_else(|| Error::InstanceNotFound(trash_id.to_owned()))?;
        let id = if tokio::fs::try_exists(self.instance_dir(&instance.id)?)
            .await
            .unwrap_or(true)
        {
            self.unique_instance_id(&instance.id).await?
        } else {
            instance.id.clone()
        };
        let to = self.instance_dir(&id)?;
        tokio::fs::create_dir_all(self.paths().instances_dir())
            .await
            .map_err(|e| Error::io(self.paths().instances_dir(), e))?;
        tokio::fs::rename(&from, &to)
            .await
            .map_err(|e| Error::io(&from, e))?;
        let trashed_backups = to.join(TRASH_BACKUPS);
        if tokio::fs::try_exists(&trashed_backups)
            .await
            .unwrap_or(false)
        {
            let dest = self.paths().backups_dir().join(single_segment(&id)?);
            tokio::fs::create_dir_all(self.paths().backups_dir())
                .await
                .map_err(|e| Error::io(self.paths().backups_dir(), e))?;
            tokio::fs::rename(&trashed_backups, &dest)
                .await
                .map_err(|e| Error::io(&trashed_backups, e))?;
        }
        instance.id = id;
        self.save_instance(&instance).await?;
        Ok(instance)
    }

    /// Borra definitivamente una instancia de la papelera.
    pub async fn purge_instance(&self, trash_id: &str) -> Result<()> {
        let dir = self.paths().trash_dir().join(single_segment(trash_id)?);
        tokio::fs::remove_dir_all(&dir)
            .await
            .map_err(|e| Error::io(&dir, e))
    }

    /// Borra lo que lleve más de 30 días en la papelera. Devuelve cuántas.
    pub async fn purge_expired_trash(&self) -> Result<usize> {
        let now = now_secs();
        let mut n = 0;
        for t in self.list_trash().await? {
            if now - t.deleted_at >= TRASH_RETENTION_SECS {
                self.purge_instance(&t.trash_id).await?;
                n += 1;
            }
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use crate::secrets::MemoryStore;
    use std::sync::Arc;

    #[test]
    fn slugs() {
        assert_eq!(slugify("Fabulously Optimized"), "fabulously-optimized");
        assert_eq!(slugify("  Técnico   ñandú!! "), "tecnico-nandu");
        assert_eq!(slugify("¿?"), "instancia");
        assert_eq!(slugify(&"a".repeat(100)).len(), 48);
    }

    async fn core(dir: &std::path::Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap()
    }

    fn fabric() -> GameVersion {
        GameVersion {
            mc: "26.2".into(),
            loader: LoaderKind::Fabric,
            loader_version: Some("0.19.3".into()),
        }
    }

    #[tokio::test]
    async fn ciclo_de_vida() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let a = core
            .create_instance(NewInstance {
                name: "Mi mundo".into(),
                game: fabric(),
            })
            .await
            .unwrap();
        let b = core
            .create_instance(NewInstance {
                name: "mi MUNDO".into(),
                game: fabric(),
            })
            .await
            .unwrap();
        assert_eq!(a.id, "mi-mundo");
        assert_eq!(b.id, "mi-mundo-2");
        assert!(core.instance_game_dir(&a.id).unwrap().is_dir());
        assert_eq!(core.list_instances().await.unwrap().len(), 2);
        assert_eq!(
            core.find_instance("mi-mundo-2").await.unwrap().id,
            "mi-mundo-2"
        );
        assert!(core.find_instance("mi mundo").await.is_err()); // ambiguo

        let a = core.record_play_session(&a.id, 120).await.unwrap();
        assert_eq!(a.playtime_seconds, 120);
        assert!(a.last_played.is_some());
        assert_eq!(core.list_instances().await.unwrap()[0].id, "mi-mundo");

        let t = core.delete_instance("mi-mundo").await.unwrap();
        assert_eq!(core.list_instances().await.unwrap().len(), 1);
        assert_eq!(core.list_trash().await.unwrap().len(), 1);
        // Mientras, otra instancia ocupa el id.
        core.create_instance(NewInstance {
            name: "Mi mundo".into(),
            game: fabric(),
        })
        .await
        .unwrap();
        let restored = core.restore_instance(&t.trash_id).await.unwrap();
        assert_eq!(restored.id, "mi-mundo-3");
        assert_eq!(restored.playtime_seconds, 120);
        assert!(core.list_trash().await.unwrap().is_empty());

        let t = core.delete_instance("mi-mundo-2").await.unwrap();
        assert_eq!(core.purge_expired_trash().await.unwrap(), 0);
        core.purge_instance(&t.trash_id).await.unwrap();
        assert!(core.list_trash().await.unwrap().is_empty());
        assert!(core.get_instance("../x").await.is_err());
    }

    #[tokio::test]
    async fn backups_acompanan_a_la_instancia() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let a = core
            .create_instance(NewInstance {
                name: "Survival".into(),
                game: fabric(),
            })
            .await
            .unwrap();
        let backups = core.paths().backups_dir().join(&a.id);
        std::fs::create_dir_all(&backups).unwrap();
        std::fs::write(backups.join("backup-1.zip"), "zip").unwrap();

        let t = core.delete_instance(&a.id).await.unwrap();
        assert!(!backups.exists());
        // Una instancia nueva con el mismo nombre no hereda nada.
        let b = core
            .create_instance(NewInstance {
                name: "Survival".into(),
                game: fabric(),
            })
            .await
            .unwrap();
        assert_eq!(b.id, "survival");
        assert!(!core.paths().backups_dir().join("survival").exists());

        // Al recuperarla cambia de id y sus backups la siguen.
        let restored = core.restore_instance(&t.trash_id).await.unwrap();
        assert_eq!(restored.id, "survival-2");
        assert!(
            core.paths()
                .backups_dir()
                .join("survival-2/backup-1.zip")
                .exists()
        );

        // Un id con backups huérfanos no se reutiliza.
        std::fs::create_dir_all(core.paths().backups_dir().join("otra")).unwrap();
        let c = core
            .create_instance(NewInstance {
                name: "Otra".into(),
                game: fabric(),
            })
            .await
            .unwrap();
        assert_eq!(c.id, "otra-2");
    }

    #[tokio::test]
    async fn creaciones_simultaneas_no_comparten_carpeta() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let mut tasks = Vec::new();
        for _ in 0..6 {
            let core = core.clone();
            tasks.push(tokio::spawn(async move {
                core.create_instance(NewInstance {
                    name: "Pack".into(),
                    game: fabric(),
                })
                .await
                .unwrap()
                .id
            }));
        }
        let mut ids = Vec::new();
        for t in tasks {
            ids.push(t.await.unwrap());
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 6, "{ids:?}");
    }
}
