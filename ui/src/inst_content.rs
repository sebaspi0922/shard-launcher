//! Pestaña Contenido del detalle de instancia (content_tab.dart y la parte
//! de contenido de instance_data.dart): lista de mods, paquetes de recursos y
//! shaders con filtro por tipo y texto, iconos de Modrinth/CurseForge,
//! actualizaciones (una o todas), identificar archivos desconocidos,
//! activar/desactivar y quitar.
//!
//! `InstContent` corre en el hilo de la UI y copia el estado a
//! `InstContentState`. El trabajo con el núcleo lo hace un
//! [`ContentActions`] ([`CoreContent`] en la app, `fixtures::Recorder` en los
//! tests) que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

use crate::backend::Backend;
use crate::controller::Controller;
use crate::errors::Failure;
use crate::instance::{DetailEvent, InstanceDetail};
use crate::{ContentRow, InstContentState, InstanceTab, LoadState, MainWindow, Route, i18n};

/// Identificador sin ambigüedad de un archivo (`tipo/archivo`), el que
/// aceptan `set_content_enabled`, `remove_content` y `update_content`.
pub fn key(kind: &str, file: &str) -> String {
    format!("{kind}/{file}")
}

/// Un archivo instalado (`InstalledContentView` del bridge, lo que usa la
/// pestaña).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: String,
    /// Nombre sin `.disabled`.
    pub file: String,
    pub enabled: bool,
    /// `modrinth`, `curseforge` o `unknown`.
    pub source: String,
    pub project_name: Option<String>,
    pub version_number: Option<String>,
    pub size: u64,
    pub is_dir: bool,
}

impl Item {
    pub fn from_core(c: &lcore::content::InstalledContent) -> Self {
        Self {
            kind: c.kind.as_str().into(),
            file: c.file.clone(),
            enabled: c.enabled,
            source: c.source.clone(),
            project_name: c.project_name.clone(),
            version_number: c.version_number.clone(),
            size: c.size,
            is_dir: c.is_dir,
        }
    }

    pub fn key(&self) -> String {
        key(&self.kind, &self.file)
    }

    /// Lo que se muestra como título: el proyecto o, si no se conoce, el
    /// archivo.
    pub fn name(&self) -> &str {
        self.project_name.as_deref().unwrap_or(&self.file)
    }
}

/// Versión nueva de un archivo (`ContentUpdateView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub kind: String,
    pub file: String,
    pub project_name: Option<String>,
    pub latest: String,
}

impl Update {
    pub fn key(&self) -> String {
        key(&self.kind, &self.file)
    }
}

/// Filtro por tipo (`_Filter`), en el orden de las píldoras.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Mod,
    ResourcePack,
    Shader,
}

impl Filter {
    pub const ALL: [Filter; 4] = [Filter::All, Filter::Mod, Filter::ResourcePack, Filter::Shader];

    pub fn from_index(i: i32) -> Self {
        usize::try_from(i).ok().and_then(|i| Self::ALL.get(i).copied()).unwrap_or_default()
    }

    pub fn index(self) -> i32 {
        Self::ALL.iter().position(|f| *f == self).unwrap_or(0) as i32
    }

    /// Tipo que deja pasar (`None` = todos).
    pub fn kind(self) -> Option<&'static str> {
        match self {
            Filter::All => None,
            Filter::Mod => Some("mod"),
            Filter::ResourcePack => Some("resourcepack"),
            Filter::Shader => Some("shader"),
        }
    }

    /// Qué se busca con «Añadir» (`_add`): con «Todo», mods.
    pub fn add_kind(self) -> &'static str {
        self.kind().unwrap_or("mod")
    }
}

/// Lo que lista la pestaña: todo menos los datapacks (van en Mundos).
pub fn listed(items: &[Item]) -> impl Iterator<Item = &Item> {
    items.iter().filter(|c| c.kind != "datapack")
}

/// Cuántos hay de cada filtro, en el orden de [`Filter::ALL`].
pub fn counts(items: &[Item]) -> [i32; 4] {
    Filter::ALL.map(|f| listed(items).filter(|c| f.kind().is_none_or(|k| c.kind == k)).count() as i32)
}

/// `_matches`: tipo del filtro y texto en «proyecto archivo», sin mayúsculas.
pub fn matches(c: &Item, filter: Filter, query: &str) -> bool {
    if c.kind == "datapack" || filter.kind().is_some_and(|k| c.kind != k) {
        return false;
    }
    let q = query.trim().to_lowercase();
    q.is_empty() || format!("{} {}", c.project_name.as_deref().unwrap_or(""), c.file).to_lowercase().contains(&q)
}

/// Archivos de origen desconocido (los que «Identificar» intenta reconocer).
pub fn unknown(items: &[Item]) -> usize {
    listed(items).filter(|c| c.source == "unknown").count()
}

/// `formatBytes` (format.dart): «512 B», «760 KB», «2,2 MB», «1,5 GB».
pub fn format_bytes(lang: &str, bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    // NumberFormat('0.#'): una cifra decimal como mucho, el medio hacia
    // arriba.
    let one = |x: f64| {
        let t = (x * 10.0).round() as u64;
        if t.is_multiple_of(10) { format!("{}", t / 10) } else { format!("{}{}{}", t / 10, i18n::decimal(lang), t % 10) }
    };
    if b < K {
        format!("{bytes} B")
    } else if b < K * K {
        format!("{} KB", (b / K).round() as u64)
    } else if b < K * K * K {
        format!("{} MB", one(b / (K * K)))
    } else {
        format!("{} GB", one(b / (K * K * K)))
    }
}

/// Segunda línea de la fila: archivo (si hay nombre de proyecto), versión y
/// tamaño (si no es carpeta).
pub fn details(lang: &str, c: &Item) -> String {
    let mut parts: Vec<String> = Vec::new();
    if c.project_name.is_some() {
        parts.push(c.file.clone());
    }
    if let Some(v) = &c.version_number {
        parts.push(v.clone());
    }
    if !c.is_dir {
        parts.push(format_bytes(lang, c.size));
    }
    parts.join("  ·  ")
}

/// Texto de una clave con plural del ARB (`{n, plural, =1{…} other{…}}`)
/// para `n`, con la variable sustituida. Español e inglés: «one» solo para 1.
pub fn plural(lang: &str, key: &str, n: usize) -> String {
    let msg = i18n::text(lang, key);
    let form = match crate::arb::convert(&msg, &[]).ok().and_then(|v| v.into_iter().next()) {
        Some((_, crate::arb::Msg::Plural { one, other })) => {
            if n == 1 {
                one
            } else {
                other
            }
        }
        Some((_, crate::arb::Msg::Simple(s))) => s,
        None => msg,
    };
    form.replace("{n}", &n.to_string())
}

/// `{nombre}` → valor en un texto del ARB.
fn fill(text: &str, args: &[(&str, &str)]) -> String {
    args.iter().fold(text.to_owned(), |t, (k, v)| t.replace(&format!("{{{k}}}"), v))
}

/// Ancho al que se decodifican los iconos (`cacheWidth` de ProjectIcon: 40
/// en pantalla × 2).
pub const ICON_PX: u32 = 80;

/// Icono de proyecto a `width` px de ancho (alto proporcional), en RGBA.
pub fn decode_icon(bytes: &[u8], width: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let img = image::load_from_memory(bytes).ok()?;
    let h = (f64::from(img.height()) * f64::from(width) / f64::from(img.width().max(1))).round().max(1.0) as u32;
    let img = img.resize_exact(width, h, image::imageops::FilterType::Triangle).to_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(img.as_raw(), img.width(), img.height()))
}

/// Lo que la pestaña pide fuera de la UI. Cada resultado vuelve al
/// controlador con el mismo `id` de instancia.
pub trait ContentActions {
    /// Lista → [`InstContent::listed`].
    fn list(&self, id: &str);
    /// Iconos (mejor esfuerzo) → [`InstContent::icons_loaded`].
    fn icons(&self, id: &str);
    /// Imagen de un icono → [`InstContent::icon_loaded`].
    fn icon_image(&self, url: &str);
    /// Versiones nuevas → [`InstContent::updates_loaded`].
    fn check_updates(&self, id: &str);
    /// → [`InstContent::enabled_set`].
    fn set_enabled(&self, id: &str, key: &str, enabled: bool);
    /// → [`InstContent::removed`].
    fn remove(&self, id: &str, key: &str, name: &str);
    /// → [`InstContent::updated`].
    fn update(&self, id: &str, key: &str, name: &str);
    /// Una tras otra → [`InstContent::updated_all`].
    fn update_all(&self, id: &str, keys: &[String]);
    /// → [`InstContent::identified`].
    fn identify(&self, id: &str);
    /// «Añadir»: la pantalla Contenido para la instancia y el tipo elegidos
    /// (`/content?instance=&kind=`).
    fn browse(&self, id: &str, kind: &str);
}

thread_local! {
    static CONTENT: RefCell<Option<Rc<InstContent>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de la pestaña.
pub fn post(f: impl FnOnce(&InstContent) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        CONTENT.with(|c| {
            if let Some(x) = c.borrow().as_ref() {
                f(x);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Estado de la lista.
#[derive(Clone, Debug, PartialEq)]
enum Load {
    Loading,
    Ready,
    Failed(Failure),
}

pub struct InstContent {
    ui: slint::Weak<MainWindow>,
    ctl: Rc<Controller>,
    actions: Box<dyn ContentActions>,
    /// Instancia cuya pestaña Contenido se ve (`None` = otra pestaña).
    id: RefCell<Option<String>>,
    items: RefCell<Vec<Item>>,
    load: RefCell<Load>,
    /// URL del icono por `key`.
    icons: RefCell<HashMap<String, String>>,
    /// Imagen por URL (`None` = pedida o sin imagen: icono del tipo).
    images: RefCell<HashMap<String, Option<Image>>>,
    updates: RefCell<Vec<Update>>,
    updates_loading: Cell<bool>,
    filter: Cell<Filter>,
    query: RefCell<String>,
    /// Filas actualizándose una a una.
    updating: RefCell<HashSet<String>>,
    updating_all: Cell<bool>,
    identifying: Cell<bool>,
    rows: Rc<VecModel<ContentRow>>,
}

macro_rules! on {
    ($me:expr, $st:expr, $set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$me);
        $st.$set(move |$($arg),*| {
            if let Some($c) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl InstContent {
    pub fn new(ui: &MainWindow, detail: &InstanceDetail, actions: Box<dyn ContentActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            ctl: detail.controller().clone(),
            actions,
            id: RefCell::default(),
            items: RefCell::default(),
            load: RefCell::new(Load::Loading),
            icons: RefCell::default(),
            images: RefCell::default(),
            updates: RefCell::default(),
            updates_loading: Cell::new(false),
            filter: Cell::new(Filter::All),
            query: RefCell::default(),
            updating: RefCell::default(),
            updating_all: Cell::new(false),
            identifying: Cell::new(false),
            rows: Rc::new(VecModel::default()),
        });
        CONTENT.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<InstContentState>();
        st.set_rows(ModelRc::from(me.rows.clone()));
        on!(me, st, on_select_filter, |c, i| {
            c.filter.set(Filter::from_index(i));
            c.refresh();
        });
        on!(me, st, on_query, |c, q| {
            *c.query.borrow_mut() = q.to_string();
            c.refresh();
        });
        on!(me, st, on_refresh, |c| c.check_updates());
        on!(me, st, on_add, |c| {
            if let Some(id) = c.id() {
                c.actions.browse(&id, c.filter.get().add_kind());
            }
        });
        on!(me, st, on_update_all, |c| c.update_all());
        on!(me, st, on_identify, |c| c.identify());
        on!(me, st, on_set_enabled, |c, k, v| c.set_enabled(&k, v));
        on!(me, st, on_update, |c, k| c.update(&k));
        on!(me, st, on_remove, |c, k| c.remove(&k));
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            let Some(c) = weak.upgrade() else { return };
            match e {
                DetailEvent::Shown { id, tab: InstanceTab::Content } => c.show(id),
                DetailEvent::Shown { .. } | DetailEvent::Closed => *c.id.borrow_mut() = None,
                _ => {}
            }
        });
        me
    }

    /// Instancia cuya pestaña se ve.
    pub fn id(&self) -> Option<String> {
        self.id.borrow().clone()
    }

    /// Lo que hay en la lista (sin filtrar).
    pub fn items(&self) -> Vec<Item> {
        self.items.borrow().clone()
    }

    fn lang(&self) -> String {
        self.ctl.lang()
    }

    fn is_current(&self, id: &str) -> bool {
        self.id.borrow().as_deref() == Some(id)
    }

    /// Se muestra la pestaña de `id`: como Flutter (estado nuevo y datos
    /// pedidos otra vez), sin filtro y con la lista cargando.
    fn show(&self, id: &str) {
        *self.id.borrow_mut() = Some(id.to_owned());
        self.items.borrow_mut().clear();
        *self.load.borrow_mut() = Load::Loading;
        self.icons.borrow_mut().clear();
        self.updates.borrow_mut().clear();
        self.filter.set(Filter::All);
        self.query.borrow_mut().clear();
        self.updating.borrow_mut().clear();
        self.updating_all.set(false);
        self.identifying.set(false);
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<InstContentState>().set_remove_key("".into());
        }
        self.actions.list(id);
        self.check_updates();
    }

    /// Botón de refrescar: versiones nuevas e iconos.
    fn check_updates(&self) {
        let Some(id) = self.id() else { return };
        self.updates_loading.set(true);
        self.actions.check_updates(&id);
        self.actions.icons(&id);
        self.refresh();
    }

    /// Tras actualizar o identificar: todo otra vez (lista, versiones,
    /// iconos), sin volver al estado de carga.
    fn reload_all(&self, id: &str) {
        self.actions.list(id);
        self.check_updates();
    }

    fn toast_error(&self, f: &Failure) {
        self.ctl.toast(&f.message, true);
    }

    // --- Resultados -------------------------------------------------------

    /// Resultado de [`ContentActions::list`].
    pub fn listed(&self, id: &str, result: Result<Vec<Item>, Failure>) {
        if !self.is_current(id) {
            return;
        }
        match result {
            Ok(list) => {
                *self.items.borrow_mut() = list;
                *self.load.borrow_mut() = Load::Ready;
            }
            Err(f) => {
                tracing::warn!(id, error = %f.detail, "no se pudo leer el contenido");
                self.items.borrow_mut().clear();
                *self.load.borrow_mut() = Load::Failed(f);
            }
        }
        self.refresh();
    }

    /// Resultado de [`ContentActions::icons`]: `(key, url)`. Sin red no hay
    /// iconos y se ve el del tipo.
    pub fn icons_loaded(&self, id: &str, icons: Vec<(String, String)>) {
        if !self.is_current(id) {
            return;
        }
        for (_, url) in &icons {
            if !self.images.borrow().contains_key(url) {
                self.images.borrow_mut().insert(url.clone(), None);
                self.actions.icon_image(url);
            }
        }
        *self.icons.borrow_mut() = icons.into_iter().collect();
        self.refresh();
    }

    /// Resultado de [`ContentActions::icon_image`].
    pub fn icon_loaded(&self, url: &str, px: Option<SharedPixelBuffer<Rgba8Pixel>>) {
        self.images.borrow_mut().insert(url.to_owned(), px.map(Image::from_rgba8));
        self.refresh();
    }

    /// Resultado de [`ContentActions::check_updates`] (un error = sin avisos).
    pub fn updates_loaded(&self, id: &str, result: Result<Vec<Update>, Failure>) {
        if !self.is_current(id) {
            return;
        }
        self.updates_loading.set(false);
        *self.updates.borrow_mut() = result.unwrap_or_else(|f| {
            tracing::warn!(id, error = %f.detail, "no se pudieron buscar actualizaciones");
            Vec::new()
        });
        self.refresh();
    }

    /// Resultado de [`ContentActions::set_enabled`]: si falla, vuelve atrás.
    pub fn enabled_set(&self, id: &str, key: &str, enabled: bool, result: Result<(), Failure>) {
        let Err(f) = result else { return };
        if self.is_current(id) {
            self.patch_enabled(key, !enabled);
        }
        self.toast_error(&f);
    }

    /// Resultado de [`ContentActions::remove`].
    pub fn removed(&self, id: &str, name: &str, result: Result<(), Failure>) {
        match result {
            Ok(()) => {
                let msg = fill(&i18n::text(&self.lang(), "contentRemoved"), &[("name", name)]);
                self.ctl.toast(&msg, false);
                if self.is_current(id) {
                    self.actions.list(id);
                }
            }
            Err(f) => self.toast_error(&f),
        }
    }

    /// Resultado de [`ContentActions::update`]: versión nueva (`None` = ya
    /// estaba al día).
    pub fn updated(&self, id: &str, key: &str, name: &str, result: Result<Option<String>, Failure>) {
        self.updating.borrow_mut().remove(key);
        match result {
            Ok(v) => {
                if let Some(v) = v {
                    let msg = fill(&i18n::text(&self.lang(), "contentUpdated"), &[("name", name), ("version", &v)]);
                    self.ctl.toast(&msg, false);
                }
                if self.is_current(id) {
                    self.reload_all(id);
                }
            }
            Err(f) => self.toast_error(&f),
        }
        self.refresh();
    }

    /// Resultado de [`ContentActions::update_all`]: cuántos se actualizaron y
    /// cuántos fallaron.
    pub fn updated_all(&self, id: &str, ok: usize, failed: usize) {
        self.updating_all.set(false);
        let lang = self.lang();
        if failed == 0 {
            self.ctl.toast(&plural(&lang, "contentUpdatedAll", ok), false);
        } else {
            let msg = fill(&i18n::text(&lang, "contentUpdatedSome"), &[("ok", &ok.to_string()), ("failed", &failed.to_string())]);
            self.ctl.toast(&msg, true);
        }
        if self.is_current(id) {
            self.reload_all(id);
        }
        self.refresh();
    }

    /// Resultado de [`ContentActions::identify`]: `(reconocidos, sin
    /// reconocer)`.
    pub fn identified(&self, id: &str, result: Result<(usize, usize), Failure>) {
        self.identifying.set(false);
        match result {
            Ok((found, missing)) => {
                let msg = fill(
                    &i18n::text(&self.lang(), "contentIdentified"),
                    &[("found", &found.to_string()), ("missing", &missing.to_string())],
                );
                self.ctl.toast(&msg, false);
                if self.is_current(id) {
                    self.reload_all(id);
                }
            }
            Err(f) => self.toast_error(&f),
        }
        self.refresh();
    }

    // --- Acciones ---------------------------------------------------------

    fn name_of(&self, key: &str) -> String {
        self.items.borrow().iter().find(|c| c.key() == key).map(|c| c.name().to_owned()).unwrap_or_default()
    }

    /// Activa o desactiva al momento y avisa al núcleo (setEnabled).
    fn set_enabled(&self, key: &str, enabled: bool) {
        let Some(id) = self.id() else { return };
        self.patch_enabled(key, enabled);
        self.actions.set_enabled(&id, key, enabled);
    }

    fn patch_enabled(&self, key: &str, enabled: bool) {
        if let Some(c) = self.items.borrow_mut().iter_mut().find(|c| c.key() == key) {
            c.enabled = enabled;
        }
        self.refresh();
    }

    fn update(&self, key: &str) {
        let Some(id) = self.id() else { return };
        if self.updating_all.get() || self.updating.borrow().contains(key) {
            return;
        }
        let Some(u) = self.updates.borrow().iter().find(|u| u.key() == key).cloned() else { return };
        self.updating.borrow_mut().insert(key.to_owned());
        let name = u.project_name.unwrap_or(u.file);
        self.actions.update(&id, key, &name);
        self.refresh();
    }

    fn update_all(&self) {
        let Some(id) = self.id() else { return };
        let keys: Vec<String> = self.updates.borrow().iter().map(Update::key).collect();
        if self.updating_all.get() || keys.is_empty() {
            return;
        }
        self.updating_all.set(true);
        self.actions.update_all(&id, &keys);
        self.refresh();
    }

    fn identify(&self) {
        let Some(id) = self.id() else { return };
        if self.identifying.get() {
            return;
        }
        self.identifying.set(true);
        self.actions.identify(&id);
        self.refresh();
    }

    /// Confirmado en el diálogo de «Quitar».
    fn remove(&self, key: &str) {
        let Some(id) = self.id() else { return };
        let name = self.name_of(key);
        self.actions.remove(&id, key, &name);
    }

    /// Va a la pantalla Contenido (lo pide `CoreContent::browse`).
    /// Pendiente de F4: que esa pantalla reciba la instancia y el tipo.
    pub fn go_browse(&self, id: &str, kind: &str) {
        tracing::info!(id, kind, "Añadir contenido: pantalla Contenido (F4 la enlazará con la instancia y el tipo)");
        if let Some(ui) = self.ui.upgrade() {
            ui.set_route(Route::Content);
            ui.invoke_route_changed(Route::Content);
        }
    }

    // --- Vista ------------------------------------------------------------

    /// Copia el estado a `InstContentState`. Las filas se actualizan en su
    /// sitio (sin recrear elementos ni perder el hover).
    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<InstContentState>();
        let lang = self.lang();
        let items = self.items.borrow();
        let filter = self.filter.get();
        let query = self.query.borrow();
        let icons = self.icons.borrow();
        let images = self.images.borrow();
        let updates = self.updates.borrow();
        let latest: HashMap<String, &str> = updates.iter().map(|u| (u.key(), u.latest.as_str())).collect();
        let updating = self.updating.borrow();
        let rows: Vec<ContentRow> = items
            .iter()
            .filter(|c| matches(c, filter, &query))
            .map(|c| {
                let k = c.key();
                let image = icons.get(&k).and_then(|u| images.get(u).cloned().flatten());
                ContentRow {
                    key: k.as_str().into(),
                    kind: c.kind.as_str().into(),
                    name: c.name().into(),
                    details: details(&lang, c).into(),
                    source: c.source.as_str().into(),
                    enabled: c.enabled,
                    has_icon: image.is_some(),
                    icon: image.unwrap_or_default(),
                    update: latest.get(&k).copied().unwrap_or("").into(),
                    updating: updating.contains(&k) || self.updating_all.get(),
                }
            })
            .collect();
        let n = self.rows.row_count();
        for (i, r) in rows.iter().enumerate() {
            if i >= n {
                self.rows.push(r.clone());
            } else if self.rows.row_data(i).as_ref() != Some(r) {
                self.rows.set_row_data(i, r.clone());
            }
        }
        for i in (rows.len()..n).rev() {
            self.rows.remove(i);
        }
        st.set_counts(ModelRc::new(VecModel::from(counts(&items).to_vec())));
        st.set_any(listed(&items).next().is_some());
        st.set_filter(filter.index());
        st.set_unknown_count(unknown(&items) as i32);
        st.set_updates_count(updates.len() as i32);
        st.set_updates_loading(self.updates_loading.get());
        st.set_updating_all(self.updating_all.get());
        st.set_identifying(self.identifying.get());
        match &*self.load.borrow() {
            Load::Loading => st.set_load(LoadState::Loading),
            Load::Ready => st.set_load(LoadState::Ready),
            Load::Failed(f) => {
                st.set_load(LoadState::Failed);
                st.set_error(f.message.as_str().into());
                st.set_error_detail(f.detail.as_str().into());
            }
        }
    }
}

/// [`ContentActions`] contra el núcleo.
pub struct CoreContent(pub Backend);

/// Error del núcleo traducido en el hilo de la UI.
fn fail(c: &InstContent, e: &lcore::Error) -> Failure {
    Failure::new(&c.lang(), e)
}

impl ContentActions for CoreContent {
    fn list(&self, id: &str) {
        let (core, id) = (self.0.core.clone(), id.to_owned());
        self.0.rt.spawn(async move {
            let r = core.list_content(&id).await.map(|v| v.iter().map(Item::from_core).collect::<Vec<_>>());
            post(move |c| c.listed(&id, r.map_err(|e| fail(c, &e))));
        });
    }

    fn icons(&self, id: &str) {
        let (core, id) = (self.0.core.clone(), id.to_owned());
        self.0.rt.spawn(async move {
            let icons = match core.content_icons(&id).await {
                Ok(v) => v.into_iter().map(|i| (key(i.kind.as_str(), &i.file), i.icon_url)).collect(),
                Err(e) => {
                    tracing::debug!(error = %e, "sin iconos de contenido");
                    Vec::new()
                }
            };
            post(move |c| c.icons_loaded(&id, icons));
        });
    }

    fn icon_image(&self, url: &str) {
        let (core, url) = (self.0.core.clone(), url.to_owned());
        self.0.rt.spawn(async move {
            let px = crate::media::cached_image(&core, &url).await.and_then(|b| decode_icon(&b, ICON_PX));
            post(move |c| c.icon_loaded(&url, px));
        });
    }

    fn check_updates(&self, id: &str) {
        let (core, id) = (self.0.core.clone(), id.to_owned());
        self.0.rt.spawn(async move {
            let r = core.check_content_updates(&id).await.map(|v| {
                v.into_iter()
                    .map(|u| Update { kind: u.kind.as_str().into(), file: u.file, project_name: u.project_name, latest: u.latest.version_number })
                    .collect::<Vec<_>>()
            });
            post(move |c| c.updates_loaded(&id, r.map_err(|e| fail(c, &e))));
        });
    }

    fn set_enabled(&self, id: &str, key: &str, enabled: bool) {
        let (core, id, key) = (self.0.core.clone(), id.to_owned(), key.to_owned());
        self.0.rt.spawn(async move {
            let r = core.set_content_enabled(&id, &key, enabled).await.map(|_| ());
            post(move |c| c.enabled_set(&id, &key, enabled, r.map_err(|e| fail(c, &e))));
        });
    }

    fn remove(&self, id: &str, key: &str, name: &str) {
        let (core, id, key, name) = (self.0.core.clone(), id.to_owned(), key.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.remove_content(&id, &key).await.map(|_| ());
            post(move |c| c.removed(&id, &name, r.map_err(|e| fail(c, &e))));
        });
    }

    fn update(&self, id: &str, key: &str, name: &str) {
        let (core, id, key, name) = (self.0.core.clone(), id.to_owned(), key.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = core.update_content(&id, &key).await.map(|u| u.map(|u| u.to_version));
            post(move |c| c.updated(&id, &key, &name, r.map_err(|e| fail(c, &e))));
        });
    }

    fn update_all(&self, id: &str, keys: &[String]) {
        let (core, id, keys) = (self.0.core.clone(), id.to_owned(), keys.to_vec());
        self.0.rt.spawn(async move {
            let (mut ok, mut failed) = (0, 0);
            for k in &keys {
                match core.update_content(&id, k).await {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        tracing::warn!(file = %k, error = %e, "no se pudo actualizar");
                        failed += 1;
                    }
                }
            }
            post(move |c| c.updated_all(&id, ok, failed));
        });
    }

    fn identify(&self, id: &str) {
        let (core, id) = (self.0.core.clone(), id.to_owned());
        self.0.rt.spawn(async move {
            let r = core.identify_content(&id).await.map(|r| (r.identified.len(), r.unmatched.len()));
            post(move |c| c.identified(&id, r.map_err(|e| fail(c, &e))));
        });
    }

    fn browse(&self, id: &str, kind: &str) {
        let (id, kind) = (id.to_owned(), kind.to_owned());
        post(move |c| c.go_browse(&id, &kind));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, file: &str, name: Option<&str>, source: &str) -> Item {
        Item {
            kind: kind.into(),
            file: file.into(),
            enabled: true,
            source: source.into(),
            project_name: name.map(Into::into),
            version_number: None,
            size: 0,
            is_dir: false,
        }
    }

    #[test]
    fn tamanos_como_format_bytes() {
        assert_eq!(format_bytes("es", 512), "512 B");
        assert_eq!(format_bytes("es", 760 * 1024), "760 KB");
        assert_eq!(format_bytes("es", 2210 * 1024), "2,2 MB");
        assert_eq!(format_bytes("en", 2210 * 1024), "2.2 MB");
        assert_eq!(format_bytes("es", 6020 * 1024), "5,9 MB");
        // '0.#': sin decimal si es redondo.
        assert_eq!(format_bytes("es", 2 * 1024 * 1024), "2 MB");
        assert_eq!(format_bytes("es", 3 * 1024 * 1024 * 1024 / 2), "1,5 GB");
    }

    #[test]
    fn detalles_de_la_fila() {
        let mut c = item("mod", "sodium.jar", Some("Sodium"), "modrinth");
        c.version_number = Some("0.6.13".into());
        c.size = 1180 * 1024;
        assert_eq!(details("es", &c), "sodium.jar  ·  0.6.13  ·  1,2 MB");
        // Sin proyecto el archivo ya es el título; una carpeta no lleva tamaño.
        let mut d = item("resourcepack", "pack", None, "unknown");
        d.is_dir = true;
        assert_eq!(details("es", &d), "");
        assert_eq!(d.name(), "pack");
    }

    #[test]
    fn filtro_por_tipo_y_texto_sin_datapacks() {
        let list = vec![
            item("mod", "sodium-fabric.jar", Some("Sodium"), "modrinth"),
            item("mod", "mi-mod.jar", None, "unknown"),
            item("resourcepack", "Fresh.zip", Some("Fresh Animations"), "modrinth"),
            item("shader", "Compl.zip", Some("Complementary"), "modrinth"),
            item("datapack", "mundo/dp.zip", Some("Terralith"), "modrinth"),
        ];
        assert_eq!(counts(&list), [4, 2, 1, 1]);
        assert_eq!(unknown(&list), 1);
        let names = |f, q| list.iter().filter(|c| matches(c, f, q)).map(|c| c.name().to_owned()).collect::<Vec<_>>();
        assert_eq!(names(Filter::All, ""), ["Sodium", "mi-mod.jar", "Fresh Animations", "Complementary"]);
        assert_eq!(names(Filter::Mod, ""), ["Sodium", "mi-mod.jar"]);
        // Busca en el proyecto y en el archivo, sin mayúsculas ni espacios.
        assert_eq!(names(Filter::All, "  FRESH "), ["Fresh Animations"]);
        assert_eq!(names(Filter::All, "fabric"), ["Sodium"]);
        assert!(names(Filter::Shader, "sodium").is_empty());
        assert!(names(Filter::All, "terralith").is_empty());
    }

    #[test]
    fn anadir_busca_el_tipo_del_filtro() {
        assert_eq!(Filter::All.add_kind(), "mod");
        assert_eq!(Filter::ResourcePack.add_kind(), "resourcepack");
        assert_eq!(Filter::from_index(3), Filter::Shader);
        assert_eq!(Filter::from_index(9), Filter::All);
        assert_eq!(Filter::Shader.index(), 3);
    }

    #[test]
    fn plurales_de_los_avisos() {
        assert_eq!(plural("es", "contentUpdatedAll", 1), "1 archivo actualizado");
        assert_eq!(plural("es", "contentUpdatedAll", 3), "3 archivos actualizados");
        assert_eq!(plural("en", "contentUpdatedAll", 2), "2 files updated");
    }
}
