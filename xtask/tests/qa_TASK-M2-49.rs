//! qa's tests for TASK-M2-49, on the checked-in configuration, written from two requirements (R-547).
//!
//! REQ-TOOL-188: "the native build must set `-Zmir-enable-passes=-GVN`, scoped to the host target and never applied to
//! the SPIR-V build of the kernel"; verify: "`.cargo/config.toml` carries -Zmir-enable-passes=-GVN only under
//! `[target.'cfg(not(target_arch = \"spirv\"))']` (controls: the flag added to `[build]`, the host table emptied
//! [...])".
//! - `qa_m249_config_scopes_the_flag_to_the_host`: every `rustflags` the config sets anywhere is walked; the host table
//!   carries the flag, and no other `rustflags` (`[build]`'s, another target table's, a cfg that matches SPIR-V too)
//!   names `mir-enable-passes`. Controls: the flag in `[build]`, in a `[target.spirv-unknown-vulkan1.2]` table, in a
//!   `cfg(all())` table, and the host table emptied.
//! - `qa_m249_workflow_rustflags_keep_the_flag`: an environment RUSTFLAGS overrides the config's rustflags, so a
//!   workflow step that sets RUSTFLAGS (or CARGO_ENCODED_RUSTFLAGS, CARGO_BUILD_RUSTFLAGS) for a native build would
//!   drop the flag; every such setting in every workflow names it. Control: a RUSTFLAGS without it.
//!
//! REQ-TOOL-189: "CI must run the native tests at the release profile, in a job of its own on every push"; verify:
//! "the workflow has a job running the workspace's native tests with --release on the host target and the
//! workspace's flags".
//! - `qa_m249_ci_runs_the_native_tests_at_release`: `ci.yml` runs on every push; a step builds the whole workspace's
//!   tests with `--release`, for no `--target` (the host's), with no environment rustflags, no opt-level override
//!   and nothing excluded; those tests are run (from the step's archive, if it builds one), under a nextest profile
//!   that does not exclude the kernel's tests; the jobs doing so are not conditional or allowed to fail, and build
//!   nothing at the dev profile (a job of their own). Controls: `--release` dropped, a RUSTFLAGS set on the step, a
//!   `--target` added, the job made conditional, the release archive never run.
//!
//! Each test registers a negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `text` with `from` replaced by `to` once, `from` required to be there (the controls' edits).
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "no {from:?} to edit");
    text.replacen(from, to, 1)
}

// ---- REQ-TOOL-188: the config ---------------------------------------------------------------------------------------

/// The host table R-547 scopes the flag to.
const HOST: &str = r#"cfg(not(target_arch = "spirv"))"#;

/// Every `rustflags` array in a cargo config, with the dotted path of the table holding it.
fn rustflags(item: &toml_edit::Item, path: &str, out: &mut Vec<(String, Vec<String>)>) {
    let Some(table) = item.as_table_like() else {
        return;
    };
    for (k, v) in table.iter() {
        let here = if path.is_empty() {
            k.to_owned()
        } else {
            format!("{path}.{k}")
        };
        if k == "rustflags" {
            let flags = match v.as_array() {
                Some(a) => a
                    .iter()
                    .map(|f| f.as_str().unwrap_or("").to_owned())
                    .collect(),
                None => v
                    .as_str()
                    .map(|s| s.split_whitespace().map(str::to_owned).collect())
                    .unwrap_or_default(),
            };
            out.push((path.to_owned(), flags));
        } else {
            rustflags(v, &here, out);
        }
    }
}

/// REQ-TOOL-188 on a `.cargo/config.toml`'s text.
fn check_config(config: &str) {
    let doc: toml_edit::DocumentMut = config.parse().expect("the config parses");
    let mut found = Vec::new();
    rustflags(doc.as_item(), "", &mut found);
    let host = format!("target.{HOST}");
    let host_flags: Vec<&Vec<String>> = found
        .iter()
        .filter(|(p, _)| *p == host)
        .map(|(_, f)| f)
        .collect();
    assert!(
        host_flags
            .iter()
            .any(|f| f.iter().any(|x| x == "-Zmir-enable-passes=-GVN")),
        "the host table `[target.'{HOST}']` does not carry -Zmir-enable-passes=-GVN: {found:?}"
    );
    for (path, flags) in &found {
        if *path == host {
            continue;
        }
        assert!(
            !flags.iter().any(|f| f.contains("mir-enable-passes")),
            "`[{path}]` rustflags name mir-enable-passes, which reaches a build other than the host's: {flags:?}"
        );
    }
}

#[test]
fn qa_m249_config_scopes_the_flag_to_the_host() {
    check_config(&read(".cargo/config.toml"));
}

negative_control!(
    qa_m249_config_scopes_the_flag_to_the_host,
    "the workspace's config with the flag in [build] rustflags too, which every target takes",
    expected = "`[build]` rustflags name mir-enable-passes",
    check_config(&format!(
        "{}\n[build]\nrustflags = [\"-Zmir-enable-passes=-GVN\"]\n",
        read(".cargo/config.toml")
    ))
);

#[test]
fn qa_m249_config_no_spirv_table_flag() {
    check_config(&read(".cargo/config.toml"));
}

negative_control!(
    qa_m249_config_no_spirv_table_flag,
    "the workspace's config with the flag in a table for the SPIR-V target",
    expected = "`[target.spirv-unknown-vulkan1.2]` rustflags name mir-enable-passes",
    check_config(&format!(
        "{}\n[target.'spirv-unknown-vulkan1.2']\nrustflags = [\"-Zmir-enable-passes=-GVN\"]\n",
        read(".cargo/config.toml")
    ))
);

#[test]
fn qa_m249_config_no_catch_all_cfg_flag() {
    check_config(&read(".cargo/config.toml"));
}

negative_control!(
    qa_m249_config_no_catch_all_cfg_flag,
    "the workspace's config with the flag in a cfg table that matches SPIR-V too",
    expected = "`[target.cfg(all())]` rustflags name mir-enable-passes",
    check_config(&format!(
        "{}\n[target.'cfg(all())']\nrustflags = [\"-Z\", \"mir-enable-passes=-GVN\"]\n",
        read(".cargo/config.toml")
    ))
);

#[test]
fn qa_m249_config_host_table_carries_the_flag() {
    check_config(&read(".cargo/config.toml"));
}

negative_control!(
    qa_m249_config_host_table_carries_the_flag,
    "the workspace's config with the host table's flag removed",
    expected = "does not carry -Zmir-enable-passes=-GVN",
    check_config(&edited(
        &read(".cargo/config.toml"),
        "\"-Zmir-enable-passes=-GVN\"",
        ""
    ))
);

// ---- REQ-TOOL-188: workflow rustflags -------------------------------------------------------------------------------

/// The environment variables that override the config's rustflags for a build.
const OVERRIDES: &[&str] = &[
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
];

/// The workflows, by file name, as checked in.
fn workflows() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// Each workflow line that sets a rustflags override (as `NAME:` in an env block, or `NAME=` in a command) names
/// `-Zmir-enable-passes=-GVN`.
fn check_workflow_rustflags(workflows: &[(String, String)]) {
    let mut seen = 0;
    for (name, text) in workflows {
        for (i, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            for var in OVERRIDES {
                let sets = t.split_whitespace().any(|w| {
                    w.starts_with(&format!("{var}=")) || w.starts_with(&format!("{var}:"))
                });
                if sets {
                    seen += 1;
                    assert!(
                        t.contains("-Zmir-enable-passes=-GVN"),
                        "{name}:{} sets {var} without the native build's -Zmir-enable-passes=-GVN: {t}",
                        i + 1
                    );
                }
            }
        }
    }
    eprintln!("rustflags overrides set in the workflows: {seen}");
}

#[test]
fn qa_m249_workflow_rustflags_keep_the_flag() {
    check_workflow_rustflags(&workflows());
}

negative_control!(
    qa_m249_workflow_rustflags_keep_the_flag,
    "ci.yml with the archive step's RUSTFLAGS back to the debug-section compression alone",
    expected = "sets RUSTFLAGS without the native build's -Zmir-enable-passes=-GVN",
    {
        let mut w = workflows();
        let ci = w.iter_mut().find(|(n, _)| n == "ci.yml").unwrap();
        ci.1 = edited(
            &ci.1,
            "RUSTFLAGS: -C link-arg=-Wl,--compress-debug-sections=zlib -Zmir-enable-passes=-GVN",
            "RUSTFLAGS: -C link-arg=-Wl,--compress-debug-sections=zlib",
        );
        check_workflow_rustflags(&w)
    }
);

// ---- REQ-TOOL-189: the release-profile job --------------------------------------------------------------------------

/// A job of a workflow: its id and its non-comment lines.
struct Job {
    id: String,
    lines: Vec<String>,
}

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
        if in_jobs && line.starts_with("  ") && !line.starts_with("   ") && t.ends_with(':') {
            found.push(Job {
                id: t.trim_end_matches(':').to_owned(),
                lines: Vec::new(),
            });
        } else if in_jobs {
            if let Some(j) = found.last_mut() {
                j.lines.push(line.to_owned());
            }
        }
    }
    found
}

/// The lines of `workflow` outside `jobs:` (the workflow-level keys, `on:` and `env:` among them).
fn top_level(workflow: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') {
            in_jobs = t == "jobs:";
        }
        if !in_jobs {
            out.push(line.to_owned());
        }
    }
    out
}

impl Job {
    /// The job's steps (each from a `- ` at indent 6), each its lines.
    fn steps(&self) -> Vec<Vec<String>> {
        let mut steps: Vec<Vec<String>> = Vec::new();
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

    /// The job's own lines (indent 4) and its job-level `env:` block: everything outside its steps.
    fn own_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut in_steps = false;
        for l in &self.lines {
            let indent = l.len() - l.trim_start().len();
            if indent == 4 {
                in_steps = l.trim() == "steps:";
            }
            if !in_steps {
                out.push(l.clone());
            }
        }
        out
    }
}

/// Every command of a step's `run:` (one line, or each line of a block).
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

/// The value of `name:` among `lines`, trimmed.
fn key(lines: &[String], name: &str) -> Option<String> {
    lines.iter().find_map(|l| {
        l.trim()
            .trim_start_matches("- ")
            .strip_prefix(name)
            .and_then(|r| r.strip_prefix(':'))
            .map(|v| v.trim().to_owned())
    })
}

fn opt(words: &[&str], opt: &str) -> Option<String> {
    words
        .iter()
        .position(|w| *w == opt)
        .and_then(|i| words.get(i + 1))
        .map(|v| (*v).to_owned())
}

/// A cargo command that builds tests: `cargo test`, `cargo nextest run` from source, `cargo nextest archive`.
fn builds_tests(c: &str) -> bool {
    let from_archive = c.contains("--archive-file") && c.starts_with("cargo nextest run");
    (c.starts_with("cargo test")
        || c.starts_with("cargo nextest run")
        || c.starts_with("cargo nextest archive"))
        && !from_archive
}

fn at_release(words: &[&str]) -> bool {
    words.contains(&"--release") || opt(words, "--cargo-profile").as_deref() == Some("release")
}

/// A nextest profile's `default-filter` in `.config/nextest.toml` (none for the default profile).
fn nextest_filter(nextest: &str, profile: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = nextest.parse().expect(".config/nextest.toml parses");
    doc.get("profile")?
        .get(profile)?
        .get("default-filter")?
        .as_str()
        .map(str::to_owned)
}

/// REQ-TOOL-189 on `ci.yml`'s text and `.config/nextest.toml`'s.
fn check_release_job(workflow: &str, nextest: &str) {
    // On every push: `on:` names `push` with no filter beneath it, or, under R-656, with only a `branches:` filter that
    // names `main` (a pull request's pushes run through `pull_request`).
    let top = top_level(workflow);
    let push = top
        .iter()
        .position(|l| l.trim() == "push:" || l.trim() == "push: {}")
        .expect("ci.yml does not run on push");
    let push_indent = top[push].len() - top[push].trim_start().len();
    let deeper = |at: usize| {
        top.get(at)
            .is_some_and(|l| l.len() - l.trim_start().len() > push_indent)
    };
    let filtered = if deeper(push + 1) { push + 2 } else { push + 1 };
    if filtered == push + 2 {
        let branches = top[push + 1].trim().strip_prefix("branches:").map(|b| {
            b.trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|n| n.trim().trim_matches('"').to_owned())
                .collect::<Vec<_>>()
        });
        assert!(
            branches.is_some_and(|b| b.iter().any(|n| n == "main")),
            "ci.yml's push trigger is filtered: {:?}",
            top.get(push + 1)
        );
    }
    assert!(
        !deeper(filtered),
        "ci.yml's push trigger is filtered: {:?}",
        top.get(filtered)
    );
    let workflow_env: Vec<String> = top.clone();

    let jobs = jobs(workflow);
    let mut release_jobs: Vec<&str> = Vec::new();
    let mut release_archives: Vec<(String, String)> = Vec::new();
    let mut direct_release_run = false;
    for job in &jobs {
        for step in job.steps() {
            for c in commands(&step) {
                let words: Vec<&str> = c.split_whitespace().collect();
                if !(builds_tests(&c) && at_release(&words) && !words.contains(&"--doc")) {
                    continue;
                }
                assert!(
                    words.contains(&"--workspace"),
                    "job `{}` builds its release tests for less than the workspace: `{c}`",
                    job.id
                );
                for w in ["--exclude", "--target", "-p", "--package"] {
                    assert!(
                        !words
                            .iter()
                            .any(|x| *x == w || x.starts_with(&format!("{w}="))),
                        "job `{}`'s release tests are built with `{w}`: `{c}`",
                        job.id
                    );
                }
                assert!(
                    !words.iter().any(|w| w.contains("opt-level")),
                    "job `{}` overrides the release profile's opt-level: `{c}`",
                    job.id
                );
                // The workspace's flags: nothing in the step's, the job's or the workflow's env overrides the config.
                let envs: Vec<String> = step
                    .iter()
                    .chain(job.own_lines().iter())
                    .chain(workflow_env.iter())
                    .cloned()
                    .collect();
                for var in OVERRIDES
                    .iter()
                    .copied()
                    .chain(["CARGO_PROFILE_RELEASE_OPT_LEVEL", "RUSTC_WRAPPER"])
                {
                    assert!(
                        key(&envs, var).is_none(),
                        "job `{}`'s release tests are built with {var} set, not with the workspace's flags",
                        job.id
                    );
                }
                let own = job.own_lines();
                // R-656: a docs-only pull request skips the release jobs; no other condition does.
                assert!(
                    key(&own, "if")
                        .is_none_or(|c| c == "needs.changes.outputs.docs_only != 'true'"),
                    "the release job `{}` runs only under a condition",
                    job.id
                );
                assert!(
                    key(&own, "continue-on-error").is_none(),
                    "the release job `{}` may fail without failing the run",
                    job.id
                );
                release_jobs.push(job.id.as_str());
                if c.starts_with("cargo nextest archive") {
                    let file = opt(&words, "--archive-file").expect("an archive file");
                    release_archives.push((job.id.clone(), file));
                } else {
                    direct_release_run = true;
                    let profile = key(&step, "NEXTEST_PROFILE");
                    check_profile(nextest, profile.as_deref(), &job.id);
                }
            }
        }
    }
    assert!(
        !release_jobs.is_empty(),
        "no job of ci.yml builds the workspace's tests with --release"
    );

    // An archive's tests are run: some shard job runs from it, unconditionally, under a profile that keeps the kernel.
    for (archive_job, file) in &release_archives {
        let runners: Vec<&Job> = jobs
            .iter()
            .filter(|j| {
                j.steps().iter().flat_map(|s| commands(s)).any(|c| {
                    c.starts_with("cargo nextest run")
                        && opt(&c.split_whitespace().collect::<Vec<_>>(), "--archive-file")
                            .as_deref()
                            == Some(file.as_str())
                })
            })
            .collect();
        assert!(
            !runners.is_empty() || direct_release_run,
            "no job runs the release archive `{file}` that `{archive_job}` builds"
        );
        for r in runners {
            let own = r.own_lines();
            assert!(
                key(&own, "if").is_none() && key(&own, "continue-on-error").is_none(),
                "the job `{}` running the release tests is conditional or allowed to fail",
                r.id
            );
            for step in r.steps() {
                if commands(&step).iter().any(|c| c.contains(file.as_str())) {
                    check_profile(nextest, key(&step, "NEXTEST_PROFILE").as_deref(), &r.id);
                }
            }
            release_jobs.push(r.id.as_str());
        }
    }

    // A job of its own: the jobs that build or run the release tests build no test at another profile.
    for id in release_jobs {
        let job = jobs.iter().find(|j| j.id == id).unwrap();
        for c in job.steps().iter().flat_map(|s| commands(s)) {
            let words: Vec<&str> = c.split_whitespace().collect();
            if builds_tests(&c) || c.starts_with("cargo build") {
                assert!(
                    at_release(&words),
                    "the release job `{id}` also builds at another profile: `{c}`"
                );
            }
        }
    }
}

/// The nextest profile a release run uses does not exclude the kernel's tests (`stage_canon_native` among them, the
/// test RQ-299's miscompile failed): the default profile runs every test; another profile's filter is an exclusion
/// (`not (...)`) naming none of the kernel's binaries.
fn check_profile(nextest: &str, profile: Option<&str>, job: &str) {
    let Some(profile) = profile.filter(|p| *p != "default") else {
        return;
    };
    let Some(filter) = nextest_filter(nextest, profile) else {
        return;
    };
    let f = filter.trim();
    assert!(
        f.starts_with("not") && !f.contains("kernel::") && !f.contains("package(kernel") && !f.contains("package(=kernel"),
        "job `{job}`'s release run, under nextest profile `{profile}`, may leave out the kernel's tests: {f}"
    );
}

#[test]
fn qa_m249_ci_runs_the_native_tests_at_release() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_ci_runs_the_native_tests_at_release,
    "ci.yml with the release archive built at the dev profile",
    expected = "no job of ci.yml builds the workspace's tests with --release",
    check_release_job(
        &edited(
            &read(".github/workflows/ci.yml"),
            "run: cargo nextest archive --workspace --release ",
            "run: cargo nextest archive --workspace "
        )
        .replace("cargo test --workspace --release", "cargo test --workspace"),
        &read(".config/nextest.toml")
    )
);

#[test]
fn qa_m249_release_tests_take_the_workspace_flags() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_release_tests_take_the_workspace_flags,
    "ci.yml with a RUSTFLAGS on the release archive step, which overrides the config's host flags",
    expected = "release tests are built with RUSTFLAGS set",
    check_release_job(
        &edited(
            &read(".github/workflows/ci.yml"),
            "      - name: cargo nextest archive --workspace --release\n        env:\n",
            "      - name: cargo nextest archive --workspace --release\n        env:\n          RUSTFLAGS: -C debuginfo=0\n"
        ),
        &read(".config/nextest.toml")
    )
);

#[test]
fn qa_m249_release_tests_on_the_host_target() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_release_tests_on_the_host_target,
    "ci.yml with the release archive built for an explicit --target",
    expected = "release tests are built with `--target`",
    check_release_job(
        &edited(
            &read(".github/workflows/ci.yml"),
            "run: cargo nextest archive --workspace --release ",
            "run: cargo nextest archive --workspace --release --target x86_64-unknown-linux-musl "
        ),
        &read(".config/nextest.toml")
    )
);

#[test]
fn qa_m249_release_job_on_every_push() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_release_job_on_every_push,
    "ci.yml with the release archive job run only on main",
    expected = "runs only under a condition",
    check_release_job(
        &edited(
            &read(".github/workflows/ci.yml"),
            "  ci-release-archive:\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n",
            "  ci-release-archive:\n    needs: changes\n    if: github.ref == 'refs/heads/main'\n"
        ),
        &read(".config/nextest.toml")
    )
);

#[test]
fn qa_m249_release_archive_is_run() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_release_archive_is_run,
    "ci.yml with the release shards running another archive file than the one built",
    expected = "no job runs the release archive",
    check_release_job(
        &edited(
            &read(".github/workflows/ci.yml"),
            "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-release.tar.zst",
            "run: cargo nextest run --archive-file $RUNNER_TEMP/nextest-other.tar.zst"
        ),
        &read(".config/nextest.toml")
    )
);

#[test]
fn qa_m249_release_run_keeps_the_kernel() {
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &read(".config/nextest.toml"),
    );
}

negative_control!(
    qa_m249_release_run_keeps_the_kernel,
    "the `ci` nextest profile with the kernel's canon tests excluded",
    expected = "may leave out the kernel's tests",
    check_release_job(
        &read(".github/workflows/ci.yml"),
        &edited(
            &read(".config/nextest.toml"),
            "        binary_id(=validation::toolchain)\n        | binary_id(=validation::qa_TASK-M0-14)\n",
            "        binary_id(=validation::toolchain)\n        | binary_id(=kernel::canon)\n        | binary_id(=validation::qa_TASK-M0-14)\n"
        )
    )
);
