use std::{fs::File, io::Read, io::Write, path::PathBuf};

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

pub fn pack_crate(path: PathBuf, release: bool) -> Result<PathBuf> {
    let metadata = get_cargo_metadata(path.clone())?;
    let (_filename, mut src) = get_wasm_path(path.clone(), release, &metadata);
    let wasm_src = src.clone();
    src.pop();

    let crate_path = src.join(format!(
        "{}.gbcrate",
        get_project_name(path.clone(), &metadata)
    ));
    trace!(
        "packing crate to {:?}, wasm file at {:?}",
        crate_path, wasm_src
    );
    let file = File::create(crate_path.clone()).expect("cant create zip");
    let mut zip = zip::ZipWriter::new(file);

    let opts = SimpleFileOptions::default();

    zip.start_file("app.wasm", opts)?;
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

pub fn run_pack_command(
    release: bool,
    package: Option<String>,
    destination_path: Option<String>,
    no_copy: bool,
) -> Result<()> {
    let (path_arg, package_name_opt) = resolve_path_and_package(package)?;
    let path = resolve_project_dir(path_arg, package_name_opt.as_deref())?;
    build_project(path.clone(), release)?;

    let packed = pack_crate(path.clone(), release)?;

    if !no_copy {
        copy_crate(
            packed,
            determine_path(
                destination_path,
                get_gooseboy_crates_folder().expect("failed to get .gooseboy crates folder"),
            ),
        )?;
    }

    Ok(())
}
