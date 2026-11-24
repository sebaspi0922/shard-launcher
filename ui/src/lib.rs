//! Interfaz de Shard en Slint. `main.rs` es una capa fina; la lógica está
//! aquí para que los tests (paridad, interacción) usen el mismo código.

slint::include_modules!();

pub mod accounts;
pub mod accounts_backend;
pub mod arb;
pub mod art;
pub mod backend;
pub mod controller;
pub mod errors;
pub mod face;
pub mod fixtures;
pub mod i18n;
pub mod instances;
pub mod media;
pub mod prefs;
pub mod skins;
pub mod skin;
pub mod skin_mesh;
pub mod window;
