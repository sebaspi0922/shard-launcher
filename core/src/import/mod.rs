//! Importadores desde otros launchers: Roxy Launcher 0.6.0 (diseño en
//! `docs/migracion.md`), el `.minecraft` del launcher oficial y Prism/MultiMC.

mod common;
pub mod minecraft;
pub mod prism;
pub mod roxy;

pub use minecraft::{
    DotMinecraftImport, DotMinecraftInfo, DotMinecraftPlan, DotMinecraftReport, InstalledVersion,
    LauncherProfile, Section, SectionInfo, inspect_dot_minecraft,
};
pub use prism::{
    PrismImport, PrismInfo, PrismInstanceInfo, PrismPlanned, PrismReport, inspect_prism,
};
pub use roxy::{
    BackupEntry, BackupIndex, ContentFile, ContentIndex, ImportPlan, ImportReport, PlannedInstance,
    RoxyImportOptions, SharedPlan,
};
