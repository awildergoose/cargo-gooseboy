use anyhow::{Ok, Result};

use clap::{Parser, Subcommand};

use crate::commands::build::run_build_command;
use crate::commands::new::run_new_command;
use crate::commands::pack::run_pack_command;

pub mod commands;
pub mod utils;

#[derive(Parser)]
#[command(name = "gooseboy")]
#[command(version = "1.0")]
#[command(about = "gooseboy command line tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    New {
        package: Option<String>,
        #[arg(long)]
        buildscript: bool,
        #[arg(long)]
        no_std: bool,
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

    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            package,
            no_std,
            buildscript,
        } => run_new_command(package, no_std, buildscript)?,
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
