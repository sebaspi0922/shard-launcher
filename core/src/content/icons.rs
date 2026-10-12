//! Iconos de lo instalado en una instancia.
//!
//! Los jars no traen el icono del proyecto, así que se resuelve en línea: por
//! `project_id` si `content.json` lo conoce (Modrinth y CurseForge) y, si no,
//! por el SHA-1 del archivo contra Modrinth (`POST /version_files` →
//! `GET /projects`). El resultado por SHA-1 se guarda en
//! `<datos>/cache/content-icons.json` para no repetir la consulta en cada
//! apertura. Todo es «mejor esfuerzo»: sin red o sin clave de CurseForge
//! simplemente no hay icono y la interfaz usa el de su tipo.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::manage::{InstalledContent, disk_path};
use super::{Content, Source};
use crate::Result;
use crate::download::sha1_file;
use crate::fsutil::{now_secs, read_json, write_json};

/// Cuánto vale un SHA-1 con icono (los proyectos casi nunca lo cambian).
const TTL_FOUND: i64 = 7 * 24 * 3600;
/// Cuánto vale un SHA-1 que Modrinth no conoce (puede que lo suban luego).
const TTL_MISSING: i64 = 24 * 3600;
/// Máximo de ids por `GET /projects` (la URL lleva la lista entera).
const PROJECTS_CHUNK: usize = 50;
/// No se calcula el SHA-1 de archivos mayores (paquetes de recursos enormes).
const MAX_HASHED_BYTES: u64 = 128 * 1024 * 1024;

/// Icono resuelto de un archivo de contenido.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentIcon {
    pub kind: super::types::ContentKind,
    /// Mismo `file` que en `InstalledContent`.
    pub file: String,
    pub icon_url: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct IconCache {
    /// SHA-1 en minúsculas → icono (o `None` si Modrinth no lo conoce).
    #[serde(default)]
    hashes: HashMap<String, CachedIcon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedIcon {
    icon: Option<String>,
    at: i64,
}

impl CachedIcon {
    fn fresh(&self, now: i64) -> bool {
        let ttl = if self.icon.is_some() {
            TTL_FOUND
        } else {
            TTL_MISSING
        };
        now.saturating_sub(self.at) < ttl
    }
}

impl Content {
    /// Iconos de lo instalado en la instancia. Los archivos sin icono (carpetas,
    /// los que ninguna fuente reconoce) no aparecen en el resultado.
    pub async fn icons(&self, instance: &str) -> Result<Vec<ContentIcon>> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let list = self.list(&inst.id).await?;

        let cache_path = self
            .core
            .paths()
            .root()
            .join("cache")
            .join("content-icons.json");
        let mut cache: IconCache = read_json(&cache_path)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let now = now_secs();

        // Qué proyecto (o qué SHA-1) corresponde a cada archivo.
        let mut modrinth_ids: HashSet<String> = HashSet::new();
        let mut curseforge_ids: HashSet<String> = HashSet::new();
        let mut by_project: Vec<(&InstalledContent, Source, String)> = Vec::new();
        let mut by_hash: Vec<(&InstalledContent, String)> = Vec::new();
        for item in list.iter().filter(|c| !c.is_dir) {
            match (item.source.as_str(), item.project_id.as_deref()) {
                ("modrinth", Some(id)) => {
                    modrinth_ids.insert(id.to_owned());
                    by_project.push((item, Source::Modrinth, id.to_owned()));
                }
                ("curseforge", Some(id)) => {
                    curseforge_ids.insert(id.to_owned());
                    by_project.push((item, Source::CurseForge, id.to_owned()));
                }
                _ => {
                    let hash = match &item.sha1 {
                        Some(h) => h.to_ascii_lowercase(),
                        None if item.size > MAX_HASHED_BYTES => continue,
                        None => {
                            let path = disk_path(&game_dir, item.kind, &item.file, item.enabled)?;
                            match sha1_file(&path).await {
                                Ok(h) => h,
                                Err(_) => continue,
                            }
                        }
                    };
                    by_hash.push((item, hash));
                }
            }
        }

        // Los SHA-1 que no están en la caché (o caducaron) se preguntan a Modrinth.
        let stale: Vec<String> = by_hash
            .iter()
            .map(|(_, h)| h.clone())
            .filter(|h| !cache.hashes.get(h).is_some_and(|c| c.fresh(now)))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut hash_project: HashMap<String, String> = HashMap::new();
        let mut hashes_ok = stale.is_empty();
        if !stale.is_empty() {
            match self.modrinth.versions_by_hashes(&stale).await {
                Ok(versions) => {
                    hashes_ok = true;
                    for (hash, v) in versions {
                        modrinth_ids.insert(v.project_id.clone());
                        hash_project.insert(hash, v.project_id);
                    }
                }
                Err(e) => tracing::warn!(error = %e, "no se pudo resolver los iconos por SHA-1"),
            }
        }

        let mut modrinth_icons: HashMap<String, String> = HashMap::new();
        let mut ids: Vec<String> = modrinth_ids.into_iter().collect();
        ids.sort();
        let mut modrinth_ok = true;
        for chunk in ids.chunks(PROJECTS_CHUNK) {
            match self.modrinth.projects(chunk).await {
                Ok(projects) => {
                    for p in projects {
                        if let Some(url) = p.icon_url {
                            modrinth_icons.insert(p.id, url);
                        }
                    }
                }
                Err(e) => {
                    modrinth_ok = false;
                    tracing::warn!(error = %e, "no se pudo leer los iconos de Modrinth");
                }
            }
        }

        let mut curseforge_icons: HashMap<String, String> = HashMap::new();
        if !curseforge_ids.is_empty() {
            let ids: Vec<String> = curseforge_ids.into_iter().collect();
            match self.curseforge.projects(&ids).await {
                Ok(projects) => {
                    for p in projects {
                        if let Some(url) = p.icon_url {
                            curseforge_icons.insert(p.id, url);
                        }
                    }
                }
                Err(e) => tracing::debug!(error = %e, "sin iconos de CurseForge"),
            }
        }

        // Guarda en la caché lo que se pudo averiguar de verdad (no los fallos
        // de red, que se reintentarán la próxima vez).
        let mut dirty = false;
        if hashes_ok {
            for hash in &stale {
                let icon = match hash_project.get(hash) {
                    Some(pid) if modrinth_ok => modrinth_icons.get(pid).cloned(),
                    Some(_) => continue,
                    None => None,
                };
                cache
                    .hashes
                    .insert(hash.clone(), CachedIcon { icon, at: now });
                dirty = true;
            }
        }
        if dirty {
            cache.hashes.retain(|_, c| c.fresh(now));
            if let Err(e) = write_json(&cache_path, &cache).await {
                tracing::warn!(error = %e, "no se pudo guardar la caché de iconos");
            }
        }

        let mut out = Vec::new();
        for (item, source, pid) in by_project {
            let icons = match source {
                Source::Modrinth => &modrinth_icons,
                Source::CurseForge => &curseforge_icons,
            };
            if let Some(url) = icons.get(&pid) {
                out.push(ContentIcon {
                    kind: item.kind,
                    file: item.file.clone(),
                    icon_url: url.clone(),
                });
            }
        }
        for (item, hash) in by_hash {
            let url = cache
                .hashes
                .get(&hash)
                .and_then(|c| c.icon.clone())
                .or_else(|| {
                    hash_project
                        .get(&hash)
                        .and_then(|pid| modrinth_icons.get(pid).cloned())
                });
            if let Some(icon_url) = url {
                out.push(ContentIcon {
                    kind: item.kind,
                    file: item.file.clone(),
                    icon_url,
                });
            }
        }
        out.sort_by(|a, b| (a.kind.as_str(), &a.file).cmp(&(b.kind.as_str(), &b.file)));
        Ok(out)
    }
}
