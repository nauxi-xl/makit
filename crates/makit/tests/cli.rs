//! Black-box tests of the `makit` command line: arguments in, exit code and output out.

mod support;

use support::{Project, assert_fails};

#[test]
fn help_lists_every_planned_subcommand() {
    let out = Project::minimal().makit(&["--help"]);
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    let listed: Vec<&str> = help
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    for subcommand in [
        "build",
        "olddefconfig",
        "savedefconfig",
        "explain",
        "test",
        "fetch",
        "run",
        "tool",
        "cache",
        "repro-check",
        "clean",
        "mrproper",
    ] {
        assert!(
            listed.contains(&subcommand),
            "`{subcommand}` not in {listed:?}"
        );
    }
    assert!(help.contains("makit -O <DIR> <name>_defconfig"), "{help}");
}

#[test]
fn subcommands_using_an_output_tree_require_dash_o() {
    let project = Project::minimal();
    for args in [
        &[][..],
        &["build"],
        &["stm32f4_defconfig"],
        &["olddefconfig"],
        &["savedefconfig"],
        &["explain", "app/main.o"],
        &["test"],
        &["run", "flash"],
        &["repro-check"],
        &["clean"],
        &["mrproper"],
    ] {
        assert_fails(
            &project.makit(args),
            2,
            "needs an Output tree: pass -O <DIR>",
        );
    }
}

#[test]
fn names_that_are_neither_subcommands_nor_defconfigs_are_rejected() {
    assert_fails(
        &Project::minimal().makit(&["-O", "out", "frobnicate"]),
        2,
        "unknown subcommand `frobnicate`",
    );
}

#[test]
fn defconfig_names_need_a_non_empty_prefix_and_no_arguments() {
    let project = Project::minimal();
    assert_fails(
        &project.makit(&["-O", "out", "_defconfig"]),
        2,
        "unknown subcommand",
    );
    assert_fails(
        &project.makit(&["-O", "out", "foo_defconfig", "extra"]),
        2,
        "unknown subcommand",
    );
}

#[test]
fn unimplemented_subcommands_point_to_their_tracking_issue() {
    let project = Project::minimal();
    for (args, issue) in [
        (&["-O", "out"][..], "#8"),
        (&["-O", "out", "build"], "#8"),
        (&["build", "-O", "out"], "#8"),
        (&["-O", "out", "stm32f4_defconfig"], "#5"),
        (&["olddefconfig", "-O", "out"], "#5"),
        (&["savedefconfig", "-O", "out"], "#5"),
        (&["explain", "app/main.o", "-O", "out"], "#9"),
        (&["test", "-O", "out"], "#16"),
        (&["fetch"], "#13"),
        (&["run", "flash", "-O", "out"], "#17"),
        (&["tool", "PANDOC", "run", "--version"], "#17"),
        (&["cache", "gc"], "#10"),
        (&["repro-check", "-O", "out"], "#11"),
        (&["clean", "-O", "out"], "#20"),
        (&["mrproper", "-O", "out"], "#20"),
    ] {
        assert_fails(
            &project.makit(args),
            1,
            &format!("is not implemented yet (tracked in {issue})"),
        );
    }
}

#[test]
fn everything_after_tool_run_belongs_to_the_tool() {
    let project = Project::minimal();
    for args in [
        &["tool", "CC", "run", "--help"][..],
        &["tool", "CC", "run", "-O", "x", "foo.c"],
        &["tool", "CC", "run", "-v"],
        &["tool", "CC", "run", "-O2", "-c", "foo.c"],
    ] {
        assert_fails(
            &project.makit(args),
            1,
            "`tool CC run` is not implemented yet (tracked in #17)",
        );
    }
    assert_fails(
        &project.makit(&["tool", "CC", "exec"]),
        2,
        "unknown subcommand `tool CC exec`",
    );
}
