//! Imágenes remotas en caché de disco (compartido entre pantallas: skins,
//! iconos de mods, servidores). Mismo criterio y misma carpeta que
//! `cached_image` del bridge de Flutter (`<datos>/cache/icons/<fnv64 url>`),
//! así las dos UIs comparten caché.

use std::path::PathBuf;

use lcore::Core;

/// Tamaño máximo de una imagen descargada.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// FNV-1a de 64 bits (nombre del archivo en caché).
pub fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Descarga (una vez) una imagen `https://` de como mucho 2 MB y devuelve
/// sus bytes. `None` si no se pudo (sin red, error, demasiado grande).
pub async fn cached_image(core: &Core, url: &str) -> Option<Vec<u8>> {
    if !url.starts_with("https://") || url.len() > 2048 {
        return None;
    }
    let dir = core.paths().root().join("cache").join("icons");
    let path: PathBuf = dir.join(format!("{:016x}", fnv64(url)));
    if let Ok(bytes) = tokio::fs::read(&path).await {
        return Some(bytes);
    }
    if core.http().is_offline() {
        return None;
    }
    let resp = core.http().send(|c| c.get(url)).await.ok().filter(|r| r.status().is_success())?;
    if resp.content_length().is_some_and(|n| n as usize > MAX_BYTES) {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return None;
    }
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        tracing::debug!(error = %e, "no se pudo crear la caché de imágenes");
    } else if let Err(e) = lcore::fsutil::write_atomic(&path, &bytes).await {
        tracing::debug!(error = %e, "no se pudo guardar la imagen en caché");
    }
    Some(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    #[test]
    fn mismo_hash_que_el_bridge() {
        // Valor de referencia de FNV-1a 64 («a»).
        assert_eq!(super::fnv64("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
