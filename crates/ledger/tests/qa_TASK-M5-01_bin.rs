//! QA tests for TASK-M5-01, the `class_histogram` bin index (REQ-PAY-075; dd_generation_root §3.7 "Bin count"),
//! written from the docs, not from the implementation. The expected index is derived here from the payload contract's
//! own text (dd_simstate_payload § `sample_descriptor`: the `state` codes, and "`detail` … is meaningful only for
//! `state ∈ {escape, collision, sim_failed, decode_failed}`"), not from the ledger, and both generated sides are held to
//! it independently:
//! - the generated Rust `qr_class_histogram_bin(state, detail)` is `state | detail << 3` in a state that reads `detail`
//!   and `state` in every other (bounded, running, the reserved 6–7), at every 3-bit state and 2-bit detail;
//! - a histogram built with it over every 5-bit code sums to the copy count, leaves every bin of a nonzero `detail`
//!   under a non-reading state at 0, keeps the reserved states 6 and 7 in bins of their own, and a quad bounded
//!   everywhere with stray `detail` bits reads `dominant_outcome = 1` and `outcome_impurity = 0`;
//! - the generated WGSL bin index, run on the GPU at every 5-bit code, is the doc-derived one (not compared to Rust);
//! - §3.7 states the rule with the same four states.
//!
//! Each test has a registered negative control (R-176).

use std::path::Path;

use validation::gpu::GpuHarness;
use validation::negative_control;

/// The generated Rust struct and accessors, as checked in.
pub mod generated {
    include!("../../engine/src/quad/generated.rs");
}

fn read(path: &str) -> String {
    let p = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The `state` names by code, from the payload doc's `sample_descriptor` table ("0 escape · 1 bounded · …").
fn state_names() -> Vec<String> {
    let doc = read("docs/design/principia_dd_simstate_payload.md");
    let row = doc
        .lines()
        .find(|l| l.starts_with("| `state` | 0–2 |"))
        .expect("the payload doc's `state` row");
    let list = row
        .split("mutually-exclusive**:")
        .nth(1)
        .and_then(|s| s.split('.').next())
        .expect("the `state` code list");
    let mut names = Vec::new();
    for (k, item) in list.split('·').enumerate() {
        let mut it = item.split_whitespace();
        let code: usize = it.next().unwrap().parse().unwrap();
        assert_eq!(code, k, "state codes out of order in the payload doc");
        names.push(it.next().unwrap().to_owned());
    }
    assert_eq!(names.len(), 6, "the payload doc names six states");
    names
}

/// Per `state` code 0–7, whether payload §2 gives `detail` a meaning, from its "meaningful only for `state ∈ {…}`".
fn detail_read_from_doc() -> [bool; 8] {
    let doc = read("docs/design/principia_dd_simstate_payload.md");
    let set = doc
        .split("is meaningful only for `state ∈ {")
        .nth(1)
        .and_then(|s| s.split('}').next())
        .expect("the payload doc's `detail` meaning rule");
    let names = state_names();
    let mut read = [false; 8];
    for n in set.split(',').map(str::trim) {
        let k = names
            .iter()
            .position(|s| s == n)
            .unwrap_or_else(|| panic!("`{n}` is not a payload state"));
        read[k] = true;
    }
    read
}

/// The doc's bin index of a copy with `state` and `detail`.
fn want_bin(read: &[bool; 8], state: u32, detail: u32) -> u32 {
    if read[state as usize] {
        state | detail << 3
    } else {
        state
    }
}

fn check_rust_bin(bin: fn(u32, u32) -> u32) {
    let read = detail_read_from_doc();
    assert_eq!(
        read,
        [true, false, true, false, true, true, false, false],
        "the payload doc's detail-reading states moved; this test's derivation needs a look"
    );
    for state in 0..8u32 {
        for detail in 0..4u32 {
            let (got, want) = (bin(state, detail), want_bin(&read, state, detail));
            assert_eq!(
                got, want,
                "bin({state}, {detail}) is {got}, the doc gives {want}"
            );
        }
    }

    // A histogram over every 5-bit code, one copy each.
    let mut bins = [0u32; 32];
    for code in 0..32u32 {
        bins[bin(code & 7, code >> 3) as usize] += 1;
    }
    assert_eq!(
        bins.iter().sum::<u32>(),
        32,
        "the bins do not sum to the copy count"
    );
    for state in 0..8u32 {
        for d in 1..4u32 {
            if !read[state as usize] {
                assert_eq!(
                    bins[(state | d << 3) as usize],
                    0,
                    "bin of state {state} with detail {d} is reached, though the state does not read detail"
                );
            }
        }
    }
    for reserved in [6usize, 7] {
        assert_eq!(
            bins[reserved], 4,
            "reserved state {reserved} is not counted in a bin of its own"
        );
    }

    // A quad bounded everywhere, its copies' stray detail bits spread over 0–3.
    let mut bounded = [0u32; 32];
    for copy in 0..512u32 {
        bounded[bin(1, copy % 4) as usize] += 1;
    }
    let (dominant, &max) = bounded
        .iter()
        .enumerate()
        .max_by_key(|&(k, v)| (*v, std::cmp::Reverse(k)))
        .unwrap();
    let impurity = 1.0 - f64::from(max) / 512.0;
    assert_eq!(
        (dominant, impurity),
        (1, 0.0),
        "a quad bounded everywhere reads dominant {dominant}, impurity {impurity}"
    );
}

#[test]
fn qa_quad_reduction_bin_index_rust_is_payload_rule() {
    check_rust_bin(generated::qr_class_histogram_bin);
}

negative_control!(
    qa_quad_reduction_bin_index_rust_is_payload_rule,
    "the raw 5-bit code splits a bounded quad by its stray detail bits",
    expected = "bin(1, 1) is 9, the doc gives 1",
    check_rust_bin(|s, d| s | d << 3)
);

negative_control!(
    qa_quad_reduction_bin_index_reserved_reads_detail,
    "an index that reads detail in the reserved states 6 and 7 is not the doc's",
    expected = "bin(6, 1) is 14, the doc gives 6",
    check_rust_bin(|s, d| if s == 1 || s == 3 { s } else { s | d << 3 })
);

negative_control!(
    qa_quad_reduction_bin_index_escape_drops_detail,
    "an index that drops escape's body id merges the escape classes",
    expected = "bin(0, 1) is 0, the doc gives 8",
    check_rust_bin(|s, d| if s == 2 || s == 4 || s == 5 {
        s | d << 3
    } else {
        s
    })
);

/// The test kernel: the generated WGSL bin index of the code `a[i]`, `state` in bits 0–2 and `detail` in 3–4.
const KERNEL: &str = r"
@group(0) @binding(0) var<storage, read> a: array<u32>;
@group(0) @binding(1) var<storage, read_write> out: array<u32>;

@compute @workgroup_size(64)
fn qa_bin(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&a) { return; }
    out[i] = qr_class_histogram_bin(a[i] & 7u, (a[i] >> 3u) & 3u);
}
";

fn check_wgsl_bin(wgsl: &str) {
    let read = detail_read_from_doc();
    let codes: Vec<u32> = (0..32).collect();
    let gpu = GpuHarness::new().expect("a GPU device");
    let got = gpu.run_wgsl(&format!("{wgsl}\n{KERNEL}"), "qa_bin", &[&codes]);
    assert_eq!(got.len(), codes.len());
    for (&c, &g) in codes.iter().zip(&got) {
        let want = want_bin(&read, c & 7, c >> 3);
        assert_eq!(g, want, "WGSL bin of code {c} is {g}, the doc gives {want}");
    }
}

fn quad_wgsl() -> String {
    read("crates/render/frag/generated/quad_reduction.wgsl")
}

#[test]
fn qa_quad_reduction_bin_index_wgsl_is_payload_rule() {
    check_wgsl_bin(&quad_wgsl());
}

negative_control!(
    qa_quad_reduction_bin_index_wgsl_is_payload_rule,
    "a WGSL index that also reads running's detail is not the doc's",
    expected = "WGSL bin of code 11 is 11, the doc gives 3",
    check_wgsl_bin(&{
        let src = quad_wgsl();
        let at = src
            .find("let d = select(0u, detail, ")
            .expect("the WGSL bin index's select");
        let (head, tail) = src.split_at(at + "let d = select(0u, detail, ".len());
        format!("{head}state == 3u || {tail}")
    })
);

fn check_doc_rule(doc: &str) {
    let rule = doc
        .split("**A copy's bin index is `state | d << 3`, where `d` is")
        .nth(1)
        .expect("§3.7 does not state the bin index");
    let sentence = rule.split("**").next().unwrap();
    for s in ["escape", "collision", "sim_failed", "decode_failed"] {
        assert!(
            sentence.contains(s),
            "§3.7's bin-index rule does not name `{s}`"
        );
    }
    for s in ["bounded", "running"] {
        assert!(
            !sentence.contains(s),
            "§3.7's bin-index rule names `{s}` among the detail-reading states"
        );
    }
    assert!(
        sentence.contains("0 in"),
        "§3.7's bin-index rule does not give d = 0 elsewhere"
    );
}

#[test]
fn qa_quad_reduction_bin_index_doc_states_rule() {
    check_doc_rule(&read("docs/design/principia_dd_generation_root.md"));
}

negative_control!(
    qa_quad_reduction_bin_index_doc_states_rule,
    "a §3.7 whose bin count reads the raw code states no bin-index rule",
    expected = "§3.7 does not state the bin index",
    check_doc_rule("the code `state | detail << 3` that `dominant_outcome` holds")
);
