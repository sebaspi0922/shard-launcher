//! Lógica de Inicio (`app/lib/ui/home/home_page.dart`), sin Slint ni núcleo:
//! instancia de la tarjeta grande, recientes y servidores rápidos con su
//! estado de ping.

use crate::instances::Record;

/// `homeInstanceProvider`: la elegida si sigue existiendo; si no, la última
/// jugada. `recent` ya viene en orden «recientes».
pub fn home_instance<'a>(recent: &'a [Record], selected: Option<&str>) -> Option<&'a Record> {
    selected.and_then(|id| recent.iter().find(|r| r.id == id)).or_else(|| recent.first())
}

/// Las tres siguientes a la de la tarjeta grande.
pub fn recent_cards<'a>(recent: &'a [Record], home: Option<&str>) -> Vec<&'a Record> {
    recent.iter().filter(|r| Some(r.id.as_str()) != home).take(3).collect()
}

/// Servidor guardado (lo que Inicio necesita de `lcore::servers::Server`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub address: String,
    pub instance_id: Option<String>,
    pub account_id: Option<String>,
}

impl From<lcore::servers::Server> for Server {
    fn from(s: lcore::servers::Server) -> Self {
        Self { id: s.id, name: s.name, address: s.address, instance_id: s.instance_id, account_id: s.account_id }
    }
}

/// Estado en vivo de una dirección.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Status {
    pub online: bool,
    pub latency_ms: u64,
    pub players_online: u32,
    pub players_max: u32,
    pub favicon_png_base64: Option<String>,
}

impl From<lcore::servers::ServerStatus> for Status {
    fn from(s: lcore::servers::ServerStatus) -> Self {
        Self {
            online: s.online,
            latency_ms: s.latency_ms,
            players_online: s.players_online,
            players_max: s.players_max,
            favicon_png_base64: s.favicon_png_base64,
        }
    }
}

/// Barras de señal (`PingBars`): 0 sin conexión, 4 = menos de 60 ms.
pub fn ping_level(s: Option<&Status>) -> i32 {
    match s {
        Some(s) if s.online => match s.latency_ms {
            0..60 => 4,
            60..120 => 3,
            120..250 => 2,
            _ => 1,
        },
        _ => 0,
    }
}

/// `NumberFormat.compact` de intl para es y en (solo se usa desde 10 000):
/// tres cifras significativas, «mil»/«M» en español y «K»/«M»/«B» en inglés.
pub fn compact(lang: &str, n: u64) -> String {
    let es = lang == "es";
    let (div, suffix) = match n {
        0..1_000 => return n.to_string(),
        1_000..1_000_000 => (1e3, if es { "\u{a0}mil" } else { "K" }),
        1_000_000..1_000_000_000 => (1e6, if es { "\u{a0}M" } else { "M" }),
        _ => (1e9, if es { "\u{a0}mil\u{a0}M" } else { "B" }),
    };
    let v = n as f64 / div;
    let decimals = if v >= 100.0 { 0 } else if v >= 10.0 { 1 } else { 2 };
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    if es {
        s = s.replace('.', ",");
    }
    format!("{s}{suffix}")
}

/// `serverPlayers` + latencia: «3 / 20 · 23 ms», «Sin conexión» o «…».
pub fn status_line(lang: &str, s: Option<&Status>) -> String {
    let t = |k| crate::i18n::text(lang, k);
    let Some(s) = s else { return "…".into() };
    if !s.online {
        return t("serverOffline");
    }
    let n = |v: u32| if v >= 10_000 { compact(lang, v as u64) } else { v.to_string() };
    let players = t("serverPlayersText").replace("{online}", &n(s.players_online)).replace("{max}", &n(s.players_max));
    format!("{players} · {}", t("serverPing").replace("{ms}", &s.latency_ms.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str) -> Record {
        Record { id: id.into(), name: id.into(), mc: "1.21".into(), loader: "vanilla".into(), created_at: 0, last_played: None, playtime_seconds: 0 }
    }

    #[test]
    fn la_elegida_o_la_ultima_jugada() {
        let v = vec![rec("a"), rec("b"), rec("c"), rec("d"), rec("e")];
        assert_eq!(home_instance(&v, None).unwrap().id, "a");
        assert_eq!(home_instance(&v, Some("c")).unwrap().id, "c");
        assert_eq!(home_instance(&v, Some("borrada")).unwrap().id, "a");
        assert!(home_instance(&[], Some("a")).is_none());
        let ids: Vec<_> = recent_cards(&v, Some("c")).iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "d"]);
    }

    #[test]
    fn numeros_compactos_como_intl() {
        assert_eq!(compact("es", 31_842), "31,8\u{a0}mil");
        assert_eq!(compact("es", 200_000), "200\u{a0}mil");
        assert_eq!(compact("en", 31_842), "31.8K");
        assert_eq!(compact("en", 200_000), "200K");
        assert_eq!(compact("en", 1_234_567), "1.23M");
        assert_eq!(compact("en", 12_000), "12K");
    }

    #[test]
    fn linea_de_estado() {
        let s = Status { online: true, latency_ms: 118, players_online: 31_842, players_max: 200_000, favicon_png_base64: None };
        assert_eq!(status_line("es", Some(&s)), "31,8\u{a0}mil / 200\u{a0}mil · 118 ms");
        assert_eq!(status_line("en", Some(&s)), "31.8K / 200K · 118 ms");
        assert_eq!(status_line("es", Some(&Status::default())), "Sin conexión");
        assert_eq!(status_line("es", None), "…");
        assert_eq!(ping_level(Some(&s)), 3);
        assert_eq!(ping_level(None), 0);
    }
}
