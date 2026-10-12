//! Importación desde otros launchers (primer arranque): Roxy Launcher y el
//! `.minecraft` del launcher oficial. Siempre se simula antes de importar; el
//! origen solo se lee.

use std::path::PathBuf;

use lcore::import::{DotMinecraftImport, DotMinecraftPlan, ImportPlan, RoxyImportOptions};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

/// Orígenes encontrados en este equipo (rutas), o `None`.
#[derive(Debug, Clone)]
pub struct ImportSourcesView {
    /// Raíz de datos de Roxy, si tiene `game/accounts`.
    pub roxy: Option<String>,
    /// Carpeta `.minecraft`, si tiene versiones o perfiles.
    pub minecraft: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlannedInstanceView {
    pub original_id: String,
    pub name: String,
    pub new_id: String,
    pub mc: String,
    pub loader: String,
    pub files: u64,
    pub bytes: u64,
    pub backups: u32,
    /// `Some` = no se importa, con el motivo.
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoxyPlanView {
    pub source: String,
    pub instances: Vec<PlannedInstanceView>,
    /// Librerías y assets que faltan en el destino.
    pub shared_files: u64,
    pub shared_bytes: u64,
    pub version_jars: u64,
    pub java_majors: Vec<u32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RoxyImportView {
    /// Ids nuevos de las instancias importadas.
    pub imported: Vec<String>,
    pub linked_files: u64,
    pub copied_files: u64,
}

#[derive(Debug, Clone)]
pub struct SectionView {
    /// `saves`, `mods`, `config`, `resourcepacks`, `shaderpacks`,
    /// `screenshots`, `options` o `servers`.
    pub section: String,
    pub present: bool,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct MinecraftPlanView {
    pub source: String,
    pub name: String,
    pub new_id: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub sections: Vec<SectionView>,
    pub files: u64,
    pub bytes: u64,
    pub warnings: Vec<String>,
}

fn roxy_root(source: Option<String>) -> PathBuf {
    source
        .map(PathBuf::from)
        .unwrap_or_else(lcore::import::roxy::default_source_root)
}

fn minecraft_root(source: Option<String>) -> PathBuf {
    source
        .map(PathBuf::from)
        .unwrap_or_else(lcore::import::minecraft::default_source_root)
}

/// Busca Roxy y `.minecraft` en sus rutas por defecto (solo comprueba que existan).
pub async fn detect_import_sources() -> Result<ImportSourcesView, BridgeError> {
    on_rt(async move {
        let roxy = roxy_root(None);
        let mc = minecraft_root(None);
        let roxy_ok = tokio::fs::metadata(roxy.join("game").join("accounts"))
            .await
            .is_ok_and(|m| m.is_dir());
        let mut mc_ok = false;
        for marker in ["versions", "launcher_profiles.json", "saves"] {
            if tokio::fs::try_exists(mc.join(marker)).await.unwrap_or(false) {
                mc_ok = true;
                break;
            }
        }
        Ok(ImportSourcesView {
            roxy: roxy_ok.then(|| roxy.display().to_string()),
            minecraft: mc_ok.then(|| mc.display().to_string()),
        })
    })
    .await
}

fn roxy_options(source: Option<String>, dry_run: bool) -> RoxyImportOptions {
    RoxyImportOptions {
        source_root: roxy_root(source),
        dry_run,
        ..RoxyImportOptions::default()
    }
}

fn roxy_plan_view(source: String, p: ImportPlan) -> RoxyPlanView {
    RoxyPlanView {
        source,
        instances: p
            .instances
            .into_iter()
            .map(|i| PlannedInstanceView {
                original_id: i.original_id,
                name: i.name,
                new_id: i.new_id,
                mc: i.mc,
                loader: i.loader,
                files: i.files,
                bytes: i.bytes,
                backups: i.backups as u32,
                skipped_reason: i.skipped_reason,
            })
            .collect(),
        shared_files: p.shared.libraries_files + p.shared.assets_files,
        shared_bytes: p.shared.libraries_bytes + p.shared.assets_bytes,
        version_jars: p.shared.version_jars,
        java_majors: p.shared.java_majors,
        warnings: p.warnings,
    }
}

/// Simula la importación de Roxy (no escribe nada).
pub async fn plan_roxy_import(source: Option<String>) -> Result<RoxyPlanView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let opts = roxy_options(source, true);
        let shown = opts.source_root.display().to_string();
        Ok(roxy_plan_view(shown, core.plan_roxy_import(&opts).await?))
    })
    .await
}

/// Importa desde Roxy. El progreso llega por eventos (etapa `import`).
pub async fn import_roxy(source: Option<String>) -> Result<RoxyImportView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let r = core.import_roxy(&roxy_options(source, false)).await?;
        Ok(RoxyImportView {
            imported: r.imported,
            linked_files: r.linked_files,
            copied_files: r.copied_files,
        })
    })
    .await
}

fn minecraft_plan_view(p: DotMinecraftPlan) -> MinecraftPlanView {
    MinecraftPlanView {
        source: p.source.display().to_string(),
        name: p.name,
        new_id: p.new_id,
        mc: p.game.mc,
        loader: p.game.loader.as_str().into(),
        loader_version: p.game.loader_version,
        sections: p
            .sections
            .into_iter()
            .map(|s| SectionView {
                section: s.section.as_str().into(),
                present: s.present,
                files: s.files,
                bytes: s.bytes,
            })
            .collect(),
        files: p.files,
        bytes: p.bytes,
        warnings: p.warnings,
    }
}

/// Simula la importación de un `.minecraft` como instancia nueva.
pub async fn plan_minecraft_import(
    source: Option<String>,
) -> Result<MinecraftPlanView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let mut opts = DotMinecraftImport::new(minecraft_root(source));
        opts.dry_run = true;
        Ok(minecraft_plan_view(core.import_dot_minecraft(&opts).await?.plan))
    })
    .await
}

/// Importa un `.minecraft` (copia). Devuelve el id de la instancia creada.
pub async fn import_minecraft(source: Option<String>) -> Result<Option<String>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let opts = DotMinecraftImport::new(minecraft_root(source));
        Ok(core.import_dot_minecraft(&opts).await?.imported)
    })
    .await
}
