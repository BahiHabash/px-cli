//! commands/discover.rs — Cross-platform auto-discovery engine.
//!
//! Discovery has two phases:
//! 1. Static scan of known install locations from `app_registry`.
//! 2. Dynamic process scan for important desktop apps that are currently open.

use std::collections::HashSet;
use std::io::{self, Write};

use colored::Colorize;

use crate::app_registry::{self, AppDefinition};
use crate::config::AppEntry;
use crate::process_scan;

/// A single application found during auto-discovery.
pub struct DiscoveredApp {
    /// The shortcut name written to `[apps.<shortcut>]` in config.toml.
    pub shortcut: &'static str,
    /// The resolved executable path, execution class, and optional saved args.
    pub entry: AppEntry,
}

fn static_scan() -> Vec<DiscoveredApp> {
    app_registry::known_apps()
        .iter()
        .filter_map(|app| {
            println!(
                "\n  {} {} {}",
                "▸".cyan().bold(),
                app.shortcut.bold(),
                format!("({})", app.kind).dimmed()
            );

            let mut found_template: Option<String> = None;
            for candidate in app.path_candidates() {
                if candidate.evaluated.exists() {
                    println!(
                        "      {} {}",
                        "✔".green().bold(),
                        candidate.evaluated.display().to_string().dimmed()
                    );
                    found_template = Some(candidate.template);
                    break;
                }

                println!(
                    "      {} {}",
                    "✘".red().dimmed(),
                    candidate.evaluated.display().to_string().dimmed()
                );
            }

            let Some(template) = found_template else {
                println!("      {} not found on this machine", "→".yellow());
                return None;
            };

            if app.ai_only_proxy {
                println!("      {} ai-only proxy mode enabled", "★".cyan());
            }

            Some(DiscoveredApp {
                shortcut: app.shortcut,
                entry: AppEntry {
                    path: template,
                    kind: app.kind,
                    args: vec![],
                    ai_only_proxy: app.ai_only_proxy,
                },
            })
        })
        .collect()
}

fn process_to_discovered(process: process_scan::ProcessMatch) -> Option<DiscoveredApp> {
    let shortcut = process.detected_shortcut?;
    Some(DiscoveredApp {
        shortcut,
        entry: AppEntry {
            path: process.path,
            kind: process.kind,
            args: process.args,
            ai_only_proxy: process.ai_only_proxy,
        },
    })
}

fn process_scan_for(missing: &[&'static AppDefinition]) -> Vec<DiscoveredApp> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for process in process_scan::known_running(missing) {
        let Some(shortcut) = process.detected_shortcut else {
            continue;
        };

        if !seen.insert(shortcut) {
            continue;
        }

        let args_label = if process.args.is_empty() {
            String::new()
        } else {
            format!(
                "  args: {}",
                format!("[{}]", process.args.join(", ")).yellow()
            )
        };
        let ai_label = if process.ai_only_proxy {
            format!(" {}", "[ai-only]".cyan())
        } else {
            String::new()
        };

        println!(
            "    {} {} → {}{}{}",
            "✔".green().bold(),
            shortcut.cyan(),
            process.path.dimmed(),
            args_label,
            ai_label,
        );

        if let Some(app) = process_to_discovered(process) {
            results.push(app);
        }
    }

    for app in missing {
        if !seen.contains(app.shortcut) {
            println!(
                "    {} {} — not running (skipped)",
                "✘".red().dimmed(),
                app.shortcut.dimmed()
            );
        }
    }

    results
}

fn prompt_and_scan(missing: &[&'static AppDefinition]) -> Vec<DiscoveredApp> {
    if missing.is_empty() {
        return vec![];
    }

    println!(
        "\n{} Some apps were not auto-discovered:",
        "⚠".yellow().bold()
    );
    for app in missing {
        println!("  • {}", app.shortcut.cyan());
    }
    println!(
        "\n  {} If any are installed, open them now, then press {} to scan running processes.",
        "→".yellow(),
        "[ENTER]".bold()
    );
    print!("  > ");
    io::stdout().flush().ok();
    let mut _buf = String::new();
    io::stdin().read_line(&mut _buf).ok();

    println!("\n{} Scanning running processes …\n", "⟳".yellow().bold());
    process_scan_for(missing)
}

/// Orchestrates static and dynamic discovery.
///
/// Existing shortcuts are filtered by `init`; this function reports everything
/// it can detect on the machine.
pub fn scan() -> Vec<DiscoveredApp> {
    let found = static_scan();
    let found_shortcuts: HashSet<&str> = found.iter().map(|app| app.shortcut).collect();
    let missing: Vec<&'static AppDefinition> = app_registry::core_desktop_apps()
        .filter(|app| !found_shortcuts.contains(app.shortcut))
        .collect();

    let running = process_scan_for(&missing);
    let running_shortcuts: HashSet<&str> = running.iter().map(|app| app.shortcut).collect();
    let still_missing: Vec<&'static AppDefinition> = missing
        .into_iter()
        .filter(|app| !running_shortcuts.contains(app.shortcut))
        .collect();

    let mut all = found;
    all.extend(running);
    all.extend(prompt_and_scan(&still_missing));
    all
}
