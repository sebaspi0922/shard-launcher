//! Datos falsos de `app/test/support/fake_bridge.dart` (los de los goldens),
//! para el arnés de paridad y los tests de interacción. Mismos ids, nombres,
//! versiones, tiempos y fechas relativas a `NOW`.

use crate::instances::Record;

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

// --- Cuentas y skins (fase 2) ------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;

use crate::accounts::{Account, AccountActions, DeviceCode};
use crate::skin::{SAMPLE_CAPE_1, SAMPLE_CAPE_2, SkinTexture, sample_cape, sample_skin};
use crate::skins::{Apply, CapeRef, Profile, SkinActions, SkinRef};

fn account(id: &str, name: &str, microsoft: bool, active: bool) -> Account {
    Account { id: id.into(), name: name.into(), microsoft, skin_url: None, active }
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
    fn push(&self, s: String) {
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
    fn add_offline(&self, name: &str) {
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
    fn load_profile(&self, account: &str) {
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
