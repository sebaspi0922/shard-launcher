//! Ventana sin marco, bandeja y cierre según `close_behavior`
//! (app_shell.dart, tray_controller.dart, core/window.dart).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lcore::settings::CloseBehavior;
use slint::ComponentHandle;

use crate::backend::Backend;
use crate::{MainWindow, ResizeEdge, ShardTray};

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
        let weak = ui.as_weak();
        ui.on_resize_start(move |edge| {
            if let Some(u) = weak.upgrade() {
                drag_resize(&u, edge);
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

    /// Ajustes cambió «al cerrar la ventana».
    pub fn set_close_behavior(&self, close: CloseBehavior) {
        self.close.set(close);
        if close == CloseBehavior::Tray {
            self.ensure_tray();
        }
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

    /// Oculta la ventana dejando el icono en la bandeja. Sin bandeja en el
    /// sistema solo minimiza, como Flutter (`toBackground`): si no, la
    /// ventana quedaría oculta sin forma de volver.
    pub fn to_background(&self) {
        let Some(u) = self.ui.upgrade() else { return };
        if !tray_host_available() {
            tracing::info!("sin bandeja en el sistema: se minimiza");
            u.window().set_minimized(true);
            return;
        }
        self.ensure_tray();
        if let Err(e) = u.hide() {
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

/// ¿Hay quien pinte el icono de la bandeja? En Linux, un
/// `StatusNotifierWatcher` en el bus de sesión (waybar, Noctalia, KDE,
/// GNOME con AppIndicator…). Windows y macOS siempre tienen bandeja.
/// `SHARD_TRAY=0|1` lo fuerza (tests, entornos raros).
pub fn tray_host_available() -> bool {
    match std::env::var("SHARD_TRAY").as_deref() {
        Ok("0") => return false,
        Ok("1") => return true,
        _ => {}
    }
    #[cfg(target_os = "linux")]
    {
        let has_watcher = || -> zbus::Result<bool> {
            let conn = zbus::blocking::Connection::session()?;
            let dbus = zbus::blocking::fdo::DBusProxy::new(&conn)?;
            Ok(dbus.name_has_owner("org.kde.StatusNotifierWatcher".try_into()?)?)
        };
        has_watcher().unwrap_or_else(|e| {
            tracing::info!(error = %e, "sin bus de sesión");
            false
        })
    }
    #[cfg(not(target_os = "linux"))]
    true
}

/// Redimensionar desde un borde: lo hace el sistema (winit
/// `drag_resize_window`), sin marco no hay bordes propios.
fn drag_resize(ui: &MainWindow, edge: ResizeEdge) {
    use slint::winit_030::WinitWindowAccessor;
    use slint::winit_030::winit::window::ResizeDirection as D;
    let dir = match edge {
        ResizeEdge::N => D::North,
        ResizeEdge::S => D::South,
        ResizeEdge::E => D::East,
        ResizeEdge::W => D::West,
        ResizeEdge::Ne => D::NorthEast,
        ResizeEdge::Nw => D::NorthWest,
        ResizeEdge::Se => D::SouthEast,
        ResizeEdge::Sw => D::SouthWest,
    };
    ui.window().with_winit_window(|w| {
        if let Err(e) = w.drag_resize_window(dir) {
            tracing::debug!(error = %e, "no se pudo redimensionar desde el borde");
        }
    });
}

pub fn quit() {
    if let Err(e) = slint::quit_event_loop() {
        tracing::warn!(error = %e, "no se pudo salir del bucle de eventos");
    }
}
