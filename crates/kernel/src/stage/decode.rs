//! The decode stage (dd_decoder §2; chart_decoder_contract Part 2, Design axioms 3 and 4; core_design seams 2, 3 and
//! 5): per sample, `(u, v) → Φ → D → (m, r, p)`, in the kernel, monomorphised per chart and link set, and the
//! `ICDescriptor` written post-decode and pre-integration.
//!
//! **Monomorphised, never interpreted** (REQ-CHART-001, REQ-DEC-007; lowering Part 3, "Compute side"). [`sample`] is
//! generic over the chart `C` and the link selection `S`: each `(C, S)` is its own compiled variant, a
//! [`DecodeVariant`], whose compute entry point is a concrete `#[spirv(compute)]` wrapper ([`latent_defaults`],
//! [`latent_stick_breaking`], [`invariant_le_defaults`], [`invariant_lk_defaults`], and the charts whose masses vary per
//! pixel, [`mass_simplex_defaults`], [`burrau_defaults`], [`euclid_plane_defaults`] and [`mixed_mu1_alpha_defaults`])
//! calling [`entry`]. Nothing at run
//! time names a chart, a chart id or an axis kind: adding a chart is a `Φ` ([`crate::chart`]), its decode
//! ([`ChartDecode`]), its [`UniformChart`] layout and one wrapper.
//!
//! **Uniforms, not code** (lowering Part 3; chart_decoder_contract Design axiom 6). The chart's parameters, `z₀`, `q₁`
//! and `q₂` for the latent chart, the slice values among `z₀`'s components, are the dispatch's uniform
//! ([`DecodeUniforms::chart`], [`UniformChart`]), so a navigation edit writes the uniform and recompiles nothing.
//!
//! **`DECODE_MODE`** ([`DecodeUniforms::flags`], [`decode_mode`]): bit 0 of the quad's flags word, full (0) or
//! linearised (1) (scheduler_contract Part 5, `QuadRequest.flags`; deep_zoom §2). It is read from the uniform, so the
//! branch on it is uniform over the dispatch, and the dispatch is one quad's: a per-quad, workgroup-uniform flag, not a
//! runtime axis-kind interpreter (lowering Part 3's note). Only the full path exists here; the linearised path, `x₀ +
//! J_D·δ`, lands in M6 (REQ-DEC-033), and until then a linearised quad writes nothing and the host refuses to dispatch
//! one (`engine::decode_dispatch`).
//!
//! **The seam** (seam 3; REQ-DEC-005). [`DecodeOut::ic`], the canonical `(m, r, p)` ([`Ic`]), is the stage's only output
//! to the integrator side, written to a buffer of its own that holds `(m, r, p)` and nothing else ([`write_ic`]): no
//! chart, chart id or chart parameter is in it. The descriptor and the `DEGENERATE` tag go to the payload side's buffers
//! ([`write_descriptor`], [`tag`]).
//!
//! **Domain** (pitfalls §10): the stage is stateless; what its tests and the `decode-f32-eps` gate measure holds for
//! the decode alone, its inputs fixed, and says nothing about a trajectory.

use core::mem::offset_of;

use spirv_std::glam::{UVec3, UVec4, Vec4};
use spirv_std::spirv;

use crate::chart::invariant::{Invariant, WarpReal};
use crate::chart::mixed::{Alpha, ZMu1};
use crate::chart::{
    Burrau, Chart, ChartDecode, EuclidPlane, InvariantLE, InvariantLK, Latent, MassSimplex, Mixed,
};
use crate::decode::{descriptor, Degenerate, Ic};
use crate::generated::links::{
    tanh_sat, Defaults, LinkReal, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    StickBreaking,
};
use crate::payload::{ICDescriptor, ICDescriptorOf, PayloadReal};

/// The decode stage's per-dispatch uniform: the chart's parameters and the quad's flags word.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct DecodeUniforms {
    /// The chart's parameters, 24 floats in six 16-byte rows, laid out by the chart's [`UniformChart`].
    pub chart: [Vec4; 6],
    /// `x`: the quad's flags word, bit 0 `DECODE_MODE` ([`decode_mode`]); `y`, `z`, `w`: zero, padding to 16 bytes.
    pub flags: UVec4,
}

/// The decoder a quad runs (scheduler_contract Part 5, `QuadRequest.flags` bit 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeMode {
    /// The full nonlinear decode, `D ∘ Φ` (0).
    Full,
    /// The linearised decode `x₀ + J_D·δ` (1), deep_zoom §2; M6.
    Linearised,
}

/// The decoder `flags` selects: bit 0, `DECODE_MODE`, 0 full and 1 linearised (scheduler_contract Part 5).
#[inline]
pub fn decode_mode(flags: u32) -> DecodeMode {
    if flags & 1 == 0 {
        DecodeMode::Full
    } else {
        DecodeMode::Linearised
    }
}

/// The flags word with `DECODE_MODE` set to `mode` and no other bit.
#[inline]
pub fn flags(mode: DecodeMode) -> u32 {
    match mode {
        DecodeMode::Full => 0,
        DecodeMode::Linearised => 1,
    }
}

/// A chart whose parameters the decode stage reads from its uniform ([`DecodeUniforms::chart`]), held at f32 (R-118,
/// chart_reference §5.1: the kernel's uniform holds them at f32), and which it decodes by its own [`ChartDecode`].
pub trait UniformChart: Chart<Param = f32> + ChartDecode + Sized {
    /// The chart whose parameters are `rows`.
    fn from_uniform(rows: &[Vec4; 6]) -> Self;
    /// The chart's parameters as the uniform's rows; [`UniformChart::from_uniform`] reads them back.
    fn uniform(&self) -> [Vec4; 6];
}

/// `[a.x, a.y, a.z, a.w, b.x, b.y, b.z, b.w]`.
#[inline]
fn eight(a: Vec4, b: Vec4) -> [f32; 8] {
    [a.x, a.y, a.z, a.w, b.x, b.y, b.z, b.w]
}

/// `v[0..4]` and `v[4..8]` as two rows.
#[inline]
fn rows(v: &[f32; 8]) -> [Vec4; 2] {
    [
        Vec4::new(v[0], v[1], v[2], v[3]),
        Vec4::new(v[4], v[5], v[6], v[7]),
    ]
}

/// The latent chart's uniform: rows 0–1 `z₀`, rows 2–3 `q₁`, rows 4–5 `q₂`, each in the decoder's block layout.
impl UniformChart for Latent<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        Latent {
            z0: eight(r[0], r[1]),
            q1: eight(r[2], r[3]),
            q2: eight(r[4], r[5]),
        }
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        let [a, b] = rows(&self.z0);
        let [c, d] = rows(&self.q1);
        let [e, f] = rows(&self.q2);
        [a, b, c, d, e, f]
    }
}

/// An invariant chart's uniform: row 0 `(z_α, z_β, z_μ₁, z_μ₂)`, row 1 `(K_max, γ_K, 0, 0)`, rows 2–5 zero.
#[inline]
fn invariant_rows(c: &Invariant<f32>) -> [Vec4; 6] {
    let zero = Vec4::ZERO;
    [
        Vec4::new(c.z_cfg[0], c.z_cfg[1], c.z_mass[0], c.z_mass[1]),
        Vec4::new(c.k_max, c.gamma_k, 0.0, 0.0),
        zero,
        zero,
        zero,
        zero,
    ]
}

/// The invariant chart whose uniform is `r`, as [`invariant_rows`] lays it out.
#[inline]
fn invariant_of(r: &[Vec4; 6]) -> Invariant<f32> {
    Invariant {
        z_cfg: [r[0].x, r[0].y],
        z_mass: [r[0].z, r[0].w],
        k_max: r[1].x,
        gamma_k: r[1].y,
    }
}

/// The `(L_z, E)` chart's uniform: row 0 `(z_α, z_β, z_μ₁, z_μ₂)`, row 1 `(K_max, γ_K, 0, 0)`, rows 2–5 zero.
impl UniformChart for InvariantLE<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        InvariantLE(invariant_of(r))
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        invariant_rows(&self.0)
    }
}

/// The `(L_z, K)` chart's uniform, laid out as the `(L_z, E)` chart's.
impl UniformChart for InvariantLK<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        InvariantLK(invariant_of(r))
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        invariant_rows(&self.0)
    }
}

/// The ternary mass plot's uniform: row 0 `(ν₀, ε_m, 0, 0)`, rows 1–5 zero.
impl UniformChart for MassSimplex<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        MassSimplex {
            nu0: r[0].x,
            eps_m: r[0].y,
        }
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        let zero = Vec4::ZERO;
        [
            Vec4::new(self.nu0, self.eps_m, 0.0, 0.0),
            zero,
            zero,
            zero,
            zero,
            zero,
        ]
    }
}

/// The `(ν, K)` chart's uniform: row 0 `(ν_min, ν_max, K_max, γ_K)`, rows 1–5 zero.
impl UniformChart for Burrau<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        Burrau {
            nu_range: [r[0].x, r[0].y],
            k_max: r[0].z,
            gamma_k: r[0].w,
        }
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        let zero = Vec4::ZERO;
        [
            Vec4::new(self.nu_range[0], self.nu_range[1], self.k_max, self.gamma_k),
            zero,
            zero,
            zero,
            zero,
            zero,
        ]
    }
}

/// The Euclid plane's uniform: row 0 `(m_min, m_max, ν_min, ν_max)`, rows 1–5 zero.
impl UniformChart for EuclidPlane<f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        EuclidPlane {
            m_range: [r[0].x, r[0].y],
            nu_range: [r[0].z, r[0].w],
        }
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        let zero = Vec4::ZERO;
        [
            Vec4::new(
                self.m_range[0],
                self.m_range[1],
                self.nu_range[0],
                self.nu_range[1],
            ),
            zero,
            zero,
            zero,
            zero,
            zero,
        ]
    }
}

/// The mixed chart `(z_μ1, α)`'s uniform: rows 0–1 the frozen controls, row 2 `(h_lo, h_hi, v_lo, v_hi)`, rows 3–5
/// zero.
impl UniformChart for Mixed<ZMu1, Alpha, f32> {
    #[inline]
    fn from_uniform(r: &[Vec4; 6]) -> Self {
        Mixed::new(eight(r[0], r[1]), [r[2].x, r[2].y], [r[2].z, r[2].w])
    }

    #[inline]
    fn uniform(&self) -> [Vec4; 6] {
        let [a, b] = rows(&self.frozen);
        let zero = Vec4::ZERO;
        let ranges = Vec4::new(
            self.h_range[0],
            self.h_range[1],
            self.v_range[0],
            self.v_range[1],
        );
        [a, b, ranges, zero, zero, zero]
    }
}

/// One chart and link set the kernel is built for: the chart, the selection, and the name of its compute entry point
/// in the WGSL, as naga names it from the wrapper's module path.
pub trait DecodeVariant {
    /// The chart, a type parameter of the stage.
    type Chart: UniformChart;
    /// The link selection, a type parameter of the stage.
    type Links: Selection;
    /// The compute entry point's name.
    const ENTRY: &'static str;
}

/// The latent chart under the default links ([`Defaults`]).
pub struct LatentDefaults;

impl DecodeVariant for LatentDefaults {
    type Chart = Latent<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_latent_defaults";
}

/// The default links with `stick_breaking` the mass link: the selection whose saturated corner reaches
/// `DEGENERATE(M01_TINY)` (dd_decoder §3.1; `crate::decode::mass`).
pub type StickBreakingLinks =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// The latent chart under [`StickBreakingLinks`].
pub struct LatentStickBreaking;

impl DecodeVariant for LatentStickBreaking {
    type Chart = Latent<f32>;
    type Links = StickBreakingLinks;
    const ENTRY: &'static str = "stage_decode_latent_stick_breaking";
}

/// The `(L_z, E)` chart under the default links.
pub struct InvariantLEDefaults;

impl DecodeVariant for InvariantLEDefaults {
    type Chart = InvariantLE<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_invariant_le_defaults";
}

/// The `(L_z, K)` chart under the default links.
pub struct InvariantLKDefaults;

impl DecodeVariant for InvariantLKDefaults {
    type Chart = InvariantLK<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_invariant_lk_defaults";
}

/// The ternary mass plot under the default links, which it does not read: its masses are its own.
pub struct MassSimplexDefaults;

impl DecodeVariant for MassSimplexDefaults {
    type Chart = MassSimplex<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_mass_simplex_defaults";
}

/// The `(ν, K)` chart under the default links, which it does not read.
pub struct BurrauDefaults;

impl DecodeVariant for BurrauDefaults {
    type Chart = Burrau<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_burrau_defaults";
}

/// The Euclid plane under the default links, which it does not read.
pub struct EuclidPlaneDefaults;

impl DecodeVariant for EuclidPlaneDefaults {
    type Chart = EuclidPlane<f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_euclid_plane_defaults";
}

/// The mixed chart `(z_μ1, α)`, a mass axis and a derived configuration axis, under the default links.
pub struct MixedMu1AlphaDefaults;

impl DecodeVariant for MixedMu1AlphaDefaults {
    type Chart = Mixed<ZMu1, Alpha, f32>;
    type Links = Defaults;
    const ENTRY: &'static str = "stage_decode_mixed_mu1_alpha_defaults";
}

/// One sample's decode: the integrator's input and the payload side's record of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodeOut<R: PayloadReal> {
    /// The canonical `(m, r, p)`: the only output that crosses to the integrator (seam 3).
    pub ic: Ic<R>,
    /// The `ICDescriptor`, written post-decode, pre-integration.
    pub descriptor: ICDescriptorOf<R>,
    /// `Some(reason)` for a `DEGENERATE` decode, kept, never dropped.
    pub degenerate: Option<Degenerate>,
}

/// The decode stage at `(u, v)`: `D ∘ Φ` through the shared decoder, the chart's own ([`ChartDecode`]), under the chart
/// `C` and the selection `S`, at the float type `R`: f32 in the kernel ([`entry`]) and f64 on the CPU
/// (`engine::decode_dispatch::decode_only`), from this one source (R-118, R-265).
#[inline]
pub fn sample<C, S, R>(chart: &C, u: R, v: R) -> DecodeOut<R>
where
    C: ChartDecode,
    S: Selection,
    R: LinkReal + PayloadReal + WarpReal + From<C::Param>,
{
    let d = chart.decode_at::<S, R>(u, v);
    DecodeOut {
        ic: d.ic,
        descriptor: d.descriptor,
        degenerate: d.degenerate,
    }
}

/// The words of one sample's `(m, r, p)` in the integrator's input buffer: `m₀ m₁ m₂`, then `r₀ r₁ r₂` and `p₀ p₁ p₂`,
/// each `x` then `y`.
#[inline]
pub fn ic_words() -> usize {
    core::mem::size_of::<Ic<f32>>() / 4
}

/// The words of one sample's `ICDescriptor`, the generated struct at f32 (64 B).
#[inline]
pub fn descriptor_words() -> usize {
    core::mem::size_of::<ICDescriptor>() / 4
}

/// The tag word of `degenerate`: 0 for none, 1 for `M01_TINY`, 2 for `ENERGY_INFEASIBLE`, 3 for `ENERGY_UNSCALABLE`.
/// It is this dispatch's readback, not payload §2's `decode_failed` detail code, whose map is REQ-DEC-038's.
#[inline]
pub fn tag(degenerate: Option<Degenerate>) -> u32 {
    match degenerate {
        None => 0,
        Some(Degenerate::M01Tiny) => 1,
        Some(Degenerate::EnergyInfeasible) => 2,
        Some(Degenerate::EnergyUnscalable) => 3,
    }
}

/// The `DEGENERATE` reason of a tag word, `Err` with the word for one [`tag`] never writes.
#[inline]
pub fn untag(word: u32) -> Result<Option<Degenerate>, u32> {
    match word {
        0 => Ok(None),
        1 => Ok(Some(Degenerate::M01Tiny)),
        2 => Ok(Some(Degenerate::EnergyInfeasible)),
        3 => Ok(Some(Degenerate::EnergyUnscalable)),
        other => Err(other),
    }
}

/// Writes the f32 value `v` at word `at` of `out`.
#[inline]
fn put(out: &mut [u32], at: usize, v: f32) {
    out[at] = v.to_bits();
}

/// Writes `ic` as sample `i`'s words in `out`, the integrator's input buffer, in [`ic_words`]' order: constant offsets.
#[inline]
pub fn write_ic(out: &mut [u32], i: usize, ic: &Ic<f32>) {
    write_ic_at(out, i * ic_words(), ic);
}

/// Writes `ic`'s [`ic_words`] words in `out` from word `at`, in [`write_ic`]'s order.
#[inline]
pub fn write_ic_at(out: &mut [u32], at: usize, ic: &Ic<f32>) {
    put(out, at, ic.m[0]);
    put(out, at + 1, ic.m[1]);
    put(out, at + 2, ic.m[2]);
    put(out, at + 3, ic.r[0][0]);
    put(out, at + 4, ic.r[0][1]);
    put(out, at + 5, ic.r[1][0]);
    put(out, at + 6, ic.r[1][1]);
    put(out, at + 7, ic.r[2][0]);
    put(out, at + 8, ic.r[2][1]);
    put(out, at + 9, ic.p[0][0]);
    put(out, at + 10, ic.p[0][1]);
    put(out, at + 11, ic.p[1][0]);
    put(out, at + 12, ic.p[1][1]);
    put(out, at + 13, ic.p[2][0]);
    put(out, at + 14, ic.p[2][1]);
}

/// The f32 value at word `at` of `words`.
#[inline]
fn get(words: &[u32], at: usize) -> f32 {
    f32::from_bits(words[at])
}

/// Sample `i`'s `(m, r, p)` from the integrator's input buffer `words`, as [`write_ic`] wrote it. No closure holds the
/// slice, so the kernel's SPIR-V keeps no slice pointer in a struct, which naga refuses.
#[inline]
pub fn read_ic(words: &[u32], i: usize) -> Ic<f32> {
    read_ic_at(words, i * ic_words())
}

/// The `(m, r, p)` whose [`ic_words`] words start at word `at` of `words`, as [`write_ic_at`] wrote them.
#[inline]
pub fn read_ic_at(words: &[u32], at: usize) -> Ic<f32> {
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

/// Writes `d` as sample `i`'s `ICDescriptor` in `out`, each member at its offset in the generated struct, the padding
/// as `d` holds it (zero).
#[inline]
pub fn write_descriptor(out: &mut [u32], i: usize, d: &ICDescriptor) {
    let base = i * descriptor_words();
    let at = |offset: usize| base + offset / 4;
    put(out, at(offset_of!(ICDescriptor, m0)), d.m0);
    put(out, at(offset_of!(ICDescriptor, m1)), d.m1);
    put(out, at(offset_of!(ICDescriptor, m2)), d.m2);
    put(out, at(offset_of!(ICDescriptor, q_mass)), d.q_mass);
    put(out, at(offset_of!(ICDescriptor, rho_mag)), d.rho_mag);
    put(out, at(offset_of!(ICDescriptor, lambda_mag)), d.lambda_mag);
    put(out, at(offset_of!(ICDescriptor, rho_ratio)), d.rho_ratio);
    put(out, at(offset_of!(ICDescriptor, rho_angle)), d.rho_angle);
    put(out, at(offset_of!(ICDescriptor, K_0)), d.K_0);
    put(out, at(offset_of!(ICDescriptor, V_0)), d.V_0);
    put(
        out,
        at(offset_of!(ICDescriptor, virial_ratio)),
        d.virial_ratio,
    );
    put(
        out,
        at(offset_of!(ICDescriptor, r_min_pair_0)),
        d.r_min_pair_0,
    );
    let pad = at(offset_of!(ICDescriptor, _pad));
    out[pad] = d._pad[0];
    out[pad + 1] = d._pad[1];
    out[pad + 2] = d._pad[2];
    out[pad + 3] = d._pad[3];
}

/// Sample `i`'s `ICDescriptor` from the descriptor buffer `words`, as [`write_descriptor`] wrote it.
#[inline]
pub fn read_descriptor(words: &[u32], i: usize) -> ICDescriptor {
    let base = i * descriptor_words();
    let at = |offset: usize| base + offset / 4;
    let pad = at(offset_of!(ICDescriptor, _pad));
    ICDescriptor {
        m0: get(words, at(offset_of!(ICDescriptor, m0))),
        m1: get(words, at(offset_of!(ICDescriptor, m1))),
        m2: get(words, at(offset_of!(ICDescriptor, m2))),
        q_mass: get(words, at(offset_of!(ICDescriptor, q_mass))),
        rho_mag: get(words, at(offset_of!(ICDescriptor, rho_mag))),
        lambda_mag: get(words, at(offset_of!(ICDescriptor, lambda_mag))),
        rho_ratio: get(words, at(offset_of!(ICDescriptor, rho_ratio))),
        rho_angle: get(words, at(offset_of!(ICDescriptor, rho_angle))),
        K_0: get(words, at(offset_of!(ICDescriptor, K_0))),
        V_0: get(words, at(offset_of!(ICDescriptor, V_0))),
        virial_ratio: get(words, at(offset_of!(ICDescriptor, virial_ratio))),
        r_min_pair_0: get(words, at(offset_of!(ICDescriptor, r_min_pair_0))),
        _pad: [words[pad], words[pad + 1], words[pad + 2], words[pad + 3]],
        _tail: Default::default(),
    }
}

/// The decode stage's body for the variant `V`, invocation `i`: under `DECODE_MODE` full, sample `i`'s `(u, v)`, the
/// pair `uv[2i], uv[2i + 1]`, decoded through `V`'s chart, its parameters read from `uniforms`, and its `(m, r, p)`,
/// tag and descriptor written at sample `i` of `ic`, `tags` and `descriptors`. Invocations past the last sample write
/// nothing, and so does a linearised quad, whose path lands in M6 (the module's note).
#[inline]
pub fn entry<V: DecodeVariant>(
    i: u32,
    uniforms: &DecodeUniforms,
    uv: &[f32],
    ic: &mut [u32],
    tags: &mut [u32],
    descriptors: &mut [u32],
) {
    let i = i as usize;
    if i < uv.len() / 2 && decode_mode(uniforms.flags.x) == DecodeMode::Full {
        let chart = V::Chart::from_uniform(&uniforms.chart);
        let out = sample::<V::Chart, V::Links, f32>(&chart, uv[2 * i], uv[2 * i + 1]);
        write_ic(ic, i, &out.ic);
        tags[i] = tag(out.degenerate);
        write_descriptor(descriptors, i, &out.descriptor);
    }
}

/// The decode stage's compute entry point for [`LatentDefaults`], one invocation per sample, at the workgroup size of the
/// GPU harness (64).
#[spirv(compute(threads(64)))]
pub fn latent_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<LatentDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`LatentStickBreaking`], as [`latent_defaults`] is for the defaults.
#[spirv(compute(threads(64)))]
pub fn latent_stick_breaking(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<LatentStickBreaking>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`InvariantLEDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn invariant_le_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<InvariantLEDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`InvariantLKDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn invariant_lk_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<InvariantLKDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`MassSimplexDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn mass_simplex_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<MassSimplexDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`BurrauDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn burrau_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<BurrauDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`EuclidPlaneDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn euclid_plane_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<EuclidPlaneDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The decode stage's compute entry point for [`MixedMu1AlphaDefaults`], as [`latent_defaults`] is for the latent chart.
#[spirv(compute(threads(64)))]
pub fn mixed_mu1_alpha_defaults(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] uniforms: &DecodeUniforms,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] uv: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] ic: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] tags: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] descriptors: &mut [u32],
) {
    entry::<MixedMu1AlphaDefaults>(id.x, uniforms, uv, ic, tags, descriptors);
}

/// The descriptor writer alone, on given `(m, r, p)`: invocation `i` reads sample `i` of `ic`, as [`write_ic`] wrote it,
/// and writes its `ICDescriptor`, [`descriptor::describe`] at f32, at sample `i` of `descriptors`: the decode stage's
/// own producer, on inputs the f32 decode need not reach, so that a test can read the pole values the GPU stores (R-436).
/// A pair separation of exactly 0 is one: dd_decoder §3.6 measured the collision pole at f64, where `cos α` at the image
/// of π/2 falls under the rounding of `r₀₁`; at f32, a sweep of the defaults at saturated `α` (`z_α` from 17 to 200,
/// `z_β` on `[−12, 12]`, both mass controls on `[−3, 3]`) found no pair separation of 0.
#[spirv(compute(threads(64)))]
pub fn describe(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] ic: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] descriptors: &mut [u32],
) {
    let i = id.x as usize;
    if i < ic.len() / ic_words() {
        write_descriptor(descriptors, i, &descriptor::describe(&read_ic(ic, i)));
    }
}

/// dd_generation_root §3.9's `tanh` operator alone ([`tanh_sat`]), one invocation per word: `output[i]` is the
/// operator at the f32 `input[i]`, and an invocation past the input writes nothing. It is not a decode: it lets a test
/// read the operator as the GPU evaluates it, its range included, at arguments the decode reaches only through the
/// links that use it (R-72; REQ-DEC-047).
#[spirv(compute(threads(64)))]
pub fn link_tanh(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] input: &[f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] output: &mut [f32],
) {
    let i = id.x as usize;
    if i < input.len() {
        output[i] = tanh_sat(input[i]);
    }
}
