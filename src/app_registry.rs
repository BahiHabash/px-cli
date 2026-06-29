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
        shortcut: "vscode-d",
        display_name: "Visual Studio Code",
        aliases: &["code", "visual studio code", "vscode", "vscode desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "code",
        display_name: "Visual Studio Code CLI",
        aliases: &["code", "vscode cli"],
        kind: AppKind::Cli,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "cursor-d",
        display_name: "Cursor",
        aliases: &["cursor", "cursor desktop", "cursor-desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "cursor",
        display_name: "Cursor CLI",
        aliases: &["cursor", "cursor cli", "cursor-cli"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "codex-d",
        display_name: "Codex",
        aliases: &["codex", "codex desktop", "codex-desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "codex",
        display_name: "Codex CLI",
        aliases: &["codex", "codex cli", "codex-cli"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "antigravity-d",
        display_name: "Antigravity",
        aliases: &[
            "antigravity",
            "antigravity ide",
            "antigravity desktop",
            "antigravity-desktop",
        ],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "antigravity",
        display_name: "Antigravity CLI",
        aliases: &["antigravity", "antigravity cli", "antigravity-cli"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "kiro-d",
        display_name: "Kiro",
        aliases: &["kiro", "kiro desktop", "kiro-desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "kiro",
        display_name: "Kiro CLI",
        aliases: &["kiro", "kiro cli", "kiro-cli"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "claude-d",
        display_name: "Claude",
        aliases: &["claude", "claude desktop", "claude-desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "claude",
        display_name: "Claude CLI",
        aliases: &["claude", "claude cli", "claude-cli", "claude code"],
        kind: AppKind::Cli,
        ai_only_proxy: true,
    },
    AppDefinition {
        shortcut: "rustrover-d",
        display_name: "RustRover",
        aliases: &["rustrover", "rust rover", "rustrover desktop"],
        kind: AppKind::Desktop,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "rustrover",
        display_name: "RustRover CLI",
        aliases: &["rustrover", "rust rover cli"],
        kind: AppKind::Cli,
        ai_only_proxy: false,
    },
    AppDefinition {
        shortcut: "vim",
        display_name: "Vim",
        aliases: &["vim"],
        kind: AppKind::Cli,
        ai_only_proxy: false,
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
        .filter(|app| app.matches_process(process_name, exe_path))
        .max_by_key(|app| process_match_score(app, process_name, exe_path))
}

pub fn is_ai_tool(shortcut: &str) -> bool {
    find_by_shortcut(shortcut)
        .map(|app| app.ai_only_proxy)
        .unwrap_or_else(|| {
            let lower = shortcut.to_lowercase();
            ai_tool_name_fragments()
                .iter()
                .any(|fragment| lower.contains(fragment))
        })
}

fn ai_tool_name_fragments() -> &'static [&'static str] {
    &["cursor", "codex", "antigravity", "kiro", "claude"]
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

fn process_match_score(app: &AppDefinition, process_name: &str, exe_path: &str) -> i32 {
    let mut score = match app.kind {
        AppKind::Desktop if looks_like_desktop_process(process_name, exe_path) => 100,
        AppKind::Cli if looks_like_cli_process(exe_path) => 100,
        AppKind::Desktop if looks_like_cli_process(exe_path) => -50,
        AppKind::Cli if looks_like_desktop_process(process_name, exe_path) => -50,
        _ => 0,
    };

    if normalize_match_text(executable_name(exe_path)) == normalize_match_text(process_name) {
        score += 10;
    }

    score
}

fn looks_like_desktop_process(process_name: &str, exe_path: &str) -> bool {
    let name = normalize_match_text(process_name);
    let path = normalize_match_text(exe_path);

    path.contains(".app/contents/macos")
        || path.contains("\\programs\\")
        || path.contains("\\program files\\")
        || name == "electron"
}

fn looks_like_cli_process(exe_path: &str) -> bool {
    let path = normalize_match_text(exe_path);

    path.contains("/bin/")
        || path.contains("/.local/bin/")
        || path.contains("\\bin\\")
        || path.ends_with(".cmd")
        || path.ends_with(".bat")
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

#[cfg(target_os = "macos")]
fn desktop_app_paths(app_name: &str, executable_names: &[&str]) -> Vec<PathCandidate> {
    let mut candidates = Vec::new();

    for executable in executable_names {
        candidates.push(p(&format!(
            "/Applications/{app_name}.app/Contents/MacOS/{executable}"
        )));
        candidates.push(home_path(
            &format!("~/Applications/{app_name}.app/Contents/MacOS/{executable}"),
            &format!("Applications/{app_name}.app/Contents/MacOS/{executable}"),
        ));
    }

    candidates
}

#[cfg(target_os = "macos")]
fn cli_paths(command: &str) -> Vec<PathCandidate> {
    vec![
        p(&format!("/opt/homebrew/bin/{command}")),
        p(&format!("/usr/local/bin/{command}")),
        home_path(
            &format!("~/.local/bin/{command}"),
            &format!(".local/bin/{command}"),
        ),
        home_path(
            &format!("~/.{command}/local/{command}"),
            &format!(".{command}/local/{command}"),
        ),
    ]
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
        "vscode-d" => vec![
            lp(r"Programs\Microsoft VS Code\Code.exe"),
            pp(r"Microsoft VS Code\Code.exe"),
            p86(r"Microsoft VS Code\Code.exe"),
            lp(r"Microsoft\WindowsApps\code.exe"),
        ],
        "code" => vec![
            lp(r"Programs\Microsoft VS Code\bin\code"),
            pp(r"Microsoft VS Code\bin\code"),
            p86(r"Microsoft VS Code\bin\code"),
        ],
        "cursor-d" => vec![
            lp(r"Programs\cursor\Cursor.exe"),
            lp(r"Programs\Cursor\Cursor.exe"),
            pp(r"Cursor\Cursor.exe"),
        ],
        "cursor" => vec![
            lp(r"Programs\cursor\resources\app\bin\cursor.cmd"),
            lp(r"Programs\Cursor\resources\app\bin\cursor.cmd"),
            lp(r"Programs\cursor\cursor.cmd"),
            pp(r"Cursor\cursor.cmd"),
        ],
        "codex-d" => vec![
            lp(r"Programs\codex\Codex.exe"),
            lp(r"Programs\Codex\Codex.exe"),
            pp(r"Codex\Codex.exe"),
            lp(r"Microsoft\WindowsApps\codex.exe"),
        ],
        "codex" => vec![
            lp(r"Programs\codex\codex.cmd"),
            pp(r"nodejs\codex.cmd"),
            lp(r"Microsoft\WindowsApps\codex.exe"),
        ],
        "antigravity-d" => vec![
            lp(r"Programs\antigravity ide\Antigravity IDE.exe"),
            lp(r"Programs\Antigravity IDE\Antigravity IDE.exe"),
            pp(r"Antigravity IDE\Antigravity IDE.exe"),
        ],
        "antigravity" => vec![
            lp(r"Programs\antigravity ide\bin\antigravity.cmd"),
            lp(r"Programs\Antigravity IDE\bin\antigravity.cmd"),
            pp(r"Antigravity IDE\bin\antigravity.cmd"),
        ],
        "kiro-d" => vec![
            lp(r"Programs\Kiro\Kiro.exe"),
            pp(r"Kiro\Kiro.exe"),
            lp(r"Microsoft\WindowsApps\kiro.exe"),
        ],
        "kiro" => vec![
            lp(r"Programs\Kiro\bin\kiro.cmd"),
            pp(r"Kiro\bin\kiro.cmd"),
            lp(r"Microsoft\WindowsApps\kiro.exe"),
        ],
        "claude-d" => vec![
            lp(r"Programs\Claude\Claude.exe"),
            pp(r"Claude\Claude.exe"),
            lp(r"Microsoft\WindowsApps\claude.exe"),
        ],
        "claude" => vec![
            lp(r"Programs\Claude\bin\claude.cmd"),
            pp(r"nodejs\claude.cmd"),
            lp(r"Microsoft\WindowsApps\claude.exe"),
        ],
        "rustrover-d" => vec![
            lp(r"Programs\RustRover\bin\rustrover64.exe"),
            pp(r"JetBrains\RustRover\bin\rustrover64.exe"),
        ],
        "rustrover" => vec![
            lp(r"Programs\RustRover\bin\rustrover.cmd"),
            pp(r"JetBrains\RustRover\bin\rustrover.cmd"),
        ],
        "vim" => vec![pp(r"Vim\vim91\vim.exe"), pp(r"Vim\vim90\vim.exe")],
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
            "vscode-d" => vec![
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
            "code" => vec![p("/opt/homebrew/bin/code"), p("/usr/local/bin/code")],
            "cursor-d" => vec![
                p("/Applications/Cursor.app/Contents/MacOS/Cursor"),
                home_path(
                    "~/Applications/Cursor.app/Contents/MacOS/Cursor",
                    "Applications/Cursor.app/Contents/MacOS/Cursor",
                ),
            ],
            "cursor" => vec![
                p("/opt/homebrew/bin/cursor"),
                p("/usr/local/bin/cursor"),
                home_path("~/.local/bin/cursor", ".local/bin/cursor"),
            ],
            "codex-d" => vec![
                p("/Applications/Codex.app/Contents/MacOS/Codex"),
                home_path(
                    "~/Applications/Codex.app/Contents/MacOS/Codex",
                    "Applications/Codex.app/Contents/MacOS/Codex",
                ),
            ],
            "codex" => vec![
                p("/opt/homebrew/bin/codex"),
                p("/usr/local/bin/codex"),
                home_path("~/.local/bin/codex", ".local/bin/codex"),
            ],
            "antigravity-d" => vec![
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
            "antigravity" => vec![
                p("/opt/homebrew/bin/antigravity"),
                p("/usr/local/bin/antigravity"),
                home_path("~/.local/bin/antigravity", ".local/bin/antigravity"),
            ],
            "kiro-d" => desktop_app_paths("Kiro", &["Kiro", "kiro", "Electron"]),
            "kiro" => cli_paths("kiro"),
            "claude-d" => desktop_app_paths("Claude", &["Claude", "claude", "Electron"]),
            "claude" => cli_paths("claude"),
            "rustrover-d" => vec![
                p("/Applications/RustRover.app/Contents/MacOS/rustrover"),
                p("/Applications/RustRover.app/Contents/MacOS/RustRover"),
                home_path(
                    "~/Applications/RustRover.app/Contents/MacOS/rustrover",
                    "Applications/RustRover.app/Contents/MacOS/rustrover",
                ),
            ],
            "rustrover" => vec![
                p("/opt/homebrew/bin/rustrover"),
                p("/usr/local/bin/rustrover"),
                home_path("~/.local/bin/rustrover", ".local/bin/rustrover"),
            ],
            "vim" => vec![
                p("/usr/bin/vim"),
                p("/opt/homebrew/bin/vim"),
                p("/usr/local/bin/vim"),
            ],
            _ => vec![],
        }
    }

    #[cfg(target_os = "linux")]
    {
        match shortcut {
            "vscode-d" => vec![
                p("/usr/share/code/code"),
                p("/opt/visual-studio-code/code"),
                p("/snap/bin/code"),
            ],
            "code" => vec![
                p("/usr/bin/code"),
                p("/usr/local/bin/code"),
                p("/snap/bin/code"),
            ],
            "cursor-d" => vec![
                p("/opt/cursor/cursor"),
                p("/usr/bin/cursor"),
                p("/usr/local/bin/cursor"),
            ],
            "cursor" => vec![
                p("/usr/bin/cursor"),
                p("/usr/local/bin/cursor"),
                home_path("~/.local/bin/cursor", ".local/bin/cursor"),
            ],
            "codex-d" => vec![p("/opt/codex/codex")],
            "codex" => vec![
                p("/usr/bin/codex"),
                p("/usr/local/bin/codex"),
                home_path("~/.local/bin/codex", ".local/bin/codex"),
            ],
            "antigravity-d" => vec![
                p("/opt/antigravity/antigravity"),
                p("/opt/antigravity-ide/antigravity-ide"),
                p("/usr/local/bin/antigravity"),
                p("/usr/local/bin/antigravity-ide"),
            ],
            "antigravity" => vec![
                p("/usr/bin/antigravity"),
                p("/usr/local/bin/antigravity"),
                p("/usr/bin/antigravity-ide"),
                p("/usr/local/bin/antigravity-ide"),
                home_path("~/.local/bin/antigravity", ".local/bin/antigravity"),
            ],
            "kiro-d" => vec![
                p("/opt/kiro/kiro"),
                p("/usr/local/bin/kiro"),
                p("/usr/bin/kiro"),
            ],
            "kiro" => vec![
                p("/usr/bin/kiro"),
                p("/usr/local/bin/kiro"),
                home_path("~/.local/bin/kiro", ".local/bin/kiro"),
            ],
            "claude-d" => vec![
                p("/opt/claude/claude"),
                p("/usr/local/bin/claude"),
                p("/usr/bin/claude"),
            ],
            "claude" => vec![
                p("/usr/bin/claude"),
                p("/usr/local/bin/claude"),
                home_path("~/.local/bin/claude", ".local/bin/claude"),
            ],
            "rustrover-d" => vec![
                p("/opt/rustrover/bin/rustrover"),
                p("/usr/local/bin/rustrover"),
            ],
            "rustrover" => vec![
                p("/usr/bin/rustrover"),
                p("/usr/local/bin/rustrover"),
                home_path("~/.local/bin/rustrover", ".local/bin/rustrover"),
            ],
            "vim" => vec![p("/usr/bin/vim"), p("/usr/local/bin/vim")],
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
        assert_eq!(app.shortcut, "cursor-d");
        assert!(app.ai_only_proxy);
    }

    #[test]
    fn identifies_antigravity_process() {
        let app = identify_process(
            "Antigravity IDE",
            r"C:\Users\me\AppData\Local\Programs\Antigravity IDE\Antigravity IDE.exe",
        )
        .expect("antigravity should be known");
        assert_eq!(app.shortcut, "antigravity-d");
    }

    #[test]
    fn recognizes_ai_tool_shortcuts() {
        assert!(is_ai_tool("codex"));
        assert!(is_ai_tool("cursor-d"));
        assert!(is_ai_tool("kiro"));
        assert!(is_ai_tool("claude-d"));
        assert!(is_ai_tool("my-antigravity-wrapper"));
        assert!(!is_ai_tool("vscode-d"));
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
            "codex"
        );
        assert!(identify_process(
            "node_repl",
            "/Applications/Codex.app/Contents/Resources/cua_node/bin/node_repl"
        )
        .is_none());
        assert!(identify_process(
            "PasscodeSettingsSubscriber",
            "/System/PasscodeSettingsSubscriber"
        )
        .is_none());
        assert!(identify_process("VTDecoderXPCService", "/System/VTDecoderXPCService").is_none());
    }

    #[test]
    fn ignores_macos_cursor_ui_service() {
        assert!(identify_process(
            "CursorUIViewService",
            "/System/Library/PrivateFrameworks/TextInputUIMacHelper.framework/Versions/A/XPCServices/CursorUIViewService.xpc/Contents/MacOS/CursorUIViewService",
        )
        .is_none());
    }

    #[test]
    fn distinguishes_codex_desktop_from_cli() {
        assert_eq!(
            identify_process("Codex", "/Applications/Codex.app/Contents/MacOS/Codex")
                .expect("desktop codex should be known")
                .shortcut,
            "codex-d"
        );
        assert_eq!(
            identify_process("codex", "/Users/me/.local/bin/codex")
                .expect("cli codex should be known")
                .shortcut,
            "codex"
        );
    }

    #[test]
    fn identifies_kiro_and_claude_variants() {
        assert_eq!(
            identify_process("Kiro", "/Applications/Kiro.app/Contents/MacOS/Kiro")
                .expect("desktop kiro should be known")
                .shortcut,
            "kiro-d"
        );
        assert_eq!(
            identify_process("kiro", "/Users/me/.local/bin/kiro")
                .expect("cli kiro should be known")
                .shortcut,
            "kiro"
        );
        assert_eq!(
            identify_process("Claude", "/Applications/Claude.app/Contents/MacOS/Claude")
                .expect("desktop claude should be known")
                .shortcut,
            "claude-d"
        );
        assert_eq!(
            identify_process("claude", "/Users/me/.claude/local/claude")
                .expect("cli claude should be known")
                .shortcut,
            "claude"
        );
    }
}
