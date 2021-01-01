//! Compilation jobs, compiler spec resolution, and Arduino board discovery for EditorPanel.

use super::EditorPanel;
use cs_engine::backup;
use cs_engine::editor::{
    self, CompileCtx, CompileEvent, CompileJob, CompileResult, CompilerSpec, list_arduino_boards,
    match_board, resolve_board_fqbn, save_cfg,
};
use std::path::Path;

pub struct PendingCompile {
    pub job: CompileJob,
    pub src_path: String,
    pub debug: bool,
    pub then_upload: bool,
    pub then_run: bool,
    pub breakpoints: Vec<i32>,
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
        let tp = if tp.is_empty() {
            let s = cs_engine::settings::get();
            s.compiler_tool_paths
                .get("Arduino")
                .cloned()
                .unwrap_or_default()
        } else {
            tp
        };
        if tp == self.arduino_list_path {
            return;
        }
        self.arduino_list_path = tp.clone();
        self.arduino_boards = list_arduino_boards(&tp);
    }

    pub fn reapply_arduino_board(&mut self) {
        let Some((board, custom, device, is_arduino)) = self.current_doc().map(|d| {
            (
                d.board.clone(),
                d.custom_board.clone(),
                d.device.clone(),
                d.compiler.eq_ignore_ascii_case("arduino"),
            )
        }) else {
            return;
        };
        if !is_arduino {
            return;
        }
        let (b, cb, dev) = match_board(&board, &custom, &device, &self.arduino_boards);
        if let Some(d) = self.current_doc_mut() {
            d.board = b;
            d.custom_board = cb;
            if !dev.is_empty() {
                d.device = dev;
            }
        }
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
        if let Some(d) = self.current_doc() {
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
        if self.compiling {
            self.push_log(format!(
                "     {}",
                cs_engine::i18n::tr("Error: Compiler is already running")
            ));
            return;
        }
        if self.current_doc().is_some_and(|d| d.dirty) {
            let path = self.current_path();
            if path.is_empty() {
                self.push_log(cs_engine::i18n::tr("Error: File not saved"));
                self.request_save_as();
                return;
            }
            self.write_current(&path);
        }
        self.request_compiler_log();
        self.push_log("-------------------------------------------------------".to_string());
        let (
            src_path,
            mut compiler_name,
            board,
            custom_board,
            device,
            family,
            extra_args,
            incl_path,
            tool_path,
            breakpoints,
        ) = if let Some(d) = self.current_doc() {
            (
                d.path.clone(),
                d.compiler.clone(),
                d.board.clone(),
                d.custom_board.clone(),
                d.device.clone(),
                d.family.clone(),
                d.extra_args.clone(),
                d.incl_path.clone(),
                d.tool_path.clone(),
                d.breakpoints.clone(),
            )
        } else {
            self.push_log("     Error: No open document to compile");
            return;
        };

        if src_path.is_empty() {
            self.push_log("     Error: File not saved");
            self.request_save_as();
            return;
        }

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
            cs_engine::editor::arduino::tool_path_candidates()
                .first()
                .cloned()
                .unwrap_or_default()
        } else {
            tool_path
        };

        if let Some(d) = self.current_doc_mut() {
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
                self.file_settings_changed();
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
            let (b, cb, _) = match_board(&board, &custom_board, &device, &self.arduino_boards);
            resolve_board_fqbn(&b, &cb, &self.arduino_boards)
        } else {
            String::new()
        };
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
        self.compiling_changed();
        self.compile_job = Some(PendingCompile {
            job: CompileJob::spawn(spec, ctx),
            src_path,
            debug,
            then_upload,
            then_run,
            breakpoints,
        });
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
        for ev in events {
            match ev {
                CompileEvent::Log(line) => self.push_log_buffered(line),
                CompileEvent::Finished(result) => finished = Some(result),
            }
        }
        if let Some(result) = finished {
            let pending = self.compile_job.take().expect("compile job just finished");
            self.compiling = false;
            self.compiling_changed();
            self.apply_compile_result(pending, result);
        }
    }

    pub fn apply_compile_result(&mut self, pending: PendingCompile, result: CompileResult) {
        let src_path = pending.src_path;
        if let Some(d) = self.docs.iter_mut().find(|d| d.path == src_path) {
            d.errors = result.errors;
            d.warnings = result.warnings;
        }
        self.marks_changed();
        if result.error > 0 {
            if self.current_path() == src_path {
                self.goto_1based(result.error);
            }
            return;
        }
        if result.error != 0 {
            self.push_log("     WARNING: Compilation Not Done");
            return;
        }
        self.last_firmware = result.firmware.clone();
        self.firmware_changed();

        {
            let path_obj = Path::new(&src_path);
            let sketch_dir = path_obj.parent().unwrap_or_else(|| Path::new(""));
            let file_stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let firmware_path = Path::new(&result.firmware);

            let mut loaded_syms: Option<cs_engine::debug::DebugSymbols> = None;
            let elf_candidates = [
                sketch_dir.join(format!("{file_stem}.elf")),
                sketch_dir.join(format!("{file_stem}.ino.elf")),
                firmware_path.with_extension("elf"),
            ];
            for elf_path in &elf_candidates {
                if !elf_path.is_file() {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(elf_path) {
                    if let Some(parsed) = cs_engine::debug::ElfParser::parse(&bytes) {
                        if let Some(s) = &mut loaded_syms {
                            s.merge(parsed);
                        } else {
                            loaded_syms = Some(parsed);
                        }
                    }
                }
            }
            let lst_path = sketch_dir.join(format!("{file_stem}.lst"));
            if lst_path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&lst_path) {
                    let parsed = cs_engine::debug::LstParser::parse(
                        &content,
                        path_obj,
                        cs_engine::debug::LstKind::Auto,
                    );
                    if let Some(s) = &mut loaded_syms {
                        s.merge(parsed);
                    } else {
                        loaded_syms = Some(parsed);
                    }
                }
            }
            if let Some(syms) = loaded_syms {
                let session = cs_engine::debug::DebugSession::global();
                session.load_symbols(syms);
                if pending.debug {
                    for bp in &pending.breakpoints {
                        session.add_breakpoint(path_obj, *bp as usize);
                    }
                }
            }
        }

        if pending.then_upload {
            if self.last_firmware.is_empty() {
                self.push_log(format!(
                    "     {}",
                    cs_engine::i18n::tr("Error: Firmware file doesn't exist:")
                ));
            } else if !Path::new(&self.last_firmware).is_file() {
                self.push_log(format!(
                    "\n{}\n{}",
                    cs_engine::i18n::tr("Error: Firmware file doesn't exist:"),
                    self.last_firmware
                ));
            } else {
                let upload_prefix = cs_engine::i18n::tr("Uploading firmware:");
                self.push_log(format!("{upload_prefix} {}", self.last_firmware));
                self.request_upload();
            }
        }
        if pending.then_run
            && result.error == 0
            && !self.last_firmware.is_empty()
            && Path::new(&self.last_firmware).is_file()
        {
            if pending.debug {
                self.push_log(cs_engine::i18n::tr("Starting debug session…"));
                self.debug_start();
            }
            self.push_log(cs_engine::i18n::tr("Starting simulation…"));
            self.request_power_on();
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
