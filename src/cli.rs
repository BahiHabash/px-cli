//! cli.rs — Clap CLI definitions: top-level parser and all subcommand variants.
//!
//! Nothing in this module performs I/O.  Its only job is to declare the shape
//! of the command line so that `main` can parse and dispatch it.

use clap::{Parser, Subcommand};

use crate::config::AppKind;

/// px — proxy-injector & application launcher for internal development teams.
///
/// Wraps development tools with the necessary proxy environment variables and
/// custom CA certificates before spawning them as child processes.
#[derive(Debug, Parser)]
#[command(
    name = "px",
    version,
    about = "Proxy-injector and application launcher",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialise the config directory, config.toml, .env template, and run auto-discovery.
    ///
    /// Safe to re-run: skips files that already exist and never overwrites
    /// shortcuts that have already been registered.
    Init,

    /// Register (or update) a named shortcut pointing to an executable.
    ///
    /// Examples:
    ///   px register --name cursor-desktop --path /Applications/Cursor.app/Contents/MacOS/Cursor --kind desktop
    ///   px register --name codex-cli      --path /usr/local/bin/codex --kind cli
    Register {
        /// Short, memorable name used with `px run <name>`.
        #[arg(short, long)]
        name: String,

        /// Absolute path to the executable.
        #[arg(short, long)]
        path: String,

        /// Execution class: `cli` blocks the terminal; `desktop` detaches immediately.
        #[arg(short, long, value_enum, default_value_t = AppKind::Cli)]
        kind: AppKind,
    },

    /// Launch a registered application with proxy environment variables injected.
    ///
    /// Examples:
    ///   px run cursor-desktop
    ///   px run codex-cli --proxy-override http://user:pass@10.0.0.1:8080
    Run {
        /// The shortcut name registered via `px register`.
        shortcut: String,

        /// Bypass .env credentials and use this proxy URL directly.
        ///
        /// Format: http://user:pass@host:port
        ///
        /// Useful for advanced workflows where the default credentials are
        /// inappropriate (e.g. routing through a different upstream proxy).
        #[arg(short = 'o', long)]
        proxy_override: Option<String>,
    },

    /// Validate config.toml and check that every registered executable exists on disk.
    ///
    /// Exits with code 1 if any paths are broken — safe to use in CI scripts.
    Check,
}
