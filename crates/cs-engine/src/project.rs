//! Project session management.
//!
//! Stores open circuit and text editor files per project so they are remembered
//! upon reopening unless specifically closed.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::settings;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSession {
    #[serde(default)]
    pub circuit_file: Option<String>,
    #[serde(default)]
    pub editor_files: Vec<String>,
    #[serde(default)]
    pub active_editor_file: Option<String>,
}

pub fn normalize_path(path: &str) -> String {
    let trimmed = path.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return String::new();
    }
    let p = trimmed.trim_end_matches('/');
    if p.is_empty() {
        return "/".to_string();
    }
    p.to_string()
}

pub fn get_session(project_dir: &str) -> Option<ProjectSession> {
    let key = normalize_path(project_dir);
    if let Some(sess) = settings::get().project_sessions.get(&key).cloned() {
        return Some(sess);
    }
    discover_session(&key)
}

fn discover_session(key: &str) -> Option<ProjectSession> {
    // 1. Try reading circuitsimulator.ini from candidate locations
    let mut ini_paths = vec![settings::data_dir().join("circuitsimulator.ini")];
    if let Some(d) = dirs::data_dir() {
        ini_paths.push(d.join("circuitsimulator").join("circuitsimulator.ini"));
        ini_paths.push(d.join("Circuit Simulator").join("circuitsimulator.ini"));
    }
    if let Some(c) = dirs::config_dir() {
        ini_paths.push(c.join("circuitsimulator").join("circuitsimulator.ini"));
        ini_paths.push(c.join("Circuit Simulator").join("circuitsimulator.ini"));
    }

    for ini_path in ini_paths {
        if !ini_path.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&ini_path) {
            if let Some(sess) = parse_ini_session(&content, key) {
                save_session(key, &sess);
                return Some(sess);
            }
        }
    }

    // 2. Fallback to settings.recent_circuits and settings.recent_files
    let s = settings::get();
    let mut circ_opt: Option<String> = None;
    let mut editor_files: Vec<String> = Vec::new();

    if !key.is_empty() {
        let prefix = format!("{key}/");
        for c in &s.recent_circuits {
            if (c.starts_with(&prefix) || c == key) && Path::new(c).exists() {
                circ_opt = Some(c.clone());
                break;
            }
        }
        for f in &s.recent_files {
            if (f.starts_with(&prefix) || f == key)
                && Path::new(f).exists()
                && !editor_files.contains(f)
            {
                editor_files.push(f.clone());
            }
        }
    } else {
        if let Some(c) = s.recent_circuits.first().filter(|p| Path::new(p).exists()) {
            circ_opt = Some(c.clone());
        }
        if let Some(f) = s.recent_files.first().filter(|p| Path::new(p).exists()) {
            editor_files.push(f.clone());
        }
    }

    if circ_opt.is_some() || !editor_files.is_empty() {
        let active_opt = editor_files.first().cloned();
        let sess = ProjectSession {
            circuit_file: circ_opt,
            editor_files,
            active_editor_file: active_opt,
        };
        save_session(key, &sess);
        Some(sess)
    } else {
        None
    }
}

pub fn parse_ini_session(content: &str, key: &str) -> Option<ProjectSession> {
    let hex_key_plain: String = key
        .trim_end_matches('/')
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect();
    let hex_key_slash: String = format!("{}/", key.trim_end_matches('/'))
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect();

    let mut circ_opt: Option<String> = None;
    let mut editor_files: Vec<String> = Vec::new();
    let mut active_opt: Option<String> = None;

    for hk in [&hex_key_plain, &hex_key_slash] {
        if hk.is_empty() {
            continue;
        }
        let prefix_circ = format!("{hk}\\circuitFile=");
        let prefix_open = format!("{hk}\\openFiles=");
        let prefix_active = format!("{hk}\\activeFile=");
        let prefix_editors = format!("{hk}\\Editors\\");

        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix(&prefix_circ) {
                let trimmed = val.trim();
                if !trimmed.is_empty() && Path::new(trimmed).exists() {
                    circ_opt = Some(trimmed.to_string());
                }
            } else if let Some(val) = line.strip_prefix(&prefix_open) {
                let trimmed = val.trim();
                if !trimmed.is_empty() && !trimmed.contains("@Invalid") {
                    for part in trimmed.split(',') {
                        let f = part.trim();
                        if !f.is_empty()
                            && Path::new(f).exists()
                            && !editor_files.contains(&f.to_string())
                        {
                            editor_files.push(f.to_string());
                        }
                    }
                }
            } else if let Some(val) = line.strip_prefix(&prefix_active) {
                let trimmed = val.trim();
                if !trimmed.is_empty() && Path::new(trimmed).exists() {
                    active_opt = Some(trimmed.to_string());
                }
            } else if let Some(rest) = line.strip_prefix(&prefix_editors) {
                if let Some((_, val)) = rest.split_once('=') {
                    let f = val.trim();
                    if !f.is_empty()
                        && Path::new(f).exists()
                        && !editor_files.contains(&f.to_string())
                    {
                        editor_files.push(f.to_string());
                    }
                }
            }
        }
    }

    if circ_opt.is_some() || !editor_files.is_empty() {
        Some(ProjectSession {
            circuit_file: circ_opt,
            editor_files,
            active_editor_file: active_opt,
        })
    } else {
        None
    }
}

pub fn save_session(project_dir: &str, session: &ProjectSession) {
    let key = normalize_path(project_dir);
    let sess = session.clone();
    settings::edit(|s| {
        s.project_sessions.insert(key, sess);
    });
}

pub fn update_session(project_dir: &str, f: impl FnOnce(&mut ProjectSession)) {
    let key = normalize_path(project_dir);
    let mut sess = get_session(&key).unwrap_or_default();
    let old_sess = sess.clone();
    f(&mut sess);
    if sess != old_sess {
        save_session(&key, &sess);
    }
}

pub fn remove_session(project_dir: &str) {
    let key = normalize_path(project_dir);
    settings::edit(|s| {
        s.project_sessions.remove(&key);
    });
}

/// Filter remembered files in a session to only those that currently exist on disk.
pub fn cleanup_missing_files(session: &mut ProjectSession) {
    if let Some(ref circ) = session.circuit_file {
        if !Path::new(circ).exists() {
            session.circuit_file = None;
        }
    }
    session.editor_files.retain(|p| Path::new(p).exists());
    if let Some(ref act) = session.active_editor_file {
        if !session.editor_files.contains(act) {
            session.active_editor_file = session.editor_files.last().cloned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/tmp/foo/"), "/tmp/foo");
        assert_eq!(normalize_path("/tmp/foo"), "/tmp/foo");
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("   "), "");
    }

    #[test]
    fn test_session_lifecycle() {
        let proj = "/tmp/test_project_session";
        remove_session(proj);

        let mut sess = ProjectSession::default();
        sess.circuit_file = Some("/tmp/test_project_session/main.sim1".into());
        sess.editor_files = vec![
            "/tmp/test_project_session/a.c".into(),
            "/tmp/test_project_session/b.c".into(),
        ];
        sess.active_editor_file = Some("/tmp/test_project_session/a.c".into());

        save_session(proj, &sess);

        let loaded = get_session(proj).expect("session should exist");
        assert_eq!(loaded, sess);

        // Update session by removing a closed editor file
        update_session(proj, |s| {
            s.editor_files
                .retain(|p| p != "/tmp/test_project_session/b.c");
        });

        let updated = get_session(proj).expect("session should exist");
        assert_eq!(updated.editor_files, vec!["/tmp/test_project_session/a.c"]);
        assert_eq!(
            updated.circuit_file,
            Some("/tmp/test_project_session/main.sim1".into())
        );

        // Close circuit file
        update_session(proj, |s| {
            s.circuit_file = None;
        });
        let no_circ = get_session(proj).expect("session should exist");
        assert!(no_circ.circuit_file.is_none());

        remove_session(proj);
        assert!(get_session(proj).is_none());
    }

    #[test]
    fn test_standalone_session() {
        let proj = "";
        remove_session(proj);

        let mut sess = ProjectSession::default();
        sess.circuit_file = Some("/tmp/standalone.sim2".into());
        sess.editor_files = vec!["/tmp/code.c".into()];
        sess.active_editor_file = Some("/tmp/code.c".into());

        save_session(proj, &sess);

        let loaded = get_session(proj).expect("standalone session should exist");
        assert_eq!(loaded, sess);

        update_session(proj, |s| {
            s.circuit_file = None;
        });
        let updated = get_session(proj).expect("standalone session should exist");
        assert!(updated.circuit_file.is_none());
        assert_eq!(updated.editor_files, vec!["/tmp/code.c"]);

        remove_session(proj);
    }

    #[test]
    fn test_parse_ini_session() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cargo_toml = format!("{manifest_dir}/Cargo.toml");
        let lib_rs = format!("{manifest_dir}/src/lib.rs");

        let hex_proj: String = manifest_dir.bytes().map(|b| format!("{b:02x}")).collect();
        let ini_content = format!(
            "[ProjectContexts]\n{hex_proj}\\circuitFile={cargo_toml}\n{hex_proj}\\openFiles={lib_rs}\n{hex_proj}\\activeFile={lib_rs}\n"
        );

        let parsed = parse_ini_session(&ini_content, manifest_dir).expect("should parse session");
        assert_eq!(parsed.circuit_file, Some(cargo_toml));
        assert_eq!(parsed.editor_files, vec![lib_rs.clone()]);
        assert_eq!(parsed.active_editor_file, Some(lib_rs));
    }
}
