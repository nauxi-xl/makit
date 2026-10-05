//! Black-box tests of the `makit` binary: arguments in, exit code and output out.

use std::path::Path;
use std::process::{Command, Output};

fn makit_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_makit"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run makit")
}

fn makit(args: &[&str]) -> Output {
    makit_in(&std::env::temp_dir(), args)
}

/// Asserts that `makit <args>` exits with `code` and says `message` on stderr.
fn assert_fails(args: &[&str], code: i32, message: &str) {
    let out = makit(args);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(code), "makit {args:?}: {stderr}");
    assert!(stderr.contains(message), "makit {args:?}: {stderr}");
}

#[test]
fn help_lists_every_planned_subcommand() {
    let out = makit(&["--help"]);
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
        assert_fails(args, 2, "needs an Output tree: pass -O <DIR>");
    }
}

#[test]
fn names_that_are_neither_subcommands_nor_defconfigs_are_rejected() {
    assert_fails(
        &["-O", "out", "frobnicate"],
        2,
        "unknown subcommand `frobnicate`",
    );
}

#[test]
fn defconfig_names_need_a_non_empty_prefix_and_no_arguments() {
    assert_fails(&["-O", "out", "_defconfig"], 2, "unknown subcommand");
    assert_fails(
        &["-O", "out", "foo_defconfig", "extra"],
        2,
        "unknown subcommand",
    );
}

#[test]
fn unimplemented_subcommands_point_to_their_tracking_issue() {
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
            args,
            1,
            &format!("is not implemented yet (tracked in {issue})"),
        );
    }
}

#[test]
fn everything_after_tool_run_belongs_to_the_tool() {
    for args in [
        &["tool", "CC", "run", "--help"][..],
        &["tool", "CC", "run", "-O", "x", "foo.c"],
        &["tool", "CC", "run", "-v"],
        &["tool", "CC", "run", "-O2", "-c", "foo.c"],
    ] {
        assert_fails(
            args,
            1,
            "`tool CC run` is not implemented yet (tracked in #17)",
        );
    }
    assert_fails(
        &["tool", "CC", "exec"],
        2,
        "unknown subcommand `tool CC exec`",
    );
}

#[test]
fn makit_writes_nothing_into_the_working_directory() {
    let dir = std::env::temp_dir().join(format!("makit-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for args in [
        &["-O", "out"][..],
        &["-O", "out", "stm32f4_defconfig"],
        &["--help"],
    ] {
        makit_in(&dir, args);
    }
    let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(leftovers.is_empty(), "makit wrote {leftovers:?}");
}
