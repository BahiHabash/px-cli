//! commands/proxy.rs — Proxy configuration diagnostics.

use colored::Colorize;

use crate::{config, credentials};

pub fn run(show_secret: bool) -> anyhow::Result<()> {
    let cfg = config::load()?;
    let resolved = credentials::resolve_proxy_url(&cfg)?;
    let url = if show_secret {
        resolved.url
    } else {
        resolved.masked_url
    };

    println!("{} {}", "Proxy URL:".bold(), url);
    println!("{} {}", "Host source:".bold(), resolved.host_source);
    println!("{} {}", "Port source:".bold(), resolved.port_source);

    if !show_secret {
        println!(
            "{} Re-run with {} only if you explicitly need to inspect the password.",
            "Note:".yellow().bold(),
            "--show-secret".yellow()
        );
    }

    Ok(())
}
