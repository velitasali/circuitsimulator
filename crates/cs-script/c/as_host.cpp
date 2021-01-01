#include "as_host.h"

#include "angelscript.h"
#include "scriptarray.h"
#include "scriptstdstring.h"

#include <cstdio>
#include <string>

struct AsHost {
    asIScriptEngine* engine = nullptr;
    asIScriptContext* context = nullptr;
    asIScriptModule* module = nullptr;
    std::string last_error;
};

static void MessageCallback(const asSMessageInfo* msg, void* param) {
    auto* host = static_cast<AsHost*>(param);
    const char* kind = "ERROR";
    if (msg->type == asMSGTYPE_WARNING)
        kind = "WARNING";
    else if (msg->type == asMSGTYPE_INFORMATION)
        kind = "INFO";
    char buf[1024];
    std::snprintf(buf, sizeof(buf), "%s line: %d %d %s %s", msg->section, msg->row, msg->col,
                  kind, msg->message);
    if (!host->last_error.empty())
        host->last_error.push_back('\n');
    host->last_error += buf;
}

static AsPrintCallback g_print_cb = nullptr;

static void host_print(std::string& msg) {
    if (g_print_cb) {
        g_print_cb(msg.c_str());
    } else {
        std::fputs(msg.c_str(), stdout);
        std::fputc('\n', stdout);
    }
}

extern "C" {

void as_host_set_print_callback(AsPrintCallback cb) {
    g_print_cb = cb;
}

AsHost* as_host_create(void) {
    auto* host = new AsHost();
    host->engine = asCreateScriptEngine();
    if (!host->engine) {
        delete host;
        return nullptr;
    }
    host->context = host->engine->CreateContext();
    if (!host->context) {
        host->engine->ShutDownAndRelease();
        delete host;
        return nullptr;
    }
    host->engine->SetEngineProperty(asEP_AUTO_GARBAGE_COLLECT, false);
    host->engine->SetEngineProperty(asEP_BUILD_WITHOUT_LINE_CUES, true);
    host->engine->SetMessageCallback(asFUNCTION(MessageCallback), host, asCALL_CDECL);
    RegisterStdString(host->engine);
    RegisterScriptArray(host->engine, true);
    host->engine->RegisterGlobalFunction("void print(const string &in)", asFUNCTION(host_print),
                                         asCALL_CDECL);
    return host;
}

void as_host_destroy(AsHost* host) {
    if (!host)
        return;
    if (host->engine)
        host->engine->GarbageCollect(asGC_FULL_CYCLE);
    if (host->context)
        host->context->Release();
    if (host->engine)
        host->engine->ShutDownAndRelease();
    delete host;
}

int as_host_compile(AsHost* host, const char* section, const char* source) {
    if (!host || !host->engine || !source) {
        if (host)
            host->last_error = "no engine";
        return -1;
    }
    host->last_error.clear();
    host->engine->GarbageCollect(asGC_FULL_CYCLE);
    host->module = host->engine->GetModule(0, asGM_ALWAYS_CREATE);
    if (!host->module) {
        host->last_error = "GetModule failed";
        return -1;
    }
    const char* sec = section ? section : "script";
    int r = host->module->AddScriptSection(sec, source, std::string(source).size());
    if (r < 0) {
        if (host->last_error.empty())
            host->last_error = "AddScriptSection failed";
        return r;
    }
    r = host->module->Build();
    if (r < 0 && host->last_error.empty())
        host->last_error = "Build failed";
    return r;
}

int as_host_call_int0(AsHost* host, const char* decl, int* out) {
    if (!host || !host->module || !host->context || !decl) {
        if (host)
            host->last_error = "not compiled";
        return -1;
    }
    host->last_error.clear();
    asIScriptFunction* fn = host->module->GetFunctionByDecl(decl);
    if (!fn) {
        host->last_error = std::string("function not found: ") + decl;
        return -1;
    }
    host->context->Prepare(fn);
    int r = host->context->Execute();
    if (r != asEXECUTION_FINISHED) {
        if (r == asEXECUTION_EXCEPTION) {
            const char* desc = host->context->GetExceptionString();
            host->last_error = desc ? desc : "script exception";
        } else {
            host->last_error = "script did not finish";
        }
        return -1;
    }
    if (out)
        *out = static_cast<int>(host->context->GetReturnDWord());
    return 0;
}

int as_host_call_void0(AsHost* host, const char* decl) {
    return as_host_call_int0(host, decl, nullptr);
}

int as_host_has_function(AsHost* host, const char* decl) {
    if (!host || !host->module || !decl)
        return 0;
    return host->module->GetFunctionByDecl(decl) ? 1 : 0;
}

const char* as_host_last_error(const AsHost* host) {
    if (!host)
        return "no host";
    return host->last_error.c_str();
}

const char* as_host_library_version(void) {
    return asGetLibraryVersion();
}

} // extern "C"

static AsMcuApi g_api = {};

static void iopin_set_pin_mode(void* pin, unsigned m) {
    if (g_api.iopin_set_pin_mode)
        g_api.iopin_set_pin_mode(pin, m);
}
static bool iopin_get_inp_state(void* pin) {
    return g_api.iopin_get_inp_state ? g_api.iopin_get_inp_state(pin) != 0 : false;
}
static void iopin_set_out_state(void* pin, bool s) {
    if (g_api.iopin_set_out_state)
        g_api.iopin_set_out_state(pin, s ? 1 : 0);
}
static void iopin_set_state_z(void* pin, bool z) {
    if (g_api.iopin_set_state_z)
        g_api.iopin_set_state_z(pin, z ? 1 : 0);
}
static void iopin_set_out_stat_fast(void* pin, bool s) {
    if (g_api.iopin_set_out_stat_fast)
        g_api.iopin_set_out_stat_fast(pin, s ? 1 : 0);
}
static void iopin_schedule_state(void* pin, bool s, asQWORD time) {
    if (g_api.iopin_schedule_state)
        g_api.iopin_schedule_state(pin, s ? 1 : 0, time);
}
static double iopin_get_voltage(void* pin) {
    return g_api.iopin_get_voltage ? g_api.iopin_get_voltage(pin) : 0.0;
}
static void iopin_set_voltage(void* pin, double v) {
    if (g_api.iopin_set_voltage)
        g_api.iopin_set_voltage(pin, v);
}
static void iopin_set_out_high_v(void* pin, double v) {
    if (g_api.iopin_set_out_high_v)
        g_api.iopin_set_out_high_v(pin, v);
}
static void iopin_set_impedance(void* pin, double imp) {
    if (g_api.iopin_set_impedance)
        g_api.iopin_set_impedance(pin, imp);
}

static void ioport_set_pin_mode(void* port, unsigned m) {
    if (g_api.ioport_set_pin_mode)
        g_api.ioport_set_pin_mode(port, m);
}
static unsigned ioport_get_inp_state(void* port) {
    return g_api.ioport_get_inp_state ? g_api.ioport_get_inp_state(port) : 0;
}
static void ioport_set_out_state(void* port, unsigned s) {
    if (g_api.ioport_set_out_state)
        g_api.ioport_set_out_state(port, s);
}
static void ioport_schedule_state(void* port, unsigned s, asQWORD time) {
    if (g_api.ioport_schedule_state)
        g_api.ioport_schedule_state(port, s, time);
}
static void ioport_trigger(void* port, unsigned n) {
    if (g_api.ioport_trigger)
        g_api.ioport_trigger(port, n);
}

static void* cpu_get_pin(void* cpu, const std::string& name) {
    return g_api.cpu_get_pin ? g_api.cpu_get_pin(cpu, name.c_str()) : nullptr;
}
static void* cpu_get_port(void* cpu, const std::string& name) {
    return g_api.cpu_get_port ? g_api.cpu_get_port(cpu, name.c_str()) : nullptr;
}
static asQWORD cpu_circ_time(void* cpu) {
    return g_api.cpu_circ_time ? g_api.cpu_circ_time(cpu) : 0;
}
static void cpu_add_event(void* cpu, asQWORD time) {
    if (g_api.cpu_add_event)
        g_api.cpu_add_event(cpu, time);
}
static void cpu_cancel_events(void* cpu) {
    if (g_api.cpu_cancel_events)
        g_api.cpu_cancel_events(cpu);
}
static int cpu_read_ram(void* cpu, unsigned addr) {
    return g_api.cpu_read_ram ? g_api.cpu_read_ram(cpu, addr) : -1;
}
static void cpu_write_ram(void* cpu, unsigned addr, int v) {
    if (g_api.cpu_write_ram)
        g_api.cpu_write_ram(cpu, addr, v);
}
static int cpu_read_pgm(void* cpu, unsigned addr) {
    return g_api.cpu_read_pgm ? g_api.cpu_read_pgm(cpu, addr) : -1;
}
static void cpu_write_pgm(void* cpu, unsigned addr, int v) {
    if (g_api.cpu_write_pgm)
        g_api.cpu_write_pgm(cpu, addr, v);
}

static void mcupin_set_direction(void* pin, bool o) {
    if (g_api.mcupin_set_direction)
        g_api.mcupin_set_direction(pin, o ? 1 : 0);
}
static void mcupin_set_port_state(void* pin, bool s) {
    if (g_api.mcupin_set_port_state)
        g_api.mcupin_set_port_state(pin, s ? 1 : 0);
}
static void mcupin_control_pin(void* pin, bool outCtrl, bool dirCtrl) {
    if (g_api.mcupin_control_pin)
        g_api.mcupin_control_pin(pin, outCtrl ? 1 : 0, dirCtrl ? 1 : 0);
}
static void mcupin_set_ext_int(void* pin, unsigned mode) {
    if (g_api.mcupin_set_ext_int)
        g_api.mcupin_set_ext_int(pin, mode);
}
static void mcupin_set_out_state(void* pin, bool s) {
    if (g_api.mcupin_set_out_state)
        g_api.mcupin_set_out_state(pin, s ? 1 : 0);
}

static void mcuport_control_port(void* port, bool o, bool d) {
    if (g_api.mcuport_control_port)
        g_api.mcuport_control_port(port, o ? 1 : 0, d ? 1 : 0);
}
static void mcuport_set_direction(void* port, unsigned d) {
    if (g_api.mcuport_set_direction)
        g_api.mcuport_set_direction(port, d);
}
static void mcuport_set_out_state(void* port, unsigned s) {
    if (g_api.mcuport_set_out_state)
        g_api.mcuport_set_out_state(port, s);
}

static void* cpu_get_mcu_pin(void* cpu, const std::string& name) {
    return g_api.cpu_get_mcu_pin ? g_api.cpu_get_mcu_pin(cpu, name.c_str()) : nullptr;
}
static void* cpu_get_mcu_port(void* cpu, const std::string& name) {
    return g_api.cpu_get_mcu_port ? g_api.cpu_get_mcu_port(cpu, name.c_str()) : nullptr;
}

static void uart_set_baud(void* uart, int baud) {
    if (g_api.uart_set_baud)
        g_api.uart_set_baud(uart, baud);
}
static void uart_set_data_bits(void* uart, asBYTE bits) {
    if (g_api.uart_set_data_bits)
        g_api.uart_set_data_bits(uart, bits);
}
static void uart_send_byte(void* uart, asBYTE b) {
    if (g_api.uart_send_byte)
        g_api.uart_send_byte(uart, b);
}
static void spi_set_mode(void* spi, int mode) {
    if (g_api.spi_set_mode)
        g_api.spi_set_mode(spi, mode);
}
static void spi_send_byte(void* spi, asBYTE b) {
    if (g_api.spi_send_byte)
        g_api.spi_send_byte(spi, b);
}
static void twi_set_mode(void* twi, int mode) {
    if (g_api.twi_set_mode)
        g_api.twi_set_mode(twi, mode);
}
static void twi_send_byte(void* twi, asBYTE b) {
    if (g_api.twi_send_byte)
        g_api.twi_send_byte(twi, b);
}
static void twi_set_address(void* twi, asBYTE a) {
    if (g_api.twi_set_address)
        g_api.twi_set_address(twi, a);
}

extern "C" {

void as_host_set_mcu_api(const AsMcuApi* api) {
    if (api)
        g_api = *api;
    else
        g_api = AsMcuApi{};
}

int as_host_register_mcu(AsHost* host, void* script_cpu) {
    if (!host || !host->engine) {
        if (host)
            host->last_error = "no engine";
        return -1;
    }
    asIScriptEngine* e = host->engine;
    int r;

    r = e->RegisterObjectType("IoPin", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("IoPin", "void setPinMode(uint m)", asFUNCTION(iopin_set_pin_mode),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "bool getInpState()", asFUNCTION(iopin_get_inp_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setOutState(bool s)", asFUNCTION(iopin_set_out_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setStateZ(bool z)", asFUNCTION(iopin_set_state_z),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setOutStatFast(bool s)", asFUNCTION(iopin_set_out_stat_fast),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void scheduleState(bool state, uint64 time)",
                            asFUNCTION(iopin_schedule_state), asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "double getVoltage()", asFUNCTION(iopin_get_voltage),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setVoltage(double v)", asFUNCTION(iopin_set_voltage),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setOutHighV(double v)", asFUNCTION(iopin_set_out_high_v),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPin", "void setImpedance(double imp)", asFUNCTION(iopin_set_impedance),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("IoPort", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("IoPort", "void setPinMode(uint m)", asFUNCTION(ioport_set_pin_mode),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPort", "uint getInpState()", asFUNCTION(ioport_get_inp_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPort", "void setOutState(uint s)", asFUNCTION(ioport_set_out_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPort", "void scheduleState(uint32 state, uint64 time)",
                            asFUNCTION(ioport_schedule_state), asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("IoPort", "void trigger(uint n)", asFUNCTION(ioport_trigger),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("ScriptCpu", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("ScriptCpu", "IoPin@ getPin(const string pin)", asFUNCTION(cpu_get_pin),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "IoPort@ getPort(const string port)", asFUNCTION(cpu_get_port),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "uint64 circTime()", asFUNCTION(cpu_circ_time),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "void addEvent(uint64 t)", asFUNCTION(cpu_add_event),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "void cancelEvents()", asFUNCTION(cpu_cancel_events),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "int readRAM(uint n)", asFUNCTION(cpu_read_ram),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "void writeRAM(uint a, int v)", asFUNCTION(cpu_write_ram),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "int readPGM(uint n)", asFUNCTION(cpu_read_pgm),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "void writePGM(uint a, int v)", asFUNCTION(cpu_write_pgm),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("McuPin", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("McuPin", "void setDirection(bool o)", asFUNCTION(mcupin_set_direction),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void setPortState(bool s)", asFUNCTION(mcupin_set_port_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void controlPin(bool outCtrl, bool dirCtrl)",
                            asFUNCTION(mcupin_control_pin), asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void setExtInt(uint mode)", asFUNCTION(mcupin_set_ext_int),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void setPinMode(uint m)", asFUNCTION(iopin_set_pin_mode),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "bool getInpState()", asFUNCTION(iopin_get_inp_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void setOutState(bool s)", asFUNCTION(mcupin_set_out_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "double getVoltage()", asFUNCTION(iopin_get_voltage),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPin", "void setVoltage(double v)", asFUNCTION(iopin_set_voltage),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("McuPort", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("McuPort", "void controlPort(bool o, bool d)",
                            asFUNCTION(mcuport_control_port), asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPort", "void setDirection(uint d)", asFUNCTION(mcuport_set_direction),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPort", "uint getInpState()", asFUNCTION(ioport_get_inp_state),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("McuPort", "void setOutState(uint s)", asFUNCTION(mcuport_set_out_state),
                            asCALL_CDECL_OBJFIRST);

    e->RegisterObjectMethod("ScriptCpu", "McuPin@ getMcuPin(const string pin)",
                            asFUNCTION(cpu_get_mcu_pin), asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("ScriptCpu", "McuPort@ getMcuPort(const string port)",
                            asFUNCTION(cpu_get_mcu_port), asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("Uart", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("Uart", "void setBaudRate(int t)", asFUNCTION(uart_set_baud),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("Uart", "void setDataBits(uint8 b)", asFUNCTION(uart_set_data_bits),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("Uart", "void sendByte(uint8 b)", asFUNCTION(uart_send_byte),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("SPI", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("SPI", "void setMode(int t)", asFUNCTION(spi_set_mode),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("SPI", "void sendByte(uint8 b)", asFUNCTION(spi_send_byte),
                            asCALL_CDECL_OBJFIRST);

    r = e->RegisterObjectType("TWI", 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED)
        return r;
    e->RegisterObjectMethod("TWI", "void setMode(int t)", asFUNCTION(twi_set_mode),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("TWI", "void sendByte(uint8 b)", asFUNCTION(twi_send_byte),
                            asCALL_CDECL_OBJFIRST);
    e->RegisterObjectMethod("TWI", "void setAddress(uint8 a)", asFUNCTION(twi_set_address),
                            asCALL_CDECL_OBJFIRST);

    if (script_cpu) {
        r = e->RegisterGlobalProperty("ScriptCpu component", script_cpu);
        if (r < 0 && r != asALREADY_REGISTERED) {
            host->last_error = "RegisterGlobalProperty(component) failed";
            return r;
        }
    }
    return 0;
}

int as_host_register_global(AsHost* host, const char* decl, void* ptr) {
    if (!host || !host->engine || !decl || !ptr) {
        if (host)
            host->last_error = "register_global: bad args";
        return -1;
    }
    int r = host->engine->RegisterGlobalProperty(decl, ptr);
    if (r < 0 && r != asALREADY_REGISTERED) {
        host->last_error = std::string("RegisterGlobalProperty failed: ") + decl;
        return r;
    }
    return 0;
}

int as_host_call_void1u(AsHost* host, const char* decl, unsigned arg) {
    if (!host || !host->module || !host->context || !decl) {
        if (host)
            host->last_error = "not compiled";
        return -1;
    }
    host->last_error.clear();
    asIScriptFunction* fn = host->module->GetFunctionByDecl(decl);
    if (!fn) {
        host->last_error = std::string("function not found: ") + decl;
        return -1;
    }
    host->context->Prepare(fn);
    host->context->SetArgDWord(0, arg);
    int r = host->context->Execute();
    if (r != asEXECUTION_FINISHED) {
        if (r == asEXECUTION_EXCEPTION) {
            const char* desc = host->context->GetExceptionString();
            host->last_error = desc ? desc : "script exception";
        } else {
            host->last_error = "script did not finish";
        }
        return -1;
    }
    return 0;
}

} // extern "C"
