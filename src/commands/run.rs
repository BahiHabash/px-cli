//! commands/run.rs — Handler for `px run`.
//!
//! Resolves a registered shortcut, obtains proxy credentials (either from
//! `.env` or a runtime override), injects environment variables, and spawns
//! the target process.
//!
//! ## Execution class dispatch
//!
//! | `AppKind`   | Behaviour                                                  |
//! |-------------|------------------------------------------------------------|
//! | `Cli`       | Block until child exits; propagate exit code to the shell  |
//! | `Desktop`   | Spawn and return immediately — terminal stays free         |
//!
//! ## Proxy override
//!
//! Passing `--proxy-override http://user:pass@host:port` completely bypasses
//! the `.env` credential loading and uses the provided URL as-is for all three
//! proxy environment variables.

use std::process::{self, Stdio};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::{config, credentials, path_utils};
use crate::config::AppKind;

// ---------------------------------------------------------------------------
// Proxy URL resolution
// ---------------------------------------------------------------------------

/// Returns the proxy URL to inject.
///
/// - If `proxy_override` is `Some`, it is used as-is (credentials from `.env`
///   are **not** loaded — this is the anti-reversing guardrail).
/// - Otherwise, credentials are loaded from the `.env` file and combined with
///   the host/port from `config.toml`.
fn resolve_proxy_url(
    proxy_override: Option<&str>,
    cfg: &config::Config,
) -> Result<String> {
    if let Some(url) = proxy_override {
        return Ok(url.to_string());
    }

    let (user, pass) = credentials::get_proxy_credentials()?;
    Ok(format!(
        "http://{}:{}@{}:{}",
        user, pass, cfg.proxy.host, cfg.proxy.port
    ))
}

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

/// Inject proxy env vars and spawn the registered application.
pub fn run(shortcut: &str, proxy_override: Option<String>) -> Result<()> {
    let cfg = config::load()?;

    // 1. Resolve shortcut → AppEntry.
    let entry = cfg.apps.get(shortcut).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown shortcut '{}'. Register it first:\n  px register --name {} --path <path> --kind <cli|desktop>",
            shortcut,
            shortcut
        )
    })?;

    // 2. Resolve executable path (expanding env vars and Windows .exe / .cmd fallback).
    let exec_path = path_utils::resolve_exec_path(&entry.path);

    // 3. Resolve proxy URL — override wins over .env credentials.
    let using_override = proxy_override.is_some();
    let proxy_url = resolve_proxy_url(proxy_override.as_deref(), &cfg)?;

    // 4. Optional CA cert injection.
    let cert_path: Option<&str> = if cfg.proxy.cert_path.is_empty() {
        None
    } else {
        Some(cfg.proxy.cert_path.as_str())
    };

    // 5. Build the Command with inherited stdio, saved args, and injected env vars.
    let mut cmd = process::Command::new(&exec_path);
    // Prepend any saved launch args (e.g. --disable-gpu captured by process scanner).
    if !entry.args.is_empty() {
        cmd.args(&entry.args);
    }
    cmd.env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(cert) = cert_path {
        cmd.env("NODE_EXTRA_CA_CERTS", cert);
    }

    // 6. Status line before launch.
    let mode_label = match entry.kind {
        AppKind::Desktop => "detached".dimmed(),
        AppKind::Cli => "blocking".dimmed(),
    };
    let override_label = if using_override {
        " [proxy-override]".yellow().bold().to_string()
    } else {
        String::new()
    };

    let args_label = if entry.args.is_empty() {
        String::new()
    } else {
        format!(" [{}]", entry.args.join(" ")).dimmed().to_string()
    };

    println!(
        "{} Launching '{}' {}{}{}",
        "→".yellow().bold(),
        exec_path.display(),
        mode_label,
        override_label,
        args_label,
    );

    // 7. Dispatch based on AppKind.
    match entry.kind {
        AppKind::Desktop => {
            // Spawn and return immediately — the child owns its own lifecycle.
            // Dropping the `Child` handle without calling `.wait()` detaches it.
            cmd.spawn().with_context(|| {
                format!(
                    "Failed to launch '{}'. Check the path is correct and executable.",
                    exec_path.display()
                )
            })?;
        }

        AppKind::Cli => {
            // Block until child exits, then propagate its exit code so shell
            // scripts and CI pipelines see accurate success / failure signals.
            let status = cmd
                .spawn()
                .with_context(|| {
                    format!(
                        "Failed to launch '{}'. Check the path is correct and executable.",
                        exec_path.display()
                    )
                })?
                .wait()
                .context("Failed to wait for child process")?;

            let code = status.code().unwrap_or(1);
            if code != 0 {
                // The child has already printed its own diagnostics — stay silent.
                process::exit(code);
            }
        }
    }

    Ok(())
}
