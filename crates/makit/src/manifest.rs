//! The Project manifest `.makit/config.toml`: facts that are the same for every build of a
//! project, chiefly its Toolchain and Tool declarations (ADR-0012).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug)]
pub struct Manifest {
    pub toolchains: BTreeMap<String, Toolchain>,
    /// Tools from `[tools]`, by name.
    pub tools: BTreeMap<String, Argv>,
}

#[derive(Debug)]
pub struct Toolchain {
    pub role: Role,
    pub default: bool,
    /// The Toolchain's tools (CC, CXX, AR, ...), by name.
    pub tools: BTreeMap<String, Argv>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Host,
    Target,
}

/// How a tool is invoked: a program plus fixed leading arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argv {
    pub program: String,
    pub args: Vec<String>,
}

impl Argv {
    /// Splits a Kbuild-style value such as `HOSTCC="ccache cc"` on whitespace.
    pub fn from_words(value: &str) -> Option<Self> {
        let mut words = value.split_whitespace().map(str::to_owned);
        let program = words.next()?;
        Some(Self {
            program,
            args: words.collect(),
        })
    }
}

/// A Project manifest that cannot be used, and why.
#[derive(Debug)]
pub struct ManifestError {
    pub path: PathBuf,
    pub problem: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.problem)
    }
}

/// The manifest as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    #[serde(default)]
    project: RawProject,
    #[serde(default)]
    toolchain: BTreeMap<String, RawToolchain>,
    #[serde(default)]
    tools: BTreeMap<String, RawArgv>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawProject {
    #[allow(dead_code)] // accepted for documentation; nothing reads it yet
    name: Option<String>,
    /// Oldest Makit that can build this project.
    makit_version: Option<String>,
}

#[derive(Deserialize)]
struct RawToolchain {
    role: Role,
    #[serde(default)]
    default: bool,
    #[serde(flatten)]
    tools: BTreeMap<String, RawArgv>,
}

/// `CC = "gcc"` names a program; `CC = ["clang", "--target=arm-none-eabi"]` adds fixed arguments.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawArgv {
    Program(String),
    Words(Vec<String>),
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let fail = |problem: String| ManifestError {
            path: path.to_path_buf(),
            problem,
        };
        let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
        let raw: RawManifest = toml::from_str(&text).map_err(|e| fail(e.to_string()))?;
        if let Some(required) = &raw.project.makit_version {
            check_makit_version(required).map_err(fail)?;
        }

        let mut toolchains = BTreeMap::new();
        for (name, raw) in raw.toolchain {
            let toolchain = Toolchain {
                role: raw.role,
                default: raw.default,
                tools: argvs(raw.tools, &format!("[toolchain.{name}]"), "Toolchain tool")
                    .map_err(fail)?,
            };
            toolchains.insert(name, toolchain);
        }
        let tools = argvs(raw.tools, "[tools]", "Tool").map_err(fail)?;

        let manifest = Self { toolchains, tools };
        manifest.check_roles().map_err(fail)?;
        Ok(manifest)
    }

    /// Host Toolchain tools by name (`CC`, ...); empty when the host Toolchain is undeclared.
    pub fn host_tools(&self) -> BTreeMap<&str, &Argv> {
        self.toolchains
            .values()
            .filter(|toolchain| toolchain.role == Role::Host)
            .flat_map(|toolchain| &toolchain.tools)
            .map(|(name, argv)| (name.as_str(), argv))
            .collect()
    }

    pub fn declares_host(&self) -> bool {
        self.toolchains
            .values()
            .any(|toolchain| toolchain.role == Role::Host)
    }

    /// Names of every Tool (`[tools]`) and every target Toolchain tool.
    pub fn target_tool_names(&self) -> impl Iterator<Item = &str> {
        self.toolchains
            .values()
            .filter(|toolchain| toolchain.role == Role::Target)
            .flat_map(|toolchain| toolchain.tools.keys())
            .chain(self.tools.keys())
            .map(String::as_str)
    }

    fn names_with_role(&self, role: Role) -> Vec<&str> {
        self.toolchains
            .iter()
            .filter(|(_, toolchain)| toolchain.role == role)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// One or more target Toolchains with an unambiguous default, and at most one host Toolchain.
    fn check_roles(&self) -> Result<(), String> {
        let targets = self.names_with_role(Role::Target);
        let hosts = self.names_with_role(Role::Host);
        if targets.is_empty() {
            return Err(
                "declares no target Toolchain; add a [toolchain.<name>] table with role = \"target\""
                    .into(),
            );
        }
        if let Some(host) = hosts.iter().find(|name| self.toolchains[**name].default) {
            return Err(format!("host Toolchain `{host}` cannot be default"));
        }
        if hosts.len() > 1 {
            return Err(format!(
                "declares more than one host Toolchain ({})",
                hosts.join(", ")
            ));
        }
        let defaults = targets
            .iter()
            .filter(|name| self.toolchains[**name].default)
            .count();
        if targets.len() > 1 && defaults != 1 {
            return Err(format!(
                "mark exactly one target Toolchain with default = true ({})",
                targets.join(", ")
            ));
        }
        Ok(())
    }
}

/// Validates the tools of one table; `table` and `kind` only shape the error messages.
fn argvs(
    raw: BTreeMap<String, RawArgv>,
    table: &str,
    kind: &str,
) -> Result<BTreeMap<String, Argv>, String> {
    raw.into_iter()
        .map(|(name, command)| {
            if !is_tool_name(&name) {
                return Err(format!(
                    "`{name}` in {table} is not a valid {kind} name; names are upper-case, like CC or PANDOC"
                ));
            }
            let mut argv = match command {
                RawArgv::Program(program) => vec![program],
                RawArgv::Words(argv) => argv,
            };
            if argv.first().is_none_or(|program| program.is_empty()) {
                return Err(format!("`{name}` in {table} needs a program to run"));
            }
            let program = argv.remove(0);
            Ok((name, Argv { program, args: argv }))
        })
        .collect()
}

/// Fails when this Makit is older than `required` (`MAJOR.MINOR` or `MAJOR.MINOR.PATCH`).
fn check_makit_version(required: &str) -> Result<(), String> {
    let parse = |version: &str| -> Option<Vec<u64>> {
        let parts: Option<Vec<u64>> = version.split('.').map(|p| p.parse().ok()).collect();
        parts.filter(|p| (2..=3).contains(&p.len()))
    };
    let running = env!("CARGO_PKG_VERSION");
    let Some(mut wanted) = parse(required) else {
        return Err(format!(
            "makit-version `{required}` is not a version like 0.1 or 0.1.2"
        ));
    };
    wanted.resize(3, 0);
    let have = parse(running).expect("Makit's own version is MAJOR.MINOR.PATCH");
    if have < wanted {
        return Err(format!(
            "this project requires Makit {required} or newer, but this is Makit {running}"
        ));
    }
    Ok(())
}

/// Tool names double as Build file variables (`$(CC)`), so they follow Make's upper-case style.
pub fn is_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
