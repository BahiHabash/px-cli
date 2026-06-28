//! main.rs — Entry point for `px`.
//!
//! Responsible for exactly two things:
//!   1. Parsing the command-line arguments.
//!   2. Dispatching to the appropriate command handler.
//!
//! All business logic lives in the `commands`, `no_proxy`, and `shell` modules.

mod cli;
mod commands;
mod config;
mod credentials;
mod no_proxy;
mod path_utils;
mod shell;

use anyhow::Result;
use clap::Parser;

use cli::Commands;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Register { name, path, search, kind, ai_only } => {
            commands::register::run(&name, path.as_deref(), search.as_deref(), kind, ai_only)
        }
        Commands::Run { shortcut, proxy_override } => commands::run::run(&shortcut, proxy_override),
        Commands::Check => commands::check::run(),
        Commands::Alias { old_name, new_name } => commands::alias::run(&old_name, &new_name),
        Commands::Edit => commands::edit::run(),
    }
}
