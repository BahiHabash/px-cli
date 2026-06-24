//! commands/alias.rs — Handler for `px alias`.
//!
//! Renames an existing shortcut in the `[apps]` table of `config.toml`.

use anyhow::{bail, Result};
use colored::Colorize;

use crate::config;

/// Rename an existing app shortcut in `config.toml`.
pub fn run(old_name: &str, new_name: &str) -> Result<()> {
    let mut cfg = config::load()?;

    if cfg.apps.contains_key(new_name) {
        bail!("A shortcut named '{}' already exists.", new_name);
    }

    let entry = match cfg.apps.remove(old_name) {
        Some(e) => e,
        None => bail!("Shortcut '{}' not found in config.", old_name),
    };

    cfg.apps.insert(new_name.to_string(), entry);
    config::save(&cfg)?;

    println!(
        "{} Renamed shortcut '{}' → '{}'",
        "✔".green().bold(),
        old_name.cyan(),
        new_name.cyan()
    );

    Ok(())
}
