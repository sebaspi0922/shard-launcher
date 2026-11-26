//! Pestaña Registros: consola en vivo de la partida de una instancia
//! (console_view.dart, core/logs.dart).
//!
//! `backend::pump_events` agrupa las líneas `GameLog` con [`Batcher`] (50 ms
//! o 200 líneas, como el bridge) y las entrega a [`Console`] con [`post`].
//! `Console` guarda un [`Buffer`] acotado por instancia (su partida más
//! reciente desde que la app arrancó, como el concentrador del bridge) y
//! enseña el de la instancia abierta a través de [`LogModel`], un modelo de
//! Slint que construye cada fila cuando la lista la pide: la lista solo
//! instancia las que se ven, así 5000 líneas cuestan lo mismo que 20.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};

use lcore::events::LogStream;
use slint::{ComponentHandle, Model, ModelNotify, ModelRc, ModelTracker, VecModel};

use crate::instance::{DetailEvent, InstanceDetail};
use crate::{ConsoleState, InstanceTab, LogGroup, LogRow, MainWindow};

/// Líneas por instancia, como el anillo del núcleo (`kLogCapacity`).
pub const CAPACITY: usize = lcore::logs::DEFAULT_LOG_CAPACITY;
/// Cada cuánto cruzan las líneas al hilo de la UI (`LOG_FLUSH` del bridge).
pub const FLUSH: Duration = Duration::from_millis(50);
/// Tope de líneas de un envío (`LOG_BATCH` del bridge).
pub const BATCH: usize = 200;

/// Nivel de una línea (`LogLevel` de core/logs.dart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Other,
}

/// Grupo de niveles que se oculta desde la barra (`_LevelGroup`), en el
/// orden de los chips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl Group {
    pub fn from_index(i: i32) -> Option<Group> {
        [Group::Error, Group::Warn, Group::Info, Group::Debug].get(usize::try_from(i).ok()?).copied()
    }
}

impl Level {
    pub fn group(self) -> Group {
        match self {
            Level::Error => Group::Error,
            Level::Warn => Group::Warn,
            Level::Debug | Level::Trace => Group::Debug,
            Level::Info | Level::Other => Group::Info,
        }
    }
}

/// En el orden de la alternativa de las expresiones de Dart.
const LEVELS: [&str; 8] = ["TRACE", "DEBUG", "INFO", "WARN", "WARNING", "ERROR", "FATAL", "SEVERE"];

fn level_name(s: &str) -> Level {
    match s {
        "TRACE" => Level::Trace,
        "DEBUG" => Level::Debug,
        "INFO" => Level::Info,
        "WARN" | "WARNING" => Level::Warn,
        "ERROR" | "FATAL" | "SEVERE" => Level::Error,
        _ => Level::Other,
    }
}

/// `[hilo/NIVEL]` o `[NIVEL]` al principio de `s`: el nombre del nivel.
fn bracket(s: &str) -> Option<&str> {
    let inner = s.strip_prefix('[')?;
    let inner = &inner[..inner.find(']')?];
    let name = inner.split_once('/').map_or(inner, |(_, n)| n);
    LEVELS.contains(&name).then_some(name)
}

/// `_levelRe`: `^\s*(?:\[[^\]]*\]\s*)?\[(?:[^\]/]*/)?(NIVEL)\]` (el
/// corchete opcional se prueba primero, como la expresión).
fn leading_level(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let after_first = s.strip_prefix('[').and_then(|r| r.find(']').map(|i| r[i + 1..].trim_start()));
    after_first.and_then(bracket).or_else(|| bracket(s))
}

/// `_bareLevelRe`: `\b(NIVEL)\]?:` en cualquier sitio (el primero).
fn bare_level(line: &str) -> Option<&str> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    for (i, _) in line.char_indices() {
        if line[..i].chars().next_back().is_some_and(word) {
            continue;
        }
        let rest = &line[i..];
        for name in LEVELS {
            if let Some(after) = rest.strip_prefix(name) {
                let after = after.strip_prefix(']').unwrap_or(after);
                if after.starts_with(':') {
                    return Some(name);
                }
            }
        }
    }
    None
}

/// `_continuationRe`: líneas de un stack trace (`\tat …`, `Caused by: …`,
/// `... 12 more`).
fn continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    let indented = trimmed.len() < line.len();
    line.starts_with('\t')
        || (indented && trimmed.starts_with("at "))
        || trimmed.starts_with("Caused by: ")
        || trimmed.strip_prefix("... ").is_some_and(|r| {
            let digits = r.len() - r.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            digits > 0 && r[digits..].starts_with(" more")
        })
}

/// Nivel de una línea (`parseLogLevel`). Las continuaciones heredan el de
/// la anterior.
pub fn parse_level(line: &str, previous: Option<Level>, stderr: bool) -> Level {
    if let Some(name) = leading_level(line).or_else(|| bare_level(line)) {
        return level_name(name);
    }
    if let Some(p) = previous
        && continuation(line)
    {
        return p;
    }
    if line.contains("Exception") || line.contains("Error:") {
        return Level::Error;
    }
    if stderr { Level::Warn } else { Level::Other }
}

/// Una línea tal como sale del juego.
#[derive(Clone, Debug, PartialEq)]
pub struct RawLine {
    pub text: String,
    pub stderr: bool,
}

impl RawLine {
    pub fn new(text: impl Into<String>, stderr: bool) -> Self {
        Self { text: text.into(), stderr }
    }
}

/// Líneas pendientes de una partida.
pub type Batch = (u64, Vec<RawLine>);

/// Agrupa las líneas antes de cruzar al hilo de la UI (`subscribe_events`
/// del bridge): una partida sale entera al llegar a [`BATCH`] líneas, y
/// todas a los [`FLUSH`] de la primera pendiente o antes de otro evento.
#[derive(Default)]
pub struct Batcher {
    pending: Vec<Batch>,
    since: Option<Instant>,
}

impl Batcher {
    /// Apunta una línea; devuelve el lote de su partida si ya está lleno.
    pub fn push(&mut self, game_id: u64, stream: LogStream, text: String, now: Instant) -> Option<Batch> {
        self.since.get_or_insert(now);
        let i = match self.pending.iter().position(|(g, _)| *g == game_id) {
            Some(i) => i,
            None => {
                self.pending.push((game_id, Vec::new()));
                self.pending.len() - 1
            }
        };
        self.pending[i].1.push(RawLine { text, stderr: stream == LogStream::Stderr });
        if self.pending[i].1.len() < BATCH {
            return None;
        }
        let batch = self.pending.remove(i);
        if self.pending.is_empty() {
            self.since = None;
        }
        Some(batch)
    }

    /// Cuándo hay que vaciar (`None` = no hay nada pendiente).
    pub fn deadline(&self) -> Option<Instant> {
        self.since.map(|t| t + FLUSH)
    }

    /// Todo lo pendiente, en el orden en que empezó cada partida.
    pub fn take(&mut self) -> Vec<Batch> {
        self.since = None;
        std::mem::take(&mut self.pending)
    }
}

/// Una línea con su nivel.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub text: String,
    pub level: Level,
}

/// Últimas líneas de una instancia (`LogBuffer` de core/logs.dart), con la
/// cuenta por grupo al día.
#[derive(Clone, Debug)]
pub struct Buffer {
    lines: VecDeque<Entry>,
    counts: [usize; 4],
    capacity: usize,
    /// Partida de la que son las líneas.
    pub game: Option<u64>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new(CAPACITY)
    }
}

impl Buffer {
    pub fn new(capacity: usize) -> Self {
        Self { lines: VecDeque::new(), counts: [0; 4], capacity: capacity.max(1), game: None }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn get(&self, i: usize) -> Option<&Entry> {
        self.lines.get(i)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.lines.iter()
    }

    /// Líneas por grupo (Errores, Avisos, Info, Depuración).
    pub fn counts(&self) -> [usize; 4] {
        self.counts
    }

    /// Añade las líneas; devuelve cuántas viejas se descartaron.
    pub fn push(&mut self, lines: impl IntoIterator<Item = RawLine>) -> usize {
        let mut prev = self.lines.back().map(|e| e.level);
        let mut dropped = 0;
        for l in lines {
            let level = parse_level(&l.text, prev, l.stderr);
            prev = Some(level);
            self.counts[level.group() as usize] += 1;
            self.lines.push_back(Entry { text: l.text, level });
            if self.lines.len() > self.capacity
                && let Some(old) = self.lines.pop_front()
            {
                self.counts[old.level.group() as usize] -= 1;
                dropped += 1;
            }
        }
        dropped
    }

    /// Vacía la vista sin olvidar la partida.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.counts = [0; 4];
    }

    /// Partida nueva: líneas vacías.
    pub fn reset(&mut self, game: u64) {
        self.clear();
        self.game = Some(game);
    }
}

/// Filtro de la barra: texto (sin mayúsculas) y grupos ocultos.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    query: String,
    pub hidden: [bool; 4],
}

impl Filter {
    pub fn set_query(&mut self, q: &str) {
        self.query = q.trim().to_lowercase();
    }

    pub fn toggle(&mut self, g: Group) {
        self.hidden[g as usize] = !self.hidden[g as usize];
    }

    pub fn is_empty(&self) -> bool {
        self.query.is_empty() && !self.hidden.contains(&true)
    }

    pub fn matches(&self, e: &Entry) -> bool {
        !self.hidden[e.level.group() as usize] && (self.query.is_empty() || e.text.to_lowercase().contains(&self.query))
    }
}

/// Lo que se ve: índices de `buffer` que pasan el filtro.
fn visible(buffer: &Buffer, filter: &Filter) -> Vec<usize> {
    if filter.is_empty() {
        return (0..buffer.len()).collect();
    }
    buffer.iter().enumerate().filter(|(_, e)| filter.matches(e)).map(|(i, _)| i).collect()
}

/// Texto que copia «Copiar»: las líneas visibles, una por renglón, y
/// cuántas son.
pub fn copy_text(buffer: &Buffer, filter: &Filter) -> (String, usize) {
    let rows = visible(buffer, filter);
    let text = rows.iter().filter_map(|&i| buffer.get(i)).map(|e| e.text.as_str()).collect::<Vec<_>>().join("\n");
    (text, rows.len())
}

/// Todas las consolas: búfer por instancia, a qué instancia va cada
/// partida, cuál se ve, con qué filtro y qué filas pasan.
#[derive(Default)]
pub struct Hub {
    buffers: HashMap<String, Buffer>,
    games: HashMap<u64, String>,
    shown: Option<String>,
    filter: Filter,
    rows: Vec<usize>,
}

/// Cómo cambió lo que se ve.
#[derive(Debug, PartialEq)]
enum Change {
    None,
    Added(usize, usize),
    Reset,
}

impl Hub {
    pub fn buffer(&self, id: &str) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    fn shown_buffer(&self) -> Option<&Buffer> {
        self.shown.as_deref().and_then(|id| self.buffers.get(id))
    }

    /// `GameStarted` (o partida abierta al arrancar): la partida nueva
    /// sustituye a la anterior de la instancia.
    fn started(&mut self, game_id: u64, instance_id: &str) -> Change {
        self.games.insert(game_id, instance_id.to_owned());
        let b = self.buffers.entry(instance_id.to_owned()).or_default();
        if b.game == Some(game_id) {
            return Change::None;
        }
        b.reset(game_id);
        self.refilter_if_shown(instance_id)
    }

    fn lines(&mut self, game_id: u64, lines: Vec<RawLine>) -> Change {
        let Some(id) = self.games.get(&game_id).cloned() else { return Change::None };
        let b = self.buffers.entry(id.clone()).or_default();
        // Restos de una partida anterior de la instancia: ya no se ven.
        if b.game != Some(game_id) {
            return Change::None;
        }
        let before = b.len();
        if b.push(lines) > 0 {
            return self.refilter_if_shown(&id);
        }
        if self.shown.as_deref() != Some(id.as_str()) {
            return Change::None;
        }
        let b = &self.buffers[&id];
        let old = self.rows.len();
        self.rows.extend((before..b.len()).filter(|&i| self.filter.matches(&b.lines[i])));
        if self.rows.len() > old { Change::Added(old, self.rows.len() - old) } else { Change::None }
    }

    fn clear_shown(&mut self) -> Change {
        let Some(id) = self.shown.clone() else { return Change::None };
        if let Some(b) = self.buffers.get_mut(&id) {
            b.clear();
        }
        self.refilter()
    }

    fn show(&mut self, id: Option<&str>) -> Change {
        self.shown = id.map(str::to_owned);
        self.filter = Filter::default();
        self.refilter()
    }

    fn refilter(&mut self) -> Change {
        self.rows = self.shown_buffer().map(|b| visible(b, &self.filter)).unwrap_or_default();
        Change::Reset
    }

    fn refilter_if_shown(&mut self, id: &str) -> Change {
        if self.shown.as_deref() == Some(id) { self.refilter() } else { Change::None }
    }

    /// Fila `i` de lo que se ve (número = posición en lo que se ve, como
    /// el `ListView` de Flutter).
    fn row(&self, i: usize) -> Option<LogRow> {
        let e = self.shown_buffer()?.get(*self.rows.get(i)?)?;
        let level = match e.level.group() {
            Group::Error => LogGroup::Error,
            Group::Warn => LogGroup::Warn,
            Group::Info => LogGroup::Info,
            Group::Debug => LogGroup::Debug,
        };
        Some(LogRow { number: i as i32 + 1, text: e.text.as_str().into(), level })
    }
}

/// Modelo de Slint sobre las filas visibles del [`Hub`]: no copia nada,
/// cada fila se construye cuando la lista la pide.
pub struct LogModel {
    hub: Rc<RefCell<Hub>>,
    notify: ModelNotify,
}

impl Model for LogModel {
    type Data = LogRow;

    fn row_count(&self) -> usize {
        self.hub.borrow().rows.len()
    }

    fn row_data(&self, row: usize) -> Option<LogRow> {
        self.hub.borrow().row(row)
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }
}

thread_local! {
    static CONSOLE: RefCell<Option<Rc<Console>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con la consola.
pub fn post(f: impl FnOnce(&Console) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        CONSOLE.with(|c| {
            if let Some(k) = c.borrow().as_ref() {
                f(k);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Controlador de la pestaña Registros (hilo de la UI). No pide nada al
/// núcleo: las líneas le llegan del bucle de eventos de `backend`.
pub struct Console {
    ui: slint::Weak<MainWindow>,
    hub: Rc<RefCell<Hub>>,
    model: Rc<LogModel>,
}

impl Console {
    /// Crea la consola, la registra para [`post`] y se entera por el detalle
    /// de qué instancia se ve.
    pub fn new(ui: &MainWindow, detail: &InstanceDetail) -> Rc<Self> {
        let hub = Rc::new(RefCell::new(Hub::default()));
        let model = Rc::new(LogModel { hub: hub.clone(), notify: ModelNotify::default() });
        let me = Rc::new(Self { ui: ui.as_weak(), hub, model });
        CONSOLE.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<ConsoleState>();
        st.set_rows(ModelRc::from(me.model.clone()));
        st.set_counts(ModelRc::new(VecModel::from(vec![0; 4])));
        st.set_hidden(ModelRc::new(VecModel::from(vec![false; 4])));
        let weak = Rc::downgrade(&me);
        st.on_filter(move |q| {
            if let Some(k) = weak.upgrade() {
                k.set_query(&q);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_toggle(move |g| {
            if let Some(k) = weak.upgrade()
                && let Some(g) = Group::from_index(g)
            {
                k.toggle(g);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_clear(move || {
            if let Some(k) = weak.upgrade() {
                k.clear();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_copy_text(move || weak.upgrade().map(|k| k.copy_text().0.into()).unwrap_or_default());
        let ctl = Rc::downgrade(detail.controller());
        st.on_copied(move |msg| {
            if let Some(c) = ctl.upgrade() {
                c.toast(&msg, false);
            }
        });
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            let Some(k) = weak.upgrade() else { return };
            match e {
                // Cada vez que se abre la pestaña, la vista empieza de cero
                // (filtro, niveles y seguir al final), como el widget nuevo
                // de Flutter.
                DetailEvent::Shown { id, tab: InstanceTab::Logs } => k.show(Some(id)),
                DetailEvent::Shown { .. } | DetailEvent::Closed => k.show(None),
                _ => {}
            }
        });
        me
    }

    /// Partida nueva de `instance_id` (o abierta al arrancar la app).
    pub fn started(&self, game_id: u64, instance_id: &str) {
        let c = self.hub.borrow_mut().started(game_id, instance_id);
        self.apply(c);
    }

    /// Un lote de líneas de `game_id`.
    pub fn lines(&self, game_id: u64, lines: Vec<RawLine>) {
        let c = self.hub.borrow_mut().lines(game_id, lines);
        self.apply(c);
    }

    pub fn set_query(&self, q: &str) {
        let c = {
            let mut h = self.hub.borrow_mut();
            h.filter.set_query(q);
            h.refilter()
        };
        self.apply(c);
    }

    pub fn toggle(&self, g: Group) {
        let c = {
            let mut h = self.hub.borrow_mut();
            h.filter.toggle(g);
            h.refilter()
        };
        self.apply(c);
    }

    /// «Limpiar»: vacía la consola de la instancia que se ve.
    pub fn clear(&self) {
        let c = self.hub.borrow_mut().clear_shown();
        self.apply(c);
    }

    /// Texto que se copia y cuántas líneas lleva.
    pub fn copy_text(&self) -> (String, usize) {
        let h = self.hub.borrow();
        match h.shown_buffer() {
            Some(b) => copy_text(b, &h.filter),
            None => (String::new(), 0),
        }
    }

    /// Búfer de una instancia.
    pub fn buffer(&self, id: &str) -> Option<Buffer> {
        self.hub.borrow().buffer(id).cloned()
    }

    fn show(&self, id: Option<&str>) {
        let c = self.hub.borrow_mut().show(id);
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<ConsoleState>();
            st.set_query("".into());
            st.set_follow(true);
        }
        self.apply(c);
    }

    /// Avisa a la lista y copia contadores y filtro a `ConsoleState`.
    fn apply(&self, change: Change) {
        match change {
            Change::None => return,
            Change::Added(i, n) => self.model.notify.row_added(i, n),
            Change::Reset => self.model.notify.reset(),
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<ConsoleState>();
        let h = self.hub.borrow();
        let (total, counts) = h.shown_buffer().map_or((0, [0; 4]), |b| (b.len(), b.counts()));
        st.set_total(total as i32);
        st.set_shown(h.rows.len() as i32);
        let model = st.get_counts();
        for (i, n) in counts.iter().enumerate() {
            if model.row_data(i) != Some(*n as i32) {
                model.set_row_data(i, *n as i32);
            }
        }
        let hidden = st.get_hidden();
        for (i, v) in h.filter.hidden.iter().enumerate() {
            if hidden.row_data(i) != Some(*v) {
                hidden.set_row_data(i, *v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lvl(s: &str) -> Level {
        parse_level(s, None, false)
    }

    #[test]
    fn niveles_como_flutter() {
        assert_eq!(lvl("[18:02:11] [main/INFO]: Loading"), Level::Info);
        assert_eq!(lvl("[18:02:12] [main/WARN]: Mod resolution"), Level::Warn);
        assert_eq!(lvl("[18:02:16] [Render thread/DEBUG]: Created"), Level::Debug);
        assert_eq!(lvl("[main/ERROR] algo"), Level::Error);
        assert_eq!(lvl("  [FATAL] x"), Level::Error);
        assert_eq!(lvl("[a/b/INFO] x"), Level::Other, "dos barras no casan con _levelRe");
        assert_eq!(lvl("[12:34:56 WARN]: x"), Level::Warn, "por _bareLevelRe (`WARN]:`)");
        assert_eq!(lvl("2026-10-11 12:34:56 WARNING: lento"), Level::Warn);
        assert_eq!(lvl("Loader INFO: x"), Level::Info);
        assert_eq!(lvl("XINFO: x"), Level::Other, "\\b antes del nivel");
        assert_eq!(lvl("[Thread] [SEVERE] mal"), Level::Error);
        assert_eq!(lvl("[INFO] [main/WARN] x"), Level::Warn, "el primer corchete opcional se salta primero");
        assert_eq!(lvl("java.io.FileNotFoundException: x"), Level::Error);
        assert_eq!(lvl("algo Error: y"), Level::Error);
        assert_eq!(lvl("salida suelta"), Level::Other);
        assert_eq!(parse_level("salida suelta", None, true), Level::Warn);
    }

    #[test]
    fn las_continuaciones_heredan() {
        let e = Some(Level::Error);
        assert_eq!(parse_level("\tat net.minecraft.Foo(Foo.java:1)", e, true), Level::Error);
        assert_eq!(parse_level("    at x", e, false), Level::Error);
        assert_eq!(parse_level("Caused by: java.lang.NullPointerException", Some(Level::Info), false), Level::Info);
        assert_eq!(parse_level("\t... 12 more", e, false), Level::Error);
        assert_eq!(parse_level("... 12 more", Some(Level::Info), false), Level::Info);
        assert_eq!(parse_level("at x", Some(Level::Info), false), Level::Other, "sin sangría no es continuación");
        assert_eq!(parse_level("\t- sodium", Some(Level::Info), false), Level::Info);
        assert_eq!(parse_level("\t- sodium", None, false), Level::Other);
    }

    #[test]
    fn el_registro_de_ejemplo_cuenta_como_el_golden() {
        let mut b = Buffer::default();
        b.push(crate::fixtures::logs::sample_log());
        assert_eq!(b.len(), 21);
        // Errores 4, Avisos 2, Info 14, Depuración 1 (instance_logs_es.png).
        assert_eq!(b.counts(), [4, 2, 14, 1]);
    }

    #[test]
    fn el_bufer_se_acota_y_lleva_la_cuenta() {
        let mut b = Buffer::new(3);
        let d = b.push(["[a/ERROR] 1", "[a/INFO] 2", "[a/INFO] 3", "[a/WARN] 4"].map(|t| RawLine::new(t, false)));
        assert_eq!(d, 1);
        assert_eq!(b.len(), 3);
        assert_eq!(b.get(0).unwrap().text, "[a/INFO] 2");
        assert_eq!(b.counts(), [0, 1, 2, 0]);
        // La continuación hereda de la última línea, aunque llegue en otro lote.
        b.push([RawLine::new("\tat x", false)]);
        assert_eq!(b.get(2).unwrap().level, Level::Warn);
        b.clear();
        assert!(b.is_empty());
        assert_eq!(b.counts(), [0; 4]);
    }

    #[test]
    fn filtro_por_texto_y_nivel() {
        let mut b = Buffer::default();
        b.push(crate::fixtures::logs::sample_log());
        let mut f = Filter::default();
        assert_eq!(visible(&b, &f).len(), 21);
        f.set_query("  RENDER thread ");
        assert_eq!(visible(&b, &f).len(), 7);
        f.toggle(Group::Info);
        assert_eq!(visible(&b, &f), vec![8], "solo la de depuración del hilo de render");
        f.set_query("");
        f.toggle(Group::Info);
        f.toggle(Group::Error);
        f.toggle(Group::Warn);
        assert_eq!(visible(&b, &f).len(), 15);
        let mut f = Filter::default();
        f.set_query("cape");
        let (text, n) = copy_text(&b, &f);
        assert_eq!(n, 2);
        assert_eq!(
            text,
            "[18:02:19] [Worker-Main-4/ERROR]: Failed to load texture minecraft:textures/entity/cape.png\n\
             java.io.FileNotFoundException: minecraft:textures/entity/cape.png"
        );
    }

    #[test]
    fn el_hub_reparte_por_partida_y_avisa_solo_de_lo_que_se_ve() {
        let mut h = Hub::default();
        let l = |t: &str| vec![RawLine::new(t, false)];
        assert_eq!(h.lines(9, l("sin partida")), Change::None, "partida desconocida: se ignora");
        h.started(1, "a");
        h.started(2, "b");
        assert_eq!(h.show(Some("a")), Change::Reset);
        assert_eq!(h.lines(2, l("[x/INFO] de b")), Change::None);
        assert_eq!(h.lines(1, l("[x/INFO] de a")), Change::Added(0, 1));
        assert_eq!(h.lines(1, vec![RawLine::new("[x/INFO] 2", false), RawLine::new("[x/INFO] 3", false)]), Change::Added(1, 2));
        h.filter.set_query("3");
        h.refilter();
        assert_eq!(h.lines(1, l("[x/INFO] 4")), Change::None, "no pasa el filtro");
        assert_eq!(h.lines(1, l("[x/INFO] 33")), Change::Added(1, 1));
        assert_eq!(h.row(1).unwrap().number, 2, "número = posición en lo que se ve");
        // La misma partida otra vez no borra nada; una nueva, sí.
        assert_eq!(h.started(1, "a"), Change::None);
        assert_eq!(h.buffer("a").unwrap().len(), 5);
        assert_eq!(h.started(3, "a"), Change::Reset);
        assert!(h.buffer("a").unwrap().is_empty());
        assert_eq!(h.lines(1, l("[x/INFO] tarde")), Change::None, "línea tardía de la partida vieja");
        assert!(h.buffer("a").unwrap().is_empty());
        assert_eq!(h.buffer("b").unwrap().len(), 1);
    }

    #[test]
    fn el_hub_rehace_la_vista_al_descartar_lineas_viejas() {
        let mut h = Hub::default();
        h.started(1, "a");
        h.show(Some("a"));
        let many: Vec<RawLine> = (0..CAPACITY + 10).map(|i| RawLine::new(format!("[x/INFO] {i}"), false)).collect();
        assert_eq!(h.lines(1, many), Change::Reset);
        assert_eq!(h.rows.len(), CAPACITY);
        assert_eq!(h.row(0).unwrap().text, "[x/INFO] 10");
    }

    #[test]
    fn lotes_de_200_o_a_los_50_ms() {
        let t0 = Instant::now();
        let mut b = Batcher::default();
        assert_eq!(b.deadline(), None);
        assert!(b.push(1, LogStream::Stdout, "a".into(), t0).is_none());
        assert!(b.push(2, LogStream::Stderr, "b".into(), t0 + Duration::from_millis(10)).is_none());
        assert_eq!(b.deadline(), Some(t0 + FLUSH), "cuenta desde la primera pendiente");
        for i in 1..BATCH - 1 {
            assert!(b.push(1, LogStream::Stdout, format!("{i}"), t0).is_none());
        }
        let (g, lines) = b.push(1, LogStream::Stdout, "última".into(), t0).expect("lote lleno");
        assert_eq!((g, lines.len()), (1, BATCH));
        assert_eq!(lines[0], RawLine::new("a", false));
        assert!(b.deadline().is_some(), "queda la línea de la partida 2");
        assert_eq!(b.take(), vec![(2, vec![RawLine::new("b", true)])]);
        assert_eq!(b.deadline(), None);
        assert!(b.take().is_empty());
    }
}
