//! Lookup (inverse_encode_contract Part 6, "Lookup" and § "Chart-aware validation"; R-13; TASK-M2-19): the exact and
//! partial forms and the lock and slice they give (REQ-ENC-012), the three layers in order with each remedy and each
//! failure (REQ-ENC-013), and coincident bodies, clamped and never rejected (REQ-ENC-019). Each test registers its
//! negative control (R-176).

use engine::lookup::{
    changed, lookup, lookup_partial, residuals, sanity, slice, Class, Exact, Failed, Layer, Locked,
    Moved, Notices, Partial, Rejected, Tolerances, ORDER,
};
use kernel::chart::mixed::Coordinate;
use kernel::chart::{
    BlockTouch, Burrau, BurrauThetaK, Chart, ChartDecode, ChartOut, Constraint, Float, Latent,
    ValidationResult,
};
use kernel::decode::{self, Decoded, Ic};
use kernel::encode::by_kind::{invariants, ByKind, Coord, Invariants, Landing};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::{Defaults, LinkReal, Selection};
use kernel::payload::PayloadReal;
use validation::negative_control;
use validation::roundtrip::{charts, phys};

type Result = std::result::Result<Locked, Rejected>;

/// The curve tolerance: REQ-ENC-026's proposed `k = 128` (`fixtures/gates/curve_projection/measurement.toml`) at f64.
fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

fn exact<C: ByKind<Param = f64>>(chart: &C, entry: Exact) -> Result {
    lookup::<Defaults, C, _>(chart, &entry, phys, &tol())
}

fn latent_chart() -> Latent<f64> {
    charts::latent::<f64>()
}

fn burrau() -> Burrau<f64> {
    charts::burrau_full::<f64>()
}

/// The free decode of `z`.
fn d(z: [f64; 8]) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&decode::Latent::new(z)).ic
}

fn e(x: &Ic<f64>) -> Encoded<f64> {
    encode::<Defaults, f64>(x)
}

/// Body `i`'s and `j`'s positions `r` with each rotated by `angle` and moved by `shift`, every momentum rotated too.
fn gauge(x: &Ic<f64>, angle: f64, shift: [f64; 2]) -> Ic<f64> {
    let (s, c) = angle.sin_cos();
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    Ic {
        m: x.m,
        r: x.r.map(|v| {
            let w = rot(v);
            [w[0] + shift[0], w[1] + shift[1]]
        }),
        p: x.p.map(rot),
    }
}

/// A generic state: unequal masses, a scalene triangle below its own `ρ` (so `C` mirrors it), moving.
fn generic() -> Ic<f64> {
    Ic {
        m: [0.5, 0.3, 0.2],
        r: [[0.1, 0.2], [1.3, 0.4], [0.5, -0.9]],
        p: [[0.15, -0.05], [-0.1, 0.2], [-0.05, -0.15]],
    }
}

// ── REQ-ENC-012: the forms ─────────────────────────────────────────────────────────────────────────────────────────

/// Checks the exact forms: `(2, 1)`, `(3, 4, 5)` and `(6, 8, 10)` lock the classical Burrau system at rest on the
/// `(ν, K)` chart at `ν = ½`, `K = 0`; each entry's lock decodes to `C` of the entered state; a raw `z` is kept bit for
/// bit; a physical state's lock is `E(x)`'s `z` with its notices, its masses read as proportions.
fn check_exact(entry: Exact) {
    let chart = burrau();
    let lock = exact(&chart, entry).expect("an exact Burrau entry is refused");
    let x = kernel::burrau::euclid(2, 1)
        .expect("(2, 1)")
        .burrau::<f64>();
    let x = Ic {
        m: x.m,
        r: x.r,
        p: [[0.0; 2]; 3],
    };
    assert_eq!(
        lock.z,
        e(&x).z.z,
        "the lock is not E of the (3, 4, 5) system"
    );
    let Landing::Chart { u, v } = lock.landing else {
        panic!("the (3, 4, 5) system does not land on the (ν, K) chart");
    };
    assert!(
        (chart.nu(u) - 0.5).abs() < 1e-12,
        "lands at ν = {}",
        chart.nu(u)
    );
    assert_eq!(v, 0.0, "a rest start lands at K = 0");
    assert_eq!(lock.slice, None, "an exact entry sets a slice");
    assert!(
        !lock.notices.lookup_clamped(),
        "the (3, 4, 5) system is clamped"
    );
}

#[test]
fn lookup_forms_pythagorean_and_triple() {
    check_exact(Exact::Pythagorean { m: 2, n: 1 });
    check_exact(Exact::Triple([3, 4, 5]));
    check_exact(Exact::Triple([6, 8, 10]));
}

negative_control!(
    lookup_forms_pythagorean_and_triple,
    "the leg-swapped triple (4, 3, 5) is a different system, so its lock is not (3, 4, 5)'s",
    expected = "the lock is not E of the (3, 4, 5) system",
    check_exact(Exact::Triple([4, 3, 5]))
);

/// Checks the entry refusals: each entry is refused before the ladder, naming `failed`.
fn check_entry(entry: Exact, failed: Failed) {
    let r = exact(&latent_chart(), entry).expect_err("the entry is accepted");
    assert_eq!(
        r,
        Rejected {
            layer: Layer::Entry,
            failed
        },
        "{entry:?}"
    );
    assert!(r.to_string().contains("Entry"), "{r}");
}

#[test]
fn lookup_forms_entry_refused() {
    check_entry(Exact::Pythagorean { m: 2, n: 2 }, Failed::NotPrimitive);
    check_entry(Exact::Pythagorean { m: 3, n: 1 }, Failed::NotPrimitive);
    check_entry(Exact::Triple([3, 4, 6]), Failed::NotATriple);
    check_entry(Exact::Triple([0, 5, 5]), Failed::NotATriple);
    check_entry(Exact::Triple([5, 0, 5]), Failed::NotATriple);
    let mut z = [0.1; 8];
    z[5] = f64::NAN;
    check_entry(Exact::Raw(z), Failed::NonFinite);
    let mut x = generic();
    x.p[2][1] = f64::INFINITY;
    check_entry(Exact::Physical(x), Failed::NonFinite);
    let mut x = generic();
    x.r[0][0] = f64::NAN;
    check_entry(Exact::Physical(x), Failed::NonFinite);
    let mut x = generic();
    x.m[1] = f64::NAN;
    check_entry(Exact::Physical(x), Failed::NonFinite);
    for m in [0.0, -0.2] {
        let mut x = generic();
        x.m[2] = m;
        check_entry(Exact::Physical(x), Failed::MassNotPositive);
    }
}

negative_control!(
    lookup_forms_entry_refused,
    "a valid triple is not refused",
    expected = "the entry is accepted",
    check_entry(Exact::Triple([5, 12, 13]), Failed::NotATriple)
);

/// Checks a raw `z`: on the latent chart, every control the clamp leaves is kept bit for bit and `z[0] = big`, past
/// its clamp, is snapped to the clamp's image and named as moved.
fn check_raw(big: f64) {
    let mut z = [0.4, -0.7, 0.3, 0.9, -1.1, 0.2, 0.5, -0.6];
    let lock = exact(&latent_chart(), Exact::Raw(z)).expect("a raw z is refused");
    assert_eq!(lock.z, z, "an interior raw z is not kept bit for bit");
    assert!(!lock.notices.lookup_clamped());
    z[0] = big;
    let lock = exact(&latent_chart(), Exact::Raw(z)).expect("a saturated raw z is refused");
    assert_eq!(lock.z[1..], z[1..], "the unclamped controls are not kept");
    let image = (1.0f64 - 1e-6).ln() - (1e-6f64).ln();
    assert!(
        (lock.z[0] - image).abs() < 1e-6,
        "z_α {} is not at its clamp's image {image}",
        lock.z[0]
    );
    assert_eq!(lock.notices.moved.controls, 1, "z_α is not named as moved");
    assert!(lock.notices.lookup_clamped());
}

#[test]
fn lookup_forms_raw() {
    check_raw(30.0);
}

negative_control!(
    lookup_forms_raw,
    "a raw z_α inside its clamp is not moved",
    expected = "is not at its clamp's image",
    check_raw(3.0)
);

/// Checks a physical state: the lock is `E(x)`'s `z`, the same under a gauge and at any scale of the masses, with
/// `lookup_mirrored`, `λ` and the landing on the latent chart's plane where it holds `x`.
fn check_physical(masses: [f64; 3]) {
    let x = generic();
    let lock = exact(&latent_chart(), Exact::Physical(x)).expect("a physical state is refused");
    let enc = e(&x);
    assert_eq!(lock.z, enc.z.z, "the lock is not E(x)");
    assert!(
        lock.notices.mirrored,
        "the state below its ρ is not mirrored"
    );
    assert_eq!(lock.notices.lambda, enc.report.lambda, "λ is not reported");
    assert!(!lock.notices.past_fold);
    let moved = gauge(&x, 0.7, [3.0, -2.0]);
    let again = exact(&latent_chart(), Exact::Physical(moved)).expect("refused");
    assert!(
        phys(&d(lock.z), &d(again.z)) < 1e-13,
        "a gauge moves the lock"
    );
    let scaled = Ic { m: masses, ..x };
    let lock2 = exact(&latent_chart(), Exact::Physical(scaled)).expect("refused");
    assert!(
        phys(&d(lock.z), &d(lock2.z)) < 1e-13,
        "the masses {masses:?} are not read as proportions"
    );
    // Upper: no mirror.
    let up = Ic {
        r: x.r.map(|v| [v[0], -v[1]]),
        p: x.p.map(|v| [v[0], -v[1]]),
        ..x
    };
    let lock = exact(&latent_chart(), Exact::Physical(up)).expect("refused");
    assert!(!lock.notices.mirrored, "the state above its ρ is mirrored");
    // On a latent plane through x, it lands on the chart at the plane's centre.
    let plane = Latent {
        z0: enc.z.z,
        q1: unit(0),
        q2: unit(1),
    };
    let lock = exact(&plane, Exact::Physical(x)).expect("refused");
    assert_eq!(
        lock.landing,
        Landing::Chart { u: 0.5, v: 0.5 },
        "x does not land on its plane"
    );
}

fn unit(k: usize) -> [f64; 8] {
    std::array::from_fn(|n| if n == k { 1.0 } else { 0.0 })
}

#[test]
fn lookup_forms_physical() {
    check_physical([2.5, 1.5, 1.0]);
}

negative_control!(
    lookup_forms_physical,
    "other mass proportions are another system",
    expected = "are not read as proportions",
    check_physical([2.5, 1.0, 1.5])
);

/// Checks the fold flag reaches the lock: the leg-swapped twin `(4, 3, 5)` on the folded `(θ, K)` chart is past the
/// fold, `(3, 4, 5)` is not.
fn check_fold(twin: [u64; 3]) {
    let chart: BurrauThetaK<f64> = charts::burrau_theta_k::<f64>();
    let lock = exact(&chart, Exact::Triple(twin)).expect("refused");
    assert!(
        lock.notices.past_fold,
        "{twin:?} is not flagged past the fold"
    );
    let lock = exact(&chart, Exact::Triple([3, 4, 5])).expect("refused");
    assert!(
        !lock.notices.past_fold,
        "(3, 4, 5) is flagged past the fold"
    );
}

#[test]
fn lookup_forms_fold() {
    check_fold([4, 3, 5]);
}

negative_control!(
    lookup_forms_fold,
    "(3, 4, 5) is inside the fold",
    expected = "is not flagged past the fold",
    check_fold([3, 4, 5])
);

fn partial<C: engine::lookup::Fill<Param = f64>>(chart: &C, p: Partial, nu: Option<f64>) -> Result {
    lookup_partial::<Defaults, C, _>(chart, &p, nu, phys, &tol())
}

/// Checks the partial forms on the latent chart: the blocks not given take their defaults, and the slice centred on
/// the lock spans the first two unspecified controls in `ORDER`: `expect[k]` the pair for `cases[k]`.
fn check_partial(expect: [(usize, usize); 5]) {
    let x = generic();
    let cases = [
        Partial::default(),
        Partial {
            masses: Some([2.0, 1.0, 1.0]),
            ..Partial::default()
        },
        Partial {
            positions: Some(x.r),
            momenta: Some(x.p),
            ..Partial::default()
        },
        Partial {
            masses: Some(x.m),
            positions: Some(x.r),
            ..Partial::default()
        },
        Partial {
            masses: Some(x.m),
            momenta: Some(x.p),
            ..Partial::default()
        },
    ];
    for (case, (i, j)) in cases.iter().zip(expect) {
        let lock = partial(&latent_chart(), *case, Some(5.0)).expect("a partial entry is refused");
        let s = lock.slice.expect("a partial entry sets no slice");
        assert_eq!(s.z0, lock.z, "the slice is not centred on the lock");
        assert_eq!(
            (s.q1, s.q2),
            (unit(i), unit(j)),
            "{case:?}: the slice basis"
        );
        if case.momenta.is_none() {
            assert_eq!(
                lock.z[2..6],
                [0.0; 4],
                "{case:?}: the momenta are not at rest"
            );
        }
        if case.positions.is_none() {
            assert!(
                lock.z[..2].iter().all(|v| v.abs() < 1e-12),
                "{case:?}: z_cfg {:?}",
                &lock.z[..2]
            );
        }
    }
    // The masses default to equal: z_μ = 0 under the defaults links; given [2, 1, 1], to its proportions.
    let lock = partial(&latent_chart(), cases[0], None).expect("refused");
    assert!(
        lock.z[6..].iter().all(|v| v.abs() < 1e-12),
        "z_μ {:?}",
        &lock.z[6..]
    );
    let lock = partial(&latent_chart(), cases[1], None).expect("refused");
    let m = d(lock.z).m;
    assert!(
        (m[0] - 0.5).abs() < 1e-12 && (m[1] - 0.25).abs() < 1e-12,
        "masses {m:?}"
    );
    // All three blocks given: the exact form's lock and no slice.
    let full = Partial {
        masses: Some(x.m),
        positions: Some(x.r),
        momenta: Some(x.p),
    };
    let lock = partial(&latent_chart(), full, None).expect("refused");
    assert_eq!(lock.slice, None, "a full specification sets a slice");
    assert_eq!(lock.z, e(&x).z.z);
}

#[test]
fn lookup_forms_partial() {
    check_partial([(0, 1), (0, 1), (6, 7), (2, 3), (0, 1)]);
}

negative_control!(
    lookup_forms_partial,
    "the momenta are not before the masses",
    expected = "the slice basis",
    check_partial([(0, 1), (0, 1), (2, 3), (2, 3), (0, 1)])
);

/// Checks the partial form on the `(ν, K)` chart: with nothing given it locks the Burrau system at `ν`, the locked one
/// or `½`, at rest, on the chart; masses given keep their own with the Burrau triangle; `ν` outside `(0, 1)` is
/// layer 2's `BurrauNu`, and the latent chart ignores `ν`.
fn check_partial_burrau(nus: [f64; 4]) {
    let chart = burrau();
    for (nu, want) in [(None, 0.5), (Some(0.3), 0.3)] {
        let lock = partial(&chart, Partial::default(), nu).expect("refused");
        let Landing::Chart { u, v } = lock.landing else {
            panic!("the Burrau system at ν = {want} does not land on its chart");
        };
        assert!(
            (chart.nu(u) - want).abs() < 1e-12,
            "lands at ν = {}",
            chart.nu(u)
        );
        assert_eq!(v, 0.0);
        assert_eq!(lock.slice.map(|s| (s.q1, s.q2)), Some((unit(0), unit(1))));
    }
    let given = Partial {
        masses: Some([1.0, 1.0, 1.0]),
        ..Partial::default()
    };
    let lock = partial(&chart, given, None).expect("refused");
    assert!(d(lock.z).m.iter().all(|m| (m - 1.0 / 3.0).abs() < 1e-12));
    assert_eq!(
        lock.landing,
        Landing::Latent,
        "equal masses are on the Burrau curve"
    );
    for nu in nus {
        let r = partial(&chart, Partial::default(), Some(nu))
            .expect_err("ν outside (0, 1) is accepted");
        assert_eq!(r.layer, Layer::Feasibility, "ν = {nu}");
        assert_eq!(
            r.failed,
            Failed::Constraint(Constraint::BurrauNu),
            "ν = {nu}"
        );
        partial(&latent_chart(), Partial::default(), Some(nu)).expect("the latent chart reads ν");
    }
    partial(&chart, Partial::default(), Some(1.0f64.next_down()))
        .expect("ν just below 1 is refused");
    partial(&chart, Partial::default(), Some(1e-3)).expect("ν = 0.001 is refused");
}

#[test]
fn lookup_forms_partial_burrau() {
    check_partial_burrau([0.0, 1.0, 1.5, -0.5]);
}

negative_control!(
    lookup_forms_partial_burrau,
    "ν = ½ is inside the family",
    expected = "ν outside (0, 1) is accepted",
    check_partial_burrau([0.5, 1.0, 1.5, -0.5])
);

/// Checks [`slice`]: no unspecified block gives none; otherwise the first two controls of the unspecified blocks in
/// `ORDER`.
fn check_slice(order: [Coordinate; 8]) {
    assert_eq!(ORDER, order, "the order");
    let z0 = [0.5; 8];
    let blocks = |config, momentum, mass| BlockTouch {
        config,
        momentum,
        mass,
    };
    assert_eq!(
        slice(z0, blocks(false, false, false)),
        None,
        "no block gives a slice"
    );
    for (free, i, j) in [
        (blocks(true, true, true), 0, 1),
        (blocks(true, false, false), 0, 1),
        (blocks(false, true, false), 2, 3),
        (blocks(false, false, true), 6, 7),
        (blocks(false, true, true), 6, 7),
        (blocks(true, true, false), 0, 1),
    ] {
        let s = slice(z0, free).expect("no slice");
        assert_eq!((s.z0, s.q1, s.q2), (z0, unit(i), unit(j)), "free {free:?}");
    }
}

#[test]
fn lookup_forms_slice() {
    use Coordinate::*;
    check_slice([ZAlpha, ZBeta, ZMu1, ZMu2, ZQ0, ZQ1, ZQ2, ZQ3]);
}

negative_control!(
    lookup_forms_slice,
    "the layout order is not the slice's",
    expected = "the order",
    check_slice({
        use Coordinate::*;
        [ZAlpha, ZBeta, ZQ0, ZQ1, ZQ2, ZQ3, ZMu1, ZMu2]
    })
);

// ── REQ-ENC-013: the ladder ────────────────────────────────────────────────────────────────────────────────────────

/// The canonical state at the latent controls `z_cfg`, equal masses, with Jacobi momenta `p_ρ`, `p_λ`: `p₂ = p_λ`,
/// `p₁ = p_ρ − (m₁/M₀₁)p_λ`, `p₀ = −p₁ − p₂`.
fn jacobi_state(z_cfg: [f64; 2], p_rho: [f64; 2], p_lambda: [f64; 2]) -> Ic<f64> {
    let mut z = [0.0; 8];
    z[..2].copy_from_slice(&z_cfg);
    let x = d(z);
    let w = x.m[1] / (x.m[0] + x.m[1]);
    let p1 = [p_rho[0] - w * p_lambda[0], p_rho[1] - w * p_lambda[1]];
    let p0 = [-p1[0] - p_lambda[0], -p1[1] - p_lambda[1]];
    Ic {
        p: [p0, p1, p_lambda],
        ..x
    }
}

/// Checks layer 1's clamp: `p_λx = 2.5` past `q_max` on a wide triangle, `E > 0` before and after, is clamped, `z_q2`
/// named, and locked at its clamp's image.
fn check_layer1_clamp(p: f64) {
    let x = jacobi_state([0.0, 0.0], [0.0, 0.0], [p, 0.0]);
    let lock = exact(&latent_chart(), Exact::Physical(x))
        .expect("a clamp that keeps the classes is refused");
    assert_eq!(
        lock.notices.moved.controls,
        1 << 4,
        "z_q2 is not the control moved"
    );
    assert!(lock.notices.lookup_clamped(), "lookup_clamped is not set");
    let q = 2.0 * (2.0 / (1.0 + (-lock.z[4]).exp()) - 1.0);
    assert!((q - 2.0).abs() < 1e-5, "q₂ = {q} is not at the clamp");
}

#[test]
fn lookup_validation_ladder_layer1_clamp() {
    check_layer1_clamp(2.5);
}

negative_control!(
    lookup_validation_ladder_layer1_clamp,
    "a momentum inside q_max is not clamped",
    expected = "z_q2 is not the control moved",
    check_layer1_clamp(1.5)
);

/// Checks layer 1's rejections: a clamp that changes `sign(E)`, a close pair whose `p_λ` far past `q_max` makes
/// `E > 0` and the clamp `E < 0`, names `Energy`; one that changes `sign(L_z)` names `Rotation`; each layer 1 with
/// its controls, before layer 3 even at a tolerance of 0.
fn check_layer1_reject(class: Class) {
    let energy_p = 10.0;
    let close = Ic {
        m: [1.0 / 3.0; 3],
        r: [[-0.0005, 0.0], [0.0005, 0.0], [0.0, 1.0]],
        p: [
            [-energy_p / 2.0, 0.0],
            [-energy_p / 2.0, 0.0],
            [energy_p, 0.0],
        ],
    };
    let canon = e(&close).canonical.ic;
    assert!(
        invariants(&canon).e > 0.0,
        "the close pair's E is not positive"
    );
    let strict = Tolerances {
        sanity: 0.0,
        ..tol()
    };
    let r = lookup::<Defaults, _, _>(&latent_chart(), &Exact::Physical(close), phys, &strict)
        .expect_err("a clamp that turns E negative is accepted");
    assert_eq!(
        r,
        Rejected {
            layer: Layer::Hypercube,
            failed: Failed::Clamp {
                controls: 1 << 4,
                class
            }
        }
    );
    // L_z = ρ_x p_ρy − λ_y p_λx: p_ρy = 2.5 clamped to 2 takes it from +0.25ρ_x to −0.25ρ_x.
    let x = jacobi_state([1.4, 0.0], [0.0, 1.0], [0.0, 0.0]);
    let (rho_x, lambda_y) = (
        x.r[1][0] - x.r[0][0],
        x.r[2][1] - (x.r[0][1] + x.r[1][1]) / 2.0,
    );
    let x = jacobi_state([1.4, 0.0], [0.0, 2.5], [2.25 * rho_x / lambda_y, 0.0]);
    assert!(
        invariants(&x).lz > 0.0,
        "L_z is not positive before the clamp"
    );
    let r =
        exact(&latent_chart(), Exact::Physical(x)).expect_err("a clamp that turns L_z is accepted");
    assert_eq!(
        r,
        Rejected {
            layer: Layer::Hypercube,
            failed: Failed::Clamp {
                controls: 1 << 3,
                class: Class::Rotation
            }
        }
    );
}

#[test]
fn lookup_validation_ladder_layer1_reject() {
    check_layer1_reject(Class::Energy);
}

negative_control!(
    lookup_validation_ladder_layer1_reject,
    "the close pair's clamp keeps the sense of rotation",
    expected = "class: Energy",
    check_layer1_reject(Class::Rotation)
);

/// A latent chart whose `validate` returns `result` at every point: layer 2's remedies, one fixture each.
struct Walled {
    latent: Latent<f64>,
    result: ValidationResult,
}

impl Chart for Walled {
    type Param = f64;
    fn map<F: Float + From<f64>>(&self, u: F, v: F) -> ChartOut<F> {
        self.latent.map(u, v)
    }
    fn name(&self) -> &str {
        "walled"
    }
    fn validate(&self, _u: f64, _v: f64) -> ValidationResult {
        self.result
    }
}

impl ChartDecode for Walled {
    fn decode_at<S, R>(&self, u: R, v: R) -> Decoded<R>
    where
        S: Selection,
        R: LinkReal + PayloadReal + kernel::chart::invariant::WarpReal + From<f64>,
    {
        self.latent.decode_at::<S, R>(u, v)
    }
}

impl ByKind for Walled {
    fn coords<S, R>(&self, e: &Encoded<R>) -> [Coord<R>; 2]
    where
        S: Selection,
        R: LinkReal + PayloadReal + From<f64>,
    {
        self.latent.coords::<S, R>(e)
    }
}

/// The latent plane through `x`'s `z` along `z_α`, `z_β`, walled by `result`.
fn walled(x: &Ic<f64>, result: ValidationResult) -> Walled {
    Walled {
        latent: Latent {
            z0: e(x).z.z,
            q1: unit(0),
            q2: unit(1),
        },
        result,
    }
}

/// Checks layer 2's three remedies on a state that lands at the plane's centre: a projection moves the lock to its
/// point and names its constraint, unclamped; a clamp moves it, naming the moved coordinate; a reject refuses at
/// layer 2, before layer 3 even at a tolerance of 0. `valid` is what the wall returns first.
fn check_layer2(valid: ValidationResult) {
    let x = generic();
    let lock = exact(&walled(&x, valid), Exact::Physical(x)).expect("refused");
    assert_eq!(
        lock.landing,
        Landing::Chart { u: 0.5, v: 0.5 },
        "the valid state is moved"
    );
    assert_eq!((lock.notices.projected, lock.notices.clamped), (None, None));
    let to = |u: f64, v: f64| {
        e(&walled(&x, valid).decode_at::<Defaults, f64>(u, v).ic)
            .z
            .z
    };
    let project = ValidationResult::Project {
        u: 0.25,
        v: 0.75,
        failed: Constraint::LzEParabola,
    };
    let lock = exact(&walled(&x, project), Exact::Physical(x)).expect("a projection is refused");
    assert_eq!(lock.landing, Landing::Chart { u: 0.25, v: 0.75 });
    assert_eq!(
        lock.z,
        to(0.25, 0.75),
        "the projected lock is not the point's z"
    );
    assert_eq!(lock.notices.projected, Some(Constraint::LzEParabola));
    assert!(
        !lock.notices.lookup_clamped(),
        "a projection sets lookup_clamped"
    );
    let clamp = ValidationResult::Clamp {
        u: 0.5,
        v: 1.0,
        moved: [false, true],
        failed: Constraint::MassBuffer,
    };
    let lock = exact(&walled(&x, clamp), Exact::Physical(x)).expect("a clamp is refused");
    assert_eq!(lock.landing, Landing::Chart { u: 0.5, v: 1.0 });
    assert_eq!(lock.z, to(0.5, 1.0));
    assert_eq!(lock.notices.clamped, Some(Constraint::MassBuffer));
    assert_eq!(
        lock.notices.moved.chart,
        [false, true],
        "the moved coordinate is not shown"
    );
    assert!(lock.notices.lookup_clamped());
    let reject = ValidationResult::Reject {
        failed: Constraint::MassPositive,
    };
    let strict = Tolerances {
        sanity: 0.0,
        ..tol()
    };
    let r = lookup::<Defaults, _, _>(&walled(&x, reject), &Exact::Physical(x), phys, &strict)
        .expect_err("a rejection is accepted");
    assert_eq!(
        r,
        Rejected {
            layer: Layer::Feasibility,
            failed: Failed::Constraint(Constraint::MassPositive)
        }
    );
}

#[test]
fn lookup_validation_ladder_layer2() {
    check_layer2(ValidationResult::Valid);
}

negative_control!(
    lookup_validation_ladder_layer2,
    "a wall that projects every point moves the valid state",
    expected = "the valid state is moved",
    check_layer2(ValidationResult::Project {
        u: 0.1,
        v: 0.1,
        failed: Constraint::LzEParabola
    })
);

/// Checks layer 3: the lock's decode at a tolerance below its residual is refused at layer 3, naming the residual
/// that fails; at the tolerance equal to its residuals it passes.
fn check_layer3(scale: f64) {
    let x = generic();
    let lock = exact(&latent_chart(), Exact::Physical(x)).expect("refused");
    let (com, momentum) = residuals(&d(lock.z));
    assert!(com.max(momentum) > 0.0, "the decode's residuals are 0");
    let at = |sanity: f64| {
        lookup::<Defaults, _, _>(
            &latent_chart(),
            &Exact::Physical(x),
            phys,
            &Tolerances { sanity, ..tol() },
        )
    };
    at(com.max(momentum) * scale).expect("layer 3 refuses at its residual");
    let r = at(com.max(momentum).next_down()).expect_err("layer 3 passes below its residual");
    assert_eq!(r.layer, Layer::Sanity);
    let named = if com >= momentum {
        Constraint::ComAtOrigin
    } else {
        Constraint::MomentumZero
    };
    assert_eq!(r.failed, Failed::Constraint(named));
}

#[test]
fn lookup_validation_ladder_layer3() {
    check_layer3(1.0);
}

negative_control!(
    lookup_validation_ladder_layer3,
    "below its residual the lock fails",
    expected = "layer 3 refuses at its residual",
    check_layer3(0.5)
);

/// Checks [`sanity`] and [`residuals`] on built states: each constraint at its edge, the masses first, the extents
/// floored at 1, a NaN failing.
fn check_sanity(tol: f64) {
    let base = Ic {
        m: [0.5, 0.25, 0.25],
        r: [[0.0, 0.0], [2.0, 0.0], [-2.0, 0.0]],
        p: [[0.0, 0.0], [1.0, 0.0], [-1.0, 0.0]],
    };
    assert_eq!(residuals(&base), (0.0, 0.0));
    assert_eq!(sanity(&base, 0.0), None);
    // ‖Σmᵢrᵢ‖ = 0.25·4 = 1 over R_x = 4; ‖Σpᵢ‖ = 0.5 over P_x = 1.
    let off = Ic {
        r: [[0.0, 0.0], [4.0, 0.0], [0.0, 0.0]],
        p: [[0.5, 0.0], [0.0, 0.0], [0.0, 0.0]],
        ..base
    };
    assert_eq!(residuals(&off), (0.25, 0.5), "the residuals");
    let small = Ic {
        r: [[0.0, 0.25], [0.0, 0.0], [0.0, 0.0]],
        p: [[0.0, 0.0], [0.0, 0.0], [0.0, 3.0]],
        ..base
    };
    assert_eq!(residuals(&small), (0.125, 1.0), "the extents");
    assert_eq!(sanity(&off, tol), Some(Constraint::ComAtOrigin));
    assert_eq!(sanity(&off, 0.25), Some(Constraint::MomentumZero));
    assert_eq!(sanity(&off, 0.5), None, "the tolerance's edge");
    assert_eq!(
        sanity(&off, 0.5f64.next_down()),
        Some(Constraint::MomentumZero)
    );
    for m in [0.0, -0.25, f64::NAN] {
        let x = Ic {
            m: [0.5, m, 0.25],
            ..base
        };
        assert_eq!(
            sanity(&x, 1.0),
            Some(Constraint::DecodedMassPositive),
            "m₁ = {m}"
        );
    }
    let mut x = base;
    x.p[1][1] = f64::NAN;
    assert_eq!(
        sanity(&x, 1.0),
        Some(Constraint::MomentumZero),
        "a NaN passes"
    );
    let mut x = base;
    x.r[1][1] = f64::NAN;
    assert_eq!(
        sanity(&x, 1.0),
        Some(Constraint::ComAtOrigin),
        "a NaN passes"
    );
}

#[test]
fn lookup_validation_ladder_sanity() {
    check_sanity(0.25f64.next_down());
}

negative_control!(
    lookup_validation_ladder_sanity,
    "a tolerance at the CoM's residual passes it",
    expected = "left: Some(MomentumZero)",
    check_sanity(0.25)
);

fn inv(lz: f64, e: f64) -> Invariants<f64> {
    Invariants {
        lz,
        k: 0.0,
        u: 0.0,
        e,
    }
}

/// Checks [`changed`]: a class is `sign` with 0 its own; a change of `L_z`'s is named first; an entered value that is
/// not finite has none; and the notices' `lookup_clamped` reads each moved field.
fn check_changed(tiny: f64) {
    assert_eq!(
        changed(&inv(0.0, -1.0), &inv(tiny, -1.0)),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(0.0, -1.0), &inv(-tiny, -1.0)),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(1.0, -1.0), &inv(-1.0, 1.0)),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(1.0, 0.0), &inv(2.0, tiny)),
        Some(Class::Energy)
    );
    assert_eq!(
        changed(&inv(1.0, 0.0), &inv(2.0, -tiny)),
        Some(Class::Energy)
    );
    assert_eq!(
        changed(&inv(1.0, -1.0), &inv(2.0, f64::NAN)),
        Some(Class::Energy)
    );
    assert_eq!(changed(&inv(1.0, -1.0), &inv(3.0, -2.0)), None);
    assert_eq!(changed(&inv(0.0, 0.0), &inv(0.0, 0.0)), None);
    assert_eq!(
        changed(&inv(1.0, f64::NEG_INFINITY), &inv(1.0, 5.0)),
        None,
        "−∞ has a class"
    );
    assert_eq!(
        changed(&inv(f64::NAN, -1.0), &inv(1.0, -1.0)),
        None,
        "NaN has a class"
    );
    let quiet = Notices {
        moved: Moved::default(),
        projected: None,
        clamped: None,
        mirrored: false,
        lambda: 1.0,
        past_fold: false,
    };
    assert!(!quiet.lookup_clamped());
    for moved in [
        Moved {
            controls: 1 << 7,
            ..Moved::default()
        },
        Moved {
            coincident: true,
            ..Moved::default()
        },
        Moved {
            chart: [true, false],
            ..Moved::default()
        },
        Moved {
            chart: [false, true],
            ..Moved::default()
        },
    ] {
        assert!(Notices { moved, ..quiet }.lookup_clamped(), "{moved:?}");
    }
    let t = Tolerances::register(3.0);
    assert_eq!(
        (t.eps_phys, t.curve_tol, t.sanity),
        (64.0 * f64::EPSILON, 3.0, 16.0 * f64::EPSILON)
    );
}

#[test]
fn lookup_validation_ladder_classes() {
    check_changed(f64::MIN_POSITIVE);
}

negative_control!(
    lookup_validation_ladder_classes,
    "0 to 0 changes no class",
    expected = "left: None",
    check_changed(0.0)
);

// ── REQ-ENC-019: coincident bodies ─────────────────────────────────────────────────────────────────────────────────

/// Checks R-13 for the pair `(i, j)` made to coincide in `generic()`: the lookup locks it, clamped, never refused.
fn check_coincident(pairs: &[(usize, usize)]) {
    for &(i, j) in pairs {
        for p in [0.0, 1.0, 3.0] {
            let mut x = generic();
            x.r[j] = x.r[i];
            x.p = x.p.map(|v| [p * v[0], p * v[1]]);
            let lock = exact(&latent_chart(), Exact::Physical(gauge(&x, 1.1, [0.4, 2.0])))
                .unwrap_or_else(|r| panic!("bodies {i} and {j} coincident are refused: {r}"));
            assert!(
                lock.notices.lookup_clamped(),
                "bodies {i} and {j} coincident are not clamped"
            );
            assert_eq!(
                lock.notices.moved.coincident,
                (i, j) == (0, 1),
                "the inner pair's flag"
            );
        }
    }
}

#[test]
fn lookup_coincident_clamped() {
    check_coincident(&[(0, 1), (0, 2), (1, 2)]);
}

negative_control!(
    lookup_coincident_clamped,
    "bodies apart are not clamped",
    expected = "are not clamped",
    check_coincident(&[(0, 0)])
);

/// Checks bodies within `r_coll` (10⁻³ by default; integrator_contract Part 7): a pair `sep` apart at `I ≈ 1` locks,
/// unrefused and unclamped, and its lock's pair is still within `r_coll`.
fn check_within(sep: f64) {
    let h = (4.5f64).sqrt();
    let x = Ic {
        m: [1.0 / 3.0; 3],
        r: [[-sep / 2.0, 0.0], [sep / 2.0, 0.0], [0.0, h]],
        p: [[0.1, -0.2], [-0.3, 0.1], [0.2, 0.1]],
    };
    let lock =
        exact(&latent_chart(), Exact::Physical(x)).expect("bodies within r_coll are refused");
    let y = d(lock.z);
    let gap = (y.r[1][0] - y.r[0][0]).hypot(y.r[1][1] - y.r[0][1]);
    assert!(
        gap < 1e-3,
        "the locked pair is {gap} apart, not within r_coll"
    );
    assert!(
        !lock.notices.lookup_clamped(),
        "bodies {sep} apart are clamped"
    );
}

#[test]
fn lookup_coincident_within_r_coll() {
    check_within(5e-4);
}

negative_control!(
    lookup_coincident_within_r_coll,
    "a pair 0.1 apart is not within r_coll",
    expected = "not within r_coll",
    check_within(0.1)
);
