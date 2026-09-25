//! Guards the C/Rust interface against silent movement.
//!
//! `rm_pictorus_app.h` is compiled twice by two toolchains that never see each
//! other: once here, into a prebuilt static library, and once in whatever e2
//! studio project imported the generated pack. A field added, reordered or
//! resized between those two builds produces no error on either side -- it
//! produces a shim that hands the library a struct whose fields are at the
//! wrong offsets.
//!
//! The committed bindings are where that becomes visible. A failure here means
//! the C/Rust bindings have changed: check that the change is intended, and that
//! anything already installed in an e2 studio project gets rebuilt from the same
//! version of this header.

#[test]
fn seam_bindings_match_committed_snapshot() {
    let generated = include_str!(concat!(env!("OUT_DIR"), "/seam.rs"));
    let committed = include_str!("../bindings/seam.rs");

    if generated == committed {
        return;
    }

    let first_difference = generated
        .lines()
        .zip(committed.lines())
        .position(|(a, b)| a != b);

    panic!(
        "generated seam bindings differ from pictorus-fsp/bindings/seam.rs\n\
         \n\
         first differing line: {}\n\
         generated: {} lines, committed: {} lines\n\
         \n\
         If the seam really did change, accept the new bindings with:\n\
         \n\
         \x20   PICTORUS_FSP_UPDATE_SNAPSHOT=1 cargo build -p pictorus-fsp\n",
        first_difference
            .map(|n| (n + 1).to_string())
            .unwrap_or_else(|| "none (one file is a prefix of the other)".into()),
        generated.lines().count(),
        committed.lines().count(),
    );
}
