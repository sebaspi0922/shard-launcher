//! Lanzamiento: argumentos, proceso del juego, logs y fin de partida.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::auth::Session;
use crate::download::Verify;
use crate::events::{Event, LogStream};
use crate::install::{Installed, next_task_id};
use crate::instance::Instance;
use crate::launcher::Core;
use crate::meta::rules::{Features, Platform};
use crate::meta::version::ArgEntry;
use crate::{Error, Result};

pub const LAUNCHER_NAME: &str = "shard";
const MIN_HEAP_MB: u32 = 512;
const KEPT_LOGS: usize = 10;

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Cuenta (id o nombre); `None` = la activa.
    pub account: Option<String>,
    /// Entrar directamente a un servidor (`host[:puerto]`).
    pub server: Option<String>,
    /// Comprobar hashes de todo lo instalado.
    pub verify_files: bool,
    /// Quien lanza no se queda escuchando (CLI con `--detach`): la salida del
    /// juego va directa a `logs/latest.log` en vez de pasar por el núcleo.
    pub detached: bool,
}

/// Comando listo para ejecutar. Lleva el token en `args`: no se serializa y
/// `Debug` lo oculta; para mostrarlo usar `masked_args`.
#[derive(Clone)]
pub struct LaunchCommand {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    secret: String,
}

impl LaunchCommand {
    /// Argumentos con el token sustituido por `***`.
    pub fn masked_args(&self) -> Vec<String> {
        self.args.iter().map(|a| mask(a, &self.secret)).collect()
    }
}

impl std::fmt::Debug for LaunchCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchCommand")
            .field("java", &self.java)
            .field("args", &self.masked_args())
            .field("cwd", &self.cwd)
            .finish()
    }
}

fn mask(s: &str, secret: &str) -> String {
    if secret.len() > 1 && s.contains(secret) {
        s.replace(secret, "***")
    } else {
        s.to_owned()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GameInfo {
    pub game_id: u64,
    pub instance_id: String,
    pub pid: Option<u32>,
    pub started_at: i64,
}

/// Lo que aporta el núcleo para construir los argumentos (separado para probarlo).
pub struct ArgsInput<'a> {
    pub installed: &'a Installed,
    pub instance: &'a Instance,
    pub session: &'a Session,
    pub game_dir: &'a Path,
    pub libraries_dir: &'a Path,
    pub memory_mb: u32,
    pub jvm_args: Vec<String>,
    pub server: Option<(String, u16)>,
    pub platform: &'a Platform,
}

/// Construye la lista de argumentos (sin el ejecutable de Java).
pub fn build_args(input: &ArgsInput) -> Vec<String> {
    let v = &input.installed.version;
    let platform = input.platform;
    let mut features = Features::new();
    let window = &input.instance.window;
    let has_resolution = window.width.is_some() && window.height.is_some();
    features.insert("has_custom_resolution".into(), has_resolution);

    let modern = v.arguments.is_some();
    let quick_play = modern
        && input.server.is_some()
        && v.arguments.as_ref().is_some_and(|a| {
            a.game.iter().any(|e| matches!(e, ArgEntry::Conditional { rules, .. }
                if rules.iter().any(|r| r.features.as_ref().is_some_and(|f| f.contains_key("is_quick_play_multiplayer")))))
        });
    if quick_play {
        features.insert("is_quick_play_multiplayer".into(), true);
    }

    let sep = platform.classpath_separator();
    let mut classpath: Vec<String> = input
        .installed
        .classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    classpath.push(input.installed.client_jar.display().to_string());

    let mut vars: HashMap<&str, String> = HashMap::new();
    let s = input.session;
    vars.insert("auth_player_name", s.name.clone());
    vars.insert("auth_uuid", s.uuid.clone());
    vars.insert("auth_access_token", s.access_token.expose().to_owned());
    vars.insert(
        "auth_session",
        format!("token:{}:{}", s.access_token.expose(), s.uuid),
    );
    vars.insert("auth_xuid", String::new());
    vars.insert("clientid", String::new());
    vars.insert("user_type", s.user_type.as_arg().to_owned());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_name", v.id.clone());
    vars.insert(
        "version_type",
        v.kind.clone().unwrap_or_else(|| "release".into()),
    );
    vars.insert("game_directory", input.game_dir.display().to_string());
    vars.insert(
        "assets_root",
        input.installed.assets.root.display().to_string(),
    );
    vars.insert(
        "game_assets",
        input.installed.assets.game_assets.display().to_string(),
    );
    vars.insert(
        "assets_index_name",
        input.installed.assets.index_name.clone(),
    );
    vars.insert(
        "natives_directory",
        input.installed.natives_dir.display().to_string(),
    );
    vars.insert(
        "library_directory",
        input.libraries_dir.display().to_string(),
    );
    vars.insert("classpath_separator", sep.to_owned());
    vars.insert("classpath", classpath.join(sep));
    vars.insert("launcher_name", LAUNCHER_NAME.into());
    vars.insert("launcher_version", env!("CARGO_PKG_VERSION").into());
    if let (Some(w), Some(h)) = (window.width, window.height) {
        vars.insert("resolution_width", w.to_string());
        vars.insert("resolution_height", h.to_string());
    }
    if let Some((host, port)) = &input.server {
        vars.insert("quickPlayMultiplayer", format!("{host}:{port}"));
    }
    let subst = |arg: &str| substitute(arg, &vars);

    let mut out = vec![
        format!("-Xmx{}M", input.memory_mb),
        format!("-Xms{}M", MIN_HEAP_MB.min(input.memory_mb)),
    ];

    // JVM: del JSON o, en el formato antiguo, los mínimos equivalentes.
    match &v.arguments {
        Some(args) if !args.jvm.is_empty() => {
            for e in &args.jvm {
                out.extend(e.values(platform, &features).iter().map(|a| subst(a)));
            }
        }
        _ => {
            if platform.os == crate::meta::rules::Os::Osx {
                out.push("-XstartOnFirstThread".into());
            }
            out.push(subst("-Djava.library.path=${natives_directory}"));
            out.push(subst("-Dminecraft.launcher.brand=${launcher_name}"));
            out.push(subst("-Dminecraft.launcher.version=${launcher_version}"));
            out.push("-cp".into());
            out.push(subst("${classpath}"));
        }
    }
    // Los del usuario van después para que ganen.
    out.extend(input.jvm_args.iter().cloned());

    out.push(v.main_class.clone().unwrap_or_default());

    let mut game: Vec<String> = Vec::new();
    match (&v.arguments, &v.minecraft_arguments) {
        (Some(args), _) if !args.game.is_empty() => {
            for e in &args.game {
                game.extend(e.values(platform, &features).iter().map(|a| subst(a)));
            }
        }
        (_, Some(legacy)) => {
            game.extend(legacy.split_whitespace().map(subst));
            if has_resolution {
                game.extend(["--width".into(), vars["resolution_width"].clone()]);
                game.extend(["--height".into(), vars["resolution_height"].clone()]);
            }
        }
        _ => {}
    }
    if window.fullscreen {
        game.push("--fullscreen".into());
    }
    if let Some((host, port)) = &input.server
        && !quick_play
    {
        game.extend([
            "--server".into(),
            host.clone(),
            "--port".into(),
            port.to_string(),
        ]);
    }
    out.extend(game);
    out
}

/// Sustituye `${var}`; las desconocidas se dejan tal cual.
fn substitute(arg: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                match vars.get(name) {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&rest[start..start + 2 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn parse_server(s: &str) -> Result<(String, u16)> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::InvalidInput("dirección de servidor vacía".into()));
    }
    // IPv6 entre corchetes: [::1]:25565
    if let Some(rest) = s.strip_prefix('[') {
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| Error::InvalidInput(format!("dirección no válida: {s}")))?;
        let port = match tail.strip_prefix(':') {
            Some(p) => p
                .parse()
                .map_err(|_| Error::InvalidInput(format!("puerto no válido: {p}")))?,
            None => 25565,
        };
        return Ok((host.to_owned(), port));
    }
    match s.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => {
            let port = port
                .parse()
                .map_err(|_| Error::InvalidInput(format!("puerto no válido: {port}")))?;
            Ok((host.to_owned(), port))
        }
        _ => Ok((s.to_owned(), 25565)),
    }
}

impl Core {
    /// Instala lo que falte y devuelve el comando de lanzamiento.
    pub async fn prepare_launch(
        &self,
        instance_id: &str,
        opts: &LaunchOptions,
    ) -> Result<(Instance, LaunchCommand)> {
        let instance = self.get_instance(instance_id).await?;
        let settings = self.settings().await;
        let session = self.session(opts.account.as_deref()).await?;
        let version_id = self.prepare_loader(&instance.game).await?;
        let verify = if opts.verify_files {
            Verify::Full
        } else {
            Verify::Size
        };
        let installed = self.install_version(&version_id, verify).await?;

        let game_dir = self.instance_game_dir(&instance.id)?;
        tokio::fs::create_dir_all(&game_dir)
            .await
            .map_err(|e| Error::io(&game_dir, e))?;
        self.map_assets_to_resources(&installed.assets, &game_dir)
            .await?;

        let major = installed.version.java_major();
        let java = match &instance.java.path {
            Some(p) => p.clone(),
            None => self.java_for(major).await?,
        };

        let server = opts.server.as_deref().map(parse_server).transpose()?;
        let jvm_args = instance
            .java
            .jvm_args
            .clone()
            .unwrap_or_else(|| settings.default_jvm_args.clone());
        let platform = Platform::current();
        let libraries_dir = self.paths().libraries_dir();
        let args = build_args(&ArgsInput {
            installed: &installed,
            instance: &instance,
            session: &session,
            game_dir: &game_dir,
            libraries_dir: &libraries_dir,
            memory_mb: instance.memory_mb(settings.default_memory_mb),
            jvm_args,
            server,
            platform: &platform,
        });
        Ok((
            instance,
            LaunchCommand {
                java,
                args,
                cwd: game_dir,
                secret: session.access_token.expose().to_owned(),
            },
        ))
    }

    /// Java para una versión mayor: la encuentra o, con red, la instala.
    async fn java_for(&self, major: u32) -> Result<PathBuf> {
        if let Some(found) = self.find_java(major).await? {
            return Ok(found.path);
        }
        if self.http().is_offline() {
            return Err(Error::JavaRequired { major });
        }
        Ok(self.ensure_java(major).await?.path)
    }

    pub async fn running_games(&self) -> Vec<GameInfo> {
        self.inner.games.lock().await.values().cloned().collect()
    }

    /// Lanza la instancia. Devuelve al arrancar el proceso; el fin de la
    /// partida llega como `Event::GameExited`.
    pub async fn launch(&self, instance_id: &str, opts: LaunchOptions) -> Result<GameInfo> {
        let allow_multiple = self.settings().await.allow_multiple_games;
        if !allow_multiple && !self.inner.games.lock().await.is_empty() {
            return Err(Error::GameAlreadyRunning);
        }
        if let Err(e) = self.reconcile_sessions().await {
            tracing::warn!(error = %e, "no se pudieron revisar las partidas anteriores");
        }
        let (instance, cmd) = self.prepare_launch(instance_id, &opts).await?;
        tracing::info!(java = %cmd.java.display(), args = ?cmd.masked_args(), "lanzando");

        let logs_dir = self.instance_dir(&instance.id)?.join("logs");
        let log_file = open_new_log(&logs_dir).await?;

        let mut command = Command::new(&cmd.java);
        command
            .args(&cmd.args)
            .current_dir(&cmd.cwd)
            .stdin(Stdio::null());
        let mut log_file = log_file;
        match (opts.detached, log_file.take()) {
            (true, Some(file)) => {
                let file = file.into_std().await;
                let err = file.try_clone().map_err(|e| Error::io(&logs_dir, e))?;
                command.stdout(Stdio::from(file)).stderr(Stdio::from(err));
            }
            (detached, file) => {
                log_file = file;
                let out = if detached {
                    Stdio::null()
                } else {
                    Stdio::piped()
                };
                let err = if detached {
                    Stdio::null()
                } else {
                    Stdio::piped()
                };
                command.stdout(out).stderr(err);
            }
        }
        detach(&mut command);
        let mut child = command.spawn().map_err(|e| Error::io(&cmd.java, e))?;

        let game_id = next_task_id();
        let info = GameInfo {
            game_id,
            instance_id: instance.id.clone(),
            pid: child.id(),
            started_at: crate::fsutil::now_secs(),
        };
        self.inner.games.lock().await.insert(game_id, info.clone());
        self.write_launch_record(
            &instance.id,
            &crate::sessions::LaunchRecord {
                started_at: info.started_at,
                pid: info.pid,
                running: true,
                ended_at: None,
            },
        )
        .await
        .unwrap_or_else(
            |e| tracing::warn!(error = %e, "no se pudo guardar el registro de la partida"),
        );
        self.inner.events.emit(Event::GameStarted {
            game_id,
            instance_id: instance.id.clone(),
            pid: info.pid,
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel::<(LogStream, String)>(1024);
        if let Some(out) = child.stdout.take() {
            spawn_reader(out, LogStream::Stdout, tx.clone());
        }
        if let Some(err) = child.stderr.take() {
            spawn_reader(err, LogStream::Stderr, tx.clone());
        }
        drop(tx);

        let core = self.clone();
        let pid = info.pid;
        let secret = cmd.secret.clone();
        let instance_id = instance.id.clone();
        tokio::spawn(async move {
            let started = Instant::now();
            let mut log_file = log_file;
            let mut crash_report = None;
            while let Some((stream, line)) = rx.recv().await {
                let line = mask(&line, &secret);
                if let Some(f) = log_file.as_mut() {
                    let _ = f.write_all(line.as_bytes()).await;
                    let _ = f.write_all(b"\n").await;
                }
                if let Some(path) = crate::logs::crash_report_path(&line) {
                    crash_report = Some(path);
                }
                core.inner.events.emit(Event::GameLog {
                    game_id,
                    stream,
                    line,
                });
            }
            if let Some(f) = log_file.as_mut() {
                let _ = f.flush().await;
            }
            let code = child.wait().await.ok().and_then(|s| s.code());
            let seconds = started.elapsed().as_secs();
            // Primero se cierra el registro: así otro proceso no la cuenta también.
            if let Err(e) = core.finish_launch_record(&instance_id, pid).await {
                tracing::warn!(error = %e, "no se pudo cerrar el registro de la partida");
            }
            if let Err(e) = core.record_play_session(&instance_id, seconds).await {
                tracing::warn!(error = %e, "no se pudo guardar el tiempo de juego");
            }
            core.inner.games.lock().await.remove(&game_id);
            if crash_report.is_some() || code.is_some_and(|c| c != 0) {
                core.inner.events.emit(Event::GameCrashed {
                    game_id,
                    instance_id: instance_id.clone(),
                    crash_report,
                });
            }
            core.inner.events.emit(Event::GameExited {
                game_id,
                instance_id,
                code,
                seconds_played: seconds,
            });
        });
        Ok(info)
    }
}

fn spawn_reader<R>(reader: R, stream: LogStream, tx: tokio::sync::mpsc::Sender<(LogStream, String)>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if tx.send((stream, line)).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                // Bytes no UTF-8: se ignora la línea.
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => continue,
                Err(_) => break,
            }
        }
    });
}

/// El juego no muere si se cierra el launcher o se pulsa Ctrl+C en la CLI.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

/// Rota `logs/latest.log` y abre uno nuevo; conserva los últimos `KEPT_LOGS`.
async fn open_new_log(dir: &Path) -> Result<Option<tokio::fs::File>> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| Error::io(dir, e))?;
    let latest = dir.join("latest.log");
    if latest.exists() {
        let stamp = crate::fsutil::now_secs();
        let _ = tokio::fs::rename(&latest, dir.join(format!("launch-{stamp}.log"))).await;
    }
    let mut old: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("launch-"))
                })
                .collect()
        })
        .unwrap_or_default();
    old.sort();
    while old.len() > KEPT_LOGS {
        let _ = tokio::fs::remove_file(old.remove(0)).await;
    }
    Ok(tokio::fs::File::create(&latest).await.ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::UserType;
    use crate::install::AssetsLayout;
    use crate::instance::{GameVersion, LoaderKind};
    use crate::meta::VersionInfo;
    use crate::meta::rules::{Arch, Os};
    use crate::secrets::Secret;

    fn load(path: &str) -> VersionInfo {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn instance() -> Instance {
        serde_json::from_value(serde_json::json!({
            "schema": 1, "id": "prueba", "name": "Prueba",
            "game": { "mc": "26.2", "loader": "fabric", "loader_version": "0.19.3" },
            "created_at": 0
        }))
        .unwrap()
    }

    fn installed(version: VersionInfo, virtual_assets: bool) -> Installed {
        Installed {
            client_jar: PathBuf::from("/d/versions/x/x.jar"),
            classpath: vec![
                PathBuf::from("/d/libraries/a.jar"),
                PathBuf::from("/d/libraries/b.jar"),
            ],
            natives_dir: PathBuf::from(format!("/d/natives/{}", version.id)),
            assets: AssetsLayout {
                index_name: version.asset_index.as_ref().unwrap().id.clone(),
                root: PathBuf::from("/d/assets"),
                game_assets: PathBuf::from(if virtual_assets {
                    "/d/assets/virtual/legacy"
                } else {
                    "/d/assets"
                }),
                map_to_resources: false,
            },
            version,
        }
    }

    fn session(offline: bool) -> Session {
        Session {
            uuid: "5627dd98e6be3c21b8a8e92344183641".into(),
            name: "Steve".into(),
            access_token: Secret::new(if offline { "0" } else { "TOKEN_SECRETO" }),
            user_type: if offline {
                UserType::Legacy
            } else {
                UserType::Msa
            },
        }
    }

    fn args_for(
        version: VersionInfo,
        inst: &Instance,
        server: Option<(String, u16)>,
        os: Os,
    ) -> Vec<String> {
        let inst_val = installed(version, false);
        let platform = Platform::new(os, Arch::X86_64);
        build_args(&ArgsInput {
            installed: &inst_val,
            instance: inst,
            session: &session(false),
            game_dir: Path::new("/d/instances/prueba/game"),
            libraries_dir: Path::new("/d/libraries"),
            memory_mb: 8192,
            jvm_args: vec!["-XX:+UseG1GC".into()],
            server,
            platform: &platform,
        })
    }

    #[test]
    fn fabric_26_2_linux() {
        let v = load("fabric/fabric-loader-0.19.3-26.2.json").merge_onto(load("mojang/26.2.json"));
        let mut inst = instance();
        inst.window.width = Some(1600);
        inst.window.height = Some(900);
        let args = args_for(v, &inst, Some(("mc.example.org".into(), 25565)), Os::Linux);
        let joined = args.join(" ");
        assert_eq!(&args[..2], ["-Xmx8192M", "-Xms512M"]);
        assert!(joined.contains(
            "-Dorg.lwjgl.system.SharedLibraryExtractPath=/d/natives/fabric-loader-0.19.3-26.2/lwjgl"
        ));
        assert!(joined.contains("-cp /d/libraries/a.jar:/d/libraries/b.jar:/d/versions/x/x.jar"));
        assert!(!joined.contains("-XstartOnFirstThread"));
        // Usuario después de los del JSON; clase de Fabric; resolución y quickPlay.
        let main = args
            .iter()
            .position(|a| a == "net.fabricmc.loader.impl.launch.knot.KnotClient")
            .unwrap();
        assert_eq!(args[main - 1], "-XX:+UseG1GC");
        assert!(joined.contains("--width 1600 --height 900"));
        assert!(joined.contains("--quickPlayMultiplayer mc.example.org:25565"));
        assert!(!joined.contains("--server"));
        assert!(!joined.contains("--demo"));
        assert!(!joined.contains("${"), "{joined}");
        assert!(joined.contains("--accessToken TOKEN_SECRETO"));
    }

    #[test]
    fn legacy_1_8_9_windows() {
        let v = load("mojang/1.8.9.json");
        let inst = instance();
        let args = args_for(
            v,
            &inst,
            Some(("play.example.org".into(), 25570)),
            Os::Windows,
        );
        let joined = args.join(" ");
        assert!(joined.contains("-Djava.library.path=/d/natives/1.8.9"));
        assert!(joined.contains("/d/libraries/a.jar;/d/libraries/b.jar;/d/versions/x/x.jar"));
        assert!(joined.contains("net.minecraft.client.main.Main --username Steve"));
        assert!(joined.contains("--userType msa"));
        assert!(joined.contains("--server play.example.org --port 25570"));
        assert!(!joined.contains("--width"));
        assert!(!joined.contains("${"), "{joined}");
    }

    #[test]
    fn mac_pone_xstartonfirstthread() {
        let args = args_for(load("mojang/1.20.1.json"), &instance(), None, Os::Osx);
        assert!(args.contains(&"-XstartOnFirstThread".to_owned()));
    }

    #[test]
    fn servidores() {
        assert_eq!(parse_server("mc.org").unwrap(), ("mc.org".into(), 25565));
        assert_eq!(
            parse_server("mc.org:1234").unwrap(),
            ("mc.org".into(), 1234)
        );
        assert_eq!(parse_server("[::1]:7").unwrap(), ("::1".into(), 7));
        assert!(parse_server("mc.org:x").is_err());
        assert!(parse_server("").is_err());
    }

    #[test]
    fn mascara_y_crash() {
        assert_eq!(mask("--accessToken abc123", "abc123"), "--accessToken ***");
        assert_eq!(mask("x 0 y", "0"), "x 0 y");
        assert_eq!(
            crate::logs::crash_report_path(
                "#@!@# Game crashed! Crash report saved to: #@!@# /g/crash-reports/c.txt"
            ),
            Some(PathBuf::from("/g/crash-reports/c.txt"))
        );
        let _ = GameVersion {
            mc: "1".into(),
            loader: LoaderKind::Vanilla,
            loader_version: None,
        };
    }
}
