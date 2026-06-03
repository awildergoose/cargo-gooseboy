use std::path::PathBuf;

use anyhow::{Ok, Result};

use crate::utils::{TARGET, resolve_path_and_package, resolve_project_dir, run_command};

pub fn build_project(path: &PathBuf, release: bool) -> Result<()> {
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

pub fn run_build_command(release: bool, package: Option<String>) -> anyhow::Result<()> {
    let (path_arg, package_name_opt) = resolve_path_and_package(package)?;
    let path = resolve_project_dir(&path_arg, package_name_opt.as_deref())?;
    build_project(&path, release)?;
    Ok(())
}
