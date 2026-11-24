//! Lógica de la pantalla Instancias, sin Slint ni núcleo: orden «recientes»,
//! búsqueda y estado de lanzamiento por instancia. Rust es la fuente de
//! verdad; `controller` copia el resultado a los modelos de Slint.

use std::collections::HashMap;

/// Lo que la pantalla necesita de una instancia del núcleo.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub created_at: i64,
    pub last_played: Option<i64>,
    pub playtime_seconds: u64,
}

impl From<lcore::instance::Instance> for Record {
    fn from(i: lcore::instance::Instance) -> Self {
        Self {
            id: i.id,
            name: i.name,
            mc: i.game.mc,
            loader: i.game.loader.as_str().to_owned(),
            created_at: i.created_at,
            last_played: i.last_played,
            playtime_seconds: i.playtime_seconds,
        }
    }
}

/// `recentInstancesProvider`: última partida primero, «nunca» al final y,
/// a igualdad, la creada más tarde primero.
pub fn sort_recent(list: &mut [Record]) {
    list.sort_by(|a, b| {
        let (la, lb) = (a.last_played.unwrap_or(-1), b.last_played.unwrap_or(-1));
        lb.cmp(&la).then(b.created_at.cmp(&a.created_at))
    });
}

/// `_matches` de instances_page.dart: cada palabra de la búsqueda tiene que
/// estar en «nombre versión loader» (sin mayúsculas).
pub fn matches(r: &Record, query: &str, loader_label: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let hay = format!("{} {} {}", r.name, r.mc, loader_label).to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w))
}

/// Nombre visible del loader (`loaderName`); `vanilla` y desconocidos van
/// traducidos.
pub fn loader_label(lang: &str, loader: &str) -> String {
    match loader {
        "vanilla" => crate::i18n::text(lang, "loaderVanilla"),
        "fabric" => "Fabric".into(),
        "quilt" => "Quilt".into(),
        "forge" => "Forge".into(),
        "neoforge" => "NeoForge".into(),
        "labymod" => "LabyMod".into(),
        _ => crate::i18n::text(lang, "loaderCustom"),
    }
}

/// Fase de lanzamiento de una instancia (core/launch.dart).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Preparing,
    Downloading { done: u64, total: u64 },
    Starting,
    Running,
    Error,
}

/// Estado de la pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Load {
    #[default]
    Loading,
    Ready,
    Failed { message: String, detail: String },
}

#[derive(Debug, Default)]
pub struct Model {
    pub load: Load,
    /// Ya en orden «recientes».
    pub all: Vec<Record>,
    pub query: String,
    pub grid: bool,
    pub launches: HashMap<String, Phase>,
}

impl Model {
    pub fn set_all(&mut self, mut list: Vec<Record>) {
        sort_recent(&mut list);
        self.all = list;
        self.load = Load::Ready;
    }

    pub fn visible(&self, lang: &str) -> Vec<&Record> {
        self.all.iter().filter(|r| matches(r, &self.query, &loader_label(lang, &r.loader))).collect()
    }

    pub fn phase(&self, id: &str) -> Phase {
        self.launches.get(id).copied().unwrap_or_default()
    }

    /// La primera instancia con un lanzamiento en marcha (píldora del título).
    pub fn active(&self) -> Option<(&Record, Phase)> {
        self.all.iter().find_map(|r| match self.phase(&r.id) {
            Phase::Idle | Phase::Error => None,
            p => Some((r, p)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, name: &str, last: Option<i64>, created: i64) -> Record {
        Record {
            id: id.into(),
            name: name.into(),
            mc: "1.21.4".into(),
            loader: "fabric".into(),
            created_at: created,
            last_played: last,
            playtime_seconds: 0,
        }
    }

    #[test]
    fn recientes_primero_y_nunca_al_final() {
        let mut v = vec![rec("a", "A", None, 5), rec("b", "B", Some(10), 1), rec("c", "C", Some(20), 1), rec("d", "D", None, 9)];
        sort_recent(&mut v);
        let ids: Vec<_> = v.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["c", "b", "d", "a"]);
    }

    #[test]
    fn busqueda_por_palabras_en_nombre_version_y_loader() {
        let r = rec("s", "Survival con amigos", None, 0);
        assert!(matches(&r, "", "Fabric"));
        assert!(matches(&r, "  SUR  ", "Fabric"));
        assert!(matches(&r, "amigos fabric", "Fabric"));
        assert!(matches(&r, "1.21", "Fabric"));
        assert!(!matches(&r, "amigos forge", "Fabric"));
    }

    #[test]
    fn activa_es_la_primera_en_marcha() {
        let mut m = Model::default();
        m.set_all(vec![rec("a", "A", Some(2), 0), rec("b", "B", Some(1), 0)]);
        assert_eq!(m.active(), None);
        m.launches.insert("b".into(), Phase::Running);
        m.launches.insert("a".into(), Phase::Error);
        assert_eq!(m.active().map(|(r, p)| (r.id.as_str(), p)), Some(("b", Phase::Running)));
    }
}
