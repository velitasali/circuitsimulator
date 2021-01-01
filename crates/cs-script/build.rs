//! Compile the vendored AngelScript C++ engine plus a small C ABI host.
//!
//! Flags match `CircuitSimulator.pri` (`-fno-strict-aliasing`, the warning
//! suppressions, and the platform callfunc sources).

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let angel = manifest.join("angel");
    let include = angel.join("include");
    let src = angel.join("src");

    println!("cargo:rerun-if-changed={}", include.display());
    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed=c/as_host.cpp");
    println!("cargo:rerun-if-changed=c/as_host.h");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .include(&include)
        .include(&src)
        .include(manifest.join("c"))
        .flag_if_supported("-fno-strict-aliasing")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-implicit-fallthrough")
        .flag_if_supported("-Wno-deprecated-copy")
        .flag_if_supported("-Wno-invalid-offsetof")
        .flag_if_supported("-Wno-cast-function-type")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-variable");

    let common = [
        "as_atomic.cpp",
        "as_builder.cpp",
        "as_bytecode.cpp",
        "as_callfunc.cpp",
        "as_compiler.cpp",
        "as_configgroup.cpp",
        "as_context.cpp",
        "as_datatype.cpp",
        "as_gc.cpp",
        "as_generic.cpp",
        "as_globalproperty.cpp",
        "as_memory.cpp",
        "as_module.cpp",
        "as_objecttype.cpp",
        "as_outputbuffer.cpp",
        "as_parser.cpp",
        "as_scriptcode.cpp",
        "as_scriptengine.cpp",
        "as_scriptfunction.cpp",
        "as_scriptnode.cpp",
        "as_scriptobject.cpp",
        "as_string.cpp",
        "as_string_util.cpp",
        "as_thread.cpp",
        "as_tokenizer.cpp",
        "as_typeinfo.cpp",
        "as_variablescope.cpp",
        "scriptarray.cpp",
        "scriptstdstring.cpp",
    ];
    for f in common {
        build.file(src.join(f));
    }

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match arch.as_str() {
        "aarch64" => {
            build.file(src.join("as_callfunc_arm64.cpp"));
            let asm = if os == "macos" {
                "as_callfunc_arm64_xcode.S"
            } else {
                "as_callfunc_arm64_gcc.S"
            };
            build.file(src.join(asm));
        }
        "x86_64" => {
            if os == "windows" {
                build.file(src.join("as_callfunc_x64_mingw.cpp"));
            } else {
                build.file(src.join("as_callfunc_x64_gcc.cpp"));
            }
        }
        "x86" => {
            build.file(src.join("as_callfunc_x86.cpp"));
        }
        _ => {}
    }

    build.file(manifest.join("c/as_host.cpp"));
    build.compile("angelscript_host");
}
