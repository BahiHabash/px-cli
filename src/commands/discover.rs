//! commands/discover.rs — Cross-platform auto-discovery engine.
//!
//! ## Two-phase discovery
//!
//! **Phase 1 — Static scan:** checks a curated list of well-known install
//! locations per platform.  Paths are stored as portable env-var templates
//! (e.g. `%LOCALAPPDATA%\Programs\cursor\Cursor.exe`) so the config is
//! transferable between machines.
//!
//! **Phase 2 — Dynamic process scan (fallback):** if any core apps are still
//! missing after the static scan, the user is prompted to open them manually.
//! `sysinfo` then reads the live process list and extracts their real executable
//! paths and any captured launch flags (e.g. `--disable-gpu`).
//!
//! ## Windows Store / MSIX path guardrail
//!
//! Store apps expose two binary locations:
//!
//! | Location | Accessible? | Why |
//! |---|---|---|
//! | `C:\Program Files\WindowsApps\…` | ❌ | ACL-locked; `Command::new` raises "Access Denied" |
//! | `%LOCALAPPDATA%\Microsoft\WindowsApps\…` | ✔ | Execution aliases — reparse points that redirect to the real binary |
//!
//! When the process scanner detects a path inside the locked `\WindowsApps\`
//! directory, it automatically redirects to the user-level execution alias and
//! warns the user.

use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;

use colored::Colorize;
use sysinfo::{Process, System};

use crate::config::{AppEntry, AppKind};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single application found during auto-discovery.
pub struct DiscoveredApp {
    /// The shortcut name written to `[apps.<shortcut>]` in config.toml.
    pub shortcut: &'static str,
    /// The resolved executable path, execution class, and optional saved args.
    pub entry: AppEntry,
}

// ---------------------------------------------------------------------------
// Static scan — internal types
// ---------------------------------------------------------------------------

struct Candidate {
    shortcut: &'static str,
    kind: AppKind,
    /// `(template_saved_to_config, evaluated_path_checked_on_disk)`
    ///
    /// The template uses env-var tokens so the config is portable.
    /// The evaluated path is what we actually `stat` to detect presence.
    paths: Vec<(String, PathBuf)>,
}

/// Convenience helper for non-Windows: template == evaluated path.
#[cfg(not(target_os = "windows"))]
fn p(raw: &str) -> (String, PathBuf) {
    (raw.to_string(), PathBuf::from(raw))
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
            paths: vec![p("/Applications/Visual Studio Code.app/Contents/MacOS/Electron")],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/local/bin/code"),
                p("/opt/homebrew/bin/code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![p("/Applications/Cursor.app/Contents/MacOS/Cursor")],
        },
        Candidate {
            shortcut: "cursor-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/local/bin/cursor"),
                p("/opt/homebrew/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![p("/Applications/Codex.app/Contents/MacOS/Codex")],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/local/bin/codex"),
                p("/opt/homebrew/bin/codex"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![p("/Applications/Antigravity.app/Contents/MacOS/Antigravity")],
        },
    ]
}

#[cfg(target_os = "windows")]
fn candidates() -> Vec<Candidate> {
    // Resolve base directories at runtime so no username is ever hardcoded.
    let prog   = PathBuf::from(std::env::var("ProgramFiles")
        .unwrap_or_else(|_| r"C:\Program Files".into()));
    let prog86 = PathBuf::from(std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| r"C:\Program Files (x86)".into()));
    // %LOCALAPPDATA% = C:\Users\<current-user>\AppData\Local
    let local  = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Local"));

    // Helper: build a (template, evaluated_path) pair from a %TOKEN% prefix.
    let lp = |token: &str, sub: &str| -> (String, PathBuf) {
        (format!(r"{}\{}", token, sub), local.join(sub))
    };
    let pp = |token: &str, sub: &str| -> (String, PathBuf) {
        (format!(r"{}\{}", token, sub), prog.join(sub))
    };
    let p86 = |token: &str, sub: &str| -> (String, PathBuf) {
        (format!(r"{}\{}", token, sub), prog86.join(sub))
    };

    vec![
        Candidate {
            shortcut: "vscode-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                // ① User install (most common — no admin rights required)
                lp(r"%LOCALAPPDATA%", r"Programs\Microsoft VS Code\Code.exe"),
                // ② System-wide installs
                pp(r"%ProgramFiles%", r"Microsoft VS Code\Code.exe"),
                p86(r"%ProgramFiles(x86)%", r"Microsoft VS Code\Code.exe"),
                // ③ Windows Store execution alias
                //    NOTE: We target %LOCALAPPDATA%\Microsoft\WindowsApps\ rather than
                //    C:\Program Files\WindowsApps\ because the latter is ACL-locked and
                //    will raise "Access Denied" when passed to Command::new().
                lp(r"%LOCALAPPDATA%", r"Microsoft\WindowsApps\code.exe"),
            ],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                lp(r"%LOCALAPPDATA%", r"Programs\Microsoft VS Code\bin\code"),
                pp(r"%ProgramFiles%", r"Microsoft VS Code\bin\code"),
                p86(r"%ProgramFiles(x86)%", r"Microsoft VS Code\bin\code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                lp(r"%LOCALAPPDATA%", r"Programs\cursor\Cursor.exe"),
                lp(r"%LOCALAPPDATA%", r"Programs\Cursor\Cursor.exe"),
                pp(r"%ProgramFiles%", r"Cursor\Cursor.exe"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                lp(r"%LOCALAPPDATA%", r"Programs\codex\Codex.exe"),
                lp(r"%LOCALAPPDATA%", r"Programs\Codex\Codex.exe"),
                pp(r"%ProgramFiles%", r"Codex\Codex.exe"),
                // Windows Store execution alias
                lp(r"%LOCALAPPDATA%", r"Microsoft\WindowsApps\codex.exe"),
            ],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                lp(r"%LOCALAPPDATA%", r"Programs\codex\codex.cmd"),
                pp(r"%ProgramFiles%", r"nodejs\codex.cmd"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![
                lp(r"%LOCALAPPDATA%", r"Programs\antigravity\Antigravity.exe"),
                lp(r"%LOCALAPPDATA%", r"Programs\Antigravity\Antigravity.exe"),
                pp(r"%ProgramFiles%", r"Antigravity\Antigravity.exe"),
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
                p("/usr/share/code/code"),
                p("/opt/visual-studio-code/code"),
                p("/snap/bin/code"),
            ],
        },
        Candidate {
            shortcut: "vscode-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/bin/code"),
                p("/usr/local/bin/code"),
                p("/snap/bin/code"),
            ],
        },
        Candidate {
            shortcut: "cursor-desktop",
            kind: AppKind::Desktop,
            paths: vec![
                p("/opt/cursor/cursor"),
                p("/usr/local/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "cursor-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/local/bin/cursor"),
                p("/usr/bin/cursor"),
            ],
        },
        Candidate {
            shortcut: "codex-desktop",
            kind: AppKind::Desktop,
            paths: vec![p("/opt/codex/codex")],
        },
        Candidate {
            shortcut: "codex-cli",
            kind: AppKind::Cli,
            paths: vec![
                p("/usr/local/bin/codex"),
                p("/usr/bin/codex"),
            ],
        },
        Candidate {
            shortcut: "antigravity",
            kind: AppKind::Desktop,
            paths: vec![
                p("/opt/antigravity/antigravity"),
                p("/usr/local/bin/antigravity"),
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
// Phase 1: Static scan
// ---------------------------------------------------------------------------

fn static_scan() -> Vec<DiscoveredApp> {
    candidates()
        .into_iter()
        .filter_map(|candidate| {
            // Header: app name + kind so the user knows what we are looking for.
            println!(
                "\n  {} {} {}",
                "▸".cyan().bold(),
                candidate.shortcut.bold(),
                format!("({})", candidate.kind).dimmed()
            );

            let mut found_template: Option<String> = None;

            for (template, eval_path) in candidate.paths {
                if eval_path.exists() {
                    println!("      {} {}", "✔".green().bold(), eval_path.display().to_string().dimmed());
                    found_template = Some(template);
                    break;
                } else {
                    println!("      {} {}", "✘".red().dimmed(), eval_path.display().to_string().dimmed());
                }
            }

            // Trailing status per app.
            if found_template.is_none() {
                println!("      {} not found on this machine", "→".yellow());
            }

            let template = found_template?;

            Some(DiscoveredApp {
                shortcut: candidate.shortcut,
                entry: AppEntry {
                    path: template,
                    kind: candidate.kind,
                    args: vec![],
                },
            })
        })
        .collect()
}


// ---------------------------------------------------------------------------
// Phase 2: Dynamic process scan
// ---------------------------------------------------------------------------

/// Returns `true` if this process is an Electron renderer / gpu / crashpad
/// subprocess rather than the main app process.
///
/// Every Chromium-based subprocess receives a `--type=<role>` flag; the main
/// process never does.  Reused by both the init scanner and `px register`.
pub(crate) fn is_electron_subprocess(process: &Process) -> bool {
    process.cmd().iter().skip(1).any(|a| a.starts_with("--type="))
}

/// The core apps we always want to find.  Only desktop-class apps are
/// meaningful here — CLI tools exit too quickly to show up reliably.
struct CoreApp {
    shortcut: &'static str,
    /// Executable name to match against running processes (without extension).
    /// Matching is case-insensitive; `.exe` suffix is stripped before comparison.
    exe_name: &'static str,
    kind: AppKind,
}

static CORE_APPS: &[CoreApp] = &[
    CoreApp { shortcut: "vscode-desktop",  exe_name: "code",        kind: AppKind::Desktop },
    CoreApp { shortcut: "cursor-desktop",  exe_name: "cursor",      kind: AppKind::Desktop },
    CoreApp { shortcut: "codex-desktop",   exe_name: "codex",       kind: AppKind::Desktop },
    CoreApp { shortcut: "antigravity",     exe_name: "antigravity", kind: AppKind::Desktop },
];

/// On Windows, detects whether a process path is the ACL-locked internal
/// Store binary rather than the user-accessible execution alias.
///
/// - `C:\Program Files\WindowsApps\…`        → locked (returns `true`)
/// - `%LOCALAPPDATA%\Microsoft\WindowsApps\…` → safe alias (returns `false`)
#[cfg(target_os = "windows")]
fn is_locked_store_path(path_lower: &str) -> bool {
    path_lower.contains("\\windowsapps\\")
        && !path_lower.contains("\\local\\microsoft\\windowsapps\\")
}

/// Prompts the user to open missing apps, then scans the live process list.
fn process_scan(missing: &[&CoreApp]) -> Vec<DiscoveredApp> {
    // --- User prompt --------------------------------------------------------
    println!("\n{} Some apps were not auto-discovered:", "⚠".yellow().bold());
    for app in missing {
        println!("  • {}", app.shortcut.cyan());
    }
    println!(
        "\n  {} Open the missing apps right now, then press {} to scan running processes.",
        "→".yellow(),
        "[ENTER]".bold()
    );
    print!("  > ");
    io::stdout().flush().ok();
    let mut _buf = String::new();
    io::stdin().read_line(&mut _buf).ok();

    // --- sysinfo scan -------------------------------------------------------
    println!("\n{} Scanning running processes …\n", "⟳".yellow().bold());

    let sys = System::new_all();
    let mut results: Vec<DiscoveredApp> = Vec::new();

    'app: for core_app in missing {
        for (_pid, process) in sys.processes() {
            // Normalise name: lowercase + strip .exe for cross-platform matching.
            let name_lower = process.name().to_lowercase();
            let name_stem  = name_lower.trim_end_matches(".exe");

            if name_stem != core_app.exe_name {
                continue;
            }

            let Some(exe_path) = process.exe() else { continue };

            // ── Electron subprocess guard ────────────────────────────────────
            // See `is_electron_subprocess` for full explanation.
            // Check before collecting args to skip the Vec allocation entirely.
            if is_electron_subprocess(process) {
                continue;
            }

            // Capture all argv[1..] as saved launch args.
            // argv[0] is the executable itself — skip it.
            let args: Vec<String> = process
                .cmd()
                .iter()
                .skip(1)
                .cloned()
                .collect();

            let path_to_save: String;

            // ── Windows Store path guardrail ────────────────────────────────
            #[cfg(target_os = "windows")]
            {
                let path_lower = exe_path.to_string_lossy().to_lowercase();
                if is_locked_store_path(&path_lower) {
                    // Detected the ACL-locked internal binary.  Redirect to the
                    // user-level execution alias which Command::new can spawn.
                    let alias = format!(
                        r"%LOCALAPPDATA%\Microsoft\WindowsApps\{}.exe",
                        core_app.exe_name
                    );
                    println!(
                        "    {} {} {} locked Store path detected:",
                        "⚠".yellow().bold(),
                        core_app.shortcut.cyan(),
                        "—".dimmed()
                    );
                    println!("      actual : {}", exe_path.display().to_string().dimmed());
                    println!(
                        "      saving  : {} {}",
                        alias.dimmed(),
                        "(execution alias)".yellow()
                    );
                    path_to_save = alias;
                    // Fall through to push result below.
                } else {
                    path_to_save = exe_path.to_string_lossy().into_owned();
                }
            }

            // ── Non-Windows: use the exe path directly ──────────────────────
            #[cfg(not(target_os = "windows"))]
            {
                path_to_save = exe_path.to_string_lossy().into_owned();
            }

            // --- Print result -----------------------------------------------
            let args_label = if args.is_empty() {
                String::new()
            } else {
                format!("  args: {}", format!("[{}]", args.join(", ")).yellow())
            };

            println!(
                "    {} {} → {}{}",
                "✔".green().bold(),
                core_app.shortcut.cyan(),
                path_to_save.dimmed(),
                args_label
            );

            results.push(DiscoveredApp {
                shortcut: core_app.shortcut,
                entry: AppEntry {
                    path: path_to_save,
                    kind: core_app.kind,
                    args,
                },
            });

            continue 'app; // move on to the next core app
        }

        // No matching process found.
        println!(
            "    {} {} — not running (skipped)",
            "✘".red().dimmed(),
            core_app.shortcut.dimmed()
        );
    }

    results
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Orchestrates both discovery phases and returns all found apps.
///
/// Existing shortcuts in `config.toml` are **not** overwritten — callers must
/// filter against their existing map before inserting.
pub fn scan() -> Vec<DiscoveredApp> {
    // Phase 1: static filesystem scan.
    let found = static_scan();

    // Phase 2: check which core desktop apps are still unaccounted for.
    let found_shortcuts: HashSet<&str> = found.iter().map(|a| a.shortcut).collect();
    let missing: Vec<&CoreApp> = CORE_APPS
        .iter()
        .filter(|a| !found_shortcuts.contains(a.shortcut))
        .collect();

    if missing.is_empty() {
        return found;
    }

    // Phase 2: interactive process scan for any remaining apps.
    let mut all = found;
    all.extend(process_scan(&missing));
    all
}
