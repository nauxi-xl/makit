//! The host Toolchain: the tools that build Host Goals and run during the build itself.
//!
//! Unlike target Toolchains it may be left undeclared, in which case it is detected. The resolved
//! tools and their Toolchain identity are recorded in the Output tree, so a later build that would
//! resolve a different host stops instead of silently rebuilding everything (ADR-0012).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::manifest::{Manifest, Role, ToolCommand};
use crate::output_tree::OutputTree;

/// File in the Output tree recording the host Toolchain it was configured with.
pub const RECORD_FILE: &str = ".host-toolchain.toml";

/// Host tools Makit looks for when the manifest does not name them: Build file variable, environment
/// variable consulted, program searched on `PATH`.
const DETECTED: &[(&str, Option<&str>, &str)] = &[
    ("HOSTCC", Some("HOSTCC"), "cc"),
    ("HOSTCXX", Some("HOSTCXX"), "c++"),
    ("HOSTRUSTC", None, "rustc"),
    ("HOSTCARGO", None, "cargo"),
];

/// One resolved host tool, keyed by its Build file variable (`HOSTCC`, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTool {
    /// Absolute program path followed by its fixed arguments.
    pub command: Vec<String>,
    /// BLAKE3 of the program binary and its `--version` output.
    pub identity: String,
}

pub type HostToolchain = BTreeMap<String, ResolvedTool>;

#[derive(Debug)]
pub enum HostError {
    /// A tool the user named explicitly does not exist.
    NotFound {
        variable: String,
        program: String,
    },
    /// The Output tree was configured with a different host Toolchain.
    Changed {
        variables: Vec<String>,
    },
    Io(PathBuf, std::io::Error),
}

/// Host tools the manifest declares, by Build file variable (`HOSTCC`, ...).
fn declared(manifest: &Manifest) -> BTreeMap<String, &ToolCommand> {
    manifest
        .toolchains
        .values()
        .filter(|toolchain| toolchain.role == Role::Host)
        .flat_map(|toolchain| &toolchain.tools)
        .map(|(name, command)| (format!("HOST{name}"), command))
        .collect()
}

/// Every host tool variable: the detected ones plus whatever the manifest declares.
pub fn variables(manifest: &Manifest) -> BTreeSet<String> {
    DETECTED
        .iter()
        .map(|(variable, ..)| (*variable).to_owned())
        .chain(declared(manifest).into_keys())
        .collect()
}

/// Resolves every host tool: command-line override, then the manifest's host Toolchain, then the
/// environment, then `PATH`.
pub fn resolve(
    manifest: &Manifest,
    overrides: &BTreeMap<String, String>,
    source_root: &Path,
) -> Result<HostToolchain, HostError> {
    let declared = declared(manifest);
    let mut host = HostToolchain::new();
    for variable in variables(manifest) {
        let variable = variable.as_str();
        let detected = DETECTED.iter().find(|(v, ..)| *v == variable);
        let explicit = overrides
            .get(variable)
            .map(|program| ToolCommand {
                program: program.clone(),
                args: vec![],
            })
            .or_else(|| declared.get(variable).map(|command| (*command).clone()))
            .or_else(|| {
                let (_, env, _) = detected?;
                let program = std::env::var(env.as_ref()?).ok()?;
                Some(ToolCommand {
                    program,
                    args: vec![],
                })
            });
        let resolved =
            match explicit {
                Some(command) => Some(resolve_command(&command, source_root).ok_or_else(|| {
                    HostError::NotFound {
                        variable: variable.to_owned(),
                        program: command.program.clone(),
                    }
                })?),
                None => detected.and_then(|(_, _, program)| {
                    let command = ToolCommand {
                        program: (*program).to_owned(),
                        args: vec![],
                    };
                    resolve_command(&command, source_root)
                }),
            };
        if let Some(tool) = resolved {
            host.insert(variable.to_owned(), tool);
        }
    }
    Ok(host)
}

/// Records `host` in a fresh Output tree, or checks it against what the Output tree recorded.
pub fn record_or_check(output_tree: &OutputTree, host: &HostToolchain) -> Result<(), HostError> {
    let path = output_tree.root.join(RECORD_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let recorded: HostToolchain = toml::from_str(&text).unwrap_or_default();
            let mut variables: Vec<String> = recorded
                .keys()
                .chain(host.keys())
                .filter(|variable| recorded.get(*variable) != host.get(*variable))
                .cloned()
                .collect();
            variables.dedup();
            if variables.is_empty() {
                Ok(())
            } else {
                Err(HostError::Changed { variables })
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let io = |e| HostError::Io(path.clone(), e);
            std::fs::create_dir_all(&output_tree.root).map_err(io)?;
            let text = toml::to_string(host).expect("a host Toolchain serializes to TOML");
            std::fs::write(&path, text).map_err(io)
        }
        Err(e) => Err(HostError::Io(path, e)),
    }
}

fn resolve_command(command: &ToolCommand, source_root: &Path) -> Option<ResolvedTool> {
    let program = find_program(&command.program, source_root)?;
    let identity = identity(&program)?;
    let mut argv = vec![program.display().to_string()];
    argv.extend(command.args.iter().cloned());
    Some(ResolvedTool {
        command: argv,
        identity,
    })
}

/// Toolchain identity: BLAKE3 over the binary and what it reports for `--version`.
fn identity(program: &Path) -> Option<String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(std::fs::File::open(program).ok()?)
        .ok()?;
    hasher.update(b"\0");
    if let Ok(output) = Command::new(program).arg("--version").output() {
        hasher.update(&output.stdout);
    }
    Some(hasher.finalize().to_hex().to_string())
}

/// A program path relative to the Source tree, or a bare name searched on `PATH`.
fn find_program(program: &str, source_root: &Path) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        let path = source_root.join(candidate);
        return is_executable(&path).then_some(path);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| executable_names(program).map(move |name| dir.join(name)))
        .find(|path| is_executable(path))
}

#[cfg(unix)]
fn executable_names(program: &str) -> impl Iterator<Item = OsString> {
    std::iter::once(OsString::from(program))
}

/// `cc` may be `cc.exe` or `cc.cmd`: try every extension in `PATHEXT`.
#[cfg(windows)]
fn executable_names(program: &str) -> impl Iterator<Item = OsString> {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".into());
    let with_extensions: Vec<OsString> = extensions
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(|ext| OsString::from(format!("{program}{ext}")))
        .collect();
    std::iter::once(OsString::from(program)).chain(with_extensions)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
