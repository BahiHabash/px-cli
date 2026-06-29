//! commands/init.rs — Handler for `px init`.
//!
//! 1. Creates the platform config directory.
//! 2. Writes a default `config.toml` (skipped if already present).
//! 3. Writes a `.env` credentials template (skipped if already present).
//! 4. Runs the auto-discovery engine and appends any found applications
//!    to `config.toml`, skipping shortcuts that already exist.

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::commands::discover;
use crate::config;

/// Scaffold config files and auto-discover developer tools on this machine.
pub fn run() -> Result<()> {
    let dir = config::config_dir()?;
    std::fs::create_dir_all(&dir)?;

    // --- config.toml --------------------------------------------------------
    let config_path = config::config_path()?;
    if config_path.exists() {
        println!(
            "{} Config already exists at '{}'",
            "ℹ".cyan().bold(),
            config_path.display()
        );
    } else {
        config::save(&config::Config::default())?;
        println!(
            "{} Created config at '{}'",
            "✔".green().bold(),
            config_path.display()
        );
    }

    // --- .env ---------------------------------------------------------------
    let env_path = config::env_path()?;
    if env_path.exists() {
        println!(
            "{} .env already exists at '{}'",
            "ℹ".cyan().bold(),
            env_path.display()
        );
    } else {
        let template = "# px credentials — keep this file out of version control\n\
                        PX_PROXY_USER=your_username\n\
                        PX_PROXY_PASS=your_password\n\
                        # Optional overrides. Leave commented to use config.toml [proxy].\n\
                        # PX_HOST=127.0.0.1\n\
                        # PX_PORT=8080\n";
        std::fs::write(&env_path, template)
            .with_context(|| format!("Failed to write .env to '{}'", env_path.display()))?;
        println!(
            "{} Created .env template at '{}'",
            "✔".green().bold(),
            env_path.display()
        );
        println!(
            "  {} Edit it and set your proxy credentials before using `px run`.",
            "→".yellow()
        );
    }

    let mut cfg = match config::load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!(
                "\n{} Cannot continue auto-discovery because config.toml is invalid.",
                "✘".red().bold()
            );
            eprintln!("  {}", format!("{err:#}").red());
            eprintln!(
                "  {} Fix the file at '{}' and run `px init` again.",
                "→".yellow(),
                config_path.display()
            );
            eprintln!("  {} Existing config was left unchanged.", "ℹ".cyan());
            bail!("invalid config.toml");
        }
    };

    // --- Auto-discovery -----------------------------------------------------
    println!("\n{} Scanning for developer tools …", "⟳".yellow().bold());

    let found = discover::scan();

    if found.is_empty() {
        println!(
            "  {} No supported tools detected on this machine.",
            "ℹ".cyan()
        );
    } else {
        let mut added = 0usize;
        let mut skipped = 0usize;

        for app in found {
            if cfg.apps.contains_key(app.shortcut) {
                // Never silently overwrite a manually registered shortcut.
                println!(
                    "  {} Skipping '{}' (already registered)",
                    "·".dimmed(),
                    app.shortcut.dimmed()
                );
                skipped += 1;
            } else {
                let ai_label = if app.entry.ai_only_proxy {
                    format!(" {}", "[ai-only]".cyan().bold())
                } else {
                    String::new()
                };
                println!(
                    "  {} Found '{}' → {} ({}){}",
                    "✔".green().bold(),
                    app.shortcut.cyan(),
                    app.entry.path,
                    app.entry.kind.to_string().dimmed(),
                    ai_label,
                );
                cfg.apps.insert(app.shortcut.to_string(), app.entry);
                added += 1;
            }
        }

        config::save(&cfg)?;

        println!(
            "\n  {} Discovery complete: {} added, {} already present.",
            "→".yellow(),
            added.to_string().green().bold(),
            skipped.to_string().dimmed()
        );
    }

    Ok(())
}
