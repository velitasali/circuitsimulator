//! AngelScript via the vendored C++ engine (`angel/`) and a small C ABI
//! (`c/as_host.h`). Do not replace with Rhai. Host APIs (`IoPort`, `IoPin`,
//! `McuPort`, `McuPin`, `Uart`, `SPI`, `TWI`) register through
//! [`Engine::register_mcu`].

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

/// AngelScript's string factory is not safe to create/destroy from two
/// threads at once (debug `asASSERT` in `ReleaseReferences`).
static ENGINE_LIFE: Mutex<()> = Mutex::new(());

#[repr(C)]
struct AsHost {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct McuApi {
    pub iopin_set_pin_mode: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub iopin_get_inp_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> c_int>,
    pub iopin_set_out_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub iopin_set_state_z: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub iopin_set_out_stat_fast: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub iopin_schedule_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int, u64)>,
    pub iopin_get_voltage: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> f64>,
    pub iopin_set_voltage: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f64)>,
    pub iopin_set_out_high_v: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f64)>,
    pub iopin_set_impedance: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f64)>,
    pub ioport_set_pin_mode: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub ioport_get_inp_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u32>,
    pub ioport_set_out_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub ioport_schedule_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, u64)>,
    pub ioport_trigger: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub cpu_get_pin:
        Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const c_char) -> *mut std::ffi::c_void>,
    pub cpu_get_port:
        Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const c_char) -> *mut std::ffi::c_void>,
    pub cpu_circ_time: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u64>,
    pub cpu_add_event: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u64)>,
    pub cpu_cancel_events: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub cpu_read_ram: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32) -> c_int>,
    pub cpu_write_ram: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, c_int)>,
    pub cpu_read_pgm: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32) -> c_int>,
    pub cpu_write_pgm: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32, c_int)>,
    pub mcupin_set_direction: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub mcupin_set_port_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub mcupin_control_pin: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int, c_int)>,
    pub mcupin_set_ext_int: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub mcupin_set_out_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub mcuport_control_port: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int, c_int)>,
    pub mcuport_set_direction: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub mcuport_set_out_state: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub cpu_get_mcu_pin:
        Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const c_char) -> *mut std::ffi::c_void>,
    pub cpu_get_mcu_port:
        Option<unsafe extern "C" fn(*mut std::ffi::c_void, *const c_char) -> *mut std::ffi::c_void>,
    pub uart_set_baud: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub uart_set_data_bits: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub uart_send_byte: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub spi_set_mode: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub spi_send_byte: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub twi_set_mode: Option<unsafe extern "C" fn(*mut std::ffi::c_void, c_int)>,
    pub twi_send_byte: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
    pub twi_set_address: Option<unsafe extern "C" fn(*mut std::ffi::c_void, u32)>,
}

unsafe extern "C" {
    fn as_host_create() -> *mut AsHost;
    fn as_host_destroy(host: *mut AsHost);
    fn as_host_compile(host: *mut AsHost, section: *const c_char, source: *const c_char) -> c_int;
    fn as_host_call_int0(host: *mut AsHost, decl: *const c_char, out: *mut c_int) -> c_int;
    fn as_host_call_void0(host: *mut AsHost, decl: *const c_char) -> c_int;
    fn as_host_has_function(host: *mut AsHost, decl: *const c_char) -> c_int;
    fn as_host_last_error(host: *const AsHost) -> *const c_char;
    fn as_host_library_version() -> *const c_char;
    fn as_host_set_mcu_api(api: *const McuApi);
    fn as_host_set_print_callback(cb: Option<PrintCallback>);
    fn as_host_register_mcu(host: *mut AsHost, script_cpu: *mut std::ffi::c_void) -> c_int;
    fn as_host_register_global(
        host: *mut AsHost,
        decl: *const c_char,
        ptr: *mut std::ffi::c_void,
    ) -> c_int;
    fn as_host_call_void1u(host: *mut AsHost, decl: *const c_char, arg: u32) -> c_int;
}

pub type PrintCallback = unsafe extern "C" fn(*const c_char);

pub fn set_print_callback(cb: Option<PrintCallback>) {
    unsafe {
        as_host_set_print_callback(cb);
    }
}

/// AngelScript engine with the same defaults as C++ `ScriptBase`:
/// `std::string`, `array`, `print()`, auto-GC off.
pub struct Engine {
    ptr: *mut AsHost,
}

impl Engine {
    pub fn new() -> Result<Self, Error> {
        let _life = ENGINE_LIFE.lock().unwrap();
        let ptr = unsafe { as_host_create() };
        if ptr.is_null() {
            return Err(Error("failed to create script engine".into()));
        }
        Ok(Self { ptr })
    }

    pub fn compile(&mut self, section: &str, source: &str) -> Result<(), Error> {
        let section = CString::new(section).map_err(|_| Error("section contains NUL".into()))?;
        let source = CString::new(source).map_err(|_| Error("source contains NUL".into()))?;
        let r = unsafe { as_host_compile(self.ptr, section.as_ptr(), source.as_ptr()) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    /// Execute `decl` (AngelScript declaration, e.g. `"int answer()"`).
    pub fn call_int0(&mut self, decl: &str) -> Result<i32, Error> {
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let mut out: c_int = 0;
        let r = unsafe { as_host_call_int0(self.ptr, decl.as_ptr(), &mut out) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(out)
    }

    /// Execute a void no-arg function (e.g. `"void reset()"`).
    pub fn call_void0(&mut self, decl: &str) -> Result<(), Error> {
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r = unsafe { as_host_call_void0(self.ptr, decl.as_ptr()) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    pub fn has_function(&mut self, decl: &str) -> bool {
        let Ok(decl) = CString::new(decl) else {
            return false;
        };
        unsafe { as_host_has_function(self.ptr, decl.as_ptr()) != 0 }
    }

    /// Install IoPin / IoPort / ScriptCpu callbacks used by scripted MCUs.
    pub fn set_mcu_api(api: &McuApi) {
        unsafe { as_host_set_mcu_api(api) };
    }

    /// Register MCU types and the global `component` property. Call before
    /// [`compile`](Self::compile).
    pub fn register_mcu(&mut self, script_cpu: *mut std::ffi::c_void) -> Result<(), Error> {
        let r = unsafe { as_host_register_mcu(self.ptr, script_cpu) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    /// Register a named global (`"Uart UART0"`). Call after [`register_mcu`](Self::register_mcu)
    /// and before [`compile`](Self::compile).
    pub fn register_global(&mut self, decl: &str, ptr: *mut std::ffi::c_void) -> Result<(), Error> {
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r = unsafe { as_host_register_global(self.ptr, decl.as_ptr(), ptr) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    /// Execute `decl` with one `uint` argument (e.g. `"void byteReceived( uint d )"`).
    pub fn call_void1u(&mut self, decl: &str, arg: u32) -> Result<(), Error> {
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r = unsafe { as_host_call_void1u(self.ptr, decl.as_ptr(), arg) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    fn last_error(&self) -> Error {
        Error(unsafe { cstr(as_host_last_error(self.ptr)) })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _life = ENGINE_LIFE.lock().unwrap();
        unsafe { as_host_destroy(self.ptr) }
    }
}

/// `asGetLibraryVersion()`, e.g. `"2.35.1"`.
pub fn library_version() -> &'static str {
    unsafe { CStr::from_ptr(as_host_library_version()) }
        .to_str()
        .unwrap_or("")
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_2_35() {
        let v = library_version();
        assert!(v.contains("2.35"), "{v}");
    }

    #[test]
    fn compile_and_call_int() {
        let mut e = Engine::new().unwrap();
        e.compile("test", "int answer() { return 40 + 2; }")
            .unwrap();
        assert_eq!(e.call_int0("int answer()").unwrap(), 42);
    }

    #[test]
    fn compile_error_is_reported() {
        let mut e = Engine::new().unwrap();
        let err = e.compile("test", "int oops() { return ; }").unwrap_err();
        assert!(!err.0.is_empty(), "{err}");
    }

    #[test]
    fn print_and_string_are_registered() {
        let mut e = Engine::new().unwrap();
        e.compile(
            "test",
            r#"
            int run() {
                string s = "hi";
                print(s);
                return s.length();
            }
            "#,
        )
        .unwrap();
        assert_eq!(e.call_int0("int run()").unwrap(), 2);
    }

    #[test]
    fn mcu_types_register() {
        let mut dummy: u8 = 0;
        let mut e = Engine::new().unwrap();
        e.register_mcu((&mut dummy as *mut u8).cast()).unwrap();
        e.compile(
            "test",
            r#"
            int run() {
                if (component is null) return 0;
                return 1;
            }
            "#,
        )
        .unwrap();
        assert!(e.has_function("int run()"));
    }
}
