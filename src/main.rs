use anyhow::{Ok, Result};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};
use log::trace;
use serde_json::Value;
use zip::write::SimpleFileOptions;

#[derive(Parser)]
#[command(name = "gooseboy")]
#[command(version = "1.0")]
#[command(about = "gooseboy command line tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(short, long)]
        release: bool,
        project_path: Option<String>,
    },
    Pack {
        #[arg(short, long)]
        release: bool,
        #[arg(long)]
        no_copy: bool,
        project_path: Option<String>,
        destination_path: Option<String>,
    },
}

const TARGET: &str = "wasm32-unknown-unknown";

fn determine_path(path: Option<String>, default: PathBuf) -> PathBuf {
    path.map(PathBuf::from).unwrap_or(default) //env::current_dir().expect("could not get current dir"))
}

fn run_command(path: PathBuf, command: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(command);
    cmd.current_dir(path.clone());
    cmd.args(args);

    let status = cmd.status().map_err(|e| {
        anyhow::anyhow!(
            "failed to run command `{:?}: {}` at {:?}",
            cmd,
            e,
            path.clone()
        )
    })?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "command exited with code {:?}",
            status.code()
        ));
    }

    Ok(())
}

// TODO only call once
fn get_cargo_metadata(path: PathBuf) -> Value {
    let output = Command::new("cargo")
        .current_dir(path)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .expect("failed to run cargo metadata");
    let stdout = String::from_utf8(output.stdout).unwrap();
    serde_json::from_str(&stdout).unwrap()
}

fn get_target_directory(path: PathBuf) -> PathBuf {
    Path::new(
        &get_cargo_metadata(path)["target_directory"]
            .as_str()
            .unwrap()
            .to_string(),
    )
    .to_path_buf()
}

fn get_project_name(path: PathBuf) -> String {
    let manifest = path.clone().join("Cargo.toml");
    let v = get_cargo_metadata(path.clone());
    let pkg = v["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["manifest_path"].as_str().unwrap() == manifest.to_str().unwrap())
        .expect("package not found");

    pkg["name"]
        .as_str()
        .expect("failed to cast project name to a string")
        .to_string()
}

fn get_wasm_path(path: PathBuf, release: bool) -> (String, PathBuf) {
    let profile = if release { "release" } else { "debug" };

    let project_name = get_project_name(path.clone());
    let filename = format!("{}.wasm", project_name);

    let target_directory = get_target_directory(path);

    // target/wasm32-unknown-unknown/release/mycrate.wasm
    (
        filename.clone(),
        target_directory
            .join(TARGET)
            .join(profile)
            .join(filename.clone()),
    )
}

pub fn get_gooseboy_crates_folder() -> Result<PathBuf> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    let folder = Path::new(&home).join(".gooseboy");

    if !folder.exists() {
        fs::create_dir_all(&folder)?;
    }

    Ok(folder)
}

pub fn build_project(path: PathBuf, release: bool) -> Result<()> {
    let mut build_args = Vec::new();
    build_args.push("build");

    if release {
        build_args.push("--release");
    }

    build_args.push("--target");
    build_args.push(TARGET);

    run_command(path, "cargo", &build_args)?;

    Ok(())
}

pub fn pack_crate(path: PathBuf, release: bool) -> Result<PathBuf> {
    let (filename, mut src) = get_wasm_path(path.clone(), release);
    let wasm_src = src.clone();
    src.pop();

    let crate_path = src.join(format!("{}.gbcrate", get_project_name(path.clone())));
    trace!(
        "packing crate to {:?}, wasm file at {:?}",
        crate_path, wasm_src
    );
    let file = File::create(crate_path.clone()).expect("cant create zip");
    let mut zip = zip::ZipWriter::new(file);

    let opts = SimpleFileOptions::default();

    zip.start_file(filename, opts)?;
    let mut buf = Vec::new();
    File::open(wasm_src)
        .expect("cant open wasm file")
        .read_to_end(&mut buf)?;
    zip.write_all(&buf)?;

    zip.start_file("crate.json", opts)?;
    buf = Vec::new();
    File::open(path.join("crate.json"))
        .expect("cant open crate.json file")
        .read_to_end(&mut buf)?;
    zip.write_all(&buf)?;

    zip.finish()?;

    Ok(crate_path)
}

pub fn copy_crate(crate_path: PathBuf, destination_path: PathBuf) -> Result<()> {
    let dst = destination_path.join(crate_path.file_name().unwrap());

    if !crate_path.exists() {
        return Err(anyhow::anyhow!("{:?} not found", crate_path));
    }

    trace!("copying {:?} to {:?}", crate_path, dst);

    fs::create_dir_all(dst.parent().unwrap())?;
    fs::copy(crate_path, dst)?;

    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            release,
            project_path,
        } => {
            let path = determine_path(
                project_path,
                env::current_dir().expect("could not get current directory"),
            );
            build_project(path, release)?;
        }
        Commands::Pack {
            release,
            project_path,
            destination_path,
            no_copy,
        } => {
            let path = determine_path(
                project_path,
                env::current_dir().expect("could not get current directory"),
            );
            build_project(path.clone(), release)?;

            if !no_copy {
                copy_crate(
                    pack_crate(path.clone(), release)?,
                    determine_path(
                        destination_path,
                        get_gooseboy_crates_folder()
                            .expect("failed to get .gooseboy crates folder"),
                    ),
                )?;
            }
        }
    }

    Ok(())
}
