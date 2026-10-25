//! qa's tests for TASK-M0-52, written from REQ-SYS-078: "The `ci` CI job's test binaries must be built once per
//! feature set its shards run, as a nextest archive (`cargo nextest archive`) that every `ci` shard runs its
//! `--partition hash:<k>/4` slice from (`--archive-file`), so no shard compiles the tests; the shards together still
//! run every test the unsharded run did" (R-372); verify, as amended per R-369 (the orchestrator's decision on PR #291,
//! TASK-M2-49): "`ci.yml` builds the tests once per build profile, in one archive job each: the dev feature sets in
//! `ci-archive`, which every `ci` shard runs from, and the release set in `ci-release-archive`, which every
//! `ci-release` shard runs from (R-547, REQ-TOOL-189); each archive is built once and shared by its shards, with no
//! `cargo build` or test compile of their own; [...] the doctests still run, in shard 1". The task's deliverables add:
//! the archive is built with the `ci` profile's settings, uploaded as a workflow artifact, and each job's cache keys
//! name its job, with R-326's save rule (R-285).
//!
//! - The workflow (`.github/workflows/ci.yml`): one job builds every archive of a build profile, `ci-archive` the dev
//!   profile's, one per feature set the `ci` shards run, `ci-release-archive` the release profile's; each archive file
//!   is built once; each shard job (`ci`, a 4-shard matrix, and `ci-release`) runs from the archives of one build
//!   profile, waits on that profile's archive job (`needs:`), downloads the artifact it uploads, and runs from the very
//!   file it built; no other `cargo` command in a shard builds or compiles, but the workspace's doctests, at the job's
//!   build profile, in shard 1 alone; the archive is built under the shards' nextest profile; each cache key names its
//!   job and is saved only on a push to `main`.
//! - nextest itself, on a fixture crate of its own: the 4 `hash:` shards run from one archive, with the crate's own
//!   build directory gone, together list every test the source build lists, each once, and running them compiles
//!   nothing.
//!
//! Each test registers a negative control (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;

// ---- The workflow ---------------------------------------------------------------------------------------------------

/// CI's per-push workflow, as checked in.
fn the_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/ci.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `workflow` with `from` replaced by `to` once, `from` required to be there (the controls' edits).
#[cfg(feature = "controls")]
fn edited(workflow: &str, from: &str, to: &str) -> String {
    assert!(
        workflow.contains(from),
        "the workflow has no {from:?} to edit"
    );
    workflow.replacen(from, to, 1)
}

/// A job: its id and its lines, comment lines dropped.
struct Job {
    id: String,
    lines: Vec<String>,
}

/// A step: its lines.
type Step = Vec<String>;

impl Job {
    /// The value of the job's own key `key` (indent 4).
    fn own(&self, key: &str) -> Option<String> {
        self.lines.iter().find_map(|l| {
            let indent = l.len() - l.trim_start().len();
            if indent != 4 {
                return None;
            }
            l.trim()
                .strip_prefix(key)
                .and_then(|r| r.strip_prefix(':'))
                .map(|v| v.trim().to_owned())
        })
    }

    /// The jobs the job's `needs:` names.
    fn needs(&self) -> Vec<String> {
        self.own("needs")
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// The job's steps (each from a `- ` at indent 6).
    fn steps(&self) -> Vec<Step> {
        let mut steps: Vec<Step> = Vec::new();
        for l in &self.lines {
            if l.starts_with("      - ") {
                steps.push(vec![l.clone()]);
            } else if l.starts_with("       ") {
                if let Some(s) = steps.last_mut() {
                    s.push(l.clone());
                }
            }
        }
        steps
    }

    /// The job's matrix `shard:` list.
    fn shards(&self) -> Option<Vec<String>> {
        self.lines.iter().find_map(|l| {
            let list = l.trim().strip_prefix("shard:")?.trim();
            let list = list.strip_prefix('[')?.strip_suffix(']')?;
            Some(list.split(',').map(|v| v.trim().to_owned()).collect())
        })
    }
}

/// The value of `name` in a step (`- key:` or `key:` at any depth), trimmed.
fn key(step: &[String], name: &str) -> Option<String> {
    step.iter().find_map(|l| {
        l.trim()
            .trim_start_matches("- ")
            .strip_prefix(name)
            .and_then(|r| r.strip_prefix(':'))
            .map(|v| v.trim().to_owned())
    })
}

/// Every command line of a step's `run:`: the one-line form, or each line of a `run: |` block.
fn commands(step: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut block: Option<usize> = None;
    for l in step {
        let t = l.trim().trim_start_matches("- ");
        let indent = l.len() - l.trim_start().len();
        if let Some(at) = block {
            if indent > at {
                out.push(l.trim().to_owned());
                continue;
            }
            block = None;
        }
        if let Some(run) = t.strip_prefix("run:") {
            let run = run.trim();
            if run == "|" || run == ">" {
                block = Some(indent);
            } else {
                out.push(run.to_owned());
            }
        }
    }
    out
}

/// The jobs of `workflow`.
fn jobs(workflow: &str) -> Vec<Job> {
    let mut found: Vec<Job> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            in_jobs = t == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        if line.starts_with("  ") && !line.starts_with("   ") && t.ends_with(':') {
            found.push(Job {
                id: t.trim_end_matches(':').to_owned(),
                lines: Vec::new(),
            });
        } else if let Some(job) = found.last_mut() {
            job.lines.push(line.to_owned());
        }
    }
    found
}

/// A path as the shell sees it: `${{ runner.temp }}` is `$RUNNER_TEMP`.
fn shell_path(path: &str) -> String {
    path.replace("${{ runner.temp }}", "$RUNNER_TEMP")
}

/// The value of option `opt` among `words`.
fn opt(words: &[&str], opt: &str) -> Option<String> {
    words
        .iter()
        .position(|w| *w == opt)
        .and_then(|i| words.get(i + 1))
        .map(|v| (*v).to_owned())
}

/// The feature set a cargo command's words select: the words naming features, sorted ("" for the default).
fn feature_set(words: &[&str]) -> String {
    let mut f: Vec<String> = Vec::new();
    let mut take = false;
    for w in words {
        if std::mem::take(&mut take) {
            f.push(format!("--features {w}"));
        } else if *w == "--features" || *w == "-F" {
            take = true;
        } else if w.starts_with("--features=")
            || *w == "--all-features"
            || *w == "--no-default-features"
        {
            f.push((*w).to_owned());
        }
    }
    f.sort();
    f.join(" ")
}

/// The cargo build profile a command's words select: `release` for `--release`, the name `--cargo-profile` or
/// `--profile` gives, `dev` otherwise.
fn build_profile(words: &[&str]) -> String {
    if words.contains(&"--release") {
        return "release".to_owned();
    }
    opt(words, "--cargo-profile")
        .or_else(|| opt(words, "--profile"))
        .unwrap_or_else(|| "dev".to_owned())
}

/// An archive step: its job, its `--archive-file` (as the shell sees it), its words, its NEXTEST_PROFILE, and the
/// cargo build profile it builds the tests at.
struct Archive {
    job: String,
    file: String,
    words: Vec<String>,
    profile: Option<String>,
    build: String,
}

impl Archive {
    /// The build profile and feature set the archive's tests are built with.
    fn set(&self) -> (String, String) {
        let words: Vec<&str> = self.words.iter().map(String::as_str).collect();
        (self.build.clone(), feature_set(&words))
    }
}

/// The archive job REQ-SYS-078's verify names for each build profile, as amended per R-369 (the orchestrator's
/// decision on PR #291): the dev feature sets in `ci-archive`, the release set in `ci-release-archive`.
const ARCHIVE_JOBS: &[(&str, &str)] = &[("dev", "ci-archive"), ("release", "ci-release-archive")];

/// The shard jobs REQ-SYS-078's verify names, each with the build profile whose archive its every shard runs from:
/// `ci`'s from `ci-archive`'s, `ci-release`'s from `ci-release-archive`'s (R-547, REQ-TOOL-189).
const SHARD_JOBS: &[(&str, &str)] = &[("ci", "dev"), ("ci-release", "release")];

/// REQ-SYS-078 on `workflow`.
fn check_archive(workflow: &str) {
    let jobs = jobs(workflow);

    // The archives: every `cargo nextest archive` step, all of one build profile in one job, each file built once.
    let mut archives: Vec<Archive> = Vec::new();
    for job in &jobs {
        for step in job.steps() {
            for c in commands(&step) {
                let Some(rest) = c.strip_prefix("cargo nextest archive") else {
                    continue;
                };
                let words: Vec<&str> = rest.split_whitespace().collect();
                let file = opt(&words, "--archive-file").unwrap_or_else(|| {
                    panic!("job `{}` runs `{c}` with no --archive-file", job.id)
                });
                archives.push(Archive {
                    job: job.id.clone(),
                    file: shell_path(&file),
                    words: words.iter().map(|w| (*w).to_owned()).collect(),
                    profile: key(&step, "NEXTEST_PROFILE"),
                    build: build_profile(&words),
                });
            }
        }
    }
    let builds: BTreeSet<&str> = archives.iter().map(|a| a.build.as_str()).collect();
    let mut archive_job_of: BTreeMap<String, String> = BTreeMap::new();
    for build in &builds {
        let in_jobs: BTreeSet<&str> = archives
            .iter()
            .filter(|a| a.build == *build)
            .map(|a| a.job.as_str())
            .collect();
        assert_eq!(
            in_jobs.len(),
            1,
            "the {build} archives are built in {} jobs ({in_jobs:?}), not in one",
            in_jobs.len()
        );
        archive_job_of.insert(
            (*build).to_owned(),
            in_jobs.into_iter().next().unwrap().to_owned(),
        );
    }
    let want_archive_jobs: BTreeMap<String, String> = ARCHIVE_JOBS
        .iter()
        .map(|(b, j)| ((*b).to_owned(), (*j).to_owned()))
        .collect();
    assert_eq!(
        archive_job_of, want_archive_jobs,
        "the archive job of each build profile is not the one REQ-SYS-078 names"
    );
    let mut files: BTreeSet<&str> = BTreeSet::new();
    for a in &archives {
        assert!(
            files.insert(a.file.as_str()),
            "the archive `{}` is built more than once",
            a.file
        );
    }

    // The shard jobs: every job whose steps run nextest's `hash:` slices of a shard matrix.
    let shard_jobs: Vec<&Job> = jobs
        .iter()
        .filter(|j| {
            j.steps().iter().flat_map(|s| commands(s)).any(|c| {
                c.starts_with("cargo nextest run")
                    && c.contains("--partition hash:${{ matrix.shard }}/")
            })
        })
        .collect();
    let quarters: Vec<&str> = shard_jobs
        .iter()
        .filter(|j| {
            j.steps()
                .iter()
                .flat_map(|s| commands(s))
                .any(|c| c.contains("--partition hash:${{ matrix.shard }}/4"))
        })
        .map(|j| j.id.as_str())
        .collect();
    assert_eq!(quarters, ["ci"], "no single job runs the nextest shards");

    // The artifacts: uploaded by the archive jobs.
    let uploads: Vec<(String, String, String)> = jobs
        .iter()
        .filter(|j| archive_job_of.values().any(|a| *a == j.id))
        .flat_map(|j| {
            j.steps()
                .into_iter()
                .filter(|s| {
                    key(s, "uses").is_some_and(|u| u.starts_with("actions/upload-artifact@"))
                })
                .map(|s| {
                    (
                        j.id.clone(),
                        key(&s, "name").unwrap_or_default(),
                        shell_path(&key(&s, "path").unwrap_or_default()),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Each shard job: every `cargo` command a run from a downloaded archive of one build profile, or the doctests in
    // shard 1 at that profile.
    let mut shard_sets: BTreeSet<(String, String)> = BTreeSet::new();
    let mut runs_from: BTreeMap<String, String> = BTreeMap::new();
    let mut checks: Vec<(&Job, String, Vec<String>)> = Vec::new();
    for sj in &shard_jobs {
        let downloads: Vec<(String, String)> = sj
            .steps()
            .iter()
            .filter(|s| key(s, "uses").is_some_and(|u| u.starts_with("actions/download-artifact@")))
            .map(|s| {
                (
                    key(s, "name").unwrap_or_default(),
                    shell_path(&key(s, "path").unwrap_or_default()),
                )
            })
            .collect();
        let mut sj_builds: BTreeSet<String> = BTreeSet::new();
        let mut parts: BTreeSet<String> = BTreeSet::new();
        let mut doctests: Vec<String> = Vec::new();
        for step in sj.steps() {
            for c in commands(&step) {
                if !c.starts_with("cargo ") {
                    continue;
                }
                let words: Vec<&str> = c.split_whitespace().collect();
                if c.starts_with("cargo nextest run ") {
                    let file = opt(&words, "--archive-file")
                        .map(|f| shell_path(&f))
                        .unwrap_or_else(|| {
                            panic!(
                                "a `{}` shard compiles its tests: `{c}` runs from no archive",
                                sj.id
                            )
                        });
                    let built = archives.iter().find(|a| a.file == file).unwrap_or_else(|| {
                        panic!(
                            "a `{}` shard runs from `{file}`, which no archive step builds",
                            sj.id
                        )
                    });
                    let up = uploads
                        .iter()
                        .find(|(j, _, p)| *j == built.job && *p == built.file)
                        .unwrap_or_else(|| {
                            panic!("`{}` does not upload the archive `{file}`", built.job)
                        });
                    let file_name = Path::new(&file)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    assert!(
                        downloads.iter().any(
                            |(name, dir)| *name == up.1 && format!("{dir}/{file_name}") == file
                        ),
                        "a `{}` shard runs from `{file}`, but downloads no artifact `{}` there",
                        sj.id,
                        up.1
                    );
                    assert!(
                        built.words.iter().any(|w| w == "--workspace"),
                        "the archive `{file}` is not of the whole workspace"
                    );
                    assert_eq!(
                        built.profile,
                        key(&step, "NEXTEST_PROFILE"),
                        "the archive `{file}` is not built under the shards' nextest profile"
                    );
                    if let Some((_, n)) = c.split_once("--partition hash:${{ matrix.shard }}/") {
                        parts.insert(n.split_whitespace().next().unwrap_or("").to_owned());
                    }
                    sj_builds.insert(built.build.clone());
                    shard_sets.insert(built.set());
                } else if c.starts_with("cargo test ") && words.contains(&"--doc") {
                    assert_eq!(
                        key(&step, "if").as_deref(),
                        Some("matrix.shard == 1"),
                        "a `{}` shard's doctest step `{c}` runs in some shard other than shard 1 alone",
                        sj.id
                    );
                    if words.contains(&"--workspace") {
                        doctests.push(build_profile(&words));
                    }
                } else {
                    panic!("a `{}` shard builds or compiles of its own: `{c}`", sj.id);
                }
            }
        }
        assert_eq!(
            sj_builds.len(),
            1,
            "the `{}` shards run from the archives of {} build profiles ({sj_builds:?}), not of one",
            sj.id,
            sj_builds.len()
        );
        let build = sj_builds.into_iter().next().unwrap();

        // The matrix is the shards 1 to n of the `hash:<k>/n` the job runs.
        let n: u32 = parts
            .iter()
            .find_map(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("job `{}` runs no `hash:<k>/<n>` slice", sj.id));
        assert_eq!(
            sj.shards(),
            Some((1..=n).map(|k| k.to_string()).collect::<Vec<_>>()),
            "job `{}`'s matrix is not the shards 1 to {n}",
            sj.id
        );

        runs_from.insert(sj.id.clone(), build.clone());
        checks.push((sj, build, doctests));
    }
    let want_runs_from: BTreeMap<String, String> = SHARD_JOBS
        .iter()
        .map(|(j, b)| ((*j).to_owned(), (*b).to_owned()))
        .collect();
    assert_eq!(
        runs_from, want_runs_from,
        "the shard jobs do not each run from the archive of the build profile REQ-SYS-078 names"
    );

    // Each shard job waits on its profile's archive job, and runs the workspace's doctests once, in shard 1, at its
    // build profile.
    for (sj, build, doctests) in checks {
        let archive_job_id = &archive_job_of[&build];
        assert!(
            sj.needs().contains(archive_job_id),
            "job `{}` does not wait (`needs:`) on `{archive_job_id}`, which builds its archive",
            sj.id
        );
        assert_eq!(
            doctests,
            std::slice::from_ref(&build),
            "the workspace's doctests do not run, once, in shard 1 of `{}`, at its build profile {build}",
            sj.id
        );
    }

    // One archive per build profile and feature set the shards run, and none other.
    let mut built_sets: Vec<(String, String)> = archives.iter().map(Archive::set).collect();
    built_sets.sort();
    assert_eq!(
        built_sets,
        shard_sets.into_iter().collect::<Vec<_>>(),
        "the archives are not one per feature set the shards run"
    );

    // Each job's cache is its own, restored on pull requests and saved only on a push to `main` (R-285, R-326).
    for job in &jobs {
        for step in job.steps() {
            if !key(&step, "uses").is_some_and(|u| u.starts_with("Swatinem/rust-cache@")) {
                continue;
            }
            assert_eq!(
                key(&step, "shared-key").as_deref(),
                Some(job.id.as_str()),
                "job `{}`'s cache key does not name its job",
                job.id
            );
            assert_eq!(
                key(&step, "save-if").as_deref(),
                Some("${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}"),
                "job `{}`'s cache is saved other than on a push to `main`",
                job.id
            );
        }
    }
}

#[test]
fn qa_m0_52_ci_shards_wait_on_the_archive_job() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_ci_shards_wait_on_the_archive_job,
    "the checked-in workflow with the `ci` job's `needs:` dropped, so a shard can start before its archive exists",
    expected = "does not wait (`needs:`)",
    check_archive(&edited(&the_workflow(), "  ci:\n    needs: ci-archive\n", "  ci:\n"))
);

#[test]
fn qa_m0_52_no_shard_builds_of_its_own() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_no_shard_builds_of_its_own,
    "the checked-in workflow with a `cargo build --workspace` step added to the `ci` shards",
    expected = "shard builds or compiles of its own: `cargo build --workspace`",
    check_archive(&edited(
        &the_workflow(),
        "      - uses: actions/download-artifact@v4\n",
        "      - run: cargo build --workspace\n      - uses: actions/download-artifact@v4\n"
    ))
);

#[test]
fn qa_m0_52_shards_run_from_the_archive() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_shards_run_from_the_archive,
    "the checked-in workflow with the shards back on `cargo nextest run --workspace`, compiling their tests",
    expected = "runs from no archive",
    check_archive(&edited(
        &the_workflow(),
        "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst --extract-to . --extract-overwrite --partition",
        "run: cargo nextest run --workspace --partition"
    ))
);

#[test]
fn qa_m0_52_shards_download_the_uploaded_archive() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_shards_download_the_uploaded_archive,
    "the checked-in workflow with the shards downloading an artifact the archive job never uploads",
    expected = "downloads no artifact `nextest-ci` there",
    check_archive(&edited(
        &the_workflow(),
        "      - uses: actions/download-artifact@v4\n        with:\n          name: nextest-ci\n",
        "      - uses: actions/download-artifact@v4\n        with:\n          name: nextest-other\n"
    ))
);

#[test]
fn qa_m0_52_the_archive_is_built_in_one_job() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_the_archive_is_built_in_one_job,
    "the checked-in workflow with `ci-workspace` building a nextest archive of its own as well",
    expected = "the dev archives are built in 2 jobs",
    check_archive(&edited(
        &the_workflow(),
        "        run: cargo nextest run -p xtask\n",
        "        run: cargo nextest archive -p xtask --archive-file xtask.tar.zst\n      - run: cargo nextest run -p xtask\n"
    ))
);

#[test]
fn qa_m0_52_one_archive_per_shard_feature_set() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_one_archive_per_shard_feature_set,
    "the checked-in workflow with a second archive, of a feature set no shard runs, built beside the first",
    expected = "the archives are not one per feature set the shards run",
    check_archive(&edited(
        &the_workflow(),
        "        run: cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst\n",
        "        run: cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst\n      - run: cargo nextest archive --workspace --features controls --archive-file $RUNNER_TEMP/nextest-controls.tar.zst\n"
    ))
);

#[test]
fn qa_m0_52_doctests_run_in_shard_1() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_doctests_run_in_shard_1,
    "the checked-in workflow with the doctest step run in every shard",
    expected = "runs in some shard other than shard 1 alone",
    check_archive(&edited(
        &the_workflow(),
        "        if: matrix.shard == 1\n        run: cargo test --workspace --doc\n",
        "        run: cargo test --workspace --doc\n"
    ))
);

#[test]
fn qa_m0_52_the_archive_has_the_shards_profile() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_the_archive_has_the_shards_profile,
    "the checked-in workflow with the archive built under nextest's default profile, not the shards' `ci`",
    expected = "not built under the shards' nextest profile",
    check_archive(&edited(
        &the_workflow(),
        "          NEXTEST_PROFILE: ci\n          RUSTFLAGS:",
        "          RUSTFLAGS:"
    ))
);

#[test]
fn qa_m0_52_each_cache_names_its_job() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_each_cache_names_its_job,
    "the checked-in workflow with the archive job sharing the `ci` shards' cache key",
    expected = "job `ci-archive`'s cache key does not name its job",
    check_archive(&edited(
        &the_workflow(),
        "shared-key: ci-archive\n",
        "shared-key: ci\n"
    ))
);

#[test]
fn qa_m0_52_the_release_archive_is_built_in_one_job() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_the_release_archive_is_built_in_one_job,
    "the checked-in workflow with `ci-archive` building a release archive of its own beside `ci-release-archive`'s",
    expected = "the release archives are built in 2 jobs",
    check_archive(&edited(
        &the_workflow(),
        "        run: cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst\n",
        "        run: cargo nextest archive --workspace --archive-file $RUNNER_TEMP/nextest-ci.tar.zst\n      - run: cargo nextest archive --workspace --release --archive-file $RUNNER_TEMP/nextest-r2.tar.zst\n"
    ))
);

#[test]
fn qa_m0_52_each_archive_is_built_once() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_each_archive_is_built_once,
    "the checked-in workflow with `ci-release-archive` building its archive twice",
    expected = "the archive `$RUNNER_TEMP/nextest-release.tar.zst` is built more than once",
    check_archive(&edited(
        &the_workflow(),
        "        run: cargo nextest archive --workspace --release --archive-file $RUNNER_TEMP/nextest-release.tar.zst\n",
        "        run: cargo nextest archive --workspace --release --archive-file $RUNNER_TEMP/nextest-release.tar.zst\n      - run: cargo nextest archive --workspace --release --archive-file $RUNNER_TEMP/nextest-release.tar.zst\n"
    ))
);

#[test]
fn qa_m0_52_release_shards_wait_on_their_archive_job() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_release_shards_wait_on_their_archive_job,
    "the checked-in workflow with `ci-release`'s `needs:` dropped, so a release shard can start before its archive exists",
    expected = "job `ci-release` does not wait (`needs:`) on `ci-release-archive`",
    check_archive(&edited(
        &the_workflow(),
        "  ci-release:\n    needs: ci-release-archive\n",
        "  ci-release:\n"
    ))
);

#[test]
fn qa_m0_52_release_shards_run_from_the_release_archive() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_release_shards_run_from_the_release_archive,
    "the checked-in workflow with `ci-release`'s shards running from (and waiting on) the dev archive",
    expected = "the shard jobs do not each run from the archive of the build profile REQ-SYS-078 names",
    check_archive(&edited(
        &edited(
            &edited(
                &the_workflow(),
                "  ci-release:\n    needs: ci-release-archive\n",
                "  ci-release:\n    needs: ci-archive\n"
            ),
            "          name: nextest-release\n          path: ${{ runner.temp }}\n",
            "          name: nextest-ci\n          path: ${{ runner.temp }}\n"
        ),
        "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-release.tar.zst",
        "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-ci.tar.zst"
    ))
);

#[test]
fn qa_m0_52_release_shards_compile_nothing() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_release_shards_compile_nothing,
    "the checked-in workflow with `ci-release`'s shards back on `cargo nextest run --workspace --release`",
    expected = "a `ci-release` shard compiles its tests",
    check_archive(&edited(
        &the_workflow(),
        "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-release.tar.zst --extract-to . --extract-overwrite",
        "run: cargo nextest run --workspace --release"
    ))
);

#[test]
fn qa_m0_52_release_doctests_at_release_in_shard_1() {
    check_archive(&the_workflow());
}

negative_control!(
    qa_m0_52_release_doctests_at_release_in_shard_1,
    "the checked-in workflow with `ci-release`'s doctests at the dev profile",
    expected = "the workspace's doctests do not run, once, in shard 1 of `ci-release`, at its build profile release",
    check_archive(&edited(
        &the_workflow(),
        "        run: cargo test --workspace --release --no-fail-fast --doc\n",
        "        run: cargo test --workspace --no-fail-fast --doc\n"
    ))
);

// ---- nextest, from an archive ---------------------------------------------------------------------------------------

/// Writes a fixture crate with unit tests and an integration test into `dir`, each file written only when it changes.
fn fixture(dir: &Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    let write = |path: PathBuf, text: String| {
        if fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            fs::write(path, text).unwrap();
        }
    };
    write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"qa_m0_52_archive\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n".to_owned(),
    );
    write(
        dir.join("src/lib.rs"),
        (0..24)
            .map(|i| format!("#[test]\nfn u{i:02}() {{}}\n"))
            .collect(),
    );
    write(
        dir.join("tests/it.rs"),
        (0..12)
            .map(|i| format!("#[test]\nfn i{i:02}() {{}}\n"))
            .collect(),
    );
}

/// `cargo nextest <args>` in `dir`, with its build directory `target`; its stdout and stderr; it must pass.
///
/// Its output, which the checks parse, must not depend on the environment the test inherits: every `CARGO_TERM_*`
/// variable (CI's `CARGO_TERM_COLOR: always` puts escape codes in it, a `CARGO_TERM_QUIET` would hide the
/// `Compiling` lines) and every `NEXTEST_*` one (a profile, status levels, the variables of the nextest run this test
/// is in) is removed, and colour is off, by `--color never` and `CARGO_TERM_COLOR=never`.
fn nextest(dir: &Path, args: &[&str]) -> (String, String) {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    for (name, _) in std::env::vars_os() {
        let n = name.to_string_lossy();
        if n.starts_with("CARGO_TERM_") || n.starts_with("NEXTEST_") {
            cmd.env_remove(&name);
        }
    }
    let o = cmd
        .current_dir(dir)
        .arg("nextest")
        .args(args)
        .args(["--color", "never"])
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .env("CARGO_TERM_COLOR", "never")
        .timed_output()
        .expect("run cargo nextest");
    let (out, err) = (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    );
    assert!(
        o.status.success(),
        "cargo nextest {args:?} failed:\n{out}\n{err}"
    );
    (out, err)
}

/// The tests a oneline listing names, each as `<binary> <test>`.
fn listed(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// R-372 on nextest: the fixture's tests listed from its source build; then one archive of them, the build directory
/// removed, and each shard of `shards` (of 4, `hash:`) listed and run from that archive (from the source instead
/// when `from_archive` is false). The shards' lists must hold every test the source build lists, each once, and no
/// shard's run may compile anything or recreate the build directory.
fn check_archive_shards(case: &str, shards: &[u32], from_archive: bool) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m0_52_{case}"));
    fixture(&dir);
    let all: BTreeSet<String> = listed(&nextest(&dir, &["list", "--message-format", "oneline"]).0)
        .into_iter()
        .collect();
    assert_eq!(all.len(), 36, "the fixture lists {all:?}");
    let archive = dir.join("archive.tar.zst");
    let archive_s = archive.to_string_lossy().into_owned();
    nextest(&dir, &["archive", "--archive-file", &archive_s]);
    fs::remove_dir_all(dir.join("target")).unwrap();
    let extract = dir.join("extract");
    let _ = fs::remove_dir_all(&extract);
    fs::create_dir_all(&extract).unwrap();
    let extract_s = extract.to_string_lossy().into_owned();
    let reuse: Vec<&str> = if from_archive {
        vec![
            "--archive-file",
            &archive_s,
            "--extract-to",
            &extract_s,
            "--extract-overwrite",
        ]
    } else {
        Vec::new()
    };
    let mut seen: Vec<String> = Vec::new();
    let mut ran = 0;
    for k in shards {
        let part = format!("hash:{k}/4");
        let mut list = vec!["list", "--message-format", "oneline", "--partition", &part];
        list.extend(&reuse);
        let (out, err) = nextest(&dir, &list);
        seen.extend(listed(&out));
        let mut run = vec!["run", "--partition", &part];
        run.extend(&reuse);
        let (_, err_run) = nextest(&dir, &run);
        ran += err_run
            .lines()
            .find_map(|l| {
                let (_, after) = l.split_once("Summary")?;
                let words: Vec<&str> = after.split_whitespace().collect();
                let at = words.iter().position(|w| *w == "run:")?;
                words.get(at.checked_sub(2)?)?.parse::<usize>().ok()
            })
            .unwrap_or_else(|| panic!("shard {k} ran no test summary:\n{err_run}"));
        for e in [&err, &err_run] {
            assert!(
                !e.lines().any(|l| l.trim_start().starts_with("Compiling")),
                "shard {k} compiled, though it runs from an archive:\n{e}"
            );
        }
        assert!(
            !dir.join("target").exists(),
            "shard {k} built into the build directory, though it runs from an archive"
        );
    }
    assert_eq!(
        ran,
        seen.len(),
        "the shards ran {ran} tests but list {}",
        seen.len()
    );
    let union: BTreeSet<String> = seen.iter().cloned().collect();
    assert_eq!(union.len(), seen.len(), "a test is in two shards: {seen:?}");
    assert_eq!(
        union, all,
        "the shards together do not list every test the source build lists"
    );
}

#[test]
fn qa_m0_52_archive_shards_list_every_test_once() {
    check_archive_shards("union", &[1, 2, 3, 4], true);
}

negative_control!(
    qa_m0_52_archive_shards_list_every_test_once,
    "shards 1 to 3 of 4 alone, so the tests hashed to shard 4 are listed by none",
    expected = "the shards together do not list every test",
    check_archive_shards("union_ctl", &[1, 2, 3], true)
);

#[test]
fn qa_m0_52_archive_shards_compile_nothing() {
    check_archive_shards("compile", &[1, 2, 3, 4], true);
}

negative_control!(
    qa_m0_52_archive_shards_compile_nothing,
    "a shard run from the source rather than the archive, which rebuilds the tests",
    expected = "compiled, though it runs from an archive",
    check_archive_shards("compile_ctl", &[1, 2, 3, 4], false)
);
