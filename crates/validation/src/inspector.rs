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
//! **The norm, and the ball applied meanwhile** (RQ-476). `‖·‖` on `z` is the Euclidean norm on ℝ⁸, as the corpus
//! writes the errors with double bars. Whether "`|z| < 4`" is the ball `‖z‖ < 4` or the cube `[−4, 4]⁸` is RQ-476's;
//! meanwhile the gates draw the ball, by rejection from the cube. Every control is inside `|zₖ| < 4` either way, so
//! `s(1 − s) ≥ 0.018`, far from the `10⁻⁶` clamps. Under the cube the Rust round trip is about twice the reference's
//! bound, driven by the mass block's `artanh` at `|z_μ| → 4` (RQ-476 and REQ-VAL-121's proposal give both).
//!
//! **The random ICs, applied meanwhile** (RQ-477; `inspector-handles`): `x = g₀·D(z)`, `z` in the ball, `g₀` a
//! random gauge element (`crate::gauge`) with `s₀` on `2^[−octaves, octaves)`, `θ` on `[−π, π)`, either parity, the
//! offset on `s₀·[−span, span)²` and the boost on `s₀^{−1/2}·[−span, span)²`, so the IC is a general physical state. Each
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
//! **The shape checks** (TASK-M2-50; R-551, RQ-475; R-549, RQ-355; RQ-624). Over RQ-475's families, equal-mass
//! equilateral triangles and collinear states, each through a random gauge element ([`shape_families`]): every collinear
//! state's `z_β` at its clamp image with its `lookup_clamped` bit set, and no equilateral state's set, bit for bit;
//! `max |w − 1|` at the pole and `max |w|` at the equator, read on the canonical state `C(x)`, within the bounds
//! [`POLE_BOUND`] and [`EQUATOR_BOUND`] proposed under REQ-VAL-121. On `D(E(x))` a collinear state's `|w|` is the `β`
//! clamp's image, `sin2α·sin(π·ε_z)`, by design: it is reported, not judged (RQ-624, decided per R-369). The controls
//! (R-176): `z_β` moved one ulp off its clamp image ([`RoundtripInput::beta_nudge`]); the collinear states moved off
//! the line inside the clamp's band ([`RoundtripInput::off_line`]), which only `|w|` on `C(x)` catches; and both bounds
//! one step below their own measurement ([`ShapeBounds::BelowMeasurement`]).
//!
//! **The Randomise arm** (REQ-VAL-193; R-551, RQ-477 (c); [`RandomiseArm`]). The Inspector's Randomise distribution
//! (render GUI spec §G8's Randomise: masses on `[0.4, 1.8]` normalised, positions on `±0.9`, velocities on `±0.6`,
//! [`randomise_ic`]) runs beside REQ-VAL-021's arm, each handle at a random size and at the tool's button sizes
//! (Rotate 15°, Scale ×1.25), its max `‖Δz‖` reported over the draws as stated and over its in-ball subset
//! `‖E(x)‖ < radius` beside them, per seed. Its verdict is separate: it is reported until REQ-VAL-121 records its
//! bound, and never read by REQ-VAL-021's verdict, except in its control ([`RandomiseArm::fed`]).

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

/// The pole's bound, `max |w − 1|` on `C(x)` over the equal-mass equilateral family: proposed under REQ-VAL-121 (R-71)
/// by TASK-M2-50 from the Rust measurement, provisional until the human confirms it at the M2 gate. It is `2⁻⁵¹`, two
/// ulps of 1, exactly twice the worst measured `2⁻⁵²`: just above 1, `|w − 1|` takes only multiples of `2⁻⁵²`.
pub const POLE_BOUND: f64 = 2.0 * f64::EPSILON;

/// The equator's bound, `max |w|` on `C(x)` over the collinear family: proposed under REQ-VAL-121 (R-71) by TASK-M2-50
/// from the Rust measurement, provisional until the human confirms it at the M2 gate.
pub const EQUATOR_BOUND: f64 = 2.5e-13;

const PI: f64 = std::f64::consts::PI;

/// `‖a − b‖`, the Euclidean norm on ℝ⁸.
pub fn z_gap(a: [f64; 8], b: [f64; 8]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// A `z` uniform in the ball `‖z‖ < radius`, by rejection from the cube `[−radius, radius)⁸`. `radius` is finite and
/// positive (each gate refuses another, [`radius_refusal`]); about 1.6 % of the cube's draws are kept.
pub fn ball(rng: &mut SplitMix, radius: f64) -> [f64; 8] {
    loop {
        let z: [f64; 8] = std::array::from_fn(|_| uniform(rng, -radius, radius));
        if z.iter().map(|x| x * x).sum::<f64>() < radius * radius {
            return z;
        }
    }
}

/// The refusal of a ball radius that is not finite and positive, on which [`ball`] would never return; `None` for one
/// that is.
pub fn radius_refusal(radius: f64) -> Option<String> {
    if radius.is_finite() && radius > 0.0 {
        None
    } else {
        Some(format!(
            "refused: the radius {radius} is not finite and positive"
        ))
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
    /// The Randomise arm, if the input runs it (REQ-VAL-193).
    #[serde(default)]
    pub randomise: Option<RandomiseArm>,
}

/// The Randomise arm of the handles gate (REQ-VAL-193).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomiseArm {
    /// The seeds, each drawn and reported on its own.
    pub seeds: Vec<u64>,
    /// The draws per seed.
    pub ics: u32,
    /// The control (R-176): the arm's maximum fed into REQ-VAL-021's verdict, which must then fail.
    #[serde(default)]
    pub fed: bool,
}

/// The Inspector's Rotate button, 15°.
pub const BUTTON_TURN: f64 = PI / 12.0;

/// The Inspector's Scale button, ×1.25.
pub const BUTTON_SCALE: f64 = 1.25;

/// The Randomise arm's moves, in [`randomise_moves`]' order: each handle at a random size, and the two buttons that
/// have a size. Translate-all (body-dragging) and boost-all ("Boost →", to which the corpus gives no size) are drawn
/// only.
pub const MOVES: [&str; 6] = [
    "translate-all, random",
    "boost-all, random",
    "rotate-all, random",
    "rotate-all at 15°",
    "scale-all, random",
    "scale-all at ×1.25",
];

/// The Inspector's Randomise: masses on `[0.4, 1.8]`, normalised so that `Σmᵢ = 1` (the last `1 − m₀ − m₁`),
/// positions on `±0.9`, velocities on `±0.6`, `pᵢ = mᵢvᵢ`; the CoM and the total momentum as drawn.
pub fn randomise_ic(rng: &mut SplitMix) -> Ic<f64> {
    let raw: [f64; 3] = std::array::from_fn(|_| uniform(rng, 0.4, 1.8));
    let total: f64 = raw.iter().sum();
    let (m0, m1) = (raw[0] / total, raw[1] / total);
    let m = [m0, m1, 1.0 - m0 - m1];
    let r: [[f64; 2]; 3] =
        std::array::from_fn(|_| [uniform(rng, -0.9, 0.9), uniform(rng, -0.9, 0.9)]);
    let v: [[f64; 2]; 3] =
        std::array::from_fn(|_| [uniform(rng, -0.6, 0.6), uniform(rng, -0.6, 0.6)]);
    let p = std::array::from_fn(|i| [m[i] * v[i][0], m[i] * v[i][1]]);
    Ic { m, r, p }
}

/// The six moves of [`MOVES`] on a Randomise IC, drawn on the canvas's own scales (applied per R-369): the offset on
/// `[−0.9, 0.9)²`, the boost on `[−0.6, 0.6)²` (the Randomise velocity scale), the turn on `[−π, π)`, the factor on
/// `2^[−octaves, octaves)`; and the buttons, 15° and ×1.25.
pub fn randomise_moves(rng: &mut SplitMix, octaves: f64) -> [Gauge; 6] {
    let id = Gauge::identity();
    let mut pair = |k: f64| [uniform(rng, -k, k), uniform(rng, -k, k)];
    let a = pair(0.9);
    let u = pair(0.6);
    let theta = uniform(rng, -PI, PI);
    let s = uniform(rng, -octaves, octaves).exp2();
    [
        Gauge { a, ..id },
        Gauge { u, ..id },
        Gauge { theta, ..id },
        Gauge {
            theta: BUTTON_TURN,
            ..id
        },
        Gauge { s, ..id },
        Gauge {
            s: BUTTON_SCALE,
            ..id
        },
    ]
}

/// One seed of the Randomise arm, measured under `E` with `leak`'s step dropped: per draw, `‖E(h·x) − E(x)‖` for each
/// move of [`MOVES`], and whether `x` is in the ball, `‖E(x)‖ < radius`.
#[derive(Clone, Debug, PartialEq)]
pub struct RandomiseSeed {
    /// The seed.
    pub seed: u64,
    /// Per draw, the six moves' `‖Δz‖`.
    pub gaps: Vec<[f64; 6]>,
    /// Per draw, whether `‖E(x)‖ < radius`.
    pub in_ball: Vec<bool>,
}

/// The Randomise arm's `ics` draws from `seed`, measured (see [`RandomiseSeed`]).
pub fn randomise_seed(seed: u64, ics: u32, radius: f64, octaves: f64, leak: Leak) -> RandomiseSeed {
    let mut rng = SplitMix::new(seed);
    let mut out = RandomiseSeed {
        seed,
        gaps: Vec::new(),
        in_ball: Vec::new(),
    };
    for _ in 0..ics {
        let x = randomise_ic(&mut rng);
        let moves = randomise_moves(&mut rng, octaves);
        let z0 = encode_leaky(&x, leak);
        out.in_ball.push(inside(z0, radius));
        out.gaps
            .push(moves.map(|g| z_gap(z0, encode_leaky(&g.act(&x), leak))));
    }
    out
}

/// Whether `z` is in the open ball `‖z‖ < radius`.
pub fn inside(z: [f64; 8], radius: f64) -> bool {
    z_gap(z, [0.0; 8]) < radius
}

/// The worst of move `k` over `seed`'s draws, all of them or the in-ball subset only; and how many draws that is.
pub fn randomise_worst(seed: &RandomiseSeed, k: usize, ball_only: bool) -> (f64, usize) {
    let v: Vec<f64> = seed
        .gaps
        .iter()
        .zip(&seed.in_ball)
        .filter(|(_, &inside)| inside || !ball_only)
        .map(|(g, _)| g[k])
        .collect();
    (worst(&v).0, v.len())
}

/// The Randomise arm's lines, one per move, each its maximum over all the draws and over the in-ball subset, overall
/// and by seed; and its worst over every move and draw.
fn randomise_lines(seeds: &[RandomiseSeed], radius: f64) -> (Vec<String>, f64) {
    let mut lines = Vec::new();
    let mut all_worst = Vec::new();
    for (k, word) in MOVES.iter().enumerate() {
        let per = |ball: bool| -> (f64, usize, String) {
            let each: Vec<(f64, usize)> =
                seeds.iter().map(|s| randomise_worst(s, k, ball)).collect();
            let w = worst(&each.iter().map(|e| e.0).collect::<Vec<_>>()).0;
            let n = each.iter().map(|e| e.1).sum();
            let by: Vec<String> = seeds
                .iter()
                .zip(&each)
                .map(|(s, e)| format!("{} {:.3e}", s.seed, e.0))
                .collect();
            (w, n, by.join(", "))
        };
        let (w, n, by) = per(false);
        let (wb, nb, byb) = per(true);
        all_worst.push(w);
        lines.push(format!(
            "Randomise arm (REQ-VAL-193), reported: {word}: max ‖Δz‖ {w:.3e} over {n} draws (by seed: {by}); in the \
             ball ‖E(x)‖ < {radius}, {nb} of {n}: max {wb:.3e} (by seed: {byb})"
        ));
    }
    (lines, worst(&all_worst).0)
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
        if input
            .randomise
            .as_ref()
            .is_some_and(|arm| arm.seeds.is_empty() || arm.ics == 0)
        {
            return Err("refused: a Randomise arm with no seeds or no draws".to_owned());
        }
        if let Some(refusal) = radius_refusal(input.radius) {
            return Err(refusal);
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
        if let Some(arm) = &input.randomise {
            let seeds: Vec<RandomiseSeed> = arm
                .seeds
                .iter()
                .map(|&s| randomise_seed(s, arm.ics, input.radius, input.octaves, input.leak))
                .collect();
            let (lines, arm_worst) = randomise_lines(&seeds, input.radius);
            scatter.extend(lines);
            if arm.fed {
                let line = format!(
                    "the Randomise arm's maximum, {arm_worst:.3e}, fed into REQ-VAL-021's verdict at {bound:.3e} \
                     (R-176: it must fail)"
                );
                if verdict(arm_worst, bound) == Verdict::Fail {
                    all = Verdict::Fail;
                }
                negative.push(line);
            } else {
                negative.push(format!(
                    "the Randomise arm: its verdict separate, reported until REQ-VAL-121 records its bound; its worst, \
                     {arm_worst:.3e}, is not read by REQ-VAL-021's verdict (R-551)"
                ));
            }
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
    /// The draws of each shape family, equilateral and collinear; 0 for none.
    pub shapes: u32,
    /// The in-clamp-band control (RQ-624): each collinear state's body 2 moved off the line by this many ulps of 1,
    /// before its gauge element; 0 for on the line.
    #[serde(default)]
    pub off_line: u32,
    /// The bit-for-bit control (RQ-624): each `z_β` moved one ulp off its encoded value before it is compared.
    #[serde(default)]
    pub beta_nudge: bool,
    /// The bounds the shape checks are judged against.
    #[serde(default)]
    pub bounds: ShapeBounds,
}

/// The bounds the shape checks are judged against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeBounds {
    /// [`POLE_BOUND`] and [`EQUATOR_BOUND`].
    #[default]
    Proposed,
    /// The control (R-176): each bound the largest float below its own measurement, so both checks must fail.
    BelowMeasurement,
}

/// The largest float below a measured `w ≥ 0`: one ulp down, or below 0 at 0.
pub fn below(w: f64) -> f64 {
    if w > 0.0 {
        f64::from_bits(w.to_bits() - 1)
    } else {
        -f64::from_bits(1)
    }
}

/// `(s, e)` with `s + e = a + b` exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// `‖Σ mᵢrᵢ‖`, each sum formed without rounding but the last (products by `fma`, sums by an exact two-sum), so the CoM
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

/// RQ-475's families, measured (see [`shape_families`]): `[equilateral, collinear]` in each count.
#[derive(Clone, Debug, PartialEq)]
pub struct Shapes {
    /// The equilateral states' `|w − 1|` on `C(x)`.
    pub pole: Vec<f64>,
    /// The collinear states' `|w|` on `C(x)`.
    pub equator: Vec<f64>,
    /// The collinear states' `|w|` on `D(E(x))`, the clamp image `sin2α·sin(π·ε_z)`: reported, not judged (RQ-624).
    pub clamp_image: Vec<f64>,
    /// How many of each family have `z_β`'s clamp bit set.
    pub beta_bit: [usize; 2],
    /// How many of each family have `z_β` at its clamp image bit for bit, `β ∈ {0, π̂}` clamped.
    pub beta_image: [usize; 2],
    /// The collinear states' masses.
    pub masses: Vec<[f64; 3]>,
}

/// `z_β`'s two clamp images under the defaults, `β = 0` and `β = π̂` through the β link's inverse.
pub fn beta_images() -> [f64; 2] {
    type BetaLink = <Defaults as Selection>::BetaLink;
    [0.0, pi_image::<f64>()].map(|b| BetaLink::inverse([b])[0])
}

/// RQ-475's families, `n` of each from `seed`: the equal-mass unit equilateral triangle, and three bodies on the
/// `x`-axis with masses from the mass block at `z_μ` on `[−4, 4)²`, body 2 moved off the line by `off_line` ulps of 1;
/// each at rest, then through a random gauge element (either parity, so `L⁺` and `L⁻` both). `w` is read through
/// `kernel::shape::shape` on `C(x)`, the canonical state `E` reads, and on `D(E(x))`. `z_β` is moved one ulp where
/// `beta_nudge`, then compared with its clamp images bit for bit. Neither `w` nor `z_β` reads the momenta.
pub fn shape_families(seed: u64, n: u32, off_line: u32, beta_nudge: bool) -> Shapes {
    let mut rng = SplitMix::new(seed);
    let images = beta_images();
    let equal = mass::decode::<Defaults, f64>([0.0, 0.0]).m;
    let triangle = [[0.0, 0.0], [1.0, 0.0], [0.5, 3f64.sqrt() / 2.0]];
    let lift = f64::from(off_line) * f64::EPSILON;
    let mut out = Shapes {
        pole: Vec::new(),
        equator: Vec::new(),
        clamp_image: Vec::new(),
        beta_bit: [0; 2],
        beta_image: [0; 2],
        masses: Vec::new(),
    };
    for _ in 0..n {
        let z_mu = [uniform(&mut rng, -4.0, 4.0), uniform(&mut rng, -4.0, 4.0)];
        let line: [f64; 3] = std::array::from_fn(|_| uniform(&mut rng, -1.0, 1.0));
        let mut r = line.map(|t| [t, 0.0]);
        r[2][1] = lift;
        let states = [
            Ic {
                m: equal,
                r: triangle,
                p: [[0.0; 2]; 3],
            },
            Ic {
                m: mass::decode::<Defaults, f64>(z_mu).m,
                r,
                p: [[0.0; 2]; 3],
            },
        ];
        for (k, state) in states.iter().enumerate() {
            let x = Gauge::random(&mut rng, 2.0, 4.0).act(state);
            let e = encode::<Defaults, f64>(&x);
            let w = shape(e.canonical.ic.r, x.m)[2];
            if k == 0 {
                out.pole.push((w - 1.0).abs());
            } else {
                out.equator.push(w.abs());
                let back = decode_z(e.z.z);
                out.clamp_image.push(shape(back.r, back.m)[2].abs());
                out.masses.push(x.m);
            }
            let mut z_beta = e.z.z[1];
            if beta_nudge {
                z_beta = f64::from_bits(z_beta.to_bits() + 1);
            }
            out.beta_bit[k] += usize::from(e.report.clamped_controls & 2 != 0);
            out.beta_image[k] +=
                usize::from(images.iter().any(|i| i.to_bits() == z_beta.to_bits()));
        }
    }
    out
}

/// The shape checks' verdict and lines on `input`'s families: the bit-for-bit `z_β` check, the pole's `|w − 1|` and the
/// equator's `|w|` on `C(x)` judged; `D(E(x))`'s `|w|` reported.
fn shape_checks(input: &RoundtripInput) -> (Verdict, Vec<String>, Vec<String>) {
    let s = shape_families(input.seed, input.shapes, input.off_line, input.beta_nudge);
    let n = input.shapes as usize;
    let scatter = vec![
        summary(
            "|w − 1| on C(x), equal-mass equilateral (L⁺ and L⁻)",
            &s.pole,
        ),
        summary("|w| on C(x), collinear", &s.equator),
        summary(
            "|w| on D(E(x)), collinear, reported, not judged: the clamp image, sin2α·sin(π·ε_z)",
            &s.clamp_image,
        ),
    ];
    let bits = if s.beta_bit == [0, n] && s.beta_image[1] == n {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    let (pole_bound, equator_bound) = match input.bounds {
        ShapeBounds::Proposed => (POLE_BOUND, EQUATOR_BOUND),
        ShapeBounds::BelowMeasurement => (below(worst(&s.pole).0), below(worst(&s.equator).0)),
    };
    let (pole, pole_line) = outcome("the pole's |w − 1| on C(x)", &s.pole, pole_bound);
    let (equator, equator_line) = outcome("the equator's |w| on C(x)", &s.equator, equator_bound);
    let mut negative = vec![
        format!(
            "z_β bit for bit: the clamp bit set in {} of {n} collinear and {} of {n} equilateral states; z_β at its \
             clamp image in {} of {n} collinear states; {}",
            s.beta_bit[1],
            s.beta_bit[0],
            s.beta_image[1],
            if bits == Verdict::Pass { "passes" } else { "fails" }
        ),
        pole_line,
        equator_line,
    ];
    negative.push(match input.bounds {
        ShapeBounds::Proposed => format!(
            "the shape bounds, |w − 1| ≤ {POLE_BOUND:e} and |w| ≤ {EQUATOR_BOUND:e}: proposed, provisional \
             (REQ-VAL-121, R-71)"
        ),
        ShapeBounds::BelowMeasurement => {
            "each shape bound one ulp below its own measurement (R-176: both must fail)".to_owned()
        }
    });
    if input.off_line > 0 {
        negative.push(format!(
            "every collinear state moved {} ulps of 1 off the line (R-176: the |w| check must fail it, z_β still \
             clamped)",
            input.off_line
        ));
    }
    if input.beta_nudge {
        negative.push(
            "every z_β moved one ulp off its encoded value (R-176: the bit-for-bit check must fail)".to_owned(),
        );
    }
    let all = [bits, pole, equator]
        .into_iter()
        .all(|v| v == Verdict::Pass);
    (
        if all { Verdict::Pass } else { Verdict::Fail },
        scatter,
        negative,
    )
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
        let control =
            input.off_line > 0 || input.beta_nudge || input.bounds != ShapeBounds::Proposed;
        if input.shapes == 0 && control {
            return Err("refused: a shape control with no shapes".to_owned());
        }
        if let Some(refusal) = radius_refusal(input.radius) {
            return Err(refusal);
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
        let mut v_shape = Verdict::Pass;
        if input.shapes > 0 {
            let (v, lines, negatives) = shape_checks(input);
            v_shape = v;
            scatter.extend(lines);
            negative.extend(negatives);
        }
        let pass = v_gap == Verdict::Pass && v_com == Verdict::Pass && v_shape == Verdict::Pass;
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
