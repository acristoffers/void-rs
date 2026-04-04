/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Compiles `tests/c/test_ffi.c` against the built shared library and runs it.
//!
//! Requires a C compiler (`cc`) in PATH.  In the project's Nix dev-shell this
//! is provided by `pkgs.gcc`.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn run_c_ffi_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Workspace root is one level up from void-ffi/
    let workspace_dir = manifest_dir
        .parent()
        .expect("void-ffi must be inside a workspace");

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let lib_dir = workspace_dir.join("target").join(profile);
    let header_dir = manifest_dir.join("include");
    let test_src = manifest_dir.join("tests").join("c").join("test_ffi.c");

    // Put the compiled test binary next to the shared library so rpath works.
    let bin_path = lib_dir.join("void_ffi_c_test");

    // Ensure the cdylib is up to date before compiling the C test against it.
    // `cargo test` builds the rlib (for integration tests) but does not
    // guarantee the cdylib is linked first.  Running `cargo build` here is
    // cheap when nothing has changed.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let build_status = Command::new(&cargo)
        .args(["build", "-p", "void-ffi"])
        .current_dir(workspace_dir)
        .status()
        .expect("Failed to run cargo build");
    assert!(build_status.success(), "cargo build -p void-ffi failed");

    // -----------------------------------------------------------------------
    // Compile
    // -----------------------------------------------------------------------
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());

    let compile = Command::new(&cc)
        .args([
            test_src.to_str().unwrap(),
            "-o",
            bin_path.to_str().unwrap(),
            &format!("-I{}", header_dir.display()),
            &format!("-L{}", lib_dir.display()),
            "-lvoid_ffi",
            // Bake the library search path into the binary so LD_LIBRARY_PATH
            // is not needed at run time.
            &format!("-Wl,-rpath,{}", lib_dir.display()),
            "-Wall",
            "-Wextra",
        ])
        .output()
        .unwrap_or_else(|e| panic!("Failed to launch '{cc}': {e}"));

    assert!(
        compile.status.success(),
        "C test compilation failed:\n{}",
        String::from_utf8_lossy(&compile.stderr),
    );

    // -----------------------------------------------------------------------
    // Run
    // -----------------------------------------------------------------------
    let run = Command::new(&bin_path)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run C test binary: {e}"));

    // Always print C test output so it appears in `cargo test -- --nocapture`
    print!("{}", String::from_utf8_lossy(&run.stdout));
    if !run.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&run.stderr));
    }

    assert!(
        run.status.success(),
        "C tests failed (exit code {:?})",
        run.status.code(),
    );
}
