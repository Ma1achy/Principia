//! The benchmark runner's process, which `cargo xtask bench` runs: xtask reaches `validation` only as a dev-dependency
//! (systems_architecture §7.1; R-176, R-187), so it runs this binary rather than linking the benches.
//!
//! `bench --root <workspace> <bench> --out <path>`: runs the bench and writes its profiler schema v1 trace to <path>.
//! `bench --list`: names the registered benches, one per line.

use std::fs::File;
use std::path::Path;
use std::process::ExitCode;

fn run(root: &Path, name: &str, out: &Path) -> Result<(), String> {
    let names: Vec<&str> = validation::bench::BENCHES.iter().map(|b| b.name).collect();
    let bench = validation::bench::find(name).ok_or_else(|| {
        format!(
            "{name:?} is not a registered bench; registered: {}",
            names.join(", ")
        )
    })?;
    let trace = (bench.run)(root)?;
    let file = File::create(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    engine::contract::profile::write(&trace, file)
        .map_err(|e| format!("cannot write {}: {e}", out.display()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["--list"] => {
            for bench in validation::bench::BENCHES {
                println!("{}", bench.name);
            }
            Ok(())
        }
        ["--root", root, name, "--out", out] => run(Path::new(root), name, Path::new(out)),
        _ => Err(format!(
            "unrecognised arguments `{}`; usage: bench (--root <workspace> <bench> --out <path> | --list)",
            args.join(" ")
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("bench: {message}");
            ExitCode::FAILURE
        }
    }
}
