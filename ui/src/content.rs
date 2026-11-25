//! Contenido (content/*.dart): buscador de Modrinth y CurseForge con filtros
//! y scroll infinito, ficha del proyecto con sus versiones, instalación en
//! una instancia (con destino fijo al entrar desde una instancia), modpacks
//! con progreso y la clave de API de CurseForge.
//!
//! La lógica pura (filtros, búsqueda por generaciones, formatos) va arriba y
//! tiene tests; `Content` corre en el hilo de la UI, copia el estado a
//! `ContentState`/`CurseforgeKey` y atiende sus callbacks. El trabajo con el
//! núcleo lo hace un [`ContentActions`] (`content_backend::CoreContent` en la
//! app, un registro en los tests) que entrega los resultados con [`post`].
//! La clave de CurseForge la lleva `curseforge::Keys`; aquí solo se mira si
//! hay una.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Image, ModelRc, SharedPixelBuffer, VecModel};

use crate::controller::Controller;
use crate::errors::Failure;
use crate::instances::Record;
use crate::{
    ContentState, InstanceItem, LaunchState, MainWindow, ModpackPhase, ProjectItem, Route, SelectOption, SheetVersion,
    art, i18n,
};

// --- Lógica pura ---------------------------------------------------------

/// Tipos de contenido que se pueden buscar (valor del núcleo).
pub const KINDS: [&str; 5] = ["mod", "resourcepack", "shader", "datapack", "modpack"];
/// Órdenes de búsqueda.
pub const SORTS: [&str; 5] = ["relevance", "downloads", "follows", "newest", "updated"];
/// Loaders por los que se puede filtrar.
pub const FILTER_LOADERS: [&str; 4] = ["fabric", "quilt", "forge", "neoforge"];
/// Resultados por página.
pub const PAGE: u32 = 20;
/// Espera tras teclear antes de buscar.
pub const DEBOUNCE: Duration = Duration::from_millis(350);
/// Se pide la página siguiente cuando quedan menos de estos px por debajo.
pub const LOAD_MORE_PX: f32 = 600.0;

/// ¿El loader filtra este tipo? (solo mods y modpacks).
pub fn kind_uses_loader(kind: &str) -> bool {
    kind == "mod" || kind == "modpack"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filters {
    pub source: String,
    pub kind: String,
    pub text: String,
    pub mc: Option<String>,
    pub loader: Option<String>,
    pub sort: String,
}

impl Default for Filters {
    fn default() -> Self {
        Self { source: "modrinth".into(), kind: "mod".into(), text: String::new(), mc: None, loader: None, sort: "relevance".into() }
    }
}

impl Filters {
    /// Loader que se manda al núcleo (solo si el tipo lo usa).
    pub fn query_loader(&self) -> Option<&str> {
        self.loader.as_deref().filter(|_| kind_uses_loader(&self.kind))
    }
}

/// Un proyecto de la búsqueda o de la ficha (`ProjectView`).
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub source: String,
    pub id: String,
    pub slug: String,
    /// `None` = un tipo que no se gestiona (se trata como mod).
    pub kind: Option<String>,
    pub title: String,
    pub description: String,
    pub author: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
}

impl Project {
    pub fn kind(&self) -> &str {
        self.kind.as_deref().unwrap_or("mod")
    }
}

impl From<lcore::content::ProjectHit> for Project {
    fn from(p: lcore::content::ProjectHit) -> Self {
        Self {
            source: p.source.as_str().into(),
            id: p.id,
            slug: p.slug,
            kind: p.kind.map(|k| k.as_str().into()),
            title: p.title,
            description: p.description,
            author: p.author,
            icon_url: p.icon_url,
            downloads: p.downloads,
            follows: p.follows,
            categories: p.categories,
        }
    }
}

/// Una página de resultados.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub items: Vec<Project>,
    pub total: u32,
}

/// Versión de un proyecto (`ContentVersionView`).
#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    pub id: String,
    pub version_number: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    /// `release`, `beta` o `alpha`.
    pub release: String,
    pub date_published: String,
    pub file_url: Option<String>,
    pub file_size: u64,
}

impl From<lcore::content::ContentVersion> for Version {
    fn from(v: lcore::content::ContentVersion) -> Self {
        let file = v.primary_file().cloned();
        Self {
            release: match v.release {
                lcore::content::ReleaseType::Release => "release",
                lcore::content::ReleaseType::Beta => "beta",
                lcore::content::ReleaseType::Alpha => "alpha",
            }
            .into(),
            id: v.id,
            version_number: v.version_number,
            mc_versions: v.mc_versions,
            loaders: v.loaders,
            date_published: v.date_published,
            file_url: file.as_ref().map(|f| f.url.clone()),
            file_size: file.map(|f| f.size).unwrap_or(0),
        }
    }
}

/// Lo instalado en una instancia que importa aquí (proyecto y versión).
#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub project_id: Option<String>,
    pub version_number: Option<String>,
    pub file: String,
}

/// Resultado de una instalación (`InstallOutcomeView` resumido).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outcome {
    /// Dependencias instaladas que no estaban.
    pub new_dependencies: usize,
    pub skipped: usize,
}

/// Búsqueda con filtros y páginas (`ContentSearchController`). Cada búsqueda
/// nueva sube la generación: las respuestas de las anteriores se descartan.
#[derive(Debug, Clone, Default)]
pub struct Search {
    pub filters: Filters,
    pub items: Vec<Project>,
    pub total: u32,
    pub loading: bool,
    pub error: Option<Failure>,
    /// Ya hubo una respuesta con estos filtros.
    pub started: bool,
    generation: u64,
}

impl Search {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn has_more(&self) -> bool {
        (self.items.len() as u32) < self.total
    }

    /// Filtros nuevos: vacía y devuelve la generación a pedir (`None` si no
    /// cambió nada y ya había resultados).
    pub fn set_filters(&mut self, f: Filters) -> Option<u64> {
        if f == self.filters && self.started {
            return None;
        }
        *self = Search { filters: f, generation: self.generation, ..Default::default() };
        Some(self.reload())
    }

    /// Primera búsqueda al abrir la página.
    pub fn ensure_loaded(&mut self) -> Option<u64> {
        (!self.started && !self.loading).then(|| self.reload())
    }

    /// Vuelve a empezar con los mismos filtros.
    pub fn reload(&mut self) -> u64 {
        self.generation += 1;
        self.loading = true;
        self.error = None;
        self.items.clear();
        self.total = 0;
        self.generation
    }

    /// Página siguiente: (generación, desde).
    pub fn load_more(&mut self) -> Option<(u64, u32)> {
        if self.loading || !self.has_more() || self.error.is_some() {
            return None;
        }
        self.loading = true;
        Some((self.generation, self.items.len() as u32))
    }

    /// Llegó una página. `false` si era de una búsqueda vieja.
    pub fn page(&mut self, generation: u64, result: Result<Page, Failure>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        self.started = true;
        match result {
            Ok(p) => {
                let seen: HashSet<String> = self.items.iter().map(|x| x.id.clone()).collect();
                self.items.extend(p.items.into_iter().filter(|x| !seen.contains(&x.id)));
                self.total = p.total;
            }
            Err(f) => self.error = Some(f),
        }
        true
    }
}

/// `NumberFormat.compact` (es/en) para cualquier número.
pub fn compact(lang: &str, n: u64) -> String {
    crate::home::compact(lang, n)
}

const MONTHS_ES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"];
const MONTHS_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// `formatDate` (DateFormat.yMMMd) de una fecha ISO: «7 oct 2026» o
/// «Oct 7, 2026». Solo mira la fecha (como `DateTime.tryParse`, sin pasar a
/// hora local). `None` si no se entiende.
pub fn format_date(lang: &str, iso: &str) -> Option<String> {
    let d = iso.get(..10)?;
    let mut it = d.split('-');
    let (y, m, day): (u32, usize, u32) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    Some(if lang == "es" { format!("{day} {} {y}", MONTHS_ES[m - 1]) } else { format!("{} {day}, {y}", MONTHS_EN[m - 1]) })
}

/// `formatBytes`: «512 B», «12 KB», «1,1 MB», «2 GB».
pub fn format_bytes(lang: &str, bytes: u64) -> String {
    let b = bytes as f64;
    let one = |v: f64| {
        let s = format!("{:.1}", (v * 10.0).round() / 10.0);
        let s = s.strip_suffix(".0").unwrap_or(&s).to_owned();
        if lang == "es" { s.replace('.', ",") } else { s }
    };
    if b < 1024.0 {
        format!("{bytes} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{} KB", (b / 1024.0).round())
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{} MB", one(b / (1024.0 * 1024.0)))
    } else {
        format!("{} GB", one(b / (1024.0 * 1024.0 * 1024.0)))
    }
}

/// `loaderName` (format.dart).
pub fn loader_name(lang: &str, id: &str) -> String {
    match id {
        "vanilla" => i18n::text(lang, "loaderVanilla"),
        "fabric" => "Fabric".into(),
        "quilt" => "Quilt".into(),
        "forge" => "Forge".into(),
        "neoforge" => "NeoForge".into(),
        "labymod" => "LabyMod".into(),
        other => {
            // Loaders de shaders y otros que da Modrinth («iris», «optifine»).
            let mut c = other.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        }
    }
}

/// «1.21.4 · Fabric» (`versionLine`).
pub fn version_line(lang: &str, mc: &str, loader: &str) -> String {
    let name = match loader {
        "vanilla" | "fabric" | "quilt" | "forge" | "neoforge" | "labymod" => loader_name(lang, loader),
        _ => i18n::text(lang, "loaderCustom"),
    };
    format!("{mc} · {name}")
}

/// Línea de datos de una versión: «1.21.4  ·  Fabric  ·  7 oct 2026  ·  1,1 MB».
pub fn version_meta(lang: &str, v: &Version) -> String {
    let mc = if v.mc_versions.len() > 3 {
        format!("{} – {}", v.mc_versions[0], v.mc_versions[v.mc_versions.len() - 1])
    } else {
        v.mc_versions.join(", ")
    };
    let loaders = v.loaders.iter().map(|l| loader_name(lang, l)).collect::<Vec<_>>().join(", ");
    let mut parts = vec![];
    if !mc.is_empty() {
        parts.push(mc);
    }
    if !loaders.is_empty() {
        parts.push(loaders);
    }
    if let Some(d) = format_date(lang, &v.date_published) {
        parts.push(d);
    }
    if v.file_size > 0 {
        parts.push(format_bytes(lang, v.file_size));
    }
    parts.join("  ·  ")
}

/// Mensaje ICU del ARB con argumentos y, si hay, un plural
/// (`=0`, `=1`, `one`, `other`). Para los pocos textos que arma Rust.
pub fn message(lang: &str, key: &str, args: &[(&str, &str)]) -> String {
    let src = i18n::text(lang, key);
    let mut out = String::new();
    let mut rest = src.as_str();
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = matching(rest, open) else { break };
        let inner = &rest[open + 1..close];
        let name = inner.split(',').next().unwrap_or("").trim();
        let value = args.iter().find(|(k, _)| *k == name).map(|(_, v)| *v).unwrap_or("");
        if let Some(cases) = inner.split_once("plural,").map(|(_, c)| c) {
            let n: i64 = value.parse().unwrap_or(0);
            let pick = plural_case(cases, n);
            let filled = message_inline(pick, args, name, value);
            out.push_str(&filled);
        } else {
            out.push_str(value);
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

fn matching(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Texto del caso que toca en `=1{…} other{…}`.
fn plural_case(cases: &str, n: i64) -> &str {
    let mut found: HashMap<&str, &str> = HashMap::new();
    let mut rest = cases;
    while let Some(open) = rest.find('{') {
        let sel = rest[..open].trim();
        let Some(close) = matching(rest, open) else { break };
        found.insert(sel, &rest[open + 1..close]);
        rest = &rest[close + 1..];
    }
    let exact = format!("={n}");
    found
        .get(exact.as_str())
        .or_else(|| if n == 1 { found.get("one") } else { None })
        .or_else(|| found.get("other"))
        .copied()
        .unwrap_or("")
}

fn message_inline(text: &str, args: &[(&str, &str)], var: &str, value: &str) -> String {
    let mut s = text.replace(&format!("{{{var}}}"), value);
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// Texto del aviso tras instalar (`installAndReport`).
pub fn install_toast(lang: &str, project: &str, instance: &str, out: &Outcome) -> (String, bool) {
    if out.skipped > 0 {
        let n = out.skipped.to_string();
        (message(lang, "installPartial", &[("name", project), ("count", &n)]), true)
    } else if out.new_dependencies > 0 {
        let n = out.new_dependencies.to_string();
        (message(lang, "installedWithDeps", &[("name", project), ("instance", instance), ("count", &n)]), false)
    } else {
        (message(lang, "installedIn", &[("name", project), ("instance", instance)]), false)
    }
}

// --- Acciones contra el núcleo -------------------------------------------

/// Versiones que pide la ficha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionsQuery {
    pub source: String,
    pub project: String,
    pub mc: Option<String>,
    pub loader: Option<String>,
}

/// Una instalación pedida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    pub instance_id: String,
    pub source: String,
    pub project: String,
    pub project_title: String,
    /// `None` = la compatible más nueva.
    pub version: Option<String>,
    pub world: Option<String>,
}

/// Modpack instalado.
#[derive(Debug, Clone, PartialEq)]
pub struct ModpackDone {
    pub instance: Record,
    pub files_total: u32,
    pub files_downloaded: u32,
    pub warnings: Vec<String>,
}

/// Imagen decodificada (RGBA).
#[derive(Debug, Clone, PartialEq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Lo que la pantalla pide fuera de la UI. Cada método entrega su resultado
/// llamando a `Content` con [`post`].
pub trait ContentActions {
    /// → [`Content::page`] con la misma generación.
    fn search(&self, filters: &Filters, offset: u32, generation: u64);
    /// → [`Content::versions_loaded`].
    fn versions(&self, query: &VersionsQuery, seq: u64);
    /// → [`Content::install_done`].
    fn install(&self, request: &InstallRequest);
    /// → [`Content::installed_loaded`].
    fn load_installed(&self, instance_id: &str);
    /// Mundos (carpeta, nombre) → [`Content::worlds_loaded`].
    fn load_worlds(&self, instance_id: &str);
    /// Versiones de Minecraft (releases) → [`Content::releases_loaded`].
    fn load_releases(&self);
    /// → [`Content::icon_loaded`].
    fn load_icon(&self, url: &str);
    /// Elegir un `.mrpack`/`.zip` → [`Content::modpack_picked`].
    fn pick_modpack(&self, filter_label: &str);
    /// → [`Content::modpack_done`] con el mismo `attempt`.
    fn install_modpack(&self, source: &str, name: Option<&str>, attempt: u64);
}

thread_local! {
    static CONTENT: RefCell<Option<Rc<Content>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Contenido.
pub fn post(f: impl FnOnce(&Content) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador, ya en el hilo de la UI (si existe).
pub fn with(f: impl FnOnce(&Content)) {
    CONTENT.with(|c| {
        if let Some(x) = c.borrow().as_ref() {
            f(x);
        }
    })
}

// --- Controlador (hilo de la UI) -----------------------------------------

/// Destino de la instalación al entrar desde una instancia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub instance_id: String,
    pub world: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Load<T> {
    Loading,
    Ready(T),
    Failed(Failure),
}

/// Ficha abierta.
#[derive(Debug, Clone)]
struct Sheet {
    project: Project,
    instance: Option<String>,
    world: Option<String>,
    query: Option<VersionsQuery>,
    versions: Load<Vec<Version>>,
    /// Versión instalándose (`Some("")` = la última).
    installing: Option<String>,
}

/// Diálogo del modpack.
#[derive(Debug, Clone, Default)]
struct Modpack {
    open: bool,
    /// Fuente fija (desde la ficha): URL del archivo.
    fixed: Option<String>,
    title_name: String,
    file: Option<String>,
    busy: bool,
    error: Option<Failure>,
    done: Option<ModpackDone>,
    attempt: u64,
}

/// Icono descargado y sus versiones ya escaladas por tamaño físico.
#[derive(Default)]
struct Icon {
    source: Option<Rgba>,
    scaled: HashMap<u32, Image>,
}

pub struct Content {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn ContentActions>,
    search: RefCell<Search>,
    target: RefCell<Option<Target>>,
    /// Lo instalado por instancia (para «Instalado» y «Ya está instalado»).
    installed: RefCell<HashMap<String, Vec<Installed>>>,
    worlds: RefCell<HashMap<String, Vec<(String, String)>>>,
    releases: RefCell<Option<Vec<String>>>,
    releases_asked: Cell<bool>,
    /// `None` = sin saber aún.
    has_key: Cell<Option<bool>>,
    icons: RefCell<HashMap<String, Option<Icon>>>,
    /// Proyectos instalándose desde su tarjeta.
    busy: RefCell<HashSet<String>>,
    sheet: RefCell<Option<Sheet>>,
    sheet_seq: Cell<u64>,
    modpack: RefCell<Modpack>,
    /// Progreso de la etapa `modpack` (hecho, total).
    modpack_progress: Cell<Option<(u64, u64)>>,
    debounce: slint::Timer,
    items: Rc<VecModel<ProjectItem>>,
    versions: Rc<VecModel<SheetVersion>>,
}

fn color(c: art::Rgba) -> slint::Color {
    slint::Color::from_argb_u8((c.a * 255.0).round() as u8, c.r.round() as u8, c.g.round() as u8, c.b.round() as u8)
}

/// Icono de instancia (iniciales y degradado) para la UI.
pub fn instance_badge(r: &Record) -> InstanceItem {
    let (from, to) = art::icon_colors(&r.id);
    InstanceItem {
        id: r.id.as_str().into(),
        name: r.name.as_str().into(),
        mc: r.mc.as_str().into(),
        loader: r.loader.as_str().into(),
        initials: art::initials(&r.name).into(),
        icon_from: color(from),
        icon_to: color(to),
        launch: LaunchState::default(),
        ..Default::default()
    }
}

/// Opción de un desplegable con el icono de la instancia.
pub fn instance_option(lang: &str, r: &Record) -> SelectOption {
    let (from, to) = art::icon_colors(&r.id);
    SelectOption {
        value: r.id.as_str().into(),
        label: r.name.as_str().into(),
        subtitle: version_line(lang, &r.mc, &r.loader).into(),
        lead_initials: art::initials(&r.name).into(),
        lead_from: color(from),
        lead_to: color(to),
        ..Default::default()
    }
}

pub fn plain_option(value: &str, label: &str) -> SelectOption {
    SelectOption { value: value.into(), label: label.into(), ..Default::default() }
}

fn sort_key(s: &str) -> &'static str {
    match s {
        "downloads" => "sortDownloads",
        "follows" => "sortFollows",
        "newest" => "sortNewest",
        "updated" => "sortUpdated",
        _ => "sortRelevance",
    }
}

fn kind_plural_key(k: &str) -> &'static str {
    match k {
        "resourcepack" => "kindResourcepacks",
        "shader" => "kindShaders",
        "datapack" => "kindDatapacks",
        "modpack" => "kindModpacks",
        _ => "kindMods",
    }
}

fn kind_singular_key(k: &str) -> &'static str {
    match k {
        "resourcepack" => "kindResourcepack",
        "shader" => "kindShader",
        "datapack" => "kindDatapack",
        "modpack" => "kindModpack",
        _ => "kindMod",
    }
}

/// Escala un icono al tamaño físico pedido (como Flutter con `cacheWidth` y
/// filtro medio: bilineal desde el original).
fn scale_icon(src: &Rgba, px: u32) -> Option<Image> {
    let img = image::RgbaImage::from_raw(src.width, src.height, src.pixels.clone())?;
    // `BoxFit.cover`: recorta al cuadrado central.
    let side = src.width.min(src.height);
    let cropped = image::imageops::crop_imm(&img, (src.width - side) / 2, (src.height - side) / 2, side, side).to_image();
    let out = if side == px { cropped } else { image::imageops::resize(&cropped, px, px, image::imageops::FilterType::Triangle) };
    Some(Image::from_rgba8(SharedPixelBuffer::clone_from_slice(out.as_raw(), px, px)))
}

/// Decodifica una imagen (PNG, JPEG, WebP) a RGBA.
pub fn decode_icon(bytes: &[u8]) -> Option<Rgba> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(Rgba { width: img.width(), height: img.height(), pixels: img.into_raw() })
}

impl Content {
    /// Crea el controlador y lo registra para [`post`].
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn ContentActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            search: RefCell::default(),
            target: RefCell::default(),
            installed: RefCell::default(),
            worlds: RefCell::default(),
            releases: RefCell::default(),
            releases_asked: Cell::new(false),
            has_key: Cell::new(None),
            icons: RefCell::default(),
            busy: RefCell::default(),
            sheet: RefCell::default(),
            sheet_seq: Cell::new(0),
            modpack: RefCell::default(),
            modpack_progress: Cell::new(None),
            debounce: slint::Timer::default(),
            items: Rc::new(VecModel::default()),
            versions: Rc::new(VecModel::default()),
        });
        CONTENT.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<ContentState>();
        st.set_items(ModelRc::from(me.items.clone()));
        st.set_sheet_versions(ModelRc::from(me.versions.clone()));
        macro_rules! on {
            ($obj:expr, $set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
                let weak = Rc::downgrade(&me);
                $obj.$set(move |$($arg),*| {
                    if let Some($c) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        on!(st, on_shown, |c| c.shown());
        on!(st, on_source_changed, |c, v| c.update_filters(|f| f.source = v.to_string(), false));
        on!(st, on_kind_changed, |c, v| c.update_filters(|f| f.kind = v.to_string(), false));
        on!(st, on_mc_changed, |c, v| c.update_filters(|f| f.mc = Some(v.to_string()).filter(|s| !s.is_empty()), false));
        on!(st, on_loader_changed, |c, v| c.update_filters(|f| f.loader = Some(v.to_string()).filter(|s| !s.is_empty()), false));
        on!(st, on_sort_changed, |c, v| c.update_filters(|f| f.sort = v.to_string(), false));
        on!(st, on_text_edited, |c, v| c.update_filters(|f| f.text = v.to_string(), true));
        on!(st, on_retry, |c| c.retry());
        on!(st, on_load_more, |c| c.load_more());
        on!(st, on_scrolled, |c, after| {
            if after < LOAD_MORE_PX {
                c.load_more()
            }
        });
        on!(st, on_open, |c, id| c.open_sheet(&id));
        on!(st, on_install, |c, id| c.install_from_card(&id));
        on!(st, on_import_modpack, |c| c.open_modpack(None, ""));
        on!(st, on_back_to_instance, |c| c.back_to_instance());
        on!(st, on_clear_target, |c| c.clear_target());
        on!(st, on_add_key, |c| {
            if let Some(ui) = c.ui() {
                ui.global::<crate::CurseforgeKey>().invoke_open_dialog();
            }
        });
        on!(st, on_sheet_close, |c| c.close_sheet());
        on!(st, on_sheet_pick_instance, |c, id| c.sheet_instance(&id));
        on!(st, on_sheet_pick_world, |c, w| c.sheet_world(&w));
        on!(st, on_sheet_install, |c, v| c.sheet_install(&v));
        on!(st, on_modpack_from, |c, url| c.modpack_from(url));
        on!(st, on_modpack_pick, |c| c.actions.pick_modpack(&i18n::text(&c.lang(), "modpackFiles")));
        on!(st, on_modpack_install, |c, url, name| c.modpack_install(&url, &name));
        on!(st, on_modpack_close, |c| c.modpack_close());
        on!(st, on_modpack_open_instance, |c| c.modpack_open_instance());
        me.refresh();
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    fn ui(&self) -> Option<MainWindow> {
        self.ui.upgrade()
    }

    // --- Búsqueda ---

    /// La página se abrió: primera búsqueda y versiones de Minecraft.
    pub fn shown(&self) {
        if !self.releases_asked.replace(true) {
            self.actions.load_releases();
        }
        if let Some(t) = self.target.borrow().clone()
            && !self.installed.borrow().contains_key(&t.instance_id)
        {
            self.actions.load_installed(&t.instance_id);
        }
        let generation = self.search.borrow_mut().ensure_loaded();
        if let Some(g) = generation {
            self.fetch(g, 0);
        }
        self.refresh();
    }

    fn fetch(&self, generation: u64, offset: u32) {
        let f = self.search.borrow().filters.clone();
        if f.source == "curseforge" && self.has_key.get() == Some(false) {
            // Sin clave no se busca: se ve el aviso (la respuesta sería un error).
            self.search.borrow_mut().page(generation, Ok(Page { items: vec![], total: 0 }));
            return;
        }
        self.actions.search(&f, offset, generation);
    }

    /// Cambia los filtros y busca (con espera si es texto).
    fn update_filters(&self, change: impl FnOnce(&mut Filters), debounce: bool) {
        let mut f = self.search.borrow().filters.clone();
        change(&mut f);
        self.set_filters(f, debounce);
    }

    pub fn set_filters(&self, f: Filters, debounce: bool) {
        let generation = self.search.borrow_mut().set_filters(f);
        let Some(g) = generation else { return };
        if let Some(ui) = self.ui() {
            let st = ui.global::<ContentState>();
            st.set_scroll_serial(st.get_scroll_serial() + 1);
        }
        self.debounce.stop();
        if debounce {
            let weak = self.ui.clone();
            self.debounce.start(slint::TimerMode::SingleShot, DEBOUNCE, move || {
                if weak.upgrade().is_some() {
                    with(|c| {
                        if c.search.borrow().generation() == g {
                            c.fetch(g, 0);
                        }
                    });
                }
            });
        } else {
            self.fetch(g, 0);
        }
        self.refresh();
    }

    pub fn filters(&self) -> Filters {
        self.search.borrow().filters.clone()
    }

    pub fn search_state(&self) -> Search {
        self.search.borrow().clone()
    }

    fn retry(&self) {
        let g = self.search.borrow_mut().reload();
        self.fetch(g, 0);
        self.refresh();
    }

    fn load_more(&self) {
        let next = self.search.borrow_mut().load_more();
        if let Some((g, offset)) = next {
            self.fetch(g, offset);
            self.refresh();
        }
    }

    /// Respuesta de [`ContentActions::search`].
    pub fn page(&self, generation: u64, result: Result<Page, Failure>) {
        if self.search.borrow_mut().page(generation, result) {
            self.request_icons();
            self.refresh();
        }
    }

    pub fn releases_loaded(&self, list: Vec<String>) {
        *self.releases.borrow_mut() = Some(list);
        self.refresh();
    }

    // --- Destino (desde una instancia) ---

    /// Entra a Contenido para añadir a una instancia (`/content?instance=…
    /// &kind=…&world=…`). Ajusta tipo, versión y loader a la instancia (si
    /// ya se conoce).
    pub fn open_for(&self, instance_id: &str, kind: Option<&str>, world: Option<&str>) {
        *self.target.borrow_mut() = Some(Target { instance_id: instance_id.into(), world: world.map(Into::into) });
        let record = self.shell.model.borrow().by_id(instance_id).cloned();
        let mut f = self.filters();
        if let Some(k) = kind.filter(|k| KINDS.contains(k)) {
            f.kind = k.into();
        }
        if let Some(r) = &record {
            f.mc = Some(r.mc.clone());
            f.loader = FILTER_LOADERS.contains(&r.loader.as_str()).then(|| r.loader.clone());
        }
        self.actions.load_installed(instance_id);
        if kind.is_some() || record.is_some() {
            self.set_filters(f, false);
        }
        if let Some(ui) = self.ui() {
            ui.set_route(Route::Content);
            ui.invoke_route_changed(Route::Content);
        }
        self.refresh();
    }

    pub fn target(&self) -> Option<Target> {
        self.target.borrow().clone()
    }

    fn clear_target(&self) {
        *self.target.borrow_mut() = None;
        self.refresh();
    }

    fn back_to_instance(&self) {
        let id = self.target.borrow().as_ref().map(|t| t.instance_id.clone());
        if let Some(id) = id {
            self.shell.actions().open_instance(&id);
        }
    }

    pub fn installed_loaded(&self, instance_id: &str, list: Vec<Installed>) {
        self.installed.borrow_mut().insert(instance_id.into(), list);
        self.refresh();
    }

    pub fn worlds_loaded(&self, instance_id: &str, list: Vec<(String, String)>) {
        self.worlds.borrow_mut().insert(instance_id.into(), list);
        self.refresh();
    }

    fn installed_ids(&self, instance: &str) -> HashSet<String> {
        self.installed.borrow().get(instance).into_iter().flatten().filter_map(|x| x.project_id.clone()).collect()
    }

    // --- Instalar ---

    fn project(&self, id: &str) -> Option<Project> {
        let s = self.search.borrow();
        s.items.iter().find(|p| p.id == id).cloned()
    }

    /// «Instalar» de una tarjeta: con destino instala ya la compatible más
    /// nueva; sin él (o modpack, o datapack sin mundo) abre la ficha.
    fn install_from_card(&self, id: &str) {
        let Some(p) = self.project(id) else { return };
        let t = self.target.borrow().clone();
        let kind = p.kind().to_owned();
        match t {
            Some(t) if kind != "modpack" && !(kind == "datapack" && t.world.is_none()) => {
                self.busy.borrow_mut().insert(p.id.clone());
                self.actions.install(&InstallRequest {
                    instance_id: t.instance_id,
                    source: p.source.clone(),
                    project: p.id.clone(),
                    project_title: p.title.clone(),
                    version: None,
                    world: t.world,
                });
                self.refresh();
            }
            _ => self.open_sheet(id),
        }
    }

    /// Resultado de [`ContentActions::install`].
    pub fn install_done(&self, req: &InstallRequest, result: Result<Outcome, Failure>) {
        self.busy.borrow_mut().remove(&req.project);
        let lang = self.lang();
        let ok = result.is_ok();
        match result {
            Ok(out) => {
                self.actions.load_installed(&req.instance_id);
                let name = self.shell.model.borrow().by_id(&req.instance_id).map(|r| r.name.clone()).unwrap_or(req.instance_id.clone());
                let (text, error) = install_toast(&lang, &req.project_title, &name, &out);
                self.shell.toast(&text, error);
            }
            Err(f) => self.shell.toast(&f.message, true),
        }
        let close = {
            let mut sheet = self.sheet.borrow_mut();
            match sheet.as_mut() {
                Some(s) if s.project.id == req.project && s.installing.is_some() => {
                    s.installing = None;
                    ok
                }
                _ => false,
            }
        };
        if close {
            *self.sheet.borrow_mut() = None;
        }
        self.refresh();
    }

    // --- Ficha ---

    pub fn open_sheet(&self, id: &str) {
        let Some(p) = self.project(id) else { return };
        let t = self.target.borrow().clone();
        // Sin destino, la instancia de Inicio (`homeInstanceProvider`).
        let instance = t.as_ref().map(|t| t.instance_id.clone()).or_else(|| self.shell.home_instance_id());
        *self.sheet.borrow_mut() =
            Some(Sheet { project: p, instance, world: t.and_then(|t| t.world), query: None, versions: Load::Loading, installing: None });
        self.sheet_changed();
    }

    /// Instancia o mundo de la ficha cambiaron: lo instalado, los mundos y
    /// las versiones que tocan.
    fn sheet_changed(&self) {
        let Some((project, instance)) = self.sheet.borrow().as_ref().map(|s| (s.project.clone(), s.instance.clone())) else { return };
        let kind = project.kind().to_owned();
        let pack = kind == "modpack";
        let record = instance.as_deref().and_then(|id| self.shell.model.borrow().by_id(id).cloned());
        if let Some(id) = &instance {
            if !pack && !self.installed.borrow().contains_key(id) {
                self.actions.load_installed(id);
            }
            if kind == "datapack" && !self.worlds.borrow().contains_key(id) {
                self.actions.load_worlds(id);
            }
        }
        let f = self.filters();
        let query = if pack {
            Some(VersionsQuery { source: project.source.clone(), project: project.id.clone(), mc: f.mc.clone(), loader: f.loader.clone() })
        } else {
            record.as_ref().map(|r| VersionsQuery {
                source: project.source.clone(),
                project: project.id.clone(),
                mc: Some(r.mc.clone()),
                loader: kind_uses_loader(&kind).then(|| r.loader.clone()),
            })
        };
        // Solo una consulta nueva invalida la que esté en camino: con los
        // mismos filtros (otra instancia igual) la respuesta sigue valiendo.
        let mut ask = None;
        if let Some(s) = self.sheet.borrow_mut().as_mut() {
            if s.query != query || matches!(s.versions, Load::Failed(_)) {
                self.sheet_seq.set(self.sheet_seq.get() + 1);
                s.versions = if query.is_some() { Load::Loading } else { Load::Ready(vec![]) };
                ask = query.clone();
            }
            s.query = query;
        }
        if let Some(q) = ask {
            self.actions.versions(&q, self.sheet_seq.get());
        }
        self.request_icons();
        self.refresh();
    }

    pub fn versions_loaded(&self, seq: u64, result: Result<Vec<Version>, Failure>) {
        if seq != self.sheet_seq.get() {
            return;
        }
        if let Some(s) = self.sheet.borrow_mut().as_mut() {
            s.versions = match result {
                Ok(v) => Load::Ready(v),
                Err(f) => Load::Failed(f),
            };
        }
        self.refresh();
    }

    fn close_sheet(&self) {
        *self.sheet.borrow_mut() = None;
        self.refresh();
    }

    fn sheet_instance(&self, id: &str) {
        if let Some(s) = self.sheet.borrow_mut().as_mut() {
            s.instance = Some(id.into());
            s.world = None;
        }
        self.sheet_changed();
    }

    fn sheet_world(&self, world: &str) {
        if let Some(s) = self.sheet.borrow_mut().as_mut() {
            s.world = Some(world.into());
        }
        self.refresh();
    }

    /// «Instalar» de una versión (`""` = la última) o, en un modpack, el
    /// diálogo del modpack con el archivo de esa versión.
    fn sheet_install(&self, version: &str) {
        let Some(s) = self.sheet.borrow().clone() else { return };
        let kind = s.project.kind().to_owned();
        if kind == "modpack" {
            let url = match &s.versions {
                Load::Ready(list) => list.iter().find(|v| v.id == version).and_then(|v| v.file_url.clone()),
                _ => None,
            };
            if let Some(url) = url {
                self.open_modpack(Some(url), &s.project.title);
            }
            return;
        }
        let Some(instance) = s.instance.clone() else { return };
        if s.installing.is_some() || (kind == "datapack" && s.world.is_none()) {
            return;
        }
        if let Some(x) = self.sheet.borrow_mut().as_mut() {
            x.installing = Some(version.into());
        }
        self.actions.install(&InstallRequest {
            instance_id: instance,
            source: s.project.source.clone(),
            project: s.project.id.clone(),
            project_title: s.project.title.clone(),
            version: Some(version.to_owned()).filter(|v| !v.is_empty()),
            world: if kind == "datapack" { s.world.clone() } else { None },
        });
        self.refresh();
    }

    /// Número de la última petición de versiones de la ficha.
    pub fn versions_seq(&self) -> u64 {
        self.sheet_seq.get()
    }

    pub fn sheet_open(&self) -> bool {
        self.sheet.borrow().is_some()
    }

    // --- Modpacks ---

    /// Diálogo del modpack: con `fixed` (URL de la ficha) instala esa; sin
    /// ella deja elegir archivo o URL.
    pub fn open_modpack(&self, fixed: Option<String>, name: &str) {
        let attempt = self.modpack.borrow().attempt + 1;
        *self.modpack.borrow_mut() = Modpack { open: true, fixed, title_name: name.into(), attempt, ..Default::default() };
        self.modpack_progress.set(None);
        if let Some(ui) = self.ui() {
            let st = ui.global::<ContentState>();
            st.set_modpack_from_url(false);
            st.set_modpack_url("".into());
            st.set_modpack_name(name.into());
        }
        self.refresh();
    }

    fn modpack_from(&self, url: bool) {
        if let Some(ui) = self.ui() {
            ui.global::<ContentState>().set_modpack_from_url(url);
        }
        self.refresh();
    }

    pub fn modpack_picked(&self, path: Option<String>) {
        if let Some(p) = path {
            self.modpack.borrow_mut().file = Some(p);
            self.refresh();
        }
    }

    fn modpack_install(&self, url: &str, name: &str) {
        let source = {
            let m = self.modpack.borrow();
            if m.busy {
                return;
            }
            let from_url = self.ui().map(|u| u.global::<ContentState>().get_modpack_from_url()).unwrap_or(false);
            match &m.fixed {
                Some(s) => Some(s.clone()),
                None if from_url => Some(url.trim().to_owned()).filter(|u| !u.is_empty()),
                None => m.file.clone(),
            }
        };
        let Some(source) = source else { return };
        let attempt = {
            let mut m = self.modpack.borrow_mut();
            m.busy = true;
            m.error = None;
            m.attempt
        };
        self.modpack_progress.set(None);
        let name = name.trim();
        self.actions.install_modpack(&source, Some(name).filter(|n| !n.is_empty()), attempt);
        self.refresh();
    }

    pub fn modpack_done(&self, attempt: u64, result: Result<ModpackDone, Failure>) {
        {
            let mut m = self.modpack.borrow_mut();
            if m.attempt != attempt || !m.open {
                return;
            }
            m.busy = false;
            match result {
                Ok(d) => m.done = Some(d),
                Err(f) => m.error = Some(f),
            }
        }
        self.refresh();
    }

    fn modpack_close(&self) {
        let fixed = {
            let mut m = self.modpack.borrow_mut();
            if m.busy {
                return;
            }
            m.open = false;
            m.fixed.is_some()
        };
        // Al cerrarse el del modpack se cierra también la ficha.
        if fixed {
            *self.sheet.borrow_mut() = None;
        }
        self.refresh();
    }

    fn modpack_open_instance(&self) {
        let id = self.modpack.borrow().done.as_ref().map(|d| d.instance.id.clone());
        self.modpack_close();
        if let Some(id) = id {
            self.shell.actions().open_instance(&id);
        }
    }

    /// Progreso de las tareas del núcleo (etapa `modpack`).
    pub fn on_event(&self, e: &lcore::events::Event) {
        if let lcore::events::Event::Progress { stage, done, total, .. } = e
            && stage == "modpack"
            && self.modpack.borrow().busy
        {
            self.modpack_progress.set(Some((*done, *total)));
            self.refresh();
        }
    }

    /// Fija el progreso (escenas; `tasksProvider.debugSet`).
    pub fn set_modpack_progress(&self, done: u64, total: u64) {
        self.modpack_progress.set(Some((done, total)));
        self.refresh();
    }

    // --- Clave de CurseForge ---

    /// Se supo si hay clave (`curseforge::Keys::on_change`). Con CurseForge
    /// elegido, la búsqueda vuelve a empezar.
    pub fn key_state(&self, has: bool) {
        let was = self.has_key.replace(Some(has));
        let retry = was != Some(has) && self.search.borrow().filters.source == "curseforge" && self.search.borrow().started;
        if retry {
            self.retry();
        }
        self.refresh();
    }

    // --- Iconos ---

    fn request_icons(&self) {
        let mut urls: Vec<String> = self.search.borrow().items.iter().filter_map(|p| p.icon_url.clone()).collect();
        if let Some(s) = self.sheet.borrow().as_ref()
            && let Some(u) = &s.project.icon_url
        {
            urls.push(u.clone());
        }
        for url in urls {
            if url.is_empty() || self.icons.borrow().contains_key(&url) {
                continue;
            }
            self.icons.borrow_mut().insert(url.clone(), Some(Icon::default()));
            self.actions.load_icon(&url);
        }
    }

    /// Resultado de [`ContentActions::load_icon`] (`None` = sin icono).
    pub fn icon_loaded(&self, url: &str, icon: Option<Rgba>) {
        self.icons.borrow_mut().insert(url.into(), icon.map(|i| Icon { source: Some(i), scaled: HashMap::new() }));
        self.refresh();
    }

    /// Icono a `size` lógicos (escalado a la escala de la ventana).
    fn icon(&self, url: Option<&str>, size: f32) -> Option<Image> {
        let url = url?;
        let scale = self.ui().map(|u| u.window().scale_factor()).unwrap_or(1.0);
        let px = (size * scale).round() as u32;
        let mut icons = self.icons.borrow_mut();
        let icon = icons.get_mut(url)?.as_mut()?;
        if let Some(img) = icon.scaled.get(&px) {
            return Some(img.clone());
        }
        let img = scale_icon(icon.source.as_ref()?, px)?;
        icon.scaled.insert(px, img.clone());
        Some(img)
    }

    fn item(&self, lang: &str, p: &Project, installed: &HashSet<String>) -> ProjectItem {
        let (from, to) = art::icon_colors(&p.id);
        let icon = self.icon(p.icon_url.as_deref(), 64.0);
        ProjectItem {
            id: p.id.as_str().into(),
            title: p.title.as_str().into(),
            author: p.author.as_deref().map(|a| message(lang, "byAuthor", &[("author", a)])).unwrap_or_default().into(),
            description: p.description.as_str().into(),
            downloads: compact(lang, p.downloads).into(),
            follows: compact(lang, p.follows).into(),
            cat1: p.categories.first().cloned().unwrap_or_default().into(),
            cat2: p.categories.get(1).cloned().unwrap_or_default().into(),
            has_icon: icon.is_some(),
            icon: icon.unwrap_or_default(),
            initials: art::initials(&p.title).into(),
            icon_from: color(from),
            icon_to: color(to),
            installed: installed.contains(&p.id),
            busy: self.busy.borrow().contains(&p.id),
            pack: p.kind() == "modpack",
        }
    }

    /// Copia el estado a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui() else { return };
        let lang = self.lang();
        let t = |k: &str| i18n::text(&lang, k);
        let st = ui.global::<ContentState>();
        let s = self.search.borrow();
        let f = &s.filters;
        st.set_source(f.source.as_str().into());
        st.set_kind(f.kind.as_str().into());
        st.set_mc(f.mc.clone().unwrap_or_default().into());
        st.set_loader(f.loader.clone().unwrap_or_default().into());
        st.set_sort(f.sort.as_str().into());
        st.set_uses_loader(kind_uses_loader(&f.kind));
        st.set_search_hint(message(&lang, "contentSearchHint", &[("kind", &t(kind_plural_key(&f.kind)).to_lowercase())]).into());
        // Versión: «Cualquiera», la del filtro si no está en la lista y las releases.
        let releases = self.releases.borrow().clone().unwrap_or_default();
        let mut mc_opts = vec![plain_option("", &t("filterAny"))];
        if let Some(m) = &f.mc
            && !releases.contains(m)
        {
            mc_opts.push(plain_option(m, m));
        }
        mc_opts.extend(releases.iter().map(|v| plain_option(v, v)));
        st.set_mc_options(ModelRc::from(Rc::new(VecModel::from(mc_opts))));
        let mut lo_opts = vec![plain_option("", &t("filterAny"))];
        lo_opts.extend(FILTER_LOADERS.iter().map(|l| plain_option(l, &loader_name(&lang, l))));
        st.set_loader_options(ModelRc::from(Rc::new(VecModel::from(lo_opts))));
        let sort_opts: Vec<SelectOption> = SORTS.iter().map(|x| plain_option(x, &t(sort_key(x)))).collect();
        st.set_sort_options(ModelRc::from(Rc::new(VecModel::from(sort_opts))));

        let needs_key = f.source == "curseforge" && self.has_key.get() != Some(true);
        st.set_needs_key(needs_key);
        st.set_started(s.started);
        st.set_loading(s.loading);
        st.set_has_error(s.error.is_some());
        if let Some(e) = &s.error {
            st.set_error_message(e.message.as_str().into());
            st.set_error_detail(e.detail.as_str().into());
        }
        st.set_total(s.total.min(i32::MAX as u32) as i32);
        st.set_has_more(s.has_more());

        // Destino.
        let target = self.target.borrow().clone();
        let record = target.as_ref().and_then(|t| self.shell.model.borrow().by_id(&t.instance_id).cloned());
        st.set_has_target(record.is_some());
        if let (Some(r), Some(tg)) = (&record, &target) {
            st.set_target(instance_badge(r));
            st.set_target_line(format!("  ·  {}", version_line(&lang, &r.mc, &r.loader)).into());
            st.set_target_world(
                tg.world.as_deref().map(|w| format!("  ·  {}", message(&lang, "contentTargetWorld", &[("world", w)]))).unwrap_or_default().into(),
            );
        }
        let installed = target.as_ref().map(|t| self.installed_ids(&t.instance_id)).unwrap_or_default();
        let items: Vec<ProjectItem> = s.items.iter().map(|p| self.item(&lang, p, &installed)).collect();
        drop(s);
        sync(&self.items, items);

        self.refresh_sheet(&ui, &lang);
        self.refresh_modpack(&ui, &lang);
    }

    fn refresh_sheet(&self, ui: &MainWindow, lang: &str) {
        let st = ui.global::<ContentState>();
        let sheet = self.sheet.borrow();
        st.set_sheet_open(sheet.is_some());
        let Some(s) = sheet.as_ref() else {
            sync(&self.versions, vec![]);
            return;
        };
        let t = |k: &str| i18n::text(lang, k);
        let p = &s.project;
        let kind = p.kind();
        let pack = kind == "modpack";
        let icon = self.icon(p.icon_url.as_deref(), 76.0);
        st.set_sheet_project(self.item(lang, p, &HashSet::new()));
        st.set_sheet_has_icon(icon.is_some());
        st.set_sheet_icon(icon.unwrap_or_default());
        st.set_sheet_kind(kind.into());
        st.set_sheet_kind_label(t(kind_singular_key(kind)).into());
        st.set_sheet_source_label(if p.source == "curseforge" { "CurseForge" } else { "Modrinth" }.into());
        st.set_sheet_categories(ModelRc::from(Rc::new(VecModel::from(
            p.categories.iter().map(|c| slint::SharedString::from(c.as_str())).collect::<Vec<_>>(),
        ))));
        st.set_sheet_pack(pack);
        let m = self.shell.model.borrow();
        let mut opts = vec![];
        if s.instance.is_none() {
            opts.push(plain_option("", &t("chooseInstance")));
        }
        opts.extend(m.all.iter().map(|r| instance_option(lang, r)));
        st.set_sheet_instance_options(ModelRc::from(Rc::new(VecModel::from(opts))));
        st.set_sheet_instance(s.instance.clone().unwrap_or_default().into());
        let has_instance = s.instance.as_deref().is_some_and(|id| m.by_id(id).is_some());
        drop(m);
        let worlds = s.instance.as_deref().and_then(|id| self.worlds.borrow().get(id).cloned()).unwrap_or_default();
        st.set_sheet_has_worlds(!worlds.is_empty());
        let mut wopts = vec![];
        if s.world.is_none() {
            wopts.push(plain_option("", &t("chooseWorld")));
        }
        wopts.extend(worlds.iter().map(|(folder, name)| plain_option(folder, name)));
        st.set_sheet_world_options(ModelRc::from(Rc::new(VecModel::from(wopts))));
        st.set_sheet_world(s.world.clone().unwrap_or_default().into());
        let already = if pack {
            None
        } else {
            s.instance.as_deref().and_then(|id| {
                self.installed.borrow().get(id).and_then(|l| l.iter().find(|x| x.project_id.as_deref() == Some(p.id.as_str())).cloned())
            })
        };
        st.set_sheet_installed(
            already
                .map(|x| message(lang, "contentAlreadyInstalled", &[("version", x.version_number.as_deref().unwrap_or(&x.file))]))
                .unwrap_or_default()
                .into(),
        );
        let can = |installing: &Option<String>| {
            installing.is_none() && (pack || (has_instance && (kind != "datapack" || s.world.is_some())))
        };
        let (state, versions) = match &s.versions {
            Load::Loading => (crate::LoadState::Loading, vec![]),
            Load::Failed(f) => {
                st.set_sheet_error_message(f.message.as_str().into());
                st.set_sheet_error_detail(f.detail.as_str().into());
                (crate::LoadState::Failed, vec![])
            }
            Load::Ready(v) => (crate::LoadState::Ready, v.clone()),
        };
        st.set_sheet_state(state);
        st.set_sheet_count(if matches!(s.versions, Load::Ready(_)) { versions.len() as i32 } else { -1 });
        st.set_sheet_empty_text(if !has_instance && !pack { t("chooseInstanceFirst") } else { t("versionsNoneCompatible") }.into());
        st.set_sheet_can_latest(!pack && s.installing.is_none() && has_instance && (kind != "datapack" || s.world.is_some()) && !versions.is_empty());
        st.set_sheet_latest_busy(s.installing.as_deref() == Some(""));
        let rows: Vec<SheetVersion> = versions
            .iter()
            .map(|v| SheetVersion {
                id: v.id.as_str().into(),
                number: v.version_number.as_str().into(),
                release: v.release.as_str().into(),
                meta: version_meta(lang, v).into(),
                busy: s.installing.as_deref() == Some(v.id.as_str()),
                enabled: can(&s.installing),
            })
            .collect();
        sync(&self.versions, rows);
    }

    fn refresh_modpack(&self, ui: &MainWindow, lang: &str) {
        let st = ui.global::<ContentState>();
        let m = self.modpack.borrow();
        st.set_modpack_open(m.open);
        if !m.open {
            return;
        }
        st.set_modpack_fixed(m.fixed.is_some());
        st.set_modpack_title(
            if m.fixed.is_some() { message(lang, "modpackInstallTitle", &[("name", &m.title_name)]) } else { i18n::text(lang, "modpackImportTitle") }
                .into(),
        );
        st.set_modpack_file(m.file.clone().unwrap_or_default().into());
        st.set_modpack_phase(if m.done.is_some() {
            ModpackPhase::Done
        } else if m.busy {
            ModpackPhase::Installing
        } else {
            ModpackPhase::Form
        });
        let (done, total) = self.modpack_progress.get().unwrap_or((0, 0));
        st.set_modpack_done_count(done.min(i32::MAX as u64) as i32);
        st.set_modpack_total(total.min(i32::MAX as u64) as i32);
        st.set_modpack_progress(if total > 0 { (done as f32 / total as f32).min(1.0) } else { -1.0 });
        st.set_modpack_has_error(m.error.is_some());
        if let Some(e) = &m.error {
            st.set_modpack_error_message(e.message.as_str().into());
            st.set_modpack_error_detail(e.detail.as_str().into());
        }
        if let Some(d) = &m.done {
            st.set_modpack_instance(instance_badge(&d.instance));
            st.set_modpack_instance_line(version_line(lang, &d.instance.mc, &d.instance.loader).into());
            st.set_modpack_done_body(
                message(lang, "modpackDoneBody", &[("done", &d.files_downloaded.to_string()), ("total", &d.files_total.to_string())]).into(),
            );
            let warnings: Vec<slint::SharedString> = d.warnings.iter().take(4).map(|w| format!("• {w}").into()).collect();
            st.set_modpack_warnings(ModelRc::from(Rc::new(VecModel::from(warnings))));
        }
    }
}

/// Copia `new` en `model` tocando solo las filas que cambian.
fn sync<T: Clone + PartialEq + 'static>(model: &VecModel<T>, new: Vec<T>) {
    use slint::Model as _;
    if model.row_count() != new.len() {
        model.set_vec(new);
        return;
    }
    for (i, row) in new.into_iter().enumerate() {
        if model.row_data(i).as_ref() != Some(&row) {
            model.set_row_data(i, row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str) -> Project {
        Project {
            source: "modrinth".into(),
            id: id.into(),
            slug: id.into(),
            kind: Some("mod".into()),
            title: id.into(),
            description: String::new(),
            author: None,
            icon_url: None,
            downloads: 0,
            follows: 0,
            categories: vec![],
        }
    }

    fn fail() -> Failure {
        Failure { kind: "network".into(), message: "sin red".into(), detail: "network: x".into() }
    }

    #[test]
    fn busqueda_por_generaciones_y_paginas() {
        let mut s = Search::default();
        let g1 = s.ensure_loaded().unwrap();
        assert!(s.loading);
        assert_eq!(s.ensure_loaded(), None, "no repite si ya está cargando");
        // Cambia el filtro antes de que llegue: la respuesta vieja se ignora.
        let g2 = s.set_filters(Filters { kind: "shader".into(), ..Default::default() }).unwrap();
        assert!(!s.page(g1, Ok(Page { items: vec![p("viejo")], total: 1 })));
        assert!(s.items.is_empty());
        assert!(s.page(g2, Ok(Page { items: vec![p("a"), p("b")], total: 3 })));
        assert!(s.has_more());
        // Siguiente página, sin repetir ids.
        let (g, offset) = s.load_more().unwrap();
        assert_eq!((g, offset), (g2, 2));
        assert_eq!(s.load_more(), None, "una a la vez");
        s.page(g, Ok(Page { items: vec![p("b"), p("c")], total: 3 }));
        assert_eq!(s.items.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        assert!(!s.has_more());
        assert_eq!(s.load_more(), None);
        // Mismos filtros: no busca otra vez.
        assert_eq!(s.set_filters(s.filters.clone()), None);
    }

    #[test]
    fn error_y_reintento() {
        let mut s = Search::default();
        let g = s.ensure_loaded().unwrap();
        s.page(g, Err(fail()));
        assert!(s.started && !s.loading && s.error.is_some());
        assert_eq!(s.load_more(), None, "con error no pide más");
        let g2 = s.reload();
        assert!(g2 > g && s.error.is_none() && s.loading);
    }

    #[test]
    fn loader_solo_en_mods_y_modpacks() {
        let f = Filters { loader: Some("fabric".into()), ..Default::default() };
        assert_eq!(f.query_loader(), Some("fabric"));
        let f = Filters { kind: "shader".into(), ..f };
        assert_eq!(f.query_loader(), None);
        assert!(kind_uses_loader("modpack"));
    }

    #[test]
    fn fechas_y_tamanos_como_intl() {
        assert_eq!(format_date("es", "2026-10-07T00:00:00.000").as_deref(), Some("7 oct 2026"));
        assert_eq!(format_date("es", "2026-09-15T00:00:00Z").as_deref(), Some("15 sept 2026"));
        assert_eq!(format_date("en", "2026-08-31").as_deref(), Some("Aug 31, 2026"));
        assert_eq!(format_date("es", "mañana"), None);
        assert_eq!(format_bytes("es", 1_200_000), "1,1 MB");
        assert_eq!(format_bytes("es", 1_080_000), "1 MB");
        assert_eq!(format_bytes("en", 1_200_000), "1.1 MB");
        assert_eq!(format_bytes("es", 500), "500 B");
        assert_eq!(format_bytes("es", 12_400), "12 KB");
        assert_eq!(compact("es", 98_400_000), "98,4\u{a0}M");
        assert_eq!(compact("es", 1_093_333), "1,09\u{a0}M");
        assert_eq!(compact("es", 568_888), "569\u{a0}mil");
    }

    #[test]
    fn linea_de_una_version() {
        let v = Version {
            id: "x".into(),
            version_number: "0.6.14".into(),
            mc_versions: vec!["1.21.4".into()],
            loaders: vec!["fabric".into()],
            release: "release".into(),
            date_published: "2026-10-07T00:00:00.000".into(),
            file_url: None,
            file_size: 1_200_000,
        };
        assert_eq!(version_meta("es", &v), "1.21.4  ·  Fabric  ·  7 oct 2026  ·  1,1 MB");
        let many = Version { mc_versions: vec!["1.20".into(), "1.20.1".into(), "1.20.2".into(), "1.21".into()], file_size: 0, ..v };
        assert_eq!(version_meta("en", &many), "1.20 – 1.21  ·  Fabric  ·  Oct 7, 2026");
    }

    #[test]
    fn textos_con_plural() {
        let out = Outcome { new_dependencies: 2, skipped: 0 };
        assert_eq!(
            install_toast("es", "Sodium", "Survival", &out),
            ("Sodium instalado en Survival con 2 dependencias".into(), false)
        );
        let one = Outcome { new_dependencies: 1, skipped: 0 };
        assert_eq!(install_toast("en", "Sodium", "Survival", &one).0, "Sodium installed in Survival with 1 dependency");
        assert_eq!(install_toast("es", "Sodium", "Survival", &Outcome::default()).0, "Sodium instalado en Survival");
        let (text, error) = install_toast("es", "Sodium", "Survival", &Outcome { new_dependencies: 0, skipped: 1 });
        assert!(error);
        assert_eq!(text, "Sodium instalado, pero se omitieron 1 dependencia");
        assert_eq!(message("es", "contentResults", &[("count", "6")]), "6 resultados");
        assert_eq!(message("es", "contentResults", &[("count", "1")]), "1 resultado");
        assert_eq!(message("es", "serversCount", &[("count", "4"), ("online", "3")]), "4 servidores · 3 en línea");
    }
}
