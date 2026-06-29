//! commands/ps.rs — Inspect running processes that px can register.

use colored::Colorize;

use crate::process_scan::{self, ProcessMatch};

pub fn run(search: Option<&str>, known_only: bool) -> anyhow::Result<()> {
    let mut processes = match search {
        Some(query) => process_scan::search(query),
        None => process_scan::all_registerable(),
    };

    if known_only {
        processes.retain(|process| process.detected_shortcut.is_some());
    }

    if processes.is_empty() {
        let scope = match (search, known_only) {
            (Some(query), true) => format!("known processes matching '{}'", query),
            (Some(query), false) => format!("processes matching '{}'", query),
            (None, true) => "known running developer tools".to_string(),
            (None, false) => "registerable running processes".to_string(),
        };
        println!("{} No {} found.", "ℹ".cyan().bold(), scope);
        return Ok(());
    }

    print_processes(&processes);
    Ok(())
}

fn print_processes(processes: &[ProcessMatch]) {
    let name_col = processes
        .iter()
        .map(|process| process.process_name.len())
        .max()
        .unwrap_or(12)
        .max("process".len());

    let detected_col = processes
        .iter()
        .map(|process| process.detected_shortcut.unwrap_or("-").len())
        .max()
        .unwrap_or(8)
        .max("detected".len());

    println!(
        "{:<8} {:<name_col$} {:<detected_col$} {:<8} {:<8} {}",
        "pid".dimmed(),
        "process".dimmed(),
        "detected".dimmed(),
        "kind".dimmed(),
        "proxy".dimmed(),
        "path".dimmed(),
        name_col = name_col,
        detected_col = detected_col,
    );

    for process in processes {
        let detected = process.detected_shortcut.unwrap_or("-");
        let proxy = if process.ai_only_proxy {
            "ai-only"
        } else {
            "all"
        };
        println!(
            "{:<8} {:<name_col$} {:<detected_col$} {:<8} {:<8} {}",
            process.pid.to_string().dimmed(),
            process.process_name.cyan(),
            detected.yellow(),
            process.kind.to_string().dimmed(),
            proxy.dimmed(),
            process.path,
            name_col = name_col,
            detected_col = detected_col,
        );
    }
}
