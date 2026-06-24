//! path_utils.rs — Cross-platform path expansion and resolution.

use std::path::PathBuf;

/// Expands `~` to the user's home directory and `%ENV_VAR%` to their environment
/// values (Windows only).
pub fn expand_path(raw: &str) -> PathBuf {
    let mut expanded = raw.to_string();

    // 1. Expand `~` cross-platform
    if expanded.starts_with("~/") || expanded.starts_with("~\\") || expanded == "~" {
        if let Some(home) = dirs::home_dir() {
            expanded = expanded.replacen("~", &home.to_string_lossy(), 1);
        }
    }

    // 2. Expand `%VAR%` on Windows
    #[cfg(target_os = "windows")]
    {
        let mut result = String::new();
        let mut chars = expanded.chars().peekable();
        let mut in_var = false;
        let mut current_var = String::new();

        while let Some(c) = chars.next() {
            if c == '%' {
                if in_var {
                    // Close the variable
                    if let Ok(val) = std::env::var(&current_var) {
                        result.push_str(&val);
                    } else {
                        // Fallback: keep the original string if not found
                        result.push('%');
                        result.push_str(&current_var);
                        result.push('%');
                    }
                    current_var.clear();
                    in_var = false;
                } else {
                    // Open a variable
                    in_var = true;
                }
            } else {
                if in_var {
                    current_var.push(c);
                } else {
                    result.push(c);
                }
            }
        }
        // If we ended while in_var is true, append the unmatched part
        if in_var {
            result.push('%');
            result.push_str(&current_var);
        }
        expanded = result;
    }

    PathBuf::from(expanded)
}

/// Resolves the executable path, expanding variables and applying a
/// Windows-specific `.exe` / `.cmd` extension fallback when the bare path
/// does not exist on disk.
pub fn resolve_exec_path(raw: &str) -> PathBuf {
    let original = expand_path(raw);

    #[cfg(target_os = "windows")]
    {
        if original.exists() {
            return original;
        }
        for ext in &["exe", "cmd"] {
            let candidate = original.with_extension(ext);
            if candidate.exists() {
                return candidate;
            }
        }
        // Return original and let the OS produce a meaningful error.
        return original;
    }

    #[cfg(not(target_os = "windows"))]
    original
}
