//! The payload ledger (dd_simstate_payload §0–§1; dd_generation_root §3.1, §3.3a–§3.6): the hot `SimState` in its two
//! variants, the word buffer, `ICDescriptor`, the packed words of `SimState`, and §3.4's scalars with their
//! presentation metadata. Where the payload doc and generation-root §3 could drift, the payload doc is canonical
//! (R-70, R-86): §3.4's `delta_E_max_abs` and `delta_Lz_max_abs` are the payload's `dE_max` and `dLz_max`, and §3.8's
//! `shadow` is the payload's `r_sh` and `p_sh`.
//!
//! Where §3 gives a field no scale or range, the entry is `lin` over (−∞, ∞) (or its type's full range), as §3.8's
//! worked entries do for `n` and `ftle`; §3.6's log fields are > 0. Consumers are render, export and debug, as there.

use crate::constants::HORIZON_STEPS_MAX;
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

/// The packed words of `SimState` (payload §1; §3.1).
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
        descriptor("dmin_pair", 6, 2, Scale::Categorical(3), 2),
        descriptor("last_symbol", 8, 2, Scale::Categorical(4), 3),
        latch("d_min", packed("packed_a", 16, 16), Bound::Open(0.0)),
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

/// The structs the Rust emitter writes: `SimState`'s two variants, the word buffer's element (one `vec4<u32>`, whose
/// layout, §3.3, is not transcribed here) and `ICDescriptor` (64 B: twelve f32s and 16 B of declared padding, R-86).
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
