//! qa's tests for TASK-M0-34, written from REQ-VAL-167 (R-236, R-244) and philosophy §4.4: "When a registered control
//! fails to make its test fail, `xtask/src/controls.rs`'s finding (`ControlPasses` / `WrongPanic`) must include that
//! control's own output, and must not say it \"leaves it passing\""; verify: "a scratch control that leaves its test
//! passing, and one that fails it with the wrong panic, each show that control's output in `cargo xtask controls`'s
//! finding".
//!
//! Each test runs `xtask controls` on one copy of the fixture `fixtures/controls_qa_m0_34/output_kept`, whose three
//! controls each fail to make their test fail and each write a marker of their own (see its `src/lib.rs`), and looks
//! for that marker in the finding naming that test: the lines from `xtask controls: <crate>: test `<name>`` up to
//! the next line xtask starts. Each has a negative control (R-176): the same check for the marker of another
//! control, which must not be in that finding, so the check tells one control's output from another's.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use validation::spawn::Spawn;

#[path = "../../crates/validation/tests/support/own_target.rs"]
mod own_target;
use own_target::{Lease, FIXTURES};

const LEAKY: &str = "QA_M034_LEAKY_MARKER";
const WRONG: &str = "QA_M034_WRONG_PANIC_MARKER";
const AFTER: &str = "QA_M034_AFTER_MARKER";

/// The outcome of the one `xtask controls` run on the fixture.
struct Verdict {
    ok: bool,
    stderr: String,
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).unwrap();
        }
    }
}

/// `xtask controls --manifest-path` on a copy of the fixture, outside the workspace, with the `validation` path made
/// absolute and a target directory of its own (REQ-VAL-164); run once per process and shared.
fn verdict() -> &'static Verdict {
    static RUN: OnceLock<Verdict> = OnceLock::new();
    RUN.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("qa_m0_34")
            .join(format!("output_kept-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&copy);
        copy_dir(&root.join("fixtures/controls_qa_m0_34/output_kept"), &copy);
        let manifest = copy.join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap().replace(
            "../../../crates/validation",
            root.join("crates/validation").to_str().unwrap(),
        );
        std::fs::write(&manifest, text).unwrap();
        std::fs::copy(root.join("Cargo.lock"), copy.join("Cargo.lock")).unwrap();
        let target = Lease::take(FIXTURES);
        let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .args(["controls", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", target.dir())
            .timed_output()
            .expect("run xtask controls");
        let _ = std::fs::remove_dir_all(&copy);
        Verdict {
            ok: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    })
}

/// The finding naming `test`: its line and the lines after it, up to the next line xtask starts.
fn finding(stderr: &str, test: &str) -> Option<String> {
    let head = format!("xtask controls: qa_controls_m034: test `{test}`");
    let mut lines = stderr.lines().skip_while(|line| !line.starts_with(&head));
    let first = lines.next()?;
    let rest = lines
        .take_while(|line| !line.starts_with("xtask controls:") && !line.starts_with("xtask:"));
    Some(
        std::iter::once(first)
            .chain(rest)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// REQ-VAL-167: the command fails, its finding for `test` holds `marker` (from that control's own output) and does
/// not say "leaves it passing".
fn check_output_in_finding(v: &Verdict, test: &str, marker: &str) {
    assert!(
        !v.ok,
        "controls that do not make their tests fail passed the command:\n{}",
        v.stderr
    );
    let Some(found) = finding(&v.stderr, test) else {
        panic!("no finding names test `{test}`:\n{}", v.stderr);
    };
    assert!(
        found.contains(marker),
        "the control's output ({marker}) is not in the finding for `{test}`:\n{found}\n\nfull stderr:\n{}",
        v.stderr
    );
    assert!(
        !found.contains("leaves it passing"),
        "the finding for `{test}` says \"leaves it passing\":\n{found}"
    );
}

/// A control that leaves its test passing (`ControlPasses`): what it printed is in the finding.
#[test]
fn qa_m034_leaky_control_output_is_in_its_finding() {
    check_output_in_finding(verdict(), "leaks", LEAKY);
}

/// A control that panics without its expected message (`WrongPanic`, R-212): its panic is in the finding, which
/// says so.
#[test]
fn qa_m034_wrong_panic_control_output_is_in_its_finding() {
    let v = verdict();
    check_output_in_finding(v, "misfires", WRONG);
    let found = finding(&v.stderr, "misfires").unwrap();
    assert!(
        found.contains("panicked without its expected message"),
        "the finding for `misfires` does not name the wrong panic:\n{found}"
    );
}

/// A control whose output holds a line `failures:` (a child's libtest report embedded in it): the output after that
/// line is the control's own too, and is in the finding.
#[test]
fn qa_m034_control_output_after_a_failures_line_is_in_its_finding() {
    check_output_in_finding(verdict(), "nests", AFTER);
}

validation::negative_control!(
    qa_m034_leaky_control_output_is_in_its_finding,
    "another control's marker, which the leaky control's finding must not hold",
    expected =
        "the control's output (QA_M034_WRONG_PANIC_MARKER) is not in the finding for `leaks`",
    check_output_in_finding(verdict(), "leaks", WRONG)
);

validation::negative_control!(
    qa_m034_wrong_panic_control_output_is_in_its_finding,
    "another control's marker, which the wrong-panic control's finding must not hold",
    expected = "the control's output (QA_M034_LEAKY_MARKER) is not in the finding for `misfires`",
    check_output_in_finding(verdict(), "misfires", LEAKY)
);

validation::negative_control!(
    qa_m034_control_output_after_a_failures_line_is_in_its_finding,
    "another control's marker, which the nested control's finding must not hold",
    expected = "the control's output (QA_M034_LEAKY_MARKER) is not in the finding for `nests`",
    check_output_in_finding(verdict(), "nests", LEAKY)
);
