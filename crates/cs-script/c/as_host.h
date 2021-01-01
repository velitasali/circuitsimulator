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

/* 1 if the compiled module has `decl`, else 0. */
int as_host_has_function(AsHost* host, const char* decl);

/* Last compile/runtime error, never NULL. */
const char* as_host_last_error(const AsHost* host);

/* asGetLibraryVersion() */
const char* as_host_library_version(void);

/* MCU host callbacks. Object pointers are owned by the Rust ScriptCpu. */
typedef struct AsMcuApi {
    void (*iopin_set_pin_mode)(void* pin, unsigned m);
    int (*iopin_get_inp_state)(void* pin);
    void (*iopin_set_out_state)(void* pin, int s);
    void (*iopin_set_state_z)(void* pin, int z);
    void (*iopin_set_out_stat_fast)(void* pin, int s);
    void (*iopin_schedule_state)(void* pin, int s, unsigned long long time);
    double (*iopin_get_voltage)(void* pin);
    void (*iopin_set_voltage)(void* pin, double v);
    void (*iopin_set_out_high_v)(void* pin, double v);
    void (*iopin_set_impedance)(void* pin, double imp);

    void (*ioport_set_pin_mode)(void* port, unsigned m);
    unsigned (*ioport_get_inp_state)(void* port);
    void (*ioport_set_out_state)(void* port, unsigned s);
    void (*ioport_schedule_state)(void* port, unsigned s, unsigned long long time);
    void (*ioport_trigger)(void* port, unsigned n);

    void* (*cpu_get_pin)(void* cpu, const char* name);
    void* (*cpu_get_port)(void* cpu, const char* name);
    unsigned long long (*cpu_circ_time)(void* cpu);
    void (*cpu_add_event)(void* cpu, unsigned long long time);
    void (*cpu_cancel_events)(void* cpu);
    int (*cpu_read_ram)(void* cpu, unsigned addr);
    void (*cpu_write_ram)(void* cpu, unsigned addr, int v);
    int (*cpu_read_pgm)(void* cpu, unsigned addr);
    void (*cpu_write_pgm)(void* cpu, unsigned addr, int v);

    void (*mcupin_set_direction)(void* pin, int o);
    void (*mcupin_set_port_state)(void* pin, int s);
    void (*mcupin_control_pin)(void* pin, int outCtrl, int dirCtrl);
    void (*mcupin_set_ext_int)(void* pin, unsigned mode);
    void (*mcupin_set_out_state)(void* pin, int s);

    void (*mcuport_control_port)(void* port, int o, int d);
    void (*mcuport_set_direction)(void* port, unsigned d);
    void (*mcuport_set_out_state)(void* port, unsigned s);

    void* (*cpu_get_mcu_pin)(void* cpu, const char* name);
    void* (*cpu_get_mcu_port)(void* cpu, const char* name);

    void (*uart_set_baud)(void* uart, int baud);
    void (*uart_set_data_bits)(void* uart, unsigned bits);
    void (*uart_send_byte)(void* uart, unsigned b);
    void (*spi_set_mode)(void* spi, int mode);
    void (*spi_send_byte)(void* spi, unsigned b);
    void (*twi_set_mode)(void* twi, int mode);
    void (*twi_send_byte)(void* twi, unsigned b);
    void (*twi_set_address)(void* twi, unsigned a);
} AsMcuApi;

void as_host_set_mcu_api(const AsMcuApi* api);

typedef void (*AsPrintCallback)(const char* msg);
void as_host_set_print_callback(AsPrintCallback cb);

/* Register IoPin / IoPort / McuPin / McuPort / ScriptCpu / Uart / SPI / TWI
 * and the global `component` property. `script_cpu` is the ScriptCpu
 * instance pointer (asOBJ_NOCOUNT). */
int as_host_register_mcu(AsHost* host, void* script_cpu);

/* Register a named global (`"Uart UART0"`, `"SPI SPI0"`, `"TWI TWI0"`). */
int as_host_register_global(AsHost* host, const char* decl, void* ptr);

/* Call `decl` with one uint argument (e.g. `"void byteReceived( uint d )"`). */
int as_host_call_void1u(AsHost* host, const char* decl, unsigned arg);

#ifdef __cplusplus
}
#endif
