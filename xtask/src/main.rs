use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: cargo xtask <command>

Commands:
  ci                         run every registered per-push runner, in order (R-177)
  deps [--metadata <file>]   check the workspace crate graph against systems_architecture §7.1
                             (reads `cargo metadata --format-version 1`, or <file> if given)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["ci"] => xtask::ci::run(xtask::ci::RUNNERS),
        ["deps"] => deps(None),
        ["deps", "--metadata", path] => deps(Some(PathBuf::from(path))),
        ["--help"] | ["-h"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("xtask: unrecognised arguments: {}\n\n{USAGE}", args.join(" "));
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn deps(fixture: Option<PathBuf>) -> Result<(), String> {
    let metadata = match fixture {
        Some(path) => xtask::deps::Metadata::from_file(&path)?,
        None => xtask::deps::Metadata::from_cargo()?,
    };
    let edges = metadata.edges()?;
    let violations = xtask::deps::check(&edges);
    if violations.is_empty() {
        println!(
            "xtask deps: {} workspace edge(s), all in the allowed-edge table (systems_architecture §7.1)",
            edges.len()
        );
        return Ok(());
    }
    for violation in &violations {
        eprintln!("xtask deps: {violation}");
    }
    Err(format!("{} forbidden workspace edge(s) (REQ-SYS-004)", violations.len()))
}
