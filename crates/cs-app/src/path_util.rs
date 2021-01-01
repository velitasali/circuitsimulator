//! Path and URL normalization utilities for file dialogs and façades.

/// Percent-decodes a URL-encoded string (e.g. `%20` -> space).
pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(hex_byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Strips `file://` or `file:` schemes and decodes percent-encoded characters.
/// On Windows, removes leading slashes before drive letters (e.g. `/C:/` or `\C:\`)
/// and normalizes backslashes to forward slashes.
pub fn strip_file_url(url: &str) -> String {
    let decoded = percent_decode(url.trim());
    let s = decoded
        .strip_prefix("file://")
        .or_else(|| decoded.strip_prefix("file:"))
        .unwrap_or(&decoded);

    #[cfg(target_os = "windows")]
    {
        normalize_windows_path(s)
    }
    #[cfg(not(target_os = "windows"))]
    {
        s.to_string()
    }
}

#[cfg(any(target_os = "windows", test))]
fn normalize_windows_path(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 3
        && (bytes[0] == b'/' || bytes[0] == b'\\')
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
    {
        return s[1..].replace('\\', "/");
    }
    s.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(
            percent_decode("C%3A/path/with%20space"),
            "C:/path/with space"
        );
        assert_eq!(percent_decode("no_encoding_here"), "no_encoding_here");
        assert_eq!(percent_decode("incomplete%2"), "incomplete%2");
        assert_eq!(percent_decode("invalid%2G"), "invalid%2G");
    }

    #[test]
    fn test_normalize_windows_path() {
        assert_eq!(
            normalize_windows_path("/C:/Users/test/project"),
            "C:/Users/test/project"
        );
        assert_eq!(
            normalize_windows_path("\\C:\\Users\\test\\project"),
            "C:/Users/test/project"
        );
        assert_eq!(
            normalize_windows_path("C:\\Users\\test\\project"),
            "C:/Users/test/project"
        );
        assert_eq!(
            normalize_windows_path("C:/Users/test/project"),
            "C:/Users/test/project"
        );
    }

    #[test]
    fn test_strip_file_url_cross_platform() {
        assert_eq!(strip_file_url(""), "");
        assert_eq!(strip_file_url("   "), "");

        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                strip_file_url("file:///C:/Users/veli/project"),
                "C:/Users/veli/project"
            );
            assert_eq!(
                strip_file_url("file:///C:/Users/veli/My%20Projects/circuitsimulator"),
                "C:/Users/veli/My Projects/circuitsimulator"
            );
            assert_eq!(
                strip_file_url("/C:/Users/veli/project"),
                "C:/Users/veli/project"
            );
            assert_eq!(
                strip_file_url("file://C:/Users/veli/project"),
                "C:/Users/veli/project"
            );
            assert_eq!(
                strip_file_url("C:\\Users\\veli\\project"),
                "C:/Users/veli/project"
            );
            assert_eq!(
                strip_file_url("  file:///C:/tmp/test.wav  "),
                "C:/tmp/test.wav"
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(
                strip_file_url("file:///home/user/project"),
                "/home/user/project"
            );
            assert_eq!(
                strip_file_url("file:///home/user/My%20Projects"),
                "/home/user/My Projects"
            );
            assert_eq!(strip_file_url("/home/user/project"), "/home/user/project");
            assert_eq!(strip_file_url("  file:///tmp/test.wav  "), "/tmp/test.wav");
        }
    }
}
