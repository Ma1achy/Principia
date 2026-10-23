//! Lookup (inverse_encode_contract Part 6, "Lookup" and § "Chart-aware validation"; R-13; TASK-M2-19): the exact and
//! partial forms and the lock and slice they give (REQ-ENC-012), the three layers in order with each remedy and each
//! failure (REQ-ENC-013), and coincident bodies, clamped and never rejected (REQ-ENC-019). Each test registers its
//! negative control (R-176).

use engine::lookup::{
    after, changed, lookup, lookup_partial, moves, residuals, sanity, slice, Class, Classes, Exact,
    Extents, Failed, Layer, Locked, Moved, Moves, Notices, Partial, Rejected, Tolerances, ORDER,
};
use kernel::chart::mixed::Coordinate;
use kernel::chart::{
    BlockTouch, Burrau, BurrauThetaK, Chart, ChartDecode, ChartOut, Constraint, Float, Latent,
    ValidationResult,
};
use kernel::decode::{self, Decoded, Ic};
use kernel::encode::by_kind::{invariants, ByKind, Coord, Landing};
use kernel::encode::{encode, Encoded};
use kernel::generated::links::{Defaults, LinkReal, Selection};
use kernel::payload::PayloadReal;
use ledger::constants::ZERO_CLASS;
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

/// Checks a triple whose legs reach `u64::MAX`: `a² + b²` past `u128`'s range, and one just inside it, are refused
/// before the ladder as not a triple, with no overflow.
fn check_triple_wide(top: u64) {
    check_entry(Exact::Triple([top, top, 1]), Failed::NotATriple);
    check_entry(Exact::Triple([top, top, top]), Failed::NotATriple);
    check_entry(Exact::Triple([top, 1, top]), Failed::NotATriple);
    check_entry(Exact::Triple([1, top, top]), Failed::NotATriple);
}

#[test]
fn lookup_forms_triple_wide() {
    check_triple_wide(u64::MAX);
}

negative_control!(
    lookup_forms_triple_wide,
    "a valid triple's legs are not refused",
    expected = "the entry is accepted",
    check_entry(Exact::Triple([3, 4, 5]), Failed::NotATriple)
);

/// Checks a raw `z`: on the latent chart, every control the clamp leaves is kept bit for bit and `z[0] = big`, past
/// its clamp, is snapped to the clamp's image and named as moved.
fn check_raw(big: f64) {
    let mut z = [0.4, -0.7, 0.3, 0.9, -1.1, 0.2, 0.5, -0.6];
    let lock = exact(&latent_chart(), Exact::Raw(z)).expect("a raw z is refused");
    assert_eq!(lock.z, z, "an interior raw z is not kept bit for bit");
    assert!(!lock.notices.lookup_clamped());
    z[0] = big;
    z[3] = -big;
    let lock = exact(&latent_chart(), Exact::Raw(z)).expect("a saturated raw z is refused");
    for k in [1, 2, 4, 5, 6, 7] {
        assert_eq!(lock.z[k], z[k], "the unclamped z[{k}] is not kept");
    }
    let image = (1.0f64 - 1e-6).ln() - (1e-6f64).ln();
    for (k, want) in [(0, image), (3, -image)] {
        assert!(
            (lock.z[k] - want).abs() < 1e-6,
            "z[{k}] {} is not at its clamp's image {want}",
            lock.z[k]
        );
    }
    assert_eq!(
        lock.notices.moved.controls, 0b1001,
        "z_α and z_q1 are not named as moved"
    );
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

/// Checks a physical state: the lock is `E(x)`'s `z`, the same under a gauge and for the same system in another mass
/// unit (`mᵢ·M`, `pᵢ·M^{3/2}`), with `lookup_mirrored`, `λ`, `M` and the landing on the latent chart's plane where it
/// holds `x`.
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
    assert_eq!(lock.notices.mass, 1.0, "M is not reported");
    assert!(!lock.notices.past_fold);
    let moved = gauge(&x, 0.7, [3.0, -2.0]);
    let again = exact(&latent_chart(), Exact::Physical(moved)).expect("refused");
    assert!(
        phys(&d(lock.z), &d(again.z)) < 1e-13,
        "a gauge moves the lock"
    );
    let total: f64 = masses.iter().sum();
    let scaled = Ic {
        m: masses,
        r: x.r,
        p: x.p.map(|v| v.map(|c| c * total * total.sqrt())),
    };
    let lock2 = exact(&latent_chart(), Exact::Physical(scaled)).expect("refused");
    assert!(
        phys(&d(lock.z), &d(lock2.z)) < 1e-13,
        "the masses {masses:?} are not read as proportions"
    );
    assert_eq!(lock2.notices.mass, total, "M is not reported");
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

/// `K/|U|` and `E` of `x`.
fn virial(x: &Ic<f64>) -> (f64, f64) {
    let i = invariants(x);
    (i.k / -i.u, i.e)
}

/// The Lagrange equilateral relative equilibrium as the literature prints it (`G = mᵢ = 1`, so `M = 3`; side 1;
/// `ω² = M/a³`), its momenta times `stretch`.
fn lagrange(stretch: f64) -> Ic<f64> {
    let (m, w, s3) = (1.0, 3.0_f64.sqrt(), 3.0_f64.sqrt());
    let r = [[1.0 / s3, 0.0], [-0.5 / s3, 0.5], [-0.5 / s3, -0.5]];
    Ic {
        m: [m; 3],
        r,
        p: r.map(|v| [-w * v[1] * m * stretch, w * v[0] * m * stretch]),
    }
}

/// Checks the mass unit (Part 6, "The physical form's masses"): the `M = 3` Lagrange entry, its momenta times
/// `stretch`, locks with the entry's own `K/|U| = ½`, `E < 0` and `L_z > 0`, and records `M = 3`; the pair and triple
/// forms record their perimeter, a raw `z` 1, and the partial form 1 by default or the given masses' sum.
fn check_mass_unit(stretch: f64) {
    let entry = lagrange(1.0);
    let (ratio, energy) = virial(&entry);
    assert!(
        (ratio - 0.5).abs() < 1e-14 && energy < 0.0,
        "{ratio}, {energy}"
    );
    let lock = exact(&latent_chart(), Exact::Physical(lagrange(stretch))).expect("refused");
    let x = d(lock.z);
    let (got, e_lock) = virial(&x);
    assert!(
        (got - ratio).abs() < 1e-12,
        "the lock's K/|U| = {got} is not the entry's {ratio}"
    );
    assert!(e_lock < 0.0, "the lock's E = {e_lock} is not bound");
    assert!(invariants(&x).lz > 0.0, "the lock's sense of rotation");
    assert_eq!(lock.notices.mass, 3.0, "M = 3 is not recorded");
    let mass = |entry: Exact| exact(&burrau(), entry).expect("refused").notices.mass;
    assert_eq!(mass(Exact::Pythagorean { m: 2, n: 1 }), 12.0);
    assert_eq!(mass(Exact::Triple([6, 8, 10])), 24.0);
    assert_eq!(mass(Exact::Raw(e(&generic()).z.z)), 1.0);
    let part = |masses| {
        let p = Partial {
            masses,
            positions: None,
            momenta: None,
        };
        lookup_partial::<Defaults, _, _>(&latent_chart(), &p, None, phys, &tol())
            .expect("refused")
            .notices
            .mass
    };
    assert_eq!(part(None), 1.0);
    assert_eq!(part(Some([1.0, 2.0, 3.0])), 6.0);
}

#[test]
fn lookup_forms_mass_unit() {
    check_mass_unit(1.0);
}

negative_control!(
    lookup_forms_mass_unit,
    "the momenta kept as entered, the old reading, is the entry with its momenta times M^{3/2}: unbound",
    expected = "is not the entry's",
    check_mass_unit(3.0 * 3.0_f64.sqrt())
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
    // A chart not of the Burrau family reads no ν, so even one that is not finite leaves its defaults alone.
    let lock = partial(&latent_chart(), cases[0], Some(f64::NAN)).expect("a latent chart reads ν");
    assert_eq!(
        lock.z,
        partial(&latent_chart(), cases[0], None).expect("refused").z
    );
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

/// Classes at unit scales: `L_z = lz`, `E = e`.
fn inv(lz: f64, e: f64) -> Classes {
    Classes {
        lz,
        lz_scale: 1.0,
        e,
        e_scale: 1.0,
    }
}

/// Checks [`changed`] at the register's band `b`, `out` a value past it at unit scale: a class is `sign` outside the
/// band and 0 within it, its edge included; a change of `L_z`'s is named first; an entered value that is not finite
/// has none; [`Classes::of`] gives each value's scale; and the notices' `lookup_clamped` reads each moved field.
fn check_changed(out: f64) {
    let b = tol().zero_class;
    assert_eq!(
        changed(&inv(0.0, -1.0), &inv(out, -1.0), b),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(0.0, -1.0), &inv(-out, -1.0), b),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(1.0, -1.0), &inv(-1.0, 1.0), b),
        Some(Class::Rotation)
    );
    assert_eq!(
        changed(&inv(1.0, 0.0), &inv(2.0, out), b),
        Some(Class::Energy)
    );
    assert_eq!(
        changed(&inv(1.0, 0.0), &inv(2.0, -out), b),
        Some(Class::Energy)
    );
    assert_eq!(
        changed(&inv(1.0, -1.0), &inv(2.0, f64::NAN), b),
        Some(Class::Energy)
    );
    assert_eq!(changed(&inv(1.0, -1.0), &inv(3.0, -2.0), b), None);
    assert_eq!(changed(&inv(0.0, 0.0), &inv(0.0, 0.0), b), None);
    for edge in [b, -b] {
        assert_eq!(
            changed(&inv(0.0, -1.0), &inv(edge, -1.0), b),
            None,
            "L_z on the band's edge is class 0"
        );
        assert_eq!(
            changed(&inv(1.0, edge), &inv(1.0, 0.0), b),
            None,
            "E on the band's edge is class 0"
        );
    }
    let wide = Classes {
        lz_scale: 4.0,
        e_scale: 4.0,
        ..inv(2.0 * b, -2.0 * b)
    };
    assert_eq!(
        changed(&wide, &inv(0.0, 0.0), b),
        None,
        "the band is relative to the scale"
    );
    assert_eq!(
        changed(&inv(0.0, -1.0), &inv(-0.0, -1.0), b),
        None,
        "−0 is not 0's class"
    );
    assert_eq!(
        changed(&inv(1.0, f64::NEG_INFINITY), &inv(1.0, 5.0), b),
        None,
        "−∞ has a class"
    );
    assert_eq!(
        changed(&inv(f64::NAN, -1.0), &inv(1.0, -1.0), b),
        None,
        "NaN has a class"
    );
    assert_eq!(
        changed(&inv(0.0, f64::NEG_INFINITY), &inv(out, 5.0), b),
        None,
        "an entered E of −∞ leaves L_z no class to change"
    );
    assert_eq!(
        changed(&inv(f64::INFINITY, -1.0), &inv(1.0, 1.0), b),
        None,
        "an entered L_z of ∞ leaves E no class to change"
    );
    let unscaled = Classes {
        lz_scale: f64::INFINITY,
        ..inv(1.0, -1.0)
    };
    assert_eq!(
        changed(&unscaled, &inv(-1.0, -1.0), b),
        None,
        "a scale not finite gives no class"
    );
    assert_eq!(
        changed(&inv(1.0, -1.0), &unscaled, b),
        Some(Class::Rotation),
        "nor after the clamp"
    );
    let x = Ic {
        m: [0.5, 0.25, 0.25],
        r: [[3.0, 4.0], [0.0, -2.0], [-6.0, 0.0]],
        p: [[0.0, 1.0], [0.6, 0.8], [-0.6, -1.8]],
    };
    let i = invariants(&x);
    let want = Classes {
        lz: i.lz,
        lz_scale: 5.0 + 2.0 + 6.0 * 1.8973665961010275,
        e: i.e,
        e_scale: i.k - i.u,
    };
    let got = Classes::of(&x);
    assert_eq!(
        (got.lz, got.e, got.e_scale),
        (want.lz, want.e, want.e_scale)
    );
    assert!((got.lz_scale - want.lz_scale).abs() <= 1e-14, "{got:?}");
    let quiet = Notices {
        moved: Moved::default(),
        projected: None,
        clamped: None,
        mirrored: false,
        lambda: 1.0,
        mass: 1.0,
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
        (t.eps_phys, t.curve_tol, t.sanity, t.zero_class),
        (
            64.0 * f64::EPSILON,
            3.0,
            16.0 * f64::EPSILON,
            16.0 * f64::EPSILON
        )
    );
}

#[test]
fn lookup_validation_ladder_classes() {
    let b = tol().zero_class;
    check_changed(f64::from_bits(b.to_bits() + 1));
}

negative_control!(
    lookup_validation_ladder_classes,
    "a value on the band's edge is class 0, so it changes no class",
    expected = "left: None",
    check_changed(tol().zero_class)
);

/// A collinear, equal-mass state moving along its line, `L_z = 0` in exact arithmetic, turned by `angle`.
fn collinear(angle: f64) -> Ic<f64> {
    let x = Ic {
        m: [1.0 / 3.0; 3],
        r: [[-1.0, 0.0], [0.2, 0.0], [1.3, 0.0]],
        p: [[0.3, 0.0], [-0.5, 0.0], [0.2, 0.0]],
    };
    gauge(&x, angle, [0.0, 0.0])
}

/// The 64 orientations `2πk/64`.
fn orientations() -> impl Iterator<Item = f64> {
    (0..64).map(|k| std::f64::consts::TAU * f64::from(k) / 64.0)
}

/// Checks the zero class's gauge invariance (§ "The zero class in floating point", REQ-ENC-036): the collinear state
/// at each of 64 orientations gets one verdict at the zero-class band `band`, the contract's: its true `L_z` is class
/// 0, and the clamp of `β` at the line moves it off 0, so each is rejected as a change of the sense of rotation.
fn check_orientations(band: f64) {
    let t = Tolerances {
        zero_class: band,
        ..tol()
    };
    let verdicts: Vec<Result> = orientations()
        .map(|a| {
            lookup::<Defaults, _, _>(&latent_chart(), &Exact::Physical(collinear(a)), phys, &t)
        })
        .collect();
    for (k, v) in verdicts.iter().enumerate() {
        assert_eq!(
            v.as_ref().err().map(|r| (r.layer, r.failed)),
            Some((
                Layer::Hypercube,
                Failed::Clamp {
                    controls: 0b10,
                    class: Class::Rotation
                }
            )),
            "orientation {k} of 64 has its own verdict"
        );
    }
}

#[test]
fn lookup_validation_ladder_zero_band() {
    check_orientations(tol().zero_class);
}

negative_control!(
    lookup_validation_ladder_zero_band,
    "an exact-zero comparison gives a rounded L_z = 0 the sign of its rounding, so the verdict follows the orientation",
    expected = "has its own verdict",
    check_orientations(0.0)
);

/// The rounding the zero-class band absorbs, in f64's ε relative to each scale (§ "The zero class in floating point",
/// REQ-ENC-037): over `n` states decoded from latent `z` on `[−8, 8]⁸`, all but one in nine with one control set to
/// ±20, past its clamp, each against a random gauge image of itself (offsets and boosts on `[−8, 8)²`, scales
/// `2^±12`, either parity): `C`'s rounding of `L_z` and `E` (`C(g·x)` against `C(x)`, one canonical state, each at
/// its own entry's extents), and, where both clamp the same controls, the rounding of the clamps' [`moves`] and of the
/// values after them.
struct Rounding {
    class: f64,
    moves: f64,
}

fn rounding(n: usize) -> Rounding {
    use validation::decode_f32_eps::SplitMix;
    use validation::gauge::{uniform, Gauge};
    let eps = f64::EPSILON;
    let mut rng = SplitMix::new(36);
    let (mut class, mut moved) = (0.0_f64, 0.0_f64);
    let rel = |a: f64, b: f64, sa: f64, sb: f64| {
        let scale = sa.max(sb);
        if scale > 0.0 {
            (a - b).abs() / scale / eps
        } else {
            0.0
        }
    };
    for _ in 0..n {
        let mut z: [f64; 8] = std::array::from_fn(|_| uniform(&mut rng, -8.0, 8.0));
        let pick = (rng.next_u64() % 18) as usize;
        if pick < 16 {
            z[pick / 2] = if pick.is_multiple_of(2) { -20.0 } else { 20.0 };
        }
        let x = d(z);
        let gx = Gauge::random(&mut rng, 8.0, 12.0).act(&x);
        let (ex, ey) = (e(&x), e(&gx));
        let (tx, ty) = (
            Extents::entered(&x, ex.canonical.lambda),
            Extents::entered(&gx, ey.canonical.lambda),
        );
        let (cx, cy) = (
            Classes::within(&ex.canonical.ic, &tx),
            Classes::within(&ey.canonical.ic, &ty),
        );
        class = class
            .max(rel(cx.lz, cy.lz, cx.lz_scale, cy.lz_scale))
            .max(rel(cx.e, cy.e, cx.e_scale, cy.e_scale));
        if ex.blocks.clamped != 0 && ex.blocks.clamped == ey.blocks.clamped {
            let (mx, my) = (
                moves::<Defaults>(&ex, &ex.z.z, &tx),
                moves::<Defaults>(&ey, &ey.z.z, &ty),
            );
            moved = moved
                .max(rel(mx.lz, my.lz, mx.lz_scale, my.lz_scale))
                .max(rel(mx.e, my.e, mx.e_scale, my.e_scale))
                .max(rel(
                    mx.lz_after,
                    my.lz_after,
                    mx.lz_after_scale,
                    my.lz_after_scale,
                ))
                .max(rel(
                    mx.e_after,
                    my.e_after,
                    mx.e_after_scale,
                    my.e_after_scale,
                ));
        }
    }
    Rounding {
        class,
        moves: moved,
    }
}

/// The measurement over 2²⁰ states, made once for the tests that read it.
fn measured() -> &'static Rounding {
    static ONCE: std::sync::OnceLock<Rounding> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| rounding(1 << 20))
}

/// Checks the zero-class band against the measurement, each inequality on its own `k`: `C`'s worst rounding times 4
/// is within `k_class`, and the moves' worst rounding times 4 is within `k_moves`.
fn check_band(k_class: f64, k_moves: f64) {
    let m = measured();
    println!(
        "zero-class band: rounding of the classes {:.2} ε, of the moves {:.2} ε; k = {}",
        m.class,
        m.moves,
        ZERO_CLASS.number()
    );
    assert!(
        4.0 * m.class <= k_class,
        "C's rounding, {:.2} ε, is not within k = {k_class} by 4×",
        m.class
    );
    assert!(
        4.0 * m.moves <= k_moves,
        "the moves' rounding, {:.2} ε, is not within k = {k_moves} by 4×",
        m.moves
    );
}

#[test]
fn lookup_validation_ladder_zero_band_measure() {
    check_band(ZERO_CLASS.number(), ZERO_CLASS.number());
}

negative_control!(
    lookup_validation_ladder_zero_band_measure,
    "a band of one ε does not cover C's rounding of the classes",
    expected = "C's rounding",
    check_band(1.0, ZERO_CLASS.number())
);

#[test]
fn lookup_validation_ladder_zero_band_measure_moves() {
    check_band(ZERO_CLASS.number(), ZERO_CLASS.number());
}

negative_control!(
    lookup_validation_ladder_zero_band_measure_moves,
    "a band of one ε does not cover the moves' rounding",
    expected = "the moves' rounding",
    check_band(ZERO_CLASS.number(), 1.0)
);

/// A collinear state moving along its line, the outer body's momentum `t` against the inner pair's ±1, turned by
/// `angle` and moved: `L_z = 0` exactly, and `β` at the line, so its clamp fires.
fn along(t: f64, angle: f64) -> Ic<f64> {
    let x = Ic {
        m: [1.0 / 3.0; 3],
        r: [[-1.0, 0.0], [0.2, 0.0], [1.3, 0.0]],
        p: [[1.0, 0.0], [-1.0 - t, 0.0], [t, 0.0]],
    };
    gauge(&x, angle, [0.5, -0.2])
}

/// A rest start at the mass ratio `small : 1 : 1`, the masses summing to 1, whose mass controls clamp.
fn rest(small: f64) -> Ic<f64> {
    let total = small + 2.0;
    Ic {
        m: [small / total, 1.0 / total, 1.0 / total],
        r: [[0.0, 0.0], [1.0, 0.1], [0.3, 1.2]],
        p: [[0.0; 2]; 3],
    }
}

/// `x` entered under a gauge element with a boost and an offset, so that `C` removes a momentum and a translation
/// larger than the state's own.
fn boosted(x: &Ic<f64>, u: [f64; 2], s: f64) -> Ic<f64> {
    validation::gauge::Gauge {
        a: [40.0, -25.0],
        u,
        theta: 0.4,
        s,
        mirror: false,
    }
    .act(x)
}

/// `z` with `L_z = 0` up to rounding and a momentum control past its clamp: `z_q1` (`p_ρy`) where `pull`, else `z_q0`
/// (`p_ρx`). `α = (π/2)σ(1)`, `β = π/2`, equal masses; `p_λx` cancels `‖ρ‖·p_ρy`.
fn q_case(pull: bool) -> [f64; 8] {
    let alpha = std::f64::consts::FRAC_PI_2 / (1.0 + (-1.0_f64).exp());
    let (mu_rho, mu_lambda) = (1.0 / 6.0, 2.0 / 9.0);
    let ratio = (alpha.cos() / mu_rho.sqrt()) / (alpha.sin() / mu_lambda.sqrt());
    let logit = |q: f64| {
        let u = (q / 2.0 + 1.0) / 2.0;
        (u / (1.0 - u)).ln()
    };
    if pull {
        [1.0, 0.0, 0.0, 40.0, logit(2.0 * ratio), 0.0, 0.0, 0.0]
    } else {
        [1.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    }
}

/// The clamp-rule table (§ "The class a clamp moves", REQ-ENC-038): each state, its clamped controls and the class
/// its clamp changes in exact arithmetic.
fn table() -> Vec<(&'static str, Ic<f64>, u32, Option<Class>)> {
    let mut rows = Vec::new();
    for angle in [0.0, 0.3, 2.0] {
        for t in [1e-4, 1e-6, 1e-8] {
            rows.push(("β, p_λ ≠ 0", along(t, angle), 0b10, Some(Class::Rotation)));
        }
        rows.push(("β, p_λ = 0", along(0.0, angle), 0b10, None));
    }
    for small in [1e-6, 1e-9] {
        rows.push(("masses at rest", rest(small), 0b1100_0000, None));
    }
    rows.push(("masses and α at rest", rest(1e-12), 0b1100_0001, None));
    for u in [[3.0, 1.0], [-2.5, 4.0]] {
        rows.push((
            "masses at rest, boosted",
            boosted(&rest(1e-6), u, 1.0),
            0b1100_0000,
            None,
        ));
        rows.push((
            "β, p_λ = 0, boosted",
            boosted(&along(0.0, 0.0), u, 1e-3),
            0b10,
            None,
        ));
    }
    rows.push(("q₁ = p_ρy", d(q_case(true)), 0b1000, Some(Class::Rotation)));
    rows.push(("q₀ = p_ρx", d(q_case(false)), 0b100, None));
    rows
}

/// Checks `classify`, the class a clamp of the entered `x`, encoded, changes, on the clamp-rule table.
fn check_clamp_rule(classify: impl Fn(&Ic<f64>, &Encoded<f64>) -> Option<Class>) {
    for (what, x, bits, want) in table() {
        let en = e(&x);
        assert_eq!(en.blocks.clamped, bits, "{what}: the clamped controls");
        assert_eq!(classify(&x, &en), want, "{what}: {x:?}");
    }
}

/// The rule: [`changed`] on [`moves`].
fn by_rule(x: &Ic<f64>, en: &Encoded<f64>) -> Option<Class> {
    let b = tol().zero_class;
    let ext = Extents::entered(x, en.canonical.lambda);
    let before = Classes::within(&en.canonical.ic, &ext);
    changed(
        &before,
        &after(&before, &moves::<Defaults>(en, &en.z.z, &ext), b),
        b,
    )
}

/// The class of `v` at `scale` through the band `b`, as the decoded value would give it: the controls' reading.
#[cfg(feature = "controls")]
fn banded(v: f64, scale: f64, b: f64) -> Option<std::cmp::Ordering> {
    (v.is_finite() && scale.is_finite()).then(|| {
        if v.abs() <= b * scale {
            std::cmp::Ordering::Equal
        } else {
            v.total_cmp(&0.0)
        }
    })
}

#[test]
fn lookup_validation_ladder_clamp_rule() {
    check_clamp_rule(by_rule);
}

negative_control!(
    lookup_validation_ladder_clamp_rule,
    "the decoded value read through a band wider than the rounding (round 3's 2^24 eps) takes β's moves of L_z for \
     rounding",
    expected = "β, p_λ ≠ 0",
    check_clamp_rule(|x: &Ic<f64>, en: &Encoded<f64>| {
        let ext = Extents::entered(x, en.canonical.lambda);
        let (a, b) = (Classes::within(&en.canonical.ic, &ext), Classes::within(&d(en.z.z), &ext));
        let k = 16_777_216.0 * f64::EPSILON;
        if banded(a.lz, a.lz_scale, k) != banded(b.lz, b.lz_scale, k) {
            Some(Class::Rotation)
        } else if banded(a.e, a.e_scale, k) != banded(b.e, b.e_scale, k) {
            Some(Class::Energy)
        } else {
            None
        }
    })
);

#[test]
fn lookup_validation_ladder_clamp_rule_unmoved() {
    check_clamp_rule(by_rule);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_unmoved,
    "rejecting every clamp on a class-0 state rejects the clamps that leave L_z at 0",
    expected = "p_λ = 0",
    check_clamp_rule(|_: &Ic<f64>, en: &Encoded<f64>| {
        let c = Classes::of(&en.canonical.ic);
        (en.blocks.clamped != 0
            && banded(c.lz, c.lz_scale, tol().zero_class) == Some(std::cmp::Ordering::Equal))
        .then_some(Class::Rotation)
    })
);

#[test]
fn lookup_validation_ladder_clamp_rule_extents() {
    check_clamp_rule(by_rule);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_extents,
    "the decoded value read at the canonical state's own scales takes the rounding of C's boost for a class",
    expected = "masses at rest, boosted",
    check_clamp_rule(|_: &Ic<f64>, en: &Encoded<f64>| {
        let (a, b) = (Classes::of(&en.canonical.ic), Classes::of(&d(en.z.z)));
        let k = tol().zero_class;
        (banded(a.lz, a.lz_scale, k) != banded(b.lz, b.lz_scale, k)
            || banded(a.e, a.e_scale, k) != banded(b.e, b.e_scale, k))
        .then_some(Class::Rotation)
    })
);

/// Checks the table end to end at the zero-class band `band`: lookup refuses each row whose clamp changes a class,
/// naming it and the clamped controls, and locks every other, clamped.
fn check_clamp_verdicts(band: f64) {
    let t = Tolerances {
        zero_class: band,
        ..tol()
    };
    for (what, x, bits, want) in table() {
        let got = lookup::<Defaults, _, _>(&latent_chart(), &Exact::Physical(x), phys, &t);
        match want {
            Some(class) => assert_eq!(
                got.err(),
                Some(Rejected {
                    layer: Layer::Hypercube,
                    failed: Failed::Clamp {
                        controls: bits,
                        class
                    }
                }),
                "{what}: the clamp's verdict"
            ),
            None => assert_eq!(
                got.map(|l| l.notices.moved.controls).ok(),
                Some(bits),
                "{what}: the clamp's verdict"
            ),
        }
    }
}

#[test]
fn lookup_validation_ladder_clamp_verdicts() {
    check_clamp_verdicts(tol().zero_class);
}

negative_control!(
    lookup_validation_ladder_clamp_verdicts,
    "a band of the whole scale reads every move as rounding",
    expected = "the clamp's verdict",
    check_clamp_verdicts(1.0)
);

/// Checks [`after`] at the register's band `b`, `out` past it: a constant in class 0 takes the move alone at the move's
/// own scale, however large the state's; any other takes its value on the clamped parameters at that value's scale;
/// and a move within its own band leaves class 0 as it was.
fn check_after(out: f64) {
    let b = tol().zero_class;
    let at = |lz: f64, lz_scale: f64, e: f64, e_scale: f64| Classes {
        lz,
        lz_scale,
        e,
        e_scale,
    };
    let moves = |lz: f64, lz_scale: f64, e: f64, e_scale: f64| Moves {
        lz,
        lz_scale,
        e,
        e_scale,
        lz_after: 7.0,
        lz_after_scale: 8.0,
        e_after: -0.75,
        e_after_scale: 1.5,
    };
    let big = at(1e-6, 1e12, -1.0, 1.0);
    let small = moves(out * 1e-9, 1e-9, 0.25, 0.5);
    assert_eq!(
        after(&big, &small, b),
        at(out * 1e-9, 1e-9, -0.75, 1.5),
        "class 0 takes the move alone, a sign class its value after"
    );
    assert_eq!(
        changed(&big, &after(&big, &small, b), b),
        Some(Class::Rotation),
        "a move far below the state's band is read at its own scale"
    );
    let within = moves(b * 1e-9, 1e-9, 0.0, 0.0);
    assert_eq!(
        changed(&big, &after(&big, &within, b), b),
        None,
        "a move within its own band is no move"
    );
    let zero_e = at(1.0, 1.0, 0.0, 1.0);
    assert_eq!(
        after(&zero_e, &moves(0.5, 0.5, -out, 1.0), b),
        at(7.0, 8.0, -out, 1.0)
    );
}

#[test]
fn lookup_validation_ladder_after() {
    check_after(f64::from_bits(tol().zero_class.to_bits() + 1));
}

negative_control!(
    lookup_validation_ladder_after,
    "a move on its band's edge is class 0, so it changes no class",
    expected = "a move far below the state's band",
    check_after(tol().zero_class)
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
