//! commands/register.rs — Handler for `px register`.
//!
//! Upserts a named shortcut entry (path + execution class) into the `[apps]`
//! table of `config.toml`.
//!
//! ## Two usage modes
//!
//! **Explicit path** (`--path <path>`):
//! ```text
//! px register --name cursor-desktop --path "C:\...\Cursor.exe" --kind desktop
//! ```
//!
//! **Interactive detection** (no `--path`):
//! ```text
//! px register --name my-tool --kind desktop
//! ```
//! px snapshots the running process list, waits for you to open the app,
//! re-scans, then presents only the *new* processes as a numbered pick list.
//! If the app was already running it falls back to showing all running apps.

use std::collections::HashSet;
use std::io::{self, Write};

use anyhow::{bail, Result};
use colored::Colorize;
use sysinfo::{Pid, System};

use crate::commands::discover::is_electron_subprocess;
use crate::config::{self, AppEntry, AppKind};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Add or update an app shortcut in `config.toml`.
///
/// `path = None` triggers the interactive process scanner.
pub fn run(name: &str, path: Option<&str>, kind: AppKind) -> Result<()> {
    let resolved_path = match path {
        Some(p) => p.to_string(),
        None    => interactive_pick(name)?,
    };

    let mut cfg = config::load()?;
    let is_update = cfg.apps.contains_key(name);

    cfg.apps.insert(
        name.to_string(),
        AppEntry {
            path: resolved_path.clone(),
            kind,
            args: vec![],
        },
    );
    config::save(&cfg)?;

    let action = if is_update { "Updated" } else { "Registered" };
    println!(
        "\n{} {} '{}' → '{}' ({})",
        "✔".green().bold(),
        action,
        name.cyan(),
        resolved_path,
        kind.to_string().dimmed(),
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive process scanner
// ---------------------------------------------------------------------------

/// Snapshot → prompt → re-scan → numbered pick list.
fn interactive_pick(name_hint: &str) -> Result<String> {
    println!(
        "{} No path provided — starting interactive process scanner.\n",
        "ℹ".cyan().bold()
    );

    // ── Step 1: record all PIDs already running ──────────────────────────────
    let before_pids: HashSet<Pid> = {
        let snap = System::new_all();
        snap.processes().keys().copied().collect()
    };

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
    let sys = System::new_all();

    // Filter: only new PIDs, no Electron subprocesses, must have an exe path.
    let mut candidates: Vec<_> = sys
        .processes()
        .values()
        .filter(|p| !before_pids.contains(&p.pid()))
        .filter(|p| !is_electron_subprocess(p))
        .filter(|p| p.exe().is_some())
        .collect();

    // ── Fallback: if nothing new appeared (app was already running) ───────────
    let was_already_running = candidates.is_empty();
    if was_already_running {
        println!(
            "  {} No new processes detected — the app may already have been open.\n  {} Showing all running apps instead:\n",
            "ℹ".cyan(),
            "→".yellow(),
        );
        candidates = sys
            .processes()
            .values()
            .filter(|p| !is_electron_subprocess(p))
            .filter(|p| p.exe().is_some())
            .collect();
    }

    if candidates.is_empty() {
        bail!("No processes found. Make sure the app is running and try again.");
    }

    // Sort alphabetically for a stable, readable list.
    candidates.sort_by_key(|p| p.name().to_lowercase());

    // ── Step 4: display the numbered pick list ────────────────────────────────
    let name_col = candidates
        .iter()
        .map(|p| p.name().len())
        .max()
        .unwrap_or(12);

    for (i, proc) in candidates.iter().enumerate() {
        println!(
            "    {} {:<name_col$}  {}",
            format!("[{}]", i + 1).bold(),
            proc.name().cyan(),
            proc.exe().unwrap().display().to_string().dimmed(),
            name_col = name_col,
        );
    }

    // ── Step 5: read and validate the user's choice ───────────────────────────
    println!();
    print!(
        "  Enter number to select (or {} to cancel): ",
        "0".dimmed()
    );
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice == 0 || choice > candidates.len() {
        bail!("Cancelled.");
    }

    let selected = candidates[choice - 1];
    Ok(selected.exe().unwrap().to_string_lossy().into_owned())
}
