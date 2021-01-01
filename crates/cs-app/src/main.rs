#![recursion_limit = "1024"]

mod app;
mod app_dialog;
mod circuit_canvas;
mod circuit_panel;
mod command_center;
mod component_list;
mod editor_panel;
mod file_browser;
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

use app::App;
use app_dialog::AppDialog;
use circuit_canvas::CircuitCanvas;
use circuit_panel::CircuitPanel;
use command_center::CommandCenter;
use component_list::ComponentList;
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

fn main() {
    match cs_engine::headless::parse_args(std::env::args()) {
        cs_engine::headless::Cli::Help => {
            print!("{}", cs_engine::headless::help_text());
            return;
        }
        cs_engine::headless::Cli::Error(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
        cs_engine::headless::Cli::RunCirc { path } => {
            if let Err(e) = cs_engine::headless::run_circ(&path, |_| true) {
                eprintln!("{e}");
                std::process::exit(1);
            }
            return;
        }
        cs_engine::headless::Cli::Test { folder } => {
            match cs_engine::headless::run_batch_folder(&folder) {
                Ok(report) => {
                    let _ = cs_engine::headless::print_batch_report(&report, std::io::stderr());
                    if !report.ok() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        cs_engine::headless::Cli::Gui { open } => {
            if std::env::args().any(|a| a == "-noproject") {
                let _ = pending::NO_PROJECT.set(true);
            }
            match open {
                Some(cs_engine::headless::OpenKind::Circuit(p)) => {
                    let _ = pending::OPEN_CIRCUIT.set(p.to_string_lossy().into_owned());
                }
                Some(cs_engine::headless::OpenKind::Editor(p)) => {
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
