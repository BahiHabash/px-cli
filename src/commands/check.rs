//! commands/check.rs — Handler for `px check`.
//!
//! Parses the current `config.toml`, validates that each registered executable
//! path exists on disk, and prints a colour-coded status report.
//!
//! ## Sample output
//!
//! ```text
//! Checking 3 registered app(s) …
//!
//!   ✔  cursor-desktop   desktop   /Applications/Cursor.app/Contents/MacOS/Cursor
//!   ✔  codex-cli        cli       /usr/local/bin/codex
//!   ✘  vscode-desktop   desktop   /Applications/Visual Studio Code.app  [not found]
//!
//! Summary: 2 healthy, 1 broken
//! ```

use anyhow::Result;
use colored::Colorize;

use crate::config;

/// Validate the config file and check that every registered path exists.
pub fn run() -> Result<()> {
    let cfg = config::load()?;

    if cfg.apps.is_empty() {
        println!(
            "{} No apps registered yet. Use `px register` or `px init` to add some.",
            "ℹ".cyan().bold()
        );
        return Ok(());
    }

    println!(
        "\nChecking {} registered app(s) …\n",
        cfg.apps.len().to_string().bold()
    );

    // Determine column widths for aligned output.
    let name_width = cfg.apps.keys().map(|k| k.len()).max().unwrap_or(10).max(10);
    let kind_width = 7; // "desktop" is the longest variant

    let mut healthy = 0usize;
    let mut broken = 0usize;

    // Sort entries alphabetically for stable, readable output.
    let mut entries: Vec<_> = cfg.apps.iter().collect();
    entries.sort_by_key(|(name, _)| name.as_str());

    for (name, entry) in &entries {
        let path_exists = std::path::Path::new(&entry.path).exists();

        if path_exists {
            healthy += 1;
            println!(
                "  {}  {:<name_w$}  {:<kind_w$}  {}",
                "✔".green().bold(),
                name.green(),
                entry.kind.to_string().dimmed(),
                entry.path,
                name_w = name_width,
                kind_w = kind_width,
            );
        } else {
            broken += 1;
            println!(
                "  {}  {:<name_w$}  {:<kind_w$}  {} {}",
                "✘".red().bold(),
                name.red(),
                entry.kind.to_string().dimmed(),
                entry.path.dimmed(),
                "[not found]".red().bold(),
                name_w = name_width,
                kind_w = kind_width,
            );
        }
    }

    println!();

    // Summary line
    let healthy_str = format!("{} healthy", healthy).green().bold().to_string();
    let broken_str = if broken > 0 {
        format!("{} broken", broken).red().bold().to_string()
    } else {
        format!("{} broken", broken).dimmed().to_string()
    };
    println!("Summary: {}, {}", healthy_str, broken_str);

    // Exit with code 1 if any paths are broken, so CI pipelines can detect it.
    if broken > 0 {
        std::process::exit(1);
    }

    Ok(())
}
