//! The fragment's decode, encode and shape map, as entry points the translation exports (R-116; colour_composition §3,
//! "Fragment-side recompute"; REQ-COL-006, REQ-SYS-085): each compiles, from this one source, through rust-gpu →
//! SPIR-V → naga, and `cargo xtask build-kernel` lifts its body out of the kernel's WGSL into a fragment snippet
//! (`crates/render/wgsl/decode.wgsl`, `crates/render/wgsl/encode.wgsl`, `crates/ledger/wgsl/shape.wgsl`), with its
//! storage words made the snippet's private words and its uniform rebound in the fragment's context group. No snippet
//! is written by hand: the fragment runs the same decode, encode and shape map as the kernel and the CPU.
//!
//! Each entry point reads its inputs from, and writes its outputs to, one sized storage array, its *words*, at the
//! fixed offsets of the ledger's layout table (`crates/ledger/src/fragment.rs`, generated here as
//! `crate::generated::fragment_words`); the generated glue fills and reads them at the same offsets, read from the same
//! table. The arrays are sized, never runtime-sized, so the body
//! reads no `arrayLength` and indexes them only by constants.
//!
//! - The decode ([`decode_latent_defaults`] and the other charts' wrappers, [`decode_at`]): `(u, v)` at
//!   [`DECODE_UV`]; out, the chart's `Φ` output `z` ([`DECODE_Z`]; `Chart::map`'s `ChartOut.z`), whether it is a latent
//!   ([`DECODE_Z_LATENT`]: 1 on the latent chart, whose `Φ` produces the 8-D latent, 0 on every other chart, whose `Φ`
//!   does not; colour_composition §3, REQ-COL-057), the canonical `(m, r, p)` ([`DECODE_IC`], the decode stage's word
//!   order), the twelve `ICDescriptor` fields ([`DECODE_DESCRIPTOR`]) and the tag word ([`DECODE_TAG`],
//!   `crate::stage::decode::tag`). The chart's parameters, the flags word and the energy target are the decode stage's
//!   uniform ([`DecodeUniforms`]), read as the stage reads them, so the fragment decodes the pixel as the stage did.
//! - The descriptor alone ([`describe`]): `(m, r, p)` in, the twelve fields out, so a test reaches the producers' pole
//!   selects (R-436) on states no decode reaches at f32.
//! - The encode ([`encode()`]): `(m, r, p)` in, the default links' encode out in `crate::stage::encode`'s word order.
//! - The shape map ([`shape`]): `r` and `m` in, `n` out (`crate::shape::shape`, the read side's `shape()`).

use spirv_std::spirv;

use crate::chart::{
    BurrauThetaK, EuclidPlane, InvariantLE, InvariantLK, Latent, MassSimplex, ShapeSphere,
};
use crate::decode::{descriptor, Ic};
use crate::encode;
use crate::generated::links::{Defaults, Selection};
use crate::payload::ICDescriptorOf;
use crate::stage::decode::{sample_with, tag, target, DecodeMode, DecodeUniforms, UniformChart};

/// The words' offsets and sizes, generated from the ledger's layout table (`crates/ledger/src/fragment.rs`), which
/// `cargo xtask build-kernel`'s glue reads too.
pub use crate::generated::fragment_words::*;

/// Whether a chart's `Φ` produces the 8-D latent `z` (colour_composition §3; REQ-COL-057): the latent chart's does,
/// `z₀ + (2u − 1)·q₁ + (2v − 1)·q₂`; every other chart's hands the decoder its own output, and its `ChartOut.z` holds
/// only the latent blocks it carries (zeros elsewhere), no latent.
pub trait FragmentChart: UniformChart {
    /// `true` where `Φ`'s `z` is the 8-D latent.
    const Z_IS_LATENT: bool;
}

impl FragmentChart for Latent<f32> {
    const Z_IS_LATENT: bool = true;
}
impl FragmentChart for ShapeSphere<f32> {
    const Z_IS_LATENT: bool = false;
}
impl FragmentChart for InvariantLE<f32> {
    const Z_IS_LATENT: bool = false;
}
impl FragmentChart for InvariantLK<f32> {
    const Z_IS_LATENT: bool = false;
}
impl FragmentChart for MassSimplex<f32> {
    const Z_IS_LATENT: bool = false;
}
impl FragmentChart for EuclidPlane<f32> {
    const Z_IS_LATENT: bool = false;
}
impl FragmentChart for BurrauThetaK<f32> {
    const Z_IS_LATENT: bool = false;
}

/// The f32 value at word `at` of `words`.
#[inline]
fn get<const N: usize>(words: &[u32; N], at: usize) -> f32 {
    f32::from_bits(words[at])
}

/// Writes the f32 `v` at word `at` of `words`.
#[inline]
fn put<const N: usize>(words: &mut [u32; N], at: usize, v: f32) {
    words[at] = v.to_bits();
}

/// The `(m, r, p)` whose fifteen words start at word `at` of `words`, in the decode stage's order
/// (`crate::stage::decode::write_ic_at`'s). The words are a sized array, never unsized to a slice, which rust-gpu
/// cannot cast.
#[inline]
fn read_ic<const N: usize>(words: &[u32; N], at: usize) -> Ic<f32> {
    Ic {
        m: [get(words, at), get(words, at + 1), get(words, at + 2)],
        r: [
            [get(words, at + 3), get(words, at + 4)],
            [get(words, at + 5), get(words, at + 6)],
            [get(words, at + 7), get(words, at + 8)],
        ],
        p: [
            [get(words, at + 9), get(words, at + 10)],
            [get(words, at + 11), get(words, at + 12)],
            [get(words, at + 13), get(words, at + 14)],
        ],
    }
}

/// Writes `ic` as fifteen words from word `at` of `words`, in [`read_ic`]'s order.
#[inline]
fn write_ic<const N: usize>(words: &mut [u32; N], at: usize, ic: &Ic<f32>) {
    put(words, at, ic.m[0]);
    put(words, at + 1, ic.m[1]);
    put(words, at + 2, ic.m[2]);
    put(words, at + 3, ic.r[0][0]);
    put(words, at + 4, ic.r[0][1]);
    put(words, at + 5, ic.r[1][0]);
    put(words, at + 6, ic.r[1][1]);
    put(words, at + 7, ic.r[2][0]);
    put(words, at + 8, ic.r[2][1]);
    put(words, at + 9, ic.p[0][0]);
    put(words, at + 10, ic.p[0][1]);
    put(words, at + 11, ic.p[1][0]);
    put(words, at + 12, ic.p[1][1]);
    put(words, at + 13, ic.p[2][0]);
    put(words, at + 14, ic.p[2][1]);
}

/// Writes the twelve fields of `d` from word `at` of `words`, in the generated struct's order.
#[inline]
fn write_fields<const N: usize>(words: &mut [u32; N], at: usize, d: &ICDescriptorOf<f32>) {
    put(words, at, d.m0);
    put(words, at + 1, d.m1);
    put(words, at + 2, d.m2);
    put(words, at + 3, d.q_mass);
    put(words, at + 4, d.rho_mag);
    put(words, at + 5, d.lambda_mag);
    put(words, at + 6, d.rho_ratio);
    put(words, at + 7, d.rho_angle);
    put(words, at + 8, d.K_0);
    put(words, at + 9, d.V_0);
    put(words, at + 10, d.virial_ratio);
    put(words, at + 11, d.r_min_pair_0);
}

/// The fragment's decode of chart `C` under the links `S`, at the `(u, v)` its words hold: `Φ`'s `z`, whether it is a
/// latent, and the decode stage's own decode, [`sample_with`] toward the uniform's energy target, under `DECODE_MODE`
/// full. A linearised quad's flags write nothing past `(u, v)`, as the stage writes nothing for one (M6).
#[inline]
pub fn decode_at<C: FragmentChart, S: Selection>(
    uniforms: &DecodeUniforms,
    words: &mut [u32; DECODE_WORDS],
) {
    if crate::stage::decode::decode_mode(uniforms.flags.x) == DecodeMode::Full {
        let chart = C::from_uniform(&uniforms.chart);
        let u = get(words, DECODE_UV);
        let v = get(words, DECODE_UV + 1);
        let z = chart.map(u, v).z.z;
        put(words, DECODE_Z, z[0]);
        put(words, DECODE_Z + 1, z[1]);
        put(words, DECODE_Z + 2, z[2]);
        put(words, DECODE_Z + 3, z[3]);
        put(words, DECODE_Z + 4, z[4]);
        put(words, DECODE_Z + 5, z[5]);
        put(words, DECODE_Z + 6, z[6]);
        put(words, DECODE_Z + 7, z[7]);
        words[DECODE_Z_LATENT] = if C::Z_IS_LATENT { 1 } else { 0 };
        let out = sample_with::<C, S, f32>(&chart, u, v, target(uniforms.energy));
        write_ic(words, DECODE_IC, &out.ic);
        write_fields(words, DECODE_DESCRIPTOR, &out.descriptor);
        words[DECODE_TAG] = tag(out.degenerate);
    }
}

/// The fragment's decode of the latent chart under the default links ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_latent_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<Latent<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the shape sphere under the default links ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_shape_sphere_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<ShapeSphere<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the `(L_z, E)` chart under the default links ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_invariant_le_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<InvariantLE<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the `(L_z, K)` chart under the default links ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_invariant_lk_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<InvariantLK<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the ternary mass plot ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_mass_simplex_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<MassSimplex<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the Euclid ν plane ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_euclid_plane_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<EuclidPlane<f32>, Defaults>(uniforms, words);
}

/// The fragment's decode of the `θ × K` strip ([`decode_at`]).
#[spirv(compute(threads(1)))]
pub fn decode_burrau_theta_k_defaults(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &mut [u32; DECODE_WORDS],
) {
    decode_at::<BurrauThetaK<f32>, Defaults>(uniforms, words);
}

/// The descriptor alone, [`descriptor::describe`] at f32 on the `(m, r, p)` its words hold: the producers' pole selects
/// (R-436), as the fragment decode forms them. With no configuration block, its `rho_angle` is read from the positions;
/// the fragment decode reads it from the configuration block's `(ρ̃, λ̃)` (R-628).
#[spirv(compute(threads(1)))]
pub fn describe(
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] words: &mut [u32; DESCRIBE_WORDS],
) {
    let d = descriptor::describe(&read_ic(words, DESCRIBE_IC));
    write_fields(words, DESCRIBE_DESCRIPTOR, &d);
}

/// The encode under the default links on the `(m, r, p)` its words hold, written in `crate::stage::encode`'s order.
#[spirv(compute(threads(1)))]
pub fn encode(
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] words: &mut [u32; ENCODE_WORDS],
) {
    let e = encode::encode::<Defaults, f32>(&read_ic(words, ENCODE_IC));
    let z = e.z.z;
    put(words, ENCODE_OUT, z[0]);
    put(words, ENCODE_OUT + 1, z[1]);
    put(words, ENCODE_OUT + 2, z[2]);
    put(words, ENCODE_OUT + 3, z[3]);
    put(words, ENCODE_OUT + 4, z[4]);
    put(words, ENCODE_OUT + 5, z[5]);
    put(words, ENCODE_OUT + 6, z[6]);
    put(words, ENCODE_OUT + 7, z[7]);
    put(words, ENCODE_OUT + 8, e.blocks.alpha);
    put(words, ENCODE_OUT + 9, e.blocks.beta);
    put(words, ENCODE_OUT + 10, e.report.lambda);
    words[ENCODE_OUT + 11] = crate::stage::encode::notices(&e.report);
}

/// The shape map, `crate::shape::shape` at f32, on the `r` and `m` its words hold.
#[spirv(compute(threads(1)))]
pub fn shape(
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] words: &mut [u32; SHAPE_WORDS],
) {
    let r = [
        [get(words, SHAPE_R), get(words, SHAPE_R + 1)],
        [get(words, SHAPE_R + 2), get(words, SHAPE_R + 3)],
        [get(words, SHAPE_R + 4), get(words, SHAPE_R + 5)],
    ];
    let m = [
        get(words, SHAPE_M),
        get(words, SHAPE_M + 1),
        get(words, SHAPE_M + 2),
    ];
    let n = crate::shape::shape(r, m);
    put(words, SHAPE_N, n[0]);
    put(words, SHAPE_N + 1, n[1]);
    put(words, SHAPE_N + 2, n[2]);
}
