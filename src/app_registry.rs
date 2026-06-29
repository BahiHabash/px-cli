//! app_registry.rs — Known developer tools and their platform-specific paths.
//!
//! This module is intentionally data-oriented.  Static filesystem discovery,
//! process scanning, and `px register --search` all use these definitions so
//! app identity, aliases, defaults, and known install paths stay consistent.

use std::path::{Path, PathBuf};

use crate::config::AppKind;

/// A known application path candidate.
#[derive(Debug, Clone)]
pub struct PathCandidate {
    /// Template saved to `config.toml`.
    pub template: String,
    /// Concrete path checked on this machine.
    pub evaluated: PathBuf,
}

/// A developer tool that px knows how to identify.
#[derive(Debug, Clone, Copy)]
pub struct AppDefinition {
    pub shortcut: &'static str,
    pub display_name: &'static str,
    pub aliases: &'static [&'static str],
    pub kind: AppKind,
    pub ai_only_proxy: bool,
}

impl AppDefinition {
    /// Returns true when a process name or executable path appears to belong to
    /// this app. Matching is case-insensitive and intentionally fuzzy because
    /// process managers expose different names across OS/package formats.
    pub fn matches_process(&self, process_name: &str, exe_path: &str) -> bool {
        let name = normalize_match_text(process_name);
        let path = normalize_match_text(exe_path);

        self.aliases
            .iter()
            .any(|alias| alias_matches(&name, &path, alias))
    }

    /// Returns known install paths for this app on the current platform.
    pub fn path_candidates(&self) -> Vec<PathCandidate> {
        path_candidates_for(self.shortcut)
    }
}

/// Known apps, split by executable class where a desktop app and CLI launcher
/// are separate registered shortcuts.
pub const KNOWN_APPS: &[AppDefinition] = &[
    AppDefinition {
        shortcut: "vscode-desktop",
        display_name: "Visual Studio Code",
        aliases: &["code", "visual studio code", "vscode"],
        kind: AppKind::Desktop,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "vscode-cli",
        display_name: "Visual Studio Code CLI",
        aliases: &["code"],
        kind: AppKind::Cli,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "cursor-desktop",
        display_name: "Cursor",
        aliases: &["cursor"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "cursor-cli",
        display_name: "Cursor CLI",
        aliases: &["cursor"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "codex-desktop",
        display_name: "Codex",
        aliases: &["codex"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "codex-cli",
        display_name: "Codex CLI",
        aliases: &["codex"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "antigravity-desktop",
        display_name: "Antigravity",
        aliases: &["antigravity", "antigravity ide"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "antigravity-cli",
        display_name: "Antigravity CLI",
        aliases: &["antigravity", "antigravity ide"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
];

/// Apps that are useful to prompt for during `px init` if static discovery
/// could not find them. CLI processes usually exit too quickly to capture.
pub fn core_desktop_apps() -> impl Iterator<Item = &'static AppDefinition> {
    KNOWN_APPS.iter().filter(|app| app.kind == AppKind::Desktop)
}

pub fn known_apps() -> &'static [AppDefinition] {
    KNOWN_APPS
}

pub fn find_by_shortcut(shortcut: &str) -> Option<&'static AppDefinition> {
    KNOWN_APPS.iter().find(|app| app.shortcut == shortcut)
}

pub fn identify_process(process_name: &str, exe_path: &str) -> Option<&'static AppDefinition> {
    KNOWN_APPS
        .iter()
        .filter(|app| app.kind == AppKind::Desktop)
        .find(|app| app.matches_process(process_name, exe_path))
        .or_else(|| {
            KNOWN_APPS
                .iter()
                .find(|app| app.matches_process(process_name, exe_path))
        })
}

pub fn is_ai_tool(shortcut: &str) -> bool {
    find_by_shortcut(shortcut)
        .map(|app| app.ai_only_proxy)
        .unwrap_or_else(|| {
            let lower = shortcut.to_lowercase();
            lower.contains("cursor") || lower.contains("codex") || lower.contains("antigravity")
        })
}

fn normalize_match_text(raw: &str) -> String {
    raw.to_lowercase()
        .replace(".exe", "")
        .replace(".cmd", "")
        .replace(".bat", "")
        .replace('-', " ")
        .replace('_', " ")
}

fn alias_matches(process_name: &str, exe_path: &str, alias: &str) -> bool {
    let alias = normalize_match_text(alias);
    if alias.is_empty() {
        return false;
    }

    if alias.contains(' ') {
        return process_name.contains(&alias) || exe_path.contains(&alias);
    }

    process_tokens(process_name)
        .into_iter()
        .chain(process_tokens(executable_name(exe_path)))
        .any(|token| token == alias)
}

fn executable_name(exe_path: &str) -> &str {
    Path::new(exe_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(exe_path)
}

fn process_tokens(raw: &str) -> Vec<String> {
    raw.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn p(raw: &str) -> PathCandidate {
    PathCandidate {
        template: raw.to_string(),
        evaluated: PathBuf::from(raw),
    }
}

#[cfg(not(target_os = "windows"))]
fn home_path(template: &str, relative: &str) -> PathCandidate {
    PathCandidate {
        template: template.to_string(),
        evaluated: dirs::home_dir().unwrap_or_default().join(relative),
    }
}

#[cfg(target_os = "windows")]
fn windows_paths(shortcut: &str) -> Vec<PathCandidate> {
    let prog = PathBuf::from(
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string()),
    );
    let prog86 = PathBuf::from(
        std::env::var("ProgramFiles(x86)")
            .unwrap_or_else(|_| r"C:\Program Files (x86)".to_string()),
    );
    let local =
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"));

    let lp = |sub: &str| PathCandidate {
        template: format!(r"%LOCALAPPDATA%\{}", sub),
        evaluated: local.join(sub),
    };
    let pp = |sub: &str| PathCandidate {
        template: format!(r"%ProgramFiles%\{}", sub),
        evaluated: prog.join(sub),
    };
    let p86 = |sub: &str| PathCandidate {
        template: format!(r"%ProgramFiles(x86)%\{}", sub),
        evaluated: prog86.join(sub),
    };

    match shortcut {
        "vscode-desktop" => vec![
            lp(r"Programs\Microsoft VS Code\Code.exe"),
            pp(r"Microsoft VS Code\Code.exe"),
            p86(r"Microsoft VS Code\Code.exe"),
            lp(r"Microsoft\WindowsApps\code.exe"),
        ],
        "vscode-cli" => vec![
            lp(r"Programs\Microsoft VS Code\bin\code"),
            pp(r"Microsoft VS Code\bin\code"),
            p86(r"Microsoft VS Code\bin\code"),
        ],
        "cursor-desktop" => vec![
            lp(r"Programs\cursor\Cursor.exe"),
            lp(r"Programs\Cursor\Cursor.exe"),
            pp(r"Cursor\Cursor.exe"),
        ],
        "cursor-cli" => vec![
            lp(r"Programs\cursor\resources\app\bin\cursor.cmd"),
            lp(r"Programs\Cursor\resources\app\bin\cursor.cmd"),
            lp(r"Programs\cursor\cursor.cmd"),
            pp(r"Cursor\cursor.cmd"),
        ],
        "codex-desktop" => vec![
            lp(r"Programs\codex\Codex.exe"),
            lp(r"Programs\Codex\Codex.exe"),
            pp(r"Codex\Codex.exe"),
            lp(r"Microsoft\WindowsApps\codex.exe"),
        ],
        "codex-cli" => vec![
            lp(r"Programs\codex\codex.cmd"),
            pp(r"nodejs\codex.cmd"),
            lp(r"Microsoft\WindowsApps\codex.exe"),
        ],
        "antigravity-desktop" => vec![
            lp(r"Programs\antigravity ide\Antigravity IDE.exe"),
            lp(r"Programs\Antigravity IDE\Antigravity IDE.exe"),
            pp(r"Antigravity IDE\Antigravity IDE.exe"),
        ],
        "antigravity-cli" => vec![
            lp(r"Programs\antigravity ide\bin\antigravity.cmd"),
            lp(r"Programs\Antigravity IDE\bin\antigravity.cmd"),
            pp(r"Antigravity IDE\bin\antigravity.cmd"),
        ],
        _ => vec![],
    }
}

fn path_candidates_for(shortcut: &str) -> Vec<PathCandidate> {
    #[cfg(target_os = "windows")]
    {
        return windows_paths(shortcut);
    }

    #[cfg(target_os = "macos")]
    {
        match shortcut {
            "vscode-desktop" => vec![
                p("/Applications/Visual Studio Code.app/Contents/MacOS/Code"),
                p("/Applications/Visual Studio Code.app/Contents/MacOS/Electron"),
                home_path(
                    "~/Applications/Visual Studio Code.app/Contents/MacOS/Code",
                    "Applications/Visual Studio Code.app/Contents/MacOS/Code",
                ),
                home_path(
                    "~/Applications/Visual Studio Code.app/Contents/MacOS/Electron",
                    "Applications/Visual Studio Code.app/Contents/MacOS/Electron",
                ),
            ],
            "vscode-cli" => vec![p("/opt/homebrew/bin/code"), p("/usr/local/bin/code")],
            "cursor-desktop" => vec![
                p("/Applications/Cursor.app/Contents/MacOS/Cursor"),
                home_path(
                    "~/Applications/Cursor.app/Contents/MacOS/Cursor",
                    "Applications/Cursor.app/Contents/MacOS/Cursor",
                ),
            ],
            "cursor-cli" => vec![
                p("/opt/homebrew/bin/cursor"),
                p("/usr/local/bin/cursor"),
                home_path("~/.local/bin/cursor", ".local/bin/cursor"),
            ],
            "codex-desktop" => vec![
                p("/Applications/Codex.app/Contents/MacOS/Codex"),
                home_path(
                    "~/Applications/Codex.app/Contents/MacOS/Codex",
                    "Applications/Codex.app/Contents/MacOS/Codex",
                ),
            ],
            "codex-cli" => vec![
                p("/opt/homebrew/bin/codex"),
                p("/usr/local/bin/codex"),
                home_path("~/.local/bin/codex", ".local/bin/codex"),
            ],
            "antigravity-desktop" => vec![
                p("/Applications/Antigravity.app/Contents/MacOS/Antigravity"),
                p("/Applications/Antigravity IDE.app/Contents/MacOS/Antigravity IDE"),
                p("/Applications/Antigravity IDE.app/Contents/MacOS/Electron"),
                home_path(
                    "~/Applications/Antigravity.app/Contents/MacOS/Antigravity",
                    "Applications/Antigravity.app/Contents/MacOS/Antigravity",
                ),
                home_path(
                    "~/Applications/Antigravity IDE.app/Contents/MacOS/Antigravity IDE",
                    "Applications/Antigravity IDE.app/Contents/MacOS/Antigravity IDE",
                ),
            ],
            "antigravity-cli" => vec![
                p("/opt/homebrew/bin/antigravity"),
                p("/usr/local/bin/antigravity"),
                home_path("~/.local/bin/antigravity", ".local/bin/antigravity"),
            ],
            _ => vec![],
        }
    }

    #[cfg(target_os = "linux")]
    {
        match shortcut {
            "vscode-desktop" => vec![
                p("/usr/share/code/code"),
                p("/opt/visual-studio-code/code"),
                p("/snap/bin/code"),
            ],
            "vscode-cli" => vec![
                p("/usr/bin/code"),
                p("/usr/local/bin/code"),
                p("/snap/bin/code"),
            ],
            "cursor-desktop" => vec![
                p("/opt/cursor/cursor"),
                p("/usr/bin/cursor"),
                p("/usr/local/bin/cursor"),
            ],
            "cursor-cli" => vec![
                p("/usr/bin/cursor"),
                p("/usr/local/bin/cursor"),
                home_path("~/.local/bin/cursor", ".local/bin/cursor"),
            ],
            "codex-desktop" => vec![p("/opt/codex/codex")],
            "codex-cli" => vec![
                p("/usr/bin/codex"),
                p("/usr/local/bin/codex"),
                home_path("~/.local/bin/codex", ".local/bin/codex"),
            ],
            "antigravity-desktop" => vec![
                p("/opt/antigravity/antigravity"),
                p("/opt/antigravity-ide/antigravity-ide"),
                p("/usr/local/bin/antigravity"),
                p("/usr/local/bin/antigravity-ide"),
            ],
            "antigravity-cli" => vec![
                p("/usr/bin/antigravity"),
                p("/usr/local/bin/antigravity"),
                p("/usr/bin/antigravity-ide"),
                p("/usr/local/bin/antigravity-ide"),
                home_path("~/.local/bin/antigravity", ".local/bin/antigravity"),
            ],
            _ => vec![],
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = shortcut;
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_cursor_process() {
        let app = identify_process("Cursor", "/Applications/Cursor.app/Contents/MacOS/Cursor")
            .expect("cursor should be known");
        assert_eq!(app.shortcut, "cursor-desktop");
        assert!(app.ai_only_proxy);
    }

    #[test]
    fn identifies_antigravity_process() {
        let app = identify_process(
            "Antigravity IDE",
            r"C:\Users\me\AppData\Local\Programs\Antigravity IDE\Antigravity IDE.exe",
        )
        .expect("antigravity should be known");
        assert_eq!(app.shortcut, "antigravity-desktop");
    }

    #[test]
    fn recognizes_ai_tool_shortcuts() {
        assert!(is_ai_tool("codex-cli"));
        assert!(is_ai_tool("cursor-desktop"));
        assert!(is_ai_tool("my-antigravity-wrapper"));
        assert!(!is_ai_tool("vscode-desktop"));
    }

    #[test]
    fn short_aliases_require_token_boundaries() {
        assert!(identify_process(
            "Code",
            "/Applications/Visual Studio Code.app/Contents/MacOS/Code"
        )
        .is_some());
        assert_eq!(
            identify_process("codex", "/Users/me/.local/bin/codex")
                .expect("codex should be known")
                .shortcut,
            "codex-desktop"
        );
        assert!(identify_process(
            "node_repl",
            "/Applications/Codex.app/Contents/Resources/cua_node/bin/node_repl"
        )
        .is_none());
        assert!(
            identify_process("PasscodeSettingsSubscriber", "/System/PasscodeSettingsSubscriber")
                .is_none()
        );
        assert!(identify_process("VTDecoderXPCService", "/System/VTDecoderXPCService").is_none());
    }
}
