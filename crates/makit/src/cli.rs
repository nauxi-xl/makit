//! Command-line surface of `makit`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::manifest::is_tool_name;

/// Configuration-driven build system for C/C++ and Rust, adapted from Linux Kbuild/Kconfig.
#[derive(Debug, Parser)]
#[command(
    name = "makit",
    version,
    after_help = "Seed a Configuration from a Defconfig with `makit -O <DIR> <name>_defconfig`, \
                  which reads configs/<name>_defconfig."
)]
pub struct Cli {
    /// Output tree receiving everything this build produces (Kbuild's `O=`)
    #[arg(short = 'O', long = "output", value_name = "DIR", global = true)]
    pub output: Option<PathBuf>,

    #[command(subcommand)]
    pub subcommand: Option<CliSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum CliSubcommand {
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
    /// Fetch remote inputs (crates, tarballs, git sources); the only phase allowed to use the network
    Fetch,
    /// Run a Command declared in .makit/commands.mk
    Run {
        /// Command name, without the `run-` prefix
        command: String,
    },
    /// Run a declared Tool directly; everything after `run` is passed to it untouched
    #[command(override_usage = "makit tool <NAME> run [ARGS]...")]
    Tool {
        /// A Tool from [tools] in .makit/config.toml, or a Toolchain tool such as CC
        name: String,
        // An external subcommand keeps clap from parsing makit's own options (-O, --help) out of
        // the Tool's arguments.
        #[command(subcommand)]
        subcommand: ToolSubcommand,
    },
    /// Manage the machine-wide Cache
    Cache {
        #[command(subcommand)]
        subcommand: CacheSubcommand,
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
pub enum ToolSubcommand {
    /// `run [ARGS]...`
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Debug, Subcommand)]
pub enum CacheSubcommand {
    /// Evict least-recently-used entries down to the configured size limit
    Gc,
}

/// Splits Kbuild-style `NAME=value` overrides (`makit -O out HOSTCC=clang`) out of the arguments.
///
/// The value of `-O`/`--output` is never an override, and scanning stops once the subcommand is
/// `tool`: everything after `makit tool <NAME> run` belongs to the Tool.
pub fn split_overrides(
    args: impl IntoIterator<Item = OsString>,
) -> (Vec<OsString>, BTreeMap<String, String>) {
    let mut rest = Vec::new();
    let mut overrides = BTreeMap::new();
    let mut args = args.into_iter();
    // The program name.
    rest.extend(args.next());
    let mut subcommand_seen = false;
    while let Some(arg) = args.next() {
        let text = arg.to_str().unwrap_or_default();
        if text == "-O" || text == "--output" {
            rest.push(arg);
            rest.extend(args.next());
            continue;
        }
        if let Some((name, value)) = text.split_once('=').filter(|(name, _)| is_tool_name(name)) {
            overrides.insert(name.to_owned(), value.to_owned());
            continue;
        }
        if !subcommand_seen && !text.starts_with('-') {
            subcommand_seen = true;
            if text == "tool" {
                rest.push(arg);
                rest.extend(args);
                break;
            }
        }
        rest.push(arg);
    }
    (rest, overrides)
}
