//! QA tests for task/codegen-selftest-reuse (the owner's CI-additions order of 8 Nov, item 1): the GPU self-test prepares
//! its pipeline once and, for each case, updates the input and "fill[s] the output buffer with a sentinel before every
//! case so a case that writes nothing fails", keeping "all 256 cases, the bit-for-bit comparisons and the negative
//! control". Written from that order, not from the implementation, against what reuse could break:
//!
//! - a stale output passing for a case: a word a case does not write must read back that case's sentinel, never the
//!   word an earlier case wrote, whatever the sentinel (0, the fresh buffer's fill, included);
//! - a mismatched input length or count accepted: a prepared pipeline's buffers have fixed lengths, so a load of any
//!   other length or number of inputs must be refused, not truncated, padded or ignored;
//! - a reload leaking state between cases: every case on the one pipeline must give, bit for bit, what a pipeline
//!   prepared fresh for that case gives, whichever of several inputs changed since the last case.
//!
//! The kernels and fixtures here are qa's own. Each test registers its negative control (R-176), which feeds the test's
//! own check the output the failure would give.

use std::panic::{catch_unwind, AssertUnwindSafe};

use validation::gpu::{GpuHarness, Prepared};
use validation::negative_control;

/// Writes `output[i] = input[i]` only where `input[i]` is even, and leaves the odd words' outputs unwritten.
const PARTIAL_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn partial(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input) && (input[id.x] & 1u) == 0u) { output[id.x] = input[id.x]; }
}
";

/// Two inputs: `output[i] = a[i] ^ (b[i] * 0x9E3779B1)`, wrapping, so a change to either input changes every word.
const MIX_WGSL: &str = r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read> b: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@compute @workgroup_size(64)
fn mix(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&a)) { output[id.x] = a[id.x] ^ (b[id.x] * 0x9E3779B1u); }
}
";

/// [`MIX_WGSL`] on the host.
fn mix(a: &[u32], b: &[u32]) -> Vec<u32> {
    a.iter()
        .zip(b)
        .map(|(x, y)| x ^ y.wrapping_mul(0x9E37_79B1))
        .collect()
}

/// Words per case: off the workgroup size, so the last workgroup is partial.
const N: usize = 1000;

/// Sentinels spanning the word: 0 (what a fresh output buffer holds), all ones, a single high bit, an arbitrary word.
const SENTINELS: [u32; 4] = [0, u32::MAX, 1 << 31, 0x5A5A_0F0F];

fn harness() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("no GPU harness: {e}"))
}

/// `len` xorshift32 words from `seed`, none zero.
fn words(seed: u32, len: usize) -> Vec<u32> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        })
        .collect()
}

fn evens(seed: u32) -> Vec<u32> {
    words(seed, N).into_iter().map(|w| w & !1).collect()
}

fn odds(seed: u32) -> Vec<u32> {
    words(seed, N).into_iter().map(|w| w | 1).collect()
}

// ---- A stale output never passes ---------------------------------------------------------------------------------

/// [`PARTIAL_WGSL`]'s output for `input` loaded with `sentinel`: each even word copied, each odd word the sentinel.
fn check_partial(input: &[u32], sentinel: u32, got: &[u32]) {
    assert_eq!(
        got.len(),
        input.len(),
        "the readback's length is not the input's"
    );
    for (i, (w, g)) in input.iter().zip(got).enumerate() {
        let want = if w & 1 == 0 { *w } else { sentinel };
        assert_eq!(
            *g, want,
            "stale output: word {i} (input {w:#010x}) read back {g:#010x}, not {want:#010x} (sentinel {sentinel:#010x})"
        );
    }
}

/// A case after a case that wrote every word: the words it does not write read back its own sentinel, not the earlier
/// case's output, for every sentinel and for a case that writes half its words or none.
fn stale_cases(h: &GpuHarness) -> (Prepared<'_>, Vec<(Vec<u32>, u32)>) {
    let first = evens(1);
    let prepared = h.prepare(PARTIAL_WGSL, "partial", &[&first]);
    check_partial(&first, 0, &prepared.run());
    let mut cases = Vec::new();
    for (k, &sentinel) in SENTINELS.iter().enumerate() {
        let seed = 10 + k as u32;
        cases.push((evens(seed), sentinel));
        cases.push((words(seed, N), sentinel));
        cases.push((odds(seed), sentinel));
    }
    (prepared, cases)
}

#[test]
fn qa_reuse_unwritten_words_read_the_cases_sentinel() {
    let h = harness();
    let (prepared, cases) = stale_cases(&h);
    for (input, sentinel) in &cases {
        prepared.load(&[input], *sentinel);
        check_partial(input, *sentinel, &prepared.run());
    }
}

negative_control!(
    qa_reuse_unwritten_words_read_the_cases_sentinel,
    "the output a case writing nothing would read with no sentinel fill (the previous case's) must fail",
    expected = "stale output",
    {
        let h = harness();
        let (prepared, cases) = stale_cases(&h);
        let (written, _) = &cases[0];
        prepared.load(&[written], 0);
        let stale = prepared.run();
        let (unwritten, sentinel) = (odds(99), 0x5A5A_0F0F);
        check_partial(&unwritten, sentinel, &stale);
    }
);

/// After `load`, before any dispatch, the output is the sentinel at every word.
fn check_filled(sentinel: u32, got: &[u32]) {
    assert_eq!(got.len(), N, "the readback's length is not the input's");
    if let Some(i) = got.iter().position(|w| *w != sentinel) {
        panic!(
            "load did not fill the output with the sentinel: word {i} is {:#010x}, not {sentinel:#010x}",
            got[i]
        );
    }
}

/// Each load, after a run that wrote every word, fills the whole output with its own sentinel.
#[test]
fn qa_reuse_load_fills_every_word_with_its_sentinel() {
    let h = harness();
    let input = evens(3);
    let prepared = h.prepare(PARTIAL_WGSL, "partial", &[&input]);
    for &sentinel in &SENTINELS {
        prepared.run();
        prepared.load(&[&input], sentinel);
        check_filled(sentinel, &prepared.read());
    }
}

negative_control!(
    qa_reuse_load_fills_every_word_with_its_sentinel,
    "the output of a run with no load after it, read as if loaded, must fail",
    expected = "load did not fill the output with the sentinel",
    {
        let h = harness();
        let input = evens(3);
        let prepared = h.prepare(PARTIAL_WGSL, "partial", &[&input]);
        prepared.run();
        check_filled(0, &prepared.read());
    }
);

// ---- A mismatched input length or count is refused ---------------------------------------------------------------

/// `load(inputs)` is refused: it panics rather than take inputs of other lengths or another number than prepared.
fn check_refused(prepared: &Prepared<'_>, inputs: &[&[u32]]) {
    let lengths: Vec<usize> = inputs.iter().map(|i| i.len()).collect();
    let refused = catch_unwind(AssertUnwindSafe(|| prepared.load(inputs, 0xdead_beef))).is_err();
    assert!(
        refused,
        "load accepted inputs of lengths {lengths:?} on a pipeline prepared with two of {N}"
    );
}

/// The mismatched loads: too few inputs, too many, each one short or long, both off with the total unchanged, and
/// both empty.
fn mismatched<'a>(a: &'a [u32], b: &'a [u32], long: &'a [u32]) -> Vec<Vec<&'a [u32]>> {
    vec![
        vec![],
        vec![a],
        vec![a, b, b],
        vec![&a[..N - 1], b],
        vec![a, &b[..N - 1]],
        vec![long, b],
        vec![a, long],
        vec![long, &b[..N - 1]],
        vec![&a[..0], &b[..0]],
    ]
}

#[test]
fn qa_reuse_load_refuses_other_lengths_and_counts() {
    let h = harness();
    let (a, b) = (words(5, N), words(6, N));
    let long = words(7, N + 1);
    let prepared = h.prepare(MIX_WGSL, "mix", &[&a, &b]);
    for inputs in mismatched(&a, &b, &long) {
        check_refused(&prepared, &inputs);
    }
    // The refusals leave the pipeline usable: a load of the prepared lengths runs exactly.
    let (c, d) = (words(8, N), words(9, N));
    prepared.load(&[&c, &d], 0);
    assert_eq!(
        prepared.run(),
        mix(&c, &d),
        "a load after refused loads did not run exactly"
    );
}

negative_control!(
    qa_reuse_load_refuses_other_lengths_and_counts,
    "a load of the prepared lengths and count, which must be taken, must fail the refusal check",
    expected = "load accepted inputs",
    {
        let h = harness();
        let (a, b) = (words(5, N), words(6, N));
        let prepared = h.prepare(MIX_WGSL, "mix", &[&a, &b]);
        check_refused(&prepared, &[&a, &b]);
    }
);

// ---- A reload leaks no state between cases -----------------------------------------------------------------------

/// Case `k`'s output on the reused pipeline is, bit for bit, the host's and a freshly prepared pipeline's.
fn check_case(k: usize, host: &[u32], fresh: &[u32], got: &[u32]) {
    assert_eq!(
        host, fresh,
        "case {k}: a fresh pipeline disagrees with the host"
    );
    if let Some(i) = host.iter().zip(got).position(|(x, y)| x != y) {
        panic!(
            "reload leaked state: case {k}'s word {i} is {:#010x}, not {:#010x}",
            got[i], host[i]
        );
    }
    assert_eq!(
        got.len(),
        host.len(),
        "reload leaked state: case {k}'s length"
    );
}

/// The cases: each changes `a` only, `b` only, or both, from the one before; the last repeats the first.
fn leak_cases() -> Vec<(Vec<u32>, Vec<u32>)> {
    let mut cases = vec![(words(20, N), words(21, N))];
    for k in 0..12u32 {
        let (a, b) = cases.last().cloned().unwrap();
        cases.push(match k % 3 {
            0 => (words(30 + k, N), b),
            1 => (a, words(50 + k, N)),
            _ => (words(70 + k, N), words(90 + k, N)),
        });
    }
    cases.push(cases[0].clone());
    cases
}

#[test]
fn qa_reuse_each_case_matches_a_fresh_pipeline() {
    let h = harness();
    let cases = leak_cases();
    let (a0, b0) = &cases[0];
    let prepared = h.prepare(MIX_WGSL, "mix", &[a0, b0]);
    for (k, (a, b)) in cases.iter().enumerate() {
        prepared.load(&[a, b], 0xdead_beef);
        let got = prepared.run();
        let fresh = h.run_wgsl(MIX_WGSL, "mix", &[a, b]);
        check_case(k, &mix(a, b), &fresh, &got);
    }
}

negative_control!(
    qa_reuse_each_case_matches_a_fresh_pipeline,
    "the previous case's output, what a load that never reached `b` gives, must fail the case's check",
    expected = "reload leaked state",
    {
        let h = harness();
        let cases = leak_cases();
        // Case 2 changes only `b` from case 1.
        let ((a1, b1), (a2, b2)) = (&cases[1], &cases[2]);
        assert_eq!(a1, a2, "the control's case does not change `b` alone");
        let prepared = h.prepare(MIX_WGSL, "mix", &[a1, b1]);
        prepared.load(&[a1, b1], 0xdead_beef);
        let previous = prepared.run();
        let fresh = h.run_wgsl(MIX_WGSL, "mix", &[a2, b2]);
        check_case(2, &mix(a2, b2), &fresh, &previous);
    }
);
