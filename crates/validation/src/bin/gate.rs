//! The numerical-gate runner's process, which `cargo xtask gate` runs: xtask reaches `validation` only as a
//! dev-dependency (systems_architecture §7.1; R-176, R-187), so it runs this binary rather than linking the runner.
//!
//! `gate --root <workspace> (<gate> | --all | --list)`: runs one gate, every registered gate, or lists them.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["--root", root, "--list"] => validation::gate::list(Path::new(root)),
        ["--root", root, "--all"] => {
            let root = Path::new(root);
            validation::gate::run_all(root, &validation::gate::out_dir(root))
        }
        ["--root", root, name] if !name.starts_with('-') => {
            let root = Path::new(root);
            validation::gate::run(root, name, &validation::gate::out_dir(root))
        }
        _ => Err(format!(
            "unrecognised arguments `{}`; usage: gate --root <workspace> (<gate> | --all | --list)",
            args.join(" ")
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("gate: {message}");
            ExitCode::FAILURE
        }
    }
}
