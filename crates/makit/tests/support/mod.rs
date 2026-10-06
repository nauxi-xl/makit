//! Shared helpers for black-box tests: throwaway projects and assertions on `makit` runs.

#![allow(dead_code)] // each test crate uses a different subset

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A Project manifest with just enough to load: one target Toolchain.
pub const MINIMAL_MANIFEST: &str = r#"
[toolchain.arm-gcc]
role = "target"
CC = "arm-none-eabi-gcc"
"#;

/// A scratch directory holding a Source tree at `src/` (the working directory of every run).
pub struct Project {
    pub scratch: PathBuf,
    pub src: PathBuf,
}

impl Project {
    /// A Source tree whose `.makit/config.toml` is `manifest`.
    pub fn new(manifest: &str) -> Self {
        let project = Self::without_manifest();
        project.write(".makit/config.toml", manifest);
        project
    }

    pub fn minimal() -> Self {
        Self::new(MINIMAL_MANIFEST)
    }

    /// A Source tree directory with no `.makit/` at all.
    pub fn without_manifest() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let scratch = std::env::temp_dir().join(format!(
            "makit-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let src = scratch.join("src");
        std::fs::create_dir_all(&src).unwrap();
        Self { scratch, src }
    }

    /// Writes `contents` to `path` relative to the Source tree.
    pub fn write(&self, path: &str, contents: &str) {
        let path = self.src.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// Runs `makit <args>` in the Source tree.
    pub fn makit(&self, args: &[&str]) -> Output {
        self.makit_with(args, &[])
    }

    /// Runs `makit <args>` in the Source tree with extra environment variables.
    ///
    /// `PATH` defaults to the project's own `bin/` (see `bin()`) and `HOSTCC`/`HOSTCXX` are cleared,
    /// so host Toolchain detection never sees the machine running the tests.
    pub fn makit_with(&self, args: &[&str], env: &[(&str, &Path)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_makit"));
        command
            .args(args)
            .current_dir(&self.src)
            .env("PATH", self.bin())
            .env_remove("HOSTCC")
            .env_remove("HOSTCXX");
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().expect("failed to run makit")
    }

    /// Runs `makit <args>` in `dir`, relative to the Source tree.
    pub fn makit_in(&self, dir: &str, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_makit"));
        command
            .args(args)
            .current_dir(self.src.join(dir))
            .env("PATH", self.bin())
            .env_remove("HOSTCC")
            .env_remove("HOSTCXX");
        command.output().expect("failed to run makit")
    }

    /// The directory on `PATH` for every run; empty unless a test puts tools there.
    pub fn bin(&self) -> PathBuf {
        self.scratch.join("bin")
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Asserts that a run exited with `code` and said `message` on stderr.
#[track_caller]
pub fn assert_fails(out: &Output, code: i32, message: &str) {
    let stderr = stderr(out);
    assert_eq!(out.status.code(), Some(code), "{stderr}");
    assert!(
        stderr.contains(message),
        "expected {message:?} in: {stderr}"
    );
}

/// Writes an executable script `name` into `dir` that prints `version` (Unix only).
#[cfg(unix)]
pub fn fake_tool(dir: &Path, name: &str, version: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\necho '{version}'\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}
