//! Interfaz de Shard en Slint. `main.rs` es una capa fina; la lógica está
//! aquí para que los tests (paridad, interacción) usen el mismo código.

slint::include_modules!();

pub mod accent;
pub mod accounts;
pub mod accounts_backend;
pub mod appearance;
pub mod arb;
pub mod art;
pub mod backdrop;
pub mod backend;
pub mod content;
pub mod content_backend;
pub mod controller;
pub mod curseforge;
pub mod errors;
pub mod face;
pub mod fixtures;
pub mod home;
pub mod i18n;
pub mod instances;
pub mod launch;
pub mod media;
pub mod notices;
pub mod prefs;
pub mod skin;
pub mod skin_mesh;
pub mod servers;
pub mod skins;
pub mod theme;
pub mod window;
