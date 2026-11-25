//! File browser façade. Rows are JSON (no `QFileSystemModel` / `QModelIndex`).

use crate::file_watcher::{TreeWatcher, WatchScope};
use crate::macos_host;
use crate::path_util::strip_file_url;
use crate::ui_icon::UiIcon;
use cs_engine::compat::simulide;
use cs_engine::{i18n, project, settings};
use qtbridge::{QObjectHolder, QmlMethodInvoker, qobject};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static WATCHER_INVOKER: Mutex<Option<QmlMethodInvoker>> = Mutex::new(None);
static WATCHER_REFRESH_PENDING: AtomicBool = AtomicBool::new(false);

fn schedule_watcher_refresh() {
    if WATCHER_REFRESH_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    macos_host::post_on_main(on_watcher_refresh_main);
}

extern "C" fn on_watcher_refresh_main() {
    WATCHER_REFRESH_PENDING.store(false, Ordering::Release);
    if let Ok(guard) = WATCHER_INVOKER.lock()
        && let Some(inv) = guard.as_ref()
    {
        let _ = inv.invoke_method("refreshFromWatcher");
    }
}

struct BrowserWatcher {
    inner: Option<TreeWatcher>,
}

impl BrowserWatcher {
    fn new() -> Self {
        Self {
            inner: Some(TreeWatcher::new(schedule_watcher_refresh)),
        }
    }

    fn sync_scope(&mut self, scope: Option<WatchScope>) {
        if let Some(watcher) = self.inner.as_mut() {
            watcher.sync_scope(scope);
        }
    }
}

impl Drop for BrowserWatcher {
    fn drop(&mut self) {
        self.inner.take();
        if let Ok(mut guard) = WATCHER_INVOKER.lock() {
            *guard = None;
        }
        WATCHER_REFRESH_PENDING.store(false, Ordering::Release);
    }
}

pub struct FileBrowser {
    root_path: String,
    show_hidden: bool,
    clipboard: Option<String>,
    expanded_paths: HashSet<String>,
    expand_all: bool,
    watcher: Option<BrowserWatcher>,
    last_has_project: bool,
    last_visible_snapshot: Option<Vec<(String, bool)>>,
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
        let last_has_project = !root_path.is_empty();
        Self {
            root_path,
            show_hidden: false,
            clipboard: None,
            expanded_paths: HashSet::new(),
            expand_all: false,
            watcher: None,
            last_has_project,
            last_visible_snapshot: None,
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
    #[qsignal]
    fn path_revealed(&mut self, path: String);

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
        self.sync_watcher();
        self.root_path_changed();
        self.notify_entries_changed();
    }

    fn set_show_hidden(&mut self, hidden: bool) {
        if self.show_hidden == hidden {
            return;
        }
        self.show_hidden = hidden;
        self.sync_watcher();
        self.show_hidden_changed();
        self.notify_entries_changed();
    }

    fn entries(&self) -> Value {
        let mut rows = Vec::new();
        self.walk_visible(|name, path, is_dir, depth, expanded| {
            rows.push(json!({
                "name": name,
                "path": path,
                "isDir": is_dir,
                "depth": depth,
                "expanded": expanded,
            }));
        });
        Value::Array(rows)
    }

    fn visible_snapshot(&self) -> Vec<(String, bool)> {
        let mut out = Vec::new();
        self.walk_visible(|_, path, is_dir, _, _| {
            out.push((path, is_dir));
        });
        out
    }

    fn walk_visible(&self, mut visit: impl FnMut(String, String, bool, i32, bool)) {
        if self.root_path.is_empty() {
            return;
        }
        let root = PathBuf::from(&self.root_path);
        if !root.is_dir() {
            return;
        }
        self.walk_dir(&root, 0, &mut visit);
    }

    fn walk_dir(
        &self,
        dir: &Path,
        depth: i32,
        visit: &mut impl FnMut(String, String, bool, i32, bool),
    ) {
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

            visit(name, path_str, is_dir, depth, expanded);

            if is_dir && expanded {
                self.walk_dir(&path, depth + 1, visit);
            }
        }
    }

    fn build_watch_scope(&self) -> Option<WatchScope> {
        if self.root_path.is_empty() {
            return None;
        }
        let root = PathBuf::from(&self.root_path);
        if !root.is_dir() {
            return None;
        }
        Some(WatchScope {
            root,
            expanded_dirs: self.expanded_paths.iter().map(PathBuf::from).collect(),
            expand_all: self.expand_all,
            show_hidden: self.show_hidden,
        })
    }

    fn sync_watcher(&mut self) {
        let scope = self.build_watch_scope();
        if let Some(watcher) = self.watcher.as_mut() {
            watcher.sync_scope(scope);
        }
    }

    fn notify_entries_changed(&mut self) {
        self.last_has_project = self.has_project();
        self.last_visible_snapshot = Some(self.visible_snapshot());
        self.entries_changed();
    }

    #[qslot]
    fn start_watching(&mut self) {
        if let Ok(mut guard) = WATCHER_INVOKER.lock()
            && guard.is_none()
        {
            *guard = Some(self.get_qml_method_invoker());
        }
        if self.watcher.is_none() {
            self.watcher = Some(BrowserWatcher::new());
        }
        self.last_has_project = self.has_project();
        self.last_visible_snapshot = Some(self.visible_snapshot());
        self.sync_watcher();
    }

    #[qslot]
    fn refresh_from_watcher(&mut self) {
        let prev_expanded_len = self.expanded_paths.len();
        self.expanded_paths.retain(|p| Path::new(p).is_dir());

        if let Some(clip) = &self.clipboard
            && !Path::new(clip).exists()
        {
            self.clipboard = None;
            self.clipboard_changed();
        }

        let has_project_now = self.has_project();
        let snapshot = self.visible_snapshot();

        if has_project_now != self.last_has_project {
            self.last_has_project = has_project_now;
            self.last_visible_snapshot = Some(snapshot);
            self.sync_watcher();
            self.root_path_changed();
            self.entries_changed();
            return;
        }

        if self.expanded_paths.len() != prev_expanded_len || self.expand_all {
            self.sync_watcher();
        }

        if self.last_visible_snapshot.as_ref() != Some(&snapshot) {
            self.last_visible_snapshot = Some(snapshot);
            self.entries_changed();
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
        self.sync_watcher();
        self.notify_entries_changed();
    }

    #[qslot]
    fn expand_all(&mut self) {
        self.expand_all = true;
        self.sync_watcher();
        self.notify_entries_changed();
    }

    #[qslot]
    fn collapse_all(&mut self) {
        self.expand_all = false;
        self.expanded_paths.clear();
        self.sync_watcher();
        self.notify_entries_changed();
    }

    #[qslot]
    fn reload(&mut self) {
        self.expanded_paths.retain(|p| Path::new(p).is_dir());
        self.sync_watcher();
        self.notify_entries_changed();
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
            if self.expand_all {
                self.sync_watcher();
            }
            self.notify_entries_changed();
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
            self.sync_watcher();
            self.notify_entries_changed();
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
            self.sync_watcher();
            self.notify_entries_changed();
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
            self.expanded_paths.retain(|p| Path::new(p).is_dir());
            self.sync_watcher();
            self.notify_entries_changed();
        }
    }

    #[qslot]
    fn trash(&mut self, path: String) -> bool {
        let path = strip_file_url(&path);
        let p = PathBuf::from(&path);
        if !p.exists() {
            return true;
        }
        if macos_host::move_to_trash(&p) {
            self.expanded_paths.retain(|p| Path::new(p).is_dir());
            self.sync_watcher();
            self.notify_entries_changed();
            true
        } else {
            false
        }
    }

    #[qslot]
    fn delete_path(&mut self, path: String) -> bool {
        let path = strip_file_url(&path);
        let p = PathBuf::from(&path);
        if !p.exists() {
            return true;
        }
        let ok = remove_path_permanently(&p);
        self.expanded_paths.retain(|p| Path::new(p).is_dir());
        self.sync_watcher();
        self.notify_entries_changed();
        ok
    }

    #[qslot]
    fn open_externally(&self, path: String) {
        if let Some(target) = resolve_external_target(&self.root_path, &path) {
            open_in_system(&target);
        }
    }

    #[qslot]
    fn open_parent_externally(&self, path: String) {
        if let Some(target) = resolve_parent_target(&self.root_path, &path) {
            open_in_system(&target);
        }
    }

    #[qslot]
    fn is_in_project(&self, path: String) -> bool {
        resolve_project_entry(&self.root_path, &path).is_some()
    }

    #[qslot]
    fn path_exists(&self, path: String) -> bool {
        resolve_existing_path(&path).is_some()
    }

    #[qslot]
    fn parent_dir_exists(&self, path: String) -> bool {
        resolve_existing_parent_dir(&path).is_some()
    }

    fn reveal_path_inner(&mut self, path: &str) -> Option<(String, bool)> {
        let resolved = resolve_project_entry(&self.root_path, path)?;
        let mut hidden_changed = false;
        if resolved.has_hidden_segment && !self.show_hidden {
            self.show_hidden = true;
            hidden_changed = true;
        }
        for dir in resolved.ancestor_dirs {
            self.expanded_paths.insert(dir);
        }
        Some((resolved.entry_path, hidden_changed))
    }

    #[qslot]
    fn reveal_path(&mut self, path: String) -> bool {
        let Some((entry_path, hidden_changed)) = self.reveal_path_inner(&path) else {
            return false;
        };
        if hidden_changed {
            self.show_hidden_changed();
        }
        self.sync_watcher();
        self.notify_entries_changed();
        self.path_revealed(entry_path);
        true
    }
}

struct ResolvedProjectEntry {
    entry_path: String,
    ancestor_dirs: Vec<String>,
    has_hidden_segment: bool,
}

fn resolve_project_entry(root_path: &str, path: &str) -> Option<ResolvedProjectEntry> {
    let cleaned_root = strip_file_url(root_path);
    let cleaned_target = strip_file_url(path);
    if cleaned_root.is_empty() || cleaned_target.is_empty() {
        return None;
    }

    let root = Path::new(&cleaned_root);
    let target = Path::new(&cleaned_target);
    if !root.is_dir() || !target.exists() {
        return None;
    }

    let canon_root = fs::canonicalize(root).ok()?;
    let rel = match fs::canonicalize(target)
        .ok()
        .and_then(|ct| ct.strip_prefix(&canon_root).ok().map(Path::to_path_buf))
    {
        Some(rel) if !rel.as_os_str().is_empty() => rel,
        _ => {
            let parent = target.parent()?;
            let name = target.file_name()?;
            let canon_parent = fs::canonicalize(parent).ok()?;
            let parent_rel = canon_parent.strip_prefix(&canon_root).ok()?;
            parent_rel.join(name)
        }
    };

    let segments: Vec<&OsStr> = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s),
            _ => None,
        })
        .collect();
    if segments.is_empty() {
        return None;
    }

    let mut has_hidden_segment = false;
    let mut ancestor_dirs = Vec::new();
    let mut cur = PathBuf::from(&cleaned_root);
    for (idx, seg) in segments.iter().enumerate() {
        if seg.to_string_lossy().starts_with('.') {
            has_hidden_segment = true;
        }
        cur.push(seg);
        let formatted = cur.to_string_lossy().replace('\\', "/");
        if idx + 1 < segments.len() {
            ancestor_dirs.push(formatted);
        } else {
            return Some(ResolvedProjectEntry {
                entry_path: formatted,
                ancestor_dirs,
                has_hidden_segment,
            });
        }
    }
    None
}

fn remove_path_permanently(p: &Path) -> bool {
    if p.is_dir() {
        fs::remove_dir_all(p).is_ok()
    } else {
        fs::remove_file(p).is_ok()
    }
}

fn native_system_path(p: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(p.to_string_lossy().replace('/', "\\"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        p.to_path_buf()
    }
}

fn resolve_existing_path(path: &str) -> Option<PathBuf> {
    let cleaned = strip_file_url(path);
    if cleaned.is_empty() {
        return None;
    }
    let native = native_system_path(Path::new(&cleaned));
    native.exists().then_some(native)
}

fn resolve_existing_parent_dir(path: &str) -> Option<PathBuf> {
    let cleaned = strip_file_url(path);
    if cleaned.is_empty() {
        return None;
    }
    let native = native_system_path(Path::new(&cleaned));
    if let Some(parent) = native
        .parent()
        .filter(|p| !p.as_os_str().is_empty() && p.exists())
    {
        Some(native_system_path(parent))
    } else if native.is_dir() {
        Some(native)
    } else {
        None
    }
}

fn resolve_external_target(root_path: &str, path: &str) -> Option<PathBuf> {
    let cleaned = strip_file_url(path);
    let candidate = if cleaned.is_empty() {
        strip_file_url(root_path)
    } else {
        cleaned
    };
    if candidate.is_empty() {
        return None;
    }
    let native = native_system_path(Path::new(&candidate));
    native.exists().then_some(native)
}

fn resolve_parent_target(root_path: &str, path: &str) -> Option<PathBuf> {
    let cleaned = strip_file_url(path);
    if !cleaned.is_empty() {
        return resolve_existing_parent_dir(&cleaned);
    }
    let target = resolve_external_target(root_path, "")?;
    if let Some(parent) = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty() && p.exists())
    {
        Some(native_system_path(parent))
    } else {
        Some(target)
    }
}

fn open_in_system(p: &Path) {
    let native = native_system_path(p);
    if macos_host::open_local_path(&native) {
        return;
    }
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(&native).spawn();
    #[cfg(target_os = "windows")]
    let _ = Command::new("explorer").arg(&native).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = Command::new("xdg-open").arg(&native).spawn();
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
            watcher: None,
            last_has_project: false,
            last_visible_snapshot: None,
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

    #[test]
    fn test_remove_path_permanently_deletes_files_and_dirs() {
        let base = std::env::temp_dir().join("cs_test_remove_path_permanently");
        let _ = fs::remove_dir_all(&base);
        let nested_dir = base.join("sub");
        fs::create_dir_all(&nested_dir).unwrap();
        let file = base.join("a.txt");
        let nested_file = nested_dir.join("b.txt");
        fs::write(&file, "a").unwrap();
        fs::write(&nested_file, "b").unwrap();

        assert!(remove_path_permanently(&file));
        assert!(!file.exists());

        assert!(remove_path_permanently(&nested_dir));
        assert!(!nested_dir.exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_resolve_external_and_parent_targets() {
        let base = std::env::temp_dir().join("cs_test_external_targets");
        let _ = fs::remove_dir_all(&base);
        let nested_dir = base.join("sub");
        fs::create_dir_all(&nested_dir).unwrap();
        let nested_file = nested_dir.join("main.c");
        fs::write(&nested_file, "int main() {}").unwrap();

        let base_forward = base.to_string_lossy().replace('\\', "/");
        let dir_forward = nested_dir.to_string_lossy().replace('\\', "/");
        let file_forward = nested_file.to_string_lossy().replace('\\', "/");

        let ext_file = resolve_external_target(&base_forward, &file_forward).unwrap();
        assert_eq!(ext_file, native_system_path(&nested_file));

        let parent_of_file = resolve_parent_target(&base_forward, &file_forward).unwrap();
        assert_eq!(parent_of_file, native_system_path(&nested_dir));

        let parent_of_dir = resolve_parent_target(&base_forward, &dir_forward).unwrap();
        assert_eq!(parent_of_dir, native_system_path(&base));

        let fallback_root = resolve_external_target(&base_forward, "").unwrap();
        assert_eq!(fallback_root, native_system_path(&base));

        #[cfg(target_os = "windows")]
        {
            assert!(!ext_file.to_string_lossy().contains('/'));
            assert!(!parent_of_file.to_string_lossy().contains('/'));
            assert!(!parent_of_dir.to_string_lossy().contains('/'));
        }

        // Deleted file still resolves its existing parent directory.
        let deleted_file = nested_dir.join("removed.c");
        let deleted_forward = deleted_file.to_string_lossy().replace('\\', "/");
        assert!(resolve_existing_path(&deleted_forward).is_none());
        assert_eq!(
            resolve_parent_target(&base_forward, &deleted_forward).unwrap(),
            native_system_path(&nested_dir)
        );
        assert!(resolve_existing_path("").is_none());
        assert!(resolve_existing_parent_dir("").is_none());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_resolve_project_entry_and_reveal_expands_ancestors_and_hidden() {
        let base = std::env::temp_dir().join("cs_test_reveal_project_entry");
        let _ = fs::remove_dir_all(&base);
        let proj = base.join("proj");
        let sibling = base.join("proj2");
        let nested_dir = proj.join("src").join("drivers");
        let hidden_dir = proj.join(".hidden_cfg");
        fs::create_dir_all(&nested_dir).unwrap();
        fs::create_dir_all(&hidden_dir).unwrap();
        fs::create_dir_all(&sibling).unwrap();

        let top_file = proj.join("main.c");
        let nested_file = nested_dir.join("uart.c");
        let hidden_file = hidden_dir.join("config.json");
        let outside_file = sibling.join("other.c");
        for f in [&top_file, &nested_file, &hidden_file, &outside_file] {
            fs::write(f, "test").unwrap();
        }

        let proj_str = proj.to_string_lossy().replace('\\', "/");
        let top_str = top_file.to_string_lossy().replace('\\', "/");
        let nested_str = nested_file.to_string_lossy().replace('\\', "/");
        let hidden_str = hidden_file.to_string_lossy().replace('\\', "/");
        let outside_str = outside_file.to_string_lossy().replace('\\', "/");

        let mut fb = FileBrowser {
            root_path: proj_str.clone(),
            show_hidden: false,
            clipboard: None,
            expanded_paths: HashSet::new(),
            expand_all: false,
            watcher: None,
            last_has_project: true,
            last_visible_snapshot: None,
        };

        assert!(fb.is_in_project(top_str.clone()));
        assert!(fb.is_in_project(nested_str.clone()));
        assert!(fb.is_in_project(hidden_str.clone()));
        assert!(!fb.is_in_project(outside_str.clone()));
        assert!(!fb.is_in_project(proj_str.clone()));
        assert!(!fb.is_in_project(String::new()));

        // Nested file is not visible until revealed.
        let entries_before = fb.entries();
        assert!(
            !entries_before
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["path"] == nested_str)
        );

        let (revealed_nested, hidden_changed) = fb.reveal_path_inner(&nested_str).unwrap();
        assert_eq!(revealed_nested, nested_str);
        assert!(!hidden_changed);
        assert!(!fb.show_hidden);
        let entries_after_nested = fb.entries();
        assert!(
            entries_after_nested
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["path"] == revealed_nested)
        );

        // Revealing a file under a dot-directory enables show_hidden and exposes it in entries.
        let (revealed_hidden, hidden_changed) = fb.reveal_path_inner(&hidden_str).unwrap();
        assert_eq!(revealed_hidden, hidden_str);
        assert!(hidden_changed);
        assert!(fb.show_hidden);
        let entries_after_hidden = fb.entries();
        assert!(
            entries_after_hidden
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["path"] == revealed_hidden)
        );

        // Outside file cannot be revealed.
        assert!(fb.reveal_path_inner(&outside_str).is_none());

        let _ = fs::remove_dir_all(&base);
    }
}
