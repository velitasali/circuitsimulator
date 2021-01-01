//! Paths handed from argv to façades before `QApp` starts.

use std::sync::OnceLock;

pub static OPEN_CIRCUIT: OnceLock<String> = OnceLock::new();
pub static OPEN_EDITOR: OnceLock<String> = OnceLock::new();
pub static NO_PROJECT: OnceLock<bool> = OnceLock::new();

pub fn no_project() -> bool {
    NO_PROJECT.get().copied().unwrap_or(false)
}
