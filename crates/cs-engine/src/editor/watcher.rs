//! Disk-change file watcher. C++ `FileWatcher`: stamp, settle, acknowledge.
//! Uses polling (stat + hash) instead of inotify/FSEvents since cs-engine is Qt-free.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::time::{Instant, UNIX_EPOCH};

const SETTLE_MS: u128 = 300;
const MISSING_RETRIES: i32 = 4;
const MAX_HASH_SIZE: u64 = 8 * 1024 * 1024;

// Identity of a file at a point in time. Size and mtime alone produce false
// positives (a git checkout can rewrite a file byte for byte), so the contents
// are hashed too whenever the file is small enough to make that cheap.
#[derive(Clone, Debug)]
struct FileStamp {
    exists: bool,
    size: u64,
    mtime_ms: i64,
    hash: Option<u64>, // DefaultHasher, not cryptographic — good enough for change detection
}

impl FileStamp {
    fn of(path: &str) -> Self {
        let meta = match std::fs::metadata(path) {
            Ok(m) if m.is_file() => m,
            _ => {
                return Self {
                    exists: false,
                    size: 0,
                    mtime_ms: -1,
                    hash: None,
                };
            }
        };

        let size = meta.len();
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(-1);

        let hash = if size <= MAX_HASH_SIZE {
            std::fs::read(path).ok().map(|bytes| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut h);
                h.finish()
            })
        } else {
            None
        };

        Self {
            exists: true,
            size,
            mtime_ms,
            hash,
        }
    }

    fn matches(&self, other: &Self) -> bool {
        if self.exists != other.exists || self.size != other.size {
            return false;
        }
        // If either hash is missing (file too big or unreadable), fall back to
        // size + mtime — same logic as the C++ operator==.
        match (self.hash, other.hash) {
            (Some(a), Some(b)) => a == b,
            _ => self.mtime_ms == other.mtime_ms,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchEvent {
    Changed(String),
    Deleted(String),
}

pub struct FileWatcher {
    synced: HashMap<String, FileStamp>, // contents we are in sync with
    acked: HashMap<String, FileStamp>,  // contents the user chose not to take
    pending: HashSet<String>,           // paths to check on next poll
    missing_rounds: HashMap<String, i32>, // retry counter for missing files
    last_poll: Instant,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            synced: HashMap::new(),
            acked: HashMap::new(),
            pending: HashSet::new(),
            missing_rounds: HashMap::new(),
            last_poll: Instant::now(),
        }
    }

    // Start watching, adopting the current contents as in sync.
    pub fn watch(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if self.synced.contains_key(path) {
            // Already tracked: this is just a re-stamp.
            self.sync(path);
            return;
        }

        self.synced.insert(path.to_string(), FileStamp::of(path));
        self.acked.remove(path);
        self.missing_rounds.remove(path);
    }

    // Stop watching, forget everything about the path.
    pub fn unwatch(&mut self, path: &str) {
        if path.is_empty() || !self.synced.contains_key(path) {
            return;
        }
        self.synced.remove(path);
        self.acked.remove(path);
        self.missing_rounds.remove(path);
        self.pending.remove(path);
    }

    // We just read or wrote it: the file on disk is now ours.
    pub fn sync(&mut self, path: &str) {
        if path.is_empty() || !self.synced.contains_key(path) {
            return;
        }
        self.synced.insert(path.to_string(), FileStamp::of(path));
        self.acked.remove(path);
        self.missing_rounds.remove(path);
        self.pending.remove(path);
    }

    // The user was asked about the current on-disk contents and chose to keep
    // their own version. Stays out of sync, but does not ask again until the
    // file changes once more.
    pub fn acknowledge(&mut self, path: &str) {
        if path.is_empty() || !self.synced.contains_key(path) {
            return;
        }
        self.acked.insert(path.to_string(), FileStamp::of(path));
        self.pending.remove(path);
    }

    // Disk no longer holds what we last read or wrote.
    pub fn is_out_of_sync(&self, path: &str) -> bool {
        match self.synced.get(path) {
            Some(synced) => !FileStamp::of(path).matches(synced),
            None => false,
        }
    }

    // Check all synced paths for changes. Returns events for files that have
    // changed or been deleted since last sync/acknowledge.
    pub fn poll(&mut self) -> Vec<WatchEvent> {
        let elapsed = self.last_poll.elapsed().as_millis();
        if elapsed < SETTLE_MS {
            return Vec::new();
        }
        self.last_poll = Instant::now();

        let mut events = Vec::new();

        // In polling mode every synced path is a candidate — no OS notifications
        // to narrow the set.
        let paths: Vec<String> = self.synced.keys().cloned().collect();

        for path in paths {
            let stamp = FileStamp::of(&path);

            if !stamp.exists {
                // An atomic save deletes and recreates the file, so a missing
                // file is only really gone once it has stayed missing for a while.
                let rounds = self.missing_rounds.entry(path.clone()).or_insert(0);
                *rounds += 1;
                if *rounds <= MISSING_RETRIES {
                    continue;
                }
                self.missing_rounds.remove(&path);
                if self.synced.get(&path).map_or(false, |s| s.exists) {
                    self.synced.insert(path.clone(), stamp);
                    events.push(WatchEvent::Deleted(path));
                }
                continue;
            }
            self.missing_rounds.remove(&path);

            // Our own contents, or a touch that changed nothing.
            if self.synced.get(&path).map_or(false, |s| stamp.matches(s)) {
                continue;
            }
            // Already offered, user kept their version.
            if self.acked.get(&path).map_or(false, |a| stamp.matches(a)) {
                continue;
            }

            events.push(WatchEvent::Changed(path));
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("cs_watcher_test");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn write(path: &std::path::Path, data: &[u8]) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(data).unwrap();
        f.sync_all().unwrap();
    }

    #[test]
    fn no_events_on_fresh_watch() {
        let p = tmp_file("fresh.txt");
        write(&p, b"hello");
        let ps = p.to_str().unwrap();

        let mut w = FileWatcher::new();
        w.watch(ps);
        // Force last_poll into the past so poll() actually runs.
        w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
        let ev = w.poll();
        assert!(ev.is_empty(), "expected no events, got {:?}", ev);

        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn changed_after_modify() {
        let p = tmp_file("modify.txt");
        write(&p, b"v1");
        let ps = p.to_str().unwrap();

        let mut w = FileWatcher::new();
        w.watch(ps);

        // Modify
        write(&p, b"v2 -- different content");
        w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
        let ev = w.poll();
        assert!(
            ev.iter()
                .any(|e| matches!(e, WatchEvent::Changed(s) if s == ps)),
            "expected Changed, got {:?}",
            ev,
        );

        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn deleted_after_retries() {
        let p = tmp_file("delete.txt");
        write(&p, b"bye");
        let ps = p.to_str().unwrap();

        let mut w = FileWatcher::new();
        w.watch(ps);
        std::fs::remove_file(&p).unwrap();

        // Need MISSING_RETRIES + 1 polls to confirm deletion.
        let mut got_deleted = false;
        for _ in 0..=MISSING_RETRIES + 1 {
            w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
            let ev = w.poll();
            if ev
                .iter()
                .any(|e| matches!(e, WatchEvent::Deleted(s) if s == ps))
            {
                got_deleted = true;
                break;
            }
        }
        assert!(got_deleted, "expected Deleted after retries");
    }

    #[test]
    fn sync_suppresses_own_save() {
        let p = tmp_file("sync.txt");
        write(&p, b"original");
        let ps = p.to_str().unwrap();

        let mut w = FileWatcher::new();
        w.watch(ps);

        // "We" save new content and call sync.
        write(&p, b"our new save");
        w.sync(ps);

        w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
        let ev = w.poll();
        assert!(
            ev.is_empty(),
            "sync should suppress our own save, got {:?}",
            ev
        );

        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn acknowledge_suppresses_same_stamp() {
        let p = tmp_file("ack.txt");
        write(&p, b"start");
        let ps = p.to_str().unwrap();

        let mut w = FileWatcher::new();
        w.watch(ps);

        // External edit.
        write(&p, b"external edit");
        w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
        let ev = w.poll();
        assert!(
            ev.iter().any(|e| matches!(e, WatchEvent::Changed(_))),
            "expected initial Changed",
        );

        // User acknowledges.
        w.acknowledge(ps);

        // Same contents on disk → should not fire again.
        w.last_poll = Instant::now() - std::time::Duration::from_millis(SETTLE_MS as u64 + 10);
        let ev = w.poll();
        assert!(
            ev.is_empty(),
            "acknowledge should suppress repeated alert, got {:?}",
            ev,
        );

        std::fs::remove_file(&p).ok();
    }
}
