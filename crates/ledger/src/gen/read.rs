//! The read side (lowering Part 3a; payload §5, §6; render contract Part 2): one `SimState` type, fixed across tiers,
//! that the fragment and the host program against, emitted to both targets from the ledger the stored layouts come
//! from: Rust into `crates/kernel/src/payload/generated/read_side.rs` ([`rust`]), WGSL into
//! `crates/render/frag/generated/read_side.wgsl` ([`wgsl`]), which follows the fragment unpack layer at assembly and
//! reads its accessors and stored layouts (`cargo xtask lint wgsl` lints it so).
//!
//! Its members ([`members`]) are the stored members every tier's `SimState` variant holds, each packed word expanded
//! into its fields, then the word, then the quantities derived at read (payload §5): `ftle`, `ftle_valid`,
//! `diffusion`, `diffusion_slope_valid`, `total_substeps_log2`, the two time fractions, `orbit_count`, `retrograde`,
//! the four state predicates, `ensemble_spread` and the current drifts `energy_drift` and `Lz_drift`. None of the
//! derived ones is stored (R-79). The Benettin shadow,
//! tier-gated state, is not a read-side member: the read side keeps the cheap derived result, never resurrected state
//! (lowering Part 3a).
//!
//! In Rust a stored variant unpacks into it through `sim_state_from_ftle` or `sim_state_from_base`, each taking the
//! stored struct by reference, so each member is read on its own. In WGSL the fragment reads sample `i` through
//! `sample_read(i, …)` ([`wgsl_for`]), which loads only the stored members the fields it fills need, one at a time
//! (`simstate_buffer[i].packed_a`), never the whole stored struct, and of the word only the components they need
//! (R-378). The assembler generates it at its tier ([`Tier`]) for the fields its stain reads ([`assemble`]), so a stain
//! that reads one field loads only that field's words, on every backend; the checked-in read side is the full tier's,
//! every field filled. A tier-absent derived scalar reads the canonical quiet NaN, `CANONICAL_QNAN_BITS`
//! ([`crate::payload::canonical_qnan_bits`]): `ftle` from `SimStateBase`, and `ensemble_spread` when `has_ensemble`
//! is false (E = 0, R-145). An unbound word buffer reads `FGW_UNBOUND` ([`unbound_word`]). Both values are lowering
//! Part 3a's definition (R-72; REQ-RENDER-077). An invalid read writes the same NaN: `ftle` whenever `ftle_valid` is
//! false (R-254), `diffusion` for `n < 2` (R-245). The NaN is always written from its bits, never computed, and no
//! validity logic tests a value for NaN: validity is `ftle_valid`, `diffusion_slope_valid` and the state predicates
//! (lowering Part 3a; R-255).
//!
//! The derived accessors carry payload §6's names where it gives one (`ftle_valid`, `diffusion_slope_valid`,
//! `total_substeps_log2`, `tm_t_end_fraction`, `tm_t_dmin_fraction`, the `sd_is_*` predicates; the last five are the
//! fragment unpack layer's and the Rust emitter's, [`super::rust`], already), and payload §5's otherwise (`ftle`,
//! `diffusion_slope`, `orbit_count`, `retrograde`); the drifts carry dd_generation_root §3.8's entry names.
//!
//! The current drifts (payload §5; dd_generation_root §3.8, R-246) are `energy_drift = H(r, p) − E_0` and
//! `Lz_drift = L_z(r, p) − Lz_0`, with integrator dd §3.5's forms in CoM-frame particle coordinates and `G = 1`:
//! `H = Σᵢ ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖` (decoder dd §3.6's `K₀ + V₀`) and `L_z = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})`.
//! The masses `m0 m1 m2` are the `ICDescriptor`'s (dd_generation_root §3.6, §3.8), not the stored `SimState`'s, so each
//! read takes them as an argument, `masses` (the fragment's `ctx.ic`, render contract Part 1). Both are f32, the live
//! f32 values, not the f16 latches (payload §1, §5).

use std::fmt::Write as _;
use std::path::PathBuf;

use crate::gen::{rust as stored, wgsl as frag, Generated};
use crate::schema::{Entry, FieldType, Location, Storage, Struct, Word};

/// Where the Rust read side is written, relative to the workspace root.
pub const RUST_PATH: &str = "crates/kernel/src/payload/generated/read_side.rs";

/// Where the WGSL read side is written, relative to the workspace root: after the unpack layer,
/// [`super::wgsl::PATH`], whose accessors and stored layouts it reads.
pub const WGSL_PATH: &str = "crates/render/frag/generated/read_side.wgsl";

/// How a read-side member is filled from a stored variant `s`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fill {
    /// A stored member of every variant, copied: `s.<name>`.
    Copy,
    /// A u16 member, widened to u32: `u32::from(s.<name>)` in Rust, `<name>(s.<pair>)` in WGSL, where `pair` is the
    /// one u32 member WGSL stores it in (R-343).
    Widen { pair: String },
    /// A packed field, through its payload §6 accessor: `<accessor>(s.<word>)`.
    Packed {
        accessor: String,
        word: &'static str,
    },
    /// The sample's word, or [`unbound_word`] when the word buffer is unbound (`has_word` false).
    Word,
    /// Computed at read (payload §5), never stored.
    Derived,
}

/// One member of the read-side `SimState`: its name, its Rust and WGSL types, and how it is filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadMember {
    pub name: String,
    pub rust: &'static str,
    pub wgsl: &'static str,
    pub fill: Fill,
}

/// The derived members, in order, each with its type as `(name, Rust, WGSL)`.
pub const DERIVED: [(&str, &str, &str); 16] = [
    ("ftle", "f32", "f32"),
    ("ftle_valid", "bool", "bool"),
    ("diffusion", "f32", "f32"),
    ("diffusion_slope_valid", "bool", "bool"),
    ("total_substeps_log2", "u32", "u32"),
    ("t_end_fraction", "f32", "f32"),
    ("t_dmin_fraction", "f32", "f32"),
    ("orbit_count", "u32", "u32"),
    ("retrograde", "bool", "bool"),
    ("is_resolved_outcome", "bool", "bool"),
    ("is_running", "bool", "bool"),
    ("is_failed", "bool", "bool"),
    ("is_finished", "bool", "bool"),
    ("ensemble_spread", "f32", "f32"),
    ("energy_drift", "f32", "f32"),
    ("Lz_drift", "f32", "f32"),
];

/// The `SimState` variants the stored buffer holds, one per tier (payload §1).
fn variants(structs: &[Struct]) -> Vec<&Struct> {
    structs
        .iter()
        .filter(|s| s.buffer == Some("SimState"))
        .collect()
}

/// The read-side `SimState`'s members (lowering Part 3a): each member every stored variant holds, in the first
/// variant's order, a packed word expanded into its fields in bit order and a member whose name starts with `_`
/// skipped; then `word`; then [`DERIVED`].
pub fn members(words: &[Word], entries: &[Entry]) -> Vec<ReadMember> {
    let structs = crate::payload::structs();
    let variants = variants(&structs);
    let mut out = Vec::new();
    let Some((first, rest)) = variants.split_first() else {
        return out;
    };
    let everywhere = |name: &str| {
        rest.iter()
            .all(|s| s.members.iter().any(|m| m.name == name))
    };
    for m in first.members.iter().filter(|m| everywhere(m.name)) {
        if m.name.starts_with('_') {
            continue;
        }
        let member = |rust, wgsl, fill| ReadMember {
            name: m.name.to_owned(),
            rust,
            wgsl,
            fill,
        };
        match m.storage {
            Storage::Vec2x3 => out.push(member("[[f32; 2]; 3]", "array<vec2<f32>, 3>", Fill::Copy)),
            Storage::F32 => out.push(member("f32", "f32", Fill::Copy)),
            Storage::U32 if words.iter().any(|w| w.name == m.name) => {
                out.extend(packed(m.name, entries));
            }
            Storage::U32 => out.push(member("u32", "u32", Fill::Copy)),
            Storage::U16 => {
                let pair = frag::members(first)
                    .into_iter()
                    .find(|w| w.stores.contains(&m.name))
                    .map_or_else(|| format!("{}_has_no_wgsl_member", m.name), |w| w.name);
                out.push(member("u32", "u32", Fill::Widen { pair }));
            }
            Storage::U32x4 | Storage::Pad(_) => {}
        }
    }
    out.push(ReadMember {
        name: "word".to_owned(),
        rust: "[u32; 4]",
        wgsl: "vec4<u32>",
        fill: Fill::Word,
    });
    out.extend(DERIVED.iter().map(|&(name, rust, wgsl)| ReadMember {
        name: name.to_owned(),
        rust,
        wgsl,
        fill: Fill::Derived,
    }));
    out
}

/// The fields of the packed word `word`, in bit order, each read through its payload §6 accessor: a 1-bit field a
/// `bool`, an `f16-pair` an `f32`, any other a `u32`.
fn packed(word: &'static str, entries: &[Entry]) -> Vec<ReadMember> {
    let mut fields: Vec<(&Entry, u32, u32)> = entries
        .iter()
        .filter_map(|e| match e.location {
            Location::Packed {
                word: w,
                offset,
                width,
            } if w == word => Some((e, offset, width)),
            _ => None,
        })
        .collect();
    fields.sort_by_key(|&(_, offset, _)| offset);
    fields
        .into_iter()
        .filter_map(|(e, _, width)| {
            let prefix = stored::prefix(word, e)?;
            let ty = match (&e.ty, width) {
                (FieldType::F16Pair, _) => "f32",
                (_, 1) => "bool",
                _ => "u32",
            };
            Some(ReadMember {
                name: e.name.to_owned(),
                rust: ty,
                wgsl: ty,
                fill: Fill::Packed {
                    accessor: format!("{prefix}_{}", e.name),
                    word,
                },
            })
        })
        .collect()
}

/// The word an unbound word buffer reads (lowering Part 3a; R-72; REQ-RENDER-077): payload zero and `length_raw` the
/// truncation sentinel, from the word buffer `.w`'s `length` entry; every word-derived validity predicate reads it as
/// invalid. The line naming what the entry lacks otherwise ([`stored::fgw_length`]).
pub fn unbound_word(entries: &[Entry]) -> Result<[u32; 4], String> {
    let (offset, _, _, sentinel) = stored::fgw_length(entries)?;
    Ok([0, 0, 0, sentinel << offset])
}

/// A derived member's expression in Rust, `ftle` as the variant with the shadow (`shadow`) or without reads it.
fn rust_derived(name: &str, shadow: bool) -> String {
    match name {
        "ftle" if shadow => "ftle(s.S, delta, params.delta_0, n, params.dt_macro, ftle_ok)".into(),
        "ftle" => "canonical_nan()".into(),
        "ftle_valid" => "ftle_ok".into(),
        "diffusion" => "diffusion_slope(s.C_ty, n, params.dt_macro)".into(),
        "diffusion_slope_valid" => "diffusion_slope_valid(n)".into(),
        "total_substeps_log2" => "total_substeps_log2(s.total_substeps)".into(),
        "t_end_fraction" => "tm_t_end_fraction(s.times, params.horizon_steps)".into(),
        "t_dmin_fraction" => "tm_t_dmin_fraction(s.times, params.horizon_steps)".into(),
        "orbit_count" | "retrograde" => format!("{name}(s.theta)"),
        "energy_drift" => "energy_drift(s.r, s.p, masses, s.E_0)".into(),
        "Lz_drift" => "Lz_drift(s.r, s.p, s.Lz_0)".into(),
        "ensemble_spread" => {
            "if has_ensemble {\n    ensemble_spread\n} else {\n    canonical_nan()\n}".into()
        }
        predicate => format!("sd_{predicate}(s.packed_a)"),
    }
}

/// `text` with every line after the first indented by `pad`.
fn indent(text: &str, pad: &str) -> String {
    text.replace('\n', &format!("\n{pad}"))
}

/// The Rust read side: the sentinels, `ReadParams`, the read-side `SimState`, the derived accessors and the two
/// unpacks, `no_std` and rustfmt's layout.
pub fn rust(words: &[Word], entries: &[Entry]) -> Generated {
    let qnan = crate::payload::canonical_qnan_bits();
    let unbound = match unbound_word(entries) {
        Ok(w) => format!(
            "[{}, {}, {}, 0x{:04x}_{:04x}]",
            w[0],
            w[1],
            w[2],
            w[3] >> 16,
            w[3] & 0xffff
        ),
        Err(why) => format!("[0; 4];\ncompile_error!({why:?})"),
    };
    let members = members(words, entries);
    let mut out = format!(
        r#"//! Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.
//! The read side (lowering Part 3a; payload §5, §6): one `SimState`, fixed across tiers, each stored variant's
//! unpack into it, and the quantities derived at read, never stored (R-79).

// The float functions are called through `Float`, as `Float::ln(x)`: the rust-gpu toolchain's `core` has no inherent
// `f32` math, and a newer one's makes a method call skip the trait.
use spirv_std::num_traits::Float;

use super::*;

/// The canonical quiet NaN's f32 bits: sign 0, exponent all ones, the quiet bit alone in the significand (lowering
/// Part 3a; R-72, R-79). A tier-absent derived scalar and an invalid read hold exactly these bits.
pub const CANONICAL_QNAN_BITS: u32 = 0x{:04x}_{:04x};

/// The word an unbound word buffer reads: payload zero, `length_raw` the truncation sentinel, so every word-derived
/// validity predicate reads it as invalid (lowering Part 3a; R-72).
pub const FGW_UNBOUND: [u32; 4] = {unbound};

/// The canonical quiet NaN, written from its bits, never computed (lowering Part 3a).
#[inline]
pub fn canonical_nan() -> f32 {{
    f32::from_bits(CANONICAL_QNAN_BITS)
}}

/// The sim-key values the read side derives from (lowering Part 3; payload §4, §5): the macro-step `dt_macro`, the
/// Benettin shadow's initial separation `delta_0`, the renormalisation interval `n_renorm` and `horizon_steps`. The
/// masses are the sample's own `ICDescriptor`'s, not the sim key's, and each read takes them apart (`masses`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReadParams {{
    pub dt_macro: f32,
    pub delta_0: f32,
    pub n_renorm: u32,
    pub horizon_steps: u32,
}}

/// The read-side `SimState`, the same at every tier (lowering Part 3a): the stored members every variant holds, the
/// packed words' fields, the word, and the quantities derived at read (payload §5).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SimState {{
"#,
        qnan >> 16,
        qnan & 0xffff,
    );
    for m in &members {
        let _ = writeln!(out, "    pub {}: {},", m.name, m.rust);
    }
    out.push_str("}\n");
    out.push_str(RUST_HELPERS);
    for variant in variants(&crate::payload::structs()) {
        out.push_str(&rust_unpack(variant, &members));
    }
    Generated {
        path: PathBuf::from(RUST_PATH),
        contents: out,
    }
}

/// Whether `s` holds the Benettin shadow (`r_sh`, payload §1), so its read computes `ftle`.
fn has_shadow(s: &Struct) -> bool {
    s.members.iter().any(|m| m.name == "r_sh")
}

/// The function that unpacks the variant `s` into the read-side `SimState`: `sim_state_from_ftle` for the variant
/// with the shadow, `sim_state_from_base` for the one without.
pub fn unpack_name(s: &Struct) -> &'static str {
    if has_shadow(s) {
        "sim_state_from_ftle"
    } else {
        "sim_state_from_base"
    }
}

/// The Rust unpack of `s` into the read-side `SimState`.
fn rust_unpack(s: &Struct, members: &[ReadMember]) -> String {
    let shadow = has_shadow(s);
    let doc = if shadow {
        "/// `SimStateFTLE` read: `ftle` finalised from the shadow (payload §5), NaN whenever `ftle_valid` is false (R-254)."
    } else {
        "/// `SimStateBase` read: FTLE is baked out, so `ftle` reads the canonical quiet NaN and `ftle_valid` is false\n\
         /// (lowering Part 3a)."
    };
    let mut out = format!(
        "\n{doc}\n/// An unbound word buffer (`has_word` false) reads `FGW_UNBOUND`; E = 0 (`has_ensemble` false) reads\n\
         /// `ensemble_spread` as the canonical quiet NaN (R-145). `masses` are the sample's `ICDescriptor` `m0 m1 m2`,\n\
         /// which `energy_drift` reads (dd_generation_root §3.8).\n\
         #[inline]\n\
         pub fn {}(\n    s: &{},\n    word: [u32; 4],\n    has_word: bool,\n    ensemble_spread: f32,\n    has_ensemble: bool,\n    \
         masses: [f32; 3],\n    params: &ReadParams,\n) -> SimState {{\n    let n = tm_t_end_step(s.times);\n    \
         let ftle_ok = ftle_valid(s.packed_a, {shadow}, n, completed_renorms(n, params.n_renorm));\n",
        unpack_name(s),
        s.name,
    );
    if shadow {
        out.push_str("    let delta = benettin_delta(s.r, s.p, s.r_sh, s.p_sh);\n");
    }
    out.push_str("    SimState {\n");
    for m in members {
        let value = match &m.fill {
            Fill::Copy => format!("s.{}", m.name),
            Fill::Widen { .. } => format!("u32::from(s.{})", m.name),
            Fill::Packed { accessor, word } => format!("{accessor}(s.{word})"),
            Fill::Word => "if has_word { word } else { FGW_UNBOUND }".into(),
            Fill::Derived => rust_derived(&m.name, shadow),
        };
        let value = indent(&value, "        ");
        if value == m.name {
            let _ = writeln!(out, "        {},", m.name);
        } else {
            let _ = writeln!(out, "        {}: {value},", m.name);
        }
    }
    out.push_str("    }\n}\n");
    out
}

/// The Rust derived accessors (payload §4–§6).
const RUST_HELPERS: &str = r#"
/// The renormalisations completed by step `n`, `n / n_renorm` under the uniform schedule (payload §5); none when
/// `n_renorm` is 0.
#[inline]
pub fn completed_renorms(n: u32, n_renorm: u32) -> u32 {
    n.checked_div(n_renorm).unwrap_or(0)
}

/// Whether `ftle` is usable: the FTLE tier on, the sample not failed, at least one step and at least one completed
/// renormalisation (payload §6). Derived, never stored; `ftle` reads NaN whenever it is false (R-254).
#[inline]
pub fn ftle_valid(state: u32, ftle_tier_on: bool, n: u32, benettin_renorms: u32) -> bool {
    ftle_tier_on && !sd_is_failed(state) && n > 0 && benettin_renorms > 0
}

/// The shadow's current separation `δ = ‖x' − x‖`, the Euclidean norm of the shadow's offset over the twelve
/// phase-space components `(r, p)` (payload §5). Constant indices only: no runtime array index (GPU determinism note
/// § "The discipline", rule 5).
#[inline]
pub fn benettin_delta(
    r: [[f32; 2]; 3],
    p: [[f32; 2]; 3],
    r_sh: [[f32; 2]; 3],
    p_sh: [[f32; 2]; 3],
) -> f32 {
    let d = |a: [f32; 2], b: [f32; 2]| {
        let (x, y) = (b[0] - a[0], b[1] - a[1]);
        x * x + y * y
    };
    let r2 = d(r[0], r_sh[0]) + d(r[1], r_sh[1]) + d(r[2], r_sh[2]);
    let p2 = d(p[0], p_sh[0]) + d(p[1], p_sh[1]) + d(p[2], p_sh[2]);
    Float::sqrt(r2 + p2)
}

/// The FTLE with the partial renormalisation interval finalised: `S_final = S + ln(δ/δ₀)`, divided by the full
/// elapsed `n · dt_macro`, never plain `S / t` (payload §5); the canonical quiet NaN when `valid` is false (R-254).
#[inline]
pub fn ftle(s_sum: f32, delta: f32, delta_0: f32, n: u32, dt_macro: f32, valid: bool) -> f32 {
    if valid {
        (s_sum + Float::ln(delta / delta_0)) / (n as f32 * dt_macro)
    } else {
        canonical_nan()
    }
}

/// Whether the diffusion fit is valid: `n ≥ 2`, as `C_tt(n)` is 0 below (payload §4, §6; R-245).
#[inline]
pub fn diffusion_slope_valid(n: u32) -> bool {
    n >= 2
}

/// `C_tt(n) = h²·n(n²−1)/12`, the time-only co-moment of the uniform samples `h, 2h, …, nh`, derived closed-form,
/// never stored (payload §4). `n − 1` and `n + 1` are exact in f32 for every u16 `n`.
#[inline]
pub fn diffusion_c_tt(n: u32, h: f32) -> f32 {
    let m = n as f32;
    h * h * m * (m - 1.0) * (m + 1.0) / 12.0
}

/// The diffusion slope `C_ty / C_tt(n)`, `n` the sample's own completed-step count (`t_end_step`, latched at
/// termination), never the playhead; the canonical quiet NaN for `n < 2`, with no division by zero (payload §4;
/// R-245).
#[inline]
pub fn diffusion_slope(c_ty: f32, n: u32, h: f32) -> f32 {
    if diffusion_slope_valid(n) {
        c_ty / diffusion_c_tt(n, h)
    } else {
        canonical_nan()
    }
}

/// The winding count `⌊|θ̃| / 2π⌋` (payload §5), clamped in f32 to the largest f32 below 2³² before the cast (GPU
/// determinism note § "The discipline", rule 3).
#[inline]
pub fn orbit_count(theta: f32) -> u32 {
    let turns = Float::floor(Float::abs(theta) / core::f32::consts::TAU);
    Float::min(turns, 4_294_967_040.0) as u32
}

/// The winding sense: `θ̃ < 0` (payload §5).
#[inline]
pub fn retrograde(theta: f32) -> bool {
    theta < 0.0
}

/// The Hamiltonian of the planar three-body problem in CoM-frame particle coordinates, `G = 1`: `H(r, p) = K + V`,
/// `K = Σᵢ ‖pᵢ‖²/2mᵢ`, `V = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖` (integrator dd §3.5; decoder dd §3.6), the masses `m0 m1 m2` the
/// sample's `ICDescriptor`'s (dd_generation_root §3.6). Constant indices only.
#[inline]
pub fn hamiltonian(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3]) -> f32 {
    let kinetic = |q: [f32; 2], mass: f32| (q[0] * q[0] + q[1] * q[1]) / (2.0 * mass);
    let pair = |a: [f32; 2], b: [f32; 2], ma: f32, mb: f32| {
        let (x, y) = (a[0] - b[0], a[1] - b[1]);
        ma * mb / Float::sqrt(x * x + y * y)
    };
    let k = kinetic(p[0], m[0]) + kinetic(p[1], m[1]) + kinetic(p[2], m[2]);
    let v =
        pair(r[0], r[1], m[0], m[1]) + pair(r[0], r[2], m[0], m[2]) + pair(r[1], r[2], m[1], m[2]);
    k - v
}

/// The angular momentum `L_z(r, p) = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})` (integrator dd §3.5).
#[inline]
pub fn angular_momentum_z(r: [[f32; 2]; 3], p: [[f32; 2]; 3]) -> f32 {
    let cross = |q: [f32; 2], m: [f32; 2]| q[0] * m[1] - q[1] * m[0];
    cross(r[0], p[0]) + cross(r[1], p[1]) + cross(r[2], p[2])
}

/// The current energy drift `ΔE = H(r, p) − E_0`, in live f32 (payload §5; dd_generation_root §3.8, R-246).
#[inline]
pub fn energy_drift(r: [[f32; 2]; 3], p: [[f32; 2]; 3], m: [f32; 3], e_0: f32) -> f32 {
    hamiltonian(r, p, m) - e_0
}

/// The current angular-momentum drift `ΔLz = L_z(r, p) − Lz_0`, in live f32 (payload §5; dd_generation_root §3.8).
#[inline]
pub fn Lz_drift(r: [[f32; 2]; 3], p: [[f32; 2]; 3], lz_0: f32) -> f32 {
    angular_momentum_z(r, p) - lz_0
}
"#;

/// A tier as the read side sees it (lowering Part 3a; payload §1): whether the stored variant is `SimStateFTLE`, with
/// the Benettin shadow, or `SimStateBase`, and whether the word buffer is bound. The assembler bakes the same two as
/// its `has_ftle` and `has_word` consts; the checked-in layer and read side are the full tier, [`Tier::FULL`] (R-343).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tier {
    pub has_ftle: bool,
    pub has_word: bool,
}

impl Tier {
    /// The full tier, the one the checked-in files declare (R-343).
    pub const FULL: Tier = Tier {
        has_ftle: true,
        has_word: true,
    };

    /// Every tier.
    pub const ALL: [Tier; 4] = [
        Tier::FULL,
        Tier {
            has_ftle: true,
            has_word: false,
        },
        Tier {
            has_ftle: false,
            has_word: true,
        },
        Tier {
            has_ftle: false,
            has_word: false,
        },
    ];
}

/// The word's components, in order, as a field request names them: `word.x` … `word.w` (R-378).
pub const WORD_COMPONENTS: [&str; 4] = ["x", "y", "z", "w"];

/// Every field [`wgsl_for`] can fill: each read-side member by its name, then each of the word's components,
/// `word.x` … `word.w`, for a stain that reads only part of the word (R-378).
pub fn fields(words: &[Word], entries: &[Entry]) -> Vec<String> {
    let mut out: Vec<String> = members(words, entries)
        .into_iter()
        .map(|m| m.name)
        .collect();
    out.extend(WORD_COMPONENTS.iter().map(|c| format!("word.{c}")));
    out
}

/// What filling a field needs first (R-378): the stored members it loads, the word's components it loads, and the
/// shared values derived from them, `n` (from `times`), `ftle_ok` (from `packed_a` and `n`) and `delta` (from the
/// state and the shadow).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Needs {
    stored: Vec<String>,
    word: [bool; 4],
    n: bool,
    ftle_ok: bool,
    delta: bool,
}

impl Needs {
    fn load(&mut self, member: &str) {
        if !self.stored.iter().any(|m| m == member) {
            self.stored.push(member.to_owned());
        }
    }

    /// `n`, the sample's own completed-step count, `t_end_step`.
    fn n(&mut self) {
        self.n = true;
        self.load("times");
    }

    /// `ftle_ok`, `ftle_valid` at the FTLE tier.
    fn ftle_ok(&mut self) {
        self.ftle_ok = true;
        self.load("packed_a");
        self.n();
    }

    /// `delta`, the shadow's separation.
    fn delta(&mut self) {
        self.delta = true;
        for m in ["r", "p", "r_sh", "p_sh"] {
            self.load(m);
        }
    }
}

/// A derived member's WGSL value at `tier`, read from the loaded members (`s_<member>`) and the shared values, and
/// what it needs. A tier-absent `ftle` reads the canonical quiet NaN and is never valid, with nothing loaded.
fn wgsl_derived(name: &str, tier: Tier, needs: &mut Needs) -> String {
    match name {
        "ftle" if tier.has_ftle => {
            needs.load("S");
            needs.ftle_ok();
            needs.delta();
            "ftle(s_S, delta, params.delta_0, n, params.dt_macro, ftle_ok)".into()
        }
        "ftle" => "canonical_nan()".into(),
        "ftle_valid" if tier.has_ftle => {
            needs.ftle_ok();
            "ftle_ok".into()
        }
        "ftle_valid" => "false".into(),
        "diffusion" => {
            needs.load("C_ty");
            needs.n();
            "diffusion_slope(s_C_ty, n, params.dt_macro)".into()
        }
        "diffusion_slope_valid" => {
            needs.n();
            "diffusion_slope_valid(n)".into()
        }
        "total_substeps_log2" => {
            needs.load("total_substeps");
            "total_substeps_log2(s_total_substeps)".into()
        }
        "t_end_fraction" | "t_dmin_fraction" => {
            needs.load("times");
            format!("tm_{name}(s_times, params.horizon_steps)")
        }
        "orbit_count" | "retrograde" => {
            needs.load("theta");
            format!("{name}(s_theta)")
        }
        "energy_drift" => {
            for m in ["r", "p", "E_0"] {
                needs.load(m);
            }
            "energy_drift(s_r, s_p, masses, s_E_0)".into()
        }
        "Lz_drift" => {
            for m in ["r", "p", "Lz_0"] {
                needs.load(m);
            }
            "Lz_drift(s_r, s_p, s_Lz_0)".into()
        }
        "ensemble_spread" => "select(canonical_nan(), ensemble_spread, has_ensemble)".into(),
        predicate => {
            needs.load("packed_a");
            format!("sd_{predicate}(s_packed_a)")
        }
    }
}

/// One requested field: the read-side member, and the word's component when only that is asked for.
type Request<'a> = (&'a ReadMember, Option<usize>);

/// `fields` resolved against `members`, in member order then component order, each once, a component dropped when the
/// whole word is asked for; or the first name that is no field ([`fields`]).
fn requests<'a>(members: &'a [ReadMember], fields: &[&str]) -> Result<Vec<Request<'a>>, String> {
    let mut out: Vec<(usize, Option<usize>)> = Vec::new();
    for &f in fields {
        let (name, component) = match f.split_once('.') {
            Some((name, c)) => (
                name,
                Some(
                    WORD_COMPONENTS
                        .iter()
                        .position(|w| *w == c)
                        .ok_or_else(|| format!("`{f}` is no read-side field"))?,
                ),
            ),
            None => (f, None),
        };
        let at = members
            .iter()
            .position(|m| m.name == name && (component.is_none() || m.fill == Fill::Word))
            .ok_or_else(|| format!("`{f}` is no read-side field"))?;
        out.push((at, component));
    }
    out.sort_unstable();
    out.dedup();
    let whole: Vec<usize> = out
        .iter()
        .filter(|(_, c)| c.is_none())
        .map(|&(at, _)| at)
        .collect();
    out.retain(|&(at, c)| c.is_none() || !whole.contains(&at));
    Ok(out.into_iter().map(|(at, c)| (&members[at], c)).collect())
}

/// `sample_read(i, …)` at `tier`, filling the `fields` a stain reads (R-378): it loads each stored member those fields
/// need once, `simstate_buffer[i].<member>`, never the whole stored struct, and of the word only the components they
/// need, the whole `word_buffer[i]` only when all four are; then it fills those fields of a zeroed `SimState`. A field
/// not asked for stays zero: a plausible value beside no validity signal (`ftle = 0.0`, `d_min = 0.0`,
/// `step_count = 0`), the misreading lowering Part 3a and R-254 exist to prevent. It is not made NaN here: R-79 and
/// R-254 give NaN to a field that is tier-absent or invalid for the sample, which an unrequested field isn't, and the
/// integer, bool and word members have no NaN, so it would close the hazard only for some fields. The closing guard is
/// the assembler's (TASK-M1-04): it derives the stain's field set from its IR and refuses a stain that reads a field
/// its set misses, so no assembled stain ever reads an unfilled field. An unbound word buffer (`has_word` false)
/// reads `FGW_UNBOUND`, and E = 0 (`has_ensemble` false) reads `ensemble_spread` as the canonical quiet NaN (lowering
/// Part 3a; R-145, R-254). `masses` are the sample's `ICDescriptor` `m0 m1 m2`, which `energy_drift` reads.
fn sample_read(members: &[ReadMember], tier: Tier, fields: &[&str]) -> Result<String, String> {
    let requests = requests(members, fields)?;
    let mut needs = Needs::default();
    let mut fills = Vec::new();
    for &(m, component) in &requests {
        let target = component.map_or_else(
            || m.name.clone(),
            |c| format!("{}.{}", m.name, WORD_COMPONENTS[c]),
        );
        let value = match &m.fill {
            Fill::Copy => {
                needs.load(&m.name);
                format!("s_{}", m.name)
            }
            Fill::Widen { pair } => {
                needs.load(pair);
                format!("{}(s_{pair})", m.name)
            }
            Fill::Packed { accessor, word } => {
                needs.load(word);
                format!("{accessor}(s_{word})")
            }
            Fill::Word => {
                if tier.has_word {
                    match component {
                        Some(c) => needs.word[c] = true,
                        None => needs.word = [true; 4],
                    }
                }
                String::new()
            }
            Fill::Derived => wgsl_derived(&m.name, tier, &mut needs),
        };
        fills.push((target, component, m.fill == Fill::Word, value));
    }
    let whole_word = needs.word.iter().all(|&w| w);
    let structs = crate::payload::structs();
    let variant = variants(&structs)
        .into_iter()
        .find(|s| has_shadow(s) == tier.has_ftle)
        .ok_or("no stored variant for the tier")?;
    let order: Vec<String> = frag::members(variant).into_iter().map(|w| w.name).collect();
    let mut stored = needs.stored.clone();
    stored.sort_by_key(|m| order.iter().position(|o| o == m));
    let names: Vec<String> = fills.iter().map(|(t, ..)| format!("`{t}`")).collect();
    let named = if requests.len() == members.len() && requests.iter().all(|(_, c)| c.is_none()) {
        "every field".to_owned()
    } else if names.is_empty() {
        "no field".to_owned()
    } else {
        format!("only {}", names.join(", "))
    };
    let mut out = format!(
        "\n// Sample `i` read into the read-side `SimState` at this tier, {named} filled: each stored member a field\n\
         // needs loaded alone, `simstate_buffer[i].<member>`, never the whole stored struct, and of the word only the\n\
         // components a field needs (R-378). A field not filled stays zero and is not read. An unbound word buffer\n\
         // reads `FGW_UNBOUND`; E = 0 reads `ensemble_spread` as the canonical quiet NaN (lowering Part 3a; R-145).\n\
         // `masses` are the sample's `ICDescriptor` `m0 m1 m2` (`ctx.ic`), which `energy_drift` reads.\n\
         fn sample_read(i: u32, ensemble_spread: f32, has_ensemble: bool, masses: vec3<f32>, params: ReadParams) -> SimState {{\n"
    );
    for m in &stored {
        let _ = writeln!(out, "    let s_{m} = simstate_buffer[i].{m};");
    }
    if whole_word {
        out.push_str("    let w = word_buffer[i];\n");
    } else {
        for (c, name) in WORD_COMPONENTS.iter().enumerate() {
            if needs.word[c] {
                let _ = writeln!(out, "    let w_{name} = word_buffer[i].{name};");
            }
        }
    }
    if needs.n {
        out.push_str("    let n = tm_t_end_step(s_times);\n");
    }
    if needs.ftle_ok {
        out.push_str(
            "    let ftle_ok = ftle_valid(s_packed_a, true, n, completed_renorms(n, params.n_renorm));\n",
        );
    }
    if needs.delta {
        out.push_str("    let delta = benettin_delta(s_r, s_p, s_r_sh, s_p_sh);\n");
    }
    out.push_str("    var out: SimState;\n");
    for (target, component, word, value) in fills {
        let value = match (word, tier.has_word, component) {
            (false, ..) => value,
            (true, false, None) => "FGW_UNBOUND".to_owned(),
            (true, false, Some(c)) => format!("FGW_UNBOUND.{}", WORD_COMPONENTS[c]),
            (true, true, None) => "w".to_owned(),
            (true, true, Some(c)) if whole_word => format!("w.{}", WORD_COMPONENTS[c]),
            (true, true, Some(c)) => format!("w_{}", WORD_COMPONENTS[c]),
        };
        let _ = writeln!(out, "    out.{target} = {value};");
    }
    out.push_str("    return out;\n}\n");
    Ok(out)
}

/// The WGSL read side, which follows the fragment unpack layer: the sentinels, `ReadParams`, the read-side `SimState`,
/// the derived accessors, and `sample_read` at the full tier filling every field ([`wgsl_for`]). The assembler
/// regenerates it for its tier and for the fields its stain reads, so the stain loads only those fields' words
/// (R-378); `has_ensemble` is an argument, the assembler's uniform (lowering Part 3a; R-145).
pub fn wgsl(words: &[Word], entries: &[Entry]) -> Generated {
    let all = fields(words, entries);
    let members = members(words, entries);
    let every: Vec<&str> = all.iter().take(members.len()).map(String::as_str).collect();
    let contents = wgsl_for(words, entries, Tier::FULL, &every)
        .unwrap_or_else(|why| format!("const_assert false; // {why}\n"));
    Generated {
        path: PathBuf::from(WGSL_PATH),
        contents,
    }
}

/// The WGSL read side at `tier`, its `sample_read` filling `fields` ([`fields`]'s names), or the first name that is no
/// field (R-378).
pub fn wgsl_for(
    words: &[Word],
    entries: &[Entry],
    tier: Tier,
    fields: &[&str],
) -> Result<String, String> {
    let qnan = crate::payload::canonical_qnan_bits();
    let unbound = match unbound_word(entries) {
        Ok(w) => format!(
            "vec4<u32>({}u, {}u, {}u, {:#010x}u)",
            w[0], w[1], w[2], w[3]
        ),
        Err(why) => format!("vec4<u32>(0u);\nconst_assert false; // {why}"),
    };
    let members = members(words, entries);
    let mut out = format!(
        r"// Generated by `cargo xtask codegen` from the layout table (`crates/ledger/src/payload.rs`); do not edit.
// The read side (lowering Part 3a; payload §5, §6): one `SimState`, fixed across tiers, `sample_read`, which fills it
// from the stored buffers one member at a time (R-378), and the quantities derived at read, never stored (R-79). It
// follows the unpack layer, `payload_unpack.wgsl`, whose accessors, stored layouts and bindings it reads.

// The canonical quiet NaN's f32 bits: sign 0, exponent all ones, the quiet bit alone in the significand (lowering
// Part 3a; R-72, R-79). A tier-absent derived scalar and an invalid read hold exactly these bits.
const CANONICAL_QNAN_BITS: u32 = {qnan:#010x}u;

// The word an unbound word buffer reads: payload zero, `length_raw` the truncation sentinel, so every word-derived
// validity predicate reads it as invalid (lowering Part 3a; R-72).
const FGW_UNBOUND: vec4<u32> = {unbound};

// The canonical quiet NaN, written from its bits, never computed (lowering Part 3a).
fn canonical_nan() -> f32 {{ return bitcast<f32>(CANONICAL_QNAN_BITS); }}

// The sim-key values the read side derives from (lowering Part 3; payload §4, §5).
struct ReadParams {{
    dt_macro: f32,
    delta_0: f32,
    n_renorm: u32,
    horizon_steps: u32,
}}

// The read-side `SimState`, the same at every tier (lowering Part 3a).
struct SimState {{
"
    );
    for m in &members {
        let _ = writeln!(out, "    {}: {},", m.name, m.wgsl);
    }
    out.push_str("}\n");
    out.push_str(WGSL_HELPERS);
    out.push_str(&sample_read(&members, tier, fields)?);
    Ok(out)
}

/// The fragment's generated WGSL at `tier`: the unpack layer with that tier's bindings ([`super::wgsl::layer`]), then
/// the read side whose `sample_read` fills `fields` ([`wgsl_for`]); or the first name that is no field (R-378).
///
/// This is the assembler's one way to the read side (TASK-M1-04): `fields` is the set of read-side fields the stain
/// reads, derived from its IR, never the checked-in full-fill file, which loads every stored member. A field the stain
/// reads but `fields` misses reads zero (`sample_read`), so the assembler refuses that stain rather than assemble it.
pub fn assemble(
    words: &[Word],
    entries: &[Entry],
    tier: Tier,
    fields: &[&str],
) -> Result<String, String> {
    Ok(format!(
        "{}{}",
        frag::layer(words, entries, tier),
        wgsl_for(words, entries, tier, fields)?
    ))
}

/// The WGSL derived accessors, as [`RUST_HELPERS`]'s. `select` evaluates both arms; the invalid arm's arithmetic is
/// discarded, and its result is the canonical NaN's bits either way.
const WGSL_HELPERS: &str = r"
// The renormalisations completed by step `n`, `n / n_renorm` under the uniform schedule (payload §5); none when
// `n_renorm` is 0.
fn completed_renorms(n: u32, n_renorm: u32) -> u32 { return select(0u, n / max(n_renorm, 1u), n_renorm > 0u); }

// Whether `ftle` is usable: the FTLE tier on, the sample not failed, at least one step and at least one completed
// renormalisation (payload §6). Derived, never stored; `ftle` reads NaN whenever it is false (R-254).
fn ftle_valid(state: u32, ftle_tier_on: bool, n: u32, benettin_renorms: u32) -> bool {
    return ftle_tier_on && !sd_is_failed(state) && n > 0u && benettin_renorms > 0u;
}

// The shadow's current separation `δ = ‖x' − x‖`, the Euclidean norm of the shadow's offset over the twelve
// phase-space components `(r, p)` (payload §5).
fn benettin_delta(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, r_sh: array<vec2<f32>, 3>, p_sh: array<vec2<f32>, 3>) -> f32 {
    let r0 = r_sh[0] - r[0];
    let r1 = r_sh[1] - r[1];
    let r2 = r_sh[2] - r[2];
    let p0 = p_sh[0] - p[0];
    let p1 = p_sh[1] - p[1];
    let p2 = p_sh[2] - p[2];
    return sqrt(dot(r0, r0) + dot(r1, r1) + dot(r2, r2) + dot(p0, p0) + dot(p1, p1) + dot(p2, p2));
}

// The FTLE with the partial renormalisation interval finalised: `S_final = S + ln(δ/δ₀)`, divided by the full elapsed
// `n · dt_macro`, never plain `S / t` (payload §5); the canonical quiet NaN when `valid` is false (R-254).
fn ftle(s_sum: f32, delta: f32, delta_0: f32, n: u32, dt_macro: f32, valid: bool) -> f32 {
    return select(canonical_nan(), (s_sum + log(delta / delta_0)) / (f32(n) * dt_macro), valid);
}

// Whether the diffusion fit is valid: `n ≥ 2`, as `C_tt(n)` is 0 below (payload §4, §6; R-245).
fn diffusion_slope_valid(n: u32) -> bool { return n >= 2u; }

// `C_tt(n) = h²·n(n²−1)/12`, derived closed-form, never stored (payload §4).
fn diffusion_c_tt(n: u32, h: f32) -> f32 {
    let m = f32(n);
    return h * h * m * (m - 1.0) * (m + 1.0) / 12.0;
}

// The diffusion slope `C_ty / C_tt(n)`, `n` the sample's own `t_end_step`, never the playhead; the canonical quiet NaN
// for `n < 2` (payload §4; R-245).
fn diffusion_slope(c_ty: f32, n: u32, h: f32) -> f32 {
    return select(canonical_nan(), c_ty / diffusion_c_tt(n, h), diffusion_slope_valid(n));
}

// The winding count `⌊|θ̃| / 2π⌋` (payload §5), clamped in f32 to the largest f32 below 2³² before the cast.
fn orbit_count(theta: f32) -> u32 { return u32(min(floor(abs(theta) / 6.2831855), 4294967040.0)); }

// The winding sense: `θ̃ < 0` (payload §5).
fn retrograde(theta: f32) -> bool { return theta < 0.0; }

// The Hamiltonian of the planar three-body problem in CoM-frame particle coordinates, `G = 1`: `H(r, p) = K + V`,
// `K = Σᵢ ‖pᵢ‖²/2mᵢ`, `V = −Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖` (integrator dd §3.5; decoder dd §3.6), the masses `m` the sample's
// `ICDescriptor` `m0 m1 m2` (dd_generation_root §3.6).
fn hamiltonian(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, m: vec3<f32>) -> f32 {
    let r01 = r[0] - r[1];
    let r02 = r[0] - r[2];
    let r12 = r[1] - r[2];
    let k = dot(p[0], p[0]) / (2.0 * m.x) + dot(p[1], p[1]) / (2.0 * m.y) + dot(p[2], p[2]) / (2.0 * m.z);
    let v = m.x * m.y / sqrt(dot(r01, r01)) + m.x * m.z / sqrt(dot(r02, r02)) + m.y * m.z / sqrt(dot(r12, r12));
    return k - v;
}

// The angular momentum `L_z(r, p) = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})` (integrator dd §3.5).
fn angular_momentum_z(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>) -> f32 {
    return (r[0].x * p[0].y - r[0].y * p[0].x) + (r[1].x * p[1].y - r[1].y * p[1].x) + (r[2].x * p[2].y - r[2].y * p[2].x);
}

// The current energy drift `ΔE = H(r, p) − E_0`, in live f32 (payload §5; dd_generation_root §3.8, R-246).
fn energy_drift(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, m: vec3<f32>, e_0: f32) -> f32 {
    return hamiltonian(r, p, m) - e_0;
}

// The current angular-momentum drift `ΔLz = L_z(r, p) − Lz_0`, in live f32 (payload §5; dd_generation_root §3.8).
fn Lz_drift(r: array<vec2<f32>, 3>, p: array<vec2<f32>, 3>, lz_0: f32) -> f32 {
    return angular_momentum_z(r, p) - lz_0;
}
";
