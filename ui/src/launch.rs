//! Estado del botón «Jugar» por instancia (`app/lib/core/launch.dart`):
//! Preparando → Descargando x/y → Iniciando → En juego → (inactivo | error).
//! Lógica pura: `backend` le pasa los eventos del núcleo y el resultado de
//! `Core::launch`, y ejecuta los temporizadores que pide.

use std::collections::HashMap;

use lcore::events::Event;

pub use crate::instances::Phase;

/// Etapas de progreso que emite la preparación de un lanzamiento. Las de
/// contenido, modpacks o backups no cuentan.
pub const LAUNCH_STAGES: [&str; 3] = ["libraries", "assets", "java"];

/// Tras este tiempo en «Iniciando» se da el juego por abierto.
pub const STARTING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Motivo de un fallo de lanzamiento o de partida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// El núcleo rechazó el lanzamiento (`Error::kind` y mensaje).
    Launch { kind: String, message: String },
    /// El juego se cerró con un crash report o un código distinto de 0.
    Crash { code: Option<i32>, crash_report: Option<String> },
}

impl Failure {
    pub fn from_error(e: &lcore::Error) -> Self {
        Self::Launch { kind: e.kind().to_owned(), message: e.to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct State {
    pub phase: Phase,
    pub game_id: Option<u64>,
    pub failure: Option<Failure>,
}

impl State {
    fn of(phase: Phase) -> Self {
        Self { phase, ..Default::default() }
    }

    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Preparing | Phase::Downloading { .. } | Phase::Starting)
    }
}

/// Lo que el que llama tiene que hacer tras un cambio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Timer {
    /// Arrancar (o reiniciar) el temporizador de «Iniciando» de la instancia.
    Start(String),
    Cancel(String),
}

/// ¿Esta línea dice que la ventana del juego ya está en pie?
pub fn is_ready_line(line: &str) -> bool {
    const PLAIN: [&str; 4] = ["Sound engine started", "OpenAL initialized", "Backend library: LWJGL", "Reloading ResourceManager"];
    if PLAIN.iter().any(|p| line.contains(p)) {
        return true;
    }
    // `Created: \d+x\d+`
    line.match_indices("Created: ").any(|(i, m)| {
        let rest = &line[i + m.len()..];
        let w = rest.chars().take_while(char::is_ascii_digit).count();
        w > 0 && rest[w..].strip_prefix('x').is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    })
}

#[derive(Debug, Default)]
pub struct Launches {
    states: HashMap<String, State>,
    /// Instancias preparándose, por orden de lanzamiento.
    pending: Vec<String>,
    /// Tarea de progreso → instancia que la reclamó.
    task_owner: HashMap<u64, String>,
    games: HashMap<u64, String>,
}

impl Launches {
    pub fn state(&self, id: &str) -> State {
        self.states.get(id).cloned().unwrap_or_default()
    }

    pub fn phase(&self, id: &str) -> Phase {
        self.states.get(id).map(|s| s.phase).unwrap_or_default()
    }

    /// Ids con algo que mostrar (no inactivos).
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.states.iter().filter(|(_, s)| s.phase != Phase::Idle).map(|(k, _)| k.as_str())
    }

    fn set(&mut self, id: &str, s: State) {
        if s.phase == Phase::Idle {
            self.states.remove(id);
        } else {
            self.states.insert(id.to_owned(), s);
        }
    }

    /// Solo para tests y escenas (`debugSet`).
    pub fn force(&mut self, id: &str, phase: Phase, failure: Option<Failure>) {
        self.set(id, State { phase, failure, game_id: None });
    }

    /// Partida abierta al arrancar la app.
    pub fn restore(&mut self, id: &str, game_id: u64) {
        self.games.insert(game_id, id.to_owned());
        self.set(id, State { phase: Phase::Running, game_id: Some(game_id), failure: None });
    }

    /// Pide lanzar. `false` si ya está en marcha o preparándose.
    pub fn begin(&mut self, id: &str) -> bool {
        let s = self.state(id);
        if s.busy() || s.phase == Phase::Running {
            return false;
        }
        self.pending.push(id.to_owned());
        self.set(id, State::of(Phase::Preparing));
        true
    }

    /// `Core::launch` devolvió la partida.
    pub fn launched(&mut self, id: &str, game_id: u64) -> Vec<Timer> {
        self.finish_pending(id);
        // Los eventos del juego pueden llegar antes que esta respuesta:
        // GameStarted (ya en «Iniciando» o «En juego») o, si cerró muy
        // rápido, GameCrashed/GameExited. Solo se avanza desde la preparación.
        if !matches!(self.phase(id), Phase::Preparing | Phase::Downloading { .. }) {
            return vec![];
        }
        self.games.insert(game_id, id.to_owned());
        self.starting(id, game_id)
    }

    /// `Core::launch` falló.
    pub fn failed(&mut self, id: &str, failure: Failure) {
        self.finish_pending(id);
        self.set(id, State { phase: Phase::Error, failure: Some(failure), game_id: None });
    }

    fn finish_pending(&mut self, id: &str) {
        self.pending.retain(|p| p != id);
        self.task_owner.retain(|_, owner| owner != id);
    }

    /// Vuelve a inactivo (al cerrar el aviso de error).
    pub fn dismiss(&mut self, id: &str) {
        if self.phase(id) == Phase::Error {
            self.set(id, State::default());
        }
    }

    fn starting(&mut self, id: &str, game_id: u64) -> Vec<Timer> {
        self.set(id, State { phase: Phase::Starting, game_id: Some(game_id), failure: None });
        vec![Timer::Start(id.to_owned())]
    }

    /// Venció el temporizador de «Iniciando», o hubo una línea de «listo».
    pub fn running(&mut self, id: &str) -> Vec<Timer> {
        let s = self.state(id);
        if s.phase == Phase::Starting {
            self.set(id, State { phase: Phase::Running, game_id: s.game_id, failure: None });
        }
        vec![Timer::Cancel(id.to_owned())]
    }

    /// Instancia a la que pertenece una tarea de progreso, si es de un
    /// lanzamiento. Una tarea nueva de las etapas de lanzamiento la reclama la
    /// primera instancia en preparación sin otra tarea activa; si todas
    /// tienen una, la más antigua.
    fn owner_of(&mut self, task_id: u64, stage: &str) -> Option<String> {
        if let Some(o) = self.task_owner.get(&task_id) {
            return Some(o.clone());
        }
        if !LAUNCH_STAGES.contains(&stage) {
            return None;
        }
        let candidates: Vec<&String> = self
            .pending
            .iter()
            .filter(|id| matches!(self.phase(id), Phase::Preparing | Phase::Downloading { .. }))
            .collect();
        let first = (*candidates.first()?).clone();
        let owner = candidates
            .iter()
            .find(|id| !self.task_owner.values().any(|o| o == **id))
            .map(|s| (*s).clone())
            .unwrap_or(first);
        self.task_owner.insert(task_id, owner.clone());
        Some(owner)
    }

    /// Un evento del núcleo. Devuelve los temporizadores a tocar.
    pub fn on_event(&mut self, e: &Event) -> Vec<Timer> {
        match e {
            Event::Progress { task_id, stage, done, total, .. } => {
                if let Some(id) = self.owner_of(*task_id, stage)
                    && matches!(self.phase(&id), Phase::Preparing | Phase::Downloading { .. })
                {
                    self.set(&id, State::of(Phase::Downloading { done: *done, total: *total }));
                }
                vec![]
            }
            Event::TaskFinished { task_id, .. } => {
                // Al terminar una descarga se vuelve a «Preparando» hasta la siguiente.
                if let Some(id) = self.task_owner.remove(task_id)
                    && matches!(self.phase(&id), Phase::Downloading { .. })
                {
                    self.set(&id, State::of(Phase::Preparing));
                }
                vec![]
            }
            Event::GameStarted { game_id, instance_id, .. } => {
                self.games.insert(*game_id, instance_id.clone());
                if self.phase(instance_id) != Phase::Running {
                    return self.starting(instance_id, *game_id);
                }
                vec![]
            }
            Event::GameLog { game_id, line, .. } => match self.games.get(game_id).cloned() {
                Some(id) if self.phase(&id) == Phase::Starting && is_ready_line(line) => self.running(&id),
                _ => vec![],
            },
            Event::GameCrashed { game_id, instance_id, crash_report } => {
                self.games.remove(game_id);
                let report = crash_report.as_ref().map(|p| p.to_string_lossy().into_owned());
                self.set(
                    instance_id,
                    State { phase: Phase::Error, failure: Some(Failure::Crash { code: None, crash_report: report }), game_id: None },
                );
                vec![Timer::Cancel(instance_id.clone())]
            }
            Event::GameExited { game_id, instance_id, code, .. } => {
                self.games.remove(game_id);
                if self.phase(instance_id) != Phase::Error {
                    let failed = code.is_some_and(|c| c != 0);
                    let s = if failed {
                        State { phase: Phase::Error, failure: Some(Failure::Crash { code: *code, crash_report: None }), game_id: None }
                    } else {
                        State::default()
                    };
                    self.set(instance_id, s);
                }
                vec![Timer::Cancel(instance_id.clone())]
            }
            Event::AccountsChanged => vec![],
        }
    }

    /// Lanzamiento más relevante (píldora de actividad): el primero ocupado
    /// o, si no hay, uno en juego. `order` = ids en orden «recientes».
    pub fn active<'a>(&self, order: impl IntoIterator<Item = &'a str>) -> Option<(&'a str, Phase)> {
        let mut best = None;
        for id in order {
            let s = self.state(id);
            if s.busy() {
                return Some((id, s.phase));
            }
            if s.phase == Phase::Running && best.is_none() {
                best = Some((id, s.phase));
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(task_id: u64, stage: &str, done: u64, total: u64) -> Event {
        Event::Progress { task_id, stage: stage.into(), done, total, unit: lcore::events::ProgressUnit::Files }
    }

    #[test]
    fn flujo_completo_con_descargas() {
        let mut l = Launches::default();
        assert!(l.begin("a"));
        assert!(!l.begin("a"), "no se lanza dos veces");
        assert_eq!(l.phase("a"), Phase::Preparing);
        l.on_event(&progress(7, "assets", 10, 40));
        assert_eq!(l.phase("a"), Phase::Downloading { done: 10, total: 40 });
        l.on_event(&Event::TaskFinished { task_id: 7, ok: true, message: None });
        assert_eq!(l.phase("a"), Phase::Preparing);
        assert_eq!(l.launched("a", 3), vec![Timer::Start("a".into())]);
        assert_eq!(l.phase("a"), Phase::Starting);
        l.on_event(&Event::GameLog { game_id: 3, stream: lcore::events::LogStream::Stdout, line: "[Render thread/INFO]: Created: 1024x512x4 minecraft:textures/atlas/blocks.png-atlas".into() });
        assert_eq!(l.phase("a"), Phase::Running);
        l.on_event(&Event::GameExited { game_id: 3, instance_id: "a".into(), code: Some(0), seconds_played: 5 });
        assert_eq!(l.phase("a"), Phase::Idle);
    }

    #[test]
    fn etapas_ajenas_al_lanzamiento_no_cuentan() {
        let mut l = Launches::default();
        l.begin("a");
        l.on_event(&progress(1, "modpack", 1, 2));
        assert_eq!(l.phase("a"), Phase::Preparing);
    }

    #[test]
    fn cada_tarea_nueva_va_a_una_instancia_libre() {
        let mut l = Launches::default();
        l.begin("a");
        l.begin("b");
        l.on_event(&progress(1, "libraries", 1, 10));
        l.on_event(&progress(2, "libraries", 2, 20));
        assert_eq!(l.phase("a"), Phase::Downloading { done: 1, total: 10 });
        assert_eq!(l.phase("b"), Phase::Downloading { done: 2, total: 20 });
    }

    #[test]
    fn fallo_al_lanzar_y_cerrar_el_aviso() {
        let mut l = Launches::default();
        l.begin("a");
        l.failed("a", Failure::Launch { kind: "java_required".into(), message: "x".into() });
        assert_eq!(l.phase("a"), Phase::Error);
        assert!(l.begin("a"), "reintentar desde el error");
        l.failed("a", Failure::Launch { kind: "io".into(), message: "x".into() });
        l.dismiss("a");
        assert_eq!(l.phase("a"), Phase::Idle);
    }

    #[test]
    fn salida_con_codigo_es_un_fallo_y_el_crash_no_se_pisa() {
        let mut l = Launches::default();
        l.on_event(&Event::GameStarted { game_id: 9, instance_id: "a".into(), pid: None });
        assert_eq!(l.phase("a"), Phase::Starting);
        l.on_event(&Event::GameCrashed { game_id: 9, instance_id: "a".into(), crash_report: Some("c.txt".into()) });
        l.on_event(&Event::GameExited { game_id: 9, instance_id: "a".into(), code: Some(1), seconds_played: 1 });
        assert_eq!(l.state("a").failure, Some(Failure::Crash { code: None, crash_report: Some("c.txt".into()) }));
        let mut l = Launches::default();
        l.restore("b", 2);
        l.on_event(&Event::GameExited { game_id: 2, instance_id: "b".into(), code: Some(-1), seconds_played: 1 });
        assert_eq!(l.state("b").failure, Some(Failure::Crash { code: Some(-1), crash_report: None }));
    }

    #[test]
    fn el_temporizador_da_el_juego_por_abierto() {
        let mut l = Launches::default();
        l.begin("a");
        l.launched("a", 1);
        l.running("a");
        assert_eq!(l.phase("a"), Phase::Running);
        // GameStarted antes de que vuelva `launch`: no retrocede.
        let mut l = Launches::default();
        l.begin("b");
        l.on_event(&Event::GameStarted { game_id: 4, instance_id: "b".into(), pid: None });
        l.running("b");
        assert_eq!(l.launched("b", 4), vec![]);
        assert_eq!(l.phase("b"), Phase::Running);
    }

    #[test]
    fn un_cierre_que_llega_antes_que_la_respuesta_no_se_pisa() {
        let mut l = Launches::default();
        l.begin("a");
        l.on_event(&Event::GameStarted { game_id: 5, instance_id: "a".into(), pid: None });
        l.on_event(&Event::GameExited { game_id: 5, instance_id: "a".into(), code: Some(1), seconds_played: 0 });
        assert_eq!(l.launched("a", 5), vec![]);
        assert_eq!(l.phase("a"), Phase::Error);
        // Cierre normal antes de la respuesta: queda inactiva.
        l.begin("a");
        l.on_event(&Event::GameExited { game_id: 6, instance_id: "a".into(), code: Some(0), seconds_played: 0 });
        assert_eq!(l.launched("a", 6), vec![]);
        assert_eq!(l.phase("a"), Phase::Idle);
    }

    #[test]
    fn lineas_de_listo() {
        assert!(is_ready_line("[Sound Library Loader/INFO]: Sound engine started"));
        assert!(is_ready_line("Created: 256x256x0 minecraft:textures/atlas/x"));
        assert!(!is_ready_line("Created: axb"));
        assert!(!is_ready_line("Loading 34 mods"));
    }

    #[test]
    fn actividad_prefiere_lo_ocupado() {
        let mut l = Launches::default();
        l.restore("a", 1);
        l.begin("b");
        assert_eq!(l.active(["a", "b"]), Some(("b", Phase::Preparing)));
        let mut l = Launches::default();
        l.restore("a", 1);
        assert_eq!(l.active(["a", "b"]), Some(("a", Phase::Running)));
    }
}
