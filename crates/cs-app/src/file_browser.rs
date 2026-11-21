//! File browser façade. Rows are JSON (no `QFileSystemModel` / `QModelIndex`).

use crate::path_util::strip_file_url;
use crate::ui_icon::UiIcon;
use cs_engine::compat::simulide;
use cs_engine::{i18n, project, settings};
use qtbridge::qobject;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FileBrowser {
    root_path: String,
    show_hidden: bool,
    clipboard: Option<String>,
    expanded_paths: HashSet<String>,
    expand_all: bool,
}

impl Default for FileBrowser {
    fn default() -> Self {
        let s = cs_engine::settings::get();
        let root = if crate::pending::no_project() {
            String::new()
        } else {
            s.last_project_dir
                .clone()
                .unwrap_or_else(|| s.recent_projects.first().cloned().unwrap_or_default())
        };
        let root = strip_file_url(&root);
        let root_path = if !root.is_empty() && Path::new(&root).is_dir() {
            root
        } else {
            String::new()
        };
        Self {
            root_path,
            show_hidden: false,
            clipboard: None,
            expanded_paths: HashSet::new(),
            expand_all: false,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl FileBrowser {
    qproperty!(
        "rootPath",
        Read = root_path,
        Write = set_root_path,
        Notify = root_path_changed
    );
    qproperty!("hasProject", Read = has_project, Notify = root_path_changed);
    qproperty!("entries", Read = entries, Notify = entries_changed);
    qproperty!(
        "showHidden",
        Read = show_hidden,
        Write = set_show_hidden,
        Notify = show_hidden_changed
    );
    qproperty!("canPaste", Read = can_paste, Notify = clipboard_changed);

    #[qsignal]
    fn root_path_changed(&mut self);
    #[qsignal]
    fn entries_changed(&mut self);
    #[qsignal]
    fn show_hidden_changed(&mut self);
    #[qsignal]
    fn clipboard_changed(&mut self);
    #[qsignal]
    fn open_file(&mut self, path: String);
    #[qsignal]
    fn open_circuit(&mut self, path: String);

    fn root_path(&self) -> String {
        self.root_path.clone()
    }

    fn has_project(&self) -> bool {
        !self.root_path.is_empty() && Path::new(&self.root_path).is_dir()
    }

    fn can_paste(&self) -> bool {
        self.clipboard
            .as_ref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false)
    }

    fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    fn set_root_path(&mut self, path: String) {
        let path = strip_file_url(&path);
        if self.root_path == path {
            return;
        }
        self.root_path = path.clone();
        self.expanded_paths.clear();
        self.expand_all = false;
        if !path.is_empty() && Path::new(&path).is_dir() {
            let mut s = cs_engine::settings::get().clone();
            s.push_recent_project(path);
            s.save();
        } else if path.is_empty() {
            cs_engine::settings::edit(|s| {
                s.last_project_dir = None;
            });
        }
        self.root_path_changed();
        self.entries_changed();
    }

    fn set_show_hidden(&mut self, hidden: bool) {
        if self.show_hidden == hidden {
            return;
        }
        self.show_hidden = hidden;
        self.show_hidden_changed();
        self.entries_changed();
    }

    fn entries(&self) -> Value {
        if self.root_path.is_empty() {
            return Value::Array(vec![]);
        }
        let root = PathBuf::from(&self.root_path);
        if !root.is_dir() {
            return Value::Array(vec![]);
        }
        let mut rows = Vec::new();
        self.collect_dir(&root, 0, &mut rows);
        Value::Array(rows)
    }

    fn collect_dir(&self, dir: &Path, depth: i32, out: &mut Vec<Value>) {
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        let mut kids: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        kids.sort_by(|a, b| {
            let ad = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let bd = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            bd.cmp(&ad).then_with(|| a.file_name().cmp(&b.file_name()))
        });
        for e in kids {
            let name = e.file_name().to_string_lossy().to_string();
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }
            let path = e.path();
            let is_dir = path.is_dir();
            let path_str = path.to_string_lossy().replace('\\', "/");
            let expanded = is_dir && (self.expand_all || self.expanded_paths.contains(&path_str));

            out.push(json!({
                "name": name,
                "path": path_str,
                "isDir": is_dir,
                "depth": depth,
                "expanded": expanded,
            }));

            if is_dir && expanded {
                self.collect_dir(&path, depth + 1, out);
            }
        }
    }

    #[qslot]
    fn close_project(&mut self) {
        if self.root_path.is_empty() {
            return;
        }
        self.set_root_path(String::new());
    }

    #[qslot]
    fn open_path(&mut self, path: String) {
        let path = strip_file_url(&path);
        let p = PathBuf::from(&path);
        if p.is_dir() {
            self.toggle_expand(path);
            return;
        }
        if is_circuit_path(&p) {
            self.open_circuit(path);
        } else {
            self.open_file(path);
        }
    }

    #[qslot]
    fn recent_project_items(&self) -> Value {
        let recent = settings::get().recent_projects;
        Value::Array(build_recent_project_items(&self.root_path, &recent))
    }

    #[qslot]
    fn circuit_switcher_items(&self, current_circuit_path: String) -> Value {
        let recent = settings::get().recent_circuits;
        Value::Array(build_circuit_switcher_items(
            &self.root_path,
            self.show_hidden,
            &current_circuit_path,
            &recent,
        ))
    }

    #[qslot]
    fn open_in_editor(&mut self, path: String) {
        let path = strip_file_url(&path);
        self.open_file(path);
    }

    #[qslot]
    fn toggle_expand(&mut self, path: String) {
        let path = strip_file_url(&path);
        if self.expanded_paths.contains(&path) {
            self.expanded_paths.remove(&path);
        } else {
            self.expanded_paths.insert(path);
        }
        self.entries_changed();
    }

    #[qslot]
    fn expand_all(&mut self) {
        self.expand_all = true;
        self.entries_changed();
    }

    #[qslot]
    fn collapse_all(&mut self) {
        self.expand_all = false;
        self.expanded_paths.clear();
        self.entries_changed();
    }

    #[qslot]
    fn reload(&mut self) {
        self.entries_changed();
    }

    #[qslot]
    fn copy_to_clipboard(&mut self, path: String) {
        let path = strip_file_url(&path);
        if !path.is_empty() && Path::new(&path).exists() {
            self.clipboard = Some(path);
            self.clipboard_changed();
        }
    }

    #[qslot]
    fn paste_to(&mut self, target: String) {
        let target = strip_file_url(&target);
        let Some(src_str) = self.clipboard.clone() else {
            return;
        };
        let src = Path::new(&src_str);
        if !src.exists() {
            return;
        }
        let mut dst_dir = PathBuf::from(if target.is_empty() {
            &self.root_path
        } else {
            &target
        });
        if dst_dir.is_file() {
            if let Some(p) = dst_dir.parent() {
                dst_dir = p.to_path_buf();
            }
        }
        if dst_dir.is_dir() {
            let _ = copy_item_recursive(src, &dst_dir);
            self.entries_changed();
        }
    }

    #[qslot]
    fn create_file(&mut self, parent_or_file: String, name: String) {
        let parent_or_file = strip_file_url(&parent_or_file);
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let mut dir = PathBuf::from(if parent_or_file.is_empty() {
            &self.root_path
        } else {
            &parent_or_file
        });
        if dir.is_file() {
            if let Some(p) = dir.parent() {
                dir = p.to_path_buf();
            }
        }
        let target = dir.join(name);
        if !target.exists() {
            let _ = fs::write(&target, "");
            let dir_str = dir.to_string_lossy().replace('\\', "/");
            self.expanded_paths.insert(dir_str);
            self.entries_changed();
            let target_str = target.to_string_lossy().replace('\\', "/");
            self.open_path(target_str);
        }
    }

    #[qslot]
    fn create_directory(&mut self, parent_or_file: String, name: String) {
        let parent_or_file = strip_file_url(&parent_or_file);
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let mut dir = PathBuf::from(if parent_or_file.is_empty() {
            &self.root_path
        } else {
            &parent_or_file
        });
        if dir.is_file() {
            if let Some(p) = dir.parent() {
                dir = p.to_path_buf();
            }
        }
        let target = dir.join(name);
        if !target.exists() {
            let _ = fs::create_dir_all(&target);
            let dir_str = dir.to_string_lossy().replace('\\', "/");
            self.expanded_paths.insert(dir_str);
            self.entries_changed();
        }
    }

    #[qslot]
    fn rename_path(&mut self, old_path: String, new_name: String) {
        let old_path = strip_file_url(&old_path);
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return;
        }
        let src = PathBuf::from(&old_path);
        if let Some(parent) = src.parent() {
            let target = parent.join(new_name);
            let _ = fs::rename(src, target);
            self.entries_changed();
        }
    }

    #[qslot]
    fn trash(&mut self, path: String) {
        let path = strip_file_url(&path);
        let p = PathBuf::from(&path);
        if p.exists() {
            if p.is_dir() {
                let _ = fs::remove_dir_all(p);
            } else {
                let _ = fs::remove_file(p);
            }
            self.entries_changed();
        }
    }

    #[qslot]
    fn open_externally(&self, path: String) {
        let path = strip_file_url(&path);
        let p = Path::new(&path);
        if p.exists() {
            open_in_system(p);
        }
    }

    #[qslot]
    fn open_parent_externally(&self, path: String) {
        let path = strip_file_url(&path);
        let p = Path::new(&path);
        if let Some(parent) = p.parent() {
            if parent.exists() {
                open_in_system(parent);
            }
        } else if p.exists() {
            open_in_system(p);
        }
    }
}

fn open_in_system(p: &Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(p).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

fn copy_item_recursive(src: &Path, dst_dir: &Path) -> std::io::Result<()> {
    let file_name = src.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing file name")
    })?;
    let target = dst_dir.join(file_name);
    if src.is_dir() {
        fs::create_dir_all(&target)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_item_recursive(&entry.path(), &target)?;
        }
    } else {
        fs::copy(src, &target)?;
    }
    Ok(())
}

const MAX_PROJECT_CIRCUITS: usize = 200;
const MAX_SCAN_DEPTH: usize = 8;

fn is_circuit_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(simulide::is_circuit_extension)
}

fn path_key(normalized: &str) -> String {
    #[cfg(windows)]
    {
        normalized.to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized.to_string()
    }
}

fn shorten_home(path: &str) -> String {
    let norm = project::normalize_path(path);
    if norm.is_empty() {
        return String::new();
    }
    if let Some(home) = dirs::home_dir() {
        let home_norm = project::normalize_path(&home.to_string_lossy());
        if !home_norm.is_empty() {
            if path_key(&norm) == path_key(&home_norm) {
                return "~".to_string();
            }
            let prefix = format!("{home_norm}/");
            if path_key(&norm).starts_with(&path_key(&prefix)) {
                return format!("~/{}", &norm[prefix.len()..]);
            }
        }
    }
    norm
}

fn parent_detail(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|p| shorten_home(&p.to_string_lossy()))
        .unwrap_or_default()
}

fn scan_project_circuits(dir: &Path, show_hidden: bool, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_SCAN_DEPTH || out.len() >= MAX_PROJECT_CIRCUITS {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        let p = entry.path();
        if ft.is_dir() {
            if matches!(name.as_str(), ".git" | "target" | "build" | "node_modules") {
                continue;
            }
            scan_project_circuits(&p, show_hidden, depth + 1, out);
        } else if ft.is_file() && is_circuit_path(&p) {
            out.push(p);
            if out.len() >= MAX_PROJECT_CIRCUITS {
                return;
            }
        }
    }
}

fn build_recent_project_items(root_path: &str, recent_projects: &[String]) -> Vec<Value> {
    let current_norm = project::normalize_path(&strip_file_url(root_path));
    let current_key = path_key(&current_norm);

    let mut seen: HashSet<String> = HashSet::new();
    let mut rows: Vec<Value> = Vec::new();

    let mut push_dir = |raw: &str| {
        let norm = project::normalize_path(&strip_file_url(raw));
        if norm.is_empty() || !Path::new(&norm).is_dir() {
            return;
        }
        let key = path_key(&norm);
        if !seen.insert(key.clone()) {
            return;
        }
        let name = Path::new(&norm)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&norm)
            .to_string();
        let is_current = !current_key.is_empty() && key == current_key;
        rows.push(json!({
            "kind": "item",
            "name": name,
            "detail": parent_detail(&norm),
            "path": norm,
            "tooltip": norm,
            "current": is_current,
            "icon": if is_current { UiIcon::FolderOpen.as_str() } else { UiIcon::Folder.as_str() },
        }));
    };

    if !current_norm.is_empty() {
        push_dir(&current_norm);
    }
    for raw in recent_projects {
        push_dir(raw);
    }

    rows
}

fn build_circuit_switcher_items(
    root_path: &str,
    show_hidden: bool,
    current_circuit_path: &str,
    recent_circuits: &[String],
) -> Vec<Value> {
    let root_norm = project::normalize_path(&strip_file_url(root_path));
    let current_norm = project::normalize_path(&strip_file_url(current_circuit_path));
    let current_key = path_key(&current_norm);

    let mut seen: HashSet<String> = HashSet::new();
    let mut proj_items: Vec<Value> = Vec::new();

    if !root_norm.is_empty() {
        let root_dir = Path::new(&root_norm);
        if root_dir.is_dir() {
            struct ProjectCircuit {
                depth: usize,
                rel_lower: String,
                full_norm: String,
                name: String,
                rel_parent: String,
            }

            let mut found: Vec<PathBuf> = Vec::new();
            scan_project_circuits(root_dir, show_hidden, 0, &mut found);

            let mut entries: Vec<ProjectCircuit> = found
                .into_iter()
                .filter_map(|p| {
                    let rel = p.strip_prefix(root_dir).ok()?;
                    let rel_norm = project::normalize_path(&rel.to_string_lossy());
                    let full_norm = project::normalize_path(&p.to_string_lossy());
                    let depth = rel_norm.matches('/').count();
                    let name = p.file_name()?.to_string_lossy().to_string();
                    let rel_parent = rel
                        .parent()
                        .map(|d| project::normalize_path(&d.to_string_lossy()))
                        .unwrap_or_default();
                    Some(ProjectCircuit {
                        depth,
                        rel_lower: rel_norm.to_ascii_lowercase(),
                        full_norm,
                        name,
                        rel_parent,
                    })
                })
                .collect();

            entries.sort_by(|a, b| {
                a.depth
                    .cmp(&b.depth)
                    .then_with(|| a.rel_lower.cmp(&b.rel_lower))
            });

            for entry in entries {
                let key = path_key(&entry.full_norm);
                if !seen.insert(key.clone()) {
                    continue;
                }
                let is_current = !current_key.is_empty() && key == current_key;
                proj_items.push(json!({
                    "kind": "item",
                    "name": entry.name,
                    "detail": entry.rel_parent,
                    "path": entry.full_norm,
                    "tooltip": entry.full_norm,
                    "current": is_current,
                    "icon": UiIcon::Schema.as_str(),
                }));
            }
        }
    }

    let mut recent_items: Vec<Value> = Vec::new();
    for raw in recent_circuits {
        let norm = project::normalize_path(&strip_file_url(raw));
        if norm.is_empty() || !Path::new(&norm).is_file() {
            continue;
        }
        let key = path_key(&norm);
        if !seen.insert(key.clone()) {
            continue;
        }
        let name = Path::new(&norm)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&norm)
            .to_string();
        let is_current = !current_key.is_empty() && key == current_key;
        recent_items.push(json!({
            "kind": "item",
            "name": name,
            "detail": parent_detail(&norm),
            "path": norm,
            "tooltip": norm,
            "current": is_current,
            "icon": UiIcon::Schema.as_str(),
        }));
    }

    let mut rows: Vec<Value> = Vec::new();
    if !proj_items.is_empty() {
        rows.push(json!({
            "kind": "header",
            "text": i18n::tr("In Project"),
        }));
        rows.extend(proj_items);
    }
    if !recent_items.is_empty() {
        rows.push(json!({
            "kind": "header",
            "text": i18n::tr("Recent"),
        }));
        rows.extend(recent_items);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_browser_root_path_sanitization_and_entries() {
        let temp_dir = std::env::temp_dir().join("cs_test_fb_dir");
        let _ = fs::create_dir_all(&temp_dir);
        let temp_file = temp_dir.join("sample.c");
        let _ = fs::write(&temp_file, "int main() {}");
        let temp_str = temp_dir.to_string_lossy().replace('\\', "/");

        let mut fb = FileBrowser {
            root_path: String::new(),
            show_hidden: false,
            clipboard: None,
            expanded_paths: HashSet::new(),
            expand_all: false,
        };

        #[cfg(target_os = "windows")]
        {
            let url = format!("file:///{temp_str}");
            fb.root_path = strip_file_url(&url);
            assert_eq!(fb.root_path(), temp_str);
            assert!(fb.has_project());

            let slashed = format!("/{temp_str}");
            fb.root_path = strip_file_url(&slashed);
            assert_eq!(fb.root_path(), temp_str);
            assert!(fb.has_project());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let url = format!("file://{temp_str}");
            fb.root_path = strip_file_url(&url);
            assert_eq!(fb.root_path(), temp_str);
            assert!(fb.has_project());
        }

        let entries = fb.entries();
        assert!(entries.is_array());
        let arr = entries.as_array().unwrap();
        assert!(!arr.is_empty());
        assert_eq!(arr[0]["name"], "sample.c");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recent_project_items_dedupes_marks_current_and_skips_missing() {
        let base = std::env::temp_dir().join("cs_test_recent_projects");
        let _ = fs::remove_dir_all(&base);
        let p1 = base.join("alpha");
        let p2 = base.join("beta");
        let missing = base.join("gone");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        let p1_str = p1.to_string_lossy().replace('\\', "/");
        let p2_str = p2.to_string_lossy().replace('\\', "/");
        let missing_str = missing.to_string_lossy().replace('\\', "/");

        let rows = build_recent_project_items(
            &p1_str,
            &[p2_str.clone(), format!("{p1_str}/"), missing_str],
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "alpha");
        assert_eq!(rows[0]["current"], true);
        assert_eq!(rows[1]["name"], "beta");
        assert_eq!(rows[1]["current"], false);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_circuit_switcher_lists_project_circuits_first_then_deduped_recents() {
        let base = std::env::temp_dir().join("cs_test_circuit_switcher");
        let _ = fs::remove_dir_all(&base);
        let proj = base.join("my_proj");
        let sub = proj.join("sub");
        let ignored = proj.join("target");
        let other = base.join("other_proj");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(&ignored).unwrap();
        fs::create_dir_all(&other).unwrap();

        let root_a = proj.join("a_root.circ1");
        let root_b = proj.join("b_root.sim1");
        let nested = sub.join("nested.circ1");
        let non_circ = proj.join("firmware.c");
        let ignored_circ = ignored.join("ignored.circ1");
        let recent_ext = other.join("external.sim1");
        let missing = other.join("missing.circ1");

        for f in [
            &root_a,
            &root_b,
            &nested,
            &non_circ,
            &ignored_circ,
            &recent_ext,
        ] {
            fs::write(f, "<circuit/>").unwrap();
        }

        let proj_str = proj.to_string_lossy().replace('\\', "/");
        let root_b_str = root_b.to_string_lossy().replace('\\', "/");
        let recent_ext_str = recent_ext.to_string_lossy().replace('\\', "/");
        let missing_str = missing.to_string_lossy().replace('\\', "/");

        let rows = build_circuit_switcher_items(
            &proj_str,
            false,
            &root_b_str,
            &[root_b_str.clone(), recent_ext_str.clone(), missing_str],
        );

        let items: Vec<&Value> = rows.iter().filter(|r| r["kind"] == "item").collect();
        let headers: Vec<&Value> = rows.iter().filter(|r| r["kind"] == "header").collect();
        assert_eq!(headers.len(), 2);
        assert_eq!(items.len(), 4);

        // Project circuits come first: root-level alphabetically, then nested.
        assert_eq!(items[0]["name"], "a_root.circ1");
        assert_eq!(items[0]["detail"], "");
        assert_eq!(items[0]["current"], false);

        assert_eq!(items[1]["name"], "b_root.sim1");
        assert_eq!(items[1]["detail"], "");
        assert_eq!(items[1]["current"], true);

        assert_eq!(items[2]["name"], "nested.circ1");
        assert_eq!(items[2]["detail"], "sub");
        assert_eq!(items[2]["current"], false);

        // Recent section contains external.sim1 only (b_root.sim1 deduped, missing skipped).
        assert_eq!(items[3]["name"], "external.sim1");
        assert_eq!(items[3]["current"], false);

        let _ = fs::remove_dir_all(&base);
    }
}
