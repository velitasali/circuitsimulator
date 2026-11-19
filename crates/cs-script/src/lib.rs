//! AngelScript via the vendored C++ engine (`angel/`) and a minimal C ABI
//! (`c/as_host.h`). Do not replace with Rhai. Host types and methods register
//! directly through [`Engine::register_type`], [`Engine::register_method`], and
//! [`Engine::register_global`].

use std::ffi::{CStr, CString, c_void};
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

/// AngelScript's string factory is not safe to create/destroy from two
/// threads at once (debug `asASSERT` in `ReleaseReferences`).
static ENGINE_LIFE: Mutex<()> = Mutex::new(());

#[repr(C)]
struct AsHost {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn as_host_create() -> *mut AsHost;
    fn as_host_destroy(host: *mut AsHost);
    fn as_host_compile(host: *mut AsHost, section: *const c_char, source: *const c_char) -> c_int;
    fn as_host_call_int0(host: *mut AsHost, decl: *const c_char, out: *mut c_int) -> c_int;
    fn as_host_call_void0(host: *mut AsHost, decl: *const c_char) -> c_int;
    fn as_host_call_void1u(host: *mut AsHost, decl: *const c_char, arg: u32) -> c_int;
    fn as_host_has_function(host: *mut AsHost, decl: *const c_char) -> c_int;
    fn as_host_last_error(host: *const AsHost) -> *const c_char;
    fn as_host_library_version() -> *const c_char;
    fn as_host_set_print_callback(cb: Option<PrintCallback>);
    fn as_host_register_type(host: *mut AsHost, name: *const c_char) -> c_int;
    fn as_host_register_method(
        host: *mut AsHost,
        type_name: *const c_char,
        decl: *const c_char,
        fn_ptr: *const c_void,
    ) -> c_int;
    fn as_host_register_global(host: *mut AsHost, decl: *const c_char, ptr: *mut c_void) -> c_int;
    fn as_host_string_cstr(as_string: *const c_void) -> *const c_char;
}

pub type PrintCallback = unsafe extern "C" fn(*const c_char);

pub fn set_print_callback(cb: Option<PrintCallback>) {
    unsafe {
        as_host_set_print_callback(cb);
    }
}

/// Extract a UTF-8 view from an AngelScript `const string` object pointer
/// passed to a registered `extern "C"` method.
///
/// # Safety
/// `as_string` must be null or a valid pointer to an AngelScript `std::string`
/// for the lifetime `'a`.
pub unsafe fn as_string_cstr<'a>(as_string: *const c_void) -> &'a CStr {
    unsafe { CStr::from_ptr(as_host_string_cstr(as_string)) }
}

/// AngelScript engine with the same defaults as C++ `ScriptBase`:
/// `std::string`, `array`, `print()`, auto-GC off.
pub struct Engine {
    ptr: *mut AsHost,
}

impl Engine {
    pub fn new() -> Result<Self, Error> {
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
        let ptr = unsafe { as_host_create() };
        if ptr.is_null() {
            return Err(Error("failed to create script engine".into()));
        }
        Ok(Self { ptr })
    }

    pub fn compile(&mut self, section: &str, source: &str) -> Result<(), Error> {
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
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

    /// Execute `decl` with one `uint` argument (e.g. `"void byteReceived( uint d )"`).
    pub fn call_void1u(&mut self, decl: &str, arg: u32) -> Result<(), Error> {
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r = unsafe { as_host_call_void1u(self.ptr, decl.as_ptr(), arg) };
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

    /// Register a reference object type (`asOBJ_REF | asOBJ_NOCOUNT`).
    pub fn register_type(&mut self, name: &str) -> Result<(), Error> {
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
        let name = CString::new(name).map_err(|_| Error("type name contains NUL".into()))?;
        let r = unsafe { as_host_register_type(self.ptr, name.as_ptr()) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    /// Register an object method called with `asCALL_CDECL_OBJFIRST`.
    ///
    /// # Safety
    /// `fn_ptr` must be an `extern "C"` function whose ABI matches `decl` with
    /// the object pointer (`*mut c_void`) as the first argument.
    pub unsafe fn register_method(
        &mut self,
        type_name: &str,
        decl: &str,
        fn_ptr: *const c_void,
    ) -> Result<(), Error> {
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
        let type_name =
            CString::new(type_name).map_err(|_| Error("type name contains NUL".into()))?;
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r =
            unsafe { as_host_register_method(self.ptr, type_name.as_ptr(), decl.as_ptr(), fn_ptr) };
        if r < 0 {
            return Err(self.last_error());
        }
        Ok(())
    }

    /// Register a named global (`"ScriptCpu component"`, `"Uart UART0"`).
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn register_global(&mut self, decl: &str, ptr: *mut c_void) -> Result<(), Error> {
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
        let decl = CString::new(decl).map_err(|_| Error("decl contains NUL".into()))?;
        let r = unsafe { as_host_register_global(self.ptr, decl.as_ptr(), ptr) };
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
        let _life = ENGINE_LIFE.lock().unwrap_or_else(|e| e.into_inner());
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
    fn generic_type_and_method_register() {
        unsafe extern "C" fn get_val(obj: *mut c_void) -> c_int {
            unsafe { i32::from(*(obj as *const u8)) }
        }
        let mut dummy: u8 = 7;
        let mut e = Engine::new().unwrap();
        e.register_type("Dummy").unwrap();
        unsafe {
            e.register_method("Dummy", "int val()", get_val as *const c_void)
                .unwrap();
        }
        e.register_global("Dummy d", (&mut dummy as *mut u8).cast())
            .unwrap();
        e.compile("test", "int run() { return d.val(); }").unwrap();
        assert_eq!(e.call_int0("int run()").unwrap(), 7);
    }
}
