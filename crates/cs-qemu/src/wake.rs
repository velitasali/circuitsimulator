//! Named auto-reset events that wake the co-sim mailbox.
//!
//! The simulator creates both events before it spawns QEMU. The guest opens
//! them by the same names (`Local\cs-req-<shm key>` and `Local\cs-ack-<shm key>`,
//! leading slash stripped). A missing event is not fatal: the waiter sleeps a
//! short slice and the caller rechecks the mailbox flag, which is what an
//! older QEMU binary needs.

#[cfg(windows)]
use crate::Error;
use crate::Result;
#[cfg(windows)]
use std::ffi::CString;

/// Request and ack handles for one shared-memory arena.
pub struct CosimWake {
    #[cfg(windows)]
    req: winapi::Handle,
    #[cfg(windows)]
    ack: winapi::Handle,
}

unsafe impl Send for CosimWake {}

impl CosimWake {
    /// Create the pair. The name is the shm key (`"/"` + pid + id).
    pub fn create(shm_key: &str) -> Result<Self> {
        #[cfg(windows)]
        {
            return win_create(shm_key);
        }
        #[cfg(not(windows))]
        {
            let _ = shm_key;
            Ok(Self::sleeping())
        }
    }

    /// Waiter that only sleeps. Used when the OS events could not be created,
    /// and on hosts that have no named-event pair.
    pub fn sleeping() -> Self {
        Self {
            #[cfg(windows)]
            req: std::ptr::null_mut(),
            #[cfg(windows)]
            ack: std::ptr::null_mut(),
        }
    }

    /// Guest posted a mailbox request. Auto-reset, so a wake with nobody
    /// waiting is consumed by the next wait.
    pub fn signal_request(&self) {
        #[cfg(windows)]
        if !self.req.is_null() {
            unsafe {
                winapi::SetEvent(self.req);
            }
        }
    }

    /// Simulator finished a mailbox slot (read response or cleared write).
    pub fn signal_ack(&self) {
        #[cfg(windows)]
        if !self.ack.is_null() {
            unsafe {
                winapi::SetEvent(self.ack);
            }
        }
    }

    /// Block until the guest signals a request, or until `timeout_ms` elapses.
    ///
    /// Returns whether the event fired. `false` means the caller must read
    /// the mailbox anyway: the guest may be an older binary that never
    /// signals, and a lost auto-reset wake shows up as a later flag change.
    /// Without an OS event this sleeps at most 1 ms so the mailbox is polled
    /// instead of busy-spinning.
    pub fn wait_request(&self, timeout_ms: u32) -> bool {
        #[cfg(windows)]
        {
            if self.req.is_null() {
                sleep_slice(timeout_ms);
                return false;
            }
            let ms = timeout_ms.max(1);
            return unsafe { winapi::WaitForSingleObject(self.req, ms) == winapi::WAIT_OBJECT_0 };
        }
        #[cfg(not(windows))]
        {
            sleep_slice(timeout_ms);
            false
        }
    }
}

impl Drop for CosimWake {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            if !self.req.is_null() {
                winapi::CloseHandle(self.req);
                self.req = std::ptr::null_mut();
            }
            if !self.ack.is_null() {
                winapi::CloseHandle(self.ack);
                self.ack = std::ptr::null_mut();
            }
        }
    }
}

fn sleep_slice(timeout_ms: u32) {
    let ms = timeout_ms.min(1);
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(u64::from(ms)));
    }
}

/// `Local\cs-req-<body>` and `Local\cs-ack-<body>`. `body` is the shm key
/// without its leading slash. The guest formats the same strings.
pub fn cosim_event_names(shm_key: &str) -> (String, String) {
    let body = shm_key.strip_prefix('/').unwrap_or(shm_key);
    (
        format!("Local\\cs-req-{body}"),
        format!("Local\\cs-ack-{body}"),
    )
}

#[cfg(windows)]
fn win_create(shm_key: &str) -> Result<CosimWake> {
    let (req_name, ack_name) = cosim_event_names(shm_key);
    let req = create_event(&req_name)?;
    let ack = match create_event(&ack_name) {
        Ok(h) => h,
        Err(e) => {
            unsafe {
                winapi::CloseHandle(req);
            }
            return Err(e);
        }
    };
    Ok(CosimWake { req, ack })
}

#[cfg(windows)]
fn create_event(name: &str) -> Result<winapi::Handle> {
    let cname = CString::new(name).map_err(|_| Error::Shm("event name contains NUL".into()))?;
    let handle = unsafe {
        winapi::CreateEventA(
            std::ptr::null_mut(),
            0, // auto-reset
            0, // start unsignaled
            cname.as_ptr(),
        )
    };
    if handle.is_null() {
        return Err(Error::Shm(format!(
            "CreateEvent({name}) failed: {}",
            unsafe { winapi::GetLastError() }
        )));
    }
    Ok(handle)
}

#[cfg(windows)]
mod winapi {
    use std::ffi::c_void;

    pub type Handle = *mut c_void;

    pub const WAIT_OBJECT_0: u32 = 0;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn CreateEventA(
            attrs: *mut c_void,
            manual_reset: i32,
            initial_state: i32,
            name: *const i8,
        ) -> Handle;
        pub fn SetEvent(event: Handle) -> i32;
        pub fn WaitForSingleObject(event: Handle, millis: u32) -> u32;
        pub fn CloseHandle(handle: Handle) -> i32;
        pub fn GetLastError() -> u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_names_strip_the_leading_slash() {
        let (req, ack) = cosim_event_names("/12345Esp32-1");
        assert_eq!(req, "Local\\cs-req-12345Esp32-1");
        assert_eq!(ack, "Local\\cs-ack-12345Esp32-1");
    }

    #[cfg(windows)]
    #[test]
    fn request_event_wakes_the_waiter() {
        let key = format!("/cs-wake-{}", std::process::id());
        let wake = CosimWake::create(&key).expect("create events");
        assert!(!wake.wait_request(1));
        wake.signal_request();
        assert!(wake.wait_request(1_000));
        wake.signal_ack();
    }

    #[test]
    fn sleeping_wake_does_not_report_a_signal() {
        let wake = CosimWake::sleeping();
        assert!(!wake.wait_request(0));
    }
}
