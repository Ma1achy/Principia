//! The word buffer and its generated accessors, from the ledger and in the generated files (dd_simstate_payload §0,
//! §3, §5, §6; dd_generation_root §3.3a; symbolic_dynamics_contract §4; R-86). The WGSL accessors run on the GPU through
//! `word_entry.wgsl`; `kernel/tests/word.rs` holds the Rust target to the same and the two to each other.
//! - REQ-PAY-025: no `SimState` type holds the word, in the ledger, the generated Rust or the generated WGSL; the word
//!   buffer's element is one `vec4<u32>`, exactly 16 B (`word_buffer_split`).
//! - REQ-PAY-028: the truncated fixture word (`fixtures/words/truncated.txt`) reads `fgw_reduced_length_valid` false,
//!   `sd_last_symbol_valid` false and its retained prefix 76 (`truncated_word_validity`).
//! - REQ-PAY-033: the accessors at lengths 0, 1, 76 and 127; each word accessor has payload §6's name and no second
//!   one, such as `fgw_prefix_length`, in either generated file (`word_accessor_names`).

use std::collections::BTreeSet;
use std::path::Path;

use ledger::gen::rust;
use ledger::schema::{Storage, Struct};
use naga::{Module, Scalar, TypeInner, VectorSize};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn checked_in(rel: &str) -> String {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn layer() -> String {
    checked_in(ledger::gen::wgsl::PATH)
}

fn generated_rust() -> String {
    checked_in(rust::PATH)
}

fn parse(source: &str) -> Module {
    naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
}

/// Whether `ty` is `vec4<u32>`.
fn is_vec4u(module: &Module, ty: naga::Handle<naga::Type>) -> bool {
    module.types[ty].inner
        == TypeInner::Vector {
            size: VectorSize::Quad,
            scalar: Scalar::U32,
        }
}

// ── word_buffer_split (REQ-PAY-025) ─────────────────────────────────────────────────────────────────────────────────

/// In the ledger's structs, no `SimState` variant has a `vec4<u32>` or a member naming the word, and the word buffer's
/// element is one `vec4<u32>` of 16 B, aligned to 16.
fn check_ledger_split(structs: &[Struct]) {
    let variants: Vec<&Struct> = structs
        .iter()
        .filter(|s| s.buffer == Some("SimState"))
        .collect();
    assert_eq!(variants.len(), 2, "the two SimState variants");
    for s in variants {
        for m in &s.members {
            assert!(
                m.storage != Storage::U32x4 && !m.name.contains("word"),
                "`{}.{}` puts the word in SimState: it lives in its own buffer (payload §0)",
                s.name,
                m.name
            );
        }
    }
    let word: Vec<&Struct> = structs
        .iter()
        .filter(|s| s.buffer == Some("word"))
        .collect();
    let [element] = word.as_slice() else {
        panic!(
            "the word buffer has {} element structs, not one",
            word.len()
        )
    };
    let storages: Vec<Storage> = element.members.iter().map(|m| m.storage).collect();
    assert_eq!(
        storages,
        [Storage::U32x4],
        "the word buffer's element is one vec4<u32>"
    );
    assert_eq!(
        (rust::offsets(element).1, element.align),
        (16, 16),
        "the word buffer's element is exactly 16 B"
    );
}

#[test]
fn word_buffer_split_in_the_ledger() {
    check_ledger_split(&ledger::payload::structs());
}

negative_control!(
    word_buffer_split_in_the_ledger,
    "a SimStateFTLE that stores the word inline must fail",
    expected = "puts the word in SimState",
    check_ledger_split(&{
        let mut structs = ledger::payload::structs();
        structs[0].members.push(ledger::schema::Member {
            name: "free_group_word",
            storage: Storage::U32x4,
        });
        structs
    })
);

/// The generated `SimState` structs hold no word: in `rust_source` neither `SimStateFTLEOf` nor `SimStateBaseOf` has a
/// `[u32; 4]` or a member naming the word; in the WGSL `wgsl_source` neither `SimStateFTLE` nor `SimStateBase` has a
/// `vec4<u32>` or a member naming the word. `word_buffer` is `array<vec4<u32>>`, its element 16 B at a 16 B stride.
fn check_generated_split(rust_source: &str, wgsl_source: &str) {
    for name in ["SimStateFTLEOf", "SimStateBaseOf"] {
        let open = format!("pub struct {name}<");
        let at = rust_source
            .find(&open)
            .unwrap_or_else(|| panic!("the generated Rust has no `{name}`"));
        let body = &rust_source[at..at + rust_source[at..].find("\n}\n").expect("its end")];
        assert!(
            !body.contains("[u32; 4]") && !body.contains("word"),
            "the generated Rust `{name}` holds the word: {body}"
        );
    }
    let module = parse(wgsl_source);
    for name in ["SimStateFTLE", "SimStateBase"] {
        let (_, t) = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("the WGSL has no `{name}`"));
        let TypeInner::Struct { members, .. } = &t.inner else {
            panic!("`{name}` is not a struct")
        };
        for m in members {
            let member = m.name.clone().unwrap_or_default();
            assert!(
                !is_vec4u(&module, m.ty) && !member.contains("word"),
                "the generated WGSL `{name}` holds the word in `{member}`"
            );
        }
    }
    let mut layouter = naga::proc::Layouter::default();
    layouter
        .update(module.to_ctx())
        .expect("the layouts compute");
    let (_, g) = module
        .global_variables
        .iter()
        .find(|(_, g)| g.name.as_deref() == Some("word_buffer"))
        .expect("the WGSL binds word_buffer");
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
        "word_buffer's element is not exactly 16 B"
    );
}

#[test]
fn word_buffer_split_in_the_generated_rust_and_wgsl() {
    check_generated_split(&generated_rust(), &layer());
}

negative_control!(
    word_buffer_split_in_the_generated_rust_and_wgsl,
    "a generated WGSL SimStateFTLE with the word inline must fail",
    expected = "the generated WGSL `SimStateFTLE` holds the word",
    check_generated_split(
        &generated_rust(),
        &layer().replacen(
            "struct SimStateFTLE {\n",
            "struct SimStateFTLE {\n    free_group_word: vec4<u32>,\n",
            1
        )
    )
);

// ── The WGSL accessors on the GPU ────────────────────────────────────────────────────────────────────────────────────

const ENTRY_WGSL: &str = include_str!("word_entry.wgsl");

/// What `word_entry.wgsl` reads: the six accessors, then `fgw_symbol(word, what − 6)`.
const ACCESSORS: u32 = 6;

/// Each of `reads`, `(word, what)`, read on the GPU through `layer`, the generated unpack layer.
fn gpu_read(gpu: &GpuHarness, layer: &str, reads: &[([u32; 4], u32)]) -> Vec<u32> {
    let module = format!("{layer}\n{ENTRY_WGSL}");
    let sels: Vec<u32> = (0u32..)
        .zip(reads)
        .map(|(c, &(_, what))| (c << 8) | what)
        .collect();
    let words: Vec<u32> = reads.iter().flat_map(|&(w, _)| w).collect();
    gpu.run_wgsl(&module, "t_word_read", &[&sels, &words])
}

/// `fixtures/words/truncated.txt`'s word and retained prefix.
fn fixture() -> ([u32; 4], Vec<u32>) {
    let text = checked_in("fixtures/words/truncated.txt");
    let field = |key: &str| -> Vec<String> {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('='))
            .unwrap_or_else(|| panic!("the fixture has no `{key}`"))
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    };
    let word: Vec<u32> = field("word")
        .iter()
        .map(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).expect("hex"))
        .collect();
    let prefix = field("prefix")
        .iter()
        .map(|s| {
            ["a", "A", "b", "B"]
                .iter()
                .position(|c| c == s)
                .expect("a symbol") as u32
        })
        .collect();
    (word.try_into().expect("four u32s"), prefix)
}

// ── truncated_word_validity (REQ-PAY-028) ───────────────────────────────────────────────────────────────────────────

/// The fixture `word`, read through `layer`: truncated, its reduced crossing count and last symbol invalid, its
/// retained prefix 76, never 127, and `fgw_symbol` giving the fixture's `prefix`, then none (4) past it.
fn check_truncated(gpu: &GpuHarness, layer: &str, word: [u32; 4], prefix: &[u32]) {
    let what: Vec<u32> = (0..ACCESSORS + 80).collect();
    let reads: Vec<([u32; 4], u32)> = what.iter().map(|&k| (word, k)).collect();
    let got = gpu_read(gpu, layer, &reads);
    assert_eq!(
        got[1], 1,
        "fgw_truncated is false for the truncated fixture word"
    );
    assert_eq!(
        got[2], 0,
        "fgw_reduced_length_valid is true for the truncated fixture word"
    );
    assert_eq!(
        got[5], 0,
        "sd_last_symbol_valid is true for the truncated fixture word"
    );
    assert_eq!(got[4], 76, "the retained prefix length is not 76");
    let mut want = prefix.to_vec();
    want.extend([4; 4]);
    assert_eq!(
        &got[ACCESSORS as usize..],
        want,
        "fgw_symbol of the retained prefix"
    );
}

#[test]
fn truncated_word_validity_wgsl_fixture() {
    let (word, prefix) = fixture();
    assert_eq!(prefix.len(), 76, "the fixture's retained prefix");
    check_truncated(
        &GpuHarness::new().expect("a GPU device"),
        &layer(),
        word,
        &prefix,
    );
}

negative_control!(
    truncated_word_validity_wgsl_fixture,
    "the fixture word with its length set to 76, a valid word, must fail",
    expected = "fgw_truncated is false",
    {
        let (mut word, prefix) = fixture();
        word[3] = (word[3] & 0x01ff_ffff) | (76 << 25);
        check_truncated(
            &GpuHarness::new().expect("a GPU device"),
            &layer(),
            word,
            &prefix,
        )
    }
);

// ── word_accessor_names (REQ-PAY-033) ───────────────────────────────────────────────────────────────────────────────

/// A word of `length_raw` `length` over a payload of all ones.
fn word_of_length(length: u32) -> [u32; 4] {
    [u32::MAX, u32::MAX, u32::MAX, (length << 25) | 0x01ff_ffff]
}

/// The six accessors at `length_raw` 0, 1, 76 and 127: `fgw_length_raw`, `fgw_truncated`, `fgw_reduced_length_valid`,
/// `fgw_reduced_length`, `fgw_retained_prefix_length` and `sd_last_symbol_valid(fgw_length_raw(word))`, a bool as 0 or
/// 1. The reduced count reads 127 at 127, but is invalid there; the retained prefix clamps it to 76.
const AT_LENGTHS: [(u32, [u32; 6]); 4] = [
    (0, [0, 0, 1, 0, 0, 0]),
    (1, [1, 0, 1, 1, 1, 1]),
    (76, [76, 0, 1, 76, 76, 1]),
    (127, [127, 1, 0, 127, 76, 0]),
];

fn check_at_lengths(gpu: &GpuHarness, layer: &str) {
    let reads: Vec<([u32; 4], u32)> = AT_LENGTHS
        .iter()
        .flat_map(|&(n, _)| (0..ACCESSORS).map(move |k| (word_of_length(n), k)))
        .collect();
    let got = gpu_read(gpu, layer, &reads);
    for ((n, want), got) in AT_LENGTHS.iter().zip(got.chunks(ACCESSORS as usize)) {
        assert_eq!(got, want, "the WGSL accessors at length {n}");
    }
}

#[test]
fn word_accessor_names_outputs_at_lengths_0_1_76_127() {
    check_at_lengths(&GpuHarness::new().expect("a GPU device"), &layer());
}

negative_control!(
    word_accessor_names_outputs_at_lengths_0_1_76_127,
    "a retained prefix that exposes 127 must fail",
    expected = "the WGSL accessors at length 127",
    {
        let from = "select(fgw_length_raw(word), FGW_CAPACITY, fgw_truncated(word))";
        let src = layer();
        assert_eq!(src.matches(from).count(), 1, "the layer has one `{from}`");
        check_at_lengths(
            &GpuHarness::new().expect("a GPU device"),
            &src.replacen(from, "fgw_length_raw(word)", 1),
        )
    }
);

/// Payload §6's word accessors, each to be defined once by its name (R-86).
const PAYLOAD_6: [&str; 6] = [
    "fgw_length_raw",
    "fgw_truncated",
    "fgw_reduced_length_valid",
    "fgw_reduced_length",
    "fgw_retained_prefix_length",
    "sd_last_symbol_valid",
];

/// The functions of the unpack layer that take a word, a `vec4<u32>`: payload §6's five and `fgw_symbol`
/// (REQ-TOOL-025's `fgw_symbol(word, k)`), with its two steps over `W`'s limbs.
const WGSL_WORD_FNS: [&str; 8] = [
    "fgw_length_raw",
    "fgw_truncated",
    "fgw_reduced_length_valid",
    "fgw_reduced_length",
    "fgw_retained_prefix_length",
    "fgw_mixed_radix",
    "fgw_div3",
    "fgw_symbol",
];

/// The functions of the generated Rust that take a `[u32; 4]`: the WGSL's, and the pack the append writes with.
const RUST_WORD_FNS: [&str; 9] = [
    "fgw_length_raw",
    "fgw_truncated",
    "fgw_reduced_length_valid",
    "fgw_reduced_length",
    "fgw_retained_prefix_length",
    "fgw_mixed_radix",
    "fgw_div3",
    "fgw_symbol",
    "fgw_pack",
];

/// Names a word accessor has had beside payload §6's, none of which may be defined (R-86: one name per accessor).
const SECOND_NAMES: [&str; 5] = [
    "fgw_prefix_length",
    "fgw_length",
    "fgw_encounter_count",
    "encounter_count",
    "fgw_crossing_count",
];

/// Each generated file names each word accessor once, by payload §6's name, and no second one: the functions taking a
/// word in the WGSL `wgsl_source` are exactly [`WGSL_WORD_FNS`], those taking a `[u32; 4]` in the Rust `rust_source`
/// exactly [`RUST_WORD_FNS`], each of [`PAYLOAD_6`] is defined once in each, and none of [`SECOND_NAMES`] is defined.
fn check_one_name(wgsl_source: &str, rust_source: &str) {
    let module = parse(wgsl_source);
    let wgsl_fns: Vec<String> = module
        .functions
        .iter()
        .filter_map(|(_, f)| f.name.clone())
        .collect();
    let rust_fns: Vec<(String, String)> = rust_source
        .match_indices("pub fn ")
        .chain(rust_source.match_indices("pub const fn "))
        .map(|(at, open)| {
            let rest = &rust_source[at + open.len()..];
            let name = &rest[..rest.find('(').expect("a parameter list")];
            let params = &rest[name.len()..rest.find(')').expect("a parameter list's end")];
            (name.to_owned(), params.to_owned())
        })
        .collect();
    for (target, names) in [
        ("WGSL", wgsl_fns.clone()),
        ("Rust", rust_fns.iter().map(|(n, _)| n.clone()).collect()),
    ] {
        for name in PAYLOAD_6 {
            let n = names.iter().filter(|f| *f == name).count();
            assert_eq!(
                n, 1,
                "the generated {target} defines `{name}` {n} times, not once"
            );
        }
        for name in SECOND_NAMES {
            assert!(
                !names.iter().any(|f| f == name),
                "the generated {target} defines `{name}`, a second name for a word accessor"
            );
        }
    }
    let wgsl_word: BTreeSet<String> = module
        .functions
        .iter()
        .filter(|(_, f)| f.arguments.iter().any(|a| is_vec4u(&module, a.ty)))
        .filter_map(|(_, f)| f.name.clone())
        .collect();
    let want: BTreeSet<String> = WGSL_WORD_FNS.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        wgsl_word, want,
        "the generated WGSL's functions of a word are not payload §6's and fgw_symbol's"
    );
    let rust_word: BTreeSet<String> = rust_fns
        .iter()
        .filter(|(_, params)| params.contains("[u32; 4]"))
        .map(|(n, _)| n.clone())
        .collect();
    let want: BTreeSet<String> = RUST_WORD_FNS.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        rust_word, want,
        "the generated Rust's functions of a word are not payload §6's and fgw_symbol's"
    );
}

#[test]
fn word_accessor_names_one_name_each() {
    check_one_name(&layer(), &generated_rust());
}

negative_control!(
    word_accessor_names_one_name_each,
    "an unpack layer that also defines `fgw_prefix_length` must fail",
    expected = "a second name for a word accessor",
    check_one_name(
        &format!(
            "{}\nfn fgw_prefix_length(word: vec4<u32>) -> u32 {{ return fgw_retained_prefix_length(word); }}\n",
            layer()
        ),
        &generated_rust()
    )
);

// ── The `payload` entry the accessors need ───────────────────────────────────────────────────────────────────────────

/// With `fgw_w`'s `payload` entry gone, generation is refused naming it, and each emitter called past the driver
/// writes a failure naming it, `compile_error!` in Rust and `const_assert` in WGSL, so `fgw_symbol` and `fgw_pack` are
/// never emitted reading bits the ledger does not give (dd_generation_root §3.8: "A field without a complete entry
/// fails generation loudly"). With it, generation succeeds.
fn check_payload_refused(ledger: &ledger::schema::Ledger) {
    let message = match ledger::gen::generate(ledger, ledger::gen::EMITTERS) {
        Ok(_) => panic!("generation was not refused without `fgw_w.payload`"),
        Err(e) => e.to_string(),
    };
    assert!(
        message.contains("`fgw_w.payload` has no entry"),
        "the refusal does not name `fgw_w.payload`: {message}"
    );
    let full = ledger::payload::ledger();
    let entries: Vec<ledger::schema::Entry> = ledger::gen::validate(&full)
        .expect("the ledger validates")
        .into_iter()
        .filter(|e| e.name != "payload")
        .collect();
    let rust = rust::fgw(&entries);
    assert!(
        rust.contains("compile_error!(") && rust.contains("`fgw_w.payload`"),
        "the Rust emitter past the driver writes no compile_error naming `fgw_w.payload`: {rust:?}"
    );
    let wgsl = ledger::gen::wgsl::layer(&full.words, &entries, ledger::gen::read::Tier::FULL);
    assert!(
        wgsl.contains("const_assert false; // field `fgw_w.payload` has no entry"),
        "the WGSL emitter past the driver writes no const_assert naming `fgw_w.payload`"
    );
}

#[test]
fn word_accessor_names_need_the_payload_entry() {
    assert!(
        ledger::gen::generate(&ledger::payload::ledger(), ledger::gen::EMITTERS).is_ok(),
        "the payload ledger generates"
    );
    let mut missing = ledger::payload::ledger();
    missing.entries.retain(|e| e.name != Some("payload"));
    check_payload_refused(&missing);
}

negative_control!(
    word_accessor_names_need_the_payload_entry,
    "the complete ledger, which generates, must fail the refusal check",
    expected = "generation was not refused without `fgw_w.payload`",
    check_payload_refused(&ledger::payload::ledger())
);
