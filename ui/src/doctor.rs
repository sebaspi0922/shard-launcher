//! Doctor (doctor/doctor_dialog.dart): informe de la última partida de una
//! instancia, con arreglos automáticos.
//!
//! `Doctor` corre en el hilo de la UI y rellena `DoctorState`; el trabajo con
//! el núcleo lo hace un `DoctorActions` (`CoreDoctor` en la app, un registro
//! en los tests), que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::controller::Controller;
use crate::errors::Failure;
use crate::{DiagnosisItem, DoctorState, LoadState, MainWindow, i18n};

/// Un diagnóstico (`DiagnosisView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnosis {
    pub id: String,
    pub category: String,
    /// `critical`, `warning` o `info`.
    pub severity: String,
    pub title: String,
    pub explanation: String,
    pub confidence: u8,
    pub culprit: Option<String>,
    /// Descripción del arreglo automático.
    pub fix: Option<String>,
}

/// `DoctorReportView`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    pub diagnoses: Vec<Diagnosis>,
    pub crash_excerpt: Option<String>,
    pub crash_report_path: Option<String>,
}

fn category(c: lcore::doctor::Category) -> &'static str {
    use lcore::doctor::Category::*;
    match c {
        Java => "java",
        Memory => "memory",
        JvmArgs => "jvm_args",
        ModConflict => "mod_conflict",
        MissingDependency => "missing_dependency",
        CorruptedFile => "corrupted_file",
        GraphicsDriver => "graphics_driver",
        Loader => "loader",
        Account => "account",
        Network => "network",
        System => "system",
        WorldCorruption => "world_corruption",
        Unknown => "unknown",
    }
}

impl From<&lcore::doctor::DoctorReport> for Report {
    fn from(r: &lcore::doctor::DoctorReport) -> Self {
        Self {
            diagnoses: r
                .diagnoses
                .iter()
                .map(|d| Diagnosis {
                    id: d.id.clone(),
                    category: category(d.category).into(),
                    severity: d.severity.as_str().into(),
                    title: d.title.clone(),
                    explanation: d.explanation.clone(),
                    confidence: d.confidence,
                    culprit: d.culprit.clone(),
                    fix: d.fix.as_ref().map(|f| f.describe()),
                })
                .collect(),
            crash_excerpt: r.crash_excerpt.clone(),
            crash_report_path: r.crash_report_path.as_ref().map(|p| p.display().to_string()),
        }
    }
}

/// Lo que Doctor pide fuera de la UI.
pub trait DoctorActions {
    /// Analiza → [`Doctor::report`] con el mismo `seq`.
    fn diagnose(&self, instance_id: &str, seq: u64);
    /// Aplica el arreglo de un diagnóstico → [`Doctor::fixed`] con la misma
    /// instancia.
    fn apply_fix(&self, instance_id: &str, diagnosis_id: &str);
    /// Abre el crash report con la aplicación del sistema.
    fn open_path(&self, path: &str);
}

thread_local! {
    static DOCTOR: RefCell<Option<Rc<Doctor>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Doctor.
pub fn post(f: impl FnOnce(&Doctor) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&Doctor)) {
    DOCTOR.with(|c| {
        if let Some(d) = c.borrow().as_ref() {
            f(d);
        }
    })
}

pub struct Doctor {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn DoctorActions>,
    instance: RefCell<Option<String>>,
    /// Último análisis pedido (los viejos no pisan).
    seq: Cell<u64>,
    report: RefCell<Option<Report>>,
    /// Arreglo en curso: (instancia, diagnóstico). Sobrevive a cerrar el
    /// diálogo: hay uno a la vez.
    applying: RefCell<Option<(String, String)>>,
    items: Rc<VecModel<DiagnosisItem>>,
}

impl Doctor {
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn DoctorActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            instance: RefCell::default(),
            seq: Cell::new(0),
            report: RefCell::default(),
            applying: RefCell::default(),
            items: Rc::new(VecModel::default()),
        });
        DOCTOR.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<DoctorState>();
        st.set_diagnoses(ModelRc::from(me.items.clone()));
        let weak = Rc::downgrade(&me);
        st.on_rerun(move || {
            if let Some(d) = weak.upgrade() {
                d.analyze();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_apply(move |id| {
            if let Some(d) = weak.upgrade() {
                d.apply(&id);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_open_report(move || {
            if let Some(d) = weak.upgrade()
                && let Some(p) = d.report.borrow().as_ref().and_then(|r| r.crash_report_path.clone())
            {
                d.actions.open_path(&p);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_closed(move || {
            if let Some(d) = weak.upgrade() {
                *d.instance.borrow_mut() = None;
            }
        });
        me
    }

    /// Abre Doctor para una instancia (`showDoctor`).
    pub fn open(&self, instance_id: &str) {
        let name = self.shell.model.borrow().by_id(instance_id).map(|r| r.name.clone()).unwrap_or_else(|| instance_id.to_owned());
        *self.instance.borrow_mut() = Some(instance_id.to_owned());
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<DoctorState>();
            st.set_instance_name(name.into());
            st.set_open(true);
        }
        self.analyze();
    }

    pub fn is_open(&self) -> bool {
        self.ui.upgrade().is_some_and(|u| u.global::<DoctorState>().get_open())
    }

    fn analyze(&self) {
        let Some(id) = self.instance.borrow().clone() else { return };
        let seq = self.seq.get() + 1;
        self.seq.set(seq);
        *self.report.borrow_mut() = None;
        self.refresh(None);
        self.actions.diagnose(&id, seq);
    }

    /// Resultado del análisis.
    pub fn report(&self, seq: u64, r: Result<Report, Failure>) {
        if seq != self.seq.get() {
            return;
        }
        match r {
            Ok(rep) => {
                *self.report.borrow_mut() = Some(rep);
                self.refresh(None);
            }
            Err(f) => self.refresh(Some(f)),
        }
    }

    fn apply(&self, diagnosis_id: &str) {
        if self.applying.borrow().is_some() {
            return;
        }
        let Some(id) = self.instance.borrow().clone() else { return };
        *self.applying.borrow_mut() = Some((id.clone(), diagnosis_id.to_owned()));
        self.refresh(None);
        self.actions.apply_fix(&id, diagnosis_id);
    }

    /// Resultado del arreglo: aviso y nuevo análisis (memoria, JVM o loader
    /// cambian la instancia: la lista se recarga en `CoreDoctor`).
    pub fn fixed(&self, instance_id: &str, r: Result<String, Failure>) {
        *self.applying.borrow_mut() = None;
        // Solo se vuelve a analizar si el diálogo sigue en esa instancia.
        let current = self.instance.borrow().as_deref() == Some(instance_id);
        let lang = self.shell.lang();
        match r {
            Ok(msg) => {
                let text = if msg.is_empty() { i18n::text(&lang, "doctorFixApplied") } else { msg };
                self.shell.toast(&text, false);
                if current {
                    self.analyze();
                    return;
                }
            }
            Err(f) => {
                self.shell.toast(&f.message, true);
                if current {
                    self.refresh(None);
                    return;
                }
            }
        }
        // Otra instancia abierta: sus botones vuelven a estar libres; su
        // informe (o su error) no se toca.
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<DoctorState>().set_applying(false);
        }
    }

    fn refresh(&self, error: Option<Failure>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<DoctorState>();
        let report = self.report.borrow();
        let applying = self.applying.borrow().clone();
        let state = match (&*report, &error) {
            (_, Some(f)) => {
                st.set_error_message(f.message.as_str().into());
                st.set_error_detail(f.detail.as_str().into());
                LoadState::Failed
            }
            (Some(_), None) => LoadState::Ready,
            (None, None) => LoadState::Loading,
        };
        st.set_state(state);
        let empty = Report::default();
        let r = report.as_ref().unwrap_or(&empty);
        let count = |s: &str| r.diagnoses.iter().filter(|d| d.severity == s).count() as i32;
        st.set_critical(count("critical"));
        st.set_warnings(count("warning"));
        st.set_notes(count("info"));
        st.set_crash_excerpt(r.crash_excerpt.clone().unwrap_or_default().into());
        st.set_has_report(r.crash_report_path.is_some());
        st.set_applying(applying.is_some());
        let items: Vec<DiagnosisItem> = r
            .diagnoses
            .iter()
            .map(|d| DiagnosisItem {
                id: d.id.as_str().into(),
                category: d.category.as_str().into(),
                severity: match d.severity.as_str() {
                    "critical" => 0,
                    "warning" => 1,
                    _ => 2,
                },
                title: d.title.as_str().into(),
                explanation: d.explanation.as_str().into(),
                confidence: d.confidence as i32,
                culprit: d.culprit.clone().unwrap_or_default().into(),
                fix: d.fix.clone().unwrap_or_default().into(),
                applying: applying.as_ref().is_some_and(|(i, a)| self.instance.borrow().as_deref() == Some(i.as_str()) && *a == d.id),
            })
            .collect();
        crate::controller::sync(&self.items, items);
    }
}

/// Doctor contra el núcleo.
pub struct CoreDoctor {
    pub backend: crate::backend::Backend,
    pub lang: String,
}

impl DoctorActions for CoreDoctor {
    fn diagnose(&self, instance_id: &str, seq: u64) {
        let (core, lang, id) = (self.backend.core.clone(), self.lang.clone(), instance_id.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.diagnose(&id).await.map(|r| Report::from(&r)).map_err(|e| Failure::new(&lang, &e));
            post(move |d| d.report(seq, r));
        });
    }

    fn apply_fix(&self, instance_id: &str, diagnosis_id: &str) {
        let b = self.backend.clone();
        let (lang, id, did) = (self.lang.clone(), instance_id.to_owned(), diagnosis_id.to_owned());
        self.backend.rt.spawn(async move {
            // Como el bridge: se vuelve a analizar y se aplica el arreglo de
            // ese diagnóstico si sigue ahí.
            let r = async {
                let report = b.core.diagnose(&id).await?;
                let d = report
                    .diagnoses
                    .iter()
                    .find(|d| d.id == did)
                    .ok_or_else(|| lcore::Error::InvalidInput(format!("el diagnóstico {did:?} ya no aparece")))?;
                let fix = d.fix.as_ref().ok_or_else(|| lcore::Error::InvalidInput(format!("el diagnóstico {did:?} no tiene arreglo automático")))?;
                b.core.apply_fix(&id, fix).await
            }
            .await
            .map_err(|e| Failure::new(&lang, &e));
            if r.is_ok() {
                b.load();
            }
            post(move |d| d.fixed(&id, r));
        });
    }

    fn open_path(&self, path: &str) {
        if let Err(e) = open::that_detached(path) {
            tracing::warn!(path, error = %e, "no se pudo abrir el crash report");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorias_con_los_nombres_del_bridge() {
        assert_eq!(category(lcore::doctor::Category::MissingDependency), "missing_dependency");
        assert_eq!(category(lcore::doctor::Category::WorldCorruption), "world_corruption");
    }
}
