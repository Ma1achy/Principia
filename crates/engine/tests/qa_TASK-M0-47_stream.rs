//! QA tests for TASK-M0-47 (REQ-TOOL-147) on the engine's streaming writer, `engine::contract::profile::Stream`,
//! written from the requirement's statement and verify detail, R-341, R-286, R-298 and telemetry §5 ("The file",
//! "`prin profile` streams its trace"), not from the implementation:
//!
//! - over a writer that records its flushes and a clock the test drives, with R-341's policy (every 60 frames or 1 s,
//!   whichever comes first): the header line is flushed when written; frame lines are flushed at the 60th frame since
//!   the last flush, or once 1 s has passed since it, never later; each flush leaves whole lines; the summary line is
//!   written last and flushed;
//! - the stream refuses what `profile::write` refuses, with the same error, naming the same line ("An error names the
//!   line, from 1"), and writes nothing of a refused header or frame;
//! - a finished stream is the file `profile::write` writes for the same trace.
//!
//! The header and frame are hand-written from §5, key for key, so the writer is tested on input it did not produce.
//! Each test has its negative control (R-176).
// The file name `qa_TASK-M0-47_stream` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

use engine::contract::profile::{
    self, Flush, FrameRecord, SchemaId, Session, SessionHeader, Stream, Trace,
};
use serde_json::{json, Value};

/// R-341's flush, from the ruling's text: every 60 frames or 1 s, whichever comes first.
const R341: Flush = Flush {
    frames: 60,
    interval: Duration::from_secs(1),
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// One stage's nested sections (§5), with known values.
fn sections(name: &str) -> Value {
    json!({
        "scopes": [{ "name": name, "start_ms": 0.0, "ms": 0.5, "children": [] }],
        "gpu_passes": [],
        "allocations": [{ "kind": "buffer", "pool": "heap", "count": 1, "bytes": 64 }],
        "events": [{ "name": "done", "at_ms": 0.5, "detail": null }]
    })
}

/// A batch-render frame record (§5, §5.5: no present stage), hand-filled.
fn frame_value(index: u64, frame_ms: f64) -> Value {
    json!({
        "frame": index, "frame_ms": frame_ms, "quads_computed": 4, "quads_reused": 12, "samples": 4096,
        "substeps_total": 777, "playhead_dt": 0.25, "camera_delta": 0.0, "tree_depth_max": 3, "leaf_count": 16,
        "dmin_nan_unset": 1, "dmin_negative_floored": 2,
        "stage_ms": { "integrate": 1.0, "reduce": 0.5, "colour": 0.25, "upload": 0.125, "present": null },
        "stages": {
            "integrate": sections("dispatch"), "reduce": sections("reduction"), "colour": sections("stain"),
            "upload": sections("upload"), "present": null
        },
        "live_memory": {
            "heap": { "bytes": 64, "by_kind": [{ "kind": "buffer", "count": 1, "bytes": 64 }] },
            "gpu": { "bytes": 0, "by_kind": [] },
            "tile_cache": { "bytes": 0, "by_kind": [] }
        }
    })
}

/// A no-GPU header line (§5, R-308), hand-filled.
fn header_line() -> Value {
    json!({
        "schema": "principia-profile-v1",
        "header": {
            "device": { "gpu": null, "cpu": "test cpu", "cpu_cores_available": 4, "cpu_cores_total": null,
                        "gpu_cores": null, "memory": null },
            "backend": { "api": "none", "driver": null },
            "precision": null,
            "build": { "commit": "0123abc", "profile": "dev", "features": [] },
            "display": null,
            "config": { "scenario": "qa", "frames": 3 }
        }
    })
}

/// The fixture's header and a frame record, through the reader.
fn sample() -> (SessionHeader, FrameRecord) {
    let text = format!("{}\n{}\n", header_line(), frame_value(0, 2.5));
    let mut trace = profile::read(text.as_bytes()).expect("the hand-written fixture does not read");
    let frame = trace.frames.pop().expect("the fixture has no frame");
    (trace.header, frame)
}

fn numbered(frame: &FrameRecord, i: u64) -> FrameRecord {
    FrameRecord {
        frame: i,
        ..frame.clone()
    }
}

/// A writer that keeps its bytes and, at each flush, how many bytes it held then.
#[derive(Clone, Default)]
struct Spy(Rc<RefCell<(Vec<u8>, Vec<usize>)>>);

impl Spy {
    fn flushes(&self) -> Vec<usize> {
        self.0.borrow().1.clone()
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.borrow().0.clone()
    }
    /// The whole lines held at each flush.
    fn flushed_lines(&self) -> Vec<usize> {
        let held = self.0.borrow();
        held.1
            .iter()
            .map(|&n| held.0[..n].iter().filter(|b| **b == b'\n').count())
            .collect()
    }
}

impl Write for Spy {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut held = self.0.borrow_mut();
        let n = held.0.len();
        held.1.push(n);
        Ok(())
    }
}

/// Streams frame `i` at `t0 + at[i]` (offsets from the header's flush), then the summary line; the whole lines held
/// at each flush.
fn flushed_lines(flush: Flush, at: &[Duration]) -> Vec<usize> {
    let (header, frame) = sample();
    let spy = Spy::default();
    let t0 = Instant::now();
    let mut stream = Stream::start(spy.clone(), &header, flush, t0).expect("the header is refused");
    for (i, offset) in at.iter().enumerate() {
        stream
            .frame(&numbered(&frame, i as u64), t0 + *offset)
            .expect("a frame is refused");
    }
    stream.finish(&None, &None).expect("the summary is refused");
    spy.flushed_lines()
}

/// The lines held at each flush for `n` frames all at the header's instant: the header (line 1), every 60th frame
/// since the last flush (lines 61, 121, ...), and the summary line (n + 2).
fn by_count(n: usize) -> Vec<usize> {
    let mut want = vec![1];
    want.extend((1..=n / 60).map(|k| 1 + 60 * k));
    want.push(n + 2);
    want
}

/// The count rule at and around its edges, the clock held still: 0 frames (header and summary only), 1, 59, 60, 61,
/// 119, 120, 121 and 300 (REQ-TOOL-147: "flushed at the 60th frame since the last flush").
fn check_count_rule(flush: Flush) {
    for n in [0usize, 1, 59, 60, 61, 119, 120, 121, 300] {
        let got = flushed_lines(flush, &vec![Duration::ZERO; n]);
        let want = by_count(n);
        assert_eq!(
            got, want,
            "count rule: {n} frames flushed at lines {got:?}, not {want:?}"
        );
    }
}

#[test]
fn qa_m047_stream_count_rule_at_its_edges() {
    check_count_rule(R341);
}

validation::negative_control!(
    qa_m047_stream_count_rule_at_its_edges,
    "a policy flushing every 120 frames must fail the count rule",
    expected = "count rule:",
    check_count_rule(Flush {
        frames: 120,
        ..R341
    })
);

/// The clock rule at its edge: 999 ms after the last flush is not yet 1 s, 1000 ms is ("once 1 s has passed"); the
/// clock counts from the last flush, the header's or a count flush; a slow frame flushes alone (REQ-TOOL-147, R-341).
fn check_clock_rule(flush: Flush) {
    let got = flushed_lines(flush, &[ms(999), ms(1000)]);
    assert_eq!(got, [1, 3, 4], "clock rule: 999/1000 ms flushed at {got:?}");
    let got = flushed_lines(flush, &[ms(1500)]);
    assert_eq!(
        got,
        [1, 2, 3],
        "clock rule: a late first frame flushed at {got:?}"
    );
    // 60 frames at 10 ms flush by count (line 61); the clock then counts from 10 ms: 1009 ms holds, 1010 ms flushes.
    let mut at = vec![ms(10); 60];
    at.extend([ms(1009), ms(1010)]);
    let got = flushed_lines(flush, &at);
    assert_eq!(
        got,
        [1, 61, 63, 64],
        "clock rule: after a count flush, flushed at {got:?}"
    );
    // Frames 400 ms apart: the clock flushes at 1.2 s (frame 3, line 4) and at 2.4 s (frame 6, line 7).
    let at: Vec<Duration> = (1..=7).map(|k| ms(400 * k)).collect();
    let got = flushed_lines(flush, &at);
    assert_eq!(
        got,
        [1, 4, 7, 9],
        "clock rule: 400 ms frames flushed at {got:?}"
    );
}

#[test]
fn qa_m047_stream_clock_rule_at_its_edge() {
    check_clock_rule(R341);
}

validation::negative_control!(
    qa_m047_stream_clock_rule_at_its_edge,
    "a policy flushing only after 1.001 s must fail the clock rule",
    expected = "clock rule:",
    check_clock_rule(Flush {
        interval: ms(1001),
        ..R341
    })
);

/// A small deterministic generator (xorshift64), so each run is reproducible from its seed.
struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// "Never later" over random frame timings (mostly 0–40 ms apart, some up to 700 ms, a few after a pause over 1 s):
/// after each frame is written, either a flush has just happened, or fewer than 60 frame lines are unflushed and less
/// than 1 s has passed since the last flush. Every flush leaves whole lines, so a crash loses only whole frames since
/// the last flush; the summary line's flush comes last; the file reads as a complete session.
fn check_never_later(flush: Flush) {
    let (header, frame) = sample();
    for seed in 1..=20u64 {
        let mut gen = Gen(0x9E37_79B9_7F4A_7C15 ^ seed);
        let spy = Spy::default();
        let t0 = Instant::now();
        let mut stream =
            Stream::start(spy.clone(), &header, flush, t0).expect("the header is refused");
        let (mut now, mut last_flush, mut unflushed) = (t0, t0, 0u32);
        for i in 0..600u64 {
            let gap = match gen.next() % 100 {
                0..=79 => gen.next() % 41,
                80..=96 => gen.next() % 701,
                _ => 1000 + gen.next() % 600,
            };
            now += ms(gap);
            let before = spy.flushes().len();
            stream
                .frame(&numbered(&frame, i), now)
                .expect("a frame is refused");
            unflushed += 1;
            if spy.flushes().len() > before {
                unflushed = 0;
                last_flush = now;
            } else {
                assert!(
                    unflushed < 60 && now.duration_since(last_flush) < Duration::from_secs(1),
                    "flushed later than R-341 allows: seed {seed}, frame {i}: {unflushed} frame lines unflushed, \
                     {:?} since the last flush",
                    now.duration_since(last_flush)
                );
            }
        }
        stream.finish(&None, &None).expect("the summary is refused");
        let bytes = spy.bytes();
        let flushes = spy.flushes();
        for &n in &flushes {
            assert!(
                n > 0 && bytes[n - 1] == b'\n',
                "seed {seed}: a flush at byte {n} left a partial line"
            );
        }
        assert_eq!(
            flushes.last(),
            Some(&bytes.len()),
            "seed {seed}: the summary line was not flushed last"
        );
        let trace = profile::read(bytes.as_slice()).expect("the streamed trace does not read");
        assert_eq!(trace.session, Session::Complete);
        assert_eq!(trace.frames.len(), 600);
    }
}

#[test]
fn qa_m047_stream_never_flushes_later() {
    check_never_later(R341);
}

validation::negative_control!(
    qa_m047_stream_never_flushes_later,
    "a policy of every 90 frames or 1.5 s must be caught flushing later",
    expected = "flushed later than R-341 allows",
    check_never_later(Flush {
        frames: 90,
        interval: ms(1500)
    })
);

/// The trace `profile::write` would be given for the same header and frames.
fn trace_of(header: &SessionHeader, frames: Vec<FrameRecord>) -> Trace {
    Trace {
        schema: SchemaId::V1,
        header: header.clone(),
        frames,
        leak_flags: None,
        hot_paths: None,
        session: Session::Complete,
        dropped_bytes: 0,
    }
}

/// A frame refused after `before` good ones (a negative `frame_ms`: "a number ≥ 0", §5) is refused by the stream with
/// the error `profile::write` gives for the same frames, naming the same line, frame i on line i + 2; nothing of it is
/// written, and the lines before it stand.
fn check_frame_refusal(bad_ms: f64) {
    let (header, frame) = sample();
    for before in [0u64, 1, 3, 64] {
        let good: Vec<FrameRecord> = (0..before).map(|i| numbered(&frame, i)).collect();
        let bad = FrameRecord {
            frame_ms: bad_ms,
            ..numbered(&frame, before)
        };
        let spy = Spy::default();
        let t0 = Instant::now();
        let mut stream =
            Stream::start(spy.clone(), &header, R341, t0).expect("the header is refused");
        for f in &good {
            stream.frame(f, t0).expect("a good frame is refused");
        }
        let held = spy.bytes();
        let streamed = stream.frame(&bad, t0);
        let mut frames = good.clone();
        frames.push(bad.clone());
        let written = profile::write(&trace_of(&header, frames), io::sink());
        let (Err(s), Err(w)) = (&streamed, &written) else {
            panic!(
                "refusal: after {before} frames, frame_ms {bad_ms} was not refused by both (stream {}, write {})",
                streamed.is_err(),
                written.is_err()
            );
        };
        assert_eq!(
            s.to_string(),
            w.to_string(),
            "refusal: the stream's error is not write's"
        );
        assert!(
            s.to_string().contains(&format!("line {}", before + 2)),
            "refusal: the error {s} does not name line {}",
            before + 2
        );
        assert_eq!(
            spy.bytes(),
            held,
            "refusal: part of a refused frame was written"
        );
    }
}

#[test]
fn qa_m047_stream_refuses_a_frame_as_write_does() {
    check_frame_refusal(-0.5);
}

validation::negative_control!(
    qa_m047_stream_refuses_a_frame_as_write_does,
    "a frame write accepts must fail the refusal check",
    expected = "refusal:",
    check_frame_refusal(0.5)
);

/// A header `profile::write` refuses (a NaN `refresh_hz`: "Every number is finite", §5) is refused by the stream with
/// write's error; nothing is written or flushed.
fn check_header_refusal(refresh_hz: f64) {
    let mut line = header_line();
    line["header"]["display"] =
        json!({ "width_px": 800, "height_px": 600, "refresh_hz": 60.0, "dpi_scale": 1.0 });
    let text = format!("{line}\n");
    let mut header = profile::read(text.as_bytes())
        .expect("the fixture does not read")
        .header;
    header.display.as_mut().expect("a display").refresh_hz = refresh_hz;
    let spy = Spy::default();
    let streamed = Stream::start(spy.clone(), &header, R341, Instant::now());
    let written = profile::write(&trace_of(&header, vec![]), io::sink());
    let (Err(s), Err(w)) = (&streamed, &written) else {
        panic!("header refusal: refresh_hz {refresh_hz} was not refused by both");
    };
    assert_eq!(
        s.to_string(),
        w.to_string(),
        "header refusal: the errors differ"
    );
    assert!(spy.bytes().is_empty(), "header refusal: bytes were written");
    assert!(spy.flushes().is_empty(), "header refusal: a flush was made");
}

#[test]
fn qa_m047_stream_refuses_a_header_as_write_does() {
    check_header_refusal(f64::NAN);
}

validation::negative_control!(
    qa_m047_stream_refuses_a_header_as_write_does,
    "a finite refresh rate must fail the refusal check",
    expected = "header refusal:",
    check_header_refusal(60.0)
);

/// A finished stream is the file `profile::write` writes for the same trace (§5: "A writer can stream the frames"; one
/// format), for 0, 1, 2 and 61 frames, with varied `frame_ms`; the header line was flushed alone, first.
fn check_same_file(drop_last: bool) {
    let (header, _) = sample();
    for n in [0u64, 1, 2, 61] {
        let frames: Vec<FrameRecord> = (0..n)
            .map(|i| {
                let v = frame_value(i, 0.1 + i as f64 / 7.0);
                serde_json::from_value(v).expect("the fixture frame does not deserialise")
            })
            .collect();
        let spy = Spy::default();
        let t0 = Instant::now();
        let mut stream =
            Stream::start(spy.clone(), &header, R341, t0).expect("the header is refused");
        assert_eq!(
            spy.flushed_lines(),
            [1],
            "same file: the header was not flushed alone"
        );
        for f in &frames {
            stream.frame(f, t0).expect("a frame is refused");
        }
        stream.finish(&None, &None).expect("the summary is refused");
        let mut want_frames = frames.clone();
        if drop_last {
            want_frames.pop();
        }
        let mut want = Vec::new();
        profile::write(&trace_of(&header, want_frames), &mut want).expect("write refuses");
        assert_eq!(
            String::from_utf8(spy.bytes()).expect("UTF-8"),
            String::from_utf8(want).expect("UTF-8"),
            "same file: {n} streamed frames are not the file write writes"
        );
    }
}

#[test]
fn qa_m047_stream_is_the_file_write_writes() {
    check_same_file(false);
}

validation::negative_control!(
    qa_m047_stream_is_the_file_write_writes,
    "a file one frame short must fail the comparison",
    expected = "same file:",
    check_same_file(true)
);
