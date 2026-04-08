use glib_build_tools::compile_resources;

use std::env;
use std::path::{Path, PathBuf};

fn get_output_path() -> PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let target = out_dir
        .split_at(out_dir.find(build_type.as_str()).unwrap())
        .0;
    Path::new(&target).join(build_type)
}

fn main() {
    // Always rerun on every build
    println!("cargo:rerun-if-changed=.");

    compile_resources(&["assets"], "assets/resources.xml", "void.gresource");

    let pkg_name = env::var("CARGO_PKG_NAME").unwrap();
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap();

    let schema_dir = get_output_path().join(format!(
        "share/gsettings-schema/{}-{}/glib-2.0/schemas",
        pkg_name, pkg_version
    ));

    // Ensure directory exists (self-healing)
    std::fs::create_dir_all(&schema_dir).expect("Failed to create schema directory");

    // Remove old schema file if it exists
    let schema_path = schema_dir.join("me.acristoffers.void.gschema.xml");
    std::fs::remove_file(&schema_path).ok();

    // Copy fresh schema file
    std::fs::copy("assets/gschema.xml", &schema_path).expect("Could not copy gschema.xml");

    // Compile schemas
    let output = std::process::Command::new("glib-compile-schemas")
        .arg(schema_dir.to_str().unwrap())
        .output()
        .expect("Failed to compile schemas, did you install the package `glibc2`?");
    if !output.status.success() {
        panic!(
            "Failed to compile schemas: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let share_dir = get_output_path().join("share");

    // Install desktop file
    let applications_dir = share_dir.join("applications");
    std::fs::create_dir_all(&applications_dir).expect("Failed to create applications directory");
    std::fs::copy(
        "assets/me.acristoffers.void.desktop",
        applications_dir.join("me.acristoffers.void.desktop"),
    )
    .expect("Could not copy desktop file");

    // Install metainfo file
    let metainfo_dir = share_dir.join("metainfo");
    std::fs::create_dir_all(&metainfo_dir).expect("Failed to create metainfo directory");
    std::fs::copy(
        "assets/me.acristoffers.void.metainfo.xml",
        metainfo_dir.join("me.acristoffers.void.metainfo.xml"),
    )
    .expect("Could not copy metainfo file");

    // Install icon
    let icons_dir = share_dir.join("icons/hicolor/scalable/apps");
    std::fs::create_dir_all(&icons_dir).expect("Failed to create icons directory");
    std::fs::copy(
        "assets/icon.svg",
        icons_dir.join("me.acristoffers.void.svg"),
    )
    .expect("Could not copy icon");
}
