//! REQ-VAL-165 (R-231): CI's test steps run through `cargo nextest run`, pinned, with doctests, which nextest does not
//! run, through `cargo test --doc`, and no test is dropped: in each feature set, the nextest and `--doc` steps
//! together list every test that the `cargo test` steps they replaced list (`cargo test <args> -- --list`); a step
//! sharded by its job's matrix (`--partition hash:${{ matrix.shard }}/4`, R-336, R-366) is listed once per shard, and a
//! step that runs from a nextest archive (`--archive-file`, R-372, REQ-SYS-078) is listed with the arguments the
//! `cargo nextest archive` step that builds that archive was given. The documented local run (README) installs the same
//! pinned version.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use validation::spawn::Spawn;

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

fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).unwrap()
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

/// The commands of a job's `run:` steps, each with the nextest profile it runs under: the `NEXTEST_PROFILE` its step's
/// `env:` sets, or else the job's, or none (the default profile). CI's `ci`, `ci-workspace` and `gpu-kernel` jobs split
/// the workspace's tests by profile (R-325, R-372, `.config/nextest.toml`).
fn runs<'a>(job: &[&'a str]) -> Vec<(&'a str, Option<&'a str>)> {
    let mut out = Vec::new();
    let mut in_steps = false;
    let (mut job_profile, mut step_profile) = (None, None);
    for line in job {
        let line = line.trim_start();
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
            out.push((run.trim(), step_profile.or(job_profile)));
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

/// The feature that adds gui's mock engine, which CI's archive and clippy build (RQ-252): it adds the mock to gui and
/// nothing else, so a step with it alone is in the default feature set.
const GUI_MOCK: &str = "gui/mock";

/// The value of `--features` in `args`, or "" for the default feature set, which [`GUI_MOCK`] alone stays in.
fn features(args: &[String]) -> &str {
    let features = args
        .iter()
        .position(|a| a == "--features")
        .and_then(|i| args.get(i + 1))
        .map_or("", String::as_str);
    if features == GUI_MOCK {
        ""
    } else {
        features
    }
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

/// The test steps of `workflows`, and the pinned cargo-nextest version they install. Every job with a nextest step
/// installs cargo-nextest at an exact version, and every job that installs it installs the same one; no step runs
/// `cargo test` but for doctests. A nextest step that runs from an archive (`--archive-file`) has, in place of its reuse
/// options, the arguments of the step in its workflow that builds that archive.
fn test_steps(workflows: &[String]) -> (Vec<Step>, String) {
    let mut steps = Vec::new();
    let mut pins: Vec<String> = Vec::new();
    for workflow in workflows {
        let archives = archives(workflow);
        for job in jobs(workflow) {
            let mut nextest = false;
            for (run, profile) in runs(&job).into_iter().flat_map(|(run, profile)| {
                shards(&job, run).into_iter().map(move |run| (run, profile))
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
                let version = line.trim().strip_prefix("tool: cargo-nextest@")?;
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

/// The documented local run in `readme` installs cargo-nextest at `pin` and runs the suite and the doctests.
fn check_documented_run(readme: &str, pin: &str) {
    for line in [
        format!("cargo install cargo-nextest --version {pin} --locked"),
        "cargo nextest run --workspace".to_owned(),
        "cargo test --workspace --doc".to_owned(),
    ] {
        assert!(
            readme.lines().any(|l| l.trim() == line),
            "the README's local run lacks `{line}`"
        );
    }
}

#[test]
fn nextest_ci_and_the_documented_run_install_the_pinned_nextest() {
    let (_, pin) = test_steps(&workflows());
    check_documented_run(&readme(), &pin);
}

validation::negative_control!(
    nextest_ci_and_the_documented_run_install_the_pinned_nextest,
    "a README installing another cargo-nextest version than CI's, required to document the pinned one",
    expected = "the README's local run lacks `cargo install cargo-nextest --version",
    {
        let (_, pin) = test_steps(&workflows());
        check_documented_run(
            &readme().replace(&format!("--version {pin} "), "--version 0.0.1 "),
            &pin,
        )
    }
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

/// For each of `replaced` (some of `REPLACED`), the test steps of `workflows` in its feature set together list every
/// test it lists, as many times.
fn check_no_test_dropped(workflows: &[String], replaced: &[&str]) {
    let (steps, _) = test_steps(workflows);
    for replaced in replaced {
        let replaced: Vec<String> = replaced.split_whitespace().map(str::to_owned).collect();
        let mut covered: BTreeMap<String, usize> = BTreeMap::new();
        for step in steps
            .iter()
            .filter(|s| features(&s.args) == features(&replaced))
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
                "run: cargo nextest archive --workspace --features gui/mock",
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
