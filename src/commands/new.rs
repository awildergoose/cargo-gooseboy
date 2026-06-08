use std::{env, fs};

use anyhow::{Ok, Result};
use toml_edit::{DocumentMut, Item, Table, value};

use crate::utils::{resolve_path_and_package, resolve_project_dir, run_command};

pub fn run_new_command(
    package: Option<String>,
    no_std: bool,
    buildscript: bool,
    binary: bool,
) -> Result<()> {
    let (path_arg, package_name_opt) = resolve_path_and_package(package.clone())?;
    let path = resolve_project_dir(&path_arg, package_name_opt.as_deref())?;
    let project_name = package
        .or_else(|| {
            env::current_dir()
                .ok()
                .and_then(|dir| dir.file_name().map(|os| os.to_string_lossy().into_owned()))
        })
        .ok_or_else(|| anyhow::anyhow!("failed to get project name"))?;

    // 1) run cargo init
    run_command(
        &path,
        "cargo",
        &["init", "--lib", path.display().to_string().as_str()],
    )?;

    // 2) set lib.crate-type to ["cdylib"] in Cargo.toml
    if binary {
        let manifest = path.join("Cargo.toml");
        let manifest = fs::canonicalize(&manifest)?;
        let manifest_str = fs::read_to_string(manifest.clone())?;
        let mut manifest_doc = manifest_str.parse::<DocumentMut>()?;
        manifest_doc.insert("lib", Item::Table(Table::new()));
        manifest_doc["lib"].as_table_mut().ok_or_else(|| {
            anyhow::anyhow!("if you get this error then something has TRULY gone wrong")
        })?["crate-type"] = value({
            let mut a = toml_edit::Array::new();
            a.push("cdylib");
            a
        });
        fs::write(manifest, manifest_doc.to_string())?;
    }

    // 3) add gooseboy to the packages if !no_std
    if !no_std {
        if binary {
            run_command(&path, "cargo", &["add", "gooseboy", "--features", "binary"])?;
        } else {
            run_command(&path, "cargo", &["add", "gooseboy"])?;
        }
    }

    // 4) add buildscript
    if buildscript {
        run_command(&path, "cargo", &["add", "gooseboy_buildscript"])?;

        // 4.1) place build.rs for converting files
        let build_rs_path = path.join("build.rs");
        fs::write(build_rs_path, BUILDSCIRPT)?;

        // 4.2) create folders
        let images_path = path.join("images");
        let audio_path = path.join("audio");
        fs::create_dir_all(images_path)?;
        fs::create_dir_all(audio_path)?;
    }

    // 5) overwrite src/lib.rs with our custom example
    let lib_rs_path = path.join("src").join("lib.rs");

    if binary {
        if no_std {
            fs::write(lib_rs_path, EXAMPLE_BIN_NO_STD)?;
        } else {
            fs::write(lib_rs_path, EXAMPLE_BIN)?;
        }
    } else {
        fs::write(lib_rs_path, EXAMPLE_LIB)?;
    }

    // 6) make crate.json
    if binary {
        let crate_json_path = path.join("crate.json");
        fs::write(crate_json_path, CRATE_JSON.replace("%name%", &project_name))?;
    }

    Ok(())
}

const EXAMPLE_LIB: &str = "#![no_main]\n";

const EXAMPLE_BIN: &str = r#"#![no_main]
use gooseboy::framebuffer::{get_framebuffer_width, init_fb};
use gooseboy::text::{draw_text, get_text_width};
use gooseboy::{color::Color, framebuffer::clear_framebuffer};

// Every crate has to have a main function, make sure to decorate it
// with gooseboy::main though, or else the crate won't start
#[gooseboy::main]
fn main() {
    // Initializes the framebuffer, you are required to initialize this
    // here if you plan to draw to the screen (which is very likely)
    init_fb();
}

// This is also required in every crate, the gooseboy::update is required
// here too, this function runs X times per second where X is equal to your
// maximum framerate in the options
#[gooseboy::update]
fn update(nano_time: i64) {
    // Clear out the screen, erasing everything that was there previously
    clear_framebuffer(Color::BLACK);

    // Initialize the string we want to draw to the screen, You can also use Rust's
    // String type here, with the caveat of having to clone it at draw_text
    let text = "Hello, world!";
    // Convert the time from nanoseconds to seconds
    let time_sec = nano_time as f64 / 1_000_000_000.0;
    // Get the position of the right corner and subtract the width of the text
    // to make the text fit into the screen, You can also use draw_text_wrapped
    // to automatically wrap text if it passes the end of the framebuffer
    let right_corner = (get_framebuffer_width() - get_text_width(text)) as f64;
    // Gets us an X position that smoothly moves from the left to the right using sine
    let x_pos = ((time_sec.sin() * 0.5 + 0.5) * (right_corner - 1.0)) as usize;

    // Finally, draw the text with the red color (or use Color::new(r, g, b, a) or Color::new_opaque(r, g, b))
    draw_text(x_pos, 0, text, Color::RED);
}"#;

const EXAMPLE_BIN_NO_STD: &str = r#"#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn main() {

}

#[unsafe(no_mangle)]
pub extern "C" fn update(nano_time: i64) {

}
"#;

const BUILDSCIRPT: &str = r"fn main() {
    gooseboy_buildscript::convert_audio();
    gooseboy_buildscript::convert_images();
}
";

const CRATE_JSON: &str = r#"{
	"version": 1,
	"name": "%name%",
	"description": "%name%",
	"entrypoint": "app.wasm",
	"permissions": []
}"#;
