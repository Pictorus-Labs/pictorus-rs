//! Guards the FSP ABI against silent movement.
//!
//! Nothing in this repository compiles the FSP C sources, so a pack upgrade
//! that reorders a struct field or renumbers an enum produces no compile error
//! here at all -- it produces a static library that links cleanly against the
//! new FSP and reads the wrong offsets. The committed bindings are the only
//! place that change becomes visible before it reaches a board.
//!
//! A failure is not necessarily a defect. It is a statement that the generated
//! declarations moved, and the diff is the thing to read.

/// The snapshot is generated with layout tests off, which is also how bindings
/// are generated for a host build (see `build.rs`), so on a host the two are
/// directly comparable. Under a cross build bindings carry layout assertions
/// and would differ for an uninteresting reason -- but `cargo test` does not
/// run there either, so the case does not arise.
#[test]
fn bindings_match_committed_snapshot() {
    let generated = include_str!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    let committed = include_str!("../bindings/generated.rs");

    if generated == committed {
        return;
    }

    let first_difference = generated
        .lines()
        .zip(committed.lines())
        .position(|(a, b)| a != b);

    panic!(
        "generated FSP bindings differ from renesas-fsp-sys/bindings/generated.rs\n\
         \n\
         first differing line: {}\n\
         generated: {} lines, committed: {} lines\n\
         \n\
         Review the change, then accept it with:\n\
         \n\
         \x20   RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys\n",
        first_difference
            .map(|n| (n + 1).to_string())
            .unwrap_or_else(|| "none (one file is a prefix of the other)".into()),
        generated.lines().count(),
        committed.lines().count(),
    );
}
