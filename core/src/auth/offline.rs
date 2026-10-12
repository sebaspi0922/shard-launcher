//! Cuentas offline: UUID igual al que calcula el servidor en modo offline.

use md5::{Digest, Md5};
use uuid::{Builder, Uuid};

use super::AuthError;

/// `UUID.nameUUIDFromBytes("OfflinePlayer:" + nombre)` de Java (UUID v3 sin namespace).
pub fn offline_uuid(name: &str) -> Uuid {
    let digest = Md5::digest(format!("OfflinePlayer:{name}").as_bytes());
    let bytes: [u8; 16] = digest.into();
    Builder::from_md5_bytes(bytes).into_uuid()
}

/// Mismas reglas que los nombres de cuentas de Minecraft.
pub fn validate_name(name: &str) -> Result<(), AuthError> {
    let ok = (3..=16).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok {
        Ok(())
    } else {
        Err(AuthError::InvalidOfflineName(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_como_java() {
        // Valores de UUID.nameUUIDFromBytes en Java.
        assert_eq!(
            offline_uuid("Notch").to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(offline_uuid("Notch").get_version_num(), 3);
    }

    #[test]
    fn nombres() {
        assert!(validate_name("Steve_01").is_ok());
        for bad in ["ab", "nombre con espacio", "ñandú", "abcdefghijklmnopq"] {
            assert!(validate_name(bad).is_err(), "{bad}");
        }
    }
}
