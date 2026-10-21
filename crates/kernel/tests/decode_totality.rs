//! Decode totality and the identities on every chart (dd_decoder §2 "From totality", §5 tests 1 and 4; chart_reference
//! §0.7, §5.2; TASK-M2-24), over the chart registry (`kernel::chart::REGISTRY`), at f32 and f64:
//! - `degenerate_totality_closed_set`: `DEGENERATE`'s cause set is closed, five reasons, each with dd_decoder §2's
//!   `decode_failed` detail code, none of them 0 (REQ-DEC-008, REQ-DEC-038).
//! - `degenerate_totality_registry`: the charts this file visits are the registry's, and the registry names every
//!   `Chart` impl in `kernel/src/chart` (REQ-DEC-008).
//! - `degenerate_totality_every_chart`: fuzzed `(u, v)` on every chart, `η_E` off and on where the chart allows it, past
//!   `[0,1]²` where the chart's own `validate` takes the point: every pixel carries a label, the reason its emitted
//!   values call for (`M01_TINY` from the masses, the construction's from its targets, `η_E`'s from `V₀`, `E*`, `s` and
//!   `r_min_pair_0`) or none, its `(m, r, p)` finite and its descriptor never NaN (REQ-DEC-008).
//! - `degenerate_totality_reasons_reached`: each reason a chart can raise is raised, on the decode it names, and the
//!   one no chart raises, `NO_QUALIFYING_SEED`, by the construction's rule on computed norms (REQ-DEC-008).
//! - `degenerate_totality_stage_entry`: the decode stage's body, natively at f32, writes a tag word every sample, for
//!   every compute variant, under `η_E` off and on (REQ-DEC-008).
//! - `decode_identities_every_chart`: test 1 on every chart over random `(u, v)` and the edges: `Σmᵢ = 1`, CoM `= 0`,
//!   `Σpᵢ = 0` and `I = 1`, each within REQ-DEC-044's and REQ-DEC-045's proposals at each precision (REQ-DEC-025).
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; no trajectory.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::path::Path;

use kernel::chart::burrau::{Burrau, EuclidPlane};
use kernel::chart::invariant::{InvariantChart, WarpReal};
use kernel::chart::mixed::{Alpha, Beta, PLambdaY, PRhoX, ZBeta, ZMu1, ZMu2, ZQ0, ZQ3};
use kernel::chart::{
    BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, Float, InvariantLE, InvariantLK, Latent,
    MassSimplex, Mixed, ShapeMap, ShapeSphere, ValidationResult, REGISTRY,
};
use kernel::decode::degenerate::{
    Degenerate, ALL, DETAIL_DEGENERATE_CONFIGURATION, DETAIL_INVALID_MASS, DETAIL_NON_FINITE,
    DETAIL_OTHER,
};
use kernel::decode::energy_norm::{self, largest};
use kernel::decode::invariant::{construct_by_norms, infeasible, Outcome};
use kernel::decode::{config, mass, Decoded};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, LinkReal, Selection};
use kernel::payload::{PayloadReal, STATE_DECODE_FAILED};
use kernel::stage::decode::{
    self as stage, entry, ic_words, read_ic, sample_with, untag, BurrauDefaults, DecodeUniforms,
    DecodeVariant, EuclidPlaneDefaults, InvariantLEDefaults, InvariantLKDefaults, LatentDefaults,
    LatentStickBreaking, MassSimplexDefaults, MixedMu1AlphaDefaults, StickBreakingLinks,
    UniformChart,
};
use validation::decode_f32_eps::SplitMix;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the decoder is instantiated at.
trait Q: LinkReal + PayloadReal + WarpReal + From<f32> + Into<f64> + Debug {
    const LABEL: &'static str;
    /// The machine epsilon, as f64.
    const EPS: f64;
}

impl Q for f32 {
    const LABEL: &'static str = "f32";
    const EPS: f64 = f32::EPSILON as f64;
}

impl Q for f64 {
    const LABEL: &'static str = "f64";
    const EPS: f64 = f64::EPSILON;
}

fn f<R: Q>(x: R) -> f64 {
    x.into()
}

/// The f32 `x` at `R`, exactly.
fn of<R: Q>(x: f32) -> R {
    <R as From<f32>>::from(x)
}

// ── The registry's charts ───────────────────────────────────────────────────────────────────────────────────────

/// The link set a chart decodes under: the defaults, or `stick_breaking` the mass link, whose corner reaches `M01_TINY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Links {
    Defaults,
    StickBreaking,
}

/// What a chart's decode must carry beyond the mass block's tag: the invariant construction's tag, recomputed from the
/// chart's own targets at `(u, v)` by the exact test (chart_reference §2.2), none for a chart with no construction.
trait Totality: Chart<Param = f32> + Sized {
    fn construction<R: Q>(&self, _u: R, _v: R) -> Option<Degenerate> {
        None
    }
}

/// `INVARIANT_INFEASIBLE` where `K* < L_z²/2` at `I = 1`.
fn short<R: Q>(k: R, lz: R) -> Option<Degenerate> {
    infeasible(R::one(), k, lz).then_some(Degenerate::InvariantInfeasible)
}

impl Totality for Latent<f32> {}
impl Totality for ShapeSphere<f32> {}
impl Totality for EuclidPlane<f32> {}
impl Totality for BurrauThetaDeltaM<f32> {}
impl Totality for MassSimplex<f32> {}
impl<H: kernel::chart::mixed::Quantity, V: kernel::chart::mixed::Quantity> Totality
    for Mixed<H, V, f32>
{
}

impl Totality for Burrau<f32> {
    fn construction<R: Q>(&self, _u: R, v: R) -> Option<Degenerate> {
        short(self.kinetic(v), R::zero())
    }
}

impl Totality for BurrauThetaK<f32> {
    fn construction<R: Q>(&self, _u: R, v: R) -> Option<Degenerate> {
        short(self.kinetic(v), R::zero())
    }
}

impl Totality for BurrauThetaLz<f32> {
    fn construction<R: Q>(&self, _u: R, v: R) -> Option<Degenerate> {
        short(self.kinetic(), self.lz(v))
    }
}

impl Totality for InvariantLE<f32> {
    fn construction<R: Q>(&self, u: R, v: R) -> Option<Degenerate> {
        let t = self.invariant().targets(u, v);
        short(t.k, t.lz)
    }
}

impl Totality for InvariantLK<f32> {
    fn construction<R: Q>(&self, u: R, v: R) -> Option<Degenerate> {
        let t = self.invariant().targets(u, v);
        short(t.k, t.lz)
    }
}

/// A check run on each chart of the registry.
trait Visit {
    fn chart<C: Totality>(&mut self, chart: &C, links: Links);
}

/// A latent chart from `rng`: `z₀` on `[−1, 1]`, `q₁` and `q₂` on `[−spread, spread]`, the momentum block zero where
/// `rest` holds, so every pixel is a rest start.
fn latent(rng: &mut SplitMix, spread: f32, rest: bool) -> Latent<f32> {
    let mut draw = |lo: f32, hi: f32| -> [f32; 8] { core::array::from_fn(|_| rng.uniform(lo, hi)) };
    let (mut z0, mut q1, mut q2) = (
        draw(-1.0, 1.0),
        draw(-spread, spread),
        draw(-spread, spread),
    );
    if rest {
        for k in 2..6 {
            (z0[k], q1[k], q2[k]) = (0.0, 0.0, 0.0);
        }
    }
    Latent { z0, q1, q2 }
}

/// The shape sphere under `map`, its held blocks from `rng`.
fn sphere(rng: &mut SplitMix, map: ShapeMap) -> ShapeSphere<f32> {
    ShapeSphere {
        map,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)],
        z_mom: core::array::from_fn(|_| rng.uniform(-1.0, 1.0)),
    }
}

/// An invariant chart's frozen part from `rng`, under the register's warp.
fn invariant(rng: &mut SplitMix) -> kernel::chart::invariant::Invariant<f32> {
    kernel::chart::invariant::Invariant {
        z_cfg: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        z_mass: [rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)],
        k_max: K_MAX.number() as f32,
        gamma_k: GAMMA_K.number() as f32,
    }
}

/// The eight controls from `rng`, each on `[−2, 2]`.
fn controls(rng: &mut SplitMix) -> [f32; 8] {
    core::array::from_fn(|_| rng.uniform(-2.0, 2.0))
}

fn theta_range() -> [f32; 2] {
    [
        BURRAU_THETA_MIN.number() as f32,
        BURRAU_THETA_MAX.number() as f32,
    ]
}

/// Every chart of the registry, `v`'s check on each: several latent planes (interior, saturated, at rest, and under
/// `stick_breaking`), the shape sphere under both maps, the Burrau family, the invariant charts, the ternary plot and
/// five mixed-axis charts.
fn every_chart(v: &mut impl Visit) {
    let mut rng = SplitMix::new(24);
    for _ in 0..4 {
        v.chart(&latent(&mut rng, 0.5, false), Links::Defaults);
    }
    v.chart(&latent(&mut rng, 20.0, false), Links::Defaults);
    v.chart(&latent(&mut rng, 0.5, true), Links::Defaults);
    v.chart(&latent(&mut rng, 20.0, false), Links::StickBreaking);
    v.chart(&sphere(&mut rng, ShapeMap::Spherical), Links::Defaults);
    v.chart(&sphere(&mut rng, ShapeMap::Exponential), Links::Defaults);
    v.chart(
        &Burrau {
            nu_range: [0.05, 0.95],
            k_max: K_MAX.number() as f32,
            gamma_k: GAMMA_K.number() as f32,
        },
        Links::Defaults,
    );
    v.chart(
        &EuclidPlane {
            m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
            nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
        },
        Links::Defaults,
    );
    v.chart(
        &BurrauThetaK {
            theta_range: theta_range(),
            k_max: K_MAX.number() as f32,
            gamma_k: GAMMA_K.number() as f32,
        },
        Links::Defaults,
    );
    v.chart(
        &BurrauThetaLz {
            theta_range: theta_range(),
            k: BURRAU_THETA_LZ_K.number() as f32,
            lz_fraction: BURRAU_THETA_LZ_FRACTION.number() as f32,
        },
        Links::Defaults,
    );
    v.chart(
        &BurrauThetaDeltaM {
            theta_range: theta_range(),
            // A target whose sum is 1 exactly at f32 and f64, so the f64 decode reads a target on its simplex: the
            // default, equal masses, rounds at f32 to a sum `2⁻²⁵` short of 1, which the decode keeps.
            m_target: [0.25, 0.375, 0.375],
        },
        Links::Defaults,
    );
    for _ in 0..2 {
        v.chart(&InvariantLE(invariant(&mut rng)), Links::Defaults);
        v.chart(&InvariantLK(invariant(&mut rng)), Links::Defaults);
    }
    v.chart(
        &MassSimplex {
            nu0: 0.5,
            eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
        },
        Links::Defaults,
    );
    let wide = [-2.0, 2.0];
    v.chart(
        &Mixed::<ZMu1, Alpha, f32>::new(controls(&mut rng), wide, [0.1, 1.4]),
        Links::Defaults,
    );
    v.chart(
        &Mixed::<Alpha, PRhoX, f32>::new(controls(&mut rng), [0.1, 1.4], [-1.5, 1.5]),
        Links::Defaults,
    );
    v.chart(
        &Mixed::<ZQ0, ZQ3, f32>::new(controls(&mut rng), wide, wide),
        Links::Defaults,
    );
    v.chart(
        &Mixed::<Beta, PLambdaY, f32>::new(controls(&mut rng), [0.2, 3.0], [-1.5, 1.5]),
        Links::Defaults,
    );
    v.chart(
        &Mixed::<ZMu2, ZBeta, f32>::new(controls(&mut rng), wide, wide),
        Links::Defaults,
    );
}

/// `n` fuzzed `(u, v)` on `[lo, hi]²`, then the corners and the edge midpoints of `[0,1]²`.
fn points(rng: &mut SplitMix, n: usize, lo: f32, hi: f32) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = (0..n)
        .map(|_| [rng.uniform(lo, hi), rng.uniform(lo, hi)])
        .collect();
    for u in [0.0, 0.5, 1.0] {
        for v in [0.0, 0.5, 1.0] {
            out.push([u, v]);
        }
    }
    out
}

/// `D ∘ Φ` at `(u, v)` toward `e_star` under `links`, through the shared entry (`energy_norm::decode_chart`).
fn decode<C: Totality, R: Q>(chart: &C, links: Links, u: R, v: R, e_star: Option<R>) -> Decoded<R> {
    fn under<S: Selection, C: Totality, R: Q>(c: &C, u: R, v: R, e: Option<R>) -> Decoded<R> {
        energy_norm::decode_chart::<S, R, C>(c, u, v, e).0
    }
    match links {
        Links::Defaults => under::<Defaults, C, R>(chart, u, v, e_star),
        Links::StickBreaking => under::<StickBreakingLinks, C, R>(chart, u, v, e_star),
    }
}

// ── degenerate_totality_closed_set ──────────────────────────────────────────────────────────────────────────────

/// The reason's place in the type. The match has no wildcard, so a reason added to `Degenerate` fails this file's
/// build until it is listed here, and [`check_closed`] then asks for it in `ALL`.
fn index(d: Degenerate) -> usize {
    match d {
        Degenerate::M01Tiny => 0,
        Degenerate::EnergyInfeasible => 1,
        Degenerate::EnergyUnscalable => 2,
        Degenerate::InvariantInfeasible => 3,
        Degenerate::NoQualifyingSeed => 4,
    }
}

/// dd_decoder §2's map of each reason onto `decode_failed`'s detail codes, and its name.
const MAP: [(Degenerate, u32, &str); 5] = [
    (Degenerate::M01Tiny, DETAIL_INVALID_MASS, "M01_TINY"),
    (
        Degenerate::EnergyInfeasible,
        DETAIL_OTHER,
        "ENERGY_INFEASIBLE",
    ),
    (
        Degenerate::EnergyUnscalable,
        DETAIL_OTHER,
        "ENERGY_UNSCALABLE",
    ),
    (
        Degenerate::InvariantInfeasible,
        DETAIL_OTHER,
        "INVARIANT_INFEASIBLE",
    ),
    (
        Degenerate::NoQualifyingSeed,
        DETAIL_DEGENERATE_CONFIGURATION,
        "NO_QUALIFYING_SEED",
    ),
];

/// `ALL` is every reason once, in the type's order; each reason's label is `decode_failed` with dd_decoder §2's code,
/// within payload §2's four, never 0 (the non-finite output, which no reason writes); and each tag word round-trips.
fn check_closed(detail: fn(Degenerate) -> u32) {
    let places: Vec<usize> = ALL.iter().map(|&d| index(d)).collect();
    assert_eq!(
        places,
        (0..5).collect::<Vec<_>>(),
        "ALL is not the closed set in order"
    );
    for (d, code, name) in MAP {
        assert_eq!(detail(d), code, "{name}'s decode_failed detail");
        assert_eq!(
            d.label(),
            (STATE_DECODE_FAILED, d.detail()),
            "{name}'s label"
        );
        assert_eq!(d.name(), name, "the reason's name");
        assert!(
            code <= DETAIL_OTHER && code != DETAIL_NON_FINITE,
            "{name}'s code {code}"
        );
        assert_eq!(untag(stage::tag(Some(d))), Ok(Some(d)), "{name}'s tag word");
    }
}

#[test]
fn degenerate_totality_closed_set() {
    check_closed(Degenerate::detail);
}

negative_control!(
    degenerate_totality_closed_set,
    "a map that records M01_TINY as a degenerate configuration must fail",
    expected = "M01_TINY's decode_failed detail",
    check_closed(|d| match d {
        Degenerate::M01Tiny => DETAIL_DEGENERATE_CONFIGURATION,
        other => other.detail(),
    })
);

// ── degenerate_totality_registry ────────────────────────────────────────────────────────────────────────────────

/// Collects the names of the charts visited.
struct Names(BTreeSet<String>);

impl Visit for Names {
    fn chart<C: Totality>(&mut self, chart: &C, _links: Links) {
        self.0.insert(chart.name().to_owned());
    }
}

/// Every `impl … Chart for <Name>` in `kernel/src/chart`.
fn impls() -> BTreeSet<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/chart");
    let mut out = BTreeSet::new();
    for e in std::fs::read_dir(&dir).expect("kernel/src/chart") {
        let text = std::fs::read_to_string(e.expect("an entry").path()).expect("a chart source");
        for line in text.lines().filter(|l| l.starts_with("impl")) {
            if let Some(rest) = line.split(" Chart for ").nth(1) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
                out.insert(name);
            }
        }
    }
    out
}

fn check_registry(visited: BTreeSet<String>) {
    let registry: BTreeSet<String> = REGISTRY.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        registry.len(),
        REGISTRY.len(),
        "a chart is registered twice"
    );
    assert_eq!(impls(), registry, "the registry is not every Chart impl");
    assert_eq!(visited, registry, "the charts visited are not the registry");
}

#[test]
fn degenerate_totality_registry() {
    let mut names = Names(BTreeSet::new());
    every_chart(&mut names);
    check_registry(names.0);
}

negative_control!(
    degenerate_totality_registry,
    "a visit that skips the shape sphere must fail",
    expected = "the charts visited are not the registry",
    {
        let mut names = Names(BTreeSet::new());
        every_chart(&mut names);
        names.0.remove("ShapeSphere");
        check_registry(names.0);
    }
);

// ── degenerate_totality_every_chart ─────────────────────────────────────────────────────────────────────────────

/// The reasons the fuzz met, and the count of decodes.
struct Labels {
    /// The tag as the check reads it: the decode's own, or one a control rewrites.
    retag: fn(Option<Degenerate>) -> Option<Degenerate>,
    /// A control's defect: a NaN in the momenta of each decode `η_E` scaled.
    nan_scaled: bool,
    seen: BTreeSet<usize>,
    decodes: usize,
}

/// The tag `η_E` must raise (dd_decoder §3.7), from the emitted `V₀`, `E*`, `r_min_pair_0` and `s` of the momenta before
/// the rescale, each a comparison with no arithmetic at the decision.
fn energy_tag<R: Q>(v0: R, r_min: R, s: R, e_star: R) -> Option<Degenerate> {
    let zero = R::zero();
    if r_min == zero {
        Some(Degenerate::EnergyUnscalable)
    } else if e_star < v0 {
        Some(Degenerate::EnergyInfeasible)
    } else if s == zero && e_star != v0 {
        Some(Degenerate::EnergyUnscalable)
    } else {
        None
    }
}

/// The decode's `(m, r, p)` is finite and its descriptor never NaN, `±∞` only at R-436's poles (`rho_ratio`'s `+∞`,
/// `V_0`'s `−∞`).
fn check_finite<R: Q>(d: &Decoded<R>, at: &str) {
    let ic = &d.ic;
    assert!(
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten())
            .all(|&x| f(x).is_finite()),
        "a non-finite (m, r, p) at {at}: {ic:?}"
    );
    let x = &d.descriptor;
    let fields = [
        x.m0,
        x.m1,
        x.m2,
        x.q_mass,
        x.rho_mag,
        x.lambda_mag,
        x.rho_angle,
        x.K_0,
        x.virial_ratio,
        x.r_min_pair_0,
    ];
    assert!(
        fields.iter().all(|&v| f(v).is_finite()),
        "a non-finite descriptor field at {at}: {x:?}"
    );
    assert!(
        f(x.rho_ratio).is_finite() || f(x.rho_ratio) == f64::INFINITY,
        "rho_ratio = {:?} at {at}",
        x.rho_ratio
    );
    assert!(
        f(x.V_0).is_finite() || f(x.V_0) == f64::NEG_INFINITY,
        "V_0 = {:?} at {at}",
        x.V_0
    );
}

impl Labels {
    /// One pixel at `(u, v)` toward `e_star`: its decode is finite and carries exactly the label its emitted values call
    /// for, the mass block's first, then the construction's, then `η_E`'s.
    fn pixel<C: Totality, R: Q>(
        &mut self,
        chart: &C,
        links: Links,
        uv: [f32; 2],
        e_star: Option<R>,
    ) {
        let (u, v) = (of::<R>(uv[0]), of::<R>(uv[1]));
        let at = format!(
            "{} {} {links:?} ({u:?}, {v:?}) E* {e_star:?}",
            chart.name(),
            R::LABEL
        );
        let off = decode::<C, R>(chart, links, u, v, None);
        let mut d = decode::<C, R>(chart, links, u, v, e_star);
        let target = e_star.filter(|_| !chart.forbids_energy_normalisation());
        d.degenerate = (self.retag)(d.degenerate);
        if self.nan_scaled && d.ic.p != off.ic.p {
            d.ic.p[0][0] = R::nan();
        }
        check_finite(&d, &at);
        let m01 = Float::mul_add(d.ic.m[1], R::one(), d.ic.m[0]);
        let tiny = (m01 < <R as ChartConstants>::EPS_M01).then_some(Degenerate::M01Tiny);
        let energy = target.and_then(|e| {
            let x = &off.descriptor;
            energy_tag(x.V_0, x.r_min_pair_0, largest(&off.ic.p), e)
        });
        let want = tiny.or(chart.construction(u, v)).or(energy);
        assert_eq!(d.degenerate, want, "the label at {at}");
        if let Some(r) = d.degenerate {
            self.seen.insert(index(r));
        }
        self.decodes += 1;
    }

    /// The point the chart's decode takes for `(u, v)`: the point itself where `validate` finds it valid, the point it
    /// projects or clamps to, none where it refuses it (a refused point is labelled by the refusal and never decoded).
    fn admitted<C: Totality>(chart: &C, uv: [f32; 2]) -> Option<[f32; 2]> {
        match chart.validate(f64::from(uv[0]), f64::from(uv[1])) {
            ValidationResult::Valid => Some(uv),
            ValidationResult::Project { u, v, .. } | ValidationResult::Clamp { u, v, .. } => {
                Some([u as f32, v as f32])
            }
            ValidationResult::Reject { .. } => None,
        }
    }
}

impl Visit for Labels {
    fn chart<C: Totality>(&mut self, chart: &C, links: Links) {
        let mut rng = SplitMix::new(chart.name().len() as u64 + self.decodes as u64);
        let mut uv = points(&mut rng, 192, 0.0, 1.0);
        uv.extend(points(&mut rng, 64, -0.5, 1.5));
        for p in uv.into_iter().filter_map(|p| Self::admitted(chart, p)) {
            for r in [None, Some(0.0f32), Some(-0.3), Some(-2.0), Some(1.5)] {
                self.pixel::<C, f32>(chart, links, p, r);
                self.pixel::<C, f64>(chart, links, p, r.map(f64::from));
            }
            // `E* = V₀` exactly: the rest start's untouched, untagged case.
            let v0 = decode::<C, f32>(chart, links, p[0], p[1], None)
                .descriptor
                .V_0;
            if v0.is_finite() {
                self.pixel::<C, f32>(chart, links, p, Some(v0));
            }
            let v0 = decode::<C, f64>(chart, links, f64::from(p[0]), f64::from(p[1]), None);
            if v0.descriptor.V_0.is_finite() {
                self.pixel::<C, f64>(chart, links, p, Some(v0.descriptor.V_0));
            }
        }
    }
}

/// The tag as the decode wrote it.
fn same(d: Option<Degenerate>) -> Option<Degenerate> {
    d
}

fn check_every_chart(nan_scaled: bool) {
    let mut t = Labels {
        retag: same,
        nan_scaled,
        seen: BTreeSet::new(),
        decodes: 0,
    };
    every_chart(&mut t);
    let reached: Vec<&str> = t.seen.iter().map(|&k| ALL[k].name()).collect();
    eprintln!(
        "degenerate_totality_every_chart: {} decodes; reasons met {reached:?}",
        t.decodes
    );
    for d in [
        Degenerate::M01Tiny,
        Degenerate::EnergyInfeasible,
        Degenerate::EnergyUnscalable,
    ] {
        assert!(
            t.seen.contains(&index(d)),
            "the fuzz never met {}",
            d.name()
        );
    }
}

#[test]
fn degenerate_totality_every_chart() {
    check_every_chart(false);
}

negative_control!(
    degenerate_totality_every_chart,
    "a decode with a NaN momentum where η_E scaled must fail",
    expected = "a non-finite (m, r, p)",
    check_every_chart(true)
);

// ── degenerate_totality_reasons_reached ─────────────────────────────────────────────────────────────────────────

/// Each reason a chart raises, raised: `INVARIANT_INFEASIBLE` on the invariant charts and the `L_z` strip at a point
/// past their feasible region, with the momenta finite, the rigid rotation; `NO_QUALIFYING_SEED` by the construction's
/// rule on computed norms at a collapsed configuration (`I = 0`), the one place the rule finds no seed; and, under the
/// totality check, the label each must carry.
fn check_reached(retag: fn(Option<Degenerate>) -> Option<Degenerate>) {
    let mut t = Labels {
        retag,
        nan_scaled: false,
        seen: BTreeSet::new(),
        decodes: 0,
    };
    let mut rng = SplitMix::new(5);
    let le = InvariantLE(invariant(&mut rng));
    let lk = InvariantLK(invariant(&mut rng));
    let strip = BurrauThetaLz {
        theta_range: theta_range(),
        k: BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: 1.5,
    };
    for p in [[-0.25f32, 0.5], [1.25, 0.75], [1.5, 1.0]] {
        t.pixel::<_, f32>(&le, Links::Defaults, p, None);
        t.pixel::<_, f64>(&le, Links::Defaults, p, None);
        t.pixel::<_, f32>(&lk, Links::Defaults, p, None);
        t.pixel::<_, f64>(&lk, Links::Defaults, p, None);
    }
    for v in [0.0f32, 1.0] {
        t.pixel::<_, f32>(&strip, Links::Defaults, [0.5, v], None);
        t.pixel::<_, f64>(&strip, Links::Defaults, [0.5, v], None);
    }
    assert!(
        t.seen.contains(&index(Degenerate::InvariantInfeasible)),
        "INVARIANT_INFEASIBLE was never raised"
    );
    let (lz, k) = (0.5f64, 1.0f64);
    let masses = mass::decode::<Defaults, f64>([0.3, -0.2]);
    let mut collapsed = config::decode::<Defaults, f64>([0.4, -0.6], &masses);
    collapsed.r = [[0.0; 2]; 3];
    collapsed.rho = [0.0; 2];
    collapsed.lambda = [0.0; 2];
    let c = construct_by_norms(&masses, &collapsed, lz, k);
    assert_eq!(
        c.outcome,
        Outcome::Degenerate,
        "the collapsed configuration's outcome"
    );
    assert_eq!(
        c.outcome.tag(),
        Some(Degenerate::NoQualifyingSeed),
        "the seed rule's tag"
    );
    assert!(
        c.p.iter().flatten().all(|x| x.is_finite()),
        "the seed rule's momenta are not finite"
    );
}

#[test]
fn degenerate_totality_reasons_reached() {
    check_reached(same);
}

negative_control!(
    degenerate_totality_reasons_reached,
    "a decode that drops the construction's tag must fail",
    expected = "the label at InvariantLE",
    check_reached(|d| d.filter(|&r| r != Degenerate::InvariantInfeasible))
);

// ── degenerate_totality_stage_entry ─────────────────────────────────────────────────────────────────────────────

/// The decode stage's body for `V`, natively at f32, over `uv` with `chart`, toward `e_star`: every sample's tag word is
/// written, a reason or none, and is [`sample_with`]'s, whose decode is the CPU's `η_E` (`energy_norm::decode_chart`) bit
/// for bit, and every `(m, r, p)` is written and finite.
fn check_entry<V: DecodeVariant>(
    chart: &V::Chart,
    uv: &[[f32; 2]],
    e_star: Option<f32>,
    blank: bool,
) {
    let uniforms = DecodeUniforms {
        chart: chart.uniform(),
        flags: [stage::flags(stage::DecodeMode::Full), 0, 0, 0].into(),
        energy: stage::energy(e_star),
    };
    let n = uv.len();
    let flat: Vec<f32> = uv.iter().flatten().copied().collect();
    let mut ic = vec![f32::NAN.to_bits(); n * ic_words()];
    let mut tags = vec![u32::MAX; n];
    let mut descriptors = vec![0; n * stage::descriptor_words()];
    for i in 0..n as u32 {
        if !(blank && i == 3) {
            entry::<V>(i, &uniforms, &flat, &mut ic, &mut tags, &mut descriptors);
        }
    }
    let chart = V::Chart::from_uniform(&uniforms.chart);
    for (i, p) in uv.iter().enumerate() {
        let want = sample_with::<V::Chart, V::Links, f32>(&chart, p[0], p[1], e_star);
        // The stage's branch-free `η_E` (`energy_norm::apply_shared`) is the CPU's (`energy_norm::apply`), bit for bit:
        // the same IC, descriptor and tag, the switch's off and on cases both.
        let cpu =
            energy_norm::decode_chart::<V::Links, f32, V::Chart>(&chart, p[0], p[1], e_star).0;
        assert_eq!(
            format!("{:?}", (want.ic, want.descriptor, want.degenerate)),
            format!("{:?}", (cpu.ic, cpu.descriptor, cpu.degenerate)),
            "{}: sample {i}'s stage decode is not the CPU's toward {e_star:?}",
            V::ENTRY
        );
        let got = untag(tags[i]);
        assert_eq!(
            got,
            Ok(want.degenerate),
            "{}: sample {i}'s tag word {:#x}",
            V::ENTRY,
            tags[i]
        );
        let out = read_ic(&ic, i);
        assert!(
            out.m
                .iter()
                .chain(out.r.iter().flatten())
                .chain(out.p.iter().flatten())
                .all(|x| x.is_finite()),
            "{}: sample {i}'s (m, r, p) is not written or not finite",
            V::ENTRY
        );
    }
}

/// [`check_entry`] on every compute variant the stage is built for, `η_E` off and at three targets.
fn check_entries(blank: bool) {
    let mut rng = SplitMix::new(11);
    let uv = points(&mut rng, 40, 0.0, 1.0);
    for e in [None, Some(0.0), Some(-1.0), Some(2.0)] {
        check_entry::<LatentDefaults>(&latent(&mut rng, 0.5, false), &uv, e, blank);
        check_entry::<LatentStickBreaking>(&latent(&mut rng, 20.0, false), &uv, e, blank);
        check_entry::<InvariantLEDefaults>(&InvariantLE(invariant(&mut rng)), &uv, e, blank);
        check_entry::<InvariantLKDefaults>(&InvariantLK(invariant(&mut rng)), &uv, e, blank);
        let simplex = MassSimplex {
            nu0: 0.5,
            eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
        };
        check_entry::<MassSimplexDefaults>(&simplex, &uv, e, blank);
        let burrau = Burrau {
            nu_range: [0.05, 0.95],
            k_max: K_MAX.number() as f32,
            gamma_k: GAMMA_K.number() as f32,
        };
        check_entry::<BurrauDefaults>(&burrau, &uv, e, blank);
        let plane = EuclidPlane {
            m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
            nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
        };
        check_entry::<EuclidPlaneDefaults>(&plane, &uv, e, blank);
        let mixed = Mixed::<ZMu1, Alpha, f32>::new(controls(&mut rng), [-2.0, 2.0], [0.1, 1.4]);
        check_entry::<MixedMu1AlphaDefaults>(&mixed, &uv, e, blank);
    }
}

#[test]
fn degenerate_totality_stage_entry() {
    check_entries(false);
}

negative_control!(
    degenerate_totality_stage_entry,
    "a stage that skips a sample leaves its tag unwritten and must fail",
    expected = "sample 3's tag word 0xffffffff",
    check_entries(true)
);

// ── decode_identities_every_chart ───────────────────────────────────────────────────────────────────────────────

/// REQ-DEC-044's and REQ-DEC-045's proposals for test 1 (`kernel/tests/decode_blocks.rs`, `decode_join.rs`), in the
/// precision's eps: `Σmᵢ − 1`; the CoM relative to `Σmᵢ‖rᵢ‖`; `Σpᵢ` relative to `Σ‖pᵢ‖`; `I − 1`.
const SUM_EPS: f64 = 4.0;
const COM_EPS: f64 = 4.0;
const SUM_P_EPS: f64 = 4.0;
const INERTIA_EPS: f64 = 8.0;

/// The worst of each identity, in eps, with where.
#[derive(Default)]
struct Identities {
    worst: [(f64, String); 4],
    /// A transcription defect a control introduces: `p₀`'s `p_λ` weight read with `m₁` (chart_reference §0.3's hazard).
    crossed: bool,
}

impl Identities {
    fn note(&mut self, k: usize, err: f64, at: &str) {
        if err > self.worst[k].0 || err.is_nan() {
            self.worst[k] = (err, at.to_owned());
        }
    }

    fn one<C: Totality, R: Q>(&mut self, chart: &C, links: Links, uv: [f32; 2]) {
        let (u, v) = (of::<R>(uv[0]), of::<R>(uv[1]));
        let at = format!("{} {} ({u:?}, {v:?})", chart.name(), R::LABEL);
        let d = decode::<C, R>(chart, links, u, v, None);
        let m = d.ic.m.map(f::<R>);
        let r = d.ic.r.map(|x| x.map(f::<R>));
        let mut p = d.ic.p.map(|x| x.map(f::<R>));
        if self.crossed {
            let w = (m[1] - m[0]) / (m[0] + m[1]);
            p[0] = [p[0][0] + w * p[2][0], p[0][1] + w * p[2][1]];
        }
        let eps = R::EPS;
        self.note(0, (m[0] + m[1] + m[2] - 1.0).abs() / eps, &at);
        let com = [0, 1].map(|k| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>());
        let size: f64 = (0..3).map(|i| m[i] * r[i][0].hypot(r[i][1])).sum();
        self.note(1, com[0].hypot(com[1]) / (size * eps), &at);
        let scale: f64 = p.iter().map(|x| x[0].hypot(x[1])).sum();
        for s in [0, 1].map(|k| (p[0][k] + p[1][k] + p[2][k]).abs()) {
            self.note(2, if scale == 0.0 { s } else { s / (scale * eps) }, &at);
        }
        let inertia: f64 = (0..3)
            .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
            .sum();
        self.note(3, (inertia - 1.0).abs() / eps, &at);
    }
}

impl Visit for Identities {
    fn chart<C: Totality>(&mut self, chart: &C, links: Links) {
        let mut rng = SplitMix::new(chart.name().len() as u64);
        for p in points(&mut rng, 256, 0.0, 1.0) {
            self.one::<C, f32>(chart, links, p);
            self.one::<C, f64>(chart, links, p);
        }
    }
}

fn check_identities(crossed: bool) {
    let mut ids = Identities {
        crossed,
        ..Identities::default()
    };
    every_chart(&mut ids);
    let names = ["Σm − 1", "CoM", "Σp", "I − 1"];
    for (k, tol) in [SUM_EPS, COM_EPS, SUM_P_EPS, INERTIA_EPS]
        .into_iter()
        .enumerate()
    {
        let (err, at) = &ids.worst[k];
        eprintln!(
            "decode_identities_every_chart: {} worst {err:.3} eps at {at}",
            names[k]
        );
        assert!(*err <= tol, "{} = {err} eps at {at}, over {tol}", names[k]);
    }
}

#[test]
fn decode_identities_every_chart() {
    check_identities(false);
}

negative_control!(
    decode_identities_every_chart,
    "the crossed-mass transcription of p₀ must fail Σp = 0",
    expected = "Σp = ",
    check_identities(true)
);
