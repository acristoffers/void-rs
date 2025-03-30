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
    compile_resources(&["assets"], "assets/resources.xml", "void.gresource");

    let pkg_name = env::var("CARGO_PKG_NAME").unwrap();
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap();

    let schema_dir = get_output_path().join(format!(
        "share/gsettings-schema/{}-{}/glib-2.0/schemas",
        pkg_name, pkg_version
    ));
    std::fs::create_dir_all(&schema_dir).expect("Failed to create schema directory");
    std::fs::remove_file(schema_dir.join("me.acristoffers.void.gschema.xml")).ok();
    std::fs::copy(
        "assets/gschema.xml",
        schema_dir.join("me.acristoffers.void.gschema.xml"),
    )
    .expect("Could not copy gschema.xml");
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
}
