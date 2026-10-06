//! The `makit` binary: resolves the command line into an Invocation and dispatches it.

mod cli;
mod host_toolchain;
mod manifest;
mod output_tree;
mod source_tree;

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use cli::{CacheSubcommand, Cli, CliSubcommand, ToolSubcommand};
use host_toolchain::HostError;
use manifest::{Manifest, ManifestError};
use output_tree::{OutputTree, OutputTreeError};
use source_tree::{MANIFEST_PATH, SourceTree};

/// What the user asked Makit to do, after resolving CLI syntax.
#[derive(Debug)]
enum Invocation {
    Build,
    /// Seed a Configuration from `configs/<name>_defconfig`.
    Defconfig(String),
    Olddefconfig,
    Savedefconfig,
    Explain(PathBuf),
    Test,
    Fetch,
    /// Run the named Command.
    Run(String),
    /// Run the named Tool; its arguments are forwarded once #17 lands.
    Tool(String),
    CacheGc,
    ReproCheck,
    Clean,
    Mrproper,
}

impl Invocation {
    fn from_cli(subcommand: Option<CliSubcommand>) -> Result<Self, Error> {
        Ok(match subcommand {
            None | Some(CliSubcommand::Build) => Self::Build,
            Some(CliSubcommand::Olddefconfig) => Self::Olddefconfig,
            Some(CliSubcommand::Savedefconfig) => Self::Savedefconfig,
            Some(CliSubcommand::Explain { goal }) => Self::Explain(goal),
            Some(CliSubcommand::Test) => Self::Test,
            Some(CliSubcommand::Fetch) => Self::Fetch,
            Some(CliSubcommand::Run { command }) => Self::Run(command),
            Some(CliSubcommand::Tool {
                name,
                subcommand: ToolSubcommand::External(args),
            }) => {
                if args[0] != "run" {
                    let typed = args[0].to_string_lossy();
                    return Err(Error::UnknownSubcommand(format!("tool {name} {typed}")));
                }
                Self::Tool(name)
            }
            Some(CliSubcommand::Cache {
                subcommand: CacheSubcommand::Gc,
            }) => Self::CacheGc,
            Some(CliSubcommand::ReproCheck) => Self::ReproCheck,
            Some(CliSubcommand::Clean) => Self::Clean,
            Some(CliSubcommand::Mrproper) => Self::Mrproper,
            Some(CliSubcommand::External(args)) => {
                let name = args[0].to_string_lossy().into_owned();
                match name.strip_suffix("_defconfig") {
                    Some(defconfig) if args.len() == 1 && !defconfig.is_empty() => {
                        Self::Defconfig(defconfig.to_owned())
                    }
                    _ => return Err(Error::UnknownSubcommand(name)),
                }
            }
        })
    }

    /// The invocation as typed on the command line, without options.
    fn name(&self) -> String {
        match self {
            Self::Build => "build".into(),
            Self::Defconfig(name) => format!("{name}_defconfig"),
            Self::Olddefconfig => "olddefconfig".into(),
            Self::Savedefconfig => "savedefconfig".into(),
            Self::Explain(goal) => format!("explain {}", goal.display()),
            Self::Test => "test".into(),
            Self::Fetch => "fetch".into(),
            Self::Run(command) => format!("run {command}"),
            Self::Tool(name) => format!("tool {name} run"),
            Self::CacheGc => "cache gc".into(),
            Self::ReproCheck => "repro-check".into(),
            Self::Clean => "clean".into(),
            Self::Mrproper => "mrproper".into(),
        }
    }

    /// GitHub issue tracking the implementation of this invocation.
    fn tracking_issue(&self) -> u32 {
        match self {
            Self::Defconfig(_) | Self::Olddefconfig | Self::Savedefconfig => 5,
            Self::Build => 8,
            Self::Explain(_) => 9,
            Self::CacheGc => 10,
            Self::ReproCheck => 11,
            Self::Fetch => 13,
            Self::Test => 16,
            Self::Run(_) | Self::Tool(_) => 17,
            Self::Clean | Self::Mrproper => 20,
        }
    }

    /// Whether the invocation reads or writes an Output tree, and so needs `-O`.
    ///
    /// Makit never picks a default Output tree, so it can never write into the Source tree by accident.
    fn needs_output_tree(&self) -> bool {
        match self {
            Self::Build
            | Self::Defconfig(_)
            | Self::Olddefconfig
            | Self::Savedefconfig
            | Self::Explain(_)
            | Self::Test
            | Self::Run(_)
            | Self::ReproCheck
            | Self::Clean
            | Self::Mrproper => true,
            // Toolchain tools (CC, CARGO, ...) will need -O to know the selected Toolchain; that is
            // checked once the Project manifest is loaded.
            Self::Tool(_) | Self::Fetch | Self::CacheGc => false,
        }
    }

    /// Whether the invocation works on a project, and so needs its Source tree and Project manifest.
    fn needs_source_tree(&self) -> bool {
        !matches!(self, Self::CacheGc)
    }
}

#[derive(Debug)]
enum Error {
    UnknownSubcommand(String),
    MissingOutputTree(String),
    WorkingDirectory(std::io::Error),
    NoProjectManifest(PathBuf),
    Manifest(ManifestError),
    OutputTree(OutputTreeError),
    Host(HostError),
    UnknownOverride(String),
    NotImplemented { invocation: String, issue: u32 },
}

impl Error {
    /// 1 when a valid invocation cannot run yet, 2 for everything the user has to fix.
    fn exit_code(&self) -> u8 {
        match self {
            Self::NotImplemented { .. } => 1,
            _ => 2,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSubcommand(name) => write!(
                f,
                "unknown subcommand `{name}` (Defconfigs are named `<name>_defconfig`)"
            ),
            Self::MissingOutputTree(name) => {
                write!(f, "`{name}` needs an Output tree: pass -O <DIR>")
            }
            Self::WorkingDirectory(error) => {
                write!(f, "cannot read the working directory: {error}")
            }
            Self::NoProjectManifest(dir) => write!(
                f,
                "no Project manifest ({MANIFEST_PATH}) in {} or any parent directory",
                dir.display()
            ),
            Self::Manifest(error) => error.fmt(f),
            Self::OutputTree(error) => error.fmt(f),
            Self::Host(error) => error.fmt(f),
            Self::UnknownOverride(name) => write!(
                f,
                "unknown override `{name}`; overrides name a Tool, a target Toolchain tool or a host tool like HOSTCC"
            ),
            Self::NotImplemented { invocation, issue } => {
                write!(
                    f,
                    "`{invocation}` is not implemented yet (tracked in #{issue})"
                )
            }
        }
    }
}

fn run(cli: Cli, overrides: &BTreeMap<String, String>) -> Result<(), Error> {
    let invocation = Invocation::from_cli(cli.subcommand)?;
    if invocation.needs_output_tree() && cli.output.is_none() {
        return Err(Error::MissingOutputTree(invocation.name()));
    }
    if invocation.needs_source_tree() {
        let cwd = std::env::current_dir().map_err(Error::WorkingDirectory)?;
        let source_tree =
            SourceTree::discover(&cwd).ok_or(Error::NoProjectManifest(cwd.clone()))?;
        let manifest = Manifest::load(&source_tree.manifest_path()).map_err(Error::Manifest)?;
        check_overrides(&manifest, overrides)?;
        if let (true, Some(dir)) = (invocation.needs_output_tree(), &cli.output) {
            let output_tree =
                OutputTree::new(dir, &cwd, &source_tree).map_err(Error::OutputTree)?;
            let host = host_toolchain::resolve(&manifest, overrides, &source_tree.root, &cwd)
                .map_err(Error::Host)?;
            host_toolchain::record_or_check(&output_tree, &host).map_err(Error::Host)?;
        }
    }
    Err(Error::NotImplemented {
        issue: invocation.tracking_issue(),
        invocation: invocation.name(),
    })
}

/// Command-line overrides must name a known tool; each one is allowed but flagged, because the
/// build no longer matches what the Project manifest and Defconfig describe (ADR-0012).
fn check_overrides(manifest: &Manifest, overrides: &BTreeMap<String, String>) -> Result<(), Error> {
    let host = host_toolchain::variables(manifest);
    for (name, value) in overrides {
        if !host.contains(name) && !manifest.target_tool_names().any(|tool| tool == name) {
            return Err(Error::UnknownOverride(name.clone()));
        }
        eprintln!(
            "makit: warning: {name}={value} overrides the Project manifest; \
             this build diverges from its Defconfig"
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    let (args, overrides) = cli::split_overrides(std::env::args_os());
    match run(Cli::parse_from(args), &overrides) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("makit: error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
