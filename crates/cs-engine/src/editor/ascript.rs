//! AngelScript compilation. C++ asDebugger::compile.

use std::path::Path;

use super::compiler::CompileResult;

pub fn compile_ascript(file_path: &str) -> CompileResult {
    let mut r = CompileResult::default();
    crate::logging::log_compiler(format!("Checking AngelScript file '{file_path}'..."));

    if !Path::new(file_path).exists() {
        r.error = -1;
        let msg = format!("Error: script file doesn't exist: {file_path}");
        crate::logging::log_compiler(&msg);
        r.log.push(msg);
        return r;
    }

    // The actual AngelScript compilation happens when the circuit uploads the
    // script to ScriptCpu::compile() (in cs-engine::script).  The editor's
    // compile step just validates the script file is present and readable.
    r.firmware = file_path.to_string();
    let msg = "     Ready to upload to Scripted Device";
    crate::logging::log_compiler(msg);
    r.log.push(msg.into());
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_error() {
        let r = compile_ascript("/tmp/no_such_script_999.as");
        assert!(r.error != 0);
        assert!(r.firmware.is_empty());
        assert!(r.log.iter().any(|l| l.contains("doesn't exist")));
    }

    #[test]
    fn existing_file_returns_success() {
        let dir = std::env::temp_dir();
        let file = dir.join("cs_ascript_test.as");
        std::fs::write(&file, "void main() {}\n").unwrap();

        let path = file.to_string_lossy().to_string();
        let r = compile_ascript(&path);

        assert_eq!(r.error, 0);
        assert_eq!(r.firmware, path);
        assert!(r.errors.is_empty());
        assert!(r.warnings.is_empty());
        assert!(r.log.iter().any(|l| l.contains("Ready to upload")));

        let _ = std::fs::remove_file(&file);
    }
}
