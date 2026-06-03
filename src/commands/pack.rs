use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Ok, Result};
use log::trace;
use zip::write::SimpleFileOptions;

use crate::{
    commands::build::build_project,
    utils::{
        copy_crate, determine_path, get_cargo_metadata, get_gooseboy_crates_folder,
        get_project_name, get_wasm_path, resolve_path_and_package, resolve_project_dir,
    },
};

pub fn pack_crate(path: &PathBuf, release: bool) -> Result<PathBuf> {
    let metadata = get_cargo_metadata(path)?;
    let (_filename, mut src) = get_wasm_path(path, release, &metadata)?;
    let wasm_src = src.clone();
    src.pop();

    let crate_path = src.join(format!("{}.gbcrate", get_project_name(path, &metadata)?));
    trace!("packing crate to {crate_path:?}, wasm file at {wasm_src:?}");
    let file = File::create(crate_path.clone())?;
    let mut zip = zip::ZipWriter::new(file);

    let opts = SimpleFileOptions::default();

    zip.start_file("app.wasm", opts)?;
    let mut buf = Vec::new();
    File::open(wasm_src)?.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;

    zip.start_file("crate.json", opts)?;
    buf = Vec::new();
    File::open(path.join("crate.json"))?.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;

    zip.finish()?;

    Ok(crate_path)
}

pub fn run_pack_command(
    release: bool,
    package: Option<String>,
    destination_path: Option<String>,
    no_copy: bool,
) -> Result<()> {
    let (path_arg, package_name_opt) = resolve_path_and_package(package)?;
    let path = resolve_project_dir(&path_arg, package_name_opt.as_deref())?;
    build_project(&path, release)?;

    let packed = pack_crate(&path, release)?;

    if !no_copy {
        copy_crate(
            &packed,
            &determine_path(destination_path, get_gooseboy_crates_folder()?),
        )?;
    }

    Ok(())
}
