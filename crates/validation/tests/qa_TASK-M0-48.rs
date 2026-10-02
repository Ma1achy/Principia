//! QA tests for TASK-M0-48, written from REQ-VAL-178 (R-342, R-359).
//!
//! - R-342: "A test's scratch folder or scratch file must be deleted when the test passes and kept, its path in the
//!   failure output, when the test fails." Checked on a child test run as libtest runs it by default (output captured),
//!   so the path must be in libtest's own failure report, not only on a `--nocapture` stderr.
//! - R-359: "A negative control passes only when it panics with its expected message (R-212): `negative_control!`
//!   catches the control's panic, compares the message with its expected one, deletes the control's scratch only on a
//!   match, and resumes the panic so libtest's verdict is unchanged; a control that panics with the wrong message keeps
//!   its scratch and its path is printed." The comparison is the one `#[should_panic(expected = …)]` makes: the
//!   message, a `&str` or a `String`, contains the expected text; a payload that is not a string matches nothing. A
//!   control that does not panic keeps its scratch too (the task's RQ-194 A4 note). Every guard made in the control,
//!   folder or file, live at the panic or dropped before it, is settled by the match.
//!
//! Each test registers its negative control (R-176). The R-359 test's control runs the same check against a stand-in
//! for #114's first design, a control wrapper that deletes the scratch on any panic (the task's acceptance line).
//!
//! `negative_control!` compiles only under `controls`, which `cargo mutants` does not enable, so a mutant in what the
//! macro calls survives every test above (as `delete ! in run` did on #114's mutants job). The last test checks the
//! same R-359 rule on `validation::control::run`, the function the macro expands to, in every build.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::spawn::Spawn;

#[path = "support/scratch.rs"]
mod scratch;
use scratch::{settle, Scratch};

/// Set in a child run: the file it appends each scratch path it makes to, one a line.
const RECORD: &str = "QA48_RECORD";

/// Set in a child run: what its body does (see [`subject`]).
const MODE: &str = "QA48_MODE";

/// Set in a child run of [`qa_m048_control_scratch_kept_unless_message_matches`] itself: the body runs under the
/// stand-in for #114's first design, which deletes the scratch on any panic.
#[cfg(feature = "controls")]
const MOCK: &str = "QA48_MOCK";

/// The R-359 control's expected message, which its matching child modes contain.
#[cfg(feature = "controls")]
const EXPECTED: &str = "control's scratch was deleted";

/// Appends `path` to the child's record.
fn record(path: &Path) {
    use std::io::Write;
    let file = std::env::var_os(RECORD).expect("a child run has a record");
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
        .expect("the record opened");
    writeln!(f, "{}", path.to_str().expect("a UTF-8 path")).expect("the path recorded");
}

/// A child run of test `name` of this binary (`--exact`, output captured as libtest captures it by default), with
/// `env` set and a fresh record. Returns whether libtest passed it, its stdout and stderr together, and the scratch
/// paths it recorded. The parent holds the record's own scratch until it returns.
fn run_child(name: &str, env: &[(&str, &str)]) -> (bool, String, Vec<PathBuf>) {
    let dir = Scratch::new("qa_m048_record");
    std::fs::create_dir_all(&dir).expect("the record's folder is made");
    let record = dir.join("paths");
    let mut cmd = Command::new(std::env::current_exe().expect("the test binary"));
    cmd.args([name, "--exact", "--test-threads", "1"])
        .env(RECORD, &record);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.timed_output().expect("the child test ran");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let paths = std::fs::read_to_string(&record)
        .unwrap_or_else(|_| panic!("qa48: the child made no scratch:\n{text}"))
        .lines()
        .map(PathBuf::from)
        .collect();
    (out.status.success(), text, paths)
}

// ---------------------------------------------------------------------------------------------------------------
// R-342: a failing test's scratch is kept, its path in libtest's failure report.

/// The child's body for [`qa_m048_failing_tests_scratch_path_in_failure_output`]: a scratch folder holding a file,
/// then a panic, with the guard live (`held`) or dropped first (`dropped`, the control's).
fn failing_body(mode: &str) {
    let folder = Scratch::new("qa_m048_failing");
    std::fs::create_dir_all(folder.join("sub")).expect("the scratch folder is made");
    std::fs::write(folder.join("sub/f"), "f").expect("a file in it is written");
    record(&folder);
    if mode == "dropped" {
        drop(folder);
        panic!("qa48 child: the body fails after dropping its guard");
    }
    panic!("qa48 child: the body fails with its guard live");
}

/// A child test whose body fails keeps its scratch folder, with its contents, and its path is in libtest's report.
fn check_failing_keeps(mode: &str) {
    let (passed, text, paths) = run_child(
        "qa_m048_failing_tests_scratch_path_in_failure_output",
        &[(MODE, mode)],
    );
    let [made] = paths.as_slice() else {
        panic!("qa48: the child recorded {paths:?}, not one folder:\n{text}");
    };
    let kept = made.join("sub/f").is_file();
    // Looked at: the parent removes what the child kept, so no run leaves it behind.
    settle(made, false);
    assert!(!passed, "qa48: the failing child passed:\n{text}");
    assert!(
        kept,
        "qa48: a failing test's scratch was deleted: {}",
        made.display()
    );
    assert!(
        text.contains(made.to_str().expect("a UTF-8 path")),
        "qa48: the kept scratch's path is not in libtest's failure report:\n{text}"
    );
}

#[test]
fn qa_m048_failing_tests_scratch_path_in_failure_output() {
    match std::env::var(MODE) {
        Ok(mode) => failing_body(&mode),
        Err(_) => check_failing_keeps("held"),
    }
}

validation::negative_control!(
    qa_m048_failing_tests_scratch_path_in_failure_output,
    "a failing child that drops its guard before it panics, so its folder is gone: the check must see it gone",
    expected = "a failing test's scratch was deleted",
    check_failing_keeps("dropped")
);

// ---------------------------------------------------------------------------------------------------------------
// R-359: a control's scratch is deleted only when its panic message matches; libtest's verdict is unchanged.

/// The control child's body: a scratch file dropped before the end, then a folder (holding a file) and a file live
/// at the end, each recorded; then, by `mode`, a panic whose message matches [`EXPECTED`] (`match_str`, a `&str`;
/// `match_string`, a formatted `String` holding it mid-message), one that does not (`wrong_str`, `wrong_string`), a
/// payload that is not a string (`not_a_string`), or no panic (`none`).
#[cfg(feature = "controls")]
fn subject(mode: &str) {
    {
        let early = Scratch::new("qa_m048_ctl_early");
        std::fs::write(&early, "early").expect("the early scratch file is written");
        record(&early);
    }
    let folder = Scratch::new("qa_m048_ctl_folder");
    std::fs::create_dir_all(folder.join("sub")).expect("the scratch folder is made");
    std::fs::write(folder.join("sub/f"), "f").expect("a file in it is written");
    record(&folder);
    let file = Scratch::new("qa_m048_ctl_file");
    std::fs::write(&file, "f").expect("the scratch file is written");
    record(&file);
    let n = 48;
    match mode {
        "match_str" => panic!("qa48 child: control's scratch was deleted (its expected message)"),
        "match_string" => panic!("qa48 child {n}: control's scratch was deleted, mid-message"),
        "wrong_str" => panic!("qa48 child: another message, a &str"),
        "wrong_string" => panic!("qa48 child {n}: another message, a String"),
        "not_a_string" => std::panic::panic_any(n),
        _ => {}
    }
}

/// The stand-in for #114's first design: a control wrapper that deletes every scratch path its body recorded on any
/// panic, then gives libtest's `should_panic(expected)` verdict (pass on a match; fail otherwise).
#[cfg(feature = "controls")]
fn deletes_on_any_panic(body: impl FnOnce()) {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    let Err(payload) = caught else {
        panic!("qa48 mock: the control did not panic");
    };
    let record = std::env::var_os(RECORD).expect("a child run has a record");
    for line in std::fs::read_to_string(record).unwrap_or_default().lines() {
        settle(Path::new(line), false);
    }
    let message = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned());
    if !message.is_some_and(|m| m.contains(EXPECTED)) {
        std::panic::resume_unwind(payload);
    }
}

/// Every child mode, under the real `negative_control!` (`mock` false) or the stand-in: libtest passes the control
/// only on a matching message; on a match every scratch the control made is gone; otherwise each is kept, with its
/// path in libtest's failure report, and a wrong `&str` or `String` message reaches libtest unchanged (its report
/// quotes it).
#[cfg(feature = "controls")]
fn check_control_scratch(mock: bool) {
    let name = "qa_m048_control_scratch_kept_unless_message_matches";
    let real = format!("{name}::negative_control");
    for (mode, matched, quoted) in [
        ("wrong_str", false, Some("another message, a &str")),
        (
            "wrong_string",
            false,
            Some("qa48 child 48: another message, a String"),
        ),
        ("not_a_string", false, None),
        ("none", false, None),
        ("match_str", true, None),
        ("match_string", true, None),
    ] {
        let mut env = vec![(MODE, mode)];
        if mock {
            env.push((MOCK, "1"));
        }
        let (passed, text, paths) = run_child(if mock { name } else { &real }, &env);
        assert_eq!(
            paths.len(),
            3,
            "qa48: the `{mode}` control recorded {paths:?}, not its three scratch paths:\n{text}"
        );
        let kept: Vec<&PathBuf> = paths.iter().filter(|p| p.exists()).collect();
        let whole = paths[1].join("sub/f").is_file();
        for path in &paths {
            settle(path, false);
        }
        assert_eq!(
            passed, matched,
            "qa48: libtest's verdict on the `{mode}` control is not should_panic's:\n{text}"
        );
        if matched {
            assert!(
                kept.is_empty(),
                "qa48: a `{mode}` control that panicked with its expected message kept {kept:?}"
            );
            continue;
        }
        assert!(
            kept.len() == paths.len() && whole,
            "qa48: a `{mode}` control's scratch was deleted: kept {kept:?} of {paths:?}\n{text}"
        );
        for path in &paths {
            assert!(
                text.contains(path.to_str().expect("a UTF-8 path")),
                "qa48: the `{mode}` control's kept scratch {} is not in libtest's failure report:\n{text}",
                path.display()
            );
        }
        if let Some(quoted) = quoted {
            assert!(
                text.matches(quoted).count() >= 2,
                "qa48: libtest's report on the `{mode}` control does not quote the original panic message:\n{text}"
            );
        }
    }
}

#[cfg(feature = "controls")]
#[test]
fn qa_m048_control_scratch_kept_unless_message_matches() {
    match std::env::var(MODE) {
        Ok(mode) if std::env::var_os(MOCK).is_some() => deletes_on_any_panic(|| subject(&mode)),
        Ok(mode) => panic!("qa48: a child run of the test itself without {MOCK} ({mode})"),
        Err(_) => check_control_scratch(false),
    }
}

// Run as a child (MODE set), the control is the subject of the check under the real macro. Run in the suite, it runs
// the check against the stand-in that deletes on any panic, which must trip it at the first wrong message.
validation::negative_control!(
    qa_m048_control_scratch_kept_unless_message_matches,
    "a control wrapper that deletes the scratch on any panic (#114's first design) must fail the check",
    expected = "control's scratch was deleted",
    match std::env::var(MODE) {
        Ok(mode) => subject(&mode),
        Err(_) => check_control_scratch(true),
    }
);

// ---------------------------------------------------------------------------------------------------------------
// R-359 on the function `negative_control!` expands to, `validation::control::run`, without the `controls` feature.

/// The expected message every case of [`check_run_settles`] is run against.
const RUN_EXPECTED: &str = "the run's expected trip";

/// A payload `run` resumed: its message, `<u32 n>` for a `u32`, or `<other>`.
fn payload_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .or_else(|| payload.downcast_ref::<u32>().map(|n| format!("<u32 {n}>")))
        .unwrap_or_else(|| "<other>".to_owned())
}

/// Each case runs on a thread of its own through `run(expected, body)`, `expected` being [`RUN_EXPECTED`] or, for the
/// control, `with`: the body makes a scratch folder holding a file, then panics by the case or not at all. The folder
/// is gone only when the message, a `&str` or `String`, contains [`RUN_EXPECTED`]; the panic reaches the thread's
/// join with its original payload; and once `run` returns, a guard on the same thread settles itself again.
fn check_run_settles(with: Option<&'static str>) {
    type Case = (&'static str, bool, fn());
    let cases: [Case; 6] = [
        ("wrong_str", false, || panic!("another message, a &str")),
        ("wrong_string", false, || {
            panic!("another message, a String {}", 48)
        }),
        ("not_a_string", false, || std::panic::panic_any(48u32)),
        ("none", false, || {}),
        ("match_str", true, || {
            panic!("qa48: the run's expected trip")
        }),
        ("match_string", true, || {
            panic!("qa48 {}: the run's expected trip, mid-message", 48)
        }),
    ];
    let expected = with.unwrap_or(RUN_EXPECTED);
    for (case, matched, body) in cases {
        let (tx, rx) = std::sync::mpsc::channel();
        let joined = std::thread::spawn(move || {
            validation::control::run(expected, || {
                let folder = Scratch::new("qa_m048_run");
                std::fs::create_dir_all(folder.join("sub")).expect("the scratch folder is made");
                std::fs::write(folder.join("sub/f"), "f").expect("a file in it is written");
                tx.send(folder.to_path_buf()).expect("the path is sent");
                body();
            });
            // `run` has returned (the body did not panic): a guard made now is no control's, and settles itself.
            let after = Scratch::new("qa_m048_after_run");
            std::fs::create_dir_all(&after).expect("the scratch folder is made");
            after.to_path_buf()
        })
        .join();
        let made = rx.recv().expect("the body made its scratch");
        let kept = made.join("sub/f").is_file();
        settle(&made, false);
        let resumed = joined.as_ref().err().map(|p| payload_text(p.as_ref()));
        let after = joined.as_ref().ok().cloned();
        if let Some(after) = &after {
            let left = after.exists();
            settle(after, false);
            assert!(
                !left,
                "qa48: after `run` returned, a passing guard's scratch was left: {}",
                after.display()
            );
        }
        let want = match case {
            "wrong_str" => Some("another message, a &str".to_owned()),
            "wrong_string" => Some("another message, a String 48".to_owned()),
            "not_a_string" => Some("<u32 48>".to_owned()),
            "none" => None,
            "match_str" => Some("qa48: the run's expected trip".to_owned()),
            _ => Some("qa48 48: the run's expected trip, mid-message".to_owned()),
        };
        assert_eq!(
            resumed, want,
            "qa48: `run` did not resume the `{case}` body's own panic payload"
        );
        if matched {
            assert!(
                !kept,
                "qa48: a `{case}` run that panicked with its expected message kept {}",
                made.display()
            );
        } else {
            assert!(
                kept,
                "qa48: a `{case}` run's scratch was deleted: {}",
                made.display()
            );
        }
    }
}

#[test]
fn qa_m048_control_run_settles_by_the_message() {
    check_run_settles(None);
}

validation::negative_control!(
    qa_m048_control_run_settles_by_the_message,
    "run with an empty expected message, which every string panic contains: it deletes on any panic, as #114's \
     first design did, and must fail the check",
    expected = "run's scratch was deleted",
    check_run_settles(Some(""))
);
