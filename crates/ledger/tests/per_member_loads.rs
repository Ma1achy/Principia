//! R-378 on the compiled shader output (REQ-RENDER-001; render contract Part 5, "Per-member loads"; lowering Part 3a's
//! R-378 note): the generated read side loads only the stored members each field needs, never the whole stored
//! struct, so a stain that reads one field loads only that field's words. Each check assembles a minimal stain, a
//! fragment entry point reading `sample` through the generated `sample_read`, compiles it through naga to MSL, SPIR-V
//! and HLSL, and reads off every load of `simstate_buffer` and `word_buffer` the backend wrote:
//! - MSL: each `simstate_buffer[…].<member>` and `word_buffer[…]` (`.x` … `.w` for a component);
//! - HLSL: each `simstate_buffer.Load<n>(<offset>+i*<stride>)`, a `ByteAddressBuffer` read at a byte offset, mapped to
//!   the member that offset falls in by the ledger's layout;
//! - SPIR-V: each `OpLoad` of an `OpAccessChain` rooted at the buffer's variable, its member index after the
//!   wrapper and the sample index.
//!
//! The field set `sample_read` fills is the stain's own ([`fields_read`]): each member of the read-side `SimState` the
//! stain's IR reads, the word's components where it reads only those, following the word into the function it is
//! passed to (`fgw_length_raw(sample.word)` reads `.w` alone). The negative controls run the same checks with the
//! checked-in read side, whose `sample_read` fills every field, and show they fail (pitfalls §9).
//!
//! Run with `--nocapture` for the excerpt of each backend's loads.

use std::collections::BTreeSet;
use std::path::Path;

use ledger::gen::read::{self, Tier};
use ledger::gen::{self, rust, wgsl};
use ledger::schema::{Entry, Word};
use naga::valid::{Capabilities, FunctionInfo, ModuleInfo, ValidationFlags, Validator};
use naga::{Block, Expression, Function, Handle, Module, Statement, TypeInner};
use validation::negative_control;

/// The ledger's words and validated entries, the generator's inputs.
fn ledger() -> (Vec<Word>, Vec<Entry>) {
    let ledger = ledger::payload::ledger();
    let entries = gen::validate(&ledger).expect("the ledger validates");
    (ledger.words, entries)
}

fn checked_in(rel: &str) -> String {
    let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A minimal stain: a fragment entry point that reads sample `i` through `sample_read` and returns `colour`.
fn stain(colour: &str) -> String {
    format!(
        "\n@fragment\nfn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {{\n    \
         let sample = sample_read(u32(p.x), 0.5, true, vec3<f32>(0.25, 0.35, 0.4), ReadParams(0.01, 1e-6, 16u, 1000u));\n    \
         return {colour};\n}}\n"
    )
}

fn parse(source: &str) -> (Module, ModuleInfo) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    (module, info)
}

// ── The fields a stain reads ───────────────────────────────────────────────────────────────────────────────────────

/// Each `Call` in `block`, nested blocks included: the callee and its arguments.
fn calls(block: &Block, out: &mut Vec<(Handle<Function>, Vec<Handle<Expression>>)>) {
    for s in block.iter() {
        match s {
            Statement::Call {
                function,
                arguments,
                ..
            } => out.push((*function, arguments.clone())),
            Statement::Block(b) => calls(b, out),
            Statement::If { accept, reject, .. } => {
                calls(accept, out);
                calls(reject, out);
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                calls(body, out);
                calls(continuing, out);
            }
            Statement::Switch { cases, .. } => cases.iter().for_each(|c| calls(&c.body, out)),
            _ => {}
        }
    }
}

/// The components of the `vec4<u32>` value `e` in `f` that are read: each `.x` … `.w` taken of it, and through each
/// function it is passed to, that function's reads of its argument; all four if it is used any other way.
fn components(
    module: &Module,
    info: &ModuleInfo,
    f: &Function,
    fn_info: &FunctionInfo,
    e: Handle<Expression>,
) -> [bool; 4] {
    let mut out = [false; 4];
    let mut uses = 0;
    for (_, x) in f.expressions.iter() {
        if let Expression::AccessIndex { base, index } = *x {
            if base == e {
                out[index as usize] = true;
                uses += 1;
            }
        }
    }
    let mut cs = Vec::new();
    calls(&f.body, &mut cs);
    for (callee, args) in cs {
        for (k, &a) in args.iter().enumerate() {
            if a != e {
                continue;
            }
            uses += 1;
            let g = &module.functions[callee];
            let arg = g
                .expressions
                .iter()
                .find(|(_, x)| matches!(x, Expression::FunctionArgument(j) if *j as usize == k))
                .map(|(h, _)| h);
            let inner = arg.map_or([true; 4], |h| components(module, info, g, &info[callee], h));
            for (o, i) in out.iter_mut().zip(inner) {
                *o |= i;
            }
        }
    }
    if fn_info[e].ref_count > uses {
        return [true; 4];
    }
    out
}

/// The read-side fields the stain in `module` reads, by naga's IR: each member of the read-side `SimState` taken in
/// any function but `sample_read`, which fills it; the word as `word`, or as `word.x` … `word.w` where only those
/// components are read ([`components`]). These are the fields its `sample_read` must fill (R-378).
fn fields_read(module: &Module, info: &ModuleInfo) -> BTreeSet<String> {
    let simstate = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("SimState"))
        .map(|(h, _)| h)
        .expect("the read-side SimState");
    let TypeInner::Struct { members, .. } = &module.types[simstate].inner else {
        panic!("SimState is not a struct");
    };
    let functions: Vec<(Handle<Function>, &Function)> = module
        .functions
        .iter()
        .filter(|(_, f)| f.name.as_deref() != Some("sample_read"))
        .collect();
    let mut out = BTreeSet::new();
    let mut read = |f: &Function, fn_info: &FunctionInfo| {
        for (h, x) in f.expressions.iter() {
            let Expression::AccessIndex { base, index } = *x else {
                continue;
            };
            let on_simstate = match *fn_info[base].ty.inner_with(&module.types) {
                TypeInner::Struct { .. } => fn_info[base].ty.handle() == Some(simstate),
                TypeInner::Pointer { base, .. } => base == simstate,
                _ => false,
            };
            if !on_simstate {
                continue;
            }
            let name = members[index as usize].name.clone().unwrap_or_default();
            if name != "word" {
                out.insert(name);
                continue;
            }
            let parts = components(module, info, f, fn_info, h);
            if parts.iter().all(|&p| p) {
                out.insert(name);
            } else {
                for (c, _) in parts.iter().enumerate().filter(|(_, &p)| p) {
                    out.insert(format!("word.{}", read::WORD_COMPONENTS[c]));
                }
            }
        }
    };
    for (h, f) in functions {
        read(f, &info[h]);
    }
    for (k, ep) in module.entry_points.iter().enumerate() {
        read(&ep.function, info.get_entry_point(k));
    }
    if out.contains("word") {
        out.retain(|f| !f.starts_with("word."));
    }
    out
}

// ── The loads each backend writes ──────────────────────────────────────────────────────────────────────────────────

/// The stored words a compiled stain loads: each `simstate_buffer` member by its WGSL name (`(whole)` for a load of the
/// whole stored struct), and each `word_buffer` component; and the lines that show them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Loads {
    state: BTreeSet<String>,
    word: BTreeSet<String>,
    excerpt: Vec<String>,
}

const WHOLE: &str = "(whole)";

/// The stored variant `tier` binds, and its WGSL members with their byte ranges, by the ledger's layout.
fn layout(tier: Tier) -> Vec<(String, u32, u32)> {
    let name = if tier.has_ftle {
        "SimStateFTLE"
    } else {
        "SimStateBase"
    };
    let s = ledger::payload::structs()
        .into_iter()
        .find(|s| s.name == name)
        .expect("the stored variant");
    let (offsets, size) = rust::offsets(&s);
    let wgsl_names = wgsl::members(&s);
    let mut out = Vec::new();
    for (k, (m, &at)) in s.members.iter().zip(&offsets).enumerate() {
        let end = offsets.get(k + 1).copied().unwrap_or(size);
        let name = wgsl_names
            .iter()
            .find(|w| w.stores.contains(&m.name))
            .map_or_else(|| m.name.to_owned(), |w| w.name.clone());
        out.push((name, at, end));
    }
    out
}

/// The text inside the brackets that open at `open` in `s`, and the index just past the closing one.
fn bracketed(s: &str, open: usize) -> (usize, usize) {
    let mut depth = 0;
    for (k, ch) in s[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return (open + 1, open + k + 1);
                }
            }
            _ => {}
        }
    }
    panic!("unclosed bracket in {s}");
}

/// The identifier at the start of `s`.
fn ident(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

fn msl(module: &Module, info: &ModuleInfo) -> String {
    use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
    let options = naga::back::msl::Options {
        // MSL 3.2 and wgpu's checked bounds, as the compute entry point uses (`engine::compute`).
        lang_version: (3, 2),
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Restrict,
            buffer: BoundsCheckPolicy::Restrict,
            image_load: BoundsCheckPolicy::Restrict,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        ..Default::default()
    };
    let pipeline = naga::back::msl::PipelineOptions::default();
    naga::back::msl::write_string(module, info, &options, &pipeline)
        .unwrap_or_else(|e| panic!("MSL: {e}"))
        .0
}

/// The loads in naga's MSL: `simstate_buffer[…].<member>`, or `simstate_buffer[…]` alone for the whole struct;
/// `word_buffer[…]` for the whole word, `.x` … `.w` after it for one component.
fn msl_loads(source: &str) -> Loads {
    let mut out = Loads::default();
    for line in source.lines() {
        let mut hit = false;
        for (buffer, word) in [("simstate_buffer[", false), ("word_buffer[", true)] {
            let mut from = 0;
            while let Some(at) = line[from..].find(buffer) {
                let open = from + at + buffer.len() - 1;
                let (_, past) = bracketed(line, open);
                let member = line[past..].strip_prefix('.').map(ident);
                hit = true;
                match (word, member) {
                    (false, Some(m)) => out.state.insert(unsuffixed(m).to_owned()),
                    (false, None) => out.state.insert(WHOLE.to_owned()),
                    (true, Some(c)) => out.word.insert(c.to_owned()),
                    (true, None) => {
                        out.word
                            .extend(read::WORD_COMPONENTS.iter().map(|c| (*c).to_owned()));
                        true
                    }
                };
                from = past;
            }
        }
        if hit {
            out.excerpt.push(line.trim().to_owned());
        }
    }
    out
}

/// A member's WGSL name from naga's MSL one: naga's namer appends `_` to a name that ends in a digit (`E_0` is
/// written `E_0_`), so that suffix is dropped.
fn unsuffixed(name: &str) -> &str {
    match name.strip_suffix('_') {
        Some(base) if base.ends_with(|c: char| c.is_ascii_digit()) => base,
        _ => name,
    }
}

fn hlsl(module: &Module, info: &ModuleInfo) -> String {
    let mut out = String::new();
    let options = naga::back::hlsl::Options::default();
    let pipeline = naga::back::hlsl::PipelineOptions::default();
    naga::back::hlsl::Writer::new(&mut out, &options, &pipeline)
        .write(module, info, None)
        .unwrap_or_else(|e| panic!("HLSL: {e}"));
    out
}

/// The loads in naga's HLSL: each `<buffer>.Load<n>(<offset>+…)` of a `ByteAddressBuffer`, the state's offset mapped
/// to the member whose bytes hold it (`layout`), the word's to the component at `offset / 4`, `n` words long.
fn hlsl_loads(source: &str, layout: &[(String, u32, u32)]) -> Loads {
    let mut out = Loads::default();
    for line in source.lines() {
        let mut hit = false;
        for (buffer, word) in [("simstate_buffer.Load", false), ("word_buffer.Load", true)] {
            let mut from = 0;
            while let Some(at) = line[from..].find(buffer) {
                let rest = &line[from + at + buffer.len()..];
                let width: u32 = rest[..1].parse().unwrap_or(1);
                let args = rest.trim_start_matches(|c: char| c.is_ascii_digit());
                let args = args.strip_prefix('(').unwrap_or(args);
                let digits = ident(args);
                let offset: u32 = if args[digits.len()..].starts_with('+') {
                    digits.parse().unwrap_or(0)
                } else {
                    0
                };
                hit = true;
                if word {
                    for k in 0..width {
                        let c = ((offset / 4 + k) % 4) as usize;
                        out.word.insert(read::WORD_COMPONENTS[c].to_owned());
                    }
                } else {
                    let member = layout
                        .iter()
                        .find(|(_, a, b)| (*a..*b).contains(&offset))
                        .map_or_else(|| format!("(offset {offset})"), |(m, ..)| m.clone());
                    out.state.insert(member);
                }
                from += at + buffer.len();
            }
        }
        if hit {
            out.excerpt.push(line.trim().to_owned());
        }
    }
    out
}

fn spirv(module: &Module, info: &ModuleInfo) -> Vec<u32> {
    naga::back::spv::write_vec(module, info, &naga::back::spv::Options::default(), None)
        .unwrap_or_else(|e| panic!("SPIR-V: {e}"))
}

/// The loads in naga's SPIR-V: each `OpLoad` whose pointer is an `OpAccessChain` (or a chain of them) rooted at the
/// variable `OpName`d `simstate_buffer` or `word_buffer`. Its indices are the block wrapper's `0`, the sample index,
/// then the member (state) or component (word); none after the sample index loads the whole element. The member index
/// is named by `members`, the stored variant's WGSL order, which naga keeps.
fn spirv_loads(words: &[u32], members: &[String]) -> Loads {
    const OP_NAME: u32 = 5;
    const OP_CONSTANT: u32 = 43;
    const OP_LOAD: u32 = 61;
    const OP_ACCESS_CHAIN: u32 = 65;
    const OP_IN_BOUNDS_ACCESS_CHAIN: u32 = 66;
    let mut names = std::collections::HashMap::new();
    let mut constants = std::collections::HashMap::new();
    let mut chains: std::collections::HashMap<u32, (u32, Vec<u32>)> =
        std::collections::HashMap::new();
    let mut out = Loads::default();
    let mut at = 5;
    while at < words.len() {
        let (count, op) = ((words[at] >> 16) as usize, words[at] & 0xffff);
        let ins = &words[at..at + count.max(1)];
        match op {
            OP_NAME => {
                let bytes: Vec<u8> = ins[2..].iter().flat_map(|w| w.to_le_bytes()).collect();
                let name: String = bytes
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect();
                names.insert(ins[1], name);
            }
            OP_CONSTANT => {
                constants.insert(ins[2], ins[3]);
            }
            OP_ACCESS_CHAIN | OP_IN_BOUNDS_ACCESS_CHAIN => {
                let (root, mut idx) = chains.get(&ins[3]).cloned().unwrap_or((ins[3], Vec::new()));
                idx.extend_from_slice(&ins[4..]);
                chains.insert(ins[2], (root, idx));
            }
            OP_LOAD => {
                if let Some((root, idx)) = chains.get(&ins[3]) {
                    let buffer = names.get(root).map_or("", String::as_str);
                    let element = idx
                        .get(2)
                        .and_then(|i| constants.get(i))
                        .map(|&v| v as usize);
                    let shown = match buffer {
                        "simstate_buffer" => {
                            let m = element.map_or(WHOLE.to_owned(), |k| members[k].clone());
                            out.state.insert(m.clone());
                            Some(format!("OpLoad (OpAccessChain %simstate_buffer 0 %i {m})"))
                        }
                        "word_buffer" => {
                            match element {
                                Some(c) => {
                                    out.word.insert(read::WORD_COMPONENTS[c].to_owned());
                                }
                                None => out
                                    .word
                                    .extend(read::WORD_COMPONENTS.iter().map(|c| (*c).to_owned())),
                            }
                            let c = element.map_or("(whole)", |c| read::WORD_COMPONENTS[c]);
                            Some(format!("OpLoad (OpAccessChain %word_buffer 0 %i {c})"))
                        }
                        _ => None,
                    };
                    out.excerpt.extend(shown);
                }
            }
            _ => {}
        }
        at += count.max(1);
    }
    out
}

/// The stain `colour` at `tier` after the generated layer and read side, its `sample_read` filling the fields the
/// stain's IR reads ([`fields_read`]) when `specialised`, or every field, as the checked-in read side does, for a
/// control.
fn assembled(tier: Tier, colour: &str, specialised: bool) -> String {
    let (words, entries) = ledger();
    let every: Vec<String> = read::members(&words, &entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let full = read::assemble(&words, &entries, tier, &every).expect("every field");
    if !specialised {
        return format!("{full}{}", stain(colour));
    }
    let (module, info) = parse(&format!("{full}{}", stain(colour)));
    let fields: Vec<String> = fields_read(&module, &info).into_iter().collect();
    let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
    let src = read::assemble(&words, &entries, tier, &fields).expect("the stain's fields");
    format!("{src}{}", stain(colour))
}

/// One case: the tier, the stain's colour, and the state members and word components it must load, no more.
struct Case {
    tier: Tier,
    colour: &'static str,
    state: &'static [&'static str],
    word: &'static [&'static str],
}

const NO_FTLE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};
const NO_WORD: Tier = Tier {
    has_ftle: true,
    has_word: false,
};

/// Each case's stain, at its tier, loads exactly its words on every backend; each backend's excerpt is printed.
fn check_loads(cases: &[Case], specialised: bool) {
    for c in cases {
        let source = assembled(c.tier, c.colour, specialised);
        let (module, info) = parse(&source);
        let layout = layout(c.tier);
        let members: Vec<String> = layout.iter().map(|(m, ..)| m.clone()).collect();
        let want_state: BTreeSet<String> = c.state.iter().map(|s| (*s).to_owned()).collect();
        let want_word: BTreeSet<String> = c.word.iter().map(|s| (*s).to_owned()).collect();
        for (backend, loads) in [
            ("MSL", msl_loads(&msl(&module, &info))),
            ("HLSL", hlsl_loads(&hlsl(&module, &info), &layout)),
            ("SPIR-V", spirv_loads(&spirv(&module, &info), &members)),
        ] {
            println!("--- {:?}, `{}`, {backend}:", c.tier, c.colour);
            for line in &loads.excerpt {
                println!("    {line}");
            }
            assert_eq!(
                (&loads.state, &loads.word),
                (&want_state, &want_word),
                "{backend} at {:?}: the stain returning `{}` loads other stored words than its fields need",
                c.tier,
                c.colour
            );
        }
    }
}

/// The cases: one field each, at the tiers that change what it loads.
const CASES: [Case; 9] = [
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(sample.d_min)",
        state: &["packed_a"],
        word: &[],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(sample.ftle)",
        state: &["r", "p", "r_sh", "p_sh", "S", "packed_a", "times"],
        word: &[],
    },
    Case {
        tier: NO_FTLE,
        colour: "vec4<f32>(sample.ftle)",
        state: &[],
        word: &[],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(sample.diffusion)",
        state: &["C_ty", "times"],
        word: &[],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(sample.closure_step))",
        state: &["closure_step_reserved"],
        word: &[],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(fgw_length_raw(sample.word)))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(sample.word.x + sample.word.y))",
        state: &[],
        word: &["x", "y"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(sample.word.x ^ sample.word.y ^ sample.word.z ^ sample.word.w))",
        state: &[],
        word: &["x", "y", "z", "w"],
    },
    Case {
        tier: NO_WORD,
        colour: "vec4<f32>(f32(fgw_length_raw(sample.word)))",
        state: &[],
        word: &[],
    },
];

/// The current drifts: each loads the state and its own reference alone, `E_0` or `Lz_0`, at both stored variants;
/// the masses are an argument, the sample's `ICDescriptor`'s, never a load (dd_generation_root §3.8; R-378).
const DRIFT_CASES: [Case; 4] = [
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(sample.energy_drift)",
        state: &["r", "p", "E_0"],
        word: &[],
    },
    Case {
        tier: NO_FTLE,
        colour: "vec4<f32>(sample.energy_drift)",
        state: &["r", "p", "E_0"],
        word: &[],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(sample.Lz_drift)",
        state: &["r", "p", "Lz_0"],
        word: &[],
    },
    Case {
        tier: NO_FTLE,
        colour: "vec4<f32>(sample.Lz_drift)",
        state: &["r", "p", "Lz_0"],
        word: &[],
    },
];

#[test]
fn per_member_loads_a_stain_loads_only_its_fields_words() {
    check_loads(&CASES, true);
}

negative_control!(
    per_member_loads_a_stain_loads_only_its_fields_words,
    "a stain reading `d_min` through a sample_read that fills every field loads every stored member",
    expected = "loads other stored words than its fields need",
    check_loads(&CASES[..1], false)
);

#[test]
fn current_drift_loads_only_the_state_and_its_reference() {
    check_loads(&DRIFT_CASES, true);
}

negative_control!(
    current_drift_loads_only_the_state_and_its_reference,
    "a stain reading `energy_drift` through a sample_read that fills every field loads every stored member",
    expected = "loads other stored words than its fields need",
    check_loads(&DRIFT_CASES[..1], false)
);

/// The word accessors (payload §6) and `fgw_symbol` on the read-side word: each length accessor reads `.w` alone, so a
/// stain reading one loads `word_buffer[i].w` and nothing else, at the tier with the word; `fgw_symbol` decodes the whole
/// word, so it loads all four components; without the word buffer nothing is loaded (R-378).
const WORD_CASES: [Case; 8] = [
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(fgw_reduced_length(sample.word)))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(select(0.0, 1.0, fgw_reduced_length_valid(sample.word)))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(select(0.0, 1.0, fgw_truncated(sample.word)))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(fgw_retained_prefix_length(sample.word)))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(select(0.0, 1.0, sd_last_symbol_valid(fgw_length_raw(sample.word))))",
        state: &[],
        word: &["w"],
    },
    Case {
        tier: Tier::FULL,
        colour: "vec4<f32>(f32(fgw_symbol(sample.word, 3u)))",
        state: &[],
        word: &["x", "y", "z", "w"],
    },
    Case {
        tier: NO_WORD,
        colour: "vec4<f32>(f32(fgw_reduced_length(sample.word)))",
        state: &[],
        word: &[],
    },
    Case {
        tier: NO_WORD,
        colour: "vec4<f32>(f32(fgw_symbol(sample.word, 3u)))",
        state: &[],
        word: &[],
    },
];

#[test]
fn per_member_loads_word_accessors_load_only_their_components() {
    check_loads(&WORD_CASES, true);
}

negative_control!(
    per_member_loads_word_accessors_load_only_their_components,
    "a stain reading `fgw_reduced_length` through a sample_read that fills every field loads every stored member",
    expected = "loads other stored words than its fields need",
    check_loads(&WORD_CASES[..1], false)
);

// ── The field set ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The fields each stain's IR reads are `want`.
fn check_fields_read(cases: &[(&str, &[&str])]) {
    let full = checked_in(wgsl::PATH) + &checked_in(read::WGSL_PATH);
    for &(colour, want) in cases {
        let (module, info) = parse(&format!("{full}{}", stain(colour)));
        let got = fields_read(&module, &info);
        let want: BTreeSet<String> = want.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(got, want, "the fields `{colour}` reads");
    }
}

const FIELD_CASES: [(&str, &[&str]); 4] = [
    ("vec4<f32>(sample.d_min)", &["d_min"]),
    (
        "vec4<f32>(sample.ftle, sample.S, f32(sample.is_failed), 1.0)",
        &["ftle", "S", "is_failed"],
    ),
    ("vec4<f32>(f32(fgw_length_raw(sample.word)))", &["word.w"]),
    (
        "vec4<f32>(f32(sample.word.x), f32(dot(vec4<f32>(sample.word), vec4<f32>(1.0))), 0.0, 1.0)",
        &["word"],
    ),
];

#[test]
fn per_member_loads_fields_read_from_the_stains_ir() {
    check_fields_read(&FIELD_CASES);
}

negative_control!(
    per_member_loads_fields_read_from_the_stains_ir,
    "a stain reading `d_min` and `S` reads two fields, not one",
    expected = "the fields",
    check_fields_read(&[("vec4<f32>(sample.d_min + sample.S)", &["d_min"])])
);

// ── An unknown field ───────────────────────────────────────────────────────────────────────────────────────────────

/// `fields` at the full tier is refused, naming the field that is none.
fn check_refused(fields: &[&str]) {
    let (words, entries) = ledger();
    let got = read::wgsl_for(&words, &entries, Tier::FULL, fields);
    assert!(
        got.as_ref()
            .is_err_and(|e| e.contains("is no read-side field")),
        "not refused: {fields:?}"
    );
}

#[test]
fn per_member_loads_an_unknown_field_is_refused() {
    check_refused(&["d_min", "speed"]);
    check_refused(&["word.q"]);
    check_refused(&["ftle.x"]);
}

negative_control!(
    per_member_loads_an_unknown_field_is_refused,
    "every field of the read side is filled",
    expected = "not refused",
    check_refused(&["d_min", "word.w"])
);

// ── The request ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Each pair of requests, at each tier, gives the same read side, which names the fields it fills as `named`.
fn check_same_read(pairs: &[(&[&str], &[&str], &str)]) {
    let (words, entries) = ledger();
    for tier in Tier::ALL {
        for &(a, b, named) in pairs {
            let read = |f: &[&str]| read::wgsl_for(&words, &entries, tier, f).expect("fields");
            let (x, y) = (read(a), read(b));
            assert!(
                x == y,
                "{a:?} and {b:?} give different read sides at {tier:?}"
            );
            assert!(
                x.contains(&format!("at this tier, {named} filled")),
                "the read side of {a:?} does not say it fills {named}"
            );
        }
    }
}

/// Requests in another order, repeated, or with a component of a word asked for whole.
const SAME_READS: [(&[&str], &[&str], &str); 4] = [
    (&["S", "d_min"], &["d_min", "S"], "only `S`, `d_min`"),
    (&["d_min", "d_min"], &["d_min"], "only `d_min`"),
    (&["word", "word.x"], &["word"], "only `word`"),
    (&[], &[], "no field"),
];

#[test]
fn per_member_loads_a_request_is_a_set_of_fields() {
    check_same_read(&SAME_READS);
}

negative_control!(
    per_member_loads_a_request_is_a_set_of_fields,
    "a request for a component alone fills that component, not the word",
    expected = "give different read sides",
    check_same_read(&[(&["word.x"], &["word"], "only `word`")])
);

/// The checked-in read side fills every field, and says so.
fn check_every_field(read_side: &str) {
    assert!(
        read_side.contains("at this tier, every field filled"),
        "the checked-in read side does not fill every field"
    );
}

#[test]
fn per_member_loads_the_checked_in_read_side_fills_every_field() {
    check_every_field(&checked_in(read::WGSL_PATH));
}

negative_control!(
    per_member_loads_the_checked_in_read_side_fills_every_field,
    "a read side filling one field does not fill every field",
    expected = "does not fill every field",
    {
        let (words, entries) = ledger();
        check_every_field(&read::wgsl_for(&words, &entries, Tier::FULL, &["d_min"]).expect("d_min"))
    }
);
