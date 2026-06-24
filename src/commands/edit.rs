//! commands/edit.rs — Handler for `px edit`.
//!
//! Opens the `config.toml` file in the user's default text editor.

use std::process::Command;

use anyhow::Result;
use colored::Colorize;

use crate::config;

/// Open `config.toml` in the default editor.
pub fn run() -> Result<()> {
    let cfg_path = config::config_path()?;

    if !cfg_path.exists() {
        println!(
            "{} Config file not found at '{}'. Run `px init` first.",
            "⚠".yellow().bold(),
            cfg_path.display()
        );
        return Ok(());
    }

    println!(
        "{} Opening '{}' in default editor...",
        "→".yellow().bold(),
        cfg_path.display()
    );

    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(["/c", "start", "", cfg_path.to_str().unwrap()])
        .spawn()?;

    #[cfg(target_os = "macos")]
    Command::new("open").arg(cfg_path).spawn()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(cfg_path).spawn()?;

    Ok(())
}
