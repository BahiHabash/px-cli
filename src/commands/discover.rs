//! commands/discover.rs — Cross-platform auto-discovery engine.
//!
//! Scans the filesystem for known developer applications and returns a list of
//! `DiscoveredApp` entries ready to be merged into `config.toml`.
//!
//! ## Search strategy per platform
//!
//! | Platform | Directories searched                                          |
//! |----------|---------------------------------------------------------------|
//! | macOS    | `/Applications/`, `/usr/local/bin/`                          |
//! | Windows  | `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LOCALAPPDATA%`   |
//! | Linux    | `/usr/bin/`, `/usr/local/bin/`, `/opt/`                      |
//!
//! ## Supported applications
//!
//! - Visual Studio Code (desktop + cli)
//! - Cursor Desktop + Cursor CLI
//! - Codex Desktop + Codex CLI
//! - Antigravity IDE

use std::path::PathBuf;

use crate::config::{AppEntry, AppKind};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single application found during auto-discovery.
pub struct DiscoveredApp {
    /// The shortcut name that will be written to `[apps.<shortcut>]`.
    pub shortcut: &'static str,
    /// The resolved executable path and execution class.
    pub entry: AppEntry,
}

// ---------------------------------------------------------------------------
// Internal candidate definition
// ---------------------------------------------------------------------------

struct Candidate {
    shortcut: &'static str,
    kind: AppKind,
    /// Ordered list of paths to probe — the first existing path wins.
    paths: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Platform-specific candidate lists
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn candidates() -> Vec<Candidate> {
    vec![
        Candidate {
            shortcut: "vscode-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/Applications/Visual Studio Code.app/Contents/MacOS/Electron"),
            ],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/local/bin/code"),
                PathBuf::from("/opt/homebrew/bin/code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/Applications/Cursor.app/Contents/MacOS/Cursor"),
            ],
        },
        Candidate {
            shortcut: "cursor-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/local/bin/cursor"),
                PathBuf::from("/opt/homebrew/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/Applications/Codex.app/Contents/MacOS/Codex"),
            ],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/local/bin/codex"),
                PathBuf::from("/opt/homebrew/bin/codex"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/Applications/Antigravity.app/Contents/MacOS/Antigravity"),
            ],
        },
    ]
}

#[cfg(target_os = "windows")]
fn candidates() -> Vec<Candidate> {
    // %ProgramFiles%     → e.g. C:\Program Files
    // %LOCALAPPDATA%     → e.g. C:\Users\<user>\AppData\Local
    let prog  = PathBuf::from(std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()));
    let prog86 = PathBuf::from(std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()));
    let local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"));

    vec![
        Candidate {
            shortcut: "vscode-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                prog.join("Microsoft VS Code").join("Code.exe"),
                prog86.join("Microsoft VS Code").join("Code.exe"),
            ],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                prog.join("Microsoft VS Code").join("bin").join("code"),
                prog86.join("Microsoft VS Code").join("bin").join("code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                local.join("Programs").join("cursor").join("Cursor.exe"),
                local.join("Programs").join("Cursor").join("Cursor.exe"),
                prog.join("Cursor").join("Cursor.exe"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                local.join("Programs").join("codex").join("Codex.exe"),
                local.join("Programs").join("Codex").join("Codex.exe"),
                prog.join("Codex").join("Codex.exe"),
            ],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                // Codex CLI installed via npm/pip tends to land in the user's local bin
                local.join("Programs").join("codex").join("codex.cmd"),
                prog.join("nodejs").join("codex.cmd"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![
                local.join("Programs").join("antigravity").join("Antigravity.exe"),
                local.join("Programs").join("Antigravity").join("Antigravity.exe"),
                prog.join("Antigravity").join("Antigravity.exe"),
            ],
        },
    ]
}

#[cfg(target_os = "linux")]
fn candidates() -> Vec<Candidate> {
    vec![
        Candidate {
            shortcut: "vscode-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/usr/share/code/code"),
                PathBuf::from("/opt/visual-studio-code/code"),
                PathBuf::from("/snap/bin/code"),
            ],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/bin/code"),
                PathBuf::from("/usr/local/bin/code"),
                PathBuf::from("/snap/bin/code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/opt/cursor/cursor"),
                PathBuf::from("/usr/local/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "cursor-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/local/bin/cursor"),
                PathBuf::from("/usr/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/opt/codex/codex"),
            ],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                PathBuf::from("/usr/local/bin/codex"),
                PathBuf::from("/usr/bin/codex"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![
                PathBuf::from("/opt/antigravity/antigravity"),
                PathBuf::from("/usr/local/bin/antigravity"),
            ],
        },
    ]
}

// Fallback for unsupported platforms — returns nothing rather than panicking.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn candidates() -> Vec<Candidate> {
    vec![]
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scans the filesystem for known developer tools.
///
/// Returns only applications that actually exist on disk; everything else is
/// silently skipped.  Existing shortcuts in `config.toml` are **not**
/// overwritten — callers should filter against the existing map before merging.
pub fn scan() -> Vec<DiscoveredApp> {
    candidates()
        .into_iter()
        .filter_map(|candidate| {
            // Take the first path that exists on disk.
            let found = candidate.paths.into_iter().find(|p| p.exists())?;

            Some(DiscoveredApp {
                shortcut: candidate.shortcut,
                entry: AppEntry {
                    path: found.to_string_lossy().into_owned(),
                    kind: candidate.kind,
                },
            })
        })
        .collect()
}
