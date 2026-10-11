//! The `state` and `detail` accessors of the generated fragment unpack layer (`crates/render/frag/generated/
//! payload_unpack.wgsl`), run on the GPU, where the `i32`-`extractBits` sign-extension trap lives (debug_tooling_plan
//! §B, §H; dd_generation_root §5, tests 2 and 9):
//! - REQ-TOOL-021: `sd_state` round-trips every code 0–5, and the reserved 6–7, at the ledger's bits, whatever the
//!   word's other bits hold (`sd_state_roundtrip`).
//! - REQ-TOOL-022: `sd_detail` round-trips every code under every state, so `detail` decodes per state from its own
//!   bits (`sd_detail_roundtrip`).
//!
//! Every word is contaminated: the bits outside the field set in several patterns, so an accessor that reads one bit
//! too many, or sign-extends, reads a wrong code (PIT-9). Each check takes the generated WGSL as text, so its control
//! runs the same check on the text with one mutation and shows it fails (pitfalls §9).

use std::path::Path;

use ledger::gen::wgsl;
use ledger::schema::Location;
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The test entry, appended to the unpack layer: each invocation writes `sd_state` or `sd_detail` of its word, by
/// `t_sel`.
const ENTRY: &str = r"
@group(0) @binding(0) var<storage, read> t_word: array<u32>;
@group(0) @binding(1) var<storage, read> t_sel: array<u32>;
@group(0) @binding(2) var<storage, read_write> t_out: array<u32>;

@compute @workgroup_size(64)
fn t_descriptor(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= arrayLength(&t_word)) {
        return;
    }
    let w = t_word[i];
    t_out[i] = select(sd_detail(w), sd_state(w), t_sel[i] == 0u);
}
";

/// `t_sel`'s values.
const STATE: u32 = 0;
const DETAIL: u32 = 1;

/// The checked-in unpack layer.
fn unpack() -> String {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(wgsl::PATH);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `field`'s offset and width in `packed_a`, from the ledger.
fn bits(field: &str) -> (u32, u32) {
    let l = ledger::layout();
    let e = ledger::gen::validate(&l)
        .expect("the ledger validates")
        .into_iter()
        .find(|e| e.name == field)
        .unwrap_or_else(|| panic!("no `{field}`"));
    match e.location {
        Location::Packed {
            word: "packed_a",
            offset,
            width,
        } => (offset, width),
        other => panic!("`{field}` is at {other:?}, not in packed_a"),
    }
}

/// The contaminations a field's bits are written into: none, every other bit set, alternating bits both ways, the sign
/// bit alone, and words from a fixed linear congruential sequence.
fn backgrounds() -> Vec<u32> {
    let mut out = vec![0, u32::MAX, 0xAAAA_AAAA, 0x5555_5555, 0x8000_0000];
    let mut x: u32 = 0x1234_5678;
    for _ in 0..27 {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        out.push(x);
    }
    out
}

/// `value` written into `field`'s bits of `background`.
fn insert(background: u32, field: &str, value: u32) -> u32 {
    let (offset, width) = bits(field);
    let mask = ((1u32 << width) - 1) << offset;
    (background & !mask) | ((value << offset) & mask)
}

/// Runs the unpack layer `text` on `words`, `sel` choosing each one's accessor.
fn run(gpu: &GpuHarness, text: &str, words: &[u32], sel: &[u32]) -> Vec<u32> {
    gpu.run_wgsl(&format!("{text}\n{ENTRY}"), "t_descriptor", &[words, sel])
}

/// Checks that `text`'s `sd_state` reads back every code 0–7 from every contaminated word.
fn check_state(gpu: &GpuHarness, text: &str) {
    let mut words = Vec::new();
    let mut want = Vec::new();
    for code in 0..8 {
        for b in backgrounds() {
            words.push(insert(b, "state", code));
            want.push(code);
        }
    }
    let got = run(gpu, text, &words, &vec![STATE; words.len()]);
    for ((w, g), c) in words.iter().zip(&got).zip(&want) {
        assert_eq!(g, c, "sd_state({w:#010x}) is {g}, not the code {c} written");
    }
}

#[test]
fn sd_state_roundtrip() {
    assert_eq!(bits("state"), (0, 3), "payload §2: `state` is bits 0–2");
    assert_eq!(
        ledger::payload::states().len(),
        6,
        "codes 0–5 are the six states"
    );
    check_state(&GpuHarness::new().expect("a GPU device"), &unpack());
}

negative_control!(
    sd_state_roundtrip,
    "an `sd_state` reading bit 3, `detail`'s, as well reads a contaminated word wrong",
    expected = "sd_state(0xfffffff8) is 8, not the code 0 written",
    {
        let text = unpack();
        let from = "fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 3u); }";
        assert!(text.contains(from));
        let text = text.replace(
            from,
            "fn sd_state(w: u32) -> u32 { return extractBits(w, 0u, 4u); }",
        );
        check_state(&GpuHarness::new().expect("a GPU device"), &text);
    }
);

/// Checks that `text`'s `sd_detail` reads back every code 0–3 written under every state 0–5, and its `sd_state` the
/// state, from every contaminated word.
fn check_detail(gpu: &GpuHarness, text: &str) {
    let (mut words, mut sel, mut want) = (Vec::new(), Vec::new(), Vec::new());
    for state in 0..6 {
        for detail in 0..4 {
            for b in backgrounds() {
                let w = insert(insert(b, "state", state), "detail", detail);
                words.extend([w, w]);
                sel.extend([STATE, DETAIL]);
                want.extend([state, detail]);
            }
        }
    }
    let got = run(gpu, text, &words, &sel);
    for (k, ((w, g), c)) in words.iter().zip(&got).zip(&want).enumerate() {
        let what = if sel[k] == STATE {
            "sd_state"
        } else {
            "sd_detail"
        };
        assert_eq!(g, c, "{what}({w:#010x}) is {g}, not the code {c} written");
    }
}

#[test]
fn sd_detail_roundtrip() {
    assert_eq!(bits("detail"), (3, 2), "payload §2: `detail` is bits 3–4");
    check_detail(&GpuHarness::new().expect("a GPU device"), &unpack());
}

negative_control!(
    sd_detail_roundtrip,
    "an `sd_detail` through the i32 `extractBits` overload sign-extends codes 2 and 3",
    expected = "sd_detail(0x00000010) is 4294967294, not the code 2 written",
    {
        let text = unpack();
        let from = "fn sd_detail(w: u32) -> u32 { return extractBits(w, 3u, 2u); }";
        assert!(text.contains(from));
        let text = text.replace(
            from,
            "fn sd_detail(w: u32) -> u32 { return u32(extractBits(bitcast<i32>(w), 3u, 2u)); }",
        );
        check_detail(&GpuHarness::new().expect("a GPU device"), &text);
    }
);
