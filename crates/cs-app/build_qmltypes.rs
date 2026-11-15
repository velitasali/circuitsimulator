use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct QmlClass {
    name: String,
    properties: Vec<QmlProp>,
    signals: Vec<QmlMethod>,
    methods: Vec<QmlMethod>,
    fn_ret_types: BTreeMap<String, String>,
}

struct QmlProp {
    name: String,
    read_fn: String,
    write_fn: Option<String>,
    notify_sig: Option<String>,
    is_constant: bool,
}

struct QmlMethod {
    name: String,
    ret_type: Option<String>,
    params: Vec<(String, String)>,
}

pub fn generate(manifest_dir: &Path) {
    let src_dir = manifest_dir.join("src");
    let qml_cs_app_dir = manifest_dir.join("../../qml/cs_app");

    let mut rs_files = Vec::new();
    collect_rs_files(&src_dir, &mut rs_files);
    rs_files.sort();

    let mut classes: BTreeMap<String, QmlClass> = BTreeMap::new();
    let mut all_methods_by_type: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut sources = Vec::new();

    for path in &rs_files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        // Return types used by qobject getters live in other impls in this crate.
        println!("cargo:rerun-if-changed={}", path.display());
        sources.push(src);
    }

    let mut helpers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for src in &sources {
        collect_fn_bodies(src, &mut helpers);
    }

    for src in &sources {
        if src.contains("#[qobject") {
            parse_qobject_file(src, &mut classes, &helpers);
        }
        collect_impl_return_types(src, &mut all_methods_by_type, &helpers);
    }

    for (name, cls) in &mut classes {
        if let Some(methods) = all_methods_by_type.get(name) {
            for (fn_name, ret_ty) in methods {
                cls.fn_ret_types
                    .entry(fn_name.clone())
                    .or_insert_with(|| ret_ty.clone());
            }
        }
    }

    let qmldir = "module cs_app\ndepends QtQuick auto\ntypeinfo cs_app.qmltypes\n";
    let qmltypes = render_qmltypes(&classes);

    let _ = fs::create_dir_all(&qml_cs_app_dir);
    write_if_changed(&qml_cs_app_dir.join("qmldir"), qmldir);
    write_if_changed(&qml_cs_app_dir.join("cs_app.qmltypes"), &qmltypes);
}

fn write_if_changed(path: &Path, content: &str) {
    if let Ok(existing) = fs::read_to_string(path)
        && existing == content
    {
        return;
    }
    fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn snake_to_camel(input: &str) -> String {
    let mut result = String::new();
    for tok in input.split('_') {
        let mut chars = tok.chars();
        if let Some(first_char) = chars.next() {
            let first_char = if result.is_empty() {
                first_char.to_ascii_lowercase()
            } else {
                first_char.to_ascii_uppercase()
            };
            result.push(first_char);
            result.push_str(chars.as_str());
        }
    }
    result
}

fn trailing_expr(fn_slice: &str) -> &str {
    let Some(open) = fn_slice.find('{') else {
        return "";
    };
    let Some(close) = fn_slice.rfind('}') else {
        return "";
    };
    if close <= open {
        return "";
    }
    let body = &fn_slice[open + 1..close];
    let mut brace = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut last_semi = None;
    for (idx, b) in body.bytes().enumerate() {
        match b {
            b'{' => brace += 1,
            b'}' => brace = brace.saturating_sub(1),
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b';' if brace == 0 && paren == 0 && bracket == 0 => last_semi = Some(idx),
            _ => {}
        }
    }
    let tail = match last_semi {
        Some(idx) => &body[idx + 1..],
        None => body,
    };
    tail.trim()
}

fn call_name(tail: &str) -> Option<&str> {
    let tail = tail.trim();
    let open = tail.find('(')?;
    let path = tail[..open].trim();
    if path.is_empty() || path.contains([' ', '<', '>']) {
        return None;
    }
    let name = path.rsplit("::").next()?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(name)
}

fn value_body_hint<'a>(
    body: Option<&'a str>,
    helpers: &'a BTreeMap<String, Vec<String>>,
) -> Option<&'a str> {
    let body = body?;
    let Some(name) = call_name(trailing_expr(body)) else {
        return Some(body);
    };
    match helpers.get(name) {
        Some(bodies) if bodies.len() == 1 => Some(bodies[0].as_str()),
        _ => Some(body),
    }
}

fn rust_type_to_qt(
    ty: &str,
    body_hint: Option<&str>,
    helpers: &BTreeMap<String, Vec<String>>,
) -> &'static str {
    let t = ty.trim();
    match t {
        "bool" => "bool",
        "i32" | "i16" | "i8" => "int",
        "u32" | "u16" | "u8" => "uint",
        "usize" => "int",
        "i64" => "qlonglong",
        "u64" => "qulonglong",
        "f64" => "double",
        "f32" => "float",
        "String" => "QString",
        "Vec<u8>" => "QByteArray",
        "Vec<String>" => "QStringList",
        _ if t.starts_with("Vec<") => "QVariantList",
        _ if t.starts_with("BTreeMap<") || t.starts_with("HashMap<") => "QVariantMap",
        "Value" | "serde_json::Value" => {
            // A getter that only forwards to a helper has no Value::Object of its own.
            let resolved = value_body_hint(body_hint, helpers);
            if let Some(b) = resolved {
                let tail = trailing_expr(b);
                if tail.starts_with("Value::Array") || tail.starts_with("json!([") {
                    return "QVariantList";
                }
                if tail.starts_with("Value::Object") || tail.starts_with("json!({") {
                    return "QVariantMap";
                }
                let is_arr =
                    b.contains("Value::Array") || b.contains("json!([") || b.contains("Vec::new");
                let is_obj =
                    b.contains("Value::Object") || b.contains("json!({") || b.contains("Map::new");
                if is_arr && !is_obj {
                    return "QVariantList";
                }
                if is_obj && !is_arr {
                    return "QVariantMap";
                }
            }
            "QVariant"
        }
        _ => "QVariant",
    }
}

fn strip_comments_preserving_strings(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            let mut depth = 1usize;
            while i < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    if bytes[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
            continue;
        }
        if bytes[i] == b'"' {
            out.push('"');
            i += 1;
            while i < bytes.len() {
                let c = bytes[i] as char;
                out.push(c);
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 1;
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn find_matching_delim(s: &str, open_idx: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut i = open_idx;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    break;
                }
                i += 1;
            }
        } else if bytes[i] == open {
            depth += 1;
        } else if bytes[i] == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn parse_qobject_file(
    raw: &str,
    classes: &mut BTreeMap<String, QmlClass>,
    helpers: &BTreeMap<String, Vec<String>>,
) {
    let src = strip_comments_preserving_strings(raw);
    let mut pos = 0;
    while let Some(rel) = src[pos..].find("#[qobject") {
        let attr_start = pos + rel;
        let Some(attr_end) = src[attr_start..].find(']') else {
            break;
        };
        let after_attr = attr_start + attr_end + 1;
        let Some(impl_rel) = src[after_attr..].find("impl ") else {
            pos = after_attr;
            continue;
        };
        let impl_start = after_attr + impl_rel + "impl ".len();
        let Some(brace_rel) = src[impl_start..].find('{') else {
            pos = impl_start;
            continue;
        };
        let struct_name = src[impl_start..impl_start + brace_rel]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        let body_start = impl_start + brace_rel;
        let Some(body_end) = find_matching_delim(&src, body_start, b'{', b'}') else {
            pos = body_start + 1;
            continue;
        };
        let body = &src[body_start + 1..body_end];
        let cls = classes
            .entry(struct_name.clone())
            .or_insert_with(|| QmlClass {
                name: struct_name,
                ..Default::default()
            });
        parse_qobject_body(body, cls, helpers);
        pos = body_end + 1;
    }
}

fn parse_qobject_body(body: &str, cls: &mut QmlClass, helpers: &BTreeMap<String, Vec<String>>) {
    // 1. qproperty!(...)
    let mut pos = 0;
    while let Some(rel) = body[pos..].find("qproperty!") {
        let start = pos + rel;
        let Some(open_rel) = body[start..].find('(') else {
            break;
        };
        let open = start + open_rel;
        let Some(close) = find_matching_delim(body, open, b'(', b')') else {
            break;
        };
        if let Some(prop) = parse_qproperty(&body[open + 1..close]) {
            cls.properties.push(prop);
        }
        pos = close + 1;
    }

    // 2. #[qsignal] / #[qslot] and fn return types inside this impl block
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        let mut kind = None;
        if body[i..].starts_with("#[qsignal]") {
            kind = Some(true);
            i += "#[qsignal]".len();
        } else if body[i..].starts_with("#[qslot]") {
            kind = Some(false);
            i += "#[qslot]".len();
        }

        if let Some(is_signal) = kind {
            let Some(fn_rel) = body[i..].find("fn ") else {
                break;
            };
            let fn_start = i + fn_rel;
            if let Some(sig) = parse_fn_sig(body, fn_start) {
                let qt_ret = sig.ret.filter(|r| !r.is_empty()).map(|r| {
                    rust_type_to_qt(&r, Some(&body[fn_start..sig.next_idx]), helpers).to_string()
                });
                let method = QmlMethod {
                    name: snake_to_camel(&sig.name),
                    ret_type: qt_ret,
                    params: sig.params,
                };
                if is_signal {
                    cls.signals.push(method);
                } else {
                    cls.methods.push(method);
                }
                i = sig.next_idx;
                continue;
            }
        }

        if body[i..].starts_with("fn ")
            && (i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_'))
            && let Some(sig) = parse_fn_sig(body, i)
        {
            if let Some(r) = sig.ret {
                let qt_ty = rust_type_to_qt(&r, Some(&body[i..sig.next_idx]), helpers).to_string();
                cls.fn_ret_types.insert(sig.name, qt_ty);
            }
            i = sig.next_idx;
            continue;
        }
        i += 1;
    }
}

fn collect_fn_bodies(raw: &str, out: &mut BTreeMap<String, Vec<String>>) {
    let src = strip_comments_preserving_strings(raw);
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if src[i..].starts_with("fn ")
            && (i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_'))
            && let Some(sig) = parse_fn_sig(&src, i)
        {
            out.entry(sig.name)
                .or_default()
                .push(src[i..sig.next_idx].to_string());
            i = sig.next_idx;
            continue;
        }
        i += 1;
    }
}

fn collect_impl_return_types(
    raw: &str,
    out: &mut BTreeMap<String, BTreeMap<String, String>>,
    helpers: &BTreeMap<String, Vec<String>>,
) {
    let src = strip_comments_preserving_strings(raw);
    let mut pos = 0;
    while let Some(rel) = src[pos..].find("impl ") {
        let impl_start = pos + rel + "impl ".len();
        let Some(brace_rel) = src[impl_start..].find('{') else {
            break;
        };
        let header = src[impl_start..impl_start + brace_rel].trim();
        let body_start = impl_start + brace_rel;
        let Some(body_end) = find_matching_delim(&src, body_start, b'{', b'}') else {
            break;
        };
        // Skip trait impls (`impl Foo for Bar`).
        if !header.contains(" for ") && !header.starts_with('<') {
            let name = header.split_whitespace().next().unwrap_or("").to_string();
            let body = &src[body_start + 1..body_end];
            let entry = out.entry(name).or_default();
            let bytes = body.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if body[i..].starts_with("fn ")
                    && (i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_'))
                    && let Some(sig) = parse_fn_sig(body, i)
                {
                    if let Some(r) = sig.ret {
                        let qt_ty =
                            rust_type_to_qt(&r, Some(&body[i..sig.next_idx]), helpers).to_string();
                        entry.insert(sig.name, qt_ty);
                    }
                    i = sig.next_idx;
                    continue;
                }
                i += 1;
            }
        }
        pos = body_end + 1;
    }
}

struct ParsedFnSig {
    name: String,
    params: Vec<(String, String)>,
    ret: Option<String>,
    next_idx: usize,
}

fn parse_fn_sig(s: &str, fn_start: usize) -> Option<ParsedFnSig> {
    let after_fn = fn_start + "fn ".len();
    let open_paren_rel = s[after_fn..].find('(')?;
    let name = s[after_fn..after_fn + open_paren_rel].trim().to_string();
    let open_paren = after_fn + open_paren_rel;
    let close_paren = find_matching_delim(s, open_paren, b'(', b')')?;
    let params_str = &s[open_paren + 1..close_paren];
    let mut params = Vec::new();
    for raw_p in split_top_level(params_str, ',') {
        let p = raw_p.trim();
        if p.is_empty() || p.contains("self") {
            continue;
        }
        if let Some((pname, pty)) = p.split_once(':') {
            let clean_name = pname.trim().trim_start_matches('_').to_string();
            let clean_name = if clean_name.is_empty() {
                "arg".to_string()
            } else {
                snake_to_camel(&clean_name)
            };
            params.push((clean_name, pty.trim().to_string()));
        }
    }

    let rest = &s[close_paren + 1..];
    let semi_rel = rest.find(';');
    let brace_rel = rest.find('{');
    let term_rel = match (semi_rel, brace_rel) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return None,
    };
    let sig_tail = rest[..term_rel].trim();
    let ret = sig_tail.strip_prefix("->").map(|r| r.trim().to_string());

    let next_idx = if rest.as_bytes()[term_rel] == b'{' {
        let body_open = close_paren + 1 + term_rel;
        find_matching_delim(s, body_open, b'{', b'}')
            .map(|end| end + 1)
            .unwrap_or(body_open + 1)
    } else {
        close_paren + 1 + term_rel + 1
    };

    Some(ParsedFnSig {
        name,
        params,
        ret,
        next_idx,
    })
}

fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut angle = 0usize;
    let mut paren = 0usize;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            _ if c == sep && angle == 0 && paren == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

fn parse_qproperty(inner: &str) -> Option<QmlProp> {
    let parts = split_top_level(inner, ',');
    let name_lit = parts.first()?.trim();
    let name = name_lit.trim_matches('"').to_string();
    if name.is_empty() {
        return None;
    }
    let mut read_fn = None;
    let mut write_fn = None;
    let mut notify_sig = None;
    let mut is_constant = false;

    for part in parts.iter().skip(1) {
        let p = part.trim();
        if p == "Constant" {
            is_constant = true;
        } else if let Some((k, v)) = p.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "Read" => read_fn = Some(v.to_string()),
                "Write" => write_fn = Some(v.to_string()),
                "Notify" => notify_sig = Some(v.to_string()),
                _ => {}
            }
        }
    }

    Some(QmlProp {
        name,
        read_fn: read_fn?,
        write_fn,
        notify_sig,
        is_constant,
    })
}

fn render_qmltypes(classes: &BTreeMap<String, QmlClass>) -> String {
    let mut out = String::from(
        "import QtQuick.tooling 1.2\n\n\
         // Auto-generated by crates/cs-app/build_qmltypes.rs for qmllint / qmlls.\n\n\
         Module {\n",
    );

    // C++ QQuickItem types registered in cpp/circuit_canvas_item.cpp
    out.push_str(
        "    Component {\n\
         \x20       file: \"cpp/circuit_canvas_item.h\"\n\
         \x20       name: \"CircuitCanvasItem\"\n\
         \x20       accessSemantics: \"reference\"\n\
         \x20       prototype: \"QQuickItem\"\n\
         \x20       exports: [\"cs_app/CircuitCanvasItem 1.0\"]\n\
         \x20       exportMetaObjectRevisions: [256]\n\
         \x20       Signal { name: \"hostReady\" }\n\
         \x20       Signal { name: \"frameReady\" }\n\
         \x20   }\n\
         \x20   Component {\n\
         \x20       file: \"cpp/instrument_screen_item.h\"\n\
         \x20       name: \"ScopeScreenItem\"\n\
         \x20       accessSemantics: \"reference\"\n\
         \x20       prototype: \"QQuickItem\"\n\
         \x20       exports: [\"cs_app/ScopeScreen 1.0\"]\n\
         \x20       exportMetaObjectRevisions: [256]\n\
         \x20   }\n\
         \x20   Component {\n\
         \x20       file: \"cpp/instrument_screen_item.h\"\n\
         \x20       name: \"LogicScreenItem\"\n\
         \x20       accessSemantics: \"reference\"\n\
         \x20       prototype: \"QQuickItem\"\n\
         \x20       exports: [\"cs_app/LogicScreen 1.0\"]\n\
         \x20       exportMetaObjectRevisions: [256]\n\
         \x20   }\n",
    );

    for cls in classes.values() {
        out.push_str(&format!(
            "    Component {{\n\
             \x20       file: \"{0}\"\n\
             \x20       name: \"{0}\"\n\
             \x20       accessSemantics: \"reference\"\n\
             \x20       prototype: \"QObject\"\n\
             \x20       exports: [\"cs_app/{0} 1.0\"]\n\
             \x20       isCreatable: false\n\
             \x20       isSingleton: true\n\
             \x20       exportMetaObjectRevisions: [256]\n",
            cls.name
        ));

        for (idx, prop) in cls.properties.iter().enumerate() {
            let qt_ty = cls
                .fn_ret_types
                .get(&prop.read_fn)
                .map(String::as_str)
                .unwrap_or("QVariant");
            out.push_str("        Property {\n");
            out.push_str(&format!("            name: \"{}\"\n", prop.name));
            out.push_str(&format!("            type: \"{}\"\n", qt_ty));
            out.push_str(&format!(
                "            read: \"{}\"\n",
                snake_to_camel(&prop.read_fn)
            ));
            if let Some(w) = &prop.write_fn {
                out.push_str(&format!("            write: \"{}\"\n", snake_to_camel(w)));
            }
            if let Some(n) = &prop.notify_sig {
                out.push_str(&format!("            notify: \"{}\"\n", snake_to_camel(n)));
            }
            out.push_str(&format!("            index: {idx}\n"));
            if prop.write_fn.is_none() {
                out.push_str("            isReadonly: true\n");
            }
            if prop.is_constant {
                out.push_str("            isConstant: true\n");
            }
            out.push_str("            isFinal: true\n");
            out.push_str("        }\n");
        }

        for sig in &cls.signals {
            render_method(&mut out, "Signal", sig);
        }
        for meth in &cls.methods {
            render_method(&mut out, "Method", meth);
        }

        out.push_str("    }\n");
    }

    out.push_str("}\n");
    out
}

fn render_method(out: &mut String, kind: &str, m: &QmlMethod) {
    let ret = m.ret_type.as_deref();
    if m.params.is_empty() && ret.is_none() {
        out.push_str(&format!("        {kind} {{ name: \"{}\" }}\n", m.name));
        return;
    }
    out.push_str(&format!("        {kind} {{\n"));
    out.push_str(&format!("            name: \"{}\"\n", m.name));
    if let Some(r) = ret {
        out.push_str(&format!("            type: \"{r}\"\n"));
    }
    for (pname, pty) in &m.params {
        out.push_str(&format!(
            "            Parameter {{ name: \"{pname}\"; type: \"{}\" }}\n",
            rust_type_to_qt(pty, None, &BTreeMap::new())
        ));
    }
    out.push_str("        }\n");
}
