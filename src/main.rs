//! main.rs — Entry point for `px`.
//!
//! Responsible for exactly two things:
//!   1. Parsing the command-line arguments.
//!   2. Dispatching to the appropriate command handler.
//!
//! All business logic lives in the `commands` module.

mod cli;
mod commands;
mod config;
mod credentials;
mod path_utils;

use anyhow::Result;
use clap::Parser;

use cli::Commands;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Register { name, path, kind } => commands::register::run(&name, path.as_deref(), kind),
        Commands::Run { shortcut, proxy_override } => commands::run::run(&shortcut, proxy_override),
        Commands::Check => commands::check::run(),
    }
}
