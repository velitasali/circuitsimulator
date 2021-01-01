//! `{file}.cfg` sidecar. C++ `CodeEditor::loadConfig` / `saveConfig`.

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub compiler: String,
    pub load_compiler: bool,
    pub load_breakp: bool,
    pub open_files: bool,
    pub circuit: String,
    pub file_list: String,
    pub breakpoints: Vec<i32>,
    pub board: String,
    pub custom_board: String,
    pub device: String,
    pub family: String,
    pub extra_args: String,
    pub tool_path: String,
    pub incl_path: String,
}

fn attr_map(line: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut rest = line;
    while let Some(eq) = rest.find("=\"") {
        let key_start = rest[..eq]
            .rfind(|c: char| c.is_whitespace() || c == '<')
            .map(|i| i + 1)
            .unwrap_or(0);
        let key = rest[key_start..eq].trim().to_string();
        let after = &rest[eq + 2..];
        if let Some(end) = after.find('"') {
            map.insert(key, after[..end].to_string());
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    map
}

fn parse_bool(s: &str) -> bool {
    matches!(s, "true" | "True" | "1")
}

fn parse_breakpoints(s: &str) -> Vec<i32> {
    s.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|t| {
            let t = t.trim();
            if t.is_empty() { None } else { t.parse().ok() }
        })
        .filter(|n| *n > 0)
        .collect()
}

/// Load `{path}.cfg`, or the old `{path}.brk` comma list.
pub fn load_cfg(source_path: &str) -> FileConfig {
    let mut cfg = FileConfig::default();
    if source_path.is_empty() || source_path.to_ascii_lowercase().ends_with(".cfg") {
        return cfg;
    }
    let cfg_path = format!("{source_path}.cfg");
    if let Ok(text) = std::fs::read_to_string(&cfg_path) {
        let mut editor = BTreeMap::new();
        let mut compiler = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("<item") {
                continue;
            }
            let attrs = attr_map(line);
            match attrs.get("itemtype").map(|s| s.as_str()) {
                Some("CodeEditor") => editor = attrs,
                Some("Compiler") => compiler = attrs,
                _ => {}
            }
        }
        if let Some(v) = editor.get("Compiler") {
            cfg.compiler = v.clone();
        }
        cfg.load_compiler = editor
            .get("LoadCompiler")
            .map(|s| parse_bool(s))
            .unwrap_or(false);
        cfg.load_breakp = editor
            .get("LoadBreakp")
            .map(|s| parse_bool(s))
            .unwrap_or(false);
        cfg.open_files = editor
            .get("OpenFiles")
            .map(|s| parse_bool(s))
            .unwrap_or(false);
        cfg.circuit = editor.get("Circuit").cloned().unwrap_or_default();
        cfg.file_list = editor.get("FileList").cloned().unwrap_or_default();
        if let Some(bp) = editor.get("Breakpoints") {
            cfg.breakpoints = parse_breakpoints(bp);
        }
        if let Some(v) = compiler.get("Board") {
            cfg.board = v.clone();
        }
        if let Some(v) = compiler.get("CustomBoard") {
            cfg.custom_board = v.clone();
        }
        if let Some(v) = compiler.get("Device") {
            cfg.device = v.clone();
        }
        if let Some(v) = compiler.get("Family") {
            cfg.family = v.clone();
        }
        if let Some(v) = compiler.get("extraArgs") {
            cfg.extra_args = v.clone();
        }
        if let Some(v) = compiler.get("ToolPath") {
            cfg.tool_path = v.clone();
        }
        if let Some(v) = compiler.get("InclPath") {
            cfg.incl_path = v.clone();
        }
        if cfg.compiler.is_empty() {
            if let Some(v) = compiler.get("compilertype") {
                cfg.compiler = v.clone();
            }
        }
        return cfg;
    }

    let brk_path = format!("{source_path}.brk");
    if let Ok(text) = std::fs::read_to_string(&brk_path) {
        cfg.breakpoints = parse_breakpoints(&text);
    }
    cfg
}

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn save_cfg(source_path: &str, cfg: &FileConfig) -> std::io::Result<()> {
    if source_path.is_empty() || source_path.to_ascii_lowercase().ends_with(".cfg") {
        return Ok(());
    }
    let path = format!("{source_path}.cfg");
    let bp = cfg
        .breakpoints
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let compiler = if cfg.compiler.is_empty() {
        "None"
    } else {
        cfg.compiler.as_str()
    };
    let mut out = String::from("<document>\n");
    out.push_str(&format!(
        "<item itemtype=\"CodeEditor\" Compiler=\"{}\" LoadCompiler=\"{}\" LoadBreakp=\"{}\" OpenFiles=\"{}\"",
        xml_esc(compiler),
        cfg.load_compiler,
        cfg.load_breakp,
        cfg.open_files,
    ));
    if !cfg.circuit.is_empty() {
        out.push_str(&format!(" Circuit=\"{}\"", xml_esc(&cfg.circuit)));
    }
    if !cfg.file_list.is_empty() {
        out.push_str(&format!(" FileList=\"{}\"", xml_esc(&cfg.file_list)));
    }
    if !bp.is_empty() {
        out.push_str(&format!(" Breakpoints=\"{bp}\""));
    }
    out.push_str(" />\n");
    if compiler != "None" {
        out.push_str(&format!(
            "<item itemtype=\"Compiler\" compilertype=\"{}\"",
            xml_esc(compiler)
        ));
        if !cfg.board.is_empty() {
            out.push_str(&format!(" Board=\"{}\"", xml_esc(&cfg.board)));
        }
        if !cfg.custom_board.is_empty() {
            out.push_str(&format!(" CustomBoard=\"{}\"", xml_esc(&cfg.custom_board)));
        }
        if !cfg.tool_path.is_empty() {
            out.push_str(&format!(" ToolPath=\"{}\"", xml_esc(&cfg.tool_path)));
        }
        if !cfg.incl_path.is_empty() {
            out.push_str(&format!(" InclPath=\"{}\"", xml_esc(&cfg.incl_path)));
        }
        if !cfg.extra_args.is_empty() {
            out.push_str(&format!(" extraArgs=\"{}\"", xml_esc(&cfg.extra_args)));
        }
        if !cfg.device.is_empty() {
            out.push_str(&format!(" Device=\"{}\"", xml_esc(&cfg.device)));
        }
        if !cfg.family.is_empty() {
            out.push_str(&format!(" Family=\"{}\"", xml_esc(&cfg.family)));
        }
        out.push_str(" />\n");
    }
    out.push_str("</document>\n");
    if let Some(dir) = Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join("cs_cfg_test");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("blink.c");
        let _ = std::fs::write(&src, "int main(){}\n");
        let path = src.to_string_lossy().into_owned();
        let cfg = FileConfig {
            compiler: "Avrgcc".into(),
            load_compiler: true,
            load_breakp: true,
            breakpoints: vec![3, 7, 12],
            board: "Mega".into(),
            custom_board: "custom:board:fqbn".into(),
            device: "atmega328p".into(),
            extra_args: "-Os".into(),
            ..Default::default()
        };
        save_cfg(&path, &cfg).unwrap();
        let loaded = load_cfg(&path);
        assert_eq!(loaded.compiler, "Avrgcc");
        assert!(loaded.load_compiler);
        assert!(loaded.load_breakp);
        assert_eq!(loaded.breakpoints, vec![3, 7, 12]);
        assert_eq!(loaded.board, "Mega");
        assert_eq!(loaded.custom_board, "custom:board:fqbn");
        assert_eq!(loaded.device, "atmega328p");
        assert_eq!(loaded.extra_args, "-Os");
        let _ = std::fs::remove_file(format!("{path}.cfg"));
    }

    #[test]
    fn old_brk_file() {
        let dir = std::env::temp_dir().join("cs_cfg_brk");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("a.c");
        let _ = std::fs::write(&src, "");
        let path = src.to_string_lossy().into_owned();
        let _ = std::fs::write(format!("{path}.brk"), "4,8,");
        let loaded = load_cfg(&path);
        assert_eq!(loaded.breakpoints, vec![4, 8]);
        let _ = std::fs::remove_file(format!("{path}.brk"));
    }
}
