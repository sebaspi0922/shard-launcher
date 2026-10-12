//! CLI del núcleo (docs/arquitectura.md §3.7).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use lcore::auth::mc::{Profile, SkinVariant};
use lcore::auth::{Account, AccountKind};
use lcore::backup::{BackupEntry, BackupRequest, RestoreMode};
use lcore::content::{
    ContentKind, ContentQuery, ContentVersion, InstallOutcome, InstalledContent, SortOrder, Source,
    UpdateAvailable,
};
use lcore::download::Verify;
use lcore::events::Event;
use lcore::import::minecraft::default_source_root as default_dot_minecraft;
use lcore::import::{
    DotMinecraftImport, DotMinecraftReport, ImportReport, PrismImport, PrismReport,
    RoxyImportOptions, Section,
};
use lcore::instance::{GameVersion, Instance, LoaderKind, NewInstance};
use lcore::java::JavaInstall;
use lcore::launch::LaunchOptions;
use lcore::servers::{NewServer, Server, ServerStatus};
use lcore::settings::{CloseBehavior, Settings};
use lcore::world::{Difficulty, GameMode, WorldInfo, WorldPatch};
use lcore::{Core, Error, Options, Result};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "lcore", version, about = "Núcleo del launcher de Minecraft")]
struct Cli {
    /// Directorio de datos (por defecto, el del SO o LCORE_DATA_DIR).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// No usar la red: solo lo que haya en caché.
    #[arg(long, global = true)]
    offline: bool,
    /// Salida en JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Más detalle en el log (-v, -vv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Muestra los directorios en uso.
    Paths,
    /// Ajustes globales.
    #[command(subcommand)]
    Settings(SettingsCmd),
    /// Cuentas de Microsoft y offline.
    #[command(subcommand)]
    Account(AccountCmd),
    /// Skin de la cuenta (activa o --account).
    #[command(subcommand)]
    Skin(SkinCmd),
    /// Capa de la cuenta (activa o --account).
    #[command(subcommand)]
    Cape(CapeCmd),
    /// Versiones de Minecraft.
    #[command(subcommand)]
    Version(VersionCmd),
    /// Versiones de los loaders.
    #[command(subcommand)]
    Loader(LoaderCmd),
    /// Instancias.
    #[command(subcommand)]
    Instance(InstanceCmd),
    /// Lanza una instancia y muestra su log hasta que se cierre.
    Launch(LaunchArgs),
    /// Java instalada y gestionada.
    #[command(subcommand)]
    Java(JavaCmd),
    /// Importa desde otros launchers.
    #[command(subcommand)]
    Import(ImportCmd),
    /// Copias de seguridad de una instancia.
    #[command(subcommand)]
    Backup(BackupCmd),
    /// Mundos de una instancia.
    #[command(subcommand)]
    World(WorldCmd),
    /// Datapacks de un mundo.
    #[command(subcommand)]
    Datapack(DatapackCmd),
    /// Capturas de pantalla de una instancia.
    #[command(subcommand)]
    Screenshot(ScreenshotCmd),
    /// Mods, resource packs, shaders y datapacks (Modrinth y CurseForge).
    #[command(subcommand)]
    Content(ContentCmd),
    /// Modpacks de Modrinth (.mrpack) y CurseForge (zip).
    #[command(subcommand)]
    Modpack(ModpackCmd),
    /// Servidores de acceso rápido.
    #[command(subcommand)]
    Servers(ServersCmd),
    /// Diagnostica una instancia (crash reports, logs, mods, memoria).
    Doctor {
        /// Id, nombre o prefijo de id de la instancia.
        instance: String,
        /// Aplica el arreglo del diagnóstico con ese id.
        #[arg(long, value_name = "ID")]
        fix: Option<String>,
    },
}

#[derive(Subcommand)]
enum ContentCmd {
    /// Busca proyectos.
    Search {
        text: String,
        /// Tipo de contenido.
        #[arg(long = "type", value_enum, default_value = "mod")]
        kind: TypeArg,
        #[arg(long)]
        mc: Option<String>,
        #[arg(long)]
        loader: Option<String>,
        /// Toma la versión de Minecraft y el loader de esta instancia.
        #[arg(long)]
        instance: Option<String>,
        #[arg(long, value_enum, default_value = "modrinth")]
        source: SourceArg,
        #[arg(long, value_enum, default_value = "relevance")]
        sort: SortArg,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Versiones compatibles de un proyecto.
    Versions {
        project: String,
        #[arg(long)]
        instance: Option<String>,
        #[arg(long)]
        mc: Option<String>,
        #[arg(long)]
        loader: Option<String>,
        #[arg(long, value_enum, default_value = "modrinth")]
        source: SourceArg,
    },
    /// Instala un proyecto y sus dependencias obligatorias.
    Install {
        instance: String,
        project: String,
        /// Id de versión concreta (por defecto, la más nueva compatible).
        #[arg(long)]
        version: Option<String>,
        /// Mundo destino (obligatorio para datapacks).
        #[arg(long)]
        world: Option<String>,
        #[arg(long, value_enum, default_value = "modrinth")]
        source: SourceArg,
    },
    /// Contenido de la instancia (disco + procedencia).
    List { instance: String },
    /// Activa un archivo.
    Enable { instance: String, file: String },
    /// Desactiva un archivo (`.disabled`).
    Disable { instance: String, file: String },
    /// Borra un archivo.
    Remove { instance: String, file: String },
    /// Busca actualizaciones en Modrinth (con --apply las instala).
    Updates {
        instance: String,
        #[arg(long)]
        apply: bool,
    },
    /// Reconoce por SHA-1 los archivos de origen desconocido.
    Identify { instance: String },
}

#[derive(Subcommand)]
enum ModpackCmd {
    /// Instala un .mrpack o un zip de CurseForge (ruta o URL) en una instancia nueva.
    Install {
        source: String,
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SourceArg {
    Modrinth,
    Curseforge,
}

impl From<SourceArg> for Source {
    fn from(s: SourceArg) -> Self {
        match s {
            SourceArg::Modrinth => Source::Modrinth,
            SourceArg::Curseforge => Source::CurseForge,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum TypeArg {
    Mod,
    Resourcepack,
    Shader,
    Datapack,
    Modpack,
}

impl From<TypeArg> for ContentKind {
    fn from(t: TypeArg) -> Self {
        match t {
            TypeArg::Mod => ContentKind::Mod,
            TypeArg::Resourcepack => ContentKind::ResourcePack,
            TypeArg::Shader => ContentKind::Shader,
            TypeArg::Datapack => ContentKind::Datapack,
            TypeArg::Modpack => ContentKind::Modpack,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SortArg {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<SortArg> for SortOrder {
    fn from(s: SortArg) -> Self {
        match s {
            SortArg::Relevance => SortOrder::Relevance,
            SortArg::Downloads => SortOrder::Downloads,
            SortArg::Follows => SortOrder::Follows,
            SortArg::Newest => SortOrder::Newest,
            SortArg::Updated => SortOrder::Updated,
        }
    }
}

#[derive(Subcommand)]
enum JavaCmd {
    /// Java gestionada, elegida a mano y del sistema.
    List,
    /// Busca una Java para esa versión mayor.
    Find { major: u32 },
    /// Instala un JRE de Adoptium en el directorio de datos.
    Install { major: u32 },
}

#[derive(Subcommand)]
enum ServersCmd {
    /// Lista los servidores guardados.
    List,
    /// Añade un servidor (host o host:puerto).
    Add {
        address: String,
        /// Nombre; por defecto, la dirección.
        #[arg(long)]
        name: Option<String>,
        /// Instancia con la que se entra (id, nombre o prefijo de id).
        #[arg(long)]
        instance: Option<String>,
    },
    /// Quita un servidor (id, prefijo de id o nombre).
    Remove { server: String },
    /// Mueve un servidor a una posición (empieza en 0).
    Move { server: String, index: usize },
    /// Consulta el estado de una dirección o, sin dirección, de toda la lista.
    Ping { address: Option<String> },
    /// Añade los servidores de servers.dat de una instancia.
    Import { instance: String },
}

#[derive(Subcommand)]
enum ImportCmd {
    /// Importa un directorio .minecraft del launcher oficial como instancia (copia).
    DotMinecraft {
        /// Carpeta .minecraft (por defecto, la del sistema).
        path: Option<PathBuf>,
        /// Nombre de la instancia (por defecto, «Minecraft <versión>»).
        #[arg(long)]
        name: Option<String>,
        /// Versión instalada, perfil o versión de Minecraft (por defecto, la del último perfil usado).
        #[arg(long, value_name = "ID")]
        version: Option<String>,
        /// Fuerza el loader (vanilla, fabric, quilt, forge, neoforge).
        #[arg(long)]
        loader: Option<String>,
        /// Qué copiar, separado por comas: saves, mods, config, resourcepacks,
        /// shaderpacks, screenshots, options, servers (por defecto, todo).
        #[arg(long, value_delimiter = ',', value_name = "LISTA")]
        sections: Vec<String>,
        /// Muestra qué se importaría sin escribir nada.
        #[arg(long)]
        dry_run: bool,
    },
    /// Importa instancias de Prism Launcher o MultiMC (una carpeta de instancia o la raíz).
    Prism {
        path: PathBuf,
        /// Solo esta instancia (nombre o carpeta); se puede repetir.
        #[arg(long, value_name = "NOMBRE")]
        instance: Vec<String>,
        /// Muestra qué se importaría sin escribir nada.
        #[arg(long)]
        dry_run: bool,
        /// Reimporta instancias ya importadas (con id nuevo).
        #[arg(long)]
        force: bool,
    },
    /// Importa instancias y contenido desde Roxy Launcher (solo lectura sobre el origen).
    Roxy {
        /// Raíz de datos de Roxy (por defecto ~/.local/share/.roxy).
        #[arg(long)]
        source: Option<PathBuf>,
        /// Muestra qué se importaría sin escribir nada.
        #[arg(long)]
        dry_run: bool,
        /// Incluye las instancias de la papelera de Roxy.
        #[arg(long)]
        include_trash: bool,
        /// No importa los backups.
        #[arg(long)]
        skip_backups: bool,
        /// No copia logs/ ni crash-reports/.
        #[arg(long)]
        skip_logs: bool,
        /// Reimporta instancias ya importadas (con id nuevo).
        #[arg(long)]
        force: bool,
        /// No importa librerías, assets, jars ni Java (se descargarán de nuevo).
        #[arg(long)]
        no_link: bool,
    },
}

#[derive(Subcommand)]
enum BackupCmd {
    /// Crea un backup ahora (el juego debe estar cerrado).
    Create {
        instance: String,
        #[arg(long)]
        label: Option<String>,
        /// Lo fija: nunca se borra solo.
        #[arg(long)]
        pin: bool,
    },
    List {
        instance: String,
    },
    /// Restaura un backup (sustituye game/, o solo saves/ con --worlds-only).
    Restore {
        instance: String,
        backup_id: String,
        #[arg(long)]
        worlds_only: bool,
    },
    Delete {
        instance: String,
        backup_id: String,
    },
    /// Fija un backup para que la poda automática no lo borre.
    Pin {
        instance: String,
        backup_id: String,
    },
    Unpin {
        instance: String,
        backup_id: String,
    },
    /// Configura los backups automáticos de la instancia.
    Auto {
        instance: String,
        #[arg(long, conflicts_with = "disable")]
        enable: bool,
        #[arg(long)]
        disable: bool,
        /// Horas entre copias (24 si no se fija).
        #[arg(long)]
        interval_hours: Option<u32>,
        /// Copias automáticas que se conservan (5 si no se fija).
        #[arg(long)]
        max: Option<u32>,
    },
    /// Una pasada del planificador: crea los backups automáticos que toquen.
    RunScheduler,
}

#[derive(Subcommand)]
enum WorldCmd {
    List {
        instance: String,
    },
    /// Cambia el nombre que se ve en el juego (la carpeta no se renombra).
    Rename {
        instance: String,
        folder: String,
        name: String,
    },
    /// Cambia ajustes del mundo (con el juego cerrado).
    Set {
        instance: String,
        folder: String,
        #[arg(long)]
        mode: Option<ModeArg>,
        #[arg(long)]
        difficulty: Option<DifficultyArg>,
        /// true | false
        #[arg(long)]
        hardcore: Option<String>,
        /// true | false (permitir comandos)
        #[arg(long)]
        cheats: Option<String>,
    },
    /// Borra el mundo para siempre.
    Delete {
        instance: String,
        folder: String,
        /// Confirma el borrado definitivo.
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Clone, Copy, ValueEnum)]
enum DifficultyArg {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Subcommand)]
enum DatapackCmd {
    List {
        instance: String,
        world: String,
    },
    Enable {
        instance: String,
        world: String,
        name: String,
    },
    Disable {
        instance: String,
        world: String,
        name: String,
    },
    Remove {
        instance: String,
        world: String,
        name: String,
    },
}

#[derive(Subcommand)]
enum ScreenshotCmd {
    /// Capturas de la instancia, las más nuevas primero.
    List { instance: String },
}

#[derive(Subcommand)]
enum VersionCmd {
    /// Lista las versiones (solo releases salvo --snapshots o --all).
    List {
        #[arg(long)]
        snapshots: bool,
        #[arg(long)]
        all: bool,
    },
    /// Descarga una versión (con su loader si es un perfil instalado).
    Install {
        id: String,
        /// Comprueba el hash de todos los archivos.
        #[arg(long)]
        verify: bool,
    },
}

#[derive(Subcommand)]
enum LoaderCmd {
    /// Versiones de un loader para una versión de Minecraft.
    Versions { loader: String, mc: String },
}

#[derive(Subcommand)]
enum InstanceCmd {
    List,
    Create {
        name: String,
        #[arg(long)]
        mc: String,
        #[arg(long, default_value = "vanilla")]
        loader: String,
        #[arg(long)]
        loader_version: Option<String>,
    },
    Show {
        instance: String,
    },
    /// Cambia un ajuste de la instancia (`default` vuelve al valor global).
    Set {
        instance: String,
        key: InstanceKey,
        value: String,
    },
    /// Manda la instancia a la papelera (30 días).
    Delete {
        instance: String,
    },
    /// Instancias en la papelera.
    Trash,
    Restore {
        trash_id: String,
    },
    /// Borra definitivamente una instancia de la papelera.
    Purge {
        trash_id: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum InstanceKey {
    Name,
    MemoryMb,
    JvmArgs,
    Width,
    Height,
    Fullscreen,
    JavaPath,
    DevConsole,
    Mc,
    Loader,
    LoaderVersion,
}

#[derive(clap::Args)]
struct LaunchArgs {
    instance: String,
    #[arg(long)]
    account: Option<String>,
    /// Entra directamente a un servidor (host[:puerto]).
    #[arg(long)]
    server: Option<String>,
    /// Comprueba el hash de todos los archivos antes de lanzar.
    #[arg(long)]
    verify: bool,
    /// Solo instala y muestra el comando (con el token oculto).
    #[arg(long)]
    print: bool,
    /// Vuelve en cuanto arranca el juego.
    #[arg(long)]
    detach: bool,
}

#[derive(Subcommand)]
enum SettingsCmd {
    Show,
    /// Cambia un ajuste.
    Set {
        key: SettingKey,
        value: String,
    },
    /// Guarda la clave de API de CurseForge en el keyring (se lee de stdin).
    SetCurseforgeKey,
    /// Borra la clave de API de CurseForge del keyring.
    ClearCurseforgeKey,
}

#[derive(Clone, Copy, ValueEnum)]
enum SettingKey {
    MsaClientId,
    Language,
    CloseBehavior,
    AllowMultipleGames,
    DefaultMemoryMb,
    MaxConcurrentDownloads,
    DiscordRpc,
}

#[derive(Subcommand)]
enum AccountCmd {
    List,
    /// Inicia sesión con Microsoft (device code).
    Login,
    /// Añade una cuenta offline.
    AddOffline {
        name: String,
    },
    /// Cambia la cuenta activa.
    Use {
        account: String,
    },
    Remove {
        account: String,
    },
    /// Renueva la sesión de una cuenta de Microsoft.
    Refresh {
        account: Option<String>,
    },
    /// Perfil, skins y capas.
    Profile {
        account: Option<String>,
    },
}

#[derive(Subcommand)]
enum SkinCmd {
    /// Usa una skin desde una URL pública.
    SetUrl {
        url: String,
        #[arg(long)]
        slim: bool,
        #[arg(long)]
        account: Option<String>,
    },
    /// Sube una skin PNG.
    Upload {
        file: PathBuf,
        #[arg(long)]
        slim: bool,
        #[arg(long)]
        account: Option<String>,
    },
    /// Vuelve a la skin por defecto.
    Reset {
        #[arg(long)]
        account: Option<String>,
    },
}

#[derive(Subcommand)]
enum CapeCmd {
    Use {
        cape_id: String,
        #[arg(long)]
        account: Option<String>,
    },
    Hide {
        #[arg(long)]
        account: Option<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    // Que `lcore … | head` termine en silencio en vez de abortar.
    #[cfg(unix)]
    // SAFETY: se restaura la acción por defecto de SIGPIPE antes de crear hilos de E/S.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    let json = cli.json;
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if json {
                let body = serde_json::json!({ "error": e.kind(), "message": e.to_string() });
                println!("{body}");
            } else {
                eprintln!("error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info,lcore=debug",
        _ => "debug",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

async fn run(cli: Cli) -> Result<()> {
    let offline = cli.offline;
    let core = Core::open(Options {
        data_dir: cli.data_dir,
        offline: cli.offline,
        ..Options::default()
    })
    .await?;
    let out = Out { json: cli.json };

    match cli.command {
        Command::Paths => {
            let p = core.paths();
            out.value(
                &serde_json::json!({
                    "root": p.root(),
                    "settings": p.settings_file(),
                    "accounts": p.accounts_file(),
                    "instances": p.instances_dir(),
                }),
                || println!("{}", p.root().display()),
            );
        }
        Command::Settings(SettingsCmd::Show) => {
            let s = core.settings().await;
            out.value(&s, || print_settings(&s));
        }
        Command::Settings(SettingsCmd::Set { key, value }) => {
            let parsed = parse_setting(key, &value)?;
            let s = core.update_settings(|s| parsed(s)).await?;
            out.value(&s, || println!("Guardado."));
        }
        Command::Settings(SettingsCmd::SetCurseforgeKey) => {
            // La clave solo se lee de stdin: nunca de argv ni se muestra.
            let mut key = String::new();
            std::io::stdin()
                .read_line(&mut key)
                .map_err(|e| Error::InvalidInput(format!("no se pudo leer stdin: {e}")))?;
            core.set_curseforge_key(&key).await?;
            out.value(&serde_json::json!({ "saved": true }), || {
                println!("Clave de CurseForge guardada.")
            });
        }
        Command::Settings(SettingsCmd::ClearCurseforgeKey) => {
            core.clear_curseforge_key().await?;
            out.value(&serde_json::json!({ "cleared": true }), || {
                println!("Clave de CurseForge borrada.")
            });
        }
        Command::Account(cmd) => account(&core, &out, cmd).await?,
        Command::Skin(cmd) => {
            let profile = match cmd {
                SkinCmd::SetUrl { url, slim, account } => {
                    core.set_skin_url(account.as_deref(), &url, variant(slim))
                        .await?
                }
                SkinCmd::Upload {
                    file,
                    slim,
                    account,
                } => {
                    core.upload_skin(account.as_deref(), &file, variant(slim))
                        .await?
                }
                SkinCmd::Reset { account } => core.reset_skin(account.as_deref()).await?,
            };
            out.value(&profile, || print_profile(&profile));
        }
        Command::Cape(cmd) => {
            let profile = match cmd {
                CapeCmd::Use { cape_id, account } => {
                    core.show_cape(account.as_deref(), &cape_id).await?
                }
                CapeCmd::Hide { account } => core.hide_cape(account.as_deref()).await?,
            };
            out.value(&profile, || print_profile(&profile));
        }
        Command::Version(cmd) => version(&core, &out, cmd).await?,
        Command::Loader(LoaderCmd::Versions { loader, mc }) => {
            let kind = parse_loader(&loader)?;
            let list = core.loader_versions(kind, &mc).await?;
            out.value(&list, || {
                for v in list.iter().take(30) {
                    let mark = if v.stable { "" } else { " (inestable)" };
                    println!("{}{mark}", v.version);
                }
            });
        }
        Command::Instance(cmd) => instance(&core, &out, cmd).await?,
        Command::Launch(args) => launch(&core, &out, args).await?,
        Command::Content(cmd) => content(&core, &out, cmd).await?,
        Command::Modpack(ModpackCmd::Install { source, name }) => {
            let progress = spawn_progress(&core, out.json);
            let result = core.install_modpack(&source, name).await;
            progress.abort();
            let result = result?;
            out.value(&result, || {
                eprintln!();
                let i = &result.instance;
                println!(
                    "Instancia {} creada ({} {} {}): {} archivos descargados, {} de overrides.",
                    i.id,
                    i.game.mc,
                    i.game.loader.as_str(),
                    i.game.loader_version.as_deref().unwrap_or(""),
                    result.files_downloaded,
                    result.overrides_extracted
                );
                for w in &result.warnings {
                    eprintln!("aviso: {w}");
                }
                println!(
                    "Instala el juego y el loader con `lcore launch {} --print`.",
                    i.id
                );
            });
        }
        Command::Java(cmd) => java(&core, &out, cmd).await?,
        Command::Backup(cmd) => backup(&core, &out, cmd).await?,
        Command::World(cmd) => world(&core, &out, cmd).await?,
        Command::Datapack(cmd) => datapack(&core, &out, cmd).await?,
        Command::Screenshot(ScreenshotCmd::List { instance }) => {
            let id = core.find_instance(&instance).await?.id;
            let list = core.list_screenshots(&id).await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No hay capturas.");
                }
                for s in &list {
                    println!(
                        "{}  {:>9}  {}",
                        s.modified.map_or_else(|| "-".into(), fmt_utc),
                        fmt_bytes(s.size),
                        s.file
                    );
                }
            });
        }
        Command::Doctor { instance, fix } => doctor(&core, &out, &instance, fix.as_deref()).await?,
        Command::Servers(cmd) => servers(&core, &out, cmd, offline).await?,
        Command::Import(ImportCmd::DotMinecraft {
            path,
            name,
            version,
            loader,
            sections,
            dry_run,
        }) => {
            let sections = sections
                .iter()
                .map(|s| {
                    Section::parse(s)
                        .ok_or_else(|| Error::InvalidInput(format!("sección desconocida: {s}")))
                })
                .collect::<Result<Vec<_>>>()?;
            let opts = DotMinecraftImport {
                source: path.unwrap_or_else(default_dot_minecraft),
                name,
                mc_version: version,
                loader: loader.as_deref().map(parse_loader).transpose()?,
                sections,
                dry_run,
            };
            let progress = (!out.json && !dry_run).then(|| spawn_progress(&core, false));
            let report = core.import_dot_minecraft(&opts).await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let report = report?;
            out.value(&report, || print_dot_minecraft(&report, dry_run));
        }
        Command::Import(ImportCmd::Prism {
            path,
            instance,
            dry_run,
            force,
        }) => {
            let opts = PrismImport {
                source: path,
                instances: instance,
                dry_run,
                force,
            };
            let progress = (!out.json && !dry_run).then(|| spawn_progress(&core, false));
            let report = core.import_prism(&opts).await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let report = report?;
            out.value(&report, || print_prism(&report, dry_run));
        }
        Command::Import(ImportCmd::Roxy {
            source,
            dry_run,
            include_trash,
            skip_backups,
            skip_logs,
            force,
            no_link,
        }) => {
            let mut opts = RoxyImportOptions {
                dry_run,
                include_trash,
                skip_backups,
                skip_logs,
                force,
                link_shared: !no_link,
                ..RoxyImportOptions::default()
            };
            if let Some(source) = source {
                opts.source_root = source;
            }
            let progress = (!out.json && !dry_run).then(|| spawn_progress(&core, false));
            let report = core.import_roxy(&opts).await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let report = report?;
            out.value(&report, || print_import(&report, dry_run));
        }
    }
    Ok(())
}

/// `--mc` / `--loader` explícitos mandan sobre los de la instancia.
async fn mc_and_loader(
    core: &Core,
    instance: Option<&str>,
    mc: Option<String>,
    loader: Option<&str>,
) -> Result<(Option<String>, Option<LoaderKind>)> {
    let mut mc_out = mc;
    let mut loader_out = loader.map(parse_loader).transpose()?;
    if let Some(q) = instance {
        let inst = core.find_instance(q).await?;
        mc_out = mc_out.or(Some(inst.game.mc));
        loader_out = loader_out.or(Some(inst.game.loader));
    }
    Ok((mc_out, loader_out))
}

async fn content(core: &Core, out: &Out, cmd: ContentCmd) -> Result<()> {
    match cmd {
        ContentCmd::Search {
            text,
            kind,
            mc,
            loader,
            instance,
            source,
            sort,
            limit,
            offset,
        } => {
            let kind: ContentKind = kind.into();
            let (mc, loader) =
                mc_and_loader(core, instance.as_deref(), mc, loader.as_deref()).await?;
            let page = core
                .search_content(&ContentQuery {
                    source: source.into(),
                    kind,
                    text,
                    mc,
                    loader: loader.filter(|_| kind.uses_loader()),
                    offset,
                    limit,
                    sort: sort.into(),
                })
                .await?;
            out.value(&page, || {
                if page.items.is_empty() {
                    println!("Sin resultados.");
                }
                for h in &page.items {
                    println!(
                        "{:<10} {:<28} {:>10} desc.  {} — {}",
                        h.id,
                        h.slug,
                        h.downloads,
                        h.title,
                        truncate(&h.description, 70)
                    );
                }
                println!(
                    "{} de {} (desde {})",
                    page.items.len(),
                    page.total,
                    page.offset
                );
            });
        }
        ContentCmd::Versions {
            project,
            instance,
            mc,
            loader,
            source,
        } => {
            let source: Source = source.into();
            let (mc, loader) =
                mc_and_loader(core, instance.as_deref(), mc, loader.as_deref()).await?;
            // El loader solo filtra mods y modpacks.
            let loader = match loader {
                Some(l) => {
                    let p = core.content().project(source, &project).await?;
                    p.kind.is_none_or(|k| k.uses_loader()).then_some(l)
                }
                None => None,
            };
            let versions = core
                .content_versions(source, &project, mc.as_deref(), loader)
                .await?;
            out.value(&versions, || {
                if versions.is_empty() {
                    println!("No hay versiones compatibles.");
                }
                for v in &versions {
                    print_version(v);
                }
            });
        }
        ContentCmd::Install {
            instance,
            project,
            version,
            world,
            source,
        } => {
            let progress = spawn_progress(core, out.json);
            let result = core
                .install_content(
                    &instance,
                    source.into(),
                    &project,
                    version.as_deref(),
                    world.as_deref(),
                )
                .await;
            progress.abort();
            let outcome = result?;
            out.value(&outcome, || print_outcome(&outcome));
        }
        ContentCmd::List { instance } => {
            let list = core.list_content(&instance).await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("La instancia no tiene contenido.");
                }
                for c in &list {
                    print_installed(c);
                }
            });
        }
        ContentCmd::Enable { instance, file } => {
            let item = core.set_content_enabled(&instance, &file, true).await?;
            out.value(&item, || println!("Activado: {}", item.file));
        }
        ContentCmd::Disable { instance, file } => {
            let item = core.set_content_enabled(&instance, &file, false).await?;
            out.value(&item, || println!("Desactivado: {}", item.file));
        }
        ContentCmd::Remove { instance, file } => {
            let item = core.remove_content(&instance, &file).await?;
            out.value(&item, || println!("Borrado: {}", item.file));
        }
        ContentCmd::Updates { instance, apply } => {
            let updates = core.check_content_updates(&instance).await?;
            if !apply {
                out.value(&updates, || {
                    if updates.is_empty() {
                        println!("Todo está al día.");
                    }
                    for u in &updates {
                        print_update(u);
                    }
                });
                return Ok(());
            }
            let mut done = Vec::new();
            for u in &updates {
                done.push(core.content().apply_update(&instance, u).await?);
            }
            out.value(&done, || {
                if done.is_empty() {
                    println!("Todo está al día.");
                }
                for d in &done {
                    println!(
                        "Actualizado: {} ({} -> {})",
                        d.file,
                        d.from_version.as_deref().unwrap_or("?"),
                        d.to_version
                    );
                }
            });
        }
        ContentCmd::Identify { instance } => {
            let report = core.identify_content(&instance).await?;
            out.value(&report, || {
                for i in &report.identified {
                    println!("{:<40} {} {}", i.file, i.project_name, i.version_number);
                }
                println!(
                    "{} reconocidos, {} sin reconocer.",
                    report.identified.len(),
                    report.unmatched.len()
                );
            });
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let line = s.lines().next().unwrap_or("");
    if line.chars().count() <= max {
        line.to_owned()
    } else {
        let cut: String = line.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

fn print_version(v: &ContentVersion) {
    let mc = v
        .mc_versions
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{:<12} {:<22} {:?} {} [{}]",
        v.id,
        v.version_number,
        v.release,
        mc,
        v.loaders.join(",")
    );
}

fn print_outcome(o: &InstallOutcome) {
    eprintln!();
    for i in &o.installed {
        let how = if i.dependency { " (dependencia)" } else { "" };
        let present = if i.already_present {
            " (ya estaba)"
        } else {
            ""
        };
        println!(
            "Instalado: {} {} -> {}{how}{present}",
            i.project_name, i.version_number, i.file
        );
    }
    for s in &o.skipped {
        println!(
            "Omitido: {} ({})",
            s.project_name.as_deref().unwrap_or(&s.project_id),
            s.reason
        );
    }
    for w in &o.warnings {
        eprintln!("aviso: {w}");
    }
}

fn print_installed(c: &InstalledContent) {
    let state = if c.enabled { "on " } else { "off" };
    println!(
        "{:<12} {state} {:<10} {}  {} {}",
        c.kind.as_str(),
        c.source,
        c.file,
        c.project_name.as_deref().unwrap_or(""),
        c.version_number.as_deref().unwrap_or("")
    );
}

fn print_update(u: &UpdateAvailable) {
    println!(
        "{:<12} {}  {} -> {}",
        u.kind.as_str(),
        u.file,
        u.current_version.as_deref().unwrap_or("?"),
        u.latest.version_number
    );
}

async fn version(core: &Core, out: &Out, cmd: VersionCmd) -> Result<()> {
    match cmd {
        VersionCmd::List { snapshots, all } => {
            let m = core.version_manifest().await?;
            let list: Vec<_> = m
                .versions
                .iter()
                .filter(|v| all || v.kind == "release" || (snapshots && v.kind == "snapshot"))
                .collect();
            out.value(&list, || {
                println!(
                    "Última: {} (snapshot {})",
                    m.latest.release, m.latest.snapshot
                );
                for v in list.iter().take(40) {
                    println!("{:<24} {}", v.id, v.kind);
                }
            });
        }
        VersionCmd::Install { id, verify } => {
            let progress = spawn_progress(core, out.json);
            let verify = if verify { Verify::Full } else { Verify::Size };
            let installed = core.install_version(&id, verify).await;
            progress.abort();
            let installed = installed?;
            let summary = serde_json::json!({
                "id": installed.version.id,
                "java": installed.version.java_major(),
                "libraries": installed.classpath.len(),
            });
            out.value(&summary, || {
                eprintln!();
                println!(
                    "{} instalada ({} librerías, Java {}).",
                    installed.version.id,
                    installed.classpath.len(),
                    installed.version.java_major()
                )
            });
        }
    }
    Ok(())
}

async fn instance(core: &Core, out: &Out, cmd: InstanceCmd) -> Result<()> {
    match cmd {
        InstanceCmd::List => {
            // Suma el tiempo de partidas que terminaron con el launcher cerrado.
            if let Err(e) = core.reconcile_sessions().await {
                tracing::warn!(error = %e, "no se pudieron revisar las partidas");
            }
            let list = core.list_instances().await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No hay instancias. Crea una con `lcore instance create`.");
                }
                for i in &list {
                    println!(
                        "{:<28} {:<24} {} {} {}",
                        i.id,
                        i.name,
                        i.game.mc,
                        i.game.loader.as_str(),
                        i.game.loader_version.as_deref().unwrap_or("")
                    );
                }
            });
        }
        InstanceCmd::Create {
            name,
            mc,
            loader,
            loader_version,
        } => {
            let game = GameVersion {
                mc,
                loader: parse_loader(&loader)?,
                loader_version,
            };
            let i = core.create_instance(NewInstance { name, game }).await?;
            out.value(&i, || println!("Instancia {} creada.", i.id));
        }
        InstanceCmd::Show { instance } => {
            let i = core.find_instance(&instance).await?;
            out.value(&i, || {
                println!("{} ({})", i.name, i.id);
                println!(
                    "  versión:   {} {} {}",
                    i.game.mc,
                    i.game.loader.as_str(),
                    i.game.loader_version.as_deref().unwrap_or("")
                );
                println!(
                    "  memoria:   {}",
                    i.java
                        .memory_mb
                        .map_or("global".into(), |m| format!("{m} MB"))
                );
                println!(
                    "  jvm:       {}",
                    i.java
                        .jvm_args
                        .as_ref()
                        .map_or("globales".into(), |a| a.join(" "))
                );
                println!("  jugado:    {} min", i.playtime_seconds / 60);
                if let Ok(dir) = core.instance_game_dir(&i.id) {
                    println!("  carpeta:   {}", dir.display());
                }
            });
        }
        InstanceCmd::Set {
            instance,
            key,
            value,
        } => {
            let i = core.find_instance(&instance).await?;
            let apply = parse_instance_setting(key, &value)?;
            let i = core.update_instance(&i.id, apply).await?;
            out.value(&i, || println!("Guardado."));
        }
        InstanceCmd::Delete { instance } => {
            let i = core.find_instance(&instance).await?;
            let t = core.delete_instance(&i.id).await?;
            out.value(&t, || {
                println!("{} enviada a la papelera ({}).", i.name, t.trash_id)
            });
        }
        InstanceCmd::Trash => {
            let list = core.list_trash().await?;
            out.value(&list, || {
                for t in &list {
                    println!("{:<40} {}", t.trash_id, t.instance.name);
                }
            });
        }
        InstanceCmd::Restore { trash_id } => {
            let i = core.restore_instance(&trash_id).await?;
            out.value(&i, || println!("Instancia {} recuperada.", i.id));
        }
        InstanceCmd::Purge { trash_id } => {
            core.purge_instance(&trash_id).await?;
            out.value(&serde_json::json!({ "purged": trash_id }), || {
                println!("Borrada.")
            });
        }
    }
    Ok(())
}

async fn launch(core: &Core, out: &Out, args: LaunchArgs) -> Result<()> {
    let instance = core.find_instance(&args.instance).await?;
    let opts = LaunchOptions {
        account: args.account,
        server: args.server,
        verify_files: args.verify,
        detached: args.detach,
    };
    let progress = spawn_progress(core, out.json);
    if args.print {
        let result = core.prepare_launch(&instance.id, &opts).await;
        progress.abort();
        let (_, cmd) = result?;
        out.value(
            &serde_json::json!({ "java": cmd.java, "args": cmd.masked_args(), "cwd": cmd.cwd }),
            || {
                eprintln!();
                println!("{} {}", cmd.java.display(), cmd.masked_args().join(" "));
            },
        );
        return Ok(());
    }
    let mut events = core.subscribe();
    let started = core.launch(&instance.id, opts).await;
    progress.abort();
    let info = started?;
    if !out.json {
        eprintln!();
        eprintln!(
            "Juego iniciado (pid {}).",
            info.pid.map_or("?".into(), |p| p.to_string())
        );
    }
    if args.detach {
        out.value(&info, || {});
        return Ok(());
    }
    loop {
        tokio::select! {
            ev = events.recv() => match ev {
                Ok(Event::GameLog { game_id, line, .. }) if game_id == info.game_id => {
                    if !out.json { println!("{line}"); }
                }
                Ok(Event::GameExited { game_id, code, seconds_played, .. }) if game_id == info.game_id => {
                    let summary = serde_json::json!({ "code": code, "seconds_played": seconds_played });
                    out.value(&summary, || {
                        eprintln!("Juego cerrado (código {}, {} min).", code.map_or("?".into(), |c| c.to_string()), seconds_played / 60)
                    });
                    return Ok(());
                }
                Ok(_) => {}
                // Incluye el progreso de la instalación acumulado antes del arranque.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!(n, "eventos omitidos");
                }
                Err(_) => return Ok(()),
            },
            _ = tokio::signal::ctrl_c() => {
                eprintln!("La CLI se cierra; el juego sigue abierto.");
                return Ok(());
            }
        }
    }
}

/// Imprime el progreso de descargas en stderr.
fn spawn_progress(core: &Core, quiet: bool) -> tokio::task::JoinHandle<()> {
    let mut rx = core.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            if let Event::Progress {
                stage, done, total, ..
            } = ev
                && !quiet
            {
                eprint!("\r{stage}: {done}/{total}      ");
            }
        }
    })
}

fn parse_loader(s: &str) -> Result<LoaderKind> {
    LoaderKind::parse(s).ok_or_else(|| Error::InvalidInput(format!("loader desconocido: {s}")))
}

fn parse_instance_setting(key: InstanceKey, value: &str) -> Result<InstanceApply> {
    let v = value.trim().to_owned();
    let default = v == "default" || v.is_empty();
    let bad = |what: &str| Error::InvalidInput(format!("valor no válido para {what}: {v:?}"));
    let num = |what: &str| -> Result<Option<u32>> {
        if default {
            Ok(None)
        } else {
            v.parse().map(Some).map_err(|_| bad(what))
        }
    };
    Ok(match key {
        InstanceKey::Name => {
            if default {
                return Err(bad("name"));
            }
            Box::new(move |i| i.name = v)
        }
        InstanceKey::MemoryMb => {
            let n = num("memory-mb")?;
            if n.is_some_and(|n| n < 512) {
                return Err(bad("memory-mb"));
            }
            Box::new(move |i| i.java.memory_mb = n)
        }
        InstanceKey::JvmArgs => {
            let args = (!default).then(|| v.split_whitespace().map(str::to_owned).collect());
            Box::new(move |i| i.java.jvm_args = args)
        }
        InstanceKey::Width => {
            let n = num("width")?;
            Box::new(move |i| i.window.width = n)
        }
        InstanceKey::Height => {
            let n = num("height")?;
            Box::new(move |i| i.window.height = n)
        }
        InstanceKey::Fullscreen => {
            let b = parse_bool(&v).ok_or_else(|| bad("fullscreen"))?;
            Box::new(move |i| i.window.fullscreen = b)
        }
        InstanceKey::JavaPath => {
            let p = (!default).then(|| PathBuf::from(&v));
            Box::new(move |i| i.java.path = p)
        }
        InstanceKey::DevConsole => {
            let b = parse_bool(&v).ok_or_else(|| bad("dev-console"))?;
            Box::new(move |i| i.dev_console = b)
        }
        InstanceKey::Mc => {
            if default {
                return Err(bad("mc"));
            }
            Box::new(move |i| i.game.mc = v)
        }
        InstanceKey::Loader => {
            let l = parse_loader(&v)?;
            Box::new(move |i| {
                i.game.loader = l;
                i.game.loader_version = None;
            })
        }
        InstanceKey::LoaderVersion => {
            let lv = (!default).then_some(v);
            Box::new(move |i| i.game.loader_version = lv)
        }
    })
}

async fn java(core: &Core, out: &Out, cmd: JavaCmd) -> Result<()> {
    match cmd {
        JavaCmd::List => {
            let list = core.list_java().await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No se encontró ninguna Java. Usa `lcore java install <versión>`.");
                }
                for j in &list {
                    print_java(j);
                }
            });
        }
        JavaCmd::Find { major } => {
            let found = core.find_java(major).await?;
            out.value(&found, || match &found {
                Some(j) => print_java(j),
                None => println!("No hay una Java adecuada para la versión {major}."),
            });
        }
        JavaCmd::Install { major } => {
            let show = !out.json && std::io::IsTerminal::is_terminal(&std::io::stderr());
            let progress = show.then(|| {
                let mut events = core.subscribe();
                tokio::spawn(async move {
                    let mut last = u64::MAX;
                    while let Ok(event) = events.recv().await {
                        if let Event::Progress { done, total, .. } = event
                            && total > 0
                        {
                            let pct = done * 100 / total;
                            if pct != last {
                                last = pct;
                                eprint!("\rDescargando Java {major}… {pct}%");
                            }
                        }
                    }
                })
            });
            let result = core.install_java(major).await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let install = result?;
            out.value(&install, || {
                println!("Java {major} instalada en {}.", install.path.display())
            });
        }
    }
    Ok(())
}

fn print_java(j: &JavaInstall) {
    println!(
        "{:<4} {:<14} {:<9} {}",
        j.major,
        j.version,
        format!("{:?}", j.source).to_lowercase(),
        j.path.display()
    );
}

async fn doctor(core: &Core, out: &Out, query: &str, fix: Option<&str>) -> Result<()> {
    let instance = core.find_instance(query).await?;
    let report = core.diagnose(&instance.id).await?;
    let Some(fix_id) = fix else {
        out.value(&report, || {
            print_report(&instance.name, &instance.id, &report)
        });
        return Ok(());
    };
    let diagnosis = report
        .diagnoses
        .iter()
        .find(|d| d.id == fix_id)
        .ok_or_else(|| {
            Error::InvalidInput(format!("no hay ningún diagnóstico con id {fix_id:?}"))
        })?;
    let action = diagnosis.fix.as_ref().ok_or_else(|| {
        Error::InvalidInput(format!(
            "el diagnóstico {fix_id:?} no tiene arreglo automático"
        ))
    })?;
    let message = core.apply_fix(&instance.id, action).await?;
    out.value(
        &serde_json::json!({ "applied": fix_id, "fix": action, "message": message }),
        || println!("{message}"),
    );
    Ok(())
}

fn print_report(name: &str, id: &str, report: &lcore::doctor::DoctorReport) {
    use lcore::doctor::Severity;
    if let Some(path) = &report.crash_report_path {
        println!("Último crash report: {}", path.display());
    }
    if let Some(excerpt) = &report.crash_excerpt {
        println!("Extracto: {excerpt}");
    }
    if report.diagnoses.is_empty() {
        println!("{name}: sin problemas. Java, memoria, mods y ajustes parecen correctos.");
        return;
    }
    println!(
        "{name}: {}",
        if report.healthy {
            "sin problemas graves"
        } else {
            "se encontraron problemas"
        }
    );
    for d in &report.diagnoses {
        let mark = match d.severity {
            Severity::Critical => "[CRÍTICO]",
            Severity::Warning => "[AVISO]  ",
            Severity::Info => "[INFO]   ",
        };
        println!("\n{mark} {} ({}%)  id: {}", d.title, d.confidence, d.id);
        println!("    {}", d.explanation);
        if let Some(c) = &d.culprit {
            println!("    Culpable: {c}");
        }
        if let Some(f) = &d.fix {
            println!(
                "    Arreglo: {}  ->  lcore doctor {id} --fix {}",
                f.describe(),
                d.id
            );
        }
    }
}

async fn account(core: &Core, out: &Out, cmd: AccountCmd) -> Result<()> {
    match cmd {
        AccountCmd::List => {
            let file = core.accounts().await?;
            out.value(&file, || {
                if file.accounts.is_empty() {
                    println!("No hay cuentas. Usa `lcore account login` o `lcore account add-offline <nombre>`.");
                }
                for a in &file.accounts {
                    let mark = if file.active.as_deref() == Some(&a.id) { "*" } else { " " };
                    println!("{mark} {:<16} {:<8} {}", a.name, kind(a), a.id);
                }
            });
        }
        AccountCmd::Login => {
            let code = core.begin_msa_login().await?;
            if out.json {
                println!(
                    "{}",
                    serde_json::json!({ "verification_uri": code.verification_uri, "user_code": code.user_code })
                );
            } else {
                println!(
                    "Abre {} e introduce el código {}",
                    code.verification_uri, code.user_code
                );
                println!("Esperando (Ctrl+C para cancelar)…");
            }
            let account = tokio::select! {
                r = core.complete_msa_login(&code) => r?,
                _ = tokio::signal::ctrl_c() => {
                    return Err(Error::InvalidInput("inicio de sesión cancelado".into()));
                }
            };
            out.value(&account, || {
                println!("Sesión iniciada como {}.", account.name)
            });
        }
        AccountCmd::AddOffline { name } => {
            let a = core.add_offline_account(&name).await?;
            out.value(&a, || {
                println!("Cuenta offline {} añadida ({}).", a.name, a.id)
            });
        }
        AccountCmd::Use { account } => {
            let a = core.find_account(&account).await?;
            core.set_active_account(&a.id).await?;
            out.value(&a, || println!("Cuenta activa: {}.", a.name));
        }
        AccountCmd::Remove { account } => {
            let a = core.find_account(&account).await?;
            let a = core.remove_account(&a.id).await?;
            out.value(&a, || println!("Cuenta {} eliminada.", a.name));
        }
        AccountCmd::Refresh { account } => {
            let id = match account {
                Some(q) => core.find_account(&q).await?.id,
                None => core
                    .accounts()
                    .await?
                    .active
                    .ok_or_else(|| Error::AccountNotFound("(activa)".into()))?,
            };
            let a = core.refresh_account(&id).await?;
            out.value(&a, || println!("Sesión de {} renovada.", a.name));
        }
        AccountCmd::Profile { account } => {
            let p = core.profile(account.as_deref()).await?;
            out.value(&p, || print_profile(&p));
        }
    }
    Ok(())
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

async fn servers(core: &Core, out: &Out, cmd: ServersCmd, offline: bool) -> Result<()> {
    match cmd {
        ServersCmd::List => {
            let list = core.list_servers().await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No hay servidores. Usa `lcore servers add <dirección>`.");
                }
                for (i, s) in list.iter().enumerate() {
                    let via = s.instance_id.as_deref().unwrap_or("-");
                    println!(
                        "{i:>2}  {:.8}  {:<24} {:<32} {via}",
                        s.id, s.name, s.address
                    );
                }
            });
        }
        ServersCmd::Add {
            address,
            name,
            instance,
        } => {
            let instance_id = match instance {
                Some(q) => Some(core.find_instance(&q).await?.id),
                None => None,
            };
            let server = core
                .add_server(NewServer {
                    name: name.unwrap_or_default(),
                    address,
                    instance_id,
                    account_id: None,
                })
                .await?;
            out.value(&server, || {
                println!("Servidor {} añadido ({:.8}).", server.name, server.id)
            });
        }
        ServersCmd::Remove { server } => {
            let all = core.list_servers().await?;
            let found = find_server(&all, &server)?;
            let removed = core.remove_server(&found.id).await?;
            out.value(&removed, || println!("Servidor {} quitado.", removed.name));
        }
        ServersCmd::Move { server, index } => {
            let all = core.list_servers().await?;
            let found = find_server(&all, &server)?;
            let list = core.move_server(&found.id, index).await?;
            out.value(&list, || {
                for (i, s) in list.iter().enumerate() {
                    println!("{i:>2}  {}", s.name);
                }
            });
        }
        ServersCmd::Ping { address } => {
            if offline {
                return Err(Error::InvalidInput(
                    "con --offline no se pueden consultar servidores".into(),
                ));
            }
            let (labels, addresses): (Vec<String>, Vec<String>) = match address {
                Some(a) => (vec![a.clone()], vec![a]),
                None => core
                    .list_servers()
                    .await?
                    .into_iter()
                    .map(|s| (s.name, s.address))
                    .unzip(),
            };
            let statuses = lcore::servers::ping_servers(&addresses).await;
            out.value(&statuses, || {
                if statuses.is_empty() {
                    println!("No hay servidores. Usa `lcore servers add <dirección>`.");
                }
                for (label, st) in labels.iter().zip(&statuses) {
                    print_status(label, st);
                }
            });
        }
        ServersCmd::Import { instance } => {
            let inst = core.find_instance(&instance).await?;
            let added = core.import_servers_dat(&inst.id).await?;
            out.value(&serde_json::json!({ "added": added }), || {
                println!("Servidores añadidos desde servers.dat: {added}.")
            });
        }
    }
    Ok(())
}

/// Id exacto, prefijo único de id o nombre único (sin distinguir mayúsculas).
fn find_server<'a>(list: &'a [Server], query: &str) -> Result<&'a Server> {
    if let Some(s) = list.iter().find(|s| s.id == query) {
        return Ok(s);
    }
    let by_prefix: Vec<_> = list.iter().filter(|s| s.id.starts_with(query)).collect();
    if let [one] = by_prefix.as_slice() {
        return Ok(one);
    }
    let by_name: Vec<_> = list
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case(query))
        .collect();
    match by_name.as_slice() {
        [one] => Ok(one),
        [] if by_prefix.is_empty() => Err(Error::InvalidInput(format!(
            "no existe el servidor {query}"
        ))),
        _ => Err(Error::InvalidInput(format!(
            "{query:?} es ambiguo: usa el id completo"
        ))),
    }
}

fn print_status(label: &str, st: &ServerStatus) {
    if !st.online {
        println!(
            "{label}: sin respuesta ({})",
            st.error.as_deref().unwrap_or("error desconocido")
        );
        return;
    }
    let motd = st.motd_plain.replace('\n', " | ");
    println!(
        "{label}: en línea, {} ms, {}/{} jugadores, {} ({})",
        st.latency_ms, st.players_online, st.players_max, st.version_name, st.resolved
    );
    if !motd.is_empty() {
        println!("  {motd}");
    }
    if !st.sample.is_empty() {
        println!("  jugadores: {}", st.sample.join(", "));
    }
}

fn print_dot_minecraft(report: &DotMinecraftReport, dry_run: bool) {
    let plan = &report.plan;
    println!("Origen: {}", plan.source.display());
    println!(
        "Instancia: {} ({}) — Minecraft {}, {}{}",
        plan.name,
        plan.new_id,
        plan.game.mc,
        plan.game.loader.as_str(),
        plan.game
            .loader_version
            .as_deref()
            .map(|v| format!(" {v}"))
            .unwrap_or_default()
    );
    println!("{:<14} {:>9} {:>10}", "sección", "archivos", "MB");
    for s in &plan.sections {
        if s.present {
            println!(
                "{:<14} {:>9} {:>10.1}",
                s.section.as_str(),
                s.files,
                mb(s.bytes)
            );
        } else {
            println!("{:<14} {:>9}", s.section.as_str(), "no existe");
        }
    }
    println!("Total: {} archivos, {:.1} MB", plan.files, mb(plan.bytes));
    if plan.copies_version_dir {
        println!(
            "Se copiará también la versión instalada {:?}.",
            plan.game.loader_version
        );
    }
    for w in &plan.warnings {
        eprintln!("aviso: {w}");
    }
    if dry_run {
        println!("Simulación: no se ha escrito nada.");
    } else if let Some(id) = &report.imported {
        println!(
            "Importada como {id} ({} archivos copiados).",
            report.copied_files
        );
    }
}

fn print_prism(report: &PrismReport, dry_run: bool) {
    println!(
        "{:<22} {:<24} {:<8} {:<16} {:>8} {:>10}",
        "id nuevo", "nombre", "MC", "loader", "archivos", "MB"
    );
    for i in &report.instances {
        match &i.skipped_reason {
            Some(reason) => println!("{:<22} {:<24} omitida: {reason}", i.new_id, i.name),
            None => {
                let (mc, loader) = i.game.as_ref().map_or((String::new(), String::new()), |g| {
                    (
                        g.mc.clone(),
                        format!(
                            "{} {}",
                            g.loader.as_str(),
                            g.loader_version.as_deref().unwrap_or("")
                        ),
                    )
                });
                println!(
                    "{:<22} {:<24} {:<8} {:<16} {:>8} {:>10.1}",
                    i.new_id,
                    i.name,
                    mc,
                    loader.trim(),
                    i.files,
                    mb(i.bytes)
                );
            }
        }
    }
    for w in &report.warnings {
        eprintln!("aviso: {w}");
    }
    if dry_run {
        println!("Simulación: no se ha escrito nada.");
    } else {
        println!(
            "Importadas {}: {}. Archivos copiados: {}.",
            report.imported.len(),
            report.imported.join(", "),
            report.copied_files
        );
    }
}

fn print_import(report: &ImportReport, dry_run: bool) {
    let plan = &report.plan;
    println!(
        "{:<22} {:<24} {:<8} {:<16} {:>8} {:>10} {:>8}",
        "id nuevo", "nombre", "MC", "loader", "archivos", "MB", "backups"
    );
    let (mut files, mut bytes, mut count) = (0, 0, 0);
    for i in &plan.instances {
        match &i.skipped_reason {
            Some(reason) => println!("{:<22} {:<24} omitida: {reason}", i.new_id, i.name),
            None => {
                println!(
                    "{:<22} {:<24} {:<8} {:<16} {:>8} {:>10.1} {:>8}",
                    i.new_id,
                    i.name,
                    i.mc,
                    i.loader,
                    i.files,
                    mb(i.bytes),
                    i.backups
                );
                files += i.files;
                bytes += i.bytes;
                count += 1;
            }
        }
    }
    println!(
        "\nInstancias: {count} ({files} archivos, {:.1} MB)",
        mb(bytes)
    );
    let s = &plan.shared;
    println!(
        "Compartido: librerías {} ({:.1} MB), assets {} ({:.1} MB), jars de versión {}, Java {:?}",
        s.libraries_files,
        mb(s.libraries_bytes),
        s.assets_files,
        mb(s.assets_bytes),
        s.version_jars,
        s.java_majors
    );
    for w in &plan.warnings {
        eprintln!("aviso: {w}");
    }
    if dry_run {
        println!("Simulación: no se ha escrito nada.");
    } else {
        println!(
            "Importadas {}: {}. Enlaces duros: {}, copias: {}.",
            report.imported.len(),
            report.imported.join(", "),
            report.linked_files,
            report.copied_files
        );
    }
}

struct Out {
    json: bool,
}

impl Out {
    fn value<T: Serialize>(&self, value: &T, human: impl FnOnce()) {
        if self.json {
            match serde_json::to_string_pretty(value) {
                Ok(s) => println!("{s}"),
                Err(e) => eprintln!("error al serializar: {e}"),
            }
        } else {
            human();
        }
    }
}

fn kind(a: &Account) -> &'static str {
    match a.kind {
        AccountKind::Msa => "microsoft",
        AccountKind::Offline => "offline",
    }
}

fn variant(slim: bool) -> SkinVariant {
    if slim {
        SkinVariant::Slim
    } else {
        SkinVariant::Classic
    }
}

fn print_profile(p: &Profile) {
    println!("{} ({})", p.name, p.id);
    if let Some(s) = p.active_skin() {
        println!("  skin: {} [{}]", s.url, s.variant);
    }
    for c in &p.capes {
        let mark = if c.state == "ACTIVE" { "*" } else { " " };
        println!(
            "  {mark} capa {} {}",
            c.id,
            c.alias.as_deref().unwrap_or("")
        );
    }
}

fn print_settings(s: &Settings) {
    let client_id = match s.effective_msa_client_id() {
        Some(id) => id,
        None => "(sin configurar)".into(),
    };
    println!("msa-client-id:            {client_id}");
    println!(
        "language:                 {}",
        s.language.as_deref().unwrap_or("(sistema)")
    );
    println!("close-behavior:           {:?}", s.close_behavior);
    println!("allow-multiple-games:     {}", s.allow_multiple_games);
    println!("default-memory-mb:        {}", s.default_memory_mb);
    println!("max-concurrent-downloads: {}", s.max_concurrent_downloads);
    println!("discord-rpc:              {}", s.discord_rpc);
}

type Apply = Box<dyn FnOnce(&mut Settings) + Send>;
type InstanceApply = Box<dyn FnOnce(&mut Instance) + Send>;

fn parse_setting(key: SettingKey, value: &str) -> Result<Apply> {
    let bad = |what: &str| Error::InvalidInput(format!("valor no válido para {what}: {value:?}"));
    let opt = |v: &str| (!v.is_empty() && v != "none").then(|| v.to_owned());
    let v = value.trim().to_owned();
    Ok(match key {
        SettingKey::MsaClientId => {
            let id = opt(&v);
            if let Some(id) = &id {
                uuid::Uuid::parse_str(id).map_err(|_| bad("msa-client-id (debe ser un UUID)"))?;
            }
            Box::new(move |s| s.msa_client_id = id)
        }
        SettingKey::Language => {
            let lang = opt(&v);
            Box::new(move |s| s.language = lang)
        }
        SettingKey::CloseBehavior => {
            let b = match v.as_str() {
                "ask" => CloseBehavior::Ask,
                "tray" => CloseBehavior::Tray,
                "quit" => CloseBehavior::Quit,
                _ => return Err(bad("close-behavior (ask|tray|quit)")),
            };
            Box::new(move |s| s.close_behavior = b)
        }
        SettingKey::AllowMultipleGames => {
            let b = parse_bool(&v).ok_or_else(|| bad("allow-multiple-games"))?;
            Box::new(move |s| s.allow_multiple_games = b)
        }
        SettingKey::DefaultMemoryMb => {
            let n: u32 = v
                .parse()
                .ok()
                .filter(|n| *n >= 512)
                .ok_or_else(|| bad("default-memory-mb"))?;
            Box::new(move |s| s.default_memory_mb = n)
        }
        SettingKey::MaxConcurrentDownloads => {
            let n: usize = v
                .parse()
                .ok()
                .filter(|n| (1..=64).contains(n))
                .ok_or_else(|| bad("max-concurrent-downloads (1-64)"))?;
            Box::new(move |s| s.max_concurrent_downloads = n)
        }
        SettingKey::DiscordRpc => {
            let b = parse_bool(&v).ok_or_else(|| bad("discord-rpc"))?;
            Box::new(move |s| s.discord_rpc = b)
        }
    })
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" | "on" | "yes" | "1" | "si" | "sí" => Some(true),
        "false" | "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

async fn backup(core: &Core, out: &Out, cmd: BackupCmd) -> Result<()> {
    match cmd {
        BackupCmd::Create {
            instance,
            label,
            pin,
        } => {
            let id = core.find_instance(&instance).await?.id;
            let progress = (!out.json).then(|| spawn_progress(core, false));
            let entry = core
                .create_backup(
                    &id,
                    BackupRequest {
                        label,
                        pinned: pin,
                        automatic: false,
                    },
                )
                .await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let entry = entry?;
            out.value(&entry, || {
                println!(
                    "Backup {} creado ({}).",
                    entry.id,
                    fmt_bytes(entry.size_bytes)
                )
            });
        }
        BackupCmd::List { instance } => {
            let id = core.find_instance(&instance).await?.id;
            let list = core.list_backups(&id).await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No hay backups. Crea uno con `lcore backup create`.");
                }
                for b in &list {
                    print_backup(b);
                }
            });
        }
        BackupCmd::Restore {
            instance,
            backup_id,
            worlds_only,
        } => {
            let id = core.find_instance(&instance).await?.id;
            let mode = if worlds_only {
                RestoreMode::WorldsOnly
            } else {
                RestoreMode::Full
            };
            let progress = (!out.json).then(|| spawn_progress(core, false));
            let report = core.restore_backup(&id, &backup_id, mode).await;
            if let Some(task) = progress {
                task.abort();
                eprintln!();
            }
            let report = report?;
            out.value(&report, || {
                println!(
                    "Backup {} restaurado ({} archivos).",
                    report.backup_id, report.files
                );
                for note in &report.notes {
                    println!("  - {note}");
                }
            });
        }
        BackupCmd::Delete {
            instance,
            backup_id,
        } => {
            let id = core.find_instance(&instance).await?.id;
            core.delete_backup(&id, &backup_id).await?;
            out.value(&serde_json::json!({ "deleted": backup_id }), || {
                println!("Backup {backup_id} borrado.")
            });
        }
        BackupCmd::Pin {
            instance,
            backup_id,
        } => set_pinned(core, out, &instance, &backup_id, true).await?,
        BackupCmd::Unpin {
            instance,
            backup_id,
        } => set_pinned(core, out, &instance, &backup_id, false).await?,
        BackupCmd::Auto {
            instance,
            enable,
            disable,
            interval_hours,
            max,
        } => {
            let inst = core.find_instance(&instance).await?;
            let mut settings = inst.backups.clone();
            if enable {
                settings.auto = true;
            }
            if disable {
                settings.auto = false;
            }
            if interval_hours.is_some() {
                settings.interval_hours = interval_hours;
            }
            if max.is_some() {
                settings.max_count = max;
            }
            let settings = core.update_backup_settings(&inst.id, settings).await?;
            out.value(&settings, || {
                println!(
                    "Backups automáticos: {} (cada {} h, conserva {}).",
                    if settings.auto {
                        "activados"
                    } else {
                        "desactivados"
                    },
                    settings
                        .interval_hours
                        .unwrap_or(lcore::backup::DEFAULT_INTERVAL_HOURS),
                    settings
                        .max_count
                        .unwrap_or(lcore::backup::DEFAULT_MAX_AUTOMATIC)
                )
            });
        }
        BackupCmd::RunScheduler => {
            let results = core.run_backup_scheduler_once().await;
            let view: Vec<serde_json::Value> = results
                .iter()
                .map(|(id, r)| match r {
                    Ok(entry) => serde_json::json!({ "instance": id, "backup": entry }),
                    Err(e) => {
                        serde_json::json!({ "instance": id, "error": e.kind(), "message": e.to_string() })
                    }
                })
                .collect();
            out.value(&view, || {
                if results.is_empty() {
                    println!("Nada que hacer: ninguna instancia necesita backup ahora.");
                }
                for (id, r) in &results {
                    match r {
                        Ok(e) => println!("{id}: {} ({})", e.id, fmt_bytes(e.size_bytes)),
                        Err(e) => println!("{id}: error: {e}"),
                    }
                }
            });
        }
    }
    Ok(())
}

async fn set_pinned(
    core: &Core,
    out: &Out,
    instance: &str,
    backup_id: &str,
    pinned: bool,
) -> Result<()> {
    let id = core.find_instance(instance).await?.id;
    let entry = core.set_backup_pinned(&id, backup_id, pinned).await?;
    out.value(&entry, || {
        println!(
            "Backup {backup_id} {}.",
            if pinned { "fijado" } else { "sin fijar" }
        )
    });
    Ok(())
}

async fn set_datapack(
    core: &Core,
    out: &Out,
    instance: &str,
    world: &str,
    name: &str,
    enabled: bool,
) -> Result<()> {
    let id = core.find_instance(instance).await?.id;
    core.set_datapack_enabled(&id, world, name, enabled).await?;
    out.value(
        &serde_json::json!({ "datapack": name, "enabled": enabled }),
        || {
            println!(
                "Datapack {name} {}.",
                if enabled { "activado" } else { "desactivado" }
            )
        },
    );
    Ok(())
}

fn print_backup(b: &BackupEntry) {
    let mut flags = Vec::new();
    if b.pinned {
        flags.push("fijado");
    }
    if b.automatic {
        flags.push("auto");
    }
    println!(
        "{:<22} {}  {:>9}  {:<11} {}",
        b.id,
        fmt_utc(b.created_at),
        fmt_bytes(b.size_bytes),
        flags.join(","),
        b.label.as_deref().unwrap_or("")
    );
}

async fn world(core: &Core, out: &Out, cmd: WorldCmd) -> Result<()> {
    match cmd {
        WorldCmd::List { instance } => {
            let id = core.find_instance(&instance).await?.id;
            let list = core.list_worlds(&id).await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("No hay mundos.");
                }
                for w in &list {
                    print_world(w);
                }
            });
        }
        WorldCmd::Rename {
            instance,
            folder,
            name,
        } => {
            let id = core.find_instance(&instance).await?.id;
            let info = core.rename_world(&id, &folder, &name).await?;
            out.value(&info, || {
                println!("«{folder}» ahora se llama «{}».", info.name)
            });
        }
        WorldCmd::Set {
            instance,
            folder,
            mode,
            difficulty,
            hardcore,
            cheats,
        } => {
            let flag = |what: &str, v: Option<String>| -> Result<Option<bool>> {
                v.map(|v| {
                    parse_bool(v.trim()).ok_or_else(|| {
                        Error::InvalidInput(format!(
                            "valor no válido para {what}: {v:?} (true|false)"
                        ))
                    })
                })
                .transpose()
            };
            let patch = WorldPatch {
                game_mode: mode.map(|m| match m {
                    ModeArg::Survival => GameMode::Survival,
                    ModeArg::Creative => GameMode::Creative,
                    ModeArg::Adventure => GameMode::Adventure,
                    ModeArg::Spectator => GameMode::Spectator,
                }),
                difficulty: difficulty.map(|d| match d {
                    DifficultyArg::Peaceful => Difficulty::Peaceful,
                    DifficultyArg::Easy => Difficulty::Easy,
                    DifficultyArg::Normal => Difficulty::Normal,
                    DifficultyArg::Hard => Difficulty::Hard,
                }),
                hardcore: flag("hardcore", hardcore)?,
                allow_commands: flag("cheats", cheats)?,
            };
            if patch.game_mode.is_none()
                && patch.difficulty.is_none()
                && patch.hardcore.is_none()
                && patch.allow_commands.is_none()
            {
                return Err(Error::InvalidInput(
                    "indica algo que cambiar: --mode, --difficulty, --hardcore o --cheats".into(),
                ));
            }
            let id = core.find_instance(&instance).await?.id;
            let info = core.update_world(&id, &folder, patch).await?;
            out.value(&info, || {
                println!("Guardado.");
                print_world(&info);
            });
        }
        WorldCmd::Delete {
            instance,
            folder,
            yes,
        } => {
            if !yes {
                return Err(Error::InvalidInput(format!(
                    "borrar el mundo «{folder}» es definitivo: repite el comando con --yes"
                )));
            }
            let id = core.find_instance(&instance).await?.id;
            core.delete_world(&id, &folder).await?;
            out.value(&serde_json::json!({ "deleted": folder }), || {
                println!("Mundo «{folder}» borrado.")
            });
        }
    }
    Ok(())
}

fn print_world(w: &WorldInfo) {
    let yes_no = |b: Option<bool>| match b {
        Some(true) => "sí",
        Some(false) => "no",
        None => "?",
    };
    println!(
        "{:<24} {:<24} {:<10} {:<9} hardcore:{:<2} trucos:{:<2} {:>9}  {}",
        w.folder,
        w.name,
        w.game_mode.map_or("?", |m| m.as_str()),
        w.difficulty.map_or("?", |d| d.as_str()),
        yes_no(w.hardcore),
        yes_no(w.cheats),
        fmt_bytes(w.size_bytes),
        w.last_played_ms
            .map_or_else(|| "nunca".into(), |ms| fmt_utc(ms / 1000)),
    );
    if let Some(e) = &w.error {
        println!("  aviso: {e}");
    }
}

async fn datapack(core: &Core, out: &Out, cmd: DatapackCmd) -> Result<()> {
    match cmd {
        DatapackCmd::List { instance, world } => {
            let id = core.find_instance(&instance).await?.id;
            let list = core.list_datapacks(&id, &world).await?;
            out.value(&list, || {
                if list.is_empty() {
                    println!("El mundo no tiene datapacks.");
                }
                for p in &list {
                    println!(
                        "{:<9} {:<8} {:>9}  {}",
                        if p.enabled { "activo" } else { "inactivo" },
                        if p.is_dir { "carpeta" } else { "zip" },
                        fmt_bytes(p.size_bytes),
                        p.name
                    );
                }
            });
        }
        DatapackCmd::Enable {
            instance,
            world,
            name,
        } => set_datapack(core, out, &instance, &world, &name, true).await?,
        DatapackCmd::Disable {
            instance,
            world,
            name,
        } => set_datapack(core, out, &instance, &world, &name, false).await?,
        DatapackCmd::Remove {
            instance,
            world,
            name,
        } => {
            let id = core.find_instance(&instance).await?.id;
            core.remove_datapack(&id, &world, &name).await?;
            out.value(&serde_json::json!({ "removed": name }), || {
                println!("Datapack {name} borrado.")
            });
        }
    }
    Ok(())
}

fn fmt_bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// `YYYY-MM-DD HH:MM` en UTC a partir de segundos Unix.
fn fmt_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Días desde 1970-01-01 → fecha civil (algoritmo de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rem / 3_600,
        rem % 3_600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fechas_y_tamanos() {
        assert_eq!(fmt_utc(0), "1970-01-01 00:00");
        assert_eq!(fmt_utc(1_791_764_045), "2026-10-12 00:14");
        assert_eq!(fmt_utc(951_782_400 + 3_600), "2000-02-29 01:00");
        assert_eq!(fmt_bytes(12), "12 B");
        assert_eq!(fmt_bytes(1536), "1.5 KB");
        assert_eq!(fmt_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
    }
}
