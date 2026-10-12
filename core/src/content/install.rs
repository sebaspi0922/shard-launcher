//! Instalar, actualizar e identificar contenido con dependencias.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::manage::{InstalledContent, content_path, disabled_path, disk_path, resolve_one};
use super::types::{
    ContentKind, ContentVersion, NON_MOD_LOADERS, ProjectHit, Source, VersionFile, loader_names,
};
use super::{
    Content, ContentFile, SOURCE_UNKNOWN, is_compatible, load_index, pick_best, update_index,
};
use crate::download::{DownloadTask, Verify, download_all, download_one, sha1_file};
use crate::events::{Event, ProgressUnit};
use crate::install::next_task_id;
use crate::instance::Instance;
use crate::paths::single_segment;
use crate::{Error, Result};

/// Niveles de dependencias que se siguen (la raíz es el nivel 0).
const MAX_DEPTH: u32 = 3;

#[derive(Debug, Clone, Serialize)]
pub struct InstalledItem {
    pub kind: ContentKind,
    pub project_id: String,
    pub project_name: String,
    pub version_id: String,
    pub version_number: String,
    /// Nombre en `content.json` (en los datapacks, `<mundo>/<archivo>`).
    pub file: String,
    /// Se instaló por ser dependencia de otro.
    pub dependency: bool,
    /// Ya estaba en disco idéntico: solo se registró.
    pub already_present: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedItem {
    pub project_id: String,
    pub project_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallOutcome {
    pub instance_id: String,
    pub installed: Vec<InstalledItem>,
    pub skipped: Vec<SkippedItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAvailable {
    pub kind: ContentKind,
    pub file: String,
    pub enabled: bool,
    pub project_id: String,
    pub project_name: Option<String>,
    pub current_version: Option<String>,
    pub latest: ContentVersion,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatedContent {
    pub kind: ContentKind,
    /// Nombre nuevo en `content.json`.
    pub file: String,
    pub from_version: Option<String>,
    pub to_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentifiedContent {
    pub kind: ContentKind,
    pub file: String,
    pub project_id: String,
    pub project_name: String,
    pub version_number: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct IdentifyReport {
    pub identified: Vec<IdentifiedContent>,
    /// Archivos que Modrinth no conoce.
    pub unmatched: Vec<String>,
}

struct Work {
    project: Option<String>,
    version: Option<String>,
    depth: u32,
}

enum DestState {
    Free,
    /// Ya existe idéntico (`true` si está como `.disabled`).
    Identical {
        disabled: bool,
    },
}

struct Planned {
    kind: ContentKind,
    project: ProjectHit,
    version: ContentVersion,
    file: VersionFile,
    entry_file: String,
    dest: PathBuf,
    state: DestState,
    dependency: bool,
}

#[derive(Default)]
struct Plan {
    planned: Vec<Planned>,
    seen: HashSet<String>,
    skipped: Vec<SkippedItem>,
    warnings: Vec<String>,
}

impl Plan {
    /// Un problema del que se puede seguir si no es el proyecto pedido.
    fn soft(&mut self, root: bool, msg: String) -> Result<()> {
        if root {
            Err(Error::InvalidInput(msg))
        } else {
            self.warnings.push(msg);
            Ok(())
        }
    }
}

/// Nombre de archivo apto para escribirlo en una carpeta de contenido.
fn validate_filename(name: &str) -> Result<()> {
    single_segment(name)?;
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".jar") || lower.ends_with(".zip") || lower.ends_with(".litemod") {
        Ok(())
    } else {
        Err(Error::InvalidPath(format!(
            "tipo de archivo no admitido: {name:?}"
        )))
    }
}

async fn check_dest(dest: &Path, expected_sha1: Option<&str>) -> Result<DestState> {
    for (path, disabled) in [(dest.to_path_buf(), false), (disabled_path(dest), true)] {
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            continue;
        }
        let identical = match expected_sha1 {
            Some(expected) if path.is_file() => sha1_file(&path)
                .await
                .is_ok_and(|h| h.eq_ignore_ascii_case(expected)),
            _ => false,
        };
        if identical {
            return Ok(DestState::Identical { disabled });
        }
        return Err(Error::InvalidInput(format!(
            "ya existe {} y no es el mismo archivo; quítalo o muévelo antes (`lcore content remove`)",
            path.display()
        )));
    }
    Ok(DestState::Free)
}

fn entry_for(
    kind: ContentKind,
    project: &ProjectHit,
    version: &ContentVersion,
    file: &VersionFile,
    entry_file: &str,
    enabled: bool,
) -> ContentFile {
    ContentFile {
        kind: kind.as_str().into(),
        file: entry_file.into(),
        enabled,
        source: version.source.as_str().into(),
        project_id: Some(project.id.clone()),
        project_name: Some(project.title.clone()),
        version_id: Some(version.id.clone()),
        version_number: Some(version.version_number.clone()),
        sha1: file.sha1.clone(),
        url: Some(file.url.clone()),
        category: None,
    }
}

impl Content {
    /// Instala un proyecto y sus dependencias obligatorias (hasta 3 niveles).
    ///
    /// Elige la versión más nueva compatible con el Minecraft y el loader de la
    /// instancia (los resource packs, shaders y datapacks no se filtran por
    /// loader). Con `version` se fuerza esa versión. Los datapacks necesitan
    /// `world`.
    pub async fn install(
        &self,
        instance: &str,
        source: Source,
        project: &str,
        version: Option<&str>,
        world: Option<&str>,
    ) -> Result<InstallOutcome> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let index_path = self.index_path(&inst.id)?;
        let index = load_index(&index_path).await?;

        let mut plan = Plan::default();
        let mut queue = VecDeque::from([Work {
            project: Some(project.to_owned()),
            version: version.map(str::to_owned),
            depth: 0,
        }]);
        while let Some(work) = queue.pop_front() {
            self.plan_one(
                &inst, &index, &game_dir, source, world, work, &mut queue, &mut plan,
            )
            .await?;
        }

        // Destinos repetidos con distinto contenido: dos proyectos que traen el mismo nombre.
        let mut dests = HashSet::new();
        for p in &plan.planned {
            if !dests.insert(p.dest.clone()) {
                return Err(Error::InvalidInput(format!(
                    "dos proyectos instalarían el mismo archivo: {}",
                    p.entry_file
                )));
            }
        }

        let tasks: Vec<DownloadTask> = plan
            .planned
            .iter()
            .filter(|p| matches!(p.state, DestState::Free))
            .map(|p| DownloadTask {
                url: p.file.url.clone(),
                dest: p.dest.clone(),
                sha1: p.file.sha1.clone(),
                size: (p.file.size > 0).then_some(p.file.size),
            })
            .collect();
        for p in &plan.planned {
            if p.file.sha1.is_none() {
                plan.warnings.push(format!(
                    "{}: la API no da el hash; no se pudo verificar",
                    p.file.filename
                ));
            }
        }
        let fresh: Vec<PathBuf> = tasks.iter().map(|t| t.dest.clone()).collect();

        let task_id = next_task_id();
        let events = self.core.inner.events.clone();
        let concurrency = self.core.settings().await.max_concurrent_downloads;
        let result = download_all(
            self.core.http(),
            tasks,
            concurrency,
            Verify::Size,
            move |done, total| {
                events.emit(Event::Progress {
                    task_id,
                    stage: "content".into(),
                    done,
                    total,
                    unit: ProgressUnit::Files,
                });
            },
        )
        .await;
        let recorded = match result {
            Ok(_) => {
                let entries: Vec<ContentFile> = plan
                    .planned
                    .iter()
                    .map(|p| {
                        let enabled = !matches!(p.state, DestState::Identical { disabled: true });
                        entry_for(
                            p.kind,
                            &p.project,
                            &p.version,
                            &p.file,
                            &p.entry_file,
                            enabled,
                        )
                    })
                    .collect();
                update_index(&index_path, |idx| {
                    for e in entries {
                        idx.upsert(e);
                    }
                    Ok(())
                })
                .await
            }
            Err(e) => Err(e),
        };
        if let Err(e) = recorded {
            for path in fresh {
                let _ = tokio::fs::remove_file(&path).await;
            }
            self.core.inner.events.emit(Event::TaskFinished {
                task_id,
                ok: false,
                message: Some(e.to_string()),
            });
            return Err(e);
        }
        self.core.inner.events.emit(Event::TaskFinished {
            task_id,
            ok: true,
            message: None,
        });

        let installed = plan
            .planned
            .iter()
            .map(|p| InstalledItem {
                kind: p.kind,
                project_id: p.project.id.clone(),
                project_name: p.project.title.clone(),
                version_id: p.version.id.clone(),
                version_number: p.version.version_number.clone(),
                file: p.entry_file.clone(),
                dependency: p.dependency,
                already_present: matches!(p.state, DestState::Identical { .. }),
            })
            .collect();
        Ok(InstallOutcome {
            instance_id: inst.id,
            installed,
            skipped: plan.skipped,
            warnings: plan.warnings,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn plan_one(
        &self,
        inst: &Instance,
        index: &super::ContentIndex,
        game_dir: &Path,
        source: Source,
        world: Option<&str>,
        work: Work,
        queue: &mut VecDeque<Work>,
        plan: &mut Plan,
    ) -> Result<()> {
        let root = work.depth == 0;
        let (project, pre): (ProjectHit, Option<ContentVersion>) =
            match (&work.project, &work.version) {
                (Some(p), _) => (self.project(source, p).await?, None),
                (None, Some(v)) => {
                    let ver = self.version(source, "", v).await?;
                    (self.project(source, &ver.project_id).await?, Some(ver))
                }
                (None, None) => return Ok(()),
            };

        if !plan.seen.insert(project.id.clone()) {
            plan.skipped.push(SkippedItem {
                project_id: project.id,
                project_name: Some(project.title),
                reason: "ya se instala en esta operación".into(),
            });
            return Ok(());
        }
        if index.has_project(source.as_str(), &project.id) {
            if root {
                return Err(Error::InvalidInput(format!(
                    "«{}» ya está instalado; para cambiar de versión usa las actualizaciones",
                    project.title
                )));
            }
            plan.skipped.push(SkippedItem {
                project_id: project.id,
                project_name: Some(project.title),
                reason: "ya instalado".into(),
            });
            return Ok(());
        }

        let Some(kind) = project.kind else {
            return plan.soft(
                root,
                format!("«{}» es de un tipo que no se gestiona", project.title),
            );
        };
        if kind == ContentKind::Modpack {
            return plan.soft(
                root,
                format!(
                    "«{}» es un modpack: instálalo con `lcore modpack install`",
                    project.title
                ),
            );
        }

        // Mundo de los datapacks.
        let target_world = if kind == ContentKind::Datapack {
            match world {
                None => {
                    return plan.soft(
                        root,
                        format!(
                            "«{}» es un datapack: indica el mundo (--world)",
                            project.title
                        ),
                    );
                }
                Some(w) => {
                    single_segment(w)?;
                    if !game_dir.join("saves").join(w).is_dir() {
                        return plan
                            .soft(root, format!("no existe el mundo «{w}» en la instancia"));
                    }
                    Some(w)
                }
            }
        } else {
            None
        };

        // Filtro de loader: solo mods (y modpacks).
        let loader = kind.uses_loader().then_some(inst.game.loader);
        if let Some(l) = loader
            && loader_names(l).is_empty()
        {
            return plan.soft(
                root,
                format!(
                    "la instancia usa el loader «{}», que no admite mods de {}",
                    l.as_str(),
                    source.as_str()
                ),
            );
        }
        let mc = inst.game.mc.as_str();
        let loaders: Vec<String> = loader
            .map(|l| loader_names(l).into_iter().map(str::to_owned).collect())
            .unwrap_or_default();

        // Versión: la fijada o la mejor compatible.
        let pinned = match (pre, &work.version) {
            (Some(v), _) => Some(v),
            (None, Some(vid)) => Some(self.version(source, &project.id, vid).await?),
            (None, None) => None,
        };
        let version = match pinned {
            Some(v) => {
                if v.project_id != project.id {
                    return Err(Error::InvalidInput(format!(
                        "la versión {} no es de «{}»",
                        v.id, project.title
                    )));
                }
                if is_compatible(&v, Some(mc), &loaders) {
                    Some(v)
                } else if root {
                    plan.warnings.push(format!(
                        "la versión {} de «{}» no declara compatibilidad con Minecraft {mc}",
                        v.version_number, project.title
                    ));
                    Some(v)
                } else {
                    None
                }
            }
            None => None,
        };
        let version = match version {
            Some(v) => v,
            None => {
                let all = self.versions(source, &project.id, Some(mc), loader).await?;
                match pick_best(all) {
                    Some(v) => v,
                    None => {
                        let loader_txt = loader
                            .map(|l| format!(" y {}", l.as_str()))
                            .unwrap_or_default();
                        return plan.soft(
                            root,
                            format!(
                                "«{}» no tiene versión para Minecraft {mc}{loader_txt}",
                                project.title
                            ),
                        );
                    }
                }
            }
        };

        let Some(file) = version.primary_file().cloned() else {
            return plan.soft(root, format!("«{}» no tiene archivos", project.title));
        };
        if let Err(e) = validate_filename(&file.filename) {
            return plan.soft(root, format!("«{}»: {e}", project.title));
        }
        if !(file.url.starts_with("https://") || file.url.starts_with("http://")) {
            return plan.soft(
                root,
                format!("«{}»: URL de descarga no válida", project.title),
            );
        }
        let entry_file = match target_world {
            Some(w) => format!("{w}/{}", file.filename),
            None => file.filename.clone(),
        };
        let dest = content_path(game_dir, kind, &entry_file)?;
        let state = check_dest(&dest, file.sha1.as_deref()).await?;
        if file.fallback_url {
            plan.warnings.push(format!(
                "{}: el autor no da URL de descarga; se usa la del CDN, que puede fallar",
                file.filename
            ));
        }

        // Dependencias obligatorias.
        let deps: Vec<(Option<String>, Option<String>)> = version
            .required_dependencies()
            .map(|d| (d.project_id.clone(), d.version_id.clone()))
            .collect();
        for (dep_project, dep_version) in deps {
            if dep_project.is_none() && dep_version.is_none() {
                continue;
            }
            if work.depth >= MAX_DEPTH {
                plan.warnings.push(format!(
                    "dependencia de «{}» omitida: supera {MAX_DEPTH} niveles",
                    project.title
                ));
                continue;
            }
            queue.push_back(Work {
                project: dep_project,
                version: dep_version,
                depth: work.depth + 1,
            });
        }

        plan.planned.push(Planned {
            kind,
            project,
            version,
            file,
            entry_file,
            dest,
            state,
            dependency: !root,
        });
        Ok(())
    }

    /// Busca actualizaciones de los archivos de Modrinth de la instancia
    /// (`POST /version_files/update` con el SHA-1 de cada archivo).
    pub async fn check_updates(&self, instance: &str) -> Result<Vec<UpdateAvailable>> {
        let inst = self.core.find_instance(instance).await?;
        let list = self.list(&inst.id).await?;
        self.updates_for(&inst, list).await
    }

    async fn updates_for(
        &self,
        inst: &Instance,
        items: Vec<InstalledContent>,
    ) -> Result<Vec<UpdateAvailable>> {
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let mut mods: Vec<(InstalledContent, String)> = Vec::new();
        let mut others: Vec<(InstalledContent, String)> = Vec::new();
        for item in items {
            // La búsqueda es por hash: no hace falta el id de proyecto (los
            // archivos de modpacks importados de Roxy no lo tienen).
            if item.source != Source::Modrinth.as_str() || item.is_dir {
                continue;
            }
            let path = disk_path(&game_dir, item.kind, &item.file, item.enabled)?;
            let hash = sha1_file(&path).await?;
            match item.kind {
                ContentKind::Mod => mods.push((item, hash)),
                ContentKind::Modpack => {}
                _ => others.push((item, hash)),
            }
        }

        let mc = vec![inst.game.mc.clone()];
        let mut found: Vec<(InstalledContent, String, ContentVersion)> = Vec::new();
        let mod_loaders: Vec<String> = loader_names(inst.game.loader)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let other_loaders: Vec<String> = NON_MOD_LOADERS.iter().map(|s| (*s).to_owned()).collect();
        for (group, loaders) in [(mods, mod_loaders), (others, other_loaders)] {
            if group.is_empty() || loaders.is_empty() {
                continue;
            }
            let hashes: Vec<String> = group.iter().map(|(_, h)| h.clone()).collect();
            let latest = self
                .modrinth
                .latest_by_hashes(&hashes, &loaders, &mc)
                .await?;
            for (item, hash) in group {
                if let Some(v) = latest.get(&hash) {
                    found.push((item, hash, v.clone()));
                }
            }
        }

        let mut out = Vec::new();
        for (item, hash, latest) in found {
            if latest.files.iter().any(|f| {
                f.sha1
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case(&hash))
            }) {
                continue;
            }
            out.push(UpdateAvailable {
                kind: item.kind,
                file: item.file,
                enabled: item.enabled,
                project_id: item.project_id.unwrap_or_default(),
                project_name: item.project_name,
                current_version: item.version_number,
                latest,
            });
        }
        out.sort_by(|a, b| (a.kind.as_str(), &a.file).cmp(&(b.kind.as_str(), &b.file)));
        Ok(out)
    }

    /// Actualiza un archivo; `None` si ya está al día o no es de Modrinth.
    pub async fn update(&self, instance: &str, file: &str) -> Result<Option<UpdatedContent>> {
        let inst = self.core.find_instance(instance).await?;
        let list = self.list(&inst.id).await?;
        let item = resolve_one(&list, file)?.clone();
        let updates = self.updates_for(&inst, vec![item]).await?;
        match updates.first() {
            Some(u) => Ok(Some(self.apply_update(&inst.id, u).await?)),
            None => Ok(None),
        }
    }

    /// Descarga la versión nueva y sustituye el archivo y su entrada.
    pub async fn apply_update(
        &self,
        instance: &str,
        update: &UpdateAvailable,
    ) -> Result<UpdatedContent> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let index_path = self.index_path(&inst.id)?;
        let new_file = update
            .latest
            .primary_file()
            .cloned()
            .ok_or_else(|| Error::InvalidInput("la versión nueva no tiene archivos".into()))?;
        validate_filename(&new_file.filename)?;

        let old_active = content_path(&game_dir, update.kind, &update.file)?;
        let old_disk = if update.enabled {
            old_active.clone()
        } else {
            disabled_path(&old_active)
        };
        let new_entry_file = match update.file.split_once('/') {
            Some((world, _)) if update.kind == ContentKind::Datapack => {
                format!("{world}/{}", new_file.filename)
            }
            _ => new_file.filename.clone(),
        };
        let new_active = content_path(&game_dir, update.kind, &new_entry_file)?;
        let new_disk = if update.enabled {
            new_active.clone()
        } else {
            disabled_path(&new_active)
        };
        let renamed = new_disk != old_disk;
        if renamed
            && (tokio::fs::try_exists(&new_active).await.unwrap_or(false)
                || tokio::fs::try_exists(&disabled_path(&new_active))
                    .await
                    .unwrap_or(false))
        {
            return Err(Error::InvalidInput(format!(
                "no se puede actualizar: ya existe {}",
                new_active.display()
            )));
        }

        // Se descarga junto al destino y se renombra: nunca queda a medias.
        let tmp = new_active.with_file_name(format!(".{}.update", new_file.filename));
        let task = DownloadTask {
            url: new_file.url.clone(),
            dest: tmp.clone(),
            sha1: new_file.sha1.clone(),
            size: (new_file.size > 0).then_some(new_file.size),
        };
        if let Err(e) = download_one(self.core.http(), &task).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }
        if let Err(e) = tokio::fs::rename(&tmp, &new_disk).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(Error::io(&new_disk, e));
        }
        if renamed {
            let _ = tokio::fs::remove_file(&old_disk).await;
        }

        let kind = update.kind.as_str();
        let old_file = update.file.clone();
        let latest = update.latest.clone();
        let entry_file = new_entry_file.clone();
        let enabled = update.enabled;
        update_index(&index_path, |idx| {
            let previous = idx.remove(kind, &old_file);
            idx.upsert(ContentFile {
                kind: kind.into(),
                file: entry_file,
                enabled,
                source: Source::Modrinth.as_str().into(),
                project_id: Some(latest.project_id.clone()),
                project_name: previous.as_ref().and_then(|p| p.project_name.clone()),
                version_id: Some(latest.id.clone()),
                version_number: Some(latest.version_number.clone()),
                sha1: new_file.sha1.clone(),
                url: Some(new_file.url.clone()),
                category: previous.and_then(|p| p.category),
            });
            Ok(())
        })
        .await?;
        Ok(UpdatedContent {
            kind: update.kind,
            file: new_entry_file,
            from_version: update.current_version.clone(),
            to_version: update.latest.version_number.clone(),
        })
    }

    /// Reconoce por SHA-1 (Modrinth) los archivos de origen desconocido y rellena
    /// su procedencia en `content.json`.
    pub async fn identify(&self, instance: &str) -> Result<IdentifyReport> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let list = self.list(&inst.id).await?;
        let mut targets: Vec<(InstalledContent, String)> = Vec::new();
        for item in list {
            if item.source != SOURCE_UNKNOWN || item.is_dir {
                continue;
            }
            let path = disk_path(&game_dir, item.kind, &item.file, item.enabled)?;
            let hash = sha1_file(&path).await?;
            targets.push((item, hash));
        }
        let hashes: Vec<String> = targets.iter().map(|(_, h)| h.clone()).collect();
        let versions = self.modrinth.versions_by_hashes(&hashes).await?;
        let mut project_ids: Vec<String> =
            versions.values().map(|v| v.project_id.clone()).collect();
        project_ids.sort();
        project_ids.dedup();
        let projects: HashMap<String, ProjectHit> = self
            .modrinth
            .projects(&project_ids)
            .await?
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();

        let mut report = IdentifyReport::default();
        let mut entries = Vec::new();
        for (item, hash) in targets {
            let Some(version) = versions.get(&hash) else {
                report.unmatched.push(item.file);
                continue;
            };
            let name = projects
                .get(&version.project_id)
                .map(|p| p.title.clone())
                .unwrap_or_else(|| version.project_id.clone());
            let url = version
                .files
                .iter()
                .find(|f| f.sha1.as_deref() == Some(hash.as_str()))
                .map(|f| f.url.clone());
            entries.push(ContentFile {
                kind: item.kind.as_str().into(),
                file: item.file.clone(),
                enabled: item.enabled,
                source: Source::Modrinth.as_str().into(),
                project_id: Some(version.project_id.clone()),
                project_name: Some(name.clone()),
                version_id: Some(version.id.clone()),
                version_number: Some(version.version_number.clone()),
                sha1: Some(hash),
                url,
                category: None,
            });
            report.identified.push(IdentifiedContent {
                kind: item.kind,
                file: item.file,
                project_id: version.project_id.clone(),
                project_name: name,
                version_number: version.version_number.clone(),
            });
        }
        if !entries.is_empty() {
            update_index(&self.index_path(&inst.id)?, |idx| {
                for e in entries {
                    idx.upsert(e);
                }
                Ok(())
            })
            .await?;
        }
        Ok(report)
    }
}
