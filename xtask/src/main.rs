use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::controls::Mode;
use xtask::deps::{self, CompileCheck, Metadata};
use xtask::workspace_manifest;

const USAGE: &str = "\
Usage: cargo xtask <command>

Commands:
  ci                              run every registered per-push runner, in order (R-177)
  controls [--list] [--manifest-path <Cargo.toml>]
                                  check that every test of each crate declaring the `controls` feature has a
                                  negative control that makes it fail (REQ-VAL-147, R-199, R-201); on this
                                  workspace or on <Cargo.toml>'s; a crate without the feature is skipped (R-176);
                                  --list lists each test's controls and checks the listing, running none (R-226)
  deps [--metadata <file> | --manifest-path <Cargo.toml>]
                                  check the workspace crate graph against systems_architecture §7.1, and
                                  that no unit test of kernel or ledger uses validation, by compiling them
                                  without it (R-187, R-191); reads `cargo metadata --format-version 1` on
                                  this workspace or on <Cargo.toml>'s, or reads <file>, a metadata fixture
                                  (the compile check is then skipped)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["ci"] => xtask::ci::run(xtask::ci::RUNNERS),
        ["controls"] => xtask::controls::run(&workspace_manifest(), Mode::Run),
        ["controls", "--list"] => xtask::controls::run(&workspace_manifest(), Mode::List),
        ["controls", "--manifest-path", path] => xtask::controls::run(Path::new(path), Mode::Run),
        ["controls", "--list", "--manifest-path", path] => {
            xtask::controls::run(Path::new(path), Mode::List)
        }
        ["deps"] => run_deps(Source::Workspace(None)),
        ["deps", "--manifest-path", path] => run_deps(Source::Workspace(Some(Path::new(path)))),
        ["deps", "--metadata", path] => run_deps(Source::Fixture(PathBuf::from(path))),
        ["--help"] | ["-h"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!(
                "xtask: unrecognised arguments: {}\n\n{USAGE}",
                args.join(" ")
            );
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

enum Source<'a> {
    /// A workspace on disk: this one, or the one of the given `Cargo.toml`.
    Workspace(Option<&'a Path>),
    /// A metadata fixture, with no sources behind it.
    Fixture(PathBuf),
}

fn run_deps(source: Source<'_>) -> Result<(), String> {
    let (metadata, fixture) = match &source {
        Source::Workspace(None) => (Metadata::from_cargo()?, None),
        Source::Workspace(Some(manifest)) => (Metadata::from_cargo_at(manifest)?, None),
        Source::Fixture(path) => (Metadata::from_file(path)?, Some(path)),
    };
    let edges = metadata.edges()?;
    let violations = deps::check(&edges);
    for violation in &violations {
        eprintln!("xtask deps: {violation}");
    }
    // A fixture may describe a graph with no sources behind it; the workspace must have them.
    metadata.check_targets(fixture.is_none())?;
    let compiled = match fixture {
        Some(path) => {
            println!(
                "xtask deps: compile check skipped: {} is a metadata fixture, with no sources to compile \
                 (R-187, R-191)",
                path.display()
            );
            Ok(())
        }
        None => match deps::compile_check(&metadata) {
            Ok(CompileCheck::NotNeeded) => {
                println!(
                    "xtask deps: compile check not needed: neither kernel nor ledger takes validation as a \
                     dev-dependency (R-187, R-191)"
                );
                Ok(())
            }
            Ok(CompileCheck::Passed(crates)) => {
                let verb = if crates.len() == 1 {
                    "compiles"
                } else {
                    "compile"
                };
                println!(
                    "xtask deps: compile check passed: {} {verb} without the validation dev-dependency, so no \
                     unit test uses it (R-187, R-191)",
                    crates.join(" and ")
                );
                Ok(())
            }
            Err(message) => Err(message),
        },
    };
    if violations.is_empty() {
        compiled?;
        println!(
            "xtask deps: {} workspace edge(s), all in the allowed-edge table (systems_architecture §7.1)",
            edges.len()
        );
        return Ok(());
    }
    if let Err(message) = compiled {
        eprintln!("xtask deps: {message}");
    }
    Err(format!(
        "{} forbidden workspace edge(s) (REQ-SYS-004)",
        violations.len()
    ))
}
