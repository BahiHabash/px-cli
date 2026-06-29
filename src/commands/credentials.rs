//! commands/credentials.rs — Configure proxy credentials in the local `.env`.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::{config, credentials};

pub fn set(
    user: Option<String>,
    pass: Option<String>,
    host: Option<String>,
    port: Option<u16>,
) -> Result<()> {
    let dir = config::config_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config directory '{}'", dir.display()))?;

    let has_all_values = user.is_some() && pass.is_some() && host.is_some() && port.is_some();
    let existing = if has_all_values {
        HashMap::new()
    } else {
        read_existing_env()?
    };
    let defaults = default_proxy_config()?;

    let user = match user {
        Some(value) => value,
        None => prompt_required(
            "Proxy username",
            existing.get("PX_PROXY_USER").map(String::as_str),
        )?,
    };

    let pass = match pass {
        Some(value) => value,
        None => prompt_password(existing.get("PX_PROXY_PASS").map(String::as_str))?,
    };

    let host = match host {
        Some(value) => value,
        None => prompt_required(
            "Proxy host",
            existing
                .get("PX_HOST")
                .map(String::as_str)
                .or(Some(defaults.proxy.host.as_str())),
        )?,
    };

    let port = match port {
        Some(value) => value,
        None => {
            let existing_port = existing
                .get("PX_PORT")
                .and_then(|value| value.parse::<u16>().ok());
            prompt_port(existing_port.unwrap_or(defaults.proxy.port))?
        }
    };

    let env_path = config::env_path()?;
    let body = format!(
        "# px credentials - keep this file out of version control\n\
         PX_PROXY_USER={}\n\
         PX_PROXY_PASS={}\n\
         PX_HOST={}\n\
         PX_PORT={}\n",
        dotenv_quote(&user)?,
        dotenv_quote(&pass)?,
        dotenv_quote(&host)?,
        port
    );

    fs::write(&env_path, body)
        .with_context(|| format!("Failed to write credentials to '{}'", env_path.display()))?;

    println!(
        "{} Saved proxy credentials to '{}'",
        "✔".green().bold(),
        env_path.display()
    );
    println!(
        "  {} Run {} to verify the resolved proxy URL.",
        "→".yellow(),
        "px credentials show".cyan()
    );

    Ok(())
}

pub fn show(show_secret: bool) -> Result<()> {
    let env_path = config::env_path()?;
    println!("{} {}", "Credentials file:".bold(), env_path.display());

    let cfg = default_proxy_config()?;
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

fn read_existing_env() -> Result<HashMap<String, String>> {
    let env_path = config::env_path()?;
    if !env_path.exists() {
        return Ok(HashMap::new());
    }

    let mut values = HashMap::new();
    for item in dotenvy::from_path_iter(&env_path)
        .with_context(|| format!("Failed to read credentials file '{}'", env_path.display()))?
    {
        let (key, value) =
            item.with_context(|| format!("Failed to parse '{}'", env_path.display()))?;
        values.insert(key, value);
    }

    Ok(values)
}

fn default_proxy_config() -> Result<config::Config> {
    if config::config_path()?.exists() {
        config::load()
    } else {
        Ok(config::Config::default())
    }
}

fn prompt_required(label: &str, default: Option<&str>) -> Result<String> {
    loop {
        let input = prompt(label, default)?;
        if !input.trim().is_empty() {
            return Ok(input);
        }
        println!("  {} {} is required.", "⚠".yellow().bold(), label);
    }
}

fn prompt_password(existing: Option<&str>) -> Result<String> {
    if existing.is_some() {
        println!(
            "  {} Password input is visible. Leave blank to keep the existing password.",
            "Note:".yellow().bold()
        );
    } else {
        println!(
            "  {} Password input is visible in this version.",
            "Note:".yellow().bold()
        );
    }

    loop {
        print!("Proxy password");
        if existing.is_some() {
            print!(" [keep existing]");
        }
        print!(": ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim_end_matches(['\r', '\n']).to_string();

        if input.is_empty() {
            if let Some(value) = existing {
                return Ok(value.to_string());
            }
            println!("  {} Proxy password is required.", "⚠".yellow().bold());
        } else {
            return Ok(input);
        }
    }
}

fn prompt_port(default: u16) -> Result<u16> {
    loop {
        let input = prompt("Proxy port", Some(&default.to_string()))?;
        match input.parse::<u16>() {
            Ok(port) => return Ok(port),
            Err(_) => println!(
                "  {} Proxy port must be a number from 0 to 65535.",
                "⚠".yellow().bold()
            ),
        }
    }
}

fn prompt(label: &str, default: Option<&str>) -> Result<String> {
    print!("{}", label);
    if let Some(value) = default {
        print!(" [{}]", value);
    }
    print!(": ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim_end_matches(['\r', '\n']);

    if input.is_empty() {
        Ok(default.unwrap_or_default().to_string())
    } else {
        Ok(input.to_string())
    }
}

fn dotenv_quote(value: &str) -> Result<String> {
    if value.contains(['\r', '\n']) {
        bail!("Credential values cannot contain newlines.");
    }

    let escaped = value.replace('\\', r"\\").replace('"', r#"\""#);
    Ok(format!(r#""{}""#, escaped))
}

#[cfg(test)]
mod tests {
    use super::dotenv_quote;

    #[test]
    fn dotenv_quote_preserves_special_password_chars() {
        assert_eq!(
            dotenv_quote(r#"p#ss word "quoted" \ path"#).unwrap(),
            r#""p#ss word \"quoted\" \\ path""#
        );
    }

    #[test]
    fn dotenv_quote_rejects_newlines() {
        assert!(dotenv_quote("line\nbreak").is_err());
    }
}
