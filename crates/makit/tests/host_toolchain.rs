//! Resolving the host Toolchain and recording it in the Output tree (ADR-0012).
#![cfg(unix)]

mod support;

use support::{Project, assert_fails, fake_tool};

/// The record a build leaves in its Output tree.
fn host_record(project: &Project) -> String {
    std::fs::read_to_string(project.scratch.join("out/.host-toolchain.toml")).unwrap()
}

#[test]
fn an_undeclared_host_toolchain_is_detected_on_path_and_recorded() {
    let project = Project::minimal();
    let cc = fake_tool(&project.bin(), "cc", "fakecc 1.0");

    let out = project.makit(&["-O", "../out", "build"]);

    assert_fails(&out, 1, "not implemented yet");
    let record = host_record(&project);
    assert!(record.contains("[HOSTCC]"), "{record}");
    assert!(
        record.contains(&format!("{:?}", cc.display().to_string())),
        "{record}"
    );
    assert!(!record.contains("HOSTCXX"), "no c++ on PATH: {record}");
    let identity = record
        .lines()
        .find_map(|line| line.strip_prefix("identity = \""))
        .expect("an identity line");
    assert_eq!(identity.trim_end_matches('"').len(), 64, "{record}");
}

#[test]
fn a_changed_host_toolchain_stops_the_build() {
    let project = Project::minimal();
    fake_tool(&project.bin(), "cc", "fakecc 1.0");
    let build = || project.makit(&["-O", "../out", "build"]);

    assert_fails(&build(), 1, "not implemented yet");
    assert_fails(&build(), 1, "not implemented yet");

    fake_tool(&project.bin(), "cc", "fakecc 2.0");
    assert_fails(
        &build(),
        2,
        "the host Toolchain changed since this Output tree was configured (HOSTCC)",
    );

    fake_tool(&project.bin(), "cc", "fakecc 1.0");
    fake_tool(&project.bin(), "c++", "fakec++ 1.0");
    assert_fails(&build(), 2, "(HOSTCXX)");
}

/// Fake tools that exist but are not on `PATH`.
fn off_path(project: &Project, name: &str) -> std::path::PathBuf {
    fake_tool(&project.scratch.join("elsewhere"), name, name)
}

fn recorded_hostcc(project: &Project) -> String {
    let record = host_record(project);
    let section = record.split("[HOSTCC]").nth(1).expect("a HOSTCC entry");
    section
        .lines()
        .find(|line| line.starts_with("command"))
        .unwrap()
        .to_owned()
}

#[test]
fn hostcc_in_the_environment_beats_path() {
    let project = Project::minimal();
    fake_tool(&project.bin(), "cc", "path cc");
    let env_cc = off_path(&project, "env-cc");
    project.makit_with(&["-O", "../out", "build"], &[("HOSTCC", &env_cc)]);
    assert!(
        recorded_hostcc(&project).contains("env-cc"),
        "{}",
        host_record(&project)
    );
}

#[test]
fn a_declared_host_toolchain_beats_the_environment() {
    let manifest = format!(
        "{}\n[toolchain.host]\nrole = \"host\"\nCC = \"tools/declared-cc\"\n",
        support::MINIMAL_MANIFEST
    );
    let project = Project::new(&manifest);
    fake_tool(&project.src.join("tools"), "declared-cc", "declared");
    let env_cc = off_path(&project, "env-cc");
    project.makit_with(&["-O", "../out", "build"], &[("HOSTCC", &env_cc)]);
    assert!(
        recorded_hostcc(&project).contains("declared-cc"),
        "{}",
        host_record(&project)
    );
}

#[test]
fn a_command_line_override_beats_the_manifest() {
    let manifest = format!(
        "{}\n[toolchain.host]\nrole = \"host\"\nCC = \"tools/declared-cc\"\n",
        support::MINIMAL_MANIFEST
    );
    let project = Project::new(&manifest);
    fake_tool(&project.src.join("tools"), "declared-cc", "declared");
    let cli_cc = off_path(&project, "cli-cc");
    let assignment = format!("HOSTCC={}", cli_cc.display());
    assert_fails(
        &project.makit(&["-O", "../out", &assignment, "build"]),
        1,
        "not implemented yet",
    );
    assert!(
        recorded_hostcc(&project).contains("cli-cc"),
        "{}",
        host_record(&project)
    );
}

#[test]
fn an_explicitly_named_host_tool_must_exist() {
    let project = Project::minimal();
    assert_fails(
        &project.makit(&["-O", "../out", "HOSTCC=no-such-cc", "build"]),
        2,
        "host tool HOSTCC: cannot find `no-such-cc`",
    );
}
