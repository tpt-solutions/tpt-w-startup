// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Parsing of Run-key command lines. Pure string handling, so it is tested on
//! every platform.

use std::path::PathBuf;

/// Expand `%NAME%` references from the environment. Unknown names are left as
/// written, so a missing variable does not silently change the path.
pub fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                // Lone percent sign: keep it and continue after it.
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Executable path from a Run-key command line.
///
/// Handles `"C:\Program Files\App\app.exe" --flag`, unquoted paths with spaces
/// that end in `.exe`, and a plain first token. Returns `None` for empty input.
pub fn exe_from_command(command: &str) -> Option<PathBuf> {
    let s = command.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"')?;
        return Some(PathBuf::from(&rest[..end]));
    }
    let lower = s.to_ascii_lowercase();
    if let Some(pos) = lower.find(".exe") {
        let candidate = &s[..pos + 4];
        // Ensure the match ends at a word boundary (".exe" followed by space or end).
        let boundary = s[pos + 4..].chars().next().is_none_or(char::is_whitespace);
        if boundary {
            return Some(PathBuf::from(candidate));
        }
    }
    s.split_whitespace().next().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_path_with_arguments() {
        assert_eq!(
            exe_from_command(r#""C:\Program Files\App\app.exe" --minimized"#),
            Some(PathBuf::from(r"C:\Program Files\App\app.exe"))
        );
    }

    #[test]
    fn unquoted_path_with_spaces_ending_in_exe() {
        assert_eq!(
            exe_from_command(r"C:\Program Files\App\app.exe /background"),
            Some(PathBuf::from(r"C:\Program Files\App\app.exe"))
        );
    }

    #[test]
    fn plain_first_token_fallback() {
        assert_eq!(
            exe_from_command("powershell -File x.ps1"),
            Some(PathBuf::from("powershell"))
        );
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(exe_from_command("   "), None);
    }

    #[test]
    fn env_expansion() {
        std::env::set_var("TPT_TEST_VAR", r"C:\Base");
        assert_eq!(expand_env(r"%TPT_TEST_VAR%\x.exe"), r"C:\Base\x.exe");
        assert_eq!(
            expand_env(r"%TPT_UNSET_VAR_XYZ%\x.exe"),
            r"%TPT_UNSET_VAR_XYZ%\x.exe"
        );
        assert_eq!(expand_env("100% sure"), "100% sure");
    }
}
