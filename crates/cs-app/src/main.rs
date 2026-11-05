#![recursion_limit = "1024"]
#![windows_subsystem = "windows"]

mod app;
mod app_dialog;
mod circuit_canvas;
mod circuit_panel;
mod command_center;
mod component_list;
mod editor_panel;
mod file_browser;
mod image_frame;
mod info_widget;
mod installer;
mod logs;
mod macos_host;
mod mcu_monitor;
mod memory_table;
mod menu_bar;
mod native_canvas;
mod path_util;
mod pending;
mod plots;
mod serial;
mod tab_switcher;
mod vello_preview;

use std::env;
use std::fs;
use std::io;
use std::process;

use app::App;
use app_dialog::AppDialog;
use circuit_canvas::CircuitCanvas;
use circuit_panel::CircuitPanel;
use command_center::CommandCenter;
use component_list::ComponentList;
use cs_engine::headless::{self, Cli, OpenKind};
use editor_panel::EditorPanel;
use file_browser::FileBrowser;
use info_widget::InfoWidget;
use installer::Installer;
use logs::{CompilerLog, SimulatorLog};
use mcu_monitor::McuMonitor;
use memory_table::MemoryTable;
use menu_bar::AppMenuBar;
use plots::{LogicAnalyzer, Oscilloscope};
use qtbridge::QApp;
use serial::{SerialMonitor, SerialTerminal};
use tab_switcher::TabSwitcher;

#[cfg(windows)]
fn attach_parent_console() {
    unsafe extern "system" {
        fn AttachConsole(dw_process_id: u32) -> i32;
        fn GetStdHandle(n_std_handle: u32) -> isize;
        fn SetStdHandle(n_std_handle: u32, h_handle: isize) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const INVALID_HANDLE_VALUE: isize = -1;

    unsafe {
        let out = GetStdHandle(STD_OUTPUT_HANDLE);
        let err = GetStdHandle(STD_ERROR_HANDLE);
        let has_out = out != 0 && out != INVALID_HANDLE_VALUE;
        let has_err = err != 0 && err != INVALID_HANDLE_VALUE;

        if (!has_out || !has_err) && AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            if has_out {
                let _ = SetStdHandle(STD_OUTPUT_HANDLE, out);
            }
            if has_err {
                let _ = SetStdHandle(STD_ERROR_HANDLE, err);
            }
        }
    }
}

#[cfg(windows)]
fn remove_legacy_dlls() {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let legacy = dir.join("D3Dcompiler_47.dll");
            if legacy.exists() {
                let _ = fs::remove_file(legacy);
            }
            let sw = dir.join("opengl32sw.dll");
            if sw.exists() {
                let _ = fs::remove_file(sw);
            }
        }
    }
}

fn main() {
    #[cfg(windows)]
    remove_legacy_dlls();

    let cli = headless::parse_args(env::args());
    #[cfg(windows)]
    if !matches!(cli, Cli::Gui { .. }) {
        attach_parent_console();
    }

    match cli {
        Cli::Help => {
            print!("{}", headless::help_text());
            return;
        }
        Cli::Error(msg) => {
            eprintln!("{msg}");
            process::exit(2);
        }
        Cli::RunCirc { path } => {
            if let Err(e) = headless::run_circ(&path, |_| true) {
                eprintln!("{e}");
                process::exit(1);
            }
            return;
        }
        Cli::Test { folder } => {
            match headless::run_batch_folder(&folder) {
                Ok(report) => {
                    let _ = headless::print_batch_report(&report, io::stderr());
                    if !report.ok() {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            }
            return;
        }
        Cli::Gui { open } => {
            if env::args().any(|a| a == "-noproject") {
                let _ = pending::NO_PROJECT.set(true);
            }
            match open {
                Some(OpenKind::Circuit(p)) => {
                    let _ = pending::OPEN_CIRCUIT.set(p.to_string_lossy().into_owned());
                }
                Some(OpenKind::Editor(p)) => {
                    let _ = pending::OPEN_EDITOR.set(p.to_string_lossy().into_owned());
                }
                None => {}
            }
        }
    }

    gui_main();
}

fn gui_main() {
    const RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/resources.rcc"));
    qtbridge::qresource::register_bytes(RESOURCES);

    macos_host::init_process("Circuit Simulator");
    native_canvas::set_native_text_rendering();
    native_canvas::register_canvas_item();

    let settings = cs_engine::settings::get();
    cs_engine::i18n::set_locale(&settings.language);

    let mut app = QApp::new();
    app.application_name("Circuit Simulator");
    // Palette / colorScheme must be set before QML constructs SystemPalette.
    let _ = macos_host::apply_theme(&settings.theme);
    macos_host::apply_app_icon();
    app.register::<App>()
        .register::<AppDialog>()
        .register::<CircuitCanvas>()
        .register::<CircuitPanel>()
        .register::<CommandCenter>()
        .register::<ComponentList>()
        .register::<EditorPanel>()
        .register::<FileBrowser>()
        .register::<InfoWidget>()
        .register::<SimulatorLog>()
        .register::<CompilerLog>()
        .register::<Oscilloscope>()
        .register::<LogicAnalyzer>()
        .register::<McuMonitor>()
        .register::<MemoryTable>()
        .register::<SerialMonitor>()
        .register::<SerialTerminal>()
        .register::<Installer>()
        .register::<AppMenuBar>()
        .register::<TabSwitcher>()
        .load_qml_from_file("qrc:/qt/qml/cs_app/Main.qml")
        .run();
}
