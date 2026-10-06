//! The host Toolchain: the tools that build Host Goals and run during the build itself.
//!
//! Unlike target Toolchains it may be left undeclared, in which case it is detected. The resolved
//! tools and their Toolchain identity are recorded in the Output tree, so a later build that would
//! resolve a different host stops instead of silently rebuilding everything (ADR-0012).

use std::collections::{BTreeMap, BTreeSet};
#[cfg(windows)]
use std::ffi::OsString;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::manifest::{Argv, Manifest};
use crate::output_tree::{OutputTree, normalize};

/// File in the Output tree recording the host Toolchain it was configured with.
pub const RECORD_FILE: &str = ".host-toolchain.toml";

/// How long a tool may take to answer `--version` before Makit gives up on it.
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);

/// A host tool Makit can find without a manifest declaration.
struct Detected {
    /// Build file variable, e.g. `HOSTCC`.
    variable: &'static str,
    /// Environment variable consulted first, Kbuild-style.
    env: Option<&'static str>,
    /// Program searched on `PATH`.
    program: &'static str,
}

const DETECTED: &[Detected] = &[
    Detected {
        variable: "HOSTCC",
        env: Some("HOSTCC"),
        program: "cc",
    },
    Detected {
        variable: "HOSTCXX",
        env: Some("HOSTCXX"),
        program: "c++",
    },
    Detected {
        variable: "HOSTRUSTC",
        env: None,
        program: "rustc",
    },
    Detected {
        variable: "HOSTCARGO",
        env: None,
        program: "cargo",
    },
];

/// One resolved host tool, keyed by its Build file variable (`HOSTCC`, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTool {
    /// Absolute program path followed by its fixed arguments.
    pub argv: Vec<String>,
    /// BLAKE3 of the program binary, its `--version` exit status and output.
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
        variables: BTreeSet<String>,
    },
    /// The record in the Output tree cannot be read back.
    DamagedRecord(PathBuf),
    Io(PathBuf, std::io::Error),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { variable, program } => {
                write!(f, "host tool {variable}: cannot find `{program}`")
            }
            Self::Changed { variables } => write!(
                f,
                "the host Toolchain changed since this Output tree was configured ({}); \
                 rebuild in a new Output tree, or delete {RECORD_FILE} from this one to accept \
                 the change",
                variables.iter().cloned().collect::<Vec<_>>().join(", ")
            ),
            Self::DamagedRecord(path) => write!(
                f,
                "{} is damaged; delete it to record the host Toolchain again",
                path.display()
            ),
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
        }
    }
}

/// Every host tool variable that can be overridden: the detected ones plus whatever the manifest
/// declares.
pub fn variables(manifest: &Manifest) -> BTreeSet<String> {
    DETECTED
        .iter()
        .map(|detected| detected.variable.to_owned())
        .chain(
            manifest
                .host_tools()
                .keys()
                .map(|name| format!("HOST{name}")),
        )
        .collect()
}

/// Resolves every host tool. In order: a command-line override; else, when the manifest declares
/// a host Toolchain, exactly what it declares; else the environment, then `PATH`.
///
/// Manifest paths are relative to the Source tree; command-line and environment paths to `cwd`.
pub fn resolve(
    manifest: &Manifest,
    overrides: &BTreeMap<String, String>,
    source_root: &Path,
    cwd: &Path,
) -> Result<HostToolchain, HostError> {
    let declared = manifest.host_tools();
    let mut host = HostToolchain::new();
    for variable in variables(manifest) {
        let detected = DETECTED.iter().find(|d| d.variable == variable);
        let from_user = |value: &str| Argv::from_words(value).map(|argv| (argv, cwd));
        let explicit = match overrides.get(&variable) {
            Some(value) => from_user(value),
            None if manifest.declares_host() => variable
                .strip_prefix("HOST")
                .and_then(|name| declared.get(name))
                .map(|argv| ((*argv).clone(), source_root)),
            None => detected
                .and_then(|d| std::env::var(d.env?).ok())
                .and_then(|value| from_user(&value)),
        };
        let resolved = match explicit {
            Some((argv, base)) => Some(resolve_argv(&argv, base).ok_or(HostError::NotFound {
                variable: variable.clone(),
                program: argv.program,
            })?),
            None if manifest.declares_host() => None,
            None => detected.and_then(|d| {
                let argv = Argv {
                    program: d.program.to_owned(),
                    args: vec![],
                };
                resolve_argv(&argv, cwd)
            }),
        };
        if let Some(tool) = resolved {
            host.insert(variable, tool);
        }
    }
    Ok(host)
}

/// Records `host` in a fresh Output tree, or checks it against what the Output tree recorded.
pub fn record_or_check(output_tree: &OutputTree, host: &HostToolchain) -> Result<(), HostError> {
    let path = output_tree.root.join(RECORD_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let recorded: HostToolchain =
                toml::from_str(&text).map_err(|_| HostError::DamagedRecord(path.clone()))?;
            let variables: BTreeSet<String> = recorded
                .keys()
                .chain(host.keys())
                .filter(|variable| recorded.get(*variable) != host.get(*variable))
                .cloned()
                .collect();
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

fn resolve_argv(argv: &Argv, base: &Path) -> Option<ResolvedTool> {
    let program = find_program(&argv.program, base)?;
    let identity = identity(&program)?;
    let mut resolved = vec![program.display().to_string()];
    resolved.extend(argv.args.iter().cloned());
    Some(ResolvedTool {
        argv: resolved,
        identity,
    })
}

/// Toolchain identity: BLAKE3 over the binary and what it answers to `--version`.
fn identity(program: &Path) -> Option<String> {
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(std::fs::File::open(program).ok()?)
        .ok()?;
    hasher.update(b"\0");
    hasher.update(&version_answer(program));
    Some(hasher.finalize().to_hex().to_string())
}

/// The exit status and stdout of `program --version`, or nothing if it does not finish in time.
fn version_answer(program: &Path) -> Vec<u8> {
    let Ok(mut child) = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return b"cannot run".to_vec();
    };
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        output
    });
    let deadline = Instant::now() + VERSION_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let Some(status) = status else {
        return b"timed out".to_vec();
    };
    let mut answer = format!("{:?}\0", status.code()).into_bytes();
    answer.extend(reader.join().unwrap_or_default());
    answer
}

/// A program path (relative to `base`), or a bare name searched on `PATH`.
fn find_program(program: &str, base: &Path) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        let path = normalize(&base.join(candidate));
        return executable_names(&path).find(|path| is_executable(path));
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| executable_names(&dir.join(program)).collect::<Vec<_>>())
        .find(|path| is_executable(path))
}

#[cfg(unix)]
fn executable_names(path: &Path) -> impl Iterator<Item = PathBuf> {
    std::iter::once(path.to_path_buf())
}

/// `cc` may be `cc.exe` or `cc.cmd`: try every extension in `PATHEXT` before the bare name, as
/// Windows itself does.
#[cfg(windows)]
fn executable_names(path: &Path) -> impl Iterator<Item = PathBuf> {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".into());
    let mut names: Vec<PathBuf> = extensions
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(|ext| {
            let mut name = OsString::from(path.as_os_str());
            name.push(ext);
            PathBuf::from(name)
        })
        .collect();
    names.push(path.to_path_buf());
    names.into_iter()
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
