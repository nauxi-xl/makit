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
        .find(|line| line.starts_with("argv"))
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

#[test]
fn overrides_may_follow_the_subcommand_and_dash_o_values_are_not_scanned() {
    let project = Project::minimal();
    let cli_cc = off_path(&project, "cli-cc");
    let assignment = format!("HOSTCC={}", cli_cc.display());
    // `tool` here is the Output tree's name, not the `tool` subcommand.
    project.makit(&["-O", "tool", "build", &assignment]);
    let record = std::fs::read_to_string(project.src.join("tool/.host-toolchain.toml")).unwrap();
    assert!(record.contains("cli-cc"), "{record}");
}

#[test]
fn relative_command_line_and_environment_paths_are_relative_to_the_working_directory() {
    let project = Project::minimal();
    fake_tool(&project.src.join("sub"), "here-cc", "here");
    let out = project.makit_in("sub", &["-O", "../../out", "HOSTCC=./here-cc", "build"]);
    assert_fails(&out, 1, "not implemented yet");
    assert!(
        recorded_hostcc(&project).contains("sub/here-cc"),
        "{}",
        host_record(&project)
    );
}

#[test]
fn hostcc_may_carry_arguments_like_in_kbuild() {
    let project = Project::minimal();
    let cc = off_path(&project, "env-cc");
    let value = std::path::PathBuf::from(format!("{} -m32", cc.display()));
    project.makit_with(&["-O", "../out", "build"], &[("HOSTCC", &value)]);
    let line = recorded_hostcc(&project);
    assert!(
        line.contains("env-cc") && line.contains("\"-m32\""),
        "{line}"
    );
}

#[test]
fn a_tool_that_hangs_on_version_does_not_hang_makit() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::minimal();
    let cc = project.bin().join("cc");
    std::fs::create_dir_all(project.bin()).unwrap();
    let sleep = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("sleep"))
        .find(|path| path.is_file())
        .expect("sleep on PATH");
    std::fs::write(&cc, format!("#!/bin/sh\nexec {} 60\n", sleep.display())).unwrap();
    std::fs::set_permissions(&cc, std::fs::Permissions::from_mode(0o755)).unwrap();
    let started = std::time::Instant::now();
    assert_fails(
        &project.makit(&["-O", "../out", "build"]),
        1,
        "not implemented yet",
    );
    assert!(
        started.elapsed().as_secs() < 30,
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_damaged_record_is_reported_not_treated_as_a_change() {
    let project = Project::minimal();
    fake_tool(&project.bin(), "cc", "fakecc 1.0");
    project.makit(&["-O", "../out", "build"]);
    std::fs::write(project.scratch.join("out/.host-toolchain.toml"), "[[[").unwrap();
    assert_fails(&project.makit(&["-O", "../out", "build"]), 2, "is damaged");
}

#[test]
fn every_changed_host_tool_is_listed_once() {
    let project = Project::minimal();
    fake_tool(&project.bin(), "cc", "1");
    fake_tool(&project.bin(), "c++", "1");
    project.makit(&["-O", "../out", "build"]);
    fake_tool(&project.bin(), "cc", "2");
    fake_tool(&project.bin(), "c++", "2");
    assert_fails(
        &project.makit(&["-O", "../out", "build"]),
        2,
        "configured (HOSTCC, HOSTCXX);",
    );
}

#[test]
fn a_declared_host_toolchain_is_not_topped_up_from_path() {
    let manifest = format!(
        "{}\n[toolchain.host]\nrole = \"host\"\nCC = \"tools/declared-cc\"\n",
        support::MINIMAL_MANIFEST
    );
    let project = Project::new(&manifest);
    fake_tool(&project.src.join("tools"), "declared-cc", "declared");
    fake_tool(&project.bin(), "c++", "path c++");
    project.makit(&["-O", "../out", "build"]);
    let record = host_record(&project);
    assert!(
        record.contains("[HOSTCC]") && !record.contains("HOSTCXX"),
        "{record}"
    );
}

#[test]
fn the_output_tree_must_not_reach_the_source_tree_through_a_symlink() {
    let project = Project::minimal();
    std::os::unix::fs::symlink(&project.scratch, project.scratch.join("link")).unwrap();
    assert_fails(
        &project.makit(&["-O", "../link", "build"]),
        2,
        "must not be or contain the Source tree",
    );
}
