//! Compiles `c/test_runtime.c` against the fixture staticlib and runs it.

use std::path::PathBuf;
use std::process::Command;

/// Builds the staticlib into the same target directory and profile as this test binary.
///
/// `cargo test` only builds the rlib of a library that has several crate types, so the staticlib
/// has to be requested explicitly. The outer cargo has finished building by the time tests run,
/// so it no longer holds the build lock.
fn build_staticlib() -> PathBuf {
    let test_exe = std::env::current_exe().unwrap();
    let profile_dir = test_exe.parent().unwrap().parent().unwrap();
    let target_dir = profile_dir.parent().unwrap();

    let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    build
        .args([
            "build",
            "--lib",
            "-p",
            "pictorus-ffi-fixture",
            "--target-dir",
        ])
        .arg(target_dir);
    if profile_dir.file_name().unwrap() == "release" {
        build.arg("--release");
    }
    assert!(
        build.status().unwrap().success(),
        "building the staticlib failed"
    );

    profile_dir.join("libpictorus_ffi_fixture.a")
}

#[test]
fn c_program_runs_against_staticlib() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ffi_include = manifest_dir.join("../pictorus-ffi/include");
    let fixture_include = manifest_dir.join("include");
    let source = manifest_dir.join("c/test_runtime.c");

    let lib = build_staticlib();
    assert!(lib.exists(), "staticlib not found at {}", lib.display());

    let out_dir = std::env::temp_dir().join("pictorus_ffi_c_integration");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("test_runtime");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut compile = Command::new(cc);
    compile
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(&ffi_include)
        .arg("-I")
        .arg(&fixture_include)
        .arg(&source)
        .arg(&lib)
        .arg("-o")
        .arg(&exe);
    // The Rust standard library inside the staticlib needs these from the platform.
    if cfg!(target_os = "linux") {
        compile.args(["-lpthread", "-ldl", "-lm"]);
    }
    let status = compile.status().expect("failed to run the C compiler");
    assert!(status.success(), "C compilation or linking failed");

    let output = Command::new(&exe).output().unwrap();
    assert!(
        output.status.success(),
        "C test failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
