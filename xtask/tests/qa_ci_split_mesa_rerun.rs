//! qa's tests for the CI PR "split the ci job, cache Mesa, re-run install-only failures once" (PR #495), written from
//! the owner's words of 11 Nov 2026 and the task's items, not from the code:
//!
//! - "Split the ci job as planned": three shards under one aggregate check that keeps the name `ci`; "the aggregate fails
//!   if any shard fails or is missing"; "same tests, partitioned".
//!   `qa_split_ci_fails_unless_every_shard_passed`: `ci.yml`'s job reporting `ci`, its condition and its steps run under
//!   bash on every result a shard's matrix can end in. `qa_split_every_test_in_exactly_one_shard`: every test command of
//!   the shards is partitioned over exactly the shards there are, and the doctests, which nextest cannot partition, run
//!   in exactly one shard.
//! - "Cache the Mesa packages for gpu-lavapipe and the release jobs, keyed on the package list".
//!   `qa_mesa_key_follows_the_package_list`: the action's own key step, run under bash: the key changes with the package
//!   list and the runner image, and there is none off a GitHub-hosted image; every apt step installing Mesa in every
//!   workflow has the cache's step before it, naming the very packages it installs, and skips only on its output.
//!   `qa_mesa_cache_install_and_rerun_mark`: the install step on a miss, a hit and a hit that does not install, and a
//!   re-run marked in the job's summary.
//! - "one automatic re-run, on GitHub-hosted runners, for a job whose only failure is the package-install cap … it is
//!   marked as a re-run and never turns a real failure green"; the task: "once per job per run", "any failure outside the
//!   install step, or a second install failure, stands. Never on the self-hosted Windows runner".
//!   `qa_rerun_install_only_once_hosted_never_real`: `ci_retry.py decide`, its whole path, against a stand-in `gh` that
//!   serves the run's attempts from files: what it asks GitHub to re-run, and what its summary says.
//! - Retired with the behaviour it tested: "per-merge's task filing failed with gh: Validation Failed (HTTP 422) … fix
//!   it, so a task verdict always files (or finds) its issue" (`qa_per_merge_files_without_remaking_its_label`). R-982
//!   (12 Nov 2026) makes any failed job of the run red, so the verdict files no task and `file_task` is gone;
//!   `qa_ci_per_merge.rs`'s `qa_per_merge_decide_fails_unless_green_and_files_nothing` checks that `decide` files nothing.
//!
//! Each test has its negative control (R-176); a control edits the workflow, the script or the run it reads.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_split: {rel}: {e}"))
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_split: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_split_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("wf.yml");
    std::fs::write(&file, yml).expect("written");
    let out = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa_split: python3 ran");
    assert!(
        out.status.success(),
        "qa_split: not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_split: JSON")
}

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn needs_of(job: &Value) -> Vec<String> {
    match job.get("needs") {
        Some(Value::Array(a)) => a.iter().map(s).collect(),
        Some(one) => vec![s(one)],
        None => Vec::new(),
    }
}

/// `expr` inside `${{ }}` or not, as the bare expression.
fn bare(expr: &str) -> String {
    let t = expr.trim();
    t.strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .map_or(t, str::trim)
        .to_owned()
}

/// The `&&` clauses of a condition, trimmed.
fn clauses(cond: &str) -> Vec<String> {
    bare(cond)
        .split("&&")
        .map(|c| c.trim().to_owned())
        .collect()
}

/// The job reporting the required check `ci` (exactly one), and the shards' job it waits on beside `changes` (exactly
/// one, a matrix over `shard`): their ids.
fn ci_and_shards(yml: &Value) -> (String, String) {
    let jobs = yml["jobs"].as_object().expect("qa_split: jobs");
    let bearing: Vec<&String> = jobs
        .iter()
        .filter(|(_, j)| s(&j["name"]) == "ci")
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        bearing.len(),
        1,
        "qa_split: `ci` is reported by {bearing:?}"
    );
    let id = bearing[0].clone();
    let others: Vec<String> = needs_of(&jobs[&id])
        .into_iter()
        .filter(|n| n != "changes")
        .collect();
    assert_eq!(
        others.len(),
        1,
        "qa_split: `ci` ({id}) waits on {others:?}, not on one shards' job"
    );
    let shards = others[0].clone();
    assert!(
        jobs[&shards]["strategy"]["matrix"].get("shard").is_some(),
        "qa_split: `{shards}`, which `ci` waits on, is no matrix of shards"
    );
    (id, shards)
}

// ---- The split: the aggregate `ci` ----------------------------------------------------------------------------------

/// What a matrix job's `needs.<id>.result` can be, as GitHub documents it: success only when every leg succeeded.
const SHARD_RESULTS: [&str; 4] = ["success", "failure", "cancelled", "skipped"];

/// Runs every step of `job` under bash, each `${{ needs.<id>.result }}` in a step's env read from `results`; any other
/// expression stops the test. Whether every step passed.
fn job_passes(job: &Value, results: &BTreeMap<&str, &str>) -> bool {
    for step in job["steps"].as_array().expect("qa_split: steps") {
        assert!(
            step.get("uses").is_none(),
            "qa_split: the job bearing `ci` runs an action, so it is no verdict alone: {step}"
        );
        assert!(
            step.get("if").is_none(),
            "qa_split: a step of the job bearing `ci` can be skipped: {step}"
        );
        let mut cmd = Command::new("bash");
        cmd.args(["-e", "-c", &s(&step["run"])]);
        if let Some(env) = step["env"].as_object() {
            for (k, v) in env {
                let e = bare(&s(v));
                let parts: Vec<&str> = e.split('.').collect();
                let value = match parts.as_slice() {
                    ["needs", id, "result"] => results.get(id).copied().unwrap_or(""),
                    _ => panic!(
                        "qa_split: the step's env reads `{e}`, which this test does not know"
                    ),
                };
                cmd.env(k, value);
            }
        }
        let out = cmd.timed_output().expect("qa_split: bash ran");
        if !out.status.success() {
            return false;
        }
    }
    true
}

fn check_aggregate(ci: &str) {
    let yml = parse(ci);
    let jobs = yml["jobs"].as_object().expect("qa_split: jobs");
    // Exactly one job reports `ci`, and it is not a shard: a shard's own name is never the required check's.
    let (id, shard_id) = ci_and_shards(&yml);
    let id = id.as_str();
    let job = &jobs[id];
    let shard_job = &jobs[&shard_id];
    assert!(
        s(&shard_job["name"]).contains("${{ matrix.shard }}"),
        "qa_split: the shards share one check name"
    );
    // It waits for the shards and for the classification.
    let needs = needs_of(job);
    assert!(
        needs.contains(&shard_id) && needs.contains(&"changes".to_owned()),
        "qa_split: `ci` ({id}) does not wait for the shards and the classification: {needs:?}"
    );
    // It runs whatever its needs ended in: a job skipped when a shard failed would report a skip, which branch
    // protection reads as passing.
    let cond = s(&job["if"]);
    assert!(
        clauses(&cond)
            .iter()
            .any(|c| c == "!cancelled()" || c == "always()"),
        "qa_split: `ci` is skipped when a shard fails: `if: {cond}`"
    );
    // Its steps, on every end each of its needs can have: it passes only when the classification and every shard did.
    for changes in SHARD_RESULTS {
        for shards in SHARD_RESULTS.iter().chain(&[""]) {
            let results: BTreeMap<&str, &str> =
                [("changes", changes), (shard_id.as_str(), *shards)].into();
            let passes = job_passes(job, &results);
            let want = changes == "success" && *shards == "success";
            assert_eq!(
                passes, want,
                "qa_split: `ci` passes={passes} on a classification that ended {changes} and shards that ended `{shards}`"
            );
        }
    }
}

#[test]
fn qa_split_ci_fails_unless_every_shard_passed() {
    check_aggregate(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa_split_ci_fails_unless_every_shard_passed,
    "ci.yml's `ci` passing on a shard that failed",
    expected =
        "`ci` passes=true on a classification that ended success and shards that ended `failure`",
    check_aggregate(&swap(
        &read(".github/workflows/ci.yml"),
        "'[ \"$SHARDS\" = success ] ||",
        "'[ \"$SHARDS\" != cancelled ] ||"
    ))
);

mod verdict_waits_on_success {
    use super::*;
    negative_control!(
        qa_split_ci_fails_unless_every_shard_passed,
        "ci.yml's `ci` running only when every shard passed, so skipped (green) when one failed",
        expected = "`ci` is skipped when a shard fails",
        check_aggregate(&swap(
            &read(".github/workflows/ci.yml"),
            "    needs: [changes, tests]\n    if: ${{ !cancelled() && github.event_name == 'pull_request' }}\n",
            "    needs: [changes, tests]\n    if: ${{ github.event_name == 'pull_request' }}\n"
        ))
    );
}

mod verdict_on_a_missing_shard {
    use super::*;
    negative_control!(
        qa_split_ci_fails_unless_every_shard_passed,
        "ci.yml's `ci` failing only on a shard that failed, passing one that never ran",
        expected = "shards that ended `skipped`",
        check_aggregate(&swap(
            &read(".github/workflows/ci.yml"),
            "'[ \"$SHARDS\" = success ] ||",
            "'[ \"$SHARDS\" != failure ] && [ \"$SHARDS\" != cancelled ] ||"
        ))
    );
}

// ---- The split: every test in exactly one shard ---------------------------------------------------------------------

fn check_partition(ci: &str) {
    let yml = parse(ci);
    let (_, id) = ci_and_shards(&yml);
    let job = &yml["jobs"][&id];
    let legs: Vec<u64> = job["strategy"]["matrix"]["shard"]
        .as_array()
        .expect("qa_split: the shard list")
        .iter()
        .map(|v| v.as_u64().expect("qa_split: a shard is a number"))
        .collect();
    let n = legs.len() as u64;
    assert!(n >= 2, "qa_split: {n} shard(s)");
    // nextest's `--partition` numbers its slices 1..=n: each leg one slice, every slice a leg.
    assert_eq!(
        legs,
        (1..=n).collect::<Vec<_>>(),
        "qa_split: the shards are not 1 to {n}"
    );
    let mut tests = 0;
    for step in job["steps"].as_array().expect("qa_split: steps") {
        let run = s(&step["run"]);
        let name = s(&step["name"]);
        if run.contains("cargo nextest run") {
            tests += 1;
            for line in run.lines().filter(|l| l.contains("cargo nextest run")) {
                // Exactly one `--partition <kind>:<the shard's own number>/<the number of shards>`.
                let at: Vec<usize> = line.match_indices("--partition").map(|(i, _)| i).collect();
                assert!(
                    at.len() == 1,
                    "qa_split: `{name}` runs without its shard's partition: {line}"
                );
                let rest = line[at[0] + "--partition".len()..].trim_start_matches([' ', '=']);
                let value = rest.split(" --").next().unwrap_or_default().trim();
                let slice = value
                    .strip_prefix("hash:")
                    .or_else(|| value.strip_prefix("count:"))
                    .unwrap_or_else(|| {
                        panic!("qa_split: `{name}`'s partition is no slice: {value}")
                    });
                let (k, of) = slice.rsplit_once('/').unwrap_or_else(|| {
                    panic!("qa_split: `{name}`'s partition has no count: {slice}")
                });
                assert_eq!(
                    k, "${{ matrix.shard }}",
                    "qa_split: `{name}`'s partition is not its shard's own"
                );
                assert_eq!(
                    of,
                    n.to_string(),
                    "qa_split: `{name}`'s partition's count is not the number of shards ({n})"
                );
            }
        } else if run.contains("cargo test") {
            // What nextest does not run (the doctests) runs in exactly one shard.
            let cond = s(&step["if"]);
            let legs_on: Vec<u64> = legs
                .iter()
                .copied()
                .filter(|leg| {
                    clauses(&cond).iter().all(|c| match c.split_once("==") {
                        Some((a, b)) if a.trim() == "matrix.shard" => b.trim() == leg.to_string(),
                        _ => !c.contains("matrix.shard"),
                    })
                })
                .collect();
            assert_eq!(
                legs_on.len(),
                1,
                "qa_split: `{name}` runs in shards {legs_on:?} of {id}'s {n}, not exactly one: `if: {cond}`"
            );
        }
    }
    assert!(tests >= 1, "qa_split: the shards run no nextest step");
}

#[test]
fn qa_split_every_test_in_exactly_one_shard() {
    check_partition(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa_split_every_test_in_exactly_one_shard,
    "ci.yml's product tests run whole in every shard",
    expected = "runs without its shard's partition",
    check_partition(&swap(
        &read(".github/workflows/ci.yml"),
        "run: cargo nextest run $PACKAGES --partition hash:${{ matrix.shard }}/3",
        "run: cargo nextest run $PACKAGES"
    ))
);

mod a_fourth_shard {
    use super::*;
    negative_control!(
        qa_split_every_test_in_exactly_one_shard,
        "ci.yml with a fourth shard while the partitions still count three",
        expected = "partition's count is not the number of shards",
        check_partition(&swap(
            &read(".github/workflows/ci.yml"),
            "        shard: [1, 2, 3]\n",
            "        shard: [1, 2, 3, 4]\n"
        ))
    );
}

mod doctests_everywhere {
    use super::*;
    negative_control!(
        qa_split_every_test_in_exactly_one_shard,
        "ci.yml's doctests in every shard",
        expected = "not exactly one",
        check_partition(&swap(
            &read(".github/workflows/ci.yml"),
            " && matrix.shard == 1\n",
            "\n"
        ))
    );
}

// ---- The Mesa cache --------------------------------------------------------------------------------------------------

const MESA: &str = ".github/actions/mesa-debs/mesa_debs.sh";

/// A runner's environment: GitHub-hosted, an image named.
const HOSTED: [(&str, &str); 3] = [
    ("RUNNER_OS", "Linux"),
    ("ImageOS", "ubuntu24"),
    ("ImageVersion", "20261109.1.0"),
];

/// `mesa_debs.sh` (its text `script`) run with `args` in `env` alone, PATH from `path` first: its `$GITHUB_OUTPUT` as
/// key=value, its step summary's text, and its exit status.
fn mesa(
    script: &str,
    args: &[&str],
    env: &[(&str, &str)],
    path: Option<&Path>,
    dir: &Path,
) -> (BTreeMap<String, String>, String, bool) {
    std::fs::create_dir_all(dir).expect("qa_split: scratch");
    let output = dir.join("output");
    let summary = dir.join("summary");
    let _ = std::fs::remove_file(&output);
    let _ = std::fs::remove_file(&summary);
    let file = dir.join("mesa_debs.sh");
    std::fs::write(&file, script).expect("qa_split: script written");
    let sys_path = std::env::var("PATH").unwrap_or_default();
    let full_path = match path {
        Some(p) => format!("{}:{sys_path}", p.display()),
        None => sys_path,
    };
    let out = Command::new("bash")
        .arg(&file)
        .args(args)
        .env_clear()
        .env("PATH", full_path)
        .env("HOME", dir)
        .env("GITHUB_OUTPUT", &output)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("RUNNER_TEMP", dir)
        .env("MESA_DEBS_DIR", dir.join("cached"))
        .env("MESA_APT_ARCHIVES", dir.join("archives"))
        .envs(env.iter().copied())
        .timed_output()
        .expect("qa_split: bash ran");
    let text = std::fs::read_to_string(&output).unwrap_or_default();
    let outputs = text
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    (
        outputs,
        std::fs::read_to_string(&summary).unwrap_or_default(),
        out.status.success(),
    )
}

fn key(script: &str, packages: &str, env: &[(&str, &str)]) -> Option<String> {
    let dir = Scratch::new("qa_split_mesa_key");
    let mut all: Vec<(&str, &str)> = env.to_vec();
    all.push(("PACKAGES", packages));
    let (out, _, ok) = mesa(script, &["key"], &all, None, &dir);
    assert!(ok, "qa_split: the key step failed");
    out.get("key").cloned()
}

/// The packages an apt step installs: the words after `apt-get install` that are no option.
fn apt_packages(run: &str) -> Vec<String> {
    let at = run
        .find("apt-get install")
        .expect("qa_split: no apt-get install");
    run[at + "apt-get install".len()..]
        .split([';', '&', '|', '\n'])
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .filter(|w| !w.starts_with('-'))
        .map(str::to_owned)
        .collect()
}

fn check_key(script: &str, workflows: &[(String, String)]) {
    let base =
        key(script, "mesa-vulkan-drivers", &HOSTED).expect("qa_split: no key on a hosted runner");
    // The same list, the same key: a warm run finds what an earlier one kept.
    assert_eq!(
        key(script, "mesa-vulkan-drivers", &HOSTED).as_deref(),
        Some(base.as_str()),
        "qa_split: the key is not stable"
    );
    // Any change to the list, a new key.
    for other in [
        "mesa-vulkan-drivers libvulkan1",
        "libvulkan1 mesa-vulkan-drivers",
        "mesa-vulkan-drivers-x",
        "mesa-opencl-icd",
    ] {
        let k = key(script, other, &HOSTED).expect("qa_split: no key");
        assert_ne!(
            k, base,
            "qa_split: the key does not change with the package list ({other})"
        );
    }
    // A new runner image, a new key: packages built for one image are not installed on another.
    let mut image = HOSTED;
    image[2] = ("ImageVersion", "20261116.1.0");
    assert_ne!(
        key(script, "mesa-vulkan-drivers", &image).expect("qa_split: no key"),
        base,
        "qa_split: the key does not change with the runner image"
    );
    // Off a GitHub-hosted image (the self-hosted PC names none), no key, so nothing is cached or restored.
    assert_eq!(
        key(script, "mesa-vulkan-drivers", &[("RUNNER_OS", "Windows")]),
        None,
        "qa_split: a runner that names no image has a key"
    );
    // Every apt step installing Mesa has the cache's step before it, with its id `mesa`, naming the very packages the
    // apt step installs, and the apt step skips on that step's `installed` alone.
    let mut seen = 0;
    for (file, text) in workflows {
        let yml = parse(text);
        let Some(jobs) = yml["jobs"].as_object() else {
            continue;
        };
        for (jid, job) in jobs {
            let Some(steps) = job["steps"].as_array() else {
                continue;
            };
            for (i, step) in steps.iter().enumerate() {
                let run = s(&step["run"]);
                if !(run.contains("apt-get install") && run.contains("mesa-vulkan-drivers")) {
                    continue;
                }
                seen += 1;
                let prev = if i > 0 { &steps[i - 1] } else { &Value::Null };
                assert!(
                    s(&prev["uses"]) == "./.github/actions/mesa-debs" && s(&prev["id"]) == "mesa",
                    "qa_split: {file} `{jid}`: the apt step for Mesa has no cache step (id `mesa`) before it"
                );
                assert_ne!(
                    s(&prev["with"]["save"]),
                    "true",
                    "qa_split: {file} `{jid}`: the step before the apt step only saves"
                );
                let mut cached: Vec<String> = s(&prev["with"]["packages"])
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect();
                let mut installed = apt_packages(&run);
                cached.sort();
                installed.sort();
                assert_eq!(
                    cached, installed,
                    "qa_split: {file} `{jid}`: the cache is keyed on other packages than the apt step installs"
                );
                assert!(
                    clauses(&s(&step["if"]))
                        .iter()
                        .any(|c| c == "steps.mesa.outputs.installed != 'true'"),
                    "qa_split: {file} `{jid}`: the apt step does not run on a cache that did not install"
                );
            }
        }
    }
    assert!(seen >= 10, "qa_split: only {seen} apt steps install Mesa");
}

fn workflows() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root().join(".github/workflows")).expect("qa_split: workflows") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            out.push((name, std::fs::read_to_string(&path).expect("workflow")));
        }
    }
    out.sort();
    out
}

#[test]
fn qa_mesa_key_follows_the_package_list() {
    check_key(&read(MESA), &workflows());
}

negative_control!(
    qa_mesa_key_follows_the_package_list,
    "mesa_debs.sh keyed on the image alone, not the package list",
    expected = "the key does not change with the package list",
    check_key(&swap(&read(MESA), "-${list%+}\"", "\""), &workflows())
);

mod key_without_image {
    use super::*;
    negative_control!(
        qa_mesa_key_follows_the_package_list,
        "mesa_debs.sh keyed on the package list but not the runner's image",
        expected = "the key does not change with the runner image",
        check_key(&swap(&read(MESA), "-$ImageVersion-", "-"), &workflows())
    );
}

mod apt_names_more {
    use super::*;
    negative_control!(
        qa_mesa_key_follows_the_package_list,
        "a workflow's apt step installing a package its cache step does not name",
        expected = "the cache is keyed on other packages than the apt step installs",
        check_key(&read(MESA), &{
            let mut all = workflows();
            let (_, ci) = all.iter_mut().find(|(n, _)| n == "ci.yml").expect("ci.yml");
            *ci = swap(
                ci,
                "apt-get install -y mesa-vulkan-drivers -oDPkg",
                "apt-get install -y mesa-vulkan-drivers libvulkan-dev -oDPkg",
            );
            all
        })
    );
}

/// A folder of stand-ins: `sudo` running its command (past `timeout`'s words), `dpkg` passing or failing on
/// `dpkg_ok`, and `dpkg-query` saying each package is installed on `query_ok`.
fn stand_ins(dir: &Path, dpkg_ok: bool, query_ok: bool) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("qa_split: bin");
    let write = |name: &str, body: &str| {
        let p = bin.join(name);
        std::fs::write(&p, format!("#!/bin/bash\n{body}\n")).expect("stand-in");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
    };
    write(
        "sudo",
        "if [ \"$1\" = timeout ]; then shift; while [ \"${1#-}\" != \"$1\" ] || [ \"$1\" -eq \"$1\" ] 2>/dev/null; do shift; done; fi; exec \"$@\"",
    );
    write("dpkg", if dpkg_ok { "exit 0" } else { "exit 1" });
    write(
        "dpkg-query",
        if query_ok {
            "printf 'install ok installed'"
        } else {
            "printf 'deinstall ok config-files'"
        },
    );
    bin
}

fn check_install(script: &str) {
    let pk = ("PACKAGES", "mesa-vulkan-drivers");
    // A miss: nothing installed from the cache, so the apt step runs.
    let dir = Scratch::new("qa_split_mesa_miss");
    let bin = stand_ins(&dir, true, true);
    let (out, _, ok) = mesa(
        script,
        &["install"],
        &[pk, ("HIT", "false")],
        Some(&bin),
        &dir,
    );
    assert!(ok, "qa_split: the install step failed its step on a miss");
    assert_eq!(
        out.get("installed").map(String::as_str),
        Some("false"),
        "qa_split: a miss reads as installed"
    );
    // A hit that installs: the apt step skips.
    let dir = Scratch::new("qa_split_mesa_hit");
    let bin = stand_ins(&dir, true, true);
    std::fs::create_dir_all(dir.join("cached")).expect("cached");
    std::fs::write(dir.join("cached/mesa-vulkan-drivers_1_amd64.deb"), b"x").expect("deb");
    let (out, _, ok) = mesa(
        script,
        &["install"],
        &[pk, ("HIT", "true")],
        Some(&bin),
        &dir,
    );
    assert!(ok, "qa_split: the install step failed its step on a hit");
    assert_eq!(
        out.get("installed").map(String::as_str),
        Some("true"),
        "qa_split: a hit that installs reads as not installed"
    );
    // A hit whose packages do not install, or leave a package uninstalled: the apt step runs, and the step passes.
    for (dpkg_ok, query_ok) in [(false, true), (true, false)] {
        let dir = Scratch::new("qa_split_mesa_bad");
        let bin = stand_ins(&dir, dpkg_ok, query_ok);
        std::fs::create_dir_all(dir.join("cached")).expect("cached");
        std::fs::write(dir.join("cached/mesa-vulkan-drivers_1_amd64.deb"), b"x").expect("deb");
        let (out, _, ok) = mesa(
            script,
            &["install"],
            &[pk, ("HIT", "true")],
            Some(&bin),
            &dir,
        );
        assert!(ok, "qa_split: a cache that did not install failed its step");
        assert_eq!(
            out.get("installed").map(String::as_str),
            Some("false"),
            "qa_split: a cache that did not install (dpkg {dpkg_ok}, installed {query_ok}) reads as installed"
        );
    }
    // A re-run is marked in the job's own summary; a first run is not.
    let dir = Scratch::new("qa_split_mesa_mark");
    let bin = stand_ins(&dir, true, true);
    let (_, first, _) = mesa(
        script,
        &["install"],
        &[pk, ("GITHUB_RUN_ATTEMPT", "1"), ("GITHUB_RUN_ID", "77")],
        Some(&bin),
        &dir,
    );
    assert!(
        !first.to_lowercase().contains("re-run"),
        "qa_split: a first run is marked as a re-run: {first}"
    );
    let (_, again, _) = mesa(
        script,
        &["install"],
        &[pk, ("GITHUB_RUN_ATTEMPT", "2"), ("GITHUB_RUN_ID", "77")],
        Some(&bin),
        &dir,
    );
    assert!(
        again.to_lowercase().contains("re-run") && again.contains('2'),
        "qa_split: a re-run is not marked in its summary: {again:?}"
    );
}

#[test]
fn qa_mesa_cache_install_and_rerun_mark() {
    check_install(&read(MESA));
}

negative_control!(
    qa_mesa_cache_install_and_rerun_mark,
    "mesa_debs.sh not marking a re-run",
    expected = "a re-run is not marked in its summary",
    check_install(&swap(
        &read(MESA),
        "[ \"${GITHUB_RUN_ATTEMPT:-1}\" -gt 1 ]",
        "[ \"${GITHUB_RUN_ATTEMPT:-1}\" -gt 9 ]"
    ))
);

mod installed_unchecked {
    use super::*;
    negative_control!(
        qa_mesa_cache_install_and_rerun_mark,
        "mesa_debs.sh taking dpkg's success alone, never checking each package installed",
        expected = "(dpkg true, installed false) reads as installed",
        check_install(&swap(
            &read(MESA),
            "            installed=false\n          fi\n        done",
            "            :\n          fi\n        done"
        ))
    );
}

// ---- A stand-in `gh` ----------------------------------------------------------------------------------------------

/// A folder holding a stand-in `gh` on PATH: `gh api … <path>` prints `files/<path, every non-alphanumeric as _>` when
/// there is one, exits 1 with that file's `.fail` twin's text when there is one, answers any other POST with `{}`, and
/// anything else with a 404. Every call's words go to `calls`, one call a line.
fn stand_in_gh(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("qa_split: bin");
    std::fs::create_dir_all(dir.join("files")).expect("qa_split: files");
    let gh = bin.join("gh");
    std::fs::write(
        &gh,
        format!(
            "#!/bin/bash\n\
             echo \"$*\" >> '{calls}'\n\
             path=\n\
             for a in \"$@\"; do case \"$a\" in repos/*) path=\"$a\";; esac; done\n\
             key=$(printf '%s' \"$path\" | tr -c 'A-Za-z0-9' '_')\n\
             f='{files}'/\"$key\"\n\
             if [ -f \"$f.fail\" ]; then cat \"$f.fail\" >&2; exit 1; fi\n\
             if [ -f \"$f\" ]; then cat \"$f\"; exit 0; fi\n\
             case \" $* \" in *' --method POST '*) echo '{{}}'; exit 0;; esac\n\
             echo 'gh: Not Found (HTTP 404)' >&2; exit 1\n",
            calls = dir.join("calls").display(),
            files = dir.join("files").display()
        ),
    )
    .expect("qa_split: gh");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    bin
}

/// Serves `text` for the API path `path` (or fails with it, on `fail`).
fn serve(dir: &Path, path: &str, text: &str, fail: bool) {
    let key: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let name = if fail { format!("{key}.fail") } else { key };
    std::fs::write(dir.join("files").join(name), text).expect("qa_split: served");
}

fn calls(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("calls"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

// ---- The install-only re-run, `ci_retry.py decide` ------------------------------------------------------------------

const REPO: &str = "o/r";
const RUN: &str = "900";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A job of a run as the jobs API lists it: name, conclusion, its runner's labels, and its steps (name, conclusion).
fn job(id: u64, name: &str, conclusion: &str, labels: &[&str], steps: &[(&str, &str)]) -> Value {
    json!({
        "id": id,
        "name": name,
        "status": "completed",
        "conclusion": conclusion,
        "labels": labels,
        "steps": steps.iter().map(|(n, c)| json!({"name": n, "status": "completed", "conclusion": c})).collect::<Vec<_>>(),
    })
}

const HOSTED_LABELS: &[&str] = &["ubuntu-24.04"];
const PC_LABELS: &[&str] = &["self-hosted", "Windows", "X64", "gpu"];

/// A shard's steps, the install's ending `install` and the tests' `tests`.
fn shard_steps(install: &'static str, tests: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("Set up job", "success"),
        ("The classification ran", "success"),
        ("Restore the Mesa packages", "success"),
        ("Install Mesa (lavapipe)", install),
        (
            "cargo nextest run (the changed crates and their reverse dependencies)",
            tests,
        ),
        ("Runner disk after the tests", "success"),
        ("Complete job", "success"),
    ]
}

/// A shard that failed at its install alone.
fn install_failed(id: u64, shard: u32, labels: &[&str]) -> Value {
    job(
        id,
        &format!("ci shard {shard}/3"),
        "failure",
        labels,
        &shard_steps("failure", "skipped"),
    )
}

fn shard_passed(id: u64, shard: u32) -> Value {
    job(
        id,
        &format!("ci shard {shard}/3"),
        "success",
        HOSTED_LABELS,
        &shard_steps("success", "success"),
    )
}

/// `ci`, failing because a shard did.
fn verdict_failed(id: u64) -> Value {
    job(
        id,
        "ci",
        "failure",
        HOSTED_LABELS,
        &[
            ("Set up job", "success"),
            ("The classification ran", "success"),
            ("Every shard passed", "failure"),
        ],
    )
}

fn changes(id: u64) -> Value {
    job(
        id,
        "changes",
        "success",
        HOSTED_LABELS,
        &[("Set up job", "success")],
    )
}

/// The outcome of `ci_retry.py decide` on attempt `attempt` of a failed run whose attempts' jobs are `attempts` (the
/// last the current one), each failed job's annotations from `annotations`: whether it asked GitHub to re-run, and its
/// summary.
fn decide(attempts: &[Vec<Value>], annotations: &[(u64, &str)], script: &str) -> (bool, String) {
    let dir = Scratch::new("qa_split_retry");
    let bin = stand_in_gh(&dir);
    let attempt = attempts.len();
    for (i, jobs) in attempts.iter().enumerate() {
        serve(
            &dir,
            &format!(
                "repos/{REPO}/actions/runs/{RUN}/attempts/{}/jobs?per_page=100",
                i + 1
            ),
            &json!({"total_count": jobs.len(), "jobs": jobs}).to_string(),
            false,
        );
    }
    for (id, message) in annotations {
        serve(
            &dir,
            &format!("repos/{REPO}/check-runs/{id}/annotations?per_page=100"),
            &json!([{"annotation_level": "failure", "message": message}]).to_string(),
            false,
        );
    }
    serve(
        &dir,
        &format!("repos/{REPO}/git/ref/heads/task/x"),
        &json!({"object": {"sha": SHA}}).to_string(),
        false,
    );
    let file = dir.join("ci_retry.py");
    std::fs::write(&file, script).expect("qa_split: script");
    let summary = dir.join("summary");
    let out = Command::new("python3")
        .arg("-I")
        .arg(&file)
        .arg("decide")
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("GH_REPO", REPO)
        .env("RUN_ID", RUN)
        .env("RUN_ATTEMPT", attempt.to_string())
        .env("RUN_CONCLUSION", "failure")
        .env("WORKFLOW_PATH", root().join(".github/workflows/ci.yml"))
        .env("HEAD_SHA", SHA)
        .env("HEAD_BRANCH", "task/x")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .timed_output()
        .expect("qa_split: python3 ran");
    let text = std::fs::read_to_string(&summary).unwrap_or_default();
    let asked = calls(&dir)
        .iter()
        .any(|c| c.contains(&format!("actions/runs/{RUN}/rerun-failed-jobs")));
    assert!(
        out.status.success() || asked,
        "qa_split: decide failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (asked, text)
}

fn check_rerun(script: &str) {
    let first = |labels: &[&str]| {
        vec![
            changes(1),
            install_failed(2, 1, labels),
            shard_passed(3, 2),
            shard_passed(4, 3),
            verdict_failed(5),
        ]
    };
    // A shard on a hosted runner whose only failure is its install: re-run, and the decision says it is for that.
    let (asked, summary) = decide(&[first(HOSTED_LABELS)], &[], script);
    assert!(
        asked,
        "qa_split: a hosted shard failing at its install alone is not re-run:\n{summary}"
    );
    assert!(
        summary.contains("ci shard 1/3") && summary.to_lowercase().contains("install"),
        "qa_split: the re-run's decision does not name the install failure:\n{summary}"
    );
    // The same on the self-hosted PC, or on a runner whose labels are unknown: never.
    let unknown: &[&str] = &[];
    for labels in [PC_LABELS, unknown] {
        let (asked, summary) = decide(&[first(labels)], &[], script);
        assert!(
            !asked,
            "qa_split: an install failure on a runner labelled {labels:?} is re-run:\n{summary}"
        );
    }
    // A real failure beside it, in another shard: the run stands.
    let beside = vec![
        changes(1),
        install_failed(2, 1, HOSTED_LABELS),
        job(
            3,
            "ci shard 2/3",
            "failure",
            HOSTED_LABELS,
            &shard_steps("success", "failure"),
        ),
        shard_passed(4, 3),
        verdict_failed(5),
    ];
    let (asked, summary) = decide(
        &[beside],
        &[(3, "Process completed with exit code 100.")],
        script,
    );
    assert!(
        !asked,
        "qa_split: an install failure beside a real failure in another shard is re-run:\n{summary}"
    );
    // A real failure in the same job: an install failure and a step after it, run whatever happened, that failed.
    let mut steps = shard_steps("failure", "skipped");
    steps[5] = ("Runner disk after the tests", "failure");
    let same = vec![
        changes(1),
        job(2, "ci shard 1/3", "failure", HOSTED_LABELS, &steps),
        shard_passed(3, 2),
        shard_passed(4, 3),
        verdict_failed(5),
    ];
    let (asked, summary) = decide(&[same], &[], script);
    assert!(
        !asked,
        "qa_split: an install failure beside a real failure in the same job is re-run:\n{summary}"
    );
    // A failure in a test step alone (the install passed): never.
    let tests = vec![
        changes(1),
        job(
            2,
            "ci shard 1/3",
            "failure",
            HOSTED_LABELS,
            &shard_steps("success", "failure"),
        ),
        shard_passed(3, 2),
        shard_passed(4, 3),
        verdict_failed(5),
    ];
    let (asked, summary) = decide(
        &[tests],
        &[(2, "Process completed with exit code 100.")],
        script,
    );
    assert!(!asked, "qa_split: a test failure is re-run:\n{summary}");
    // The job's own limit reached while it installed: a timeout, never re-run.
    let (asked, summary) = decide(
        &[first(HOSTED_LABELS)],
        &[(
            2,
            "The job running on runner GitHub Actions 3 has exceeded the maximum execution time of 90 minutes.",
        )],
        script,
    );
    assert!(
        !asked,
        "qa_split: an install that ran into the job's own limit is re-run:\n{summary}"
    );
    // Once per job per run: the same shard failing at its install again on the re-run stands.
    let (asked, summary) = decide(&[first(HOSTED_LABELS), first(HOSTED_LABELS)], &[], script);
    assert!(
        !asked,
        "qa_split: a second install failure of the same job is re-run:\n{summary}"
    );
    // Per job: another shard's first install failure on the re-run is its own first, and re-run.
    let other = vec![
        changes(1),
        shard_passed(2, 1),
        install_failed(3, 2, HOSTED_LABELS),
        shard_passed(4, 3),
        verdict_failed(5),
    ];
    let (asked, summary) = decide(&[first(HOSTED_LABELS), other], &[], script);
    assert!(
        asked,
        "qa_split: another job's first install failure, on the re-run, is not re-run:\n{summary}"
    );
}

#[test]
fn qa_rerun_install_only_once_hosted_never_real() {
    check_rerun(&read(".github/scripts/ci_retry.py"));
}

negative_control!(
    qa_rerun_install_only_once_hosted_never_real,
    "ci_retry.py re-running an install failure however often it recurs",
    expected = "a second install failure of the same job is re-run",
    check_rerun(&swap(
        &read(".github/scripts/ci_retry.py"),
        "if why and j[\"name\"] in before:",
        "if why and False:"
    ))
);

mod rerun_on_the_pc {
    use super::*;
    negative_control!(
        qa_rerun_install_only_once_hosted_never_real,
        "ci_retry.py taking every runner as GitHub-hosted",
        expected = "on a runner labelled [\"self-hosted\"",
        check_rerun(&swap(
            &read(".github/scripts/ci_retry.py"),
            "    if not failed or not hosted(job):\n",
            "    if not failed:\n"
        ))
    );
}

mod rerun_any_step {
    use super::*;
    negative_control!(
        qa_rerun_install_only_once_hosted_never_real,
        "ci_retry.py taking a job whose install failed as install-only whatever else failed in it",
        expected = "beside a real failure in the same job is re-run",
        check_rerun(&swap(
            &read(".github/scripts/ci_retry.py"),
            "    if not failed or any(name not in INSTALL_STEPS for name in failed):\n",
            "    if not any(name in INSTALL_STEPS for name in failed):\n"
        ))
    );
}

mod rerun_beside_a_real_failure {
    use super::*;
    negative_control!(
        qa_rerun_install_only_once_hosted_never_real,
        "ci_retry.py re-running whatever failed beside an install failure",
        expected = "beside a real failure in another shard is re-run",
        check_rerun(&swap(
            &read(".github/scripts/ci_retry.py"),
            "    if blocking:\n        return False,",
            "    if blocking and not installs:\n        return False,"
        ))
    );
}
