//! The kernel's physics layer (`kernel::physics`; dd_integrator §3.1; integrator_contract Part 1, Part 2c):
//! - `real_instantiations_*`: the kernel is instantiated at f32 and f64, as the ledger's precision rows say, and no
//!   double-double instantiation exists (R-265); the same golden-IC force and energy check runs at each, the M2
//!   decoder's `z = 0` decode (`fixtures/golden_ic/z0.json`: the equilateral triangle of side √3, at rest) run through
//!   [`PairGeom`], [`force`] and [`energy`]; and `fma` is fused and `square_root` the root at each (REQ-INT-004).
//! - `force_known_state*`: the force on a hand-computed three-body state, the 3-4-5 triangle with masses 1, 2, 3,
//!   matches to f64 rounding, and `Σ mᵢaᵢ = 0` (REQ-INT-036); so do the energy and `L_z` of the same state with
//!   momenta.
//! - `pair_geometry_shared*`: an instrumented `Real`, which counts square roots, shows one pairwise-distance computation
//!   per force evaluation, and the potential and `r_min` reading the same `PairGeom` and forming none (REQ-PERF-001);
//!   the geometry's values by hand; `closest`'s least `d²`, ties going to the first pair.
//!
//! **Tolerances** (applied per R-369: test tolerances): a value is compared relative to its scale, in units of the
//! precision's `eps`. [`KNOWN_EPS`] = 2 for the hand-computed state at f64, whose inputs are all exact, so the error is
//! the evaluation's own few roundings (measured: the force exact, `K` 0.56 `eps`). [`GOLDEN_EPS`] = 4 for the golden IC,
//! whose decoded inputs are themselves within the decoder's 4 `eps` (decode_join's `GOLDEN_EPS`); the worst measured
//! is 1.0 `eps`, the force at f32, and the test prints each.

mod decode_support;

use std::cell::Cell;
use std::ops::{Add, Div, Mul, Neg, Sub};

use decode_support::*;
use kernel::decode;
use kernel::generated::links::Defaults;
use kernel::payload::PAYLOAD_LAYOUTS;
use kernel::physics::energy::{angular_momentum, energy, kinetic, potential};
use kernel::physics::force::force;
use kernel::physics::PairGeom;
use kernel::real::RealOps;
use kernel::Real;
use validation::negative_control;

/// The relative error bound, in `eps`, for the hand-computed state at f64.
const KNOWN_EPS: f64 = 2.0;

/// The relative error bound, in `eps`, for the golden IC at f32 and f64.
const GOLDEN_EPS: f64 = 4.0;

/// `got`'s error from `want`, relative to `scale`, in units of `eps`.
fn err_eps(got: f64, want: f64, scale: f64, eps: f64) -> f64 {
    (got - want).abs() / scale / eps
}

// ── real_instantiations (REQ-INT-004, R-265) ─────────────────────────────────────────────────────────────────────

/// The precision rows the ledger instantiates are exactly `names`, and the double-double row is a stub only.
fn check_rows(names: &[&str]) {
    let instantiated: Vec<&str> = PAYLOAD_LAYOUTS
        .iter()
        .filter(|row| row.instantiated)
        .map(|row| row.real)
        .collect();
    assert_eq!(
        instantiated, names,
        "the kernel's instantiations are not f32 and f64 only"
    );
    let dd = PAYLOAD_LAYOUTS
        .iter()
        .find(|row| row.real == "DoubleF64")
        .expect("the double-double stub row");
    assert!(!dd.instantiated, "double-double is instantiated (R-265)");
}

#[test]
fn real_instantiations_rows() {
    check_rows(&[<f32 as Real>::NAME, <f64 as Real>::NAME]);
}

negative_control!(
    real_instantiations_rows,
    "a third instantiation, double-double, listed beside f32 and f64",
    expected = "the kernel's instantiations are not f32 and f64 only",
    check_rows(&["f32", "f64", "DoubleF64"])
);

/// The value named `n` in the golden fixture, as its exact decimal.
fn exact(g: &serde_json::Value, n: &str) -> f64 {
    let v = golden_values(g)
        .into_iter()
        .find(|v| name(v) == n)
        .unwrap_or_else(|| panic!("no golden value `{n}`"));
    v["decimal"]
        .as_str()
        .expect("a decimal")
        .parse()
        .expect("a number")
}

/// The golden IC's force, energy, `L_z` and `r_min` at `R`, from the decoder's `z = 0`, run through the physics layer.
struct Golden {
    a: [[f64; 2]; 3],
    e: f64,
    k: f64,
    lz: f64,
    r_min: f64,
    m_a: [f64; 2],
}

/// Decodes `z = 0` at `R` and runs the result through [`PairGeom`], [`force`], [`energy`] and [`angular_momentum`].
fn golden_at<R: P>() -> Golden {
    let ic = decode::decode::<Defaults, R>(&latent::<R>([0.0; 8])).ic;
    let g = PairGeom::new(&ic.r);
    let a = force(&ic.m, &g);
    let m_a = [0, 1].map(|k| {
        (0..3)
            .map(|i| ic.m[i] * a[i][k])
            .fold(R::of(0.0), |s, x| s + x)
    });
    Golden {
        a: a.map(|v| v.map(R::f)),
        e: energy(&ic.m, &ic.p, &g).f(),
        k: kinetic(&ic.m, &ic.p).f(),
        lz: angular_momentum(&ic.r, &ic.p).f(),
        r_min: g.r_min().f(),
        m_a: m_a.map(R::f),
    }
}

/// Checks `got`, the golden IC run at `R`, against the closed forms: at the CoM, with equal masses `1/3` and side
/// `√3`, `Σ_{j≠i}(rⱼ − rᵢ) = −3rᵢ`, so `aᵢ = −(1/3)·3rᵢ/(√3)³ = −(√3/9)rᵢ = V₀·rᵢ`; `E = V₀ = −√3/9` (`E_0`); at rest
/// `K = L_z = 0` exactly; `r_min = √3`; and `Σmᵢaᵢ = 0`.
fn check_golden_values<R: P>(got: &Golden, worst: &Worst) {
    let g = golden();
    let v0 = exact(&g, "E_0");
    let eps = R::eps();
    let scale = v0.abs();
    for (i, body) in ["r0", "r1", "r2"].iter().enumerate() {
        for (k, axis) in ["x", "y"].iter().enumerate() {
            let want = v0 * exact(&g, &format!("{body}_{axis}"));
            let err = err_eps(got.a[i][k], want, scale, eps);
            worst.see(&format!("a at {}", R::NAME), err);
            assert!(
                err <= GOLDEN_EPS,
                "the golden IC's a{i}_{axis} = {} at {}, not {want} ({err} eps)",
                got.a[i][k],
                R::NAME
            );
        }
    }
    let err = err_eps(got.e, v0, scale, eps);
    worst.see(&format!("E at {}", R::NAME), err);
    assert!(
        err <= GOLDEN_EPS,
        "the golden IC's E = {} at {}, not E_0 = {v0} ({err} eps)",
        got.e,
        R::NAME
    );
    assert_eq!(
        (got.k, got.lz),
        (0.0, 0.0),
        "the golden IC at rest has K or L_z non-zero at {}",
        R::NAME
    );
    let side = exact(&g, "pair_01");
    let err = err_eps(got.r_min, side, side, eps);
    worst.see(&format!("r_min at {}", R::NAME), err);
    assert!(
        err <= GOLDEN_EPS,
        "the golden IC's r_min = {} at {}, not √3 ({err} eps)",
        got.r_min,
        R::NAME
    );
    for (k, s) in got.m_a.iter().enumerate() {
        let err = s.abs() / scale / eps;
        worst.see(&format!("Σmᵢaᵢ at {}", R::NAME), err);
        assert!(
            err <= GOLDEN_EPS,
            "the golden IC's Σmᵢaᵢ[{k}] = {s} at {} ({err} eps)",
            R::NAME
        );
    }
}

fn check_golden<R: P>(worst: &Worst) {
    check_golden_values::<R>(&golden_at::<R>(), worst);
}

#[test]
fn real_instantiations_golden() {
    let worst = Worst::default();
    check_golden::<f64>(&worst);
    check_golden::<f32>(&worst);
    worst.print("real_instantiations_golden");
}

negative_control!(
    real_instantiations_golden,
    "the golden IC's force with one body's acceleration negated",
    expected = "the golden IC's a2_y",
    {
        let mut got = golden_at::<f64>();
        got.a[2] = got.a[2].map(|x| -x);
        check_golden_values::<f64>(&got, &Worst::default());
    }
);

/// `x·x − c`, where `x = 1 + 2⁻ʰ` and `c = 1 + 2^(1−h)`: `2^(−2h)` when fused, 0 when the product rounds first.
fn check_fma<R: Real + P>(h: i32) {
    let x = R::of(1.0 + 2f64.powi(-h));
    let c = R::of(1.0 + 2f64.powi(1 - h));
    let fused = x.fma(x, -c);
    assert_eq!(
        (fused.f(), (x * x - c).f()),
        (2f64.powi(-2 * h), 0.0),
        "`fma` at {} is not fused",
        R::NAME
    );
}

/// `square_root` of each perfect square `n²` is `n`, and of 2 is √2 rounded to the precision.
fn check_square_root<R: Real + P>(sqrt: fn(R) -> R) {
    for n in [0.0, 1.0, 3.0, 5.0, 1024.0, 0.5] {
        assert_eq!(
            sqrt(R::of(n * n)).f(),
            n,
            "`square_root` at {} is not √{}",
            R::NAME,
            n * n
        );
    }
    assert_eq!(
        sqrt(R::of(2.0)),
        R::of(std::f64::consts::SQRT_2),
        "`square_root` at {} is not √2",
        R::NAME
    );
}

#[test]
fn real_instantiations_fma_and_square_root() {
    check_fma::<f64>(27);
    check_fma::<f32>(12);
    check_square_root::<f64>(RealOps::square_root);
    check_square_root::<f32>(RealOps::square_root);
}

negative_control!(
    real_instantiations_fma_and_square_root,
    "a square root that halves is not the root",
    expected = "`square_root` at f64 is not √",
    check_square_root::<f64>(|x| x / 2.0)
);

// ── force_known_state (REQ-INT-036) ──────────────────────────────────────────────────────────────────────────────

/// Masses 1, 2, 3 at `(0, 0)`, `(3, 0)` and `(0, 4)`: the pairs' distances are 3, 5 and 4, each cube exact.
const M: [f64; 3] = [1.0, 2.0, 3.0];
const R: [[f64; 2]; 3] = [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]];

/// The accelerations, by hand: `a₀ = 2(3, 0)/27 + 3(0, 4)/64`, `a₁ = 1(−3, 0)/27 + 3(−3, 4)/125`,
/// `a₂ = 1(0, −4)/64 + 2(3, −4)/125`.
fn known_force() -> [[f64; 2]; 3] {
    [
        [2.0 / 9.0, 3.0 / 16.0],
        [-1.0 / 9.0 - 9.0 / 125.0, 12.0 / 125.0],
        [6.0 / 125.0, -1.0 / 16.0 - 8.0 / 125.0],
    ]
}

/// `a` matches [`known_force`] to [`KNOWN_EPS`], relative to the largest component.
fn check_known_force(a: [[f64; 2]; 3]) {
    let want = known_force();
    let scale = want.iter().flatten().fold(0.0f64, |s, x| s.max(x.abs()));
    for i in 0..3 {
        for k in 0..2 {
            let err = err_eps(a[i][k], want[i][k], scale, f64::EPSILON);
            assert!(
                err <= KNOWN_EPS,
                "a{i}[{k}] = {}, not the hand-computed {} ({err} eps)",
                a[i][k],
                want[i][k]
            );
        }
    }
}

/// `Σmᵢaᵢ = 0` for masses `m`, to [`KNOWN_EPS`] relative to the largest `mᵢ|aᵢ|`, componentwise.
fn check_balance(m: [f64; 3], a: [[f64; 2]; 3]) {
    let components = [0, 1].map(|k| [0, 1, 2].map(|i| m[i] * a[i][k]));
    for (k, terms) in components.iter().enumerate() {
        let scale = terms.iter().fold(0.0f64, |s, x| s.max(x.abs()));
        let sum: f64 = terms.iter().sum();
        assert!(
            sum.abs() / scale / f64::EPSILON <= KNOWN_EPS,
            "Σmᵢaᵢ[{k}] = {sum}, not 0"
        );
    }
}

#[test]
fn force_known_state() {
    check_known_force(force(&M, &PairGeom::new(&R)));
}

negative_control!(
    force_known_state,
    "the force with body 2's mass dropped is not the hand-computed one",
    expected = "not the hand-computed",
    check_known_force(force(&[1.0, 2.0, 0.0], &PairGeom::new(&R)))
);

#[test]
fn force_known_state_balance() {
    check_balance(M, force(&M, &PairGeom::new(&R)));
}

negative_control!(
    force_known_state_balance,
    "the force of masses 1, 2, 0 weighed by masses 1, 2, 3",
    expected = "Σmᵢaᵢ[0]",
    check_balance(M, force(&[1.0, 2.0, 0.0], &PairGeom::new(&R)))
);

/// Momenta summing to 0 for the state of [`M`] and [`R`].
const P: [[f64; 2]; 3] = [[1.0, 2.0], [-3.0, 1.0], [2.0, -3.0]];

/// `K = 5/2 + 10/4 + 13/6 = 43/6`, `V = −(2/3 + 6/5 + 3/4) = −157/60`, `E = 91/20`,
/// `L_z = 0 + (3·1 − 0) + (0 − 4·2) = −5`, each matched to [`KNOWN_EPS`].
fn check_known_energy(k: f64, v: f64, e: f64, lz: f64) {
    for (name, got, want) in [
        ("K", k, 43.0 / 6.0),
        ("V", v, -157.0 / 60.0),
        ("E", e, 91.0 / 20.0),
        ("L_z", lz, -5.0),
    ] {
        let err = err_eps(got, want, want.abs(), f64::EPSILON);
        assert!(
            err <= KNOWN_EPS,
            "{name} = {got}, not the hand-computed {want} ({err} eps)"
        );
    }
}

#[test]
fn force_known_state_energy() {
    let g = PairGeom::new(&R);
    check_known_energy(
        kinetic(&M, &P),
        potential(&M, &g),
        energy(&M, &P, &g),
        angular_momentum(&R, &P),
    );
}

negative_control!(
    force_known_state_energy,
    "the momenta of bodies 1 and 2 exchanged change K and L_z",
    expected = "not the hand-computed",
    {
        let g = PairGeom::new(&R);
        let p = [P[0], P[2], P[1]];
        check_known_energy(
            kinetic(&M, &p),
            potential(&M, &g),
            energy(&M, &p, &g),
            angular_momentum(&R, &p),
        )
    }
);

// ── pair_geometry_shared (REQ-PERF-001) ──────────────────────────────────────────────────────────────────────────

thread_local! {
    /// The square roots taken on this thread by [`Counted`].
    static ROOTS: Cell<usize> = const { Cell::new(0) };
}

/// The instrumented build's `Real`: f64, counting each square root, the one operation a distance needs and nothing
/// else in the physics layer takes. A test harness, not a kernel instantiation: the kernel ships at f32 and f64 only.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
struct Counted(f64);

macro_rules! binary {
    ($($op:ident $f:ident $sym:tt),*) => {$(
        impl $op for Counted {
            type Output = Counted;
            fn $f(self, b: Counted) -> Counted {
                Counted(self.0 $sym b.0)
            }
        }
    )*};
}

binary!(Add add +, Sub sub -, Mul mul *, Div div /);

impl Neg for Counted {
    type Output = Counted;
    fn neg(self) -> Counted {
        Counted(-self.0)
    }
}

impl RealOps for Counted {
    fn fma(self, a: Counted, b: Counted) -> Counted {
        Counted(self.0.mul_add(a.0, b.0))
    }
    fn square_root(self) -> Counted {
        ROOTS.set(ROOTS.get() + 1);
        Counted(self.0.sqrt())
    }
}

impl Real for Counted {
    const NAME: &'static str = "counted";
}

/// The square roots taken while `f` runs.
fn roots<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ROOTS.get();
    let out = f();
    (out, ROOTS.get() - before)
}

/// What one force evaluation and the monitoring after it cost: the roots taken building the geometry, and those taken
/// by the force, the potential and `r_min` reading it.
struct Shared {
    built: usize,
    read: usize,
    potential: f64,
    r_min: f64,
    norm: [f64; 3],
    closest: usize,
}

/// Builds the geometry of [`R`] at [`Counted`] with `build`, then evaluates the force, the potential and `r_min` from
/// it, counting the roots of each part.
fn shared(build: fn(&[[Counted; 2]; 3]) -> PairGeom<Counted>) -> Shared {
    let m = M.map(Counted);
    let r = R.map(|v| v.map(Counted));
    let (g, built) = roots(|| build(&r));
    let ((potential, r_min), read) = roots(|| {
        let _ = force(&m, &g);
        (potential(&m, &g), g.r_min())
    });
    Shared {
        built,
        read,
        potential: potential.0,
        r_min: r_min.0,
        norm: g.norm().map(|x| x.0),
        closest: g.closest(),
    }
}

/// One distance computation per force evaluation: three roots, one per pair, building the geometry, and none reading
/// it; the potential and `r_min` are the ones the geometry's distances give.
fn check_shared(s: &Shared) {
    assert_eq!(
        (s.built, s.read),
        (3, 0),
        "not one pairwise-distance computation per force evaluation (roots building, reading)"
    );
    let n = s.norm;
    assert_eq!(
        s.potential,
        -(M[0] * M[1] / n[0] + M[1] * M[2] / n[1] + M[2] * M[0] / n[2]),
        "the potential does not read the geometry's distances"
    );
    assert_eq!(
        s.r_min, n[s.closest],
        "r_min does not read the geometry's distances"
    );
}

#[test]
fn pair_geometry_shared() {
    check_shared(&shared(PairGeom::new));
}

negative_control!(
    pair_geometry_shared,
    "a geometry built twice, as a potential forming its own distances would",
    expected = "not one pairwise-distance computation per force evaluation",
    check_shared(&shared(|r| {
        let _ = PairGeom::new(r);
        PairGeom::new(r)
    }))
);

/// The 3-4-5 state's geometry, by hand: separations `(3, 0)`, `(−3, 4)`, `(0, −4)`; `d²` 9, 25, 16; distances 3, 5,
/// 4; cubes 27, 125, 64; the closest pair 0, `r_min = 3`.
fn check_geometry(g: &PairGeom<f64>) {
    assert_eq!(
        (g.sep(), g.d2(), g.norm(), g.cube(), g.closest(), g.r_min()),
        (
            [[3.0, 0.0], [-3.0, 4.0], [0.0, -4.0]],
            [9.0, 25.0, 16.0],
            [3.0, 5.0, 4.0],
            [27.0, 125.0, 64.0],
            0,
            3.0
        ),
        "the geometry is not the hand-computed one"
    );
}

#[test]
fn pair_geometry_shared_values() {
    check_geometry(&PairGeom::new(&R));
}

negative_control!(
    pair_geometry_shared_values,
    "the bodies relabelled is another geometry",
    expected = "the geometry is not the hand-computed one",
    check_geometry(&PairGeom::new(&[R[1], R[2], R[0]]))
);

/// `closest` on each layout `(positions, pair)`: the least `d²`, the first of a tie.
fn check_closest(layouts: &[([[f64; 2]; 3], usize)]) {
    for (r, want) in layouts {
        let g = PairGeom::new(r);
        assert_eq!(
            (g.closest(), g.r_min()),
            (*want, g.norm()[*want]),
            "closest is not pair {want} for {r:?} (d² {:?})",
            g.d2()
        );
    }
}

/// Each pair the least alone, and each tie: pairs 0 and 1 (`d²` 1, 1, 4), 0 and 2 (1, 2, 1), 1 and 2 (4, 2, 2), and
/// all three (the bodies coincident).
const LAYOUTS: [([[f64; 2]; 3], usize); 7] = [
    ([[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]], 0),
    ([[0.0, 0.0], [4.0, 0.0], [4.0, 3.0]], 1),
    ([[0.0, 0.0], [5.0, 0.0], [0.0, 3.0]], 2),
    ([[-1.0, 0.0], [0.0, 0.0], [1.0, 0.0]], 0),
    ([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], 0),
    ([[0.0, 0.0], [2.0, 0.0], [1.0, 1.0]], 1),
    ([[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]], 0),
];

#[test]
fn pair_geometry_shared_closest() {
    check_closest(&LAYOUTS);
}

negative_control!(
    pair_geometry_shared_closest,
    "a tie of pairs 0 and 2 claimed for pair 2",
    expected = "closest is not pair 2",
    check_closest(&[([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], 2)])
);
