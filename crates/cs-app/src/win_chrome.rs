//! Windows custom window chrome, DWM dark mode, auto-hide taskbar clearance,
//! and system dark-theme registry query.
//!
//! `cpp/theme.cpp` forwards Qt's `QAbstractNativeEventFilter` messages and
//! provides `cs_win_hit_test_titlebar` for walking the `QQuickItem` tree.

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::mem;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{BOOL, FALSE, HWND, POINT, RECT, TRUE};
use windows::Win32::Graphics::Dwm::{
    DWMWINDOWATTRIBUTE, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect,
    MonitorFromWindow, ScreenToClient,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{
    ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_GETTASKBARPOS, ABS_AUTOHIDE,
    APPBARDATA, SHAppBarMessage,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GWL_STYLE, GetClientRect, GetWindowLongPtrW, GetWindowRect,
    HTCAPTION, HTCLIENT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS,
    SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SM_CYSIZEFRAME, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos,
    WM_DISPLAYCHANGE, WM_NCCALCSIZE, WM_NCHITTEST, WM_SETTINGCHANGE, WS_CAPTION, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
};
use windows::core::{PCWSTR, w};

unsafe extern "C" {
    fn cs_win_hit_test_titlebar(
        hwnd: usize,
        client_x: i32,
        client_y: i32,
        out_over_title: *mut i32,
        out_over_control: *mut i32,
    ) -> i32;
}

const MAX_AUTOHIDE_MONITORS: usize = 8;

#[derive(Clone, Copy)]
struct AutoHideClearance {
    monitor: isize,
    edge: u32,
}

static CHROME_APPLIED: Mutex<Option<HashSet<usize>>> = Mutex::new(None);
static DWM_DARK_APPLIED: Mutex<Option<HashMap<usize, bool>>> = Mutex::new(None);
static AUTOHIDE: Mutex<Vec<AutoHideClearance>> = Mutex::new(Vec::new());
static REFRESHING_AUTOHIDE: AtomicBool = AtomicBool::new(false);

fn hwnd_from_usize(raw: usize) -> HWND {
    HWND(raw as *mut c_void)
}

fn frame_metrics(hwnd: HWND) -> (i32, i32) {
    unsafe {
        let mut dpi = GetDpiForWindow(hwnd);
        if dpi == 0 {
            dpi = 96;
        }
        let frame_x = GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
            + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
        let frame_y = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
            + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
        (frame_x, frame_y)
    }
}

fn tray_edge(tray: &RECT, monitor: &RECT) -> u32 {
    let width = tray.right - tray.left;
    let height = tray.bottom - tray.top;
    let left_gap = tray.left - monitor.left;
    let right_gap = monitor.right - tray.right;
    let top_gap = tray.top - monitor.top;
    let bottom_gap = monitor.bottom - tray.bottom;
    if width < height {
        if left_gap <= right_gap {
            ABE_LEFT
        } else {
            ABE_RIGHT
        }
    } else if top_gap <= bottom_gap {
        ABE_TOP
    } else {
        ABE_BOTTOM
    }
}

fn remember_autohide(list: &mut Vec<AutoHideClearance>, monitor: HMONITOR, edge: u32) {
    if monitor.0.is_null() || list.len() >= MAX_AUTOHIDE_MONITORS {
        return;
    }
    if !matches!(edge, ABE_LEFT | ABE_TOP | ABE_RIGHT | ABE_BOTTOM) {
        return;
    }
    let key = monitor.0 as isize;
    if list.iter().any(|entry| entry.monitor == key) {
        return;
    }
    list.push(AutoHideClearance { monitor: key, edge });
}

fn refresh_autohide_clearance() {
    if REFRESHING_AUTOHIDE.swap(true, Ordering::AcqRel) {
        return;
    }

    let mut next = Vec::with_capacity(MAX_AUTOHIDE_MONITORS);
    unsafe {
        let mut abd = APPBARDATA {
            cbSize: mem::size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };
        if (SHAppBarMessage(ABM_GETSTATE, &mut abd) & (ABS_AUTOHIDE as usize)) != 0 {
            let mut pos = APPBARDATA {
                cbSize: mem::size_of::<APPBARDATA>() as u32,
                ..Default::default()
            };
            if SHAppBarMessage(ABM_GETTASKBARPOS, &mut pos) != 0 {
                let mut mon = MonitorFromRect(&pos.rc, MONITOR_DEFAULTTONEAREST);
                if mon.0.is_null() {
                    if let Ok(primary) = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) {
                        if !primary.0.is_null() {
                            mon = MonitorFromWindow(primary, MONITOR_DEFAULTTONEAREST);
                        }
                    }
                }
                remember_autohide(&mut next, mon, pos.uEdge);
            }

            let mut tray = HWND::default();
            while let Ok(found) = FindWindowExW(
                HWND::default(),
                tray,
                w!("Shell_SecondaryTrayWnd"),
                PCWSTR::null(),
            ) {
                if found.0.is_null() {
                    break;
                }
                tray = found;
                let mut tray_rect = RECT::default();
                if GetWindowRect(tray, &mut tray_rect).is_err() {
                    continue;
                }
                let mon = MonitorFromRect(&tray_rect, MONITOR_DEFAULTTONEAREST);
                if mon.0.is_null() {
                    continue;
                }
                let mut mi = MONITORINFO {
                    cbSize: mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if !GetMonitorInfoW(mon, &mut mi).as_bool() {
                    continue;
                }
                remember_autohide(&mut next, mon, tray_edge(&tray_rect, &mi.rcMonitor));
            }
        }
    }

    if let Ok(mut list) = AUTOHIDE.lock() {
        *list = next;
    }
    REFRESHING_AUTOHIDE.store(false, Ordering::Release);
}

fn inset_autohide_edge(hwnd: HWND, rc: &mut RECT) {
    let mon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if mon.0.is_null() {
        return;
    }
    let key = mon.0 as isize;
    let Ok(list) = AUTOHIDE.lock() else {
        return;
    };
    for entry in list.iter() {
        if entry.monitor != key {
            continue;
        }
        match entry.edge {
            ABE_LEFT => rc.left += 1,
            ABE_TOP => rc.top += 1,
            ABE_RIGHT => rc.right -= 1,
            ABE_BOTTOM => rc.bottom -= 1,
            _ => {}
        }
        return;
    }
}

pub fn apps_use_dark() -> bool {
    let mut value: u32 = 1;
    let mut size: u32 = mem::size_of::<u32>() as u32;
    let err = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    err.is_ok() && value == 0
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_win_register_chrome(raw_hwnd: usize) {
    if raw_hwnd == 0 {
        return;
    }
    let newly_inserted = {
        let mut guard = CHROME_APPLIED.lock().unwrap_or_else(|e| e.into_inner());
        guard.get_or_insert_with(HashSet::new).insert(raw_hwnd)
    };
    if !newly_inserted {
        return;
    }
    let hwnd = hwnd_from_usize(raw_hwnd);
    unsafe {
        // Keep WS_CAPTION + WS_THICKFRAME so DWM preserves rounded corners,
        // drop shadow, Aero Snap, and Alt+Space system menu; WM_NCCALCSIZE
        // removes only the top native titlebar strip.
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let desired = style
            | (WS_CAPTION.0 | WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0)
                as isize;
        if desired != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, desired);
        }

        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        // Before SWP_FRAMECHANGED, which sends WM_NCCALCSIZE synchronously.
        refresh_autohide_clearance();
        let _ = SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_win_unregister_chrome(raw_hwnd: usize) {
    if let Ok(mut guard) = CHROME_APPLIED.lock() {
        if let Some(set) = guard.as_mut() {
            set.remove(&raw_hwnd);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_win_apply_dwm(raw_hwnd: usize, dark: i32, refresh_frame: i32) {
    if raw_hwnd == 0 {
        return;
    }
    let want_dark = dark != 0;
    {
        let mut guard = DWM_DARK_APPLIED.lock().unwrap_or_else(|e| e.into_inner());
        let map = guard.get_or_insert_with(HashMap::new);
        if map.get(&raw_hwnd).copied() == Some(want_dark) {
            return;
        }
        map.insert(raw_hwnd, want_dark);
    }
    let hwnd = hwnd_from_usize(raw_hwnd);
    let value: BOOL = if want_dark { TRUE } else { FALSE };
    let val_ptr = (&value as *const BOOL).cast::<c_void>();
    let val_size = mem::size_of::<BOOL>() as u32;
    unsafe {
        // 20 = DWMWA_USE_IMMERSIVE_DARK_MODE (Win10 1903+); 19 is the older constant.
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(20), val_ptr, val_size);
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(19), val_ptr, val_size);
        if refresh_frame != 0 {
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER
                    | SWP_FRAMECHANGED,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cs_win_forget_window(raw_hwnd: usize) {
    if raw_hwnd == 0 {
        return;
    }
    if let Ok(mut guard) = DWM_DARK_APPLIED.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(&raw_hwnd);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cs_win_native_event(
    raw_hwnd: usize,
    message: u32,
    w_param: usize,
    l_param: isize,
    out_result: *mut isize,
) -> i32 {
    if raw_hwnd == 0 || out_result.is_null() {
        return 0;
    }
    let hwnd = hwnd_from_usize(raw_hwnd);

    if message == WM_SETTINGCHANGE || message == WM_DISPLAYCHANGE {
        refresh_autohide_clearance();
        return 0;
    }

    if message == WM_NCCALCSIZE && w_param != 0 {
        let params = unsafe { &mut *(l_param as *mut NCCALCSIZE_PARAMS) };
        let (frame_x, frame_y) = frame_metrics(hwnd);
        if unsafe { IsZoomed(hwnd) }.as_bool() {
            params.rgrc[0].left += frame_x;
            params.rgrc[0].top += frame_y;
            params.rgrc[0].right -= frame_x;
            params.rgrc[0].bottom -= frame_y;
            // One pixel on the auto-hide edge lets that edge reveal the bar.
            // A monitor with no taskbar is absent from the cache, so it stays flush.
            inset_autohide_edge(hwnd, &mut params.rgrc[0]);
        } else {
            // Eliminate top titlebar while preserving invisible left/right/bottom resize borders.
            params.rgrc[0].left += frame_x;
            params.rgrc[0].right -= frame_x;
            params.rgrc[0].bottom -= frame_y;
        }
        unsafe { *out_result = 0 };
        return 1;
    }

    if message == WM_NCHITTEST {
        let mut pt = POINT {
            x: (l_param as i16) as i32,
            y: ((l_param >> 16) as i16) as i32,
        };
        let mut cr = RECT::default();
        unsafe {
            let _ = ScreenToClient(hwnd, &mut pt);
            let _ = GetClientRect(hwnd, &mut cr);
        }
        // Let DefWindowProc handle left/right/bottom non-client resize borders.
        if pt.x < 0 || pt.x >= cr.right || pt.y < 0 || pt.y >= cr.bottom {
            return 0;
        }

        let mut over_title = 0i32;
        let mut over_control = 0i32;
        unsafe {
            cs_win_hit_test_titlebar(raw_hwnd, pt.x, pt.y, &mut over_title, &mut over_control);
        }

        if !unsafe { IsZoomed(hwnd) }.as_bool() {
            let (frame_x, frame_y) = frame_metrics(hwnd);
            // Restored top frame is client area because NCCALCSIZE drops the
            // native caption. Same thickness as the other edges; buttons stay clickable.
            if pt.y < frame_y && over_control == 0 {
                let hit = if pt.x < frame_x {
                    HTTOPLEFT
                } else if pt.x >= cr.right - frame_x {
                    HTTOPRIGHT
                } else {
                    HTTOP
                };
                unsafe { *out_result = hit as isize };
                return 1;
            }
        }

        if over_title != 0 {
            let hit = if over_control != 0 {
                HTCLIENT
            } else {
                HTCAPTION
            };
            unsafe { *out_result = hit as isize };
            return 1;
        }
    }

    0
}
