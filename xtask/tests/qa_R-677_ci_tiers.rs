//! qa's tests for R-677, written from the owner's words of 2 Nov 2026 and the ruling, not from `scripts/ci_changes.sh`'s
//! or `scripts/ci_req_fields.py`'s code.
//!
//! The owner, item 3: "a three-tier classifier in scripts/ci_changes.sh. (a) CODE: anything under crates/, xtask/,
//! Cargo.*, rust-toolchain, .github/, .cargo/, fixtures/, the classifier and contain.sh → full CI as now. (b)
//! PLAN-DATA: the reads_code files (plan/tasks/*, plan/tasks.yaml, plan/requirements.yaml, plan/MILESTONES.md) → only
//! the checks that read them … no test shards, no GPU, no mutants. docs/gui/design/* and docs/gui/reference/* → only
//! the screenshot/golden tests that read them. (c) DOCS: as now, cheap checks only. Mutants run only when crates/
//! source changes, on the merge-ready head. Every required check still reports under its own name and concludes
//! success only where the tier logic allows it; failing toward full CI on anything unknown stays." And on
//! `plan/requirements.yaml`, the field-level split ("only fields no code reads → plan-data; any code-read field,
//! new/unknown field, new requirement, or unparseable diff → code tier"): "Yeah do that then, make it good and work
//! well"; "keep it simple and make ci fast without sacrificing testing on actual code".
//!
//! - `qa_r677_each_path_lands_in_its_tier`: the owner's lists, path by path and mixed, through `classify`: the tier,
//!   `docs_only`, `gpu`, which of tier (b)'s data, and `mutants` (only Rust under `crates/`).
//! - `qa_r677_requirements_split_by_field`: `plan/requirements.yaml` with base and head copies, on a fixture holding
//!   every field R-677 lists: a change to only the fields no code reads (`area`, `statement`, `source`, `rulings`,
//!   `note`, `rq`) is tier (b); one to a field code reads (`id`, `kind`, `retired`, `milestone`, `verify` and its
//!   `method` and `detail`), an added or removed requirement, an unknown field, a file that doesn't parse or is missing
//!   is code; the reason line names the requirement and the field. One case runs on a copy of the real file.
//! - `qa_r677_detect_reads_the_requirements_at_base_and_head`: `detect` on real merge commits, the base's and the
//!   head's copies taken from git.
//! - `qa_r677_required_checks_on_every_tier`: `ci.yml`'s jobs walked as GitHub would on each tier's verdict, each
//!   gate's own script run under bash: every required check of `ci.yml` concludes success on every tier; a data or docs
//!   verdict runs no shard, build or GPU suite; `ci-workspace` runs, on tier (b), the readers' tests R-677 names (each
//!   one's test file real), and on tier (c) no test at all; a failing reader turns `ci` red.
//! - `qa_r677_mutants_fixture_skips_only_on_docs`: `mutants.yml`'s `mutants-fixture`, the gate's self-test, skips its
//!   work only on a docs verdict (`tier=docs`) and runs all of it on code, with or without Rust under `crates/`, on
//!   tier (b)'s data and on a classification that says nothing; the classifier gives each of the gate's own files
//!   (`.cargo/mutants.toml`, `fixtures/mutants/`, `scripts/contain.sh`, `mutants.yml`, xtask's `mutants_check.rs`) a
//!   verdict it runs on (R-677 as amended after PR #382's code review: tier (a) is "full CI as now").
//!
//! Every path under `docs/` these name for the classifier is a made-up one or a directory walker's, so this file is no
//! reader of a real data file. Each test has its negative control (R-176).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa_r677: {rel}: {e}"))
}

fn script() -> String {
    read("scripts/ci_changes.sh")
}

fn fields_script() -> String {
    read("scripts/ci_req_fields.py")
}

/// `text` with `from` replaced once by `to`; `from` must be there, so a control cannot pass vacuously.
#[cfg(feature = "controls")]
fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa_r677: the control's edit target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

/// A classifier's verdict, `key=value` a line, as a map; each key once.
fn parse_verdict(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            assert!(
                out.insert(k.to_owned(), v.to_owned()).is_none(),
                "qa_r677: `{k}=` twice in: {text}"
            );
        }
    }
    for key in [
        "tier",
        "docs_only",
        "gpu",
        "plan_data",
        "gui_data",
        "mutants",
    ] {
        assert!(out.contains_key(key), "qa_r677: no `{key}=` in: {text}");
    }
    out
}

/// What one `classify` run gives: the verdict, and what it printed on stderr.
struct Classified {
    verdict: BTreeMap<String, String>,
    stderr: String,
}

/// `classify` of `script` on `paths`, with `env` (the requirements' copies, the field check) set.
fn classify(script: &str, paths: &[&str], env: &[(&str, &Path)]) -> Classified {
    let dir = Scratch::new("qa_r677_classify");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("ci_changes.sh");
    let input = dir.join("paths");
    std::fs::write(&file, script).expect("script written");
    std::fs::write(&input, paths.join("\n")).expect("paths written");
    let mut cmd = Command::new("bash");
    cmd.args(["-c", "bash \"$0\" classify < \"$1\""])
        .arg(&file)
        .arg(&input)
        .env_remove("CI_REQS_BASE")
        .env_remove("CI_REQS_HEAD")
        .env("CI_REQ_FIELDS", root().join("scripts/ci_req_fields.py"));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("qa_r677: bash ran");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "qa_r677: classify failed: {stdout}{stderr}"
    );
    Classified {
        verdict: parse_verdict(&stdout),
        stderr,
    }
}

/// Whether `verdict` holds every `(key, value)` of `want`.
fn holds(verdict: &BTreeMap<String, String>, want: &[(&str, &str)]) -> bool {
    want.iter()
        .all(|(k, v)| verdict.get(*k).map(String::as_str) == Some(*v))
}

// ---- Each path lands in its tier ---------------------------------------------------------------------------------

/// Tier (a): the whole of full CI runs, the GPU suites among it.
const CODE: &[(&str, &str)] = &[("tier", "code"), ("docs_only", "false"), ("gpu", "true")];
/// Tier (b), the plan's task data: no full CI, no GPU suite, no mutants; the plan's readers run.
const PLAN: &[(&str, &str)] = &[
    ("tier", "data"),
    ("docs_only", "true"),
    ("gpu", "false"),
    ("plan_data", "true"),
    ("gui_data", "false"),
    ("mutants", "false"),
];
/// Tier (b), the GUI's artboards and references: likewise, the GUI's readers run.
const GUI: &[(&str, &str)] = &[
    ("tier", "data"),
    ("docs_only", "true"),
    ("gpu", "false"),
    ("plan_data", "false"),
    ("gui_data", "true"),
    ("mutants", "false"),
];
/// Tier (c): the cheap checks only.
const DOCS: &[(&str, &str)] = &[
    ("tier", "docs"),
    ("docs_only", "true"),
    ("gpu", "false"),
    ("plan_data", "false"),
    ("gui_data", "false"),
    ("mutants", "false"),
];

/// What a verdict must hold: `(key, value)` pairs.
type Holds = &'static [(&'static str, &'static str)];

/// A change's paths, what its verdict must hold, and whether its mutants run.
type Case = (&'static [&'static str], Holds, bool);

/// The cases, each a [`Case`].
const CASES: &[Case] = &[
    // (a) code, the owner's list. Mutants only for Rust under `crates/`.
    (&["crates/kernel/src/qa.rs"], CODE, true),
    (&["crates/kernel/src/qa.wgsl"], CODE, false),
    (&["crates/ledger/tests/qa.rs"], CODE, true),
    (&["xtask/src/qa.rs"], CODE, false),
    (&["xtask/tests/qa.rs"], CODE, false),
    (&["fixtures/qa/a.json"], CODE, false),
    (&["fixtures/qa/a.rs"], CODE, false),
    (&["Cargo.toml"], CODE, false),
    (&["Cargo.lock"], CODE, false),
    (&["rust-toolchain.toml"], CODE, false),
    (&[".github/workflows/qa.yml"], CODE, false),
    (&[".cargo/config.toml"], CODE, false),
    (&["scripts/ci_changes.sh"], CODE, false),
    (&["scripts/ci_req_fields.py"], CODE, false),
    (&["scripts/contain.sh"], CODE, false),
    // `plan/requirements.yaml` with no copies to compare: the field check can't clear it, so it is code.
    (&["plan/requirements.yaml"], CODE, false),
    // Failing toward full CI on anything unknown.
    (&["newtop/qa.md"], CODE, false),
    (&["LICENSE"], CODE, false),
    (&["docs/qa.md", "newtop/qa.md"], CODE, false),
    // (b) the plan's task data, an existing task file and a new one among it.
    (&["plan/tasks/M0/TASK-M0-01.md"], PLAN, false),
    (&["plan/tasks/M9/TASK-M9-99.md"], PLAN, false),
    (&["plan/tasks.yaml"], PLAN, false),
    (&["plan/MILESTONES.md"], PLAN, false),
    // (b) the GUI's artboards and references, new ones among them.
    (&["docs/gui/design/99_qa.png"], GUI, false),
    (&["docs/gui/reference/qa.html"], GUI, false),
    (
        &["docs/gui/reference/print_graph/catalogue/qa.json"],
        GUI,
        false,
    ),
    // (c) docs: the rest of the corpus, the plan's prose and Python, the scripts, the workbench, the root's markdown.
    (&["docs/qa.md"], DOCS, false),
    (&["docs/gui/qa_notes.md"], DOCS, false),
    (&["plan/WORKFLOW.md"], DOCS, false),
    (&["plan/tools/qa.py"], DOCS, false),
    (&["scripts/qa.py"], DOCS, false),
    (&["workbench/qa.html"], DOCS, false),
    (&["decisions.md"], DOCS, false),
    (&["docs/reference/qa/a.rs"], DOCS, false),
    // The highest tier wins: data over docs, code over data.
    (
        &["docs/qa.md", "plan/tasks.yaml", "decisions.md"],
        PLAN,
        false,
    ),
    (&["decisions.md", "docs/gui/design/99_qa.png"], GUI, false),
    (
        &["plan/MILESTONES.md", "docs/gui/reference/qa.html"],
        &[
            ("tier", "data"),
            ("docs_only", "true"),
            ("gpu", "false"),
            ("plan_data", "true"),
            ("gui_data", "true"),
        ],
        false,
    ),
    (&["plan/tasks.yaml", "crates/kernel/src/qa.rs"], CODE, true),
    (&["crates/kernel/src/qa.rs", "plan/tasks.yaml"], CODE, true),
    (
        &["docs/gui/design/99_qa.png", "xtask/src/qa.rs"],
        CODE,
        false,
    ),
    (&["docs/qa.md", "Cargo.lock", "decisions.md"], CODE, false),
    // `crates/gui/` is code with no GPU suite (R-656), its Rust mutated; beside data it stays so.
    (
        &["crates/gui/src/qa.rs", "plan/tasks.yaml"],
        &[("tier", "code"), ("docs_only", "false"), ("gpu", "false")],
        true,
    ),
    // An empty change runs everything, its mutants among it.
    (&[], CODE, true),
];

fn check_tiers(script: &str) {
    for (paths, want, mutants) in CASES {
        let got = classify(script, paths, &[]).verdict;
        let mutants = if *mutants { "true" } else { "false" };
        assert!(
            holds(&got, want) && holds(&got, &[("mutants", mutants)]),
            "qa_r677: {paths:?} classifies {got:?}, the ruling gives {want:?} with mutants={mutants}"
        );
    }
}

#[test]
fn qa_r677_each_path_lands_in_its_tier() {
    check_tiers(&script());
}

negative_control!(
    qa_r677_each_path_lands_in_its_tier,
    "the script with plan/tasks.yaml off the plan's task data",
    expected = "[\"plan/tasks.yaml\"] classifies",
    check_tiers(&swap(&script(), "    plan/tasks.yaml) return 0 ;;\n", ""))
);

mod mutants_on_any_rust {
    use super::*;
    negative_control!(
        qa_r677_each_path_lands_in_its_tier,
        "the script running mutants on Rust anywhere, not only under crates/",
        expected = "[\"xtask/src/qa.rs\"] classifies",
        check_tiers(&swap(
            &script(),
            "    crates/*.rs) return 0 ;;\n",
            "    *.rs) return 0 ;;\n"
        ))
    );
}

mod data_over_code {
    use super::*;
    negative_control!(
        qa_r677_each_path_lands_in_its_tier,
        "the script letting a later data path lower a code change's tier",
        expected = "[\"crates/kernel/src/qa.rs\", \"plan/tasks.yaml\"] classifies",
        check_tiers(&swap(
            &script(),
            "      data) if [ \"$tier\" = docs ]; then tier=data; fi ;;\n",
            "      data) tier=data ;;\n"
        ))
    );
}

// ---- plan/requirements.yaml, field by field ----------------------------------------------------------------------

/// A requirements file holding every field R-677 lists, shaped as `plan/requirements.yaml` is.
const BASE: &str = "\
- id: REQ-QA-001
  area: QA
  kind: calibration
  statement: \"The first.\"
  source:
  - file: docs/qa.md
    section: \"One\"
  verify:
    method: unit test
    detail: \"cargo test -p qa one\"
  rulings:
  - R-1
  milestone: M0
  note: \"A note.\"
- id: REQ-QA-002
  area: QA
  statement: \"The second.\"
  source:
  - file: docs/qa.md
    section: \"Two\"
  verify:
    method: review checklist
    detail: \"qa: the second\"
  milestone: M1
  rq: RQ-1
- id: REQ-QA-003
  area: QA
  statement: \"The third.\"
  verify:
    method: unit test
    detail: \"cargo test -p qa three\"
  milestone: M2
  retired: \"R-2: gone\"
";

/// A requirement added at the end.
const ADDED: &str = "\
- id: REQ-QA-004
  area: QA
  statement: \"The fourth.\"
  verify:
    method: unit test
    detail: \"cargo test -p qa four\"
  milestone: M2
";

/// What a head must classify as: tier (b), the plan's data (R-677), or code.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Want {
    Data,
    Code,
}

/// (what the head changes, the head's text, the tier, and the words its reason line must hold).
fn field_cases() -> Vec<(&'static str, Option<String>, Want, &'static [&'static str])> {
    let edit = |from: &str, to: &str| -> Option<String> {
        assert!(BASE.contains(from), "qa_r677: the fixture has no {from}");
        Some(BASE.replacen(from, to, 1))
    };
    vec![
        ("nothing", Some(BASE.to_owned()), Want::Data, &[]),
        // The fields no code reads.
        (
            "a statement",
            edit("\"The second.\"", "\"The second, reworded.\""),
            Want::Data,
            &["REQ-QA-002", "statement"],
        ),
        (
            "a note",
            edit("\"A note.\"", "\"Another note.\""),
            Want::Data,
            &["REQ-QA-001", "note"],
        ),
        (
            "an area",
            edit(
                "  area: QA\n  statement: \"The third.\"",
                "  area: QB\n  statement: \"The third.\"",
            ),
            Want::Data,
            &["REQ-QA-003", "area"],
        ),
        (
            "a source's section",
            edit("\"Two\"", "\"Two and a half\""),
            Want::Data,
            &["REQ-QA-002", "source"],
        ),
        (
            "a ruling added",
            edit("  - R-1\n", "  - R-1\n  - R-9\n"),
            Want::Data,
            &["REQ-QA-001", "rulings"],
        ),
        (
            "an rq",
            edit("  rq: RQ-1\n", "  rq: RQ-2\n"),
            Want::Data,
            &["REQ-QA-002", "rq"],
        ),
        (
            "a statement and a note at once",
            edit("\"The first.\"", "\"The first, again.\"")
                .map(|t| t.replacen("\"A note.\"", "\"Note two.\"", 1)),
            Want::Data,
            &["REQ-QA-001"],
        ),
        // The fields code reads.
        (
            "a kind",
            edit("  kind: calibration\n", "  kind: definition\n"),
            Want::Code,
            &["REQ-QA-001", "kind"],
        ),
        (
            "a kind added",
            edit(
                "  area: QA\n  statement: \"The second.\"",
                "  area: QA\n  kind: calibration\n  statement: \"The second.\"",
            ),
            Want::Code,
            &["REQ-QA-002", "kind"],
        ),
        (
            "a retirement",
            edit("  rq: RQ-1\n", "  rq: RQ-1\n  retired: \"R-3: gone\"\n"),
            Want::Code,
            &["REQ-QA-002", "retired"],
        ),
        (
            "a retirement undone",
            edit("  retired: \"R-2: gone\"\n", ""),
            Want::Code,
            &["REQ-QA-003", "retired"],
        ),
        (
            "a milestone",
            edit("  milestone: M1\n", "  milestone: M3\n"),
            Want::Code,
            &["REQ-QA-002", "milestone"],
        ),
        (
            "a verify method",
            edit("    method: review checklist\n", "    method: unit test\n"),
            Want::Code,
            &["REQ-QA-002", "verify"],
        ),
        (
            "a verify detail",
            edit("\"cargo test -p qa three\"", "\"cargo test -p qa four\""),
            Want::Code,
            &["REQ-QA-003", "verify"],
        ),
        (
            "an id",
            edit("- id: REQ-QA-003\n", "- id: REQ-QA-009\n"),
            Want::Code,
            &["REQ-QA-00"],
        ),
        // Requirements added or removed.
        (
            "a requirement added",
            Some(format!("{BASE}{ADDED}")),
            Want::Code,
            &["REQ-QA-004"],
        ),
        (
            "a requirement removed",
            Some(BASE[..BASE.find("- id: REQ-QA-003").expect("third")].to_owned()),
            Want::Code,
            &["REQ-QA-003"],
        ),
        // Unknown, unparseable, missing.
        (
            "an unknown field",
            edit("  rq: RQ-1\n", "  rq: RQ-1\n  owner: qa\n"),
            Want::Code,
            &["REQ-QA-002", "owner"],
        ),
        (
            "a file that does not parse",
            Some(format!("{BASE}- id: [REQ-QA-005\n")),
            Want::Code,
            &[],
        ),
        (
            "a mapping, not a list",
            Some("REQ-QA-001:\n  statement: x\n".to_owned()),
            Want::Code,
            &[],
        ),
        (
            "an id twice",
            Some(format!(
                "{BASE}{}",
                ADDED.replace("REQ-QA-004", "REQ-QA-001")
            )),
            Want::Code,
            &[],
        ),
        ("an empty file", Some(String::new()), Want::Code, &[]),
        ("a missing head copy", None, Want::Code, &[]),
    ]
}

/// The verdict on a change of `paths` whose `plan/requirements.yaml` goes from BASE to `head` (`None`: no head copy),
/// the field check being `fields`.
fn classify_reqs(
    script: &str,
    fields: &str,
    paths: &[&str],
    base: &str,
    head: Option<&str>,
) -> Classified {
    let dir = Scratch::new("qa_r677_reqs");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let base_file = dir.join("base.yaml");
    let head_file = dir.join("head.yaml");
    let fields_file = dir.join("ci_req_fields.py");
    std::fs::write(&base_file, base).expect("base written");
    if let Some(head) = head {
        std::fs::write(&head_file, head).expect("head written");
    }
    std::fs::write(&fields_file, fields).expect("field check written");
    classify(
        script,
        paths,
        &[
            ("CI_REQS_BASE", base_file.as_path()),
            ("CI_REQS_HEAD", head_file.as_path()),
            ("CI_REQ_FIELDS", fields_file.as_path()),
        ],
    )
}

fn check_fields(script: &str, fields: &str) {
    let reqs = ["plan/requirements.yaml"];
    for (what, head, want, reason) in field_cases() {
        let got = classify_reqs(script, fields, &reqs, BASE, head.as_deref());
        let tier = if holds(&got.verdict, PLAN) {
            Some(Want::Data)
        } else if holds(&got.verdict, CODE) && holds(&got.verdict, &[("mutants", "false")]) {
            Some(Want::Code)
        } else {
            None
        };
        assert!(
            tier == Some(want),
            "qa_r677: plan/requirements.yaml changing {what} is {tier:?}, the ruling gives {want:?}: {:?}\n{}",
            got.verdict,
            got.stderr
        );
        // One reason line, naming the requirement and the field.
        let lines: Vec<&str> = got
            .stderr
            .lines()
            .filter(|l| l.contains("plan/requirements.yaml"))
            .collect();
        assert!(
            lines.len() == 1 && reason.iter().all(|w| lines[0].contains(w)),
            "qa_r677: plan/requirements.yaml changing {what}: the reason line does not name {reason:?}:\n{}",
            got.stderr
        );
    }

    // With no copies to compare, the file is code.
    let got = classify(script, &reqs, &[]).verdict;
    assert!(
        holds(&got, CODE),
        "qa_r677: plan/requirements.yaml with no copies is not code: {got:?}"
    );

    // A tier (b) edit beside other paths takes the highest tier.
    let head = BASE.replacen("\"The second.\"", "\"The second, reworded.\"", 1);
    for (paths, want) in [
        (&["plan/requirements.yaml", "docs/qa.md"][..], PLAN),
        (&["plan/tasks.yaml", "plan/requirements.yaml"][..], PLAN),
        (
            &["plan/requirements.yaml", "docs/gui/design/99_qa.png"][..],
            &[
                ("tier", "data"),
                ("plan_data", "true"),
                ("gui_data", "true"),
                ("gpu", "false"),
            ][..],
        ),
        (
            &["plan/requirements.yaml", "crates/kernel/src/qa.rs"][..],
            CODE,
        ),
        (&["xtask/src/qa.rs", "plan/requirements.yaml"][..], CODE),
    ] {
        let got = classify_reqs(script, fields, paths, BASE, Some(&head)).verdict;
        assert!(
            holds(&got, want),
            "qa_r677: a statement edit beside {paths:?} classifies {got:?}, the ruling gives {want:?}"
        );
    }

    // The real file: a statement edit is tier (b), a milestone edit is code.
    let real = read("plan/requirements.yaml");
    let quoted = "\n  statement: \"";
    let statement = real
        .find(quoted)
        .expect("qa_r677: the real file has no quoted statement")
        + quoted.len();
    let edited = format!("{}qa, {}", &real[..statement], &real[statement..]);
    let got = classify_reqs(script, fields, &reqs, &real, Some(&edited));
    assert!(
        holds(&got.verdict, PLAN),
        "qa_r677: a statement edit to the real plan/requirements.yaml is not tier (b): {:?}\n{}",
        got.verdict,
        got.stderr
    );
    let milestone = real
        .find("\n  milestone: M")
        .expect("qa_r677: no milestone")
        + "\n  milestone: ".len();
    let other = if &real[milestone..milestone + 2] == "M8" {
        "M7"
    } else {
        "M8"
    };
    let edited = format!("{}{}{}", &real[..milestone], other, &real[milestone + 2..]);
    let got = classify_reqs(script, fields, &reqs, &real, Some(&edited));
    assert!(
        holds(&got.verdict, CODE),
        "qa_r677: a milestone edit to the real plan/requirements.yaml is not code: {:?}\n{}",
        got.verdict,
        got.stderr
    );
}

#[test]
fn qa_r677_requirements_split_by_field() {
    check_fields(&script(), &fields_script());
}

negative_control!(
    qa_r677_requirements_split_by_field,
    "the field check with `kind` among the fields no code reads",
    expected = "plan/requirements.yaml changing a kind is Some(Data)",
    check_fields(
        &script(),
        &swap(
            &swap(&fields_script(), "    \"kind\",\n", ""),
            "DATA_FIELDS = {\"area\"",
            "DATA_FIELDS = {\"kind\", \"area\""
        )
    )
);

mod field_check_ignored {
    use super::*;
    negative_control!(
        qa_r677_requirements_split_by_field,
        "the script taking plan/requirements.yaml as data whatever the field check says",
        expected = "plan/requirements.yaml changing a kind is Some(Data)",
        check_fields(
            &swap(
                &script(),
                "  [ \"$1\" = plan/requirements.yaml ] && [ \"$reqs_data\" = true ]\n",
                "  [ \"$1\" = plan/requirements.yaml ] && [ -n \"${CI_REQS_HEAD:-}\" ]\n"
            ),
            &fields_script()
        )
    );
}

mod unknown_fields_cleared {
    use super::*;
    negative_control!(
        qa_r677_requirements_split_by_field,
        "the field check clearing a field on neither list",
        expected = "plan/requirements.yaml changing an unknown field is Some(Data)",
        check_fields(
            &script(),
            &swap(
                &fields_script(),
                "            if field not in DATA_FIELDS:\n",
                "            if False:\n"
            )
        )
    );
}

// ---- detect, on real merge commits --------------------------------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "user.name=qa",
            "-c",
            "user.email=qa@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .timed_output()
        .expect("qa_r677: git ran");
    assert!(
        out.status.success(),
        "qa_r677: git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn write(repo: &Path, rel: &str, text: &str) {
    let path = repo.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("written");
}

/// A repository whose `pr` branch changes `plan/requirements.yaml` from BASE to `head` (and writes `also`), merged
/// into a `main` that moved on with a requirements edit of its own elsewhere and code, as GitHub's merge commit is.
fn merge_repo(repo: &Path, head: &str, also: &[&str]) -> String {
    std::fs::create_dir_all(repo).expect("repo");
    git(repo, &["init", "-q"]);
    write(repo, "crates/kernel/src/lib.rs", "// base\n");
    write(repo, "plan/requirements.yaml", BASE);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["checkout", "-q", "-b", "pr"]);
    write(repo, "plan/requirements.yaml", head);
    for path in also {
        write(repo, path, "the pull request's\n");
    }
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", "pr"]);
    git(repo, &["checkout", "-q", "main"]);
    write(repo, "crates/kernel/src/lib.rs", "// main moved on\n");
    git(repo, &["commit", "-q", "-am", "main"]);
    git(repo, &["merge", "-q", "--no-ff", "-m", "merge", "pr"]);
    git(repo, &["rev-parse", "HEAD"])
}

/// `detect` on `sha` in `repo` as a pull request's run: the verdict it wrote to GITHUB_OUTPUT, and its log.
fn detect(script: &str, repo: &Path, sha: &str) -> (BTreeMap<String, String>, String) {
    let file = repo.join("ci_changes_under_test.sh");
    let output = repo.join("github_output");
    std::fs::write(&file, script).expect("script written");
    let _ = std::fs::remove_file(&output);
    let out = Command::new("bash")
        .arg(&file)
        .arg("detect")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GITHUB_EVENT_NAME", "pull_request")
        .env("GITHUB_SHA", sha)
        .env("GITHUB_OUTPUT", &output)
        .env("CI_REQ_FIELDS", root().join("scripts/ci_req_fields.py"))
        .env_remove("CI_REQS_BASE")
        .env_remove("CI_REQS_HEAD")
        .timed_output()
        .expect("qa_r677: bash ran");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "qa_r677: detect failed: {log}");
    let text = std::fs::read_to_string(&output).expect("qa_r677: detect wrote no GITHUB_OUTPUT");
    (parse_verdict(&text), log)
}

/// A `detect` case: what it is, the head's `plan/requirements.yaml`, the other paths it changes, what its verdict must
/// hold.
type DetectCase = (&'static str, String, &'static [&'static str], Holds);

fn check_detect(script: &str) {
    let dir = Scratch::new("qa_r677_detect");
    let cases: [DetectCase; 4] = [
        (
            "a kind edit",
            BASE.replacen("  kind: calibration\n", "  kind: definition\n", 1),
            &[],
            CODE,
        ),
        (
            "a statement edit",
            BASE.replacen("\"The first.\"", "\"The first, reworded.\"", 1),
            &[],
            PLAN,
        ),
        ("a requirement added", format!("{BASE}{ADDED}"), &[], CODE),
        (
            "a statement edit beside a crate's file",
            BASE.replacen("\"The first.\"", "\"The first, reworded.\"", 1),
            &["crates/render/src/qa.rs"],
            CODE,
        ),
    ];
    for (i, (what, head, also, want)) in cases.iter().enumerate() {
        let repo = dir.join(format!("r{i}"));
        let sha = merge_repo(&repo, head, also);
        let (got, log) = detect(script, &repo, &sha);
        assert!(
            holds(&got, want),
            "qa_r677: detect on {what} gives {got:?}, the ruling gives {want:?}\n{log}"
        );
        assert!(
            log.lines()
                .any(|l| l.contains("plan/requirements.yaml") && l.contains("REQ-QA-00")),
            "qa_r677: detect on {what} logs no reason line naming the requirement:\n{log}"
        );
    }
}

#[test]
fn qa_r677_detect_reads_the_requirements_at_base_and_head() {
    check_detect(&script());
}

negative_control!(
    qa_r677_detect_reads_the_requirements_at_base_and_head,
    "detect taking the head's copy from the base, so it compares the base with itself",
    expected = "detect on a kind edit gives",
    check_detect(&swap(
        &script(),
        "git show \"$head:plan/requirements.yaml\"",
        "git show \"$head^1:plan/requirements.yaml\""
    ))
);

// ---- The required checks on every tier ------------------------------------------------------------------------------

fn parse(yml: &str) -> Value {
    let dir = Scratch::new("qa_r677_yaml");
    std::fs::create_dir_all(&*dir).expect("scratch");
    let file = dir.join("wf.yml");
    std::fs::write(&file, yml).expect("written");
    let out = Command::new("python3")
        .args([
            "-c",
            "import json, sys, yaml\n\
             d = yaml.safe_load(open(sys.argv[1]))\n\
             if True in d: d['on'] = d.pop(True)\n\
             print(json.dumps(d))",
        ])
        .arg(&file)
        .timed_output()
        .expect("qa_r677: python3 ran");
    assert!(
        out.status.success(),
        "qa_r677: not YAML: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("qa_r677: JSON")
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

/// A finished run: each job's result, and each job's outputs.
#[derive(Default)]
struct Walk {
    results: BTreeMap<String, String>,
    outputs: BTreeMap<String, BTreeMap<String, String>>,
}

impl Walk {
    /// One operand: `needs.<job>.result`, `needs.<job>.outputs.<key>` or a quoted string. Anything else stops the test.
    fn term(&self, term: &str) -> String {
        let t = term.trim();
        if let Some(q) = t.strip_prefix('\'') {
            return q.trim_end_matches('\'').to_owned();
        }
        let parts: Vec<&str> = t.split('.').collect();
        match parts.as_slice() {
            ["needs", job, "result"] => self.results.get(*job).cloned().unwrap_or_default(),
            ["needs", job, "outputs", key] => self
                .outputs
                .get(*job)
                .and_then(|o| o.get(*key))
                .cloned()
                .unwrap_or_default(),
            // A step's status functions, in a run with no failed step, which is the run this walk is of.
            ["always()"] => "true".to_owned(),
            ["failure()"] => "false".to_owned(),
            _ => panic!("qa_r677: the walk does not know the expression `{t}`"),
        }
    }

    /// An expression's value: `a == b`, `a != b`, `&&` between them, or one operand.
    fn eval(&self, expr: &str) -> String {
        let e = expr
            .trim()
            .trim_start_matches("${{")
            .trim_end_matches("}}")
            .trim();
        assert!(
            !e.contains("||"),
            "qa_r677: the walk does not know the expression `{e}`"
        );
        let mut all = true;
        let mut value = String::new();
        for clause in e.split("&&") {
            value = if let Some((a, b)) = clause.split_once("==") {
                (self.term(a) == self.term(b)).to_string()
            } else if let Some((a, b)) = clause.split_once("!=") {
                (self.term(a) != self.term(b)).to_string()
            } else {
                self.term(clause)
            };
            all &= value == "true";
        }
        if e.contains("&&") {
            all.to_string()
        } else {
            value
        }
    }
}

/// `ci.yml` walked as GitHub runs it, `changes` outputting `verdict` (or failing, when it is in `fail`); each job in
/// `fail` fails. A job whose `needs` didn't all succeed is skipped unless its `if` holds `always()`; a matrix job counts
/// as one. A gate (a job of one step with `RESULTS` in its env) has its own script run under bash.
fn walk(yml: &Value, verdict: &[(&str, &str)], fail: &[&str]) -> Walk {
    let jobs = yml["jobs"].as_object().expect("qa_r677: jobs");
    let mut w = Walk::default();
    while w.results.len() < jobs.len() {
        let before = w.results.len();
        for (id, job) in jobs {
            let needs = needs_of(job);
            if w.results.contains_key(id) || needs.iter().any(|n| !w.results.contains_key(n)) {
                continue;
            }
            let cond = s(&job["if"]);
            let ok = needs.iter().all(|n| w.results[n] == "success");
            let runs = if cond.contains("always()") {
                true
            } else {
                ok && (cond.is_empty() || w.eval(&cond) == "true")
            };
            let result = if !runs {
                "skipped"
            } else if fail.contains(&id.as_str()) {
                "failure"
            } else if let Some(env) = job["steps"][0]
                .get("env")
                .filter(|e| e.get("RESULTS").is_some())
            {
                let results: Vec<&str> = needs.iter().map(|n| w.results[n].as_str()).collect();
                let mut cmd = Command::new("bash");
                cmd.args(["-e", "-c", &s(&job["steps"][0]["run"])]);
                cmd.env("RESULTS", results.join(" "));
                for (k, v) in env.as_object().expect("env") {
                    if k != "RESULTS" {
                        cmd.env(k, w.eval(&s(v)));
                    }
                }
                let out = cmd.timed_output().expect("qa_r677: bash ran");
                if out.status.success() {
                    "success"
                } else {
                    "failure"
                }
            } else {
                "success"
            };
            let mut outputs = BTreeMap::new();
            if id == "changes" {
                if result == "success" {
                    for (k, v) in verdict {
                        outputs.insert((*k).to_owned(), (*v).to_owned());
                    }
                }
            } else if result != "skipped" {
                if let Some(o) = job["outputs"].as_object() {
                    for (k, v) in o {
                        if s(v).contains("needs.") {
                            outputs.insert(k.clone(), w.eval(&s(v)));
                        }
                    }
                }
            }
            w.outputs.insert(id.clone(), outputs);
            w.results.insert(id.clone(), result.to_owned());
        }
        assert!(
            w.results.len() > before,
            "qa_r677: the jobs' `needs` never resolve"
        );
    }
    w
}

/// The result of the job reporting the check `name`, which must be exactly one job.
fn check_result(yml: &Value, w: &Walk, name: &str) -> String {
    let by: Vec<&String> = yml["jobs"]
        .as_object()
        .expect("jobs")
        .iter()
        .filter(|(id, j)| j.get("name").map_or(id.as_str() == name, |n| s(n) == name))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(by.len(), 1, "qa_r677: `{name}` is reported by {by:?}");
    w.results[by[0]].clone()
}

/// `ci.yml`'s required checks.
const GATES: [&str; 5] = ["ci", "xtask-ci", "gpu-kernel", "gpu-metal", "gpu-lavapipe"];

/// The status functions a step's `if` holds read as a run with no failed step, and any other expression still stops
/// the walk: it must not read as a false that hides a step.
fn check_walk_reads_status(walk: &Walk, unknown: &[&str]) {
    assert_eq!(walk.eval("always()"), "true", "qa_r677: `always()`");
    assert_eq!(walk.eval("failure()"), "false", "qa_r677: `failure()`");
    let mut outputs = BTreeMap::new();
    outputs.insert("docs_only".to_owned(), "false".to_owned());
    let mut seen = Walk::default();
    seen.outputs.insert("changes".to_owned(), outputs);
    let upload = "always() && needs.changes.outputs.docs_only != 'true'";
    assert_eq!(seen.eval(upload), "true", "qa_r677: `{upload}`");
    for expr in unknown {
        let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| walk.eval(expr)));
        assert!(read.is_err(), "qa_r677: the walk read `{expr}`");
    }
}

#[test]
fn qa_r677_walk_reads_status_and_refuses_the_rest() {
    check_walk_reads_status(
        &Walk::default(),
        &[
            "success()",
            "cancelled()",
            "hashFiles('x')",
            "always() && cancelled()",
        ],
    );
}

negative_control!(
    qa_r677_walk_reads_status_and_refuses_the_rest,
    "a walk taken to refuse what it knows",
    expected = "qa_r677: the walk read `failure()`",
    check_walk_reads_status(&Walk::default(), &["failure()"])
);

/// The commands `ci-workspace` runs on `verdict`, each step's `if:` evaluated.
fn workspace_runs(yml: &Value, verdict: &[(&str, &str)]) -> Vec<String> {
    let mut w = Walk::default();
    w.outputs.insert(
        "changes".to_owned(),
        verdict
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    );
    yml["jobs"]["ci-workspace"]["steps"]
        .as_array()
        .expect("qa_r677: ci-workspace's steps")
        .iter()
        .filter(|step| step.get("run").is_some())
        .filter(|step| step.get("if").is_none_or(|c| w.eval(&s(c)) == "true"))
        .map(|step| s(&step["run"]))
        .collect()
}

/// The `(package, test)` pairs the `cargo nextest run` commands of `runs` select with `-p` and `--test`.
fn selected(runs: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for run in runs.iter().filter(|r| r.starts_with("cargo nextest run")) {
        let words: Vec<&str> = run.split_whitespace().collect();
        let pkg = words
            .windows(2)
            .find(|w| w[0] == "-p")
            .map(|w| w[1].to_owned())
            .unwrap_or_default();
        for w in words.windows(2).filter(|w| w[0] == "--test") {
            out.push((pkg.clone(), w[1].to_owned()));
        }
    }
    out
}

/// A test binary's source: `xtask/tests/<test>.rs` or `crates/<package>/tests/<test>.rs`.
fn test_file(pkg: &str, test: &str) -> PathBuf {
    if pkg == "xtask" {
        root().join(format!("xtask/tests/{test}.rs"))
    } else {
        root().join(format!("crates/{pkg}/tests/{test}.rs"))
    }
}

/// The readers R-677 names for the plan's task data ("Tier (b)'s tests"): gate-report's and gate-provisional's,
/// reviews-check's, the plan checks', the progress banner's and the print-graph coverage in xtask; validation's Part 6
/// catalogue.
const PLAN_READERS: &[(&str, &str)] = &[
    ("xtask", "gate_report"),
    ("xtask", "gate_provisional"),
    ("xtask", "reviews_check"),
    ("xtask", "plan_check"),
    ("xtask", "progress_banner"),
    ("xtask", "qa_R-485_print_graph_coverage"),
    ("validation", "part6_catalogue"),
];

/// The readers R-677 names for the GUI's artboards and references: xtask's screenshot tests and the print-graph
/// coverage, kernel's initial-conditions inspector and render's colour explorer.
const GUI_READERS: &[(&str, &str)] = &[
    ("xtask", "screenshot"),
    ("xtask", "qa_R-485_print_graph_coverage"),
    ("kernel", "qa_TASK-M2-08"),
    ("render", "qa_TASK-M1-03"),
];

/// Each tier's verdict, as the classifier gives it.
fn verdicts() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let v = |tier, docs_only, gpu, plan, gui, mutants| {
        vec![
            ("tier", tier),
            ("docs_only", docs_only),
            ("gpu", gpu),
            ("plan_data", plan),
            ("gui_data", gui),
            ("mutants", mutants),
        ]
    };
    vec![
        ("code", v("code", "false", "true", "false", "false", "true")),
        (
            "code with no GPU path",
            v("code", "false", "false", "false", "false", "true"),
        ),
        (
            "the plan's task data",
            v("data", "true", "false", "true", "false", "false"),
        ),
        (
            "the GUI's data",
            v("data", "true", "false", "false", "true", "false"),
        ),
        (
            "both kinds of data",
            v("data", "true", "false", "true", "true", "false"),
        ),
        (
            "docs",
            v("docs", "true", "false", "false", "false", "false"),
        ),
    ]
}

fn check_required(ci: &str) {
    let yml = parse(ci);
    for (what, verdict) in verdicts() {
        let w = walk(&yml, &verdict, &[]);
        for gate in GATES {
            assert_eq!(
                check_result(&yml, &w, gate),
                "success",
                "qa_r677: the required check `{gate}` is not a success on {what}"
            );
        }
        let tier = verdict[0].1;
        let runs = workspace_runs(&yml, &verdict);
        let picked = selected(&runs);
        for (pkg, test) in &picked {
            assert!(
                test_file(pkg, test).is_file(),
                "qa_r677: on {what}, `ci-workspace` selects `-p {pkg} --test {test}`, which is no test file"
            );
        }
        if tier == "code" {
            assert!(
                runs.iter()
                    .any(|r| r.trim() == "cargo nextest run -p xtask"),
                "qa_r677: on {what}, `ci-workspace` does not run xtask's suite:\n{runs:#?}"
            );
            continue;
        }
        // No full CI: nothing but the classification, `ci-workspace` and the gates runs.
        let ran: Vec<&str> = w
            .results
            .iter()
            .filter(|(_, r)| r.as_str() != "skipped")
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(
            ran,
            vec![
                "changes",
                "ci-gate",
                "ci-workspace",
                "gpu-kernel-gate",
                "gpu-lavapipe-gate",
                "gpu-metal-gate",
                "xtask-ci-gate"
            ],
            "qa_r677: {what} ran other jobs"
        );
        for want in ["python3 plan/check_plan.py", "cargo xtask lint vocab"] {
            assert!(
                runs.iter().any(|r| r.trim() == want),
                "qa_r677: on {what}, `ci-workspace` does not run `{want}`:\n{runs:#?}"
            );
        }
        // Only the readers, each test binary alone: no workspace, no package's whole suite, no build or doctest.
        for run in &runs {
            for never in [
                "--workspace",
                "cargo test",
                "cargo build",
                "clippy",
                "cargo doc",
                "cargo xtask ci",
            ] {
                assert!(
                    !run.contains(never),
                    "qa_r677: on {what}, `ci-workspace` runs `{run}`"
                );
            }
            if run.starts_with("cargo nextest run") {
                assert!(
                    run.contains("--test "),
                    "qa_r677: on {what}, `ci-workspace` runs a whole suite: `{run}`"
                );
            }
        }
        let plan = verdict.contains(&("plan_data", "true"));
        let gui = verdict.contains(&("gui_data", "true"));
        let mut want: Vec<(&str, &str)> = Vec::new();
        if plan {
            want.extend(PLAN_READERS);
        }
        if gui {
            want.extend(GUI_READERS);
        }
        for (pkg, test) in &want {
            assert!(
                picked.iter().any(|(p, t)| p == pkg && t == test),
                "qa_r677: on {what}, `ci-workspace` does not run `-p {pkg} --test {test}`:\n{runs:#?}"
            );
        }
        if !plan && !gui {
            assert!(
                picked.is_empty() && !runs.iter().any(|r| r.contains("nextest")),
                "qa_r677: on {what}, `ci-workspace` runs tests:\n{runs:#?}"
            );
        }
        // A failing reader is a red `ci`, never hidden by the skips.
        let red = walk(&yml, &verdict, &["ci-workspace"]);
        assert_eq!(
            check_result(&yml, &red, "ci"),
            "failure",
            "qa_r677: `ci` passes a failed `ci-workspace` on {what}"
        );
    }
    // A classification that failed is no data verdict: every gate is red.
    let lost = walk(&yml, &verdicts()[2].1, &["changes"]);
    for gate in GATES {
        assert_eq!(
            check_result(&yml, &lost, gate),
            "failure",
            "qa_r677: the required check `{gate}` is not red when the classification failed"
        );
    }
}

#[test]
fn qa_r677_required_checks_on_every_tier() {
    check_required(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa_r677_required_checks_on_every_tier,
    "ci.yml running the plan's task data's xtask readers only on code",
    expected = "on the plan's task data, `ci-workspace` does not run `-p xtask --test gate_report`",
    check_required(
        &swap(
            &read(".github/workflows/ci.yml"),
            "        if: needs.changes.outputs.tier == 'data' && needs.changes.outputs.plan_data == 'true'\n        env:\n          NEXTEST_PROFILE: ci\n        run: cargo nextest run -p xtask",
            "        if: needs.changes.outputs.tier == 'code' && needs.changes.outputs.plan_data == 'true'\n        env:\n          NEXTEST_PROFILE: ci\n        run: cargo nextest run -p xtask"
    ))
);

mod checks_on_data {
    use super::*;
    negative_control!(
        qa_r677_required_checks_on_every_tier,
        "ci.yml running `ci-checks`, the build and lints, on every tier",
        expected = "the plan's task data ran other jobs",
        check_required(&swap(
            &read(".github/workflows/ci.yml"),
            "  ci-checks:\n    needs: changes\n    if: needs.changes.outputs.docs_only != 'true'\n",
            "  ci-checks:\n    needs: changes\n"
        ))
    );
}

// ---- The mutants gate's self-test ----------------------------------------------------------------------------------

/// Verdicts `mutants-fixture` must run its work on, each with what it is: every code verdict, one with no Rust under
/// `crates/` among them (the gate's own configuration is code, "full CI as now"), tier (b)'s data (code before R-677,
/// and still run as code was, by R-677 as amended), and a classification that says nothing (failing toward full CI).
fn fixture_verdicts() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let mut out = verdicts();
    out.insert(
        0,
        (
            "code with no Rust under crates/",
            vec![
                ("tier", "code"),
                ("docs_only", "false"),
                ("gpu", "true"),
                ("plan_data", "false"),
                ("gui_data", "false"),
                ("mutants", "false"),
            ],
        ),
    );
    out.push((
        "a classification that says nothing",
        vec![
            ("tier", ""),
            ("docs_only", ""),
            ("gpu", ""),
            ("plan_data", ""),
            ("gui_data", ""),
            ("mutants", ""),
        ],
    ));
    out
}

/// The gate's own files, each alone, as a pull request would change them: the owner's tier (a) ".cargo/, fixtures/,
/// … contain.sh" and the workflow and xtask code that run the gate. None is Rust under `crates/`.
const GATE_FILES: &[&str] = &[
    ".cargo/mutants.toml",
    "fixtures/mutants/untested/change.diff",
    "fixtures/mutants/untested/equivalent.toml",
    "fixtures/mutants/caps/src/lib.rs",
    "scripts/contain.sh",
    ".github/workflows/mutants.yml",
    "xtask/src/mutants_check.rs",
];

/// `mutants.yml`'s `mutants-fixture` steps after its classification, on `verdict`: each step's name and whether it
/// runs, every `steps.changes.outputs.<key>` in its `if:` read from `verdict`. The artifact upload, which runs whatever
/// happened and costs nothing, is left out.
fn fixture_runs(yml: &Value, verdict: &[(&str, &str)]) -> Vec<(String, bool)> {
    let steps = yml["jobs"]["mutants-fixture"]["steps"]
        .as_array()
        .expect("qa_r677: mutants-fixture's steps");
    let after = steps
        .iter()
        .position(|st| s(&st["id"]) == "changes")
        .expect("qa_r677: mutants-fixture classifies no change");
    let mut out = Vec::new();
    for st in &steps[after + 1..] {
        if s(&st["uses"]).starts_with("actions/upload-artifact") {
            continue;
        }
        let mut cond = s(&st["if"]);
        for (k, v) in verdict {
            cond = cond.replace(&format!("steps.changes.outputs.{k}"), &format!("'{v}'"));
        }
        let runs = cond.trim().is_empty() || Walk::default().eval(&cond) == "true";
        out.push((format!("{}{}", s(&st["name"]), s(&st["uses"])), runs));
    }
    assert!(
        out.len() >= 2,
        "qa_r677: mutants-fixture has no work after its classification"
    );
    out
}

/// `mutants-fixture` skips its work only on a docs verdict, and runs all of it on any other; the classifier, `script`,
/// gives each of the gate's own files a verdict it runs on, and a docs file one it skips on.
fn check_fixture(mutants: &str, script: &str) {
    let yml = parse(mutants);
    for (what, verdict) in fixture_verdicts() {
        let docs = verdict.contains(&("tier", "docs"));
        for (step, runs) in fixture_runs(&yml, &verdict) {
            assert_eq!(
                runs, !docs,
                "qa_r677: mutants-fixture's step `{step}` on {what}: runs={runs}, R-677 gives runs={}",
                !docs
            );
        }
    }
    for path in GATE_FILES {
        let got = classify(script, &[path], &[]).verdict;
        let verdict: Vec<(&str, &str)> =
            got.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        for (step, runs) in fixture_runs(&yml, &verdict) {
            assert!(
                runs,
                "qa_r677: mutants-fixture skips `{step}` on a change to the gate's own `{path}` ({got:?})"
            );
        }
    }
    let got = classify(script, &["docs/qa.md"], &[]).verdict;
    let verdict: Vec<(&str, &str)> = got.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    for (step, runs) in fixture_runs(&yml, &verdict) {
        assert!(
            !runs,
            "qa_r677: mutants-fixture runs `{step}` on a docs change"
        );
    }
}

#[test]
fn qa_r677_mutants_fixture_skips_only_on_docs() {
    check_fixture(&read(".github/workflows/mutants.yml"), &script());
}

/// `mutants.yml` with each `mutants-fixture` step's condition replaced by `to`; at least two such steps, so the control
/// cannot pass vacuously.
#[cfg(feature = "controls")]
fn fixture_gated(to: &str) -> String {
    let text = read(".github/workflows/mutants.yml");
    let from = "        if: steps.changes.outputs.tier != 'docs'\n";
    assert!(
        text.matches(from).count() >= 2,
        "qa_r677: the control's edit target is gone: {from}"
    );
    text.replace(from, &format!("        if: {to}\n"))
}

mod fixture_on_crates_rust {
    use super::*;
    negative_control!(
        qa_r677_mutants_fixture_skips_only_on_docs,
        "mutants.yml's fixture steps running only on Rust under crates/, as R-677 was first written",
        expected = "on code with no Rust under crates/",
        check_fixture(
            &fixture_gated("steps.changes.outputs.mutants != 'false'"),
            &script()
        )
    );
}

mod fixture_on_code_only {
    use super::*;
    negative_control!(
        qa_r677_mutants_fixture_skips_only_on_docs,
        "mutants.yml's fixture steps running only on tier (a)",
        expected = "on the plan's task data",
        check_fixture(
            &fixture_gated("steps.changes.outputs.tier == 'code'"),
            &script()
        )
    );
}

mod fixture_on_docs_too {
    use super::*;
    negative_control!(
        qa_r677_mutants_fixture_skips_only_on_docs,
        "mutants.yml's fixture steps running on every change",
        expected = "on docs: runs=true",
        check_fixture(
            &fixture_gated("steps.changes.outputs.tier != 'none'"),
            &script()
        )
    );
}

mod fixture_skips_unknown {
    use super::*;
    negative_control!(
        qa_r677_mutants_fixture_skips_only_on_docs,
        "mutants.yml's fixture steps skipping a classification that says nothing",
        expected = "on a classification that says nothing",
        check_fixture(
            &fixture_gated(
                "steps.changes.outputs.tier != 'docs' && steps.changes.outputs.tier != ''"
            ),
            &script()
        )
    );
}

mod contain_as_docs {
    use super::*;
    negative_control!(
        qa_r677_mutants_fixture_skips_only_on_docs,
        "the classifier taking scripts/contain.sh for docs",
        expected = "on a change to the gate's own `scripts/contain.sh`",
        check_fixture(
            &read(".github/workflows/mutants.yml"),
            &swap(&script(), "    scripts/contain.sh) return 0 ;;\n", "")
        )
    );
}
