//! Ventana sin marco, bandeja y cierre según `close_behavior`
//! (app_shell.dart, tray_controller.dart, core/window.dart).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lcore::settings::CloseBehavior;
use slint::ComponentHandle;

use crate::backend::Backend;
use crate::{MainWindow, ShardTray};

pub struct Shell {
    ui: slint::Weak<MainWindow>,
    tray: RefCell<Option<ShardTray>>,
    close: Cell<CloseBehavior>,
    backend: Option<Backend>,
}

impl Shell {
    pub fn new(ui: &MainWindow, close: CloseBehavior, backend: Option<Backend>) -> Rc<Self> {
        let shell = Rc::new(Self { ui: ui.as_weak(), tray: RefCell::default(), close: Cell::new(close), backend });
        let weak = ui.as_weak();
        ui.on_minimize(move || {
            if let Some(u) = weak.upgrade() {
                u.window().set_minimized(true);
            }
        });
        let weak = ui.as_weak();
        ui.on_toggle_max(move || {
            if let Some(u) = weak.upgrade() {
                let max = !u.window().is_maximized();
                u.window().set_maximized(max);
                u.set_is_maximized(max);
            }
        });
        let s = Rc::downgrade(&shell);
        ui.on_close_requested(move || {
            if let Some(s) = s.upgrade() {
                s.request_close();
            }
        });
        let s = Rc::downgrade(&shell);
        ui.on_close_chosen(move |quit, remember| {
            if let Some(s) = s.upgrade() {
                s.chosen(quit, remember);
            }
        });
        // El cierre del sistema (Alt+F4, botón del gestor) pasa por lo mismo.
        let s = Rc::downgrade(&shell);
        ui.window().on_close_requested(move || {
            if let Some(s) = s.upgrade() {
                s.request_close();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
        if close == CloseBehavior::Tray {
            shell.ensure_tray();
        }
        shell
    }

    fn request_close(&self) {
        match self.close.get() {
            CloseBehavior::Quit => quit(),
            CloseBehavior::Tray => self.to_background(),
            CloseBehavior::Ask => {
                if let Some(u) = self.ui.upgrade() {
                    u.set_close_dialog_open(true);
                }
            }
        }
    }

    fn chosen(&self, quit_app: bool, remember: bool) {
        if remember {
            let behavior = if quit_app { CloseBehavior::Quit } else { CloseBehavior::Tray };
            self.close.set(behavior);
            if let Some(b) = &self.backend {
                let core = b.core.clone();
                b.rt.spawn(async move {
                    if let Err(e) = core.update_settings(|s| s.close_behavior = behavior).await {
                        tracing::warn!(error = %e, "no se pudo guardar close_behavior");
                    }
                });
            }
        }
        if quit_app { quit() } else { self.to_background() }
    }

    /// Oculta la ventana dejando el icono en la bandeja.
    pub fn to_background(&self) {
        self.ensure_tray();
        if let Some(u) = self.ui.upgrade()
            && let Err(e) = u.hide()
        {
            tracing::warn!(error = %e, "no se pudo ocultar la ventana");
        }
    }

    pub fn ensure_tray(&self) {
        if self.tray.borrow().is_some() {
            return;
        }
        let tray = match ShardTray::new() {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "sin bandeja");
                return;
            }
        };
        let weak = self.ui.clone();
        tray.on_open(move || {
            if let Some(u) = weak.upgrade() {
                u.show().ok();
            }
        });
        tray.on_quit(quit);
        tray.on_play_last(|| {
            crate::backend::with_controller(|c| {
                let first = c.model.borrow().all.first().map(|r| r.id.clone());
                if let Some(id) = first {
                    c.play(&id);
                }
            });
        });
        *self.tray.borrow_mut() = Some(tray);
    }

    /// Nombre de la última instancia para «Jugar …» del menú.
    pub fn set_tray_last(&self, name: &str) {
        if let Some(t) = self.tray.borrow().as_ref() {
            t.set_last_instance(name.into());
        }
    }
}

pub fn quit() {
    if let Err(e) = slint::quit_event_loop() {
        tracing::warn!(error = %e, "no se pudo salir del bucle de eventos");
    }
}
