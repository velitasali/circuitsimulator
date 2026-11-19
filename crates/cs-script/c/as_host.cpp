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

static int prepare_and_execute(AsHost* host, const char* decl, bool has_u32_arg, unsigned arg,
                               int* out) {
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
    if (has_u32_arg)
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
    if (out)
        *out = static_cast<int>(host->context->GetReturnDWord());
    return 0;
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
    return prepare_and_execute(host, decl, false, 0, out);
}

int as_host_call_void0(AsHost* host, const char* decl) {
    return prepare_and_execute(host, decl, false, 0, nullptr);
}

int as_host_call_void1u(AsHost* host, const char* decl, unsigned arg) {
    return prepare_and_execute(host, decl, true, arg, nullptr);
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

int as_host_register_type(AsHost* host, const char* name) {
    if (!host || !host->engine || !name) {
        if (host)
            host->last_error = "register_type: bad args";
        return -1;
    }
    int r = host->engine->RegisterObjectType(name, 0, asOBJ_REF | asOBJ_NOCOUNT);
    if (r < 0 && r != asALREADY_REGISTERED) {
        host->last_error = std::string("RegisterObjectType failed: ") + name;
        return r;
    }
    return 0;
}

int as_host_register_method(AsHost* host, const char* type_name, const char* decl,
                            const void* fn_ptr) {
    if (!host || !host->engine || !type_name || !decl || !fn_ptr) {
        if (host)
            host->last_error = "register_method: bad args";
        return -1;
    }
    int r = host->engine->RegisterObjectMethod(
        type_name, decl, asFunctionPtr(reinterpret_cast<asFUNCTION_t>(const_cast<void*>(fn_ptr))),
        asCALL_CDECL_OBJFIRST);
    if (r < 0 && r != asALREADY_REGISTERED) {
        host->last_error =
            std::string("RegisterObjectMethod failed: ") + type_name + "::" + decl;
        return r;
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

const char* as_host_string_cstr(const void* as_string) {
    if (!as_string)
        return "";
    return static_cast<const std::string*>(as_string)->c_str();
}

} // extern "C"
