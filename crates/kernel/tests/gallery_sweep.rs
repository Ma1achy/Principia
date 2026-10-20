//! The gallery sweep (R-467 clause 7, D11 and F5 as revised on physics review 5585225466 of PR #253; REQ-CHART-058):
//! every chart `kernel::chart` ships, at the parameter sets TASK-M2-35 names, swept over a 257 × 257 grid on `[0,1]²`,
//! `u = i/256` and `v = j/256` for `i, j ∈ 0..=256`, its edges and corners included, through the kernel's own chart
//! map, `validate` and decode, the code the GPU kernel monomorphises (R-313), run on the CPU at f64 and at the f32
//! instantiation (the chart held at f32 and mapped and decoded at f32; `validate` is the CPU-side f64 check, R-26).
//!
//! **What a pixel is counted as** (chart_decoder_contract Part 3 and Part 5; inverse_encode_contract § "Chart-aware
//! validation"; R-446, R-467 F5):
//! - **forbidden**: `validate(u, v)` is not `Valid`, a failure of layer 2 or layer 3. `Chart::map` returns a value at
//!   every point, so a map that fails is a panic, which fails the test, or a non-finite output, counted next;
//! - **non-finite**: a chart output `z` or a decoded `(m, r, p)` component that is not finite;
//! - **`INVARIANT_INFEASIBLE`**: the construction's (chart_reference §2.2) `Outcome::Infeasible`, on a chart whose
//!   decode runs it: `(L_z, E)`, `(L_z, K)`, `(ν, K)`, `Φ_θ,K`, `Φ_θ,L_z` and the `K` strip. Asserted 0 (F5 as revised);
//! - **the other tags**, printed only (R-446: a tag never marks a pixel forbidden): the decode's `DEGENERATE` reason
//!   (`Decoded::degenerate`) and the construction's seed-rule `DEGENERATE` (`Outcome::Degenerate`);
//! - **collision poles**, printed only: the grid points whose decoded configuration has an exact binary collision,
//!   `r_min_pair_0 = 0`, where `V₀` and `E₀` are R-436's defined `−∞`, a `t = 0` collision and a system (R-13, R-60),
//!   outside the non-finite count (physics review P5). The count of non-finite `V₀` is printed beside it.
//!
//! **The parameter sets** (no chart has a `Default`, physics review P4): the register's value where it holds one, read
//! from it (`validation`'s re-export of `ledger::constants`), and where that value is a pending calibration the one the
//! chart ships today, each said so at its use. One set per chart sits at the edge of the admissible space (R-467 D11):
//! `γ_K = 1`, `λ_L = 1`, a mixed-axis range at its link's closed codomain edge.
//!
//! **The control arm** (R-467 D11; pitfalls §9, a check whose reachable output set holds the failure it guards
//! against): an absolute-energy `(L_z, E)` test chart, whose forbidden set the sweep reports exactly and which fails
//! the gallery check; a warp with `L_max` one ulp too large, which fires `INVARIANT_INFEASIBLE` on the `s ∈ {0, 1}`
//! columns; and a test chart non-finite on one column, which the sweep catches as non-finite.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory. Bounded by the grid: every chart
//! runs 257² points at each precision, and nothing loops on data.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use kernel::chart::burrau::{self as nu_axis};
use kernel::chart::burrau_acute::{decode_theta_k, decode_theta_lz};
use kernel::chart::invariant::{self, Invariant, InvariantBlocks, WarpReal};
use kernel::chart::mixed::{
    Alpha, Beta, PLambdaX, PLambdaY, PRhoX, PRhoY, Quantity, ZAlpha, ZBeta, ZMu1, ZMu2, ZQ0, ZQ1,
    ZQ2, ZQ3,
};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, ChartOut,
    Constraint, EuclidPlane, Float, InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap,
    ShapeSphere, ValidationResult,
};
use kernel::decode::invariant::{infeasible, Outcome};
use kernel::decode::{self, Decoded};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use spirv_std::num_traits::FloatConst;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

/// The grid's divisions per axis: 257 points, `0, 1/256, …, 1`, each exact at f32 and f64.
const N: u32 = 256;

/// The sweep's worker threads, at most (R-228's four).
const WORKERS: usize = 4;

/// The `γ_K` of the edge set: the least admissible exponent (chart_reference §2.1, "`γ_K ≥ 1`").
const GAMMA_K_EDGE: f64 = 1.0;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the charts are instantiated at.
trait Q: LinkReal + PayloadReal + WarpReal + FloatConst + Send + Sync + 'static {
    const LABEL: &'static str;
    /// `x` rounded to this precision.
    fn of(x: f64) -> Self;
    /// `self` widened to f64.
    fn f(self) -> f64;
    /// The next float above `self`.
    fn up(self) -> Self;
    /// The next float below `self`.
    fn down(self) -> Self;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn up(self) -> Self {
        self.next_up()
    }
    fn down(self) -> Self {
        self.next_down()
    }
}

/// The grid coordinate `k/256`, exact at either precision.
fn grid<R: Q>(k: u32) -> R {
    R::of(k as f64 / N as f64)
}

/// `[lo, hi]` at `R`, each end rounded toward the inside, so that a range reaching a closed edge at f64 reaches the
/// last float inside it at f32 (as the register's `θ_max` is π/4 rounded down to f32).
fn inward<R: Q>(range: [f64; 2]) -> [R; 2] {
    let mut lo = R::of(range[0]);
    if lo.f() < range[0] {
        lo = lo.up();
    }
    let mut hi = R::of(range[1]);
    if hi.f() > range[1] {
        hi = hi.down();
    }
    [lo, hi]
}

/// The seeded draws (TASK-M2-35's "one seeded draw"): SplitMix64 from a fixed seed, uniform on `[−1, 1]`, drawn at
/// f64 and narrowed, so that the f64 and f32 charts hold one draw.
struct Draw(u64);

impl Draw {
    fn new(seed: u64) -> Self {
        Draw(seed)
    }

    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }

    fn take<const K: usize>(&mut self) -> [f64; K] {
        std::array::from_fn(|_| self.next())
    }
}

/// The draw's seeds, one per family, fixed.
const SEED_LATENT: u64 = 0x5EED_0001;
const SEED_SHAPE: u64 = 0x5EED_0002;
const SEED_INVARIANT: u64 = 0x5EED_0003;

// ── The sweep ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A grid point, `[i, j]`: `(u, v) = (i/256, j/256)`.
type Point = [u32; 2];

/// What the sweep counted on one chart at one precision.
#[derive(Clone, Debug, Default, PartialEq)]
struct Tally {
    /// The points `validate` does not find `Valid`.
    forbidden: Vec<Point>,
    /// The points whose chart output `z` has a non-finite component.
    z_nonfinite: Vec<Point>,
    /// The points whose decoded `(m, r, p)` has a non-finite component.
    state_nonfinite: Vec<Point>,
    /// Whether the chart's decode runs §2.2's construction, so that it has an `Outcome`.
    construction: bool,
    /// The points the construction found infeasible, `INVARIANT_INFEASIBLE`.
    infeasible: Vec<Point>,
    /// The points the construction's seed rule found `DEGENERATE` (R-82).
    seed_degenerate: u32,
    /// The decode's `DEGENERATE` reasons, by name, and their counts.
    tags: BTreeMap<String, u32>,
    /// The points with an exact binary collision, `r_min_pair_0 = 0`.
    collisions: u32,
    /// The points whose `V₀` is not finite.
    v0_nonfinite: u32,
}

/// A chart's `validate`, or a stand-in for it (the control arm's sweep that accepts every pixel).
type Validate<C> = fn(&C, f64, f64) -> ValidationResult;

/// A chart's decode at `R`, with the construction's outcome where it runs one.
type Decode<C, R> = fn(&C, R, R) -> (Decoded<R>, Option<Outcome>);

/// Whether every component is finite.
fn finite<R: Q>(xs: impl IntoIterator<Item = R>) -> bool {
    xs.into_iter().all(Float::is_finite)
}

/// Sweeps `chart` over the grid: its map, `validate` and `decode` at every point, row by row from the bottom.
fn sweep<R: Q, C: Chart<Param = R>>(
    chart: &C,
    validate: Validate<C>,
    decode: impl Fn(&C, R, R) -> (Decoded<R>, Option<Outcome>),
) -> Tally {
    let mut t = Tally::default();
    for j in 0..=N {
        for i in 0..=N {
            let point = [i, j];
            let (u, v) = (grid::<R>(i), grid::<R>(j));
            let ChartOut { z } = chart.map(u, v);
            if !finite(z.z) {
                t.z_nonfinite.push(point);
            }
            if validate(chart, u.f(), v.f()) != ValidationResult::Valid {
                t.forbidden.push(point);
            }
            let (d, outcome) = decode(chart, u, v);
            let ic = d.ic;
            if !finite(
                ic.m.into_iter()
                    .chain(ic.r.into_iter().flatten())
                    .chain(ic.p.into_iter().flatten()),
            ) {
                t.state_nonfinite.push(point);
            }
            if let Some(outcome) = outcome {
                t.construction = true;
                match outcome {
                    Outcome::Infeasible => t.infeasible.push(point),
                    Outcome::Degenerate => t.seed_degenerate += 1,
                    Outcome::Feasible => {}
                }
            }
            if let Some(reason) = d.degenerate {
                *t.tags.entry(format!("{reason:?}")).or_default() += 1;
            }
            if d.descriptor.r_min_pair_0 == R::zero() {
                t.collisions += 1;
            }
            if !d.descriptor.V_0.is_finite() {
                t.v0_nonfinite += 1;
            }
        }
    }
    t
}

/// One chart's sweep at one parameter set and precision.
#[derive(Clone, Debug)]
struct Report {
    /// The gallery's name for the chart (R-467 D11's list).
    chart: &'static str,
    /// The chart's `Chart::name`.
    name: String,
    /// The parameter set, in words.
    set: String,
    /// `f64` or `f32`.
    precision: &'static str,
    tally: Tally,
}

impl Report {
    /// The printed line: the counts the sweep asserts, then those it prints only.
    fn line(&self) -> String {
        let t = &self.tally;
        let mut s = format!(
            "{} [{}] {} | {}: forbidden {}, non-finite z {}, non-finite (m, r, p) {}",
            self.chart,
            self.name,
            self.set,
            self.precision,
            t.forbidden.len(),
            t.z_nonfinite.len(),
            t.state_nonfinite.len()
        );
        if t.construction {
            let _ = write!(
                s,
                ", INVARIANT_INFEASIBLE {}, seed DEGENERATE {}",
                t.infeasible.len(),
                t.seed_degenerate
            );
        }
        let tags: Vec<String> = t.tags.iter().map(|(k, n)| format!("{k} {n}")).collect();
        let _ = write!(
            s,
            " | tags: {} | collision poles {} (V₀ non-finite {})",
            if tags.is_empty() {
                "none".to_string()
            } else {
                tags.join(", ")
            },
            t.collisions,
            t.v0_nonfinite
        );
        s
    }
}

/// The first point of a list, for a fault's message.
fn first(points: &[Point]) -> String {
    points.first().map_or(String::new(), |[i, j]| {
        format!(", first at (u, v) = ({i}/256, {j}/256)")
    })
}

/// What the gallery check finds wrong with one report: forbidden pixels, non-finite outputs or states, and
/// `INVARIANT_INFEASIBLE` on a chart whose decode runs the construction (R-467 D11, F5).
fn faults(r: &Report) -> Vec<String> {
    let t = &r.tally;
    let at = format!("{} {} at {}", r.chart, r.set, r.precision);
    let mut out = Vec::new();
    if !t.forbidden.is_empty() {
        out.push(format!(
            "{at}: {} forbidden pixels{}",
            t.forbidden.len(),
            first(&t.forbidden)
        ));
    }
    if !t.z_nonfinite.is_empty() {
        out.push(format!(
            "{at}: {} non-finite chart outputs z{}",
            t.z_nonfinite.len(),
            first(&t.z_nonfinite)
        ));
    }
    if !t.state_nonfinite.is_empty() {
        out.push(format!(
            "{at}: {} non-finite decoded (m, r, p){}",
            t.state_nonfinite.len(),
            first(&t.state_nonfinite)
        ));
    }
    if !t.infeasible.is_empty() {
        out.push(format!(
            "{at}: {} INVARIANT_INFEASIBLE pixels{}",
            t.infeasible.len(),
            first(&t.infeasible)
        ));
    }
    out
}

/// One sweep to run: a chart at a parameter set and precision.
type Job = Box<dyn Fn() -> Report + Send + Sync>;

/// The job sweeping `chart` with its own `validate` and `decode`.
fn job<R: Q, C: Chart<Param = R> + Send + Sync + 'static>(
    gallery_name: &'static str,
    set: String,
    chart: C,
    decode: Decode<C, R>,
) -> Job {
    Box::new(move || Report {
        chart: gallery_name,
        name: chart.name().to_string(),
        set: set.clone(),
        precision: R::LABEL,
        tally: sweep(&chart, C::validate, decode),
    })
}

/// Runs the jobs on up to [`WORKERS`] threads, the reports in the jobs' order.
fn run(jobs: Vec<Job>) -> Vec<Report> {
    let next = AtomicUsize::new(0);
    let reports = Mutex::new(vec![None; jobs.len()]);
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(WORKERS);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(k) else { break };
                let report = job();
                reports.lock().unwrap()[k] = Some(report);
            });
        }
    });
    reports
        .into_inner()
        .unwrap()
        .into_iter()
        .map(|r| r.expect("every job ran"))
        .collect()
}

/// The decode stage's own decode ([`ChartDecode::decode_at`], `kernel::stage::decode::sample`'s), no construction.
fn staged<R: Q, C: ChartDecode<Param = R>>(chart: &C, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    (chart.decode_at::<Defaults, R>(u, v), None)
}

// ── The gallery: every chart `kernel::chart` ships, at its parameter sets ─────────────────────────────────────────

/// The unit vector `ê_k` in the decoder's block layout.
fn axis(k: usize) -> [f64; 8] {
    std::array::from_fn(|i| if i == k { 1.0 } else { 0.0 })
}

/// An orthonormal pair `(q₁, q₂)` from two draws, by Gram–Schmidt: a tilted plane, no coordinate axis.
fn tilted(draw: &mut Draw) -> ([f64; 8], [f64; 8]) {
    let norm = |x: &[f64; 8]| x.iter().map(|c| c * c).sum::<f64>().sqrt();
    let a: [f64; 8] = draw.take();
    let q1 = a.map(|c| c / norm(&a));
    let b: [f64; 8] = draw.take();
    let dot: f64 = b.iter().zip(&q1).map(|(x, y)| x * y).sum();
    let c: [f64; 8] = std::array::from_fn(|i| b[i] - dot * q1[i]);
    (q1, c.map(|x| x / norm(&c)))
}

/// The latent chart and the flat slices: `z₀ = 0` with `q₁`, `q₂` every pair of coordinate axes `(ê_a, ê_b)`, `a < b`,
/// and one seeded `z₀` on `[−1, 1]⁸` with a tilted orthonormal pair.
fn latent_jobs<R: Q>(jobs: &mut Vec<Job>) {
    let at = |x: [f64; 8]| x.map(R::of);
    for a in 0..8 {
        for b in a + 1..8 {
            let chart = Latent {
                z0: [R::zero(); 8],
                q1: at(axis(a)),
                q2: at(axis(b)),
            };
            jobs.push(job(
                "flat slice",
                format!("z₀ = 0, q₁ = ê_{a}, q₂ = ê_{b}"),
                chart,
                staged,
            ));
        }
    }
    let mut draw = Draw::new(SEED_LATENT);
    let z0: [f64; 8] = draw.take();
    let (q1, q2) = tilted(&mut draw);
    let chart = Latent {
        z0: at(z0),
        q1: at(q1),
        q2: at(q2),
    };
    jobs.push(job(
        "latent chart",
        "seeded z₀, tilted orthonormal (q₁, q₂)".into(),
        chart,
        staged,
    ));
}

/// The shape sphere's decode, its own [`Chart::decode`] (the chart sets the configuration explicitly), no construction.
fn shape_decode<R: Q>(chart: &ShapeSphere<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    (Chart::decode::<Defaults, R>(chart, u, v), None)
}

/// The shape sphere: `I = 1`, `φ_f = 0`, `z₀`'s mass and momentum blocks 0 and one seeded draw on `[−1, 1]⁶`; the
/// spherical map, and the exponential map about `n₀ = (0, 0, 1)`, `e₁ = (1, 0, 0)`, `e₂ = (0, 1, 0)` at `s = π/4` and
/// `s = π/2`.
fn shape_jobs<R: Q>(jobs: &mut Vec<Job>) {
    let mut draw = Draw::new(SEED_SHAPE);
    let seeded: [f64; 6] = draw.take();
    let held = [("held blocks 0", [0.0; 6]), ("held blocks seeded", seeded)];
    let maps = [
        (
            "spherical map",
            ShapeMap::Spherical,
            0.0,
            "shape sphere, spherical",
        ),
        (
            "exponential map, s = π/4",
            ShapeMap::Exponential,
            FRAC_PI_4,
            "shape sphere, exponential",
        ),
        (
            "exponential map, s = π/2",
            ShapeMap::Exponential,
            FRAC_PI_2,
            "shape sphere, exponential",
        ),
    ];
    for (map_name, map, scale, gallery_name) in maps {
        for (held_name, h) in held {
            let chart = ShapeSphere {
                map,
                n0: [R::zero(), R::zero(), R::one()],
                e1: [R::one(), R::zero(), R::zero()],
                e2: [R::zero(), R::one(), R::zero()],
                scale: R::of(scale),
                inertia: R::one(),
                phase: R::zero(),
                z_mass: [R::of(h[0]), R::of(h[1])],
                z_mom: [R::of(h[2]), R::of(h[3]), R::of(h[4]), R::of(h[5])],
            };
            jobs.push(job(
                gallery_name,
                format!("{map_name}, I = 1, φ_f = 0, {held_name}"),
                chart,
                shape_decode,
            ));
        }
    }
}

/// `(L_z, E)`'s decode, [`invariant::decode`], with the construction's outcome.
fn le_decode<R: Q>(chart: &InvariantLE<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    let (d, o) = invariant::decode::<Defaults, R, _>(chart, u, v);
    (d, Some(o))
}

/// `(L_z, K)`'s decode, [`invariant::decode`], with the construction's outcome.
fn lk_decode<R: Q>(chart: &InvariantLK<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    let (d, o) = invariant::decode::<Defaults, R, _>(chart, u, v);
    (d, Some(o))
}

/// The warp's parameter sets: `K_max` and `γ_K` the register's (REQ-CHART-044, proposed, provisional until the M2
/// gate: the values the charts ship today, 1 and 2), and the edge set `γ_K = 1`.
fn warp_sets() -> [(f64, f64, &'static str); 2] {
    [
        (
            K_MAX.number(),
            GAMMA_K.number(),
            "K_max, γ_K the register's (REQ-CHART-044, proposed)",
        ),
        (
            K_MAX.number(),
            GAMMA_K_EDGE,
            "K_max the register's, γ_K = 1 (edge set)",
        ),
    ]
}

/// The invariant charts: each warp set, with `z₀`'s configuration and mass controls at 0 and at one seeded draw on
/// `[−1, 1]⁴`.
fn invariant_jobs<R: Q>(jobs: &mut Vec<Job>) {
    let mut draw = Draw::new(SEED_INVARIANT);
    let seeded: [f64; 4] = draw.take();
    for (k_max, gamma_k, warp_name) in warp_sets() {
        for (held_name, h) in [
            ("z_cfg = z_mass = 0", [0.0; 4]),
            ("z_cfg, z_mass seeded", seeded),
        ] {
            let inv = Invariant {
                z_cfg: [R::of(h[0]), R::of(h[1])],
                z_mass: [R::of(h[2]), R::of(h[3])],
                k_max: R::of(k_max),
                gamma_k: R::of(gamma_k),
            };
            let set = format!("{warp_name}, {held_name}");
            jobs.push(job("(L_z, E)", set.clone(), InvariantLE(inv), le_decode));
            jobs.push(job("(L_z, K)", set, InvariantLK(inv), lk_decode));
        }
    }
}

/// The `(ν, K)` chart's decode, [`nu_axis::decode`], with the construction's outcome.
fn nu_k_decode<R: Q>(chart: &Burrau<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    let (d, o) = nu_axis::decode(chart, u, v);
    (d, Some(o))
}

/// `Φ_θ,K`'s decode, [`decode_theta_k`], with the construction's outcome.
fn theta_k_decode<R: Q>(chart: &BurrauThetaK<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>)
where
    f64: From<R>,
{
    let (d, o) = decode_theta_k(chart, u, v);
    (d, Some(o))
}

/// `Φ_θ,L_z`'s decode, [`decode_theta_lz`], with the construction's outcome.
fn theta_lz_decode<R: Q>(chart: &BurrauThetaLz<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>)
where
    f64: From<R>,
{
    let (d, o) = decode_theta_lz(chart, u, v);
    (d, Some(o))
}

/// The register's proposed acute-angle range (REQ-CHART-045, proposed, provisional until the M2 gate: the range the
/// charts ship today), each end an f32, so both precisions hold the same numbers.
fn theta_range<R: Q>() -> [R; 2] {
    [
        R::of(BURRAU_THETA_MIN.number()),
        R::of(BURRAU_THETA_MAX.number()),
    ]
}

/// The register's Euclid-plane `ν` range, `[1/32, 31/32]` (chart_reference §4.5).
fn nu_range<R: Q>() -> [R; 2] {
    [R::of(EUCLID_NU_MIN.number()), R::of(EUCLID_NU_MAX.number())]
}

/// The mass simplex's `ν₀` (TASK-M2-35): the classical triangle.
const MASS_SIMPLEX_NU0: f64 = 0.5;

/// The Burrau family and the mass simplex: the Euclid plane on the register's `m ∈ [1, 32]`, `ν ∈ [1/32, 31/32]`;
/// `(ν, K)` on the same `ν` range at each warp set; `Φ_θ,K` and the strip `Φ_strip,K` (one map, chart_reference §4.5)
/// at each warp set; `Φ_θ,L_z` at the register's `K = 1/8` and `λ_L = 1` (REQ-CHART-046, proposed, provisional until
/// the M2 gate: the values the chart ships today; `λ_L = 1` is also the edge set); `Φ_θ,δm` and the strip `Φ_strip,δm`
/// (one map) at the equal-mass target; the mass simplex at the register's `ε_m = 10⁻⁴` and `ν₀ = 1/2`.
fn burrau_jobs<R: Q>(jobs: &mut Vec<Job>)
where
    f64: From<R>,
{
    let plane = EuclidPlane {
        m_range: [R::of(EUCLID_M_MIN.number()), R::of(EUCLID_M_MAX.number())],
        nu_range: nu_range(),
    };
    jobs.push(job(
        "Euclid plane",
        "m, ν ranges the register's".into(),
        plane,
        staged,
    ));
    for (k_max, gamma_k, warp_name) in warp_sets() {
        let nu_k = Burrau {
            nu_range: nu_range(),
            k_max: R::of(k_max),
            gamma_k: R::of(gamma_k),
        };
        jobs.push(job(
            "(ν, K)",
            format!("ν range the Euclid plane's, {warp_name}"),
            nu_k,
            nu_k_decode,
        ));
        let theta_k = BurrauThetaK {
            theta_range: theta_range(),
            k_max: R::of(k_max),
            gamma_k: R::of(gamma_k),
        };
        let set = format!("θ range the register's (REQ-CHART-045, proposed), {warp_name}");
        jobs.push(job("Φ_θ,K", set.clone(), theta_k, theta_k_decode));
        jobs.push(job("Φ_strip,K", set, theta_k, theta_k_decode));
    }
    let theta_lz = BurrauThetaLz {
        theta_range: theta_range(),
        k: R::of(BURRAU_THETA_LZ_K.number()),
        lz_fraction: R::of(BURRAU_THETA_LZ_FRACTION.number()),
    };
    let set = "θ range, K and λ_L the register's (REQ-CHART-045, REQ-CHART-046, proposed; λ_L = 1 the edge set)";
    jobs.push(job("Φ_θ,L_z", set.into(), theta_lz, theta_lz_decode));
    let delta_m = BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<R>::equal_masses(),
    };
    let set = "θ range the register's (REQ-CHART-045, proposed), equal-mass target";
    jobs.push(job("Φ_θ,δm", set.into(), delta_m, staged));
    jobs.push(job("Φ_strip,δm", set.into(), delta_m, staged));
    let simplex = MassSimplex {
        nu0: R::of(MASS_SIMPLEX_NU0),
        eps_m: R::of(MASS_SIMPLEX_EPS_M.number()),
    };
    jobs.push(job(
        "mass simplex",
        "ε_m the register's, ν₀ = 1/2".into(),
        simplex,
        staged,
    ));
}

/// A mixed-axis axis's range at a set: inside its codomain (a latent coordinate `[−1, 1]`, a derived quantity the
/// middle half of its link's codomain), or at the edge set reaching it (a derived quantity the whole closed codomain,
/// `α ∈ [0, π/2]`, `β ∈ [0, π]`, `p ∈ [−q_max, q_max]`; a latent coordinate, whose codomain is the line, `[−8, 8]`,
/// where every link saturates), rounded inward at f32.
fn mixed_range<A: Quantity, R: Q>(edge: bool) -> [R; 2] {
    let [lo, hi] = A::DOMAIN;
    let range = match (A::DERIVED, edge) {
        (false, false) => [-1.0, 1.0],
        (false, true) => [-8.0, 8.0],
        (true, false) => [lo + (hi - lo) / 4.0, hi - (hi - lo) / 4.0],
        (true, true) => [lo, hi],
    };
    inward(range)
}

/// The mixed-axis chart `Mixed<H, V>` at both range sets, the other six controls frozen at 0.
fn mixed_pair<H, V, R>(jobs: &mut Vec<Job>)
where
    H: Quantity + Send + Sync + 'static,
    V: Quantity + Send + Sync + 'static,
    R: Q,
    f64: From<R>,
{
    for (edge, set_name) in [
        (false, "inside the codomains"),
        (true, "at the closed codomain edges"),
    ] {
        let chart = Mixed::<H, V, R>::new(
            [R::zero(); 8],
            mixed_range::<H, R>(edge),
            mixed_range::<V, R>(edge),
        );
        let set = format!(
            "({}, {}), frozen z = 0, ranges {set_name}",
            H::NAME,
            V::NAME
        );
        jobs.push(job("mixed-axis", set, chart, staged));
    }
}

/// Every unordered pair of the module's axes that set two different coordinates, as `Mixed<H, V>`, the axes grouped
/// by the coordinate they set (a latent coordinate and the derived quantity its link gives share one): every pair of
/// groups, every member of the one with every member of the other. The transposed chart `Mixed<V, H>` covers the same
/// `z` set over the same grid.
macro_rules! mixed_pairs {
    ($jobs:expr, $r:ty; $head:tt $($tail:tt)*) => {
        $( mixed_pairs!(@cross $jobs, $r; $head $tail); )*
        mixed_pairs!($jobs, $r; $($tail)*);
    };
    ($jobs:expr, $r:ty;) => {};
    (@cross $jobs:expr, $r:ty; [$($h:ident)*] $v:tt) => {
        $( mixed_pairs!(@row $jobs, $r; $h $v); )*
    };
    (@row $jobs:expr, $r:ty; $h:ident [$($v:ident)*]) => {
        $( mixed_pair::<$h, $v, $r>($jobs); )*
    };
}

/// The mixed-axis charts: every pair of the module's fourteen axes, the eight latent coordinates and the six derived
/// quantities, that set two coordinates (R-467 D11's "each pair the module defines"): 85 charts.
fn mixed_jobs<R: Q>(jobs: &mut Vec<Job>)
where
    f64: From<R>,
{
    mixed_pairs!(jobs, R;
        [ZAlpha Alpha] [ZBeta Beta] [ZQ0 PRhoX] [ZQ1 PRhoY] [ZQ2 PLambdaX] [ZQ3 PLambdaY] [ZMu1] [ZMu2]);
}

/// Every chart of the gallery at every parameter set, at `R`.
fn gallery_at<R: Q>(jobs: &mut Vec<Job>)
where
    f64: From<R>,
{
    latent_jobs::<R>(jobs);
    shape_jobs::<R>(jobs);
    invariant_jobs::<R>(jobs);
    burrau_jobs::<R>(jobs);
    mixed_jobs::<R>(jobs);
}

/// The gallery at f64 and at f32.
fn gallery() -> Vec<Job> {
    let mut jobs = Vec::new();
    gallery_at::<f64>(&mut jobs);
    gallery_at::<f32>(&mut jobs);
    jobs
}

/// Every `Chart` implementation `kernel::chart` ships, by its `Chart::name`; BodyPlane, not built at M2, is
/// REQ-CHART-059's (TASK-M3-36), and a Chart-builder chart RQ-372's.
const SHIPPED: [&str; 11] = [
    "Latent",
    "ShapeSphere",
    "InvariantLE",
    "InvariantLK",
    "MassSimplex",
    "EuclidPlane",
    "Burrau",
    "BurrauThetaK",
    "BurrauThetaLz",
    "BurrauThetaDeltaM",
    "Mixed",
];

/// The gallery check (R-467 D11): sweeps every job, prints each report's line, and asserts that no report has a fault.
fn check_gallery(jobs: Vec<Job>) -> Vec<Report> {
    let reports = run(jobs);
    for r in &reports {
        println!("{}", r.line());
    }
    let faults: Vec<String> = reports.iter().flat_map(faults).collect();
    assert!(
        faults.is_empty(),
        "the gallery sweep found {} faults:\n{}",
        faults.len(),
        faults.join("\n")
    );
    reports
}

#[test]
fn gallery_sweep_unit_square() {
    let reports = check_gallery(gallery());
    for name in SHIPPED {
        for precision in ["f64", "f32"] {
            assert!(
                reports
                    .iter()
                    .any(|r| r.name == name && r.precision == precision),
                "the gallery does not sweep {name} at {precision}"
            );
        }
    }
    println!(
        "{} sweeps of {} points each",
        reports.len(),
        (N + 1) * (N + 1)
    );
}

negative_control!(
    gallery_sweep_unit_square,
    "the absolute-energy (L_z, E) control chart, swept as a gallery chart, must fail the gallery check",
    expected = "forbidden pixels",
    {
        check_gallery(vec![job("(L_z, E) absolute", "the control's".into(), absolute::<f64>(), absolute_decode)]);
    }
);

// ── The control arm ─────────────────────────────────────────────────────────────────────────────────────────────

/// The control arm's absolute-energy `(L_z, E)` chart (R-467 D11): the configuration and mass controls held, as on
/// the invariant charts, and both axes absolute, `L_z = L_lo + s·(L_hi − L_lo)` and `E = E_lo + t·(E_hi − E_lo)` over
/// fixed ranges, not relative to the slice's reach as chart_reference §2.1's warp is. The slice's potential `U` lies
/// inside the energy range, so the chart holds both `E < U` rows and, above `U`, pixels outside the parabola.
#[derive(Clone, Copy, Debug)]
struct AbsoluteLE<P> {
    /// The configuration controls `(z_α, z_β)`.
    z_cfg: [P; 2],
    /// The mass controls `(z_μ₁, z_μ₂)`.
    z_mass: [P; 2],
    /// `[L_lo, L_hi]`.
    lz_range: [P; 2],
    /// `[E_lo, E_hi]`.
    e_range: [P; 2],
}

/// The control chart's ranges: `L_z ∈ [−2, 2]`, `E ∈ [−1, 1]`, on the slice `z_cfg = z_mass = 0`, whose `U` lies inside
/// `(−1, 1)` (checked where the control runs).
fn absolute<R: Q>() -> AbsoluteLE<R> {
    AbsoluteLE {
        z_cfg: [R::zero(); 2],
        z_mass: [R::zero(); 2],
        lz_range: [R::of(-2.0), R::of(2.0)],
        e_range: [R::of(-1.0), R::of(1.0)],
    }
}

impl<P: Copy> AbsoluteLE<P> {
    /// The held controls, the momentum controls 0 and unread, as [`Invariant::frozen`] holds them.
    fn frozen<F: Float + From<P>>(&self) -> decode::Latent<F> {
        let w = <F as From<P>>::from;
        let o = F::zero();
        decode::Latent::new([
            w(self.z_cfg[0]),
            w(self.z_cfg[1]),
            o,
            o,
            o,
            o,
            w(self.z_mass[0]),
            w(self.z_mass[1]),
        ])
    }

    /// `(L_z, E)` at `(s, t)`.
    fn at<F: Float + From<P>>(&self, s: F, t: F) -> (F, F) {
        let w = <F as From<P>>::from;
        let lerp = |r: [P; 2], x: F| w(r[0]) + (w(r[1]) - w(r[0])) * x;
        (lerp(self.lz_range, s), lerp(self.e_range, t))
    }

    /// The held configuration's potential `U`, the decode's `V₀` (it reads the positions and masses alone).
    fn potential<F: Q + From<P>>(&self) -> F {
        decode::decode::<Defaults, F>(&self.frozen()).descriptor.V_0
    }
}

impl<P: Q> Chart for AbsoluteLE<P>
where
    f64: From<P>,
{
    type Param = P;

    fn map<F: Float + From<P>>(&self, _u: F, _v: F) -> ChartOut<F> {
        ChartOut { z: self.frozen() }
    }

    fn name(&self) -> &str {
        "AbsoluteLE (control)"
    }

    /// Layer 2's parabola `|L_z| ≤ √(2I(E − U))`, `I = 1` (inverse_encode_contract § "Chart-aware validation"), decided
    /// by the construction's exact test `2·I·K < L_z²` at `K = E − U`, which a negative `K`, `E < U`, fails too.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        let (lz, e) = self.at(u, v);
        let k = e - self.potential::<f64>();
        if !(lz.is_finite() && k.is_finite()) || infeasible(1.0, k, lz) {
            ValidationResult::Reject {
                failed: Constraint::LzEParabola,
            }
        } else {
            ValidationResult::Valid
        }
    }
}

/// The control chart's decode: the shared mass and configuration blocks and §2.2's construction at `(L_z, K* = E − U)`,
/// as [`invariant::decode`] runs it, with its outcome.
fn absolute_decode<R: Q>(chart: &AbsoluteLE<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    let (lz, e) = chart.at(u, v);
    let mut blocks = InvariantBlocks::<Defaults, R>::new(lz, e - chart.potential::<R>());
    let d = decode::decode_with(&mut blocks, &chart.frozen());
    (d, Some(blocks.outcome()))
}

/// Layer 2's forbidden set of the control chart, computed here from its definition and not by the chart: the points
/// with `E < U` or `L_z² > 2(E − U)`, in the sweep's order. Every point is first checked to lie off the parabola by a
/// margin far above rounding, so that the set does not depend on how the comparison rounds.
fn layer2_set<R: Q>(chart: &AbsoluteLE<R>) -> Vec<Point>
where
    f64: From<R>,
{
    let u_pot = chart.potential::<f64>();
    let mut set = Vec::new();
    for j in 0..=N {
        for i in 0..=N {
            let (lz, e) = chart.at(grid::<R>(i).f(), grid::<R>(j).f());
            let gap = 2.0 * (e - u_pot) - lz * lz;
            assert!(
                gap.abs() > 1e-9,
                "the control's grid point ({i}, {j}) lies on the parabola, gap {gap:e}"
            );
            if e < u_pot || gap < 0.0 {
                set.push([i, j]);
            }
        }
    }
    set
}

/// The control chart swept with `validate` in place of its own reports exactly layer 2's forbidden set, `E < U`
/// included, and so fails the gallery check (R-467 D11).
fn check_absolute<R: Q>(validate: Validate<AbsoluteLE<R>>)
where
    f64: From<R>,
{
    let chart = absolute::<R>();
    let u_pot = chart.potential::<f64>();
    let [e_lo, e_hi] = chart.e_range.map(Q::f);
    assert!(
        e_lo < u_pot && u_pot < e_hi,
        "U = {u_pot} is not inside the control's energy range"
    );
    let expected = layer2_set(&chart);
    let below = expected
        .iter()
        .filter(|p| chart.at(0.0, grid::<R>(p[1]).f()).1 < u_pot)
        .count();
    assert!(
        below > 0 && below < expected.len(),
        "the control's set holds {below} E < U points of {}",
        expected.len()
    );
    let tally = sweep(&chart, validate, absolute_decode);
    assert_eq!(
        tally.forbidden.len(),
        expected.len(),
        "at {}: the sweep's forbidden set is not layer 2's |L_z| > √(2I(E − U)) set",
        R::LABEL
    );
    assert!(
        tally.forbidden == expected,
        "at {}: the sweep's forbidden set is not layer 2's set, point for point",
        R::LABEL
    );
    let report = Report {
        chart: "(L_z, E) absolute",
        name: chart.name().into(),
        set: String::new(),
        precision: R::LABEL,
        tally,
    };
    println!("{}", report.line());
    assert!(
        faults(&report)
            .iter()
            .any(|f| f.contains("forbidden pixels")),
        "the gallery check passes the control chart"
    );
}

#[test]
fn gallery_sweep_unit_square_absolute_energy() {
    check_absolute::<f64>(AbsoluteLE::validate);
    check_absolute::<f32>(AbsoluteLE::validate);
}

negative_control!(
    gallery_sweep_unit_square_absolute_energy,
    "a sweep whose validate accepts every pixel passes the control chart, and so must fail the control",
    expected = "is not layer 2's",
    check_absolute::<f64>(|_, _, _| ValidationResult::Valid)
);

/// `(L_z, E)` with the warp's `L_max(t)` replaced by `raise(L_max(t))`: `L_z = (2s − 1)·raise(L_max)` through the shared
/// blocks and §2.2's construction, as [`invariant::decode`] runs them, with its outcome.
fn raised_decode<R: Q>(
    chart: &InvariantLE<R>,
    u: R,
    v: R,
    raise: fn(R) -> R,
) -> (Decoded<R>, Option<Outcome>) {
    let t = chart.0.targets(u, v);
    let lz = (u + u - R::one()) * raise(t.l_max);
    let mut blocks = InvariantBlocks::<Defaults, R>::new(lz, t.k);
    let d = decode::decode_with(&mut blocks, &chart.map(u, v).z);
    (d, Some(blocks.outcome()))
}

/// A warp with `L_max` one ulp too large, swept on `(L_z, E)` at each warp set and at `z_cfg = z_mass = 0`, fires
/// `INVARIANT_INFEASIBLE` at every point of the `s ∈ {0, 1}` columns, and so fails the gallery check; off those columns
/// it fires only on the row `t = 0`, where `L_max = 0`, one ulp above it the least subnormal, which `(2s − 1)` rounds to
/// itself wherever `|2s − 1| > ½`. `raise` is the warp's change: one ulp up, or for the control none.
fn check_one_ulp<R: Q>(raise: fn(R) -> R) {
    for (k_max, gamma_k, warp_name) in warp_sets() {
        let chart = InvariantLE(Invariant {
            z_cfg: [R::zero(); 2],
            z_mass: [R::zero(); 2],
            k_max: R::of(k_max),
            gamma_k: R::of(gamma_k),
        });
        let tally = sweep(&chart, InvariantLE::validate, |c, u, v| {
            raised_decode(c, u, v, raise)
        });
        for j in 0..=N {
            for i in [0, N] {
                assert!(
                    tally.infeasible.contains(&[i, j]),
                    "at {} ({warp_name}): the one-ulp warp does not fire INVARIANT_INFEASIBLE at (u, v) = ({i}/256, {j}/256)",
                    R::LABEL
                );
            }
        }
        let stray: Vec<&Point> = tally
            .infeasible
            .iter()
            .filter(|[i, j]| *i != 0 && *i != N && *j != 0)
            .collect();
        assert!(
            stray.is_empty(),
            "at {}: INVARIANT_INFEASIBLE off the edge columns and row 0: {stray:?}",
            R::LABEL
        );
        let report = Report {
            chart: "(L_z, E) one ulp",
            name: chart.name().into(),
            set: warp_name.into(),
            precision: R::LABEL,
            tally,
        };
        println!("{}", report.line());
        assert!(
            faults(&report)
                .iter()
                .any(|f| f.contains("INVARIANT_INFEASIBLE")),
            "the gallery check passes the one-ulp warp"
        );
    }
}

#[test]
fn gallery_sweep_unit_square_one_ulp_warp() {
    check_one_ulp::<f64>(Q::up);
    check_one_ulp::<f32>(Q::up);
}

negative_control!(
    gallery_sweep_unit_square_one_ulp_warp,
    "the shipped warp, L_max not raised, fires nowhere, and so must fail the control",
    expected = "the one-ulp warp does not fire",
    check_one_ulp::<f64>(|l| l)
);

/// The control arm's chart non-finite on one column: the flat slice `z₀ = 0`, `q₁ = ê_0`, `q₂ = ê_1`, whose map sets
/// `z_q0` to NaN on the column `u = column`.
#[derive(Clone, Copy, Debug)]
struct NanColumn<P> {
    slice: Latent<P>,
    column: P,
}

impl<P: Q> Chart for NanColumn<P> {
    type Param = P;

    fn map<F: Float + From<P>>(&self, u: F, v: F) -> ChartOut<F> {
        let mut out = self.slice.map(u, v);
        if u == <F as From<P>>::from(self.column) {
            out.z.z[2] = F::nan();
        }
        out
    }

    fn name(&self) -> &str {
        "NanColumn (control)"
    }

    /// The latent chart's: `Valid` everywhere, so only the non-finite count can catch the column.
    fn validate(&self, u: f64, v: f64) -> ValidationResult {
        self.slice.validate(u, v)
    }
}

/// The non-finite chart's decode, the shared decoder on its map's `z` ([`Chart::decode`]'s default).
fn nan_decode<R: Q>(chart: &NanColumn<R>, u: R, v: R) -> (Decoded<R>, Option<Outcome>) {
    (Chart::decode::<Defaults, R>(chart, u, v), None)
}

/// The chart non-finite on the column `u = column` is caught as non-finite there, its chart output `z` and its decoded
/// `(m, r, p)` at every point of the column and nowhere else, and fails the gallery check.
fn check_nan_column<R: Q>(column: f64) {
    let at = |x: [f64; 8]| x.map(R::of);
    let chart = NanColumn {
        slice: Latent {
            z0: [R::zero(); 8],
            q1: at(axis(0)),
            q2: at(axis(1)),
        },
        column: R::of(column),
    };
    let tally = sweep(&chart, NanColumn::validate, nan_decode);
    let expected: Vec<Point> = (0..=N).map(|j| [N / 2, j]).collect();
    assert!(
        tally.z_nonfinite == expected,
        "at {}: the non-finite column is not caught in z: {} points",
        R::LABEL,
        tally.z_nonfinite.len()
    );
    assert!(
        tally.state_nonfinite == expected,
        "at {}: the non-finite column is not caught in (m, r, p): {} points",
        R::LABEL,
        tally.state_nonfinite.len()
    );
    let report = Report {
        chart: "NaN column",
        name: chart.name().into(),
        set: String::new(),
        precision: R::LABEL,
        tally,
    };
    println!("{}", report.line());
    assert!(
        faults(&report).iter().any(|f| f.contains("non-finite")),
        "the gallery check passes the non-finite column"
    );
}

#[test]
fn gallery_sweep_unit_square_non_finite_column() {
    check_nan_column::<f64>(0.5);
    check_nan_column::<f32>(0.5);
}

negative_control!(
    gallery_sweep_unit_square_non_finite_column,
    "a chart whose non-finite column lies between the grid's columns is finite on the grid, and so must fail the control",
    expected = "the non-finite column is not caught",
    check_nan_column::<f64>(0.5 + 1.0 / 1024.0)
);
