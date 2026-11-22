//! Cross-platform project file-tree watcher backed by `notify`.
//!
//! Watches for created, deleted, and renamed files or directories:
//! - Windows (`ReadDirectoryChangesW`) and macOS (`FSEvents`) use a single
//!   recursive watch on the project root and filter events in-process to the
//!   currently visible directories. This avoids locking child folders on
//!   Windows and avoids FSEvents stream rebuilds / FD pressure on macOS.
//! - Linux (`inotify`) and other Unix platforms watch only the root and
//!   currently expanded directories non-recursively so large ignored trees
//!   (`.git`, `target`, `build`, `node_modules`) never exhaust
//!   `fs.inotify.max_user_watches`.
//! - If a native watcher fails to start (e.g. unsupported FS or watch limit),
//!   it automatically falls back to `PollWatcher` over the visible directories.

use notify::event::ModifyKind;
use notify::{
    Config, ErrorKind, Event, EventKind, EventKindMask, PollWatcher, RecommendedWatcher,
    RecursiveMode, Watcher, WindowsPathSeparatorStyle,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(120);
const MAX_DEBOUNCE_DELAY: Duration = Duration::from_millis(500);
const POLL_FALLBACK_INTERVAL: Duration = Duration::from_secs(2);
const MAX_NON_RECURSIVE_WATCHES: usize = 512;
const IGNORED_EXPAND_ALL_DIRS: &[&str] = &[".git", "target", "build", "node_modules"];

/// On Windows and macOS a single recursive watch on the root uses one OS handle
/// / stream and avoids locking child folders or rebuilding FSEvents streams.
const USE_RECURSIVE_ROOT_WATCH: bool = cfg!(any(windows, target_os = "macos"));

/// Describes which directories in the file browser are currently visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchScope {
    pub root: PathBuf,
    pub expanded_dirs: Vec<PathBuf>,
    pub expand_all: bool,
    pub show_hidden: bool,
}

#[derive(Default)]
struct FilterState {
    root_keys: HashSet<String>,
    visible_dir_keys: HashSet<String>,
    expand_all: bool,
    show_hidden: bool,
}

impl FilterState {
    fn from_scope(scope: &WatchScope) -> Self {
        let mut root_keys = HashSet::new();
        let mut visible_dir_keys = HashSet::new();

        insert_path_variants(&scope.root, &mut root_keys);
        insert_path_variants(&scope.root, &mut visible_dir_keys);

        for dir in &scope.expanded_dirs {
            insert_path_variants(dir, &mut visible_dir_keys);
        }

        Self {
            root_keys,
            visible_dir_keys,
            expand_all: scope.expand_all,
            show_hidden: scope.show_hidden,
        }
    }
}

enum BackendWatcher {
    Native {
        watcher: RecommendedWatcher,
        recursive_root: bool,
    },
    Poll {
        watcher: PollWatcher,
    },
}

impl BackendWatcher {
    fn is_recursive_root(&self) -> bool {
        matches!(
            self,
            Self::Native {
                recursive_root: true,
                ..
            }
        )
    }

    fn watch(&mut self, path: &Path, mode: RecursiveMode) -> notify::Result<()> {
        match self {
            Self::Native { watcher, .. } => watcher.watch(path, mode),
            Self::Poll { watcher } => watcher.watch(path, mode),
        }
    }

    fn unwatch(&mut self, path: &Path) -> notify::Result<()> {
        match self {
            Self::Native { watcher, .. } => watcher.unwatch(path),
            Self::Poll { watcher } => watcher.unwatch(path),
        }
    }
}

struct ActiveWatch {
    root: PathBuf,
    backend: Option<BackendWatcher>,
    watched_dirs: HashSet<PathBuf>,
    filter: Arc<Mutex<FilterState>>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for ActiveWatch {
    fn drop(&mut self) {
        // Drop the OS watcher first so its event sender closes and wakes the worker.
        self.backend.take();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

/// Manages a debounced filesystem watcher for the active project tree.
pub struct TreeWatcher {
    on_change: Arc<dyn Fn() + Send + Sync>,
    debounce: Duration,
    max_delay: Duration,
    active: Option<ActiveWatch>,
}

impl TreeWatcher {
    pub fn new<F>(on_change: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self::with_timings(on_change, DEBOUNCE_WINDOW, MAX_DEBOUNCE_DELAY)
    }

    fn with_timings<F>(on_change: F, debounce: Duration, max_delay: Duration) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            on_change: Arc::new(on_change),
            debounce,
            max_delay,
            active: None,
        }
    }

    /// Stops watching the current project, if any.
    pub fn clear(&mut self) {
        self.active = None;
    }

    /// Synchronizes the watcher with the current project root and visible folders.
    pub fn sync_scope(&mut self, scope: Option<WatchScope>) {
        let Some(scope) = scope else {
            self.clear();
            return;
        };

        if !scope.root.is_dir() {
            self.clear();
            return;
        }

        let same_root = self
            .active
            .as_ref()
            .is_some_and(|active| active.root == scope.root);

        if same_root {
            if let Some(active) = self.active.as_mut() {
                update_filter(&active.filter, &scope);
                if let Some(backend) = active.backend.as_mut()
                    && !backend.is_recursive_root()
                {
                    let desired = collect_non_recursive_dirs(&scope);
                    sync_non_recursive_watches(backend, &mut active.watched_dirs, desired);
                }
            }
            return;
        }

        // Switching to a new project root: drop the old watcher before creating the new one.
        self.active = None;
        self.active = self.start_watch(scope);
    }

    fn start_watch(&self, scope: WatchScope) -> Option<ActiveWatch> {
        let filter = Arc::new(Mutex::new(FilterState::from_scope(&scope)));
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let (mut backend, mut watched_dirs) = create_backend(&scope, tx)?;

        if !backend.is_recursive_root() {
            let desired = collect_non_recursive_dirs(&scope);
            sync_non_recursive_watches(&mut backend, &mut watched_dirs, desired);
        }

        let worker = spawn_debounce_worker(
            rx,
            Arc::clone(&filter),
            Arc::clone(&self.on_change),
            self.debounce,
            self.max_delay,
        )?;

        Some(ActiveWatch {
            root: scope.root,
            backend: Some(backend),
            watched_dirs,
            filter,
            worker: Some(worker),
        })
    }
}

fn tree_event_mask() -> EventKindMask {
    EventKindMask::CREATE | EventKindMask::REMOVE | EventKindMask::MODIFY_NAME
}

fn create_backend(
    scope: &WatchScope,
    tx: mpsc::Sender<notify::Result<Event>>,
) -> Option<(BackendWatcher, HashSet<PathBuf>)> {
    let native_config = Config::default()
        .with_follow_symlinks(false)
        .with_event_kinds(tree_event_mask())
        .with_windows_path_separator_style(WindowsPathSeparatorStyle::Slash);

    if let Ok(mut watcher) = RecommendedWatcher::new(tx.clone(), native_config) {
        let mode = if USE_RECURSIVE_ROOT_WATCH {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        if watcher.watch(&scope.root, mode).is_ok() {
            let mut watched = HashSet::new();
            watched.insert(scope.root.clone());
            return Some((
                BackendWatcher::Native {
                    watcher,
                    recursive_root: USE_RECURSIVE_ROOT_WATCH,
                },
                watched,
            ));
        }
    }

    // Fallback for unsupported filesystems, containers, or exhausted native limits.
    let poll_config = Config::default()
        .with_poll_interval(POLL_FALLBACK_INTERVAL)
        .with_compare_contents(false)
        .with_follow_symlinks(false)
        .with_event_kinds(tree_event_mask());

    let mut poll = PollWatcher::new(tx, poll_config).ok()?;
    poll.watch(&scope.root, RecursiveMode::NonRecursive).ok()?;
    let mut watched = HashSet::new();
    watched.insert(scope.root.clone());
    Some((BackendWatcher::Poll { watcher: poll }, watched))
}

fn update_filter(filter: &Arc<Mutex<FilterState>>, scope: &WatchScope) {
    let next = FilterState::from_scope(scope);
    let mut guard = filter.lock().unwrap_or_else(|e| e.into_inner());
    *guard = next;
}

fn collect_non_recursive_dirs(scope: &WatchScope) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    if !scope.root.is_dir() {
        return dirs;
    }
    dirs.insert(scope.root.clone());

    if scope.expand_all {
        collect_expanded_subtrees(&scope.root, scope.show_hidden, &mut dirs);
    } else {
        for dir in &scope.expanded_dirs {
            if dirs.len() >= MAX_NON_RECURSIVE_WATCHES {
                break;
            }
            if dir.is_dir() {
                dirs.insert(dir.clone());
            }
        }
    }
    dirs
}

fn collect_expanded_subtrees(dir: &Path, show_hidden: bool, out: &mut HashSet<PathBuf>) {
    if out.len() >= MAX_NON_RECURSIVE_WATCHES {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        if out.len() >= MAX_NON_RECURSIVE_WATCHES {
            return;
        }
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() || ft.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if IGNORED_EXPAND_ALL_DIRS.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        if out.insert(path.clone()) {
            collect_expanded_subtrees(&path, show_hidden, out);
        }
    }
}

fn sync_non_recursive_watches(
    backend: &mut BackendWatcher,
    watched: &mut HashSet<PathBuf>,
    desired: HashSet<PathBuf>,
) {
    let to_remove: Vec<PathBuf> = watched.difference(&desired).cloned().collect();
    for path in to_remove {
        let _ = backend.unwatch(&path);
        watched.remove(&path);
    }

    let to_add: Vec<PathBuf> = desired.difference(watched).cloned().collect();
    for path in to_add {
        match backend.watch(&path, RecursiveMode::NonRecursive) {
            Ok(()) => {
                watched.insert(path);
            }
            Err(err) if matches!(err.kind, ErrorKind::MaxFilesWatch) => break,
            Err(_) => {}
        }
    }
}

fn spawn_debounce_worker(
    rx: mpsc::Receiver<notify::Result<Event>>,
    filter: Arc<Mutex<FilterState>>,
    on_change: Arc<dyn Fn() + Send + Sync>,
    debounce: Duration,
    max_delay: Duration,
) -> Option<JoinHandle<()>> {
    let builder = thread::Builder::new().name("cs-file-watcher".to_string());
    builder
        .spawn(move || {
            let mut first_event_at: Option<Instant> = None;
            let mut flush_deadline: Option<Instant> = None;

            loop {
                let received = match flush_deadline {
                    None => match rx.recv() {
                        Ok(ev) => Some(ev),
                        Err(_) => break,
                    },
                    Some(deadline) => {
                        let wait = deadline.saturating_duration_since(Instant::now());
                        match rx.recv_timeout(wait) {
                            Ok(ev) => Some(ev),
                            Err(RecvTimeoutError::Timeout) => {
                                first_event_at = None;
                                flush_deadline = None;
                                on_change();
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    }
                };

                let Some(Ok(event)) = received else {
                    continue;
                };

                let relevant = {
                    let guard = filter.lock().unwrap_or_else(|e| e.into_inner());
                    is_relevant_event(&event, &guard)
                };

                if !relevant {
                    continue;
                }

                let now = Instant::now();
                let first = *first_event_at.get_or_insert(now);
                let max_deadline = first + max_delay;
                let next_deadline = (now + debounce).min(max_deadline);
                flush_deadline = Some(next_deadline);

                if now >= max_deadline {
                    first_event_at = None;
                    flush_deadline = None;
                    on_change();
                }
            }
        })
        .ok()
}

fn is_structural_event(event: &Event) -> bool {
    if event.need_rescan() {
        return true;
    }
    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Any
    )
}

fn is_relevant_event(event: &Event, state: &FilterState) -> bool {
    if !is_structural_event(event) {
        return false;
    }
    if event.need_rescan() || event.paths.is_empty() {
        return true;
    }

    for path in &event.paths {
        if is_relevant_path(path, state) {
            return true;
        }
    }
    false
}

fn is_relevant_path(path: &Path, state: &FilterState) -> bool {
    let key = normalize_key(path);
    if key.is_empty() {
        return false;
    }
    if state.root_keys.contains(&key) {
        return true;
    }

    if !state.show_hidden && has_hidden_relative_component(&key, &state.root_keys) {
        return false;
    }

    if state.expand_all {
        return is_under_any_root(&key, &state.root_keys);
    }

    if state.visible_dir_keys.contains(&key) {
        return true;
    }

    parent_key(&key).is_some_and(|parent| state.visible_dir_keys.contains(parent))
}

fn insert_path_variants(path: &Path, out: &mut HashSet<String>) {
    let raw = normalize_key(path);
    if !raw.is_empty() {
        out.insert(raw);
    }
    if let Ok(canon) = fs::canonicalize(path) {
        let canon_key = normalize_key(&canon);
        if !canon_key.is_empty() {
            out.insert(canon_key);
        }
    }
}

fn normalize_key(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/UNC/") {
        s = format!("//{stripped}");
    } else if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    while s.len() > 1 && s.ends_with('/') && !(s.len() == 3 && s.as_bytes()[1] == b':') {
        s.pop();
    }
    #[cfg(windows)]
    {
        s.make_ascii_lowercase();
    }
    s
}

fn parent_key(key: &str) -> Option<&str> {
    let idx = key.rfind('/')?;
    if idx == 0 {
        return Some("/");
    }
    if idx == 2 && key.as_bytes().get(1) == Some(&b':') {
        return Some(&key[..=idx]);
    }
    Some(&key[..idx])
}

fn relative_to_roots<'a>(key: &'a str, root_keys: &HashSet<String>) -> Option<&'a str> {
    for root in root_keys {
        if root == "/" {
            if let Some(rel) = key.strip_prefix('/') {
                return Some(rel);
            }
        } else if root.ends_with('/') {
            if let Some(rel) = key.strip_prefix(root.as_str()) {
                return Some(rel);
            }
        } else if let Some(rest) = key.strip_prefix(root.as_str())
            && let Some(rel) = rest.strip_prefix('/')
        {
            return Some(rel);
        }
    }
    None
}

fn is_under_any_root(key: &str, root_keys: &HashSet<String>) -> bool {
    relative_to_roots(key, root_keys).is_some()
}

fn has_hidden_relative_component(key: &str, root_keys: &HashSet<String>) -> bool {
    if let Some(rel) = relative_to_roots(key, root_keys) {
        return rel.split('/').any(|part| part.starts_with('.'));
    }
    key.rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, Flag, RemoveKind, RenameMode};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("cs_fw_{label}_{}_{nanos}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn wait_for_count(counter: &AtomicUsize, target: usize, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if counter.load(Ordering::Acquire) >= target {
                return true;
            }
            thread::sleep(Duration::from_millis(15));
        }
        counter.load(Ordering::Acquire) >= target
    }

    #[test]
    fn filter_ignores_content_writes_and_collapsed_dirs() {
        let root = PathBuf::from("/workspace/project");
        let expanded = root.join("src");
        let state = FilterState::from_scope(&WatchScope {
            root: root.clone(),
            expanded_dirs: vec![expanded.clone()],
            expand_all: false,
            show_hidden: false,
        });

        let write_ev = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)))
            .add_path(root.join("main.c"));
        assert!(!is_relevant_event(&write_ev, &state));

        let create_in_root =
            Event::new(EventKind::Create(CreateKind::File)).add_path(root.join("new.c"));
        assert!(is_relevant_event(&create_in_root, &state));

        let remove_in_expanded =
            Event::new(EventKind::Remove(RemoveKind::File)).add_path(expanded.join("lib.h"));
        assert!(is_relevant_event(&remove_in_expanded, &state));

        let create_in_collapsed = Event::new(EventKind::Create(CreateKind::File))
            .add_path(root.join("collapsed").join("deep.c"));
        assert!(!is_relevant_event(&create_in_collapsed, &state));

        let hidden_create =
            Event::new(EventKind::Create(CreateKind::File)).add_path(root.join(".env"));
        assert!(!is_relevant_event(&hidden_create, &state));

        let rename_ev = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(root.join(".tmp_save"))
            .add_path(root.join("saved.c"));
        assert!(is_relevant_event(&rename_ev, &state));

        let rescan_ev = Event::new(EventKind::Other).set_flag(Flag::Rescan);
        assert!(is_relevant_event(&rescan_ev, &state));
    }

    #[test]
    fn filter_allows_project_inside_hidden_ancestor_dir() {
        let root = PathBuf::from("/home/user/.config/my_project");
        let state = FilterState::from_scope(&WatchScope {
            root: root.clone(),
            expanded_dirs: vec![],
            expand_all: false,
            show_hidden: false,
        });

        let visible_file =
            Event::new(EventKind::Create(CreateKind::File)).add_path(root.join("board.sim1"));
        assert!(is_relevant_event(&visible_file, &state));

        let hidden_child =
            Event::new(EventKind::Create(CreateKind::File)).add_path(root.join(".DS_Store"));
        assert!(!is_relevant_event(&hidden_child, &state));
    }

    #[test]
    fn filter_expand_all_respects_show_hidden() {
        let root = PathBuf::from("/workspace/project");
        let state = FilterState::from_scope(&WatchScope {
            root: root.clone(),
            expanded_dirs: vec![],
            expand_all: true,
            show_hidden: false,
        });

        let deep_visible = Event::new(EventKind::Create(CreateKind::File))
            .add_path(root.join("a").join("b").join("c.circ1"));
        assert!(is_relevant_event(&deep_visible, &state));

        let deep_hidden = Event::new(EventKind::Create(CreateKind::File))
            .add_path(root.join(".git").join("objects").join("pack"));
        assert!(!is_relevant_event(&deep_hidden, &state));
    }

    #[test]
    fn watcher_detects_create_rename_delete_and_expanded_subdirs() {
        let root = unique_temp_dir("crud");
        let sub = root.join("sub");
        fs::create_dir_all(&sub).unwrap();

        let hits = Arc::new(AtomicUsize::new(0));
        let hits_cb = Arc::clone(&hits);
        let mut watcher = TreeWatcher::with_timings(
            move || {
                hits_cb.fetch_add(1, Ordering::Release);
            },
            Duration::from_millis(25),
            Duration::from_millis(120),
        );

        watcher.sync_scope(Some(WatchScope {
            root: root.clone(),
            expanded_dirs: vec![],
            expand_all: false,
            show_hidden: false,
        }));

        // Let the OS watch settle before mutating the directory.
        thread::sleep(Duration::from_millis(50));

        // 1. Create file in root.
        let file_a = root.join("alpha.c");
        fs::write(&file_a, "int a = 1;").unwrap();
        assert!(
            wait_for_count(&hits, 1, Duration::from_secs(3)),
            "expected notification on file creation"
        );

        // 2. Rename file in root.
        let before_rename = hits.load(Ordering::Acquire);
        let file_b = root.join("beta.c");
        fs::rename(&file_a, &file_b).unwrap();
        assert!(
            wait_for_count(&hits, before_rename + 1, Duration::from_secs(3)),
            "expected notification on file rename"
        );

        // 3. Delete file in root.
        let before_delete = hits.load(Ordering::Acquire);
        fs::remove_file(&file_b).unwrap();
        assert!(
            wait_for_count(&hits, before_delete + 1, Duration::from_secs(3)),
            "expected notification on file removal"
        );

        // 4. Create & remove a subdirectory in root.
        let before_dir = hits.load(Ordering::Acquire);
        let new_dir = root.join("firmware");
        fs::create_dir(&new_dir).unwrap();
        assert!(
            wait_for_count(&hits, before_dir + 1, Duration::from_secs(3)),
            "expected notification on directory creation"
        );
        let before_rmdir = hits.load(Ordering::Acquire);
        fs::remove_dir(&new_dir).unwrap();
        assert!(
            wait_for_count(&hits, before_rmdir + 1, Duration::from_secs(3)),
            "expected notification on directory removal"
        );

        // 5. File created inside collapsed subdirectory should NOT fire.
        thread::sleep(Duration::from_millis(100));
        let before_collapsed = hits.load(Ordering::Acquire);
        fs::write(sub.join("hidden_while_collapsed.c"), "int x;").unwrap();
        thread::sleep(Duration::from_millis(200));
        assert_eq!(
            hits.load(Ordering::Acquire),
            before_collapsed,
            "collapsed directory changes should be ignored"
        );

        // 6. Expand the subdirectory and create a new file inside it.
        watcher.sync_scope(Some(WatchScope {
            root: root.clone(),
            expanded_dirs: vec![sub.clone()],
            expand_all: false,
            show_hidden: false,
        }));
        thread::sleep(Duration::from_millis(50));
        let before_expanded = hits.load(Ordering::Acquire);
        fs::write(sub.join("visible_now.c"), "int y;").unwrap();
        assert!(
            wait_for_count(&hits, before_expanded + 1, Duration::from_secs(3)),
            "expected notification inside expanded subdirectory"
        );

        drop(watcher);
        let _ = fs::remove_dir_all(&root);
    }
}
