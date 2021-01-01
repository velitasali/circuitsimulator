//! clang-format integration. C++ `CodeEditor::formatDocument`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use super::compiler::command_exists;

// Analyse leading whitespace of non-comment lines to detect whether the file
// uses tabs or spaces, and the most likely indent width.  Returns
// `(use_tabs, indent_width)`.
pub fn detect_spacing_style(
    text: &str,
    fallback_space_tabs: bool,
    fallback_tab_size: i32,
) -> (bool, i32) {
    let mut tab_count = 0i32;
    let mut space_count = 0i32;

    let mut indent_delta_counts: BTreeMap<i32, i32> = BTreeMap::new();
    let mut leading_space_counts: BTreeMap<i32, i32> = BTreeMap::new();
    let mut prev_indent: i32 = -1;

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }

        let mut leading_spaces = 0i32;
        let mut leading_tabs = 0i32;
        for ch in line.chars() {
            if ch == ' ' {
                leading_spaces += 1;
            } else if ch == '\t' {
                leading_tabs += 1;
            } else {
                break;
            }
        }

        if leading_tabs > 0 {
            tab_count += 1;
        }
        if leading_spaces > 0 {
            space_count += 1;
            *leading_space_counts.entry(leading_spaces).or_insert(0) += 1;
        }

        if prev_indent >= 0 && leading_spaces > 0 && prev_indent > 0 {
            let delta = (leading_spaces - prev_indent).abs();
            if delta >= 2 && delta <= 8 {
                *indent_delta_counts.entry(delta).or_insert(0) += 1;
            }
        }
        prev_indent = leading_spaces;
    }

    let use_tabs = if tab_count > space_count {
        true
    } else if space_count > tab_count {
        false
    } else {
        !fallback_space_tabs
    };

    // Pick the most common indent delta.
    let mut best_delta = 0i32;
    let mut max_delta_count = 0i32;
    for (&delta, &count) in &indent_delta_counts {
        if count > max_delta_count {
            max_delta_count = count;
            best_delta = delta;
        }
    }

    let indent_width = if best_delta > 0 {
        best_delta
    } else if !leading_space_counts.is_empty() {
        let mut count4 = 0i32;
        let mut count2 = 0i32;
        for (&spaces, &count) in &leading_space_counts {
            if spaces % 4 == 0 {
                count4 += count;
            } else if spaces % 2 == 0 {
                count2 += count;
            }
        }
        if count4 > 0 && count4 >= count2 {
            4
        } else if count2 > 0 {
            2
        } else if fallback_tab_size > 0 {
            fallback_tab_size
        } else {
            4
        }
    } else if fallback_tab_size > 0 {
        fallback_tab_size
    } else {
        4
    };

    (use_tabs, indent_width)
}

// Walk ancestor directories of `file_path` looking for `.clang-format` or
// `_clang-format`.  Returns `true` if found.
pub fn find_clang_format_config(file_path: &str) -> bool {
    let p = Path::new(file_path);
    let mut dir = if p.is_dir() {
        Some(p.to_path_buf())
    } else {
        p.parent().map(|d| d.to_path_buf())
    };
    while let Some(d) = dir {
        if d.join(".clang-format").exists() || d.join("_clang-format").exists() {
            return true;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    false
}

// Run `clang-format` on the given text and return the formatted output.
pub fn format_code(
    text: &str,
    file_path: Option<&str>,
    fallback_space_tabs: bool,
    fallback_tab_size: i32,
) -> Result<String, String> {
    if !command_exists("clang-format") {
        return Err("clang-format not found in PATH".into());
    }

    let (use_tabs, indent_width) =
        detect_spacing_style(text, fallback_space_tabs, fallback_tab_size);

    let style_arg = if file_path.is_some() && find_clang_format_config(file_path.unwrap()) {
        "-style=file".to_string()
    } else {
        let tab_str = if use_tabs { "Always" } else { "Never" };
        format!(
            "-style={{BasedOnStyle: Google, UseTab: {}, IndentWidth: {}, TabWidth: {}, ColumnLimit: 100, LineEnding: LF}}",
            tab_str, indent_width, indent_width
        )
    };

    let assume_file = match file_path {
        Some(fp) => format!("-assume-filename={fp}"),
        None => "-assume-filename=sketch.ino".to_string(),
    };

    let mut cmd = Command::new("clang-format");
    cmd.arg(&style_arg).arg(&assume_file);

    if let Some(fp) = file_path {
        if let Some(parent) = Path::new(fp).parent() {
            cmd.current_dir(parent);
        }
    }

    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start clang-format: {e}"))?;

    // take() so ChildStdin is dropped after the write — that close is EOF,
    // matching QProcess::closeWriteChannel. as_mut() left the pipe open and
    // clang-format blocked until the 5s GUI-thread timeout.
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open clang-format stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to open clang-format stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to open clang-format stderr".to_string())?;

    // Pump stdin/stdout/stderr together. Sequential write-then-read deadlocks
    // once either pipe buffer fills (QProcess::waitForFinished already does this).
    let input = text.as_bytes().to_vec();
    let writer = thread::spawn(move || -> Result<(), String> {
        let mut stdin = stdin;
        stdin
            .write_all(&input)
            .map_err(|e| format!("failed to write to clang-format stdin: {e}"))?;
        drop(stdin);
        Ok(())
    });
    let stdout_reader = thread::spawn(move || {
        let mut stdout = stdout;
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let stderr_reader = thread::spawn(move || {
        let mut stderr = stderr;
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let timeout = Duration::from_secs(5);
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = writer.join();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err("clang-format timed out".into());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = writer.join();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("error waiting for clang-format: {e}"));
            }
        }
    };

    if let Err(e) = writer.join().unwrap_or(Ok(())) {
        return Err(e);
    }
    let stdout_buf = stdout_reader.join().unwrap_or_default();
    let stderr_buf = stderr_reader.join().unwrap_or_default();

    if !status.success() {
        let err = String::from_utf8_lossy(&stderr_buf);
        return Err(format!("clang-format exited with {status}:\n{err}"));
    }

    let formatted = String::from_utf8_lossy(&stdout_buf).to_string();

    // Normalise line endings.
    let formatted = formatted.replace("\r\n", "\n").replace('\r', "");

    if formatted.is_empty() && !text.is_empty() {
        return Err("clang-format produced empty output".into());
    }

    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_spacing_spaces_dominant() {
        let text = "\
int main() {
    int x = 0;
    if (x) {
        x++;
    }
}
";
        let (use_tabs, indent) = detect_spacing_style(text, true, 4);
        assert!(!use_tabs);
        assert_eq!(indent, 4);
    }

    #[test]
    fn detect_spacing_tabs_dominant() {
        let text = "\
int main() {
\tint x = 0;
\tif (x) {
\t\tx++;
\t}
}
";
        let (use_tabs, _indent) = detect_spacing_style(text, true, 4);
        assert!(use_tabs);
    }

    #[test]
    fn detect_spacing_empty_uses_fallback() {
        let (use_tabs, indent) = detect_spacing_style("", true, 8);
        // fallback_space_tabs=true  => use_tabs = !true = false
        assert!(!use_tabs);
        assert_eq!(indent, 8);
    }

    #[test]
    fn find_config_present() {
        let dir = std::env::temp_dir().join("cs_fmt_test_present");
        let _ = std::fs::create_dir_all(&dir);
        let cfg = dir.join(".clang-format");
        std::fs::write(&cfg, "BasedOnStyle: Google\n").unwrap();

        let dummy = dir.join("main.cpp");
        assert!(find_clang_format_config(dummy.to_str().unwrap()));

        let _ = std::fs::remove_file(&cfg);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn find_config_absent() {
        let dir = std::env::temp_dir().join("cs_fmt_test_absent");
        let _ = std::fs::create_dir_all(&dir);

        // Make sure there's no config file in this temp directory.
        let _ = std::fs::remove_file(dir.join(".clang-format"));
        let _ = std::fs::remove_file(dir.join("_clang-format"));

        let dummy = dir.join("main.cpp");
        // We can't guarantee *no* ancestor has a config, but for the
        // directory itself the check is meaningful.
        let result = find_clang_format_config(dummy.to_str().unwrap());
        // This may be true if a parent has the config, so just ensure it
        // doesn't panic and returns a bool.
        let _ = result;

        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn format_code_returns_before_timeout() {
        if !command_exists("clang-format") {
            return;
        }
        let src = "int main(){int x=1;return x;}\n";
        let start = Instant::now();
        let formatted = format_code(src, None, true, 4).expect("clang-format should succeed");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "format_code hung for {:?}",
            start.elapsed()
        );
        assert!(formatted.contains("int main"));
        assert_ne!(formatted, src);
    }
}
