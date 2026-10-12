//! Almacén de secretos: keyring del SO en uso normal, memoria en las pruebas.
//! Los secretos nunca se escriben en JSON ni en logs.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use crate::{Error, Result};

pub trait SecretStore: Send + Sync + fmt::Debug {
    fn get(&self, service: &str, user: &str) -> Result<Option<String>>;
    fn set(&self, service: &str, user: &str, secret: &str) -> Result<()>;
    /// Borra el secreto; no es error si no existía.
    fn delete(&self, service: &str, user: &str) -> Result<()>;
}

/// Keyring del sistema: Secret Service, Credential Manager o Keychain.
#[derive(Debug, Default)]
pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn get(&self, service: &str, user: &str) -> Result<Option<String>> {
        match entry(service, user)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Secrets(e.to_string())),
        }
    }

    fn set(&self, service: &str, user: &str, secret: &str) -> Result<()> {
        entry(service, user)?
            .set_password(secret)
            .map_err(|e| Error::Secrets(e.to_string()))
    }

    fn delete(&self, service: &str, user: &str) -> Result<()> {
        match entry(service, user)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Secrets(e.to_string())),
        }
    }
}

fn entry(service: &str, user: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(service, user).map_err(|e| Error::Secrets(e.to_string()))
}

/// Almacén en memoria para pruebas.
#[derive(Default)]
pub struct MemoryStore {
    items: Mutex<HashMap<(String, String), String>>,
}

impl fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.items.lock().map(|m| m.len()).unwrap_or(0);
        f.debug_struct("MemoryStore").field("items", &n).finish()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, service: &str, user: &str) -> Result<Option<String>> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .get(&(service.to_owned(), user.to_owned()))
            .cloned())
    }

    fn set(&self, service: &str, user: &str, secret: &str) -> Result<()> {
        self.items
            .lock()
            .unwrap()
            .insert((service.to_owned(), user.to_owned()), secret.to_owned());
        Ok(())
    }

    fn delete(&self, service: &str, user: &str) -> Result<()> {
        self.items
            .lock()
            .unwrap()
            .remove(&(service.to_owned(), user.to_owned()));
        Ok(())
    }
}

/// Cadena secreta que no se muestra en `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(store: &dyn SecretStore) {
        let (svc, user) = ("shard.test", "prueba");
        store.delete(svc, user).unwrap();
        assert_eq!(store.get(svc, user).unwrap(), None);
        store.set(svc, user, "valor-de-prueba").unwrap();
        assert_eq!(
            store.get(svc, user).unwrap().as_deref(),
            Some("valor-de-prueba")
        );
        store.delete(svc, user).unwrap();
        assert_eq!(store.get(svc, user).unwrap(), None);
    }

    #[test]
    fn memoria() {
        roundtrip(&MemoryStore::default());
    }

    /// Usa el keyring real con una entrada temporal: `cargo test -- --ignored keyring`.
    #[test]
    #[ignore = "toca el keyring del sistema"]
    fn keyring_del_sistema() {
        roundtrip(&KeyringStore);
    }

    #[test]
    fn secret_no_se_muestra() {
        assert_eq!(format!("{:?}", Secret::new("abc")), "Secret(***)");
    }
}
