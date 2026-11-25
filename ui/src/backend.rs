//! Trabajo con el núcleo (`lcore::Core`) en tokio, sin bloquear la UI.
//! Sustituye a las funciones del bridge de flutter_rust_bridge: cada acción
//! es una tarea que llama al núcleo directo y devuelve el resultado al hilo
//! de la UI con `post`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lcore::Core;
use lcore::events::Event;
use lcore::launch::LaunchOptions;
use lcore::update::{Package, UpdateInfo};
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;

use crate::controller::{Actions, Controller};
use crate::errors;
use crate::home::{Server, Status};
use crate::instances::Record;
use crate::launch::{Failure, is_ready_line};
use crate::notices::UpdateView;

thread_local! {
    static CONTROLLER: RefCell<Option<Rc<Controller>>> = const { RefCell::new(None) };
}

/// Registra el controlador del hilo de la UI (una vez, al arrancar).
pub fn install(ctl: Rc<Controller>) {
    CONTROLLER.with(|c| *c.borrow_mut() = Some(ctl));
}

/// `f` con el controlador, ya en el hilo de la UI.
pub fn with_controller(f: impl FnOnce(&Controller)) {
    CONTROLLER.with(|c| {
        if let Some(ctl) = c.borrow().as_ref() {
            f(ctl);
        }
    })
}

/// Ejecuta `f` en el hilo de la UI con el controlador.
pub fn post(f: impl FnOnce(&Controller) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with_controller(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Espera tras el arranque y entre consultas automáticas de versión nueva.
pub const UPDATE_FIRST_CHECK: Duration = Duration::from_secs(20);
pub const UPDATE_INTERVAL: Duration = Duration::from_secs(6 * 3600);

/// Versión del paquete (`0.1.0.r23`), la pone el empaquetado al compilar;
/// vacía en las compilaciones propias (como `kPackageVersion`).
pub const PACKAGE_VERSION: &str = match option_env!("SHARD_VERSION") {
    Some(v) => v,
    None => "",
};

#[derive(Clone)]
pub struct Backend {
    pub core: Core,
    pub rt: Handle,
    /// Última versión nueva consultada (la que instala `install_update`).
    pending_update: Arc<Mutex<Option<UpdateInfo>>>,
}

impl Backend {
    pub fn new(core: Core, rt: Handle) -> Self {
        Self { core, rt, pending_update: Arc::default() }
    }

    /// Instancias y servidores (la cuenta activa la lleva `accounts`).
    pub fn load(&self) {
        let core = self.core.clone();
        let me = self.clone();
        self.rt.spawn(async move {
            match core.list_instances().await {
                Ok(list) => {
                    let list: Vec<Record> = list.into_iter().map(Record::from).collect();
                    post(move |c| c.set_instances(list));
                }
                Err(e) => post(move |c| {
                    let (msg, detail) = errors::describe(&c.lang(), &e);
                    c.set_failed(msg, detail);
                }),
            }
            me.load_servers();
        });
    }

    /// Servidores guardados y, después, su estado (ping en paralelo; el núcleo
    /// guarda 30 s el resultado).
    pub fn load_servers(&self) {
        let core = self.core.clone();
        self.rt.spawn(async move {
            let list = match core.list_servers().await {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!(error = %e, "no se pudieron leer los servidores");
                    return;
                }
            };
            let servers: Vec<Server> = list.into_iter().map(Server::from).collect();
            let addresses: Vec<String> = servers.iter().map(|s| s.address.clone()).collect();
            post(move |c| c.set_servers(servers));
            ping(addresses).await;
        });
    }

    /// Reenvía a la UI los eventos que cambian lo que se ve. De las líneas de
    /// registro solo pasan las que dicen que el juego ya abrió (la consola,
    /// fase 3, las agrupará como hace el bridge).
    ///
    /// Antes, las partidas que siguen abiertas al arrancar (`_restore`), en
    /// la misma tarea y ya suscrita: un cierre posterior llega a la UI
    /// después y no lo pisa la lista vieja.
    pub fn pump_events(&self) {
        let mut rx = self.core.subscribe();
        let me = self.clone();
        self.rt.spawn(async move {
            for g in me.core.running_games().await {
                post(move |c| c.restore_game(&g.instance_id, g.game_id));
            }
            loop {
                match rx.recv().await {
                    Ok(Event::GameLog { ref line, .. }) if !is_ready_line(line) => {}
                    Ok(e) => {
                        let reload = matches!(e, Event::GameExited { .. } | Event::GameCrashed { .. } | Event::AccountsChanged);
                        if matches!(e, Event::AccountsChanged) {
                            crate::accounts::post(|a| a.reload());
                        }
                        post(move |c| c.on_event(&e));
                        if reload {
                            // Tiempo jugado, última partida o cuenta cambiaron.
                            me.load();
                        }
                    }
                    Err(RecvError::Lagged(n)) => tracing::debug!(n, "eventos omitidos hacia la UI"),
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }

    /// Consultas automáticas de versión nueva, solo en una app empaquetada y
    /// con el ajuste activo (`UpdateController._autoCheck`).
    pub fn schedule_update_checks(&self) {
        if PACKAGE_VERSION.is_empty() || matches!(Package::detect(), Package::Unpackaged) {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            tokio::time::sleep(UPDATE_FIRST_CHECK).await;
            loop {
                if me.core.settings().await.check_updates {
                    me.check_update(false).await;
                }
                tokio::time::sleep(UPDATE_INTERVAL).await;
            }
        });
    }

    async fn check_update(&self, manual: bool) {
        match self.core.check_update(PACKAGE_VERSION, &Package::detect()).await {
            Ok(info) => {
                // La versión a instalar se guarda solo si la UI acepta el
                // resultado (no lo acepta durante una instalación).
                let pending = self.pending_update.clone();
                post(move |c| {
                    if c.update_checked(info.as_ref().map(UpdateView::from), manual) {
                        *pending.lock().expect("candado de actualización") = info;
                    }
                });
            }
            Err(e) => {
                tracing::info!(error = %e, "no se pudo consultar si hay versión nueva");
                post(move |c| {
                    let err = errors::describe(&c.lang(), &e);
                    c.update_check_failed(err, manual);
                });
            }
        }
    }

    /// Acento de la imagen de fondo (decodificar y cuantizar lleva un rato).
    pub fn image_accent(&self, path: std::path::PathBuf) {
        self.rt.spawn_blocking(move || {
            let accent = crate::accent::from_path(&path);
            post(move |c| c.appearance.set_image_accent(accent));
        });
    }
}

/// Pide el estado de las direcciones y lo entrega a la UI.
async fn ping(addresses: Vec<String>) {
    if addresses.is_empty() {
        return;
    }
    let list = lcore::servers::ping_servers(&addresses).await;
    let st: Vec<(String, Status)> = list.into_iter().map(|s| (s.address.clone(), Status::from(s))).collect();
    post(move |c| c.set_statuses(st));
}

/// Acciones de la UI contra el núcleo.
pub struct CoreActions(pub Backend);

impl CoreActions {
    fn launch(&self, id: &str, mut opts: LaunchOptions) {
        let id = id.to_owned();
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            let result = async {
                let file = core.accounts().await?;
                let supported = || file.accounts.iter()
                    .filter(|a| a.kind != lcore::auth::AccountKind::Shard);
                let find = |id: &str| supported().find(|a| a.id == id);
                let account = opts.account.as_deref().and_then(find)
                    .or_else(|| file.active.as_deref().and_then(find))
                    .or_else(|| supported().next())
                    .ok_or_else(|| lcore::Error::AccountNotFound(String::new()))?;
                // El núcleo busca por nombre antes que por ID.
                let resolved = core.find_account(&account.id).await?;
                if resolved.id != account.id || resolved.kind != account.kind {
                    return Err(lcore::Error::AccountNotFound(account.id.clone()));
                }
                opts.account = Some(account.id.clone());
                core.launch(&id, opts).await
            }.await;
            match result {
                Ok(game) => post(move |c| c.launch_done(&id, Ok(game.game_id))),
                Err(e) => {
                    tracing::warn!(id, error = %e, "no se pudo lanzar");
                    let failure = Failure::from_error(&e);
                    post(move |c| c.launch_done(&id, Err(failure)));
                }
            }
        });
    }
}

impl Actions for CoreActions {
    fn open_instance(&self, id: &str) {
        // El detalle de la instancia llega en la fase 3.
        tracing::info!(id, "abrir instancia: pendiente (fase 3)");
    }

    fn play(&self, id: &str) {
        self.launch(id, LaunchOptions::default());
    }

    fn join_server(&self, instance_id: &str, address: &str, account: Option<&str>) {
        self.launch(
            instance_id,
            LaunchOptions { server: Some(address.to_owned()), account: account.map(str::to_owned), ..Default::default() },
        );
    }

    fn new_instance(&self) {
        tracing::info!("nueva instancia: pendiente (fase 3)");
    }

    fn show_trash(&self) {
        tracing::info!("papelera: pendiente (fase 3)");
    }

    fn accounts(&self) {
        tracing::info!("cuentas: pendiente (fase 2)");
    }

    fn add_microsoft_account(&self) {
        tracing::info!("cuenta de Microsoft: pendiente (fase 2)");
    }

    fn add_offline_account(&self) {
        tracing::info!("cuenta offline: pendiente (fase 2)");
    }

    fn add_server(&self) {
        tracing::info!("añadir servidor: pendiente (fase 4)");
    }

    fn diagnose(&self, id: &str) {
        tracing::info!(id, "Doctor: pendiente (fase 5)");
    }

    fn install_update(&self) {
        let b = self.0.clone();
        self.0.rt.spawn(async move {
            let info = b.pending_update.lock().expect("candado de actualización").clone();
            let Some(info) = info else { return };
            match b.core.install_update(&info).await {
                Ok(path) => post(move |c| c.update_installed(Ok(path))),
                Err(e) => post(move |c| {
                    let err = errors::describe(&c.lang(), &e);
                    c.update_installed(Err(err));
                }),
            }
        });
    }

    fn restart_update(&self, path: &std::path::Path) {
        // Las partidas abiertas siguen: el juego no depende del launcher.
        match std::process::Command::new(path).spawn() {
            Ok(_) => crate::window::quit(),
            Err(e) => tracing::warn!(path = %path.display(), error = %e, "no se pudo abrir la versión nueva"),
        }
    }

    fn open_url(&self, url: &str) {
        if let Err(e) = webbrowser::open(url) {
            tracing::warn!(url, error = %e, "no se pudo abrir el enlace");
        }
    }

    fn refresh_servers(&self) {
        let b = self.0.clone();
        self.0.rt.spawn(async move {
            let addresses = match b.core.list_servers().await {
                Ok(l) => l.into_iter().map(|s| s.address).collect(),
                Err(_) => vec![],
            };
            ping(addresses).await;
        });
    }
}
