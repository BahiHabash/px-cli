//! commands/register.rs — Handler for `px register`.
//!
//! Upserts a named shortcut entry (path + execution class) into the `[apps]`
//! table of `config.toml`.

use anyhow::Result;
use colored::Colorize;

use crate::config::{self, AppEntry, AppKind};

/// Add or update an app shortcut in `config.toml`.
pub fn run(name: &str, path: &str, kind: AppKind) -> Result<()> {
    let mut cfg = config::load()?;

    let is_update = cfg.apps.contains_key(name);
    cfg.apps.insert(
        name.to_string(),
        AppEntry {
            path: path.to_string(),
            kind,
        },
    );
    config::save(&cfg)?;

    let action = if is_update { "Updated" } else { "Registered" };

    println!(
        "{} {} shortcut '{}' → '{}' ({})",
        "✔".green().bold(),
        action,
        name.cyan(),
        path,
        kind.to_string().dimmed()
    );

    Ok(())
}
