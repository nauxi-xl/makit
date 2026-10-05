mod cli;

use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use cli::{CacheAction, Cli, CliCommand, ToolAction};

/// What the user asked Makit to do, after resolving CLI syntax.
#[derive(Debug)]
enum Command {
    Build,
    /// Seed a Configuration from `configs/<name>_defconfig`.
    Defconfig(String),
    Olddefconfig,
    Savedefconfig,
    Explain(PathBuf),
    Test,
    Fetch,
    Run(String),
    /// Arguments are forwarded to the Tool once #17 lands.
    Tool(String),
    CacheGc,
    ReproCheck,
    Clean,
    Mrproper,
}

impl Command {
    fn from_cli(command: Option<CliCommand>) -> Result<Self, Error> {
        Ok(match command {
            None | Some(CliCommand::Build) => Self::Build,
            Some(CliCommand::Olddefconfig) => Self::Olddefconfig,
            Some(CliCommand::Savedefconfig) => Self::Savedefconfig,
            Some(CliCommand::Explain { goal }) => Self::Explain(goal),
            Some(CliCommand::Test) => Self::Test,
            Some(CliCommand::Fetch) => Self::Fetch,
            Some(CliCommand::Run { command }) => Self::Run(command),
            Some(CliCommand::Tool {
                name,
                action: ToolAction::Run { .. },
            }) => Self::Tool(name),
            Some(CliCommand::Cache {
                action: CacheAction::Gc,
            }) => Self::CacheGc,
            Some(CliCommand::ReproCheck) => Self::ReproCheck,
            Some(CliCommand::Clean) => Self::Clean,
            Some(CliCommand::Mrproper) => Self::Mrproper,
            Some(CliCommand::External(args)) => {
                let name = args[0].to_string_lossy().into_owned();
                match name.strip_suffix("_defconfig") {
                    Some(defconfig) if args.len() == 1 && !defconfig.is_empty() => {
                        Self::Defconfig(defconfig.to_owned())
                    }
                    _ => return Err(Error::UnknownCommand(name)),
                }
            }
        })
    }

    /// The command as typed on the command line, without options.
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

    /// GitHub issue tracking the implementation of this command.
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

    /// Whether the command reads or writes an Output tree, and so needs `-O`.
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
            | Self::Clean
            | Self::Mrproper => true,
            // Toolchain Tools will need -O to know the selected Toolchain; that is checked once the
            // Project manifest is loaded.
            Self::Tool(_) | Self::Fetch | Self::CacheGc | Self::ReproCheck => false,
        }
    }
}

#[derive(Debug)]
enum Error {
    /// Exit code 2: the invocation itself is wrong.
    UnknownCommand(String),
    MissingOutputTree(String),
    /// Exit code 1: a valid command whose implementation has not landed yet.
    NotImplemented {
        command: String,
        issue: u32,
    },
}

impl Error {
    fn exit_code(&self) -> u8 {
        match self {
            Self::UnknownCommand(_) | Self::MissingOutputTree(_) => 2,
            Self::NotImplemented { .. } => 1,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(name) => {
                write!(
                    f,
                    "unknown command `{name}` (Defconfigs are named `<name>_defconfig`)"
                )
            }
            Self::MissingOutputTree(name) => {
                write!(f, "`{name}` needs an Output tree: pass -O <DIR>")
            }
            Self::NotImplemented { command, issue } => {
                write!(
                    f,
                    "`{command}` is not implemented yet (tracked in #{issue})"
                )
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let command = Command::from_cli(cli.command)?;
    if command.needs_output_tree() && cli.output.is_none() {
        return Err(Error::MissingOutputTree(command.name()));
    }
    Err(Error::NotImplemented {
        issue: command.tracking_issue(),
        command: command.name(),
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
