//! shell.rs — Cross-platform shell-wrapper launcher.
//!
//! Instead of spawning applications directly with `Command::new(exec)`, `px`
//! routes every launch through the platform's native shell.  This provides a
//! single, consistent execution path for all env-var injection, leveraging the
//! shell's own `export` / `set` semantics that every application already
//! understands.
//!
//! ## Why a shell wrapper?
//!
//! | Approach              | Env-var mechanism         | Extra PID? | Terminal held? |
//! |-----------------------|---------------------------|------------|----------------|
//! | `Command::env()`      | Rust runtime injection    | No         | Depends on kind |
//! | **Shell wrapper** ✓   | Shell `export` / `set`    | No (exec)  | Depends on kind |
//!
//! On Unix the shell command ends with `exec <app>` which **replaces** the
//! shell process with the app — no extra PID, no wasted memory, perfectly
//! transparent to the OS.  On Windows, `cmd /c` achieves the same effect
//! (cmd.exe exits as soon as the child finishes).
//!
//! ## Platform dispatch
//!
//! | Platform | Shell    | Arg   | Env syntax            | Exec                  |
//! |----------|----------|-------|-----------------------|-----------------------|
//! | Unix     | `/bin/sh`| `-c`  | `VAR="val" VAR2="v2"` | `exec <path> <args>`  |
//! | Windows  | `cmd`    | `/c`  | `set VAR=val&&set V2` | `start "" /b <path>`  |
//!
//! On Windows, desktop (detached) apps use `start "" "<path>"` so cmd.exe
//! returns immediately and the app owns its own window/lifecycle.

use std::path::Path;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A fully-built shell invocation ready to be passed to `std::process::Command`.
///
/// ```ignore
/// let inv = build(/* … */);
/// let mut cmd = std::process::Command::new(&inv.shell);
/// cmd.args(&inv.shell_args);
/// cmd.spawn()?;
/// ```
pub struct ShellInvocation {
    /// The shell binary to invoke (`/bin/sh` on Unix, `cmd` on Windows).
    pub shell: String,
    /// The arguments passed to the shell, typically `["-c", "<script>"]`.
    pub shell_args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Builds a [`ShellInvocation`] that:
///
/// 1. Sets all key-value pairs in `env_vars` using the platform shell's export
///    syntax.
/// 2. Executes `exec_path` with `exec_args`.
/// 3. On Unix, uses `exec` so the shell process is *replaced* by the app
///    (zero extra PIDs).
/// 4. On Windows, uses `start "" /b` for detached (Desktop) apps and a plain
///    invocation for CLI apps.
///
/// # Arguments
///
/// * `exec_path`  – Resolved path to the executable.
/// * `exec_args`  – Arguments to forward to the executable.
/// * `env_vars`   – `(KEY, value)` pairs to export before launching.
/// * `detach`     – `true` for Desktop apps; `false` for CLI (blocking) apps.
pub fn build(
    exec_path: &Path,
    exec_args: &[String],
    env_vars: &[(&str, &str)],
    #[allow(unused_variables)]
    detach: bool,
) -> ShellInvocation {
    #[cfg(not(target_os = "windows"))]
    return build_unix(exec_path, exec_args, env_vars);

    #[cfg(target_os = "windows")]
    return build_windows(exec_path, exec_args, env_vars, detach);
}

// ---------------------------------------------------------------------------
// Unix implementation
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
fn build_unix(
    exec_path: &Path,
    exec_args: &[String],
    env_vars: &[(&str, &str)],
) -> ShellInvocation {
    // Build: VAR1="val1" VAR2="val2" exec /path/to/app arg1 arg2
    //
    // We use inline assignment rather than `export` so it works on both bash
    // and dash (the POSIX sh on Ubuntu/Debian).  Values are double-quoted and
    // internal double-quotes are escaped.
    let mut parts: Vec<String> = Vec::new();

    for (key, val) in env_vars {
        let escaped = escape_for_sh(val);
        parts.push(format!(r#"{}="{}""#, key, escaped));
    }

    // `exec` replaces the shell process — no leftover sh in the process tree.
    let path_str = exec_path.to_string_lossy();
    let escaped_path = escape_for_sh(&path_str);
    parts.push(format!(r#"exec "{}""#, escaped_path));

    for arg in exec_args {
        parts.push(format!(r#""{}""#, escape_for_sh(arg)));
    }

    ShellInvocation {
        shell: "/bin/sh".to_string(),
        shell_args: vec!["-c".to_string(), parts.join(" ")],
    }
}

/// Escape a string for use inside a double-quoted POSIX shell argument.
/// Only `"`, `\`, `` ` ``, and `$` need escaping inside double-quotes.
#[cfg(not(target_os = "windows"))]
fn escape_for_sh(s: &str) -> String {
    s.replace('\\', r"\\")
     .replace('"', r#"\""#)
     .replace('`', r"\`")
     .replace('$', r"\$")
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn build_windows(
    exec_path: &Path,
    exec_args: &[String],
    env_vars: &[(&str, &str)],
    detach: bool,
) -> ShellInvocation {
    // Build: set VAR1=val1&& set VAR2=val2&& [start "" /b] "path\app.exe" arg1 arg2
    //
    // Notes:
    //  - `&&` chains commands; no spaces before `&&` to avoid trailing-space in values.
    //  - Values with spaces/special chars are not additionally quoted in `set` —
    //    cmd handles them fine when there are no `&|<>` characters.
    //  - For detached Desktop apps we use `start "" /b` which launches the child
    //    in its own process group so cmd.exe can return immediately.

    let mut parts: Vec<String> = Vec::new();

    for (key, val) in env_vars {
        let escaped = escape_for_cmd(val);
        parts.push(format!("set {}={}&&", key, escaped));
    }

    let path_str = exec_path.to_string_lossy();
    let quoted_path = format!("\"{}\"", path_str.replace('"', "\"\""));

    if detach {
        // `start "" /b` launches detached, returns immediately.
        parts.push(format!("start \"\" /b {}", quoted_path));
    } else {
        parts.push(quoted_path);
    }

    for arg in exec_args {
        parts.push(format!("\"{}\"", arg.replace('"', "\"\"")));
    }

    ShellInvocation {
        shell: "cmd".to_string(),
        shell_args: vec!["/c".to_string(), parts.join(" ")],
    }
}

/// Minimal escaping for cmd.exe `set VAR=<value>`.
/// Escapes `&`, `|`, `<`, `>`, `^` with a caret.
#[cfg(target_os = "windows")]
fn escape_for_cmd(s: &str) -> String {
    s.replace('^', "^^")
     .replace('&', "^&")
     .replace('|', "^|")
     .replace('<', "^<")
     .replace('>', "^>")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_uses_sh_minus_c() {
        let path = PathBuf::from("/usr/local/bin/codex");
        let inv = build(&path, &[], &[("HTTP_PROXY", "http://p:1234")], false);
        assert_eq!(inv.shell, "/bin/sh");
        assert_eq!(inv.shell_args[0], "-c");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_includes_exec() {
        let path = PathBuf::from("/usr/bin/cursor");
        let inv = build(&path, &[], &[("HTTP_PROXY", "http://p:1234")], false);
        let script = &inv.shell_args[1];
        assert!(script.contains("exec"), "script must contain 'exec': {}", script);
        assert!(script.contains("/usr/bin/cursor"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_env_vars_inline() {
        let path = PathBuf::from("/bin/true");
        let inv = build(
            &path,
            &[],
            &[("HTTP_PROXY", "http://u:p@host:8080"), ("NO_PROXY", "localhost")],
            false,
        );
        let script = &inv.shell_args[1];
        assert!(script.contains(r#"HTTP_PROXY="http://u:p@host:8080""#));
        assert!(script.contains(r#"NO_PROXY="localhost""#));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_quotes_escaped() {
        let path = PathBuf::from("/bin/true");
        let inv = build(&path, &[], &[("TEST", r#"has"quote"#)], false);
        let script = &inv.shell_args[1];
        assert!(script.contains(r#"has\"quote"#));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_uses_cmd_slash_c() {
        let path = PathBuf::from(r"C:\Program Files\Cursor\Cursor.exe");
        let inv = build(&path, &[], &[("HTTP_PROXY", "http://p:1234")], false);
        assert_eq!(inv.shell, "cmd");
        assert_eq!(inv.shell_args[0], "/c");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_desktop_uses_start() {
        let path = PathBuf::from(r"C:\Program Files\Cursor\Cursor.exe");
        let inv = build(&path, &[], &[], true);
        assert!(inv.shell_args[1].contains("start"));
    }
}
