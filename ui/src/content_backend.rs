//! [`ContentActions`] contra el núcleo: búsqueda, versiones, instalación,
//! lo instalado, mundos, versiones de Minecraft, iconos y modpacks. Cada
//! respuesta vuelve al hilo de la UI con [`content::post`].

use lcore::instance::LoaderKind;
use lcore::content::{ContentKind, ContentQuery, SortOrder, Source};

use crate::backend::Backend;
use crate::content::{self, ContentActions, Filters, InstallRequest, Installed, ModpackDone, Outcome, Page, VersionsQuery};
use crate::errors::Failure;
use crate::instances::Record;

pub struct CoreContent {
    pub backend: Backend,
    pub lang: String,
}

fn source(s: &str) -> Source {
    Source::parse(s).unwrap_or(Source::Modrinth)
}

/// Consulta del núcleo para unos filtros.
pub fn query(f: &Filters, offset: u32) -> ContentQuery {
    ContentQuery {
        source: source(&f.source),
        kind: ContentKind::parse(&f.kind).unwrap_or(ContentKind::Mod),
        text: f.text.trim().to_owned(),
        mc: f.mc.clone(),
        loader: f.query_loader().and_then(LoaderKind::parse),
        offset,
        limit: content::PAGE,
        sort: SortOrder::parse(&f.sort).unwrap_or_default(),
    }
}

/// Resultado de una instalación: dependencias nuevas y omitidas.
pub fn outcome(o: &lcore::content::InstallOutcome, project: &str) -> Outcome {
    Outcome {
        new_dependencies: o.installed.iter().filter(|i| i.dependency && i.project_id != project).count(),
        skipped: o.skipped.len(),
    }
}

impl ContentActions for CoreContent {
    fn search(&self, f: &Filters, offset: u32, generation: u64) {
        let (core, lang, q) = (self.backend.core.clone(), self.lang.clone(), query(f, offset));
        self.backend.rt.spawn(async move {
            let r = core
                .search_content(&q)
                .await
                .map(|p| Page { items: p.items.into_iter().map(Into::into).collect(), total: p.total })
                .map_err(|e| Failure::new(&lang, &e));
            content::post(move |c| c.page(generation, r));
        });
    }

    fn versions(&self, q: &VersionsQuery, seq: u64) {
        let (core, lang, q) = (self.backend.core.clone(), self.lang.clone(), q.clone());
        self.backend.rt.spawn(async move {
            let loader = q.loader.as_deref().and_then(LoaderKind::parse);
            let r = core
                .content_versions(source(&q.source), &q.project, q.mc.as_deref(), loader)
                .await
                .map(|v| v.into_iter().map(Into::into).collect())
                .map_err(|e| Failure::new(&lang, &e));
            content::post(move |c| c.versions_loaded(seq, r));
        });
    }

    fn install(&self, r: &InstallRequest) {
        let (core, lang, r) = (self.backend.core.clone(), self.lang.clone(), r.clone());
        self.backend.rt.spawn(async move {
            let out = core
                .install_content(&r.instance_id, source(&r.source), &r.project, r.version.as_deref(), r.world.as_deref())
                .await
                .map(|o| outcome(&o, &r.project))
                .map_err(|e| Failure::new(&lang, &e));
            content::post(move |c| c.install_done(&r, out));
        });
    }

    fn load_installed(&self, instance_id: &str) {
        let (core, id) = (self.backend.core.clone(), instance_id.to_owned());
        self.backend.rt.spawn(async move {
            let list = match core.list_content(&id).await {
                Ok(l) => l
                    .into_iter()
                    .map(|x| Installed { project_id: x.project_id, version_number: x.version_number, file: x.file })
                    .collect(),
                Err(e) => {
                    tracing::warn!(error = %e, instance = %id, "no se pudo leer el contenido instalado");
                    Vec::new()
                }
            };
            content::post(move |c| c.installed_loaded(&id, list));
        });
    }

    fn load_worlds(&self, instance_id: &str) {
        let (core, id) = (self.backend.core.clone(), instance_id.to_owned());
        self.backend.rt.spawn(async move {
            let list = core.list_worlds(&id).await.unwrap_or_default().into_iter().map(|w| (w.folder, w.name)).collect();
            content::post(move |c| c.worlds_loaded(&id, list));
        });
    }

    fn load_releases(&self) {
        let core = self.backend.core.clone();
        self.backend.rt.spawn(async move {
            match core.version_manifest().await {
                Ok(m) => {
                    let list = m.versions.into_iter().filter(|v| v.kind == "release").map(|v| v.id).collect();
                    content::post(move |c| c.releases_loaded(list));
                }
                Err(e) => tracing::warn!(error = %e, "no se pudo leer la lista de versiones"),
            }
        });
    }

    fn load_icon(&self, url: &str) {
        let (core, url) = (self.backend.core.clone(), url.to_owned());
        self.backend.rt.spawn(async move {
            let bytes = crate::media::cached_image(&core, &url).await;
            let icon = tokio::task::spawn_blocking(move || bytes.as_deref().and_then(content::decode_icon)).await.ok().flatten();
            content::post(move |c| c.icon_loaded(&url, icon));
        });
    }

    fn pick_modpack(&self, filter_label: &str) {
        let label = filter_label.to_owned();
        self.backend.rt.spawn(async move {
            let f = rfd::AsyncFileDialog::new().add_filter(label, &["mrpack", "zip"]).pick_file().await;
            let path = f.map(|h| h.path().display().to_string());
            content::post(move |c| c.modpack_picked(path));
        });
    }

    fn install_modpack(&self, source: &str, name: Option<&str>, attempt: u64) {
        let (backend, lang, source, name) = (self.backend.clone(), self.lang.clone(), source.to_owned(), name.map(str::to_owned));
        self.backend.rt.spawn(async move {
            let r = backend
                .core
                .install_modpack(&source, name)
                .await
                .map(|m| ModpackDone {
                    instance: Record::from(m.instance),
                    files_total: m.files_total as u32,
                    files_downloaded: m.files_downloaded as u32,
                    warnings: m.warnings,
                })
                .map_err(|e| Failure::new(&lang, &e));
            // La instancia nueva aparece en la lista.
            if r.is_ok() {
                backend.load();
            }
            content::post(move |c| c.modpack_done(attempt, r));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consulta_del_nucleo() {
        let f = Filters {
            source: "curseforge".into(),
            kind: "shader".into(),
            text: "  bsl ".into(),
            mc: Some("1.21.4".into()),
            loader: Some("fabric".into()),
            sort: "downloads".into(),
        };
        let q = query(&f, 40);
        assert_eq!(q.source, Source::CurseForge);
        assert_eq!(q.kind, ContentKind::Shader);
        assert_eq!(q.text, "bsl");
        assert_eq!(q.loader, None, "los shaders no filtran por loader");
        assert_eq!((q.offset, q.limit), (40, content::PAGE));
        assert_eq!(q.sort, SortOrder::Downloads);
        let q = query(&Filters { loader: Some("quilt".into()), ..Default::default() }, 0);
        assert_eq!(q.loader, Some(LoaderKind::Quilt));
    }
}
