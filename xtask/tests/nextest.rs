//! REQ-VAL-165 (R-231): CI's test steps run through `cargo nextest run`, pinned, with doctests, which nextest does not
//! run, through `cargo test --doc`, and no test is dropped: in each feature set, the nextest and `--doc` steps
//! together list every test that the `cargo test` steps they replaced list (`cargo test <args> -- --list`); a step
//! sharded by its job's matrix (`--partition hash:${{ matrix.shard }}/4`, R-336, R-366) is listed once per shard, and a
//! step that runs from a nextest archive (`--archive-file`, R-372, REQ-SYS-078) is listed with the arguments the
//! `cargo nextest archive` step that builds that archive was given. The documented local run (`plan/WORKFLOW.md`,
//! "Running the suite locally") installs the same pinned version. A `cargo test --no-run` builds and runs no test, so
//! it is no test step. The one test step outside nextest is `stand-in-soak.yml`'s 50-run step, which REQ-SYS-070 needs
//! ([`LIBTEST_SOAK`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;
use xtask::controls::IN_CI_WORKSPACE;

/// The arguments of the `cargo test` steps CI ran before nextest (TASK-M0-33's base): the whole workspace in the
/// default feature set, and the GPU jobs' suites in the `controls` one.
const REPLACED: &[&str] = &[
    "--workspace",
    "-p validation --features controls metal_hosted_probe",
    "-p validation --features controls gpu_harness",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The text of every workflow under `.github/workflows/`.
fn workflows() -> Vec<String> {
    let dir = root().join(".github/workflows");
    let mut texts: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect();
    texts.sort();
    texts
}

/// The text of `.github/workflows/ci.yml` alone: the no-test-dropped checks below hold the per-push CI's test steps
/// (REQ-VAL-165), so that the nightly and gate workflows' unsharded runs cannot cover a test the per-push CI drops.
/// The pinned-nextest check still reads every workflow (`workflows`).
fn ci_workflow() -> Vec<String> {
    vec![std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap()]
}

/// The text of `plan/WORKFLOW.md`, which documents the local run (the README did until the README of 18 Oct 2026).
fn documented_run_doc() -> String {
    std::fs::read_to_string(root().join("plan/WORKFLOW.md")).unwrap()
}

/// The jobs of `workflow`: each job's lines, from its name (two spaces in, under `jobs:`) to the next job's.
fn jobs(workflow: &str) -> Vec<Vec<&str>> {
    let mut jobs: Vec<Vec<&str>> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            in_jobs = line.trim_end() == "jobs:";
        } else if in_jobs && line.starts_with("  ") && !line[2..].starts_with([' ', '#']) {
            jobs.push(vec![line]);
        } else if let Some(job) = jobs.last_mut().filter(|_| in_jobs) {
            job.push(line);
        }
    }
    jobs
}

/// The shell commands of a `run:` step, from its value and, for a block scalar, the scalar's lines. A one-line value is
/// one command, and so is a folded scalar (`>`), whose lines the shell gets as one. A literal scalar (`|`) has a
/// command a line, without blank lines and comments, and a line ending in `\` continued on the next, with a space
/// between the two, the indented form every workflow writes.
fn commands(value: &str, block: &[&str]) -> Vec<String> {
    if value.starts_with('>') {
        return vec![block
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join(" ")];
    }
    if !value.starts_with('|') {
        return vec![value.to_owned()];
    }
    let mut out = Vec::new();
    let mut command = String::new();
    for line in block.iter().map(|line| line.trim()) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        command.push_str(line.trim_end_matches('\\').trim_end());
        if line.ends_with('\\') {
            command.push(' ');
        } else {
            out.push(std::mem::take(&mut command));
        }
    }
    out
}

/// The commands of a job's `run:` steps, one-line, folded and block alike ([`commands`]), each with the nextest profile
/// it runs under: the `NEXTEST_PROFILE` its step's `env:` sets, or else the job's, or none (the default profile). CI's
/// `ci`, `ci-workspace` and `gpu-kernel` jobs split the workspace's tests by profile (R-325, R-372,
/// `.config/nextest.toml`).
fn runs<'a>(job: &[&'a str]) -> Vec<(String, Option<&'a str>)> {
    let mut out = Vec::new();
    let mut in_steps = false;
    let (mut job_profile, mut step_profile) = (None, None);
    let mut lines = job.iter().peekable();
    while let Some(full_line) = lines.next() {
        let line = full_line.trim_start();
        if line.trim_end() == "steps:" {
            in_steps = true;
        }
        if line.starts_with("- ") {
            step_profile = None;
        }
        if let Some(profile) = line.strip_prefix("NEXTEST_PROFILE:") {
            let profile = Some(profile.trim().trim_matches('"'));
            if in_steps {
                step_profile = profile;
            } else {
                job_profile = profile;
            }
        }
        if let Some(run) = line
            .strip_prefix("- run: ")
            .or_else(|| line.strip_prefix("run: "))
        {
            let run = run.trim();
            // A block scalar's lines are those indented past its `run:` key, and blank ones.
            let key = full_line.len() - line.len() + if line.starts_with("- ") { 2 } else { 0 };
            let mut block = Vec::new();
            if run.starts_with(['|', '>']) {
                while let Some(next) = lines.next_if(|next| {
                    next.trim().is_empty() || next.len() - next.trim_start().len() > key
                }) {
                    block.push(*next);
                }
            }
            let profile = step_profile.or(job_profile);
            out.extend(commands(run, &block).into_iter().map(|c| (c, profile)));
        }
    }
    out
}

/// The matrix expression a sharded step names its shard by (R-336, REQ-SYS-077).
const SHARD: &str = "${{ matrix.shard }}";

/// `run` once for each shard of its job's matrix (`shard: [1, 2, …]`), [`SHARD`] replaced by the shard; or `run`
/// alone when it names no shard. So a sharded nextest step is listed under each shard's `--partition`.
fn shards(job: &[&str], run: &str) -> Vec<String> {
    if !run.contains(SHARD) {
        return vec![run.to_owned()];
    }
    let values = job
        .iter()
        .find_map(|line| line.trim().strip_prefix("shard: ["))
        .and_then(|rest| rest.trim_end().strip_suffix(']'))
        .unwrap_or_else(|| {
            panic!(
                "job {} names {SHARD} but has no `shard: [...]` matrix",
                job[0].trim()
            )
        });
    values
        .split(',')
        .map(|shard| run.replace(SHARD, shard.trim()))
        .collect()
}

/// A CI test step: `cargo nextest run <args>` (`doc` false) or `cargo test <args>` with `--doc` (`doc` true), its
/// arguments without `--no-capture` and `--doc`, and the nextest profile it runs under (`None` for the default). A
/// nextest step that runs from an archive has, in place of the archive's options, the arguments that built the archive.
struct Step {
    doc: bool,
    args: Vec<String>,
    profile: Option<String>,
}

/// The value of `--features` in `args`, or "" for the default feature set.
fn features(args: &[String]) -> &str {
    args.iter()
        .position(|a| a == "--features")
        .and_then(|i| args.get(i + 1))
        .map_or("", String::as_str)
}

/// The options of `cargo nextest archive` that name the archive, not what goes in it, each with its value.
const ARCHIVE_OPTIONS: &[&str] = &["--archive-file", "--archive-format", "--zstd-level"];

/// The options of `cargo nextest run` that reuse a build (`--archive-file` and those beside it), each followed by its
/// value; and its two reuse flags, which take none.
const REUSE_OPTIONS: &[&str] = &[
    "--archive-file",
    "--archive-format",
    "--extract-to",
    "--cargo-metadata",
    "--workspace-remap",
    "--binaries-metadata",
    "--target-dir-remap",
    "--build-dir-remap",
];
const REUSE_FLAGS: &[&str] = &["--extract-overwrite", "--persist-extract-tempdir"];

/// `words` without each option of `options` and its value, nor any flag of `flags`; and the value of `--archive-file`,
/// if it was there.
fn without(words: &[String], options: &[&str], flags: &[&str]) -> (Vec<String>, Option<String>) {
    let (mut kept, mut archive) = (Vec::new(), None);
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if options.contains(&word.as_str()) {
            let value = words.next().cloned();
            if word == "--archive-file" {
                archive = value;
            }
        } else if !flags.contains(&word.as_str()) {
            kept.push(word.clone());
        }
    }
    (kept, archive)
}

/// The archives the `cargo nextest archive` steps of `workflow` build: each archive file's path, and the arguments that
/// build it, without those naming the archive. A path is built by one step.
fn archives(workflow: &str) -> BTreeMap<String, Vec<String>> {
    let mut archives = BTreeMap::new();
    for job in jobs(workflow) {
        for (run, _) in runs(&job) {
            let Some(rest) = run.strip_prefix("cargo nextest archive") else {
                continue;
            };
            let words: Vec<String> = rest.split_whitespace().map(str::to_owned).collect();
            let (args, path) = without(&words, ARCHIVE_OPTIONS, &[]);
            let path = path.unwrap_or_else(|| {
                panic!(
                    "job {} runs `{run}`, naming no --archive-file",
                    job[0].trim()
                )
            });
            assert!(
                archives.insert(path.clone(), args).is_none(),
                "two steps build the nextest archive `{path}`"
            );
        }
    }
    archives
}

/// The one command that runs tests through `cargo test` and not nextest, doctests aside: the workflow's `name:`, its
/// job and the command. `stand-in-soak.yml`'s 50-run step soaks for the race REQ-SYS-070 names, which needs the test
/// binary's tests overlapping on its threads, and nextest gives each test a process of its own.
const LIBTEST_SOAK: (&str, &str, &str) = (
    "stand-in-soak",
    "soak:",
    r#"cargo test -p xtask --test ${{ matrix.test }} || { echo "run $run of 50 failed"; exit 1; }"#,
);

/// `run`, in `job` of `workflow`, is [`LIBTEST_SOAK`].
fn is_libtest_soak(workflow: &str, job: &[&str], run: &str) -> bool {
    let name = workflow
        .lines()
        .find_map(|line| line.strip_prefix("name: "));
    (name, job[0].trim(), run) == (Some(LIBTEST_SOAK.0), LIBTEST_SOAK.1, LIBTEST_SOAK.2)
}

/// `rest`, the arguments of a `cargo test`, ask cargo only to build: `--no-run` among cargo's own arguments, before any
/// `--`, past which it goes to the test binary and every test still runs.
fn only_builds(rest: &str) -> bool {
    rest.split_whitespace()
        .take_while(|word| *word != "--")
        .any(|word| word == "--no-run")
}

/// The test steps of `workflows`, and the pinned cargo-nextest version they install. Every job with a nextest step
/// installs cargo-nextest at an exact version, and every job that installs it installs the same one; no step runs
/// `cargo test` but for doctests, a build (`--no-run`) and [`LIBTEST_SOAK`]. A nextest step that runs from an archive
/// (`--archive-file`) has, in place of its reuse options, the arguments of the step in its workflow that builds that
/// archive.
fn test_steps(workflows: &[String]) -> (Vec<Step>, String) {
    let mut steps = Vec::new();
    let mut pins: Vec<String> = Vec::new();
    for workflow in workflows {
        let archives = archives(workflow);
        for job in jobs(workflow) {
            let mut nextest = false;
            for (run, profile) in runs(&job).into_iter().flat_map(|(run, profile)| {
                shards(&job, &run)
                    .into_iter()
                    .map(move |run| (run, profile))
            }) {
                let run = run.as_str();
                let words = |rest: &str| -> Vec<String> {
                    rest.split_whitespace()
                        .filter(|w| !matches!(*w, "--no-capture" | "--nocapture" | "--doc"))
                        .map(str::to_owned)
                        .collect()
                };
                if run.starts_with("cargo nextest archive") {
                    nextest = true;
                } else if let Some(rest) = run.strip_prefix("cargo nextest run") {
                    nextest = true;
                    let (mut args, archive) = without(&words(rest), REUSE_OPTIONS, REUSE_FLAGS);
                    if let Some(archive) = archive {
                        let built = archives.get(&archive).unwrap_or_else(|| {
                            panic!(
                                "job {} runs `{run}` from the archive `{archive}`, which no step of its workflow \
                                 builds",
                                job[0].trim()
                            )
                        });
                        args.splice(0..0, built.iter().cloned());
                    }
                    steps.push(Step {
                        doc: false,
                        args,
                        profile: profile.map(str::to_owned),
                    });
                } else if let Some(rest) = run.strip_prefix("cargo test") {
                    if only_builds(rest) || is_libtest_soak(workflow, &job, run) {
                        continue;
                    }
                    assert!(
                        rest.split_whitespace().any(|w| w == "--doc"),
                        "a CI step runs tests through `cargo test`, not nextest: `{run}` (job {})",
                        job[0].trim()
                    );
                    steps.push(Step {
                        doc: true,
                        args: words(rest),
                        profile: None,
                    });
                }
            }
            let pin = job.iter().find_map(|line| {
                // `tool:` is a comma-separated list, `cargo-mutants@…,cargo-nextest@…` in the mutants jobs.
                let tools = line.trim().strip_prefix("tool: ")?;
                let version = tools
                    .split(',')
                    .find_map(|tool| tool.trim().strip_prefix("cargo-nextest@"))?;
                let exact = version.split('.').count() == 3
                    && version.split('.').all(|n| n.parse::<u32>().is_ok());
                exact.then(|| version.to_owned())
            });
            assert!(
                !nextest || pin.is_some(),
                "job {} runs nextest without installing cargo-nextest at an exact version",
                job[0].trim()
            );
            pins.extend(pin);
        }
    }
    pins.sort();
    pins.dedup();
    assert_eq!(
        pins.len(),
        1,
        "CI installs cargo-nextest at more than one version, or none: {pins:?}"
    );
    (steps, pins.remove(0))
}

/// The documented local run in `doc` installs cargo-nextest at `pin` and runs the suite and the doctests.
fn check_documented_run(doc: &str, pin: &str) {
    for line in [
        format!("cargo install cargo-nextest --version {pin} --locked"),
        "cargo nextest run --workspace".to_owned(),
        "cargo test --workspace --doc".to_owned(),
    ] {
        assert!(
            doc.lines().any(|l| l.trim() == line),
            "the documented local run lacks `{line}`"
        );
    }
}

#[test]
fn nextest_ci_and_the_documented_run_install_the_pinned_nextest() {
    let (_, pin) = test_steps(&workflows());
    check_documented_run(&documented_run_doc(), &pin);
}

validation::negative_control!(
    nextest_ci_and_the_documented_run_install_the_pinned_nextest,
    "a documented run installing another cargo-nextest version than CI's, required to document the pinned one",
    expected = "the documented local run lacks `cargo install cargo-nextest --version",
    {
        let (_, pin) = test_steps(&workflows());
        check_documented_run(
            &documented_run_doc().replace(&format!("--version {pin} "), "--version 0.0.1 "),
            &pin,
        )
    }
);

/// A workflow named `name`, with one job `job`, which installs the pinned cargo-nextest and then has `steps`, YAML
/// list items at a step's indent.
fn workflow_with(name: &str, job: &str, steps: &str) -> String {
    format!(
        "name: {name}\njobs:\n  {job}:\n    steps:\n      - uses: taiki-e/install-action@v2\n        with:\n          \
         tool: cargo-nextest@0.9.146\n{steps}"
    )
}

/// The arguments of each test step of `workflow`, in the workflow's order.
fn step_args(workflow: &str) -> Vec<String> {
    let (steps, _) = test_steps(&[workflow.to_owned()]);
    steps.iter().map(|step| step.args.join(" ")).collect()
}

/// A literal block step: a comment, a line that is not cargo and a blank one are no commands, and the block ends at the
/// next step, which is read as well.
const BLOCK_STEPS: &str =
    "      - name: A block\n        run: |\n          # cargo test -p commented\n          \
    echo cargo test\n\n          cargo nextest run -p a\n      - run: cargo nextest run -p b\n";

#[test]
fn nextest_the_guard_reads_a_block_step() {
    assert_eq!(
        step_args(&workflow_with("w", "j", BLOCK_STEPS)),
        ["-p a", "-p b"]
    );
}

validation::negative_control!(
    nextest_the_guard_reads_a_block_step,
    "a `cargo test` in a block step of a workflow with no exception",
    expected = "a CI step runs tests through `cargo test`",
    drop(step_args(&workflow_with(
        "w",
        "j",
        &BLOCK_STEPS.replace("cargo nextest run -p a", "cargo test -p a")
    )))
);

/// A folded step, its command across two lines, as `run: >-` and `run: >` both write it.
const FOLDED_STEP: &str =
    "      - run: >-\n          cargo nextest run\n          -p a\n      - run: >\n          \
    cargo nextest run\n          -p b\n";

#[test]
fn nextest_the_guard_reads_a_folded_step() {
    assert_eq!(
        step_args(&workflow_with("w", "j", FOLDED_STEP)),
        ["-p a", "-p b"]
    );
}

validation::negative_control!(
    nextest_the_guard_reads_a_folded_step,
    "a `cargo test` in a folded step of a workflow with no exception",
    expected = "a CI step runs tests through `cargo test`",
    drop(step_args(&workflow_with(
        "w",
        "j",
        &FOLDED_STEP.replacen("cargo nextest run", "cargo test", 1)
    )))
);

/// A block step's doctest command, its `--doc` on a continuation line, so the guard passes it only if it reads the
/// two lines as the one command.
const CONTINUED_STEP: &str =
    "      - run: |\n          cargo test -p a \\\n            --doc\n          echo done\n";

#[test]
fn nextest_the_guard_reads_a_continued_command_whole() {
    assert_eq!(
        step_args(&workflow_with("w", "j", CONTINUED_STEP)),
        ["-p a"]
    );
}

validation::negative_control!(
    nextest_the_guard_reads_a_continued_command_whole,
    "the same two lines without the `\\`: a `cargo test -p a`, and a `--doc` that is no part of it",
    expected = "a CI step runs tests through `cargo test`",
    drop(step_args(&workflow_with(
        "w",
        "j",
        &CONTINUED_STEP.replace(" \\\n", "\n")
    )))
);

#[test]
fn nextest_a_no_run_build_is_no_test_step() {
    let build = "      - run: cargo test -p a --no-run --test b\n";
    assert!(step_args(&workflow_with("w", "j", build)).is_empty());
}

validation::negative_control!(
    nextest_a_no_run_build_is_no_test_step,
    "`--no-run` past `--`, where it goes to the test binary and every test runs",
    expected = "a CI step runs tests through `cargo test`",
    drop(step_args(&workflow_with(
        "w",
        "j",
        "      - run: cargo test -p a --test b -- --no-run\n"
    )))
);

/// Every workflow, with `from` replaced by `to` in the one that names it.
fn workflows_edited(from: &str, to: &str) -> Vec<String> {
    let edited = workflows();
    assert!(
        edited.iter().any(|workflow| workflow.contains(from)),
        "no workflow has `{from}`"
    );
    edited.iter().map(|w| w.replace(from, to)).collect()
}

/// The workflows as they are: every `cargo test` step in them is a doctest, a build or [`LIBTEST_SOAK`].
#[test]
fn nextest_the_soak_is_the_exception_in_its_workflow_alone() {
    test_steps(&workflows());
}

validation::negative_control!(
    nextest_the_soak_is_the_exception_in_its_workflow_alone,
    "the soak's job and step in a workflow of another name",
    expected = "a CI step runs tests through `cargo test`",
    drop(test_steps(&workflows_edited(
        "name: stand-in-soak\n",
        "name: another-soak\n"
    )))
);

#[test]
fn nextest_the_soak_is_the_exception_in_its_job_alone() {
    test_steps(&workflows());
}

validation::negative_control!(
    nextest_the_soak_is_the_exception_in_its_job_alone,
    "the soak's step in another job of its workflow",
    expected = "a CI step runs tests through `cargo test`",
    drop(test_steps(&workflows_edited(
        "\n  soak:\n",
        "\n  another:\n"
    )))
);

#[test]
fn nextest_the_soak_is_the_exception_for_its_command_alone() {
    test_steps(&workflows());
}

validation::negative_control!(
    nextest_the_soak_is_the_exception_for_its_command_alone,
    "another `cargo test` in the soak's job",
    expected = "a CI step runs tests through `cargo test`",
    drop(test_steps(&workflows_edited(
        "--test ${{ matrix.test }} || {",
        "--test ${{ matrix.test }} --features controls || {"
    )))
);

/// The text of `.github/workflows/mutants.yml`, whose `mutants` job has a block `cargo nextest run`, its own `tool:`
/// list installing cargo-mutants beside the pin.
fn mutants_workflow() -> String {
    std::fs::read_to_string(root().join(".github/workflows/mutants.yml")).unwrap()
}

#[test]
fn nextest_a_pin_in_a_tool_list_counts() {
    let (steps, pin) = test_steps(&[mutants_workflow()]);
    assert!(
        steps.iter().any(|step| step
            .args
            .join(" ")
            .contains("--test jobq --test qa_R-658 --test")),
        "the block `cargo nextest run` in `mutants` was not read whole"
    );
    assert_eq!(pin, "0.9.146");
}

validation::negative_control!(
    nextest_a_pin_in_a_tool_list_counts,
    "the `mutants` job's block `cargo nextest run` with cargo-nextest gone from its `tool:` list",
    expected = "job mutants: runs nextest without installing cargo-nextest at an exact version",
    drop(test_steps(&[
        mutants_workflow().replace(",cargo-nextest@0.9.146", "")
    ]))
);

/// The arguments of the `ci-workspace` step of `workflow` that runs controls, and the profile it runs under.
fn controls_step(workflow: &str) -> (Vec<String>, Option<String>) {
    let job = jobs(workflow)
        .into_iter()
        .find(|job| job[0].trim() == "ci-workspace:")
        .expect("no job `ci-workspace`");
    runs(&job)
        .into_iter()
        .find_map(|(run, profile)| {
            let rest = run.strip_prefix("cargo nextest run")?;
            rest.contains("--features controls").then(|| {
                (
                    rest.split_whitespace().map(str::to_owned).collect(),
                    profile.map(str::to_owned),
                )
            })
        })
        .expect("`ci-workspace` has no step that runs controls")
}

/// The control names in `words`, sorted.
fn control_names<'a>(words: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut names: Vec<&str> = words
        .into_iter()
        .filter(|word| word.ends_with("::negative_control"))
        .collect();
    names.sort_unstable();
    names
}

/// What the `xtask-ci` shards leave out, `workflow`'s `ci-workspace` job runs: one step, in the features the controls
/// compile under, and under the profile that selects their binary, names each of them. So each still runs once.
fn check_ci_workspace_runs_the_controls_the_shards_leave(workflow: &str) {
    let (args, profile) = controls_step(workflow);
    assert_eq!(
        control_names(args.iter().map(String::as_str)),
        control_names(IN_CI_WORKSPACE),
        "the controls `ci-workspace` runs are not the ones the shards leave to it"
    );
    assert_eq!(
        profile.as_deref(),
        Some("ci-workspace"),
        "the step that runs the controls the shards leave is not under the `ci-workspace` profile"
    );
}

#[test]
fn nextest_ci_workspace_runs_the_controls_the_shards_leave() {
    check_ci_workspace_runs_the_controls_the_shards_leave(&ci_workflow()[0]);
}

validation::negative_control!(
    nextest_ci_workspace_runs_the_controls_the_shards_leave,
    "a `ci-workspace` step without one of the controls the shards leave, which then runs nowhere",
    expected = "are not the ones the shards leave to it",
    check_ci_workspace_runs_the_controls_the_shards_leave(
        &ci_workflow()[0].replace(&format!(" {}", IN_CI_WORKSPACE[0]), "")
    )
);

/// `listed`, the names the `ci-workspace` step selects, is every control the shards leave and nothing more: a control
/// renamed in `xtask/tests/nextest.rs` and not here would run in its shard after all, and cost the build again.
fn check_the_step_selects_exactly_those_controls(listed: &[String]) {
    assert_eq!(
        control_names(listed.iter().map(String::as_str)),
        control_names(IN_CI_WORKSPACE),
        "the `ci-workspace` step does not select exactly the controls the shards leave"
    );
    assert_eq!(
        listed.len(),
        IN_CI_WORKSPACE.len(),
        "the `ci-workspace` step selects more than the controls the shards leave: {listed:?}"
    );
}

#[test]
fn nextest_the_ci_workspace_step_selects_exactly_those_controls() {
    let (args, profile) = controls_step(&ci_workflow()[0]);
    check_the_step_selects_exactly_those_controls(&nextest_list(&args, profile.as_deref()));
}

validation::negative_control!(
    nextest_the_ci_workspace_step_selects_exactly_those_controls,
    "a listing that holds a control name no longer in `xtask/tests/nextest.rs`",
    expected = "does not select exactly the controls the shards leave",
    check_the_step_selects_exactly_those_controls(&[
        IN_CI_WORKSPACE[0].to_owned(),
        IN_CI_WORKSPACE[1].to_owned(),
        "renamed::negative_control".to_owned(),
    ])
);

/// Runs `cargo <args>` on this workspace; its stdout, which it must have exited well to give.
fn cargo(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO"))
        .args(args)
        .current_dir(root())
        .timed_output()
        .expect("run cargo");
    assert!(
        output.status.success(),
        "cargo {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The names `cargo test <args> -- --list` lists: every test, ignored or not, doctests included.
fn libtest_list(args: &[String]) -> Vec<String> {
    let mut command: Vec<&str> = vec!["test"];
    command.extend(args.iter().map(String::as_str));
    command.extend(["--", "--list"]);
    cargo(&command)
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect()
}

/// The names `cargo nextest list <args>` lists under `profile` (`None`: the default profile), ignored tests included.
/// The profile is always passed, so a NEXTEST_PROFILE this test runs under does not reach the listing.
fn nextest_list(args: &[String], profile: Option<&str>) -> Vec<String> {
    let mut command: Vec<&str> = vec!["nextest", "list"];
    command.extend(args.iter().map(String::as_str));
    command.extend(["--profile", profile.unwrap_or("default")]);
    command.extend([
        "--run-ignored",
        "all",
        "--message-format",
        "oneline",
        "--color",
        "never",
    ]);
    cargo(&command)
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(_, name)| name.to_owned())
        .collect()
}

/// A step at the release profile: `ci-release`'s, which runs the `ci` profile's tests again at `--release` (R-547,
/// REQ-TOOL-189).
fn at_release(step: &Step) -> bool {
    step.args.iter().any(|a| a == "--release")
}

/// For each of `replaced` (some of `REPLACED`), the test steps of `workflows` in its feature set together list every
/// test it lists, as many times. The steps at the release profile ([`at_release`]) are not counted: they run again
/// tests the others run, so the others must list every test without them, and a shard or profile dropped from those
/// is still seen; listing them would also build the workspace at release inside the listing's timeout.
fn check_no_test_dropped(workflows: &[String], replaced: &[&str]) {
    let (steps, _) = test_steps(workflows);
    for replaced in replaced {
        let replaced: Vec<String> = replaced.split_whitespace().map(str::to_owned).collect();
        let mut covered: BTreeMap<String, usize> = BTreeMap::new();
        for step in steps
            .iter()
            .filter(|s| features(&s.args) == features(&replaced) && !at_release(s))
        {
            let listed = if step.doc {
                let mut args = step.args.clone();
                args.push("--doc".to_owned());
                libtest_list(&args)
            } else {
                nextest_list(&step.args, step.profile.as_deref())
            };
            for name in listed {
                *covered.entry(name).or_default() += 1;
            }
        }
        let mut wanted: BTreeMap<String, usize> = BTreeMap::new();
        for name in libtest_list(&replaced) {
            *wanted.entry(name).or_default() += 1;
        }
        let dropped: Vec<&String> = wanted
            .iter()
            .filter(|(name, n)| covered.get(*name).copied().unwrap_or(0) < **n)
            .map(|(name, _)| name)
            .collect();
        assert!(
            !wanted.is_empty() && dropped.is_empty(),
            "the CI test steps drop tests `cargo test {}` lists: {dropped:?}",
            replaced.join(" ")
        );
    }
}

// The check on all of `REPLACED`, split one test per entry, so that no one test holds a `ci` shard (R-366,
// REQ-SYS-077): each lists the steps of its own feature set. Their names place them by nextest's `hash:` partition:
// the default feature set's in shard 2; the two `controls` ones in shard 4, beside the two full checks below, which
// build the same `controls` test binaries first.

#[test]
fn nextest_ci_steps_list_every_test_cargo_test_listed_without_features() {
    check_no_test_dropped(&ci_workflow(), &[REPLACED[0]]);
}

validation::negative_control!(
    nextest_ci_steps_list_every_test_cargo_test_listed_without_features,
    "a CI whose `ci` shards run from an archive of kernel's tests alone, required to list every test",
    expected = "the CI test steps drop tests `cargo test --workspace` lists",
    check_no_test_dropped(
        &ci_workflow()
            .iter()
            .map(|w| w.replace(
                "run: cargo nextest archive --workspace",
                "run: cargo nextest archive -p kernel"
            ))
            .collect::<Vec<_>>(),
        &[REPLACED[0]]
    )
);

#[test]
fn nextest_ci_steps_list_every_test_cargo_test_listed_under_metal_hosted_probe() {
    check_no_test_dropped(&ci_workflow(), &[REPLACED[1]]);
}

validation::negative_control!(
    nextest_ci_steps_list_every_test_cargo_test_listed_under_metal_hosted_probe,
    "a CI whose metal_hosted_probe nextest step filters on a name no test has, required to list every test",
    expected = "the CI test steps drop tests `cargo test -p validation --features controls metal_hosted_probe` lists",
    check_no_test_dropped(
        &ci_workflow()
            .iter()
            .map(|w| w.replace(
                "run: cargo nextest run -p validation --features controls metal_hosted_probe ",
                "run: cargo nextest run -p validation --features controls no_such_test "
            ))
            .collect::<Vec<_>>(),
        &[REPLACED[1]]
    )
);

#[test]
fn nextest_ci_steps_list_every_gpu_harness_test_cargo_test_listed() {
    check_no_test_dropped(&ci_workflow(), &[REPLACED[2]]);
}

validation::negative_control!(
    nextest_ci_steps_list_every_gpu_harness_test_cargo_test_listed,
    "a CI whose gpu_harness nextest steps filter on a name no test has, required to list every test",
    expected = "the CI test steps drop tests `cargo test -p validation --features controls gpu_harness` lists",
    check_no_test_dropped(
        &ci_workflow()
            .iter()
            .map(|w| w.replace(
                "run: cargo nextest run -p validation --features controls gpu_harness ",
                "run: cargo nextest run -p validation --features controls no_such_test "
            ))
            .collect::<Vec<_>>(),
        &[REPLACED[2]]
    )
);

/// The same check, here for its control on the shards: the `ci` job's 4 nextest shards, each running its slice from the
/// one archive `ci-archive` builds (R-372, REQ-SYS-078), together list every test the unsharded run did (R-336,
/// REQ-SYS-077).
#[test]
fn nextest_ci_shards_together_list_every_test() {
    check_no_test_dropped(&ci_workflow(), REPLACED);
}

validation::negative_control!(
    nextest_ci_shards_together_list_every_test,
    "a CI whose `ci` job runs 3 of its 4 shards, required to list every test",
    expected = "the CI test steps drop tests `cargo test --workspace` lists",
    check_no_test_dropped(
        &ci_workflow()
            .iter()
            .map(|w| w.replacen("shard: [1, 2, 3, 4]", "shard: [1, 2, 3]", 1))
            .collect::<Vec<_>>(),
        REPLACED
    )
);

/// The same check, here for its control on the profiles: CI's `ci`, `ci-workspace` and `gpu-kernel` jobs split the
/// workspace's tests by nextest profile (R-325, R-372), and each step is listed under its own.
#[test]
fn nextest_ci_steps_list_every_test_under_their_profiles() {
    check_no_test_dropped(&ci_workflow(), REPLACED);
}

validation::negative_control!(
    nextest_ci_steps_list_every_test_under_their_profiles,
    "a CI whose gpu-kernel step runs under the ci profile, whose filter leaves out the tests that need the built \
     kernel (R-325), required to list every test",
    expected = "the CI test steps drop tests `cargo test --workspace` lists",
    check_no_test_dropped(
        &ci_workflow()
            .iter()
            .map(|w| w.replace("NEXTEST_PROFILE: gpu-kernel", "NEXTEST_PROFILE: ci"))
            .collect::<Vec<_>>(),
        REPLACED
    )
);

/// Validation's `qa_TASK-M0-24`, whose `cargo xtask controls --list` rebuilt inside a `ci` shard from the archive.
const CONTROLS_LISTING_BINARY: &str = "binary_id(=validation::qa_TASK-M0-24)";

/// The text of the nextest profile `profile` of `config`: from its header to the next `[` header.
fn profile_section<'a>(config: &'a str, profile: &str) -> &'a str {
    let header = format!("[profile.{profile}]");
    let start = config.find(&header).expect("the nextest profile exists");
    let rest = &config[start + header.len()..];
    rest.find("\n[").map_or(rest, |end| &rest[..end])
}

/// `qa_TASK-M0-24` is out of the `ci` shards, whose filter excludes by name what it lists (`not (…)`), in the
/// `ci-workspace` profile, and in each of `workflows` one step runs validation under that profile, which selects only
/// this binary of it; so its tests run once per run, and the build they need is the one that job has.
fn check_the_controls_listing_runs_in_ci_workspace(config: &str, workflows: &[String]) {
    let ci = profile_section(config, "ci");
    assert!(
        ci.contains("not (") && ci.contains(CONTROLS_LISTING_BINARY),
        "the `ci` profile selects `qa_TASK-M0-24`, which rebuilds the workspace in its shard"
    );
    assert!(
        profile_section(config, "ci-workspace").contains(CONTROLS_LISTING_BINARY),
        "the `ci-workspace` profile does not select `qa_TASK-M0-24`"
    );
    for workflow in workflows {
        let steps = jobs(workflow)
            .iter()
            .flat_map(|job| runs(job))
            .filter(|(run, profile)| {
                *profile == Some("ci-workspace") && run.contains("-p validation")
            })
            .count();
        assert_eq!(
            steps, 1,
            "a workflow does not run `qa_TASK-M0-24` once, under the `ci-workspace` profile"
        );
    }
}

/// `ci.yml`, and the two other workflows that run the `ci` profile, which would otherwise lose the test.
fn workflows_running_the_ci_profile() -> Vec<String> {
    ["ci", "gate", "nightly"]
        .map(|name| {
            std::fs::read_to_string(root().join(format!(".github/workflows/{name}.yml"))).unwrap()
        })
        .to_vec()
}

fn nextest_config() -> String {
    std::fs::read_to_string(root().join(".config/nextest.toml")).unwrap()
}

#[test]
fn nextest_the_controls_listing_runs_in_ci_workspace() {
    check_the_controls_listing_runs_in_ci_workspace(
        &nextest_config(),
        &workflows_running_the_ci_profile(),
    );
}

validation::negative_control!(
    nextest_the_controls_listing_runs_in_ci_workspace,
    "workflows whose run of `qa_TASK-M0-24` is under the `ci` profile, which leaves it out",
    expected = "does not run `qa_TASK-M0-24` once, under the `ci-workspace` profile",
    check_the_controls_listing_runs_in_ci_workspace(
        &nextest_config(),
        &workflows_running_the_ci_profile()
            .into_iter()
            .map(|w| w.replace(
                "NEXTEST_PROFILE: ci-workspace\n        run: cargo nextest run -p validation",
                "NEXTEST_PROFILE: ci\n        run: cargo nextest run -p validation"
            ))
            .collect::<Vec<_>>()
    )
);

/// The test binary of `xtask ci --list`'s test.
const CI_LISTING_BINARY: &str = "qa_TASK-M0-06_list";

/// That test: its child's real `controls --list` rebuilt the `--features controls` workspace inside a `ci` shard, from
/// the archive, which carries no cargo build state (run 41775231980, `ci (3)`).
const CI_LISTING_TEST: &str = "qa_m006_ci_list_lists_golden_cases_without_a_device";

/// Given the tests of the binary that nextest lists under the `ci` and the `ci-workspace` profile: the one test moved,
/// and only it, so the binary's others stay in the shards.
fn check_the_ci_listing_left_the_shards(in_ci: &[String], in_ci_workspace: &[String]) {
    assert!(
        !in_ci.iter().any(|name| name == CI_LISTING_TEST),
        "the `ci` profile selects `xtask ci --list`'s test, which rebuilds the workspace in its shard"
    );
    assert!(
        !in_ci.is_empty(),
        "the `ci` profile left the rest of `xtask ci --list`'s test's binary out too"
    );
    assert_eq!(
        in_ci_workspace,
        [CI_LISTING_TEST],
        "the `ci-workspace` profile does not select exactly `xtask ci --list`'s test of its binary"
    );
}

#[test]
fn nextest_the_ci_listing_left_the_shards() {
    let args: Vec<String> = ["-p", "xtask", "--test", CI_LISTING_BINARY]
        .map(str::to_owned)
        .to_vec();
    check_the_ci_listing_left_the_shards(
        &nextest_list(&args, Some("ci")),
        &nextest_list(&args, Some("ci-workspace")),
    );
}

validation::negative_control!(
    nextest_the_ci_listing_left_the_shards,
    "`xtask ci --list`'s test still listed under the `ci` profile",
    expected = "the `ci` profile selects `xtask ci --list`'s test",
    check_the_ci_listing_left_the_shards(
        &[CI_LISTING_TEST.to_owned()],
        &[CI_LISTING_TEST.to_owned()]
    )
);

/// Each of `workflows` runs that test once, in the one step under `ci-workspace` that selects it: `-p xtask` whole, or
/// its binary by name, and in the default feature set, for the profile selects it there.
fn check_the_ci_listing_runs_in_each_workflow(workflows: &[String]) {
    let by_name = format!("--test {CI_LISTING_BINARY}");
    for workflow in workflows {
        let steps = jobs(workflow)
            .iter()
            .flat_map(|job| runs(job))
            .filter(|(run, profile)| {
                *profile == Some("ci-workspace")
                    && run.contains("-p xtask")
                    && !run.contains("--features")
                    && (!run.contains("--test ") || run.contains(&by_name))
            })
            .count();
        assert_eq!(
            steps, 1,
            "a workflow does not run `xtask ci --list`'s test once, under the `ci-workspace` profile"
        );
    }
}

#[test]
fn nextest_the_ci_listing_runs_in_each_workflow() {
    check_the_ci_listing_runs_in_each_workflow(&workflows_running_the_ci_profile());
}

validation::negative_control!(
    nextest_the_ci_listing_runs_in_each_workflow,
    "a nightly run of `xtask ci --list`'s test under the `ci` profile, which leaves it out",
    expected = "does not run `xtask ci --list`'s test once, under the `ci-workspace` profile",
    check_the_ci_listing_runs_in_each_workflow(
        &workflows_running_the_ci_profile()
            .into_iter()
            .map(|w| w.replace(
                "NEXTEST_PROFILE: ci-workspace\n        run: cargo nextest run -p xtask --test",
                "NEXTEST_PROFILE: ci\n        run: cargo nextest run -p xtask --test"
            ))
            .collect::<Vec<_>>()
    )
);
