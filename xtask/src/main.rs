use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::controls::Mode;
use xtask::deps::{self, CompileCheck, Metadata};
use xtask::workspace_manifest;

const USAGE: &str = "\
Usage: cargo xtask <command>

Commands:
  ci [--list]                     run every registered per-push runner, in order (R-177); --list runs each
                                  runner's listing-only form, which runs no control (R-235)
  codegen                         regenerate the checked-in generated files from the layout table; refuses when
                                  an entry lacks a §3.8 key, naming the field and the key (dd_generation_root §3.8)
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
                                  (the compile check is then skipped)
  gate (<gate> | --all | --list)  run the numerical gate <gate>, or every registered gate, on its inputs in
                                  fixtures/gates/<gate>/, against the threshold its gate.json names by requirement
                                  id, writing each report under target/gates/; fails naming each input whose outcome
                                  is not its expected one (TASK-M0-05); --list lists the gates and runs none
  lint constants                  fail on a numeric const or static in crates/{kernel,ledger,engine} not read
                                  from the constants register, naming file and line (dd_generation_root §3.8)
  plan-check                      run plan/check_plan.py from the repo root (it also runs coverage.py,
                                  milestones.py and reviewer_lists.py with --check), streaming its output and
                                  exiting with its status; needs python3 and PyYAML (REQ-SYS-007, REQ-SYS-008)
  pr-check [--event <file>]       fail naming each section the PR's labels (design, investigation, validation)
                                  make mandatory that is missing or empty, and each validation meter or
                                  discriminator line with no statement (R-180); reads the pull_request event JSON
                                  at <file>, or at $GITHUB_EVENT_PATH
  reviews-check [--pr <N>]        the reviews-complete check (R-175): fail naming each role the task file's
                                  Reviewers field names that has not approved on the head commit; reads PR <N>, or
                                  the PR of the event at $GITHUB_EVENT_PATH, through `gh api`
  screenshot (<suite> | --all)    run the GUI screenshot suite fixtures/screenshot/<suite>/, or every suite: a layout
                                  case renders its surface headless (native wgpu offscreen) and writes the capture
                                  beside a copy of its artboard under target/screenshot/, for layout comparison only
                                  (R-68); a presence-only case fails naming each listed control its surface lacks
                                  (R-129). Not in `ci`: GUI PRs and the gates run it (R-110, R-177)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["ci"] => xtask::ci::run(xtask::ci::RUNNERS),
        ["ci", "--list"] => xtask::ci::list(xtask::ci::RUNNERS),
        ["codegen"] => xtask::codegen::run(&workspace_manifest()),
        ["controls"] => xtask::controls::run(&workspace_manifest(), Mode::Run),
        ["controls", "--list"] => xtask::controls::run(&workspace_manifest(), Mode::List),
        ["controls", "--manifest-path", path] => xtask::controls::run(Path::new(path), Mode::Run),
        ["controls", "--list", "--manifest-path", path] => {
            xtask::controls::run(Path::new(path), Mode::List)
        }
        ["gate", "--all"] => xtask::gate::run(&workspace_manifest(), xtask::gate::Which::All),
        ["gate", "--list"] => xtask::gate::run(&workspace_manifest(), xtask::gate::Which::List),
        ["gate", name] if !name.starts_with('-') => {
            xtask::gate::run(&workspace_manifest(), xtask::gate::Which::One(name))
        }
        ["plan-check"] => {
            return match xtask::plan_check::status(&xtask::plan_check::repo_root()) {
                Ok(status) => {
                    ExitCode::from(status.code().map_or(1, |code| code.clamp(0, 255) as u8))
                }
                Err(message) => {
                    eprintln!("xtask plan-check: {message}");
                    ExitCode::FAILURE
                }
            };
        }
        ["lint", "constants"] => xtask::lint_constants::run(&workspace_manifest()),
        ["pr-check"] => match std::env::var("GITHUB_EVENT_PATH") {
            Ok(path) => xtask::pr_check::run(Path::new(&path)),
            Err(_) => {
                Err("pr-check: no --event <file>, and $GITHUB_EVENT_PATH is not set".to_owned())
            }
        },
        ["pr-check", "--event", path] => xtask::pr_check::run(Path::new(path)),
        ["reviews-check"] => xtask::reviews_check::run(&workspace_root(), None),
        ["reviews-check", "--pr", n] => match n.parse() {
            Ok(n) => xtask::reviews_check::run(&workspace_root(), Some(n)),
            Err(_) => Err(format!("reviews-check: --pr takes a PR number, not `{n}`")),
        },
        ["screenshot", "--all"] => {
            xtask::screenshot::run(&workspace_root(), xtask::screenshot::Which::All)
        }
        ["screenshot", suite] if !suite.starts_with('-') => {
            xtask::screenshot::run(&workspace_root(), xtask::screenshot::Which::One(suite))
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

/// This workspace's root directory.
fn workspace_root() -> PathBuf {
    workspace_manifest()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
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
