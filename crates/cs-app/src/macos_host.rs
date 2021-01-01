//! Platform chrome: macOS AppKit (`macos/*.mm`) and Qt theme on other OSes.

#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::sync::Mutex;

#[cfg(target_os = "macos")]
use qtbridge::{QObjectHolder, QmlMethodInvoker};

static SIM_RUNNING: AtomicBool = AtomicBool::new(false);
static SIM_PAUSED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn cs_macos_install_touchbar(
        power: extern "C" fn(),
        pause: extern "C" fn(),
        power_state: extern "C" fn() -> i32,
        pause_state: extern "C" fn() -> i32,
    );
    fn cs_macos_refresh_touchbar();
    fn cs_macos_init_process(name: *const c_char);
    fn cs_macos_setup_window();
    fn cs_macos_set_titlebar_dark(dark: i32);
    fn cs_macos_apply_theme(theme_name: *const c_char) -> i32;
    fn cs_macos_apply_app_icon(path: *const c_char);
    fn cs_macos_set_menu_callbacks(
        trigger: extern "C" fn(*const c_char),
        about_to_show: extern "C" fn(*const c_char),
    );
    fn cs_macos_set_menu(json: *const c_char);
    fn cs_macos_beep();
}

#[cfg(not(target_os = "macos"))]
unsafe extern "C" {
    fn cs_apply_theme(theme_name: *const c_char) -> i32;
    fn cs_set_titlebar_dark(dark: i32);
    fn cs_apply_app_icon();
}

pub fn setup_window() {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_setup_window();
    }
}

pub fn beep() {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_beep();
    }
    #[cfg(not(target_os = "macos"))]
    {
        print!("\x07");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
}

pub fn init_process(name: &str) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(c) = CString::new(name) {
            unsafe { cs_macos_init_process(c.as_ptr()) };
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = name;
    }
}

pub fn apply_theme(theme_name: &str) -> bool {
    let Ok(c) = CString::new(theme_name) else {
        return false;
    };
    #[cfg(target_os = "macos")]
    {
        let res = unsafe { cs_macos_apply_theme(c.as_ptr()) };
        return res != 0;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let res = unsafe { cs_apply_theme(c.as_ptr()) };
        res != 0
    }
}

#[cfg(target_os = "macos")]
static TOUCH_INVOKER: Mutex<Option<QmlMethodInvoker>> = Mutex::new(None);
#[cfg(target_os = "macos")]
static MENU_INVOKER: Mutex<Option<QmlMethodInvoker>> = Mutex::new(None);

pub fn set_sim_state(running: bool, paused: bool) {
    SIM_RUNNING.store(running, Ordering::SeqCst);
    SIM_PAUSED.store(paused, Ordering::SeqCst);
    refresh_touchbar();
}

#[cfg(target_os = "macos")]
extern "C" fn on_power() {
    if let Ok(g) = TOUCH_INVOKER.lock() {
        if let Some(inv) = g.as_ref() {
            let _ = inv.invoke_method("powerCirc");
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn on_pause() {
    if let Ok(g) = TOUCH_INVOKER.lock() {
        if let Some(inv) = g.as_ref() {
            let _ = inv.invoke_method("pauseCirc");
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn power_state() -> i32 {
    i32::from(SIM_RUNNING.load(Ordering::SeqCst))
}

#[cfg(target_os = "macos")]
extern "C" fn pause_state() -> i32 {
    if !SIM_RUNNING.load(Ordering::SeqCst) {
        -1
    } else if SIM_PAUSED.load(Ordering::SeqCst) {
        1
    } else {
        0
    }
}

#[cfg(target_os = "macos")]
extern "C" fn on_menu_trigger(path: *const c_char) {
    if path.is_null() {
        return;
    }
    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    if let Ok(g) = MENU_INVOKER.lock() {
        if let Some(inv) = g.as_ref() {
            qtbridge::invoke_method!(inv, "trigger", path);
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn on_menu_about_to_show(path: *const c_char) {
    if path.is_null() {
        return;
    }
    let path = unsafe { CStr::from_ptr(path) }
        .to_string_lossy()
        .into_owned();
    if let Ok(g) = MENU_INVOKER.lock() {
        if let Some(inv) = g.as_ref() {
            qtbridge::invoke_method!(inv, "aboutToShow", path);
        }
    }
}

#[cfg(target_os = "macos")]
pub fn install_touchbar<T: QObjectHolder>(obj: &T) {
    let inv = obj.get_qml_method_invoker();
    *TOUCH_INVOKER.lock().unwrap() = Some(inv);
    unsafe {
        cs_macos_install_touchbar(on_power, on_pause, power_state, pause_state);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_touchbar<T>(_obj: &T) {}

pub fn refresh_touchbar() {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_refresh_touchbar();
    }
}

pub fn set_titlebar_dark(dark: bool) {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_set_titlebar_dark(i32::from(dark));
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        cs_set_titlebar_dark(i32::from(dark));
    }
}

pub fn apply_app_icon() {
    #[cfg(target_os = "macos")]
    {
        let path = option_env!("CS_ICON_PATH").unwrap_or("");
        if let Ok(c) = CString::new(path) {
            unsafe { cs_macos_apply_app_icon(c.as_ptr()) };
        }
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        cs_apply_app_icon();
    }
}

#[cfg(target_os = "macos")]
pub fn install_native_menu<T: QObjectHolder>(obj: &T) {
    let inv = obj.get_qml_method_invoker();
    *MENU_INVOKER.lock().unwrap() = Some(inv);
    unsafe {
        cs_macos_set_menu_callbacks(on_menu_trigger, on_menu_about_to_show);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_native_menu<T>(_obj: &T) {}

pub fn set_native_menu_json(_json: &str) {
    #[cfg(target_os = "macos")]
    if let Ok(c) = CString::new(_json) {
        unsafe { cs_macos_set_menu(c.as_ptr()) };
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    #[test]
    fn apply_theme_resolves_without_qapp() {
        assert!(super::apply_theme("Dark"));
        assert!(!super::apply_theme("Light"));
        assert!(!super::apply_theme("System"));
    }
}
