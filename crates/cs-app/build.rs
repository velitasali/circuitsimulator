use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let (rev, date) = compile_stamp();
    println!("cargo:rustc-env=CS_REVNO={rev}");
    println!("cargo:rustc-env=CS_BUILDDATE={date}");
    compile_grid_shader();
    compile_resources();
    compile_macos();
    compile_windows();
    compile_theme();
    compile_canvas_item();
}

fn compile_windows() {
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
            return;
        }
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let icon = manifest.join("../../resources/icons/circuitsimulator.ico");
        println!("cargo:rerun-if-changed={}", icon.display());

        let mut res = winres::WindowsResource::new();
        res.set_icon(icon.to_str().unwrap());
        if let Err(e) = res.compile() {
            eprintln!("warning: failed to compile Windows icon resource: {e}");
        }
    }
}

fn compile_macos() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let macos = manifest.join("../../macos");
    println!("cargo:rerun-if-changed={}", macos.display());
    let icon = manifest.join("../../resources/icons/circuitsimulator.icns");
    println!("cargo:rustc-env=CS_ICON_PATH={}", icon.display());

    let mut build = cc::Build::new();
    build
        .file(macos.join("menu.mm"))
        .file(macos.join("touchbar.mm"))
        .file(macos.join("titlebar.mm"))
        .file(macos.join("appicon.mm"))
        .include(&macos)
        .flag("-fobjc-arc")
        .flag("-std=c++17");

    if let Some(libs) = qmake_query("QT_INSTALL_LIBS") {
        let p = PathBuf::from(&libs);
        build.flag(&format!("-F{libs}"));
        build.include(p.join("QtCore.framework/Headers"));
        build.include(p.join("QtGui.framework/Headers"));
    }
    if let Some(inc) = qmake_query("QT_INSTALL_HEADERS") {
        let p = PathBuf::from(inc);
        build.include(&p);
        build.include(p.join("QtCore"));
        build.include(p.join("QtGui"));
    }

    build.compile("cs_macos");
    println!("cargo:rustc-link-lib=framework=AppKit");
    if let Some(libs) = qmake_query("QT_INSTALL_LIBS") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{libs}");
    }
}

fn compile_theme() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        return;
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cpp_dir = manifest.join("cpp");
    let header = cpp_dir.join("theme.h");
    let source = cpp_dir.join("theme.cpp");
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed={}", source.display());

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(&source)
        .include(&cpp_dir)
        .std("c++17")
        .flag_if_supported("/Zc:__cplusplus")
        .flag_if_supported("/permissive-");

    if let Some(inc) = qmake_query("QT_INSTALL_HEADERS") {
        let p = PathBuf::from(inc);
        build.include(&p);
        build.include(p.join("QtCore"));
        build.include(p.join("QtGui"));
    }

    build.compile("cs_theme");

    if let Some(libs) = qmake_query("QT_INSTALL_LIBS") {
        println!("cargo:rustc-link-search=native={libs}");
        println!("cargo:rustc-link-lib=Qt6Gui");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dwmapi");
        println!("cargo:rustc-link-lib=user32");
    }
}

/// Same qsb flags as CircuitSimulator.pri. Output sits next to qml/embed.rs
/// because include_bytes_qml cannot use `..` in the file argument.
fn compile_grid_shader() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let frag = manifest.join("../../resources/shaders/grid.frag");
    let qsb_out = manifest.join("../../qml/grid.frag.qsb");
    println!("cargo:rerun-if-changed={}", frag.display());

    let qsb = qsb_bin();
    let status = Command::new(&qsb)
        .args([
            "--glsl",
            "120,150,300es",
            "--hlsl",
            "50",
            "--msl",
            "12",
            "-o",
        ])
        .arg(&qsb_out)
        .arg(&frag)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", qsb.display()));
    if !status.success() {
        panic!("qsb failed on {}", frag.display());
    }
}

fn qsb_bin() -> PathBuf {
    if let Some(p) = qmake_query("QT_HOST_BINS") {
        let candidate = Path::new(&p).join("qsb");
        if candidate.exists() {
            return candidate;
        }
        let exe = Path::new(&p).join("qsb.exe");
        if exe.exists() {
            return exe;
        }
    }
    PathBuf::from("qsb")
}

fn moc_bin() -> PathBuf {
    if let Some(p) = qmake_query("QT_HOST_LIBEXECS") {
        let candidate = Path::new(&p).join("moc");
        if candidate.exists() {
            return candidate;
        }
    }
    if let Some(p) = qmake_query("QT_HOST_BINS") {
        let candidate = Path::new(&p).join("moc");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("moc")
}

fn compile_canvas_item() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let cpp_dir = manifest.join("cpp");
    let header = cpp_dir.join("circuit_canvas_item.h");
    let source = cpp_dir.join("circuit_canvas_item.cpp");
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed={}", source.display());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let moc_out = out_dir.join("moc_circuit_canvas_item.cpp");

    let moc = moc_bin();
    let status = Command::new(&moc)
        .arg(&header)
        .arg("-o")
        .arg(&moc_out)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", moc.display()));
    if !status.success() {
        panic!("moc failed on {}", header.display());
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(&source)
        .file(&moc_out)
        .include(&cpp_dir)
        .flag("-std=c++17");

    if let Some(libs) = qmake_query("QT_INSTALL_LIBS") {
        let p = PathBuf::from(&libs);
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            build.flag(&format!("-F{libs}"));
            build.include(p.join("QtCore.framework/Headers"));
            build.include(p.join("QtGui.framework/Headers"));
            build.include(p.join("QtQuick.framework/Headers"));
            build.include(p.join("QtQml.framework/Headers"));
        }
    }
    if let Some(inc) = qmake_query("QT_INSTALL_HEADERS") {
        let p = PathBuf::from(inc);
        build.include(&p);
        build.include(p.join("QtCore"));
        build.include(p.join("QtGui"));
        build.include(p.join("QtQuick"));
        build.include(p.join("QtQml"));
    }

    build.compile("cs_canvas_item");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=QtQuick");
        println!("cargo:rustc-link-lib=framework=QtQml");
    } else if let Some(libs) = qmake_query("QT_INSTALL_LIBS") {
        println!("cargo:rustc-link-search=native={libs}");
        println!("cargo:rustc-link-lib=Qt6Quick");
    }
}

fn qmake_query(key: &str) -> Option<String> {
    let out = Command::new("qmake").args(["-query", key]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn compile_stamp() -> (String, String) {
    match (date_format("%y%m%d"), date_format("%d-%m-%y")) {
        (Some(rev), Some(date)) => (rev, date),
        _ => ("dev".into(), "dev".into()),
    }
}

fn date_format(fmt: &str) -> Option<String> {
    let out = Command::new("date").arg(format!("+{fmt}")).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn compile_resources() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let qml_dir = manifest.join("../../qml");
    let icons_dir = manifest.join("../../resources/icons");
    let fonts_dir = manifest.join("../../resources/fonts");
    let conf_path = manifest.join("../../resources/qtquickcontrols2.conf");

    println!("cargo:rerun-if-changed={}", qml_dir.display());
    println!("cargo:rerun-if-changed={}", icons_dir.display());
    println!("cargo:rerun-if-changed={}", fonts_dir.display());
    println!("cargo:rerun-if-changed={}", conf_path.display());

    let mut qrc = String::from("<!DOCTYPE RCC>\n<RCC version=\"1.0\">\n");

    // 1. QML files + grid shader
    qrc.push_str("  <qresource prefix=\"/qt/qml/cs_app\">\n");
    let qml_files = [
        "Main.qml",
        "AppWindow.qml",
        "about.qml",
        "aboutqt.qml",
        "appdialog.qml",
        "circuitdialog.qml",
        "AppComboBox.qml",
        "PropDialog.qml",
        "AppToolButton.qml",
        "AppButton.qml",
        "AppIcon.qml",
        "AppCheckBox.qml",
        "AppTabBar.qml",
        "AppTabButton.qml",
        "AppTextField.qml",
        "AppTextArea.qml",
        "AppSpinBox.qml",
        "AppSlider.qml",
        "AppSwitch.qml",
        "AppTextContextMenu.qml",
        "AppScrollBar.qml",
        "CircuitPanel.qml",
        "CircuitView.qml",
        "AppContextMenu.qml",
        "ContextMenuItem.qml",
        "ContextMenuSeparator.qml",
        "ComponentListView.qml",
        "EditorPanel.qml",
        "CodeEditor.qml",
        "FindReplace.qml",
        "FileSettings.qml",
        "CompilerSettings.qml",
        "FileBrowserView.qml",
        "OutPanelText.qml",
        "InfoCard.qml",
        "PlotCanvas.qml",
        "PlotValueControl.qml",
        "OscView.qml",
        "LaView.qml",
        "McuView.qml",
        "serialmon.qml",
        "terminal.qml",
        "MemTablePanel.qml",
        "EditPinDialog.qml",
        "GeneratePinsDialog.qml",
        "CircuitTooltip.qml",
        "CommandCenterDialog.qml",
        "ShortcutDialog.qml",
        "TabSwitcherDialog.qml",
        "AppConfirmDialog.qml",
        "AppListPopup.qml",
        "OverloadPanel.qml",
        "CanvasOverflowPanel.qml",
        "installer.qml",
        "RepaintDebugOverlay.qml",
        "grid.frag.qsb",
    ];
    for f in qml_files {
        let p = qml_dir.join(f);
        println!("cargo:rerun-if-changed={}", p.display());
        qrc.push_str(&format!("    <file alias=\"{f}\">{}</file>\n", p.display()));
    }
    qrc.push_str("  </qresource>\n");

    // 2. Icons
    let src_icons_comp = manifest.join("../../resources/icons/components");
    let src_icons_main = manifest.join("../../resources/icons/mainwindow");
    println!("cargo:rerun-if-changed={}", src_icons_comp.display());
    println!("cargo:rerun-if-changed={}", src_icons_main.display());

    qrc.push_str("  <qresource prefix=\"/icons\">\n");
    if let Ok(entries) = std::fs::read_dir(&icons_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    println!("cargo:rerun-if-changed={}", p.display());
                    qrc.push_str(&format!(
                        "    <file alias=\"{name}\">{}</file>\n",
                        p.display()
                    ));
                }
            }
        }
    }
    qrc.push_str("  </qresource>\n");

    qrc.push_str("  <qresource prefix=\"/icons/components\">\n");
    if let Ok(entries) = std::fs::read_dir(&src_icons_comp) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    println!("cargo:rerun-if-changed={}", p.display());
                    qrc.push_str(&format!(
                        "    <file alias=\"{name}\">{}</file>\n",
                        p.display()
                    ));
                }
            }
        }
    }
    qrc.push_str("  </qresource>\n");

    qrc.push_str("  <qresource prefix=\"/\">\n");
    if conf_path.exists() {
        qrc.push_str(&format!(
            "    <file alias=\"qtquickcontrols2.conf\">{}</file>\n",
            conf_path.display()
        ));
    }
    if let Ok(entries) = std::fs::read_dir(&src_icons_comp) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    qrc.push_str(&format!(
                        "    <file alias=\"{name}\">{}</file>\n",
                        p.display()
                    ));
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(&src_icons_main) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    qrc.push_str(&format!(
                        "    <file alias=\"{name}\">{}</file>\n",
                        p.display()
                    ));
                }
            }
        }
    }
    qrc.push_str("  </qresource>\n");
    if conf_path.exists() {
        qrc.push_str("  <qresource prefix=\"/qt/qml\">\n");
        qrc.push_str(&format!(
            "    <file alias=\"qtquickcontrols2.conf\">{}</file>\n",
            conf_path.display()
        ));
        qrc.push_str("  </qresource>\n");
    }

    // 3. Fonts
    qrc.push_str("  <qresource prefix=\"/fonts\">\n");
    let font_files = [
        "MaterialSymbolsRounded.ttf",
        "Ubuntu-R.ttf",
        "Ubuntu-B.ttf",
        "UbuntuMono-R.ttf",
        "UbuntuMono-RI.ttf",
        "UbuntuMono-B.ttf",
        "UbuntuMono-BI.ttf",
    ];
    for f in font_files {
        let p = fonts_dir.join(f);
        println!("cargo:rerun-if-changed={}", p.display());
        qrc.push_str(&format!("    <file alias=\"{f}\">{}</file>\n", p.display()));
    }
    qrc.push_str("  </qresource>\n");
    qrc.push_str("</RCC>\n");

    let qrc_file = out_dir.join("resources.qrc");
    let rcc_file = out_dir.join("resources.rcc");
    std::fs::write(&qrc_file, qrc).expect("failed to write resources.qrc");

    let rcc = rcc_bin();
    let status = Command::new(&rcc)
        .args(["--binary", "-o"])
        .arg(&rcc_file)
        .arg(&qrc_file)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", rcc.display()));
    if !status.success() {
        panic!("rcc failed on {}", qrc_file.display());
    }
}

fn rcc_bin() -> PathBuf {
    if let Some(p) = qmake_query("QT_HOST_LIBEXECS") {
        let candidate = Path::new(&p).join("rcc");
        if candidate.exists() {
            return candidate;
        }
        let exe = Path::new(&p).join("rcc.exe");
        if exe.exists() {
            return exe;
        }
    }
    if let Some(p) = qmake_query("QT_HOST_BINS") {
        let candidate = Path::new(&p).join("rcc");
        if candidate.exists() {
            return candidate;
        }
        let exe = Path::new(&p).join("rcc.exe");
        if exe.exists() {
            return exe;
        }
    }
    PathBuf::from("rcc")
}
