//! SI prefix parsing matching `getMultiplier` / `NumProp::getVal` in C++.

/// First character or word of the unit string determines the prefix.
pub fn si_multiplier(unit: &str) -> f64 {
    let u = unit.trim();
    if u.is_empty() {
        return 1.0;
    }
    if u == "m" {
        return 1.0;
    }
    let u_lower = u.to_ascii_lowercase();
    if u_lower.starts_with("meg") {
        return 1e6;
    }
    if u_lower.starts_with("cm") {
        return 1e-2;
    }
    let Some(c) = u.chars().next() else {
        return 1.0;
    };
    match c {
        'p' | 'P' => 1e-12,
        'n' | 'N' => 1e-9,
        'µ' | 'μ' | 'u' | 'U' => 1e-6,
        'm' => 1e-3,
        'k' | 'K' => 1e3,
        'M' => 1e6,
        'G' | 'g' => 1e9,
        'T' | 't' => 1e12,
        _ => 1.0,
    }
}

/// Format `val` with an SI prefix and `unit` (`"1 kΩ"`, `"5 V"`), with at most 2 fractional digits.
pub fn format_si(val: f64, unit: &str) -> String {
    format_si_precision(val, unit, 2)
}

/// Format `val` with an SI prefix and `unit`, with a configurable maximum number of decimal places.
pub fn format_si_precision(val: f64, unit: &str, max_decimals: usize) -> String {
    let a = val.abs();
    let (scaled, prefix) = if a >= 1e9 {
        (val / 1e9, "G")
    } else if a >= 1e6 {
        (val / 1e6, "M")
    } else if a >= 1e3 {
        (val / 1e3, "k")
    } else if a >= 1.0 || a == 0.0 {
        (val, "")
    } else if a >= 1e-3 {
        (val * 1e3, "m")
    } else if a >= 1e-6 {
        (val * 1e6, "µ")
    } else if a >= 1e-9 {
        (val * 1e9, "n")
    } else {
        (val * 1e12, "p")
    };
    let n = if (scaled - scaled.round()).abs() < 1e-9 {
        format!("{}", scaled.round() as i64)
    } else {
        let s = format!("{scaled:.prec$}", prec = max_decimals);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    if unit.is_empty() {
        if prefix.is_empty() {
            n
        } else {
            format!("{n} {prefix}")
        }
    } else if prefix.is_empty() {
        format!("{n} {unit}")
    } else {
        format!("{n} {prefix}{unit}")
    }
}

/// Format `val` with an SI prefix and `unit` without separating spaces (`"1kΩ"`, `"5V"`).
pub fn format_si_compact(val: f64, unit: &str) -> String {
    format_si_compact_precision(val, unit, 2)
}

/// Format `val` with an SI prefix and `unit` without separating spaces, with configurable decimals.
pub fn format_si_compact_precision(val: f64, unit: &str, max_decimals: usize) -> String {
    let a = val.abs();
    let (scaled, prefix) = if a >= 1e9 {
        (val / 1e9, "G")
    } else if a >= 1e6 {
        (val / 1e6, "M")
    } else if a >= 1e3 {
        (val / 1e3, "k")
    } else if a >= 1.0 || a == 0.0 {
        (val, "")
    } else if a >= 1e-3 {
        (val * 1e3, "m")
    } else if a >= 1e-6 {
        (val * 1e6, "µ")
    } else if a >= 1e-9 {
        (val * 1e9, "n")
    } else {
        (val * 1e12, "p")
    };
    let n = if (scaled - scaled.round()).abs() < 1e-9 {
        format!("{}", scaled.round() as i64)
    } else {
        let s = format!("{scaled:.prec$}", prec = max_decimals);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    format!("{n}{prefix}{unit}")
}

/// C++ `splitTime`: reduce picoseconds by thousands while they divide evenly.
pub fn split_time(ps: u64) -> (u64, i32) {
    let mut value = ps;
    let mut unit = 0i32;
    while unit < 4 && value >= 1000 && value % 1000 == 0 {
        value /= 1000;
        unit += 1;
    }
    (value, unit)
}

/// C++ `joinTime`.
pub fn join_time(value: u64, unit: i32) -> u64 {
    let mut mult = 1u64;
    for _ in 0..unit.clamp(0, 4) {
        mult = mult.saturating_mul(1000);
    }
    value.saturating_mul(mult)
}

/// Resolve the clean base unit without prefix or decoration (e.g. `"kΩ"` -> `"Ω"`, `"_px"` -> `"px"`, `"ns"` -> `"s"`).
pub fn base_unit_for(unit: &str) -> &str {
    let u = unit.trim();
    if let Some(rest) = u.strip_prefix('_') {
        return rest;
    }
    match u {
        "ns" | "µs" | "ms" | "ps" => "s",
        "mV" | "kV" => "V",
        "mA" | "µA" => "A",
        "mΩ" | "kΩ" | "MΩ" => "Ω",
        "kHz" | "MHz" | "GHz" => "Hz",
        "pF" | "nF" | "µF" | "uF" | "mF" => "F",
        "pH" | "nH" | "µH" | "uH" | "mH" => "H",
        other => other,
    }
}

/// Extract prefix and base unit from a unit string (e.g. `"kHz"` -> `("k", "Hz")`, `"Ω"` -> `("", "Ω")`).
pub fn extract_base_unit(unit: &str) -> (&str, &str) {
    let u = unit.trim();
    if u.is_empty() {
        return ("", "");
    }
    if let Some(rest) = u.strip_prefix('_') {
        return ("_", rest);
    }
    let mut chars = u.chars();
    let first = chars.next().unwrap();
    let rest = chars.as_str();

    if !rest.is_empty()
        && (first == 'p'
            || first == 'n'
            || first == 'µ'
            || first == 'u'
            || first == 'm'
            || first == 'k'
            || first == 'M'
            || first == 'G'
            || first == 'T')
    {
        let prefix_len = first.len_utf8();
        (&u[..prefix_len], &u[prefix_len..])
    } else {
        ("", u)
    }
}

/// Build unit dropdown options from a unit string and whether it's floating-point.
/// Mirrors `PropRow::buildUnits()` in C++.
pub fn build_unit_options(unit: &str, is_double: bool) -> (Vec<String>, bool, bool) {
    let u = unit.trim();
    if u.is_empty() {
        return (Vec::new(), false, false);
    }
    if let Some(rest) = u.strip_prefix('_') {
        return (vec![rest.to_string()], true, false);
    }
    let (_prefix, base) = extract_base_unit(u);
    let mut options = Vec::new();
    if is_double {
        for p in ["p", "n", "µ", "m", "", "k", "M", "G", "T"] {
            options.push(format!("{p}{base}"));
        }
    } else {
        for p in ["", "k", "M", "G", "T"] {
            options.push(format!("{p}{base}"));
        }
    }
    (options, true, true)
}

/// Split formatted text (e.g. `"2.5 kHz"`, `"100 Ω"`, `"5V"`, `"100"`) into `(number_str, unit_str)`.
pub fn split_number_and_unit(val: &str) -> (&str, &str) {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return ("", "");
    }
    if let Some((n_str, u_str)) = trimmed.split_once(char::is_whitespace) {
        return (n_str.trim(), u_str.trim());
    }
    let num_end = find_numeric_end(trimmed);
    let (n_str, u_str) = trimmed.split_at(num_end);
    (n_str.trim(), u_str.trim())
}

/// Resolve the index of the active unit within `options`.
pub fn resolve_unit_index(options: &[String], unit_str: &str, default_unit: &str) -> usize {
    if options.is_empty() {
        return 0;
    }
    if !unit_str.is_empty() {
        if let Some(idx) = options.iter().position(|o| o == unit_str) {
            return idx;
        }
    }
    if !default_unit.is_empty() {
        if let Some(idx) = options.iter().position(|o| o == default_unit) {
            return idx;
        }
        let (_, base) = extract_base_unit(default_unit);
        if let Some(idx) = options.iter().position(|o| o == base) {
            return idx;
        }
    }
    0
}

fn find_numeric_end(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        has_digits = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_digits = true;
            i += 1;
        }
    }
    if !has_digits {
        return 0;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut exp_i = i + 1;
        if exp_i < bytes.len() && (bytes[exp_i] == b'+' || bytes[exp_i] == b'-') {
            exp_i += 1;
        }
        let exp_digits_start = exp_i;
        while exp_i < bytes.len() && bytes[exp_i].is_ascii_digit() {
            exp_i += 1;
        }
        if exp_i > exp_digits_start {
            i = exp_i;
        }
    }
    i
}

fn prefix_char_multiplier(c: char) -> f64 {
    match c {
        'p' | 'P' => 1e-12,
        'n' | 'N' => 1e-9,
        'µ' | 'μ' | 'u' | 'U' => 1e-6,
        'm' => 1e-3,
        'k' | 'K' => 1e3,
        'M' => 1e6,
        'G' | 'g' => 1e9,
        'T' | 't' => 1e12,
        'r' | 'R' => 1.0,
        _ => 1.0,
    }
}

fn parse_shorthand_si(val: &str) -> Option<f64> {
    let s = val.trim();
    if s.is_empty() {
        return None;
    }
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (-1.0, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (1.0, stripped)
    } else {
        (1.0, s)
    };

    let prefix_pos = rest.find(|c: char| {
        matches!(
            c,
            'p' | 'P'
                | 'n'
                | 'N'
                | 'u'
                | 'U'
                | 'µ'
                | 'μ'
                | 'm'
                | 'k'
                | 'K'
                | 'M'
                | 'G'
                | 'g'
                | 'T'
                | 't'
                | 'r'
                | 'R'
        )
    })?;

    let (before, after_prefix) = rest.split_at(prefix_pos);
    let mut chars = after_prefix.chars();
    let prefix_char = chars.next()?;
    let after = chars
        .as_str()
        .trim_end_matches(|c: char| !c.is_ascii_digit());

    if !before.is_empty()
        && before.chars().all(|c| c.is_ascii_digit())
        && !after.is_empty()
        && after.chars().all(|c| c.is_ascii_digit())
    {
        let reconstructed = format!("{before}.{after}");
        let n: f64 = reconstructed.parse().ok()?;
        let mult = prefix_char_multiplier(prefix_char);
        return Some(sign * n * mult);
    }

    None
}

/// Parse a number that may carry a unit or metric prefix (`"1 kΩ"`, `"5 V"`, `"10k"`, `"4k7"`, `"100"`).
///
/// If the value has no unit or prefix, `default_unit` supplies the prefix (Battery
/// resistance is stored in mΩ, so `"1"` means 1 mΩ).
pub fn parse_si(val: &str, default_unit: &str) -> f64 {
    let val = val.trim();
    if val.is_empty() {
        return 0.0;
    }

    if let Some(v) = parse_shorthand_si(val) {
        return v;
    }

    let val = val.strip_suffix('%').unwrap_or(val).trim();
    let def_u = default_unit.trim();

    if let Some((n_str, unit_str)) = val.split_once(char::is_whitespace) {
        let n: f64 = n_str.trim().parse().unwrap_or(0.0);
        let unit = unit_str.trim();
        if unit.is_empty() {
            return n * si_multiplier(def_u);
        }
        if unit == "m" && def_u != "m" {
            return n * 1e-3;
        }
        return n * si_multiplier(unit);
    }
    let num_end = find_numeric_end(val);
    let (n_str, unit_str) = val.split_at(num_end);
    let n: f64 = n_str.parse().unwrap_or(0.0);
    let unit = unit_str.trim();
    if unit.is_empty() {
        return n * si_multiplier(def_u);
    }
    if unit == "m" && def_u != "m" {
        return n * 1e-3;
    }
    n * si_multiplier(unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes() {
        assert_eq!(si_multiplier("pF"), 1e-12);
        assert_eq!(si_multiplier("nF"), 1e-9);
        assert_eq!(si_multiplier("µF"), 1e-6);
        assert_eq!(si_multiplier("uF"), 1e-6);
        assert_eq!(si_multiplier("u"), 1e-6);
        assert_eq!(si_multiplier("mΩ"), 1e-3);
        assert_eq!(si_multiplier("m"), 1.0);
        assert_eq!(si_multiplier("mm"), 1e-3);
        assert_eq!(si_multiplier("cm"), 1e-2);
        assert_eq!(si_multiplier("kΩ"), 1e3);
        assert_eq!(si_multiplier("k"), 1e3);
        assert_eq!(si_multiplier("K"), 1e3);
        assert_eq!(si_multiplier("MΩ"), 1e6);
        assert_eq!(si_multiplier("M"), 1e6);
        assert_eq!(si_multiplier("Meg"), 1e6);
        assert_eq!(si_multiplier("meg"), 1e6);
        assert_eq!(si_multiplier("MEG"), 1e6);
        assert_eq!(si_multiplier("Ω"), 1.0);
        assert_eq!(si_multiplier("V"), 1.0);
        assert_eq!(si_multiplier(""), 1.0);
    }

    #[test]
    fn parse_with_default_unit() {
        assert!((parse_si("100", "Ω") - 100.0).abs() < f64::EPSILON);
        assert!((parse_si("1 kΩ", "Ω") - 1000.0).abs() < 1e-9);
        assert!((parse_si("1", "mΩ") - 0.001).abs() < 1e-12);
        assert!((parse_si("5 V", "V") - 5.0).abs() < f64::EPSILON);
        assert!((parse_si("1e3", "Ω") - 1000.0).abs() < f64::EPSILON);
        assert!((parse_si("1.5 m", "m") - 1.5).abs() < 1e-9);
        assert!((parse_si("200 mm", "m") - 0.2).abs() < 1e-9);
    }

    #[test]
    fn parse_direct_metric_subunits_and_shorthand() {
        // Direct metric subunits without space
        assert!((parse_si("10k", "Ω") - 10000.0).abs() < 1e-9);
        assert!((parse_si("10kΩ", "Ω") - 10000.0).abs() < 1e-9);
        assert!((parse_si("4.7u", "F") - 4.7e-6).abs() < 1e-12);
        assert!((parse_si("4.7uF", "F") - 4.7e-6).abs() < 1e-12);
        assert!((parse_si("4.7µF", "F") - 4.7e-6).abs() < 1e-12);
        assert!((parse_si("100n", "F") - 100e-9).abs() < 1e-15);
        assert!((parse_si("25m", "V") - 0.025).abs() < 1e-9);
        assert!((parse_si("25mV", "V") - 0.025).abs() < 1e-9);
        assert!((parse_si("1.2M", "Ω") - 1.2e6).abs() < 1e-6);
        assert!((parse_si("1.2Meg", "Ω") - 1.2e6).abs() < 1e-6);
        assert!((parse_si("75%", "%") - 75.0).abs() < 1e-9);

        // Electronics shorthand notation (prefix as decimal separator)
        assert!((parse_si("4k7", "Ω") - 4700.0).abs() < 1e-9);
        assert!((parse_si("4K7", "Ω") - 4700.0).abs() < 1e-9);
        assert!((parse_si("2u2", "F") - 2.2e-6).abs() < 1e-12);
        assert!((parse_si("1n5", "F") - 1.5e-9).abs() < 1e-15);
        assert!((parse_si("1R5", "Ω") - 1.5).abs() < 1e-9);
        assert!((parse_si("0R5", "Ω") - 0.5).abs() < 1e-9);
        assert!((parse_si("3m3", "A") - 0.0033).abs() < 1e-9);
        assert!((parse_si("1M2", "Ω") - 1.2e6).abs() < 1e-6);
        assert!((parse_si("-4k7", "Ω") - -4700.0).abs() < 1e-9);
    }

    #[test]
    fn format_roundtrip() {
        assert_eq!(format_si(1000.0, "Ω"), "1 kΩ");
        assert_eq!(format_si(5.0, "V"), "5 V");
        assert_eq!(format_si(0.001, "Ω"), "1 mΩ");
        assert_eq!(format_si(5.12345, "V"), "5.12 V");
        assert_eq!(format_si(5.1, "V"), "5.1 V");
        assert_eq!(format_si(0.01234, "V"), "12.34 mV");
        assert_eq!(format_si(-12.567, "V"), "-12.57 V");
        assert!((parse_si(&format_si(2200.0, "Ω"), "Ω") - 2200.0).abs() < 1e-9);
    }

    #[test]
    fn time_units() {
        assert_eq!(split_time(1_000_000), (1, 2));
        assert_eq!(join_time(1, 2), 1_000_000);
        assert_eq!(split_time(1500), (1500, 0));
        assert_eq!(join_time(10, 1), 10_000);
    }

    #[test]
    fn unit_options_and_splitting() {
        assert_eq!(extract_base_unit("kHz"), ("k", "Hz"));
        assert_eq!(extract_base_unit("Hz"), ("", "Hz"));
        assert_eq!(extract_base_unit("MΩ"), ("M", "Ω"));
        assert_eq!(extract_base_unit("Ω"), ("", "Ω"));
        assert_eq!(extract_base_unit("_px"), ("_", "px"));
        assert_eq!(extract_base_unit(""), ("", ""));

        let (opts, has_box, use_mult) = build_unit_options("Hz", true);
        assert!(has_box);
        assert!(use_mult);
        assert_eq!(
            opts,
            vec!["pHz", "nHz", "µHz", "mHz", "Hz", "kHz", "MHz", "GHz", "THz"]
        );

        let (opts_px, has_box_px, use_mult_px) = build_unit_options("_px", false);
        assert!(has_box_px);
        assert!(!use_mult_px);
        assert_eq!(opts_px, vec!["px"]);

        let (opts_empty, has_box_empty, _) = build_unit_options("", false);
        assert!(!has_box_empty);
        assert!(opts_empty.is_empty());

        assert_eq!(split_number_and_unit("2.5 kHz"), ("2.5", "kHz"));
        assert_eq!(split_number_and_unit("100 Ω"), ("100", "Ω"));
        assert_eq!(split_number_and_unit("5V"), ("5", "V"));
        assert_eq!(split_number_and_unit("100"), ("100", ""));
        assert_eq!(split_number_and_unit("-12.5 V"), ("-12.5", "V"));

        assert_eq!(resolve_unit_index(&opts, "kHz", "Hz"), 5);
        assert_eq!(resolve_unit_index(&opts, "", "kHz"), 5);
        assert_eq!(resolve_unit_index(&opts, "", "Hz"), 4);
    }

    #[test]
    fn test_base_unit_for() {
        assert_eq!(base_unit_for("Ω"), "Ω");
        assert_eq!(base_unit_for("mΩ"), "Ω");
        assert_eq!(base_unit_for("kΩ"), "Ω");
        assert_eq!(base_unit_for("V"), "V");
        assert_eq!(base_unit_for("mV"), "V");
        assert_eq!(base_unit_for("F"), "F");
        assert_eq!(base_unit_for("nF"), "F");
        assert_eq!(base_unit_for("Hz"), "Hz");
        assert_eq!(base_unit_for("kHz"), "Hz");
        assert_eq!(base_unit_for("_px"), "px");
        assert_eq!(base_unit_for("_%"), "%");
        assert_eq!(base_unit_for("°C"), "°C");
        assert_eq!(base_unit_for("m"), "m");
        assert_eq!(base_unit_for(""), "");
    }
}
