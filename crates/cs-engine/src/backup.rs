//! Persistent drafts of unsaved circuit and editor buffers.
//!
//! Unsaved content is written here on every logical edit so closing the app
//! does not need a save prompt: reopening a file overlays the draft. Drafts
//! are keyed by file path + project so modifications in different projects
//! are isolated.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::settings;

const FILES_DIR: &str = "files";
const CIRCUITS_DIR: &str = "circuits";
const CONTENT_FILE: &str = "content.txt";
const META_FILE: &str = "meta.json";
const SNAPSHOT_FILE: &str = "snapshot.circ1";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct DraftMeta {
    #[serde(default)]
    path: String,
    #[serde(default)]
    project: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    pid: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HistorySnap {
    pub sim1: String,
    #[serde(default)]
    pub selected_items: Vec<String>,
    #[serde(default)]
    pub selected_wires: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CircuitHistory {
    #[serde(default)]
    pub undo: Vec<HistorySnap>,
    #[serde(default)]
    pub redo: Vec<HistorySnap>,
}

#[derive(Clone, Debug)]
pub struct FileDraft {
    pub id: String,
    pub path: String,
    pub project: String,
    pub title: String,
    pub text: String,
    pub pid: u32,
}

#[derive(Clone, Debug)]
pub struct CircuitDraft {
    pub id: String,
    pub path: String,
    pub project: String,
    pub snapshot: String,
    pub pid: u32,
    pub mtime: SystemTime,
}

pub struct RecoveryStore {
    root: PathBuf,
}

impl RecoveryStore {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> PathBuf {
        let mut dir = settings::data_dir();
        dir.push("recovery");
        dir
    }

    fn files_dir(&self) -> PathBuf {
        self.root.join(FILES_DIR)
    }

    fn circuits_dir(&self) -> PathBuf {
        self.root.join(CIRCUITS_DIR)
    }

    fn file_dir(&self, id: &str) -> PathBuf {
        self.files_dir().join(id)
    }

    fn circuit_dir(&self, id: &str) -> PathBuf {
        self.circuits_dir().join(id)
    }

    pub fn put_file(
        &self,
        id: &str,
        path: &str,
        project: &str,
        title: &str,
        text: &str,
    ) -> io::Result<()> {
        let dir = self.file_dir(id);
        fs::create_dir_all(&dir)?;
        atomic_write(&dir.join(CONTENT_FILE), text.as_bytes())?;
        let meta = DraftMeta {
            path: path.to_string(),
            project: project.to_string(),
            title: title.to_string(),
            pid: std::process::id(),
        };
        write_meta(&dir.join(META_FILE), &meta)
    }

    pub fn get_file(&self, id: &str) -> io::Result<Option<FileDraft>> {
        let dir = self.file_dir(id);
        let meta_path = dir.join(META_FILE);
        if !meta_path.exists() {
            return Ok(None);
        }
        let meta = read_meta(&meta_path)?;
        let text = fs::read_to_string(dir.join(CONTENT_FILE)).unwrap_or_default();
        Ok(Some(FileDraft {
            id: id.to_string(),
            path: meta.path,
            project: meta.project,
            title: meta.title,
            text,
            pid: meta.pid,
        }))
    }

    pub fn get_file_for_path(&self, path: &str, project: &str) -> io::Result<Option<FileDraft>> {
        if path.is_empty() {
            return Ok(None);
        }
        let id = id_for_path(path, project);
        match self.get_file(&id)? {
            Some(d) if (d.path == path || d.path.is_empty()) && d.project == project => Ok(Some(d)),
            Some(_) | None => Ok(self
                .list_files()?
                .into_iter()
                .find(|d| d.path == path && d.project == project)),
        }
    }

    pub fn clear_file(&self, id: &str) -> io::Result<()> {
        let dir = self.file_dir(id);
        if dir.exists() {
            remove_dir_all_robust(&dir)?;
        }
        Ok(())
    }

    pub fn clear_file_for_path(&self, path: &str, project: &str) -> io::Result<()> {
        if let Some(d) = self.get_file_for_path(path, project)? {
            self.clear_file(&d.id)?;
        } else if !path.is_empty() {
            self.clear_file(&id_for_path(path, project))?;
        }
        Ok(())
    }

    pub fn list_files(&self) -> io::Result<Vec<FileDraft>> {
        let mut out = Vec::new();
        for id in list_ids(&self.files_dir())? {
            if let Some(d) = self.get_file(&id)? {
                out.push(d);
            }
        }
        Ok(out)
    }

    pub fn orphan_untitled_files(&self) -> io::Result<Vec<FileDraft>> {
        Ok(self
            .list_files()?
            .into_iter()
            .filter(|d| d.path.is_empty() && !pid_alive(d.pid))
            .collect())
    }

    pub fn put_circuit(&self, path: &str, project: &str, snapshot: &str) -> io::Result<()> {
        self.put_circuit_id(&circuit_id_for(path, project), path, project, snapshot)
    }

    pub fn put_circuit_id(
        &self,
        id: &str,
        path: &str,
        project: &str,
        snapshot: &str,
    ) -> io::Result<()> {
        let dir = self.circuit_dir(id);
        fs::create_dir_all(&dir)?;
        atomic_write(&dir.join(SNAPSHOT_FILE), snapshot.as_bytes())?;
        let title = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let meta = DraftMeta {
            path: path.to_string(),
            project: project.to_string(),
            title,
            pid: std::process::id(),
        };
        write_meta(&dir.join(META_FILE), &meta)
    }

    pub fn get_circuit(&self, id: &str) -> io::Result<Option<CircuitDraft>> {
        let dir = self.circuit_dir(id);
        let meta_path = dir.join(META_FILE);
        if !meta_path.exists() {
            return Ok(None);
        }
        let meta = read_meta(&meta_path)?;
        let mut snapshot_path = dir.join(SNAPSHOT_FILE);
        if !snapshot_path.exists() {
            let legacy_path = dir.join("snapshot.sim2");
            if legacy_path.exists() {
                snapshot_path = legacy_path;
            } else {
                return Ok(None);
            }
        }
        let snapshot = fs::read_to_string(&snapshot_path)?;
        let mtime = fs::metadata(&snapshot_path)
            .and_then(|m| m.modified())
            .unwrap_or(UNIX_EPOCH);
        Ok(Some(CircuitDraft {
            id: id.to_string(),
            path: meta.path,
            project: meta.project,
            snapshot,
            pid: meta.pid,
            mtime,
        }))
    }

    pub fn get_circuit_for(&self, path: &str, project: &str) -> io::Result<Option<CircuitDraft>> {
        let id = circuit_id_for(path, project);
        match self.get_circuit(&id)? {
            Some(d) if d.path == path && d.project == project => Ok(Some(d)),
            Some(_) | None => Ok(self
                .list_circuits()?
                .into_iter()
                .find(|d| d.path == path && d.project == project)),
        }
    }

    pub fn clear_circuit(&self, id: &str) -> io::Result<()> {
        let dir = self.circuit_dir(id);
        if dir.exists() {
            remove_dir_all_robust(&dir)?;
        }
        Ok(())
    }

    pub fn clear_circuit_for(&self, path: &str, project: &str) -> io::Result<()> {
        if let Some(d) = self.get_circuit_for(path, project)? {
            self.clear_circuit(&d.id)?;
        } else {
            self.clear_circuit(&circuit_id_for(path, project))?;
        }
        Ok(())
    }

    pub fn list_circuits(&self) -> io::Result<Vec<CircuitDraft>> {
        Ok(list_ids(&self.circuits_dir())?
            .into_iter()
            .filter_map(|id| self.get_circuit(&id).ok().flatten())
            .collect())
    }

    pub fn orphan_untitled_circuits(&self) -> io::Result<Vec<CircuitDraft>> {
        Ok(self
            .list_circuits()?
            .into_iter()
            .filter(|d| d.path.is_empty() && !pid_alive(d.pid))
            .collect())
    }

    pub fn end_session(&self) -> io::Result<()> {
        Ok(())
    }
}

fn list_ids(dir: &Path) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for e in fs::read_dir(dir)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            ids.push(e.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(ids)
}

fn remove_dir_all_robust(dir: &Path) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for _ in 0..5 {
        match fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) if e.raw_os_error() == Some(66) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(e),
        }
    }
    fs::remove_dir_all(dir)
}

fn write_meta(path: &Path, meta: &DraftMeta) -> io::Result<()> {
    let json =
        serde_json::to_vec_pretty(meta).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    atomic_write(path, &json)
}

fn read_meta(path: &Path) -> io::Result<DraftMeta> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, contents)?;
    #[cfg(windows)]
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Stable path and project identity: djb2 hex. Verified against `meta.path` and `meta.project`.
pub fn id_for_path(path: &str, project: &str) -> String {
    let key = if project.is_empty() {
        path.to_string()
    } else {
        format!("{project}::{path}")
    };
    format!("{:016x}", djb2(key.as_bytes()))
}

pub fn circuit_id_for(path: &str, project: &str) -> String {
    if path.is_empty() {
        if project.is_empty() {
            format!("untitled-circuit-{}", std::process::id())
        } else {
            format!(
                "untitled-circuit-{:016x}-{}",
                djb2(project.as_bytes()),
                std::process::id()
            )
        }
    } else {
        id_for_path(path, project)
    }
}

pub fn new_untitled_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("u-{nanos:x}-{:x}", std::process::id())
}

fn djb2(bytes: &[u8]) -> u64 {
    let mut h: u64 = 5381;
    for b in bytes {
        h = h.wrapping_mul(33).wrapping_add(*b as u64);
    }
    h
}

pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if pid == std::process::id() {
        return true;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| {
                let text = String::from_utf8_lossy(&o.stdout);
                text.split_whitespace().any(|tok| tok == pid.to_string())
            })
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

static STORE: OnceLock<Mutex<RecoveryStore>> = OnceLock::new();

fn store() -> MutexGuard<'static, RecoveryStore> {
    STORE
        .get_or_init(|| Mutex::new(RecoveryStore::at(RecoveryStore::default_root())))
        .lock()
        .expect("recovery store lock")
}

fn with_store<R>(f: impl FnOnce(&mut RecoveryStore) -> io::Result<R>) -> io::Result<R> {
    let mut g = store();
    f(&mut g)
}

fn log_err(op: &str, err: io::Error) {
    eprintln!("recovery {op}: {err}");
}

pub fn put_file(id: &str, path: &str, project: &str, title: &str, text: &str) {
    if let Err(e) = with_store(|s| s.put_file(id, path, project, title, text)) {
        log_err("put_file", e);
    }
}

pub fn get_file(id: &str) -> Option<FileDraft> {
    match with_store(|s| s.get_file(id)) {
        Ok(v) => v,
        Err(e) => {
            log_err("get_file", e);
            None
        }
    }
}

pub fn get_file_for_path(path: &str, project: &str) -> Option<FileDraft> {
    match with_store(|s| s.get_file_for_path(path, project)) {
        Ok(v) => v,
        Err(e) => {
            log_err("get_file_for_path", e);
            None
        }
    }
}

pub fn clear_file(id: &str) {
    if let Err(e) = with_store(|s| s.clear_file(id)) {
        log_err("clear_file", e);
    }
}

pub fn clear_file_for_path(path: &str, project: &str) {
    if let Err(e) = with_store(|s| s.clear_file_for_path(path, project)) {
        log_err("clear_file_for_path", e);
    }
}

pub fn orphan_untitled_files() -> Vec<FileDraft> {
    match with_store(|s| s.orphan_untitled_files()) {
        Ok(v) => v,
        Err(e) => {
            log_err("orphan_untitled_files", e);
            Vec::new()
        }
    }
}

pub fn put_circuit(path: &str, project: &str, snapshot: &str) {
    if let Err(e) = with_store(|s| s.put_circuit(path, project, snapshot)) {
        log_err("put_circuit", e);
    }
}

pub fn put_circuit_id(id: &str, path: &str, project: &str, snapshot: &str) {
    if let Err(e) = with_store(|s| s.put_circuit_id(id, path, project, snapshot)) {
        log_err("put_circuit_id", e);
    }
}

pub fn get_circuit_for(path: &str, project: &str) -> Option<CircuitDraft> {
    match with_store(|s| s.get_circuit_for(path, project)) {
        Ok(v) => v,
        Err(e) => {
            log_err("get_circuit_for", e);
            None
        }
    }
}

pub fn get_circuit(id: &str) -> Option<CircuitDraft> {
    match with_store(|s| s.get_circuit(id)) {
        Ok(v) => v,
        Err(e) => {
            log_err("get_circuit", e);
            None
        }
    }
}

pub fn clear_circuit(id: &str) {
    if let Err(e) = with_store(|s| s.clear_circuit(id)) {
        log_err("clear_circuit", e);
    }
}

pub fn clear_circuit_for(path: &str, project: &str) {
    if let Err(e) = with_store(|s| s.clear_circuit_for(path, project)) {
        log_err("clear_circuit_for", e);
    }
}

pub fn list_circuits() -> Vec<CircuitDraft> {
    match with_store(|s| s.list_circuits()) {
        Ok(v) => v,
        Err(e) => {
            log_err("list_circuits", e);
            Vec::new()
        }
    }
}

pub fn orphan_untitled_circuits() -> Vec<CircuitDraft> {
    match with_store(|s| s.orphan_untitled_circuits()) {
        Ok(v) => v,
        Err(e) => {
            log_err("orphan_untitled_circuits", e);
            Vec::new()
        }
    }
}

pub fn end_session() {
    if let Err(e) = with_store(|s| s.end_session()) {
        log_err("end_session", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> (PathBuf, RecoveryStore) {
        let cnt = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cs-backup-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            cnt
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        (dir.clone(), RecoveryStore::at(dir))
    }

    fn cleanup(dir: &Path) {
        let _ = remove_dir_all_robust(dir);
    }

    #[test]
    fn id_for_path_is_stable_and_isolated_by_project() {
        let a1 = id_for_path("/Users/me/file.c", "/project_a");
        let a2 = id_for_path("/Users/me/file.c", "/project_a");
        let b = id_for_path("/Users/me/file.c", "/project_b");
        let no_proj = id_for_path("/Users/me/file.c", "");

        assert_eq!(a1, a2);
        assert_ne!(a1, b);
        assert_ne!(a1, no_proj);
        assert_eq!(a1.len(), 16);
    }

    #[test]
    fn file_draft_isolated_by_project_and_cleared() {
        let (dir, store) = temp_store();
        let path = "/tmp/shared.c";

        let id_a = id_for_path(path, "/project_a");
        let id_b = id_for_path(path, "/project_b");

        store
            .put_file(&id_a, path, "/project_a", "shared.c", "draft in A")
            .unwrap();
        store
            .put_file(&id_b, path, "/project_b", "shared.c", "draft in B")
            .unwrap();

        let got_a = store
            .get_file_for_path(path, "/project_a")
            .unwrap()
            .unwrap();
        let got_b = store
            .get_file_for_path(path, "/project_b")
            .unwrap()
            .unwrap();

        assert_eq!(got_a.text, "draft in A");
        assert_eq!(got_a.project, "/project_a");
        assert_eq!(got_b.text, "draft in B");
        assert_eq!(got_b.project, "/project_b");

        // Clearing for project_a leaves project_b intact
        store.clear_file_for_path(path, "/project_a").unwrap();
        assert!(
            store
                .get_file_for_path(path, "/project_a")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            store
                .get_file_for_path(path, "/project_b")
                .unwrap()
                .unwrap()
                .text,
            "draft in B"
        );

        cleanup(&dir);
    }

    #[test]
    fn circuit_draft_isolated_by_project_and_no_undo_restore() {
        let (dir, store) = temp_store();
        let path = "/tmp/circuit.sim2";

        store.put_circuit(path, "/proj_x", "<circuit x/>").unwrap();
        store.put_circuit(path, "/proj_y", "<circuit y/>").unwrap();

        let got_x = store.get_circuit_for(path, "/proj_x").unwrap().unwrap();
        let got_y = store.get_circuit_for(path, "/proj_y").unwrap().unwrap();

        assert_eq!(got_x.snapshot, "<circuit x/>");
        assert_eq!(got_x.project, "/proj_x");
        assert_eq!(got_y.snapshot, "<circuit y/>");
        assert_eq!(got_y.project, "/proj_y");

        store.end_session().unwrap();

        // Still holds snapshot after end_session
        let got_x_again = store.get_circuit_for(path, "/proj_x").unwrap().unwrap();
        assert_eq!(got_x_again.snapshot, "<circuit x/>");

        cleanup(&dir);
    }

    #[test]
    fn untitled_file_is_orphan_after_foreign_pid() {
        let (dir, store) = temp_store();
        let id = "u-dead-pid";
        store.put_file(id, "", "", "untitled", "hello").unwrap();
        let dead_pid: u32 = 2_000_000_000;
        let meta = DraftMeta {
            path: String::new(),
            project: String::new(),
            title: "untitled".into(),
            pid: dead_pid,
        };
        write_meta(&store.file_dir(id).join(META_FILE), &meta).unwrap();
        assert!(!pid_alive(dead_pid), "synthetic pid must be dead");
        let orphans = store.orphan_untitled_files().unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].text, "hello");
        cleanup(&dir);
    }

    #[test]
    fn circuit_untitled_id_is_per_process() {
        let id = circuit_id_for("", "");
        assert!(id.contains(&std::process::id().to_string()));
        assert_eq!(circuit_id_for("/x.sim2", ""), id_for_path("/x.sim2", ""));
    }
}
