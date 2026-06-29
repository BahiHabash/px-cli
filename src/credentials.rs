//! credentials.rs — Runtime credential loading from a `.env` file.
//!
//! ## File location
//!
//! The `.env` file lives alongside `config.toml` in the platform config dir:
//!
//! | Platform      | Path                                        |
//! |---------------|---------------------------------------------|
//! | Linux / macOS | `~/.config/proxy-launcher/.env`             |
//! | Windows       | `%APPDATA%\proxy-launcher\.env`             |
//!
//! ## Required keys in `.env`
//!
//! ```env
//! PX_PROXY_USER=your_username
//! PX_PROXY_PASS=your_password
//! ```
//!
//! ## V2 Migration Path
//!
//! To switch to OS-native keychain storage, replace **only the body** of
//! `get_proxy_credentials()` with a `keyring::Entry` lookup.  No other file
//! in the codebase needs to change:
//!
//! ```rust,ignore
//! use keyring::Entry;
//!
//! pub fn get_proxy_credentials() -> anyhow::Result<(String, String)> {
//!     let entry = Entry::new("proxy-launcher", "proxy-user")?;
//!     let pass  = entry.get_password()?;
//!     let user  = std::env::var("PX_PROXY_USER")?;
//!     Ok((user, pass))
//! }
//! ```

use anyhow::{bail, Context, Result};

use crate::config;

const PROXY_SCHEME: &str = "socks5";

pub struct ProxyCredentials {
    pub user: String,
    pub pass: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}

pub struct ResolvedProxyUrl {
    pub url: String,
    pub masked_url: String,
    pub host_source: &'static str,
    pub port_source: &'static str,
}

/// Loads the `.env` file from the config directory and returns
/// proxy credentials and optional host/port overrides.
///
/// # Errors
///
/// Returns an error if:
/// - The `.env` file cannot be found or read.
/// - Either `PX_PROXY_USER` or `PX_PROXY_PASS` is missing from the file.
pub fn get_proxy_credentials() -> Result<ProxyCredentials> {
    let env_file = config::env_path()?;

    // Load the .env file into the current process's environment.
    // `dotenvy::from_path` does NOT override variables that are already set
    // in the environment, which is the safe, expected behaviour.
    dotenvy::from_path(&env_file).with_context(|| {
        format!(
            "Could not load credentials from '{}'. \
             Create the file with PX_PROXY_USER and PX_PROXY_PASS set. \
             Run `px init` to generate a template.",
            env_file.display()
        )
    })?;

    let user = std::env::var("PX_PROXY_USER").with_context(|| {
        format!(
            "PX_PROXY_USER is not set in '{}'. \
             Add a line: PX_PROXY_USER=your_username",
            env_file.display()
        )
    })?;

    let pass = std::env::var("PX_PROXY_PASS").with_context(|| {
        format!(
            "PX_PROXY_PASS is not set in '{}'. \
             Add a line: PX_PROXY_PASS=your_password",
            env_file.display()
        )
    })?;

    if user == "your_username" || pass == "your_password" {
        bail!(
            "Proxy credentials in '{}' still contain template placeholder values. \
             Set PX_PROXY_USER and PX_PROXY_PASS before running apps.",
            env_file.display()
        );
    }

    let host = std::env::var("PX_HOST").ok();
    let port = std::env::var("PX_PORT").ok().and_then(|p| p.parse().ok());

    Ok(ProxyCredentials {
        user,
        pass,
        host,
        port,
    })
}

/// Builds the same proxy URL used by `px run`, plus redacted diagnostics.
pub fn resolve_proxy_url(cfg: &config::Config) -> Result<ResolvedProxyUrl> {
    let creds = get_proxy_credentials()?;

    let (host, host_source) = match creds.host.as_deref() {
        Some(host) => (host, ".env PX_HOST"),
        None => (cfg.proxy.host.as_str(), "config.toml proxy.host"),
    };
    let (port, port_source) = match creds.port {
        Some(port) => (port, ".env PX_PORT"),
        None => (cfg.proxy.port, "config.toml proxy.port"),
    };

    Ok(ResolvedProxyUrl {
        url: format!(
            "{}://{}:{}@{}:{}",
            PROXY_SCHEME, creds.user, creds.pass, host, port
        ),
        masked_url: format!(
            "{}://{}:<redacted>@{}:{}",
            PROXY_SCHEME, creds.user, host, port
        ),
        host_source,
        port_source,
    })
}
