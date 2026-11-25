//! Avisos flotantes de abajo a la derecha: versión nueva
//! (`app/lib/core/updates.dart`) y cierre inesperado del juego
//! (`app/lib/core/crash_notice.dart`). Lógica pura.

use std::collections::HashSet;

use lcore::events::Event;

/// Versión nueva (lo que muestra el aviso; `lcore::update::UpdateInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateView {
    pub version: String,
    pub current: String,
    pub notes: Option<String>,
    pub notes_url: Option<String>,
    /// Se descarga e instala desde la app (AppImage).
    pub can_install: bool,
    /// Archivo del paquete o página de descargas.
    pub download_url: String,
}

impl From<&lcore::update::UpdateInfo> for UpdateView {
    fn from(i: &lcore::update::UpdateInfo) -> Self {
        Self {
            version: i.version.clone(),
            current: i.current.clone(),
            notes: i.notes.clone(),
            notes_url: i.notes_url.clone(),
            can_install: i.can_install(),
            download_url: i.download_url().to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Installing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Update {
    pub status: UpdateStatus,
    pub info: Option<UpdateView>,
    /// (texto, detalle) del último fallo.
    pub error: Option<(String, String)>,
    /// AppImage ya actualizado (`Ready`).
    pub path: Option<std::path::PathBuf>,
    /// El aviso se cerró; Ajustes sigue mostrando el estado.
    pub dismissed: bool,
}

impl Update {
    /// ¿Se ve el aviso?
    pub fn visible(&self) -> bool {
        self.info.is_some()
            && !self.dismissed
            && matches!(self.status, UpdateStatus::Available | UpdateStatus::Installing | UpdateStatus::Ready | UpdateStatus::Failed)
    }

    /// Resultado de una consulta. Un aviso cerrado vuelve a salir solo si
    /// hay otra versión (o si la consulta la pidió el usuario).
    /// `false` si se descarta: una consulta automática no pisa una
    /// instalación en curso o lista (Flutter ni la lanza en esos estados;
    /// esta pudo salir antes).
    pub fn checked(&mut self, info: Option<UpdateView>, manual: bool) -> bool {
        if !manual && matches!(self.status, UpdateStatus::Installing | UpdateStatus::Ready) {
            return false;
        }
        *self = match info {
            None => Update { status: UpdateStatus::UpToDate, ..Default::default() },
            Some(info) => {
                let same = self.info.as_ref().is_some_and(|p| p.version == info.version);
                Update { status: UpdateStatus::Available, dismissed: !manual && same && self.dismissed, info: Some(info), ..Default::default() }
            }
        };
        true
    }

    /// Falló una consulta: las automáticas no se muestran.
    pub fn check_failed(&mut self, error: (String, String), manual: bool) {
        if manual {
            self.status = UpdateStatus::Failed;
            self.error = Some(error);
        } else if self.status == UpdateStatus::Checking {
            self.status = if self.info.is_some() { UpdateStatus::Available } else { UpdateStatus::Idle };
        }
    }

    /// Empieza a instalar. `false` si no se puede.
    pub fn begin_install(&mut self) -> bool {
        if !self.info.as_ref().is_some_and(|i| i.can_install) {
            return false;
        }
        self.status = UpdateStatus::Installing;
        self.dismissed = false;
        self.error = None;
        true
    }
}

/// `CrashNotice`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashNotice {
    pub game_id: u64,
    pub instance_id: String,
    pub code: Option<i32>,
    pub crash_report: Option<String>,
}

/// `CrashNoticeController`: un aviso por partida que se cierra mal.
#[derive(Debug, Default)]
pub struct Crashes {
    pub notice: Option<CrashNotice>,
    /// Partidas ya avisadas (llegan `GameCrashed` y `GameExited` de la misma).
    seen: HashSet<u64>,
}

impl Crashes {
    fn show(&mut self, n: CrashNotice) {
        if self.seen.insert(n.game_id) {
            self.notice = Some(n);
        }
    }

    /// `true` si cambió el aviso.
    pub fn on_event(&mut self, e: &Event) -> bool {
        let before = self.notice.clone();
        match e {
            Event::GameCrashed { game_id, instance_id, crash_report } => self.show(CrashNotice {
                game_id: *game_id,
                instance_id: instance_id.clone(),
                code: None,
                crash_report: crash_report.as_ref().map(|p| p.to_string_lossy().into_owned()),
            }),
            Event::GameExited { game_id, instance_id, code: Some(code), .. } if *code != 0 => {
                self.show(CrashNotice { game_id: *game_id, instance_id: instance_id.clone(), code: Some(*code), crash_report: None })
            }
            // Una partida nueva de la misma instancia deja el aviso viejo sin sentido.
            Event::GameStarted { instance_id, .. } if self.notice.as_ref().is_some_and(|n| &n.instance_id == instance_id) => {
                self.notice = None
            }
            _ => {}
        }
        before != self.notice
    }

    pub fn dismiss(&mut self) {
        self.notice = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(v: &str) -> UpdateView {
        UpdateView { version: v.into(), current: "0.1".into(), notes: None, notes_url: None, can_install: true, download_url: "u".into() }
    }

    #[test]
    fn aviso_cerrado_solo_vuelve_con_otra_version() {
        let mut u = Update::default();
        u.checked(Some(info("0.2")), false);
        assert!(u.visible());
        u.dismissed = true;
        u.checked(Some(info("0.2")), false);
        assert!(!u.visible());
        u.checked(Some(info("0.3")), false);
        assert!(u.visible());
        u.dismissed = true;
        u.checked(Some(info("0.3")), true);
        assert!(u.visible(), "la consulta manual lo vuelve a mostrar");
        u.checked(None, false);
        assert_eq!(u.status, UpdateStatus::UpToDate);
        assert!(!u.visible());
    }

    #[test]
    fn consulta_automatica_no_pisa_la_instalacion() {
        let mut u = Update::default();
        u.checked(Some(info("0.2")), false);
        u.begin_install();
        assert!(!u.checked(Some(info("0.2")), false), "se descarta");
        assert_eq!(u.status, UpdateStatus::Installing);
        u.status = UpdateStatus::Ready;
        u.path = Some("/x".into());
        u.checked(None, false);
        assert_eq!((u.status, u.path.as_deref()), (UpdateStatus::Ready, Some(std::path::Path::new("/x"))));
    }

    #[test]
    fn fallo_automatico_no_se_muestra() {
        let mut u = Update { status: UpdateStatus::Checking, ..Default::default() };
        u.check_failed(("sin red".into(), "network".into()), false);
        assert_eq!(u.status, UpdateStatus::Idle);
        u.check_failed(("sin red".into(), "network".into()), true);
        assert_eq!(u.status, UpdateStatus::Failed);
    }

    #[test]
    fn instalar_solo_si_se_puede() {
        let mut u = Update::default();
        assert!(!u.begin_install());
        u.checked(Some(UpdateView { can_install: false, ..info("0.2") }), false);
        assert!(!u.begin_install());
        u.checked(Some(info("0.2")), false);
        assert!(u.begin_install());
        assert_eq!(u.status, UpdateStatus::Installing);
    }

    #[test]
    fn un_aviso_por_partida_y_se_quita_al_relanzar() {
        let mut c = Crashes::default();
        assert!(c.on_event(&Event::GameCrashed { game_id: 1, instance_id: "a".into(), crash_report: None }));
        assert!(!c.on_event(&Event::GameExited { game_id: 1, instance_id: "a".into(), code: Some(1), seconds_played: 3 }));
        c.dismiss();
        assert!(!c.on_event(&Event::GameExited { game_id: 2, instance_id: "a".into(), code: Some(0), seconds_played: 3 }));
        assert!(c.on_event(&Event::GameExited { game_id: 3, instance_id: "a".into(), code: Some(-1), seconds_played: 3 }));
        assert_eq!(c.notice.as_ref().unwrap().code, Some(-1));
        assert!(c.on_event(&Event::GameStarted { game_id: 4, instance_id: "a".into(), pid: None }));
        assert!(c.notice.is_none());
    }
}
