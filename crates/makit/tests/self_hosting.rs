//! Makit's own Source tree is a Makit project (dogfooding; see `./bootstrap`).
//!
//! Unlike the other tests this one sees the real machine: host Toolchain detection uses the
//! `rustc` and `cargo` that built these tests.

mod support;

use std::path::Path;
use std::process::Command;

use support::{Project, assert_fails};

#[test]
fn makits_own_source_tree_is_a_valid_makit_project() {
    let source_tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // An Output tree outside the Source tree; the scratch project contributes only its directory.
    let scratch = Project::without_manifest();
    let output_tree = scratch.scratch.join("out");

    let run = Command::new(env!("CARGO_BIN_EXE_makit"))
        .arg("-O")
        .arg(&output_tree)
        .arg("build")
        .current_dir(&source_tree)
        .output()
        .unwrap();

    // Reaching the executor stub means the Project manifest, Output tree and host Toolchain all
    // checked out. Tripwire: once #8 lands this becomes a real build, and so must this test.
    assert_fails(&run, 1, "`build` is not implemented yet (tracked in #8)");
    let record = std::fs::read_to_string(output_tree.join(".host-toolchain.toml")).unwrap();
    assert!(record.contains("[HOSTCARGO]"), "{record}");
    assert!(
        !record.contains("[HOSTCC]"),
        "the declared host Toolchain has no CC: {record}"
    );
}
