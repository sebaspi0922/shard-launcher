//! ARB (los `.arb` de Flutter, `app/lib/l10n`) → catálogos gettext `.po`
//! que Slint empaqueta con `@tr`. Lo usa `build.rs` (con `#[path]`) y los
//! tests; no depende de nada de la app.
//!
//! Convención: el `msgid` es la clave del ARB (`@tr("instancesTitle")`).
//! - `{nombre}` pasa a `{0}`, `{1}`… en el orden en que aparece por primera
//!   vez en la plantilla (`app_es.arb`, igual que `l10n.yaml`). En Slint los
//!   argumentos van en ese orden: `@tr("createDone", name)`.
//! - `{n, plural, =1{…} one{…} other{…}}` pasa a entrada plural; la variable
//!   del plural es `{n}`: `@tr("instancesCount" | "instancesCount" % total)`.
//! - `=0{…}` no existe en gettext: sale como otra clave, `<clave>.zero`.

use serde_json::Value;

/// Un mensaje ya convertido a la sintaxis de `@tr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Simple(String),
    Plural { one: String, other: String },
}

/// `(clave, mensaje ICU)` en el orden del archivo, sin metadatos (`@…`).
pub fn parse_arb(json: &str) -> Result<Vec<(String, String)>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let obj = v.as_object().ok_or("el ARB no es un objeto")?;
    Ok(obj
        .iter()
        .filter(|(k, _)| !k.starts_with('@'))
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
        .collect())
}

/// Un trozo de un mensaje ICU de primer nivel.
#[derive(Debug)]
enum Part<'a> {
    Text(&'a str),
    Arg(&'a str),
    /// (variable, [(selector, texto)])
    Plural(&'a str, Vec<(&'a str, &'a str)>),
}

/// Índice de la `}` que cierra la `{` de `s[open]`.
fn matching(s: &str, open: usize) -> Result<usize, String> {
    let mut depth = 0;
    for (i, ch) in s[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(open + i);
                }
            }
            _ => {}
        }
    }
    Err(format!("llave sin cerrar en «{s}»"))
}

fn parts(msg: &str) -> Result<Vec<Part<'_>>, String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = msg[pos..].find('{') {
        let open = pos + rel;
        if open > pos {
            out.push(Part::Text(&msg[pos..open]));
        }
        let close = matching(msg, open)?;
        let inner = &msg[open + 1..close];
        let mut fields = inner.splitn(3, ',');
        let var = fields.next().unwrap_or("").trim();
        match (fields.next().map(str::trim), fields.next()) {
            (None, _) => out.push(Part::Arg(var)),
            (Some("plural"), Some(cases)) => out.push(Part::Plural(var, plural_cases(cases)?)),
            (Some(kind), _) => return Err(format!("formato ICU no soportado: {kind} en «{msg}»")),
        }
        pos = close + 1;
    }
    if pos < msg.len() {
        out.push(Part::Text(&msg[pos..]));
    }
    Ok(out)
}

fn plural_cases(s: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = s[pos..].find('{') {
        let open = pos + rel;
        let selector = s[pos..open].trim();
        let close = matching(s, open)?;
        out.push((selector, &s[open + 1..close]));
        pos = close + 1;
    }
    Ok(out)
}

/// Nombres de los argumentos (sin la variable del plural) en orden de
/// aparición. Se saca de la plantilla para que todos los idiomas compartan
/// el mismo orden de argumentos.
pub fn arg_order(msg: &str) -> Result<Vec<String>, String> {
    fn walk(msg: &str, plural_var: Option<&str>, out: &mut Vec<String>) -> Result<(), String> {
        for p in parts(msg)? {
            match p {
                Part::Text(_) => {}
                Part::Arg(a) => {
                    if Some(a) != plural_var && !out.iter().any(|o| o == a) {
                        out.push(a.to_owned());
                    }
                }
                Part::Plural(var, cases) => {
                    for (_, text) in cases {
                        walk(text, Some(var), out)?;
                    }
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(msg, None, &mut out)?;
    Ok(out)
}

/// Sustituye `{arg}` por `{i}` (o `{n}` si es la variable del plural).
fn render(msg: &str, order: &[String], plural_var: Option<&str>) -> Result<String, String> {
    let mut out = String::new();
    for p in parts(msg)? {
        match p {
            Part::Text(t) => out.push_str(t),
            Part::Arg(a) if Some(a) == plural_var => out.push_str("{n}"),
            Part::Arg(a) => {
                let i = order
                    .iter()
                    .position(|o| o == a)
                    .ok_or_else(|| format!("argumento «{a}» que no está en la plantilla"))?;
                out.push_str(&format!("{{{i}}}"));
            }
            Part::Plural(..) => return Err(format!("plural anidado en «{msg}»")),
        }
    }
    Ok(out)
}

/// Convierte un mensaje. Devuelve `(sufijo de clave, mensaje)`: `""` para
/// la clave y `".zero"` para el caso `=0`.
pub fn convert(msg: &str, order: &[String]) -> Result<Vec<(&'static str, Msg)>, String> {
    let ps = parts(msg)?;
    let Some(pi) = ps.iter().position(|p| matches!(p, Part::Plural(..))) else {
        return Ok(vec![("", Msg::Simple(render(msg, order, None)?))]);
    };
    let Part::Plural(var, cases) = &ps[pi] else { unreachable!() };
    // El texto de fuera del plural (prefijo y sufijo) se copia en cada forma.
    let piece = |range: std::ops::Range<usize>| -> String {
        ps[range]
            .iter()
            .map(|p| match p {
                Part::Text(t) => (*t).to_owned(),
                Part::Arg(a) => format!("{{{a}}}"),
                Part::Plural(..) => String::new(),
            })
            .collect()
    };
    let (pre, post) = (piece(0..pi), piece(pi + 1..ps.len()));
    let form = |sel: &[&str]| -> Result<Option<String>, String> {
        cases
            .iter()
            .find(|(s, _)| sel.contains(s))
            .map(|(_, text)| render(&format!("{pre}{text}{post}"), order, Some(var)))
            .transpose()
    };
    let other = form(&["other"])?.ok_or_else(|| format!("plural sin «other» en «{msg}»"))?;
    let one = form(&["one", "=1"])?.unwrap_or_else(|| other.clone());
    let mut out = vec![("", Msg::Plural { one, other })];
    if let Some(zero) = form(&["=0"])? {
        out.push((".zero", Msg::Simple(zero)));
    }
    Ok(out)
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Catálogo `.po` de un idioma. `template` fija el orden de argumentos.
pub fn to_po(lang_arb: &str, template_arb: &str) -> Result<String, String> {
    let template = parse_arb(template_arb)?;
    let msgs = parse_arb(lang_arb)?;
    let mut po = String::from(
        "msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\
         \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
    );
    for (key, msg) in &msgs {
        let order = match template.iter().find(|(k, _)| k == key) {
            Some((_, t)) => arg_order(t)?,
            None => arg_order(msg)?,
        };
        for (suffix, m) in convert(msg, &order).map_err(|e| format!("{key}: {e}"))? {
            let id = quote(&format!("{key}{suffix}"));
            match m {
                Msg::Simple(s) => po.push_str(&format!("\nmsgid {id}\nmsgstr {}\n", quote(&s))),
                Msg::Plural { one, other } => po.push_str(&format!(
                    "\nmsgid {id}\nmsgid_plural {id}\nmsgstr[0] {}\nmsgstr[1] {}\n",
                    quote(&one),
                    quote(&other)
                )),
            }
        }
    }
    Ok(po)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(msg: &str) -> Vec<(&'static str, Msg)> {
        convert(msg, &arg_order(msg).unwrap()).unwrap()
    }

    #[test]
    fn simple_y_argumentos() {
        assert_eq!(conv("Instancias"), [("", Msg::Simple("Instancias".into()))]);
        assert_eq!(conv("Instancia «{name}» creada"), [("", Msg::Simple("Instancia «{0}» creada".into()))]);
        assert_eq!(conv("{a} y {b} y {a}"), [("", Msg::Simple("{0} y {1} y {0}".into()))]);
    }

    #[test]
    fn plural_con_cero_y_texto_alrededor() {
        let m = "{count, plural, =0{Nada} =1{1 instancia · {size}} other{{count} instancias · {size}}} · usa {shared}";
        assert_eq!(arg_order(m).unwrap(), ["size", "shared"]);
        assert_eq!(
            conv(m),
            [
                (
                    "",
                    Msg::Plural {
                        one: "1 instancia · {0} · usa {1}".into(),
                        other: "{n} instancias · {0} · usa {1}".into()
                    }
                ),
                (".zero", Msg::Simple("Nada · usa {1}".into())),
            ]
        );
    }

    #[test]
    fn orden_de_argumentos_de_la_plantilla() {
        // En inglés el orden cambia, pero los índices salen de la plantilla.
        let es = r#"{"k": "{count, plural, other{{count} omitidas}} al instalar {name}"}"#;
        let en = r#"{"k": "{name} installed, but {count, plural, =1{1 was} other{{count} were}} skipped"}"#;
        let po = to_po(en, es).unwrap();
        assert!(po.contains("msgstr[0] \"{0} installed, but 1 was skipped\""), "{po}");
        assert!(po.contains("msgstr[1] \"{0} installed, but {n} were skipped\""), "{po}");
    }

    #[test]
    fn los_arb_reales_se_convierten_enteros() {
        let es = include_str!("../../app/lib/l10n/app_es.arb");
        let en = include_str!("../../app/lib/l10n/app_en.arb");
        for lang in [es, en] {
            let po = to_po(lang, es).unwrap();
            assert!(po.contains("msgid \"instancesTitle\""));
        }
        // Mismas claves en los dos idiomas.
        let keys = |s| parse_arb(s).unwrap().into_iter().map(|(k, _)| k).collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys(es), keys(en));
    }
}
