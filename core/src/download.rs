//! Descargas en paralelo con verificación de hash, `.part` + rename y reintentos.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::net::{Http, check_status};
use crate::{Error, Result};

const ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

/// Cómo se decide que un archivo existente es válido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verify {
    /// Existe y, si se conoce, coincide el tamaño. Rápido: para cada arranque.
    Size,
    /// Se calcula el SHA-1 de lo que hay en disco. Para reparar.
    Full,
}

/// Comprueba si `task.dest` ya es válido.
pub async fn is_valid(task: &DownloadTask, verify: Verify) -> bool {
    let Ok(meta) = tokio::fs::metadata(&task.dest).await else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    if let Some(size) = task.size
        && meta.len() != size
    {
        return false;
    }
    match (verify, &task.sha1) {
        (Verify::Full, Some(expected)) => sha1_file(&task.dest)
            .await
            .is_ok_and(|h| h.eq_ignore_ascii_case(expected)),
        _ => true,
    }
}

pub async fn sha1_file(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path).map_err(|e| Error::io(&path, e))?;
        let mut hasher = Sha1::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = std::io::Read::read(&mut file, &mut buf).map_err(|e| Error::io(&path, e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hex::encode(hasher.finalize()))
    })
    .await
    .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
}

/// Descarga un archivo verificando el hash al vuelo.
pub async fn download_one(http: &Http, task: &DownloadTask) -> Result<()> {
    if http.is_offline() {
        return Err(Error::Offline {
            url: task.url.clone(),
        });
    }
    let mut last_err = None;
    for attempt in 1..=ATTEMPTS {
        match try_download(http, task).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let retry = matches!(e, Error::Network(_) | Error::HashMismatch { .. })
                    || matches!(e, Error::HttpStatus { status, .. } if status >= 500);
                tracing::debug!(url = %task.url, attempt, error = %e, "descarga fallida");
                last_err = Some(e);
                if !retry {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(300 * attempt as u64)).await;
            }
        }
    }
    Err(last_err.expect("al menos un intento"))
}

async fn try_download(http: &Http, task: &DownloadTask) -> Result<()> {
    let parent = task
        .dest
        .parent()
        .ok_or_else(|| Error::InvalidPath(task.dest.display().to_string()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| Error::io(parent, e))?;
    let part = part_path(&task.dest);
    let resp = http.send(|c| c.get(&task.url)).await?;
    let mut resp = check_status(resp)?;

    let mut file = tokio::fs::File::create(&part)
        .await
        .map_err(|e| Error::io(&part, e))?;
    let mut hasher = Sha1::new();
    let mut written = 0u64;
    let result = async {
        while let Some(chunk) = resp.chunk().await? {
            hasher.update(&chunk);
            written += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::io(&part, e))?;
        }
        file.flush().await.map_err(|e| Error::io(&part, e))?;
        drop(file);
        let actual = hex::encode(hasher.finalize());
        if let Some(expected) = &task.sha1
            && !actual.eq_ignore_ascii_case(expected)
        {
            return Err(Error::HashMismatch {
                url: task.url.clone(),
                expected: expected.clone(),
                actual,
            });
        }
        if let Some(size) = task.size
            && written != size
        {
            return Err(Error::HashMismatch {
                url: task.url.clone(),
                expected: format!("{size} bytes"),
                actual: format!("{written} bytes"),
            });
        }
        tokio::fs::rename(&part, &task.dest)
            .await
            .map_err(|e| Error::io(&task.dest, e))
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}

fn part_path(dest: &Path) -> PathBuf {
    // Único por descarga: dos descargas del mismo destino a la vez no se pisan.
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{:08x}.part", fastrand::u32(..)));
    dest.with_file_name(name)
}

/// Descarga lo que falte de `tasks` con `concurrency` descargas a la vez.
/// `progress(hechas, total)` se llama tras cada archivo (incluidos los que ya estaban).
/// Devuelve el número de archivos descargados.
pub async fn download_all(
    http: &Http,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    verify: Verify,
    progress: impl Fn(u64, u64) + Send + Sync + 'static,
) -> Result<usize> {
    let mut tasks = tasks;
    tasks.sort_by(|a, b| a.dest.cmp(&b.dest));
    tasks.dedup_by(|a, b| a.dest == b.dest);
    let total = tasks.len() as u64;
    let done = Arc::new(AtomicU64::new(0));
    let progress = Arc::new(progress);
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut set = JoinSet::new();

    for task in tasks {
        let http = http.clone();
        let semaphore = semaphore.clone();
        let done = done.clone();
        let progress = progress.clone();
        set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.expect("semáforo abierto");
            let downloaded = if is_valid(&task, verify).await {
                false
            } else {
                download_one(&http, &task).await?;
                true
            };
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            progress(n, total);
            Ok::<bool, Error>(downloaded)
        });
    }

    let mut downloaded = 0;
    let mut first_err = None;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(true)) => downloaded += 1,
            Ok(Ok(false)) => {}
            Ok(Err(e)) => {
                if first_err.is_none() {
                    first_err = Some(e);
                    set.abort_all();
                }
            }
            Err(e) if e.is_cancelled() => {}
            Err(e) => {
                first_err.get_or_insert(Error::InvalidInput(format!("tarea de descarga: {e}")));
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(downloaded),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sha1_hex(data: &[u8]) -> String {
        hex::encode(Sha1::digest(data))
    }

    #[tokio::test]
    async fn descarga_verifica_y_salta_existentes() {
        let server = MockServer::start().await;
        for i in 0..5 {
            Mock::given(method("GET"))
                .and(path(format!("/f{i}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(format!("data{i}")))
                .expect(1)
                .mount(&server)
                .await;
        }
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().join("cache"), false).unwrap();
        let tasks: Vec<_> = (0..5)
            .map(|i| DownloadTask {
                url: format!("{}/f{i}", server.uri()),
                dest: dir.path().join(format!("out/sub{i}/f{i}")),
                sha1: Some(sha1_hex(format!("data{i}").as_bytes())),
                size: Some(5),
            })
            .collect();
        let calls = Arc::new(AtomicU64::new(0));
        let c = calls.clone();
        let n = download_all(&http, tasks.clone(), 2, Verify::Size, move |_, _| {
            c.fetch_add(1, Ordering::Relaxed);
        })
        .await
        .unwrap();
        assert_eq!(n, 5);
        assert_eq!(calls.load(Ordering::Relaxed), 5);
        assert_eq!(
            std::fs::read(dir.path().join("out/sub3/f3")).unwrap(),
            b"data3"
        );
        // Segunda vez: nada que descargar (expect(1) lo comprueba al soltar el servidor).
        let n = download_all(&http, tasks, 2, Verify::Full, |_, _| {})
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn hash_incorrecto_falla_sin_dejar_basura() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes("malo"))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new(dir.path().join("cache"), false).unwrap();
        let task = DownloadTask {
            url: format!("{}/x", server.uri()),
            dest: dir.path().join("x.jar"),
            sha1: Some(sha1_hex(b"bueno")),
            size: None,
        };
        let err = download_all(&http, vec![task], 4, Verify::Size, |_, _| {})
            .await
            .unwrap_err();
        assert!(matches!(err, Error::HashMismatch { .. }), "{err:?}");
        assert!(!dir.path().join("x.jar").exists());
        assert!(!dir.path().join("x.jar.part").exists());
    }
}
