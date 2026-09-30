//! qa's tests for TASK-M0-38, written from its requirements (R-267):
//!
//! - REQ-SYS-069 (R-212): "`parse_wrong_panics` must read a control's section only up to the next libtest section
//!   header at the start of a line, so a `---- x stdout ----` string inside a control's own output is not taken as a
//!   new section"; verify: "a control whose output contains an embedded `---- x stdout ----` line is still attributed
//!   to its own control, with its full note". Checked end to end: `xtask controls` runs on a copy of the fixture
//!   `fixtures/controls_qa_m0_38/embedded_header` (see its `src/lib.rs`), whose controls print a child's libtest
//!   report, header and wrong-panic note included, and each finding is read from real libtest output.
//! - REQ-SYS-071 (R-180): "`cargo xtask pr-check` must refuse a `- meter:` or `- discriminator:` line that names
//!   nothing before its dash"; verify: "a validation body whose only meter line is `- meter: — COM drift` fails naming
//!   the line; one with a name passes". Checked through the `xtask pr-check --event` binary, on event files.
//!
//! Each test has a negative control (R-176, R-212).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use validation::negative_control;
use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/own_target.rs"]
mod own_target;
use own_target::{fixture_files, Lease, FIXTURES};

// ---------------------------------------------------------------------------------------------------------------
// REQ-SYS-069

/// The child's note, embedded in the controls' output: never a control's own.
const DECOY: &str = "QA_M038_DECOY";
/// `embeds`'s own panic, printed after the child's report.
const EMBEDS_OWN: &str = "QA_M038_EMBEDS_OWN_PANIC";
/// `plain`'s own panic, with nothing embedded.
const PLAIN_OWN: &str = "QA_M038_PLAIN_OWN_PANIC";
/// The message every control of the fixture expects.
const EXPECTED: &str = "not the double of 3";
/// The words of a `WrongPanic` finding (R-212).
const WRONG_PANIC: &str = "panicked without its expected message";

/// The outcome of the one `xtask controls` run on the fixture.
struct Verdict {
    ok: bool,
    stderr: String,
}

/// `xtask controls --manifest-path` on a copy of the fixture, outside the workspace, with the `validation` path made
/// absolute, in its fixture type's directory, held while it runs (REQ-VAL-164, R-270); run once per process and
/// shared.
fn verdict() -> &'static Verdict {
    static RUN: OnceLock<Verdict> = OnceLock::new();
    RUN.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut files = fixture_files(&root.join("fixtures/controls_qa_m0_38/embedded_header"));
        for (path, bytes) in &mut files {
            if path == Path::new("Cargo.toml") {
                *bytes = String::from_utf8_lossy(bytes)
                    .replace(
                        "../../../crates/validation",
                        root.join("crates/validation").to_str().unwrap(),
                    )
                    .into_bytes();
            }
        }
        files.push((
            PathBuf::from("Cargo.lock"),
            std::fs::read(root.join("Cargo.lock")).unwrap(),
        ));
        // The copy is kept across runs and written only where it changed (R-231, REQ-VAL-165).
        let target = Lease::take(FIXTURES, Some("controls_qa_m0_38"));
        let manifest = target.copy("embedded_header", &files).join("Cargo.toml");
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["controls", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", target.dir())
            .timed_output()
            .expect("run xtask controls");
        Verdict {
            ok: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    })
}

/// The first line of the finding naming `test`: the finding itself, with its note, before the control's output.
fn finding_line<'a>(v: &'a Verdict, test: &str) -> &'a str {
    assert!(
        !v.ok,
        "controls that do not make their tests fail passed the command:\n{}",
        v.stderr
    );
    let head = format!("xtask controls: qa_controls_m038: test `{test}`");
    v.stderr
        .lines()
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("no finding names test `{test}`:\n{}", v.stderr))
}

/// REQ-SYS-069: the finding for `test` is a wrong panic whose note is the control's own: its panic `own` and the
/// expected substring, and nothing of the child's note it embeds.
fn check_own_note(test: &str, own: &str) {
    let v = verdict();
    let line = finding_line(v, test);
    assert!(
        line.contains(WRONG_PANIC) && line.contains(own) && line.contains(EXPECTED),
        "the finding for `{test}` does not carry its control's own note ({own}, {EXPECTED:?}):\n{line}\n\n\
         full stderr:\n{}",
        v.stderr
    );
    assert!(
        !line.contains(DECOY),
        "the finding for `{test}` carries the embedded child's note:\n{line}"
    );
}

/// The control of `embeds` prints a child's report, `---- x stdout ----` header and note included, then panics
/// without its expected message: its finding carries its own whole note.
#[test]
fn qa_m038_note_after_an_embedded_header_is_the_controls_own() {
    check_own_note("embeds", EMBEDS_OWN);
}

negative_control!(
    qa_m038_note_after_an_embedded_header_is_the_controls_own,
    "another control's panic, which `embeds`'s note must not be",
    expected = "does not carry its control's own note",
    check_own_note("embeds", PLAIN_OWN)
);

/// The same, for a control with nothing embedded: the embedded case is not read at the plain case's expense.
#[test]
fn qa_m038_note_with_nothing_embedded_is_the_controls_own() {
    check_own_note("plain", PLAIN_OWN);
}

negative_control!(
    qa_m038_note_with_nothing_embedded_is_the_controls_own,
    "another control's panic, which `plain`'s note must not be",
    expected = "does not carry its control's own note",
    check_own_note("plain", EMBEDS_OWN)
);

/// The finding for `test` is not a wrong panic: its control did not panic, and the only wrong-panic note in its
/// section is the child's it embeds.
fn check_no_wrong_panic(test: &str) {
    let v = verdict();
    let line = finding_line(v, test);
    assert!(
        !line.contains(WRONG_PANIC) && !line.contains(DECOY),
        "the embedded child's note was taken as `{test}`'s control's own:\n{line}"
    );
}

/// The control of `decoy` prints the child's report and leaves its test passing: the child's note is not its own.
#[test]
fn qa_m038_embedded_note_alone_is_not_the_controls() {
    check_no_wrong_panic("decoy");
}

negative_control!(
    qa_m038_embedded_note_alone_is_not_the_controls,
    "a control that did panic without its message, whose finding must be a wrong panic",
    expected = "the embedded child's note was taken as",
    check_no_wrong_panic("embeds")
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-SYS-071

/// `xtask pr-check --event` on a `validation`-labelled PR whose `## Validation record` holds `record`. Returns whether
/// it passed, and its stderr.
fn pr_check(record: &str) -> (bool, String) {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qa_m0_38_pr_check");
    std::fs::create_dir_all(&dir).unwrap();
    let event = dir.join(format!(
        "event-{}-{}.json",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    let body = format!("## Summary\nA change.\n\n## Validation record\n{record}\n");
    let json = serde_json::json!({
        "pull_request": {"body": body, "labels": [{"name": "validation"}]}
    });
    std::fs::write(&event, json.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["pr-check", "--event"])
        .arg(&event)
        .env_remove("GITHUB_EVENT_PATH")
        .timed_output()
        .expect("xtask pr-check ran");
    let _ = std::fs::remove_file(&event);
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// REQ-SYS-071: the record fails, and a problem quotes `line` (trimmed) and says it names no `kind`.
fn fails_quoting(record: &str, line: &str, kind: &str) {
    let (ok, stderr) = pr_check(record);
    let quoted = format!("`{}`", line.trim());
    let named = stderr
        .lines()
        .any(|problem| problem.contains(&quoted) && problem.contains(&format!("names no {kind}")));
    assert!(
        !ok && named,
        "pr-check did not refuse the nameless {kind} line {quoted} (passed: {ok}):\n{stderr}"
    );
}

/// REQ-SYS-071: the record passes.
fn passes(record: &str) {
    let (ok, stderr) = pr_check(record);
    assert!(ok, "pr-check refused a record naming its meter:\n{stderr}");
}

/// The verify detail's case: the only meter line is `- meter: — COM drift`.
#[test]
fn qa_m038_only_meter_line_nameless_fails_naming_it() {
    fails_quoting("- meter: — COM drift", "- meter: — COM drift", "meter");
}

negative_control!(
    qa_m038_only_meter_line_nameless_fails_naming_it,
    "the same line with its meter named, which pr-check must not refuse",
    expected = "pr-check did not refuse the nameless meter line",
    fails_quoting(
        "- meter: COM drift — the centre of mass the occupant integrates",
        "- meter: COM drift — the centre of mass the occupant integrates",
        "meter"
    )
);

/// The same for a discriminator, and for names of blanks only, no space before the dash, or indentation: each still
/// names nothing before its dash.
#[test]
fn qa_m038_nameless_lines_of_every_form_fail_naming_them() {
    for (line, kind) in [
        (
            "- discriminator: — d_min grows as the step shrinks",
            "discriminator",
        ),
        ("- meter:    — COM drift", "meter"),
        ("- meter:— COM drift", "meter"),
        ("  - meter: — COM drift", "meter"),
    ] {
        fails_quoting(line, line, kind);
    }
}

negative_control!(
    qa_m038_nameless_lines_of_every_form_fail_naming_them,
    "a discriminator line with its name, which pr-check must not refuse",
    expected = "pr-check did not refuse the nameless discriminator line",
    fails_quoting(
        "- discriminator: d_min — grows as the step shrinks",
        "- discriminator: d_min — grows as the step shrinks",
        "discriminator"
    )
);

/// A named meter beside a nameless one does not carry it: the record fails on the nameless line.
#[test]
fn qa_m038_named_meter_does_not_carry_a_nameless_one() {
    fails_quoting(
        "- meter: COM drift — the centre of mass the occupant integrates\n- meter: — energy drift",
        "- meter: — energy drift",
        "meter",
    );
}

negative_control!(
    qa_m038_named_meter_does_not_carry_a_nameless_one,
    "both meters named, which pr-check must not refuse",
    expected = "pr-check did not refuse the nameless meter line",
    fails_quoting(
        "- meter: COM drift — the centre of mass the occupant integrates\n- meter: energy — energy drift",
        "- meter: energy — energy drift",
        "meter"
    )
);

/// The verify detail's passing case: the meter and discriminator named, each with its statement.
#[test]
fn qa_m038_named_meter_and_discriminator_pass() {
    passes(
        "- meter: COM drift — the centre of mass the occupant integrates\n\
         - discriminator: d_min — grows as the step shrinks, so it depends on termination",
    );
}

negative_control!(
    qa_m038_named_meter_and_discriminator_pass,
    "the verify detail's nameless meter line, which pr-check must refuse",
    expected = "pr-check refused a record naming its meter",
    passes("- meter: — COM drift")
);
