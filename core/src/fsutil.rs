//! Lectura y escritura de JSON con escritura atómica.

use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use crate::{Error, Result};

/// Lee un JSON; `Ok(None)` si el archivo no existe.
pub async fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| Error::json(path.display().to_string(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// Escribe JSON con formato en un temporal del mismo directorio y lo renombra.
pub async fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|e| Error::json(path.display().to_string(), e))?;
    bytes.push(b'\n');
    write_atomic(path, &bytes).await
}

pub async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| Error::io(dir, e))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;
    let tmp = dir.join(format!(
        ".{}.{:08x}.tmp",
        name.to_string_lossy(),
        fastrand::u32(..)
    ));
    let result = async {
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| Error::io(&tmp, e))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, bytes)
            .await
            .map_err(|e| Error::io(&tmp, e))?;
        file.sync_all().await.map_err(|e| Error::io(&tmp, e))?;
        tokio::fs::rename(&tmp, path)
            .await
            .map_err(|e| Error::io(path, e))
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    result
}

/// Segundos desde la época Unix.
pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/x.json");
        assert!(read_json::<Vec<u32>>(&path).await.unwrap().is_none());
        write_json(&path, &vec![1u32, 2, 3]).await.unwrap();
        assert_eq!(
            read_json::<Vec<u32>>(&path).await.unwrap(),
            Some(vec![1, 2, 3])
        );
        // No quedan temporales.
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec!["x.json"]);
    }
}
