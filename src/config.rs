//! config.rs — Serde models for `config.toml` and helpers for loading / saving it.
//!
//! ## On-disk layout (`config.toml`)
//!
//! ```toml
//! [proxy]
//! host      = "127.0.0.1"
//! port      = 8080
//! cert_path = ""           # leave empty to skip NODE_EXTRA_CA_CERTS injection
//!
//! [apps.cursor-d]
//! path = "/Applications/Cursor.app/Contents/MacOS/Cursor"
//! kind = "desktop"         # detach immediately — don't hold the terminal hostage
//!
//! [apps.codex]
//! path = "/usr/local/bin/codex"
//! kind = "cli"             # block, inherit IO, and propagate exit code
//! ```
//!
//! ## Credential storage
//!
//! Credentials are **not** stored in `config.toml`.  They live in a separate
//! `.env` file in the same directory, loaded at runtime by `credentials`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Logical application name — drives the config directory path via `dirs`.
const APP_NAME: &str = "proxy-launcher";

/// Primary config file.
const CONFIG_FILE: &str = "config.toml";

// ---------------------------------------------------------------------------
// App classification
// ---------------------------------------------------------------------------

/// Explicit execution class for a registered application.
///
/// This replaces the old `detached: bool` field with a named, self-documenting
/// variant that is also usable directly as a `clap::ValueEnum`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    /// CLI tool — block the terminal, inherit stdio, and propagate exit code.
    Cli,
    /// Desktop / GUI app — detach immediately so the terminal stays free.
    Desktop,
}

impl Default for AppKind {
    fn default() -> Self {
        AppKind::Cli
    }
}

impl std::fmt::Display for AppKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppKind::Cli => write!(f, "cli"),
            AppKind::Desktop => write!(f, "desktop"),
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Top-level configuration, serialised to / deserialised from `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Proxy connection settings.
    pub proxy: ProxyConfig,

    /// Map of shortcut name → app entry (path + execution class).
    ///
    /// Stored in TOML as `[apps.<name>]` sub-tables.
    /// Defaults to an empty map so a freshly-created config is always valid.
    #[serde(default)]
    pub apps: HashMap<String, AppEntry>,
}

/// Per-application registration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    /// Absolute path to the executable (may contain env-var tokens like %LOCALAPPDATA%).
    pub path: String,

    /// Execution class: `cli` (blocking) or `desktop` (detached).
    ///
    /// Defaults to `cli` when absent from the config file.
    #[serde(default)]
    pub kind: AppKind,

    /// Optional launch arguments appended to the executable on every run.
    ///
    /// Automatically populated by the dynamic process scanner when the app
    /// was captured running with specific flags (e.g. `--disable-gpu`).
    /// Empty by default; omitted from TOML when empty to keep the file clean.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    /// When `true`, the proxy is applied only for AI/LLM API hosts.
    ///
    /// `NO_PROXY` is automatically set to a broad exclusion list so that all
    /// non-AI traffic (npm, git, pip, health-checks, etc.) bypasses the proxy
    /// and goes direct.  Set automatically for Cursor and Codex entries during
    /// `px init`; toggle manually with `px register --ai-only`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ai_only_proxy: bool,
}

/// Proxy host, port and optional CA certificate settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Hostname or IP address of the proxy server.
    pub host: String,

    /// Port the proxy listens on.
    pub port: u16,

    /// Optional path to a corporate / internal CA certificate (PEM).
    ///
    /// When non-empty this is injected as `NODE_EXTRA_CA_CERTS` so that
    /// Electron-based apps (Cursor, VS Code …) trust the intercepting proxy.
    #[serde(default)]
    pub cert_path: String,

    /// Additional hosts / domains to exclude from the proxy when `ai_only_proxy`
    /// is active.  These are merged with the built-in baseline (localhost,
    /// common registries, Git forges) produced by the `no_proxy` module.
    ///
    /// Example: `["registry.company.internal", "git.company.internal"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub no_proxy_extra: Vec<String>,
}

// ---------------------------------------------------------------------------
// Default config — written by `px init`
// ---------------------------------------------------------------------------

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                cert_path: String::new(),
                no_proxy_extra: vec![],
            },
            apps: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Returns the platform-appropriate config directory:
///
/// | Platform       | Path                              |
/// |----------------|-----------------------------------|
/// | Linux / macOS  | `~/.config/proxy-launcher/`       |
/// | Windows        | `%APPDATA%\proxy-launcher\`       |
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .context("Could not determine the platform config directory. Is $HOME / %APPDATA% set?")?;
    Ok(base.join(APP_NAME))
}

/// Full path to `config.toml`.
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE))
}

/// Full path to the `.env` credentials file (same directory as `config.toml`).
pub fn env_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(".env"))
}

// ---------------------------------------------------------------------------
// Load / Save
// ---------------------------------------------------------------------------

/// Reads and parses `config.toml`.
///
/// Returns a descriptive error if the file is absent (directing the user to
/// run `px init` first) or if the TOML is malformed.
pub fn load() -> Result<Config> {
    let path = config_path()?;

    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "Could not read config file at '{}'. Have you run `px init`?",
            path.display()
        )
    })?;

    toml::from_str::<Config>(&raw)
        .with_context(|| format!("Failed to parse config file at '{}'", path.display()))
}

/// Serialises `config` and writes it to `config.toml`.
///
/// The parent directory must already exist (created by `px init`).
pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;

    let toml_string =
        toml::to_string_pretty(config).context("Failed to serialise config to TOML")?;

    fs::write(&path, toml_string)
        .with_context(|| format!("Failed to write config file to '{}'", path.display()))
}
