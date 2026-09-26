//! `prin`, the command-line interface (systems_architecture §7.1: depends on `engine`).
//!
//! Stub (TASK-M0-01): `prin --help` prints usage; there are no commands yet.

use std::process::ExitCode;

const USAGE: &str = "\
prin — the Principia command-line interface

Usage: prin [--help]

Options:
  -h, --help  Print this help

No commands yet.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("prin: unrecognised arguments: {}\n\n{USAGE}", args.join(" "));
            ExitCode::from(2)
        }
    }
}
