//! Qt Linguist `.ts` catalog. `QApp` has no `QTranslator`; chrome looks up
//! source strings here.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const TS: &[(&str, &str)] = &[
    (
        "en",
        include_str!("../../../resources/translations/circuitsimulator_en.ts"),
    ),
    (
        "tr",
        include_str!("../../../resources/translations/circuitsimulator_tr.ts"),
    ),
];

struct Catalog {
    locale: String,
    map: HashMap<String, String>,
}

impl Catalog {
    fn load(locale: &str) -> Self {
        let src = TS
            .iter()
            .find(|(c, _)| *c == locale)
            .map(|(_, s)| *s)
            .unwrap_or("");
        Self {
            locale: locale.into(),
            map: parse_ts(src),
        }
    }
}

fn state() -> &'static Mutex<Catalog> {
    static STATE: OnceLock<Mutex<Catalog>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(Catalog::load("en")))
}

pub fn set_locale(locale: &str) {
    let loc = if TS.iter().any(|(c, _)| *c == locale) {
        locale
    } else {
        "en"
    };
    let mut g = state().lock().expect("i18n lock");
    if g.locale == loc {
        return;
    }
    *g = Catalog::load(loc);
}

pub fn locale() -> String {
    state().lock().expect("i18n lock").locale.clone()
}

/// Look up `source` (the English `qsTr` / `tr()` string). Missing entries
/// return the source unchanged.
pub fn tr(source: &str) -> String {
    let g = state().lock().expect("i18n lock");
    g.map
        .get(source)
        .cloned()
        .unwrap_or_else(|| source.to_string())
}

/// No-op macro that marks a static string literal for translation extraction tooling.
/// Expands directly to the expression at compile time.
#[macro_export]
macro_rules! tr_noop {
    ($s:expr) => {
        $s
    };
}

fn parse_ts(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut rest = src;
    while let Some(start) = rest.find("<message") {
        let after = &rest[start..];
        let Some(end) = after.find("</message>") else {
            break;
        };
        let block = &after[..end];
        rest = &after[end + 10..];
        if translation_attr(block, "vanished") {
            continue;
        }
        let Some(source) = extract_tag(block, "source") else {
            continue;
        };
        let Some(translation) = extract_tag(block, "translation") else {
            continue;
        };
        if translation.is_empty() {
            continue;
        }
        map.insert(source, translation);
    }
    map
}

fn translation_attr(block: &str, attr: &str) -> bool {
    if let Some(i) = block.find("<translation") {
        if let Some(gt) = block[i..].find('>') {
            return block[i..i + gt].contains(&format!("type=\"{attr}\""));
        }
    }
    false
}

fn extract_tag(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let i = block.find(&open)?;
    let after = &block[i + open.len()..];
    let gt = after.find('>')?;
    let content = &after[gt + 1..];
    let close = format!("</{tag}>");
    let j = content.find(&close)?;
    Some(decode_entities(&content[..j]))
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        if let Some(end) = rest.find(';') {
            let ent = &rest[..end];
            rest = &rest[end + 1..];
            match ent {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                "nbsp" => out.push('\u{a0}'),
                other if other.starts_with('#') => {
                    let num = other.trim_start_matches('#');
                    let cp = if let Some(hex) = num.strip_prefix('x') {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        num.parse().ok()
                    };
                    if let Some(c) = cp.and_then(char::from_u32) {
                        out.push(c);
                    }
                }
                other => {
                    out.push('&');
                    out.push_str(other);
                    out.push(';');
                }
            }
        } else {
            out.push('&');
            out.push_str(rest);
            break;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn turkish_file_menu() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_locale("tr");
        assert_eq!(tr("&File"), "&Dosya");
        assert_ne!(tr("Settings"), "Settings");
        assert_eq!(tr("this is not a key"), "this is not a key");
        set_locale("en");
        // English catalog is identity or close; missing still returns source.
        let s = tr("&File");
        assert!(s.contains("File"), "{s}");
    }

    #[test]
    fn turkish_status_messages() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_locale("tr");
        assert_eq!(tr("Running"), "Çalışıyor");
        assert_eq!(tr("Paused"), "Duraklatıldı");
        assert_eq!(tr("Stopped"), "Durduruldu");
        assert_eq!(
            tr("NonLinear Not Converging"),
            "Doğrusal Olmayan Yakınsamıyor"
        );
        set_locale("en");
        assert_eq!(tr("Running"), "Running");
        assert_eq!(tr("NonLinear Not Converging"), "NonLinear Not Converging");
    }

    #[test]
    fn entities() {
        assert_eq!(decode_entities("&amp;File"), "&File");
        assert_eq!(decode_entities("A &#xa; B"), "A \n B");
    }
}
