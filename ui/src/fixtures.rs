//! Datos falsos de `app/test/support/fake_bridge.dart` (los de los goldens),
//! para el arnés de paridad y los tests de interacción. Mismos ids, nombres,
//! versiones, tiempos y fechas relativas a `NOW`.

use crate::instances::Record;

// Fase 3: un archivo por pestaña o diálogo (`fixtures/<nombre>.rs`).
pub mod backups;
pub mod inst_content;
pub mod detail;
pub mod logs;
pub mod screenshots;
pub mod trash;
pub mod versions;
pub mod worlds;

/// «Ahora» de las escenas. Solo importan las diferencias con él.
pub const NOW: i64 = 1_791_822_600;

const H: i64 = 3600;
const D: i64 = 86_400;

fn instance(id: &str, name: &str, mc: &str, loader: &str, ago: Option<i64>, minutes: u64) -> Record {
    Record {
        id: id.into(),
        name: name.into(),
        mc: mc.into(),
        loader: loader.into(),
        created_at: NOW - 200 * D,
        last_played: ago.map(|a| NOW - a),
        playtime_seconds: minutes * 60,
    }
}

/// `sampleInstances()`.
pub fn sample_instances() -> Vec<Record> {
    vec![
        instance("survival-amigos", "Survival con amigos", "1.21.4", "fabric", Some(2 * H), 86 * 60 + 12),
        instance("create-aeronautica", "Create: Aeronáutica", "1.20.1", "forge", Some(3 * D), 41 * 60),
        instance("vanilla-puro", "Vanilla puro", "1.21.4", "vanilla", Some(D + 2 * H), 640),
        instance("skyblock-tecnico", "Skyblock técnico", "1.21.1", "neoforge", Some(64 * D), 19 * 60 + 45),
        instance("quilt-pruebas", "Pruebas de shaders", "1.21.5", "quilt", None, 0),
    ]
}

/// Cuenta activa de `sampleAccounts()` (Microsoft, sin skin: cara generada).
pub const ACTIVE_ACCOUNT: &str = "NadiaK";

/// Escribe con `lcore` las instancias de ejemplo en una carpeta de datos
/// (fechas relativas a la hora real), con «cerrar a la bandeja» y español.
/// Flutter lee la misma carpeta (`SHARD_DATA_DIR`): así `tools/medir.sh`
/// mide las dos apps con los mismos datos.
pub async fn seed(core: &lcore::Core) -> lcore::Result<()> {
    use lcore::instance::{GameVersion, INSTANCE_SCHEMA, Instance, LoaderKind};
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    for r in sample_instances() {
        let loader = match r.loader.as_str() {
            "fabric" => LoaderKind::Fabric,
            "forge" => LoaderKind::Forge,
            "neoforge" => LoaderKind::NeoForge,
            "quilt" => LoaderKind::Quilt,
            _ => LoaderKind::Vanilla,
        };
        let i = Instance {
            schema: INSTANCE_SCHEMA,
            id: r.id,
            name: r.name,
            game: GameVersion { mc: r.mc, loader, loader_version: None },
            created_at: r.created_at - NOW + now,
            last_played: r.last_played.map(|t| t - NOW + now),
            playtime_seconds: r.playtime_seconds,
            java: Default::default(),
            window: Default::default(),
            backups: Default::default(),
            dev_console: false,
            imported_from: None,
        };
        core.save_instance(&i).await?;
    }
    core.update_settings(|s| {
        s.close_behavior = lcore::settings::CloseBehavior::Tray;
        s.language = Some("es".into());
    })
    .await?;
    // Flutter: sin primer arranque (tema grafito, el de los goldens).
    std::fs::write(core.paths().root().join("ui.json"), br#"{"firstRunDone":true}"#)
        .map_err(|source| lcore::Error::Io { path: core.paths().root().join("ui.json"), source })?;
    Ok(())
}

/// `sampleServers()` (el cuarto no tiene instancia).
pub fn sample_servers() -> Vec<crate::home::Server> {
    let s = |id: &str, name: &str, address: &str, instance: Option<&str>| crate::home::Server {
        id: id.into(),
        name: name.into(),
        address: address.into(),
        instance_id: instance.map(Into::into),
        account_id: None,
    };
    vec![
        s("s1", "Servidor de los amigos", "mc.amigos.example", Some("survival-amigos")),
        s("s2", "Hypixel", "mc.hypixel.net", Some("vanilla-puro")),
        s("s3", "Create SMP", "create.smp.example:25570", Some("create-aeronautica")),
        s("s4", "Servidor de pruebas", "localhost", None),
    ]
}

/// `sampleStatuses()` (sin favicon: se ve el paisaje de la dirección).
pub fn sample_statuses() -> Vec<(String, crate::home::Status)> {
    let st = |latency_ms, players_online, players_max, version: &str, motd: &str, sample: &[&str]| crate::home::Status {
        online: true,
        latency_ms,
        players_online,
        players_max,
        favicon_png_base64: None,
        version_name: version.into(),
        motd: motd.into(),
        sample: sample.iter().map(|s| (*s).into()).collect(),
    };
    vec![
        ("mc.amigos.example".into(), st(23, 3, 20, "1.21.4", "Temporada 4 · ¡Nueva aldea en el spawn!", &["NadiaK", "Rulo", "Mai"])),
        ("mc.hypixel.net".into(), st(118, 31_842, 200_000, "Requires MC 1.8 / 1.21", "Hypixel Network  [1.8-1.21]", &[])),
        ("create.smp.example:25570".into(), st(64, 0, 12, "1.20.1", "Create SMP — trenes, molinos y mucha paciencia", &[])),
    ]
}

/// `appImageUpdate` de test/support/updates.dart.
pub fn sample_update() -> crate::notices::UpdateView {
    crate::notices::UpdateView {
        version: "0.2.0.r40".into(),
        current: "0.1.0.r27".into(),
        notes: Some("Actualizador integrado y arreglos de la bandeja.".into()),
        notes_url: Some("https://shardlauncher.com/novedades".into()),
        can_install: true,
        download_url: "https://dl.shardlauncher.com/linux/0.2.0.r40/Shard-0.2.0.r40-x86_64.AppImage".into(),
    }
}

// --- Cuentas y skins (fase 2) ------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;

use crate::accounts::{Account, AccountActions, DeviceCode};
use crate::skin::{SAMPLE_CAPE_1, SAMPLE_CAPE_2, SkinTexture, sample_cape, sample_skin};
use crate::skins::{Apply, CapeRef, Profile, SkinActions, SkinRef};

fn account(id: &str, name: &str, microsoft: bool, active: bool) -> Account {
    Account { id: id.into(), name: name.into(), microsoft, unavailable: false, skin_url: None, active }
}

/// `sampleAccounts()`: NadiaK (Microsoft, activa) y Pruebas_01 (offline).
pub fn sample_accounts() -> Vec<Account> {
    vec![account("a1", ACTIVE_ACCOUNT, true, true), account("a2", "Pruebas_01", false, false)]
}

/// Solo la cuenta offline, activa (escena `skins_offline_en`).
pub fn offline_accounts() -> Vec<Account> {
    vec![account("o1", "Pruebas_01", false, true)]
}

pub const SKIN_URL: &str = "http://textures.minecraft.net/texture/muestra-skin";
pub const CAPE_1_URL: &str = "http://textures.minecraft.net/texture/muestra-capa-1";
pub const CAPE_2_URL: &str = "http://textures.minecraft.net/texture/muestra-capa-2";

/// `sampleProfile()`: skin delgada y dos capas, la primera puesta.
pub fn sample_profile() -> Profile {
    Profile {
        skins: vec![SkinRef { url: SKIN_URL.into(), slim: true, active: true }],
        capes: vec![
            CapeRef { id: "c1".into(), url: CAPE_1_URL.into(), alias: Some("Pionera".into()), active: true },
            CapeRef { id: "c2".into(), url: CAPE_2_URL.into(), alias: Some("Aniversario".into()), active: false },
        ],
    }
}

/// Textura de ejemplo de una URL (como `skinTextureProvider` en los tests de
/// Flutter: capa-2, capa o skin).
pub fn sample_texture(url: &str) -> SkinTexture {
    if url.contains("capa-2") {
        sample_cape(SAMPLE_CAPE_2.0, SAMPLE_CAPE_2.1)
    } else if url.contains("capa") {
        sample_cape(SAMPLE_CAPE_1.0, SAMPLE_CAPE_1.1)
    } else {
        sample_skin()
    }
}

/// Código de `FakeBridge.beginMicrosoftLogin`.
pub fn sample_device_code() -> DeviceCode {
    DeviceCode { user_code: "K7QF-M2XP".into(), verification_uri: "https://www.microsoft.com/link".into(), expires_in: 900 }
}

/// Acciones de cuentas y skins que solo se apuntan (tests y escenas): el
/// test entrega los resultados llamando al controlador.
#[derive(Clone, Default)]
pub struct Recorder(pub Rc<RefCell<Vec<String>>>);

impl Recorder {
    pub fn push(&self, s: String) {
        self.0.borrow_mut().push(s);
    }

    /// Lo apuntado hasta ahora (y lo vacía).
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.0.borrow_mut())
    }
}

impl AccountActions for Recorder {
    fn load(&self) {
        self.push("load".into());
    }
    fn set_active(&self, id: &str) {
        self.push(format!("activate {id}"));
    }
    fn remove(&self, id: &str, name: &str) {
        self.push(format!("remove {id} {name}"));
    }
    fn add_offline(&self, name: &str, _: u64) {
        self.push(format!("offline {name}"));
    }
    fn begin_login(&self, attempt: u64) {
        self.push(format!("login {attempt}"));
    }
    fn cancel_login(&self) {
        self.push("cancel-login".into());
    }
    fn open_url(&self, url: &str) {
        self.push(format!("open {url}"));
    }
    fn load_face(&self, url: &str) {
        self.push(format!("face {url}"));
    }
}

impl SkinActions for Recorder {
    fn load_profile(&self, account: &str, _: u64) {
        self.push(format!("profile {account}"));
    }
    fn load_texture(&self, url: &str) {
        self.push(format!("texture {url}"));
    }
    fn pick_file(&self, _: &str) {
        self.push("pick".into());
    }
    fn load_local(&self, path: &str) {
        self.push(format!("local {path}"));
    }
    fn apply(&self, account: &str, what: &Apply) {
        self.push(format!("apply {account} {what:?}"));
    }
    fn reset(&self, account: &str) {
        self.push(format!("reset {account}"));
    }
    fn cape(&self, account: &str, id: Option<&str>) {
        self.push(format!("cape {account} {}", id.unwrap_or("-")));
    }
}

impl crate::instance::InstanceActions for Recorder {
    fn load(&self, id: &str) {
        self.push(format!("load-instance {id}"));
    }
    fn open_folder(&self, path: &std::path::Path) {
        self.push(format!("open-folder {}", path.display()));
    }
    fn delete(&self, id: &str, name: &str) {
        self.push(format!("delete-instance {id} {name}"));
    }
}

// --- Contenido y servidores (fase 4) -----------------------------------------

use crate::content::{ContentActions, Filters, InstallRequest, Installed, Page, Project, Rgba, Version, VersionsQuery};

/// Base de las URLs de los iconos de ejemplo (`kFixtureIconBase`): se sirven
/// desde `ui/tests/data/icons/<slug>.png`, sin red.
pub const ICON_BASE: &str = "https://cdn.modrinth.com/data/test/";

fn project(id: &str, title: &str, author: &str, description: &str, downloads: u64, kind: &str, categories: &[&str]) -> Project {
    let slug = if id == "complementary" { "complementary-reimagined" } else { id };
    Project {
        source: "modrinth".into(),
        id: id.into(),
        slug: id.into(),
        kind: Some(kind.into()),
        title: title.into(),
        description: description.into(),
        author: Some(author.into()),
        icon_url: Some(format!("{ICON_BASE}{slug}.png")),
        downloads,
        follows: downloads / 90,
        categories: categories.iter().map(|c| (*c).into()).collect(),
    }
}

/// `sampleProjects()`.
pub fn sample_projects() -> Vec<Project> {
    vec![
        project("sodium", "Sodium", "jellysquid3", "El motor de renderizado más rápido para Minecraft, con muchas más imágenes por segundo.", 98_400_000, "mod", &["optimización"]),
        project("lithium", "Lithium", "jellysquid3", "Optimiza la física, la IA de las criaturas y la generación sin cambiar el juego.", 51_200_000, "mod", &["optimización"]),
        project("iris", "Iris Shaders", "coderbot", "Shaders modernos compatibles con OptiFine y con Sodium.", 77_300_000, "mod", &["decoración"]),
        project("entityculling", "Entity Culling", "tr7zw", "Deja de dibujar entidades y bloques que no se ven.", 31_900_000, "mod", &["optimización"]),
        project("modmenu", "Mod Menu", "Prospector", "Añade un menú de mods para ver y configurar lo que tienes instalado.", 64_100_000, "mod", &["utilidad"]),
        project("ferrite-core", "FerriteCore", "malte0811", "Reduce el uso de memoria del juego.", 45_800_000, "mod", &["optimización"]),
        project("fresh-animations", "Fresh Animations", "FreshLX", "Animaciones nuevas para las criaturas del juego.", 12_300_000, "resourcepack", &["animaciones"]),
        project("complementary", "Complementary Shaders", "EminGT", "Shaders bonitos y equilibrados.", 18_800_000, "shader", &["realista"]),
        project("terralith", "Terralith", "Starmute", "Más de 95 biomas nuevos sin bloques nuevos.", 9_100_000, "datapack", &["generación"]),
        project("fabulously-optimized", "Fabulously Optimized", "robotkoer", "Modpack de rendimiento con buenos gráficos.", 13_200_000, "modpack", &["optimización"]),
    ]
}

/// `FakeBridge.searchContent`: por tipo y título.
pub fn search_page(f: &Filters, offset: u32, limit: u32) -> Page {
    let text = f.text.trim().to_lowercase();
    let all: Vec<Project> = sample_projects()
        .into_iter()
        .filter(|p| p.kind() == f.kind)
        .filter(|p| text.is_empty() || p.title.to_lowercase().contains(&text))
        .collect();
    let total = all.len() as u32;
    Page { items: all.into_iter().skip(offset as usize).take(limit as usize).collect(), total }
}

/// `sampleVersions()`: cuatro versiones a 4, 26, 41 y 63 días del 11 oct 2026.
pub fn sample_versions(project: &str, mc: Option<&str>, loader: Option<&str>) -> Vec<Version> {
    let dates = ["2026-10-07", "2026-09-15", "2026-08-31", "2026-08-09"];
    [("0.6.14", "release"), ("0.6.13", "release"), ("0.6.12-beta.2", "beta"), ("0.6.11", "release")]
        .iter()
        .enumerate()
        .map(|(i, (v, rel))| Version {
            id: format!("{project}-v{i}"),
            version_number: (*v).into(),
            mc_versions: vec![mc.unwrap_or("1.21.4").into()],
            loaders: vec![loader.unwrap_or("fabric").into()],
            release: (*rel).into(),
            date_published: format!("{}T00:00:00.000", dates[i]),
            file_url: Some(format!("https://cdn.modrinth.com/data/{project}/{v}.jar")),
            file_size: 1_200_000 - i as u64 * 40_000,
        })
        .collect()
}

/// Lo instalado en `survival-amigos` (`sampleContent()`): el proyecto es lo
/// que va antes del primer `-`, `_` o `.` del archivo.
pub fn sample_installed() -> Vec<Installed> {
    let c = |file: &str, version: Option<&str>, known: bool| Installed {
        project_id: known.then(|| file.split(['-', '_', '.']).next().unwrap_or(file).to_owned()),
        version_number: version.map(Into::into),
        file: file.into(),
    };
    vec![
        c("fabric-api-0.119.2+1.21.4.jar", Some("0.119.2+1.21.4"), true),
        c("sodium-fabric-0.6.13+mc1.21.4.jar", Some("0.6.13+mc1.21.4"), true),
        c("iris-fabric-1.8.8+mc1.21.4.jar", Some("1.8.8+mc1.21.4"), true),
        c("lithium-fabric-0.14.7+mc1.21.4.jar", Some("0.14.7"), true),
        c("journeymap-1.21.4-6.0.0.jar", Some("6.0.0"), true),
        c("appleskin-fabric-mc1.21.4-3.0.6.jar", Some("3.0.6"), true),
        c("mi-mod-casero.jar", None, false),
        c("FreshAnimations_v1.9.3.zip", Some("1.9.3"), true),
        c("ComplementaryReimagined_r5.4.zip", Some("r5.4"), true),
    ]
}

/// Icono de ejemplo de una URL (`fixtureIconPath`), o `None` (iniciales).
pub fn sample_icon(url: &str) -> Option<Rgba> {
    let name = url.strip_prefix(ICON_BASE)?;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/icons").join(name);
    crate::content::decode_icon(&std::fs::read(path).ok()?)
}

impl ContentActions for Recorder {
    fn search(&self, f: &Filters, offset: u32, generation: u64) {
        self.push(format!("search {} {} «{}» {offset} g{generation}", f.source, f.kind, f.text));
    }
    fn versions(&self, q: &VersionsQuery, seq: u64) {
        self.push(format!("versions {} {:?} {:?} s{seq}", q.project, q.mc, q.loader));
    }
    fn install(&self, r: &InstallRequest) {
        self.push(format!("install {} {} {:?} {:?}", r.instance_id, r.project, r.version, r.world));
    }
    fn load_installed(&self, instance_id: &str) {
        self.push(format!("installed {instance_id}"));
    }
    fn load_worlds(&self, instance_id: &str) {
        self.push(format!("worlds {instance_id}"));
    }
    fn load_releases(&self) {
        self.push("releases".into());
    }
    fn load_icon(&self, url: &str) {
        self.push(format!("icon {url}"));
    }
    fn pick_modpack(&self, _: &str) {
        self.push("pick-modpack".into());
    }
    fn install_modpack(&self, source: &str, name: Option<&str>, attempt: u64) {
        self.push(format!("modpack {source} {name:?} a{attempt}"));
    }
}

impl crate::curseforge::KeyActions for Recorder {
    fn check(&self) {
        self.push("check-key".into());
    }
    fn set(&self, key: &str) {
        // Solo el largo: una clave no se apunta.
        self.push(format!("set-key ({} caracteres)", key.chars().count()));
    }
    fn clear(&self) {
        self.push("clear-key".into());
    }
}

impl crate::servers::ServerActions for Recorder {
    fn save(&self, d: &crate::servers::Draft) {
        self.push(format!("save-server {:?} «{}» {} {:?} {:?}", d.id, d.name, d.address, d.instance_id, d.account_id));
    }
    fn remove(&self, id: &str, name: &str) {
        self.push(format!("remove-server {id} «{name}»"));
    }
    fn move_to(&self, id: &str, index: usize) {
        self.push(format!("move-server {id} {index}"));
    }
    fn import(&self, instance_id: &str, _: &str) {
        self.push(format!("import-servers {instance_id}"));
    }
    fn reload(&self) {
        self.push("reload-servers".into());
    }
}

/// Estados de `sampleStatuses()` más `localhost` sin conexión (lo que
/// devuelve `FakeBridge.pingServers` para las que no conoce).
pub fn sample_statuses_all() -> Vec<(String, crate::home::Status)> {
    let mut v = sample_statuses();
    v.push(("localhost".into(), crate::home::Status::default()));
    v
}

impl crate::controller::Actions for Recorder {
    fn open_instance(&self, id: &str) {
        self.push(format!("open-instance {id}"));
    }
    fn play(&self, id: &str) {
        self.push(format!("play {id}"));
    }
    fn new_instance(&self) {}
    fn show_trash(&self) {}
    fn join_server(&self, instance_id: &str, address: &str, account: Option<&str>) {
        self.push(format!("join {instance_id} {address} {account:?}"));
    }
}
// --- Ajustes (fase 5) ------------------------------------------------------

use crate::settings::{Java, SettingsActions, Values};

/// RAM del equipo de ejemplo (`systemMemoryMb`).
pub const TOTAL_MB: u64 = 16384;
pub const DATA_DIR: &str = "/home/user/.local/share/shard";
pub const CORE_VERSION: &str = "0.1.0";

/// `getSettings` de fake_bridge.dart.
pub fn sample_values(language: Option<&str>) -> Values {
    Values {
        language: language.map(str::to_owned),
        close: lcore::settings::CloseBehavior::Ask,
        multiple: false,
        memory_mb: 4096,
        jvm_args: vec!["-XX:+UseG1GC".into()],
        concurrency: 8,
        discord: false,
        check_updates: true,
        msa_set: true,
    }
}

/// `sampleJavas`.
pub fn sample_javas() -> Vec<Java> {
    use lcore::java::JavaSource;
    let j = |path: &str, major, version: &str, vendor: &str, source| Java {
        major,
        version: version.into(),
        vendor: Some(vendor.into()),
        path: path.into(),
        source,
    };
    vec![
        j("/home/user/.local/share/shard/java/21/bin/java", 21, "21.0.5", "Eclipse Adoptium", JavaSource::Managed),
        j("/usr/lib/jvm/java-17-openjdk/bin/java", 17, "17.0.13", "Arch Linux", JavaSource::System),
        j("/usr/lib/jvm/java-8-openjdk/jre/bin/java", 8, "1.8.0_432", "Oracle Corporation", JavaSource::System),
    ]
}

/// `flatpakUpdate` (se actualiza a mano).
pub fn flatpak_update() -> crate::notices::UpdateView {
    crate::notices::UpdateView {
        version: "0.2.0.r40".into(),
        current: "0.1.0.r27".into(),
        notes: None,
        notes_url: None,
        can_install: false,
        download_url: "https://dl.shardlauncher.com/linux/0.2.0.r40/Shard-0.2.0.r40-x86_64.flatpak".into(),
    }
}

impl SettingsActions for Recorder {
    fn load(&self, _: u64) {
        self.push("settings".into());
    }
    fn save(&self, v: &Values, _: u64) {
        self.push(format!("save {v:?}"));
    }
    fn list_java(&self) {
        self.push("java".into());
    }
    fn install_java(&self, major: u32) {
        self.push(format!("install-java {major}"));
    }
    fn load_autostart(&self) {
        self.push("autostart?".into());
    }
    fn set_autostart(&self, on: bool) {
        self.push(format!("autostart {on}"));
    }
    fn save_prefs(&self, p: &crate::prefs::UiPrefs) {
        self.push(format!("prefs {}", p.to_json()));
    }
    fn pick_background(&self) {
        self.push("pick-background".into());
    }
    fn image_accent(&self, path: &std::path::Path) {
        self.push(format!("image-accent {}", path.display()));
    }
    fn copy_text(&self, text: &str) -> bool {
        self.push(format!("copy {text}"));
        true
    }
    fn check_update(&self) {
        self.push("check-update".into());
    }
}

// --- Doctor (fase 5) ---------------------------------------------------------

use crate::doctor::{Diagnosis, DoctorActions, Report};

/// `sampleReport`.
pub fn sample_report() -> Report {
    Report {
        crash_excerpt: Some("Description: Mod resolution failed\nMod sodium-extra requires sodium 0.6.0 or later, found 0.5.11".into()),
        crash_report_path: Some(
            "/home/user/.local/share/shard/instances/survival-amigos/minecraft/crash-reports/crash-2026-10-11_18.02.31-client.txt".into(),
        ),
        diagnoses: vec![
            Diagnosis {
                id: "mod_dependency".into(),
                category: "missing_dependency".into(),
                severity: "critical".into(),
                title: "Falta una dependencia de un mod".into(),
                explanation: "sodium-extra necesita una versión más nueva de Sodium. Actualiza Sodium o desactiva sodium-extra.".into(),
                confidence: 92,
                culprit: Some("sodium-extra-0.5.4+mc1.21.4.jar".into()),
                fix: Some("Desactivar el mod sodium-extra-0.5.4+mc1.21.4.jar".into()),
            },
            Diagnosis {
                id: "low_memory".into(),
                category: "memory".into(),
                severity: "warning".into(),
                title: "Poca memoria para tantos mods".into(),
                explanation: "La instancia tiene 87 mods y 2 GB asignados. Con menos de 4 GB es habitual que se quede sin memoria.".into(),
                confidence: 64,
                culprit: None,
                fix: Some("Asignar 4096 MB de memoria".into()),
            },
            Diagnosis {
                id: "world_lock".into(),
                category: "world_corruption".into(),
                severity: "info".into(),
                title: "Quedó un bloqueo de mundo".into(),
                explanation: "Un session.lock de la última partida sigue ahí. No suele dar problemas, pero puede impedir abrir el mundo.".into(),
                confidence: 40,
                culprit: Some("saves/Mundo de verano/session.lock".into()),
                fix: None,
            },
        ],
    }
}

impl DoctorActions for Recorder {
    fn diagnose(&self, instance_id: &str, seq: u64) {
        self.push(format!("diagnose {instance_id} {seq}"));
    }
    fn apply_fix(&self, instance_id: &str, diagnosis_id: &str) {
        self.push(format!("fix {instance_id} {diagnosis_id}"));
    }
    fn open_path(&self, path: &str) {
        self.push(format!("open-path {path}"));
    }
}

// --- Primer arranque (fase 5) --------------------------------------------------

use crate::first_run::{FirstRunActions, MinecraftPlan, Planned, RoxyPlan, Section, Sources};

/// `sources` de fake_bridge.dart.
pub fn sample_sources() -> Sources {
    Sources { roxy: Some("/home/user/.local/share/.roxy".into()), minecraft: Some("/home/user/.minecraft".into()) }
}

/// `sampleRoxyPlan`.
pub fn sample_roxy_plan() -> RoxyPlan {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: u64 = 1024 * 1024;
    let p = |id: &str, name: &str, mc: &str, loader: &str, bytes: u64, backups: u32, skipped: Option<&str>| Planned {
        original_id: id.into(),
        name: name.into(),
        mc: mc.into(),
        loader: loader.into(),
        bytes,
        backups,
        skipped: skipped.map(str::to_owned),
    };
    RoxyPlan {
        instances: vec![
            p("survival-amigos", "Survival con amigos", "1.21.4", "fabric", (1.4 * GB).round() as u64, 3, None),
            p("create-aeronautica", "Create: Aeronáutica", "1.20.1", "forge", (2.1 * GB).round() as u64, 1, None),
            p("vanilla-puro", "Vanilla puro", "1.21.4", "vanilla", 412 * MB, 0, None),
            p("viejo", "Pruebas 2023", "1.19.2", "fabric", 0, 0, Some("ya importada")),
        ],
        shared_bytes: (1.2 * GB).round() as u64,
        warnings: vec![],
    }
}

/// `sampleMinecraftPlan`.
pub fn sample_minecraft_plan() -> MinecraftPlan {
    const MB: u64 = 1024 * 1024;
    let s = |name: &str, files: u64, bytes: u64| Section { section: name.into(), present: files > 0, bytes };
    MinecraftPlan {
        name: "Minecraft 1.21.4".into(),
        mc: "1.21.4".into(),
        loader: "vanilla".into(),
        bytes: 852 * MB,
        sections: vec![
            s("saves", 812, 640 * MB),
            s("resourcepacks", 3, 92 * MB),
            s("screenshots", 57, 120 * MB),
            s("options", 1, 6 * 1024),
            s("servers", 1, 2 * 1024),
            s("mods", 0, 0),
        ],
        warnings: vec![],
    }
}

impl FirstRunActions for Recorder {
    fn detect(&self) {
        self.push("detect".into());
    }
    fn plan_roxy(&self) {
        self.push("plan-roxy".into());
    }
    fn plan_minecraft(&self) {
        self.push("plan-minecraft".into());
    }
    fn import(&self, roxy: bool, minecraft: bool) {
        self.push(format!("import roxy={roxy} minecraft={minecraft}"));
    }
    fn save_language(&self, language: Option<String>) {
        self.push(format!("language {language:?}"));
    }
    fn save_prefs(&self, p: &crate::prefs::UiPrefs) {
        self.push(format!("prefs {}", p.to_json()));
    }
    fn reload(&self) {
        self.push("reload".into());
    }
}
