//! `prin`, the command-line interface (systems_architecture §7.1: depends on `engine`).
//!
//! `prin profile` runs a registered scenario headless and writes profiler schema v1, `prin profile diff` compares two
//! traces, and `prin profile show` prints one (render_gui_spec § "Profiler", "For agents, the same data structured";
//! TASK-M0-18).

mod profile;

use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

/// prin — the Principia command-line interface
#[derive(Parser)]
#[command(name = "prin")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run a fixed scenario headless and write its profiler trace; diff or show traces.
    Profile(profile::ProfileArgs),
}

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            // No command: the usage, as `--help` prints it.
            println!("{}", Cli::command().render_help());
            ExitCode::SUCCESS
        }
        Some(Command::Profile(args)) => profile::main(args),
    }
}
