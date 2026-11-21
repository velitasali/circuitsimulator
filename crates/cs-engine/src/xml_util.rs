//! Format-neutral XML attribute, coordinate, and digital family parsing helpers
//! shared by native `.circ1`, package files, and the SimulIDE compatibility layer.

use crate::digital::LogicFamily;
use crate::units::parse_si;

/// Parse semicolon-separated `key=value;` attributes (used in `.package` pin specs).
pub fn parse_props(line: &str) -> Vec<(String, String)> {
    line.split(';')
        .filter_map(|p| {
            let trimmed = p.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut kv = trimmed.splitn(2, '=');
            let k = kv.next()?.trim();
            let v = kv.next().map(str::trim).unwrap_or("");
            if k.is_empty() {
                None
            } else {
                Some((k.to_string(), v.to_string()))
            }
        })
        .collect()
}

/// Parse XML `key="value"` attributes from a single `<item ... />` or `<circuit ... >` line.
/// Matches C++ `parseXmlProps`: split on `"`, attribute name is the token after the last space
/// with the trailing `=` dropped.
pub fn parse_xml_props(line: &str) -> Vec<(String, String)> {
    let mut tokens: Vec<&str> = line.split('"').collect();
    if tokens.len() < 2 {
        return Vec::new();
    }
    tokens.pop();
    let mut properties = Vec::new();
    let mut name = String::new();
    for token in tokens {
        if name.is_empty() {
            let start = token.rfind(' ').map(|i| i + 1).unwrap_or(0);
            let mut n = token[start..].to_string();
            if n.ends_with('=') {
                n.pop();
            }
            name = n;
        } else {
            properties.push((name.clone(), token.to_string()));
            name.clear();
        }
    }
    properties
}

pub fn prop<'a>(properties: &'a [(String, String)], name: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

pub fn parse_bool(v: &str) -> bool {
    v == "true" || v == "1"
}

pub fn parse_flip(v: Option<&str>) -> i32 {
    match v.and_then(|s| s.parse::<i32>().ok()) {
        Some(-1) => -1,
        _ => 1,
    }
}

pub fn parse_point(val: &str) -> Option<(f64, f64)> {
    let mut parts = val.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    Some((x, y))
}

pub fn parse_point_list(val: &str) -> Vec<(f64, f64)> {
    let nums: Vec<f64> = val
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    nums.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0], c[1]))
            } else {
                None
            }
        })
        .collect()
}

/// Format a number that C++ `QString::number` would emit for a grid point.
pub fn fmt_coord(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v}")
    }
}

pub fn fmt_pos(x: f64, y: f64) -> String {
    format!("{},{}", fmt_coord(x), fmt_coord(y))
}

pub fn fmt_point_list(points: &[(f64, f64)]) -> String {
    points
        .iter()
        .flat_map(|(x, y)| [fmt_coord(*x), fmt_coord(*y)])
        .collect::<Vec<_>>()
        .join(",")
}

/// Apply optional per-gate digital family timing and voltage level attributes.
pub(crate) fn apply_family_props(properties: &[(String, String)], family: &mut LogicFamily) {
    if let Some(v) = prop(properties, "Tpd_ps") {
        family.set_prop_delay_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, "pd_n") {
        family.delay_mult = parse_si(v, "");
    }
    if let Some(v) = prop(properties, "Tr_ps") {
        family.set_rise_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, "Tf_ps") {
        family.set_fall_s(parse_si(v, "s"));
    }
    if let Some(v) = prop(properties, "Out_High_V") {
        family.out_high_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, "Out_Low_V") {
        family.out_low_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, "Out_Imped") {
        family.out_imp = parse_si(v, "Ω").max(1e-14);
    }
    if let Some(v) = prop(properties, "Input_High_V") {
        family.inp_high_v = parse_si(v, "V");
    }
    if let Some(v) = prop(properties, "Input_Low_V") {
        family.inp_low_v = parse_si(v, "V");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_props() {
        let p = parse_xml_props(
            r#"<item itemtype="Resistor" CircId="Resistor-1" Resistance="1 kΩ" />"#,
        );
        assert_eq!(p[0], ("itemtype".into(), "Resistor".into()));
        assert_eq!(p[1], ("CircId".into(), "Resistor-1".into()));
        assert_eq!(p[2], ("Resistance".into(), "1 kΩ".into()));
    }

    #[test]
    fn semicolon_props() {
        let p = parse_props("Pin; type=; xpos=-8; ypos=8; id=in");
        assert_eq!(p[0], ("Pin".into(), String::new()));
        assert_eq!(p[1], ("type".into(), String::new()));
        assert_eq!(p[2], ("xpos".into(), "-8".into()));
        assert_eq!(p[4], ("id".into(), "in".into()));
    }

    #[test]
    fn point_list() {
        assert_eq!(
            parse_point_list("100,-100,148,-100"),
            vec![(100.0, -100.0), (148.0, -100.0)]
        );
        assert_eq!(parse_point("200,-100"), Some((200.0, -100.0)));
    }
}
