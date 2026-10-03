//! The payload ledger (dd_simstate_payload §0–§3; dd_generation_root §3.1, §3.3–§3.7): the hot `SimState` in its two
//! variants, the word buffer and its `.w` word, `ICDescriptor`, the packed words of `SimState`, §3.4's scalars with
//! their presentation metadata, payload §3's frozen continuation table, and §3.7's `QuadReduction` member list. Where the payload doc and generation-root §3 could drift, the payload doc is canonical
//! (R-70, R-86): §3.4's `delta_E_max_abs` and `delta_Lz_max_abs` are the payload's `dE_max` and `dLz_max`, and §3.8's
//! `shadow` is the payload's `r_sh` and `p_sh`.
//!
//! Where §3 gives a field no scale or range, the entry is `lin` over (−∞, ∞) (or its type's full range), as §3.8's
//! worked entries do for `n` and `ftle`; §3.6's log fields are > 0. Consumers are render, export and debug, as there.

use crate::constants::{FGW_CAPACITY, FGW_LENGTH_SENTINEL, HORIZON_STEPS_MAX};
use crate::schema::{
    Bound, Consumer, EntryBuilder, FieldType, Ledger, Location, Member, Overflow, Provenance,
    Range, Scale, Span, Storage, Struct, Word,
};

const SHOWN: &[Consumer] = &[Consumer::Render, Consumer::Export, Consumer::Debug];

fn entry(name: &'static str, location: Location, ty: FieldType, scale: Scale) -> EntryBuilder {
    EntryBuilder::new(name)
        .location(location)
        .ty(ty)
        .scale(scale)
        .range(Range {
            lo: Bound::Unbounded,
            hi: Bound::Unbounded,
        })
        .provenance(Provenance::Kernel)
        .consumers(SHOWN)
}

/// An f32 at scalar index `slot`.
fn f32_at(name: &'static str, slot: u32, scale: Scale) -> EntryBuilder {
    entry(name, Location::Scalar(slot), FieldType::F32, scale)
}

/// Three CoM-frame particle vectors, `vector(f32, 6)` stored as `array<vec2<f32>, 3>` (§3.8, R-86).
fn vec6_at(name: &'static str, slot: u32) -> EntryBuilder {
    let ty = FieldType::Vector {
        component: Box::new(FieldType::F32),
        k: 6,
    };
    entry(name, Location::Scalar(slot), ty, Scale::Lin)
}

fn packed(word: &'static str, offset: u32, width: u32) -> Location {
    Location::Packed {
        word,
        offset,
        width,
    }
}

/// A u-bits field of `packed_a`'s descriptor (§3.1).
fn descriptor(name: &'static str, offset: u32, width: u32, scale: Scale, hi: i64) -> EntryBuilder {
    let location = packed("packed_a", offset, width);
    entry(name, location, FieldType::UBits, scale).range(Range::int(0, hi))
}

/// An exact macro-step index, at most `horizon_steps_max` (§3.1, R-86).
fn step(name: &'static str, location: Location) -> EntryBuilder {
    let max = HORIZON_STEPS_MAX.number() as i64;
    entry(name, location, FieldType::UBits, Scale::Lin).range(Range::int(0, max))
}

/// An `f16-pair` latch ≥ 0 (or > 0), clamped to ±65504 before packing (payload §1).
fn latch(name: &'static str, location: Location, lo: Bound) -> EntryBuilder {
    let range = Range {
        lo,
        hi: Bound::Unbounded,
    };
    let f16 = entry(name, location, FieldType::F16Pair, Scale::Log).range(range);
    f16.overflow(Overflow::Saturate)
}

fn derived(name: &'static str, from: &[&'static str], scale: Scale) -> EntryBuilder {
    let from = from.to_vec();
    entry(name, Location::Derived { from }, FieldType::F32, scale)
}

/// `ICDescriptor`'s field at slot `slot` (§3.6); provenance decode.
fn ic(name: &'static str, slot: u32, scale: Scale) -> EntryBuilder {
    let e = f32_at(name, slot, scale).provenance(Provenance::Decode);
    match scale {
        Scale::Log => e.range(Range {
            lo: Bound::Open(0.0),
            hi: Bound::Unbounded,
        }),
        _ => e,
    }
}

/// The packed words of `SimState` (payload §1; §3.1), and `fgw_w`, the `.w` of the word buffer's `vec4<u32>` (payload
/// §3's `.w` bit map), whose fields tile it with nothing reserved.
fn words() -> Vec<Word> {
    let word = |name, reserved| Word {
        name,
        bits: 32,
        reserved,
    };
    vec![
        word(
            "packed_a",
            vec![Span {
                offset: 10,
                width: 6,
            }],
        ),
        word("packed_b", vec![]),
        word("times", vec![]),
        word("fgw_w", vec![]),
    ]
}

/// Every entry: `SimState`'s stored fields at their `SimStateFTLE` slots, the packed words' fields, §3.4's derived
/// scalars, and `ICDescriptor`'s twelve fields.
fn entries() -> Vec<EntryBuilder> {
    let from_zero = Range {
        lo: Bound::Closed(0.0),
        hi: Bound::Unbounded,
    };
    let u32s = Range::int(0, u32::MAX.into());
    let ubits = |name, slot| entry(name, Location::Scalar(slot), FieldType::UBits, Scale::Lin);
    vec![
        vec6_at("r", 0),
        vec6_at("p", 6),
        vec6_at("r_sh", 12),
        vec6_at("p_sh", 18),
        f32_at("S", 24, Scale::Lin),
        f32_at("theta", 25, Scale::Lin),
        f32_at("mean_y", 26, Scale::Lin),
        f32_at("C_ty", 27, Scale::Lin),
        f32_at("E_0", 28, Scale::Diverging),
        f32_at("Lz_0", 29, Scale::Diverging),
        ubits("total_substeps", 33).range(u32s),
        f32_at("closure_min", 34, Scale::Log).range(from_zero),
        step("closure_step", Location::Scalar(35)),
        descriptor("state", 0, 3, Scale::Categorical(6), 5),
        descriptor("detail", 3, 2, Scale::Categorical(4), 3),
        descriptor("saturated", 5, 1, Scale::Flag, 1),
        // Pair ids 0–2 (R-22); the stored code 3 is "unset/invalid" (payload §2), so it is the sentinel.
        descriptor("dmin_pair", 6, 2, Scale::Categorical(3), 2).sentinel(3.0),
        descriptor("last_symbol", 8, 2, Scale::Categorical(4), 3),
        // Unset, the minimum of an empty set, is f16 +∞: a failed sample's, and any sample's before its first step
        // (R-271, payload §1). A stored value is never 0.0: one below f16's smallest positive subnormal is stored as it.
        latch("d_min", packed("packed_a", 16, 16), Bound::Open(0.0)).sentinel(f64::INFINITY),
        latch("dE_max", packed("packed_b", 0, 16), Bound::Closed(0.0)),
        latch("dLz_max", packed("packed_b", 16, 16), Bound::Closed(0.0)),
        step("t_end_step", packed("times", 0, 16)),
        step("t_dmin_step", packed("times", 16, 16)),
        // No sentinel: `ftle` reads NaN whenever `ftle_valid` is false (R-253, R-254).
        derived(
            "ftle",
            &["S", "r_sh", "p_sh", "r", "p", "t_end_step"],
            Scale::Lin,
        )
        .tier_gate("ftle_valid"),
        // The drifts' log-magnitude views floor at the sim-key parameters `eps_E` and `eps_L` (§3.4, §3.8, R-263).
        derived(
            "energy_drift",
            &["r", "p", "m0", "m1", "m2", "E_0"],
            Scale::Diverging,
        )
        .floor("eps_E"),
        derived("Lz_drift", &["r", "p", "Lz_0"], Scale::Diverging).floor("eps_L"),
        derived("diffusion", &["C_ty", "t_end_step"], Scale::Lin),
        ic("m0", 0, Scale::Lin),
        ic("m1", 1, Scale::Lin),
        ic("m2", 2, Scale::Lin),
        ic("q_mass", 3, Scale::Lin),
        ic("rho_mag", 4, Scale::Lin),
        ic("lambda_mag", 5, Scale::Lin),
        ic("rho_ratio", 6, Scale::Log),
        ic("rho_angle", 7, Scale::Cyclic),
        ic("K_0", 8, Scale::Diverging),
        ic("V_0", 9, Scale::Diverging),
        ic("virial_ratio", 10, Scale::Lin),
        ic("r_min_pair_0", 11, Scale::Log),
        // Payload §3's `.w` bit map: the high 25 bits of the mixed-radix `W` (its low 96 bits fill `x`, `y`, `z`), then
        // `length`, 0…76 valid (the register's `fgw_capacity`) with 127 the truncation sentinel (`fgw_length_sentinel`).
        entry(
            "payload",
            packed("fgw_w", 0, 25),
            FieldType::UBits,
            Scale::Lin,
        )
        .range(Range::int(0, (1 << 25) - 1)),
        entry(
            "length",
            packed("fgw_w", 25, 7),
            FieldType::UBits,
            Scale::Lin,
        )
        .range(Range::int(0, FGW_CAPACITY.number() as i64))
        .sentinel(FGW_LENGTH_SENTINEL.number()),
    ]
}

/// The payload ledger's words and entries.
pub fn ledger() -> Ledger {
    Ledger {
        words: words(),
        entries: entries(),
    }
}

/// `SimStateFTLE`, or `SimStateBase` (the same without `r_sh` and `p_sh`), as payload §1 orders them.
fn simstate(ftle: bool) -> Struct {
    let m = |name, storage| Member { name, storage };
    let mut members = vec![m("r", Storage::Vec2x3), m("p", Storage::Vec2x3)];
    if ftle {
        members.extend([m("r_sh", Storage::Vec2x3), m("p_sh", Storage::Vec2x3)]);
    }
    for name in ["S", "theta", "mean_y", "C_ty", "E_0", "Lz_0"] {
        members.push(m(name, Storage::F32));
    }
    for name in ["packed_a", "packed_b", "times", "total_substeps"] {
        members.push(m(name, Storage::U32));
    }
    members.extend([
        m("closure_min", Storage::F32),
        m("closure_step", Storage::U16),
        m("_reserved", Storage::U16),
    ]);
    Struct {
        name: if ftle { "SimStateFTLE" } else { "SimStateBase" },
        align: 8,
        buffer: Some("SimState"),
        indexed: ftle,
        members,
    }
}

/// Struct members with no ledger entry or word of their own, which the struct check ([`crate::gen::rust::check`])
/// exempts: `free_group_word`, the word buffer's one `vec4<u32>`. Its `.w` is the word `fgw_w`, but its `x`, `y` and `z`
/// hold the low 96 bits of one mixed-radix integer spanning all four (payload §3), which no §3.8 location addresses.
pub const PENDING: &[&str] = &["free_group_word"];

/// The structs the Rust emitter writes: `SimState`'s two variants, the word buffer's element (one `vec4<u32>`, whose
/// `.w` is the word `fgw_w`, payload §3) and `ICDescriptor` (twelve f32s and 16 B of declared padding, 64 B: its f32
/// instantiation, R-86; its f32s widen with the `Real`, R-313).
pub fn structs() -> Vec<Struct> {
    let ic = [
        "m0",
        "m1",
        "m2",
        "q_mass",
        "rho_mag",
        "lambda_mag",
        "rho_ratio",
        "rho_angle",
        "K_0",
        "V_0",
        "virial_ratio",
        "r_min_pair_0",
    ];
    let mut ic: Vec<Member> = ic
        .map(|name| Member {
            name,
            storage: Storage::F32,
        })
        .to_vec();
    ic.push(Member {
        name: "_pad",
        storage: Storage::Pad(4),
    });
    let word = Member {
        name: "free_group_word",
        storage: Storage::U32x4,
    };
    vec![
        simstate(true),
        simstate(false),
        Struct {
            name: "FreeGroupWord",
            align: 16,
            buffer: Some("word"),
            indexed: false,
            members: vec![word],
        },
        Struct {
            name: "ICDescriptor",
            align: 4,
            buffer: None,
            indexed: true,
            members: ic,
        },
    ]
}

/// One stored buffer's binding in the fragment-side unpack layer (R-343).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The WGSL global the buffer is bound to.
    pub buffer: &'static str,
    /// The [`Struct::buffer`] whose elements it holds.
    pub holds: &'static str,
    /// The one generated function that reads it, by the sample index: the read side's `sample_read`, which loads one
    /// stored member or word component at a time, never the whole stored struct (R-343, R-378).
    pub reader: &'static str,
    /// The prefix of its generated constants, `<prefix>_GROUP` and `<prefix>_BINDING`.
    pub constant: &'static str,
    pub group: u32,
    pub binding: u32,
}

/// The one table of the stored buffers' bindings (R-343), the source of the numbers both emitters write, as the
/// constants `SIMSTATE_GROUP`, `SIMSTATE_BINDING`, `WORD_GROUP` and `WORD_BINDING` and as the WGSL `@group`/`@binding`
/// attributes: `SimStateFTLE` at group 1, binding 0, the word buffer at group 1, binding 1. Group 0 is the
/// assembler's per-frame uniforms. A binding number decides no stored bit's meaning, so the table is not hashed into
/// the schema version (R-343's applied note; dd_generation_root §3.8 "The hash").
pub const fn bindings() -> [Binding; 2] {
    [
        Binding {
            buffer: "simstate_buffer",
            holds: "SimState",
            reader: "sample_read",
            constant: "SIMSTATE",
            group: 1,
            binding: 0,
        },
        Binding {
            buffer: "word_buffer",
            holds: "word",
            reader: "sample_read",
            constant: "WORD",
            group: 1,
            binding: 1,
        },
    ]
}

/// Payload §3's frozen `inverse`, each symbol's code to its inverse's, the symbol codes `a = 0, A = 1, b = 2, B = 3`.
/// The table is ledger data written in function bodies, as the words' bits are: it is part of the binary format, not
/// a register constant (REQ-SYS-001).
pub const fn inverse() -> [u32; 4] {
    [1, 0, 3, 2]
}

/// Payload §3's frozen `cont_symbol`: `next = cont_symbol()[digit][prev]`. Each digit's map is a permutation of the
/// symbols that never gives `inverse(prev)`, and is its own inverse.
pub const fn cont_symbol() -> [[u32; 4]; 3] {
    [[0, 1, 2, 3], [2, 3, 0, 1], [3, 2, 1, 0]]
}

/// `continuation_index`'s value where no digit continues `prev`, at `next = inverse(prev)`: 3, "invalid", as
/// `dmin_pair`'s 3 is (R-307, payload §3).
pub const fn continuation_invalid() -> u32 {
    3
}

/// `predecessor_symbol`, derived from [`cont_symbol`] by inverting each digit's permutation:
/// `prev = predecessor_symbol()[digit][next]`, the `prev` with `cont_symbol()[digit][prev] == next` (payload §3). Each
/// map is an involution, so this equals `cont_symbol()`.
pub const fn predecessor_symbol() -> [[u32; 4]; 3] {
    let cont = cont_symbol();
    let mut out = [[0; 4]; 3];
    let mut e = 0;
    while e < 3 {
        let mut prev = 0;
        while prev < 4 {
            out[e][cont[e][prev] as usize] = prev as u32;
            prev += 1;
        }
        e += 1;
    }
    out
}

/// `continuation_index`, derived from [`cont_symbol`]: `continuation_index()[prev][next]` is the digit `e` with
/// `cont_symbol()[e][prev] == next`, and [`continuation_invalid`] in the four cells where no digit gives `next`, those
/// with `next = inverse(prev)` (R-307, payload §3).
pub const fn continuation_index() -> [[u32; 4]; 4] {
    let cont = cont_symbol();
    let mut out = [[continuation_invalid(); 4]; 4];
    let mut e = 0;
    while e < 3 {
        let mut prev = 0;
        while prev < 4 {
            out[prev][cont[e][prev] as usize] = e as u32;
            prev += 1;
        }
        e += 1;
    }
    out
}

/// The canonical quiet NaN's f32 bits, `0x7fc0_0000`: sign 0, exponent all ones, the quiet bit (bit 22) alone in the
/// significand (lowering Part 3a; R-72, R-79; REQ-RENDER-077). A tier-absent derived scalar reads exactly these bits at
/// unpack, as does an invalid read (`ftle`, R-254; `diffusion`, R-245), and the bitcast absence test compares against
/// them. Interface data written into the read side, as the continuation table is; not a register constant.
pub const fn canonical_qnan_bits() -> u32 {
    0x7fc0_0000
}

/// Payload §3's frozen symbol codes, each symbol at its code: `a = 0, A = 1, b = 2, B = 3` ("part of the binary
/// format"). `inverse` and the digit maps are over these codes; `last_symbol` stores one (payload §2).
pub const fn symbols() -> [&'static str; 4] {
    ["a", "A", "b", "B"]
}

/// Payload §2's `state` codes, each state at its code: 0 escape · 1 bounded · 2 collision · 3 running ·
/// 4 sim_failed · 5 decode_failed. Codes 6–7 are reserved and read as finished and untrusted.
pub const fn states() -> [&'static str; 6] {
    [
        "escape",
        "bounded",
        "collision",
        "running",
        "sim_failed",
        "decode_failed",
    ]
}

/// The pair-id map (R-22; payload §2): pair `k` names the side opposite body `k`, `pair_bodies()[k]` its two bodies,
/// pair 0 = (1, 2), pair 1 = (2, 0), pair 2 = (0, 1). `detail` under collision and `dmin_pair` use it.
pub const fn pair_bodies() -> [[u32; 2]; 3] {
    [[1, 2], [2, 0], [0, 1]]
}

/// `detail`'s meaning in each state it is meaningful in, each code's at its code (payload §2): escape a body id and
/// collision a pair id, `3` all three; sim_failed and decode_failed their failure enums.
pub const fn detail_meanings() -> [(&'static str, [&'static str; 4]); 4] {
    [
        (
            "escape",
            ["body 0", "body 1", "body 2", "all three: triple ejection"],
        ),
        (
            "collision",
            ["pair 0", "pair 1", "pair 2", "all three: triple collision"],
        ),
        (
            "sim_failed",
            [
                "NaN in state",
                "Inf/overflow in state",
                "non-finite derived quantity",
                "reserved",
            ],
        ),
        (
            "decode_failed",
            [
                "non-finite decode output",
                "degenerate configuration",
                "invalid mass construction",
                "other/reserved",
            ],
        ),
    ]
}

/// One member of generation-root §3.7's `QuadReduction`: its name, its §3.7 type as written there, and the §3.7
/// subsection that gives it. Ledger data, not a §3.8 entry, and not emitted: the struct, its members' §3.8 entries and
/// their placement are TASK-M5-01's (R-306, R-113).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionMember {
    pub name: &'static str,
    /// The type column of §3.7, verbatim; `None` where §3.7 gives the member no type.
    pub ty: Option<&'static str>,
    pub section: &'static str,
}

/// Generation-root §3.7's `QuadReduction` members, in its order (R-306). `spread_event` is the stored f16 agreement
/// value; `ensemble_outcome_agreement` is a retired name and no member (R-18). Not members: `id` (omitted, identity is
/// positional), the latch `running_max_divergence` (per footprint, R-99) and the conditional `spread_t_end` ("not yet
/// included"). §3.7's one row `` `escape_time_min` / `_max` ``, typed `f16 × 2`, is its two members, each f16.
/// `n_unresolved`, the count of the quad's unresolved footprints, latched ones included (the latch's verdict, R-142),
/// is u16 like `valid_sample_count` (R-315).
pub const QUAD_REDUCTION: &[ReductionMember] = &[
    reduction("level", "u8", "Identity"),
    reduction("class_histogram[N]", "u8 × N", "Outcome"),
    reduction("dominant_outcome", "packed", "Outcome"),
    reduction("outcome_impurity", "f16", "Outcome"),
    reduction("terminated_fraction", "f16", "Outcome"),
    reduction("spread_shape", "f16", "Ensemble spread"),
    reduction("spread_event", "f16", "Ensemble spread"),
    reduction("error_ratio", "f16", "Ensemble spread"),
    reduction("roundtrip_error", "f16", "Ensemble spread"),
    reduction("spread_winner", "2 bits", "Ensemble spread"),
    reduction("alpha_area", "f16", "Refinement"),
    reduction("alpha_energy", "f16", "Refinement"),
    reduction("worst_energy_drift", "f16", "Refinement"),
    reduction("running_mean_divergence", "f32", "Temporal accumulators"),
    reduction("first_divergence_t", "f32", "Temporal accumulators"),
    reduction("n_unresolved", "u16", "Temporal accumulators"),
    reduction("suspect_fraction", "f16", "Validity and diagnostics"),
    reduction("saturated_fraction", "f16", "Validity and diagnostics"),
    reduction("valid_sample_count", "u16", "Validity and diagnostics"),
    reduction("coherence", "f32", "Validity and diagnostics"),
    reduction("escape_time_min", "f16", "Validity and diagnostics"),
    reduction("escape_time_max", "f16", "Validity and diagnostics"),
    reduction("retrograde_fraction", "f16", "Validity and diagnostics"),
    reduction("mean_orbit_count_spread", "f16", "Validity and diagnostics"),
];

const fn reduction(name: &'static str, ty: &'static str, section: &'static str) -> ReductionMember {
    ReductionMember {
        name,
        ty: Some(ty),
        section,
    }
}
