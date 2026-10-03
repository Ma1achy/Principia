//! qa's tests for TASK-M0-51, written from the requirement it closes, not from the implementation:
//!
//! - REQ-TOOL-148 (R-356, R-358; telemetry §5 "A line cut off at the end"): a last line with no newline after it that
//!   is not one complete JSON value, following the summary line, is dropped and its bytes reported in
//!   `Trace::dropped_bytes`; the header line, the frames and the summary line are kept, `leak_flags` and `hot_paths`
//!   are read from it, and the session is complete. There is no third session state.
//! - R-299, as R-356 keeps it: a file whose only line is a cut-off header line stays an error that states its bytes; a
//!   cut-off line after the header line or a frame record is an incomplete session with its bytes stated; a malformed
//!   line that ends in a newline stays an error, wherever it is, after the summary line included; a last line with no
//!   newline that is one complete JSON value is read as any last line is, so after the summary line it is an error.
//!
//! The fixture is hand-written from telemetry §5's keys and laid out as JSON Lines here. Each test registers the
//! control that must make it fail (R-176).
// The file name `qa_TASK-M0-51` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use engine::contract::profile::{read, Session, Trace};
use serde_json::{json, Value};

// ----- the hand-written fixture (§5, key for key) -----

fn sections(outer: &str, finer: &str, start: f64) -> Value {
    json!({
        "scopes": [
            { "name": outer, "start_ms": start, "ms": 1.0, "children": [
                { "name": finer, "start_ms": start + 0.25, "ms": 0.5, "children": [] }
            ] }
        ],
        "gpu_passes": [ { "name": format!("{outer} pass"), "start_ms": start, "ms": 0.75 } ],
        "allocations": [ { "kind": format!("{outer} buffer"), "pool": "gpu", "count": 2, "bytes": 4096 } ],
        "events": [ { "name": format!("{outer} done"), "at_ms": start + 1.0, "detail": null } ]
    })
}

fn frame(index: u64, detail: &str) -> Value {
    let mut integrate = sections("dispatch", "IC decode", 0.0);
    integrate["events"][0]["detail"] = json!(detail);
    json!({
        "frame": index,
        "frame_ms": 12.5 + index as f64,
        "quads_computed": 16,
        "quads_reused": 48,
        "samples": 4096,
        "substeps_total": 1_234_567,
        "playhead_dt": 0.25,
        "camera_delta": 0.0,
        "tree_depth_max": 7,
        "leaf_count": 64,
        "dmin_nan_unset": index,
        "dmin_negative_floored": 2 * index,
        "stage_ms": { "integrate": 6.0, "reduce": 2.0, "colour": 1.5, "upload": 1.0, "present": 2.0 },
        "stages": {
            "integrate": integrate,
            "reduce": sections("reduction", "quadtree", 6.0),
            "colour": sections("stain", "stain + style", 8.0),
            "upload": sections("upload", "readback", 9.5),
            "present": sections("present", "egui", 10.5)
        },
        "live_memory": {
            "heap": { "bytes": 100, "by_kind": [ { "kind": "quad", "count": 1, "bytes": 100 } ] },
            "gpu": { "bytes": 0, "by_kind": [] },
            "tile_cache": { "bytes": 0, "by_kind": [] }
        }
    })
}

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
            "display": { "width_px": 800, "height_px": 600, "refresh_hz": 60.0, "dpi_scale": 1.0 },
            "config": { "scenario": "qa", "frames": 3 }
        }
    })
}

/// The summary line, both summaries set, so that summaries read from it cannot pass for absent or `null` ones.
fn summary_line() -> Value {
    json!({
        "leak_flags": [ { "growth_bytes_per_s": 1.5, "pool": "heap" } ],
        "hot_paths": [ { "scope": "integrate", "p95_ms": 6.0 } ]
    })
}

/// The lines of a complete session's file, each without its newline: the header line, `frames` frame records (the
/// second with multi-byte characters, so a cut can fall inside one), then the summary line.
fn lines(frames: u64) -> Vec<String> {
    let mut out = vec![header_line().to_string()];
    for i in 0..frames {
        let detail = if i == 1 {
            "naïve – 日本 {[\"x\"]}"
        } else {
            "text"
        };
        out.push(frame(i, detail).to_string());
    }
    out.push(summary_line().to_string());
    out
}

/// `lines` as a file, each ended by its newline.
fn file(lines: &[String]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|l| format!("{l}\n").into_bytes())
        .collect()
}

/// The complete trace a whole file reads as, its summaries both set (checked, so the comparison means something).
fn whole(lines: &[String]) -> Trace {
    let t =
        read(&file(lines)[..]).unwrap_or_else(|e| panic!("the qa fixture is not schema v1: {e}"));
    assert!(
        t.session == Session::Complete
            && t.dropped_bytes == 0
            && t.leak_flags().is_ok_and(|f| f.len() == 1)
            && t.hot_paths().is_ok_and(|h| h.len() == 1),
        "the whole qa fixture does not read as a complete trace with both summaries"
    );
    t
}

// ----- REQ-TOOL-148: a cut-off line after the summary line -----

/// `bytes` is `lines`' whole file followed by a `dropped`-byte cut-off last line: it reads as the complete trace the
/// whole file is, the header, every frame and both summaries, the session complete and `dropped` bytes stated.
fn check_complete_with_dropped(bytes: &[u8], lines: &[String], dropped: usize, what: &str) {
    let want = whole(lines);
    let got = read(bytes).unwrap_or_else(|e| panic!("the reader rejects {what}: {e}"));
    assert_eq!(
        got.session,
        Session::Complete,
        "{what}: not a complete session"
    );
    assert_eq!(
        got.dropped_bytes, dropped as u64,
        "{what}: dropped_bytes is not the {dropped} bytes cut off"
    );
    assert!(
        got.header == want.header,
        "{what}: the header is not the one written"
    );
    assert!(
        got.frames == want.frames,
        "{what}: the frames read are not the {} written",
        want.frames.len()
    );
    assert!(
        got.leak_flags() == want.leak_flags() && got.hot_paths() == want.hot_paths(),
        "{what}: leak_flags and hot_paths are not the summary line's"
    );
}

/// Every line shape (the header line, a frame record, one with multi-byte characters, the summary line) cut at every
/// byte inside it, after the whole file of `frames` frames.
fn check_every_cut_after_summary(frames: u64) {
    let lines = lines(frames);
    let full = file(&lines);
    let mut inside_a_character = 0;
    for shape in &lines {
        let shape = shape.as_bytes();
        for cut in 1..shape.len() {
            let mut bytes = full.clone();
            bytes.extend_from_slice(&shape[..cut]);
            inside_a_character += usize::from(std::str::from_utf8(&shape[..cut]).is_err());
            check_complete_with_dropped(
                &bytes,
                &lines,
                cut,
                &format!(
                    "{frames} frames, then a line cut after {cut} of its {} bytes after the summary line",
                    shape.len()
                ),
            );
        }
    }
    if frames > 1 {
        assert!(inside_a_character > 0, "no cut falls inside a character");
    }
}

#[test]
fn qa_m051_cut_off_line_after_the_summary_line_is_a_complete_session() {
    check_every_cut_after_summary(3);
}

validation::negative_control!(
    qa_m051_cut_off_line_after_the_summary_line_is_a_complete_session,
    "the same cut-off line after the last frame record, no summary line, is an incomplete session",
    expected = "not a complete session",
    {
        let lines = lines(3);
        let mut bytes = file(&lines[..lines.len() - 1]);
        let piece = &lines[1].as_bytes()[..40];
        bytes.extend_from_slice(piece);
        check_complete_with_dropped(
            &bytes,
            &lines,
            piece.len(),
            "a cut-off line after the last frame",
        )
    }
);

validation::negative_control!(
    qa_m051_cut_off_line_after_the_summary_line_is_a_complete_session_count,
    "a wrong byte count must fail the dropped-bytes check",
    expected = "dropped_bytes is not the",
    {
        let lines = lines(3);
        let mut bytes = file(&lines);
        bytes.extend_from_slice(&lines[1].as_bytes()[..40]);
        check_complete_with_dropped(&bytes, &lines, 41, "a cut-off line, miscounted")
    }
);

/// The summary line with no frame record before it, then a cut-off line: complete, no frames, both summaries.
#[test]
fn qa_m051_no_frames_then_summary_then_cut_off_is_complete() {
    check_every_cut_after_summary(0);
}

validation::negative_control!(
    qa_m051_no_frames_then_summary_then_cut_off_is_complete,
    "the header line then a cut-off line, no summary line, is an incomplete session",
    expected = "not a complete session",
    {
        let lines = lines(0);
        let mut bytes = file(&lines[..1]);
        let piece = &lines[1].as_bytes()[..10];
        bytes.extend_from_slice(piece);
        check_complete_with_dropped(
            &bytes,
            &lines,
            piece.len(),
            "a cut-off line after the header line",
        )
    }
);

// ----- R-299 as R-356 keeps it: every other cut-off case -----

/// A file whose only line is the header line cut after `cut` bytes is an error, and the error states the bytes.
fn check_cut_header_alone(cut: usize) {
    let head = header_line().to_string();
    match read(&head.as_bytes()[..cut]) {
        Ok(_) => panic!(
            "the reader accepts a file whose only line is a header line cut after {cut} bytes"
        ),
        Err(e) => assert!(
            e.to_string()
                .split(|c: char| !c.is_ascii_digit())
                .any(|tok| tok == cut.to_string()),
            "the error for a header line cut after {cut} bytes does not state its bytes: {e}"
        ),
    }
}

#[test]
fn qa_m051_cut_off_header_line_alone_stays_an_error() {
    let len = header_line().to_string().len();
    for cut in [1, 2, 37, len / 2, len - 1] {
        check_cut_header_alone(cut);
    }
}

validation::negative_control!(
    qa_m051_cut_off_header_line_alone_stays_an_error,
    "the whole header line with no newline is one complete JSON value, read, so must fail the error check",
    expected = "the reader accepts a file whose only line",
    check_cut_header_alone(header_line().to_string().len())
);

/// The first `keep` lines, then a cut-off frame-record-shaped line. After the header line or a frame record: dropped, its bytes stated, the session
/// incomplete and both summaries absent with "session incomplete" (R-299, unchanged by R-356).
fn check_incomplete_with_dropped(keep: usize) {
    let lines = lines(3);
    let mut bytes = file(&lines[..keep]);
    let piece = &lines[1].as_bytes()[..50];
    bytes.extend_from_slice(piece);
    let got = read(&bytes[..])
        .unwrap_or_else(|e| panic!("the reader rejects a cut-off line {}: {e}", keep + 1));
    let want = whole(&lines);
    assert!(
        got.session == Session::Incomplete
            && got.dropped_bytes == piece.len() as u64
            && got.frames[..] == want.frames[..keep - 1]
            && got.leak_flags().map_err(|a| a.to_string()) == Err("session incomplete".to_owned())
            && got.hot_paths().map_err(|a| a.to_string()) == Err("session incomplete".to_owned()),
        "a cut-off line {} after {} frames does not read as an incomplete session with its {} bytes dropped",
        keep + 1,
        keep - 1,
        piece.len()
    );
}

#[test]
fn qa_m051_cut_off_line_before_the_summary_line_is_incomplete() {
    // After the header line (no frames), after the first frame, after the last frame.
    for keep in [1, 2, 4] {
        check_incomplete_with_dropped(keep);
    }
}

validation::negative_control!(
    qa_m051_cut_off_line_before_the_summary_line_is_incomplete,
    "a cut-off line after the summary line is a complete session, so must fail the incomplete check",
    expected = "does not read as an incomplete session",
    check_incomplete_with_dropped(5)
);

/// `bytes`, a file with something after its summary line that is not a cut-off last line, is not schema v1.
fn check_rejected(bytes: &[u8], what: &str) {
    assert!(
        read(bytes).is_err(),
        "not v1, but the reader accepts {what}"
    );
}

#[test]
fn qa_m051_after_the_summary_line_only_a_cut_off_line_is_dropped() {
    let lines = lines(3);
    let full = file(&lines);
    let with = |tail: &[u8]| -> Vec<u8> {
        let mut b = full.clone();
        b.extend_from_slice(tail);
        b
    };
    let piece = &lines[1].as_bytes()[..50];
    // The same cut-off piece ended by a newline: a malformed line, wherever it is.
    check_rejected(
        &with(&[piece, b"\n"].concat()),
        "a cut-off frame line ended by a newline after the summary line",
    );
    // ... and followed by another cut-off piece: only the last line can be dropped.
    check_rejected(
        &with(&[piece, b"\n", piece].concat()),
        "a malformed line, then a cut-off line, after the summary line",
    );
    // An unterminated last line that is one complete JSON value is read as any last line is: the summary line then
    // holds a frame's place, so the file is not v1.
    for (last, what) in [
        (lines[1].clone(), "a whole frame record"),
        (lines[lines.len() - 1].clone(), "a second summary line"),
        ("{}".to_owned(), "an empty object"),
        ("1".to_owned(), "a number"),
    ] {
        check_rejected(
            &with(last.as_bytes()),
            &format!("{what} with no newline after the summary line"),
        );
    }
    // A line in the summary line's place that is neither it nor a frame record is not v1, with a cut-off line after
    // it as without.
    for bad in [
        json!({ "leak_flags": null }),
        json!({ "leak_flags": null, "hot_paths": null, "extra": 1 }),
        json!({ "leak_flags": 1, "hot_paths": null }),
    ] {
        let mut kept = lines[..lines.len() - 1].to_vec();
        kept.push(bad.to_string());
        check_rejected(&file(&kept), &format!("{bad} as the last line"));
        let mut cut = file(&kept);
        cut.extend_from_slice(piece);
        check_rejected(&cut, &format!("{bad} before a cut-off line"));
    }
}

validation::negative_control!(
    qa_m051_after_the_summary_line_only_a_cut_off_line_is_dropped,
    "a cut-off line with no newline after the summary line is read, so must fail the rejection check",
    expected = "not v1, but the reader accepts",
    {
        let lines = lines(3);
        let mut bytes = file(&lines);
        bytes.extend_from_slice(&lines[1].as_bytes()[..50]);
        check_rejected(&bytes, "a cut-off line after the summary line")
    }
);
