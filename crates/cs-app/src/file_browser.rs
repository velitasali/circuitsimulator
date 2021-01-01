//! File browser façade. Rows are JSON (no `QFileSystemModel` / `QModelIndex`).

use crate::path_util::strip_file_url;
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
        let ext = p
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "circ1" | "circ" | "sim" | "sim1" | "sim2" => self.open_circuit(path),
            _ => self.open_file(path),
        }
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
}
