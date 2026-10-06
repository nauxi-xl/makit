//! The Makit repository is itself a Makit project (dogfooding; see `./bootstrap`).

mod support;

use std::path::Path;
use std::process::Command;

use support::{Project, assert_fails};

#[test]
fn the_makit_repository_is_a_valid_makit_project() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Only the scratch directory is used, as an Output tree outside the repository.
    let scratch = Project::without_manifest();
    let output = scratch.scratch.join("out");

    let out = Command::new(env!("CARGO_BIN_EXE_makit"))
        .arg("-O")
        .arg(&output)
        .arg("build")
        .current_dir(&repo)
        .output()
        .unwrap();

    // Reaching the executor stub means the Project manifest, Output tree and host Toolchain all
    // checked out.
    assert_fails(&out, 1, "`build` is not implemented yet (tracked in #8)");
    let record = std::fs::read_to_string(output.join(".host-toolchain.toml")).unwrap();
    assert!(record.contains("[HOSTCARGO]"), "{record}");
}
