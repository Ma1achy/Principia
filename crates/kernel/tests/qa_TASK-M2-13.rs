//! QA tests for TASK-M2-13, written from the requirements (REQ-CHART-026, REQ-CHART-008, REQ-DEC-006; chart_reference
//! §1.3, §4.4, §4.5; chart_decoder_contract § "The four axis kinds"; inverse_encode_contract § "Chart-aware validation";
//! lowering Part 3 "Mass source"; R-22), not from the implementation. Each decode check runs at the f64 and the f32
//! instantiation of the one source, natively (R-265).
//! - `qa_m213_ternary_masses` (REQ-CHART-026): over a 65 × 65 lattice of `[0,1]²` (edges and corners included) and a
//!   low-discrepancy interior set, every pixel's decoded masses are `≥ ε_m` and sum to 1, and they are §4.5's formula
//!   evaluated here: `x = u`, `y = (1 − u)v`, `(m₀, m₁, m₂) = (1 − x − y, x, y)`, `m ← (1 − 3ε_m)m + ε_m`; the default
//!   `ε_m` is `10⁻⁴`.
//! - `qa_m213_ternary_geometry` (REQ-CHART-026): the geometry is fixed at `ν₀`: at every pixel the decoded triangle has
//!   its right angle at body 0 and sides opposite bodies 0, 1, 2 in the ratio `c : b : a` of `ν₀`'s Burrau triangle
//!   (`a = 1 − ν₀²`, `b = 2ν₀`, `c = 1 + ν₀²`; R-22: mass `i` is the side opposite body `i`); every momentum is zero.
//! - `qa_m213_ternary_marker` (REQ-CHART-026): the Burrau marker lies in `[0,1]²` and the decode there gives the Burrau
//!   masses `(c, b, a)/(a + b + c)` (at `ν₀ = 1/2`, `(5, 4, 3)/12`).
//! - `qa_m213_ternary_validate` (inverse_encode_contract layer 2, "Mass simplex"): every point of `[0,1]²` is valid;
//!   any point whose raw masses leave the simplex (some `mᵢ < 0`) is not; `ν₀ ∉ (0, 1)` is refused; a clamp lands on a
//!   valid point.
//! - `qa_m213_mixed_alpha_prho` (REQ-CHART-008): the `(α, p_ρ)` chart decodes through the shared decoder: its decode
//!   is bit-identical to `kernel::decode::decode` on `z(u, v)`, the frozen controls with `z_α` and `z_q0` set through
//!   the selection's own link inverses at `α(u) = lo + (hi − lo)u`, `p_ρ(v)` likewise; the decoded `α` and `p_ρ,x` are
//!   the axes' values to within one pixel's step of the axis.
//! - `qa_m213_mixed_feasibility` (REQ-CHART-008, inverse_encode layer 2 "Mixed-axis charts inherit both axes'
//!   constraints"): on a lattice reaching past both axes' domains (`α ∈ [0, π/2]`, `p ∈ [−q_max, q_max]`), validate is
//!   `Valid` exactly where both axes are inside their domains; outside, a clamp moves exactly the failing axes and lands
//!   on a valid point; an axis that is a latent coordinate adds no constraint.
//! - `qa_m213_per_pixel_mass_flags` (REQ-DEC-006): the ternary plot, the `(ν, K)` chart, the Euclid plane and mixed
//!   charts with a mass axis declare `requires_per_pixel_mass`; each chart that declares it *false* has bit-identical
//!   masses at every pixel, so a uniform mass describes it; each mass-varying chart's masses do vary, from its decode.
//!
//! **Tolerances.** Every quantity compared has magnitude at most a few and comes from a handful of operations: `RND =
//! 64` eps absolute (relative for ratios), a rounding bound, as `qa_TASK-M2-10` states it. The axis round trip is held to
//! the pixel step, the axis's own resolution, with no constant. `ε_m` is the register's `mass_simplex_eps_m`.
//!
//! **Domain** (pitfalls §10): the stateless decode, its inputs held fixed; no trajectory. The GPU half of REQ-DEC-006
//! is the implementer's `engine/tests/per_pixel_mass_gpu.rs`.
//!
//! Each test has a registered negative control naming the panic it expects (R-176, R-212).

use std::fmt::Debug;

use kernel::chart::invariant::{Invariant, WarpReal};
use kernel::chart::mixed::{Alpha, PRhoX, ZMu1, ZMu2, ZQ0};
use kernel::chart::shape_sphere::ShapeMap;
use kernel::chart::{
    Burrau, Chart, ChartDecode, EuclidPlane, InvariantLE, InvariantLK, MassSimplex, Mixed,
    ShapeSphere, ValidationResult,
};
use kernel::decode::{self, Decoded, Latent};
use kernel::generated::links::{Codomain, Defaults, Link, LinkReal, Momentum, Selection};
use kernel::payload::PayloadReal;
use validation::{
    negative_control, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
    MASS_SIMPLEX_EPS_M,
};

/// Rounding bound, in eps (the module's note).
const RND: f64 = 64.0;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait Q:
    LinkReal + PayloadReal + WarpReal + Copy + Debug + PartialEq + PartialOrd + From<Self>
{
    const EPS: f64;
    const LABEL: &'static str;
    fn f(self) -> f64;
    fn of(x: f64) -> Self;
}

impl Q for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    fn f(self) -> f64 {
        self
    }
    fn of(x: f64) -> Self {
        x
    }
}

impl Q for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn f(self) -> f64 {
        self as f64
    }
    fn of(x: f64) -> Self {
        x as f32
    }
}

/// A deterministic low-discrepancy sequence on `(0, 1)`.
fn golden(i: usize, k: f64) -> f64 {
    (0.5 + i as f64 * k).fract()
}

/// The 65 × 65 lattice of `[0,1]²`, then 400 interior points.
fn pixels() -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for i in 0..=64 {
        for j in 0..=64 {
            out.push((i as f64 / 64.0, j as f64 / 64.0));
        }
    }
    for i in 0..400 {
        out.push((
            golden(i, 0.618_033_988_749_894_9),
            golden(i, 0.754_877_666_246_692_7),
        ));
    }
    out
}

fn eps_m() -> f64 {
    MASS_SIMPLEX_EPS_M.number()
}

/// The Burrau triangle's sides at `ν` (chart_reference §4.5).
fn sides(nu: f64) -> (f64, f64, f64) {
    (1.0 - nu * nu, 2.0 * nu, 1.0 + nu * nu)
}

/// §4.5's buffered simplex masses at `(u, v)`, in f64, from the corpus's formula.
fn formula(eps: f64, u: f64, v: f64) -> [f64; 3] {
    let x = u;
    let y = (1.0 - u) * v;
    let raw = [1.0 - x - y, x, y];
    raw.map(|m| (1.0 - 3.0 * eps) * m + eps)
}

fn ternary<R: Q>(nu0: f64, eps: f64) -> MassSimplex<R> {
    MassSimplex {
        nu0: R::of(nu0),
        eps_m: R::of(eps),
    }
}

fn ternary_decode<R: Q>(chart: &MassSimplex<R>, u: f64, v: f64) -> Decoded<R>
where
    f64: From<R>,
{
    Chart::decode::<Defaults, R>(chart, R::of(u), R::of(v))
}

// ── REQ-CHART-026: the masses ──────────────────────────────────────────────────────────────────────────────────────

/// Every pixel's masses are `≥ want_floor`, sum to 1 and are the formula's at `formula_eps`.
fn check_masses<R: Q>(chart_eps: f64, want_floor: f64, formula_eps: f64)
where
    f64: From<R>,
{
    let chart = ternary::<R>(0.5, chart_eps);
    let tol = RND * R::EPS;
    let floor = R::of(want_floor);
    for (u, v) in pixels() {
        let d = ternary_decode(&chart, u, v);
        let m = d.ic.m;
        for (k, mk) in m.iter().enumerate() {
            assert!(
                *mk >= floor,
                "{}: mass {k} at ({u}, {v}) is {mk:?}, below ε_m",
                R::LABEL
            );
        }
        let sum: f64 = m.iter().map(|x| x.f()).sum();
        assert!(
            (sum - 1.0).abs() <= tol,
            "{}: the masses at ({u}, {v}) sum to {sum}",
            R::LABEL
        );
        // The pixel's own (u, v) at this precision, so the formula sees what the chart saw.
        let want = formula(formula_eps, R::of(u).f(), R::of(v).f());
        for k in 0..3 {
            assert!(
                (m[k].f() - want[k]).abs() <= tol,
                "{}: mass {k} at ({u}, {v}) is {:?}, the formula's {}",
                R::LABEL,
                m[k],
                want[k]
            );
        }
        assert_eq!(d.degenerate, None, "{}: DEGENERATE at ({u}, {v})", R::LABEL);
    }
}

#[test]
fn qa_m213_ternary_masses() {
    assert_eq!(
        eps_m(),
        1e-4,
        "the default ε_m is chart_reference §4.5's 10⁻⁴"
    );
    check_masses::<f64>(eps_m(), eps_m(), eps_m());
    check_masses::<f32>(eps_m(), eps_m(), eps_m());
}

negative_control!(
    qa_m213_ternary_masses,
    "a chart with no buffer reaches 0 at the simplex's corners, below ε_m",
    expected = "below ε_m",
    check_masses::<f64>(0.0, eps_m(), 0.0)
);

negative_control!(
    qa_m213_ternary_masses_formula,
    "the formula at 3ε_m is not the chart's at ε_m",
    expected = "the formula's",
    check_masses::<f64>(eps_m(), 0.0, 3.0 * eps_m())
);

// ── REQ-CHART-026: the geometry, fixed at ν₀, at rest ───────────────────────────────────────────────────────────────

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// At every pixel the triangle is `want_nu`'s: right angle at body 0, sides opposite bodies 0, 1, 2 as `c : b : a`;
/// every momentum 0.
fn check_geometry<R: Q>(nu0: f64, want_nu: f64)
where
    f64: From<R>,
{
    let chart = ternary::<R>(nu0, eps_m());
    let (a, b, c) = sides(want_nu);
    let tol = RND * R::EPS;
    for (u, v) in pixels() {
        let d = ternary_decode(&chart, u, v);
        let r = d.ic.r.map(|p| p.map(|x| x.f()));
        let opp = [dist(r[1], r[2]), dist(r[2], r[0]), dist(r[0], r[1])];
        let want = [c, b, a];
        for k in 0..3 {
            let got = opp[k] / opp[0];
            let w = want[k] / want[0];
            assert!(
                (got - w).abs() <= tol,
                "{}: at ({u}, {v}) side opposite body {k} over the hypotenuse is {got}, the triangle's {w}",
                R::LABEL
            );
        }
        let e1 = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
        let e2 = [r[2][0] - r[0][0], r[2][1] - r[0][1]];
        let cos = (e1[0] * e2[0] + e1[1] * e2[1]) / (opp[2] * opp[1]);
        assert!(
            cos.abs() <= tol,
            "{}: at ({u}, {v}) the angle at body 0 has cosine {cos}",
            R::LABEL
        );
        for (i, p) in d.ic.p.iter().enumerate() {
            assert!(
                p[0] == R::of(0.0) && p[1] == R::of(0.0),
                "{}: at ({u}, {v}) body {i} moves: {p:?}",
                R::LABEL
            );
        }
    }
}

#[test]
fn qa_m213_ternary_geometry() {
    for nu0 in [0.5, 0.25, 0.8, 1.0 / 3.0] {
        check_geometry::<f64>(nu0, nu0);
        check_geometry::<f32>(nu0, nu0);
    }
}

negative_control!(
    qa_m213_ternary_geometry,
    "the triangle at ν₀ = 1/2 is not the one at ν = 1/3",
    expected = "the triangle's",
    check_geometry::<f64>(0.5, 1.0 / 3.0)
);

// ── REQ-CHART-026: the Burrau marker ───────────────────────────────────────────────────────────────────────────────

/// The marker of the chart at `nu0` lies in `[0,1]²`, and the decode there gives the Burrau masses at `want_nu`.
fn check_marker<R: Q>(nu0: f64, want_nu: f64)
where
    f64: From<R>,
{
    let chart = ternary::<R>(nu0, eps_m());
    let [u, v] = chart.marker();
    assert!(
        (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v),
        "{}: the marker ({u}, {v}) is outside [0,1]²",
        R::LABEL
    );
    let (a, b, c) = sides(want_nu);
    let s = a + b + c;
    let want = [c / s, b / s, a / s];
    let m = ternary_decode(&chart, u, v).ic.m;
    for k in 0..3 {
        assert!(
            (m[k].f() - want[k]).abs() <= RND * R::EPS,
            "{}: at the marker mass {k} is {:?}, not the Burrau mass {}",
            R::LABEL,
            m[k],
            want[k]
        );
    }
}

#[test]
fn qa_m213_ternary_marker() {
    // ν₀ = 1/2: the (3, 4, 5) triangle, masses (5, 4, 3)/12 (R-22, body 0 at the right angle).
    let (a, b, c) = sides(0.5);
    assert_eq!(
        [c / (a + b + c), b / (a + b + c), a / (a + b + c)],
        [5.0 / 12.0, 4.0 / 12.0, 3.0 / 12.0]
    );
    for nu0 in [0.5, 0.25, 0.8, 0.05, 0.95] {
        check_marker::<f64>(nu0, nu0);
        check_marker::<f32>(nu0, nu0);
    }
}

negative_control!(
    qa_m213_ternary_marker,
    "the marker at ν₀ = 1/2 is not where the masses of ν = 1/4 are",
    expected = "not the Burrau mass",
    check_marker::<f64>(0.5, 0.25)
);

// ── Layer 2 for the ternary plot ───────────────────────────────────────────────────────────────────────────────────

/// Whether every raw mass of `(u, v)` is `≥ 0`, by §4.5's formula.
fn raw_in_simplex(u: f64, v: f64) -> bool {
    let x = u;
    let y = (1.0 - u) * v;
    x >= 0.0 && y >= 0.0 && 1.0 - x - y >= 0.0
}

fn check_ternary_validate(eps: f64, in_simplex: fn(f64, f64) -> bool) {
    let chart = ternary::<f64>(0.5, eps);
    for i in 0..=40 {
        for j in 0..=40 {
            let (u, v) = (-0.5 + i as f64 / 20.0, -0.5 + j as f64 / 20.0);
            let res = chart.validate(u, v);
            let want = in_simplex(u, v);
            assert_eq!(
                res == ValidationResult::Valid,
                want,
                "validate({u}, {v}) = {res:?}, the simplex says valid = {want}"
            );
            match res {
                ValidationResult::Clamp { u: cu, v: cv, .. }
                | ValidationResult::Project { u: cu, v: cv, .. } => {
                    assert_eq!(
                        chart.validate(cu, cv),
                        ValidationResult::Valid,
                        "the corrected point ({cu}, {cv}) of ({u}, {v}) is not valid"
                    );
                    let m = formula(eps, cu, cv);
                    assert!(
                        m.iter().all(|&x| x >= eps),
                        "masses below ε_m at the clamp {m:?}"
                    );
                }
                _ => {}
            }
        }
    }
    for nu0 in [0.0, 1.0, -0.2, 1.5, f64::NAN] {
        assert!(
            matches!(
                ternary::<f64>(nu0, eps).validate(0.5, 0.5),
                ValidationResult::Reject { .. }
            ),
            "ν₀ = {nu0} is not refused"
        );
    }
    assert!(
        matches!(
            chart.validate(f64::NAN, 0.5),
            ValidationResult::Reject { .. }
        ),
        "a NaN coordinate is not refused"
    );
}

#[test]
fn qa_m213_ternary_validate() {
    check_ternary_validate(eps_m(), raw_in_simplex);
}

negative_control!(
    qa_m213_ternary_validate,
    "the whole plane is not the simplex",
    expected = "the simplex says valid",
    check_ternary_validate(eps_m(), |_, _| true)
);

// ── REQ-CHART-008: the (α, p_ρ) chart ──────────────────────────────────────────────────────────────────────────────

type AlphaPRho<R> = Mixed<Alpha, PRhoX, R>;

const FROZEN: [f64; 8] = [0.3, -0.4, 0.5, 0.25, -0.6, 0.1, 0.2, -0.3];
const H_RANGE: [f64; 2] = [0.1, 1.4];
const V_RANGE: [f64; 2] = [-1.5, 1.7];

fn alpha_prho<R: Q>(h: [f64; 2], v: [f64; 2]) -> AlphaPRho<R> {
    Mixed::new(FROZEN.map(R::of), h.map(R::of), v.map(R::of))
}

fn lerp<R: Q>(r: [f64; 2], t: R) -> R {
    let (lo, hi) = (R::of(r[0]), R::of(r[1]));
    lo + (hi - lo) * t
}

/// The control `z` the `S` link maps to `x`, by the link's own inverse.
fn alpha_control<R: Q>(x: R) -> R {
    <<Defaults as Selection>::AlphaLink as Link<1, 1>>::inverse([x])[0]
}

fn q0_control<R: Q>(x: R) -> R {
    <<Defaults as Selection>::Q0Link as Link<1, 1>>::inverse([x])[0]
}

/// The chart's decode is the shared decoder on `z(u, v)` built here: `z_α` at slot `alpha_slot`, `z_q0` at slot 2,
/// the rest frozen. The decoded `α`, `p_ρ,x` are the axes' values to within one pixel step.
fn check_alpha_prho<R: Q>(alpha_slot: usize)
where
    f64: From<R>,
{
    let chart = alpha_prho::<R>(H_RANGE, V_RANGE);
    let n = 64;
    for i in 0..=n {
        for j in 0..=n {
            let (u, v) = (R::of(i as f64 / n as f64), R::of(j as f64 / n as f64));
            let got = Chart::decode::<Defaults, R>(&chart, u, v);
            let alpha = lerp(H_RANGE, u);
            let p = lerp(V_RANGE, v);
            let mut z = FROZEN.map(R::of);
            z[alpha_slot] = alpha_control(alpha);
            z[2] = q0_control(p);
            let want = decode::decode::<Defaults, R>(&Latent::new(z));
            assert!(
                got.ic == want.ic
                    && got.descriptor == want.descriptor
                    && got.degenerate == want.degenerate,
                "{}: at ({u:?}, {v:?}) the chart's decode is not the shared decoder's on z(u, v)",
                R::LABEL
            );
            let step_a = (H_RANGE[1] - H_RANGE[0]) / n as f64;
            let step_p = (V_RANGE[1] - V_RANGE[0]) / n as f64;
            assert!(
                (got.config.alpha.f() - alpha.f()).abs() < step_a,
                "{}: the decoded α {:?} is not α(u) = {alpha:?}",
                R::LABEL,
                got.config.alpha
            );
            assert!(
                (got.momentum.p_rho[0].f() - p.f()).abs() < step_p,
                "{}: the decoded p_ρ,x {:?} is not p(v) = {p:?}",
                R::LABEL,
                got.momentum.p_rho[0]
            );
        }
    }
}

#[test]
fn qa_m213_mixed_alpha_prho() {
    check_alpha_prho::<f64>(0);
    check_alpha_prho::<f32>(0);
    // The (α, p_ρ) chart has no mass axis: its masses are the frozen controls', and the shared decoder's.
    let chart = alpha_prho::<f64>(H_RANGE, V_RANGE);
    let a = Chart::decode::<Defaults, f64>(&chart, 0.1, 0.9).ic.m;
    let b = Chart::decode::<Defaults, f64>(&chart, 0.8, 0.2).ic.m;
    assert_eq!(a, b, "the (α, p_ρ) chart's masses vary with (u, v)");
}

negative_control!(
    qa_m213_mixed_alpha_prho,
    "a z with α's control in β's slot is not the chart's",
    expected = "is not the shared decoder's",
    check_alpha_prho::<f64>(1)
);

// ── REQ-CHART-008: feasibility, the conjunction ────────────────────────────────────────────────────────────────────

fn alpha_ok(x: f64) -> bool {
    (0.0..=core::f64::consts::FRAC_PI_2).contains(&x)
}

fn p_ok(x: f64) -> bool {
    (-Momentum::HI..=Momentum::HI).contains(&x) && Momentum::LO == -Momentum::HI
}

/// On a lattice of `[−0.5, 1.5]²`, the `(α, p_ρ)` chart over ranges reaching past both domains: valid exactly where
/// `expect` says, and a clamp moves exactly the failing axes onto a valid point.
fn check_conjunction(expect: fn(bool, bool) -> bool) {
    // α(u) on [−0.5, 1.5] spans [−0.55, 2.05], past [0, π/2] both ways; p(v) spans [−4.7, 4.9], past [−2, 2].
    let chart = alpha_prho::<f64>(H_RANGE, V_RANGE);
    let mut seen = [[false; 2]; 2];
    for i in 0..=80 {
        for j in 0..=80 {
            let (u, v) = (-0.5 + i as f64 / 40.0, -0.5 + j as f64 / 40.0);
            let (ha, vp) = (
                H_RANGE[0] + (H_RANGE[1] - H_RANGE[0]) * u,
                V_RANGE[0] + (V_RANGE[1] - V_RANGE[0]) * v,
            );
            // Skip the lattice points within a rounding of an edge, where either answer is right.
            let near = |x: f64, e: f64| (x - e).abs() < 1e-9;
            if near(ha, 0.0)
                || near(ha, core::f64::consts::FRAC_PI_2)
                || near(vp, Momentum::HI)
                || near(vp, -Momentum::HI)
            {
                continue;
            }
            let (ok_h, ok_v) = (alpha_ok(ha), p_ok(vp));
            seen[ok_h as usize][ok_v as usize] = true;
            let res = chart.validate(u, v);
            let want = expect(ok_h, ok_v);
            assert_eq!(
                res == ValidationResult::Valid,
                want,
                "validate({u}, {v}) (α = {ha}, p = {vp}) = {res:?}, the conjunction says valid = {want}"
            );
            if let ValidationResult::Clamp {
                u: cu,
                v: cv,
                moved,
                ..
            } = res
            {
                assert_eq!(
                    moved,
                    [!ok_h, !ok_v],
                    "the clamp of ({u}, {v}) moved {moved:?}"
                );
                assert_eq!(
                    cu == u,
                    ok_h,
                    "the clamp of ({u}, {v}) moved u wrongly: {cu}"
                );
                assert_eq!(
                    cv == v,
                    ok_v,
                    "the clamp of ({u}, {v}) moved v wrongly: {cv}"
                );
                assert_eq!(
                    chart.validate(cu, cv),
                    ValidationResult::Valid,
                    "the clamp ({cu}, {cv}) of ({u}, {v}) is not valid"
                );
            }
        }
    }
    assert_eq!(seen, [[true; 2]; 2], "the lattice missed a quadrant");
    // A latent axis adds no constraint: (z_q0, α) is α's check alone.
    let latent_h: Mixed<ZQ0, Alpha, f64> = Mixed::new(FROZEN, [-50.0, 50.0], H_RANGE);
    for i in 0..=80 {
        for j in 0..=80 {
            let (u, v) = (-0.5 + i as f64 / 40.0, -0.5 + j as f64 / 40.0);
            let ha = H_RANGE[0] + (H_RANGE[1] - H_RANGE[0]) * v;
            if (ha).abs() < 1e-9 || (ha - core::f64::consts::FRAC_PI_2).abs() < 1e-9 {
                continue;
            }
            let res = latent_h.validate(u, v);
            assert_eq!(
                res == ValidationResult::Valid,
                expect(alpha_ok(ha), true),
                "(z_q0, α): validate({u}, {v}) = {res:?}, the conjunction disagrees"
            );
        }
    }
}

#[test]
fn qa_m213_mixed_feasibility() {
    check_conjunction(|h, v| h && v);
}

negative_control!(
    qa_m213_mixed_feasibility,
    "feasibility is not α's check alone: the p_ρ axis's must be inherited too",
    expected = "the conjunction says valid",
    check_conjunction(|h, _| h)
);

// ── REQ-DEC-006: every chart declares requires_per_pixel_mass ─────────────────────────────────────────────────────

/// The distinct mass triples `decode` gives at a 9 × 9 lattice of `[0,1]²`.
fn mass_set(decode: &dyn Fn(f64, f64) -> Decoded<f64>) -> usize {
    let mut out = Vec::new();
    for i in 0..=8 {
        for j in 0..=8 {
            out.push(
                decode(i as f64 / 8.0, j as f64 / 8.0)
                    .ic
                    .m
                    .map(f64::to_bits),
            );
        }
    }
    out.sort_unstable();
    out.dedup();
    out.len()
}

/// The chart's decode as the decode stage runs it, [`ChartDecode::decode_at`].
fn stage<C: ChartDecode<Param = f64>>(chart: &C) -> impl Fn(f64, f64) -> Decoded<f64> + '_ {
    move |u, v| chart.decode_at::<Defaults, f64>(u, v)
}

/// `flag`, a chart's declaration, is `want`, and matches its masses under `decode`: declared false, one mass triple at
/// every pixel; declared true and `varies`, more than one.
fn check_flag(
    name: &str,
    flag: bool,
    decode: &dyn Fn(f64, f64) -> Decoded<f64>,
    want: bool,
    varies: bool,
) {
    assert_eq!(flag, want, "{name}: requires_per_pixel_mass is {flag}");
    let n = mass_set(decode);
    if !flag {
        assert_eq!(
            n, 1,
            "{name}: declares no per-pixel mass but its masses vary"
        );
    }
    if varies {
        assert!(n > 1, "{name}: one mass triple over the lattice");
    }
}

fn invariant() -> Invariant<f64> {
    Invariant {
        z_cfg: [0.2, -0.3],
        z_mass: [0.4, -0.1],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    }
}

fn check_flags(simplex_flag: bool) {
    let simplex = ternary::<f64>(0.5, eps_m());
    check_flag(
        "MassSimplex",
        simplex_flag,
        &stage(&simplex),
        simplex_flag,
        true,
    );
    let burrau = Burrau {
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
        k_max: K_MAX.number(),
        gamma_k: GAMMA_K.number(),
    };
    check_flag(
        "Burrau",
        burrau.requires_per_pixel_mass(),
        &stage(&burrau),
        true,
        true,
    );
    let plane = EuclidPlane {
        m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
    };
    check_flag(
        "EuclidPlane",
        plane.requires_per_pixel_mass(),
        &stage(&plane),
        true,
        true,
    );
    let mu1_alpha: Mixed<ZMu1, Alpha, f64> = Mixed::new(FROZEN, [-2.0, 2.0], H_RANGE);
    check_flag(
        "Mixed<z_mu1, alpha>",
        mu1_alpha.requires_per_pixel_mass(),
        &stage(&mu1_alpha),
        true,
        true,
    );
    let alpha_mu2: Mixed<Alpha, ZMu2, f64> = Mixed::new(FROZEN, H_RANGE, [-2.0, 2.0]);
    check_flag(
        "Mixed<alpha, z_mu2>",
        alpha_mu2.requires_per_pixel_mass(),
        &stage(&alpha_mu2),
        true,
        true,
    );
    let mu1_mu2: Mixed<ZMu1, ZMu2, f64> = Mixed::new(FROZEN, [-2.0, 2.0], [-1.0, 1.5]);
    check_flag(
        "Mixed<z_mu1, z_mu2>",
        mu1_mu2.requires_per_pixel_mass(),
        &stage(&mu1_mu2),
        true,
        true,
    );
    // No mass axis: whatever each declares must agree with its masses.
    let a_p = alpha_prho::<f64>(H_RANGE, V_RANGE);
    let flag = a_p.requires_per_pixel_mass();
    check_flag("Mixed<alpha, p_rho_x>", flag, &stage(&a_p), flag, false);
    let le = InvariantLE(invariant());
    check_flag(
        "InvariantLE",
        le.requires_per_pixel_mass(),
        &stage(&le),
        le.requires_per_pixel_mass(),
        false,
    );
    let lk = InvariantLK(invariant());
    check_flag(
        "InvariantLK",
        lk.requires_per_pixel_mass(),
        &stage(&lk),
        lk.requires_per_pixel_mass(),
        false,
    );
    let sphere = ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.3, -0.2],
        z_mom: [0.0; 4],
    };
    let sphere_decode = |u: f64, v: f64| Chart::decode::<Defaults, f64>(&sphere, u, v);
    let flag = sphere.requires_per_pixel_mass();
    check_flag("ShapeSphere", flag, &sphere_decode, flag, false);
    // The latent chart with a mass direction must declare it.
    let latent = kernel::chart::Latent {
        z0: [0.0; 8],
        q1: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        q2: [0.0; 8],
    };
    check_flag(
        "Latent (mass direction)",
        latent.requires_per_pixel_mass(),
        &stage(&latent),
        true,
        true,
    );
}

#[test]
fn qa_m213_per_pixel_mass_flags() {
    let declared = ternary::<f64>(0.5, eps_m()).requires_per_pixel_mass();
    assert!(declared, "MassSimplex: requires_per_pixel_mass is false");
    check_flags(declared);
}

negative_control!(
    qa_m213_per_pixel_mass_flags,
    "a ternary plot declared without per-pixel mass is refused: its masses vary",
    expected = "declares no per-pixel mass but its masses vary",
    check_flags(false)
);
