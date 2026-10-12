//! Utilidades de copia compartidas por los importadores de `.minecraft` y de
//! Prism/MultiMC: recorrido sin seguir enlaces simbólicos, copia con
//! progreso y reserva de ids.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::events::{Event, EventBus, ProgressUnit};
use crate::instance::slugify;
use crate::paths::Paths;
use crate::{Error, Result};

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1 << 40);

/// Tamaño máximo de los JSON/INI pequeños que se leen de otro launcher.
pub(crate) const MAX_META_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// Ruta relativa a la raíz del origen.
    pub rel: PathBuf,
    pub is_dir: bool,
}

/// Lista de entradas a copiar con sus totales.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tree {
    pub entries: Vec<Entry>,
    pub files: u64,
    pub bytes: u64,
}

impl Tree {
    pub fn push_dir(&mut self, rel: PathBuf) {
        self.entries.push(Entry { rel, is_dir: true });
    }

    pub fn push_file(&mut self, rel: PathBuf, size: u64) {
        self.files += 1;
        self.bytes += size;
        self.entries.push(Entry { rel, is_dir: false });
    }

    /// Añade `root/rel` (archivo o carpeta) a la lista. Los enlaces simbólicos
    /// se omiten con un aviso: la importación copia, no enlaza ni sale del
    /// origen. Devuelve `false` si `rel` no existe.
    pub fn add_path(
        &mut self,
        root: &Path,
        rel: &Path,
        warnings: &mut Vec<String>,
    ) -> io::Result<bool> {
        let path = root.join(rel);
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            warnings.push(format!("enlace simbólico, se omite: {}", rel.display()));
            return Ok(false);
        }
        if ft.is_file() {
            self.push_file(rel.to_path_buf(), meta.len());
        } else if ft.is_dir() {
            self.push_dir(rel.to_path_buf());
            self.add_dir(root, rel, warnings)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// Contenido de una carpeta, en orden alfabético y en preorden.
    pub fn add_dir(
        &mut self,
        root: &Path,
        rel_dir: &Path,
        warnings: &mut Vec<String>,
    ) -> io::Result<()> {
        let mut names: Vec<OsString> = fs::read_dir(root.join(rel_dir))?
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<_>>()?;
        names.sort();
        for name in names {
            let rel = rel_dir.join(&name);
            let meta = match fs::symlink_metadata(root.join(&rel)) {
                Ok(m) => m,
                // Desapareció mientras se recorría.
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                warnings.push(format!("enlace simbólico, se omite: {}", rel.display()));
            } else if ft.is_dir() {
                self.push_dir(rel.clone());
                self.add_dir(root, &rel, warnings)?;
            } else if ft.is_file() {
                self.push_file(rel, meta.len());
            }
        }
        Ok(())
    }
}

/// Copia un archivo conservando la fecha de modificación y los permisos (en
/// lo que se pueda).
pub(crate) fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let meta = input.metadata()?;
    let mut out = fs::File::create(dst)?;
    io::copy(&mut input, &mut out)?;
    if let Ok(modified) = meta.modified() {
        let _ = out.set_modified(modified);
    }
    drop(out);
    let _ = fs::set_permissions(dst, meta.permissions());
    Ok(())
}

/// Copia las entradas de `src_root` a `dst_root`. Devuelve los archivos creados.
pub(crate) fn copy_tree(
    src_root: &Path,
    dst_root: &Path,
    tree: &Tree,
    progress: &Progress,
) -> Result<u64> {
    fs::create_dir_all(dst_root).map_err(|e| Error::io(dst_root, e))?;
    let mut copied = 0;
    for entry in &tree.entries {
        let dst = dst_root.join(&entry.rel);
        if entry.is_dir {
            fs::create_dir_all(&dst).map_err(|e| Error::io(&dst, e))?;
        } else {
            let src = src_root.join(&entry.rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            copy_file(&src, &dst).map_err(|e| Error::io(&src, e))?;
            copied += 1;
            progress.tick(1);
        }
    }
    Ok(copied)
}

/// Lee un archivo pequeño (JSON, INI) rechazando los demasiado grandes.
pub(crate) fn read_capped(path: &Path, max: u64) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    fs::File::open(path)?.take(max + 1).read_to_end(&mut out)?;
    if out.len() as u64 > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "archivo demasiado grande",
        ));
    }
    Ok(out)
}

/// El origen y el destino no pueden contenerse entre sí.
pub(crate) fn guard_overlap(source: &Path, dest: &Path) -> Result<()> {
    let s = fs::canonicalize(source).map_err(|e| Error::io(source, e))?;
    let Ok(d) = fs::canonicalize(dest) else {
        return Ok(());
    };
    if s.starts_with(&d) || d.starts_with(&s) {
        return Err(Error::InvalidInput(format!(
            "el origen ({}) y el directorio de datos ({}) no pueden contenerse",
            source.display(),
            dest.display()
        )));
    }
    Ok(())
}

/// Id libre: como `unique_instance_id`, pero además descarta los ya reservados
/// en este mismo plan (todavía no existen en disco).
pub(crate) fn alloc_id(
    paths: &Paths,
    base: &str,
    reserved: &mut HashSet<String>,
) -> Result<String> {
    let base = slugify(base);
    let mut candidate = base.clone();
    let mut n = 2;
    loop {
        let taken = reserved.contains(&candidate)
            || paths.instance_dir(&candidate)?.exists()
            || paths.backups_dir().join(&candidate).exists();
        if !taken {
            reserved.insert(candidate.clone());
            return Ok(candidate);
        }
        candidate = format!("{base}-{n}");
        n += 1;
    }
}

/// Argumentos de JVM en lista, sin `-Xmx`/`-Xms` (los pone la memoria).
/// En blanco = `None` (valor global).
pub(crate) fn split_jvm_args(args: &str) -> Option<Vec<String>> {
    let args = args.trim();
    if args.is_empty() {
        return None;
    }
    Some(
        args.split_whitespace()
            .filter(|a| !a.starts_with("-Xmx") && !a.starts_with("-Xms"))
            .map(str::to_owned)
            .collect(),
    )
}

/// Progreso por archivos copiados, como evento `Progress` con etapa `import`.
pub(crate) struct Progress {
    events: EventBus,
    pub task_id: u64,
    total: u64,
    done: AtomicU64,
}

impl Progress {
    pub fn new(events: EventBus, total: u64) -> Self {
        Self {
            events,
            task_id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            total,
            done: AtomicU64::new(0),
        }
    }

    pub fn tick(&self, n: u64) {
        let done = self.done.fetch_add(n, Ordering::Relaxed) + n;
        if done / 64 != (done - n) / 64 || done >= self.total {
            self.events.emit(Event::Progress {
                task_id: self.task_id,
                stage: "import".into(),
                done,
                total: self.total.max(done),
                unit: ProgressUnit::Files,
            });
        }
    }

    pub fn finish<T>(&self, result: &Result<T>) {
        self.events.emit(Event::TaskFinished {
            task_id: self.task_id,
            ok: result.is_ok(),
            message: result.as_ref().err().map(|e| e.to_string()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jvm_args_sin_memoria() {
        assert_eq!(split_jvm_args("   "), None);
        assert_eq!(
            split_jvm_args("-Xmx12G -Xms2G  -XX:+UseG1GC -Dx=1"),
            Some(vec!["-XX:+UseG1GC".to_owned(), "-Dx=1".to_owned()])
        );
        assert_eq!(split_jvm_args("-Xmx4G"), Some(vec![]));
    }

    #[cfg(unix)]
    #[test]
    fn el_recorrido_omite_enlaces() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("saves/a")).unwrap();
        fs::write(root.join("saves/a/level.dat"), b"12345").unwrap();
        fs::write(root.join("secreto.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(root.join("secreto.txt"), root.join("saves/enlace")).unwrap();
        std::os::unix::fs::symlink(root.join("saves"), root.join("mods")).unwrap();

        let mut warnings = Vec::new();
        let mut tree = Tree::default();
        assert!(
            tree.add_path(root, Path::new("saves"), &mut warnings)
                .unwrap()
        );
        assert!(
            !tree
                .add_path(root, Path::new("mods"), &mut warnings)
                .unwrap()
        );
        assert!(
            !tree
                .add_path(root, Path::new("no-existe"), &mut warnings)
                .unwrap()
        );
        assert_eq!((tree.files, tree.bytes), (1, 5));
        assert_eq!(warnings.len(), 2);
        let rels: Vec<_> = tree
            .entries
            .iter()
            .map(|e| e.rel.to_str().unwrap())
            .collect();
        assert_eq!(rels, ["saves", "saves/a", "saves/a/level.dat"]);
    }
}
