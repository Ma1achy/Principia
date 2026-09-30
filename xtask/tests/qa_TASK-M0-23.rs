//! QA tests for TASK-M0-23, written from REQ-VAL-148 and REQ-VAL-149 (R-196, R-202, R-71, R-182), not from the
//! implementation:
//!
//! - REQ-VAL-148 (as reworded by R-302): "Every pull request must run `cargo mutants --in-diff` on its changed code in
//!   CI, sharded across parallel CI jobs (`--shard k/n`), each shard within the per-shard time limit (REQ-VAL-149),
//!   excluding generated code, GPU-only (spirv-gated) paths and xtask's own harness plumbing ... The job must fail on
//!   any surviving mutant not in a checked-in list of equivalent mutants, each entry carrying a one-line
//!   justification". verify: "`ci.yml` runs `cargo mutants --in-diff` against the PR's base on pull_request events as
//!   a matrix of n shards ... each exclusion names which of R-196's three categories it falls in".
//! - REQ-VAL-149 (as reworded by R-302): "each shard runs under that limit, marked provisional until the human
//!   confirms both at the M0 gate". The `mutants` job below is the matrix job, so its limit is each shard's; the
//!   sharding itself (every shard runs, a cut-off shard fails, the aggregate reads every shard) is tested in
//!   `qa_TASK-M0-23_shards.rs`.
//! - R-196, as applied per R-204 (RQ-162): "xtask's own harness plumbing" is `xtask/src/main.rs` and
//!   `xtask/src/codegen.rs` only; xtask's checks stay mutated.
//! - TASK-M0-23 Deliverables: generated code is "the files `cargo xtask codegen` writes, matched so that a generated
//!   file added later is excluded too". The plan names where codegen writes Rust: `crates/kernel/src/payload/
//!   generated.rs` (TASK-M0-09, M0-10, M0-12), `crates/ledger/generated/read_side.rs` (TASK-M1-01),
//!   `crates/kernel/src/generated/{links,constants,nsub_thresholds}.rs` (TASK-M2-01, M3-03).
//!
//! The glob matcher below follows cargo-mutants 27.1.0's `exclude_globs` as observed with `cargo mutants --list`: a
//! glob with a `/` matches the whole tree-relative path (`**` any number of components, `*` within one); a glob
//! without one matches any single path component.
//!
//! Each test's control (R-176, R-199) feeds the same assertion an input differing in the one respect the requirement
//! turns on, and trips the assertion by its message.
// The file name `qa_TASK-M0-23` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::mutants_check::{equivalents, run};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

// ---------------------------------------------------------------------------------------------------------------
// ci.yml: the job, its trigger, its diff against the base, its report and its time limit.

/// The lines of job `name` in a workflow's text: from `  <name>:` to the next job key at the same indent.
fn job<'a>(yml: &'a str, name: &str) -> Vec<&'a str> {
    let head = format!("  {name}:");
    let mut lines = yml.lines().skip_while(|l| l.trim_end() != head);
    let Some(first) = lines.next() else {
        panic!("qa23: no `{name}` job in the workflow");
    };
    std::iter::once(first)
        .chain(lines.take_while(|l| {
            l.is_empty() || l.starts_with("   ") || l.trim_start().starts_with('#')
        }))
        .collect()
}

/// REQ-VAL-148: on pull_request events, on ubuntu-latest (R-186), `cargo mutants --in-diff` reads the diff written
/// by `git diff origin/<base>...HEAD`, the checked-in list decides, and the report is uploaded.
fn checks_job(yml: &str) {
    assert!(
        yml.lines().any(|l| l.trim_end() == "  pull_request:"),
        "qa23: the workflow does not run on pull_request events"
    );
    let lines = job(yml, "mutants");
    let has = |s: &str| lines.iter().any(|l| l.contains(s));
    assert!(
        has("if: github.event_name == 'pull_request'"),
        "qa23: the mutants job is not limited to pull_request events"
    );
    assert!(
        has("runs-on: ubuntu-latest"),
        "qa23: the mutants job is not on ubuntu-latest"
    );
    let diff_file = lines
        .iter()
        .filter(|l| l.contains("git diff") && l.contains("origin/${{ github.base_ref }}...HEAD"))
        .find_map(|l| l.split_once('>').map(|(_, f)| f.trim().to_owned()))
        .expect("qa23: no `git diff origin/<base>...HEAD > <file>` in the mutants job");
    assert!(
        lines.iter().any(|l| {
            let l = l.trim();
            l.starts_with("cargo mutants")
                && !l.contains("--dir")
                && l.contains(&format!("--in-diff {diff_file}"))
        }),
        "qa23: the job's cargo mutants does not read the PR's diff ({diff_file})"
    );
    assert!(
        lines.iter().any(|l| {
            let l = l.trim();
            l.starts_with("run: cargo xtask mutants-check") && !l.contains("--equivalent")
        }),
        "qa23: the job's verdict is not mutants-check against the checked-in list"
    );
    assert!(
        has("actions/upload-artifact") && has("mutants.out"),
        "qa23: the mutants report is not uploaded"
    );
}

#[test]
fn qa23_mutants_job_runs_in_diff_against_the_base_on_pull_requests() {
    checks_job(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa23_mutants_job_runs_in_diff_against_the_base_on_pull_requests,
    "cargo mutants pointed at a file other than the PR's diff",
    expected = "qa23: the job's cargo mutants does not read the PR's diff",
    checks_job(&read(".github/workflows/ci.yml").replace(
        "--in-diff \"$RUNNER_TEMP/pr.diff\"",
        "--in-diff \"$RUNNER_TEMP/other.diff\""
    ))
);

/// REQ-VAL-149 / R-182: the job (or its `cargo mutants` step) runs under a `timeout-minutes` limit, with a comment
/// just above it naming REQ-VAL-149 and marking the value provisional.
fn under_limit(yml: &str) {
    let lines = job(yml, "mutants");
    let at = lines.iter().position(|l| {
        l.trim()
            .strip_prefix("timeout-minutes:")
            .is_some_and(|v| v.trim().parse::<u32>().is_ok_and(|m| m > 0))
    });
    let Some(at) = at else {
        panic!("qa23: the mutants job runs under no time limit (no timeout-minutes)");
    };
    let above = lines[at.saturating_sub(4)..at].join("\n");
    assert!(
        above.contains("REQ-VAL-149") && above.to_lowercase().contains("provisional"),
        "qa23: the time limit is not marked as REQ-VAL-149, provisional: {above}"
    );
}

const LIMITED: &str = "on:\n  pull_request:\njobs:\n  mutants:\n    runs-on: ubuntu-latest\n    \
                       # REQ-VAL-149, provisional until the M0 gate (R-182).\n    timeout-minutes: 30\n    \
                       steps:\n      - run: cargo mutants\n";

#[test]
fn qa23_mutants_job_runs_under_the_provisional_time_limit() {
    under_limit(LIMITED);
    under_limit(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa23_mutants_job_runs_under_the_provisional_time_limit,
    "a job with its timeout-minutes removed",
    expected = "qa23: the mutants job runs under no time limit",
    under_limit(&LIMITED.replace("    timeout-minutes: 30\n", ""))
);

// ---------------------------------------------------------------------------------------------------------------
// .cargo/mutants.toml: the exclusions.

/// One glob component against one path component: `*` matches any run of characters.
fn component(pat: &str, s: &str) -> bool {
    match pat.split_once('*') {
        None => pat == s,
        Some((head, rest)) => {
            s.starts_with(head)
                && (0..=s.len() - head.len()).any(|i| {
                    s.is_char_boundary(head.len() + i) && component(rest, &s[head.len() + i..])
                })
        }
    }
}

fn components(pat: &[&str], path: &[&str]) -> bool {
    match (pat.first(), path.first()) {
        (None, None) => true,
        (Some(&"**"), _) => {
            components(&pat[1..], path) || (!path.is_empty() && components(pat, &path[1..]))
        }
        (Some(p), Some(c)) => component(p, c) && components(&pat[1..], &path[1..]),
        _ => false,
    }
}

/// Whether cargo-mutants' `exclude_globs` entry `glob` excludes the tree-relative `path`.
fn excludes(glob: &str, path: &str) -> bool {
    let path: Vec<&str> = path.split('/').collect();
    if glob.contains('/') {
        components(&glob.split('/').collect::<Vec<_>>(), &path)
    } else {
        path.iter().any(|c| component(glob, c))
    }
}

/// The config's exclusions, each with the R-196 category its comments put it under (`None`: no category).
fn exclusions(toml: &str) -> Vec<(String, Option<&'static str>)> {
    let doc: toml_edit::DocumentMut = toml.parse().expect("qa23: .cargo/mutants.toml is not TOML");
    let mut out = Vec::new();
    for key in ["exclude_globs", "exclude_re"] {
        let Some(array) = doc.get(key).and_then(toml_edit::Item::as_array) else {
            continue;
        };
        let mut category = None;
        for value in array.iter() {
            let comment = value
                .decor()
                .prefix()
                .and_then(|p| p.as_str())
                .unwrap_or_default()
                .to_lowercase();
            for (needle, name) in [
                ("generated code", "generated"),
                ("gpu-only", "gpu"),
                ("spirv", "gpu"),
                ("harness plumbing", "harness"),
            ] {
                if comment.contains(needle) {
                    category = Some(name);
                }
            }
            let text = value.as_str().unwrap_or_default().to_owned();
            out.push((format!("{key}: {text}"), category));
        }
    }
    out
}

fn globs(toml: &str) -> Vec<String> {
    exclusions(toml)
        .into_iter()
        .filter_map(|(e, _)| e.strip_prefix("exclude_globs: ").map(str::to_owned))
        .collect()
}

/// The glob matcher agrees with what `cargo mutants --list` (27.1.0) did on these paths.
fn matcher_is_cargo_mutants() {
    let dir = "crates/kernel/src/generated/links.rs";
    let file = "crates/kernel/src/payload/generated.rs";
    assert!(excludes("**/generated/**", dir) && !excludes("**/generated/**", file));
    assert!(excludes("generated.rs", file) && !excludes("generated.rs", dir));
    assert!(excludes("generated", dir) && !excludes("generated", file));
    assert!(
        excludes("crates/kernel/src/generated/*.rs", dir)
            && !excludes("crates/kernel/src/generated/*.rs", file)
    );
    assert!(
        excludes("xtask/src/main.rs", "xtask/src/main.rs")
            && !excludes("xtask/src/main.rs", "xtask/src/ci.rs")
    );
}

/// The Rust files `cargo xtask codegen` writes, or the plan says it will write.
fn generated_paths() -> Vec<String> {
    let mut paths: Vec<String> = ledger::gen::generate(&ledger::layout(), ledger::gen::EMITTERS)
        .expect("qa23: the ledger generates")
        .into_iter()
        .map(|g| g.path.to_string_lossy().replace('\\', "/"))
        .filter(|p| p.ends_with(".rs"))
        .collect();
    paths.extend(
        [
            "crates/kernel/src/payload/generated.rs",
            "crates/ledger/generated/read_side.rs",
            "crates/kernel/src/generated/links.rs",
            "crates/kernel/src/generated/constants.rs",
            "crates/kernel/src/generated/nsub_thresholds.rs",
        ]
        .map(str::to_owned),
    );
    paths
}

/// REQ-VAL-148 / R-196: every generated file is excluded.
fn generated_excluded(globs: &[String], paths: &[String]) {
    let missed: Vec<&String> = paths
        .iter()
        .filter(|p| !globs.iter().any(|g| excludes(g, p)))
        .collect();
    assert!(
        missed.is_empty(),
        "qa23: generated file(s) not excluded by {globs:?}: {missed:?}"
    );
}

#[test]
fn qa23_exclusions_cover_every_generated_file() {
    matcher_is_cargo_mutants();
    generated_excluded(&globs(&read(".cargo/mutants.toml")), &generated_paths());
}

negative_control!(
    qa23_exclusions_cover_every_generated_file,
    "no exclusion at all",
    expected = "qa23: generated file(s) not excluded",
    generated_excluded(&[], &generated_paths())
);

/// Sources that are none of R-196's three categories, and stay mutated: xtask's checks and `ci.rs` (RQ-162), and
/// the crates' hand-written code.
const MUTATED: &[&str] = &[
    "xtask/src/ci.rs",
    "xtask/src/controls.rs",
    "xtask/src/deps.rs",
    "xtask/src/lint_constants.rs",
    "xtask/src/mutants_check.rs",
    "xtask/src/pr_check.rs",
    "xtask/src/reviews_check.rs",
    "xtask/src/lib.rs",
    "crates/ledger/src/gen/mod.rs",
    "crates/ledger/src/lib.rs",
    "crates/kernel/src/lib.rs",
    "crates/kernel/src/payload/mod.rs",
    "crates/validation/src/lib.rs",
];

/// REQ-VAL-148: each exclusion names its R-196 category, the harness plumbing is `main.rs` and `codegen.rs`, and no
/// exclusion reaches a source outside the three categories. The GPU-only category is carried by the marker, named in
/// the file.
fn categorised(toml: &str) {
    for (exclusion, category) in exclusions(toml) {
        assert!(
            category.is_some(),
            "qa23: exclusion `{exclusion}` names no R-196 category"
        );
    }
    let globs = globs(toml);
    for plumbing in ["xtask/src/main.rs", "xtask/src/codegen.rs"] {
        assert!(
            globs.iter().any(|g| excludes(g, plumbing)),
            "qa23: harness plumbing {plumbing} is not excluded"
        );
    }
    for source in MUTATED {
        assert!(
            !globs.iter().any(|g| excludes(g, source)),
            "qa23: {source} is excluded though it is none of R-196's categories"
        );
    }
    assert!(
        toml.contains("GPU-only") && toml.contains("#[cfg_attr(test, mutants::skip)]"),
        "qa23: the GPU-only category's marker is not named"
    );
}

#[test]
fn qa23_exclusions_each_name_a_category_and_spare_the_checks() {
    categorised(&read(".cargo/mutants.toml"));
}

negative_control!(
    qa23_exclusions_each_name_a_category_and_spare_the_checks,
    "an exclusion of xtask's checks, with no category",
    expected = "qa23: exclusion `exclude_globs: xtask/src/**` names no R-196 category",
    categorised(&read(".cargo/mutants.toml").replacen(
        "exclude_globs = [",
        "exclude_globs = [\n    \"xtask/src/**\",",
        1
    ))
);

// ---------------------------------------------------------------------------------------------------------------
// The equivalent-mutants list and the verdict over a run.

/// R-202: the checked-in list reads, each entry with a one-line justification, and holds none of the fixture's
/// stand-ins (they are not equivalent).
fn list_is_justified(text: &str) {
    let list = equivalents(text);
    assert!(
        list.as_ref().is_ok_and(|l| l
            .iter()
            .all(|e| !e.justification.contains("fixture stand-in"))),
        "qa23: the checked-in equivalent-mutants list is refused or holds a stand-in: {list:?}"
    );
}

#[test]
fn qa23_checked_in_equivalents_each_carry_a_one_line_justification() {
    list_is_justified(&read(".cargo/mutants-equivalent.toml"));
}

negative_control!(
    qa23_checked_in_equivalents_each_carry_a_one_line_justification,
    "the checked-in list with an entry whose justification spans two lines",
    expected = "qa23: the checked-in equivalent-mutants list is refused",
    list_is_justified(&format!(
        "{}\n[[equivalent]]\nmutant = \"src/x.rs:1:1: replace f -> u32 with 0\"\njustification = \"\"\"a\nb\"\"\"\n",
        read(".cargo/mutants-equivalent.toml")
    ))
);

/// A run's output directory holding a hand-written `outcomes.json` in cargo-mutants' schema: a passing baseline and
/// `(name, summary)` per mutant.
fn run_dir(tag: &str, mutants: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa23_{tag}"));
    std::fs::create_dir_all(&dir).expect("run dir");
    let mut outcomes = vec![serde_json::json!({"scenario": "Baseline", "summary": "Success"})];
    outcomes.extend(mutants.iter().map(|(name, summary)| {
        serde_json::json!({"scenario": {"Mutant": {"name": name, "package": "p", "file": "src/a.rs"}},
                           "summary": summary})
    }));
    std::fs::write(
        dir.join("outcomes.json"),
        serde_json::json!({ "outcomes": outcomes }).to_string(),
    )
    .expect("outcomes written");
    dir
}

fn list(tag: &str, mutants: &[&str]) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa23_{tag}.toml"));
    let text: String = mutants
        .iter()
        .map(|m| {
            format!("[[equivalent]]\nmutant = \"{m}\"\njustification = \"why, in one line\"\n")
        })
        .collect();
    std::fs::write(&path, text).expect("list written");
    path
}

const SURVIVOR: &str = "src/a.rs:10:5: replace + with - in f";
const CAUGHT: &str = "src/a.rs:10:5: replace + with * in f";
const UNVIABLE: &str = "src/a.rs:9:1: replace f -> u32 with Default::default()";

/// R-202: the verdict over `dir` against `list` fails, naming `SURVIVOR` and nothing caught or unviable.
fn fails_naming_survivor(dir: &Path, list: &Path) {
    let verdict = run(dir, list);
    assert!(
        verdict
            .as_ref()
            .is_err_and(|e| e.contains(SURVIVOR) && !e.contains(CAUGHT) && !e.contains(UNVIABLE)),
        "qa23: the verdict did not fail naming only the survivor: {verdict:?}"
    );
}

fn mixed(tag: &str) -> PathBuf {
    run_dir(
        tag,
        &[
            (CAUGHT, "CaughtMutant"),
            (SURVIVOR, "MissedMutant"),
            (UNVIABLE, "Unviable"),
        ],
    )
}

#[test]
fn qa23_verdict_fails_on_an_unlisted_or_stale_listed_survivor() {
    // Unlisted, with a caught and an unviable mutant beside it.
    fails_naming_survivor(&mixed("unlisted"), &list("none", &[]));
    // Listed at another line: an entry names one mutant, so a stale entry does not cover it.
    fails_naming_survivor(
        &mixed("stale"),
        &list("stale", &["src/a.rs:11:5: replace + with - in f"]),
    );
    // Listed under a caught mutant's name only.
    fails_naming_survivor(&mixed("other"), &list("other", &[CAUGHT, UNVIABLE]));
    // Listed exactly: passes.
    let listed = run(&mixed("listed"), &list("exact", &[SURVIVOR]));
    assert!(listed.is_ok(), "qa23: a listed survivor failed: {listed:?}");
    // No run output at all is no evidence: refused, not passed.
    let empty = Path::new(env!("CARGO_TARGET_TMPDIR")).join("qa23_no_outcomes");
    std::fs::create_dir_all(&empty).expect("dir");
    assert!(
        run(&empty, &list("none2", &[])).is_err(),
        "qa23: a run with no outcomes.json passed"
    );
}

negative_control!(
    qa23_verdict_fails_on_an_unlisted_or_stale_listed_survivor,
    "the survivor listed exactly",
    expected = "qa23: the verdict did not fail naming only the survivor",
    fails_naming_survivor(&mixed("control"), &list("control", &[SURVIVOR]))
);

// ---------------------------------------------------------------------------------------------------------------
// The fixture crate.

/// REQ-VAL-148's fixture: a crate outside the workspace, whose diff adds lines present in its source.
fn fixture_is_standalone(members: &str, fixture_manifest: &str, diff: &str, source: &str) {
    assert!(
        !members.contains("fixtures"),
        "qa23: the fixture crate is a workspace member"
    );
    assert!(
        fixture_manifest.lines().any(|l| l.trim() == "[workspace]"),
        "qa23: the fixture crate is not its own workspace"
    );
    let added: Vec<&str> = diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .map(|l| &l[1..])
        .collect();
    let at = source
        .lines()
        .collect::<Vec<_>>()
        .windows(added.len().max(1))
        .position(|w| w == added.as_slice());
    assert!(
        !added.is_empty() && at.is_some(),
        "qa23: the fixture's diff adds lines not in its source: {added:?}"
    );
}

fn workspace_members() -> String {
    let manifest = read("Cargo.toml");
    let start = manifest.find("members").expect("members");
    manifest[start
        ..manifest[start..]
            .find(']')
            .map_or(manifest.len(), |e| start + e)]
        .to_owned()
}

#[test]
fn qa23_fixture_is_outside_the_workspace_and_its_diff_adds_the_branch() {
    fixture_is_standalone(
        &workspace_members(),
        &read("fixtures/mutants/untested/Cargo.toml"),
        &read("fixtures/mutants/untested/change.diff"),
        &read("fixtures/mutants/untested/src/lib.rs"),
    );
}

negative_control!(
    qa23_fixture_is_outside_the_workspace_and_its_diff_adds_the_branch,
    "a diff adding a line the source does not have",
    expected = "qa23: the fixture's diff adds lines not in its source",
    fixture_is_standalone(
        &workspace_members(),
        &read("fixtures/mutants/untested/Cargo.toml"),
        &read("fixtures/mutants/untested/change.diff").replace("-1", "-2"),
        &read("fixtures/mutants/untested/src/lib.rs"),
    )
);
