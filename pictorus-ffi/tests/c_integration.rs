//! Compiles `tests/c/test_runtime.c` against the `mock_ffi_lib` example and runs it.
//!
//! The example is a staticlib standing in for the library codegen generates for an app, so this
//! exercises the library the way a C integrator does: through the headers, linked statically.
#![cfg(feature = "alloc")]

use std::path::PathBuf;
use std::process::Command;

/// Cargo builds examples before running integration tests, into the `examples` directory next
/// to the test binary's `deps` directory.
fn mock_ffi_lib() -> PathBuf {
    let test_exe = std::env::current_exe().unwrap();
    let profile_dir = test_exe.parent().unwrap().parent().unwrap();
    let lib = profile_dir.join("examples/libmock_ffi_lib.a");
    assert!(lib.exists(), "staticlib not found at {}", lib.display());
    lib
}

#[test]
fn c_program_runs_against_staticlib() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = mock_ffi_lib();

    let out_dir = std::env::temp_dir().join("pictorus_ffi_c_integration");
    std::fs::create_dir_all(&out_dir).unwrap();
    let exe = out_dir.join("test_runtime");

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut compile = Command::new(cc);
    compile
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(manifest_dir.join("include"))
        .arg("-I")
        .arg(manifest_dir.join("tests/c"))
        .arg(manifest_dir.join("tests/c/test_runtime.c"))
        .arg(&lib)
        .arg("-o")
        .arg(&exe);
    // The Rust standard library inside the staticlib needs these from the platform. The
    // authoritative list for a build is `cargo rustc --example mock_ffi_lib --crate-type
    // staticlib -- --print native-static-libs`.
    if cfg!(target_os = "linux") {
        // The test takes the address of the absolute `PICTORUS_APP_INTERFACE_*` symbols, which
        // a position independent executable (the default on most distributions) cannot do.
        // Real users read these symbols in a linker script, not from C.
        compile.args(["-fno-pie", "-no-pie"]);
        compile.args([
            "-lgcc_s",
            "-lutil",
            "-lrt",
            "-lpthread",
            "-lm",
            "-ldl",
            "-lc",
        ]);
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
