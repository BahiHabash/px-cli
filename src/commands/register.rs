//! commands/register.rs — Handler for `px register`.
//!
//! Upserts a named shortcut entry (path + execution class + ai_only_proxy)
//! into the `[apps]` table of `config.toml`.
//!
//! ## Two usage modes
//!
//! **Explicit path** (`--path <path>`):
//! ```text
//! px register --name cursor-d --path "C:\...\Cursor.exe" --kind desktop
//! ```
//!
//! **Interactive detection** (no `--path`):
//! ```text
//! px register --name my-tool --kind desktop
//! ```
//! px snapshots the running process list, waits for you to open the app,
//! re-scans, then presents only the *new* processes as a numbered pick list.
//! If the app was already running it falls back to showing all running apps.

use std::io::{self, Write};

use anyhow::{bail, Result};
use colored::Colorize;

use crate::app_registry;
use crate::config::{self, AppEntry, AppKind};
use crate::process_scan::{self, ProcessMatch};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Add or update an app shortcut in `config.toml`.
///
/// `path = None` triggers the interactive process scanner.
/// `ai_only` sets `ai_only_proxy = true` on the entry.
pub fn run(
    name: &str,
    path: Option<&str>,
    search: Option<&str>,
    kind: Option<AppKind>,
    ai_only: bool,
) -> Result<()> {
    let entry = match path {
        Some(p) => AppEntry {
            path: p.to_string(),
            kind: kind.unwrap_or(AppKind::Cli),
            args: vec![],
            ai_only_proxy: ai_only || app_registry::is_ai_tool(name),
        },
        None => {
            let selected = match search {
                Some(q) => search_running_processes(q)?,
                None => interactive_pick(name)?,
            };

            AppEntry {
                path: selected.path,
                kind: kind.unwrap_or(selected.kind),
                args: selected.args,
                ai_only_proxy: ai_only || selected.ai_only_proxy || app_registry::is_ai_tool(name),
            }
        }
    };

    let mut cfg = config::load()?;
    let is_update = cfg.apps.contains_key(name);

    cfg.apps.insert(name.to_string(), entry.clone());
    config::save(&cfg)?;

    let action = if is_update { "Updated" } else { "Registered" };
    let ai_label = if entry.ai_only_proxy {
        format!(" {}", "[ai-only]".cyan().bold())
    } else {
        String::new()
    };
    let args_label = if entry.args.is_empty() {
        String::new()
    } else {
        format!(" args={}", format!("[{}]", entry.args.join(", ")).dimmed())
    };
    println!(
        "\n{} {} '{}' → '{}' ({}){}{}",
        "✔".green().bold(),
        action,
        name.cyan(),
        entry.path,
        entry.kind.to_string().dimmed(),
        ai_label,
        args_label,
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive process scanner
// ---------------------------------------------------------------------------

/// Snapshot → prompt → re-scan → numbered pick list.
fn interactive_pick(name_hint: &str) -> Result<ProcessMatch> {
    println!(
        "{} No path provided — starting interactive process scanner.\n",
        "ℹ".cyan().bold()
    );

    // ── Step 1: record all PIDs already running ──────────────────────────────
    let before_pids = process_scan::snapshot_pids();

    // ── Step 2: prompt the user to open the target app ───────────────────────
    println!(
        "  {} Open {} now, then press {}.",
        "→".yellow(),
        format!("'{}'", name_hint).cyan().bold(),
        "[ENTER]".bold()
    );
    print!("  > ");
    io::stdout().flush().ok();
    let mut _buf = String::new();
    io::stdin().read_line(&mut _buf).ok();

    // ── Step 3: re-scan after the user opened the app ────────────────────────
    println!("\n{} Scanning for new processes …\n", "⟳".yellow().bold());
    let mut candidates = process_scan::new_since(&before_pids);

    // ── Fallback: if nothing new appeared (app was already running) ───────────
    let was_already_running = candidates.is_empty();
    if was_already_running {
        let fallback_query = name_hint
            .split(|c| c == '-' || c == '_')
            .next()
            .unwrap_or(name_hint)
            .to_lowercase();

        println!(
            "  {} No new processes detected — the app may already have been open.\n  {} Automatically searching running apps for '{}':\n",
            "ℹ".cyan(),
            "→".yellow(),
            fallback_query
        );
        candidates = process_scan::search(&fallback_query);
    }

    if candidates.is_empty() {
        bail!("No processes found. Make sure the app is running and try again.");
    }

    // ── Step 4: display the numbered pick list ────────────────────────────────
    let name_col = candidates
        .iter()
        .map(|p| p.process_name.len())
        .max()
        .unwrap_or(12);

    for (i, proc) in candidates.iter().enumerate() {
        let detected = proc
            .detected_shortcut
            .map(|shortcut| format!(" {}", format!("[{}]", shortcut).cyan()))
            .unwrap_or_default();
        println!(
            "    {} {:<name_col$}  {}{}",
            format!("[{}]", i + 1).bold(),
            proc.process_name.cyan(),
            proc.path.dimmed(),
            detected,
            name_col = name_col,
        );
    }

    // ── Step 5: read and validate the user's choice ───────────────────────────
    println!();
    print!("  Enter number to select (or {} to cancel): ", "0".dimmed());
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > candidates.len() {
        bail!("Cancelled.");
    }

    Ok(candidates.remove(choice - 1))
}

// ---------------------------------------------------------------------------
// Dynamic Search process scanner
// ---------------------------------------------------------------------------

/// Scan running processes and filter by a search query (case-insensitive)
fn search_running_processes(query: &str) -> Result<ProcessMatch> {
    println!(
        "{} Searching running processes for '{}' …\n",
        "⟳".yellow().bold(),
        query.cyan()
    );

    let mut candidates = process_scan::search(query);

    if candidates.is_empty() {
        bail!(
            "No running processes found matching '{}'. Make sure the app is running.",
            query
        );
    }

    let name_col = candidates
        .iter()
        .map(|p| p.process_name.len())
        .max()
        .unwrap_or(12);

    for (i, proc) in candidates.iter().enumerate() {
        let detected = proc
            .detected_shortcut
            .map(|shortcut| format!(" {}", format!("[{}]", shortcut).cyan()))
            .unwrap_or_default();
        println!(
            "    {} {:<name_col$}  {}{}",
            format!("[{}]", i + 1).bold(),
            proc.process_name.cyan(),
            proc.path.dimmed(),
            detected,
            name_col = name_col,
        );
    }

    println!();
    print!("  Enter number to select (or {} to cancel): ", "0".dimmed());
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > candidates.len() {
        bail!("Cancelled.");
    }

    Ok(candidates.remove(choice - 1))
}
