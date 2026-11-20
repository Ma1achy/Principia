//! qa's tests for TASK-M0-51, `prin profile show` and `prin profile diff` on a complete trace whose cut-off last line
//! follows its summary line, written from the requirement it closes, not from the implementation:
//!
//! - REQ-TOOL-148 (R-356, R-358): both commands always state that file's dropped bytes, and neither calls the session
//!   incomplete; the diff states the bytes first, then compares the file as a complete trace, so nothing is compared
//!   silently; the bytes come before a refusal for no frame records too (render_gui_spec § "Profiler", "A cut-off or
//!   incomplete trace"). The notices' words are R-358's, accepted by R-363: the diff's "<BASE|NEW>: <n> bytes of a
//!   cut-off line after the summary line dropped", show's "prin profile show: the line after the summary line was cut
//!   off, and its <n> bytes are not shown pretty". Neither prints a notice when no bytes were dropped.
//! - R-323, R-298, R-299, as R-358 keeps them: a session that ended before its summary line, its last line cut off,
//!   still gets "session incomplete" and its bytes, from both commands.
//!
//! Every fixture is built here from telemetry §5's keys. Each test has its negative control (R-176).
// The file name `qa_TASK-M0-51` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::Path;
use std::process::{Command, Output};

use engine::contract::profile;
use serde_json::{json, Value};
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn prin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_prin"))
        .args(args)
        .timed_output()
        .expect("cannot run prin")
}

fn s(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// A scratch file holding `bytes`: deleted when the test passes, kept with its path printed when it fails (R-342).
fn write(name: &str, bytes: &[u8]) -> Scratch {
    let path = Scratch::new(&format!("qa_TASK-M0-51_{name}"));
    std::fs::write(&path, bytes).expect("cannot write a scratch file");
    path
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ----- the fixture (§5, key for key) -----

fn header_line() -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": {
                "gpu": "Test GPU", "cpu": "Test CPU", "cpu_cores_available": 8, "cpu_cores_total": 8,
                "gpu_cores": 10,
                "memory": { "unified": { "bytes": 17_179_869_184_u64 } }
            },
            "backend": { "api": "metal", "driver": "1.0" },
            "precision": { "f32": true, "f64": false, "f64_rate": null },
            "fast_math": { "setting": "off", "compiled": { "compute": "off", "vertex": "on", "fragment": "on" } },
            "build": { "commit": "qa", "profile": "release", "features": [] },
            "display": null,
            "config": { "scenario": "qa", "frames": 20 }
        }
    })
}

fn empty_sections() -> Value {
    json!({ "scopes": [], "gpu_passes": [], "allocations": [], "events": [] })
}

fn pool() -> Value {
    json!({ "bytes": 0, "by_kind": [] })
}

/// Frame `i`, its frame time `ms` and one CPU scope `work` taking `ms / 2`.
fn frame(i: usize, ms: f64) -> Value {
    let mut integrate = empty_sections();
    integrate["scopes"] =
        json!([{ "name": "work", "start_ms": 0.0, "ms": ms / 2.0, "children": [] }]);
    json!({
        "frame": i, "frame_ms": ms,
        "quads_computed": 0, "quads_reused": 0, "samples": 0, "substeps_total": 0,
        "playhead_dt": 0.0, "camera_delta": 0.0, "tree_depth_max": 0, "leaf_count": 0,
        "dmin_nan_unset": 0, "dmin_negative_floored": 0,
        "stage_ms": { "integrate": ms / 2.0, "reduce": 1.0, "colour": 1.0, "upload": 0.5, "present": null },
        "stages": { "integrate": integrate, "reduce": empty_sections(), "colour": empty_sections(),
                    "upload": empty_sections(), "present": null },
        "live_memory": { "heap": pool(), "gpu": pool(), "tile_cache": pool() }
    })
}

/// The summary line, both summaries set.
fn summary_line() -> String {
    json!({
        "leak_flags": [ { "growth_bytes_per_s": 1.5 } ],
        "hot_paths": [ { "scope": "integrate", "p95_ms": 6.0 } ]
    })
    .to_string()
}

/// The lines of a session's file, without newlines: the header line, then 20 frames whose times are `k` times
/// 20..39 ms (none when `frames` is false), then the summary line when `summary` is true.
fn lines(k: f64, frames: bool, summary: bool) -> Vec<String> {
    let mut out = vec![header_line().to_string()];
    if frames {
        out.extend((0..20).map(|i| frame(i, k * (20.0 + i as f64)).to_string()));
    }
    if summary {
        out.push(summary_line());
    }
    out
}

/// `lines` as a file, each ended by its newline, then `tail` with no newline after it.
fn file(lines: &[String], tail: &str) -> Vec<u8> {
    let mut out: String = lines.iter().map(|l| format!("{l}\n")).collect();
    out.push_str(tail);
    let bytes = out.into_bytes();
    profile::read(&bytes[..]).unwrap_or_else(|e| panic!("a qa fixture is not schema v1: {e}"));
    bytes
}

/// Two cut-off pieces: a frame-record-shaped line and a summary-shaped line, each cut before its end.
fn pieces() -> [String; 2] {
    let f = frame(0, 20.0).to_string();
    let sm = summary_line();
    [f[..97].to_owned(), sm[..sm.len() - 3].to_owned()]
}

// ----- `prin profile show` -----

fn show(path: &Path, pretty: bool) -> Output {
    let mut args = vec!["profile", "show", s(path)];
    if pretty {
        args.push("--pretty");
    }
    prin(&args)
}

fn show_complete_notice(n: usize) -> String {
    format!("prin profile show: the line after the summary line was cut off, and its {n} bytes are not shown pretty")
}

fn show_incomplete_notice(n: usize) -> String {
    format!("prin profile show: session incomplete; the last line was cut off, and its {n} bytes are not shown pretty")
}

/// The JSON values `printed` holds, one after another.
fn values(printed: &str) -> Vec<Value> {
    serde_json::Deserializer::from_str(printed)
        .into_iter::<Value>()
        .map(|v| v.unwrap_or_else(|e| panic!("show --pretty printed text that is not JSON: {e}")))
        .collect()
}

/// `show` on the lines `kept` followed by `tail`: exit 0; plain prints the file unchanged; `--pretty` prints the kept
/// lines' values, header, frames and (when kept) summary line, in order; the notice on stderr is `notice`, and says
/// "session incomplete" only when `incomplete`.
fn check_show(
    run: impl Fn(&Path, bool) -> Output,
    kept: &[String],
    tail: &str,
    notice: Option<&str>,
    incomplete: bool,
    what: &str,
) {
    let bytes = file(kept, tail);
    let path = write("show.jsonl", &bytes);
    for pretty in [false, true] {
        let mode = if pretty { "--pretty" } else { "plain" };
        let out = run(&path, pretty);
        let stderr = text(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(0),
            "show ({mode}) on {what} fails: {stderr}"
        );
        if pretty {
            let want: Vec<Value> = kept
                .iter()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            assert!(
                values(&text(&out.stdout)) == want,
                "show --pretty on {what} does not print the header, frames and summary line kept"
            );
        } else {
            assert!(
                out.stdout == bytes,
                "show (plain) on {what} does not print the file unchanged"
            );
        }
        assert_eq!(
            stderr.contains("session incomplete"),
            incomplete,
            "show ({mode}) on {what}: \"session incomplete\" is wrongly {}: {stderr}",
            if incomplete { "absent" } else { "present" }
        );
        match notice {
            Some(n) => assert!(
                stderr.lines().any(|l| l == n),
                "show ({mode}) on {what} does not print the notice {n:?}: {stderr}"
            ),
            None => assert!(
                !stderr.contains("bytes") && !stderr.contains("cut off"),
                "show ({mode}) on {what} prints a notice for no dropped bytes: {stderr}"
            ),
        }
    }
}

fn check_show_cases(run: impl Fn(&Path, bool) -> Output + Copy) {
    for piece in pieces() {
        // A complete session, its cut-off last line after the summary line: the bytes, not "session incomplete".
        check_show(
            run,
            &lines(1.0, true, true),
            &piece,
            Some(&show_complete_notice(piece.len())),
            false,
            &format!("a complete trace and a {}-byte cut-off line", piece.len()),
        );
        // The same with no frame record.
        check_show(
            run,
            &lines(1.0, false, true),
            &piece,
            Some(&show_complete_notice(piece.len())),
            false,
            &format!(
                "a header line, summary line and a {}-byte cut-off line",
                piece.len()
            ),
        );
    }
    // A session cut off before its summary line: unchanged, "session incomplete" and the bytes.
    let piece = &pieces()[0];
    check_show(
        run,
        &lines(1.0, true, false),
        piece,
        Some(&show_incomplete_notice(piece.len())),
        true,
        "a trace cut off before its summary line",
    );
    // A complete trace that dropped nothing: no notice.
    check_show(
        run,
        &lines(1.0, true, true),
        "",
        None,
        false,
        "a complete trace",
    );
}

#[test]
fn qa_m051_show_states_bytes_dropped_after_the_summary_line() {
    check_show_cases(show);
}

validation::negative_control!(
    qa_m051_show_states_bytes_dropped_after_the_summary_line,
    "show with TASK-M0-18's notice, \"session incomplete\" whenever bytes were dropped, must fail",
    expected = "\"session incomplete\" is wrongly present",
    {
        fn show_keyed_on_dropped_bytes(path: &Path, pretty: bool) -> Output {
            let mut out = show(path, pretty);
            let trace = profile::read(&std::fs::read(path).unwrap()[..]).unwrap();
            if trace.dropped_bytes > 0 {
                out.stderr = format!("{}\n", show_incomplete_notice(trace.dropped_bytes as usize))
                    .into_bytes();
            }
            out
        }
        check_show_cases(show_keyed_on_dropped_bytes)
    }
);

validation::negative_control!(
    qa_m051_show_states_bytes_dropped_after_the_summary_line_silent,
    "show printing no notice for the complete trace with dropped bytes must fail",
    expected = "does not print the notice",
    {
        fn show_silent_when_complete(path: &Path, pretty: bool) -> Output {
            let mut out = show(path, pretty);
            let trace = profile::read(&std::fs::read(path).unwrap()[..]).unwrap();
            if trace.session == profile::Session::Complete {
                out.stderr.clear();
            }
            out
        }
        check_show_cases(show_silent_when_complete)
    }
);

// ----- `prin profile diff` -----

fn diff(base: &Path, new: &Path, threshold: &str) -> Output {
    prin(&["profile", "diff", s(base), s(new), "--threshold", threshold])
}

fn diff_complete_notice(name: &str, n: usize) -> String {
    format!("{name}: {n} bytes of a cut-off line after the summary line dropped")
}

/// `run` on BASE and NEW, each the lines given then a cut-off tail ("" for none), against the same files uncut: the
/// cut run's stdout is the notice line for each cut file, in either order, followed by the uncut run's stdout, its stderr
/// and exit code the uncut run's, and "session incomplete" is printed nowhere.
fn check_diff_compares(
    run: impl Fn(&Path, &Path, &str) -> Output,
    (base, base_tail): (&[String], &str),
    (new, new_tail): (&[String], &str),
    threshold: &str,
    want_exit: i32,
    what: &str,
) {
    let (b, n) = (
        write("base.jsonl", &file(base, base_tail)),
        write("new.jsonl", &file(new, new_tail)),
    );
    let (ub, un) = (
        write("base-uncut.jsonl", &file(base, "")),
        write("new-uncut.jsonl", &file(new, "")),
    );
    let got = run(&b, &n, threshold);
    let want = run(&ub, &un, threshold);
    let stdout = text(&got.stdout);
    assert_eq!(
        want.status.code(),
        Some(want_exit),
        "{what}: the uncut files give exit {:?} at --threshold {threshold}, not {want_exit}",
        want.status.code()
    );
    assert!(
        !stdout.contains("session incomplete") && !text(&got.stderr).contains("session incomplete"),
        "{what}: the diff calls a complete trace \"session incomplete\": {stdout}"
    );
    let mut notices: Vec<String> = [("BASE", base_tail), ("NEW", new_tail)]
        .iter()
        .filter(|(_, tail)| !tail.is_empty())
        .map(|(name, tail)| diff_complete_notice(name, tail.len()))
        .collect();
    notices.sort();
    let comparison = text(&want.stdout);
    let mut first: Vec<String> = stdout
        .strip_suffix(comparison.as_str())
        .map(|head| head.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    first.sort();
    assert!(
        stdout.ends_with(comparison.as_str()) && first == notices,
        "{what}: the diff does not state the bytes dropped after the summary line first, then compare as for the uncut files, at --threshold {threshold}: {stdout}"
    );
    assert_eq!(
        got.stderr, want.stderr,
        "{what}: stderr differs from the uncut files' at --threshold {threshold}"
    );
    assert_eq!(
        got.status.code(),
        want.status.code(),
        "{what}: the exit code differs from the uncut files' at --threshold {threshold}"
    );
}

fn check_diff_cases(run: impl Fn(&Path, &Path, &str) -> Output + Copy) {
    let base = lines(1.0, true, true);
    let raised = lines(1.06, true, true);
    for piece in pieces() {
        for (threshold, exit) in [("5%", 1), ("10%", 0)] {
            check_diff_compares(
                run,
                (&base, ""),
                (&raised, &piece),
                threshold,
                exit,
                "a cut-off NEW",
            );
            check_diff_compares(
                run,
                (&base, &piece),
                (&raised, ""),
                threshold,
                exit,
                "a cut-off BASE",
            );
            check_diff_compares(
                run,
                (&base, &piece),
                (&raised, &piece),
                threshold,
                exit,
                "both cut off",
            );
        }
    }
    // No cut-off line: no notice, the comparison alone.
    check_diff_compares(
        run,
        (&base, ""),
        (&raised, ""),
        "5%",
        1,
        "two complete traces",
    );
}

#[test]
fn qa_m051_diff_states_bytes_then_compares() {
    check_diff_cases(diff);
}

validation::negative_control!(
    qa_m051_diff_states_bytes_then_compares,
    "a diff with TASK-M0-18's notices, none for a complete session, must fail",
    expected = "does not state the bytes dropped after the summary line first",
    {
        fn diff_keyed_on_incomplete(base: &Path, new: &Path, threshold: &str) -> Output {
            let mut out = diff(base, new, threshold);
            out.stdout = text(&out.stdout)
                .split_inclusive('\n')
                .filter(|l| !l.contains("after the summary line dropped"))
                .collect::<String>()
                .into_bytes();
            out
        }
        check_diff_cases(diff_keyed_on_incomplete)
    }
);

validation::negative_control!(
    qa_m051_diff_states_bytes_then_compares_after,
    "a diff stating the bytes after the comparison must fail",
    expected = "does not state the bytes dropped after the summary line first",
    {
        fn diff_notice_last(base: &Path, new: &Path, threshold: &str) -> Output {
            let mut out = diff(base, new, threshold);
            let (notice, rest): (Vec<&str>, Vec<&str>) = std::str::from_utf8(&out.stdout)
                .unwrap()
                .split_inclusive('\n')
                .partition(|l| l.contains("after the summary line dropped"));
            out.stdout = (rest.concat() + &notice.concat()).into_bytes();
            out
        }
        check_diff_cases(diff_notice_last)
    }
);

/// `run` with `name` (BASE or NEW) a file with no frame records, `tail` after its last line, and the other a complete
/// trace: exit 2; the output says it has no frame records, after `notice` when one is given; "session incomplete"
/// only when `incomplete`.
fn check_no_frames(
    run: impl Fn(&Path, &Path, &str) -> Output,
    name: &str,
    kept: &[String],
    tail: &str,
    notice: &str,
    incomplete: bool,
) {
    let empty = write("empty.jsonl", &file(kept, tail));
    let full = write("full.jsonl", &file(&lines(1.0, true, true), ""));
    let out = if name == "NEW" {
        run(&full, &empty, "5%")
    } else {
        run(&empty, &full, "5%")
    };
    let all = text(&out.stdout) + &text(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a {name} with no frame records gives exit {:?}: {all}",
        out.status.code()
    );
    let refusal = all
        .find("no frame records")
        .unwrap_or_else(|| panic!("the diff does not say {name} has no frame records: {all}"));
    let at = all.find(notice).unwrap_or_else(|| {
        panic!("a {name} with no frame records: the diff does not print {notice:?} before its refusal: {all}")
    });
    assert!(
        at < refusal,
        "a {name} with no frame records: {notice:?} comes after the refusal: {all}"
    );
    assert_eq!(
        all.contains("session incomplete"),
        incomplete,
        "a {name} with no frame records: \"session incomplete\" is wrongly {}: {all}",
        if incomplete { "absent" } else { "present" }
    );
}

fn check_no_frames_cases(run: impl Fn(&Path, &Path, &str) -> Output + Copy) {
    for name in ["BASE", "NEW"] {
        for piece in pieces() {
            check_no_frames(
                run,
                name,
                &lines(1.0, false, true),
                &piece,
                &diff_complete_notice(name, piece.len()),
                false,
            );
        }
        // A header line alone, then a cut-off line: incomplete, unchanged.
        let piece = &pieces()[0];
        check_no_frames(
            run,
            name,
            &lines(1.0, false, false),
            piece,
            &format!("{name}: session incomplete; {} bytes", piece.len()),
            true,
        );
    }
}

#[test]
fn qa_m051_diff_states_bytes_before_a_no_frames_refusal() {
    check_no_frames_cases(diff);
}

validation::negative_control!(
    qa_m051_diff_states_bytes_before_a_no_frames_refusal,
    "a refusal with no notice for a complete session with dropped bytes must fail",
    expected = "before its refusal",
    {
        fn refusal_keyed_on_incomplete(base: &Path, new: &Path, threshold: &str) -> Output {
            let mut out = diff(base, new, threshold);
            let strip = |b: &[u8]| -> Vec<u8> {
                text(b)
                    .split_inclusive('\n')
                    .filter(|l| !l.contains("after the summary line dropped"))
                    .collect::<String>()
                    .into_bytes()
            };
            out.stdout = strip(&out.stdout);
            out.stderr = strip(&out.stderr);
            out
        }
        check_no_frames_cases(refusal_keyed_on_incomplete)
    }
);

/// A session cut off before its summary line still gets "session incomplete" and its bytes, and is compared.
fn check_incomplete_still_said(run: impl Fn(&Path, &Path, &str) -> Output) {
    let piece = &pieces()[0];
    let base = write("base.jsonl", &file(&lines(1.0, true, true), ""));
    let new = write("new.jsonl", &file(&lines(1.06, true, false), piece));
    let out = run(&base, &new, "5%");
    let stdout = text(&out.stdout);
    let notice = format!(
        "NEW: session incomplete; {} bytes of a cut-off last line dropped\n",
        piece.len()
    );
    assert!(
        stdout.starts_with(&notice) && !stdout.contains("after the summary line"),
        "a NEW cut off before its summary line does not get {notice:?} first: {stdout}"
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "a NEW cut off before its summary line, raised 6%, is not compared: {stdout}"
    );
}

#[test]
fn qa_m051_diff_incomplete_session_still_says_so() {
    check_incomplete_still_said(diff);
}

validation::negative_control!(
    qa_m051_diff_incomplete_session_still_says_so,
    "a diff giving an incomplete session the complete one's notice must fail",
    expected = "does not get",
    {
        fn incomplete_as_complete(base: &Path, new: &Path, threshold: &str) -> Output {
            let mut out = diff(base, new, threshold);
            out.stdout = text(&out.stdout)
                .replacen("NEW: session incomplete; ", "NEW: ", 1)
                .replacen(
                    "cut-off last line dropped",
                    "cut-off line after the summary line dropped",
                    1,
                )
                .into_bytes();
            out
        }
        check_incomplete_still_said(incomplete_as_complete)
    }
);
