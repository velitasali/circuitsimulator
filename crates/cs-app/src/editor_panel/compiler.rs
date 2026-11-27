//! Compilation jobs, compiler spec resolution, and Arduino board discovery for EditorPanel.

use super::document::Document;
use super::{EditorPanel, normalize_debug_path};
use cs_engine::backup;
use cs_engine::debug::{DebugSession, DebugSymbols, ElfParser, LstKind, LstParser};
use cs_engine::editor::{
    self, CompileCtx, CompileEvent, CompileJob, CompileResult, CompilerSpec, arduino,
    list_arduino_boards, load_cfg, match_board, resolve_board_fqbn, save_cfg,
};
use cs_engine::i18n;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedCompile {
    pub src_path: String,
    pub target_device_ids: Vec<String>,
    pub debug: bool,
    pub load_debug_symbols: bool,
    pub silent_if_cached: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingSimAction {
    RestartPowerOn,
    DebugRun,
}

pub struct PendingCompile {
    pub job: CompileJob,
    pub src_path: String,
    pub debug: bool,
    pub then_upload: bool,
    pub then_run: bool,
    pub breakpoints: Vec<i32>,
    pub target_device_ids: Vec<String>,
    pub load_debug_symbols: bool,
}

pub(super) fn collect_device_compile_targets(devices: Value, debug: bool) -> Vec<QueuedCompile> {
    let mut groups: Vec<(String, String, Vec<String>, bool)> = Vec::new();
    if let Value::Array(arr) = devices {
        for item in arr {
            let src = item
                .get("firmwareSource")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let uid = item
                .get("uid")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("id").and_then(|v| v.as_str()))
                .unwrap_or("");
            if src.is_empty() || uid.is_empty() {
                continue;
            }
            let norm = normalize_debug_path(Path::new(src));
            if norm.is_empty() {
                continue;
            }
            let is_active = item
                .get("isActive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if let Some(existing) = groups.iter_mut().find(|(k, _, _, _)| *k == norm) {
                if !existing.2.iter().any(|u| u == uid) {
                    existing.2.push(uid.to_string());
                }
                existing.3 |= is_active;
            } else {
                groups.push((norm, src.to_string(), vec![uid.to_string()], is_active));
            }
        }
    }

    if groups.is_empty() {
        return Vec::new();
    }

    // Put non-active MCU firmwares first and the active MCU's firmware last so
    // `last_firmware` and debug symbols settle on the active MCU.
    let mut non_active = Vec::new();
    let mut active = Vec::new();
    for (_, src_path, target_device_ids, is_active) in groups {
        if is_active {
            active.push((src_path, target_device_ids, true));
        } else {
            non_active.push((src_path, target_device_ids, false));
        }
    }
    let has_active = !active.is_empty();
    let ordered: Vec<(String, Vec<String>, bool)> = non_active.into_iter().chain(active).collect();
    let last_idx = ordered.len().saturating_sub(1);

    ordered
        .into_iter()
        .enumerate()
        .map(|(i, (src_path, target_device_ids, is_active))| {
            let is_primary = if has_active { is_active } else { i == last_idx };
            QueuedCompile {
                src_path,
                target_device_ids,
                debug: debug && is_primary,
                load_debug_symbols: is_primary,
                silent_if_cached: true,
            }
        })
        .collect()
}

impl EditorPanel {
    pub fn refresh_arduino_boards(&mut self) {
        let tp = if let Some(d) = self.current_doc() {
            if d.compiler.eq_ignore_ascii_case("arduino") {
                d.tool_path.clone()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let tp = if !tp.is_empty() {
            tp
        } else {
            let s = cs_engine::settings::get();
            s.compiler_tool_paths
                .get("Arduino")
                .cloned()
                .filter(|p| !p.is_empty())
                .or_else(|| arduino::tool_path_candidates().into_iter().next())
                .unwrap_or_default()
        };
        if self.arduino_list_path.as_deref() == Some(tp.as_str())
            || self.board_loading_path.as_deref() == Some(tp.as_str())
        {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.board_rx = Some(rx);
        self.board_loading_path = Some(tp.clone());
        self.compiling_changed();
        thread::spawn(move || {
            let boards = list_arduino_boards(&tp);
            let _ = tx.send((tp, boards));
        });
    }

    pub fn reapply_arduino_board(&mut self) {
        for d in &mut self.docs {
            if !d.compiler.eq_ignore_ascii_case("arduino") {
                continue;
            }
            let qb = if !d.raw_board.is_empty() {
                d.raw_board.clone()
            } else {
                d.board.clone()
            };
            let qc = if !d.raw_board.is_empty() {
                d.raw_custom_board.clone()
            } else {
                d.custom_board.clone()
            };
            let (b, cb, dev) = match_board(&qb, &qc, &d.device, &self.arduino_boards);
            d.board = b;
            d.custom_board = cb;
            if !dev.is_empty() {
                d.device = dev;
            }
        }
    }

    /// Resolve an Arduino board selection to a valid FQBN (`packager:arch:board[:opts]`).
    ///
    /// When `self.arduino_boards` has not yet been populated from `arduino-cli board listall`
    /// (e.g. cold-start Run before any sketch is opened in the editor), non-default board
    /// names such as `"DOIT ESP32 DEVKIT V1"` would otherwise fall back to `Custom` with the
    /// raw display name instead of an FQBN. In that case we first try a fast local `boards.txt`
    /// scan under `Arduino15/packages`, and only fall back to `arduino-cli board listall` if
    /// the board still isn't recognized.
    pub(super) fn resolve_arduino_fqbn(
        &mut self,
        board: &str,
        custom_board: &str,
        raw_board: &str,
        raw_custom_board: &str,
        device: &str,
        tool_path: &str,
    ) -> String {
        let qb = if !raw_board.is_empty() {
            raw_board
        } else {
            board
        };
        let qc = if !raw_board.is_empty() {
            raw_custom_board
        } else {
            custom_board
        };

        let (b, cb, _) = match_board(qb, qc, device, &self.arduino_boards);
        let fqbn = resolve_board_fqbn(&b, &cb, &self.arduino_boards);
        if fqbn.contains(':') {
            return fqbn;
        }

        let local_boards = arduino::boards_from_arduino15();
        let (b, cb, _) = match_board(qb, qc, device, &local_boards);
        let local_fqbn = resolve_board_fqbn(&b, &cb, &local_boards);
        if local_fqbn.contains(':') {
            self.refresh_arduino_boards();
            return local_fqbn;
        }

        if self.arduino_list_path.as_deref() != Some(tool_path) {
            let loaded = if self.board_loading_path.as_deref() == Some(tool_path) {
                self.board_rx.take().and_then(|rx| rx.recv().ok())
            } else {
                self.board_rx = None;
                None
            };
            let (tp, boards) = match loaded {
                Some(pair) => pair,
                None => (tool_path.to_string(), list_arduino_boards(tool_path)),
            };
            self.arduino_list_path = Some(tp);
            self.arduino_boards = boards;
            self.board_loading_path = None;
            self.reapply_arduino_board();
            self.file_settings_changed();
            self.compiling_changed();

            let (b, cb, _) = match_board(qb, qc, device, &self.arduino_boards);
            return resolve_board_fqbn(&b, &cb, &self.arduino_boards);
        }

        fqbn
    }

    pub fn current_spec(&self) -> Option<&CompilerSpec> {
        let name = self.compiler_name();
        self.specs.get(&name)
    }

    pub fn refresh_tool_path_help(&mut self) {
        let path = self.tool_path();
        let (cands, warn) = match self.current_spec() {
            Some(spec) if !spec.is_none() => (
                editor::tool_path_candidates(spec),
                editor::check_tool_path(spec, &path),
            ),
            _ => (Vec::new(), String::new()),
        };
        self.tool_path_candidates = cands;
        self.tool_path_warning = warn;
    }

    pub fn persist_cfg(&self) {
        if self.current >= 0 {
            self.persist_cfg_for(self.current as usize);
        }
    }

    pub fn persist_cfg_for(&self, index: usize) {
        if let Some(d) = self.docs.get(index) {
            if d.path.is_empty() {
                return;
            }
            let _ = save_cfg(&d.path, &d.to_cfg());
            if d.has_compiler() {
                let name = d.compiler.clone();
                let tool = d.tool_path.clone();
                let incl = d.incl_path.clone();
                let board = d.board.clone();
                let custom_board = d.custom_board.clone();
                cs_engine::settings::edit(|s| {
                    if !tool.is_empty() {
                        s.compiler_tool_paths.insert(name.clone(), tool);
                    }
                    if !incl.is_empty() {
                        s.compiler_incl_paths.insert(name.clone(), incl);
                    }
                    if !board.is_empty() {
                        s.compiler_boards.insert(name.clone(), board);
                    }
                    if !custom_board.is_empty() {
                        s.compiler_custom_boards.insert(name, custom_board);
                    }
                });
            }
        }
    }

    pub fn compile_inner(&mut self, debug: bool, then_upload: bool, then_run: bool) {
        let idx = usize::try_from(self.current).unwrap_or(usize::MAX);
        self.compile_doc_inner(idx, debug, then_upload, then_run);
    }

    pub fn compile_doc_inner(
        &mut self,
        index: usize,
        debug: bool,
        then_upload: bool,
        then_run: bool,
    ) {
        if self.compiling {
            self.push_log(format!(
                "     {}",
                i18n::tr("Error: Compiler is already running")
            ));
            return;
        }
        let Some(doc) = self.docs.get(index) else {
            self.request_compiler_log();
            self.push_log("-------------------------------------------------------".to_string());
            self.push_log("     Error: No open document to compile");
            return;
        };
        if doc.path.is_empty() {
            if self.current != index as i32 {
                self.set_current_document(index as i32);
            }
            self.push_log(i18n::tr("Error: File not saved"));
            self.request_save_as();
            return;
        }
        let doc_path = doc.path.clone();
        let (any_saved, saved_current) = self.save_dirty_docs_for_target(Some(index), &doc_path);
        if any_saved {
            self.documents_changed();
            if saved_current {
                self.sync_lsp_open();
            }
        }
        self.request_compiler_log();
        self.push_log("-------------------------------------------------------".to_string());
        let d = &self.docs[index];
        let (
            src_path,
            mut compiler_name,
            board,
            custom_board,
            raw_board,
            raw_custom_board,
            device,
            family,
            extra_args,
            incl_path,
            tool_path,
            breakpoints,
        ) = (
            d.path.clone(),
            d.compiler.clone(),
            d.board.clone(),
            d.custom_board.clone(),
            d.raw_board.clone(),
            d.raw_custom_board.clone(),
            d.device.clone(),
            d.family.clone(),
            d.extra_args.clone(),
            d.incl_path.clone(),
            d.tool_path.clone(),
            d.breakpoints.clone(),
        );

        if compiler_name.is_empty() || compiler_name == "None" {
            let lower = src_path.to_ascii_lowercase();
            if lower.ends_with(".ino") || lower.ends_with(".pde") {
                compiler_name = "Arduino".to_string();
            } else if lower.ends_with(".as") {
                compiler_name = "AScript".to_string();
            } else if lower.ends_with(".gcb") {
                compiler_name = "GcBasic".to_string();
            }
        }
        let is_arduino = compiler_name.eq_ignore_ascii_case("arduino");
        let board = if board.is_empty() && is_arduino {
            "Uno".to_string()
        } else {
            board
        };
        let tool_path = if tool_path.is_empty() && is_arduino {
            arduino::tool_path_candidates()
                .first()
                .cloned()
                .unwrap_or_default()
        } else {
            tool_path
        };

        if let Some(d) = self.docs.get_mut(index) {
            let mut changed = false;
            if d.compiler.is_empty() || d.compiler == "None" {
                d.compiler = compiler_name.clone();
                changed = true;
            }
            if d.board.is_empty() && is_arduino {
                d.board = board.clone();
                changed = true;
            }
            if d.tool_path.is_empty() && is_arduino && !tool_path.is_empty() {
                d.tool_path = tool_path.clone();
                changed = true;
            }
            if changed {
                self.documents_changed();
                if self.current == index as i32 {
                    self.file_settings_changed();
                }
            }
        }

        let spec = self
            .specs
            .get(&compiler_name)
            .cloned()
            .unwrap_or_else(|| CompilerSpec {
                name: compiler_name.clone(),
                ..Default::default()
            });
        let fqbn = if spec.is_arduino() {
            self.resolve_arduino_fqbn(
                &board,
                &custom_board,
                &raw_board,
                &raw_custom_board,
                &device,
                &tool_path,
            )
        } else {
            String::new()
        };

        let norm_src = normalize_debug_path(Path::new(&src_path));
        let target_device_ids = self
            .firmware_device_uids
            .get(&norm_src)
            .cloned()
            .unwrap_or_default();

        // Fast path for Debug / Upload+Run: if this Arduino sketch's build artifacts
        // (and companion .elf when debugging) are already up to date for this exact
        // board/FQBN, skip re-running `arduino-cli compile` so debugging starts immediately.
        // Explicit Compile (`!then_upload && !then_run`) always rebuilds.
        if (then_upload || then_run)
            && spec.is_arduino()
            && extra_args.trim().is_empty()
            && incl_path.trim().is_empty()
        {
            if let Some(cached_fw) = arduino::cached_up_to_date_firmware(&src_path, &fqbn, debug) {
                let fw_str = cached_fw.to_string_lossy().into_owned();
                self.push_log(format!(
                    "     {} ({})",
                    i18n::tr("Build up to date, skipping recompile"),
                    fw_str
                ));
                self.compile_state = "ok".to_string();
                self.compile_file = Path::new(&src_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&src_path)
                    .to_string();
                self.compile_error_count = 0;
                self.compile_warning_count = 0;
                self.compiling = false;
                self.compiling_changed();
                let mut res = CompileResult::default();
                res.firmware = fw_str;
                self.finish_compile_result(
                    src_path,
                    debug,
                    then_upload,
                    then_run,
                    breakpoints,
                    target_device_ids,
                    true,
                    res,
                );
                return;
            }
        }

        let ctx = CompileCtx {
            file: src_path.clone(),
            board,
            custom_board,
            device,
            family,
            extra_args,
            incl_path,
            tool_path,
            debug,
            fqbn,
        };
        self.compiling = true;
        self.compile_state = "compiling".to_string();
        self.compile_file = Path::new(&src_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&src_path)
            .to_string();
        self.compile_error_count = 0;
        self.compile_warning_count = 0;
        self.compile_cancel_requested = false;
        self.compiling_changed();
        self.compile_job = Some(PendingCompile {
            job: CompileJob::spawn(spec, ctx),
            src_path,
            debug,
            then_upload,
            then_run,
            breakpoints,
            target_device_ids,
            load_debug_symbols: true,
        });
    }

    /// Save the target firmware document (if open and dirty) plus any dirty sibling
    /// documents in the same directory. Returns `(any_saved, saved_current)`.
    pub(super) fn save_dirty_docs_for_target(
        &mut self,
        open_idx: Option<usize>,
        effective_src_path: &str,
    ) -> (bool, bool) {
        let mut any_saved = false;
        let mut saved_current = false;
        if let Some(idx) = open_idx {
            if self.docs[idx].dirty {
                let doc_path = self.docs[idx].path.clone();
                if self.write_doc(idx, &doc_path) {
                    any_saved = true;
                    if self.current == idx as i32 {
                        saved_current = true;
                    }
                    self.persist_cfg_for(idx);
                }
            }
        }
        if let Some(target_dir) = Path::new(effective_src_path).parent() {
            let norm_dir = normalize_debug_path(target_dir);
            if !norm_dir.is_empty() {
                let dirty_siblings: Vec<(usize, String)> = self
                    .docs
                    .iter()
                    .enumerate()
                    .filter(|(i, d)| {
                        Some(*i) != open_idx
                            && d.dirty
                            && !d.path.is_empty()
                            && Path::new(&d.path)
                                .parent()
                                .is_some_and(|p| normalize_debug_path(p) == norm_dir)
                    })
                    .map(|(i, d)| (i, d.path.clone()))
                    .collect();
                for (i, sib_path) in dirty_siblings {
                    if self.write_doc(i, &sib_path) {
                        any_saved = true;
                        if self.current == i as i32 {
                            saved_current = true;
                        }
                        self.persist_cfg_for(i);
                    }
                }
            }
        }
        (any_saved, saved_current)
    }

    pub(super) fn clear_compile_batch(&mut self) {
        self.compile_queue.clear();
        self.pending_sim_action = None;
        self.batch_log_opened = false;
        self.batch_any_handled = false;
    }

    /// Save and compile the firmware source file for a single MCU before starting
    /// simulation. Returns `true` when simulation startup is handled here (compiling
    /// or fast-path upload+run), or `false` when no compilation is applicable and
    /// the caller should start simulation directly.
    pub fn compile_path_for_sim(&mut self, src_path: String, target_device_id: String) -> bool {
        let targets = if target_device_id.is_empty() {
            Vec::new()
        } else {
            vec![target_device_id]
        };
        self.compile_path_target(None, src_path, targets, false, true, true, true, false)
    }

    /// Save and compile all MCU firmware source files in `devices` before starting
    /// simulation (or debugging when `debug` is `true`). Returns `true` when startup
    /// is handled here (compiling or fast-path upload+run), or `false` when no
    /// compilation is applicable and the caller should start simulation directly.
    pub fn compile_devices_for_sim_inner(&mut self, devices: Value, debug: bool) -> bool {
        if self.compiling {
            self.push_log(format!(
                "     {}",
                i18n::tr("Error: Compiler is already running")
            ));
            return true;
        }
        let targets = collect_device_compile_targets(devices, debug);
        if targets.is_empty() {
            return false;
        }
        self.compile_queue = VecDeque::from(targets);
        self.pending_sim_action = Some(if debug {
            PendingSimAction::DebugRun
        } else {
            PendingSimAction::RestartPowerOn
        });
        self.batch_log_opened = false;
        self.batch_any_handled = false;
        self.pump_compile_queue()
    }

    fn pump_compile_queue(&mut self) -> bool {
        while self.compile_job.is_none() {
            let Some(item) = self.compile_queue.pop_front() else {
                let action = self.pending_sim_action.take();
                let any = self.batch_any_handled;
                self.batch_log_opened = false;
                self.batch_any_handled = false;
                if any {
                    match action {
                        Some(PendingSimAction::RestartPowerOn) => {
                            self.push_log(i18n::tr("Starting simulation…"));
                            self.request_restart_power_on();
                        }
                        Some(PendingSimAction::DebugRun) => {
                            self.push_log(i18n::tr("Starting debug session…"));
                            self.request_debug_run();
                        }
                        None => {}
                    }
                    return true;
                }
                return false;
            };

            let handled = self.compile_path_target(
                None,
                item.src_path,
                item.target_device_ids,
                item.debug,
                false,
                item.load_debug_symbols,
                item.silent_if_cached,
                true,
            );
            if handled {
                self.batch_any_handled = true;
            }
            if self.compile_job.is_some() || self.pending_sim_action.is_none() {
                return true;
            }
        }
        true
    }

    /// Save, compile (if needed), and upload the document at `index` to `target_device_id`
    /// without starting simulation.
    pub fn upload_doc_to_device_inner(&mut self, index: usize, target_device_id: String) {
        let Some(doc) = self.docs.get(index) else {
            return;
        };
        if doc.path.is_empty() {
            self.set_current_document(index as i32);
            self.push_log(i18n::tr("Error: File not saved"));
            self.request_save_as();
            return;
        }
        let src_path = doc.path.clone();
        let targets = if target_device_id.is_empty() {
            Vec::new()
        } else {
            vec![target_device_id]
        };
        self.compile_path_target(
            Some(index),
            src_path,
            targets,
            false,
            false,
            true,
            false,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_path_target(
        &mut self,
        explicit_idx: Option<usize>,
        src_path: String,
        target_device_ids: Vec<String>,
        debug: bool,
        then_run: bool,
        load_debug_symbols: bool,
        silent_if_cached: bool,
        in_batch: bool,
    ) -> bool {
        if src_path.is_empty() {
            return false;
        }
        if self.compiling {
            self.push_log(format!(
                "     {}",
                i18n::tr("Error: Compiler is already running")
            ));
            return true;
        }

        let norm_target = normalize_debug_path(Path::new(&src_path));
        let open_idx = explicit_idx.filter(|&i| i < self.docs.len()).or_else(|| {
            self.docs.iter().position(|d| {
                !d.path.is_empty()
                    && (d.path == src_path
                        || normalize_debug_path(Path::new(&d.path)) == norm_target)
            })
        });

        let effective_src_path = open_idx
            .and_then(|i| self.docs.get(i))
            .map(|d| d.path.clone())
            .unwrap_or(src_path);

        if open_idx.is_none() && !Path::new(&effective_src_path).is_file() {
            return false;
        }

        let (any_saved, saved_current) =
            self.save_dirty_docs_for_target(open_idx, &effective_src_path);
        if any_saved {
            self.documents_changed();
            if saved_current {
                self.sync_lsp_open();
            }
        }

        let (
            src_path,
            mut compiler_name,
            board,
            custom_board,
            raw_board,
            raw_custom_board,
            device,
            family,
            extra_args,
            incl_path,
            tool_path,
            breakpoints,
        ) = if let Some(idx) = open_idx {
            let d = &self.docs[idx];
            (
                d.path.clone(),
                d.compiler.clone(),
                d.board.clone(),
                d.custom_board.clone(),
                d.raw_board.clone(),
                d.raw_custom_board.clone(),
                d.device.clone(),
                d.family.clone(),
                d.extra_args.clone(),
                d.incl_path.clone(),
                d.tool_path.clone(),
                d.breakpoints.clone(),
            )
        } else {
            if !Path::new(&effective_src_path).is_file() {
                return false;
            }
            let title = Path::new(&effective_src_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            let mut temp = Document::untitled(0, title);
            temp.path = effective_src_path.clone();
            temp.text = fs::read_to_string(&effective_src_path).unwrap_or_default();
            let cfg = load_cfg(&effective_src_path);
            let settings = cs_engine::settings::get();
            temp.apply_cfg(cfg, &self.specs, &settings, &self.arduino_boards);
            (
                temp.path,
                temp.compiler,
                temp.board,
                temp.custom_board,
                temp.raw_board,
                temp.raw_custom_board,
                temp.device,
                temp.family,
                temp.extra_args,
                temp.incl_path,
                temp.tool_path,
                temp.breakpoints,
            )
        };

        if compiler_name.is_empty() || compiler_name == "None" {
            let lower = src_path.to_ascii_lowercase();
            if lower.ends_with(".ino") || lower.ends_with(".pde") {
                compiler_name = "Arduino".to_string();
            } else if lower.ends_with(".as") {
                compiler_name = "AScript".to_string();
            } else if lower.ends_with(".gcb") {
                compiler_name = "GcBasic".to_string();
            }
        }
        if compiler_name.is_empty() || compiler_name == "None" {
            return false;
        }

        let is_arduino = compiler_name.eq_ignore_ascii_case("arduino");
        let board = if board.is_empty() && is_arduino {
            "Uno".to_string()
        } else {
            board
        };
        let tool_path = if tool_path.is_empty() && is_arduino {
            arduino::tool_path_candidates()
                .first()
                .cloned()
                .unwrap_or_default()
        } else {
            tool_path
        };

        if let Some(idx) = open_idx {
            if let Some(d) = self.docs.get_mut(idx) {
                let mut changed = false;
                if d.compiler.is_empty() || d.compiler == "None" {
                    d.compiler = compiler_name.clone();
                    changed = true;
                }
                if d.board.is_empty() && is_arduino {
                    d.board = board.clone();
                    changed = true;
                }
                if d.tool_path.is_empty() && is_arduino && !tool_path.is_empty() {
                    d.tool_path = tool_path.clone();
                    changed = true;
                }
                if changed {
                    self.documents_changed();
                    if self.current == idx as i32 {
                        self.file_settings_changed();
                    }
                }
            }
        }

        let spec = self
            .specs
            .get(&compiler_name)
            .cloned()
            .unwrap_or_else(|| CompilerSpec {
                name: compiler_name.clone(),
                ..Default::default()
            });
        if spec.is_none() {
            return false;
        }

        let fqbn = if spec.is_arduino() {
            self.resolve_arduino_fqbn(
                &board,
                &custom_board,
                &raw_board,
                &raw_custom_board,
                &device,
                &tool_path,
            )
        } else {
            String::new()
        };

        // Fast path: if the Arduino sketch artifacts are already up to date for this
        // exact board/FQBN, skip recompiling and upload (and optionally run) immediately.
        if spec.is_arduino() && extra_args.trim().is_empty() && incl_path.trim().is_empty() {
            if let Some(cached_fw) = arduino::cached_up_to_date_firmware(&src_path, &fqbn, debug) {
                let fw_str = cached_fw.to_string_lossy().into_owned();
                self.push_log(format!(
                    "     {} ({})",
                    i18n::tr("Build up to date, skipping recompile"),
                    fw_str
                ));
                if silent_if_cached {
                    if self.compile_state == "error" || self.compile_state == "cancelled" {
                        self.compile_state = "idle".to_string();
                        self.compile_error_count = 0;
                        self.compile_warning_count = 0;
                        self.compiling_changed();
                    }
                } else {
                    self.compile_state = "ok".to_string();
                    self.compile_file = Path::new(&src_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&src_path)
                        .to_string();
                    self.compile_error_count = 0;
                    self.compile_warning_count = 0;
                    self.compiling = false;
                    self.compiling_changed();
                }
                let mut res = CompileResult::default();
                res.firmware = fw_str;
                self.finish_compile_result(
                    src_path,
                    debug,
                    true,
                    then_run,
                    breakpoints,
                    target_device_ids,
                    load_debug_symbols,
                    res,
                );
                return true;
            }
        }

        if !in_batch || !self.batch_log_opened {
            self.request_compiler_log();
            if in_batch {
                self.batch_log_opened = true;
            }
        }
        self.push_log("-------------------------------------------------------".to_string());

        let ctx = CompileCtx {
            file: src_path.clone(),
            board,
            custom_board,
            device,
            family,
            extra_args,
            incl_path,
            tool_path,
            debug,
            fqbn,
        };
        self.compiling = true;
        self.compile_state = "compiling".to_string();
        self.compile_file = Path::new(&src_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&src_path)
            .to_string();
        self.compile_error_count = 0;
        self.compile_warning_count = 0;
        self.compile_cancel_requested = false;
        self.compiling_changed();
        self.compile_job = Some(PendingCompile {
            job: CompileJob::spawn(spec, ctx),
            src_path,
            debug,
            then_upload: true,
            then_run,
            breakpoints,
            target_device_ids,
            load_debug_symbols,
        });
        true
    }

    pub fn drain_compile_job(&mut self) {
        let Some(pending) = self.compile_job.as_mut() else {
            return;
        };
        let events = pending.job.drain();
        if events.is_empty() {
            return;
        }
        let mut finished = None;
        let mut logged = false;
        for ev in events {
            match ev {
                CompileEvent::Log(line) => {
                    self.push_log_buffered(line);
                    logged = true;
                }
                CompileEvent::Finished(result) => finished = Some(result),
            }
        }
        if logged {
            self.log_changed();
        }
        if let Some(result) = finished {
            let pending = self.compile_job.take().expect("compile job just finished");
            let was_cancelled = self.compile_cancel_requested
                || result
                    .log
                    .iter()
                    .any(|l| l.to_ascii_lowercase().contains("cancelled by user"));
            self.compile_cancel_requested = false;
            self.compile_error_count = result.errors.len() as i32;
            self.compile_warning_count = result.warnings.len() as i32;
            self.compile_state = if was_cancelled {
                "cancelled"
            } else if result.error != 0 {
                "error"
            } else if !result.warnings.is_empty() {
                "warn"
            } else {
                "ok"
            }
            .to_string();
            self.compiling = false;
            self.compiling_changed();
            if was_cancelled {
                self.clear_compile_batch();
            }
            self.apply_compile_result(pending, result);
            if self.pending_sim_action.is_some() {
                let _ = self.pump_compile_queue();
            }
        }
    }

    pub fn apply_compile_result(&mut self, pending: PendingCompile, result: CompileResult) {
        self.finish_compile_result(
            pending.src_path,
            pending.debug,
            pending.then_upload,
            pending.then_run,
            pending.breakpoints,
            pending.target_device_ids,
            pending.load_debug_symbols,
            result,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_compile_result(
        &mut self,
        src_path: String,
        debug: bool,
        then_upload: bool,
        then_run: bool,
        breakpoints: Vec<i32>,
        target_device_ids: Vec<String>,
        load_debug_symbols: bool,
        result: CompileResult,
    ) {
        let norm_src = normalize_debug_path(Path::new(&src_path));
        let matched_idx = self.docs.iter().position(|d| {
            !d.path.is_empty()
                && (d.path == src_path || normalize_debug_path(Path::new(&d.path)) == norm_src)
        });
        if let Some(idx) = matched_idx {
            self.docs[idx].errors = result.errors;
            self.docs[idx].warnings = result.warnings;
        }
        self.marks_changed();
        if result.error > 0 {
            self.clear_compile_batch();
            if let Some(idx) = matched_idx {
                if self.current != idx as i32 {
                    self.set_current_document(idx as i32);
                }
                self.goto_1based(result.error);
            }
            return;
        }
        if result.error != 0 {
            self.clear_compile_batch();
            self.push_log("     WARNING: Compilation Not Done");
            return;
        }
        self.last_firmware = result.firmware.clone();
        self.last_upload_target = target_device_ids.last().cloned().unwrap_or_default();
        self.firmware_changed();

        if load_debug_symbols {
            let path_obj = Path::new(&src_path);
            let sketch_dir = path_obj.parent().unwrap_or_else(|| Path::new(""));
            let file_stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let firmware_path = Path::new(&result.firmware);
            let session = DebugSession::global();

            let bp_lines: Vec<usize> = breakpoints
                .iter()
                .filter(|&&l| l > 0)
                .map(|&l| l as usize)
                .collect();

            let raw_elfs = [
                sketch_dir.join(format!("{file_stem}.elf")),
                sketch_dir.join(format!("{file_stem}.ino.elf")),
                firmware_path.with_extension("elf"),
            ];
            let mut unique_elfs = Vec::new();
            for cand in raw_elfs {
                if !cand.is_file() {
                    continue;
                }
                let canon = fs::canonicalize(&cand).unwrap_or(cand);
                if !unique_elfs.iter().any(|p| p == &canon) {
                    unique_elfs.push(canon);
                }
            }
            let lst_path = sketch_dir.join(format!("{file_stem}.lst"));
            let has_lst = lst_path.is_file();

            if !has_lst && unique_elfs.len() == 1 {
                session.set_breakpoints_for_file(path_obj, &bp_lines);
                let _ = session.load_elf_if_changed(&unique_elfs[0]);
            } else {
                let mut loaded_syms: Option<DebugSymbols> = None;
                let mut primary_elf = None;
                for elf_path in &unique_elfs {
                    if let Ok(bytes) = fs::read(elf_path) {
                        if let Some(parsed) = ElfParser::parse(&bytes) {
                            if primary_elf.is_none() {
                                primary_elf = Some(elf_path.clone());
                            }
                            if let Some(s) = &mut loaded_syms {
                                s.merge(parsed);
                            } else {
                                loaded_syms = Some(parsed);
                            }
                        }
                    }
                }
                if has_lst {
                    if let Ok(content) = fs::read_to_string(&lst_path) {
                        let parsed = LstParser::parse(&content, path_obj, LstKind::Auto);
                        if let Some(s) = &mut loaded_syms {
                            s.merge(parsed);
                        } else {
                            loaded_syms = Some(parsed);
                        }
                    }
                }
                if let Some(syms) = loaded_syms {
                    session.set_breakpoints_for_file(path_obj, &bp_lines);
                    session.load_symbols(syms);
                    if !has_lst {
                        if let Some(elf) = primary_elf {
                            session.mark_loaded_elf(&elf);
                        }
                    }
                }
            }
        }

        if then_upload {
            if self.last_firmware.is_empty() {
                self.clear_compile_batch();
                self.push_log(format!(
                    "     {}",
                    i18n::tr("Error: Firmware file doesn't exist:")
                ));
            } else if !Path::new(&self.last_firmware).is_file() {
                self.clear_compile_batch();
                self.push_log(format!(
                    "\n{}\n{}",
                    i18n::tr("Error: Firmware file doesn't exist:"),
                    self.last_firmware
                ));
            } else {
                let upload_prefix = i18n::tr("Uploading firmware:");
                let fw = self.last_firmware.clone();
                self.push_log(format!("{upload_prefix} {fw}"));
                if target_device_ids.is_empty() {
                    self.last_upload_target.clear();
                    self.request_upload_to(fw, String::new());
                } else {
                    for uid in &target_device_ids {
                        self.last_upload_target = uid.clone();
                        self.request_upload_to(fw.clone(), uid.clone());
                    }
                }
            }
        }
        if then_run
            && result.error == 0
            && !self.last_firmware.is_empty()
            && Path::new(&self.last_firmware).is_file()
        {
            if debug {
                self.push_log(i18n::tr("Starting debug session…"));
                self.request_debug_run();
            } else {
                self.push_log(i18n::tr("Starting simulation…"));
                self.request_restart_power_on();
            }
        }
    }

    pub fn write_doc(&mut self, index: usize, path: &str) -> bool {
        let Some(d) = self.docs.get(index) else {
            return false;
        };
        let rec_id = d.recovery_id(&self.project_path);
        let text = d.text.clone();
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("save file: {e}");
            return false;
        }
        self.file_watcher.sync(path);
        if let Some(d) = self.docs.get_mut(index) {
            d.dirty = false;
        }
        backup::clear_file(&rec_id);
        true
    }

    pub fn write_current(&mut self, path: &str) {
        if self.current < 0 {
            return;
        }
        if self.write_doc(self.current as usize, path) {
            self.sync_lsp_open();
            self.documents_changed();
            self.persist_cfg();
        }
    }
}
