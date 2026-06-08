#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_debug_formatting)]
use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};

use crate::commands::{build::run_build_command, new::run_new_command, pack::run_pack_command};

pub mod commands;
pub mod utils;

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(styles = CLAP_STYLING)]
#[command(version = "1.0")]
#[command(about = "gooseboy command line tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: SCommands,
}

#[derive(Subcommand)]
pub enum SCommands {
    Gooseboy {
        #[command(subcommand)]
        command: Commands,
    },
}

#[derive(Subcommand)]
#[command(about = "gooseboy command line tool", long_about = None)]
pub enum Commands {
    New {
        package: Option<String>,
        #[arg(long)]
        buildscript: bool,
        #[arg(long)]
        no_std: bool,
        #[arg(long, alias = "lib")]
        library: bool,
    },
    Build {
        #[arg(short, long)]
        release: bool,
        package: Option<String>,
    },
    Pack {
        #[arg(short, long)]
        release: bool,
        #[arg(long)]
        no_copy: bool,
        package: Option<String>,
        destination_path: Option<String>,
    },
}

pub fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let SCommands::Gooseboy { command } = Cli::parse().command;

    match command {
        Commands::New {
            package,
            no_std,
            buildscript,
            library,
        } => run_new_command(package, no_std, buildscript, !library)?,
        Commands::Build { release, package } => run_build_command(release, package)?,
        Commands::Pack {
            release,
            package,
            destination_path,
            no_copy,
        } => run_pack_command(release, package, destination_path, no_copy)?,
    }

    Ok(())
}
