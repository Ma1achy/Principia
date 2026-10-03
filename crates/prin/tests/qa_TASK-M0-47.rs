//! QA tests for TASK-M0-47 (REQ-TOOL-147) on `prin profile` itself, written from the requirement's statement and
//! verify detail, R-341, R-286, R-298, R-299 and telemetry §5 ("`prin profile` streams its trace", "A session that
//! ended before its summary line", "A line cut off at the end"), not from the implementation:
//!
//! - a completed run's trace is a complete session: the header, the frames from 0, the summary line last, the same file
//!   `profile::write` gives for what `read` returns (for 0, 1, 60, 61 and 125 frames);
//! - a run killed mid-session left a trace that grew while it ran; the reader returns the header (the run's own config)
//!   and the frames from 0 in order and reports "session incomplete", dropping a cut-off last line with its byte count;
//! - a run whose trace cannot be written fails, and says which trace (telemetry §5: a writer "fails rather than write").
//!
//! The flush policy itself is tested on the engine's streaming writer, in `crates/engine/tests/qa_TASK-M0-47_stream.rs`.
//! Each test has its negative control (R-176). The memory bound ("must not grow with `--frames`") is the perf
//! reviewer's, at two measured frame counts (R-341, applied per R-204, accepted by R-346); no threshold is set here.
// The file name `qa_TASK-M0-47` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use engine::contract::profile::{self, Absent, FrameRecord, Session};
use serde_json::{json, Value};
use validation::spawn::Spawn;

#[path = "../../validation/tests/support/scratch.rs"]
mod scratch;
use scratch::Scratch;

fn scratch(name: &str) -> Scratch {
    Scratch::new(&format!("qa_TASK-M0-47_{name}"))
}

fn s(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

/// `prin profile --scenario synthetic_frames --frames N --json PATH`.
fn profile_command(frames: u32, path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prin"));
    command.args([
        "profile",
        "--scenario",
        "synthetic_frames",
        "--frames",
        &frames.to_string(),
        "--json",
        s(path),
    ]);
    command
}

/// The bytes of a completed `prin profile` run of `frames` frames.
fn completed_run(frames: u32) -> Vec<u8> {
    let path = scratch("complete.jsonl");
    let out = profile_command(frames, &path)
        .timed_output()
        .expect("cannot run prin");
    assert!(
        out.status.success(),
        "prin profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::read(&*path).expect("the run left no trace")
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// A completed run's trace is a complete session (R-286, R-298, R-341): the header line first, the frames from 0 in
/// order, the summary line `{"leak_flags": null, "hot_paths": null}` last and newline-terminated, and the file is the
/// one `profile::write` gives for what `read` returns.
fn check_complete(bytes: &[u8], frames: u32) {
    let trace = profile::read(bytes).expect("the trace does not read");
    assert_eq!(
        trace.session,
        Session::Complete,
        "the completed run's trace is not complete ({frames} frames)"
    );
    assert_eq!(trace.dropped_bytes, 0);
    assert_eq!(trace.frames.len(), frames as usize);
    for (i, f) in trace.frames.iter().enumerate() {
        assert_eq!(f.frame, i as u64, "frame {i} is out of order");
    }
    let text = std::str::from_utf8(bytes).expect("the trace is not UTF-8");
    assert!(text.ends_with('\n'), "the summary line has no newline");
    let last: Value = serde_json::from_str(text.lines().last().expect("an empty trace"))
        .expect("the last line is not JSON");
    assert_eq!(last, json!({"leak_flags": null, "hot_paths": null}));
    let mut again = Vec::new();
    profile::write(&trace, &mut again).expect("write refuses the trace");
    assert_eq!(again, bytes, "the run's file is not the file write writes");
}

#[test]
fn qa_profile_stream_completed_run_is_a_complete_session() {
    for frames in [0, 1, 60, 61, 125] {
        check_complete(&completed_run(frames), frames);
    }
}

validation::negative_control!(
    qa_profile_stream_completed_run_is_a_complete_session,
    "a trace with its summary line cut must fail the completeness check",
    expected = "is not complete",
    {
        let bytes = completed_run(61);
        let text = String::from_utf8(bytes).expect("UTF-8");
        let cut = text.trim_end_matches('\n');
        let without = &text[..cut.rfind('\n').expect("one line") + 1];
        check_complete(without.as_bytes(), 61)
    }
);

/// The file sizes a watcher saw while a run ran, and whether it saw a header line and a frame line.
#[derive(Default)]
struct Watched {
    sizes: Vec<u64>,
    two_lines: bool,
}

fn watch(path: std::path::PathBuf, running: Arc<AtomicBool>) -> thread::JoinHandle<Watched> {
    thread::spawn(move || {
        let mut seen = Watched::default();
        while running.load(Ordering::SeqCst) {
            if let Ok(bytes) = fs::read(&path) {
                let size = bytes.len() as u64;
                if seen.sizes.last() != Some(&size) {
                    seen.sizes.push(size);
                }
                seen.two_lines |= bytes.iter().filter(|b| **b == b'\n').count() >= 2;
            }
            thread::sleep(ms(3));
            // Stop reading whole files once enough is seen; the run's trace grows fast.
            if seen.two_lines && seen.sizes.len() >= 3 {
                break;
            }
        }
        seen
    })
}

/// A run of a frame count it cannot finish, `prin profile --frames 4294967295`, stopped by the spawn helper (R-214)
/// once it has run for `after`; then what it left. The trace grew while the run ran; the reader returns the header
/// (the run's own config) and the frames from 0 in order; the session is incomplete with "session incomplete"; and
/// the bytes after the last newline are dropped and counted, unless they are one whole frame record still waiting for
/// its newline (R-299: a complete JSON last line is read as any last line is).
fn check_killed(run: impl Fn(&Path) -> Command) {
    for after in [ms(700), ms(1500), ms(3000)] {
        let path = scratch("killed.jsonl");
        let running = Arc::new(AtomicBool::new(true));
        let watcher = watch(path.to_path_buf(), Arc::clone(&running));
        let ran = run(&path).output_within(after);
        running.store(false, Ordering::SeqCst);
        let seen = watcher.join().expect("the watcher panicked");
        match ran {
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
            other => panic!("the run was not stopped mid-session: {other:?}"),
        }
        if !seen.two_lines {
            continue;
        }
        let growth: Vec<&u64> = seen.sizes.iter().filter(|s| **s > 0).collect();
        assert!(
            growth.len() >= 2,
            "the trace did not grow while the run ran: sizes {:?}",
            seen.sizes
        );
        let bytes = fs::read(&*path).expect("the killed run left no trace");
        let tail = bytes.len() - bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        let trace =
            profile::read(bytes.as_slice()).expect("the reader rejects the killed run's trace");
        assert_eq!(trace.header.config["scenario"], json!("synthetic_frames"));
        assert_eq!(trace.header.config["frames"], json!(u32::MAX));
        assert!(!trace.frames.is_empty(), "the trace holds no frame record");
        for (i, f) in trace.frames.iter().enumerate() {
            assert_eq!(f.frame, i as u64, "frame {i} is out of order");
        }
        assert_eq!(
            trace.session,
            Session::Incomplete,
            "the session is not incomplete"
        );
        assert_eq!(trace.leak_flags(), Err(Absent::SessionIncomplete));
        assert_eq!(trace.hot_paths(), Err(Absent::SessionIncomplete));
        assert_eq!(Absent::SessionIncomplete.to_string(), "session incomplete");
        let tail_text = &bytes[bytes.len() - tail..];
        let tail_whole = tail > 0 && serde_json::from_slice::<FrameRecord>(tail_text).is_ok();
        let want = if tail_whole { 0 } else { tail as u64 };
        assert_eq!(
            trace.dropped_bytes, want,
            "the cut-off last line of {tail} bytes was not dropped and counted"
        );
        return;
    }
    panic!("the stopped run left no header line and frame line while it ran");
}

#[test]
fn qa_profile_stream_killed_mid_session() {
    check_killed(|path| profile_command(u32::MAX, path));
}

validation::negative_control!(
    qa_profile_stream_killed_mid_session,
    "a run whose trace appears whole, in one step, must fail the growth check",
    expected = "did not grow while the run ran",
    check_killed(|path| {
        // The trace is written elsewhere, then moved into place whole, and the run waits to be stopped.
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "\"$0\" profile --scenario synthetic_frames --frames 50 --json \"$1.tmp\" && mv \"$1.tmp\" \"$1\" \
             && sleep 30",
            env!("CARGO_BIN_EXE_prin"),
            s(path),
        ]);
        command
    })
);

/// A run whose trace cannot be written past its first bytes (the file-size limit at 0 bytes, its signal ignored, so
/// each write fails with "file too large") fails, exits non-zero, and names the trace it could not write; with
/// `limit_kib` large enough, the same run succeeds.
fn check_unwritable(limit_kib: &str) {
    let path = scratch("unwritable.jsonl");
    let out = Command::new("sh")
        .args([
            "-c",
            "trap '' XFSZ; ulimit -f \"$1\" && exec \"$0\" profile --scenario synthetic_frames --frames 5 --json \"$2\"",
            env!("CARGO_BIN_EXE_prin"),
            limit_kib,
            s(&path),
        ])
        .timed_output()
        .expect("cannot run prin");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "unwritable: the run succeeded with a file-size limit of {limit_kib} blocks"
    );
    assert!(
        stderr.contains(s(&path)),
        "unwritable: the error does not name the trace {}: {stderr}",
        path.display()
    );
}

#[test]
fn qa_profile_stream_unwritable_trace_fails_naming_it() {
    check_unwritable("0");
}

validation::negative_control!(
    qa_profile_stream_unwritable_trace_fails_naming_it,
    "a run with room to write must fail the unwritable check",
    expected = "unwritable: the run succeeded",
    check_unwritable("unlimited")
);
