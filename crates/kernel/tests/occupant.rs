//! The stepper occupants and their capability profile (`kernel::occupant`; integrator_contract Part 2, Part 2a;
//! dd_integrator §3.2, §5):
//! - `occupant_profiles_*`: each built occupant's advertised profile, both reversibility flags included, and its
//!   default tier binding equal the Part 2 / Part 2a tables, transcribed here; its force evaluations per `STEP`,
//!   counted, are its profile's; Euler is in the debug selection only; Aarseth–Zare + time-transformed leapfrog is an
//!   allowed future occupant, not built (REQ-INT-013, REQ-INT-014, REQ-INT-015; R-162, R-168, R-500).
//! - `capability_profile_readers`: the readings downstream features take from the profile (Part 2; REQ-INT-078).
//! - `occupant_update_rules`: one `STEP` of each occupant equals dd_integrator §3.2's update rule, written here
//!   independently in its printed velocity form, Yoshida's as three or seven unfused KDK steps (REQ-INT-037).
//! - `occupant_coefficients_*`: the shipped constants equal dd_integrator §3.2 to all printed digits, each printed value
//!   parsing to the shipped f64; Yoshida-4's `w₀` is bitwise `1 − 2w₁` and `w₀ + 2w₁` is exactly 1 in f64 (R-515);
//!   Yoshida-6's `w₀ = 1 − 2(w₁ + w₂ + w₃)`; the kick sequences are `(d_{i−1} + dᵢ)/2`; the f32 constants are the f64
//!   ones rounded once (REQ-INT-037).
//! - `yoshida6_published*`: the Yoshida-6 constants equal dd §3.2's printed digits, their block carrying the citation
//!   (Yoshida 1990, Table 1, solution A), the Yoshida-4 block carrying its own (the fourth-order closed form), and
//!   REQ-INT-090's checks pass on them (REQ-INT-056; R-501).
//! - `yoshida_convergence_order_*`: on the Kepler-embedded fixture the fitted order of the maximum relative energy error
//!   over one orbit rounds to 4 (Yoshida-4) and 6 (Yoshida-6); each free coefficient with its 4th significant digit
//!   changed fits lower; the ladder, the fit and the detection floor are printed (REQ-INT-090; dd §5 test 1a).
//! - `yoshida_order_conditions_*`: the order conditions, evaluated exactly in rational arithmetic on the shipped f64
//!   constants, are within 3·10⁻¹⁵ (Yoshida-4) and 5·10⁻¹⁴ (Yoshida-6); each coefficient moved one unit of its 15th
//!   (Yoshida-4) or 13th (Yoshida-6) significant digit, each way, exceeds the bound (REQ-INT-090).
//! - `step_reversibility`: dd test 3, `STEP(dt)`, negate `p`, `STEP(dt)`, negate `p`, returns the state within the
//!   rounding-level criterion `step_reversal_tol` (REQ-VAL-191, proposed) for KDK, Yoshida-4 and Yoshida-6 at f64 and
//!   f32 over the dt ladder, with no projection or substepping; it prints the evidence, those residuals beside RK4's,
//!   Euler's and the asymmetric Yoshida-4's (REQ-VAL-032, REQ-VAL-191).

use std::cell::Cell;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Neg, Sub};

use kernel::occupant::kdk::{sequence3, sequence7};
use kernel::occupant::table::{default_occupant, table, Binding, Built, Row, Tier};
use kernel::occupant::{
    yoshida4, yoshida6, CapabilityProfile, DebugOccupant, FtleConfidence, Geom, Occupant,
    Selection, State, StepReal, StepperCoefficients,
};
use kernel::physics::energy::energy;
use kernel::real::RealOps;
use kernel::Real;
use num_bigint::BigInt;
use num_rational::BigRational;
use spirv_std::num_traits::{One, Signed, ToPrimitive, Zero};
use validation::{negative_control, STEP_REVERSAL_TOL};

// ── Shared ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A stepper as a function.
type Step<R> = fn(&State<R>, R, &Geom<R>) -> (State<R>, Geom<R>);

/// A precision the tests run at: the kernel's two, with their machine epsilon.
trait P: StepReal + Into<f64> {
    /// The machine epsilon, `2⁻²³` or `2⁻⁵²`.
    const EPS: f64;
    /// `x` rounded once to the precision.
    fn of(x: f64) -> Self;
}

impl P for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> f64 {
        x
    }
}

/// The state of masses `m`, positions `r` and momenta `p` at `R`.
fn state<R: P>(m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> State<R> {
    State {
        m: m.map(R::of),
        r: r.map(|v| v.map(R::of)),
        p: p.map(|v| v.map(R::of)),
    }
}

/// A generic bound state: unequal masses, no symmetry, `Σp = 0`.
fn generic<R: P>() -> State<R> {
    state(
        [0.2, 0.3, 0.5],
        [[-0.7, 0.2], [0.6, 0.5], [0.04, -0.38]],
        [[0.05, -0.12], [-0.09, 0.08], [0.04, 0.04]],
    )
}

/// A close pair, bodies 0 and 1 at distance 0.124, the third far off.
fn close_pair<R: P>() -> State<R> {
    state(
        [0.4, 0.35, 0.25],
        [[0.0, 0.0], [0.12, 0.03], [-0.9, 0.6]],
        [[0.02, 0.3], [-0.05, -0.29], [0.03, -0.01]],
    )
}

/// The fixtures, by name.
fn fixtures<R: P>() -> [(&'static str, State<R>); 2] {
    [("generic", generic()), ("close pair", close_pair())]
}

/// The first `STEP` from `s`, its force evaluation built here.
fn step_from<R: StepReal>(step: Step<R>, s: &State<R>, dt: R) -> (State<R>, Geom<R>) {
    step(s, dt, &Geom::new(&s.m, &s.r))
}

/// `(r, p)` as one vector of 12 components, in f64.
fn flat<R: P>(s: &State<R>) -> Vec<f64> {
    s.r.iter()
        .chain(s.p.iter())
        .flat_map(|v| v.iter().map(|&x| x.into()))
        .collect()
}

/// The Euclidean norm of `v`.
fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// The relative difference of `a` from `b` over `(r, p)`, `‖a − b‖ / ‖b‖`.
fn rel_diff<R: P>(a: &State<R>, b: &State<R>) -> f64 {
    let (x, y) = (flat(a), flat(b));
    let d: Vec<f64> = x.iter().zip(&y).map(|(p, q)| p - q).collect();
    norm(&d) / norm(&y)
}

/// The momenta negated.
fn negated<R: StepReal>(s: &State<R>) -> State<R> {
    State {
        m: s.m,
        r: s.r,
        p: s.p.map(|v| v.map(|x| -x)),
    }
}

// The steppers as functions at each precision.
fn euler<R: StepReal>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    DebugOccupant::Euler.step(s, dt, g)
}
fn kdk<R: StepReal>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    Occupant::Kdk.step(s, dt, g)
}
fn y4<R: StepReal>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    Occupant::Yoshida4.step(s, dt, g)
}
fn y6<R: StepReal>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    Occupant::Yoshida6.step(s, dt, g)
}
fn rk4<R: StepReal>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    Occupant::Rk4.step(s, dt, g)
}

/// The kick sequence of the drift sequence `d`, `cᵢ = (d_{i−1} + dᵢ)/2` with `d₀ = d_{n+1} = 0`, in f64.
fn kicks(d: &[f64]) -> Vec<f64> {
    (0..=d.len())
        .map(|i| {
            let before = if i == 0 { 0.0 } else { d[i - 1] };
            let after = if i == d.len() { 0.0 } else { d[i] };
            (before + after) / 2.0
        })
        .collect()
}

/// The composition of drift sequence `d` (three or seven drifts) at `R`, its kicks [`kicks`] rounded once to `R`.
fn composition<R: P>(d: &[f64], s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    let c: Vec<R> = kicks(d).into_iter().map(R::of).collect();
    let d: Vec<R> = d.iter().map(|&x| R::of(x)).collect();
    match d.len() {
        3 => sequence3(s, dt, g, &[d[0], d[1], d[2]], &[c[0], c[1], c[2], c[3]]),
        7 => sequence7(
            s,
            dt,
            g,
            &[d[0], d[1], d[2], d[3], d[4], d[5], d[6]],
            &[c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]],
        ),
        n => panic!("no composition of {n} drifts"),
    }
}

/// Yoshida-4's drift sequence in f64, from the shipped weights.
fn y4_drift() -> [f64; 3] {
    let (w1, w0) = (f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    [w1, w0, w1]
}

/// Yoshida-6's drift sequence in f64, from the shipped weights.
fn y6_drift() -> [f64; 7] {
    let (w1, w2, w3, w0) = (
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        f64::YOSHIDA6_W0,
    );
    [w3, w2, w1, w0, w1, w2, w3]
}

/// Yoshida-4 with its stages made asymmetric, `(w₁, w₁, w₀)` (REQ-VAL-032's control).
fn y4_asymmetric<R: P>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    let (w1, w0) = (f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    composition(&[w1, w1, w0], s, dt, g)
}

// ── occupant_profiles (REQ-INT-013, REQ-INT-014, REQ-INT-015; R-162, R-168, R-500) ─────────────────────────────

/// A row as the contract prints it: name, order, force evaluations, symplectic, step-reversible, run-reversible, owns
/// time mapping, default binding.
type Printed = (&'static str, u32, u32, bool, bool, bool, bool, Binding);

/// Part 2's and Part 2a's tables as the contract prints them.
const EXPECTED: [Printed; 7] = [
    (
        "Euler",
        1,
        1,
        false,
        false,
        false,
        false,
        Binding::DebugOnly,
    ),
    (
        "KDK leapfrog",
        2,
        1,
        true,
        true,
        false,
        false,
        Binding::Tiers {
            from: Tier::Potato,
            to: Tier::Low,
        },
    ),
    (
        "Yoshida-4",
        4,
        3,
        true,
        true,
        false,
        false,
        Binding::Tiers {
            from: Tier::Medium,
            to: Tier::High,
        },
    ),
    (
        "Yoshida-6",
        6,
        7,
        true,
        true,
        false,
        false,
        Binding::Tiers {
            from: Tier::Ultra,
            to: Tier::Extreme,
        },
    ),
    (
        "RK4",
        4,
        4,
        false,
        false,
        false,
        false,
        Binding::ValidationCrossCheck,
    ),
    (
        "Aarseth–Zare + RK4",
        4,
        4,
        false,
        false,
        false,
        true,
        Binding::CloseEncounter,
    ),
    (
        "Aarseth–Zare + time-transformed leapfrog",
        2,
        1,
        true,
        true,
        false,
        true,
        Binding::AllowedFuture,
    ),
];

/// Checks `table` against [`EXPECTED`], row by row.
fn check_table(table: &[Row]) {
    assert_eq!(table.len(), EXPECTED.len(), "the table's rows");
    for (row, &(name, order, evals, sym, step_rev, run_rev, owns, binding)) in
        table.iter().zip(&EXPECTED)
    {
        let want = CapabilityProfile {
            order,
            force_evals: evals,
            symplectic: sym,
            step_reversible: step_rev,
            run_reversible: run_rev,
            owns_time_mapping: owns,
        };
        assert_eq!(row.name, name, "the table's row order");
        assert_eq!(row.profile, want, "{name}'s profile is not the contract's");
        assert_eq!(
            row.binding, binding,
            "{name}'s default binding is not the contract's"
        );
    }
}

#[test]
fn occupant_profiles_table() {
    check_table(&table());
}

negative_control!(
    occupant_profiles_table,
    "a table giving Yoshida-4 step_reversible false",
    expected = "Yoshida-4's profile is not the contract's",
    {
        let mut t = table();
        t[2].profile.step_reversible = false;
        check_table(&t)
    }
);

/// Checks that no row of `table` is run-reversible: the one run-reversible occupant is logH's TTL time mode (R-162,
/// R-500), which is not in Part 2's or Part 2a's table.
fn check_run_reversible(table: &[Row]) {
    for row in table {
        assert!(
            !row.profile.run_reversible,
            "{} is run-reversible; only logH's TTL time mode is (R-162, R-500)",
            row.name
        );
    }
}

#[test]
fn occupant_profiles_run_reversible() {
    check_run_reversible(&table());
}

negative_control!(
    occupant_profiles_run_reversible,
    "a table giving KDK run_reversible true",
    expected = "KDK leapfrog is run-reversible",
    {
        let mut t = table();
        t[1].profile.run_reversible = true;
        check_run_reversible(&t)
    }
);

/// Checks that every built occupant, production and debug, advertises its row's profile, and is built in exactly
/// one row.
fn check_advertised(table: &[Row], built: &[Selection]) {
    for &sel in built {
        let rows: Vec<&Row> = table
            .iter()
            .filter(|r| r.built == Built::Stepper(sel))
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{sel:?} is built in {} rows, not one",
            rows.len()
        );
        assert_eq!(
            sel.profile(),
            rows[0].profile,
            "{sel:?} advertises a profile other than its row's ({})",
            rows[0].name
        );
    }
    let stepper_rows = table
        .iter()
        .filter(|r| matches!(r.built, Built::Stepper(_)))
        .count();
    assert_eq!(stepper_rows, built.len(), "rows built as steppers");
}

/// Every built occupant, as its selection.
fn built() -> Vec<Selection> {
    Occupant::PRODUCTION
        .iter()
        .map(|&o| Selection::Production(o))
        .chain(DebugOccupant::DEBUG.iter().map(|&o| Selection::Debug(o)))
        .collect()
}

#[test]
fn occupant_profiles_advertised() {
    check_advertised(&table(), &built());
}

negative_control!(
    occupant_profiles_advertised,
    "a table whose RK4 row carries Yoshida-4's profile",
    expected = "Production(Rk4) advertises a profile other than its row's",
    {
        let mut t = table();
        t[4].profile = t[2].profile;
        check_advertised(&t, &built())
    }
);

/// Checks that each tier is covered by exactly one row's tier binding and that `default` gives that row's occupant.
fn check_tier_defaults(table: &[Row], default: fn(Tier) -> Occupant) {
    for tier in Tier::ALL {
        let rows: Vec<&Row> = table
            .iter()
            .filter(
                |r| matches!(r.binding, Binding::Tiers { from, to } if from <= tier && tier <= to),
            )
            .collect();
        assert_eq!(rows.len(), 1, "{tier:?} is bound by {} rows", rows.len());
        assert_eq!(
            rows[0].built,
            Built::Stepper(Selection::Production(default(tier))),
            "{tier:?}'s default is not its row's ({})",
            rows[0].name
        );
    }
}

#[test]
fn occupant_profiles_tier_defaults() {
    check_tier_defaults(&table(), default_occupant);
}

negative_control!(
    occupant_profiles_tier_defaults,
    "a default giving the Medium tier KDK",
    expected = "Medium's default is not its row's",
    check_tier_defaults(&table(), |t| match t {
        Tier::Medium => Occupant::Kdk,
        t => default_occupant(t),
    })
);

/// Checks that Euler is in the debug selection only: no production occupant, no tier's default, its row bound debug
/// only, and selectable as a debug occupant.
fn check_euler(production: &[Selection], debug: &[Selection]) {
    let euler = Selection::Debug(DebugOccupant::Euler);
    assert!(
        !production.contains(&euler),
        "Euler is a production occupant"
    );
    assert!(
        debug.contains(&euler),
        "Euler is not in the debug selection"
    );
    for tier in Tier::ALL {
        assert_ne!(
            Selection::Production(default_occupant(tier)),
            euler,
            "Euler is {tier:?}'s default"
        );
    }
    let t = table();
    let row = t.iter().find(|r| r.built == Built::Stepper(euler));
    assert_eq!(
        row.map(|r| r.binding),
        Some(Binding::DebugOnly),
        "Euler's row is not debug only"
    );
}

#[test]
fn occupant_profiles_euler_debug_only() {
    let production: Vec<Selection> = Occupant::PRODUCTION
        .iter()
        .map(|&o| Selection::Production(o))
        .collect();
    let debug: Vec<Selection> = DebugOccupant::DEBUG
        .iter()
        .map(|&o| Selection::Debug(o))
        .collect();
    check_euler(&production, &debug);
}

negative_control!(
    occupant_profiles_euler_debug_only,
    "a production list that offers Euler",
    expected = "Euler is a production occupant",
    check_euler(&built(), &built())
);

/// Checks that Aarseth–Zare + time-transformed leapfrog is an allowed future occupant and not built, and that no
/// built stepper owns its time mapping (R-162, R-168).
fn check_future(table: &[Row]) {
    let row = table
        .iter()
        .find(|r| r.name == "Aarseth–Zare + time-transformed leapfrog")
        .expect("the row");
    assert_eq!(
        row.binding,
        Binding::AllowedFuture,
        "AZ + TTL is not an allowed future occupant"
    );
    assert_eq!(row.built, Built::NotBuilt, "AZ + TTL is built");
    for sel in built() {
        assert!(
            !sel.profile().owns_time_mapping,
            "{sel:?} owns its time mapping"
        );
    }
}

#[test]
fn occupant_profiles_az_ttl_not_built() {
    check_future(&table());
}

negative_control!(
    occupant_profiles_az_ttl_not_built,
    "a table marking AZ + TTL built",
    expected = "AZ + TTL is built",
    {
        let mut t = table();
        t[6].built = Built::Stepper(Selection::Production(Occupant::Kdk));
        check_future(&t)
    }
);

thread_local! {
    /// The square roots taken on this thread by [`Counted`].
    static ROOTS: Cell<usize> = const { Cell::new(0) };
}

/// f64, counting each square root: a [`Geom::new`] takes three, one per pair, and nothing else in a step takes any.
/// A test harness, not a kernel instantiation.
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

impl One for Counted {
    fn one() -> Counted {
        Counted(1.0)
    }
}

impl StepperCoefficients for Counted {
    const YOSHIDA4_W1: Counted = Counted(f64::YOSHIDA4_W1);
    const YOSHIDA4_W0: Counted = Counted(f64::YOSHIDA4_W0);
    const YOSHIDA4_KICK_1: Counted = Counted(f64::YOSHIDA4_KICK_1);
    const YOSHIDA4_KICK_2: Counted = Counted(f64::YOSHIDA4_KICK_2);
    const YOSHIDA6_W1: Counted = Counted(f64::YOSHIDA6_W1);
    const YOSHIDA6_W2: Counted = Counted(f64::YOSHIDA6_W2);
    const YOSHIDA6_W3: Counted = Counted(f64::YOSHIDA6_W3);
    const YOSHIDA6_W0: Counted = Counted(f64::YOSHIDA6_W0);
    const YOSHIDA6_KICK_1: Counted = Counted(f64::YOSHIDA6_KICK_1);
    const YOSHIDA6_KICK_2: Counted = Counted(f64::YOSHIDA6_KICK_2);
    const YOSHIDA6_KICK_3: Counted = Counted(f64::YOSHIDA6_KICK_3);
    const YOSHIDA6_KICK_4: Counted = Counted(f64::YOSHIDA6_KICK_4);
}

/// The force evaluations one `STEP` of `sel` takes from the generic fixture: its square roots over three.
fn evals(sel: Selection) -> usize {
    let s: State<Counted> = State {
        m: generic::<f64>().m.map(Counted),
        r: generic::<f64>().r.map(|v| v.map(Counted)),
        p: generic::<f64>().p.map(|v| v.map(Counted)),
    };
    let g = Geom::new(&s.m, &s.r);
    let before = ROOTS.get();
    let _ = sel.step(&s, Counted(1e-3), &g);
    let roots = ROOTS.get() - before;
    assert_eq!(
        roots % 3,
        0,
        "{sel:?} took {roots} square roots, not whole evaluations"
    );
    roots / 3
}

/// Checks that each built occupant's force evaluations per `STEP`, counted, are `claimed(sel)`'s `force_evals`.
fn check_evals(claimed: fn(Selection) -> CapabilityProfile) {
    for sel in built() {
        let n = evals(sel);
        println!("{sel:?}: {n} force evaluation(s) per STEP");
        assert_eq!(
            n as u32,
            claimed(sel).force_evals,
            "{sel:?} takes {n} force evaluations per STEP, not its profile's"
        );
    }
}

#[test]
fn occupant_profiles_force_evals() {
    check_evals(Selection::profile);
}

negative_control!(
    occupant_profiles_force_evals,
    "a profile claiming Yoshida-4 takes four force evaluations",
    expected = "Yoshida4) takes 3 force evaluations per STEP, not its profile's",
    check_evals(|sel| match sel {
        Selection::Production(Occupant::Yoshida4) => CapabilityProfile {
            force_evals: 4,
            ..sel.profile()
        },
        sel => sel.profile(),
    })
);

// ── capability_profile_readers (REQ-INT-013, REQ-INT-078) ───────────────────────────────────────────────────────

/// The readings Part 2 defines, per profile checked: energy drift reads the dynamics (`symplectic`), the
/// reversibility diagnostic qualifies (`step_reversible`), the reversibility test binds it (`run_reversible`), the
/// substep count is the wrapper's law (`!owns_time_mapping`), the error exponent (`order`), and the force evaluations
/// a step costs (`force_evals`).
type Reading = (&'static str, bool, bool, bool, bool, u32, u32);

/// [`EXPECTED`]'s rows' readings, then [`RUN_REVERSIBLE`]'s.
const READINGS: [Reading; 8] = [
    ("Euler", false, false, false, true, 1, 1),
    ("KDK leapfrog", true, true, false, true, 2, 1),
    ("Yoshida-4", true, true, false, true, 4, 3),
    ("Yoshida-6", true, true, false, true, 6, 7),
    ("RK4", false, false, false, true, 4, 4),
    ("Aarseth–Zare + RK4", false, false, false, false, 4, 4),
    (
        "Aarseth–Zare + time-transformed leapfrog",
        true,
        true,
        false,
        false,
        2,
        1,
    ),
    ("a run-reversible profile", true, true, true, false, 2, 1),
];

/// A run-reversible profile, as logH's TTL time mode's is (R-162, R-500), which no row of Part 2's or Part 2a's
/// table is: its other fields are illustrative, the reversibility test's reading is what it checks.
const RUN_REVERSIBLE: CapabilityProfile = CapabilityProfile {
    order: 2,
    force_evals: 1,
    symplectic: true,
    step_reversible: true,
    run_reversible: true,
    owns_time_mapping: true,
};

/// The profiles the readings are checked on: the table's rows, then [`RUN_REVERSIBLE`].
fn profiles(table: &[Row]) -> Vec<(&'static str, CapabilityProfile)> {
    table
        .iter()
        .map(|r| (r.name, r.profile))
        .chain([("a run-reversible profile", RUN_REVERSIBLE)])
        .collect()
}

/// Checks each profile's readings against [`READINGS`], FTLE confidence on a saturated and an unsaturated trajectory,
/// and the cost reading at no steps and at `u32::MAX` steps.
fn check_readers(profiles: &[(&str, CapabilityProfile)]) {
    assert_eq!(profiles.len(), READINGS.len(), "the profiles checked");
    for (&(got, p), &(name, dynamics, diagnostic, test, wrapper, exponent, evals)) in
        profiles.iter().zip(&READINGS)
    {
        assert_eq!(got, name, "the profiles' order");
        assert_eq!(p.drift_reads_dynamics(), dynamics, "{name}'s drift reading");
        assert_eq!(
            p.reversibility_diagnostic(),
            diagnostic,
            "{name}'s reversibility diagnostic"
        );
        assert_eq!(p.reversibility_test(), test, "{name}'s reversibility test");
        assert_eq!(
            p.substeps_follow_wrapper(),
            wrapper,
            "{name}'s substep reading"
        );
        assert_eq!(p.error_exponent(), exponent, "{name}'s error exponent");
        for saturated in [false, true] {
            assert_eq!(
                p.ftle_confidence(saturated),
                FtleConfidence {
                    volume_preserving: dynamics,
                    saturated,
                    error_exponent: exponent,
                },
                "{name}'s FTLE confidence, saturated {saturated}"
            );
        }
        assert_eq!(p.force_evaluations(0), 0, "{name}'s cost of no steps");
        assert_eq!(
            p.force_evaluations(u32::MAX),
            u32::MAX as u64 * evals as u64,
            "{name}'s cost of u32::MAX steps"
        );
    }
}

#[test]
fn capability_profile_readers() {
    check_readers(&profiles(&table()));
}

negative_control!(
    capability_profile_readers,
    "a table giving Yoshida-6 order 4",
    expected = "Yoshida-6's error exponent",
    {
        let mut t = table();
        t[3].profile.order = 4;
        check_readers(&profiles(&t))
    }
);

// ── occupant_update_rules (REQ-INT-037) ─────────────────────────────────────────────────────────────────────────

/// The velocities `p/m` of `s`.
fn velocities(s: &State<f64>) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| [s.p[i][0] / s.m[i], s.p[i][1] / s.m[i]])
}

/// The accelerations at positions `r` of masses `m`, dd §3.1, written out here.
fn accel(m: &[f64; 3], r: &[[f64; 2]; 3]) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| {
        let mut a = [0.0; 2];
        for j in 0..3 {
            if j != i {
                let d = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
                let n3 = (d[0] * d[0] + d[1] * d[1]).powf(1.5);
                a[0] += m[j] * d[0] / n3;
                a[1] += m[j] * d[1] / n3;
            }
        }
        a
    })
}

/// The state of masses `m` at positions `r` and velocities `v`.
fn from_velocities(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<f64> {
    State {
        m,
        r,
        p: std::array::from_fn(|i| [m[i] * v[i][0], m[i] * v[i][1]]),
    }
}

/// `x + h·y`, by component.
fn axpy(x: [[f64; 2]; 3], h: f64, y: [[f64; 2]; 3]) -> [[f64; 2]; 3] {
    std::array::from_fn(|i| [x[i][0] + h * y[i][0], x[i][1] + h * y[i][1]])
}

/// Euler as dd §3.2 prints it: `r' = r + dt·v ; v' = v + dt·a(r)`.
fn euler_rule(s: &State<f64>, dt: f64) -> State<f64> {
    let v = velocities(s);
    from_velocities(s.m, axpy(s.r, dt, v), axpy(v, dt, accel(&s.m, &s.r)))
}

/// KDK as dd §3.2 prints it: `v½ = v + (dt/2)·a(r) ; r' = r + dt·v½ ; v' = v½ + (dt/2)·a(r')`.
fn kdk_rule(s: &State<f64>, dt: f64) -> State<f64> {
    let half = axpy(velocities(s), dt / 2.0, accel(&s.m, &s.r));
    let r = axpy(s.r, dt, half);
    from_velocities(s.m, r, axpy(half, dt / 2.0, accel(&s.m, &r)))
}

/// Yoshida-4 as dd §3.2 prints it: `KDK(w₁·dt) ∘ KDK(w₀·dt) ∘ KDK(w₁·dt)`, unfused.
fn y4_rule(s: &State<f64>, dt: f64) -> State<f64> {
    y4_drift().iter().fold(*s, |s, w| kdk_rule(&s, w * dt))
}

/// Yoshida-6 as dd §3.2 prints it: seven KDK steps of the drift sequence, unfused.
fn y6_rule(s: &State<f64>, dt: f64) -> State<f64> {
    y6_drift().iter().fold(*s, |s, w| kdk_rule(&s, w * dt))
}

/// RK4 as dd §3.2 prints it, on `y = (r, p)`, `ẏ = (p/m, m·a(r))`.
fn rk4_rule(s: &State<f64>, dt: f64) -> State<f64> {
    let f = |r: [[f64; 2]; 3], p: [[f64; 2]; 3]| {
        let st = State { m: s.m, r, p };
        let a = accel(&s.m, &r);
        (
            velocities(&st),
            std::array::from_fn(|i| [s.m[i] * a[i][0], s.m[i] * a[i][1]]),
        )
    };
    let k1 = f(s.r, s.p);
    let k2 = f(axpy(s.r, dt / 2.0, k1.0), axpy(s.p, dt / 2.0, k1.1));
    let k3 = f(axpy(s.r, dt / 2.0, k2.0), axpy(s.p, dt / 2.0, k2.1));
    let k4 = f(axpy(s.r, dt, k3.0), axpy(s.p, dt, k3.1));
    let sum =
        |a: [[f64; 2]; 3], b: [[f64; 2]; 3], c: [[f64; 2]; 3], d: [[f64; 2]; 3]| -> [[f64; 2]; 3] {
            std::array::from_fn(|i| {
                std::array::from_fn(|k| a[i][k] + 2.0 * b[i][k] + 2.0 * c[i][k] + d[i][k])
            })
        };
    State {
        m: s.m,
        r: axpy(s.r, dt / 6.0, sum(k1.0, k2.0, k3.0, k4.0)),
        p: axpy(s.p, dt / 6.0, sum(k1.1, k2.1, k3.1, k4.1)),
    }
}

/// The bound on one step's difference from its rule, relative to the state, in units of the f64 epsilon: the two
/// differ by rounding alone (the kernel's momentum form, fused kicks and shared pair geometry against the rule's
/// velocity form). Applied per R-369 (a test tolerance); measured worst printed.
const RULE_EPS: f64 = 64.0;

/// Checks one `STEP` of `step` against `rule` on each fixture at each dt of the reversibility ladder.
fn check_rule(name: &str, step: Step<f64>, rule: fn(&State<f64>, f64) -> State<f64>) {
    let mut worst: f64 = 0.0;
    for (fixture, s) in fixtures::<f64>() {
        for dt in DT_LADDER {
            let (got, _) = step_from(step, &s, dt);
            let err = rel_diff(&got, &rule(&s, dt)) / f64::EPSILON;
            worst = worst.max(err);
            assert!(
                err <= RULE_EPS,
                "{name} on {fixture} at dt {dt:e} is {err:.1} ε from dd §3.2's rule"
            );
        }
    }
    println!("{name}: worst {worst:.2} ε from dd §3.2's rule (bound {RULE_EPS} ε)");
}

#[test]
fn occupant_update_rules() {
    check_rule("Euler", euler, euler_rule);
    check_rule("KDK", kdk, kdk_rule);
    check_rule("Yoshida-4", y4, y4_rule);
    check_rule("Yoshida-6", y6, y6_rule);
    check_rule("RK4", rk4, rk4_rule);
}

negative_control!(
    occupant_update_rules,
    "KDK against a rule with its second half-kick omitted",
    expected = "from dd §3.2's rule",
    check_rule("KDK", kdk, |s, dt| {
        let half = axpy(velocities(s), dt / 2.0, accel(&s.m, &s.r));
        from_velocities(s.m, axpy(s.r, dt, half), half)
    })
);

// ── occupant_coefficients (REQ-INT-037; R-515) ──────────────────────────────────────────────────────────────────

/// dd §3.2's printed Yoshida-4 digits.
const Y4_PRINTED: [&str; 2] = ["1.3512071919596575", "-1.702414383919315"];

/// dd §3.2's printed Yoshida-6 digits: `w₁`, `w₂`, `w₃`, and `w₀` to the 15 significant digits it is printed with.
const Y6_PRINTED: [&str; 4] = [
    "-1.17767998417887",
    "0.235573213359357",
    "0.784513610477560",
    "1.31518632068391",
];

/// Checks Yoshida-4's weights `w₁`, `w₀`: `w₀ + 2w₁` is exactly 1 in f64 and `w₀` is bitwise `1 − 2w₁` (R-515).
fn check_y4_identity(w1: f64, w0: f64) {
    let sum = w0 + 2.0 * w1;
    assert!(sum == 1.0, "w₀ + 2w₁ is {sum:?}, not exactly 1.0 in f64");
    assert_eq!(
        w0.to_bits(),
        (1.0 - 2.0 * w1).to_bits(),
        "w₀ is not bitwise 1 − 2w₁"
    );
}

/// Checks Yoshida-4's shipped constants: the printed digits, the identity, the kicks, and the f32 constants.
fn check_y4_shipped() {
    check_y4_identity(f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    let printed: Vec<f64> = Y4_PRINTED.iter().map(|d| d.parse().unwrap()).collect();
    assert_eq!(
        f64::YOSHIDA4_W1.to_bits(),
        printed[0].to_bits(),
        "w₁ is not dd §3.2's {}",
        Y4_PRINTED[0]
    );
    assert_eq!(
        f64::YOSHIDA4_W0.to_bits(),
        printed[1].to_bits(),
        "w₀ is not dd §3.2's {}",
        Y4_PRINTED[1]
    );
    let c = kicks(&y4_drift());
    assert_eq!(
        f64::YOSHIDA4_KICK_1.to_bits(),
        c[0].to_bits(),
        "c₁ is not w₁/2"
    );
    assert_eq!(
        f64::YOSHIDA4_KICK_2.to_bits(),
        c[1].to_bits(),
        "c₂ is not (w₀ + w₁)/2"
    );
    for (k32, k64) in [
        (f32::YOSHIDA4_W1, f64::YOSHIDA4_W1),
        (f32::YOSHIDA4_W0, f64::YOSHIDA4_W0),
        (f32::YOSHIDA4_KICK_1, f64::YOSHIDA4_KICK_1),
        (f32::YOSHIDA4_KICK_2, f64::YOSHIDA4_KICK_2),
    ] {
        assert_eq!(
            k32.to_bits(),
            (k64 as f32).to_bits(),
            "an f32 constant is not its f64 rounded once"
        );
    }
}

#[test]
fn occupant_coefficients_yoshida4() {
    check_y4_shipped();
    check_y4_identity(
        Y4_PRINTED[0].parse().unwrap(),
        Y4_PRINTED[1].parse().unwrap(),
    );
}

negative_control!(
    occupant_coefficients_yoshida4,
    "RQ-405's interim digits give w₀ + 2w₁ = 0.9999999999999998",
    expected = "w₀ + 2w₁ is 0.9999999999999998, not exactly 1.0 in f64",
    check_y4_identity(1.3512071919596578, -1.7024143839193157)
);

/// Checks Yoshida-6's weights `w₁`, `w₂`, `w₃`, `w₀` against dd §3.2: the first three parse from the printed digits,
/// `w₀` is bitwise `1 − 2(w₁ + w₂ + w₃)` in f64 and agrees with its printed 15 significant digits.
fn check_y6(w: [f64; 4]) {
    for (k, name) in ["w₁", "w₂", "w₃"].iter().enumerate() {
        let printed: f64 = Y6_PRINTED[k].parse().unwrap();
        assert_eq!(
            w[k].to_bits(),
            printed.to_bits(),
            "{name} is not dd §3.2's {}",
            Y6_PRINTED[k]
        );
    }
    assert_eq!(
        w[3].to_bits(),
        (1.0 - 2.0 * (w[0] + w[1] + w[2])).to_bits(),
        "w₀ is not 1 − 2(w₁ + w₂ + w₃) in f64"
    );
    let printed: f64 = Y6_PRINTED[3].parse().unwrap();
    assert_eq!(
        format!("{:.14e}", w[3]),
        format!("{printed:.14e}"),
        "w₀ is not dd §3.2's {}",
        Y6_PRINTED[3]
    );
}

#[test]
fn occupant_coefficients_yoshida6() {
    check_y6([
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        f64::YOSHIDA6_W0,
    ]);
    let c = kicks(&y6_drift());
    for (k, shipped) in [
        f64::YOSHIDA6_KICK_1,
        f64::YOSHIDA6_KICK_2,
        f64::YOSHIDA6_KICK_3,
        f64::YOSHIDA6_KICK_4,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            shipped.to_bits(),
            c[k].to_bits(),
            "c{} is not (d_(i-1) + d_i)/2",
            k + 1
        );
    }
    for (k32, k64) in [
        (f32::YOSHIDA6_W1, f64::YOSHIDA6_W1),
        (f32::YOSHIDA6_W2, f64::YOSHIDA6_W2),
        (f32::YOSHIDA6_W3, f64::YOSHIDA6_W3),
        (f32::YOSHIDA6_W0, f64::YOSHIDA6_W0),
        (f32::YOSHIDA6_KICK_1, f64::YOSHIDA6_KICK_1),
        (f32::YOSHIDA6_KICK_2, f64::YOSHIDA6_KICK_2),
        (f32::YOSHIDA6_KICK_3, f64::YOSHIDA6_KICK_3),
        (f32::YOSHIDA6_KICK_4, f64::YOSHIDA6_KICK_4),
    ] {
        assert_eq!(
            k32.to_bits(),
            (k64 as f32).to_bits(),
            "an f32 constant is not its f64 rounded once"
        );
    }
}

negative_control!(
    occupant_coefficients_yoshida6,
    "w₀ taken as its printed 15 digits instead of computed",
    expected = "w₀ is not 1 − 2(w₁ + w₂ + w₃) in f64",
    check_y6([
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        1.31518632068391,
    ])
);

/// Checks the sequences the steppers read at `R`: the drifts and kicks of dd §3.2 in order, the f64 ones rounded once.
fn check_sequences<R: P>(y4d: [R; 3], y4c: [R; 4], y6d: [R; 7], y6c: [R; 8]) {
    let same = |got: &[R], want: Vec<f64>, what: &str| {
        let got: Vec<f64> = got.iter().map(|&x| x.into()).collect();
        let want: Vec<f64> = want.into_iter().map(|x| R::of(x).into()).collect();
        assert_eq!(got, want, "{what} at {} is not dd §3.2's", R::NAME);
    };
    same(&y4d, y4_drift().to_vec(), "Yoshida-4's drift sequence");
    same(&y4c, kicks(&y4_drift()), "Yoshida-4's kick sequence");
    same(&y6d, y6_drift().to_vec(), "Yoshida-6's drift sequence");
    same(&y6c, kicks(&y6_drift()), "Yoshida-6's kick sequence");
}

#[test]
fn occupant_coefficients_sequences() {
    check_sequences::<f64>(
        yoshida4::drift(),
        yoshida4::kick(),
        yoshida6::drift(),
        yoshida6::kick(),
    );
    check_sequences::<f32>(
        yoshida4::drift(),
        yoshida4::kick(),
        yoshida6::drift(),
        yoshida6::kick(),
    );
}

negative_control!(
    occupant_coefficients_sequences,
    "Yoshida-6's drift sequence with w₁ and w₃ swapped",
    expected = "Yoshida-6's drift sequence at f64 is not dd §3.2's",
    {
        let d = yoshida6::drift::<f64>();
        let swapped = [d[2], d[1], d[0], d[3], d[6], d[5], d[4]];
        check_sequences::<f64>(
            yoshida4::drift(),
            yoshida4::kick(),
            swapped,
            yoshida6::kick(),
        )
    }
);

// ── yoshida6_published (REQ-INT-056; R-57, R-501) ───────────────────────────────────────────────────────────────

/// The Yoshida-6 block.
const Y6_SOURCE: &str = include_str!("../src/occupant/yoshida6.rs");

/// The Yoshida-4 block.
const Y4_SOURCE: &str = include_str!("../src/occupant/yoshida4.rs");

/// The citation every Yoshida block carries (R-501).
const CITATION: [&str; 3] = [
    "Yoshida, H. (1990)",
    "\"Construction of higher order symplectic\n//! integrators\"",
    "*Phys. Lett. A* 150, 262–268",
];

/// Checks that `source` carries the citation, with `extra`, the part of the paper it cites.
fn check_citation(what: &str, source: &str, extra: &str) {
    for part in CITATION.iter().chain([&extra]) {
        assert!(source.contains(part), "{what} does not cite `{part}`");
    }
}

#[test]
fn yoshida6_published() {
    check_y6([
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        f64::YOSHIDA6_W0,
    ]);
    check_citation("the Yoshida-6 block", Y6_SOURCE, "Table 1, solution A");
    check_order_conditions(&y6_drift(), Y6_BOUND, true);
    let (order, _) = fitted_order(&y6_drift(), &Y6_LADDER);
    assert_eq!(order.round(), 6.0, "Yoshida-6's fitted order is {order:.3}");
}

negative_control!(
    yoshida6_published,
    "a Yoshida-6 block without its Table 1 citation",
    expected = "the Yoshida-6 block does not cite `Table 1, solution A`",
    check_citation(
        "the Yoshida-6 block",
        &Y6_SOURCE.replace("Table 1, solution A", "Table 1"),
        "Table 1, solution A"
    )
);

#[test]
fn yoshida6_published_yoshida4_citation() {
    check_citation(
        "the Yoshida-4 block",
        Y4_SOURCE,
        "the fourth-order symmetric triple composition",
    );
}

negative_control!(
    yoshida6_published_yoshida4_citation,
    "a Yoshida-4 block without the paper",
    expected = "the Yoshida-4 block does not cite `Yoshida, H. (1990)`",
    check_citation(
        "the Yoshida-4 block",
        &Y4_SOURCE.replace("Yoshida, H. (1990)", "Yoshida"),
        ""
    )
);

// ── yoshida_convergence_order (REQ-INT-090; dd §5 test 1a) ──────────────────────────────────────────────────────

/// The Kepler-embedded fixture (dd §5 test 1): bodies 0 and 1 of masses `(1 − ε)/2` on an orbit of semi-major axis 1
/// and eccentricity 1/2, starting at apocentre about their centre of mass; body 2 of mass `ε = 10⁻⁹` at rest at
/// distance 20. Returns the state and the pair's period `2π·√(a³/(m₀ + m₁))`.
fn kepler() -> (State<f64>, f64) {
    let eps = 1e-9;
    let (m0, m1) = ((1.0 - eps) / 2.0, (1.0 - eps) / 2.0);
    let (a, e, gm): (f64, f64, f64) = (1.0, 0.5, m0 + m1);
    let sep = a * (1.0 + e);
    let v = (gm * (1.0 - e) / (a * (1.0 + e))).sqrt();
    let mu = m0 * m1 / gm;
    let s = State {
        m: [m0, m1, eps],
        r: [[-m1 / gm * sep, 0.0], [m0 / gm * sep, 0.0], [0.0, 20.0]],
        p: [[0.0, -mu * v], [0.0, mu * v], [0.0, 0.0]],
    };
    (s, 2.0 * std::f64::consts::PI * (a * a * a / gm).sqrt())
}

/// The maximum relative energy error over one orbit, `max_t |E(t) − E₀| / |E₀|`, in `n` steps of the composition of
/// drift sequence `d`.
fn max_energy_error(d: &[f64], n: usize) -> f64 {
    let (mut s, period) = kepler();
    let dt = period / n as f64;
    let mut g = Geom::new(&s.m, &s.r);
    let e0 = energy(&s.m, &s.p, &g.pair);
    let mut worst: f64 = 0.0;
    for _ in 0..n {
        (s, g) = composition(d, &s, dt, &g);
        worst = worst.max(((energy(&s.m, &s.p, &g.pair) - e0) / e0).abs());
    }
    worst
}

/// The dt ladders, in steps per orbit (applied per R-369, R-501: this task's, named in its PR). Each doubles the
/// steps; the finest keeps the error above f64 rounding (about 10⁻¹¹ for Yoshida-4 and 10⁻¹² for Yoshida-6).
const Y4_LADDER: [usize; 4] = [1000, 2000, 4000, 8000];
const Y6_LADDER: [usize; 4] = [150, 300, 600, 1200];

/// The least-squares slope of `log err` against `log dt` over `ladder`, and the ladder's `(steps, dt, err)`.
fn fitted_order(d: &[f64], ladder: &[usize]) -> (f64, Vec<(usize, f64, f64)>) {
    let period = kepler().1;
    let points: Vec<(usize, f64, f64)> = ladder
        .iter()
        .map(|&n| (n, period / n as f64, max_energy_error(d, n)))
        .collect();
    let xs: Vec<f64> = points.iter().map(|p| p.1.ln()).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.2.ln()).collect();
    let (mx, my) = (
        xs.iter().sum::<f64>() / xs.len() as f64,
        ys.iter().sum::<f64>() / ys.len() as f64,
    );
    let num: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let den: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    (num / den, points)
}

/// `x` with one unit added to the magnitude of its `n`-th significant digit.
fn digit_changed(x: f64, n: i32) -> f64 {
    x + x.signum() * 10f64.powi(x.abs().log10().floor() as i32 - n + 1)
}

/// Yoshida-4's drift sequence from `w₁`, `w₀ = 1 − 2w₁`.
fn y4_from(w1: f64) -> Vec<f64> {
    let w0 = 1.0 - 2.0 * w1;
    vec![w1, w0, w1]
}

/// Yoshida-6's drift sequence from `w₁`, `w₂`, `w₃`, `w₀ = 1 − 2(w₁ + w₂ + w₃)`.
fn y6_from(w: [f64; 3]) -> Vec<f64> {
    let w0 = 1.0 - 2.0 * (w[0] + w[1] + w[2]);
    vec![w[2], w[1], w[0], w0, w[0], w[1], w[2]]
}

/// Checks that the composition `d` fits order `order` on `ladder`, printing the ladder and the fit.
fn check_order(what: &str, d: &[f64], ladder: &[usize], order: f64) {
    let (fit, points) = fitted_order(d, ladder);
    for (n, dt, err) in &points {
        println!("{what}: {n} steps/orbit, dt {dt:.4e}, max |ΔE/E₀| {err:.3e}");
    }
    println!("{what}: fitted order {fit:.4}");
    assert_eq!(
        fit.round(),
        order,
        "{what}'s fitted order {fit:.3} does not round to {order}"
    );
}

/// The detection floor of the order test for the free coefficient `k` of `from` (shipped `w`): the smallest relative
/// change `δ`, on a quarter-decade grid from 10⁻³ to 10⁻¹⁰, whose composition fits an order that rounds below `order`.
fn detection_floor(
    from: fn(&[f64]) -> Vec<f64>,
    w: &[f64],
    k: usize,
    ladder: &[usize],
    order: f64,
) -> Option<f64> {
    (12..=40)
        .map(|q| 10f64.powf(-(q as f64) / 4.0))
        .rfind(|&delta| {
            let mut v = w.to_vec();
            v[k] *= 1.0 + delta;
            fitted_order(&from(&v), ladder).0.round() < order
        })
}

/// Yoshida-4's sequence from its free weight.
fn y4_of(w: &[f64]) -> Vec<f64> {
    y4_from(w[0])
}

/// Yoshida-6's sequence from its free weights.
fn y6_of(w: &[f64]) -> Vec<f64> {
    y6_from([w[0], w[1], w[2]])
}

/// Prints the detection floor for coefficient `k` and checks it is at most `control`, the relative change of the
/// coefficient's 4th-digit control.
fn check_floor(
    what: &str,
    from: fn(&[f64]) -> Vec<f64>,
    w: &[f64],
    k: usize,
    ladder: &[usize],
    order: f64,
) {
    let control = (digit_changed(w[k], 4) - w[k]).abs() / w[k].abs();
    let floor = detection_floor(from, w, k, ladder, order);
    println!(
        "{what}: detection floor {floor:?} (relative change), the 4th-digit control {control:.2e}"
    );
    assert!(
        floor.is_some_and(|f| f <= control),
        "{what}'s floor {floor:?} is above its control's {control:.2e}"
    );
}

#[test]
fn yoshida_convergence_order_yoshida4() {
    check_order("Yoshida-4", &y4_drift(), &Y4_LADDER, 4.0);
    check_floor(
        "Yoshida-4 w₁",
        y4_of,
        &[f64::YOSHIDA4_W1],
        0,
        &Y4_LADDER,
        4.0,
    );
}

negative_control!(
    yoshida_convergence_order_yoshida4,
    "Yoshida-4's w₁ with its 4th significant digit changed",
    expected = "does not round to 4",
    check_order(
        "Yoshida-4 (w₁ changed)",
        &y4_from(digit_changed(f64::YOSHIDA4_W1, 4)),
        &Y4_LADDER,
        4.0
    )
);

/// Yoshida-6's shipped free weights.
const Y6_FREE: [f64; 3] = [f64::YOSHIDA6_W1, f64::YOSHIDA6_W2, f64::YOSHIDA6_W3];

/// Yoshida-6's free weights with weight `k`'s 4th significant digit changed.
#[cfg(feature = "controls")]
fn y6_changed(k: usize) -> Vec<f64> {
    let mut w = Y6_FREE;
    w[k] = digit_changed(w[k], 4);
    y6_from(w)
}

macro_rules! y6_order_test {
    ($name:ident, $k:expr, $w:literal) => {
        #[test]
        fn $name() {
            check_order("Yoshida-6", &y6_drift(), &Y6_LADDER, 6.0);
            check_floor(
                concat!("Yoshida-6 ", $w),
                y6_of,
                &Y6_FREE,
                $k,
                &Y6_LADDER,
                6.0,
            );
        }

        negative_control!(
            $name,
            "Yoshida-6 with one free weight's 4th significant digit changed",
            expected = "does not round to 6",
            check_order(
                concat!("Yoshida-6 (", $w, " changed)"),
                &y6_changed($k),
                &Y6_LADDER,
                6.0
            )
        );
    };
}

y6_order_test!(yoshida_convergence_order_yoshida6_w1, 0, "w₁");
y6_order_test!(yoshida_convergence_order_yoshida6_w2, 1, "w₂");
y6_order_test!(yoshida_convergence_order_yoshida6_w3, 2, "w₃");

// ── yoshida_order_conditions (REQ-INT-090; dd §5 test 1a) ───────────────────────────────────────────────────────

/// An exact rational.
type Q = BigRational;

/// `x`'s exact value.
fn q(x: f64) -> Q {
    Q::from_float(x).expect("a finite coefficient")
}

/// The integer `n` as a rational.
fn qi(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

/// An element of the free associative algebra on `X = A₁` (degree 1) and `Y = A₃` (degree 3), truncated above degree
/// 5: each word, a string over `X` and `Y`, with its coefficient. `A₅` enters the composition's logarithm linearly,
/// as `Σw⁵·A₅`, and is not carried.
type Alg = BTreeMap<String, Q>;

/// A word's degree.
fn degree(w: &str) -> usize {
    w.chars().map(|c| if c == 'X' { 1 } else { 3 }).sum()
}

fn mul(a: &Alg, b: &Alg) -> Alg {
    let mut out = Alg::new();
    for (u, x) in a {
        for (v, y) in b {
            let w = format!("{u}{v}");
            if degree(&w) <= 5 {
                *out.entry(w).or_insert_with(Q::zero) += x * y;
            }
        }
    }
    out
}

fn add(a: &Alg, b: &Alg, k: &Q) -> Alg {
    let mut out = a.clone();
    for (w, x) in b {
        *out.entry(w.clone()).or_insert_with(Q::zero) += x * k;
    }
    out
}

/// `exp(a)` truncated at degree 5, `a` without a constant term.
fn exp(a: &Alg) -> Alg {
    let mut out = Alg::from([(String::new(), Q::one())]);
    let mut term = out.clone();
    for k in 1..=5 {
        term = mul(&term, a);
        term = term.into_iter().map(|(w, x)| (w, x / qi(k))).collect();
        out = add(&out, &term, &Q::one());
    }
    out
}

/// `log(a)` truncated at degree 5, `a` with constant term 1.
fn log(a: &Alg) -> Alg {
    let x = add(a, &Alg::from([(String::new(), Q::one())]), &-Q::one());
    let mut out = Alg::new();
    let mut power = Alg::from([(String::new(), Q::one())]);
    for k in 1..=5i64 {
        power = mul(&power, &x);
        let sign = if k % 2 == 1 { Q::one() } else { -Q::one() };
        out = add(&out, &power, &(sign / qi(k)));
    }
    out
}

/// The coefficient of `[A₁,[A₁,A₃]]` in the logarithm of the composition of `S(wᵢh) = exp(wᵢhA₁ + wᵢ³h³A₃ + …)`: the
/// multidegree-(2, 1) Lie elements are its multiples, so it is the logarithm's coefficient of the word `XXY`.
fn nested(w: &[Q]) -> Q {
    let mut p = Alg::from([(String::new(), Q::one())]);
    for wi in w {
        let gen = Alg::from([("X".to_owned(), wi.clone()), ("Y".to_owned(), wi * wi * wi)]);
        p = mul(&p, &exp(&gen));
    }
    log(&p).get("XXY").cloned().unwrap_or_else(Q::zero)
}

/// The order-condition residuals of the symmetric composition `d`, exactly: `|Σw − 1|`, `|Σw³| / Σ|w|³`, and, for
/// order 6, `|Σw⁵| / Σ|w|⁵` and the nested condition over the same coefficient with `|w|`.
fn residuals(d: &[f64], six: bool) -> Vec<(&'static str, f64)> {
    let w: Vec<Q> = d.iter().map(|&x| q(x)).collect();
    let a: Vec<Q> = w.iter().map(|x| x.abs()).collect();
    let pow = |v: &[Q], k: u32| v.iter().fold(Q::zero(), |s, x| s + num_traits_pow(x, k));
    let ratio = |n: Q, d: Q| (n / d).abs().to_f64().expect("a finite ratio");
    let mut out = vec![
        ("|Σw − 1|", (pow(&w, 1) - Q::one()).abs().to_f64().unwrap()),
        ("|Σw³| / Σ|w|³", ratio(pow(&w, 3), pow(&a, 3))),
    ];
    if six {
        out.push(("|Σw⁵| / Σ|w|⁵", ratio(pow(&w, 5), pow(&a, 5))));
        out.push(("nested [A₁,[A₁,A₃]]", ratio(nested(&w), nested(&a))));
    }
    out
}

/// `x^k`.
fn num_traits_pow(x: &Q, k: u32) -> Q {
    (0..k).fold(Q::one(), |p, _| p * x)
}

/// The bounds (dd §5 test 1a).
const Y4_BOUND: f64 = 3e-15;
const Y6_BOUND: f64 = 5e-14;

/// Checks that every order-condition residual of `d` is at most `bound`, printing them.
fn check_order_conditions(d: &[f64], bound: f64, six: bool) {
    for (what, r) in residuals(d, six) {
        println!("{what} = {r:.3e} (bound {bound:e})");
        assert!(
            r <= bound,
            "the order condition {what} = {r:.3e} exceeds {bound:e}"
        );
    }
}

#[test]
fn yoshida_order_conditions_yoshida4_shipped() {
    check_order_conditions(&y4_drift(), Y4_BOUND, false);
}

negative_control!(
    yoshida_order_conditions_yoshida4_shipped,
    "the nested condition of Yoshida-4, which a fourth-order composition does not meet",
    expected = "exceeds 3e-15",
    check_order_conditions(&y4_drift(), Y4_BOUND, true)
);

/// Yoshida-4's sequence with `w₁` (`k = 0`) or `w₀` (`k = 1`) moved by `sign` units of its 15th significant digit,
/// the other kept.
#[cfg(feature = "controls")]
fn y4_moved(k: usize, sign: f64) -> Vec<f64> {
    let (mut w1, mut w0) = (f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    let unit = |x: f64| 10f64.powi(x.abs().log10().floor() as i32 - 14);
    if k == 0 {
        w1 += sign * unit(w1);
    } else {
        w0 += sign * unit(w0);
    }
    vec![w1, w0, w1]
}

/// Yoshida-6's sequence with free weight `k` moved by `sign` units of its 13th significant digit, `w₀` recomputed.
#[cfg(feature = "controls")]
fn y6_moved(k: usize, sign: f64) -> Vec<f64> {
    let mut w = Y6_FREE;
    w[k] += sign * 10f64.powi(w[k].abs().log10().floor() as i32 - 12);
    y6_from(w)
}

macro_rules! condition_test {
    ($name:ident, $d:expr, $bound:expr, $six:expr, $control:expr, $what:literal, $expected:literal) => {
        #[test]
        fn $name() {
            check_order_conditions(&$d, $bound, $six);
        }

        negative_control!(
            $name,
            $what,
            expected = $expected,
            check_order_conditions(&$control, $bound, $six)
        );
    };
}

condition_test!(
    yoshida_order_conditions_yoshida4_w1_up,
    y4_drift(),
    Y4_BOUND,
    false,
    y4_moved(0, 1.0),
    "Yoshida-4's w₁ up one unit of its 15th digit",
    "exceeds 3e-15"
);
condition_test!(
    yoshida_order_conditions_yoshida4_w1_down,
    y4_drift(),
    Y4_BOUND,
    false,
    y4_moved(0, -1.0),
    "Yoshida-4's w₁ down one unit of its 15th digit",
    "exceeds 3e-15"
);
condition_test!(
    yoshida_order_conditions_yoshida4_w0_up,
    y4_drift(),
    Y4_BOUND,
    false,
    y4_moved(1, 1.0),
    "Yoshida-4's w₀ up one unit of its 15th digit",
    "exceeds 3e-15"
);
condition_test!(
    yoshida_order_conditions_yoshida4_w0_down,
    y4_drift(),
    Y4_BOUND,
    false,
    y4_moved(1, -1.0),
    "Yoshida-4's w₀ down one unit of its 15th digit",
    "exceeds 3e-15"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w1_up,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(0, 1.0),
    "Yoshida-6's w₁ up one unit of its 13th digit",
    "exceeds 5e-14"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w1_down,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(0, -1.0),
    "Yoshida-6's w₁ down one unit of its 13th digit",
    "exceeds 5e-14"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w2_up,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(1, 1.0),
    "Yoshida-6's w₂ up one unit of its 13th digit",
    "exceeds 5e-14"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w2_down,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(1, -1.0),
    "Yoshida-6's w₂ down one unit of its 13th digit",
    "exceeds 5e-14"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w3_up,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(2, 1.0),
    "Yoshida-6's w₃ up one unit of its 13th digit",
    "exceeds 5e-14"
);
condition_test!(
    yoshida_order_conditions_yoshida6_w3_down,
    y6_drift(),
    Y6_BOUND,
    true,
    y6_moved(2, -1.0),
    "Yoshida-6's w₃ down one unit of its 13th digit",
    "exceeds 5e-14"
);

// ── step_reversibility (REQ-VAL-032, REQ-VAL-191; dd §5 test 3) ─────────────────────────────────────────────────

/// The dt ladder (applied per R-369: this task's, named in its PR): `10⁻¹` and `10⁻²` above the default macro-step,
/// `dt_macro = 10⁻³` (dd_integrator §2, R-132), and `10⁻³/64`, the smallest substep at the default `dt_macro` and
/// `N_max = 64`.
const DT_LADDER: [f64; 4] = [1e-1, 1e-2, 1e-3, 1e-3 / 64.0];

/// The criterion, `step_reversal_tol` (REQ-VAL-191, provisional until M3), in units of the precision's epsilon.
const TOL: f64 = STEP_REVERSAL_TOL.number();

/// A fixture's dynamical time `τ`, the shortest two-body time `√(r³ / (G(mᵢ + mⱼ)))` over its pairs, `G = 1`: 1.09
/// for the generic fixture, 0.0502 for the close pair.
fn dynamical_time<R: P>(s: &State<R>) -> f64 {
    let mut tau = f64::INFINITY;
    for (i, j) in [(0, 1), (1, 2), (0, 2)] {
        let dx: f64 = s.r[i][0].into() - s.r[j][0].into();
        let dy: f64 = s.r[i][1].into() - s.r[j][1].into();
        let r = (dx * dx + dy * dy).sqrt();
        let m: f64 = s.m[i].into() + s.m[j].into();
        tau = tau.min((r * r * r / m).sqrt());
    }
    tau
}

/// The domain's edge at `R`, the smallest `dt/τ` at which test 3 discriminates, as `step_reversal_tol`'s proposal
/// states it (REQ-VAL-191; its register doc): `10⁻²` at f64 and `0.3` at f32. At the edge itself, `dt = edge·τ` on
/// every fixture, and at every dt of the ladder above it, dd's named controls, RK4 and the asymmetric Yoshida-4, read
/// above the criterion (`step_reversibility_domain`). Below it a control's time-asymmetry, relative `O((dt/τ)⁴)` for
/// the asymmetric Yoshida-4 and `O((dt/τ)⁶)` for RK4 (even order: the `h⁵` terms of `Φ_h` and `Φ_{−h}` cancel in
/// `Φ_{−h}∘Φ_h`), falls under the precision's rounding, and a residual under the criterion says nothing.
fn domain_edge<R: P>() -> f64 {
    if R::EPS == f64::EPSILON {
        1e-2
    } else {
        0.3
    }
}

/// dd test 3's residual of `step` from `s` at `dt`: `STEP(dt)`, negate `p`, `STEP(dt)`, negate `p`, its distance from
/// `s` over `(r, p)` relative to `‖s‖`, in units of `R`'s epsilon. No projection, no substepping.
fn reversal<R: P>(step: Step<R>, s: &State<R>, dt: R) -> f64 {
    let (forward, g) = step_from(step, s, dt);
    let (back, _) = step(&negated(&forward), dt, &g);
    rel_diff(&negated(&back), s) / R::EPS
}

/// One residual: `dt/τ`, `dt`, the fixture, and the residual in `ε`.
type Residual = (f64, f64, &'static str, f64);

/// The residuals of `step` at `R` on every fixture, at every dt of the ladder and at `dt = edge·τ`, each printed and
/// those below `edge` labelled outside the domain.
fn reversals<R: P>(name: &str, step: Step<R>, edge: f64) -> Vec<Residual> {
    let mut out = Vec::new();
    for (fixture, s) in fixtures::<R>() {
        let tau = dynamical_time(&s);
        for dt in DT_LADDER.into_iter().chain([edge * tau]) {
            let ratio = dt / tau;
            let r = reversal(step, &s, R::of(dt));
            let domain = if ratio >= edge {
                ""
            } else {
                " (outside the domain: non-discriminating)"
            };
            println!(
                "{name} at {}, {fixture}, dt {dt:.4e} (dt/τ {ratio:.3e}): residual {r:.3e} ε{domain}",
                R::NAME
            );
            out.push((ratio, dt, fixture, r));
        }
    }
    out
}

/// Checks that `step` returns every fixture within [`TOL`] at every dt of the domain, at `R`.
fn check_reversible_at<R: P>(name: &str, step: Step<R>) {
    let edge = domain_edge::<R>();
    for (ratio, dt, fixture, r) in reversals(name, step, edge) {
        if ratio >= edge {
            assert!(
                r <= TOL,
                "{name}'s residual {r:.3e} ε at {}, dt {dt:.4e}, {fixture} exceeds step_reversal_tol {TOL} ε",
                R::NAME
            );
        }
    }
}

/// [`check_reversible_at`] at f64 and f32.
fn check_reversible(name: &str, step64: Step<f64>, step32: Step<f32>) {
    check_reversible_at(name, step64);
    check_reversible_at(name, step32);
}

#[test]
fn step_reversibility() {
    check_reversible("KDK", kdk, kdk);
    check_reversible("Yoshida-4", y4, y4);
    check_reversible("Yoshida-6", y6, y6);
    // Euler, not step-reversible, printed as evidence for REQ-VAL-191; dd's named controls are checked by
    // `step_reversibility_domain`.
    reversals::<f64>("Euler", euler, domain_edge::<f64>());
    reversals::<f32>("Euler", euler, domain_edge::<f32>());
}

negative_control!(
    step_reversibility,
    "Yoshida-4 with its stages made asymmetric (w₁, w₁, w₀)",
    expected = "exceeds step_reversal_tol",
    check_reversible("Yoshida-4 asymmetric", y4_asymmetric, y4_asymmetric)
);

/// Checks that `step`, not step-reversible, reads above [`TOL`] on every fixture at `dt = edge·τ` and at every dt of
/// the ladder above it, at `R`: the domain is one where test 3 can fail.
fn check_discriminates_at<R: P>(name: &str, step: Step<R>, edge: f64) {
    for (ratio, dt, fixture, r) in reversals(name, step, edge) {
        if ratio >= edge {
            assert!(
                r > TOL,
                "{name} reads {r:.3e} ε at {}, dt {dt:.4e}, {fixture}, not above step_reversal_tol {TOL} ε: \
                 test 3 does not discriminate there",
                R::NAME
            );
        }
    }
}

/// dd test 3's controls, RK4 and the asymmetric Yoshida-4, at f64 and f32 over the domains of edges `edge64` and
/// `edge32`.
fn check_domain(edge64: f64, edge32: f64) {
    let y4a = "Yoshida-4 asymmetric (w₁, w₁, w₀)";
    check_discriminates_at::<f64>("RK4", rk4, edge64);
    check_discriminates_at::<f32>("RK4", rk4, edge32);
    check_discriminates_at::<f64>(y4a, y4_asymmetric, edge64);
    check_discriminates_at::<f32>(y4a, y4_asymmetric, edge32);
}

#[test]
fn step_reversibility_domain() {
    check_domain(domain_edge::<f64>(), domain_edge::<f32>());
}

negative_control!(
    step_reversibility_domain,
    "the first draft's f32 edge, dt/τ 10⁻¹",
    expected = "test 3 does not discriminate there",
    check_domain(domain_edge::<f64>(), 1e-1)
);
