use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::controls::{Mode, Partition};
use xtask::deps::{self, CompileCheck, Metadata};
use xtask::workspace_manifest;

const USAGE: &str = "\
Usage: cargo xtask <command>

Commands:
  bench (<bench> | --all) [--bless]
                                  run a fixed benchmark headless on the GPU (or every registered one), write its
                                  profiler schema v1 trace to target/bench/<bench>.jsonl and compare it with
                                  fixtures/bench/<bench>/baseline.json through `prin profile diff`, every rise in a
                                  scope's p95 listed (telemetry §1.1); --bless writes the trace as the baseline first.
                                  Not in `ci` nor any hosted workflow: run on the human's Mac (R-186)
  build-kernel                    compile crates/kernel to SPIR-V with rust-gpu (target/spirv/kernel.spv) and
                                  translate it to WGSL with naga (target/spirv/kernel.wgsl); refuses when
                                  rust-gpu's backend needs another nightly than rust-toolchain.toml pins
                                  (canonical_spec §1 item 2)
  ci [--list | --partition <k>/<n>]
                                  run every registered per-push runner, in order (R-177); --list runs each
                                  runner's listing-only form, which runs no control (R-235); --partition runs
                                  shard k of n: controls on its slice, build-kernel, and the other runners in
                                  shard 1 only (R-360)
  codegen                         regenerate the checked-in generated files from the layout table; refuses when
                                  an entry lacks a §3.8 key, naming the field and the key (dd_generation_root §3.8)
  controls [--list] [--partition <k>/<n>] [--manifest-path <Cargo.toml>]
                                  check that every test of each crate declaring the `controls` feature has a
                                  negative control that makes it fail (REQ-VAL-147, R-199, R-201); on this
                                  workspace or on <Cargo.toml>'s; a crate without the feature is skipped (R-176);
                                  --list lists each test's controls and checks the listing, running none (R-226);
                                  --partition runs, or lists, only the k-th of n slices of the controls, by a
                                  stable hash of the control name (R-360)
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
  gate-report --milestone <Mn> --results <file> [--bench-results <dir>] [--test-list <file>]
                                  list every requirement of <Mn>'s gate block and every earlier one
                                  (plan/MILESTONES.md) with its result from <file> (a JSON object, id to `pass` or
                                  `fail`), a benchmark requirement awaiting the human's run until <dir>/<id>.jsonl,
                                  its prin profile file, is supplied (R-177, R-186), and a review-checklist one
                                  passing when its closing task's PR merged with its reviewers' approvals, read
                                  through gh (R-369, RQ-201); a unit-test, property-test, numerical-gate, golden or
                                  screenshot one fails when a `cargo test -p <crate> [flags] <filter>` its detail
                                  names matches no test of that build in <file>, the run's listing (REQ-SYS-079);
                                  writes target/gate-report/<Mn>.txt; fails on a requirement failed or with no result
  gate-report --milestone <Mn> --write-test-list <file>
                                  run `cargo test <build> -- --list` for every build the hosted requirements of <Mn>'s
                                  gate and every earlier one name a test of, and write the listing to <file>, for
                                  gate-report's --test-list (REQ-SYS-079)
  golden (<suite> | --all | --list)
                                  render each case of fixtures/golden/<suite>/ (or of every suite) with native wgpu
                                  offscreen, compare it with its reference to the tolerance its requirement id
                                  gives, and write the difference image and summary under target/golden/ (R-110);
                                  --list loads and checks every case and lists it, opening no device
  golden repro <suite>/<case> --vary <field>=<a>,<b>
                                  render two arms differing in exactly one field (a pair differing in more is
                                  refused), and report the RGB values along the case's lines and one column per
                                  symptom per arm (philosophy §4.3a; pitfalls §8)
  lint constants                  fail on a numeric const or static in crates/{kernel,ledger,engine} not read
                                  from the constants register, naming file and line (dd_generation_root §3.8)
  lint vocab                      fail on a retired term (canonical_spec §8) or an identifier outside the locked
                                  taxonomy (memory_tiers §1) in crates/, xtask/, fixtures/, web/ or docs/ (.md,
                                  .html; not archive/ or reference/, nor passages in `retired-terms` markers),
                                  naming file, line and term (R-111, R-259)
  lint wgsl                       parse the generated WGSL with naga and fail, naming the rule, on an extractBits
                                  argument not u32, any f64, `enable f16`, r/p/r_sh/p_sh not array<vec2<f32>, 3>,
                                  a word buffer not bound on its own and indexed per copy (REQ-RENDER-001), a
                                  buffer off the ledger's binding table (simstate_buffer @group(1) @binding(0),
                                  word_buffer @group(1) @binding(1)), a generated SIMSTATE_/WORD_GROUP or _BINDING
                                  constant unequal to its attribute, a binding in group 0 (bindings, R-343), or
                                  either buffer used outside sample_state/sample_word (sample-only, R-343)
  lint compute-pipelines          fail on a compute pipeline created, or wgpu's passthrough used, outside the
                                  compute entry point (crates/engine/src/compute.rs), or a vertex or fragment
                                  pipeline beside its passthrough, naming file, line and identifier (R-297)
  mutants-check <mutants.out>... [--equivalent <file>]
                                  the per-PR mutation gate (R-196, R-202): list each mutant that survived the
                                  `cargo mutants` run whose output is <mutants.out>, or each shard's (R-302), and fail
                                  naming each one not in the equivalent-mutants list, .cargo/mutants-equivalent.toml
                                  or <file>
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
        ["bench", "--all", bless @ ..] if bless.is_empty() || bless == ["--bless"] => {
            xtask::bench::run(
                &workspace_manifest(),
                xtask::bench::Which::All,
                !bless.is_empty(),
            )
        }
        ["bench", name, bless @ ..]
            if !name.starts_with('-') && (bless.is_empty() || bless == ["--bless"]) =>
        {
            xtask::bench::run(
                &workspace_manifest(),
                xtask::bench::Which::One(name),
                !bless.is_empty(),
            )
        }
        ["gate-report", rest @ ..] => gate_report(rest),
        ["build-kernel"] => xtask::build_kernel::run(&workspace_manifest()),
        ["ci"] => xtask::ci::run(xtask::ci::RUNNERS),
        ["ci", "--list"] => xtask::ci::list(xtask::ci::RUNNERS),
        ["ci", "--partition", slice] => Partition::parse(slice)
            .and_then(|slice| xtask::ci::run_partition(xtask::ci::RUNNERS, slice)),
        ["codegen"] => xtask::codegen::run(&workspace_manifest()),
        ["controls"] => xtask::controls::run(&workspace_manifest(), Mode::Run),
        ["controls", "--list"] => xtask::controls::run(&workspace_manifest(), Mode::List),
        ["controls", "--manifest-path", path] => xtask::controls::run(Path::new(path), Mode::Run),
        ["controls", "--list", "--manifest-path", path] => {
            xtask::controls::run(Path::new(path), Mode::List)
        }
        ["controls", rest @ ..] if rest.contains(&"--partition") => controls_partition(rest),
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
        ["golden", rest @ ..] => xtask::golden::cli(&workspace_root(), rest),
        ["lint", "constants"] => xtask::lint_constants::run(&workspace_manifest()),
        ["lint", "vocab"] => xtask::lint_vocab::run(&workspace_manifest()),
        ["lint", "wgsl"] => xtask::lint_wgsl::run(&workspace_manifest()),
        ["lint", "compute-pipelines"] => xtask::lint_compute::run(&workspace_manifest()),
        ["mutants-check", outs @ .., "--equivalent", list]
            if !outs.is_empty() && !outs.contains(&"--equivalent") =>
        {
            let outs: Vec<&Path> = outs.iter().map(Path::new).collect();
            xtask::mutants_check::run_shards(&outs, Path::new(list))
        }
        ["mutants-check", outs @ ..] if !outs.is_empty() && !outs.contains(&"--equivalent") => {
            let outs: Vec<&Path> = outs.iter().map(Path::new).collect();
            xtask::mutants_check::run_shards(
                &outs,
                &workspace_root().join(xtask::mutants_check::EQUIVALENT_LIST),
            )
        }
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

/// `cargo xtask controls` with `--partition <k>/<n>` among `args`, and optionally `--list` and `--manifest-path
/// <Cargo.toml>`, in any order (R-360).
fn controls_partition(args: &[&str]) -> Result<(), String> {
    let (mut mode, mut partition, mut manifest) = (Mode::Run, None, workspace_manifest());
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match *arg {
            "--list" => mode = Mode::List,
            "--partition" => {
                let slice = args.next().ok_or("controls: --partition takes k/n")?;
                partition = Some(Partition::parse(slice)?);
            }
            "--manifest-path" => {
                let path = args
                    .next()
                    .ok_or("controls: --manifest-path takes a path")?;
                manifest = PathBuf::from(path);
            }
            other => return Err(format!("controls: unrecognised argument `{other}`")),
        }
    }
    let partition = partition.ok_or("controls: --partition takes k/n")?;
    xtask::controls::run_partition(&manifest, mode, partition)
}

/// `cargo xtask gate-report` with `args`: `--milestone <Mn>` and `--write-test-list <file>`, or `--milestone <Mn>`,
/// `--results <file>` and optionally `--bench-results <dir>` and `--test-list <file>`, in any order (REQ-SYS-079).
fn gate_report(args: &[&str]) -> Result<(), String> {
    let mut given: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    let mut args = args.iter();
    while let Some(&flag) = args.next() {
        match flag {
            "--milestone" | "--results" | "--bench-results" | "--test-list"
            | "--write-test-list" => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("gate-report: {flag} takes a value"))?;
                if given.insert(flag, value).is_some() {
                    return Err(format!("gate-report: {flag} is given twice"));
                }
            }
            other => return Err(format!("gate-report: unrecognised argument `{other}`")),
        }
    }
    let milestone = given
        .remove("--milestone")
        .ok_or("gate-report: --milestone <Mn> is required")?;
    if let Some(out) = given.remove("--write-test-list") {
        if let Some(flag) = given.keys().next() {
            return Err(format!("gate-report: --write-test-list takes no {flag}"));
        }
        return xtask::gate_report::write_test_list(
            &workspace_root(),
            milestone,
            &std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()),
            Path::new(out),
        );
    }
    let results = given
        .remove("--results")
        .ok_or("gate-report: --results <file> is required")?;
    xtask::gate_report::run_listed(
        &workspace_root(),
        milestone,
        Path::new(results),
        given.get("--bench-results").map(Path::new),
        given.get("--test-list").map(Path::new),
        &xtask::gate_report::Gh::new("gh"),
    )
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
