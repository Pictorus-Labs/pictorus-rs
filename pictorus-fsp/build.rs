use std::path::PathBuf;

use bindgen::EnumVariation;

/// Kept in step with `renesas-fsp-sys/build.rs`. The seam header includes the FSP
/// interface headers, so it has to be parsed under exactly the same ABI, or the
/// `ioport_instance_t *` this crate sees would not be the one `renesas-fsp-sys` bound.
const CLANG_TARGET: &str = "thumbv8m.main-none-eabihf";

const SNAPSHOT: &str = "bindings/seam.rs";
const UPDATE_SNAPSHOT_VAR: &str = "PICTORUS_FSP_UPDATE_SNAPSHOT";

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let header = manifest_dir.join("c/rm_pictorus_app.h");
    // Supplies rm_pictorus_app_cfg.h, which the configurator generates in a
    // real project and which the seam header includes unconditionally.
    let c_shim = manifest_dir.join("c/shim");

    // Reached by path rather than through a `DEP_*_INCLUDE` variable, because
    // exporting one would mean giving renesas-fsp-sys a `links` key -- a declaration
    // that it owns a native library, which it does not; it emits declarations
    // and links nothing. The sibling is present for a path dependency and for a
    // git dependency alike, since cargo checks out the whole repository.
    let renesas_fsp_sys = manifest_dir.parent().unwrap().join("renesas-fsp-sys");
    let shim = renesas_fsp_sys.join("shim");
    let vendor_inc = renesas_fsp_sys.join("vendor/fsp/inc");

    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-changed={}", c_shim.display());
    println!("cargo:rerun-if-changed={}", shim.display());
    println!("cargo:rerun-if-changed={}", vendor_inc.display());
    println!("cargo:rerun-if-env-changed={UPDATE_SNAPSHOT_VAR}");

    let rust_target = std::env::var("TARGET").unwrap();
    let layout_tests = rust_target.starts_with("thumb");

    let bindings = bindgen::builder()
        .header(header.display().to_string())
        .clang_arg(format!("--target={CLANG_TARGET}"))
        .clang_arg("-fshort-enums")
        .clang_arg("-ffreestanding")
        .clang_arg("-nostdlibinc")
        .clang_arg(format!("-I{}", c_shim.display()))
        .clang_arg(format!("-I{}", shim.display()))
        .clang_arg(format!("-I{}", vendor_inc.display()))
        .clang_arg(format!("-I{}", vendor_inc.join("api").display()))
        // Only what the seam header itself declares. The FSP types it names --
        // ioport_instance_t, timer_instance_t -- must be renesas-fsp-sys's,
        // not fresh copies: two structurally identical bindgen outputs are
        // still two distinct Rust types, and the wrappers in this crate take
        // renesas-fsp-sys's. The `use` below is what makes the unbound names resolve.
        .allowlist_file(".*rm_pictorus_app\\.h")
        // Without this, bindgen follows every type an allowlisted struct
        // mentions and re-emits the whole FSP interface layer -- 80 KB of
        // duplicate definitions that are distinct Rust types from renesas-fsp-sys's.
        .allowlist_recursively(false)
        // Defined in Rust, in src/app.rs. Binding it as well would put an
        // `extern` declaration and a `#[no_mangle]` definition of the same
        // symbol in one crate, which is legal but reads as though something
        // outside supplies it. The rest of the entry points genuinely are
        // external here -- the generated application crate defines them.
        .blocklist_function("pictorus_rt_bind")
        .raw_line("use renesas_fsp_sys::*;")
        .use_core()
        .ctypes_prefix("core::ffi")
        .default_enum_style(EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .generate_comments(false)
        .layout_tests(layout_tests)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate rm_pictorus_app bindings");

    bindings
        .write_to_file(out_dir.join("seam.rs"))
        .expect("failed to write seam.rs");

    if std::env::var_os(UPDATE_SNAPSHOT_VAR).is_some() {
        assert!(
            !layout_tests,
            "refusing to update {SNAPSHOT} from a cross build: the snapshot is the \
             layout-test-free form. Re-run without --target."
        );
        let path = manifest_dir.join(SNAPSHOT);
        std::fs::create_dir_all(path.parent().unwrap()).expect("failed to create bindings dir");
        std::fs::copy(out_dir.join("seam.rs"), &path).expect("failed to write seam snapshot");
        println!("cargo:warning=updated {SNAPSHOT}");
    }
}
