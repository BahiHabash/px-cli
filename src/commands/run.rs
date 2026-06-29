//! commands/run.rs — Handler for `px run`.
//!
//! Resolves a registered shortcut, obtains proxy credentials (either from
//! `.env` or a runtime override), builds the appropriate env-var set, and
//! spawns the target process **via the platform shell**.
//!
//! ## Shell-wrapper execution model
//!
//! Every app is launched through a single terminal layer:
//!
//! | Platform | Shell    | Mechanism                    |
//! |----------|----------|------------------------------|
//! | Unix     | `/bin/sh`| `VAR="v" exec <app> <args>`  |
//! | Windows  | `cmd`    | `set VAR=v&& <app> <args>`   |
//!
//! On Unix, `exec` replaces the shell process so no extra PID is left behind.
//! On Windows, `cmd /c` exits as soon as the child returns.
//!
//! ## Execution class dispatch
//!
//! | `AppKind` | Behaviour                                                  |
//! |-----------|------------------------------------------------------------|
//! | `Cli`     | Block until child exits; propagate exit code to the shell  |
//! | `Desktop` | Spawn and return immediately — terminal stays free         |
//!
//! ## Proxy mode
//!
//! | `ai_only_proxy` | Env-vars set                                      |
//! |-----------------|---------------------------------------------------|
//! | `false`         | `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`          |
//! | `true`          | Same + `NO_PROXY` / `no_proxy` exclusion list     |
//!
//! ## Proxy override
//!
//! `--proxy-override socks5://user:pass@host:port` completely bypasses `.env`
//! credential loading and uses the provided URL as-is for all three proxy
//! environment variables.

use std::process::{self, Stdio};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::AppKind;
use crate::{config, credentials, no_proxy, path_utils, shell};

// ---------------------------------------------------------------------------
// Proxy URL resolution
// ---------------------------------------------------------------------------

/// Returns the proxy URL to inject.
///
/// - If `proxy_override` is `Some`, it is used as-is (credentials from `.env`
///   are **not** loaded — this is the anti-credential-leak guardrail).
/// - Otherwise, credentials are loaded from the `.env` file and combined with
///   the host/port from `config.toml`.
fn resolve_proxy_url(proxy_override: Option<&str>, cfg: &config::Config) -> Result<String> {
    if let Some(url) = proxy_override {
        return Ok(url.to_string());
    }

    Ok(credentials::resolve_proxy_url(cfg)?.url)
}

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

/// Inject proxy env vars via a shell wrapper and spawn the registered app.
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

    // 5. Build the env-var list to inject.
    //
    //    AI-only mode: also set NO_PROXY / no_proxy so that everything except
    //    LLM API hosts bypasses the proxy.  The shell-wrapper then exports all
    //    of these before exec-ing the target app.
    let mut env_vars: Vec<(&str, String)> = vec![
        ("HTTP_PROXY", proxy_url.clone()),
        ("HTTPS_PROXY", proxy_url.clone()),
        ("ALL_PROXY", proxy_url.clone()),
    ];

    if entry.ai_only_proxy {
        let np = no_proxy::build_no_proxy(&cfg.proxy.no_proxy_extra);
        env_vars.push(("NO_PROXY", np.clone()));
        env_vars.push(("no_proxy", np)); // lowercase alias for curl/wget
    }

    if let Some(cert) = cert_path {
        env_vars.push(("NODE_EXTRA_CA_CERTS", cert.to_string()));
    }

    // Convert to &str slices for the shell builder.
    let env_refs: Vec<(&str, &str)> = env_vars.iter().map(|(k, v)| (*k, v.as_str())).collect();

    // 6. Build the shell invocation (sh -c / cmd /c).
    let detach = entry.kind == AppKind::Desktop;
    let invocation = shell::build(&exec_path, &entry.args, &env_refs, detach);

    // 7. Status line before launch.
    let mode_label = match entry.kind {
        AppKind::Desktop => "detached".dimmed(),
        AppKind::Cli => "blocking".dimmed(),
    };
    let proxy_mode_label = if entry.ai_only_proxy {
        " [ai-only]".cyan().bold().to_string()
    } else {
        String::new()
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
        "{} Launching '{}' {}{}{}{}",
        "→".yellow().bold(),
        exec_path.display(),
        mode_label,
        proxy_mode_label,
        override_label,
        args_label,
    );

    // 8. Build the underlying Command from the shell invocation.
    let mut cmd = process::Command::new(&invocation.shell);
    cmd.args(&invocation.shell_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // 9. Dispatch based on AppKind.
    match entry.kind {
        AppKind::Desktop => {
            // Spawn and return immediately — the child owns its own lifecycle.
            // On Unix the shell exec-replaces itself with the app, so dropping
            // the Child handle here correctly detaches the app process.
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
