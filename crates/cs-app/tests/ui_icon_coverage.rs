#[path = "../src/ui_icon.rs"]
mod ui_icon;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use ui_icon::UiIcon;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_quoted_identifiers(expr: &str, re_ident: &Regex, out: &mut BTreeSet<String>) {
    for cap in re_ident.captures_iter(expr) {
        out.insert(cap[1].to_string());
    }
}

/// Body of `function name(...) { ... }`, excluding the surrounding braces.
fn qml_function_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("function {name}");
    let bytes = text.as_bytes();
    let mut search_from = 0;

    while let Some(rel_idx) = text[search_from..].find(&marker) {
        let idx = search_from + rel_idx;
        let after = idx + marker.len();
        let prev_ok =
            idx == 0 || (!bytes[idx - 1].is_ascii_alphanumeric() && bytes[idx - 1] != b'_');
        let next_ok =
            after < bytes.len() && (bytes[after] == b'(' || bytes[after].is_ascii_whitespace());
        search_from = after;
        if !prev_ok || !next_ok {
            continue;
        }

        let Some(open_rel) = text[after..].find('{') else {
            return None;
        };
        let open = after + open_rel;
        let mut depth = 0i32;
        for (offset, ch) in text[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&text[open + 1..open + offset]);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    None
}

fn collect_app_icon_blocks(
    text: &str,
    re_cmp: &Regex,
    re_ident: &Regex,
    out: &mut BTreeSet<String>,
) {
    let bytes = text.as_bytes();
    let mut search_from = 0;

    while let Some(rel_idx) = text[search_from..].find("AppIcon") {
        let idx = search_from + rel_idx;
        search_from = idx + "AppIcon".len();

        let prev_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric() && bytes[idx - 1] != b'_';
        let next_ok = search_from >= bytes.len()
            || !bytes[search_from].is_ascii_alphanumeric() && bytes[search_from] != b'_';
        if !prev_ok || !next_ok {
            continue;
        }

        let Some(open_rel) = text[search_from..].find('{') else {
            continue;
        };
        let mut pos = search_from + open_rel;
        let mut depth = 0i32;
        let start = pos + 1;
        let mut end = text.len();

        for (offset, ch) in text[pos..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = pos + offset;
                        pos = end + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        search_from = search_from.max(pos);

        let block = strip_line_comments(&text[start..end]);
        assert!(
            !block.contains("font.pixelSize"),
            "AppIcon block must use `size:` instead of `font.pixelSize:`"
        );
        for line in block.lines() {
            let trimmed = line.trim();
            assert!(
                !trimmed.starts_with("text:"),
                "AppIcon block must use `icon:` instead of `text:`: {trimmed}"
            );
            if let Some(rest) = trimmed.strip_prefix("icon:") {
                let expr = re_cmp.replace_all(rest, "");
                extract_quoted_identifiers(&expr, re_ident, out);
            } else if trimmed.starts_with('?') || trimmed.starts_with(':') {
                let expr = re_cmp.replace_all(trimmed, "");
                extract_quoted_identifiers(&expr, re_ident, out);
            }
        }
    }
}

fn collect_qml_icons(root: &Path) -> BTreeMap<String, Vec<String>> {
    let qml_dir = root.join("qml");
    let re_prop = Regex::new(r#"(?:iconLigature|glyph|"icon")\s*:\s*([^\n;]+)"#).unwrap();
    let re_return = Regex::new(r"\breturn\s+([^\n;]+)").unwrap();
    let re_cmp = Regex::new(r#"===\s*"[^"]*""#).unwrap();
    let re_ident = Regex::new(r#""([a-z0-9_]+)""#).unwrap();

    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut paths: Vec<PathBuf> = fs::read_dir(&qml_dir)
        .expect("read qml dir")
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("qml"))
        .collect();
    paths.sort();

    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).expect("read qml file");
        let mut file_icons = BTreeSet::new();

        collect_app_icon_blocks(&text, &re_cmp, &re_ident, &mut file_icons);

        if name == "FileBrowserView.qml" {
            for fn_name in ["fileIconForName", "fileIconPreview"] {
                let body = qml_function_body(&text, fn_name)
                    .unwrap_or_else(|| panic!("FileBrowserView.qml is missing function {fn_name}"));
                for line in strip_line_comments(body).lines() {
                    for cap in re_return.captures_iter(line) {
                        let expr = re_cmp.replace_all(&cap[1], "");
                        extract_quoted_identifiers(&expr, &re_ident, &mut file_icons);
                    }
                }
            }
        }

        for line in text.lines() {
            let code = line.split("//").next().unwrap_or("");
            for cap in re_prop.captures_iter(code) {
                extract_quoted_identifiers(&cap[1], &re_ident, &mut file_icons);
            }
            if name == "ContextMenuItem.qml" && code.contains("control.checked") {
                extract_quoted_identifiers(code, &re_ident, &mut file_icons);
            }
        }

        for icon in file_icons {
            found.entry(icon).or_default().push(name.clone());
        }
    }

    found
}

#[test]
fn ui_icon_variants_are_unique_and_canonical() {
    let mut seen_names = BTreeSet::new();
    let mut seen_variants = BTreeSet::new();

    for &icon in UiIcon::ALL {
        let name = icon.as_str();
        assert!(!name.is_empty(), "icon name must not be empty");
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "icon name '{name}' must be lowercase snake_case"
        );
        assert!(
            seen_names.insert(name),
            "duplicate icon name '{name}' in UiIcon"
        );
        assert!(
            seen_variants.insert(icon.variant_name()),
            "duplicate variant name '{}' in UiIcon",
            icon.variant_name()
        );
        assert_eq!(UiIcon::from_name(name), Some(icon));
    }
}

#[test]
fn every_qml_icon_is_registered_in_ui_icon() {
    let root = workspace_root();
    let qml_icons = collect_qml_icons(&root);
    assert!(!qml_icons.is_empty(), "expected QML icon usages");

    let missing: Vec<_> = qml_icons
        .iter()
        .filter(|(name, _)| UiIcon::from_name(name).is_none())
        .collect();

    assert!(
        missing.is_empty(),
        "QML icons missing from UiIcon enum: {missing:?}"
    );
}

#[test]
fn every_ui_icon_is_used_in_rust_or_qml() {
    let root = workspace_root();
    let qml_icons = collect_qml_icons(&root);
    let rust_sources: Vec<String> = [
        "crates/cs-app/src/actions.rs",
        "crates/cs-app/src/menu_bar.rs",
        "crates/cs-app/src/command_center.rs",
        "crates/cs-app/src/circuit_panel.rs",
    ]
    .iter()
    .map(|rel| fs::read_to_string(root.join(rel)).expect("read rust source"))
    .collect();

    let unused: Vec<_> = UiIcon::ALL
        .iter()
        .copied()
        .filter(|icon| {
            let used_in_qml = qml_icons.contains_key(icon.as_str());
            let needle = format!("UiIcon::{}", icon.variant_name());
            let used_in_rust = rust_sources.iter().any(|src| src.contains(&needle));
            !used_in_qml && !used_in_rust
        })
        .collect();

    assert!(
        unused.is_empty(),
        "unused UiIcon variants (neither Rust nor QML): {unused:?}"
    );
}

#[test]
fn every_ui_icon_has_an_exported_svg_and_no_unreferenced_svgs() {
    let root = workspace_root();
    let ui_dir = root.join("resources/icons/ui");
    assert!(ui_dir.is_dir(), "expected resources/icons/ui directory");

    let mut disk_svgs = BTreeSet::new();
    for entry in fs::read_dir(&ui_dir).expect("read resources/icons/ui") {
        let path = entry.expect("dir entry").path();
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("svg"),
            "unexpected non-svg file in resources/icons/ui: {}",
            path.display()
        );
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("svg file stem")
            .to_string();
        let contents = fs::read_to_string(&path).expect("read svg");
        assert!(
            contents.contains("<svg") && contents.contains("<path"),
            "invalid SVG content in {}",
            path.display()
        );
        disk_svgs.insert(stem);
    }

    let enum_names: BTreeSet<String> = UiIcon::ALL
        .iter()
        .map(|icon| icon.as_str().to_string())
        .collect();

    let missing_svgs: Vec<_> = enum_names.difference(&disk_svgs).collect();
    let extra_svgs: Vec<_> = disk_svgs.difference(&enum_names).collect();

    assert!(
        missing_svgs.is_empty(),
        "UiIcon variants missing SVG files in resources/icons/ui: {missing_svgs:?}"
    );
    assert!(
        extra_svgs.is_empty(),
        "extra SVG files in resources/icons/ui not in UiIcon: {extra_svgs:?}"
    );
}
