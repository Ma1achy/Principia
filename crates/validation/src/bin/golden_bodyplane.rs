//! The body plane's numeric golden, which `cargo xtask golden bodyplane` runs (TASK-M3-36): xtask reaches `validation`
//! only as a dev-dependency (systems_architecture §7.1), so it runs this binary rather than linking the comparison.
//!
//! `golden_bodyplane --root <workspace> [--propose | --record]`.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["--root", root, rest @ ..] => validation::bodyplane::cli(Path::new(root), rest),
        _ => Err(format!(
            "unrecognised arguments `{}`; usage: golden_bodyplane --root <workspace> [--propose | --record]",
            args.join(" ")
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("golden bodyplane: {message}");
            ExitCode::FAILURE
        }
    }
}
