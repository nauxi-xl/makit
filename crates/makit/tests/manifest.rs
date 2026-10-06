//! Loading the Project manifest `.makit/config.toml` and finding the Source tree.

mod support;

use support::{Project, assert_fails};

#[test]
fn a_source_tree_needs_a_project_manifest() {
    let project = Project::without_manifest();
    assert_fails(
        &project.makit(&["-O", "out", "build"]),
        2,
        "no Project manifest (.makit/config.toml) in",
    );
}

#[test]
fn a_project_must_declare_a_target_toolchain() {
    let project = Project::new(
        r#"
[toolchain.host]
role = "host"
CC = "cc"
"#,
    );
    assert_fails(
        &project.makit(&["-O", "out", "build"]),
        2,
        "declares no target Toolchain",
    );
}

#[test]
fn manifest_mistakes_are_reported_with_the_manifest_path() {
    for (manifest, problem) in [
        ("[toolchain.arm\nrole = ", "TOML parse error"),
        (
            "[toolchains.arm-gcc]\nrole = \"target\"",
            "unknown field `toolchains`",
        ),
        (
            "[toolchain.arm-gcc]\nrole = \"cross\"",
            "unknown variant `cross`",
        ),
        (
            "[toolchain.arm-gcc]\nrole = \"target\"\ncc = \"gcc\"",
            "`cc` is not a valid Tool name",
        ),
        (
            "[toolchain.arm-gcc]\nrole = \"target\"\n[tools]\npandoc = \"pandoc\"",
            "`pandoc` is not a valid Tool name",
        ),
        (
            "[toolchain.arm-gcc]\nrole = \"target\"\nCC = []",
            "`CC` needs a program",
        ),
    ] {
        let out = Project::new(manifest).makit(&["-O", "out", "build"]);
        assert_fails(&out, 2, "config.toml: ");
        assert_fails(&out, 2, problem);
    }
}

const TWO_TARGETS: &str = r#"
[toolchain.arm-gcc]
role = "target"
CC = "arm-none-eabi-gcc"

[toolchain.arm-clang]
role = "target"
CC = ["clang", "--target=arm-none-eabi"]
"#;

#[test]
fn a_single_target_toolchain_is_the_default() {
    assert_fails(
        &Project::minimal().makit(&["-O", "out", "build"]),
        1,
        "not implemented yet",
    );
}

#[test]
fn several_target_toolchains_need_exactly_one_default() {
    assert_fails(
        &Project::new(TWO_TARGETS).makit(&["-O", "out", "build"]),
        2,
        "mark exactly one target Toolchain with default = true (arm-clang, arm-gcc)",
    );
    let both = TWO_TARGETS.replace("role = \"target\"", "role = \"target\"\ndefault = true");
    assert_fails(
        &Project::new(&both).makit(&["-O", "out", "build"]),
        2,
        "mark exactly one target Toolchain with default = true (arm-clang, arm-gcc)",
    );
    let one = TWO_TARGETS.replacen("role = \"target\"", "role = \"target\"\ndefault = true", 1);
    assert_fails(
        &Project::new(&one).makit(&["-O", "out", "build"]),
        1,
        "not implemented yet",
    );
}

#[test]
fn host_toolchains_are_never_default_and_at_most_one() {
    let host = |name: &str, extra: &str| {
        format!(
            "{}\n[toolchain.{name}]\nrole = \"host\"\n{extra}",
            support::MINIMAL_MANIFEST
        )
    };
    assert_fails(
        &Project::new(&host("host", "default = true")).makit(&["-O", "out", "build"]),
        2,
        "host Toolchain `host` cannot be default",
    );
    let two_hosts = format!("{}\n[toolchain.host2]\nrole = \"host\"", host("host", ""));
    assert_fails(
        &Project::new(&two_hosts).makit(&["-O", "out", "build"]),
        2,
        "more than one host Toolchain (host, host2)",
    );
}

#[test]
fn a_project_can_require_a_minimum_makit_version() {
    let manifest = |version: &str| {
        format!(
            "[project]\nname = \"sensord\"\nmakit-version = \"{version}\"\n{}",
            support::MINIMAL_MANIFEST
        )
    };
    assert_fails(
        &Project::new(&manifest("0.1")).makit(&["-O", "out", "build"]),
        1,
        "not implemented yet",
    );
    assert_fails(
        &Project::new(&manifest("99.0")).makit(&["-O", "out", "build"]),
        2,
        "requires Makit 99.0 or newer, but this is Makit 0.1.0",
    );
    assert_fails(
        &Project::new(&manifest("soon")).makit(&["-O", "out", "build"]),
        2,
        "makit-version `soon` is not a version like 0.1 or 0.1.2",
    );
}

#[test]
fn the_output_tree_must_not_be_or_contain_the_source_tree() {
    let project = Project::minimal();
    for dir in [".", "./", "sub/..", ".."] {
        assert_fails(
            &project.makit(&["-O", dir, "build"]),
            2,
            "must not be or contain the Source tree",
        );
    }
    for dir in ["../out", "out"] {
        assert_fails(
            &project.makit(&["-O", dir, "build"]),
            1,
            "not implemented yet",
        );
    }
}

#[test]
fn overriding_a_declared_tool_marks_the_build_as_diverging() {
    let manifest = format!(
        "{}\n[tools]\nPANDOC = \"pandoc\"\n",
        support::MINIMAL_MANIFEST
    );
    let project = Project::new(&manifest);
    for assignment in ["CC=clang", "PANDOC=/opt/pandoc"] {
        let out = project.makit(&["-O", "out", assignment, "build"]);
        assert_fails(&out, 1, "not implemented yet");
        assert_fails(
            &out,
            1,
            &format!(
                "warning: {assignment} overrides the Project manifest; this build diverges from its Defconfig"
            ),
        );
    }
}

#[test]
fn overrides_must_name_a_tool() {
    assert_fails(
        &Project::minimal().makit(&["-O", "out", "V=1", "build"]),
        2,
        "unknown override `V`; overrides name a Tool from the Project manifest or a host tool like HOSTCC",
    );
}

#[test]
fn makit_writes_nothing_into_the_source_tree() {
    let project = Project::minimal();
    for args in [
        &["-O", "../out", "build"][..],
        &["-O", "../out", "stm32f4_defconfig"],
    ] {
        project.makit(args);
    }
    let entries: Vec<_> = std::fs::read_dir(&project.src)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, [".makit"], "the Source tree gained files");
    assert!(
        project.scratch.join("out").is_dir(),
        "the Output tree was not created"
    );
}
