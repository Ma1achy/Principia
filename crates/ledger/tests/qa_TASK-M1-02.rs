//! QA tests for TASK-M1-02's WGSL side and the generated layouts, written from the requirements and payload §0, §3,
//! §6 (dd_simstate_payload), not from the implementation. Expectations come from a reference written here from the
//! corpus alone: payload §3's frozen table (`inverse = [1,0,3,2]`, `cont_symbol[e][prev]`) and Horner recurrence in a
//! u128, laid out per payload §3's `.w` bit map.
//! - REQ-PAY-023 / REQ-PAY-033 (`word_accessor_names_qa_gpu_*`): the generated WGSL accessors, run on the GPU, read
//!   random reduced words of 0…76 symbols packed by the reference, plus lengths 0, 1, 76 and 127, as payload §6 defines
//!   them, and `fgw_symbol` decodes every symbol.
//! - REQ-PAY-028 (`truncated_word_validity_qa_gpu`): truncated words (sentinel 127 over random payloads) read
//!   `fgw_reduced_length_valid` false, `sd_last_symbol_valid` false and retained prefix 76 on the GPU.
//! - REQ-PAY-025 (`word_buffer_split_qa_layouts`): the stored WGSL `SimStateFTLE` / `SimStateBase` hold no `vec4<u32>`
//!   and no word member, are 144 / 96 B at alignment 8 (payload §0: the word's removal is what drops the alignment from
//!   16), and `word_buffer` is `array<vec4<u32>>` at a 16 B stride. The read-side `SimState` (lowering Part 3a) carries
//!   `sample.word` by the lowering contract's own table, and is not stored, so it is not a stored `SimState` type.
//! - REQ-PAY-033 (`word_accessor_names_qa_no_alias`): no function in the generated WGSL (unpack layer and read side) or
//!   the generated Rust is a pure forward to a word accessor under another name, beyond payload §6's own
//!   `fgw_reduced_length`; no common second name exists.
//!
//! Each check takes what it tests as an argument; its negative control (R-176) runs it on a source with one fault.

use std::path::Path;

use naga::{Expression, Module, Scalar, Statement, TypeInner, VectorSize};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn checked_in(rel: &str) -> String {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn layer() -> String {
    checked_in("crates/render/frag/generated/payload_unpack.wgsl")
}

fn read_side() -> String {
    checked_in("crates/render/frag/generated/read_side.wgsl")
}

fn parse(source: &str) -> Module {
    naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
}

// ── Reference, from payload §3 ────────────────────────────────────────────────────────────────────────────────────

const CONT: [[u32; 4]; 3] = [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]];

fn ref_pack(word: &[u32]) -> [u32; 4] {
    let mut w: u128 = 0;
    for (k, &s) in word.iter().enumerate() {
        w = if k == 0 {
            s as u128
        } else {
            let e = (0..3)
                .find(|&e| CONT[e][word[k - 1] as usize] == s)
                .expect("reduced");
            3 * w + e as u128
        };
    }
    assert!(w < 1u128 << 121);
    [
        w as u32,
        (w >> 32) as u32,
        (w >> 64) as u32,
        ((w >> 96) as u32 & 0x01ff_ffff) | ((word.len() as u32) << 25),
    ]
}

/// A small deterministic generator (xorshift64*), so the GPU batch is fixed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }
}

/// Reduced words: lengths 0, 1, 2, 75, 76, then random lengths, each from a random first symbol and random digits.
fn words() -> Vec<Vec<u32>> {
    let mut rng = Rng(0x5eed_0f7a_5c00_0002);
    let mut lens = vec![0usize, 1, 2, 75, 76, 76];
    lens.extend((0..40).map(|_| (rng.next() % 77) as usize));
    lens.into_iter()
        .map(|len| {
            let mut w: Vec<u32> = Vec::with_capacity(len);
            for k in 0..len {
                let s = if k == 0 {
                    rng.next() % 4
                } else {
                    CONT[(rng.next() % 3) as usize][w[k - 1] as usize]
                };
                w.push(s);
            }
            w
        })
        .collect()
}

// ── The GPU entry ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Reads `t_out[i] = read(word t_case[i], what t_what[i])`: 0 length_raw, 1 truncated, 2 reduced_length_valid,
/// 3 reduced_length, 4 retained_prefix_length, 5 sd_last_symbol_valid(length_raw), 6 + k fgw_symbol(word, k).
const ENTRY: &str = r"
@group(0) @binding(0) var<storage, read> q_case: array<u32>;
@group(0) @binding(1) var<storage, read> q_what: array<u32>;
@group(0) @binding(2) var<storage, read> q_words: array<u32>;
@group(0) @binding(3) var<storage, read_write> q_out: array<u32>;

fn q_read(word: vec4<u32>, what: u32) -> u32 {
    if (what == 0u) { return fgw_length_raw(word); }
    if (what == 1u) { return select(0u, 1u, fgw_truncated(word)); }
    if (what == 2u) { return select(0u, 1u, fgw_reduced_length_valid(word)); }
    if (what == 3u) { return fgw_reduced_length(word); }
    if (what == 4u) { return fgw_retained_prefix_length(word); }
    if (what == 5u) { return select(0u, 1u, sd_last_symbol_valid(fgw_length_raw(word))); }
    return fgw_symbol(word, what - 6u);
}

@compute @workgroup_size(64)
fn q_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&q_case)) {
        let c = q_case[id.x];
        let word = vec4<u32>(q_words[4u * c], q_words[4u * c + 1u], q_words[4u * c + 2u], q_words[4u * c + 3u]);
        q_out[id.x] = q_read(word, q_what[id.x]);
    }
}
";

/// Each `(word, what)` read through `layer` on the GPU.
fn gpu_read(gpu: &GpuHarness, layer: &str, words: &[[u32; 4]], reads: &[(u32, u32)]) -> Vec<u32> {
    let module = format!("{layer}\n{ENTRY}");
    let cases: Vec<u32> = reads.iter().map(|r| r.0).collect();
    let whats: Vec<u32> = reads.iter().map(|r| r.1).collect();
    let flat: Vec<u32> = words.iter().flatten().copied().collect();
    gpu.run_wgsl(&module, "q_main", &[&cases, &whats, &flat])
}

/// Payload §6's value of read `what` on `w` whose symbols are `symbols` (the retained prefix).
fn want(w: [u32; 4], symbols: &[u32], what: u32) -> u32 {
    let raw = w[3] >> 25;
    match what {
        0 => raw,
        1 => (raw == 127) as u32,
        2 => (raw != 127) as u32,
        3 => raw,
        4 => {
            if raw == 127 {
                76
            } else {
                raw
            }
        }
        5 => (raw >= 1 && raw != 127) as u32,
        k => symbols[(k - 6) as usize],
    }
}

/// The accessors on `cases`, `(word, its retained symbols)`, against [`want`]; `fgw_symbol` at every retained index.
fn check_gpu(gpu: &GpuHarness, layer: &str, cases: &[([u32; 4], Vec<u32>)]) {
    let words: Vec<[u32; 4]> = cases.iter().map(|c| c.0).collect();
    let mut reads: Vec<(u32, u32)> = Vec::new();
    for (c, (_, symbols)) in cases.iter().enumerate() {
        for what in 0..6 + symbols.len() as u32 {
            reads.push((c as u32, what));
        }
    }
    let got = gpu_read(gpu, layer, &words, &reads);
    for (&(c, what), got) in reads.iter().zip(got) {
        let (w, symbols) = &cases[c as usize];
        assert_eq!(
            got,
            want(*w, symbols, what),
            "the WGSL read {what} of word {c} ({w:08x?}, length {})",
            symbols.len()
        );
    }
}

/// The valid cases: each reference-packed word with its symbols.
fn valid_cases() -> Vec<([u32; 4], Vec<u32>)> {
    words().into_iter().map(|w| (ref_pack(&w), w)).collect()
}

#[test]
fn word_accessor_names_qa_gpu_random_reduced_words() {
    let gpu = GpuHarness::new().expect("a GPU device");
    check_gpu(&gpu, &layer(), &valid_cases());
}

negative_control!(
    word_accessor_names_qa_gpu_random_reduced_words,
    "a layer with a corrupted continuation table row must fail",
    expected = "the WGSL read",
    {
        // fgw_after reads continuation_symbol; swapping digits 1 and 2 in the composition corrupts the decode.
        let src = layer();
        let from = "extractBits(perm, 2u * continuation_symbol(0u, e), 2u)";
        assert_eq!(src.matches(from).count(), 1, "the layer has one `{from}`");
        let bad = src.replacen(
            from,
            "extractBits(perm, 2u * continuation_symbol(1u, e), 2u)",
            1,
        );
        check_gpu(
            &GpuHarness::new().expect("a GPU device"),
            &bad,
            &valid_cases(),
        )
    }
);

/// Truncated words: a full reduced word's payload with the sentinel, plus random payloads with the sentinel.
fn truncated_cases() -> Vec<([u32; 4], Vec<u32>)> {
    let mut rng = Rng(0x7a5c_0002_dead_beef);
    let mut out = Vec::new();
    for w in words().into_iter().filter(|w| w.len() == 76) {
        let mut p = ref_pack(&w);
        p[3] = (p[3] & 0x01ff_ffff) | (127 << 25);
        out.push((p, w));
    }
    for _ in 0..8 {
        let p = [rng.next(), rng.next(), rng.next(), rng.next() | (127 << 25)];
        out.push((p, Vec::new()));
    }
    out
}

#[test]
fn truncated_word_validity_qa_gpu() {
    let gpu = GpuHarness::new().expect("a GPU device");
    check_gpu(&gpu, &layer(), &truncated_cases());
}

negative_control!(
    truncated_word_validity_qa_gpu,
    "an sd_last_symbol_valid that tests only len >= 1 must fail on truncated words",
    expected = "the WGSL read 5",
    {
        let src = layer();
        let from = "len >= 1u && len != 127u";
        assert_eq!(src.matches(from).count(), 1, "the layer has one `{from}`");
        check_gpu(
            &GpuHarness::new().expect("a GPU device"),
            &src.replacen(from, "len >= 1u", 1),
            &truncated_cases(),
        )
    }
);

// ── REQ-PAY-025: the stored layouts ───────────────────────────────────────────────────────────────────────────────

fn is_vec4u(module: &Module, ty: naga::Handle<naga::Type>) -> bool {
    module.types[ty].inner
        == TypeInner::Vector {
            size: VectorSize::Quad,
            scalar: Scalar::U32,
        }
}

fn check_layouts(source: &str) {
    let module = parse(source);
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).expect("layouts");
    for (name, size) in [("SimStateFTLE", 144), ("SimStateBase", 96)] {
        let (h, t) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no `{name}`"));
        let TypeInner::Struct { members, .. } = &t.inner else {
            panic!("`{name}` is not a struct")
        };
        for m in members {
            let n = m.name.clone().unwrap_or_default();
            assert!(
                !is_vec4u(&module, m.ty) && !n.contains("word") && !n.contains("fgw"),
                "the stored `{name}` holds the word in `{n}`"
            );
        }
        assert_eq!(
            (layouter[h].size, layouter[h].alignment.round_up(1)),
            (size, 8),
            "the stored `{name}` is not {size} B at alignment 8 (payload §0)"
        );
    }
    let (_, g) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some("word_buffer"))
        .expect("word_buffer is bound");
    let TypeInner::Array { base, stride, .. } = module.types[g.ty].inner else {
        panic!("word_buffer is not an array")
    };
    assert!(
        is_vec4u(&module, base),
        "word_buffer's element is not vec4<u32>"
    );
    assert_eq!(
        (layouter[base].size, stride),
        (16, 16),
        "word_buffer's element is not 16 B"
    );
}

#[test]
fn word_buffer_split_qa_layouts() {
    check_layouts(&layer());
}

negative_control!(
    word_buffer_split_qa_layouts,
    "a stored SimStateBase with the word appended must fail",
    expected = "the stored `SimStateBase` holds the word",
    check_layouts(&layer().replacen(
        "struct SimStateBase {\n",
        "struct SimStateBase {\n    w: vec4<u32>,\n",
        1
    ))
);

// ── REQ-PAY-033: one name each ────────────────────────────────────────────────────────────────────────────────────

/// Payload §6's word accessors and the validity helper that reads the word's length.
const ACCESSORS: [&str; 6] = [
    "fgw_length_raw",
    "fgw_truncated",
    "fgw_reduced_length_valid",
    "fgw_reduced_length",
    "fgw_retained_prefix_length",
    "sd_last_symbol_valid",
];

/// Names a second accessor might take.
const SECOND_NAMES: [&str; 9] = [
    "fgw_prefix_length",
    "fgw_length",
    "fgw_is_truncated",
    "fgw_word_truncated",
    "word_truncated",
    "fgw_reduced_count",
    "fgw_crossing_count",
    "reduced_crossing_count",
    "fgw_reduced_length_is_valid",
];

/// In WGSL `source`: each accessor defined once, no second name, and no function that only forwards its own arguments
/// to an accessor (`return f(args)`) other than payload §6's `fgw_reduced_length` over `fgw_length_raw`.
fn check_no_alias_wgsl(source: &str) {
    let module = parse(source);
    let names: Vec<String> = module
        .functions
        .iter()
        .filter_map(|(_, f)| f.name.clone())
        .collect();
    for a in ACCESSORS {
        assert_eq!(
            names.iter().filter(|n| *n == a).count(),
            1,
            "`{a}` is not defined once in the WGSL"
        );
    }
    for s in SECOND_NAMES {
        assert!(
            !names.iter().any(|n| n == s),
            "the WGSL defines `{s}`, a second name"
        );
    }
    for (_, f) in module.functions.iter() {
        let name = f.name.clone().unwrap_or_default();
        let stmts: Vec<&Statement> = f
            .body
            .iter()
            .filter(|s| !matches!(s, Statement::Emit(_)))
            .collect();
        let [Statement::Call {
            function,
            arguments,
            result: Some(r),
        }, Statement::Return { value: Some(v) }] = stmts.as_slice()
        else {
            continue;
        };
        let forwards = r == v
            && arguments.len() == f.arguments.len()
            && arguments
                .iter()
                .enumerate()
                .all(|(i, &a)| matches!(f.expressions[a], Expression::FunctionArgument(j) if j as usize == i));
        let target = module.functions[*function].name.clone().unwrap_or_default();
        if forwards && ACCESSORS.contains(&target.as_str()) {
            assert!(
                name == "fgw_reduced_length" && target == "fgw_length_raw",
                "the WGSL `{name}` is a second name for `{target}`"
            );
        }
    }
}

/// In Rust `source`: each accessor `pub fn` once, no second name, and no `pub fn x(w: [u32; 4]) -> T { y(w) }` with
/// `y` an accessor, other than `fgw_reduced_length` over `fgw_length_raw`.
fn check_no_alias_rust(source: &str) {
    let fns: Vec<(String, String)> = source
        .split("pub fn ")
        .skip(1)
        .chain(source.split("pub const fn ").skip(1))
        .map(|rest| {
            let name = rest[..rest.find('(').expect("params")].to_owned();
            let body_open = rest.find('{').expect("a body");
            let body_close = rest.find("\n}").expect("a body end");
            (name, rest[body_open + 1..body_close].trim().to_owned())
        })
        .collect();
    for a in ACCESSORS {
        assert_eq!(
            fns.iter().filter(|(n, _)| n == a).count(),
            1,
            "`{a}` is not defined once in the Rust"
        );
    }
    for s in SECOND_NAMES {
        assert!(
            !fns.iter().any(|(n, _)| n == s),
            "the Rust defines `{s}`, a second name"
        );
    }
    for (name, body) in &fns {
        for a in ACCESSORS {
            if body == &format!("{a}(w)")
                || body == &format!("{a}(word)")
                || body == &format!("{a}(len)")
            {
                assert!(
                    name == "fgw_reduced_length" && a == "fgw_length_raw",
                    "the Rust `{name}` is a second name for `{a}`"
                );
            }
        }
    }
}

fn generated_rust() -> String {
    checked_in("crates/kernel/src/payload/generated.rs")
        + &checked_in("crates/kernel/src/payload/generated/read_side.rs")
}

fn wgsl_both() -> String {
    format!("{}\n{}", layer(), read_side())
}

#[test]
fn word_accessor_names_qa_no_alias() {
    check_no_alias_wgsl(&wgsl_both());
    check_no_alias_rust(&generated_rust());
}

negative_control!(
    word_accessor_names_qa_no_alias,
    "a WGSL forward under a new name must fail",
    expected = "is a second name for `fgw_retained_prefix_length`",
    check_no_alias_wgsl(&format!(
        "{}\nfn fgw_kept_length(word: vec4<u32>) -> u32 {{ return fgw_retained_prefix_length(word); }}\n",
        wgsl_both()
    ))
);

mod rust_alias {
    use super::*;

    #[test]
    fn word_accessor_names_qa_no_alias_rust() {
        check_no_alias_rust(&generated_rust());
    }

    negative_control!(
        word_accessor_names_qa_no_alias_rust,
        "a Rust forward under a new name must fail",
        expected = "is a second name for `fgw_truncated`",
        check_no_alias_rust(&format!(
            "{}\npub fn fgw_cut(w: [u32; 4]) -> bool {{\n    fgw_truncated(w)\n}}\n",
            generated_rust()
        ))
    );
}
