//! Doctor: diagnóstico de una instancia y arreglos automáticos.
//!
//! El arreglo no cruza a Dart: la UI devuelve el id del diagnóstico y aquí se
//! vuelve a diagnosticar para aplicar el arreglo que propone el núcleo. Así la
//! UI no puede pedir una acción arbitraria.

use lcore::doctor::{Category, Diagnosis, DoctorReport};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct DiagnosisView {
    /// Único dentro del informe; es lo que se pasa a `apply_fix`.
    pub id: String,
    /// Tipo de firma (estable).
    pub kind: String,
    /// `java`, `memory`, `jvm_args`, `mod_conflict`…
    pub category: String,
    /// `critical`, `warning` o `info`.
    pub severity: String,
    pub title: String,
    pub explanation: String,
    /// 0–100.
    pub confidence: u8,
    pub culprit: Option<String>,
    /// Descripción del arreglo automático, si lo hay.
    pub fix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorReportView {
    /// Sin problemas graves ni avisos.
    pub healthy: bool,
    pub diagnoses: Vec<DiagnosisView>,
    pub crash_excerpt: Option<String>,
    pub crash_report_path: Option<String>,
}

fn category(c: Category) -> &'static str {
    match c {
        Category::Java => "java",
        Category::Memory => "memory",
        Category::JvmArgs => "jvm_args",
        Category::ModConflict => "mod_conflict",
        Category::MissingDependency => "missing_dependency",
        Category::CorruptedFile => "corrupted_file",
        Category::GraphicsDriver => "graphics_driver",
        Category::Loader => "loader",
        Category::Account => "account",
        Category::Network => "network",
        Category::System => "system",
        Category::WorldCorruption => "world_corruption",
        Category::Unknown => "unknown",
    }
}

fn diagnosis_view(d: &Diagnosis) -> DiagnosisView {
    DiagnosisView {
        id: d.id.clone(),
        kind: d.kind.clone(),
        category: category(d.category).into(),
        severity: d.severity.as_str().into(),
        title: d.title.clone(),
        explanation: d.explanation.clone(),
        confidence: d.confidence,
        culprit: d.culprit.clone(),
        fix: d.fix.as_ref().map(|f| f.describe()),
    }
}

fn report_view(r: &DoctorReport) -> DoctorReportView {
    DoctorReportView {
        healthy: r.healthy,
        diagnoses: r.diagnoses.iter().map(diagnosis_view).collect(),
        crash_excerpt: r.crash_excerpt.clone(),
        crash_report_path: r
            .crash_report_path
            .as_ref()
            .map(|p| p.display().to_string()),
    }
}

/// Analiza la última partida de la instancia (crash report, logs, mods y ajustes).
pub async fn diagnose(instance_id: String) -> Result<DoctorReportView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(report_view(&core.diagnose(&instance_id).await?)) }).await
}

/// Aplica el arreglo del diagnóstico `diagnosis_id` (se recalcula aquí) y
/// devuelve el mensaje del núcleo.
pub async fn apply_fix(instance_id: String, diagnosis_id: String) -> Result<String, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let report = core.diagnose(&instance_id).await?;
        let diagnosis = report
            .diagnoses
            .iter()
            .find(|d| d.id == diagnosis_id)
            .ok_or_else(|| BridgeError {
                kind: "invalid_input".into(),
                message: format!("el diagnóstico {diagnosis_id:?} ya no aparece"),
            })?;
        let fix = diagnosis.fix.as_ref().ok_or_else(|| BridgeError {
            kind: "invalid_input".into(),
            message: format!("el diagnóstico {diagnosis_id:?} no tiene arreglo automático"),
        })?;
        Ok(core.apply_fix(&instance_id, fix).await?)
    })
    .await
}
