//! Platform chrome: macOS AppKit (`macos/*.mm`) and Qt theme on other OSes.

#![allow(dead_code)]

#[cfg(target_os = "macos")]
use std::ffi::CString;
#[cfg(not(target_os = "macos"))]
use std::ffi::c_void;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;
#[cfg(not(target_os = "macos"))]
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

#[cfg(target_os = "macos")]
use std::ffi::CStr;

#[cfg(windows)]
use crate::win_chrome;
use cs_engine::settings;
use cs_engine::theme::{ColorTheme, QtPaletteColors};
use qtbridge::{QObjectHolder, QmlMethodInvoker};

static SIM_RUNNING: AtomicBool = AtomicBool::new(false);
static SIM_PAUSED: AtomicBool = AtomicBool::new(false);
static CURRENT_THEME_DARK: AtomicI32 = AtomicI32::new(-1);
static IN_APPLY_THEME: AtomicBool = AtomicBool::new(false);
static THEME_APPLY_PENDING: AtomicBool = AtomicBool::new(false);
static APP_THEME_INVOKER: Mutex<Option<QmlMethodInvoker>> = Mutex::new(None);

unsafe extern "C" {
    fn cs_qt_query_system_dark(unset_scheme: i32) -> i32;
    fn cs_qt_apply_palette(scheme_mode: i32, pal: *const QtPaletteColors);
    fn cs_qt_post_on_main(cb: extern "C" fn());
    fn cs_qt_move_to_trash(path_utf8: *const u8, len: usize) -> i32;
}

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
    fn cs_macos_titlebar_mouse_down();
    fn cs_macos_titlebar_leading_inset() -> i32;
    fn cs_macos_system_dark() -> i32;
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
    fn cs_set_titlebar_dark(dark: i32);
    fn cs_apply_app_icon();
    fn cs_ensure_theme_filter();
    fn cs_install_alt_menu_filter(on_alt: extern "C" fn(i32) -> i32);
    fn cs_invoke_direct(obj: *mut c_void, method: *const c_char) -> i32;
}

#[cfg(not(target_os = "macos"))]
static ALT_MENU_OPEN: AtomicBool = AtomicBool::new(false);
#[cfg(not(target_os = "macos"))]
static ALT_TOGGLED: AtomicBool = AtomicBool::new(false);
#[cfg(not(target_os = "macos"))]
static MENU_OBJECT: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn setup_window() {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_setup_window();
    }
}

pub fn titlebar_mouse_down() {
    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_titlebar_mouse_down();
    }
}

pub fn titlebar_leading_inset() -> i32 {
    #[cfg(target_os = "macos")]
    unsafe {
        return cs_macos_titlebar_leading_inset();
    }
    #[cfg(not(target_os = "macos"))]
    0
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

fn resolve_system_dark(qt_dark: i32) -> bool {
    match qt_dark {
        1 => true,
        0 => false,
        -2 => {
            #[cfg(target_os = "macos")]
            {
                unsafe { cs_macos_system_dark() != 0 }
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        }
        _ => {
            #[cfg(windows)]
            {
                win_chrome::apps_use_dark()
            }
            #[cfg(target_os = "macos")]
            {
                unsafe { cs_macos_system_dark() != 0 }
            }
            #[cfg(not(any(windows, target_os = "macos")))]
            {
                false
            }
        }
    }
}

pub fn register_theme_sync<T: QObjectHolder>(obj: &T) {
    if let Ok(mut guard) = APP_THEME_INVOKER.lock() {
        if guard.is_none() {
            *guard = Some(obj.get_qml_method_invoker());
        }
    }
}

/// Returns the last applied dark-theme flag without mutating Qt state.
pub fn current_theme_dark() -> bool {
    let cached = CURRENT_THEME_DARK.load(Ordering::Acquire);
    if cached >= 0 {
        return cached != 0;
    }
    let theme_name = settings::get().theme;
    if theme_name == "Dark" {
        true
    } else if theme_name == "Light" {
        false
    } else {
        let qt_dark = unsafe { cs_qt_query_system_dark(0) };
        resolve_system_dark(qt_dark)
    }
}

pub fn post_on_main(cb: extern "C" fn()) {
    unsafe {
        cs_qt_post_on_main(cb);
    }
}

/// Schedules theme application onto the Qt main event loop so that
/// `QStyleHints::setColorScheme`/`unsetColorScheme`, `QGuiApplication::setPalette`,
/// and native titlebar updates never execute synchronously while a `qtbridge`
/// `#[qobject]` `RefCell` is borrowed on the call stack.
pub fn schedule_apply_theme() {
    if IN_APPLY_THEME.load(Ordering::Acquire) {
        return;
    }
    if THEME_APPLY_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    unsafe {
        cs_qt_post_on_main(on_scheduled_theme_apply);
    }
}

extern "C" fn on_scheduled_theme_apply() {
    THEME_APPLY_PENDING.store(false, Ordering::Release);
    let s = settings::get();
    let _ = apply_theme(&s.theme);
    if let Ok(guard) = APP_THEME_INVOKER.lock() {
        if let Some(inv) = guard.as_ref() {
            let _ = inv.invoke_method("syncTheme");
        }
    }
}

pub fn apply_theme(theme_name: &str) -> bool {
    if IN_APPLY_THEME.swap(true, Ordering::AcqRel) {
        return CURRENT_THEME_DARK.load(Ordering::Acquire) > 0;
    }

    let named_dark = theme_name == "Dark";
    let named_light = theme_name == "Light";
    let named_system = theme_name.is_empty() || theme_name == "System";

    let qt_dark = unsafe { cs_qt_query_system_dark(i32::from(named_system)) };
    if qt_dark == -2 {
        // No QGuiApplication instance yet (e.g. unit tests).
        let use_dark = if named_dark {
            true
        } else if named_light {
            false
        } else {
            resolve_system_dark(qt_dark)
        };

        IN_APPLY_THEME.store(false, Ordering::Release);
        return use_dark;
    }

    let is_system_dark = resolve_system_dark(qt_dark);

    let use_dark = if named_dark {
        true
    } else if named_light {
        false
    } else {
        is_system_dark
    };

    // Publish before setColorScheme: that call can re-enter via QML's
    // colorSchemeChanged -> App.reloadTheme().
    CURRENT_THEME_DARK.store(i32::from(use_dark), Ordering::Release);

    let scheme_mode = if named_system {
        0
    } else if use_dark {
        2
    } else {
        1
    };
    let pal = ColorTheme::qt_palette(use_dark);
    unsafe {
        cs_qt_apply_palette(scheme_mode, &pal);
    }

    #[cfg(target_os = "macos")]
    unsafe {
        cs_macos_set_titlebar_dark(i32::from(use_dark));
    }
    #[cfg(windows)]
    unsafe {
        cs_set_titlebar_dark(i32::from(use_dark));
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    unsafe {
        cs_ensure_theme_filter();
    }

    IN_APPLY_THEME.store(false, Ordering::Release);
    use_dark
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
    if let Ok(mut guard) = MENU_INVOKER.lock() {
        *guard = Some(inv);
    }
    unsafe {
        cs_macos_set_menu_callbacks(on_menu_trigger, on_menu_about_to_show);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_alt_menu_open(open: bool) {
    ALT_MENU_OPEN.store(open, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
pub fn set_alt_menu_open(_open: bool) {}

#[cfg(not(target_os = "macos"))]
pub fn note_alt_toggled(toggled: bool) {
    ALT_TOGGLED.store(toggled, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
pub fn note_alt_toggled(_toggled: bool) {}

#[cfg(not(target_os = "macos"))]
extern "C" fn on_alt(phase: i32) -> i32 {
    if phase == 0 {
        return i32::from(ALT_MENU_OPEN.load(Ordering::SeqCst));
    }
    let obj = MENU_OBJECT.load(Ordering::SeqCst);
    if obj.is_null() {
        return 0;
    }
    ALT_TOGGLED.store(false, Ordering::SeqCst);
    let ok = unsafe { cs_invoke_direct(obj, c"requestTogglePopup".as_ptr()) } != 0;
    i32::from(ok && ALT_TOGGLED.load(Ordering::SeqCst))
}

#[cfg(not(target_os = "macos"))]
pub fn install_native_menu<T: QObjectHolder>(obj: &T) {
    MENU_OBJECT.store(obj.get_qobject_ptr().cast(), Ordering::SeqCst);
    unsafe {
        cs_install_alt_menu_filter(on_alt);
    }
}

pub fn set_native_menu_json(_json: &str) {
    #[cfg(target_os = "macos")]
    if let Ok(c) = CString::new(_json) {
        unsafe { cs_macos_set_menu(c.as_ptr()) };
    }
}

pub fn move_to_trash(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.is_empty() {
        return false;
    }
    unsafe { cs_qt_move_to_trash(s.as_ptr(), s.len()) != 0 }
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
