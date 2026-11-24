//! Trabajo con el núcleo (`lcore::Core`) en tokio, sin bloquear la UI.
//! Sustituye a las funciones del bridge de flutter_rust_bridge: cada acción
//! es una tarea que llama al núcleo directo y devuelve el resultado al hilo
//! de la UI con `ui::post`.

use std::cell::RefCell;
use std::rc::Rc;

use lcore::Core;
use lcore::events::Event;
use lcore::launch::LaunchOptions;
use tokio::runtime::Handle;
use tokio::sync::broadcast::error::RecvError;

use crate::controller::{Actions, Controller};
use crate::errors;
use crate::instances::{Phase, Record};

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

#[derive(Clone)]
pub struct Backend {
    pub core: Core,
    pub rt: Handle,
}

impl Backend {
    /// Instancias y partidas en marcha (la cuenta activa la lleva `accounts`).
    pub fn load(&self) {
        let core = self.core.clone();
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
            for g in core.running_games().await {
                post(move |c| c.set_phase(&g.instance_id, Phase::Running));
            }
        });
    }

    /// Reenvía a la UI los eventos que cambian lo que se ve. Las líneas de
    /// registro no: la consola (fase 3) las agrupará como hace el bridge.
    pub fn pump_events(&self) {
        let mut rx = self.core.subscribe();
        let me = self.clone();
        self.rt.spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(Event::GameStarted { instance_id, .. }) => {
                        post(move |c| c.set_phase(&instance_id, Phase::Running));
                    }
                    Ok(Event::GameExited { instance_id, .. } | Event::GameCrashed { instance_id, .. }) => {
                        post(move |c| c.set_phase(&instance_id, Phase::Idle));
                        // Tiempo jugado y última partida cambiaron.
                        me.load();
                    }
                    Ok(Event::AccountsChanged) => crate::accounts::post(|a| a.reload()),
                    Ok(_) => {}
                    Err(RecvError::Lagged(n)) => tracing::debug!(n, "eventos omitidos hacia la UI"),
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}

/// Acciones de la pantalla Instancias contra el núcleo.
pub struct CoreActions(pub Backend);

impl Actions for CoreActions {
    fn open_instance(&self, id: &str) {
        // El detalle de la instancia llega en la fase 3.
        tracing::info!(id, "abrir instancia: pendiente (fase 3)");
    }

    fn play(&self, id: &str) {
        let id = id.to_owned();
        post({
            let id = id.clone();
            move |c| c.set_phase(&id, Phase::Preparing)
        });
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            match core.launch(&id, LaunchOptions::default()).await {
                Ok(_) => post(move |c| c.set_phase(&id, Phase::Running)),
                Err(e) => post(move |c| {
                    let (msg, detail) = errors::describe(&c.lang(), &e);
                    tracing::warn!(id, %detail, "no se pudo lanzar");
                    c.set_phase(&id, Phase::Error);
                    c.toast(&msg, true);
                }),
            }
        });
    }

    fn new_instance(&self) {
        tracing::info!("nueva instancia: pendiente (fase 3)");
    }

    fn show_trash(&self) {
        tracing::info!("papelera: pendiente (fase 3)");
    }
}
