//! Black-box tests of the `makit` binary: arguments in, exit code and output out.

use std::process::{Command, Output};

fn makit(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_makit"))
        .args(args)
        .output()
        .expect("failed to run makit")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn help_lists_every_planned_command() {
    let out = makit(&["--help"]);
    assert!(out.status.success());
    let help = stdout(&out);
    for command in [
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
        "<name>_defconfig",
    ] {
        assert!(
            help.contains(command),
            "`{command}` missing from --help:\n{help}"
        );
    }
}

#[test]
fn commands_using_an_output_tree_require_dash_o() {
    for args in [
        &[][..],
        &["build"],
        &["stm32f4_defconfig"],
        &["olddefconfig"],
        &["savedefconfig"],
        &["explain", "app/main.o"],
        &["test"],
        &["run", "flash"],
        &["clean"],
        &["mrproper"],
    ] {
        let out = makit(args);
        assert_eq!(out.status.code(), Some(2), "makit {args:?}");
        assert!(
            stderr(&out).contains("needs an Output tree: pass -O <DIR>"),
            "makit {args:?}: {}",
            stderr(&out)
        );
    }
}

#[test]
fn names_that_are_neither_commands_nor_defconfigs_are_rejected() {
    let out = makit(&["-O", "out", "frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("unknown command `frobnicate`"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn unimplemented_commands_point_to_their_tracking_issue() {
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
        (&["repro-check"], "#11"),
        (&["clean", "-O", "out"], "#20"),
        (&["mrproper", "-O", "out"], "#20"),
    ] {
        let out = makit(args);
        assert_eq!(
            out.status.code(),
            Some(1),
            "makit {args:?}: {}",
            stderr(&out)
        );
        assert!(
            stderr(&out).contains(&format!("is not implemented yet (tracked in {issue})")),
            "makit {args:?}: {}",
            stderr(&out)
        );
    }
}

#[test]
fn defconfig_names_need_a_non_empty_prefix_and_no_arguments() {
    for args in [
        &["-O", "out", "_defconfig"][..],
        &["-O", "out", "foo_defconfig", "extra"],
    ] {
        let out = makit(args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "makit {args:?}: {}",
            stderr(&out)
        );
        assert!(
            stderr(&out).contains("unknown command"),
            "makit {args:?}: {}",
            stderr(&out)
        );
    }
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
        Command::new(env!("CARGO_BIN_EXE_makit"))
            .args(args)
            .current_dir(&dir)
            .output()
            .expect("failed to run makit");
    }
    let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(leftovers.is_empty(), "makit wrote {leftovers:?}");
}
