//! Núcleo del launcher: cuentas, instancias, descargas y lanzamiento.
//! Diseño en `docs/arquitectura.md`; la CLI está en `src/bin/lcore.rs`.

pub mod accounts;
pub mod auth;
pub mod backup;
pub mod content;
pub mod doctor;
pub mod download;
pub mod error;
pub mod events;
pub mod fsutil;
pub mod import;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod launcher;
pub mod loader;
pub mod logs;
pub mod meta;
pub mod net;
pub mod paths;
pub mod secrets;
pub mod servers;
pub mod sessions;
pub mod settings;
pub mod update;
pub mod world;

pub use error::{Error, Result};
pub use launcher::{Core, Options};
