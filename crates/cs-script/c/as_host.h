/* C ABI over the vendored AngelScript engine. Keep this bindgen-friendly. */

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AsHost AsHost;

/* Create an engine with std::string, array, and print() registered, matching
 * ScriptBase. Returns NULL on failure. */
AsHost* as_host_create(void);
void as_host_destroy(AsHost* host);

/* Compile `source` as one section. Returns 0 on success. */
int as_host_compile(AsHost* host, const char* section, const char* source);

/* Prepare and execute `decl` (e.g. "int answer()") with no args. Stores the
 * int return in *out. Returns 0 on success. */
int as_host_call_int0(AsHost* host, const char* decl, int* out);

/* Execute a void function with no args (e.g. "void reset()"). */
int as_host_call_void0(AsHost* host, const char* decl);

/* Execute `decl` with one uint argument (e.g. "void byteReceived( uint d )"). */
int as_host_call_void1u(AsHost* host, const char* decl, unsigned arg);

/* 1 if the compiled module has `decl`, else 0. */
int as_host_has_function(AsHost* host, const char* decl);

/* Last compile/runtime error, never NULL. */
const char* as_host_last_error(const AsHost* host);

/* asGetLibraryVersion() */
const char* as_host_library_version(void);

typedef void (*AsPrintCallback)(const char* msg);
void as_host_set_print_callback(AsPrintCallback cb);

/* Register a reference object type (asOBJ_REF | asOBJ_NOCOUNT). */
int as_host_register_type(AsHost* host, const char* name);

/* Register an object method using asCALL_CDECL_OBJFIRST. */
int as_host_register_method(AsHost* host, const char* type_name, const char* decl,
                            const void* fn_ptr);

/* Register a named global ("ScriptCpu component", "Uart UART0", ...). */
int as_host_register_global(AsHost* host, const char* decl, void* ptr);

/* Extract a NUL-terminated string from an AngelScript `const string` parameter. */
const char* as_host_string_cstr(const void* as_string);

#ifdef __cplusplus
}
#endif
