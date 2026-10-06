//! The `makit` binary: resolves the command line into an Invocation and dispatches it.

mod cli;

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use cli::{CacheSubcommand, Cli, CliSubcommand, ToolSubcommand};

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
}

#[derive(Debug)]
enum Error {
    UnknownSubcommand(String),
    MissingOutputTree(String),
    NotImplemented { invocation: String, issue: u32 },
}

impl Error {
    /// 2 when the command line itself is wrong, 1 when a valid invocation cannot run yet.
    fn exit_code(&self) -> u8 {
        match self {
            Self::UnknownSubcommand(_) | Self::MissingOutputTree(_) => 2,
            Self::NotImplemented { .. } => 1,
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
            Self::NotImplemented { invocation, issue } => {
                write!(
                    f,
                    "`{invocation}` is not implemented yet (tracked in #{issue})"
                )
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let invocation = Invocation::from_cli(cli.subcommand)?;
    if invocation.needs_output_tree() && cli.output.is_none() {
        return Err(Error::MissingOutputTree(invocation.name()));
    }
    Err(Error::NotImplemented {
        issue: invocation.tracking_issue(),
        invocation: invocation.name(),
    })
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("makit: error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
