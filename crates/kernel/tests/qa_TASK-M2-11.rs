//! QA tests for TASK-M2-11, written from the requirements (REQ-CHART-024, REQ-CHART-027, REQ-CHART-025, REQ-CHART-046's
//! feasibility, REQ-CHART-055's reasoning; chart_reference §4.2, §4.3, §4.5, §5.4; chart_decoder_contract Part 5; R-27,
//! R-104, R-157, R-407), not from the implementation. Every decode check runs at f64 and f32 natively (R-265).
//! - `qa_m211_theta_nu` (REQ-CHART-024): `ν(θ)` is `sec θ − tan θ` (computed here directly, not as `cos/(1 + sin)`),
//!   inverts `θ(ν) = arctan((1 − ν²)/(2ν))` on `(0, π/4]`, maps the fold onto `[√2 − 1, 1)`, and the triangle with
//!   §4.3's sides `(1 − ν², 2ν, 1 + ν²)` has acute angle `θ`.
//! - `qa_m211_theta_k` (REQ-CHART-024): `Φ_θ,K`'s decoded IC, read *geometrically*: right angle at body 0, the acute
//!   angle at body 2 equal to `θ(u) = θ_min + (θ_max − θ_min)u`, each mass the side opposite its body over the
//!   perimeter (§4.2), CoM at the origin, `I = 1` (§0.5), `Σp = 0`, `L_z(p) = 0`, `K(p) = K_max·v^γ_K`.
//! - `qa_m211_theta_lz` (REQ-CHART-024, REQ-CHART-046): `Φ_θ,L_z` at the register's proposed defaults: the same
//!   geometry, `K(p) = K` at every `v`, `L_z(p) = (2v − 1)·λ_L·√(2IK)`, `|L_z| ≤ √(2IK)` with `I` measured on the
//!   decode, and `validate` `Valid` on the whole grid, the edges of `[0,1]²` included.
//! - `qa_m211_delta_m` (REQ-CHART-027): the `v = 0` row's masses are the Burrau masses (computed here from the sides),
//!   the `v = 1` row's `m_target` (default equal masses, and a non-default target), the blend linear between, `Σm = 1`,
//!   every momentum 0, and the triangle the Burrau one (right angle at 0, angle `θ(u)` at 2) with the CoM of the
//!   *blended* masses at the origin.
//! - `qa_m211_flags`: `Φ_θ,K` and `Φ_θ,L_z` forbid energy normalisation, the δm strip (a rest start) allows it (§0.6,
//!   §4.5).
//! - `qa_m211_quotients` (REQ-CHART-025): a brute-force multiplicity count (does some shape appear twice in the chart's
//!   `ν` range, through the leg swap `ν ↦ (1 − ν)/(1 + ν)`, checked to swap the legs?) decides `DoubleCover` against
//!   bijective, on the Euclid plane's defaults, on several `(ν, K)` ranges and on the acute-angle charts; the values
//!   differ, each label shows its value and quotient ("shape × labelling" / "shape only", §4.5), and the shape-fraction
//!   guard refuses the full-range chart and admits the folded ones.
//! - `qa_m211_past_the_square` (REQ-CHART-055's reasoning, §5.4): `θ` past `0` gives `ν > 1`, a negative mass, and is
//!   refused; `θ` past `π/4` (the other leg ordering) is refused; δm past `v = 1` keeps `Σm = 1` and is valid until a
//!   mass reaches 0, then refused; `L_z` past its edge at `λ_L < 1` stays feasible, at `λ_L = 1` it is not `Valid`.
//!
//! **Tolerances.** The construction's `Σp`, `L_z` and `K` residuals are REQ-DEC-046's proposed per-precision
//! tolerances (as `invariant_construction.rs` states them: 4, 4 and 8 eps). The geometric bounds are rounding bounds of
//! a few operations on quantities of magnitude ≤ 1 (positions with `I = 1`, masses, angles ≤ π/2): [`RND`] = 64 eps
//! absolute. `ν(θ)`'s are 16 eps, a handful of operations with no cancellation on `(0, π/4]` (`sec θ ≤ √2`).
//!
//! **Domain** (pitfalls §10): the stateless decode, inputs held fixed; no trajectory. Each test has a registered
//! negative control naming the panic it expects (R-176, R-212).

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
use std::fmt::Debug;

use kernel::chart::burrau_acute::{nu_of, theta_of};
use kernel::chart::invariant::WarpReal;
use kernel::chart::{
    admit_shape_fraction, Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart,
    ChartDecode, Constraint, EuclidPlane, SystemImage, ValidationResult,
};
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::PayloadReal;
use validation::{
    negative_control, BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX,
    BURRAU_THETA_MIN, EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN, GAMMA_K, K_MAX,
};

/// Geometric rounding bound, eps, absolute (the module's note).
const RND: f64 = 64.0;
/// `|Σpᵢ|` in units of `eps·Σ|pᵢ|` (REQ-DEC-046, proposed).
const TOL_P: f64 = 4.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))` (REQ-DEC-046, proposed).
const TOL_L: f64 = 4.0;
/// `|K(p) − K*|` in units of `eps·K*` (REQ-DEC-046, proposed).
const TOL_K: f64 = 8.0;

/// The coordinates every check visits, edges included.
const GRID: [f64; 7] = [0.0, 0.125, 0.25, 0.5, 0.625, 0.875, 1.0];

trait Q: LinkReal + PayloadReal + WarpReal + Copy + Debug + Into<f64> + From<Self> {
    const EPS: f64;
    const LABEL: &'static str;
    fn of(x: f64) -> Self;
}

impl Q for f64 {
    const EPS: f64 = f64::EPSILON;
    const LABEL: &'static str = "f64";
    fn of(x: f64) -> Self {
        x
    }
}

impl Q for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const LABEL: &'static str = "f32";
    fn of(x: f64) -> Self {
        x as f32
    }
}

// ── References, computed here from the corpus's formulae ────────────────────────────────────────────────────────

/// §4.3's normalised sides `(a, b, c) = (1 − ν², 2ν, 1 + ν²)`.
fn sides(nu: f64) -> [f64; 3] {
    [1.0 - nu * nu, 2.0 * nu, 1.0 + nu * nu]
}

/// §4.2's Burrau masses: each mass its opposite side over the perimeter, `(c, b, a)/(a + b + c)`.
fn burrau_masses(nu: f64) -> [f64; 3] {
    let [a, b, c] = sides(nu);
    let p = a + b + c;
    [c / p, b / p, a / p]
}

/// `ν(θ) = sec θ − tan θ`, directly.
fn nu_ref(theta: f64) -> f64 {
    1.0 / theta.cos() - theta.tan()
}

/// The angle at vertex `i` of the triangle `r`, between the other two.
fn angle_at(r: [[f64; 2]; 3], i: usize) -> f64 {
    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
    let e = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
    let f = [r[k][0] - r[i][0], r[k][1] - r[i][1]];
    (e[0] * f[1] - e[1] * f[0])
        .abs()
        .atan2(e[0] * f[0] + e[1] * f[1])
}

fn wide<R: Q>(d: &Decoded<R>) -> ([f64; 3], [[f64; 2]; 3], [[f64; 2]; 3]) {
    let m = d.ic.m.map(Into::into);
    let r = d.ic.r.map(|x| x.map(Into::into));
    let p = d.ic.p.map(|x| x.map(Into::into));
    (m, r, p)
}

/// What a decode must be: the triangle at acute angle `theta`, the masses `m`, `L_z` and `K`.
struct Want {
    theta: f64,
    m: [f64; 3],
    lz: f64,
    k: f64,
}

/// Checks the decode `d` against `want` (the module's note).
fn check_decode<R: Q>(d: &Decoded<R>, want: &Want, at: &str) {
    let eps = R::EPS;
    let tag = format!("{} at {at}", R::LABEL);
    let (m, r, p) = wide(d);
    for (i, (got, w)) in m.iter().zip(want.m).enumerate() {
        assert!(
            (got - w).abs() <= RND * eps,
            "{tag}: mass {i} = {got} is not {w}"
        );
    }
    assert!(
        (m.iter().sum::<f64>() - 1.0).abs() <= RND * eps,
        "{tag}: Σm ≠ 1"
    );
    assert!(
        (angle_at(r, 0) - FRAC_PI_2).abs() <= RND * eps,
        "{tag}: the right angle is not at body 0: {}",
        angle_at(r, 0)
    );
    assert!(
        (angle_at(r, 2) - want.theta).abs() <= RND * eps,
        "{tag}: the acute angle at body 2 is {}, not θ = {}",
        angle_at(r, 2),
        want.theta
    );
    let com = [0, 1].map(|x| (0..3).map(|i| m[i] * r[i][x]).sum::<f64>());
    assert!(
        com.iter().all(|c| c.abs() <= RND * eps),
        "{tag}: the CoM {com:?} is not at the origin"
    );
    let inertia: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum();
    assert!(
        (inertia - 1.0).abs() <= RND * eps,
        "{tag}: I = {inertia} is not 1"
    );
    let sum = [0, 1].map(|x| (0..3).map(|i| p[i][x]).sum::<f64>());
    let scale: f64 = p.iter().flatten().map(|x| x.abs()).sum();
    assert!(
        sum.iter()
            .all(|s| s.abs() <= TOL_P * eps * scale.max(f64::MIN_POSITIVE)),
        "{tag}: Σp = {sum:?}"
    );
    let lz: f64 = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    let kin: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    assert!(
        (lz - want.lz).abs() <= TOL_L * eps * (want.lz.abs() + (2.0 * want.k).sqrt()),
        "{tag}: L_z(p) = {lz}, not {}",
        want.lz
    );
    assert!(
        (kin - want.k).abs() <= TOL_K * eps * want.k,
        "{tag}: K(p) = {kin}, not {}",
        want.k
    );
}

fn theta_range() -> [f64; 2] {
    [BURRAU_THETA_MIN.number(), BURRAU_THETA_MAX.number()]
}

/// `θ(u)` on the chart's own (precision-rounded) range.
fn theta_lin<R: Q>(range: [R; 2], u: f64) -> f64 {
    let [lo, hi]: [f64; 2] = range.map(Into::into);
    lo + (hi - lo) * u
}

// ── qa_m211_theta_nu (REQ-CHART-024) ─────────────────────────────────────────────────────────────────────────────

type Fun = fn(f64) -> f64;

/// `nu` is `sec − tan` and `theta` inverts it on the fold (the module's note).
fn check_theta_nu(nu: Fun, theta: Fun) {
    let iso = 2f64.sqrt() - 1.0;
    let n = 4096;
    let mut thetas: Vec<f64> = (1..=n).map(|k| FRAC_PI_4 * k as f64 / n as f64).collect();
    thetas.extend([
        1e-6,
        1e-3,
        BURRAU_THETA_MIN.number(),
        BURRAU_THETA_MAX.number(),
    ]);
    let mut last = f64::INFINITY;
    let mut sorted = thetas.clone();
    sorted.sort_by(f64::total_cmp);
    for t in sorted {
        let v = nu(t);
        assert!(
            (v - nu_ref(t)).abs() <= 16.0 * f64::EPSILON,
            "ν({t}) = {v} is not sec θ − tan θ = {}",
            nu_ref(t)
        );
        assert!(
            (theta(v) - t).abs() <= 16.0 * f64::EPSILON,
            "θ(ν({t})) = {} does not invert ν",
            theta(v)
        );
        let [a, b, _] = sides(v);
        assert!(
            (a.atan2(b) - t).abs() <= 16.0 * f64::EPSILON,
            "the triangle at ν({t}) has acute angle {}",
            a.atan2(b)
        );
        assert!(
            v >= iso - 4.0 * f64::EPSILON && v < 1.0,
            "ν({t}) = {v} is outside [√2 − 1, 1)"
        );
        assert!(v < last, "ν is not decreasing across the fold at θ = {t}");
        last = v;
    }
    assert!(
        (nu(FRAC_PI_4) - iso).abs() <= 4.0 * f64::EPSILON,
        "ν(π/4) is not √2 − 1"
    );
    assert!(
        (nu(0.75f64.atan()) - 0.5).abs() <= 4.0 * f64::EPSILON,
        "ν(arctan(3/4)) is not the classical 1/2"
    );
}

#[test]
fn qa_m211_theta_nu() {
    check_theta_nu(nu_of::<f64>, theta_of::<f64>);
    // f32: the same source, within f32's rounding.
    for k in 1..=512 {
        let t = FRAC_PI_4 * k as f64 / 512.0;
        let v = nu_of(t as f32) as f64;
        assert!(
            (v - nu_ref(t as f32 as f64)).abs() <= 16.0 * f32::EPSILON as f64,
            "f32 ν({t}) = {v}"
        );
        let back = theta_of(v as f32) as f64;
        assert!(
            (back - t).abs() <= 16.0 * f32::EPSILON as f64,
            "f32 θ(ν({t})) = {back}"
        );
    }
}

negative_control!(
    qa_m211_theta_nu,
    "sec θ + tan θ, the wrong sign, is not the acute-angle axis",
    expected = "is not sec θ − tan θ",
    check_theta_nu(|t| 1.0 / t.cos() + t.tan(), theta_of::<f64>)
);

// ── qa_m211_theta_k (REQ-CHART-024) ──────────────────────────────────────────────────────────────────────────────

fn theta_k<R: Q>() -> BurrauThetaK<R> {
    BurrauThetaK {
        theta_range: theta_range().map(R::of),
        k_max: R::of(K_MAX.number()),
        gamma_k: R::of(GAMMA_K.number()),
    }
}

fn check_theta_k<R: Q>(chart: &BurrauThetaK<R>, expect_range: [f64; 2])
where
    f64: From<R>,
{
    let (k_max, gamma): (f64, f64) = (chart.k_max.into(), chart.gamma_k.into());
    let range: [R; 2] = expect_range.map(R::of);
    for &u in &GRID {
        for &v in &GRID {
            let theta = theta_lin(range, u);
            let d = chart.decode_at::<Defaults, R>(R::of(u), R::of(v));
            let want = Want {
                theta,
                m: burrau_masses(nu_ref(theta)),
                lz: 0.0,
                k: k_max * v.powf(gamma),
            };
            check_decode(&d, &want, &format!("Φ_θ,K ({u}, {v})"));
        }
    }
}

#[test]
fn qa_m211_theta_k() {
    check_theta_k(&theta_k::<f64>(), theta_range());
    check_theta_k(&theta_k::<f32>(), theta_range());
    // Another range and warp, inside the fold.
    let other = BurrauThetaK {
        theta_range: [0.2, 0.6],
        k_max: 0.5,
        gamma_k: 1.5,
    };
    check_theta_k(&other, [0.2, 0.6]);
}

negative_control!(
    qa_m211_theta_k,
    "a θ axis swept backwards (θ_max at u = 0) is not θ(u) = θ_min + (θ_max − θ_min)u: its masses are θ_max's",
    expected = "f64 at Φ_θ,K (0, 0): mass 0 =",
    check_theta_k(
        &BurrauThetaK {
            theta_range: [0.6, 0.2],
            k_max: 0.5,
            gamma_k: 1.5,
        },
        [0.2, 0.6]
    )
);

negative_control!(
    qa_m211_theta_k_lz,
    "a decode with L_z ≠ 0 (Φ_θ,L_z at v = 1) fails Φ_θ,K's L_z = 0",
    expected = "L_z(p) =",
    {
        let lz = theta_lz::<f64>();
        let theta = theta_lin(lz.theta_range, 0.5);
        check_decode(
            &lz.decode_at::<Defaults, f64>(0.5, 1.0),
            &Want {
                theta,
                m: burrau_masses(nu_ref(theta)),
                lz: 0.0,
                k: BURRAU_THETA_LZ_K.number(),
            },
            "control",
        )
    }
);

negative_control!(
    qa_m211_theta_k_angle,
    "a decode whose acute angle is not the one asked for fails the geometric check, masses aside",
    expected = "the acute angle at body 2 is",
    {
        let k = theta_k::<f64>();
        let theta = theta_lin(k.theta_range, 0.5);
        check_decode(
            &k.decode_at::<Defaults, f64>(0.5, 0.5),
            &Want {
                theta: theta + 1e-3,
                m: burrau_masses(nu_ref(theta)),
                lz: 0.0,
                k: K_MAX.number() * 0.25,
            },
            "control",
        )
    }
);

// ── qa_m211_theta_lz (REQ-CHART-024, REQ-CHART-046) ──────────────────────────────────────────────────────────────

fn theta_lz<R: Q>() -> BurrauThetaLz<R> {
    BurrauThetaLz {
        theta_range: theta_range().map(R::of),
        k: R::of(BURRAU_THETA_LZ_K.number()),
        lz_fraction: R::of(BURRAU_THETA_LZ_FRACTION.number()),
    }
}

/// `Φ_θ,L_z`'s decodes hold `K` at `k_fixed` and sweep `L_z` linearly, feasibly.
fn check_theta_lz<R: Q, C: ChartDecode<Param = R>>(
    chart: &C,
    range: [R; 2],
    k_fixed: f64,
    fraction: f64,
) {
    let root = (2.0 * k_fixed).sqrt();
    for &u in &GRID {
        for &v in &GRID {
            let theta = theta_lin(range, u);
            let d = chart.decode_at::<Defaults, R>(R::of(u), R::of(v));
            let want = Want {
                theta,
                m: burrau_masses(nu_ref(theta)),
                lz: (2.0 * v - 1.0) * fraction * root,
                k: k_fixed,
            };
            let at = format!("Φ_θ,L_z ({u}, {v})");
            check_decode(&d, &want, &at);
            let (m, r, p) = wide(&d);
            let inertia: f64 = (0..3)
                .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
                .sum();
            let lz: f64 = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
            let kin: f64 = (0..3)
                .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
                .sum();
            assert!(
                lz.abs() <= (2.0 * inertia * kin).sqrt() * (1.0 + TOL_L * R::EPS),
                "{}: |L_z| = {} exceeds √(2IK) at {at}",
                R::LABEL,
                lz.abs()
            );
        }
    }
}

#[test]
fn qa_m211_theta_lz() {
    let c64 = theta_lz::<f64>();
    let c32 = theta_lz::<f32>();
    let (k, f) = (
        BURRAU_THETA_LZ_K.number(),
        BURRAU_THETA_LZ_FRACTION.number(),
    );
    // REQ-CHART-046's proposal is inside the feasible band: K > 0, λ_L ∈ (0, 1].
    assert!(k > 0.0 && f > 0.0 && f <= 1.0, "K = {k}, λ_L = {f}");
    check_theta_lz(&c64, c64.theta_range, k, f);
    check_theta_lz(&c32, c32.theta_range, k, f);
    for &u in &GRID {
        for &v in &GRID {
            assert_eq!(
                c64.validate(u, v),
                ValidationResult::Valid,
                "Φ_θ,L_z at the defaults is not feasible at ({u}, {v})"
            );
        }
    }
}

negative_control!(
    qa_m211_theta_lz,
    "Φ_θ,K, whose K sweeps with v, does not hold K fixed",
    expected = "K(p) =",
    {
        let c = BurrauThetaK {
            theta_range: theta_range(),
            k_max: BURRAU_THETA_LZ_K.number(),
            gamma_k: 2.0,
        };
        check_theta_lz(&c, c.theta_range, BURRAU_THETA_LZ_K.number(), 0.0)
    }
);

// ── qa_m211_delta_m (REQ-CHART-027) ──────────────────────────────────────────────────────────────────────────────

/// The δm strip's rows: `(1 − v)·m_Burrau + v·target`, at rest, on the Burrau triangle (the module's note).
fn check_delta_m<R: Q>(chart: &BurrauThetaDeltaM<R>, target: [f64; 3])
where
    f64: From<R>,
{
    for &u in &GRID {
        let theta = theta_lin(chart.theta_range, u);
        let mb = burrau_masses(nu_ref(theta));
        for &v in &[0.0, 0.25, 0.5, 1.0] {
            let d = chart.decode_at::<Defaults, R>(R::of(u), R::of(v));
            let m = std::array::from_fn(|i| (1.0 - v) * mb[i] + v * target[i]);
            check_decode(
                &d,
                &Want {
                    theta,
                    m,
                    lz: 0.0,
                    k: 0.0,
                },
                &format!("δm ({u}, {v})"),
            );
            let (_, _, p) = wide(&d);
            assert!(
                p.iter().flatten().all(|&x| x == 0.0),
                "{}: δm ({u}, {v}) is not at rest: {p:?}",
                R::LABEL
            );
        }
    }
}

#[test]
fn qa_m211_delta_m() {
    let third = 1.0 / 3.0;
    assert_eq!(
        BurrauThetaDeltaM::<f64>::equal_masses(),
        [third; 3],
        "the default target is not equal masses"
    );
    let def64 = BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    };
    let def32 = BurrauThetaDeltaM {
        theta_range: theta_range().map(|x| x as f32),
        m_target: BurrauThetaDeltaM::<f32>::equal_masses(),
    };
    check_delta_m(&def64, [third; 3]);
    check_delta_m(&def32, [third; 3]);
    let other = [0.5, 0.3, 0.2];
    check_delta_m(
        &BurrauThetaDeltaM {
            theta_range: [0.1, 0.7],
            m_target: other,
        },
        other,
    );
}

negative_control!(
    qa_m211_delta_m,
    "a strip toward (0.5, 0.3, 0.2) does not end at equal masses",
    expected = "is not",
    check_delta_m(
        &BurrauThetaDeltaM {
            theta_range: theta_range(),
            m_target: [0.5, 0.3, 0.2],
        },
        [1.0 / 3.0; 3]
    )
);

// ── qa_m211_flags ────────────────────────────────────────────────────────────────────────────────────────────────

fn check_flags(got: [bool; 3]) {
    assert_eq!(
        got,
        [true, true, false],
        "forbids_energy_normalisation of (Φ_θ,K, Φ_θ,L_z, δm)"
    );
}

#[test]
fn qa_m211_flags() {
    let dm = BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    };
    check_flags([
        theta_k::<f64>().forbids_energy_normalisation(),
        theta_lz::<f64>().forbids_energy_normalisation(),
        dm.forbids_energy_normalisation(),
    ]);
}

negative_control!(
    qa_m211_flags,
    "a δm strip that forbade η_E would not be the rest start §4.5 says",
    expected = "forbids_energy_normalisation of",
    check_flags([true, true, true])
);

// ── qa_m211_quotients (REQ-CHART-025) ────────────────────────────────────────────────────────────────────────────

/// The leg swap `ν ↦ (1 − ν)/(1 + ν)`.
fn swap(nu: f64) -> f64 {
    (1.0 - nu) / (1.0 + nu)
}

/// Brute force: does some shape appear twice in `[lo, hi]`, as two labelled systems? A `ν` whose leg swap, a distinct
/// `ν'`, is also in range. The swap is checked to swap the legs (sides proportional to `(b, a, c)`).
fn covers_twice(lo: f64, hi: f64) -> bool {
    let n = 4000;
    let mut twice = false;
    for k in 0..=n {
        let nu = lo + (hi - lo) * k as f64 / n as f64;
        let s = swap(nu);
        let [a, b, c] = sides(nu);
        let [a2, b2, c2] = sides(s);
        assert!(
            (a2 / c2 - b / c).abs() < 1e-12 && (b2 / c2 - a / c).abs() < 1e-12,
            "ν ↦ (1 − ν)/(1 + ν) does not swap the legs at ν = {nu}"
        );
        twice |= (lo..=hi).contains(&s) && (s - nu).abs() > 1e-9;
    }
    twice
}

/// The three charts' images and labels, by what the brute force finds.
fn check_quotients(plane: SystemImage, plane_label: &str, folded: [(SystemImage, String); 3]) {
    let (lo, hi) = (EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number());
    let want = if covers_twice(lo, hi) {
        SystemImage::DoubleCover
    } else {
        SystemImage::Bijective
    };
    assert_eq!(
        want,
        SystemImage::DoubleCover,
        "the brute force finds no shape twice in the Euclid plane's default range"
    );
    assert_eq!(plane, want, "the full-range Euclid chart's system_image");
    assert!(
        admit_shape_fraction(plane).is_err(),
        "a shape fraction on the full-range chart is admitted"
    );
    assert!(
        plane_label.contains("DoubleCover") && plane_label.contains("shape × labelling"),
        "the full-range chart's label does not show DoubleCover and its quotient: {plane_label}"
    );
    for (image, label) in folded {
        assert_eq!(image, SystemImage::Bijective, "a folded chart: {label}");
        assert_ne!(
            image, plane,
            "the two charts' system_image values are equal"
        );
        assert!(
            admit_shape_fraction(image).is_ok(),
            "a shape fraction on a folded chart is refused: {label}"
        );
        assert!(
            label.contains("bijective") && label.contains("shape only"),
            "a folded chart's label does not show bijective and its quotient: {label}"
        );
        assert_ne!(label, plane_label, "the labels do not differ");
    }
}

fn plane() -> EuclidPlane<f64> {
    EuclidPlane {
        m_range: [EUCLID_M_MIN.number(), EUCLID_M_MAX.number()],
        nu_range: [EUCLID_NU_MIN.number(), EUCLID_NU_MAX.number()],
    }
}

fn folded() -> [(SystemImage, String); 3] {
    let k = theta_k::<f64>();
    let lz = theta_lz::<f64>();
    let dm = BurrauThetaDeltaM {
        theta_range: theta_range(),
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    };
    [
        (k.system_image(), k.label().to_string()),
        (lz.system_image(), lz.label().to_string()),
        (dm.system_image(), dm.label().to_string()),
    ]
}

#[test]
fn qa_m211_quotients() {
    let p = plane();
    check_quotients(p.system_image(), &p.label().to_string(), folded());
    // The folded θ ranges hold each shape once: θ(u) is inside the fold and strictly increasing, so no two `u` give
    // the same triangle up to the leg swap (θ and π/2 − θ are both in (0, π/4] only at π/4).
    let [lo, hi] = theta_range();
    assert!(lo > 0.0 && hi <= FRAC_PI_4 && lo < hi, "θ range {lo}..{hi}");
    // The (ν, K) chart: the brute force decides every range.
    for range in [
        [0.05, 0.3],
        [0.45, 0.95],
        [0.3, 0.6],
        [1.0 / 32.0, 31.0 / 32.0],
        [0.2, 0.41],
    ] {
        let c = Burrau {
            nu_range: range,
            k_max: 1.0,
            gamma_k: 2.0,
        };
        let want = if covers_twice(range[0], range[1]) {
            SystemImage::DoubleCover
        } else {
            SystemImage::Bijective
        };
        assert_eq!(c.system_image(), want, "the (ν, K) chart on {range:?}");
        assert_eq!(
            admit_shape_fraction(c.system_image()).is_err(),
            want == SystemImage::DoubleCover,
            "the guard on the (ν, K) chart on {range:?}"
        );
    }
}

negative_control!(
    qa_m211_quotients,
    "a full-range chart labelled bijective is not what the brute force finds",
    expected = "the full-range Euclid chart's system_image",
    check_quotients(SystemImage::Bijective, "x", folded())
);

// ── qa_m211_past_the_square (REQ-CHART-055's reasoning, §5.4) ────────────────────────────────────────────────────

fn rejected(r: ValidationResult) -> Option<Constraint> {
    match r {
        ValidationResult::Reject { failed } => Some(failed),
        _ => None,
    }
}

/// `validate_k` is `Φ_θ,K`'s `validate` on `range`.
fn check_past(
    range: [f64; 2],
    validate_k: impl Fn(f64, f64) -> ValidationResult,
    dm: &BurrauThetaDeltaM<f64>,
) {
    // θ past 0: ν > 1, a = 1 − ν² < 0, a negative mass (§5.4's reasoning), and refused.
    let below = -0.05;
    assert!(
        nu_ref(below) > 1.0 && burrau_masses(nu_ref(below))[2] < 0.0,
        "θ < 0 does not give a negative mass"
    );
    let u_below = (below - range[0]) / (range[1] - range[0]);
    assert_eq!(
        rejected(validate_k(u_below, 0.5)),
        Some(Constraint::BurrauTheta),
        "θ < 0 is not refused"
    );
    // θ past π/4: the other leg ordering, positive masses, outside the fold: refused.
    let above = 1.0;
    assert!(burrau_masses(nu_ref(above)).iter().all(|&m| m > 0.0));
    let u_above = (above - range[0]) / (range[1] - range[0]);
    assert_eq!(
        rejected(validate_k(u_above, 0.5)),
        Some(Constraint::BurrauTheta),
        "θ > π/4 is not refused"
    );
    // δm past v = 1: the blend continues, Σm = 1, valid while every mass is positive.
    let m = dm.masses(0.9, 1.2);
    assert!(
        (m.iter().sum::<f64>() - 1.0).abs() < 1e-12,
        "δm past v = 1 loses Σm = 1"
    );
    assert!(m.iter().all(|&x| x > 0.0));
    assert_eq!(
        dm.validate(0.9, 1.2),
        ValidationResult::Valid,
        "δm at v = 1.2"
    );
    // ... until a mass reaches 0: far enough below v = 0 the blend moves past m_Burrau until its lightest mass is
    // negative.
    let far = -20.0;
    assert!(dm.masses(0.9, far).iter().any(|&x| x <= 0.0));
    assert_eq!(
        rejected(dm.validate(0.9, far)),
        Some(Constraint::MassPositive),
        "δm with a negative mass is not refused"
    );
}

#[test]
fn qa_m211_past_the_square() {
    let k = BurrauThetaK {
        theta_range: [0.1, FRAC_PI_4],
        k_max: 1.0,
        gamma_k: 2.0,
    };
    let dm = BurrauThetaDeltaM {
        theta_range: [0.1, FRAC_PI_4],
        m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
    };
    check_past(k.theta_range, |u, v| k.validate(u, v), &dm);
    // L_z past its edge: λ_L = 1/2 continues through feasible systems; λ_L = 1 does not stay Valid.
    let half = BurrauThetaLz {
        theta_range: [0.1, FRAC_PI_4],
        k: 0.125,
        lz_fraction: 0.5,
    };
    assert_eq!(
        half.validate(0.5, 1.2),
        ValidationResult::Valid,
        "λ_L = 1/2 at v = 1.2"
    );
    assert_eq!(
        half.validate(0.5, -0.2),
        ValidationResult::Valid,
        "λ_L = 1/2 at v = −0.2"
    );
    let full = BurrauThetaLz {
        lz_fraction: 1.0,
        ..half
    };
    assert_ne!(
        full.validate(0.5, 1.2),
        ValidationResult::Valid,
        "λ_L = 1 at v = 1.2"
    );
    assert_ne!(
        full.validate(0.5, -0.2),
        ValidationResult::Valid,
        "λ_L = 1 at v = −0.2"
    );
}

negative_control!(
    qa_m211_past_the_square,
    "a validate that accepts every θ does not hatch θ past the fold",
    expected = "θ < 0 is not refused",
    check_past(
        [0.1, FRAC_PI_4],
        |_, _| ValidationResult::Valid,
        &BurrauThetaDeltaM {
            theta_range: [0.1, FRAC_PI_4],
            m_target: BurrauThetaDeltaM::<f64>::equal_masses(),
        }
    )
);
