use anyhow::Result;
use log::trace;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

pub const TARGET: &str = "wasm32-unknown-unknown";

pub(crate) fn determine_path(path: Option<String>, default: PathBuf) -> PathBuf {
    path.map(PathBuf::from).unwrap_or(default)
}

pub(crate) fn run_command(path: PathBuf, command: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(command);
    cmd.current_dir(path.clone());
    cmd.args(args);

    trace!("running `{:?}` at {:?}", cmd, path.clone());

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

pub(crate) fn get_cargo_metadata(path: PathBuf) -> Result<Value> {
    let output = Command::new("cargo")
        .current_dir(path)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()?;
    let stdout = String::from_utf8(output.stdout).unwrap();
    Ok(serde_json::from_str(&stdout).unwrap())
}

pub(crate) fn get_target_directory(metadata: &Value) -> PathBuf {
    Path::new(&metadata["target_directory"].as_str().unwrap().to_string()).to_path_buf()
}

pub(crate) fn get_project_name(path: PathBuf, metadata: &Value) -> String {
    let manifest = path.join("Cargo.toml");
    let manifest_abs = fs::canonicalize(&manifest).unwrap_or(manifest.clone());

    let pkg = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| {
            if let Some(m) = p["manifest_path"].as_str() {
                let pkg_path = Path::new(m);
                match fs::canonicalize(pkg_path) {
                    std::result::Result::Ok(pkg_abs) => pkg_abs == manifest_abs,
                    Err(_) => m == manifest.to_str().unwrap_or_default(),
                }
            } else {
                false
            }
        })
        .expect("package not found");

    pkg["name"]
        .as_str()
        .expect("failed to cast project name to a string")
        .to_string()
}

pub(crate) fn get_wasm_path(path: PathBuf, release: bool, metadata: &Value) -> (String, PathBuf) {
    let profile = if release { "release" } else { "debug" };

    let project_name = get_project_name(path.clone(), metadata);
    let filename = format!("{}.wasm", project_name);

    let target_directory = get_target_directory(metadata);

    // target/wasm32-unknown-unknown/release/mycrate.wasm
    (
        filename.clone(),
        target_directory
            .join(TARGET)
            .join(profile)
            .join(filename.clone()),
    )
}

pub(crate) fn resolve_project_dir(path: PathBuf, package_name: Option<&str>) -> Result<PathBuf> {
    let metadata = get_cargo_metadata(path.clone())?;

    let manifest = if let Some(name) = package_name {
        metadata["packages"]
            .as_array()
            .and_then(|arr| arr.iter().find(|p| p["name"].as_str() == Some(name)))
            .and_then(|p| p["manifest_path"].as_str())
            .map(str::to_string)
    } else {
        let candidate = path.join("Cargo.toml");
        metadata["packages"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|p| p["manifest_path"].as_str() == candidate.to_str())
            })
            .and_then(|p| p["manifest_path"].as_str())
            .map(str::to_string)
    }
    .unwrap_or_else(|| path.join("Cargo.toml").to_string_lossy().into_owned());

    Ok(Path::new(&manifest).parent().unwrap().to_path_buf())
}

pub(crate) fn resolve_path_and_package(arg: Option<String>) -> Result<(PathBuf, Option<String>)> {
    if let Some(a) = arg {
        let p = PathBuf::from(&a);
        if p.exists() {
            let abs = fs::canonicalize(p)?;
            return Ok((abs, None));
        } else {
            let cwd = fs::canonicalize(env::current_dir()?)?;
            return Ok((cwd, Some(a)));
        }
    }

    let cwd = fs::canonicalize(env::current_dir()?)?;
    Ok((cwd, None))
}

pub fn get_gooseboy_crates_folder() -> Result<PathBuf> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    let folder = Path::new(&home).join(".gooseboy").join("crates");

    if !folder.exists() {
        fs::create_dir_all(&folder)?;
    }

    Ok(folder)
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
