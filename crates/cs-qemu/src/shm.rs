//! Shared-memory arena: POSIX `shm_open` on Unix, `CreateFileMappingA` on Windows.
//!
//! The name is the same `shm_key()` string on both sides (`"/" + pid + id`),
//! matching C++ `QemuDevice`.

use crate::arena::{ARENA_SIZE, Arena};
use crate::{Error, Result};
use std::ffi::CString;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

/// One coherent view of the mailbox. `simu_action` is loaded first with
/// acquire so the payload written before the guest's release store is visible.
#[derive(Clone, Copy, Debug, Default)]
pub struct MailboxView {
    pub simu_action: u64,
    pub simu_time: u64,
    pub qemu_time: u64,
    pub reg_addr: u64,
    pub reg_data: u64,
    pub running: u64,
    pub qemu_action: u64,
}

pub struct SharedArena {
    key: CString,
    #[cfg(unix)]
    fd: libc::c_int,
    #[cfg(windows)]
    handle: *mut std::ffi::c_void,
    ptr: *mut Arena,
    /// Unix: `create` unlinks the name; `open` leaves it for the owner.
    /// Windows: named objects vanish when the last handle closes.
    unlink_on_drop: bool,
}

// The mapping is process-local; the QEMU child attaches by name.
unsafe impl Send for SharedArena {}

impl SharedArena {
    pub fn create(key: &str) -> Result<Self> {
        let ckey = CString::new(key).map_err(|_| Error::Shm("shm key contains NUL".into()))?;
        #[cfg(unix)]
        {
            return unix_create(ckey);
        }
        #[cfg(windows)]
        {
            return win_create(ckey, true);
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = ckey;
            Err(Error::Unsupported(
                "shared-memory arena is unix/windows only",
            ))
        }
    }

    /// Attach to an existing mapping (the QEMU child path).
    pub fn open(key: &str) -> Result<Self> {
        let ckey = CString::new(key).map_err(|_| Error::Shm("shm key contains NUL".into()))?;
        #[cfg(unix)]
        {
            return unix_open(ckey);
        }
        #[cfg(windows)]
        {
            return win_open(ckey);
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = ckey;
            Err(Error::Unsupported(
                "shared-memory arena is unix/windows only",
            ))
        }
    }

    pub fn key(&self) -> &str {
        self.key.to_str().unwrap_or("")
    }

    pub fn as_ref(&self) -> &Arena {
        unsafe { &*self.ptr }
    }

    pub fn as_mut(&mut self) -> &mut Arena {
        unsafe { &mut *self.ptr }
    }

    pub fn load_simu_action(&self) -> u64 {
        unsafe { load_u64(ptr::addr_of!((*self.ptr).simu_action), Ordering::Acquire) }
    }

    pub fn load_qemu_time(&self) -> u64 {
        unsafe { load_u64(ptr::addr_of!((*self.ptr).qemu_time), Ordering::Relaxed) }
    }

    pub fn load_running(&self) -> u64 {
        unsafe { load_u64(ptr::addr_of!((*self.ptr).running), Ordering::Acquire) }
    }

    pub fn load_mailbox(&self) -> MailboxView {
        unsafe {
            let p = self.ptr;
            // Action first: it is the release doorbell for the payload.
            let simu_action = load_u64(ptr::addr_of!((*p).simu_action), Ordering::Acquire);
            MailboxView {
                simu_action,
                simu_time: load_u64(ptr::addr_of!((*p).simu_time), Ordering::Relaxed),
                qemu_time: load_u64(ptr::addr_of!((*p).qemu_time), Ordering::Relaxed),
                reg_addr: load_u64(ptr::addr_of!((*p).reg_addr), Ordering::Relaxed),
                reg_data: load_u64(ptr::addr_of!((*p).reg_data), Ordering::Relaxed),
                running: load_u64(ptr::addr_of!((*p).running), Ordering::Relaxed),
                qemu_action: load_u64(ptr::addr_of!((*p).qemu_action), Ordering::Acquire),
            }
        }
    }

    /// Publish the simulator's answer. `simu_action` is stored last with
    /// release so the guest cannot observe a cleared doorbell with a stale
    /// `qemu_action` or `reg_data`.
    pub fn store_response(&mut self, arena: &Arena) {
        unsafe {
            let p = self.ptr;
            store_u64(
                ptr::addr_of_mut!((*p).reg_data),
                arena.reg_data,
                Ordering::Relaxed,
            );
            store_u64(
                ptr::addr_of_mut!((*p).reg_addr),
                arena.reg_addr,
                Ordering::Relaxed,
            );
            store_u64(
                ptr::addr_of_mut!((*p).irq_number),
                arena.irq_number,
                Ordering::Relaxed,
            );
            store_u64(
                ptr::addr_of_mut!((*p).irq_level),
                arena.irq_level,
                Ordering::Relaxed,
            );
            store_u64(
                ptr::addr_of_mut!((*p).simu_time),
                arena.simu_time,
                Ordering::Relaxed,
            );
            store_u64(
                ptr::addr_of_mut!((*p).qemu_action),
                arena.qemu_action,
                Ordering::Release,
            );
            store_u64(
                ptr::addr_of_mut!((*p).simu_action),
                arena.simu_action,
                Ordering::Release,
            );
        }
    }
}

/// Mailbox words are plain `u64`s in the shared mapping. The guest uses the
/// same atomic accesses, so the host must not mix in non-atomic reads.
unsafe fn load_u64(p: *const u64, order: Ordering) -> u64 {
    unsafe { (&*p.cast::<AtomicU64>()).load(order) }
}

unsafe fn store_u64(p: *mut u64, val: u64, order: Ordering) {
    unsafe { (&*p.cast::<AtomicU64>()).store(val, order) }
}

#[cfg(unix)]
fn unix_create(ckey: CString) -> Result<SharedArena> {
    let key = ckey.to_str().unwrap_or("").to_string();
    unsafe {
        libc::shm_unlink(ckey.as_ptr());
    }
    let fd = unsafe { libc::shm_open(ckey.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o666) };
    if fd < 0 {
        return Err(Error::Shm(format!(
            "shm_open({key}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    if unsafe { libc::ftruncate(fd, ARENA_SIZE as libc::off_t) } != 0 {
        let err = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
            libc::shm_unlink(ckey.as_ptr());
        }
        return Err(Error::Shm(format!("ftruncate({key}) failed: {err}")));
    }
    let map = unsafe {
        libc::mmap(
            ptr::null_mut(),
            ARENA_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if map == libc::MAP_FAILED {
        let err = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
            libc::shm_unlink(ckey.as_ptr());
        }
        return Err(Error::Shm(format!("mmap({key}) failed: {err}")));
    }
    let ptr = map as *mut Arena;
    unsafe { ptr.write(Arena::default()) };
    Ok(SharedArena {
        key: ckey,
        fd,
        ptr,
        unlink_on_drop: true,
    })
}

#[cfg(unix)]
fn unix_open(ckey: CString) -> Result<SharedArena> {
    let key = ckey.to_str().unwrap_or("").to_string();
    let fd = unsafe { libc::shm_open(ckey.as_ptr(), libc::O_RDWR, 0o666) };
    if fd < 0 {
        return Err(Error::Shm(format!(
            "shm_open({key}) failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    let map = unsafe {
        libc::mmap(
            ptr::null_mut(),
            ARENA_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if map == libc::MAP_FAILED {
        let err = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(Error::Shm(format!("mmap({key}) failed: {err}")));
    }
    Ok(SharedArena {
        key: ckey,
        fd,
        ptr: map as *mut Arena,
        unlink_on_drop: false,
    })
}

#[cfg(windows)]
mod winapi {
    use std::ffi::c_void;

    pub type Handle = *mut c_void;

    pub const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    pub const PAGE_READWRITE: u32 = 0x04;
    pub const FILE_MAP_ALL_ACCESS: u32 = 0x000F_001F;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn CreateFileMappingA(
            h_file: Handle,
            lp_attributes: *mut c_void,
            fl_protect: u32,
            dw_max_size_high: u32,
            dw_max_size_low: u32,
            lp_name: *const i8,
        ) -> Handle;
        pub fn OpenFileMappingA(
            dw_desired_access: u32,
            b_inherit_handle: i32,
            lp_name: *const i8,
        ) -> Handle;
        pub fn MapViewOfFile(
            h_file_mapping: Handle,
            dw_desired_access: u32,
            dw_file_offset_high: u32,
            dw_file_offset_low: u32,
            dw_number_of_bytes: usize,
        ) -> *mut c_void;
        pub fn UnmapViewOfFile(lp_base: *const c_void) -> i32;
        pub fn CloseHandle(h: Handle) -> i32;
        pub fn GetLastError() -> u32;
    }
}

#[cfg(windows)]
fn win_create(ckey: CString, init: bool) -> Result<SharedArena> {
    let key = ckey.to_str().unwrap_or("").to_string();
    let handle = unsafe {
        winapi::CreateFileMappingA(
            winapi::INVALID_HANDLE_VALUE,
            ptr::null_mut(),
            winapi::PAGE_READWRITE,
            0,
            ARENA_SIZE as u32,
            ckey.as_ptr() as *const i8,
        )
    };
    if handle.is_null() {
        return Err(Error::Shm(format!(
            "CreateFileMapping({key}) failed: {}",
            unsafe { winapi::GetLastError() }
        )));
    }
    let map =
        unsafe { winapi::MapViewOfFile(handle, winapi::FILE_MAP_ALL_ACCESS, 0, 0, ARENA_SIZE) };
    if map.is_null() {
        let err = unsafe { winapi::GetLastError() };
        unsafe {
            winapi::CloseHandle(handle);
        }
        return Err(Error::Shm(format!("MapViewOfFile({key}) failed: {err}")));
    }
    let ptr = map as *mut Arena;
    if init {
        unsafe { ptr.write(Arena::default()) };
    }
    Ok(SharedArena {
        key: ckey,
        handle,
        ptr,
        unlink_on_drop: true,
    })
}

#[cfg(windows)]
fn win_open(ckey: CString) -> Result<SharedArena> {
    let key = ckey.to_str().unwrap_or("").to_string();
    let handle = unsafe {
        winapi::OpenFileMappingA(winapi::FILE_MAP_ALL_ACCESS, 0, ckey.as_ptr() as *const i8)
    };
    if handle.is_null() {
        return Err(Error::Shm(format!(
            "OpenFileMapping({key}) failed: {}",
            unsafe { winapi::GetLastError() }
        )));
    }
    let map =
        unsafe { winapi::MapViewOfFile(handle, winapi::FILE_MAP_ALL_ACCESS, 0, 0, ARENA_SIZE) };
    if map.is_null() {
        let err = unsafe { winapi::GetLastError() };
        unsafe {
            winapi::CloseHandle(handle);
        }
        return Err(Error::Shm(format!("MapViewOfFile({key}) failed: {err}")));
    }
    Ok(SharedArena {
        key: ckey,
        handle,
        ptr: map as *mut Arena,
        unlink_on_drop: false,
    })
}

impl Drop for SharedArena {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, ARENA_SIZE);
            libc::close(self.fd);
            if self.unlink_on_drop {
                libc::shm_unlink(self.key.as_ptr());
            }
        }
        #[cfg(windows)]
        unsafe {
            winapi::UnmapViewOfFile(self.ptr as *const std::ffi::c_void);
            winapi::CloseHandle(self.handle);
            let _ = self.unlink_on_drop;
        }
    }
}

#[cfg(target_os = "linux")]
#[link(name = "rt")]
extern "C" {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shm_key;

    #[test]
    fn roundtrip() {
        let key = shm_key(std::process::id(), "t");
        let mut a = SharedArena::create(&key).expect("shm create");
        a.as_mut().running = 7;
        a.as_mut().reg_addr = 0x44000;
        assert_eq!(a.as_ref().running, 7);
        assert_eq!(a.as_ref().reg_addr, 0x44000);
        assert_eq!(a.key(), key);

        let b = SharedArena::open(&key).expect("shm open");
        assert_eq!(b.as_ref().running, 7);
        assert_eq!(b.as_ref().reg_addr, 0x44000);
    }
}
