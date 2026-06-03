use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Result, anyhow};
use log::trace;
use serde_json::Value;

pub const TARGET: &str = "wasm32-unknown-unknown";

pub(crate) fn determine_path(path: Option<String>, default: PathBuf) -> PathBuf {
    path.map_or(default, PathBuf::from)
}

pub(crate) fn run_command(path: &PathBuf, command: &str, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new(command);
    cmd.current_dir(path.clone());
    cmd.args(args);

    trace!("running `{cmd:?}` at {path:?}");

    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run command `{cmd:?}: {e}` at {path:?}"))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "command exited with code {:?}",
            status.code()
        ));
    }

    Ok(())
}

pub(crate) fn get_cargo_metadata(path: &PathBuf) -> Result<Value> {
    let output = Command::new("cargo")
        .current_dir(path)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    if stdout.is_empty() {
        return Err(anyhow!("no cargo metadata found"));
    }
    Ok(serde_json::from_str(&stdout)?)
}

pub(crate) fn get_target_directory(metadata: &Value) -> PathBuf {
    Path::new(&metadata["target_directory"].as_str().unwrap().to_string()).to_path_buf()
}

pub(crate) fn get_project_name(path: &Path, metadata: &Value) -> Result<String> {
    let manifest = path.join("Cargo.toml");
    let manifest_abs = fs::canonicalize(&manifest)?;

    let pkg = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| {
            p["manifest_path"].as_str().map_or_else(
                || false,
                |m| {
                    let pkg_path = Path::new(m);
                    fs::canonicalize(pkg_path).map_or_else(
                        |_| m == manifest.to_str().unwrap_or_default(),
                        |pkg_abs| pkg_abs == manifest_abs,
                    )
                },
            )
        })
        .expect("package not found");

    Ok(pkg["name"]
        .as_str()
        .expect("failed to cast project name to a string")
        .to_string())
}

pub(crate) fn get_wasm_path(
    path: &Path,
    release: bool,
    metadata: &Value,
) -> Result<(String, PathBuf)> {
    let profile = if release { "release" } else { "debug" };

    let project_name = get_project_name(path, metadata)?;
    let filename = format!("{project_name}.wasm");

    let target_directory = get_target_directory(metadata);

    // target/wasm32-unknown-unknown/release/mycrate.wasm
    Ok((
        filename.clone(),
        target_directory.join(TARGET).join(profile).join(filename),
    ))
}

pub(crate) fn resolve_project_dir(path: &PathBuf, package_name: Option<&str>) -> Result<PathBuf> {
    if !fs::exists(path.join("Cargo.toml"))? {
        return Ok(path.clone());
    }

    let metadata = get_cargo_metadata(path)?;

    let manifest = package_name
        .map_or_else(
            || {
                let candidate = path.join("Cargo.toml");
                metadata["packages"]
                    .as_array()
                    .and_then(|arr| {
                        arr.iter()
                            .find(|p| p["manifest_path"].as_str() == candidate.to_str())
                    })
                    .and_then(|p| p["manifest_path"].as_str())
                    .map(str::to_string)
            },
            |name| {
                metadata["packages"]
                    .as_array()
                    .and_then(|arr| arr.iter().find(|p| p["name"].as_str() == Some(name)))
                    .and_then(|p| p["manifest_path"].as_str())
                    .map(str::to_string)
            },
        )
        .unwrap_or_else(|| path.join("Cargo.toml").to_string_lossy().into_owned());

    Ok(Path::new(&manifest).parent().unwrap().to_path_buf())
}

pub(crate) fn resolve_path_and_package(arg: Option<String>) -> Result<(PathBuf, Option<String>)> {
    if let Some(a) = arg {
        let p = PathBuf::from(&a);
        if p.exists() {
            let abs = fs::canonicalize(p)?;
            return Ok((abs, None));
        }

        let cwd = fs::canonicalize(env::current_dir()?)?;
        return Ok((cwd, Some(a)));
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

pub fn copy_crate(crate_path: &PathBuf, destination_path: &Path) -> Result<()> {
    let dst = destination_path.join(
        crate_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("failed to get crate filename"))?,
    );

    if !crate_path.exists() {
        anyhow::bail!("{crate_path:?} not found");
    }

    trace!("copying {crate_path:?} to {dst:?}");

    fs::create_dir_all(
        dst.parent()
            .ok_or_else(|| anyhow::anyhow!("failed to get copy destination parent"))?,
    )?;
    fs::copy(crate_path, dst)?;

    Ok(())
}
