//! Análisis de la salida de consola del juego: niveles, hilos, continuaciones
//! (stack traces), ruta del crash report y un anillo de líneas recientes.
//! Comportamiento documentado en `research/06-lanzamiento.md` §6.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

/// Capacidad por defecto del anillo de líneas de una instancia.
pub const DEFAULT_LOG_CAPACITY: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    /// Interpreta el nombre de nivel de log4j/java.util.logging.
    pub fn parse(s: &str) -> LogLevel {
        match s.to_ascii_uppercase().as_str() {
            "TRACE" | "FINEST" | "FINER" => LogLevel::Trace,
            "DEBUG" | "FINE" | "CONFIG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" | "SEVERE" => LogLevel::Error,
            "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Unknown,
        }
    }
}

pub use crate::events::LogStream;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogLine {
    /// Número de orden dentro del `LogBuffer` (0 hasta que se inserta).
    pub seq: u64,
    /// Instante en que se leyó la línea (ms desde la época Unix).
    pub timestamp_ms: i64,
    /// `HH:MM:SS` tal como aparece en la línea.
    pub time: Option<String>,
    pub level: LogLevel,
    pub thread: Option<String>,
    pub logger: Option<String>,
    pub message: String,
    /// Línea original sin secuencias ANSI ni salto final.
    pub raw: String,
    /// Línea sin cabecera (stack trace, salida de un mod…).
    pub continuation: bool,
    pub stream: LogStream,
}

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pat).expect("expresión regular válida"));
    };
}

re!(
    ANSI,
    r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[@-Z\\-_]"
);
// `[12:34:56] [Render thread/INFO] [logger/]: msg`, `[main/WARN] (Minecraft) msg`,
// y la variante de Forge con fecha: `[12Oct2026 12:34:56.789]`.
re!(
    BRACKET_THREAD_LEVEL,
    r"^\[(?P<time>(?:\d{1,2}[A-Za-z]{3}\d{4} )?\d{2}:\d{2}:\d{2}(?:[.,]\d+)?)\] \[(?P<thread>[^\]]*?)/(?P<level>[A-Za-z]+)\](?: \[(?P<logger>[^\]]*)\])?(?: \((?P<logger2>[^)]*)\))?:? ?(?P<msg>.*)$"
);
// `[12:34:56 INFO]: msg` (servidor vanilla y launchers antiguos).
re!(
    BRACKET_TIME_LEVEL,
    r"^\[(?P<time>\d{2}:\d{2}:\d{2}) (?P<level>[A-Za-z]+)\]:? ?(?P<msg>.*)$"
);
// `2026-10-11 12:34:56,123 [INFO] msg` / `2026-10-11T12:34:56 WARN msg`.
re!(
    ISO_LEVEL,
    r"^\d{4}-\d{2}-\d{2}[ T](?P<time>\d{2}:\d{2}:\d{2})(?:[.,]\d+)?\s+\[?(?P<level>TRACE|DEBUG|INFO|WARN|WARNING|ERROR|SEVERE|FATAL)\]?:?\s*(?P<msg>.*)$"
);
re!(TIME_ONLY, r"\d{2}:\d{2}:\d{2}");
re!(
    CRASH_PATH,
    r"(?i)crash report (?:has been )?saved to:\s*(?:#@!@#\s*)?(?P<path>.+?)\s*$"
);

/// Quita secuencias de escape ANSI (colores, cursor).
pub fn strip_ansi(s: &str) -> String {
    ANSI.replace_all(s, "").into_owned()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn non_empty(s: Option<regex::Match<'_>>) -> Option<String> {
    s.map(|m| m.as_str().trim().trim_end_matches('/').trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Parsea una línea de consola. `prev_level` es el nivel de la línea anterior:
/// las líneas sin cabecera lo heredan.
pub fn parse_line(raw: &str, stream: LogStream, prev_level: LogLevel) -> LogLine {
    let raw = strip_ansi(raw.trim_end_matches(['\n', '\r']));
    let mut line = LogLine {
        seq: 0,
        timestamp_ms: now_ms(),
        time: None,
        level: prev_level,
        thread: None,
        logger: None,
        message: raw.clone(),
        raw: raw.clone(),
        continuation: true,
        stream,
    };
    let time_of = |m: Option<regex::Match<'_>>| {
        m.and_then(|m| TIME_ONLY.find(m.as_str()))
            .map(|t| t.as_str().to_owned())
    };

    if let Some(c) = BRACKET_THREAD_LEVEL.captures(&raw) {
        line.time = time_of(c.name("time"));
        line.thread = non_empty(c.name("thread"));
        line.level = LogLevel::parse(&c["level"]);
        line.logger = non_empty(c.name("logger")).or_else(|| non_empty(c.name("logger2")));
        line.message = c["msg"].to_owned();
        line.continuation = false;
    } else if let Some(c) = BRACKET_TIME_LEVEL.captures(&raw) {
        line.time = time_of(c.name("time"));
        line.level = LogLevel::parse(&c["level"]);
        line.message = c["msg"].to_owned();
        line.continuation = false;
    } else if let Some(c) = ISO_LEVEL.captures(&raw) {
        line.time = time_of(c.name("time"));
        line.level = LogLevel::parse(&c["level"]);
        line.message = c["msg"].to_owned();
        line.continuation = false;
    }
    line
}

/// Ruta del crash report si la línea anuncia que se guardó uno.
pub fn crash_report_path(line: &str) -> Option<PathBuf> {
    let line = strip_ansi(line);
    let c = CRASH_PATH.captures(&line)?;
    let path = c["path"].trim().trim_matches(['"', '\'']).trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Anillo de las últimas líneas de una instancia.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    capacity: usize,
    next_seq: u64,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            lines: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            next_seq: 0,
        }
    }

    /// Asigna el siguiente `seq`, descarta la línea más antigua si está lleno
    /// y devuelve la línea guardada.
    pub fn push(&mut self, mut line: LogLine) -> &LogLine {
        line.seq = self.next_seq;
        self.next_seq += 1;
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
        self.lines.back().expect("acaba de insertarse")
    }

    /// Copia de las líneas retenidas, de la más antigua a la más reciente.
    pub fn snapshot(&self) -> Vec<LogLine> {
        self.lines.iter().cloned().collect()
    }

    /// Nivel de la última línea (para heredarlo en continuaciones).
    pub fn last_level(&self) -> LogLevel {
        self.lines.back().map_or(LogLevel::Unknown, |l| l.level)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_LOG_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(raw: &str) -> LogLine {
        parse_line(raw, LogStream::Stdout, LogLevel::Unknown)
    }

    #[test]
    fn vanilla_con_hilo_y_nivel() {
        let l = p("[12:34:56] [Render thread/INFO]: Reloading ResourceManager: vanilla");
        assert_eq!(l.time.as_deref(), Some("12:34:56"));
        assert_eq!(l.thread.as_deref(), Some("Render thread"));
        assert_eq!(l.level, LogLevel::Info);
        assert_eq!(l.logger, None);
        assert_eq!(l.message, "Reloading ResourceManager: vanilla");
        assert!(!l.continuation);
    }

    #[test]
    fn fabric_con_logger_entre_parentesis() {
        let l = p("[12:34:56] [main/WARN] (Minecraft) Failed to parse option 'x'");
        assert_eq!(l.level, LogLevel::Warn);
        assert_eq!(l.thread.as_deref(), Some("main"));
        assert_eq!(l.logger.as_deref(), Some("Minecraft"));
        assert_eq!(l.message, "Failed to parse option 'x'");
    }

    #[test]
    fn fabric_con_logger_y_dos_puntos() {
        let l = p("[12:34:56] [Worker-Main-3/ERROR] (sodium) Boom");
        assert_eq!(l.level, LogLevel::Error);
        assert_eq!(l.thread.as_deref(), Some("Worker-Main-3"));
        assert_eq!(l.logger.as_deref(), Some("sodium"));
    }

    #[test]
    fn servidor_clasico_sin_hilo() {
        let l = p("[12:34:56 INFO]: Starting minecraft server version 1.20.1");
        assert_eq!(l.time.as_deref(), Some("12:34:56"));
        assert_eq!(l.level, LogLevel::Info);
        assert_eq!(l.thread, None);
        assert_eq!(l.message, "Starting minecraft server version 1.20.1");
        assert!(!l.continuation);
    }

    #[test]
    fn forge_con_fecha_y_logger_entre_corchetes() {
        let l = p(
            "[12Oct2026 12:34:56.789] [main/INFO] [net.minecraftforge.fml.loading.moddiscovery.ModDiscoverer/SCAN]: Found 12 mods",
        );
        assert_eq!(l.time.as_deref(), Some("12:34:56"));
        assert_eq!(l.level, LogLevel::Info);
        assert_eq!(l.thread.as_deref(), Some("main"));
        assert_eq!(
            l.logger.as_deref(),
            Some("net.minecraftforge.fml.loading.moddiscovery.ModDiscoverer/SCAN")
        );
        assert_eq!(l.message, "Found 12 mods");
    }

    #[test]
    fn forge_con_logger_terminado_en_barra() {
        let l = p(
            "[12:34:56] [Server thread/WARN] [net.minecraft.server.MinecraftServer/]: Can't keep up!",
        );
        assert_eq!(l.level, LogLevel::Warn);
        assert_eq!(
            l.logger.as_deref(),
            Some("net.minecraft.server.MinecraftServer")
        );
        assert_eq!(l.message, "Can't keep up!");
    }

    #[test]
    fn formato_con_fecha_iso() {
        let l = p("2026-10-11 12:34:56,123 [ERROR] Something failed");
        assert_eq!(l.level, LogLevel::Error);
        assert_eq!(l.time.as_deref(), Some("12:34:56"));
        assert_eq!(l.message, "Something failed");
    }

    #[test]
    fn niveles_equivalentes() {
        for (name, level) in [
            ("TRACE", LogLevel::Trace),
            ("DEBUG", LogLevel::Debug),
            ("INFO", LogLevel::Info),
            ("WARN", LogLevel::Warn),
            ("WARNING", LogLevel::Warn),
            ("ERROR", LogLevel::Error),
            ("SEVERE", LogLevel::Error),
            ("FATAL", LogLevel::Fatal),
            ("FOO", LogLevel::Unknown),
        ] {
            let l = p(&format!("[01:02:03] [main/{name}]: x"));
            assert_eq!(l.level, level, "{name}");
            assert!(!l.continuation);
        }
    }

    #[test]
    fn continuaciones_heredan_el_nivel() {
        for raw in [
            "\tat net.minecraft.client.Minecraft.run(Minecraft.java:782)",
            "Caused by: java.lang.NullPointerException: boom",
            "\t... 12 more",
            "java.lang.RuntimeException: Mixin apply failed",
            "",
        ] {
            let l = parse_line(raw, LogStream::Stderr, LogLevel::Error);
            assert!(l.continuation, "{raw:?}");
            assert_eq!(l.level, LogLevel::Error);
            assert_eq!(l.stream, LogStream::Stderr);
            assert_eq!(l.thread, None);
            assert_eq!(l.message, raw);
        }
    }

    #[test]
    fn quita_ansi_y_saltos() {
        let l = p("\x1b[33m[12:34:56] [main/WARN]\x1b[0m: \x1b[1;31mcuidado\x1b[m\r\n");
        assert_eq!(l.level, LogLevel::Warn);
        assert_eq!(l.message, "cuidado");
        assert_eq!(l.raw, "[12:34:56] [main/WARN]: cuidado");
        assert_eq!(strip_ansi("\x1b[2K\x1b[1Gok"), "ok");
    }

    #[test]
    fn ruta_de_crash_report() {
        let esperado = Some(PathBuf::from(
            "/home/u/instances/a/game/crash-reports/crash-2026-10-11_12.34.56-client.txt",
        ));
        assert_eq!(
            crash_report_path(
                "[12:34:56] [Render thread/INFO]: This crash report has been saved to: /home/u/instances/a/game/crash-reports/crash-2026-10-11_12.34.56-client.txt"
            ),
            esperado
        );
        assert_eq!(
            crash_report_path(
                "#@!@# Game crashed! Crash report saved to: #@!@# /home/u/instances/a/game/crash-reports/crash-2026-10-11_12.34.56-client.txt"
            ),
            esperado
        );
        assert_eq!(
            crash_report_path(
                "[12:34:56] [Render thread/ERROR]: Crash report saved to: /home/u/instances/a/game/crash-reports/crash-2026-10-11_12.34.56-client.txt  "
            ),
            esperado
        );
        assert_eq!(crash_report_path("[12:34:56] [main/INFO]: todo bien"), None);
        assert_eq!(crash_report_path("Crash report saved to:   "), None);
    }

    #[test]
    fn anillo_da_la_vuelta() {
        let mut buf = LogBuffer::new(3);
        assert!(buf.is_empty());
        for i in 0..5 {
            let l = p(&format!("[00:00:0{i}] [main/INFO]: m{i}"));
            let seq = buf.push(l).seq;
            assert_eq!(seq, i);
        }
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.capacity(), 3);
        let snap = buf.snapshot();
        assert_eq!(
            snap.iter().map(|l| l.seq).collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        assert_eq!(snap[0].message, "m2");
        assert_eq!(buf.last_level(), LogLevel::Info);
        buf.clear();
        assert_eq!(buf.len(), 0);
        // El contador sigue creciendo tras limpiar.
        assert_eq!(buf.push(p("x")).seq, 5);
    }

    #[test]
    fn capacidad_por_defecto_y_minima() {
        assert_eq!(LogBuffer::default().capacity(), 5000);
        assert_eq!(LogBuffer::new(0).capacity(), 1);
    }
}
