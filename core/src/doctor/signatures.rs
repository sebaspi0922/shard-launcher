//! Firmas de problemas conocidos. Las comparaciones de texto se hacen en
//! minúsculas sobre el crash report, el `hs_err` y la cola del log.

use std::sync::LazyLock;

use regex::Regex;

use super::{Category, Diagnosis, DoctorInput, FixAction, ModFile, Severity};
use crate::instance::LoaderKind;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pat).expect("expresión regular válida"));
    };
}

/// Ids de mod demasiado genéricos para señalarlos como culpables.
const GENERIC_IDS: [&str; 8] = [
    "minecraft",
    "java",
    "mixin",
    "mixins",
    "mod",
    "default",
    "common",
    "json",
];

/// Texto en minúsculas listo para buscar.
pub(crate) struct Ctx<'a> {
    pub input: &'a DoctorInput,
    /// Crash report + `hs_err`.
    primary: String,
    /// `primary` + cola del log.
    all: String,
    pub crashed: bool,
}

impl<'a> Ctx<'a> {
    pub fn new(input: &'a DoctorInput) -> Self {
        let lower = |t: &Option<String>| t.as_deref().unwrap_or("").to_lowercase();
        let primary = format!(
            "{}\n{}",
            lower(&input.crash_text),
            lower(&input.hs_err_text)
        );
        let all = format!("{primary}\n{}", lower(&input.log_tail));
        let crashed = is_crash(input);
        Self {
            input,
            primary,
            all,
            crashed,
        }
    }

    fn has(&self, needle: &str) -> bool {
        self.all.contains(needle)
    }

    fn has_any(&self, needles: &[&str]) -> bool {
        needles.iter().any(|n| self.all.contains(n))
    }

    /// Confianza base; baja si la prueba solo está en el log y no en un crash report.
    fn conf(&self, base: u8, needles: &[&str]) -> u8 {
        if needles.iter().any(|n| self.primary.contains(n)) {
            base
        } else {
            base.saturating_sub(15)
        }
    }

    fn mods(&self) -> &[ModFile] {
        &self.input.mods
    }
}

/// ¿Hubo un cierre anómalo? Crash report, `hs_err` o marcas en el log.
pub(crate) fn is_crash(input: &DoctorInput) -> bool {
    if input.crash_text.is_some() || input.hs_err_text.is_some() {
        return true;
    }
    let log = input.log_tail.as_deref().unwrap_or("").to_lowercase();
    [
        "---- minecraft crash report ----",
        "game crashed",
        "a fatal error has been detected by the java runtime environment",
        "error occurred during initialization of vm",
        "could not create the java virtual machine",
        "exception in thread \"main\"",
    ]
    .iter()
    .any(|m| log.contains(m))
}

// --- utilidades ---------------------------------------------------------------

fn window(text: &str, pos: usize, before: usize, after: usize) -> &str {
    let mut start = pos.saturating_sub(before);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (pos + after).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    &text[start..end]
}

/// ¿Alguna aparición de `anchor` tiene cerca alguno de los `markers`?
fn near(text: &str, anchor: &str, markers: &[&str], radius: usize) -> bool {
    text.match_indices(anchor).any(|(pos, _)| {
        let w = window(text, pos, radius, radius + anchor.len());
        markers.iter().any(|m| w.contains(m))
    })
}

fn stem(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    lower
        .strip_suffix(".disabled")
        .unwrap_or(&lower)
        .strip_suffix(".jar")
        .unwrap_or(&lower)
        .to_owned()
}

fn tokens(stem: &str) -> Vec<&str> {
    stem.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect()
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Primer mod activo cuyo nombre de archivo aparece en `text`.
fn find_mod_in<'m>(mods: &'m [ModFile], text: &str) -> Option<&'m ModFile> {
    mods.iter().filter(|m| m.enabled).find(|m| {
        let n = m.file_name.to_lowercase();
        n.len() >= 5 && text.contains(&n)
    })
}

/// Mods activos cuyo archivo corresponde a un id de mod (`sodium`, `fabric-api`).
/// Primero los que empiezan por el id; si no hay, los que lo contienen.
fn mods_for_id<'m>(mods: &'m [ModFile], id: &str) -> Vec<&'m ModFile> {
    let id = norm(id);
    if id.len() < 3 {
        return Vec::new();
    }
    let enabled = || mods.iter().filter(|m| m.enabled);
    let starts: Vec<_> = enabled()
        .filter(|m| norm(&stem(&m.file_name)).starts_with(&id))
        .collect();
    if !starts.is_empty() {
        return starts;
    }
    enabled()
        .filter(|m| norm(&stem(&m.file_name)).contains(&id))
        .collect()
}

fn round_down_512(mb: u64) -> u32 {
    (mb / 512 * 512).min(u32::MAX as u64) as u32
}

/// Memoria razonable para este equipo: la mitad de la RAM, entre 1 y 8 GB.
fn sane_memory_mb(total: Option<u64>) -> u32 {
    match total {
        Some(t) => round_down_512((t / 2).clamp(1024, 8192)),
        None => 4096,
    }
}

/// Más memoria que la actual sin pasar del 70 % de la RAM.
fn more_memory_mb(current: u32, total: Option<u64>) -> Option<u32> {
    let current64 = u64::from(current);
    let target = (current64 + 1024).max(current64 * 3 / 2);
    let cap = total.map_or(16384, |t| t * 7 / 10);
    let suggested = round_down_512(target.min(cap));
    (suggested > current).then_some(suggested)
}

/// Menos memoria que la actual: tres cuartas partes, sin pasar del 60 % de la RAM.
fn less_memory_mb(current: u32, total: Option<u64>) -> Option<u32> {
    let mut target = u64::from(current) * 3 / 4;
    if let Some(t) = total {
        target = target.min(t * 6 / 10);
    }
    let suggested = round_down_512(target).max(1024);
    (suggested < current).then_some(suggested)
}

fn has_kind(out: &[Diagnosis], kinds: &[&str]) -> bool {
    out.iter().any(|d| kinds.contains(&d.kind.as_str()))
}

// --- ejecución ----------------------------------------------------------------

/// Firmas que dependen del texto de los logs.
pub(crate) fn run_text(ctx: &Ctx) -> Vec<Diagnosis> {
    let mut out = Vec::new();
    java_version(ctx, &mut out);
    reserve_space(ctx, &mut out);
    out_of_memory(ctx, &mut out);
    bad_jvm_flag(ctx, &mut out);
    mixin_crash(ctx, &mut out);
    dependencies(ctx, &mut out);
    duplicate_mods(ctx, &mut out);
    fallback_mod_loading(ctx, &mut out);
    corrupted_archive(ctx, &mut out);
    corrupted_options(ctx, &mut out);
    corrupted_servers_dat(ctx, &mut out);
    session_lock(ctx, &mut out);
    world_tick_crash(ctx, &mut out);
    verify_error(ctx, &mut out);
    native_driver_crash(ctx, &mut out);
    optifine_incompatibility(ctx, &mut out);
    wrong_loader_mod(ctx, &mut out);
    loader_install_broken(ctx, &mut out);
    simple_signatures(ctx, &mut out);
    out
}

/// Comprobaciones que no necesitan un crash: mods incompatibles entre sí y memoria.
pub(crate) fn run_static(input: &DoctorInput) -> Vec<Diagnosis> {
    let mut out = Vec::new();
    incompatible_pairs(input, &mut out);
    memory_checks(input, &mut out);
    out
}

pub(crate) fn unknown_crash(input: &DoctorInput) -> Diagnosis {
    let excerpt = super::crash_excerpt(input);
    let mut d = Diagnosis::new(
        "unknown_crash",
        Category::Unknown,
        Severity::Warning,
        30,
        "El juego se cerró de forma inesperada",
        "No se reconoce la causa. Revisa el extracto del error y el crash report.",
    );
    if let Some(e) = excerpt {
        d = d.param("excerpt", e);
    }
    d
}

// --- Java y memoria -------------------------------------------------------------

re!(
    CLASS_VERSION,
    r"(?:class file version|unsupported major\.minor version) (\d+)\.\d+"
);
re!(RECOGNIZES_UP_TO, r"class file versions up to (\d+)\.\d+");
re!(ASM_MAJOR, r"unsupported class file major version (\d+)");

fn java_of_class_version(v: u32) -> Option<u32> {
    (v >= 49).then(|| v - 44)
}

fn java_version(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let err = [
        "unsupportedclassversionerror",
        "has been compiled by a more recent version of the java runtime",
        "unsupported major.minor version",
    ];
    if ctx.has_any(&err) {
        let cap = |re: &Regex| {
            re.captures(&ctx.all)
                .and_then(|c| c[1].parse::<u32>().ok())
                .and_then(java_of_class_version)
        };
        let required = cap(&CLASS_VERSION).or(ctx.input.required_java);
        let mut d = Diagnosis::new(
            "java_version_mismatch",
            Category::Java,
            Severity::Critical,
            ctx.conf(90, &err),
            "Versión de Java incompatible",
            match required {
                Some(r) => format!(
                    "El juego o un mod necesita Java {r} o superior, y se está usando una más antigua."
                ),
                None => {
                    "El juego o un mod necesita una Java más reciente que la que se está usando."
                        .to_owned()
                }
            },
        );
        if let Some(r) = required {
            d = d
                .param("required", r)
                .fix(FixAction::InstallJava { major: r });
        }
        if let Some(cur) = cap(&RECOGNIZES_UP_TO) {
            d = d.param("current", cur);
        }
        out.push(d);
    } else if let Some(v) = ASM_MAJOR
        .captures(&ctx.all)
        .and_then(|c| c[1].parse::<u32>().ok())
    {
        // Lo lanza ASM: lo habitual es que la Java sea más nueva que lo que entiende el loader o un mod.
        let mut d = Diagnosis::new(
            "java_version_mismatch",
            Category::Java,
            Severity::Warning,
            ctx.conf(60, &["unsupported class file major version"]),
            "Versión de Java incompatible",
            "Una herramienta del juego no entiende las clases de la Java en uso. Prueba con la Java recomendada para esta versión de Minecraft.",
        )
        .param("class_file_major", v);
        if let Some(r) = ctx.input.required_java {
            d = d
                .param("required", r)
                .fix(FixAction::InstallJava { major: r });
        }
        out.push(d);
    }
}

re!(RESERVE_KB, r"could not reserve enough space for (\d+)kb");

fn reserve_space(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = ["could not reserve enough space for"];
    if !ctx.has_any(&key) {
        return;
    }
    let current = ctx.input.memory_mb;
    let suggested = less_memory_mb(current, ctx.input.total_memory_mb);
    let mut d = Diagnosis::new(
        "reserve_space_failed",
        Category::Memory,
        Severity::Critical,
        ctx.conf(90, &key),
        "No se pudo reservar la memoria pedida",
        format!(
            "La JVM no consiguió reservar {current} MB. Asigna menos memoria, cierra otras aplicaciones o usa una Java de 64 bits."
        ),
    )
    .param("current", current);
    if let Some(kb) = RESERVE_KB.captures(&ctx.all) {
        d = d.param("requested_kb", &kb[1]);
    }
    if let Some(t) = ctx.input.total_memory_mb {
        d = d.param("total", t);
    }
    if let Some(s) = suggested {
        d = d.param("suggested", s).fix(FixAction::SetMemory { mb: s });
    }
    out.push(d);
}

fn out_of_memory(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let native = [
        "unable to create native thread",
        "insufficient memory for the java runtime environment",
        "native memory allocation (mmap) failed",
        "native memory allocation (malloc) failed",
    ];
    let generic = ["java.lang.outofmemoryerror", "out of memory"];
    let current = ctx.input.memory_mb;
    let total = ctx.input.total_memory_mb;
    if ctx.has_any(&native) {
        let suggested = less_memory_mb(current, total).filter(|_| {
            // Solo tiene sentido bajar la memoria si es una parte grande de la RAM.
            total.is_none_or(|t| u64::from(current) * 10 > t * 5)
        });
        let mut d = Diagnosis::new(
            "out_of_memory",
            Category::Memory,
            Severity::Critical,
            ctx.conf(85, &native),
            "El sistema se quedó sin memoria nativa",
            "La JVM no pudo pedir más memoria al sistema. Cierra otras aplicaciones o reduce la memoria asignada al juego.",
        )
        .param("current", current)
        .param("scope", "native");
        if let Some(t) = total {
            d = d.param("total", t);
        }
        if let Some(s) = suggested {
            d = d.param("suggested", s).fix(FixAction::SetMemory { mb: s });
        }
        out.push(d);
    } else if ctx.has_any(&generic) && !has_kind(out, &["reserve_space_failed"]) {
        let suggested = more_memory_mb(current, total);
        let mut d = Diagnosis::new(
            "out_of_memory",
            Category::Memory,
            Severity::Critical,
            ctx.conf(85, &generic),
            "El juego se quedó sin memoria",
            format!("El juego agotó los {current} MB asignados. Prueba con más memoria."),
        )
        .param("current", current)
        .param("scope", "heap");
        if let Some(t) = total {
            d = d.param("total", t);
        }
        if let Some(s) = suggested {
            d = d.param("suggested", s).fix(FixAction::SetMemory { mb: s });
        }
        out.push(d);
    }
}

re!(UNRECOGNIZED_VM, r"unrecognized vm option '([^'\n]+)'");
re!(UNRECOGNIZED_OPT, r"unrecognized option:?\s*([^\s]+)");

fn bad_jvm_flag(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let token = UNRECOGNIZED_VM
        .captures(&ctx.all)
        .or_else(|| UNRECOGNIZED_OPT.captures(&ctx.all))
        .map(|c| c[1].trim().to_owned());
    let generic = [
        "could not create the java virtual machine",
        "error occurred during initialization of vm",
    ];
    let generic_hit = ctx.has_any(&generic)
        && !has_kind(
            out,
            &[
                "reserve_space_failed",
                "out_of_memory",
                "java_version_mismatch",
            ],
        );
    if token.is_none() && !generic_hit {
        return;
    }
    let args = &ctx.input.jvm_args;
    let base = if token.is_some() { 85 } else { 45 };
    let needles = [
        "unrecognized",
        "could not create the java virtual machine",
        "error occurred during initialization of vm",
    ];
    let culprit = token.as_deref().and_then(|t| {
        let t = t.trim_start_matches(['+', '-', ':']).to_lowercase();
        (t.len() >= 3)
            .then(|| args.iter().find(|a| a.to_lowercase().contains(&t)))
            .flatten()
    });
    let d = match (culprit, args.is_empty()) {
        (Some(arg), _) => Diagnosis::new(
            "bad_jvm_flag",
            Category::JvmArgs,
            Severity::Critical,
            ctx.conf(base + 5, &needles),
            "Argumento de JVM no válido",
            format!("La JVM no reconoce el argumento {arg}. Quítalo de los argumentos de la instancia."),
        )
        .culprit(arg.clone())
        .fix(FixAction::RemoveJvmArg { arg: arg.clone() }),
        (None, false) => Diagnosis::new(
            "bad_jvm_flag_unknown",
            Category::JvmArgs,
            Severity::Critical,
            ctx.conf(base - 20, &needles),
            "Los argumentos de JVM impiden arrancar",
            "La JVM no pudo iniciarse y la instancia tiene argumentos personalizados. Vaciarlos suele resolverlo.",
        )
        .fix(FixAction::ClearJvmArgs),
        (None, true) => Diagnosis::new(
            "bad_jvm_flag_no_custom_args",
            Category::JvmArgs,
            Severity::Critical,
            ctx.conf(base - 25, &needles),
            "La JVM no pudo iniciarse",
            "La instancia no tiene argumentos personalizados; revisa la variable JAVA_TOOL_OPTIONS y la Java elegida.",
        ),
    };
    out.push(match token {
        Some(t) => d.param("flag", t),
        None => d,
    });
}

// --- mods -------------------------------------------------------------------------

re!(
    MIXIN_FROM_MOD,
    r"(?:from mod|mixin apply for mod)\s+'?([a-z0-9_\-]+)"
);
re!(MIXIN_JSON_PREFIX, r"([a-z0-9_\-]+)\.mixins?\.json");
re!(MIXIN_JSON_SUFFIX, r"mixins?\.([a-z0-9_\-]+)\.json");

fn mixin_crash(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = [
        "mixinapplyerror",
        "mixintransformererror",
        "invalidmixinexception",
        "mixin apply failed",
        "mixin transformation of",
    ];
    if !(ctx.has_any(&key) || (ctx.has("mixin") && ctx.has("failed to apply"))) {
        return;
    }
    // Candidatos a id de mod, del más fiable al menos; cada uno también por su primer segmento.
    let mut candidates: Vec<String> = Vec::new();
    for re in [&MIXIN_FROM_MOD, &MIXIN_JSON_PREFIX, &MIXIN_JSON_SUFFIX] {
        for c in re.captures_iter(&ctx.all) {
            let id = c[1].to_owned();
            let head = id.split(['-', '_', '.']).next().unwrap_or("").to_owned();
            for t in [id, head] {
                if t.len() >= 3 && !GENERIC_IDS.contains(&t.as_str()) && !candidates.contains(&t) {
                    candidates.push(t);
                }
            }
        }
    }
    let found = candidates.iter().find_map(|c| {
        let m = mods_for_id(ctx.mods(), c);
        (!m.is_empty()).then_some((c, m))
    });
    let conf = ctx.conf(80, &key);
    let d = match found {
        Some((id, files)) => {
            let first = files[0];
            let mut d = Diagnosis::new(
                "mixin_crash_known",
                Category::ModConflict,
                Severity::Critical,
                if files.len() > 1 { conf.saturating_sub(10) } else { conf },
                "Un mod falló al aplicarse (Mixin)",
                format!("El mod «{id}» ({}) provoca un error de Mixin; suele ser una incompatibilidad con otro mod o con la versión del juego.", first.file_name),
            )
            .culprit(first.file_name.clone())
            .param("mod_id", id)
            .fix(FixAction::DisableMod { file_name: first.file_name.clone() });
            if files.len() > 1 {
                let alts: Vec<_> = files.iter().skip(1).map(|f| f.file_name.as_str()).collect();
                d = d.param("alternatives", alts.join(", "));
            }
            d
        }
        None => {
            let mut d = Diagnosis::new(
                "mixin_crash_unknown",
                Category::ModConflict,
                Severity::Critical,
                conf.saturating_sub(25),
                "Un mod falló al aplicarse (Mixin)",
                "Hay un error de Mixin, pero no se pudo identificar el mod. Prueba a desactivar los mods añadidos más recientemente.",
            );
            if let Some(c) = candidates.first() {
                d = d.culprit(c.clone());
            }
            d
        }
    };
    out.push(d);
}

re!(
    FORGE_DEP,
    r"mod id: '([^']+)', requested by: '([^']+)', expected range: '([^']+)', actual version: '([^']+)'"
);
re!(REQUIRES_LINE, r"(?m)^.*\brequires\b.*$");
re!(PAREN_ID, r"\(([a-z0-9_\-]+)\)");
re!(WORD_ID, r"^(?:mod\s+)?'?([a-z0-9_\-]+)");

/// Dependencia que falta o que no cumple la versión, con quien la pide.
struct DepIssue {
    dependency: String,
    required_by: Option<String>,
    expected: Option<String>,
    actual: Option<String>,
}

fn fabric_dep_issues(text: &str) -> (Vec<DepIssue>, Vec<DepIssue>) {
    let (mut missing, mut wrong) = (Vec::new(), Vec::new());
    for m in REQUIRES_LINE.find_iter(text) {
        let line = m.as_str();
        let is_missing = line.contains("which is missing") || line.contains("is not installed");
        let is_wrong = line.contains("wrong version")
            || line.contains("but only")
            || line.contains("which is present");
        if !(is_missing || is_wrong) {
            continue;
        }
        let Some(req_pos) = line.find("requires") else {
            continue;
        };
        let (before, after) = line.split_at(req_pos);
        let after = &after["requires".len()..];
        let Some(of_pos) = after.find(" of ") else {
            continue;
        };
        let dep_part = after[of_pos + 4..].split(", ").next().unwrap_or("");
        let dependency = PAREN_ID
            .captures_iter(dep_part)
            .last()
            .map(|c| c[1].to_owned())
            .or_else(|| WORD_ID.captures(dep_part.trim()).map(|c| c[1].to_owned()));
        let Some(dependency) = dependency else {
            continue;
        };
        let issue = DepIssue {
            dependency,
            required_by: PAREN_ID.captures(before).map(|c| c[1].to_owned()),
            expected: Some(after[..of_pos].trim().to_owned()).filter(|s| !s.is_empty()),
            actual: None,
        };
        if is_missing {
            missing.push(issue)
        } else {
            wrong.push(issue)
        }
    }
    (missing, wrong)
}

fn dependencies(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let forge_key = [
        "missing or unsupported mandatory dependencies",
        "unsupported installed mandatory dependencies",
        "unsupported installed optional dependencies",
    ];
    let (mut missing, mut wrong): (Vec<DepIssue>, Vec<DepIssue>) = (Vec::new(), Vec::new());
    let mut forge = false;
    for c in FORGE_DEP.captures_iter(&ctx.all) {
        forge = true;
        let issue = DepIssue {
            dependency: c[1].to_owned(),
            required_by: Some(c[2].to_owned()),
            expected: Some(c[3].to_owned()),
            actual: Some(c[4].to_owned()),
        };
        if c[4].contains("missing") {
            missing.push(issue)
        } else {
            wrong.push(issue)
        }
    }
    let (fm, fw) = fabric_dep_issues(&ctx.all);
    missing.extend(fm);
    wrong.extend(fw);
    let generic = ctx.has_any(&forge_key) || ctx.has("requires version ") || ctx.has("requires {");

    if !missing.is_empty() || (generic && wrong.is_empty()) {
        let ids = unique(missing.iter().map(|i| i.dependency.as_str()));
        let by = unique(missing.iter().filter_map(|i| i.required_by.as_deref()));
        let mut d = Diagnosis::new(
            "missing_dependency",
            Category::MissingDependency,
            Severity::Critical,
            ctx.conf(
                if ids.is_empty() { 60 } else { 90 },
                &["requires", "dependencies"],
            ),
            "Falta una dependencia de un mod",
            if ids.is_empty() {
                "Un mod necesita otro mod o una versión distinta que no está instalada. Revisa el mensaje del loader.".to_owned()
            } else {
                format!(
                    "Faltan mods necesarios: {}. Instálalos desde Modrinth o CurseForge.",
                    ids.join(", ")
                )
            },
        );
        if let Some(first) = ids.first() {
            d = d
                .culprit(first.clone())
                .param("dependency", first)
                .param("dependencies", ids.join(", "));
        }
        if !by.is_empty() {
            d = d.param("required_by", by.join(", "));
        }
        if let Some(e) = missing.first().and_then(|i| i.expected.clone()) {
            d = d.param("expected", e);
        }
        out.push(d);
    }
    if let Some(first) = wrong.first() {
        let kind = if forge {
            "forge_dependency_version_mismatch"
        } else {
            "dependency_version_mismatch"
        };
        let mut d = Diagnosis::new(
            kind,
            Category::MissingDependency,
            Severity::Critical,
            ctx.conf(85, &["requires", "dependencies"]),
            "Versión incorrecta de una dependencia",
            format!(
                "El mod «{}» no está en la versión que otros mods necesitan. Actualízalo o instala la versión adecuada.",
                first.dependency
            ),
        )
        .culprit(first.dependency.clone())
        .param("dependency", &first.dependency);
        if let Some(by) = &first.required_by {
            d = d.param("required_by", by);
        }
        if let Some(e) = &first.expected {
            d = d.param("expected", e);
        }
        if let Some(a) = &first.actual {
            d = d.param("actual", a);
        }
        out.push(d);
    }
}

fn unique<'a>(items: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for i in items {
        if !v.iter().any(|x| x == i) {
            v.push(i.to_owned());
        }
    }
    v
}

re!(JAR_NAME, r"[^\s,\[\]()/\\'\x22]+\.jar");
re!(DUP_ID, r"mod id:? '?([a-z0-9_\-]+)'?");

fn duplicate_mods(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = ["found a duplicate mod", "duplicate mod id", "duplicate mod"];
    let Some(pos) = key.iter().find_map(|k| ctx.all.find(k)) else {
        return;
    };
    let w = window(&ctx.all, pos, 100, 1500);
    let id = DUP_ID.captures(w).map(|c| c[1].to_owned());
    let mut files: Vec<&ModFile> = Vec::new();
    for j in JAR_NAME.find_iter(w) {
        if let Some(m) = ctx
            .mods()
            .iter()
            .find(|m| m.enabled && m.file_name.to_lowercase() == j.as_str())
            && !files.contains(&m)
        {
            files.push(m);
        }
    }
    if files.len() < 2
        && let Some(id) = &id
    {
        files = mods_for_id(ctx.mods(), id);
    }
    files.sort_by_key(|m| m.file_name.to_lowercase());
    let mut d = Diagnosis::new(
        "duplicate_mods",
        Category::ModConflict,
        Severity::Critical,
        ctx.conf(90, &key),
        "Mods duplicados",
        "Hay dos copias del mismo mod en la carpeta mods. Deja solo una (normalmente la más reciente).",
    );
    if let Some(id) = &id {
        d = d.culprit(id.clone()).param("mod_id", id);
    }
    if files.len() >= 2 {
        d = d
            .param(
                "files",
                files
                    .iter()
                    .map(|f| f.file_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .fix(FixAction::DisableMod {
                file_name: files[0].file_name.clone(),
            });
        d.explanation
            .push_str(&format!(" Se propone desactivar {}.", files[0].file_name));
    }
    out.push(d);
}

/// Mensajes genéricos de carga de mods; solo si no hay una causa más concreta.
fn fallback_mod_loading(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let specific = has_kind(
        out,
        &[
            "missing_dependency",
            "forge_dependency_version_mismatch",
            "dependency_version_mismatch",
            "duplicate_mods",
            "mixin_crash_known",
            "mixin_crash_unknown",
        ],
    );
    if specific {
        return;
    }
    if ctx.has("incompatible mod set") {
        out.push(Diagnosis::new(
            "incompatible_mod_set_fabric",
            Category::ModConflict,
            Severity::Critical,
            ctx.conf(55, &["incompatible mod set"]),
            "Conjunto de mods incompatible",
            "Fabric rechazó la combinación de mods. Lee el mensaje del loader: indica qué mod falta o sobra.",
        ));
    } else if ctx.has_any(&[
        "rejected by forge's mod resolution",
        "mod loading has failed",
        "modloadingexception",
    ]) {
        out.push(Diagnosis::new(
            "forge_mod_resolution_failed",
            Category::ModConflict,
            Severity::Critical,
            ctx.conf(55, &["mod loading", "modloadingexception"]),
            "Forge rechazó algunos mods",
            "Forge no pudo cargar la combinación de mods. Revisa el mensaje del loader para ver cuáles.",
        ));
    }
}

// --- archivos corruptos ---------------------------------------------------------------

re!(ZIP_PATH, r"error in opening zip file\s+([^\n]+)");

fn corrupted_archive(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = [
        "zip end header not found",
        "invalid cen header",
        "zip file is corrupt",
        "error in opening zip file",
        "not a valid zip file",
        "zip file closed",
    ];
    let Some(pos) = key.iter().filter_map(|k| ctx.all.find(k)).min() else {
        return;
    };
    let w = window(&ctx.all, pos, 400, 400);
    let named = ZIP_PATH
        .captures(&ctx.all)
        .map(|c| c[1].trim().trim_matches(['"', '\'']).replace('\\', "/"))
        .and_then(|p| p.rsplit('/').next().map(str::to_owned));
    let in_list = named
        .as_deref()
        .and_then(|n| {
            ctx.mods()
                .iter()
                .find(|m| m.enabled && m.file_name.to_lowercase() == n)
        })
        .or_else(|| find_mod_in(ctx.mods(), w));
    let mut d = Diagnosis::new(
        "corrupted_archive",
        Category::CorruptedFile,
        Severity::Critical,
        ctx.conf(85, &key),
        "Archivo comprimido dañado",
        "Un archivo .jar o .zip está corrupto (descarga incompleta). Vuelve a descargarlo.",
    );
    if let Some(m) = in_list {
        d = d.culprit(m.file_name.clone()).fix(FixAction::DisableMod {
            file_name: m.file_name.clone(),
        });
        d.explanation = format!(
            "El archivo {} está corrupto (descarga incompleta). Se propone desactivarlo; luego vuelve a descargarlo.",
            m.file_name
        );
    } else if let Some(n) = named {
        d = d.culprit(n);
    }
    out.push(d);
}

fn corrupted_options(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let markers = [
        "failed to load",
        "failed to parse",
        "could not parse",
        "couldn't parse",
        "unable to parse",
        "could not load",
        "couldn't load",
    ];
    if near(&ctx.all, "options.txt", &markers, 300) && !ctx.has("skipping bad option") {
        out.push(
            Diagnosis::new(
                "corrupted_options",
                Category::CorruptedFile,
                Severity::Warning,
                ctx.conf(75, &["options.txt"]),
                "Archivo de opciones dañado",
                "options.txt no se puede leer. Se puede borrar: el juego lo regenera con los valores por defecto (pierdes tus controles y ajustes de vídeo).",
            )
            .culprit("options.txt")
            .fix(FixAction::DeleteInstanceFile { relative_path: "options.txt".into() }),
        );
    }
}

fn corrupted_servers_dat(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let markers = [
        "couldn't load",
        "could not load",
        "failed to load",
        "failed to read",
        "could not read",
        "eofexception",
        "not in gzip format",
    ];
    if near(&ctx.all, "servers.dat", &markers, 300) {
        out.push(
            Diagnosis::new(
                "corrupted_servers_dat",
                Category::CorruptedFile,
                Severity::Warning,
                ctx.conf(75, &["servers.dat"]),
                "Lista de servidores dañada",
                "servers.dat no se puede leer. Se puede borrar: el juego crea una lista vacía (pierdes los servidores guardados).",
            )
            .culprit("servers.dat")
            .fix(FixAction::DeleteInstanceFile { relative_path: "servers.dat".into() }),
        );
    }
}

fn session_lock(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = ["failed to check session lock"];
    if ctx.has_any(&key) {
        out.push(
            Diagnosis::new(
                "session_lock_corrupted",
                Category::WorldCorruption,
                Severity::Warning,
                ctx.conf(85, &key),
                "Bloqueo de mundo sobrante",
                "Un mundo quedó bloqueado por un cierre anterior o por otra copia del juego. Con el juego cerrado, se pueden borrar los archivos session.lock.",
            )
            .fix(FixAction::ClearWorldLocks),
        );
    }
}

re!(ENTITY_TYPE, r"(?m)^\s*(?:block )?entity type:\s*([^\n(]+)");

fn world_tick_crash(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = [
        "description: ticking entity",
        "description: ticking block entity",
        "description: exception ticking world",
        "description: exception ticking level",
        "description: exception generating new chunk",
    ];
    if !ctx.has_any(&key) {
        return;
    }
    let entity = ENTITY_TYPE
        .captures(&ctx.all)
        .map(|c| c[1].trim().to_owned());
    let d = match &entity {
        Some(e) => {
            let ns = e
                .split(':')
                .next()
                .filter(|_| e.contains(':'))
                .unwrap_or("minecraft");
            let mut d = Diagnosis::new(
                "world_tick_crash_known",
                Category::WorldCorruption,
                Severity::Critical,
                ctx.conf(80, &key),
                "Una entidad bloqueó el juego",
                format!("«{e}» provocó el error al actualizarse. Puede ser un fallo del mod que la añade o un dato dañado del mundo; restaura una copia si persiste."),
            )
            .culprit(e.clone())
            .param("namespace", ns);
            if ns != "minecraft"
                && let Some(m) = mods_for_id(ctx.mods(), ns).first()
            {
                d = d.param("mod_file", m.file_name.clone());
            }
            d
        }
        None => Diagnosis::new(
            "world_tick_crash_unknown",
            Category::WorldCorruption,
            Severity::Critical,
            ctx.conf(60, &key),
            "Error al actualizar el mundo",
            "El juego falló mientras actualizaba el mundo; puede deberse a un mod o a datos dañados. Restaura una copia del mundo si persiste.",
        ),
    };
    out.push(d);
}

fn verify_error(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = ["java.lang.verifyerror"];
    let Some(pos) = ctx.all.find(key[0]) else {
        return;
    };
    let mut d = Diagnosis::new(
        "verify_error",
        Category::ModConflict,
        Severity::Critical,
        ctx.conf(80, &key),
        "Código de un mod no válido",
        "Un mod (o una modificación de otro) tiene código que la JVM rechaza: suele ser una versión incompatible con el juego o con otro mod.",
    );
    if let Some(m) = find_mod_in(ctx.mods(), window(&ctx.all, pos, 600, 1200)) {
        d = d.culprit(m.file_name.clone());
    }
    out.push(d);
}

// --- sistema y gráficos --------------------------------------------------------------------

re!(
    PROBLEMATIC_FRAME,
    r"problematic frame:\s*#\s*([a-z])\s+\[([^\]\n]*)\]"
);

fn gpu_vendor(lib: &str) -> Option<&'static str> {
    const NVIDIA: [&str; 5] = ["nvoglv", "nvidia", "nvwgf", "nvd3d", "nvcuda"];
    const AMD: [&str; 7] = ["amd", "radeon", "atio", "atig", "atifx", "vlk", "r600"];
    const INTEL: [&str; 6] = ["intel", "iris_dri", "i965", "igd", "igxel", "anv_"];
    let intel_ig = lib.starts_with("ig") && lib.chars().nth(2).is_some_and(|c| c.is_ascii_digit());
    if NVIDIA.iter().any(|n| lib.contains(n)) {
        Some("NVIDIA")
    } else if AMD.iter().any(|n| lib.contains(n)) {
        Some("AMD")
    } else if intel_ig || INTEL.iter().any(|n| lib.contains(n)) {
        Some("Intel")
    } else {
        None
    }
}

fn native_driver_crash(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    if !ctx
        .primary
        .contains("a fatal error has been detected by the java runtime environment")
    {
        return;
    }
    let Some(c) = PROBLEMATIC_FRAME.captures(&ctx.primary) else {
        return;
    };
    if &c[1] != "c" && &c[1] != "v" {
        return;
    }
    let lib = c[2].split('+').next().unwrap_or("").trim().to_owned();
    let vendor = gpu_vendor(&lib);
    let graphics = [
        "lwjgl", "opengl", "glfw", "libgl", "mesa", "vulkan", "d3d", "dri",
    ]
    .iter()
    .any(|g| lib.contains(g));
    if vendor.is_none() && !graphics {
        return;
    }
    let mut d = Diagnosis::new(
        "native_driver_crash",
        Category::GraphicsDriver,
        Severity::Critical,
        if vendor.is_some() { 85 } else { 65 },
        "Fallo del driver gráfico",
        format!(
            "El juego se cerró por un error en la biblioteca nativa {lib}. Actualiza el driver de la tarjeta gráfica{}.",
            if vendor.is_some() { "" } else { " o prueba sin mods de rendimiento y shaders" }
        ),
    )
    .culprit(lib.clone())
    .param("library", lib);
    if let Some(v) = vendor {
        d = d.param("gpu", v);
    }
    out.push(d);
}

/// Firmas de una sola cadena y sin arreglo automático.
fn simple_signatures(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let mut add = |kind: &str, cat, sev, conf, title: &str, expl: &str, keys: &[&str]| {
        if ctx.has_any(keys) {
            out.push(Diagnosis::new(
                kind,
                cat,
                sev,
                ctx.conf(conf, keys),
                title,
                expl,
            ));
        }
    };
    add(
        "vanilla_opengl_not_supported",
        Category::GraphicsDriver,
        Severity::Critical,
        85,
        "OpenGL no compatible",
        "La tarjeta gráfica o su driver no cumple la versión mínima de OpenGL. Actualiza el driver o usa la GPU dedicada.",
        &[
            "not compatible with minimum opengl version",
            "the driver does not appear to support opengl",
            "glx: failed to find a suitable glxfbconfig",
            "glfw error 65543",
        ],
    );
    add(
        "pixel_format_not_accelerated",
        Category::GraphicsDriver,
        Severity::Critical,
        85,
        "Formato de píxel sin aceleración",
        "Se está usando el renderizador por software. Instala o actualiza el driver gráfico y elige la GPU correcta.",
        &["pixel format not accelerated"],
    );
    add(
        "address_already_in_use",
        Category::Network,
        Severity::Warning,
        80,
        "Puerto en uso",
        "Otro programa (o otra copia del juego) está usando el puerto que el juego quiere abrir.",
        &["address already in use"],
    );
    add(
        "too_many_open_files",
        Category::System,
        Severity::Warning,
        85,
        "Demasiados archivos abiertos",
        "El sistema alcanzó el límite de archivos abiertos. Sube el límite (ulimit -n) o reinicia la sesión.",
        &["too many open files"],
    );
    add(
        "disk_full",
        Category::System,
        Severity::Critical,
        90,
        "Disco lleno",
        "No queda espacio en el disco. Libera espacio y vuelve a intentarlo.",
        &["no space left on device"],
    );
    let denied = ctx.has("java.nio.file.accessdeniedexception")
        || (ctx.has("java.io.ioexception")
            && ctx.has_any(&["access is denied", "permission denied", "access denied"]));
    if denied {
        out.push(Diagnosis::new(
            "access_denied_writing",
            Category::System,
            Severity::Warning,
            ctx.conf(75, &["accessdeniedexception", "permission denied", "access is denied", "access denied"]),
            "Acceso denegado al escribir",
            "El juego no tiene permiso para escribir en un archivo o carpeta. Revisa los permisos de la carpeta de la instancia.",
        ));
    }
}

// --- loader y optifine ----------------------------------------------------------------------

fn optifine_incompatibility(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let key = ["nosuchmethoderror", "classcastexception"];
    if !(ctx.has_any(&key) && ctx.has("optifine")) {
        return;
    }
    let file = ctx.mods().iter().find(|m| {
        m.enabled
            && stem(&m.file_name).contains("optifine")
            && !stem(&m.file_name).contains("optifabric")
    });
    let mut d = Diagnosis::new(
        "optifine_incompatibility",
        Category::ModConflict,
        Severity::Critical,
        ctx.conf(if file.is_some() { 75 } else { 55 }, &key),
        "OptiFine es incompatible con algo del juego",
        "OptiFine modifica el renderizado y rompe otros mods o versiones. Desactívalo, o usa Sodium e Iris como alternativa en Fabric.",
    );
    if let Some(f) = file {
        d = d.culprit(f.file_name.clone()).fix(FixAction::DisableMod {
            file_name: f.file_name.clone(),
        });
    }
    out.push(d);
}

const LOADER_EVIDENCE: [&str; 10] = [
    "net/minecraftforge",
    "net.minecraftforge",
    "net/neoforged",
    "net.neoforged",
    "net/fabricmc",
    "net.fabricmc",
    "fabric.mod.json",
    "mods.toml",
    "quilt.mod.json",
    "modloadingexception",
];

fn wrong_loader_mod(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    // Conservador: solo con un cierre anómalo y rastros del loader en el texto.
    if !ctx.crashed || !ctx.has_any(&LOADER_EVIDENCE) {
        return;
    }
    let loader = ctx.input.loader;
    let has_connector = ctx
        .mods()
        .iter()
        .any(|m| m.enabled && tokens(&stem(&m.file_name)).contains(&"connector"));
    let mut count = 0;
    for m in ctx.mods().iter().filter(|m| m.enabled) {
        let stem = stem(&m.file_name);
        let t = tokens(&stem);
        let forge = t.iter().any(|x| *x == "forge" || *x == "neoforge");
        let fabric = t.contains(&"fabric");
        let quilt = t.contains(&"quilt");
        let (wrong, expected) = match loader {
            LoaderKind::Fabric => ((forge || quilt) && !fabric, "Fabric"),
            LoaderKind::Quilt => (forge && !fabric && !quilt, "Quilt"),
            LoaderKind::Forge | LoaderKind::NeoForge => (
                (fabric || quilt) && !forge && !has_connector,
                loader.as_str(),
            ),
            _ => (false, ""),
        };
        if !wrong {
            continue;
        }
        count += 1;
        if count > 5 {
            break;
        }
        out.push(
            Diagnosis::new(
                "wrong_loader_mod",
                Category::Loader,
                Severity::Critical,
                ctx.conf(70, &LOADER_EVIDENCE),
                "Mod de otro loader",
                format!(
                    "El archivo {} parece hecho para otro loader y esta instancia usa {expected}. Descarga la versión correcta o desactívalo.",
                    m.file_name
                ),
            )
            .culprit(m.file_name.clone())
            .param("instance_loader", loader.as_str())
            .fix(FixAction::DisableMod { file_name: m.file_name.clone() }),
        );
    }
}

re!(
    LOADER_CLASS_MISSING,
    r"noclassdeffounderror:?\s*net/(?:minecraftforge|neoforged|fabricmc)"
);

fn loader_install_broken(ctx: &Ctx, out: &mut Vec<Diagnosis>) {
    let class_missing =
        LOADER_CLASS_MISSING.is_match(&ctx.all) && !has_kind(out, &["wrong_loader_mod"]);
    let jni = ctx.has("a jni error has occurred") && !has_kind(out, &["java_version_mismatch"]);
    let other = ctx.has_any(&[
        "could not find or load main class",
        "missing or corrupt launcher profile",
    ]);
    if !(class_missing || jni || other) {
        return;
    }
    let loader = ctx.input.loader;
    let mut d = Diagnosis::new(
        "loader_install_broken",
        Category::Loader,
        Severity::Critical,
        ctx.conf(if class_missing { 75 } else { 65 }, &["noclassdeffounderror", "main class", "jni error", "launcher profile"]),
        "Instalación del loader dañada",
        "Faltan archivos del loader o de la versión del juego. Reinstalar el loader no toca tus mods, mundos ni ajustes.",
    )
    .param("loader", loader.as_str());
    if loader != LoaderKind::Vanilla {
        d = d.fix(FixAction::ReinstallLoader);
    }
    out.push(d);
}

// --- pares incompatibles y memoria ------------------------------------------------------------

type Pred = fn(&str) -> bool;

fn first_token_is(stem: &str, name: &str) -> bool {
    tokens(stem).first() == Some(&name)
}

fn is_optifine(s: &str) -> bool {
    s.contains("optifine") && !s.contains("optifabric")
}
fn is_sodium(s: &str) -> bool {
    first_token_is(s, "sodium")
}
fn is_iris(s: &str) -> bool {
    first_token_is(s, "iris") && !s.contains("flywheel")
}
fn is_phosphor(s: &str) -> bool {
    first_token_is(s, "phosphor")
}
fn is_starlight(s: &str) -> bool {
    first_token_is(s, "starlight")
}
fn is_fabricproxy_lite(s: &str) -> bool {
    s.starts_with("fabricproxy-lite") || s.starts_with("fabricproxylite")
}
fn is_fabricproxy(s: &str) -> bool {
    s.starts_with("fabricproxy") && !is_fabricproxy_lite(s)
}

fn incompatible_pairs(input: &DoctorInput, out: &mut Vec<Diagnosis>) {
    // (kind, a, b, nombres, cuál se propone desactivar: 0 = a, 1 = b)
    let pairs: [(&str, Pred, Pred, &str, &str, usize); 4] = [
        (
            "incompatible_pair_optifine_sodium",
            is_optifine,
            is_sodium,
            "OptiFine",
            "Sodium",
            0,
        ),
        (
            "incompatible_pair_optifine_iris",
            is_optifine,
            is_iris,
            "OptiFine",
            "Iris",
            0,
        ),
        (
            "incompatible_pair_phosphor_starlight",
            is_phosphor,
            is_starlight,
            "Phosphor",
            "Starlight",
            0,
        ),
        (
            "incompatible_pair_fabricproxy_fabricproxy-lite",
            is_fabricproxy,
            is_fabricproxy_lite,
            "FabricProxy",
            "FabricProxy-Lite",
            0,
        ),
    ];
    let enabled: Vec<(&ModFile, String)> = input
        .mods
        .iter()
        .filter(|m| m.enabled)
        .map(|m| (m, stem(&m.file_name)))
        .collect();
    for (kind, fa, fb, na, nb, disable) in pairs {
        let a = enabled.iter().find(|(_, s)| fa(s));
        let b = enabled.iter().find(|(_, s)| fb(s));
        let (Some((ma, _)), Some((mb, _))) = (a, b) else {
            continue;
        };
        let target = if disable == 0 { ma } else { mb };
        out.push(
            Diagnosis::new(
                kind,
                Category::ModConflict,
                Severity::Warning,
                85,
                format!("{na} y {nb} no son compatibles"),
                format!(
                    "{} y {} no pueden usarse a la vez. Se propone desactivar {}.",
                    ma.file_name, mb.file_name, target.file_name
                ),
            )
            .culprit(target.file_name.clone())
            .param("mods", format!("{}, {}", ma.file_name, mb.file_name))
            .fix(FixAction::DisableMod {
                file_name: target.file_name.clone(),
            }),
        );
    }
}

fn memory_checks(input: &DoctorInput, out: &mut Vec<Diagnosis>) {
    let current = input.memory_mb;
    if let Some(total) = input.total_memory_mb
        && u64::from(current) * 10 > total * 9
    {
        let sane = sane_memory_mb(Some(total));
        let mut d = Diagnosis::new(
            "memory_exceeds_ram",
            Category::Memory,
            Severity::Warning,
            85,
            "Memoria asignada casi igual a la RAM",
            format!("Se asignan {current} MB de un total de {total} MB. El sistema se queda sin memoria y puede cerrar o ralentizar el juego."),
        )
        .param("current", current)
        .param("total", total);
        if sane < current {
            d = d
                .param("suggested", sane)
                .fix(FixAction::SetMemory { mb: sane });
        }
        out.push(d);
    }
    let enabled_mods = input.mods.iter().filter(|m| m.enabled).count();
    if current < 2048 && enabled_mods > 40 {
        let suggested = 4096.min(sane_memory_mb(input.total_memory_mb)).max(2048);
        let mut d = Diagnosis::new(
            "low_memory_for_mods",
            Category::Memory,
            Severity::Info,
            70,
            "Poca memoria para tantos mods",
            format!("Con {enabled_mods} mods y {current} MB es fácil quedarse sin memoria. Se recomiendan al menos 4096 MB."),
        )
        .param("current", current)
        .param("mods", enabled_mods);
        if suggested > current {
            d = d
                .param("suggested", suggested)
                .fix(FixAction::SetMemory { mb: suggested });
        }
        out.push(d);
    }
}
