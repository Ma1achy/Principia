//! The IC Inspector's two gates on the Rust decode and encode (TASK-M2-27; REQ-VAL-021, REQ-VAL-022, REQ-VAL-121): the
//! reference tool's verified bounds (ic_inspector_scratchpad § "Status — as built"), re-established on
//! `kernel::decode` and `kernel::encode` at f64, the Inspector's CPU path.
//!
//! - **`inspector-handles`** (REQ-VAL-021; scratchpad § "Whole-system handles — the invariance audit"): each of the
//!   four whole-system handles, translate-all, boost-all, rotate-all and scale-all, leaves `z` fixed. Each handle is
//!   one reduction step of § "The quotient set": the CoM (Jacobi), the CoM frame (`P = 0`), the `ρ̃ → +x` pin and
//!   `R̃ = 1`. Per IC `x`, `max ‖E(h·x) − E(x)‖` over the four handles `h`.
//! - **`inspector-roundtrip`** (REQ-VAL-022): decode → canonicalise → `z`, `‖z − E(D(z))‖`; and the canonical CoM,
//!   `‖Σ mᵢrᵢ‖` of `C`'s output, within [`COM_BOUND`].
//!
//! **The norm and the ball** (applied per R-369). `‖·‖` on `z` is the Euclidean norm on ℝ⁸, and "`|z| < 4`" is the
//! ball `‖z‖ < 4`, drawn by rejection from the cube `[−4, 4]⁸`; every control is then inside `|zₖ| < 4`, so
//! `s(1 − s) ≥ 0.018`, far from the `10⁻⁶` clamps. The componentwise reading, the whole cube, puts all eight controls
//! at its corners at once; the Rust round trip there is about twice the reference's bound, driven by the mass block's
//! `artanh` at `|z_μ| → 4` (the calibration proposal, REQ-VAL-121, gives both).
//!
//! **The random ICs** (`inspector-handles`): `x = g₀·D(z)`, `z` in the ball, `g₀` a random gauge element
//! (`crate::gauge`) with `s₀` on `2^[−octaves, octaves)`, `θ` on `[−π, π)`, either parity, the offset on
//! `s₀·[−span, span)²` and the boost on `s₀^{−1/2}·[−span, span)²`, so the IC is a general physical state. Each
//! handle's size is drawn alike, in proportion to the state it moves: the offset `s₀·[−span, span)²`, the boost
//! `s₀^{−1/2}·[−span, span)²`, the turn `[−π, π)`, the factor `2^[−octaves, octaves)`.
//!
//! **The negative inputs** (PIT-3). `inspector-handles`'s `leak` drops one of `C`'s steps, each a gauge fixing that
//! leaks ([`Leak`]), so its own handle moves `z` by order 1 and the others only by rounding (on the typical IC; the
//! leaky state is off the section, so its tail can pass the bound): the first handle that nudges `z` localises the
//! leak (§ "Whole-system handles"). `inspector-roundtrip`'s `encode: "mismatched"` encodes under every link of the
//! defaults swapped (`crate::eps_phys::Mismatched`), and its `shift` moves `C`'s output by a known offset before the CoM
//! is read, so the CoM check must fail it.
//!
//! **Held** (RQ-355, RQ-471). REQ-VAL-022's "equilateral → pole and collinear → `w = 0` exactly" is reported, not
//! judged: what "exactly" means on floating-point states is RQ-471's, and the pole is the equilateral triangle's at
//! equal masses only (RQ-355). The gate prints the measured `w` on both families.

use serde::Deserialize;

use kernel::canon::{canonicalise, inertia};
use kernel::decode::descriptor::pi_image;
use kernel::decode::{self, mass, Ic, Latent};
use kernel::encode::blocks::invert;
use kernel::encode::encode;
use kernel::generated::links::{Defaults, Link, Selection};
use kernel::shape::shape;

use crate::decode_f32_eps::SplitMix;
use crate::eps_phys::{EncodeKind, Mismatched};
use crate::gate::report::{GateReport, RegionCount, Regions, Verdict};
use crate::gate::{Gate, GateConfig};
use crate::gauge::{uniform, Gauge};
use crate::oklab::quantile;

/// The handles gate's name, its directory under `fixtures/gates/`.
pub const HANDLES: &str = "inspector-handles";

/// The round-trip gate's name.
pub const ROUNDTRIP: &str = "inspector-roundtrip";

/// The canonical CoM's bound, REQ-VAL-022's own value: `‖Σ mᵢrᵢ‖ ≤ 1.7×10⁻¹⁶` on `C`'s output.
pub const COM_BOUND: f64 = 1.7e-16;

const PI: f64 = std::f64::consts::PI;

/// `‖a − b‖`, the Euclidean norm on ℝ⁸.
pub fn z_gap(a: [f64; 8], b: [f64; 8]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// A `z` uniform in the ball `‖z‖ < radius`, by rejection from the cube `[−radius, radius)⁸`.
pub fn ball(rng: &mut SplitMix, radius: f64) -> [f64; 8] {
    loop {
        let z: [f64; 8] = std::array::from_fn(|_| uniform(rng, -radius, radius));
        if z.iter().map(|x| x * x).sum::<f64>() < radius * radius {
            return z;
        }
    }
}

/// `D(z)`, the free decode under the defaults links.
pub fn decode_z(z: [f64; 8]) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&Latent::new(z)).ic
}

/// The four whole-system handles, in the order of § "Whole-system handles".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// Translate-all: every position moved by one offset.
    Translate,
    /// Boost-all: every velocity moved by one `u`, `pᵢ += mᵢu`.
    Boost,
    /// Rotate-all: positions and momenta turned together.
    Rotate,
    /// Scale-all: the similarity, `r → s·r`, `p → s^{−1/2}·p`.
    Scale,
}

impl Handle {
    /// The four, in order.
    pub const ALL: [Handle; 4] = [
        Handle::Translate,
        Handle::Boost,
        Handle::Rotate,
        Handle::Scale,
    ];

    /// The handle's name.
    pub fn word(self) -> &'static str {
        match self {
            Handle::Translate => "translate-all",
            Handle::Boost => "boost-all",
            Handle::Rotate => "rotate-all",
            Handle::Scale => "scale-all",
        }
    }

    /// This handle as a gauge element, its size drawn from `rng` in proportion to a state of scale `s0` (the module's
    /// note): every other component the identity's.
    pub fn draw(self, rng: &mut SplitMix, s0: f64, span: f64, octaves: f64) -> Gauge {
        let id = Gauge::identity();
        let mut pair = |k: f64| [uniform(rng, -span, span) * k, uniform(rng, -span, span) * k];
        match self {
            Handle::Translate => Gauge { a: pair(s0), ..id },
            Handle::Boost => Gauge {
                u: pair(1.0 / s0.sqrt()),
                ..id
            },
            Handle::Rotate => Gauge {
                theta: uniform(rng, -PI, PI),
                ..id
            },
            Handle::Scale => Gauge {
                s: uniform(rng, -octaves, octaves).exp2(),
                ..id
            },
        }
    }
}

/// One of `C`'s steps dropped from the encode: a gauge fixing that leaks, for the handles gate's negative inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Leak {
    /// None: `E` itself.
    None,
    /// Step 0's `I` taken about the origin, not in the CoM frame (R-23's order broken): translate-all moves `z`.
    Com,
    /// Step 1b dropped, the total momentum left in: boost-all moves `z`.
    Boost,
    /// Step 2 dropped, `ρ̃` not pinned to `+x`: rotate-all moves `z`.
    Rotate,
    /// Step 0's momentum factor `I^{1/4}` dropped: scale-all moves `z`.
    Scale,
}

/// `R(φ)·v`.
fn turn(v: [f64; 2], phi: f64) -> [f64; 2] {
    let (s, c) = phi.sin_cos();
    [c * v[0] - s * v[1], s * v[0] + c * v[1]]
}

/// `E(x)` under the defaults with `leak`'s step dropped: `C`'s output with that step undone, then the block inverses.
pub fn encode_leaky(x: &Ic<f64>, leak: Leak) -> [f64; 8] {
    let c = canonicalise(x);
    let mut ic = c.ic;
    let i_in = c.i_in;
    match leak {
        Leak::None => return encode::<Defaults, f64>(x).z.z,
        Leak::Com => {
            // Step 0 at `I₀ = Σ mᵢ‖rᵢ‖²` about the origin: the momenta by `(I₀/I_in)^{1/4}` more. The positions'
            // factor `√(I_in/I₀)` is left out, as `z` reads them only through `α` and `β`, which are scale-free.
            let k = (inertia(x.m, x.r) / i_in).sqrt().sqrt();
            ic.p = ic.p.map(|v| v.map(|w| w * k));
        }
        Leak::Boost => {
            let total = [0, 1].map(|k| x.p.iter().map(|p| p[k]).sum::<f64>());
            let mut t = turn(total, -c.phi).map(|w| w * i_in.sqrt().sqrt());
            if c.mirrored {
                t[1] = -t[1];
            }
            for (p, m) in ic.p.iter_mut().zip(x.m) {
                *p = [p[0] + m * t[0], p[1] + m * t[1]];
            }
        }
        Leak::Rotate => {
            ic.r = ic.r.map(|v| turn(v, c.phi));
            ic.p = ic.p.map(|v| turn(v, c.phi));
        }
        Leak::Scale => {
            ic.p = ic.p.map(|v| v.map(|w| w / i_in.sqrt().sqrt()));
        }
    }
    invert::<Defaults, f64>(&ic).z
}

/// A handles input, `fixtures/gates/inspector-handles/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlesInput {
    /// What the input is.
    pub description: String,
    /// The seed every draw comes from.
    pub seed: u64,
    /// The random ICs.
    pub ics: u32,
    /// The ball's radius in `z`.
    pub radius: f64,
    /// The offsets' and boosts' half-width, in units of the state's own size.
    pub span: f64,
    /// The scales' half-width, in octaves.
    pub octaves: f64,
    /// The step of `C` dropped, `none` for `E` itself.
    pub leak: Leak,
}

/// One random IC and its four handles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleSample {
    /// The IC, `g₀·D(z)`.
    pub ic: Ic<f64>,
    /// The four handles, in [`Handle::ALL`]'s order.
    pub handles: [Gauge; 4],
}

/// The input's ICs and handles, in draw order.
pub fn handle_samples(input: &HandlesInput) -> Vec<HandleSample> {
    let mut rng = SplitMix::new(input.seed);
    (0..input.ics)
        .map(|_| {
            let x = decode_z(ball(&mut rng, input.radius));
            let mut g0 = Gauge::random(&mut rng, input.span, input.octaves);
            g0.a = g0.a.map(|a| a * g0.s);
            g0.u = g0.u.map(|u| u / g0.s.sqrt());
            let handles = Handle::ALL.map(|h| h.draw(&mut rng, g0.s, input.span, input.octaves));
            HandleSample {
                ic: g0.act(&x),
                handles,
            }
        })
        .collect()
}

/// `‖E(h·x) − E(x)‖` for each of the sample's four handles, `E` with `leak`'s step dropped.
pub fn handle_gaps(sample: &HandleSample, leak: Leak) -> [f64; 4] {
    let z0 = encode_leaky(&sample.ic, leak);
    sample
        .handles
        .map(|h| z_gap(z0, encode_leaky(&h.act(&sample.ic), leak)))
}

/// The verdict on a worst value against its bound: a pass at or below it, a NaN a fail.
pub fn verdict(worst: f64, bound: f64) -> Verdict {
    if worst <= bound {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

/// The largest of `v`, a NaN anywhere giving NaN; and its first index.
pub fn worst(v: &[f64]) -> (f64, usize) {
    let mut out = (0.0f64, 0usize);
    for (k, &x) in v.iter().enumerate() {
        if x.is_nan() {
            return (f64::NAN, k);
        }
        if x > out.0 {
            out = (x, k);
        }
    }
    out
}

/// "max … at sample k; median …, p99 …, over n".
fn summary(what: &str, v: &[f64]) -> String {
    let (w, at) = worst(v);
    let q = |x| quantile(v, x).unwrap_or(f64::NAN);
    format!(
        "{what}: max {w:.3e} at sample {at}; median {:.3e}, p99 {:.3e}, over {} samples",
        q(0.5),
        q(0.99),
        v.len()
    )
}

/// "margin …×" on a pass, "k of n beyond …" on a fail.
fn outcome(what: &str, v: &[f64], bound: f64) -> (Verdict, String) {
    let (w, at) = worst(v);
    let over = v
        .iter()
        .filter(|&&x| verdict(x, bound) == Verdict::Fail)
        .count();
    match verdict(w, bound) {
        Verdict::Pass => (
            Verdict::Pass,
            format!(
                "{what}: 0 of {} beyond {bound:.3e}; margin {:.2}×",
                v.len(),
                bound / w
            ),
        ),
        Verdict::Fail => (
            Verdict::Fail,
            format!(
                "{what}: {over} of {} beyond {bound:.3e}; the worst, {w:.3e} at sample {at}, fails",
                v.len()
            ),
        ),
    }
}

fn regions(config: &GateConfig, n: usize) -> Result<Option<Regions>, String> {
    Ok(Some(Regions {
        count: RegionCount::Recorded(u32::try_from(n).map_err(|e| e.to_string())?),
        minimum: config.min_regions.clone(),
    }))
}

/// The `inspector-handles` gate.
pub struct InspectorHandlesGate;

impl InspectorHandlesGate {
    /// Measures `input` and judges it against `config`'s threshold, or refuses it.
    pub fn judge(
        config: &GateConfig,
        name: &str,
        input: &HandlesInput,
    ) -> Result<GateReport, String> {
        if input.ics == 0 {
            return Err("refused: no ICs".to_owned());
        }
        let bound = config.threshold.value;
        let gaps: Vec<[f64; 4]> = handle_samples(input)
            .iter()
            .map(|s| handle_gaps(s, input.leak))
            .collect();
        let mut scatter = Vec::new();
        let mut negative = Vec::new();
        let mut all = Verdict::Pass;
        for (k, h) in Handle::ALL.iter().enumerate() {
            let v: Vec<f64> = gaps.iter().map(|g| g[k]).collect();
            scatter.push(summary(&format!("{}: ‖Δz‖", h.word()), &v));
            let (verdict, line) = outcome(h.word(), &v, bound);
            if verdict == Verdict::Fail {
                all = Verdict::Fail;
            }
            negative.push(line);
        }
        if input.leak != Leak::None {
            negative.push(format!(
                "measured with C's {:?} step dropped (PIT-3: its own handle must fail)",
                input.leak
            ));
        }
        Ok(GateReport {
            gate: HANDLES.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: all,
            threshold: config.threshold.clone(),
            scatter,
            regions: regions(config, gaps.len())?,
            negative_results: negative,
        })
    }
}

impl Gate for InspectorHandlesGate {
    fn name(&self) -> &'static str {
        HANDLES
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = HandlesInput::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not an {HANDLES} input: {e}"))?;
        Self::judge(config, name, &input)
    }
}

/// A round-trip input, `fixtures/gates/inspector-roundtrip/<input>.json`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundtripInput {
    /// What the input is.
    pub description: String,
    /// The seed every draw comes from.
    pub seed: u64,
    /// The random `z`.
    pub samples: u32,
    /// The ball's radius.
    pub radius: f64,
    /// The encode: the defaults, or every link swapped.
    pub encode: EncodeKind,
    /// An offset added to every coordinate of `C`'s output before its CoM is read: 0, or the CoM control's.
    pub shift: f64,
    /// The draws of each held family, equilateral and collinear, reported only (RQ-471).
    pub shapes: u32,
}

/// `(s, e)` with `s + e = a + b` exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// `‖Σ mᵢrᵢ‖`, each sum formed without rounding but the last (products by `fma`, sums by [`two_sum`]), so the CoM
/// read is the state's own, not the reading's rounding.
pub fn com(x: &Ic<f64>) -> f64 {
    let axis = |k: usize| {
        let (mut s, mut e) = (0.0f64, 0.0f64);
        for (m, r) in x.m.iter().zip(&x.r) {
            let p = m * r[k];
            let (t, f) = two_sum(s, p);
            s = t;
            e += f + m.mul_add(r[k], -p);
        }
        s + e
    };
    axis(0).hypot(axis(1))
}

/// One round trip from `z`: `‖z − E(D(z))‖`, and the CoM of `C(D(z))` moved by `shift`.
pub fn round_trip(z: [f64; 8], kind: EncodeKind, shift: f64) -> (f64, f64) {
    let x = decode_z(z);
    let back = match kind {
        EncodeKind::Defaults => encode::<Defaults, f64>(&x).z.z,
        EncodeKind::Mismatched => encode::<Mismatched, f64>(&x).z.z,
    };
    let mut c = canonicalise(&x).ic;
    c.r = c.r.map(|v| v.map(|w| w + shift));
    (z_gap(z, back), com(&c))
}

/// The input's `z`, in draw order.
pub fn latents(input: &RoundtripInput) -> Vec<[f64; 8]> {
    let mut rng = SplitMix::new(input.seed);
    (0..input.samples)
        .map(|_| ball(&mut rng, input.radius))
        .collect()
}

/// The held families (RQ-471), measured: `[equilateral, collinear]` in each count.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    /// The equilateral states' `|w − 1|`.
    pub pole: Vec<f64>,
    /// The collinear states' `|w|`.
    pub equator: Vec<f64>,
    /// How many of each family have `z_β`'s clamp bit set.
    pub beta_bit: [usize; 2],
    /// How many of each family have `z_β` exactly at its clamp image, `β ∈ {0, π̂}` clamped.
    pub beta_edge: [usize; 2],
}

/// The held families, `n` of each, `w` read on `C`'s output through `kernel::shape::shape`: the equal-mass unit
/// equilateral triangle, and three bodies on the `x`-axis with masses from the mass block at `z_μ` on `[−4, 4)²`, each
/// through a random gauge element (either parity, so `L⁺` and `L⁻` both), at rest before it. Neither `w` nor `z_β`
/// reads the momenta.
pub fn held_shapes(seed: u64, n: u32) -> Held {
    let mut rng = SplitMix::new(seed);
    type BetaLink = <Defaults as Selection>::BetaLink;
    let edge = [0.0, pi_image::<f64>()].map(|b| BetaLink::inverse([b])[0]);
    let equal = mass::decode::<Defaults, f64>([0.0, 0.0]).m;
    let triangle = [[0.0, 0.0], [1.0, 0.0], [0.5, 3f64.sqrt() / 2.0]];
    let mut held = Held {
        pole: Vec::new(),
        equator: Vec::new(),
        beta_bit: [0; 2],
        beta_edge: [0; 2],
    };
    for _ in 0..n {
        let z_mu = [uniform(&mut rng, -4.0, 4.0), uniform(&mut rng, -4.0, 4.0)];
        let line: [f64; 3] = std::array::from_fn(|_| uniform(&mut rng, -1.0, 1.0));
        let states = [
            Ic {
                m: equal,
                r: triangle,
                p: [[0.0; 2]; 3],
            },
            Ic {
                m: mass::decode::<Defaults, f64>(z_mu).m,
                r: line.map(|t| [t, 0.0]),
                p: [[0.0; 2]; 3],
            },
        ];
        for (k, state) in states.iter().enumerate() {
            let x = Gauge::random(&mut rng, 2.0, 4.0).act(state);
            let e = encode::<Defaults, f64>(&x);
            let w = shape(e.canonical.ic.r, x.m)[2];
            if k == 0 {
                held.pole.push((w - 1.0).abs());
            } else {
                held.equator.push(w.abs());
            }
            held.beta_bit[k] += usize::from(e.report.clamped_controls & 2 != 0);
            held.beta_edge[k] += usize::from(edge.contains(&e.z.z[1]));
        }
    }
    held
}

/// The `inspector-roundtrip` gate.
pub struct InspectorRoundtripGate;

impl InspectorRoundtripGate {
    /// Measures `input` and judges it against `config`'s threshold and [`COM_BOUND`], or refuses it.
    pub fn judge(
        config: &GateConfig,
        name: &str,
        input: &RoundtripInput,
    ) -> Result<GateReport, String> {
        if input.samples == 0 {
            return Err("refused: no samples".to_owned());
        }
        let bound = config.threshold.value;
        let trips: Vec<(f64, f64)> = latents(input)
            .into_iter()
            .map(|z| round_trip(z, input.encode, input.shift))
            .collect();
        let gaps: Vec<f64> = trips.iter().map(|t| t.0).collect();
        let coms: Vec<f64> = trips.iter().map(|t| t.1).collect();
        let mut scatter = vec![
            summary("‖z − E(D(z))‖", &gaps),
            summary("‖Σ mᵢrᵢ‖ of C(D(z))", &coms),
        ];
        let (v_gap, line_gap) = outcome("the round trip", &gaps, bound);
        let (v_com, line_com) = outcome("the canonical CoM (REQ-VAL-022)", &coms, COM_BOUND);
        let mut negative = vec![line_gap, line_com];
        if input.encode == EncodeKind::Mismatched {
            negative.push(
                "encoded under every link of the defaults swapped (PIT-3: it must fail)".to_owned(),
            );
        }
        if input.shift != 0.0 {
            negative.push(format!(
                "C's output moved by {:e} before its CoM is read (PIT-3: the CoM check must fail)",
                input.shift
            ));
        }
        if input.shapes > 0 {
            let held = held_shapes(input.seed, input.shapes);
            let exact = |v: &[f64]| v.iter().filter(|&&x| x == 0.0).count();
            let n = input.shapes;
            scatter.push(format!(
                "held (RQ-471, RQ-355), not judged: equal-mass equilateral, L⁺ and L⁻ through a random gauge: max \
                 |w − 1| {:.3e}, w = 1 exactly in {} of {n}; z_β's clamp bit set in {}, at its clamp image in {}",
                worst(&held.pole).0,
                exact(&held.pole),
                held.beta_bit[0],
                held.beta_edge[0]
            ));
            scatter.push(format!(
                "held (RQ-471), not judged: collinear on a random line through a random gauge: max |w| {:.3e}, w = 0 \
                 exactly in {} of {n}; z_β's clamp bit set in {}, at its clamp image in {}",
                worst(&held.equator).0,
                exact(&held.equator),
                held.beta_bit[1],
                held.beta_edge[1]
            ));
        }
        let pass = v_gap == Verdict::Pass && v_com == Verdict::Pass;
        Ok(GateReport {
            gate: ROUNDTRIP.to_owned(),
            input: name.to_owned(),
            description: input.description.clone(),
            verdict: if pass { Verdict::Pass } else { Verdict::Fail },
            threshold: config.threshold.clone(),
            scatter,
            regions: regions(config, gaps.len())?,
            negative_results: negative,
        })
    }
}

impl Gate for InspectorRoundtripGate {
    fn name(&self) -> &'static str {
        ROUNDTRIP
    }

    fn run(
        &self,
        config: &GateConfig,
        name: &str,
        fixture: &serde_json::Value,
    ) -> Result<GateReport, String> {
        let input = RoundtripInput::deserialize(fixture)
            .map_err(|e| format!("refused: {name} is not an {ROUNDTRIP} input: {e}"))?;
        Self::judge(config, name, &input)
    }
}
