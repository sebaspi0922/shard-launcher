//! Diálogo «Nueva instancia» (create_instance_dialog.dart): nombre, versión
//! de Minecraft (con o sin snapshots, con buscador), loader y versión del
//! loader. Al crear, recarga la lista, avisa y abre la instancia nueva.
//!
//! El selector de versión y loader ([`Picker`]) lo comparte con «Cambiar
//! versión» (`change_version.rs`). Su lógica es pura y está probada aquí; el
//! controlador [`CreateInstance`] solo la copia a `CreateInstanceState`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::accounts::fill;
use crate::backend::Backend;
use crate::controller::Controller;
use crate::errors::Failure;
use crate::instances::{Record, loader_label};
use crate::{CreateInstanceState, MainWindow, PickerData, SelectOption, VersionRow, i18n};

/// Loaders que se pueden elegir al crear o cambiar de versión
/// (`kCreatableLoaders`).
pub const LOADERS: [&str; 5] = ["vanilla", "fabric", "quilt", "forge", "neoforge"];

/// Una versión de Minecraft (`VersionView`): id y tipo (`release`, `snapshot`…).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub id: String,
    pub kind: String,
}

impl Version {
    pub fn new(id: &str, kind: &str) -> Self {
        Self { id: id.into(), kind: kind.into() }
    }
}

/// Del manifiesto del núcleo, lo que ofrece el diálogo: releases y, si se
/// piden, snapshots (`list_versions` del bridge).
pub fn from_manifest(entries: Vec<lcore::meta::ManifestEntry>, snapshots: bool) -> Vec<Version> {
    entries
        .into_iter()
        .filter(|v| v.kind == "release" || (snapshots && v.kind == "snapshot"))
        .map(|v| Version { id: v.id, kind: v.kind })
        .collect()
}

/// Última release de la lista (la etiqueta «Última»).
pub fn latest_release(list: &[Version]) -> Option<&str> {
    list.iter().find(|v| v.kind == "release").map(|v| v.id.as_str())
}

/// Versión que se elige al cargar: la última release o, si no hay, la primera.
pub fn default_version(list: &[Version]) -> Option<&str> {
    latest_release(list).or(list.first().map(|v| v.id.as_str()))
}

/// Versiones cuyo id contiene la búsqueda (sin mayúsculas ni espacios en los
/// bordes).
pub fn filter<'a>(list: &'a [Version], query: &str) -> Vec<&'a Version> {
    let q = query.trim().to_lowercase();
    list.iter().filter(|v| q.is_empty() || v.id.to_lowercase().contains(&q)).collect()
}

/// Nombre propuesto: «Minecraft 1.21.5» o «Fabric 1.21.5».
pub fn suggested_name(lang: &str, loader: &str, mc: &str) -> String {
    if loader == "vanilla" { format!("Minecraft {mc}") } else { format!("{} {mc}", loader_label(lang, loader)) }
}

/// «1.21.4 · Fabric» (`versionLine`).
pub fn version_line(lang: &str, mc: &str, loader: &str) -> String {
    format!("{mc} · {}", loader_label(lang, loader))
}

/// Algo que se está cargando, ya cargado o que falló.
#[derive(Clone, Debug, PartialEq)]
pub enum Load<T> {
    Loading,
    Ready(T),
    Failed(Failure),
}

/// Lo que el selector necesita pedir al núcleo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Versions { snapshots: bool },
    LoaderVersions { loader: String, mc: String },
}

/// Selector de versión y loader (`VersionPicker`, `LoaderOption`,
/// `LoaderVersionField`) con las listas que ya llegaron (como los
/// `FutureProvider.family` de Flutter, no se vuelven a pedir).
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub snapshots: bool,
    pub query: String,
    pub mc: Option<String>,
    pub loader: String,
    /// `None` = la más reciente.
    pub loader_version: Option<String>,
    versions: HashMap<bool, Load<Vec<Version>>>,
    loader_versions: HashMap<(String, String), Load<Vec<String>>>,
}

impl Picker {
    pub fn new(mc: Option<String>, loader: &str, loader_version: Option<String>) -> Self {
        Self {
            snapshots: false,
            query: String::new(),
            mc,
            loader: loader.to_owned(),
            loader_version,
            versions: HashMap::new(),
            loader_versions: HashMap::new(),
        }
    }

    /// Lista de versiones de la vista actual (con o sin snapshots).
    pub fn versions(&self) -> Option<&Load<Vec<Version>>> {
        self.versions.get(&self.snapshots)
    }

    /// Versiones del loader elegido para la versión elegida (`None` = aún no
    /// pedidas, o no hacen falta: vanilla o sin versión).
    pub fn loader_versions(&self) -> Option<&Load<Vec<String>>> {
        let mc = self.mc.as_ref()?;
        if self.loader == "vanilla" {
            return None;
        }
        self.loader_versions.get(&(self.loader.clone(), mc.clone()))
    }

    pub fn select_mc(&mut self, id: &str) {
        self.mc = Some(id.to_owned());
        self.loader_version = None;
    }

    pub fn select_loader(&mut self, loader: &str) {
        self.loader = loader.to_owned();
        self.loader_version = None;
    }

    pub fn set_versions(&mut self, snapshots: bool, r: Result<Vec<Version>, Failure>) {
        self.versions.insert(snapshots, r.map_or_else(Load::Failed, Load::Ready));
    }

    pub fn set_loader_versions(&mut self, loader: &str, mc: &str, r: Result<Vec<String>, Failure>) {
        self.loader_versions.insert((loader.to_owned(), mc.to_owned()), r.map_or_else(Load::Failed, Load::Ready));
    }

    /// Lo que falta pedir para lo que se ve; lo marca como «cargando» para no
    /// pedirlo dos veces.
    pub fn requests(&mut self) -> Vec<Request> {
        let mut out = Vec::new();
        if !self.versions.contains_key(&self.snapshots) {
            self.versions.insert(self.snapshots, Load::Loading);
            out.push(Request::Versions { snapshots: self.snapshots });
        }
        if let Some(mc) = self.mc.clone()
            && self.loader != "vanilla"
            && let Entry::Vacant(e) = self.loader_versions.entry((self.loader.clone(), mc.clone()))
        {
            e.insert(Load::Loading);
            out.push(Request::LoaderVersions { loader: self.loader.clone(), mc });
        }
        out
    }

    /// Versión del loader que se guarda (`None` en vanilla o «la más reciente»).
    pub fn chosen_loader_version(&self) -> Option<String> {
        if self.loader == "vanilla" { None } else { self.loader_version.clone() }
    }

    /// Lo que pinta Slint.
    pub fn view(&self, lang: &str) -> PickerData {
        let (all, loading, failure): (&[Version], bool, Option<&Failure>) = match self.versions() {
            Some(Load::Ready(l)) => (l, false, None),
            Some(Load::Failed(f)) => (&[], false, Some(f)),
            Some(Load::Loading) | None => (&[], true, None),
        };
        let latest = latest_release(all);
        let rows: Vec<VersionRow> = filter(all, &self.query)
            .into_iter()
            .map(|v| VersionRow { id: v.id.as_str().into(), kind: v.kind.as_str().into(), latest: Some(v.id.as_str()) == latest })
            .collect();
        let latest_label = i18n::text(lang, "createLoaderLatest");
        let mut data = PickerData {
            rows: ModelRc::new(VecModel::from(rows)),
            loading,
            error: failure.map(|f| f.message.as_str()).unwrap_or_default().into(),
            error_detail: failure.map(|f| f.detail.as_str()).unwrap_or_default().into(),
            snapshots: self.snapshots,
            selected: self.mc.clone().unwrap_or_default().into(),
            loader: self.loader.as_str().into(),
            lv_value: self.loader_version.clone().unwrap_or_default().into(),
            lv_label: self.loader_version.clone().unwrap_or(latest_label.clone()).into(),
            ..Default::default()
        };
        // 0 = sin campo, 1 = cargando, 2 = lista, 3 = vacía, 4 = error.
        data.lv_state = match self.loader_versions() {
            None if self.mc.is_none() || self.loader == "vanilla" => 0,
            None | Some(Load::Loading) => 1,
            Some(Load::Ready(l)) if l.is_empty() => {
                let mc = self.mc.as_deref().unwrap_or_default();
                let text = i18n::text(lang, "createNoLoaderVersions")
                    .replace("{loader}", &loader_label(lang, &self.loader))
                    .replace("{mc}", mc);
                data.lv_empty = text.into();
                3
            }
            Some(Load::Ready(l)) => {
                let mut opts = vec![SelectOption { value: "".into(), label: latest_label.into(), ..Default::default() }];
                opts.extend(l.iter().map(|v| SelectOption { value: v.as_str().into(), label: v.as_str().into(), ..Default::default() }));
                data.lv_options = ModelRc::new(VecModel::from(opts));
                2
            }
            Some(Load::Failed(f)) => {
                data.lv_error = f.message.as_str().into();
                data.lv_error_detail = f.detail.as_str().into();
                4
            }
        };
        data
    }
}

/// Instancia que se pide crear.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewInstance {
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: Option<String>,
}

/// Lo que el diálogo pide fuera de la UI.
pub trait CreateActions {
    /// → [`CreateInstance::versions_loaded`].
    fn list_versions(&self, snapshots: bool);
    /// → [`CreateInstance::loader_versions_loaded`].
    fn loader_versions(&self, loader: &str, mc: &str);
    /// Crea, recarga la lista de instancias y → [`CreateInstance::created`]
    /// con el mismo `attempt`.
    fn create(&self, new: &NewInstance, attempt: u64);
}

thread_local! {
    static CREATE: RefCell<Option<Rc<CreateInstance>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador del diálogo.
pub fn post(f: impl FnOnce(&CreateInstance) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador del diálogo, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&CreateInstance)) {
    CREATE.with(|c| {
        if let Some(x) = c.borrow().as_ref() {
            f(x);
        }
    })
}

/// Estado del diálogo abierto.
struct Session {
    picker: Picker,
    name: String,
    /// El usuario tocó el nombre: ya no se propone.
    name_edited: bool,
    /// Se pulsó «Crear» (el error de nombre vacío se ve desde entonces).
    touched: bool,
    busy: bool,
    error: Option<Failure>,
}

pub struct CreateInstance {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn CreateActions>,
    session: RefCell<Option<Session>>,
    /// Cambia al abrir y al cerrar: un resultado de otro intento no toca el
    /// diálogo.
    attempt: Cell<u64>,
}

impl CreateInstance {
    /// Crea el controlador y lo registra para [`post`]. Lo abre
    /// [`CreateInstance::open`] (ver [`dialog_hooks`]).
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn CreateActions>) -> Rc<Self> {
        let me = Rc::new(Self { ui: ui.as_weak(), shell, actions, session: RefCell::default(), attempt: Cell::new(0) });
        CREATE.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<CreateInstanceState>();
        macro_rules! on {
            ($set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
                let weak = Rc::downgrade(&me);
                st.$set(move |$($arg),*| {
                    if let Some($c) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        on!(on_cancel, |c| c.close());
        on!(on_create, |c| c.create());
        on!(on_name_edited, |c, t| c.update(|s| {
            s.name = t.to_string();
            s.name_edited = true;
        }));
        on!(on_search_edited, |c, t| c.update(|s| s.picker.query = t.to_string()));
        on!(on_snapshots, |c, v| c.update(|s| {
            s.picker.snapshots = v;
            s.picker.mc = None;
        }));
        on!(on_select_version, |c, id| c.update(|s| s.picker.select_mc(&id)));
        on!(on_select_loader, |c, l| c.update(|s| s.picker.select_loader(&l)));
        on!(on_select_loader_version, |c, v| c.update(|s| s.picker.loader_version = (!v.is_empty()).then(|| v.to_string())));
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    pub fn is_open(&self) -> bool {
        self.session.borrow().is_some()
    }

    /// Lo que se va a crear si se pulsa «Crear» ahora (tests).
    pub fn pending(&self) -> Option<NewInstance> {
        let s = self.session.borrow();
        let s = s.as_ref()?;
        Some(NewInstance {
            name: s.name.trim().to_owned(),
            mc: s.picker.mc.clone()?,
            loader: s.picker.loader.clone(),
            loader_version: s.picker.chosen_loader_version(),
        })
    }

    /// Abre el diálogo limpio (vanilla, sin nombre) y pide las versiones.
    pub fn open(&self) {
        self.attempt.set(self.attempt.get() + 1);
        *self.session.borrow_mut() = Some(Session {
            picker: Picker::new(None, "vanilla", None),
            name: String::new(),
            name_edited: false,
            touched: false,
            busy: false,
            error: None,
        });
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<CreateInstanceState>();
            st.set_search("".into());
            st.set_name("".into());
            st.set_open(true);
        }
        self.update(|_| {});
    }

    pub fn close(&self) {
        self.attempt.set(self.attempt.get() + 1);
        *self.session.borrow_mut() = None;
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CreateInstanceState>().set_open(false);
        }
    }

    /// Cambia el estado, elige la versión por defecto si falta, propone el
    /// nombre, pide lo que falte y lo copia a la UI.
    fn update(&self, f: impl FnOnce(&mut Session)) {
        let lang = self.lang();
        let requests = {
            let mut guard = self.session.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            let before = (s.picker.mc.clone(), s.picker.loader.clone());
            f(s);
            // Elegir la última release al cargar (Flutter, tras el frame).
            if s.picker.mc.is_none()
                && let Some(Load::Ready(list)) = s.picker.versions()
                && let Some(id) = default_version(list).map(str::to_owned)
            {
                s.picker.mc = Some(id);
            }
            if !s.name_edited
                && (s.picker.mc.clone(), s.picker.loader.clone()) != before
                && let Some(mc) = &s.picker.mc
            {
                s.name = suggested_name(&lang, &s.picker.loader, mc);
            }
            s.picker.requests()
        };
        for r in requests {
            match r {
                Request::Versions { snapshots } => self.actions.list_versions(snapshots),
                Request::LoaderVersions { loader, mc } => self.actions.loader_versions(&loader, &mc),
            }
        }
        self.refresh();
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let guard = self.session.borrow();
        let Some(s) = guard.as_ref() else { return };
        let lang = self.lang();
        let st = ui.global::<CreateInstanceState>();
        st.set_picker(s.picker.view(&lang));
        if st.get_name().as_str() != s.name {
            st.set_name(s.name.as_str().into());
        }
        let name_error = if s.touched && s.name.trim().is_empty() { i18n::text(&lang, "createNameRequired").into() } else { SharedString::new() };
        st.set_name_error(name_error);
        st.set_busy(s.busy);
        st.set_can_create(!s.busy && s.picker.mc.is_some());
        st.set_error(s.error.as_ref().map(|f| f.message.as_str()).unwrap_or_default().into());
        st.set_error_detail(s.error.as_ref().map(|f| f.detail.as_str()).unwrap_or_default().into());
    }

    /// «Crear»: valida y pide crear.
    fn create(&self) {
        let new = {
            let mut guard = self.session.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            s.touched = true;
            match &s.picker.mc {
                Some(mc) if !s.name.trim().is_empty() && !s.busy => {
                    s.busy = true;
                    s.error = None;
                    Some(NewInstance {
                        name: s.name.trim().to_owned(),
                        mc: mc.clone(),
                        loader: s.picker.loader.clone(),
                        loader_version: s.picker.chosen_loader_version(),
                    })
                }
                _ => None,
            }
        };
        if let Some(new) = new {
            self.actions.create(&new, self.attempt.get());
        }
        self.refresh();
    }

    pub fn versions_loaded(&self, snapshots: bool, r: Result<Vec<Version>, Failure>) {
        self.update(|s| s.picker.set_versions(snapshots, r));
    }

    pub fn loader_versions_loaded(&self, loader: &str, mc: &str, r: Result<Vec<String>, Failure>) {
        self.update(|s| s.picker.set_loader_versions(loader, mc, r));
    }

    /// Resultado de [`CreateActions::create`]: `(id, nombre)` de la nueva.
    pub fn created(&self, attempt: u64, r: Result<(String, String), Failure>) {
        // Diálogo cerrado o reabierto: la lista ya se recargó, nada más.
        if attempt != self.attempt.get() || !self.is_open() {
            return;
        }
        match r {
            Ok((id, name)) => {
                self.close();
                self.shell.toast(&fill(&i18n::text(&self.lang(), "createDone"), &name), false);
                self.shell.open_instance(&id, None);
            }
            Err(f) => self.update(|s| {
                s.busy = false;
                s.error = Some(f);
            }),
        }
    }
}

/// Registra «Nueva instancia» (Instancias e Inicio) y la papelera en el
/// controlador principal.
pub fn dialog_hooks(ctl: &Controller, create: &Rc<CreateInstance>, trash: &Rc<crate::trash::Trash>) {
    let (c, t) = (Rc::downgrade(create), Rc::downgrade(trash));
    ctl.set_dialog_hooks(
        move || {
            if let Some(c) = c.upgrade() {
                c.open();
            }
        },
        move || {
            if let Some(t) = t.upgrade() {
                t.open();
            }
        },
    );
}

/// Versiones de Minecraft del núcleo (`list_versions` del bridge).
pub(crate) async fn core_versions(core: &lcore::Core, snapshots: bool) -> lcore::Result<Vec<Version>> {
    Ok(from_manifest(core.version_manifest().await?.versions, snapshots))
}

/// Versiones de un loader del núcleo (`list_loader_versions` del bridge).
pub(crate) async fn core_loader_versions(core: &lcore::Core, loader: &str, mc: &str) -> lcore::Result<Vec<String>> {
    let kind = parse_loader(loader)?;
    Ok(core.loader_versions(kind, mc).await?.into_iter().map(|v| v.version).collect())
}

pub(crate) fn parse_loader(loader: &str) -> lcore::Result<lcore::instance::LoaderKind> {
    lcore::instance::LoaderKind::parse(loader).ok_or_else(|| lcore::Error::InvalidInput(format!("loader desconocido: {loader}")))
}

/// Recarga la lista de instancias del controlador principal (el `refresh()`
/// de Flutter tras crear, cambiar de versión o recuperar).
pub(crate) async fn reload_instances(core: &lcore::Core) {
    match core.list_instances().await {
        Ok(list) => {
            let list: Vec<Record> = list.into_iter().map(Record::from).collect();
            crate::backend::post(move |c| c.set_instances(list));
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo recargar la lista de instancias"),
    }
}

/// [`CreateActions`] contra el núcleo.
pub struct CoreCreate(pub Backend);

impl CreateActions for CoreCreate {
    fn list_versions(&self, snapshots: bool) {
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            let r = core_versions(&core, snapshots).await;
            post(move |c| c.versions_loaded(snapshots, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }

    fn loader_versions(&self, loader: &str, mc: &str) {
        let core = self.0.core.clone();
        let (loader, mc) = (loader.to_owned(), mc.to_owned());
        self.0.rt.spawn(async move {
            let r = core_loader_versions(&core, &loader, &mc).await;
            post(move |c| c.loader_versions_loaded(&loader, &mc, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }

    fn create(&self, new: &NewInstance, attempt: u64) {
        let core = self.0.core.clone();
        let new = new.clone();
        self.0.rt.spawn(async move {
            let r = create_in_core(&core, new).await;
            if r.is_ok() {
                reload_instances(&core).await;
            }
            let r = r.map(|i| (i.id, i.name));
            post(move |c| c.created(attempt, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }
}

/// `create_instance` del bridge.
pub async fn create_in_core(core: &lcore::Core, new: NewInstance) -> lcore::Result<lcore::instance::Instance> {
    let game = lcore::instance::GameVersion { mc: new.mc, loader: parse_loader(&new.loader)?, loader_version: new.loader_version };
    core.create_instance(lcore::instance::NewInstance { name: new.name, game }).await
}

impl CreateActions for crate::fixtures::Recorder {
    fn list_versions(&self, snapshots: bool) {
        self.push(format!("versions {snapshots}"));
    }
    fn loader_versions(&self, loader: &str, mc: &str) {
        self.push(format!("loader-versions {loader} {mc}"));
    }
    fn create(&self, new: &NewInstance, attempt: u64) {
        let lv = new.loader_version.as_deref().unwrap_or("-");
        self.push(format!("create {}|{}|{}|{lv} #{attempt}", new.name, new.mc, new.loader));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::versions::{sample_loader_versions, sample_versions};
    use slint::Model;

    fn failure() -> Failure {
        Failure { kind: "network".into(), message: "sin red".into(), detail: "network: x".into() }
    }

    #[test]
    fn ultima_release_y_version_por_defecto() {
        let with_snap = sample_versions(true);
        assert_eq!(with_snap[0].kind, "snapshot");
        assert_eq!(latest_release(&with_snap), Some("1.21.5"));
        assert_eq!(default_version(&with_snap), Some("1.21.5"));
        let only_snap = vec![Version::new("25w14a", "snapshot")];
        assert_eq!(latest_release(&only_snap), None);
        assert_eq!(default_version(&only_snap), Some("25w14a"));
        assert_eq!(default_version(&[]), None);
    }

    #[test]
    fn buscar_por_id_sin_mayusculas() {
        let list = sample_versions(true);
        let ids = |q| filter(&list, q).into_iter().map(|v| v.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(" 1.21. "), ["1.21.5", "1.21.4", "1.21.3", "1.21.1"]);
        assert_eq!(ids("W14"), ["25w14a"]);
        assert_eq!(ids("").len(), list.len());
        assert!(ids("zzz").is_empty());
    }

    #[test]
    fn manifiesto_sin_betas_y_snapshots_si_se_piden() {
        let e = |id: &str, kind: &str| lcore::meta::ManifestEntry {
            id: id.into(),
            kind: kind.into(),
            url: String::new(),
            sha1: None,
            release_time: None,
        };
        let all = || vec![e("25w14a", "snapshot"), e("1.21.5", "release"), e("b1.7.3", "old_beta"), e("a1.0", "old_alpha")];
        assert_eq!(from_manifest(all(), false), [Version::new("1.21.5", "release")]);
        assert_eq!(from_manifest(all(), true), [Version::new("25w14a", "snapshot"), Version::new("1.21.5", "release")]);
    }

    #[test]
    fn nombre_propuesto() {
        assert_eq!(suggested_name("es", "vanilla", "1.21.5"), "Minecraft 1.21.5");
        assert_eq!(suggested_name("es", "fabric", "1.21.5"), "Fabric 1.21.5");
        assert_eq!(suggested_name("en", "neoforge", "1.20.1"), "NeoForge 1.20.1");
        assert_eq!(version_line("en", "1.21.4", "vanilla"), "1.21.4 · Vanilla");
    }

    #[test]
    fn pide_cada_lista_una_vez() {
        let mut p = Picker::new(None, "vanilla", None);
        assert_eq!(p.requests(), [Request::Versions { snapshots: false }]);
        assert!(p.requests().is_empty(), "ya se está cargando");
        p.set_versions(false, Ok(sample_versions(false)));
        p.select_mc("1.21.5");
        assert!(p.requests().is_empty(), "vanilla no tiene versiones de loader");
        p.select_loader("fabric");
        assert_eq!(p.requests(), [Request::LoaderVersions { loader: "fabric".into(), mc: "1.21.5".into() }]);
        p.snapshots = true;
        assert_eq!(p.requests(), [Request::Versions { snapshots: true }]);
        // Volver a lo ya cargado no pide nada.
        p.snapshots = false;
        p.select_loader("quilt");
        p.select_loader("fabric");
        assert_eq!(p.requests(), [], "fabric ya está");
    }

    #[test]
    fn elegir_version_o_loader_vuelve_a_la_mas_reciente() {
        let mut p = Picker::new(Some("1.21.4".into()), "fabric", Some("0.16.10".into()));
        assert_eq!(p.chosen_loader_version().as_deref(), Some("0.16.10"));
        p.select_mc("1.21.5");
        assert_eq!(p.chosen_loader_version(), None);
        p.loader_version = Some("0.16.14".into());
        p.select_loader("vanilla");
        assert_eq!(p.chosen_loader_version(), None);
        p.loader_version = Some("x".into());
        assert_eq!(p.chosen_loader_version(), None, "vanilla nunca guarda versión de loader");
    }

    #[test]
    fn vista_del_campo_de_version_del_loader() {
        let mut p = Picker::new(Some("1.21.5".into()), "vanilla", None);
        assert_eq!(p.view("es").lv_state, 0);
        p.select_loader("fabric");
        assert_eq!(p.view("es").lv_state, 1);
        p.set_loader_versions("fabric", "1.21.5", Ok(sample_loader_versions()));
        let v = p.view("es");
        assert_eq!((v.lv_state, v.lv_label.as_str(), v.lv_value.as_str()), (2, "La más reciente", ""));
        assert_eq!(v.lv_options.row_count(), 5);
        p.set_loader_versions("fabric", "1.21.5", Ok(vec![]));
        let v = p.view("es");
        assert_eq!((v.lv_state, v.lv_empty.as_str()), (3, "No hay versiones de Fabric para 1.21.5."));
        p.set_loader_versions("fabric", "1.21.5", Err(failure()));
        assert_eq!(p.view("es").lv_state, 4);
    }

    #[test]
    fn vista_de_la_lista_marca_la_ultima_y_filtra() {
        let mut p = Picker::new(None, "vanilla", None);
        assert!(p.view("es").loading);
        p.set_versions(false, Ok(sample_versions(false)));
        p.query = "1.21".into();
        let v = p.view("es");
        assert!(!v.loading);
        let rows: Vec<_> = v.rows.iter().map(|r| (r.id.to_string(), r.latest)).collect();
        assert_eq!(rows[0], ("1.21.5".to_string(), true));
        assert_eq!(rows.len(), 5);
        assert!(rows[1..].iter().all(|r| !r.1));
        p.set_versions(false, Err(failure()));
        assert_eq!(p.view("es").error.as_str(), "sin red");
    }
}
