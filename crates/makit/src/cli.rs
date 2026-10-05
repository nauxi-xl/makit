//! Command-line surface of `makit`.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Configuration-driven build system for C/C++ and Rust, adapted from Linux Kbuild/Kconfig.
#[derive(Debug, Parser)]
#[command(
    name = "makit",
    version,
    after_help = "Seed a Configuration from a Defconfig with `makit -O <DIR> <name>_defconfig`, \
                  which reads configs/<name>_defconfig."
)]
pub struct Cli {
    /// Output tree receiving every artifact of this build (Kbuild's `O=`)
    #[arg(short = 'O', long = "output", value_name = "DIR", global = true)]
    pub output: Option<PathBuf>,

    /// Print full command lines instead of the short `CC foo.o` form
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Build the Goals selected by the Configuration (the default)
    Build,
    /// Update .config after Kconfig changes, taking defaults for new Symbols
    Olddefconfig,
    /// Write a minimal Defconfig from the current Configuration
    Savedefconfig,
    /// Explain why a Goal was (re)built
    Explain {
        /// Path of the Goal inside the Output tree
        goal: PathBuf,
    },
    /// Build and run every Test Goal
    Test,
    /// Download remote inputs; the only phase allowed to use the network
    Fetch,
    /// Run a Command declared in .makit/commands.mk
    Run {
        /// Command name, without the `run-` prefix
        command: String,
    },
    /// Use a declared Tool directly
    Tool {
        /// Tool name as declared in .makit/config.toml (e.g. PANDOC, CC)
        name: String,
        #[command(subcommand)]
        action: ToolAction,
    },
    /// Manage the machine-wide Cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Build twice in different Output trees and compare the outputs
    ReproCheck,
    /// Remove build outputs, keeping .config
    Clean,
    /// Remove build outputs and .config
    Mrproper,
    /// `<name>_defconfig`: create .config from the Defconfig configs/<name>_defconfig
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Subcommand)]
pub enum ToolAction {
    /// Run the Tool with the given arguments
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CacheAction {
    /// Evict least-recently-used entries down to the configured size limit
    Gc,
}
