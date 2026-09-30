//! QA tests for TASK-M0-10's R-281/R-288/R-294 clauses of REQ-PAY-092, written from the requirement's statement and
//! verify detail, R-281, R-288, R-294 and telemetry §2, not from the implementation:
//!
//! - through the release path a NaN `d_min` stores the unset bits `0x7C00` and increments `dmin_nan_unset`; a negative
//!   `d_min` stores the floor `0x0001` and increments `dmin_negative_floored`; each case increments its own counter
//!   once and the other not at all, and a valid value increments neither;
//! - the counters are atomic u32s (telemetry §2), so concurrent packers sharing one frame's pair lose no count;
//! - the counters belong to the frame, never a static (R-294): the packer's caller passes in its frame's pair and reads
//!   it back, so two frames' pairs hold their own counts, and in a release build `set_d_min` and the word packer
//!   `pack_packed_a` count into the pair their caller passes ("counted in release builds too; that's their purpose",
//!   R-288); in a debug build they assert first (R-281);
//! - `roundtrip_ctl`'s repack is an observation, not a store: it counts into no frame (R-288's applied note, R-294);
//! - the kernel holds no mutable static (R-294).
//!
//! "Negative" is IEEE 754's: a value that compares below zero. −0.0 compares equal to +0.0, so it is a zero distance,
//! stored as the floor by R-271 and counted by neither. Each test has a registered negative control (R-176).

use kernel::payload::roundtrip::{roundtrip_ctl, PackedA};
use kernel::payload::*;
use std::sync::atomic::Ordering;
use validation::negative_control;

/// `(dmin_nan_unset, dmin_negative_floored)` as the frame's caller reads them back (R-294), cross-checked against the
/// two atomics loaded directly, so a `read` that swaps or drops a counter is caught wherever this is called.
fn pair(c: &DminCounters) -> (u32, u32) {
    let direct = (
        c.dmin_nan_unset.load(Ordering::SeqCst),
        c.dmin_negative_floored.load(Ordering::SeqCst),
    );
    assert_eq!(
        c.read(),
        direct,
        "DminCounters::read is not (dmin_nan_unset, dmin_negative_floored)"
    );
    direct
}

/// The `d_min` half of a `packed_a` word (payload §2: bits 16–31).
fn d_min_half(w: u32) -> u32 {
    w >> 16
}

/// What R-271/R-281 require for an f32 input: `(stored f16 bits, counts it adds)`, from the IEEE classification of
/// the input alone. Only the classes this file's inputs use are listed; a valid value's stored bits are given where
/// they are fixed by R-271 (floor, unset) or exact in binary16.
fn required(bits: u32) -> (u32, (u32, u32)) {
    let v = f32::from_bits(bits);
    if v.is_nan() {
        (0x7c00, (1, 0))
    } else if v < 0.0 {
        (0x0001, (0, 1))
    } else if bits == 0x7f80_0000 {
        (0x7c00, (0, 0))
    } else if v < 2f32.powi(-24) {
        (0x0001, (0, 0))
    } else if v == 1.0 {
        (0x3c00, (0, 0))
    } else if v == 0.5 {
        (0x3800, (0, 0))
    } else if v >= 65504.0 {
        (0x7bff, (0, 0))
    } else {
        panic!("input {bits:#010x} has no listed requirement")
    }
}

/// A mixed stream: NaNs and negatives interleaved with valid values, so a packer counting the wrong case, counting
/// twice, or counting a valid value is caught at the step it does so.
const STREAM: [u32; 16] = [
    0x3f80_0000, // 1.0
    0x7fc0_0000, // quiet NaN
    0xbf80_0000, // −1.0
    0x8000_0000, // −0.0: a zero distance
    0x0000_0000, // +0.0
    0xff80_0000, // −∞
    0x7f80_0000, // +∞: the unset value itself
    0x7f80_0001, // signalling NaN
    0x8000_0001, // −(smallest f32 subnormal)
    0x3100_0000, // 2⁻²⁹, below the f16 floor
    0x3f00_0000, // 0.5
    0xffc0_0000, // negative quiet NaN
    0x4974_2400, // 1e6, past 65504
    0xc77f_e000, // −65504
    0x7f7f_ffff, // f32::MAX
    0xff7f_ffff, // −f32::MAX
];

type Counted = fn(u32, f32, &DminCounters) -> u32;

/// `pack_packed_a` as a `d_min` store over `w`'s descriptor half, counting into the caller's `frame` (R-294).
fn pack_store(w: u32, v: f32, frame: &DminCounters) -> u32 {
    pack_packed_a(
        sd_state(w),
        sd_detail(w),
        sd_saturated(w),
        sd_dmin_pair(w),
        sd_last_symbol(w),
        v,
        frame,
    )
}

/// Over one fresh frame pair, `set` stores what [`required`] says for each input of [`STREAM`], keeps the descriptor
/// half, and after every input the pair equals the running tally. `lows` are the descriptor halves to cycle through.
fn check_stream_over(set: Counted, lows: &[u32]) {
    let c = DminCounters::new();
    assert_eq!(pair(&c), (0, 0), "a new frame's pair starts at zero");
    let mut tally = (0u32, 0u32);
    for (i, &bits) in STREAM.iter().enumerate() {
        let (want, add) = required(bits);
        let low = lows[i % lows.len()];
        let w = set(0x7bff_0000 | low, f32::from_bits(bits), &c);
        assert_eq!(d_min_half(w), want, "input {bits:#010x}: stored d_min bits");
        assert_eq!(w & 0xffff, low, "input {bits:#010x}: descriptor half kept");
        tally = (tally.0 + add.0, tally.1 + add.1);
        assert_eq!(
            pair(&c),
            tally,
            "after input {i} ({bits:#010x}): (dmin_nan_unset, dmin_negative_floored)"
        );
    }
    assert_eq!(
        tally,
        (3, 5),
        "the stream holds three NaNs and five negatives"
    );
}

/// [`check_stream_over`] with descriptor halves that exercise every bit of the low half, reserved bits included.
fn check_stream(set: Counted) {
    check_stream_over(set, &[0x0000, 0x03ff, 0xffff]);
}

#[test]
fn dmin_unset_qa_r288_release_path_stores_and_counts_each_case() {
    check_stream(set_d_min_release);
}

/// A release packer that stores the right bits but never counts (its pair is a throwaway).
#[cfg(feature = "controls")]
fn stores_only(w: u32, v: f32, _: &DminCounters) -> u32 {
    set_d_min_release(w, v, &DminCounters::new())
}

negative_control!(
    dmin_unset_qa_r288_release_path_stores_and_counts_each_case,
    "a packer that stores 0x7C00/0x0001 but increments nothing, so the running tally check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_stream(stores_only)
);

// A second control for the same test: counting −0.0 as a negative value (a sign-bit test instead of a comparison).
negative_control!(
    dmin_unset_qa_r288_release_path_negative_zero_is_not_counted,
    "a packer that counts every sign-bit-set non-NaN input, −0.0 included, so the tally check must fail at −0.0",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_stream(|w, v, c| {
        let out = set_d_min_release(w, v, &DminCounters::new());
        if v.is_nan() {
            c.dmin_nan_unset.fetch_add(1, Ordering::SeqCst);
        } else if v.is_sign_negative() {
            c.dmin_negative_floored.fetch_add(1, Ordering::SeqCst);
        }
        out
    })
);

/// A frame that stored `nans` NaNs and `negs` negatives through the release packer reads back `(nans, negs)` through
/// `read`, the caller's read-back (R-294).
fn check_read(read: fn(&DminCounters) -> (u32, u32), nans: u32, negs: u32) {
    let c = DminCounters::new();
    for _ in 0..nans {
        set_d_min_release(0, f32::NAN, &c);
    }
    for _ in 0..negs {
        set_d_min_release(0, -1.0, &c);
    }
    assert_eq!(
        read(&c),
        (nans, negs),
        "the caller's read-back: (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[test]
fn dmin_unset_qa_r294_read_returns_nan_then_negative() {
    check_read(DminCounters::read, 0, 0);
    check_read(DminCounters::read, 2, 5);
    check_read(DminCounters::read, 7, 0);
}

negative_control!(
    dmin_unset_qa_r294_read_returns_nan_then_negative,
    "a read-back with the counters swapped, so the read-back check must fail",
    expected = "the caller's read-back",
    check_read(
        |c| {
            let (a, b) = c.read();
            (b, a)
        },
        2,
        5
    )
);

/// In a release build `set_d_min` and `pack_packed_a` have no assertion and count into the frame pair their caller
/// passes, exactly as the release packer does (R-288, R-294). In a debug build they assert first, so only their
/// valid inputs are exercised here, and those leave the frame's pair at zero.
fn check_frame_packer_matches_release(set: Counted) {
    #[cfg(not(debug_assertions))]
    check_stream_over(set, &[0x0000, 0x03ff]);
    #[cfg(debug_assertions)]
    {
        let c = DminCounters::new();
        for &bits in STREAM.iter().filter(|&&b| required(b).1 == (0, 0)) {
            let w = set(0x03ff, f32::from_bits(bits), &c);
            assert_eq!(
                d_min_half(w),
                required(bits).0,
                "input {bits:#010x}: stored d_min bits"
            );
            assert_eq!(
                w & 0xffff,
                0x03ff,
                "input {bits:#010x}: descriptor half kept"
            );
        }
        assert_eq!(
            pair(&c),
            (0, 0),
            "valid inputs: (dmin_nan_unset, dmin_negative_floored)"
        );
    }
}

#[test]
fn dmin_unset_qa_r288_set_d_min_counts_into_the_callers_frame() {
    check_frame_packer_matches_release(set_d_min);
}

#[test]
fn dmin_unset_qa_r288_pack_packed_a_counts_into_the_callers_frame() {
    check_frame_packer_matches_release(pack_store);
}

negative_control!(
    dmin_unset_qa_r288_set_d_min_counts_into_the_callers_frame,
    "a packer that counts every store as a NaN, so the pair check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_frame_packer_matches_release(|w, v, c| {
        c.dmin_nan_unset.fetch_add(1, Ordering::SeqCst);
        set_d_min_release(w, v, &DminCounters::new())
    })
);

negative_control!(
    dmin_unset_qa_r288_pack_packed_a_counts_into_the_callers_frame,
    "a word packer that counts every store as negative in the caller's frame, so the pair check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_frame_packer_matches_release(|w, v, c| {
        c.dmin_negative_floored.fetch_add(1, Ordering::SeqCst);
        pack_store(w, v, c)
    })
);

// Release builds only, a second control: a word packer that stores the right bits but counts into a throwaway pair,
// not the caller's frame. In a debug build the check sees only valid inputs, which count nothing either way.
#[cfg(not(debug_assertions))]
negative_control!(
    dmin_unset_qa_r288_pack_packed_a_counts_into_no_other_pair,
    "a packer that counts into a pair of its own, not the caller's frame, so the tally check must fail",
    expected = "(dmin_nan_unset, dmin_negative_floored)",
    check_frame_packer_matches_release(|w, v, _| pack_store(w, v, &DminCounters::new()))
);

/// Debug builds: `store` (with the caller's frame pair) trips its `debug_assert!` for a NaN and for a negative `d_min`
/// (R-281), so storage never silently holds either.
#[cfg(debug_assertions)]
fn check_debug_asserts(store: Counted) {
    for v in [f32::NAN, -1.0, -f32::MIN_POSITIVE, f32::NEG_INFINITY] {
        let frame = DminCounters::new();
        let tripped = std::panic::catch_unwind(|| store(0, v, &frame)).is_err();
        assert!(
            tripped,
            "d_min {v:e} stored without a debug_assert! failure"
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn dmin_unset_qa_r288_debug_packers_assert_on_nan_and_negative() {
    check_debug_asserts(set_d_min);
    check_debug_asserts(pack_store);
}

#[cfg(debug_assertions)]
negative_control!(
    dmin_unset_qa_r288_debug_packers_assert_on_nan_and_negative,
    "the release packer has no assertion, so the debug-assert check must fail",
    expected = "without a debug_assert! failure",
    check_debug_asserts(set_d_min_release)
);

/// Threads sharing one frame's pair, owned by the caller (R-294), each storing `PER_THREAD` NaNs and as many
/// negatives through `set`, leave exactly `THREADS × PER_THREAD` in each counter: an atomic u32 loses no increment
/// (telemetry §2).
fn check_concurrent(set: Counted) {
    const THREADS: u32 = 8;
    const PER_THREAD: u32 = 5_000;
    let frame = DminCounters::new();
    std::thread::scope(|s| {
        for t in 0..THREADS {
            let frame = &frame;
            s.spawn(move || {
                for i in 0..PER_THREAD {
                    let nan = f32::from_bits(0x7fc0_0000 | (t << 8) | (i & 0xff));
                    let neg = -1.0 - i as f32;
                    assert_eq!(d_min_half(set(0, nan, frame)), 0x7c00);
                    assert_eq!(d_min_half(set(0, neg, frame)), 0x0001);
                }
            });
        }
    });
    assert_eq!(
        pair(&frame),
        (THREADS * PER_THREAD, THREADS * PER_THREAD),
        "concurrent stores: (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[test]
fn dmin_unset_qa_r288_counters_lose_no_concurrent_count() {
    check_concurrent(set_d_min_release);
    #[cfg(not(debug_assertions))]
    {
        check_concurrent(set_d_min);
        check_concurrent(pack_store);
    }
}

negative_control!(
    dmin_unset_qa_r288_counters_lose_no_concurrent_count,
    "a packer that drops every NaN count on the odd inputs, so the concurrent total must fall short",
    expected = "concurrent stores: (dmin_nan_unset, dmin_negative_floored)",
    check_concurrent(|w, v, c| {
        let throwaway = DminCounters::new();
        let keep = !v.is_nan() || v.to_bits() & 1 == 0;
        set_d_min_release(w, v, if keep { c } else { &throwaway })
    })
);

/// Two frames, each with its own pair from its caller (R-294): frame 0 stores two NaNs and one negative, frame 1 one
/// NaN and three negatives, interleaved store by store. Each frame reads back its own counts, and a third, new frame
/// starts at zero (a frame's counts are its own, never carried over).
fn check_frames_separate(set: Counted) {
    let frames = [DminCounters::new(), DminCounters::new()];
    let plan: [(usize, f32); 7] = [
        (0, f32::NAN),
        (1, -1.0),
        (1, f32::NAN),
        (0, -2.0),
        (1, -3.0),
        (0, f32::NAN),
        (1, f32::NEG_INFINITY),
    ];
    for (f, v) in plan {
        set(0, v, &frames[f]);
    }
    assert_eq!(
        (pair(&frames[0]), pair(&frames[1])),
        ((2, 1), (1, 3)),
        "each frame's read-back: (dmin_nan_unset, dmin_negative_floored)"
    );
    assert_eq!(
        pair(&DminCounters::new()),
        (0, 0),
        "a new frame starts at zero"
    );
}

#[test]
fn dmin_unset_qa_r294_frames_count_separately() {
    check_frames_separate(set_d_min_release);
    #[cfg(not(debug_assertions))]
    {
        check_frames_separate(set_d_min);
        check_frames_separate(pack_store);
    }
}

// The vetoed shape: every store counts into one shared pair, whatever frame the caller passed.
negative_control!(
    dmin_unset_qa_r294_frames_count_separately,
    "packs that all count into one shared pair, as the vetoed static did, so each frame's read-back check must fail",
    expected = "each frame's read-back",
    {
        let shared = DminCounters::new();
        let frames = [DminCounters::new(), DminCounters::new()];
        let plan: [(usize, f32); 7] = [
            (0, f32::NAN),
            (1, -1.0),
            (1, f32::NAN),
            (0, -2.0),
            (1, -3.0),
            (0, f32::NAN),
            (1, f32::NEG_INFINITY),
        ];
        for (f, v) in plan {
            let _ = &frames[f];
            set_d_min_release(0, v, &shared);
        }
        assert_eq!(
            (pair(&frames[0]), pair(&frames[1])),
            ((2, 1), (1, 3)),
            "each frame's read-back: (dmin_nan_unset, dmin_negative_floored)"
        );
    }
);

/// `ctl` fails every word whose `d_min` bits hold a value the packer would count (f16 NaNs, negatives) and passes the
/// clean word, while the caller's frame, which has counted one NaN and one negative of its own, still reads `(1, 1)`:
/// the check counts into no frame (R-288, R-294). `ctl` gets the frame so the control can misuse it.
fn check_repack_counts_nothing(ctl: &dyn Fn(&PackedA, u32, &DminCounters) -> bool) {
    let frame = DminCounters::new();
    set_d_min_release(0, f32::NAN, &frame);
    set_d_min_release(0, -1.0, &frame);
    assert_eq!(pair(&frame), (1, 1), "the frame's own stores");
    let good = pack_packed_a(1, 0, false, 2, 1, 0.5, &frame);
    let expected = PackedA::unpack(good);
    for h in [
        0x7c01u32, 0x7e00, 0xfe00, 0xffff, 0x8001, 0xbc00, 0xfbff, 0xfc00,
    ] {
        let observed = (h << 16) | (good & 0xffff);
        assert!(
            !ctl(&expected, observed, &frame),
            "roundtrip_ctl passed a contaminated d_min {h:#06x}"
        );
    }
    assert!(
        ctl(&expected, good, &frame),
        "roundtrip_ctl failed the clean word"
    );
    assert_eq!(
        pair(&frame),
        (1, 1),
        "roundtrip_ctl: the frame's (dmin_nan_unset, dmin_negative_floored)"
    );
}

#[test]
fn dmin_unset_qa_r288_roundtrip_repack_counts_nothing() {
    check_repack_counts_nothing(&|e, w, _| roundtrip_ctl(e, w));
}

negative_control!(
    dmin_unset_qa_r288_roundtrip_repack_counts_nothing,
    "a check whose repack stores each observed d_min through the caller's frame, so the unchanged-frame check must fail",
    expected = "roundtrip_ctl: the frame's",
    check_repack_counts_nothing(&|e, w, frame| {
        set_d_min_release(0, pa_d_min(w), frame);
        roundtrip_ctl(e, w)
    })
);

/// The kernel's sources hold no `static` item (R-294: "No mutable statics in the kernel"; an atomic in a plain
/// `static` is the vetoed shape, so every `static` item counts, not only `static mut`), nor a `thread_local!`. A token
/// scan: line comments are dropped, and `'static` is a lifetime, not an item.
fn static_items(path: &str, src: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let words: Vec<&str> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\'' || c == '!'))
            .collect();
        if words.iter().any(|&w| w == "static" || w == "thread_local!") {
            found.push(format!("{path}:{}: {}", n + 1, line.trim()));
        }
    }
    found
}

fn check_no_statics(sources: &[(String, String)]) {
    assert!(!sources.is_empty(), "no kernel sources read");
    let found: Vec<String> = sources
        .iter()
        .flat_map(|(p, s)| static_items(p, s))
        .collect();
    assert!(found.is_empty(), "a static item in the kernel: {found:?}");
}

/// Every `.rs` file under `dir`, recursively, as `(path, contents)`.
fn rust_sources(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            out.extend(rust_sources(&p));
        } else if p.extension().is_some_and(|x| x == "rs") {
            let s = std::fs::read_to_string(&p).unwrap();
            out.push((p.display().to_string(), s));
        }
    }
    out
}

#[test]
fn dmin_unset_qa_r294_kernel_holds_no_static() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = rust_sources(&src);
    assert!(
        sources.iter().any(|(p, _)| p.ends_with("counters.rs")),
        "the scan did not reach payload/counters.rs"
    );
    check_no_statics(&sources);
}

negative_control!(
    dmin_unset_qa_r294_kernel_holds_no_static,
    "the vetoed crate-level pair, added to the kernel's sources, so the no-static check must fail",
    expected = "a static item in the kernel",
    {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut sources = rust_sources(&src);
        sources.push((
            "counters.rs (vetoed)".to_owned(),
            "fn f(_: &'static str) {}\npub static DMIN_COUNTERS: DminCounters = DminCounters::new();\n".to_owned(),
        ));
        check_no_statics(&sources);
    }
);
