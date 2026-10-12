//! Actualizaciones de la app (decisión D8 en `docs/decisiones.md`).
//!
//! `latest.json` en `dl.shardlauncher.com` describe la última versión y, por
//! paquete, un archivo con su SHA-256 y su firma minisign. El comentario
//! confiable de la firma es `shard <versión> <paquete>`, así que una firma
//! vieja no sirve para otra versión ni para otro paquete. Solo el AppImage se
//! sustituye solo; con Flatpak, pacman o una compilación propia se avisa y se
//! enlaza la descarga.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::download::{Sha256File, download_sha256};
use crate::events::{Event, ProgressUnit};
use crate::install::next_task_id;
use crate::launcher::Core;
use crate::{Error, Result};

pub const MANIFEST_URL: &str = "https://dl.shardlauncher.com/latest.json";
/// Sustituye `MANIFEST_URL` (pruebas con un servidor local).
pub const MANIFEST_URL_ENV: &str = "SHARD_UPDATE_URL";
/// Página de descargas, para paquetes sin archivo propio en el manifiesto.
pub const DOWNLOAD_PAGE: &str = "https://shardlauncher.com/#descargas";
pub const MANIFEST_SCHEMA: u32 = 1;
/// Clave pública de las releases (`tools/release/minisign.pub`). La privada
/// no está en el repo.
const PUBLIC_KEY: &str = "RWTmkvLgmOjHgKb3OEtqeeURolP31OsOIIUv0FKvt8DZzSyX4yxdHlQV";

/// `latest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub notes_url: Option<String>,
    /// Clave: `linux-x86_64-appimage`, `linux-x86_64-flatpak`, `linux-x86_64-arch`…
    #[serde(default)]
    pub files: BTreeMap<String, Artifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub url: String,
    pub size: u64,
    /// En hexadecimal.
    pub sha256: String,
    /// Contenido del `.minisig`.
    pub signature: String,
}

/// Cómo está instalada la app que pregunta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Package {
    /// `path` es el archivo `.AppImage` (variable `APPIMAGE`).
    AppImage {
        path: PathBuf,
    },
    Flatpak,
    /// Paquete de pacman (`/usr/lib/shard`).
    Arch,
    /// Compilación propia o sin empaquetar.
    Unpackaged,
}

impl Package {
    /// Detecta el paquete por el entorno y la ruta del ejecutable.
    pub fn detect() -> Self {
        if let Some(path) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
            return Package::AppImage { path: path.into() };
        }
        if std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists() {
            return Package::Flatpak;
        }
        let exe = std::env::current_exe().and_then(std::fs::canonicalize);
        if exe.is_ok_and(|p| p.starts_with("/usr/lib/shard")) {
            return Package::Arch;
        }
        Package::Unpackaged
    }

    /// Clave del archivo de este paquete en el manifiesto.
    pub fn artifact_key(&self) -> Option<String> {
        let kind = match self {
            Package::AppImage { .. } => "appimage",
            Package::Flatpak => "flatpak",
            Package::Arch => "arch",
            Package::Unpackaged => return None,
        };
        Some(format!(
            "{}-{}-{kind}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

/// Versión del paquete: `0.1.0` o `0.1.0.r23` (la revisión es el número de
/// commits). Se ignora un sufijo `+…` y una `v` inicial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version([u64; 4]);

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('v');
        let s = s.split('+').next().unwrap_or_default();
        let mut parts = s.split('.');
        let mut out = [0u64; 4];
        for slot in out.iter_mut().take(3) {
            *slot = parts.next()?.parse().ok()?;
        }
        if let Some(rev) = parts.next() {
            out[3] = rev.strip_prefix('r')?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        Some(Version(out))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub pub_date: Option<String>,
    pub notes: Option<String>,
    pub notes_url: Option<String>,
    pub package: Package,
    /// Archivo para el paquete actual, si el manifiesto lo trae.
    pub artifact: Option<Artifact>,
}

impl UpdateInfo {
    /// Se puede instalar desde la app (solo AppImage).
    pub fn can_install(&self) -> bool {
        matches!(self.package, Package::AppImage { .. }) && self.artifact.is_some()
    }

    /// Enlace para descargarla a mano.
    pub fn download_url(&self) -> &str {
        self.artifact
            .as_ref()
            .map_or(DOWNLOAD_PAGE, |a| a.url.as_str())
    }
}

pub fn manifest_url() -> String {
    std::env::var(MANIFEST_URL_ENV)
        .ok()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| MANIFEST_URL.to_owned())
}

impl Core {
    /// Consulta el manifiesto. `None` si `current` ya es la última versión.
    pub async fn check_update(
        &self,
        current: &str,
        package: &Package,
    ) -> Result<Option<UpdateInfo>> {
        let current_v = Version::parse(current)
            .ok_or_else(|| Error::InvalidInput(format!("versión actual no válida: {current}")))?;
        let manifest: Manifest = self.http().get_json(&manifest_url()).await?;
        if manifest.schema != MANIFEST_SCHEMA {
            return Err(Error::UpdateRejected(format!(
                "formato de manifiesto desconocido ({})",
                manifest.schema
            )));
        }
        let latest = Version::parse(&manifest.version).ok_or_else(|| {
            Error::UpdateRejected(format!("versión no válida: {}", manifest.version))
        })?;
        if latest <= current_v {
            return Ok(None);
        }
        let artifact = package
            .artifact_key()
            .and_then(|k| manifest.files.get(&k).cloned());
        Ok(Some(UpdateInfo {
            version: manifest.version,
            current: current.to_owned(),
            pub_date: manifest.pub_date,
            notes: manifest.notes,
            notes_url: manifest.notes_url,
            package: package.clone(),
            artifact,
        }))
    }

    /// Descarga, verifica y pone el AppImage nuevo en lugar del actual.
    /// Devuelve la ruta del AppImage (la misma); hay que reiniciar para usarlo.
    pub async fn install_update(&self, info: &UpdateInfo) -> Result<PathBuf> {
        self.install_update_with_key(info, PUBLIC_KEY).await
    }

    async fn install_update_with_key(
        &self,
        info: &UpdateInfo,
        public_key: &str,
    ) -> Result<PathBuf> {
        let Package::AppImage { path } = &info.package else {
            return Err(Error::InvalidInput(
                "solo el AppImage se actualiza desde la app".into(),
            ));
        };
        let (Some(key), Some(artifact)) = (info.package.artifact_key(), &info.artifact) else {
            return Err(Error::UpdateRejected(
                "el manifiesto no trae AppImage para esta plataforma".into(),
            ));
        };
        // Firma y comentario se comprueban antes de descargar nada.
        let signature = parse_signature(&artifact.signature, &info.version, &key)?;
        let name = path
            .file_name()
            .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?
            .to_string_lossy()
            .into_owned();
        let dir = path
            .parent()
            .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;
        // Junto al actual, para que el cambio sea un `rename` atómico.
        let tmp = dir.join(format!(".{name}.update"));

        let task_id = next_task_id();
        let events = self.inner.events.clone();
        let progress = move |done: u64, total: u64| {
            events.emit(Event::Progress {
                task_id,
                stage: "update".into(),
                done,
                total,
                unit: ProgressUnit::Bytes,
            });
        };
        let result = async {
            download_sha256(
                self.http(),
                &Sha256File {
                    url: &artifact.url,
                    sha256: &artifact.sha256,
                    size: Some(artifact.size),
                },
                &tmp,
                progress,
            )
            .await?;
            let key = public_key.to_owned();
            let file = tmp.clone();
            tokio::task::spawn_blocking(move || verify_file(&file, &key, &signature))
                .await
                .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
            make_executable(&tmp)?;
            std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
        }
        .await;
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        self.inner.events.emit(Event::TaskFinished {
            task_id,
            ok: result.is_ok(),
            message: result.as_ref().err().map(|e| e.to_string()),
        });
        result.map(|()| path.clone())
    }
}

/// Comentario confiable que debe llevar la firma.
pub fn expected_comment(version: &str, key: &str) -> String {
    format!("shard {version} {key}")
}

fn parse_signature(text: &str, version: &str, key: &str) -> Result<minisign_verify::Signature> {
    let signature = minisign_verify::Signature::decode(text)
        .map_err(|e| Error::UpdateRejected(format!("firma ilegible: {e}")))?;
    let expected = expected_comment(version, key);
    if signature.trusted_comment() != expected {
        return Err(Error::UpdateRejected(format!(
            "la firma es de «{}», no de «{expected}»",
            signature.trusted_comment()
        )));
    }
    Ok(signature)
}

fn verify_file(
    path: &Path,
    public_key: &str,
    signature: &minisign_verify::Signature,
) -> Result<()> {
    use std::io::Read;
    let pk = minisign_verify::PublicKey::from_base64(public_key)
        .map_err(|e| Error::UpdateRejected(format!("clave pública no válida: {e}")))?;
    let mut verifier = pk
        .verify_stream(signature)
        .map_err(|e| Error::UpdateRejected(format!("firma no válida: {e}")))?;
    let mut file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        verifier.update(&buf[..n]);
    }
    verifier
        .finalize()
        .map_err(|e| Error::UpdateRejected(format!("firma no válida: {e}")))
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| Error::io(path, e))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use wiremock::matchers::{method, path as url_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/tests/fixtures/update/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn fixture_str(name: &str) -> String {
        String::from_utf8(fixture(name)).unwrap()
    }

    fn test_key() -> String {
        fixture_str("test.pub").trim().to_owned()
    }

    const PAYLOAD_SHA256: &str = "9f4a004dc45ce42d7e5d4a5e7915eb77a050881cf66cf6ea588a75c59c553286";

    async fn core(dir: &Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.join("datos")),
            secrets: Some(std::sync::Arc::new(crate::secrets::MemoryStore::default())),
            ..Options::default()
        })
        .await
        .unwrap()
    }

    fn appimage_info(path: PathBuf, url: String, signature: &str) -> UpdateInfo {
        UpdateInfo {
            version: "0.2.0".into(),
            current: "0.1.0.r5".into(),
            pub_date: None,
            notes: None,
            notes_url: None,
            package: Package::AppImage { path },
            artifact: Some(Artifact {
                url,
                size: fixture("payload.AppImage").len() as u64,
                sha256: PAYLOAD_SHA256.into(),
                signature: fixture_str(signature),
            }),
        }
    }

    #[test]
    fn versiones_con_revision() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.1.0.r23") > v("0.1.0.r9"));
        assert!(v("0.1.0.r1") > v("0.1.0"));
        assert!(v("0.2.0") > v("0.1.0.r99"));
        assert_eq!(v("v0.1.0+1"), v("0.1.0"));
        for bad in ["", "0.1", "0.1.0.23", "0.1.0.r", "0.1.0.r1.x", "a.b.c"] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn clave_de_release_valida() {
        assert!(minisign_verify::PublicKey::from_base64(PUBLIC_KEY).is_ok());
        let in_repo = std::fs::read_to_string(format!(
            "{}/../tools/release/minisign.pub",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert_eq!(in_repo.lines().last().unwrap().trim(), PUBLIC_KEY);
    }

    #[tokio::test]
    async fn consulta_el_manifiesto() {
        let server = MockServer::start().await;
        let key = Package::AppImage { path: "/x".into() }
            .artifact_key()
            .unwrap();
        let manifest = serde_json::json!({
            "schema": 1, "version": "0.2.0", "notes": "Novedades",
            "files": { key.clone(): {
                "url": format!("{}/a", server.uri()), "size": 1, "sha256": "00", "signature": "x"
            }}
        });
        Mock::given(method("GET"))
            .and(url_path("/latest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&manifest))
            .mount(&server)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        // SAFETY: solo esta prueba usa la variable.
        unsafe { std::env::set_var(MANIFEST_URL_ENV, format!("{}/latest.json", server.uri())) };

        let app = Package::AppImage { path: "/x".into() };
        let info = core.check_update("0.1.0.r5", &app).await.unwrap().unwrap();
        assert_eq!(info.version, "0.2.0");
        assert!(info.can_install());
        assert!(core.check_update("0.2.0", &app).await.unwrap().is_none());
        assert!(core.check_update("0.2.0.r1", &app).await.unwrap().is_none());

        let dev = core
            .check_update("0.1.0", &Package::Unpackaged)
            .await
            .unwrap()
            .unwrap();
        assert!(!dev.can_install());
        assert_eq!(dev.download_url(), DOWNLOAD_PAGE);
        unsafe { std::env::remove_var(MANIFEST_URL_ENV) };
    }

    #[tokio::test]
    async fn instala_el_appimage_firmado() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(url_path("/Shard.AppImage"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("payload.AppImage")))
            .mount(&server)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let app = tmp.path().join("Shard.AppImage");
        std::fs::write(&app, "versión vieja").unwrap();
        let url = format!("{}/Shard.AppImage", server.uri());

        let info = appimage_info(app.clone(), url, "payload.minisig");
        let out = core
            .install_update_with_key(&info, &test_key())
            .await
            .unwrap();
        assert_eq!(out, app);
        assert_eq!(std::fs::read(&app).unwrap(), fixture("payload.AppImage"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&app).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
        // Sin restos del archivo temporal.
        let leftovers: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".update"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[tokio::test]
    async fn rechaza_firmas_ajenas_o_de_otra_version() {
        let server = MockServer::start().await;
        let mut tampered = fixture("payload.AppImage");
        tampered[20] ^= 1;
        Mock::given(method("GET"))
            .and(url_path("/bueno"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("payload.AppImage")))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(url_path("/alterado"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(tampered.clone()))
            .mount(&server)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let app = tmp.path().join("Shard.AppImage");
        std::fs::write(&app, "versión vieja").unwrap();
        let reject = |r: Result<PathBuf>| match r {
            Err(Error::UpdateRejected(_)) => {}
            other => panic!("se esperaba rechazo: {other:?}"),
        };

        // Firma de otra versión (comentario confiable distinto).
        let info = appimage_info(
            app.clone(),
            format!("{}/bueno", server.uri()),
            "payload-0.1.9.minisig",
        );
        reject(core.install_update_with_key(&info, &test_key()).await);

        // Firma válida, pero con la clave de releases y no la de prueba.
        let info = appimage_info(
            app.clone(),
            format!("{}/bueno", server.uri()),
            "payload.minisig",
        );
        reject(core.install_update_with_key(&info, PUBLIC_KEY).await);

        // Archivo alterado con el SHA-256 que le corresponde: lo para la firma.
        let mut info = appimage_info(
            app.clone(),
            format!("{}/alterado", server.uri()),
            "payload.minisig",
        );
        {
            use sha2::Digest;
            info.artifact.as_mut().unwrap().sha256 = hex::encode(sha2::Sha256::digest(&tampered));
        }
        reject(core.install_update_with_key(&info, &test_key()).await);

        assert_eq!(std::fs::read_to_string(&app).unwrap(), "versión vieja");
        let leftovers = std::fs::read_dir(tmp.path())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains(".update")
            })
            .count();
        assert_eq!(leftovers, 0);
    }
}
