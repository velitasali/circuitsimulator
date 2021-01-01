//! File I/O, SIM1/SIM2 loading and saving, backups, drafts, and path handling.

use crate::path_util::strip_file_url;
use cs_engine::backup;
use cs_engine::canvas::{Canvas, Change};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum PendingReplace {
    #[default]
    None,
    New,
    Load(String),
}

/// Migration to native .circ1 format:
/// - If path ends with .sim1 or .sim2, replace with .circ1
/// - Otherwise if it does not end with .circ1, append .circ1
pub fn ensure_circ1_path(path: &str) -> String {
    let p = path.trim();
    if p.is_empty() {
        return String::new();
    }
    let mut out = p.to_string();
    if out.ends_with(".sim1") {
        out.replace_range(out.len() - 5.., ".circ1");
    } else if out.ends_with(".sim2") {
        out.replace_range(out.len() - 5.., ".circ1");
    } else if !out.ends_with(".circ1") {
        out.push_str(".circ1");
    }
    out
}

pub(super) fn suggest_image_url(p: &str) -> String {
    if std::path::Path::new(p).is_absolute() {
        let p_clean = p.replace('\\', "/");
        if p_clean.starts_with('/') {
            format!("file://{p_clean}")
        } else {
            format!("file:///{p_clean}")
        }
    } else {
        String::new()
    }
}

pub(super) fn replace_with_new(canvas: &mut Canvas, project_path: &str) -> Change {
    let path = canvas
        .file_path()
        .map(|s| s.to_string())
        .unwrap_or_default();
    backup::clear_circuit_for(&path, project_path);
    cs_engine::project::update_session(project_path, |s| {
        s.circuit_file = None;
    });
    cs_engine::logging::log_sim(cs_engine::i18n::tr("New circuit created"));
    let defaults = cs_engine::settings::get().to_circ();
    canvas.new_circuit_with(defaults)
}

pub(super) fn load_path_now(
    canvas: &mut Canvas,
    project_path: &str,
    path: &str,
) -> Result<Change, String> {
    match std::fs::read_to_string(path) {
        Ok(src) => {
            let draft = backup::get_circuit_for(path, project_path)
                .filter(|d| d.path == path && d.snapshot != src);
            let loaded = if let Some(ref draft) = draft {
                canvas.load_recovered(&draft.snapshot, Some(path.to_string()), &src)
            } else {
                canvas.load_sim1(&src, Some(path.to_string()))
            };
            match loaded {
                Ok(c) => {
                    let mut st = cs_engine::settings::get().clone();
                    st.push_recent_circuit(path.to_string());
                    st.save();
                    cs_engine::project::update_session(project_path, |s| {
                        s.circuit_file = Some(path.to_string());
                    });
                    let loaded_prefix = cs_engine::i18n::tr("Loaded circuit:");
                    cs_engine::logging::log_sim(format!("{loaded_prefix} {path}"));
                    Ok(c)
                }
                Err(e) => {
                    canvas.power_off();
                    let err_prefix = cs_engine::i18n::tr("Error loading circuit:");
                    cs_engine::logging::log_sim(format!("{err_prefix} {e}"));
                    Err(e.to_string())
                }
            }
        }
        Err(e) => {
            let err_prefix = cs_engine::i18n::tr("Error reading circuit file:");
            cs_engine::logging::log_sim(format!("{err_prefix} {e}"));
            Err(e.to_string())
        }
    }
}

pub(super) fn apply_circuit_draft(
    canvas: &mut Canvas,
    project_path: &str,
    draft: backup::CircuitDraft,
) -> Result<Option<Change>, String> {
    let disk = if draft.path.is_empty() {
        String::new()
    } else {
        std::fs::read_to_string(&draft.path).unwrap_or_default()
    };
    if draft.snapshot == disk {
        backup::clear_circuit(&draft.id);
        return Ok(None);
    }
    let path = if draft.path.is_empty() {
        None
    } else {
        Some(draft.path.clone())
    };
    match canvas.load_recovered(&draft.snapshot, path, &disk) {
        Ok(c) => {
            if draft.path.is_empty() && draft.id != backup::circuit_id_for("", project_path) {
                backup::put_circuit("", project_path, &draft.snapshot);
                backup::clear_circuit(&draft.id);
            }
            cs_engine::logging::log_sim(cs_engine::i18n::tr("Restored unsaved circuit changes"));
            Ok(Some(c))
        }
        Err(e) => {
            let err_prefix = cs_engine::i18n::tr("Error restoring circuit draft:");
            cs_engine::logging::log_sim(format!("{err_prefix} {e}"));
            Err(e.to_string())
        }
    }
}

pub(super) fn persist_circuit_draft_now(canvas: &Canvas, project_path: &str) {
    let path = canvas
        .file_path()
        .map(|s| s.to_string())
        .unwrap_or_default();
    if !canvas.is_modified() {
        backup::clear_circuit_for(&path, project_path);
        return;
    }
    let snapshot = canvas.to_sim1();
    backup::put_circuit(&path, project_path, &snapshot);
}

pub(super) fn save_path(
    canvas: &mut Canvas,
    project_path: &str,
    raw_path: &str,
) -> Result<String, String> {
    let path = ensure_circ1_path(raw_path);
    if path.is_empty() {
        return Ok(String::new());
    }
    let old_path = canvas
        .file_path()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let src = canvas.to_sim1();
    if let Err(e) = std::fs::write(&path, src) {
        let err_prefix = cs_engine::i18n::tr("Error saving circuit:");
        cs_engine::logging::log_sim(format!("{err_prefix} {e}"));
        return Err(e.to_string());
    }
    let saved_prefix = cs_engine::i18n::tr("Saved circuit:");
    cs_engine::logging::log_sim(format!("{saved_prefix} {path}"));
    cs_engine::settings::edit(|st| {
        st.push_recent_circuit(path.clone());
    });
    backup::clear_circuit_for(&old_path, project_path);
    if old_path != path {
        backup::clear_circuit_for(&path, project_path);
    }
    if !path.is_empty() {
        cs_engine::project::update_session(project_path, |s| {
            s.circuit_file = Some(path.clone());
        });
    }
    canvas.set_file_path(Some(path.clone()));
    canvas.mark_saved();
    Ok(path)
}

pub(super) fn restore_draft_draft(
    canvas: &Canvas,
    project_path: &str,
) -> Option<backup::CircuitDraft> {
    if crate::pending::no_project() {
        return None;
    }
    let cli = crate::pending::OPEN_CIRCUIT.get().cloned();
    let current_path = canvas
        .file_path()
        .map(|s| s.to_string())
        .unwrap_or_default();

    if let Some(cli) = cli {
        let path = strip_file_url(&cli);
        backup::get_circuit_for(&path, project_path)
    } else if !current_path.is_empty() {
        backup::get_circuit_for(&current_path, project_path)
    } else {
        backup::orphan_untitled_circuits()
            .into_iter()
            .max_by(|a, b| {
                a.mtime
                    .partial_cmp(&b.mtime)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}
