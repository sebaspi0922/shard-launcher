//! Pruebas con servidores simulados (wiremock). Las de red real están `#[ignore]`.

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use wiremock::matchers::{
    body_partial_json, header, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::modrinth::facets;
use super::*;
use crate::events::Event;
use crate::instance::{GameVersion, Instance, NewInstance};
use crate::net::RetryPolicy;
use crate::secrets::MemoryStore;
use crate::{Core, Options};

fn sha1_hex(data: &[u8]) -> String {
    hex::encode(Sha1::digest(data))
}

struct Env {
    server: MockServer,
    core: Core,
    content: Content,
    dir: tempfile::TempDir,
}

fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 2,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(2),
    }
}

async fn env() -> Env {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(Options {
        data_dir: Some(dir.path().to_path_buf()),
        secrets: Some(Arc::new(MemoryStore::default())),
        retry: Some(fast_retry()),
        ..Options::default()
    })
    .await
    .unwrap();
    let content = core
        .content()
        .with_endpoints(ContentEndpoints {
            modrinth: format!("{}/v2", server.uri()),
            curseforge: format!("{}/cf/v1", server.uri()),
        })
        .with_extra_download_hosts(vec!["127.0.0.1".into()]);
    Env {
        server,
        core,
        content,
        dir,
    }
}

async fn instance(env: &Env, mc: &str, loader: LoaderKind) -> Instance {
    env.core
        .create_instance(NewInstance {
            name: "Prueba".into(),
            game: GameVersion {
                mc: mc.into(),
                loader,
                loader_version: None,
            },
        })
        .await
        .unwrap()
}

fn game_dir(env: &Env, inst: &Instance) -> std::path::PathBuf {
    env.core.instance_game_dir(&inst.id).unwrap()
}

async fn index_of(env: &Env, inst: &Instance) -> ContentIndex {
    load_index(
        &env.core
            .instance_dir(&inst.id)
            .unwrap()
            .join("content.json"),
    )
    .await
    .unwrap()
}

// ---- Ayudas para simular Modrinth ----

struct Ver<'a> {
    id: &'a str,
    project: &'a str,
    number: &'a str,
    file: &'a str,
    data: &'a [u8],
    deps: Vec<(&'a str, &'a str)>,
    date: &'a str,
    loader: &'a str,
    game: &'a str,
}

impl<'a> Ver<'a> {
    fn new(id: &'a str, project: &'a str, file: &'a str, data: &'a [u8]) -> Self {
        Self {
            id,
            project,
            number: "1.0.0",
            file,
            data,
            deps: Vec::new(),
            date: "2024-06-01T00:00:00Z",
            loader: "fabric",
            game: "1.21.1",
        }
    }

    fn json(&self, server: &MockServer) -> Value {
        json!({
            "id": self.id,
            "project_id": self.project,
            "name": self.number,
            "version_number": self.number,
            "game_versions": [self.game],
            "loaders": [self.loader],
            "version_type": "release",
            "date_published": self.date,
            "files": [{
                "hashes": {"sha1": sha1_hex(self.data), "sha512": "x"},
                "url": format!("{}/files/{}", server.uri(), self.file),
                "filename": self.file,
                "primary": true,
                "size": self.data.len(),
            }],
            "dependencies": self.deps.iter().map(|(p, t)| json!({
                "project_id": p, "version_id": null, "dependency_type": t
            })).collect::<Vec<_>>(),
        })
    }
}

async fn mount_file(server: &MockServer, name: &str, data: &[u8]) {
    Mock::given(method("GET"))
        .and(path(format!("/files/{name}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(data.to_vec()))
        .mount(server)
        .await;
}

async fn mount_project(server: &MockServer, id: &str, ptype: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/v2/project/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": id, "slug": id, "title": format!("Proyecto {id}"),
            "description": "d", "project_type": ptype, "downloads": 5, "followers": 1,
            "categories": [], "icon_url": null,
        })))
        .mount(server)
        .await;
}

async fn mount_versions(server: &MockServer, project: &str, versions: Vec<Value>) {
    Mock::given(method("GET"))
        .and(path(format!("/v2/project/{project}/version")))
        .respond_with(ResponseTemplate::new(200).set_body_json(versions))
        .mount(server)
        .await;
}

/// Proyecto con una versión y su archivo, listo para instalar.
async fn mount_simple(server: &MockServer, project: &str, ptype: &str, file: &str, data: &[u8]) {
    mount_project(server, project, ptype).await;
    mount_versions(
        server,
        project,
        vec![Ver::new(&format!("v-{project}"), project, file, data).json(server)],
    )
    .await;
    mount_file(server, file, data).await;
}

// ---- Cliente de Modrinth ----

#[test]
fn facets_se_codifican_como_la_api_espera() {
    let q = ContentQuery {
        kind: ContentKind::Mod,
        mc: Some("1.21.1".into()),
        loader: Some(LoaderKind::Fabric),
        ..ContentQuery::default()
    };
    assert_eq!(
        facets(&q),
        r#"[["project_type:mod"],["versions:1.21.1"],["categories:fabric"]]"#
    );
    // Quilt acepta también los mods de Fabric (grupo «o»).
    let q = ContentQuery {
        loader: Some(LoaderKind::Quilt),
        ..q
    };
    assert_eq!(
        facets(&q),
        r#"[["project_type:mod"],["versions:1.21.1"],["categories:quilt","categories:fabric"]]"#
    );
    // Los resource packs y shaders no se filtran por loader.
    let q = ContentQuery {
        kind: ContentKind::Shader,
        ..q
    };
    assert_eq!(
        facets(&q),
        r#"[["project_type:shader"],["versions:1.21.1"]]"#
    );
}

#[tokio::test]
async fn busca_en_modrinth_con_facets_y_orden() {
    let env = env().await;
    Mock::given(method("GET"))
        .and(path("/v2/search"))
        .and(query_param("query", "sodium"))
        .and(query_param(
            "facets",
            r#"[["project_type:mod"],["versions:1.21.1"],["categories:fabric"]]"#,
        ))
        .and(query_param("index", "downloads"))
        .and(query_param("limit", "5"))
        .and(query_param("offset", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hits": [{
                "project_id": "AANobbMI", "slug": "sodium", "project_type": "mod",
                "title": "Sodium", "description": "Rápido", "author": "jellysquid3",
                "icon_url": "", "downloads": 100, "follows": 7,
                "categories": ["fabric"], "date_modified": "2024-01-01T00:00:00Z"
            }],
            "offset": 10, "limit": 5, "total_hits": 42
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    let page = env
        .content
        .search(&ContentQuery {
            text: "sodium".into(),
            mc: Some("1.21.1".into()),
            loader: Some(LoaderKind::Fabric),
            offset: 10,
            limit: 5,
            sort: SortOrder::Downloads,
            ..ContentQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 42);
    assert_eq!(page.items.len(), 1);
    let hit = &page.items[0];
    assert_eq!((hit.id.as_str(), hit.slug.as_str()), ("AANobbMI", "sodium"));
    assert_eq!(hit.kind, Some(ContentKind::Mod));
    assert_eq!(hit.icon_url, None);
}

#[tokio::test]
async fn versiones_se_filtran_por_loader_y_minecraft() {
    let env = env().await;
    let ok = Ver::new("good", "sodium", "ok.jar", b"ok");
    let mut old_mc = Ver::new("old", "sodium", "old.jar", b"old");
    old_mc.game = "1.20.1";
    let mut forge = Ver::new("forge", "sodium", "forge.jar", b"forge");
    forge.loader = "forge";
    // El servidor «ignora» el filtro: el cliente vuelve a filtrar.
    Mock::given(method("GET"))
        .and(path("/v2/project/sodium/version"))
        .and(query_param("loaders", r#"["fabric"]"#))
        .and(query_param("game_versions", r#"["1.21.1"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(vec![
            old_mc.json(&env.server),
            forge.json(&env.server),
            ok.json(&env.server),
        ]))
        .expect(1)
        .mount(&env.server)
        .await;
    let versions = env
        .content
        .versions(
            Source::Modrinth,
            "sodium",
            Some("1.21.1"),
            Some(LoaderKind::Fabric),
        )
        .await
        .unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id, "good");
    assert_eq!(
        versions[0].files[0].sha1.as_deref(),
        Some(sha1_hex(b"ok").as_str())
    );
}

// ---- Instalación ----

#[tokio::test]
async fn instala_cadena_de_dependencias_y_salta_las_ya_instaladas() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    let game = game_dir(&env, &inst);

    // C ya está instalado (y registrado): no se pide ni su lista de versiones.
    std::fs::create_dir_all(game.join("mods")).unwrap();
    std::fs::write(game.join("mods/c.jar"), b"c").unwrap();
    let cdir = env.core.instance_dir(&inst.id).unwrap();
    save_index(
        &cdir.join("content.json"),
        &ContentIndex {
            schema: 1,
            files: vec![ContentFile {
                kind: "mod".into(),
                file: "c.jar".into(),
                enabled: true,
                source: "modrinth".into(),
                project_id: Some("C".into()),
                project_name: Some("C".into()),
                version_id: None,
                version_number: None,
                sha1: Some(sha1_hex(b"c")),
                url: None,
                category: None,
            }],
        },
    )
    .await
    .unwrap();

    // A: dos versiones; la nueva exige B (y recomienda X, que se ignora).
    let mut a_new = Ver::new("a2", "A", "a-new.jar", b"a-new");
    a_new.number = "2.0.0";
    a_new.deps = vec![("B", "required"), ("X", "optional")];
    let mut a_old = Ver::new("a1", "A", "a-old.jar", b"a-old");
    a_old.date = "2023-01-01T00:00:00Z";
    mount_project(&env.server, "A", "mod").await;
    mount_versions(
        &env.server,
        "A",
        vec![a_old.json(&env.server), a_new.json(&env.server)],
    )
    .await;
    mount_file(&env.server, "a-new.jar", b"a-new").await;
    // B exige C.
    let mut b = Ver::new("b1", "B", "b.jar", b"b");
    b.deps = vec![("C", "required")];
    mount_project(&env.server, "B", "mod").await;
    mount_versions(&env.server, "B", vec![b.json(&env.server)]).await;
    mount_file(&env.server, "b.jar", b"b").await;
    mount_project(&env.server, "C", "mod").await;

    let mut events = env.core.subscribe();
    let out = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    let names: Vec<_> = out.installed.iter().map(|i| i.file.as_str()).collect();
    assert_eq!(names, ["a-new.jar", "b.jar"]);
    assert!(!out.installed[0].dependency);
    assert!(out.installed[1].dependency);
    assert_eq!(out.skipped.len(), 1);
    assert_eq!(out.skipped[0].project_id, "C");
    assert!(game.join("mods/a-new.jar").is_file());
    assert!(game.join("mods/b.jar").is_file());
    assert!(!game.join("mods/a-old.jar").exists());

    let index = index_of(&env, &inst).await;
    assert_eq!(index.files.len(), 3);
    let a = index.find("mod", "a-new.jar").unwrap();
    assert_eq!(a.source, "modrinth");
    assert_eq!(a.project_id.as_deref(), Some("A"));
    assert_eq!(a.version_id.as_deref(), Some("a2"));
    assert_eq!(a.version_number.as_deref(), Some("2.0.0"));
    assert_eq!(a.sha1.as_deref(), Some(sha1_hex(b"a-new").as_str()));

    let mut progress = false;
    while let Ok(ev) = events.try_recv() {
        progress |= matches!(ev, Event::Progress { ref stage, .. } if stage == "content");
    }
    assert!(progress);

    // Volver a pedir el mismo proyecto no es silencioso.
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ya está instalado"), "{err}");
}

#[tokio::test]
async fn dependencias_circulares_no_dan_vueltas() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    let mut a = Ver::new("a1", "A", "a.jar", b"a");
    a.deps = vec![("B", "required")];
    let mut b = Ver::new("b1", "B", "b.jar", b"b");
    b.deps = vec![("A", "required")];
    mount_project(&env.server, "A", "mod").await;
    mount_project(&env.server, "B", "mod").await;
    mount_versions(&env.server, "A", vec![a.json(&env.server)]).await;
    mount_versions(&env.server, "B", vec![b.json(&env.server)]).await;
    mount_file(&env.server, "a.jar", b"a").await;
    mount_file(&env.server, "b.jar", b"b").await;
    let out = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    assert_eq!(out.installed.len(), 2);
    assert_eq!(out.skipped.len(), 1);
}

#[tokio::test]
async fn las_dependencias_se_cortan_a_tres_niveles() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    for i in 0..5 {
        let (id, file) = (format!("P{i}"), format!("p{i}.jar"));
        let next = format!("P{}", i + 1);
        let vid = format!("v{i}");
        let mut v = Ver::new(&vid, &id, &file, file.as_bytes());
        if i < 4 {
            v.deps = vec![(next.as_str(), "required")];
        }
        mount_project(&env.server, &id, "mod").await;
        mount_versions(&env.server, &id, vec![v.json(&env.server)]).await;
        mount_file(&env.server, &file, file.as_bytes()).await;
    }
    let out = env
        .content
        .install(&inst.id, Source::Modrinth, "P0", None, None)
        .await
        .unwrap();
    assert_eq!(out.installed.len(), 4, "raíz + 3 niveles");
    assert!(
        out.warnings.iter().any(|w| w.contains("3 niveles")),
        "{:?}",
        out.warnings
    );
    assert!(!game_dir(&env, &inst).join("mods/p4.jar").exists());
}

#[tokio::test]
async fn hash_incorrecto_no_deja_nada_instalado() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    let mut a = Ver::new("a1", "A", "a.jar", b"esperado");
    a.deps = vec![("B", "required")];
    mount_project(&env.server, "A", "mod").await;
    mount_project(&env.server, "B", "mod").await;
    mount_versions(&env.server, "A", vec![a.json(&env.server)]).await;
    mount_versions(
        &env.server,
        "B",
        vec![Ver::new("b1", "B", "b.jar", b"b").json(&env.server)],
    )
    .await;
    mount_file(&env.server, "a.jar", b"corrupto").await;
    mount_file(&env.server, "b.jar", b"b").await;
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::HashMismatch { .. }), "{err:?}");
    let game = game_dir(&env, &inst);
    assert!(!game.join("mods/a.jar").exists());
    assert!(
        !game.join("mods/b.jar").exists(),
        "se limpian también las demás"
    );
    assert!(index_of(&env, &inst).await.files.is_empty());
}

#[tokio::test]
async fn no_sobrescribe_archivos_ajenos_salvo_que_sean_identicos() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    mount_simple(&env.server, "A", "mod", "a.jar", b"real").await;
    let mods = game_dir(&env, &inst).join("mods");
    std::fs::create_dir_all(&mods).unwrap();

    std::fs::write(mods.join("a.jar"), b"otro contenido").unwrap();
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("ya existe"), "{err}");
    assert_eq!(
        std::fs::read(mods.join("a.jar")).unwrap(),
        b"otro contenido"
    );
    assert!(index_of(&env, &inst).await.files.is_empty());

    // Idéntico (aquí desactivado): se registra sin descargar.
    std::fs::remove_file(mods.join("a.jar")).unwrap();
    std::fs::write(mods.join("a.jar.disabled"), b"real").unwrap();
    let out = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    assert!(out.installed[0].already_present);
    let index = index_of(&env, &inst).await;
    assert!(!index.find("mod", "a.jar").unwrap().enabled);
    assert!(!mods.join("a.jar").exists());
}

#[tokio::test]
async fn resourcepacks_y_shaders_no_filtran_por_loader() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    for (id, ptype, file) in [("RP", "resourcepack", "rp.zip"), ("SH", "shader", "sh.zip")] {
        mount_project(&env.server, id, ptype).await;
        let vid = format!("v-{id}");
        let mut v = Ver::new(&vid, id, file, id.as_bytes());
        v.loader = if ptype == "shader" {
            "iris"
        } else {
            "minecraft"
        };
        Mock::given(method("GET"))
            .and(path(format!("/v2/project/{id}/version")))
            .and(query_param_is_missing("loaders"))
            .and(query_param("game_versions", r#"["1.21.1"]"#))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![v.json(&env.server)]))
            .expect(1)
            .mount(&env.server)
            .await;
        mount_file(&env.server, file, id.as_bytes()).await;
        env.content
            .install(&inst.id, Source::Modrinth, id, None, None)
            .await
            .unwrap();
    }
    let game = game_dir(&env, &inst);
    assert!(game.join("resourcepacks/rp.zip").is_file());
    assert!(game.join("shaderpacks/sh.zip").is_file());
    let index = index_of(&env, &inst).await;
    assert!(index.find("resourcepack", "rp.zip").is_some());
    assert!(index.find("shader", "sh.zip").is_some());
}

#[tokio::test]
async fn datapacks_necesitan_mundo() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    mount_project(&env.server, "DP", "datapack").await;
    let mut v = Ver::new("v-dp", "DP", "dp.zip", b"dp");
    v.loader = "datapack";
    mount_versions(&env.server, "DP", vec![v.json(&env.server)]).await;
    mount_file(&env.server, "dp.zip", b"dp").await;

    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "DP", None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("--world"), "{err}");
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "DP", None, Some("../x"))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid_path");
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "DP", None, Some("Mundo"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no existe el mundo"), "{err}");

    let game = game_dir(&env, &inst);
    std::fs::create_dir_all(game.join("saves/Mundo")).unwrap();
    env.content
        .install(&inst.id, Source::Modrinth, "DP", None, Some("Mundo"))
        .await
        .unwrap();
    assert!(game.join("saves/Mundo/datapacks/dp.zip").is_file());
    let index = index_of(&env, &inst).await;
    assert!(index.find("datapack", "Mundo/dp.zip").is_some());

    let list = env.content.list(&inst.id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].world.as_deref(), Some("Mundo"));
    assert!(list[0].tracked);
}

#[tokio::test]
async fn una_instancia_vanilla_no_acepta_mods() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Vanilla).await;
    mount_simple(&env.server, "A", "mod", "a.jar", b"a").await;
    let err = env
        .content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("vanilla"), "{err}");
}

// ---- Lista, activar, quitar ----

#[tokio::test]
async fn lista_mezcla_disco_y_registro_y_conmuta_y_quita() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    mount_simple(&env.server, "A", "mod", "a.jar", b"a").await;
    env.content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    let game = game_dir(&env, &inst);
    std::fs::write(game.join("mods/manual.jar"), b"m").unwrap();
    std::fs::write(game.join("mods/off.jar.disabled"), b"o").unwrap();
    std::fs::write(game.join("mods/leeme.txt"), b"no es un mod").unwrap();
    std::fs::create_dir_all(game.join("resourcepacks/carpeta")).unwrap();
    std::fs::write(game.join("resourcepacks/p.zip"), b"p").unwrap();

    let list = env.content.list(&inst.id).await.unwrap();
    let files: Vec<_> = list
        .iter()
        .map(|c| (c.kind.as_str(), c.file.as_str()))
        .collect();
    assert_eq!(
        files,
        [
            ("mod", "a.jar"),
            ("mod", "manual.jar"),
            ("mod", "off.jar"),
            ("resourcepack", "carpeta"),
            ("resourcepack", "p.zip"),
        ]
    );
    let a = &list[0];
    assert!(a.tracked && a.enabled && a.source == "modrinth");
    let manual = &list[1];
    assert!(!manual.tracked && manual.source == "unknown");
    assert!(!list[2].enabled);

    // Desactivar / activar un archivo registrado.
    let item = env
        .content
        .set_enabled(&inst.id, "a.jar", false)
        .await
        .unwrap();
    assert!(!item.enabled);
    assert!(game.join("mods/a.jar.disabled").is_file());
    assert!(!game.join("mods/a.jar").exists());
    assert!(
        !index_of(&env, &inst)
            .await
            .find("mod", "a.jar")
            .unwrap()
            .enabled
    );
    // Idempotente, y acepta el nombre con `.disabled` y con prefijo de tipo.
    env.content
        .set_enabled(&inst.id, "mod/a.jar.disabled", false)
        .await
        .unwrap();
    env.content
        .set_enabled(&inst.id, "a.jar", true)
        .await
        .unwrap();
    assert!(game.join("mods/a.jar").is_file());
    assert!(
        index_of(&env, &inst)
            .await
            .find("mod", "a.jar")
            .unwrap()
            .enabled
    );

    // Un archivo sin registrar se renombra sin crear entrada.
    env.content
        .set_enabled(&inst.id, "manual.jar", false)
        .await
        .unwrap();
    assert!(game.join("mods/manual.jar.disabled").is_file());
    assert!(
        index_of(&env, &inst)
            .await
            .find("mod", "manual.jar")
            .is_none()
    );

    // Quitar.
    env.content
        .set_enabled(&inst.id, "a.jar", false)
        .await
        .unwrap();
    let removed = env.content.remove(&inst.id, "a.jar").await.unwrap();
    assert_eq!(removed.file, "a.jar");
    assert!(!game.join("mods/a.jar.disabled").exists());
    assert!(index_of(&env, &inst).await.files.is_empty());

    let err = env
        .content
        .remove(&inst.id, "no-existe.jar")
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid_input");
}

// ---- Actualizaciones e identificación ----

#[tokio::test]
async fn busca_y_aplica_actualizaciones() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    mount_simple(&env.server, "A", "mod", "a-1.jar", b"viejo").await;
    env.content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    let game = game_dir(&env, &inst);
    env.content
        .set_enabled(&inst.id, "a-1.jar", false)
        .await
        .unwrap();

    let mut new = Ver::new("v2", "A", "a-2.jar", b"nuevo");
    new.number = "2.0.0";
    let old_hash = sha1_hex(b"viejo");
    Mock::given(method("POST"))
        .and(path("/v2/version_files/update"))
        .and(body_partial_json(json!({
            "hashes": [old_hash],
            "algorithm": "sha1",
            "loaders": ["fabric"],
            "game_versions": ["1.21.1"],
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ old_hash.clone(): new.json(&env.server) })),
        )
        .expect(2)
        .mount(&env.server)
        .await;
    mount_file(&env.server, "a-2.jar", b"nuevo").await;

    let updates = env.content.check_updates(&inst.id).await.unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].file, "a-1.jar");
    assert_eq!(updates[0].latest.version_number, "2.0.0");
    assert!(!updates[0].enabled);

    let done = env
        .content
        .update(&inst.id, "a-1.jar")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.file, "a-2.jar");
    assert_eq!(done.to_version, "2.0.0");
    // Sigue desactivado y el archivo viejo desaparece.
    assert!(game.join("mods/a-2.jar.disabled").is_file());
    assert!(!game.join("mods/a-1.jar.disabled").exists());
    let index = index_of(&env, &inst).await;
    assert!(index.find("mod", "a-1.jar").is_none());
    let e = index.find("mod", "a-2.jar").unwrap();
    assert!(!e.enabled);
    assert_eq!(e.version_id.as_deref(), Some("v2"));
    assert_eq!(e.sha1.as_deref(), Some(sha1_hex(b"nuevo").as_str()));
    assert_eq!(e.project_name.as_deref(), Some("Proyecto A"));
}

#[tokio::test]
async fn sin_actualizaciones_cuando_ya_es_la_ultima() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    mount_simple(&env.server, "A", "mod", "a.jar", b"actual").await;
    env.content
        .install(&inst.id, Source::Modrinth, "A", None, None)
        .await
        .unwrap();
    let h = sha1_hex(b"actual");
    let same = Ver::new("v-A", "A", "a.jar", b"actual").json(&env.server);
    Mock::given(method("POST"))
        .and(path("/v2/version_files/update"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ h: same })))
        .mount(&env.server)
        .await;
    assert!(
        env.content
            .check_updates(&inst.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        env.content
            .update(&inst.id, "a.jar")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn identifica_archivos_desconocidos_por_sha1() {
    let env = env().await;
    let inst = instance(&env, "1.21.1", LoaderKind::Fabric).await;
    let game = game_dir(&env, &inst);
    std::fs::create_dir_all(game.join("mods")).unwrap();
    std::fs::write(game.join("mods/raro.jar"), b"conocido").unwrap();
    std::fs::write(game.join("mods/casero.jar.disabled"), b"hecho a mano").unwrap();

    let h = sha1_hex(b"conocido");
    let v = Ver::new("v9", "P9", "sodium.jar", b"conocido");
    Mock::given(method("POST"))
        .and(path("/v2/version_files"))
        .and(body_partial_json(json!({ "algorithm": "sha1" })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ h.clone(): v.json(&env.server) })),
        )
        .expect(1)
        .mount(&env.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/projects"))
        .and(query_param("ids", r#"["P9"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "id": "P9", "slug": "sodium", "title": "Sodium", "description": "",
            "project_type": "mod", "downloads": 1, "followers": 1, "categories": []
        }])))
        .mount(&env.server)
        .await;

    let report = env.content.identify(&inst.id).await.unwrap();
    assert_eq!(report.identified.len(), 1);
    assert_eq!(report.identified[0].project_name, "Sodium");
    assert_eq!(report.unmatched, ["casero.jar"]);
    let index = index_of(&env, &inst).await;
    let e = index.find("mod", "raro.jar").unwrap();
    assert_eq!(e.source, "modrinth");
    assert_eq!(e.project_id.as_deref(), Some("P9"));
    assert_eq!(e.sha1.as_deref(), Some(h.as_str()));
    assert!(index.find("mod", "casero.jar").is_none());
    let list = env.content.list(&inst.id).await.unwrap();
    assert!(list.iter().find(|c| c.file == "raro.jar").unwrap().tracked);
}

// ---- content.json ----

#[test]
fn content_json_conserva_el_formato() {
    let raw = r#"{
  "schema": 1,
  "files": [
    {
      "type": "mod",
      "file": "x.jar",
      "enabled": false,
      "source": "unknown",
      "project_id": null,
      "project_name": null,
      "version_number": null,
      "sha1": null,
      "url": null,
      "category": null
    }
  ]
}"#;
    let idx: ContentIndex = serde_json::from_str(raw).unwrap();
    assert_eq!(idx.files[0].kind, "mod");
    assert_eq!(idx.files[0].version_id, None);
    let back = serde_json::to_string_pretty(&idx).unwrap();
    assert_eq!(back, raw);
}

// ---- Modpacks de Modrinth ----

fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        for (name, data) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf.into_inner()
}

fn mr_file(path: &str, url: &str, data: &[u8], client: Option<&str>) -> Value {
    let mut f = json!({
        "path": path,
        "hashes": {"sha1": sha1_hex(data), "sha512": "x"},
        "downloads": [url],
        "fileSize": data.len(),
    });
    if let Some(c) = client {
        f["env"] = json!({"client": c, "server": "required"});
    }
    f
}

fn mr_index(files: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0",
        "name": "Mi pack",
        "files": files,
        "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.5"},
    }))
    .unwrap()
}

async fn serve_pack(env: &Env, name: &str, zip: Vec<u8>) -> String {
    Mock::given(method("GET"))
        .and(path(format!("/packs/{name}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip))
        .mount(&env.server)
        .await;
    format!("{}/packs/{name}", env.server.uri())
}

#[tokio::test]
async fn instala_un_mrpack_desde_url() {
    let env = env().await;
    let ok = b"mod bueno";
    let res = b"resource pack";
    mount_file(&env.server, "ok.jar", ok).await;
    mount_file(&env.server, "rp.zip", res).await;
    // No se debe pedir: cliente no soportado.
    Mock::given(method("GET"))
        .and(path("/files/solo-servidor.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes("x"))
        .expect(0)
        .mount(&env.server)
        .await;
    let base = format!("{}/files", env.server.uri());
    let index = mr_index(vec![
        mr_file(
            "mods/ok.jar",
            &format!("{base}/ok.jar"),
            ok,
            Some("required"),
        ),
        mr_file("resourcepacks/rp.zip", &format!("{base}/rp.zip"), res, None),
        mr_file(
            "mods/solo-servidor.jar",
            &format!("{base}/solo-servidor.jar"),
            b"x",
            Some("unsupported"),
        ),
        // Servidor no permitido: se omite con aviso.
        mr_file(
            "mods/ajeno.jar",
            "https://evil.example.com/ajeno.jar",
            b"y",
            Some("required"),
        ),
    ]);
    let zip = build_zip(&[
        ("modrinth.index.json", &index),
        ("overrides/config/a.txt", b"base"),
        ("overrides/config/solo-base.txt", b"base"),
        ("client-overrides/config/a.txt", b"cliente"),
        ("overrides/../../escape.txt", b"zip-slip"),
    ]);
    // La entrada maliciosa debe estar de verdad en el zip para que la prueba valga.
    let archive = zip::ZipArchive::new(std::io::Cursor::new(zip.clone())).unwrap();
    assert!(
        archive
            .file_names()
            .any(|n| n.is_ok_and(|n| n.contains("../..")))
    );
    let url = serve_pack(&env, "pack.mrpack", zip).await;

    let mut events = env.core.subscribe();
    let out = env.content.install_mrpack(&url, None).await.unwrap();
    let inst = &out.instance;
    assert_eq!(inst.name, "Mi pack");
    assert_eq!(inst.game.mc, "1.21.1");
    assert_eq!(inst.game.loader, LoaderKind::Fabric);
    assert_eq!(inst.game.loader_version.as_deref(), Some("0.16.5"));
    assert_eq!(out.files_downloaded, 2);
    assert_eq!(out.warnings.len(), 1);
    assert!(out.warnings[0].contains("ajeno.jar"), "{:?}", out.warnings);

    let game = game_dir(&env, inst);
    assert_eq!(std::fs::read(game.join("mods/ok.jar")).unwrap(), ok);
    assert_eq!(
        std::fs::read(game.join("resourcepacks/rp.zip")).unwrap(),
        res
    );
    assert!(!game.join("mods/solo-servidor.jar").exists());
    assert!(!game.join("mods/ajeno.jar").exists());
    // `client-overrides` pisa a `overrides`.
    assert_eq!(
        std::fs::read(game.join("config/a.txt")).unwrap(),
        b"cliente"
    );
    assert_eq!(
        std::fs::read(game.join("config/solo-base.txt")).unwrap(),
        b"base"
    );
    // Zip-slip: nada fuera de la instancia.
    assert!(!env.dir.path().join("escape.txt").exists());
    assert!(
        !env.core
            .instance_dir(&inst.id)
            .unwrap()
            .join("escape.txt")
            .exists()
    );
    assert!(!env.dir.path().join("instances/escape.txt").exists());

    let index = index_of(&env, inst).await;
    assert_eq!(index.files.len(), 2);
    let m = index.find("mod", "ok.jar").unwrap();
    assert_eq!(m.sha1.as_deref(), Some(sha1_hex(ok).as_str()));
    assert!(index.find("resourcepack", "rp.zip").is_some());
    // Se guardó una copia en la caché de modpacks.
    assert!(env.dir.path().join("cache/modpacks").is_dir());

    let mut seen = false;
    while let Ok(ev) = events.try_recv() {
        seen |= matches!(ev, Event::Progress { ref stage, .. } if stage == "modpack");
    }
    assert!(seen, "debe emitir progreso con stage modpack");
}

#[tokio::test]
async fn mrpack_con_hash_incorrecto_falla_y_limpia() {
    let env = env().await;
    mount_file(&env.server, "bueno.jar", b"bueno").await;
    mount_file(&env.server, "malo.jar", b"contenido distinto").await;
    let base = format!("{}/files", env.server.uri());
    let index = mr_index(vec![
        mr_file(
            "mods/bueno.jar",
            &format!("{base}/bueno.jar"),
            b"bueno",
            None,
        ),
        mr_file(
            "mods/malo.jar",
            &format!("{base}/malo.jar"),
            b"el hash es de otro",
            None,
        ),
    ]);
    let zip = build_zip(&[("modrinth.index.json", &index), ("overrides/a.txt", b"a")]);
    let path = env.dir.path().join("malo.mrpack");
    std::fs::write(&path, zip).unwrap();

    let err = env
        .content
        .install_mrpack(path.to_str().unwrap(), None)
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::HashMismatch { .. }), "{err:?}");
    assert!(env.core.list_instances().await.unwrap().is_empty());
    let leftovers: Vec<_> = std::fs::read_dir(env.dir.path().join("instances"))
        .map(|d| d.map(|e| e.unwrap().file_name()).collect())
        .unwrap_or_default();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[tokio::test]
async fn mrpack_con_ruta_insegura_se_rechaza_sin_crear_nada() {
    let env = env().await;
    mount_file(&env.server, "evil.jar", b"evil").await;
    let base = format!("{}/files", env.server.uri());
    for bad in [
        "../evil.jar",
        "mods/../../evil.jar",
        "/etc/evil.jar",
        "mods\\..\\x.jar",
    ] {
        let index = mr_index(vec![mr_file(
            bad,
            &format!("{base}/evil.jar"),
            b"evil",
            None,
        )]);
        let path = env.dir.path().join("evil.mrpack");
        std::fs::write(&path, build_zip(&[("modrinth.index.json", &index)])).unwrap();
        let err = env
            .content
            .install_mrpack(path.to_str().unwrap(), None)
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "invalid_path", "{bad}: {err:?}");
        assert!(env.core.list_instances().await.unwrap().is_empty());
    }
    assert!(!env.dir.path().join("evil.jar").exists());
}

#[tokio::test]
async fn install_modpack_detecta_el_formato() {
    let env = env().await;
    let index = mr_index(vec![]);
    let path = env.dir.path().join("auto.zip");
    std::fs::write(&path, build_zip(&[("modrinth.index.json", &index)])).unwrap();
    let out = env
        .content
        .install_modpack(path.to_str().unwrap(), Some("Auto".into()))
        .await
        .unwrap();
    assert_eq!(out.instance.name, "Auto");
    assert_eq!(out.files_total, 0);

    std::fs::write(&path, build_zip(&[("otra-cosa.txt", b"x")])).unwrap();
    let err = env
        .content
        .install_modpack(path.to_str().unwrap(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no es un modpack"), "{err}");
    assert_eq!(env.core.list_instances().await.unwrap().len(), 1);
}

#[tokio::test]
async fn mrpack_invalido_se_rechaza() {
    let env = env().await;
    let path = env.dir.path().join("x.mrpack");
    std::fs::write(&path, build_zip(&[("otra-cosa.txt", b"x")])).unwrap();
    let err = env
        .content
        .install_mrpack(path.to_str().unwrap(), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("modrinth.index.json"), "{err}");

    let bad = serde_json::to_vec(&json!({
        "formatVersion": 2, "game": "minecraft", "files": [], "dependencies": {"minecraft": "1.21"}
    }))
    .unwrap();
    std::fs::write(&path, build_zip(&[("modrinth.index.json", &bad)])).unwrap();
    let err = env
        .content
        .install_mrpack(path.to_str().unwrap(), Some("Nombre".into()))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("formatVersion"), "{err}");
    assert!(env.core.list_instances().await.unwrap().is_empty());
}

// ---- CurseForge ----

#[tokio::test]
async fn curseforge_sin_clave_da_error_claro() {
    let env = env().await;
    let err = env
        .content
        .search(&ContentQuery {
            source: Source::CurseForge,
            text: "jei".into(),
            ..ContentQuery::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::CurseForgeKeyMissing));
    assert_eq!(err.kind(), "curseforge_key_missing");
    assert!(!env.core.has_curseforge_key().await.unwrap());
    // Una clave en blanco no cuenta.
    assert!(env.core.set_curseforge_key("   ").await.is_err());
}

fn cf_file(id: u64, mod_id: u64, name: &str, url: Option<String>, data: &[u8]) -> Value {
    json!({
        "id": id, "modId": mod_id, "displayName": name, "fileName": name,
        "releaseType": 1, "fileDate": "2024-05-01T00:00:00Z", "fileLength": data.len(),
        "downloadUrl": url,
        "gameVersions": ["1.21.1", "Fabric"],
        "hashes": [{"value": sha1_hex(data), "algo": 1}, {"value": "md5", "algo": 2}],
        "dependencies": [{"modId": 999, "relationType": 3}, {"modId": 5, "relationType": 2}],
    })
}

#[tokio::test]
async fn curseforge_busca_con_la_clave_del_keyring() {
    let env = env().await;
    env.core
        .set_curseforge_key("  clave-de-prueba ")
        .await
        .unwrap();
    assert!(env.core.has_curseforge_key().await.unwrap());
    Mock::given(method("GET"))
        .and(path("/cf/v1/mods/search"))
        .and(header("x-api-key", "clave-de-prueba"))
        .and(query_param("gameId", "432"))
        .and(query_param("classId", "6"))
        .and(query_param("searchFilter", "jei"))
        .and(query_param("gameVersion", "1.21.1"))
        .and(query_param("modLoaderType", "4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{
                "id": 238222, "slug": "jei", "name": "JEI", "summary": "Items",
                "authors": [{"name": "mezz"}], "logo": {"url": "https://x/l.png"},
                "downloadCount": 1.5e8, "thumbsUpCount": 3, "classId": 6,
                "categories": [{"name": "Utilidades"}], "dateModified": "2024-01-01T00:00:00Z"
            }],
            "pagination": {"index": 0, "pageSize": 20, "resultCount": 1, "totalCount": 1}
        })))
        .expect(1)
        .mount(&env.server)
        .await;
    let page = env
        .content
        .search(&ContentQuery {
            source: Source::CurseForge,
            text: "jei".into(),
            mc: Some("1.21.1".into()),
            loader: Some(LoaderKind::Fabric),
            ..ContentQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 1);
    let hit = &page.items[0];
    assert_eq!(hit.id, "238222");
    assert_eq!(hit.source, Source::CurseForge);
    assert_eq!(hit.kind, Some(ContentKind::Mod));
    assert_eq!(hit.author.as_deref(), Some("mezz"));
    assert_eq!(hit.downloads, 150_000_000);
}

#[tokio::test]
async fn curseforge_usa_la_url_del_cdn_si_falta_downloadurl() {
    let env = env().await;
    env.core.set_curseforge_key("k").await.unwrap();
    Mock::given(method("GET"))
        .and(path("/cf/v1/mods/42/files"))
        .and(query_param("gameVersion", "1.21.1"))
        .and(query_param("modLoaderType", "4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                cf_file(5_123_456, 42, "sin-url.jar", None, b"a"),
                cf_file(5_123_457, 42, "con-url.jar", Some("https://mediafilez.forgecdn.net/x.jar".into()), b"b"),
            ],
            "pagination": {"index": 0, "pageSize": 50, "resultCount": 2, "totalCount": 2}
        })))
        .mount(&env.server)
        .await;
    let versions = env
        .content
        .versions(
            Source::CurseForge,
            "42",
            Some("1.21.1"),
            Some(LoaderKind::Fabric),
        )
        .await
        .unwrap();
    assert_eq!(versions.len(), 2);
    let f = &versions[0].files[0];
    assert_eq!(f.filename, "sin-url.jar");
    assert_eq!(
        f.url,
        "https://edge.forgecdn.net/files/5123/456/sin-url.jar"
    );
    assert!(f.fallback_url);
    assert_eq!(f.sha1.as_deref(), Some(sha1_hex(b"a").as_str()));
    let g = &versions[1].files[0];
    assert!(!g.fallback_url);
    assert_eq!(g.url, "https://mediafilez.forgecdn.net/x.jar");
    assert_eq!(versions[0].loaders, ["fabric"]);
    assert_eq!(versions[0].mc_versions, ["1.21.1"]);
    // Dependencias: 3 = obligatoria, 2 = opcional.
    assert_eq!(versions[0].required_dependencies().count(), 1);
    assert_eq!(
        versions[0].dependencies[0].project_id.as_deref(),
        Some("999")
    );
}

#[tokio::test]
async fn instala_un_modpack_de_curseforge() {
    let env = env().await;
    let manifest = |files: Value| {
        serde_json::to_vec(&json!({
            "minecraft": {"version": "1.20.1", "modLoaders": [
                {"id": "forge-47.2.0", "primary": true}
            ]},
            "manifestType": "minecraftModpack", "manifestVersion": 1,
            "name": "Pack CF", "overrides": "overrides", "files": files,
        }))
        .unwrap()
    };
    let files = json!([
        {"projectID": 10, "fileID": 100, "required": true},
        {"projectID": 20, "fileID": 200, "required": true},
        {"projectID": 30, "fileID": 300, "required": false},
    ]);
    let zip = build_zip(&[
        ("manifest.json", &manifest(files.clone())),
        ("overrides/options.txt", b"opciones"),
        ("overrides/../../escape.txt", b"zip-slip"),
    ]);
    let zip_path = env.dir.path().join("cf.zip");
    std::fs::write(&zip_path, &zip).unwrap();

    // Sin clave: error antes de crear la instancia.
    let err = env
        .content
        .install_curseforge_pack(&zip_path, None)
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::CurseForgeKeyMissing));
    assert!(env.core.list_instances().await.unwrap().is_empty());

    env.core.set_curseforge_key("clave").await.unwrap();
    mount_file(&env.server, "mod.jar", b"mod").await;
    mount_file(&env.server, "pack.zip", b"rp").await;
    let url = |n: &str| Some(format!("{}/files/{n}", env.server.uri()));
    Mock::given(method("POST"))
        .and(path("/cf/v1/mods/files"))
        .and(header("x-api-key", "clave"))
        .and(body_partial_json(json!({"fileIds": [100, 200, 300]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
            cf_file(100, 10, "mod.jar", url("mod.jar"), b"mod"),
            cf_file(200, 20, "pack.zip", url("pack.zip"), b"rp"),
        ]})))
        .mount(&env.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/cf/v1/mods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
            {"id": 10, "slug": "m", "name": "Mod", "summary": "", "classId": 6},
            {"id": 20, "slug": "r", "name": "Textures", "summary": "", "classId": 12},
        ]})))
        .mount(&env.server)
        .await;

    let out = env
        .content
        .install_curseforge_pack(&zip_path, None)
        .await
        .unwrap();
    let inst = &out.instance;
    assert_eq!(inst.name, "Pack CF");
    assert_eq!(inst.game.loader, LoaderKind::Forge);
    assert_eq!(inst.game.loader_version.as_deref(), Some("47.2.0"));
    assert_eq!(out.files_downloaded, 2);
    // El archivo 300 es opcional y ya no existe: solo aviso.
    assert!(
        out.warnings.iter().any(|w| w.contains("300")),
        "{:?}",
        out.warnings
    );
    let game = game_dir(&env, inst);
    assert_eq!(std::fs::read(game.join("mods/mod.jar")).unwrap(), b"mod");
    assert_eq!(
        std::fs::read(game.join("resourcepacks/pack.zip")).unwrap(),
        b"rp"
    );
    assert_eq!(
        std::fs::read(game.join("options.txt")).unwrap(),
        b"opciones"
    );
    assert!(!env.dir.path().join("escape.txt").exists());
    let index = index_of(&env, inst).await;
    let e = index.find("mod", "mod.jar").unwrap();
    assert_eq!(e.source, "curseforge");
    assert_eq!(e.project_id.as_deref(), Some("10"));
    assert_eq!(e.version_id.as_deref(), Some("100"));
    assert!(index.find("resourcepack", "pack.zip").is_some());

    // Si falta un archivo obligatorio, falla y no deja instancia.
    let before = env.core.list_instances().await.unwrap().len();
    let files = json!([{"projectID": 10, "fileID": 777, "required": true}]);
    Mock::given(method("POST"))
        .and(path("/cf/v1/mods/files"))
        .and(body_partial_json(json!({"fileIds": [777]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": []})))
        .mount(&env.server)
        .await;
    std::fs::write(&zip_path, build_zip(&[("manifest.json", &manifest(files))])).unwrap();
    let err = env
        .content
        .install_curseforge_pack(&zip_path, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("777"), "{err}");
    assert_eq!(env.core.list_instances().await.unwrap().len(), before);
}

// ---- Red real ----

async fn real_endpoints_core() -> (Core, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let core = Core::open(Options {
        data_dir: Some(dir.path().to_path_buf()),
        secrets: Some(Arc::new(MemoryStore::default())),
        ..Options::default()
    })
    .await
    .unwrap();
    (core, dir)
}

#[tokio::test]
#[ignore = "usa la red (api.modrinth.com)"]
async fn red_real_busca_sodium() {
    let (core, _dir) = real_endpoints_core().await;
    let page = core
        .search_content(&ContentQuery {
            text: "sodium".into(),
            kind: ContentKind::Mod,
            mc: Some("1.21.1".into()),
            loader: Some(LoaderKind::Fabric),
            limit: 5,
            ..ContentQuery::default()
        })
        .await
        .unwrap();
    assert!(!page.items.is_empty());
    assert!(
        page.items.iter().any(|h| h.slug == "sodium"),
        "{:?}",
        page.items
    );
}

#[tokio::test]
#[ignore = "usa la red (api.modrinth.com, cdn.modrinth.com)"]
async fn red_real_instala_fabric_api() {
    let (core, dir) = real_endpoints_core().await;
    let inst = core
        .create_instance(NewInstance {
            name: "Fabric 1.21.1".into(),
            game: GameVersion {
                mc: "1.21.1".into(),
                loader: LoaderKind::Fabric,
                loader_version: None,
            },
        })
        .await
        .unwrap();
    let out = core
        .install_content(&inst.id, Source::Modrinth, "fabric-api", None, None)
        .await
        .unwrap();
    assert_eq!(out.installed.len(), 1, "{out:?}");
    let mods = dir
        .path()
        .join("instances")
        .join(&inst.id)
        .join("game/mods");
    let jars: Vec<_> = std::fs::read_dir(&mods)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(jars.len(), 1, "{jars:?}");
    assert!(jars[0].starts_with("fabric-api"), "{jars:?}");
    let list = core.list_content(&inst.id).await.unwrap();
    assert!(list[0].tracked && list[0].source == "modrinth");
}
