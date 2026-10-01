//! `prin profile` (render_gui_spec § "Profiler", "For agents, the same data structured"; telemetry §5):
//!
//! - `prin profile --scenario NAME --frames N --json PATH` runs a registered, fixed, deterministic scenario headless
//!   and writes one profiler schema v1 file, JSON Lines (R-286) — [`run`];
//! - `prin profile diff BASE NEW --threshold P%` exits non-zero when NEW regresses on BASE by more than P% in any
//!   scope's p95 (REQ-TOOL-119) — [`diff`];
//! - `prin profile show PATH [--pretty]` prints a trace, each line indented with `--pretty` (R-286) — [`show`].
//!
//! Exit status: 0 on success (and on a diff with no regression), 1 on a regression, 2 on a usage or read error, or a
//! diff whose BASE or NEW has no frame records (R-323; BASE's case applied per R-204).

mod diff;
mod run;
mod show;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Subcommand};

/// `prin profile`'s arguments: a run's flags, or a subcommand.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub struct ProfileArgs {
    #[command(subcommand)]
    command: Option<ProfileCommand>,
    /// The registered scenario to run: synthetic_frames.
    #[arg(long, required = true, value_name = "NAME")]
    scenario: Option<String>,
    /// How many frames to run.
    #[arg(long, required = true, value_name = "N")]
    frames: Option<u64>,
    /// Where to write the trace: profiler schema v1, JSON Lines.
    #[arg(long, required = true, value_name = "PATH")]
    json: Option<PathBuf>,
}

#[derive(Subcommand)]
enum ProfileCommand {
    /// Exit non-zero when NEW regresses on BASE by more than the threshold in any scope's p95.
    Diff {
        /// The baseline trace.
        base: PathBuf,
        /// The trace compared against it.
        new: PathBuf,
        /// The largest rise in a scope's p95 that is not a regression, in percent: `5%`, `5` or `7.5%`.
        #[arg(long, value_name = "P%", value_parser = diff::parse_threshold)]
        threshold: diff::Threshold,
    },
    /// Print a trace: its lines unchanged, or each indented for reading with --pretty.
    Show {
        /// The trace.
        path: PathBuf,
        /// Indent each line for reading.
        #[arg(long)]
        pretty: bool,
    },
}

/// Runs `prin profile`.
pub fn main(args: ProfileArgs) -> ExitCode {
    let outcome = match args.command {
        Some(ProfileCommand::Diff {
            base,
            new,
            threshold,
        }) => diff::main(&base, &new, &threshold),
        Some(ProfileCommand::Show { path, pretty }) => show::main(&path, pretty),
        None => match (args.scenario, args.frames, args.json) {
            (Some(scenario), Some(frames), Some(json)) => run::main(&scenario, frames, &json),
            // clap requires all three when there is no subcommand.
            _ => Err("prin profile: --scenario, --frames and --json are required".to_owned()),
        },
    };
    match outcome {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
