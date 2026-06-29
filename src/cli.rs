//! cli.rs — Clap CLI definitions: top-level parser and all subcommand variants.
//!
//! Nothing in this module performs I/O.  Its only job is to declare the shape
//! of the command line so that `main` can parse and dispatch it.

use clap::{Parser, Subcommand};

use crate::config::AppKind;

const HELP_GUIDE: &str = r#"Quick start:
  px init
  px credentials set
  px run <shortcut>

Credential setup:
  px credentials set
  px credentials set --user <user> --pass <password> --host <proxy-host> --port <proxy-port>
  px credentials show

Credentials are written to the .env file next to config.toml:
  Linux / macOS: ~/.config/proxy-launcher/.env
  Windows:       %APPDATA%\proxy-launcher\.env

Required .env keys:
  PX_PROXY_USER=<proxy username>
  PX_PROXY_PASS=<proxy password>

Optional .env keys override config.toml proxy.host/proxy.port:
  PX_HOST=<proxy host>
  PX_PORT=<proxy port>

Common commands:
  px init
      Create config.toml, create a .env template, and auto-register known tools.

  px ps [--search <text>] [--known]
      Inspect running processes that can be registered.

  px register --name <shortcut> --path <executable> --kind <cli|desktop> [--ai-only]
      Register or update an app shortcut. Omit --path to use the interactive scanner.

  px run <shortcut> [--proxy-override socks5://user:pass@host:port]
      Launch a registered app with proxy variables injected.

  px proxy [--show-secret]
      Print the resolved proxy URL and whether host/port came from .env or config.toml.

  px credentials set [--user <user>] [--pass <password>] [--host <host>] [--port <port>]
      Write proxy credentials to the local .env file.

  px credentials show [--show-secret]
      Show the credentials file path and resolved proxy URL.

  px alias <old-name> <new-name>
      Rename an existing shortcut.

  px edit
      Open config.toml in the default editor.

  px check
      Validate config.toml and registered executable paths.

Use `px help <command>` or `px <command> --help` for command-specific flags."#;

/// px — proxy-injector & application launcher for internal development teams.
///
/// Wraps development tools with the necessary proxy environment variables and
/// custom CA certificates before spawning them as child processes.
#[derive(Debug, Parser)]
#[command(
    name = "px",
    version,
    about = "Proxy-injector and application launcher",
    long_about = "Proxy-injector and application launcher for registered developer tools.",
    after_long_help = HELP_GUIDE,
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
    ///   px register --name cursor-d --path /Applications/Cursor.app/Contents/MacOS/Cursor --kind desktop --ai-only
    ///   px register --name codex   --path /usr/local/bin/codex --kind cli --ai-only
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
        /// Auto-enabled for built-in AI tool entries discovered by `px init`.
        #[arg(long, default_value_t = false)]
        ai_only: bool,
    },

    /// Launch a registered application with proxy environment variables injected.
    ///
    /// Examples:
    ///   px run cursor-d
    ///   px run codex --proxy-override socks5://user:pass@10.0.0.1:8080
    Run {
        /// The shortcut name registered via `px register`.
        shortcut: String,

        /// Bypass .env credentials and use this proxy URL directly.
        ///
        /// Format: socks5://user:pass@host:port
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

    /// Configure proxy credentials stored in the local .env file.
    Credentials {
        #[command(subcommand)]
        command: CredentialCommands,
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

#[derive(Debug, Subcommand)]
pub enum CredentialCommands {
    /// Write proxy credentials to the local .env file.
    ///
    /// Omitted values are prompted interactively. Existing values are used as
    /// defaults where possible, so this command can also update only one field.
    ///
    /// Examples:
    ///   px credentials set
    ///   px credentials set --user alice --pass secret --host proxy.local --port 8080
    Set {
        /// Proxy username.
        #[arg(long)]
        user: Option<String>,

        /// Proxy password.
        #[arg(long)]
        pass: Option<String>,

        /// Proxy host override written to .env as PX_HOST.
        #[arg(long)]
        host: Option<String>,

        /// Proxy port override written to .env as PX_PORT.
        #[arg(long)]
        port: Option<u16>,
    },

    /// Show the credentials file path and resolved proxy URL.
    Show {
        /// Show the raw proxy URL, including the password.
        #[arg(long, default_value_t = false)]
        show_secret: bool,
    },
}
