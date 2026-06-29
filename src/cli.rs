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
    ///   px register --name cursor-desktop --path /Applications/Cursor.app/Contents/MacOS/Cursor --kind desktop --ai-only
    ///   px register --name codex-cli      --path /usr/local/bin/codex --kind cli --ai-only
    Register {
        /// Short, memorable name used with `px run <name>`.
        #[arg(short, long)]
        name: String,

        /// Absolute path to the executable.
        ///
        /// Omit this flag to use the interactive process scanner: px will
        /// snapshot running processes, prompt you to open the app, then
        /// detect its path automatically from the new processes.
        #[arg(short, long)]
        path: Option<String>,

        /// Search running processes by name (case-insensitive) instead of using
        /// the interactive snapshot scanner.
        #[arg(short, long)]
        search: Option<String>,

        /// Execution class: `cli` blocks the terminal; `desktop` detaches immediately.
        ///
        /// If omitted during process-based registration, px infers it from the
        /// detected app and falls back to `desktop` for unknown running apps.
        #[arg(short, long, value_enum)]
        kind: Option<AppKind>,

        /// Route only AI/LLM API traffic through the proxy.
        ///
        /// When set, `NO_PROXY` is automatically populated with a broad exclusion
        /// list (localhost, npm, git, pip, crates.io …) so that only endpoints
        /// like api.openai.com and api.anthropic.com go through the proxy.
        /// All other traffic goes direct.
        ///
        /// Auto-enabled for Cursor and Codex entries discovered by `px init`.
        #[arg(long, default_value_t = false)]
        ai_only: bool,
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

    /// Inspect running processes that px can register.
    ///
    /// Examples:
    ///   px ps
    ///   px ps --search cursor
    ///   px ps --known
    Ps {
        /// Filter running processes by name, detected app, or executable path.
        #[arg(short, long)]
        search: Option<String>,

        /// Show only processes recognized as known developer tools.
        #[arg(long, default_value_t = false)]
        known: bool,
    },

    /// Print the constructed proxy URL and where its host/port came from.
    Proxy {
        /// Show the raw proxy URL, including the password.
        #[arg(long, default_value_t = false)]
        show_secret: bool,
    },

    /// Rename an existing registered shortcut.
    ///
    /// Examples:
    ///   px alias old-name new-name
    Alias {
        /// The current name of the shortcut.
        old_name: String,

        /// The new name for the shortcut.
        new_name: String,
    },

    /// Open `config.toml` in the default text editor for manual editing.
    Edit,
}
