//! Pestaña Mundos (worlds_tab.dart y `WorldsController` de
//! instance_data.dart): lista de mundos de la instancia, el elegido con su
//! modo, dificultad, hardcore y trucos, renombrar, borrar, abrir la carpeta
//! y sus datapacks (activar, quitar y añadir `.zip`).
//!
//! `Worlds` corre en el hilo de la UI: copia el estado a `WorldsState` y
//! atiende sus callbacks. El trabajo con el núcleo lo hace un
//! [`WorldActions`] ([`CoreWorlds`] en la app, `fixtures::Recorder` en los
//! tests) que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lcore::world::{DatapackInfo, Difficulty, GameMode, WorldInfo, WorldPatch};
use slint::{ComponentHandle, Image, Model, ModelRc, VecModel};

use crate::backend::Backend;
use crate::errors::Failure;
use crate::instance::{DetailEvent, InstanceDetail};
use crate::instances::Phase;
use crate::{DatapackItem, InstanceTab, MainWindow, WorldItem, WorldsState, i18n};

/// Modos en el orden de los segmentos.
pub const MODES: [GameMode; 4] = [GameMode::Survival, GameMode::Creative, GameMode::Adventure, GameMode::Spectator];
/// Dificultades en el orden de los segmentos.
pub const DIFFICULTIES: [Difficulty; 4] = [Difficulty::Peaceful, Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

/// Índice del segmento de un modo (-1 = desconocido: ninguno elegido).
pub fn mode_index(m: Option<GameMode>) -> i32 {
    m.and_then(|m| MODES.iter().position(|x| *x == m)).map_or(-1, |i| i as i32)
}

/// Índice del segmento de una dificultad (-1 = desconocida).
pub fn difficulty_index(d: Option<Difficulty>) -> i32 {
    d.and_then(|d| DIFFICULTIES.iter().position(|x| *x == d)).map_or(-1, |i| i as i32)
}

/// Segundos desde la última partida (`lastPlayedMs`) hasta `now`; -1 = nunca.
pub fn last_ago(now: i64, last_played_ms: Option<i64>) -> i32 {
    last_played_ms.map_or(-1, |ms| (now - ms.div_euclid(1000)).clamp(0, i32::MAX as i64) as i32)
}

/// `formatBytes`: «512 B», «12 KB», «2,8 MB», «1,5 GB» (un decimal como
/// mucho, con el separador del idioma).
pub fn format_bytes(lang: &str, bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    let one_decimal = |v: f64| {
        let tenths = (v * 10.0).round() as u64;
        let sep = if lang == "es" { "," } else { "." };
        if tenths.is_multiple_of(10) { format!("{}", tenths / 10) } else { format!("{}{sep}{}", tenths / 10, tenths % 10) }
    };
    if b < K {
        format!("{bytes} B")
    } else if b < K * K {
        format!("{} KB", (b / K).round())
    } else if b < K * K * K {
        format!("{} MB", one_decimal(b / (K * K)))
    } else {
        format!("{} GB", one_decimal(b / (K * K * K)))
    }
}

/// Mundo elegido: el de `folder` o, si ya no está, el primero.
pub fn selected_index(list: &[WorldInfo], folder: Option<&str>) -> usize {
    folder.and_then(|f| list.iter().position(|w| w.folder == f)).unwrap_or(0)
}

/// Lo que el controlador pide fuera de la UI. Todas responden con [`post`].
pub trait WorldActions {
    /// → [`Worlds::listed`].
    fn list(&self, instance: &str);
    /// → [`Worlds::changed`].
    fn rename(&self, instance: &str, folder: &str, name: &str);
    /// → [`Worlds::changed`].
    fn update(&self, instance: &str, folder: &str, patch: WorldPatch);
    /// → [`Worlds::deleted`].
    fn delete(&self, instance: &str, folder: &str, name: &str);
    /// → [`Worlds::datapacks_listed`].
    fn list_datapacks(&self, instance: &str, world: &str);
    /// → [`Worlds::datapack_done`].
    fn set_datapack(&self, instance: &str, world: &str, name: &str, enabled: bool);
    /// → [`Worlds::datapack_done`].
    fn remove_datapack(&self, instance: &str, world: &str, name: &str);
    /// Elige `.zip` con el selector de archivos y los copia al mundo →
    /// [`Worlds::datapack_done`] (nada si se cancela).
    fn add_datapacks(&self, instance: &str, world: &str);
    fn open_folder(&self, path: &Path);
}

thread_local! {
    static WORLDS: RefCell<Option<Rc<Worlds>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Mundos.
pub fn post(f: impl FnOnce(&Worlds) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        WORLDS.with(|c| {
            if let Some(w) = c.borrow().as_ref() {
                f(w);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

pub struct Worlds {
    ui: slint::Weak<MainWindow>,
    detail: Rc<InstanceDetail>,
    actions: Box<dyn WorldActions>,
    /// Segundos Unix de «ahora» (fijo en las escenas).
    now: Box<dyn Fn() -> i64>,
    /// Instancia cuyos mundos están en `list`.
    instance: RefCell<Option<String>>,
    list: RefCell<Vec<WorldInfo>>,
    /// Carpeta del mundo elegido.
    selected: RefCell<Option<String>>,
    running: Cell<bool>,
    revision: Cell<i32>,
    items: Rc<VecModel<WorldItem>>,
    packs: Rc<VecModel<DatapackItem>>,
    /// icon.png ya leídos (`None` = no se pudo: paisaje).
    icons: RefCell<HashMap<PathBuf, Option<Image>>>,
}

macro_rules! on {
    ($me:expr, $st:expr, $set:ident, | $w:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$me);
        $st.$set(move |$($arg),*| {
            if let Some($w) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl Worlds {
    pub fn new(ui: &MainWindow, detail: &Rc<InstanceDetail>, actions: Box<dyn WorldActions>, now: Box<dyn Fn() -> i64>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            detail: detail.clone(),
            actions,
            now,
            instance: RefCell::default(),
            list: RefCell::default(),
            selected: RefCell::default(),
            running: Cell::new(false),
            revision: Cell::new(0),
            items: Rc::new(VecModel::default()),
            packs: Rc::new(VecModel::default()),
            icons: RefCell::default(),
        });
        WORLDS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<WorldsState>();
        st.set_items(ModelRc::from(me.items.clone()));
        st.set_datapacks(ModelRc::from(me.packs.clone()));
        st.on_name_valid(|n| !n.trim().is_empty());
        on!(me, st, on_select, |w, folder| w.select(&folder));
        on!(me, st, on_rename, |w, name| w.rename(&name));
        on!(me, st, on_set_mode, |w, i| w.patch(WorldPatch { game_mode: MODES.get(i as usize).copied(), ..Default::default() }));
        on!(me, st, on_set_difficulty, |w, i| w.patch(WorldPatch { difficulty: DIFFICULTIES.get(i as usize).copied(), ..Default::default() }));
        on!(me, st, on_set_hardcore, |w, v| w.patch(WorldPatch { hardcore: Some(v), ..Default::default() }));
        on!(me, st, on_set_cheats, |w, v| w.patch(WorldPatch { allow_commands: Some(v), ..Default::default() }));
        on!(me, st, on_set_datapack, |w, name, v| w.with_world(|i, f| w.actions.set_datapack(i, f, &name, v)));
        on!(me, st, on_remove_datapack, |w, name| w.with_world(|i, f| w.actions.remove_datapack(i, f, &name)));
        on!(me, st, on_add_datapacks, |w| w.with_world(|i, f| w.actions.add_datapacks(i, f)));
        on!(me, st, on_delete, |w| w.delete());
        on!(me, st, on_open_folder, |w| w.open_folder());
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            if let Some(w) = weak.upgrade() {
                w.on_detail(e);
            }
        });
        // El juego abierto bloquea la edición; al cerrarse, la lista puede
        // haber cambiado (invalidateInstanceData).
        let weak = Rc::downgrade(&me);
        detail.controller().on_refresh(move |m| {
            if let Some(w) = weak.upgrade() {
                let Some(id) = w.instance.borrow().clone() else { return };
                let running = m.phase(&id) == Phase::Running;
                if w.running.replace(running) != running {
                    w.set_running(running);
                    if !running && w.detail.tab() == InstanceTab::Worlds && w.detail.id().as_deref() == Some(id.as_str()) {
                        w.actions.list(&id);
                    }
                }
            }
        });
        me
    }

    pub fn lang(&self) -> String {
        self.detail.lang()
    }

    fn on_detail(&self, e: &DetailEvent) {
        match *e {
            DetailEvent::Shown { id, tab } => {
                if self.instance.borrow().as_deref() != Some(id) {
                    self.reset(Some(id));
                }
                if tab == InstanceTab::Worlds {
                    self.load();
                }
            }
            DetailEvent::Closed => self.reset(None),
            _ => {}
        }
    }

    /// Olvida la lista (otra instancia o vuelta a Instancias).
    fn reset(&self, id: Option<&str>) {
        *self.instance.borrow_mut() = id.map(str::to_owned);
        self.list.borrow_mut().clear();
        *self.selected.borrow_mut() = None;
        self.items.set_vec(Vec::new());
        self.packs.set_vec(Vec::new());
        let running = id.is_some_and(|id| self.detail.controller().model.borrow().phase(id) == Phase::Running);
        self.running.set(running);
        self.set_running(running);
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<WorldsState>();
            st.set_error("".into());
            st.set_error_detail("".into());
            st.set_loading(false);
            st.set_rename_open(false);
            st.set_delete_open(false);
            st.set_remove_pack("".into());
        }
    }

    fn set_running(&self, running: bool) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<WorldsState>().set_running(running);
        }
    }

    /// Pide la lista (al mostrarse la pestaña).
    pub fn load(&self) {
        let Some(id) = self.instance.borrow().clone() else { return };
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<WorldsState>().set_loading(true);
        }
        self.actions.list(&id);
    }

    /// Resultado de [`WorldActions::list`].
    pub fn listed(&self, instance: &str, result: Result<Vec<WorldInfo>, Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<WorldsState>();
        st.set_loading(false);
        match result {
            Ok(list) => {
                st.set_error("".into());
                st.set_error_detail("".into());
                *self.list.borrow_mut() = list;
                self.refresh(true);
            }
            Err(f) => {
                // Con datos a la vista se quedan; sin ellos, el error.
                if self.list.borrow().is_empty() {
                    st.set_error(f.message.into());
                    st.set_error_detail(f.detail.into());
                } else {
                    self.detail.controller().toast(&f.message, true);
                }
            }
        }
    }

    /// Copia la lista a la UI; `packs` = volver a pedir los datapacks del
    /// elegido.
    fn refresh(&self, packs: bool) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<WorldsState>();
        let lang = self.lang();
        let now = (self.now)();
        let list = self.list.borrow();
        let items: Vec<WorldItem> = list.iter().map(|w| self.item(w, &lang, now)).collect();
        let sel = selected_index(&list, self.selected.borrow().as_deref());
        let folder = list.get(sel).map(|w| w.folder.clone());
        drop(list);
        *self.selected.borrow_mut() = folder.clone();
        if self.items.row_count() == items.len() {
            for (i, it) in items.into_iter().enumerate() {
                if self.items.row_data(i).as_ref() != Some(&it) {
                    self.items.set_row_data(i, it);
                }
            }
        } else {
            self.items.set_vec(items);
        }
        st.set_selected(sel as i32);
        self.bump();
        if packs {
            self.packs.set_vec(Vec::new());
            if let (Some(id), Some(folder)) = (self.instance.borrow().clone(), folder) {
                st.set_datapacks_loading(true);
                self.actions.list_datapacks(&id, &folder);
            }
        }
    }

    /// Los controles del panel vuelven al valor del mundo.
    fn bump(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<WorldsState>().set_revision(self.revision.get());
        }
    }

    fn item(&self, w: &WorldInfo, lang: &str, now: i64) -> WorldItem {
        let (icon, icon_large) = match self.icon(w.icon.as_deref()) {
            Some(img) => (img.clone(), img),
            None => {
                // BlockLandscape(seed: carpeta, block: size / 8) dentro del borde.
                let ctl = self.detail.controller();
                (ctl.landscape(&w.folder, 52.0 / 8.0, 50.0, 50.0), ctl.landscape(&w.folder, 72.0 / 8.0, 70.0, 70.0))
            }
        };
        WorldItem {
            folder: w.folder.as_str().into(),
            name: w.name.as_str().into(),
            mode: mode_index(w.game_mode),
            difficulty: difficulty_index(w.difficulty),
            hardcore: w.hardcore == Some(true),
            cheats: w.cheats == Some(true),
            last_ago: last_ago(now, w.last_played_ms),
            size: format_bytes(lang, w.size_bytes).into(),
            version: w.version_name.clone().unwrap_or_default().into(),
            error: w.error.clone().unwrap_or_default().into(),
            icon,
            icon_large,
        }
    }

    /// icon.png del mundo (64 × 64: se lee aquí, una vez).
    fn icon(&self, path: Option<&Path>) -> Option<Image> {
        let path = path?;
        self.icons.borrow_mut().entry(path.to_owned()).or_insert_with(|| Image::load_from_path(path).ok()).clone()
    }

    fn current(&self) -> Option<(String, WorldInfo)> {
        let id = self.instance.borrow().clone()?;
        let folder = self.selected.borrow().clone()?;
        let w = self.list.borrow().iter().find(|w| w.folder == folder)?.clone();
        Some((id, w))
    }

    /// `f(instancia, carpeta)` del mundo elegido.
    fn with_world(&self, f: impl FnOnce(&str, &str)) {
        if let Some((id, w)) = self.current() {
            f(&id, &w.folder);
        }
    }

    pub fn select(&self, folder: &str) {
        if self.selected.borrow().as_deref() == Some(folder) {
            return;
        }
        *self.selected.borrow_mut() = Some(folder.to_owned());
        self.refresh(true);
    }

    fn rename(&self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        self.with_world(|id, folder| self.actions.rename(id, folder, name));
    }

    fn patch(&self, patch: WorldPatch) {
        self.with_world(|id, folder| self.actions.update(id, folder, patch));
    }

    fn delete(&self) {
        if let Some((id, w)) = self.current() {
            self.actions.delete(&id, &w.folder, &w.name);
        }
    }

    fn open_folder(&self) {
        if let (Some(d), Some(folder)) = (self.detail.detail(), self.selected.borrow().clone()) {
            self.actions.open_folder(&d.game_dir.join("saves").join(folder));
        }
    }

    /// Resultado de renombrar o cambiar un mundo: el mundo nuevo o el error
    /// (aviso y los controles vuelven a su valor).
    pub fn changed(&self, instance: &str, result: Result<WorldInfo, Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        match result {
            Ok(w) => {
                if let Some(x) = self.list.borrow_mut().iter_mut().find(|x| x.folder == w.folder) {
                    *x = w;
                }
                self.refresh(false);
            }
            Err(f) => {
                self.detail.controller().toast(&f.message, true);
                self.bump();
            }
        }
    }

    /// Resultado de [`WorldActions::delete`].
    pub fn deleted(&self, instance: &str, folder: &str, name: &str, result: Result<(), Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        match result {
            Ok(()) => {
                self.list.borrow_mut().retain(|w| w.folder != folder);
                let msg = i18n::text(&self.lang(), "worldDeleted").replace("{name}", name);
                self.detail.controller().toast(&msg, false);
                self.refresh(true);
            }
            Err(f) => self.detail.controller().toast(&f.message, true),
        }
    }

    /// Resultado de [`WorldActions::list_datapacks`].
    pub fn datapacks_listed(&self, instance: &str, world: &str, result: Result<Vec<DatapackInfo>, Failure>) {
        let current = self.instance.borrow().as_deref() == Some(instance) && self.selected.borrow().as_deref() == Some(world);
        if !current {
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        ui.global::<WorldsState>().set_datapacks_loading(false);
        match result {
            Ok(list) => {
                let lang = self.lang();
                let items: Vec<DatapackItem> = list
                    .iter()
                    .map(|p| DatapackItem {
                        name: p.name.as_str().into(),
                        is_dir: p.is_dir,
                        enabled: p.enabled,
                        size: format_bytes(&lang, p.size_bytes).into(),
                    })
                    .collect();
                self.packs.set_vec(items);
                self.bump();
            }
            Err(f) => {
                self.packs.set_vec(Vec::new());
                self.detail.controller().toast(&f.message, true);
            }
        }
    }

    /// Resultado de activar, quitar o añadir datapacks: aviso si falló y la
    /// lista de nuevo (como `ref.invalidate(datapacksProvider)`).
    pub fn datapack_done(&self, instance: &str, world: &str, result: Result<(), Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        if let Err(f) = result {
            self.detail.controller().toast(&f.message, true);
        }
        if self.selected.borrow().as_deref() == Some(world) {
            self.actions.list_datapacks(instance, world);
        }
    }
}

/// [`WorldActions`] contra el núcleo.
pub struct CoreWorlds(pub Backend);

/// Traduce el error en el hilo de la UI (con el idioma de ese momento).
fn fail(w: &Worlds, e: &lcore::Error) -> Failure {
    Failure::new(&w.lang(), e)
}

impl WorldActions for CoreWorlds {
    fn list(&self, instance: &str) {
        let (core, id) = (self.0.core.clone(), instance.to_owned());
        self.0.rt.spawn(async move {
            let r = core.list_worlds(&id).await;
            post(move |w| w.listed(&id, r.map_err(|e| fail(w, &e))));
        });
    }

    fn rename(&self, instance: &str, folder: &str, name: &str) {
        let (core, id, folder, name) = (self.0.core.clone(), instance.to_owned(), folder.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.rename_world(&id, &folder, &name).await;
            post(move |w| w.changed(&id, r.map_err(|e| fail(w, &e))));
        });
    }

    fn update(&self, instance: &str, folder: &str, patch: WorldPatch) {
        let (core, id, folder) = (self.0.core.clone(), instance.to_owned(), folder.to_owned());
        self.0.rt.spawn(async move {
            let r = core.update_world(&id, &folder, patch).await;
            post(move |w| w.changed(&id, r.map_err(|e| fail(w, &e))));
        });
    }

    fn delete(&self, instance: &str, folder: &str, name: &str) {
        let (core, id, folder, name) = (self.0.core.clone(), instance.to_owned(), folder.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.delete_world(&id, &folder).await;
            post(move |w| w.deleted(&id, &folder, &name, r.map_err(|e| fail(w, &e))));
        });
    }

    fn list_datapacks(&self, instance: &str, world: &str) {
        let (core, id, world) = (self.0.core.clone(), instance.to_owned(), world.to_owned());
        self.0.rt.spawn(async move {
            let r = core.list_datapacks(&id, &world).await;
            post(move |w| w.datapacks_listed(&id, &world, r.map_err(|e| fail(w, &e))));
        });
    }

    fn set_datapack(&self, instance: &str, world: &str, name: &str, enabled: bool) {
        let (core, id, world, name) = (self.0.core.clone(), instance.to_owned(), world.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.set_datapack_enabled(&id, &world, &name, enabled).await;
            post(move |w| w.datapack_done(&id, &world, r.map_err(|e| fail(w, &e))));
        });
    }

    fn remove_datapack(&self, instance: &str, world: &str, name: &str) {
        let (core, id, world, name) = (self.0.core.clone(), instance.to_owned(), world.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.remove_datapack(&id, &world, &name).await;
            post(move |w| w.datapack_done(&id, &world, r.map_err(|e| fail(w, &e))));
        });
    }

    fn add_datapacks(&self, instance: &str, world: &str) {
        let (core, id, world) = (self.0.core.clone(), instance.to_owned(), world.to_owned());
        self.0.rt.spawn(async move {
            let Some(files) = rfd::AsyncFileDialog::new().add_filter("Datapack", &["zip", "ZIP"]).pick_files().await else {
                return;
            };
            let paths: Vec<PathBuf> = files.iter().map(|f| f.path().to_owned()).collect();
            let r = add_all(&core, &id, &world, &paths).await;
            post(move |w| w.datapack_done(&id, &world, r.map_err(|e| fail(w, &e))));
        });
    }

    fn open_folder(&self, path: &Path) {
        crate::instance::open_path(path);
    }
}

/// Copia cada `.zip` al mundo; sigue con los demás si uno falla y devuelve
/// el primer error.
pub async fn add_all(core: &lcore::Core, instance: &str, world: &str, paths: &[PathBuf]) -> lcore::Result<()> {
    let mut first = None;
    for p in paths {
        if let Err(e) = core.add_datapack(instance, world, p).await {
            first.get_or_insert(e);
        }
    }
    first.map_or(Ok(()), Err)
}

impl WorldActions for crate::fixtures::Recorder {
    fn list(&self, instance: &str) {
        self.push(format!("worlds {instance}"));
    }
    fn rename(&self, instance: &str, folder: &str, name: &str) {
        self.push(format!("rename-world {instance} {folder} {name}"));
    }
    fn update(&self, instance: &str, folder: &str, patch: WorldPatch) {
        self.push(format!("update-world {instance} {folder} {patch:?}"));
    }
    fn delete(&self, instance: &str, folder: &str, name: &str) {
        self.push(format!("delete-world {instance} {folder} {name}"));
    }
    fn list_datapacks(&self, instance: &str, world: &str) {
        self.push(format!("datapacks {instance} {world}"));
    }
    fn set_datapack(&self, instance: &str, world: &str, name: &str, enabled: bool) {
        self.push(format!("set-datapack {instance} {world} {name} {enabled}"));
    }
    fn remove_datapack(&self, instance: &str, world: &str, name: &str) {
        self.push(format!("remove-datapack {instance} {world} {name}"));
    }
    fn add_datapacks(&self, instance: &str, world: &str) {
        self.push(format!("add-datapacks {instance} {world}"));
    }
    fn open_folder(&self, path: &Path) {
        self.push(format!("open-folder {}", path.display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_como_format_dart() {
        assert_eq!(format_bytes("es", 512), "512 B");
        assert_eq!(format_bytes("es", 12_000), "12 KB");
        assert_eq!(format_bytes("es", 410_000), "400 KB");
        assert_eq!(format_bytes("es", 2_900_000), "2,8 MB");
        assert_eq!(format_bytes("en", 2_900_000), "2.8 MB");
        assert_eq!(format_bytes("es", 842 * 1024 * 1024), "842 MB");
        assert_eq!(format_bytes("es", 3 * 1024 * 1024 * 1024 / 2), "1,5 GB");
    }

    #[test]
    fn segmentos_y_desconocidos() {
        assert_eq!(mode_index(Some(GameMode::Survival)), 0);
        assert_eq!(mode_index(Some(GameMode::Spectator)), 3);
        assert_eq!(mode_index(None), -1);
        assert_eq!(difficulty_index(Some(Difficulty::Normal)), 2);
        assert_eq!(difficulty_index(None), -1);
    }

    #[test]
    fn hace_cuanto() {
        assert_eq!(last_ago(10_000, Some(9_000_000)), 1000);
        assert_eq!(last_ago(10_000, None), -1);
        // Reloj atrasado: «ahora», no negativo.
        assert_eq!(last_ago(10, Some(50_000)), 0);
    }

    #[test]
    fn elegido_o_el_primero() {
        let list = crate::fixtures::worlds::sample_worlds();
        assert_eq!(selected_index(&list, Some("Hardcore 2")), 2);
        assert_eq!(selected_index(&list, Some("borrado")), 0);
        assert_eq!(selected_index(&list, None), 0);
    }
}
