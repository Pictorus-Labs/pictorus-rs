use std::path::{Path, PathBuf};

use bindgen::EnumVariation;
use bindgen::callbacks::{IntKind, ParseCallbacks};

/// The Arm target whose C ABI the bindings describe.
///
/// Fixed rather than taken from `TARGET`, so that the generated declarations
/// are byte-identical whether this crate is being cross-compiled or checked on
/// a developer's host, which is what makes the committed snapshot in
/// `bindings/` a meaningful diff. All Cortex-M parts in the RA family agree on
/// everything the interface layer exposes -- 32-bit pointers, 4-byte alignment,
/// and, given `-fshort-enums`, identical enum widths -- so there is nothing for
/// a per-part target to say that this one does not.
///
/// This describes the *C* side only. Rust struct layouts still come from the
/// real `TARGET`, which is why the layout tests below are conditional.
const CLANG_TARGET: &str = "thumbv8m.main-none-eabihf";

/// Committed copy of the generated bindings, compared against by
/// `tests/snapshot.rs`. Refresh with:
///
/// ```text
/// RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys
/// ```
///
/// A non-empty diff after a pack upgrade means the FSP ABI moved, which is
/// exactly the event that would otherwise be discovered as a misbehaving board.
const SNAPSHOT: &str = "bindings/generated.rs";

const UPDATE_SNAPSHOT_VAR: &str = "RENESAS_FSP_SYS_UPDATE_SNAPSHOT";

/// `fsp_err_t` runs to 0x40000 with large gaps -- UART errors start at 200, SPI
/// at 300, CAN at 60000 -- and FSP is free to return a value this crate's
/// vendored headers have never heard of. Materialising an unlisted discriminant
/// as a Rust enum would be instant UB, so this one is pinned to a newtype
/// regardless of the default.
const ERROR_ENUM: &str = "fsp_err_t";

/// Items to keep out of the crate's public API. Everything here is incidental
/// to the FSP surface, and this crate re-exports the generated module wholesale.
const BLOCKLIST: &[&str] = &[
    // The `typedef char name[1]` artifacts of the static assertions in
    // `shim/bsp_api.h`. They do their work at bindgen's parse time.
    "fsp_sys_.*_size",
    // Pulled in from clang's own stdint.h/stddef.h rather than from FSP.
    // Nothing in the bound surface refers to them.
    "u?int_(least|fast)(8|16|32|64)_t",
    "u?intmax_t",
    "wchar_t",
    "max_align_t",
];

#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    /// `#define`d integers default to `u32`, which turns every FSP macro that
    /// is conceptually a count or a size into a cast at the use site. The
    /// interface layer's macros are all small non-negative counts.
    fn int_macro(&self, _name: &str, _value: i64) -> Option<IntKind> {
        Some(IntKind::U32)
    }
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let shim = manifest_dir.join("shim");
    let vendor_inc = manifest_dir.join("vendor/fsp/inc");
    let wrapper = manifest_dir.join("wrapper.h");

    // Watching only build.rs is a real bug that serves stale bindings after a
    // re-vendor, silently and for as long as nothing else in the crate changes.
    println!("cargo:rerun-if-changed={}", wrapper.display());
    println!("cargo:rerun-if-changed={}", shim.display());
    println!("cargo:rerun-if-changed={}", vendor_inc.display());
    println!("cargo:rerun-if-env-changed={UPDATE_SNAPSHOT_VAR}");

    // Rust struct layouts follow the real target, so bindgen's layout
    // assertions -- which come from clang and therefore describe 32-bit Arm --
    // only hold when the two agree. Emitting them on a 64-bit host would fail
    // on every struct containing a pointer, which would break `cargo check`
    // and rust-analyzer for no benefit.
    let rust_target = std::env::var("TARGET").unwrap();
    let layout_tests = rust_target.starts_with("thumb");

    let mut builder = bindgen::builder()
        .header(wrapper.display().to_string())
        .clang_arg(format!("--target={CLANG_TARGET}"))
        // Sizes every enum the way FSP's own toolchains do. clang's bare-metal
        // Arm default is `int`-sized enums, which disagrees with the
        // `Tag_ABI_enum_size = small` on every Renesas prebuilt library, and
        // with arm-none-eabi-gcc. Without this, bsp_io_level_t binds as 4 bytes
        // against a C side that sees 1. The static assertions in
        // shim/bsp_api.h fail the build if this is ever dropped.
        .clang_arg("-fshort-enums")
        .clang_arg("-ffreestanding")
        // Keeps the host SDK off the include path so a host header cannot leak
        // a host-sized type into a target-ABI binding. clang's own resource
        // headers (stdint.h, stddef.h, stdbool.h) stay available, which
        // -nostdinc would also have removed; shim/assert.h covers the one libc
        // header the interface layer includes.
        .clang_arg("-nostdlibinc")
        .clang_arg(format!("-I{}", shim.display()))
        .clang_arg(format!("-I{}", vendor_inc.display()))
        .clang_arg(format!("-I{}", vendor_inc.join("api").display()))
        .use_core()
        .ctypes_prefix("core::ffi")
        // A C enum's value is whatever the peripheral driver put there. Rust
        // enums make an out-of-range discriminant undefined behaviour, and FSP
        // has several enums whose full range is device-dependent, so no enum in
        // this crate is a Rust enum.
        .default_enum_style(EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .newtype_enum(ERROR_ENUM)
        // Doxygen, not Rust: the bodies are full of `@ref`, `@note` and bare
        // indented blocks that bindgen renders as doc comments containing
        // would-be doctests. The vendored headers are the place to read them.
        .generate_comments(false)
        .layout_tests(layout_tests)
        .parse_callbacks(Box::new(Callbacks))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    for item in BLOCKLIST {
        builder = builder.blocklist_item(item);
    }

    let bindings = builder.generate().expect("failed to generate FSP bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");

    if std::env::var_os(UPDATE_SNAPSHOT_VAR).is_some() {
        update_snapshot(&manifest_dir, &out_dir, layout_tests);
    }
}

/// Copy the freshly generated bindings over the committed snapshot.
///
/// Only the host build may do this. The layout assertions are
/// target-conditional, so a cross build's `bindings.rs` carries several hundred
/// extra lines; accepting it here would make the snapshot's contents depend on
/// who last ran the command. The layout-test-free form is the canonical one.
fn update_snapshot(manifest_dir: &Path, out_dir: &Path, layout_tests: bool) {
    assert!(
        !layout_tests,
        "refusing to update {SNAPSHOT} from a cross build: the snapshot is the \
         layout-test-free form. Re-run without --target."
    );

    let path = manifest_dir.join(SNAPSHOT);
    std::fs::create_dir_all(path.parent().unwrap()).expect("failed to create bindings dir");
    std::fs::copy(out_dir.join("bindings.rs"), &path).expect("failed to write bindings snapshot");

    println!("cargo:warning=updated {SNAPSHOT}");
}
