//! Aarseth–Zare (`kernel::regularisation::az`; integrator_contract Part 2b § "Aarseth–Zare: the equations, the
//! reference body, the Levi-Civita inverse and the step control (transcribed)", Part 2a, Part 6; R-927, R-953):
//! - `az_doubly_degenerate_is_finite`: an f32 state degenerate at the interval's entry, `A` and `B` both below `TINY`
//!   so that `TINY·TINY` underflows and the entry size is infinite, ends its march at the explicit finiteness test,
//!   its state latched finite; the same state degenerate only mid-interval takes a finite step, a counted cap hit, and
//!   does not reach the test; its control removes the test, and the march carries a non-finite state on (REQ-INT-105's
//!   finiteness arm; R-953).
//! - `regularisation_compose_az_runs`: every stepper × Aarseth–Zare pair is selected, runs through the seam (the state
//!   moves and stays finite, the cadence follows each kept step) and carries the Part 2b table's warning where the
//!   table marks it measured bad and none otherwise; its control is an Aarseth–Zare that takes no step (REQ-INT-017's
//!   arm; R-561).
//! - `regularisation_compose_az_profile`: the AZ + RK4 profile is not symplectic, step- or run-reversible, owns its
//!   time mapping, order 4 with 4 evaluations (Part 2a's table); every pair's profile is non-symplectic; and the slot
//!   takes a Mikkola–Tanikawa leapfrog type, whose row is symplectic (REQ-INT-024); its control is the stepper's own
//!   row reported for the pair.
//! - `owns_time_mapping_composed_az`: under Aarseth–Zare `ADVANCE` to a sync boundary ends at exactly its time, and to a
//!   target between boundaries at the boundary before it, in physical time and through the seam's macro-step grid,
//!   bitwise the same state (R-565; REQ-INT-021's arm); its control lands on the off-grid target.
//! - `tau_schedule_bound`: over fuzzed ICs, at every sync boundary `t_k` a copy's running total of steps is at most
//!   `max(N_min, step_budget_rate·t_k)`, and a copy that would exceed it has ended undetermined and is never run on;
//!   no cap per interval is asserted, and the largest count in one interval is printed for the record; its control
//!   resets the counter at each boundary (REQ-INT-054; R-604, R-953).
//! - `az_reference_body_tie_break`, `az_lc_inverse_branches`, `az_round_trip`, `az_energy_held`: the reference-body
//!   rule with its tie-break, the conditioned inverse's branches, registration and return, and the interval's energy;
//!   their controls a last-maximum tie-break, the unstable inverse, a quarter-turned `u₁` and a flow whose energy is
//!   not the state's.

use kernel::driver::advance::{Advance, AdvanceParams, Advanced, Cadence};
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{
    CapabilityProfile, DebugOccupant, Geom, Occupant, Selection, State, Stepper,
};
use kernel::regularisation::az::{
    boundary_at_or_before, entry_size, lc, lc_inverse, lc_inverse_unstable, march, reference_body,
    register, step_size, AarsethZare, AzAdvanced, AzConfig, AzCount, AzP, AzParams, AzQ, AzReal,
    AzStepper, AzSystem, End, ExplicitTest, StepSize, Watch,
};
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::pairs::{Choice, Pair, PairTable, Status};
use kernel::regularisation::{Composed, Kind, Regularisation};
use validation::negative_control;

// ── Precisions and fixtures ────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AzReal + Into<f64> {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// `x` at `R`'s width.
fn v<R: AzReal>(x: f64) -> R {
    R::at_width(x as f32, x)
}

/// Test values for the step control: `η = 0.01`, `Δ_sync = 1/8` (exact in binary), the reference's `TINY` (`1e-37` at
/// f32, `1e-300` at f64, R-953) and landing tolerance (`1e-5`, `1e-14`; RQ-792), R-160's `f = 0.02`, Part 2b's `|R₃|`
/// floor `1e-300`, a budget that does not bind, the clamp and the secant on. Test inputs, not the calibrations.
fn params<R: AzReal>() -> AzParams<R> {
    AzParams {
        eta: v(0.01),
        delta_sync: v(0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: v(0.02),
        r3_floor: v(1e-300),
        land_eps_rel: R::at_width(1e-5, 1e-14),
        step_budget_rate: v(1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
    }
}

/// The occupant's configuration in these tests: [`params`].
struct TestConfig;

impl AzConfig for TestConfig {
    fn params<R: AzReal>() -> AzParams<R> {
        params()
    }
}

/// A cadence that counts its calls and never ends the march.
#[derive(Default)]
struct Counting {
    calls: u32,
}

impl<R> Cadence<R> for Counting {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        false
    }
}

/// A state at `R` from f64 masses, positions and velocities.
fn state<R: P>(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<R> {
    State {
        m: m.map(R::of),
        r: r.map(|x| x.map(R::of)),
        p: [0, 1, 2].map(|i| [R::of(m[i] * v[i][0]), R::of(m[i] * v[i][1])]),
    }
}

/// Burrau's three bodies of masses 3, 4, 5 on their triangle, with small momenta summing to zero, in the gauge `M = 1`
/// (masses over 12, lengths over 5).
fn burrau<R: P>() -> State<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    let v = [0, 1, 2].map(|i| [p[i][0] / m[i], p[i][1] / m[i]]);
    state(m, r, v)
}

/// Whether every value of `s` is finite.
fn finite_state<R: P>(s: &State<R>) -> bool {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .all(|&x| Into::<f64>::into(x).is_finite())
}

/// The bits of a state's positions and momenta.
fn bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| Into::<f64>::into(x).to_bits())
        .collect()
}

// ── The doubly degenerate state ────────────────────────────────────────────────────────────────────────────────

/// Three bodies within `3e-38` of each other at rest, so that at registration both `A` and `B` are below f32's
/// `TINY = 1e-37`: `TINY·TINY` underflows to `0` and the entry size is infinite.
fn degenerate() -> State<f32> {
    state(
        [1.0 / 3.0; 3],
        [[0.0, 0.0], [1e-38, 0.0], [0.0, 2e-38]],
        [[0.0; 2]; 3],
    )
}

/// No finiteness test: the control's march, which takes every state as finite.
#[cfg(feature = "controls")]
struct NoTest;

#[cfg(feature = "controls")]
impl<R> Watch<R> for NoTest {
    fn finite(&mut self, _q: &AzQ<R>, _p: &AzP<R>) -> bool {
        true
    }
}

/// The degenerate fixture's march with the finiteness test `X`, a budget of 64 steps so that a march carried on stops.
fn degenerate_march<X: Watch<f32>>(mut watch: X) -> AzAdvanced<f32> {
    let mut p = params::<f32>();
    p.n_min = 64;
    p.step_budget_rate = 0.0;
    march::<Rk4, f32, Counting, X>(
        &degenerate(),
        0,
        1,
        AzCount::new(),
        &p,
        &mut Counting::default(),
        &mut watch,
    )
}

/// Checks that a march from the degenerate fixture ends at the finiteness test with no non-finite state carried past.
fn check_degenerate_ends(out: &AzAdvanced<f32>) {
    assert!(
        finite_state(&out.state) && out.t.is_finite(),
        "a non-finite state was carried past the finiteness test: {:?} at t = {}",
        out.state,
        out.t
    );
    assert_eq!(
        out.count.end,
        End::NonFinite,
        "the march ends at the finiteness test"
    );
    assert_eq!(
        out.count.steps, 1,
        "one step, the infinite one, reached the test"
    );
    assert_eq!(out.count.accepted, 0, "no non-finite step is kept");
    assert_eq!(out.boundary, 0, "the interval did not land");
}

#[test]
fn az_doubly_degenerate_is_finite() {
    let p = params::<f32>();
    let (system, q, pm) = register(&degenerate(), p.tiny, p.r3_floor);
    let (a, b) = AzSystem::ab(&q);
    assert!(
        a < p.tiny && b < p.tiny,
        "both factors below TINY: A = {a:e}, B = {b:e}"
    );
    assert_eq!(p.tiny * p.tiny, 0.0, "TINY·TINY underflows at f32");
    let entry = entry_size(&q, &p);
    assert!(entry.is_infinite(), "the entry size is infinite: {entry:e}");

    let out = degenerate_march(ExplicitTest);
    check_degenerate_ends(&out);
    assert_eq!(out.count.non_finite_entries, 1);
    assert_eq!(out.count.floored, 1, "the floor signal fired on the step");
    assert_eq!(out.count.re_registrations, 1);
    // The latched state is the registration's image, the last finite state.
    assert_eq!(
        bits(&out.state),
        bits(&system.image(&q, &pm)),
        "the latched state is the registered one"
    );

    // The same state degenerate only mid-interval, the entry size finite from a registration away from it: the
    // per-step rule skips the non-finite constraint `c₀` and takes the finite entry size, a counted cap hit, where the
    // entry-degenerate interval's infinite size reaches the stepper.
    let entry = p.eta * p.delta_sync;
    let sized = step_size(&system, &q, &pm, entry, &p);
    assert_eq!(
        sized.h.to_bits(),
        entry.to_bits(),
        "a finite step, the entry size: {:e}",
        sized.h
    );
    assert!(
        sized.cap_hit,
        "the non-finite per-step size is a counted cap hit"
    );
    assert!(sized.floored, "the floor signal fires");
    // What the stepper then gives is recorded, not asserted: at f32 the three bodies within `2·TINY` of each other put
    // `|R₃|²` below the smallest subnormal too, and `A·B/|R₃|³` reads `0/0` (the PR's note for the physics review).
    let (q1, p1) = <Rk4 as AzStepper>::regularised_step(&system, &q, &pm, sized.h);
    eprintln!(
        "az_doubly_degenerate_is_finite: the mid-interval step of size {:e} gives a state finite = {}",
        sized.h,
        ExplicitTest.finite(&q1, &p1)
    );
}

negative_control!(
    az_doubly_degenerate_is_finite,
    "the march without its finiteness test, which carries the non-finite state on",
    expected = "a non-finite state was carried past the finiteness test",
    check_degenerate_ends(&degenerate_march(NoTest))
);

// ── Every stepper × Aarseth–Zare pair runs ─────────────────────────────────────────────────────────────────────

/// Aarseth–Zare with the test values.
type Az = AarsethZare<TestConfig>;

/// A stand-in Aarseth–Zare that takes no step: the control's.
#[cfg(feature = "controls")]
struct Still;

#[cfg(feature = "controls")]
impl Regularisation for Still {
    const KIND: Kind = Kind::AarsethZare;

    fn profile<S: Stepper>() -> CapabilityProfile {
        Az::profile::<S>()
    }

    fn advance<S: Stepper, R: kernel::driver::advance::AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        _t_now: u32,
        _t_target: u32,
        _params: &AdvanceParams<R>,
        _cadence: &mut C,
    ) -> Advanced<R> {
        Advanced {
            state: *state,
            geom: *geom,
            substeps: 0,
        }
    }
}

/// The seam's ADVANCE of `A` over two sync intervals at `R`: macro-step grid `dt_macro = 1/64`, `t_now = 0`,
/// `t_target = 16` (`t = 1/4`), with the cadence's calls.
fn seam_run<A: Advance, R: P>() -> (Advanced<R>, u32) {
    let s = burrau::<R>();
    let g = Geom::new(&s.m, &s.r);
    let mut cadence = Counting::default();
    let out = A::advance(
        &s,
        &g,
        0,
        16,
        &AdvanceParams {
            dt_macro: R::of(1.0 / 64.0),
            n_sub: 1,
        },
        &mut cadence,
    );
    (out, cadence.calls)
}

/// Checks that the stepper `S` composed with the regularisation `G` runs at f64 and f32, and that the pair's warning
/// in Part 2b's table is the table's.
fn check_pair_runs<S: Stepper, G: Regularisation>(name: &str, stepper: Selection) {
    fn at<A: Advance, R: P>(name: &str) {
        let (out, calls) = seam_run::<A, R>();
        assert_ne!(
            bits(&out.state),
            bits(&burrau::<R>()),
            "{name} × Aarseth–Zare ({}) did not run: the state did not move",
            R::NAME
        );
        assert!(finite_state(&out.state), "{name} ({}): not finite", R::NAME);
        assert!(out.substeps > 0, "{name}: no step");
        assert_eq!(
            calls, out.substeps,
            "{name}: the cadence follows each kept step"
        );
    }
    at::<Composed<S, G>, f64>(name);
    at::<Composed<S, G>, f32>(name);
    let table = PairTable::part_2b();
    let pair = Pair::select(stepper, Choice::AarsethZare);
    assert_eq!(
        (pair.stepper, pair.regularisation),
        (stepper, Choice::AarsethZare)
    );
    match pair.status(&table) {
        Status::MeasuredBad { warning } => assert_eq!(pair.warning(&table), Some(warning)),
        Status::MeasuredGood | Status::NotMeasured => {
            assert_eq!(
                pair.warning(&table),
                None,
                "{name}: a warning unmeasured bad"
            )
        }
    }
    assert!(Composed::<S, G>::profile().owns_time_mapping, "{name}");
}

/// Runs [`check_pair_runs`] for each of the five steppers under `G`.
fn check_every_pair<G: Regularisation>() {
    check_pair_runs::<Euler, G>("Euler", Selection::Debug(DebugOccupant::Euler));
    check_pair_runs::<Kdk, G>("KDK", Selection::Production(Occupant::Kdk));
    check_pair_runs::<Yoshida4, G>("Yoshida-4", Selection::Production(Occupant::Yoshida4));
    check_pair_runs::<Yoshida6, G>("Yoshida-6", Selection::Production(Occupant::Yoshida6));
    check_pair_runs::<Rk4, G>("RK4", Selection::Production(Occupant::Rk4));
}

#[test]
fn regularisation_compose_az_runs() {
    assert_eq!(Az::KIND, Kind::AarsethZare);
    check_every_pair::<Az>();
    // The table's Aarseth–Zare row: RK4 measured good, the rest not measured, none with a warning.
    let table = PairTable::part_2b();
    assert_eq!(
        Pair::select(Selection::Production(Occupant::Rk4), Choice::AarsethZare).status(&table),
        Status::MeasuredGood
    );
}

negative_control!(
    regularisation_compose_az_runs,
    "an Aarseth–Zare that takes no step",
    expected = "did not run: the state did not move",
    check_every_pair::<Still>()
);

// ── The profile and the stepper slot ───────────────────────────────────────────────────────────────────────────

/// A stand-in for a time-transformed leapfrog in Mikkola–Tanikawa form, a type of the Aarseth–Zare stepper slot. Its
/// equations are not in the corpus (R-162); the stand-in steps with RK4's form, and advertises the table's row.
struct MikkolaTanikawaStandIn;

impl AzStepper for MikkolaTanikawaStandIn {
    /// Part 2a's row for Aarseth–Zare with a time-transformed leapfrog (R-162, R-168, R-500): order 2, one
    /// evaluation, symplectic, step-reversible, not run-reversible, owning its time mapping.
    fn pair_profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 2,
            force_evals: 1,
            symplectic: true,
            step_reversible: true,
            run_reversible: false,
            owns_time_mapping: true,
        }
    }

    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        Form::Rk4.step(system, q, p, h)
    }
}

/// Checks that `profile`, a stepper's pair with Aarseth–Zare, is the explicit composition's (Part 2b): not symplectic,
/// neither step- nor run-reversible, owning its time mapping.
fn check_pair_profile(name: &str, profile: CapabilityProfile) {
    assert!(
        !profile.symplectic,
        "{name} × AZ: the explicit composition is not symplectic"
    );
    assert!(
        !profile.step_reversible && !profile.run_reversible,
        "{name} × AZ: reversible"
    );
    assert!(
        profile.owns_time_mapping,
        "{name} × AZ: the wrapper's time mapping"
    );
}

#[test]
fn regularisation_compose_az_profile() {
    // Part 2a's row: AZ + RK4, order 4, four evaluations, ✗ ✗ ✗ ✓.
    assert_eq!(
        Composed::<Rk4, Az>::profile(),
        CapabilityProfile {
            order: 4,
            force_evals: 4,
            symplectic: false,
            step_reversible: false,
            run_reversible: false,
            owns_time_mapping: true,
        },
        "the AZ + RK4 profile is Part 2a's row"
    );
    for (name, profile) in [
        ("Euler", Composed::<Euler, Az>::profile()),
        ("KDK", Composed::<Kdk, Az>::profile()),
        ("Yoshida-4", Composed::<Yoshida4, Az>::profile()),
        ("Yoshida-6", Composed::<Yoshida6, Az>::profile()),
        ("RK4", Composed::<Rk4, Az>::profile()),
    ] {
        check_pair_profile(name, profile);
    }
    let evals = |p: CapabilityProfile| (p.order, p.force_evals);
    assert_eq!(evals(Composed::<Euler, Az>::profile()), (1, 1));
    assert_eq!(
        evals(Composed::<Kdk, Az>::profile()),
        (1, 3),
        "KDK: three evaluations, first order"
    );
    assert_eq!(evals(Composed::<Yoshida4, Az>::profile()), (1, 9));
    assert_eq!(evals(Composed::<Yoshida6, Az>::profile()), (1, 21));
    for (form, stepper) in [
        (Form::Euler, Euler::profile()),
        (Form::Kdk, Kdk::profile()),
        (Form::Yoshida4, Yoshida4::profile()),
        (Form::Yoshida6, Yoshida6::profile()),
        (Form::Rk4, Rk4::profile()),
    ] {
        assert_eq!(Form::of(&stepper), form, "each stepper's form is its own");
    }
    // The slot takes a Mikkola–Tanikawa leapfrog type, with the table's row: symplectic and step-reversible.
    let leapfrog = <MikkolaTanikawaStandIn as AzStepper>::pair_profile();
    assert!(leapfrog.symplectic && leapfrog.step_reversible && !leapfrog.run_reversible);
    assert!(leapfrog.owns_time_mapping);
    assert_eq!((leapfrog.order, leapfrog.force_evals), (2, 1));
    let out = march::<MikkolaTanikawaStandIn, f64, Counting, ExplicitTest>(
        &burrau(),
        0,
        1,
        AzCount::new(),
        &params(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    assert_eq!(out.boundary, 1, "the slot's type marches an interval");
}

negative_control!(
    regularisation_compose_az_profile,
    "the pair reporting its stepper's own row, KDK's, symplectic",
    expected = "the explicit composition is not symplectic",
    check_pair_profile("KDK", Kdk::profile())
);

// ── ADVANCE on the sync grid ───────────────────────────────────────────────────────────────────────────────────

/// Checks a march to physical targets on the grid `Δ_sync = 1/8`: `reach(t)` gives the time reached and the state.
/// To a boundary (`t = 3/4`) it ends at exactly that time; to a target between boundaries (`t = 0.8`, `t = 0.87`) at
/// the boundary before it, `3/4`, with the same state.
fn check_grid(reach: impl Fn(f64) -> (f64, Vec<u64>)) {
    let (t, on) = reach(0.75);
    assert_eq!(
        t.to_bits(),
        0.75f64.to_bits(),
        "to a boundary: exactly its time"
    );
    for off in [0.8, 0.87] {
        let (t, s) = reach(off);
        assert_eq!(
            t.to_bits(),
            0.75f64.to_bits(),
            "a target between boundaries must end at the boundary before it, not {t}"
        );
        assert_eq!(s, on, "between boundaries: the boundary's state, bitwise");
    }
    let (t, _) = reach(0.875);
    assert_eq!(t.to_bits(), 0.875f64.to_bits(), "the next boundary");
}

/// The march to physical time `t` from Burrau's state at `t = 0`.
fn reach_physical(t: f64) -> (f64, Vec<u64>) {
    let out = Az::advance_to::<Rk4, f64, Counting>(
        &burrau(),
        0.0,
        t,
        AzCount::new(),
        &mut Counting::default(),
    );
    assert_eq!(out.count.end, End::Marching);
    (out.t, bits(&out.state))
}

#[test]
fn owns_time_mapping_composed_az() {
    check_grid(reach_physical);
    // Through the seam: the macro-step grid at dt_macro = 1/64, t = 48/64 = 3/4 and 51/64 between boundaries.
    let seam = |index: u32| {
        let s = burrau::<f64>();
        let out = <Composed<Rk4, Az> as Advance>::advance(
            &s,
            &Geom::new(&s.m, &s.r),
            0,
            index,
            &AdvanceParams {
                dt_macro: 1.0 / 64.0,
                n_sub: 1,
            },
            &mut Counting::default(),
        );
        bits(&out.state)
    };
    let on = reach_physical(0.75).1;
    assert_eq!(seam(48), on, "the seam's ADVANCE to the boundary 3/4");
    assert_eq!(
        seam(51),
        on,
        "the seam's ADVANCE between boundaries: the boundary before"
    );
    assert_eq!(seam(56), reach_physical(0.875).1);
    // The boundary index, decided by products.
    for (t, k) in [
        (0.0, 0),
        (0.124, 0),
        (0.125, 1),
        (0.75, 6),
        (0.8, 6),
        (0.874_999, 6),
        (0.875, 7),
    ] {
        assert_eq!(
            boundary_at_or_before(t, 0.125f64),
            k,
            "boundary at or before {t}"
        );
    }
    assert_eq!(
        boundary_at_or_before(0.3f64, 0.1),
        2,
        "0.3 is below 3 × 0.1 in binary"
    );
    assert_eq!(boundary_at_or_before(0.30000000000000004f64, 0.1), 3);
}

negative_control!(
    owns_time_mapping_composed_az,
    "a march that lands on the off-grid target",
    expected = "a target between boundaries must end at the boundary before it",
    check_grid(|t| (t, reach_physical(0.75).1))
);

// ── The fixed τ-schedule's bound ───────────────────────────────────────────────────────────────────────────────

/// A tight budget, so that close encounters spend it: `N_min = 100`, 600 steps per unit time, `Δ_sync = 1/8`,
/// `η = 0.02`.
fn tight() -> AzParams<f64> {
    AzParams {
        eta: 0.02,
        n_min: 100,
        step_budget_rate: 600.0,
        ..params::<f64>()
    }
}

/// What a march over `n` boundaries saw: the largest count in one interval, and whether the copy ended undetermined.
struct Schedule {
    largest: u32,
    exhausted: bool,
}

/// A march of one interval: from the state at boundary `k` with the trajectory's counts, what it reached and the step
/// count it started from.
type Run<'a> = &'a dyn Fn(&State<f64>, u32, AzCount<f64>) -> (AzAdvanced<f64>, u32);

/// Marches `s` boundary by boundary to boundary `n`, `run` taking each call, and checks at every boundary `t_k` that
/// the true running total of steps is at most `max(N_min, rate·t_k)` unless the copy has ended undetermined, and that
/// an ended copy takes no further step.
fn check_schedule(s: &State<f64>, n: u32, params: &AzParams<f64>, run: Run<'_>) -> Schedule {
    let mut state = *s;
    let mut carry = AzCount::new();
    let mut total = 0u32;
    let mut largest = 0;
    for k in 0..n {
        let (out, from) = run(&state, k, carry);
        let taken = out.count.steps - from;
        total += taken;
        largest = largest.max(taken);
        if out.count.end == End::Exhausted {
            let (again, _) = run(&out.state, out.boundary, out.count);
            assert_eq!(
                again.count.steps, out.count.steps,
                "an exhausted copy ran on"
            );
            return Schedule {
                largest,
                exhausted: true,
            };
        }
        assert_eq!(
            out.boundary,
            k + 1,
            "a marching copy lands on the next boundary"
        );
        let t_k = f64::from(out.boundary) * params.delta_sync;
        let bound = f64::from(params.n_min).max(params.step_budget_rate * t_k);
        assert!(
            f64::from(total) <= bound,
            "the running total {total} at boundary t = {t_k} exceeds max(N_min, rate·t) = {bound} and the copy ran on"
        );
        state = out.state;
        carry = out.count;
    }
    Schedule {
        largest,
        exhausted: false,
    }
}

/// The occupant's march of one interval, the count carried.
fn carried(s: &State<f64>, k: u32, c: AzCount<f64>) -> (AzAdvanced<f64>, u32) {
    let out = march::<Rk4, f64, Counting, ExplicitTest>(
        s,
        k,
        k + 1,
        c,
        &tight(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    (out, c.steps)
}

/// The control's march: the count reset at each boundary, a cap per interval read as the budget.
#[cfg(feature = "controls")]
fn reset(s: &State<f64>, k: u32, _c: AzCount<f64>) -> (AzAdvanced<f64>, u32) {
    let out = march::<Rk4, f64, Counting, ExplicitTest>(
        s,
        k,
        k + 1,
        AzCount::new(),
        &tight(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    (out, 0)
}

/// A fuzzed IC: three masses in `[0.2, 1]` summed to 1, positions in a box of side 2, velocities in `[−0.5, 0.5]`.
fn fuzzed(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<f64> {
    let total: f64 = m.iter().sum();
    state(m.map(|x| x / total), r, v)
}

/// A copy whose total runs past the boundary bound under a counter reset at each boundary: Burrau's triangle in the
/// gauge, its first close passage spending the tight budget.
fn spender() -> State<f64> {
    burrau()
}

#[test]
fn tau_schedule_bound() {
    let p = tight();
    let n = 24;
    let largest = std::cell::Cell::new(0u32);
    let exhausted = std::cell::Cell::new(0u32);
    let spent = check_schedule(&spender(), n, &p, &carried);
    largest.set(spent.largest);
    exhausted.set(u32::from(spent.exhausted));
    let unit = -1.0f64..1.0;
    let ic = (
        [0.2f64..1.0, 0.2..1.0, 0.2..1.0],
        [
            [unit.clone(), unit.clone()],
            [unit.clone(), unit.clone()],
            [unit.clone(), unit.clone()],
        ],
        [
            [-0.5f64..0.5, -0.5..0.5],
            [-0.5..0.5, -0.5..0.5],
            [-0.5..0.5, -0.5..0.5],
        ],
    );
    validation::prop::run(&ic, |(m, r, v)| {
        let s = check_schedule(&fuzzed(m, r, v), n, &p, &carried);
        largest.set(largest.get().max(s.largest));
        exhausted.set(exhausted.get() + u32::from(s.exhausted));
        Ok(())
    });
    eprintln!(
        "tau_schedule_bound: the largest count in one interval {} (recorded, not asserted; R-953); copies ended \
         undetermined: {}; N_min = {}, rate = {}/unit time, Δ_sync = {}; {}",
        largest.get(),
        exhausted.get(),
        p.n_min,
        p.step_budget_rate,
        p.delta_sync,
        validation::prop::cases_status()
    );
    assert!(
        spent.exhausted,
        "the spender's close passage spends the tight budget"
    );
}

negative_control!(
    tau_schedule_bound,
    "a step counter reset at each boundary, a cap per interval read as the budget",
    expected = "exceeds max(N_min, rate·t)",
    {
        check_schedule(&spender(), 24, &tight(), &reset);
    }
);

// ── The transcription's pieces ─────────────────────────────────────────────────────────────────────────────────

/// The reference body by the longest pair, the first maximum in the order (0,1), (0,2), (1,2) winning a tie, read
/// from `at`.
fn check_reference(at: impl Fn([[f64; 2]; 3]) -> u32) {
    // The longest side names the body opposite it.
    assert_eq!(
        at([[0.0, 0.0], [1.0, 0.0], [0.0, 3.0]]),
        0,
        "longest (1,2): a = 0"
    );
    assert_eq!(
        at([[0.0, 0.0], [1.0, 0.1], [2.0, 0.0]]),
        1,
        "longest (0,2): a = 1"
    );
    assert_eq!(
        at([[0.0, 0.0], [3.0, 0.0], [1.5, 0.1]]),
        2,
        "longest (0,1): a = 2"
    );
    // Ties, exact in d²: the first maximum in the order (0,1), (0,2), (1,2) wins.
    assert_eq!(
        at([[0.0, 0.0], [0.5, 5.0], [1.0, 0.0]]),
        2,
        "a tie of (0,1) and (1,2) gives a = 2"
    );
    assert_eq!(
        at([[0.0, 0.0], [5.0, 0.5], [5.0, -0.5]]),
        2,
        "a tie of (0,1) and (0,2) gives a = 2"
    );
    assert_eq!(
        at([[-0.5, 0.0], [0.5, 0.0], [0.0, 5.0]]),
        1,
        "a tie of (0,2) and (1,2) gives a = 1"
    );
    assert_eq!(at([[0.0; 2]; 3]), 2, "all coincident: (0,1), the first");
}

#[test]
fn az_reference_body_tie_break() {
    check_reference(|r| reference_body(&r));
}

/// A selector whose last maximum wins a tie.
#[cfg(feature = "controls")]
fn last_maximum(r: [[f64; 2]; 3]) -> u32 {
    let d2 = |i: usize, j: usize| (r[i][0] - r[j][0]).powi(2) + (r[i][1] - r[j][1]).powi(2);
    let d = [d2(0, 1), d2(0, 2), d2(1, 2)];
    let mut k = 0;
    for (i, x) in d.iter().enumerate() {
        if *x >= d[k] {
            k = i;
        }
    }
    [2, 1, 0][k]
}

negative_control!(
    az_reference_body_tie_break,
    "the last maximum winning a tie",
    expected = "a tie of (0,1) and (1,2) gives a = 2",
    check_reference(last_maximum)
);

/// The conditioned inverse `inverse` round-trips within round-off at every angle, the unstable form loses up to `1e-6`
/// towards `−x`; the branches and guards at their edges.
fn check_lc_inverse(inverse: fn([f64; 2], f64) -> [f64; 2]) {
    let tiny = 1e-300f64;
    for angle in [
        0.0f64, 30.0, 89.0, 90.0, 91.0, 150.0, 179.9, 180.0, -179.9, -90.0, -30.0,
    ] {
        let th = angle.to_radians();
        let r = [2.0 * th.cos(), 2.0 * th.sin()];
        for (name, u) in [
            ("conditioned", inverse(r, tiny)),
            ("unstable", lc_inverse_unstable(r, tiny)),
        ] {
            let back = lc(u);
            let err = ((back[0] - r[0]).powi(2) + (back[1] - r[1]).powi(2)).sqrt() / 2.0;
            let bound = if name == "unstable" && angle.abs() > 170.0 {
                1e-6
            } else {
                1e-15
            };
            assert!(err <= bound, "{name} at {angle}°: {err:e}");
            assert!(u[0] >= 0.0, "{name} at {angle}°: u₀ < 0");
        }
    }
    // The branches at their edges: x = 0 takes the x ≥ 0 branch; y = 0 on x < 0 takes +b; the origin is fixed.
    assert_eq!(lc_inverse([0.0, 2.0], tiny), [1.0, 1.0]);
    assert_eq!(lc_inverse([-4.0, 0.0], tiny), [0.0, 2.0]);
    assert_eq!(lc_inverse([4.0, 0.0], tiny), [2.0, 0.0]);
    assert_eq!(lc_inverse([0.0, 0.0], tiny), [0.0, 0.0]);
    assert_eq!(lc_inverse_unstable([-4.0, 0.0], tiny), [0.0, 2.0]);
    assert_eq!(
        lc_inverse([-4.0, -0.0], tiny)[1],
        2.0,
        "y = −0 is not below 0"
    );
    assert_eq!(
        lc_inverse([-4.0, -1e-30], tiny)[1].signum(),
        -1.0,
        "y < 0: u₁ = −b"
    );
    // The guards at TINY: a component at or below it takes the other root.
    let x = 1e-30f64;
    let m = (x * x + x * x).sqrt();
    let (a, b) = (((m + x) / 2.0).sqrt(), ((m - x) / 2.0).sqrt());
    assert_eq!(lc_inverse([x, x], 1.0), [a, b], "a ≤ TINY: u₁ = b");
    assert_eq!(lc_inverse([-x, x], 1.0), [b, a], "|u₁| ≤ TINY: u₀ = a");
    assert_eq!(
        lc_inverse_unstable([x, x], 1.0),
        [a, b],
        "u₀ ≤ TINY: u₁ = b"
    );
}

#[test]
fn az_lc_inverse_branches() {
    check_lc_inverse(lc_inverse);
}

negative_control!(
    az_lc_inverse_branches,
    "the unstable form in the conditioned one's place",
    expected = "conditioned at",
    check_lc_inverse(lc_inverse_unstable)
);

/// The registration's image, `image`, is the state shifted to its centre of mass and momentum, and the clock starts
/// at 0.
fn check_round_trip(image: impl Fn(&AzSystem<f64>, &AzQ<f64>, &AzP<f64>) -> State<f64>) {
    let p = params::<f64>();
    for s in [
        burrau::<f64>(),
        fuzzed(
            [0.3, 0.5, 0.9],
            [[0.4, -0.2], [-0.7, 0.1], [0.2, 0.9]],
            [[0.1, 0.2], [-0.3, 0.05], [0.0, -0.1]],
        ),
    ] {
        let (system, q, pm) = register(&s, p.tiny, p.r3_floor);
        let back = image(&system, &q, &pm);
        // The image is the state shifted to its centre of mass and momentum.
        let mt: f64 = s.m.iter().sum();
        let rc = [0, 1].map(|j| (0..3).map(|i| s.m[i] * s.r[i][j]).sum::<f64>() / mt);
        let pc = [0, 1].map(|j| (0..3).map(|i| s.p[i][j]).sum::<f64>());
        for i in 0..3 {
            for j in 0..2 {
                assert!(
                    (back.r[i][j] - (s.r[i][j] - rc[j])).abs() < 1e-14,
                    "the image's r[{i}][{j}]"
                );
                assert!(
                    (back.p[i][j] - (s.p[i][j] - s.m[i] / mt * pc[j])).abs() < 1e-14,
                    "the image's p[{i}][{j}]"
                );
            }
        }
        assert_eq!(q.t, 0.0, "the interval clock starts at 0");
    }
}

#[test]
fn az_round_trip() {
    check_round_trip(|s, q, p| s.image(q, p));
}

negative_control!(
    az_round_trip,
    "u₁ turned a quarter, which maps R₁ to −R₁",
    expected = "the image's",
    check_round_trip(|s, q, p| {
        let mut q = *q;
        q.u1 = [-q.u1[1], q.u1[0]];
        s.image(&q, p)
    })
);

/// Γ at `(q, p)` (Part 2b), which the march holds at zero.
fn gamma(sys: &AzSystem<f64>, q: &AzQ<f64>, p: &AzP<f64>) -> f64 {
    let (a, b) = AzSystem::ab(q);
    let dot = |x: [f64; 2], y: [f64; 2]| x[0] * y[0] + x[1] * y[1];
    let lm = |u: [f64; 2], w: [f64; 2]| [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]];
    let [ma, mb, mc] = sys.mabc;
    let r1 = lc(q.u1);
    let r2 = lc(q.u2);
    let r3 = ((r2[0] - r1[0]).powi(2) + (r2[1] - r1[1]).powi(2)).sqrt();
    b * dot(p.p1, p.p1) / (8.0 * sys.mu[0])
        + a * dot(p.p2, p.p2) / (8.0 * sys.mu[1])
        + dot(lm(q.u1, p.p1), lm(q.u2, p.p2)) / (4.0 * ma)
        - ma * mb * b
        - ma * mc * a
        - a * b * mb * mc / r3
        - sys.e * a * b
}

/// Γ is zero on registration, the rates of a flow registered with its energy moved by `de` are Γ's derivatives, and
/// one interval keeps Γ near zero and the energy held.
fn check_energy(de: f64) {
    let p = params::<f64>();
    let s = burrau::<f64>();
    let (sys, q, pm) = register(&s, p.tiny, p.r3_floor);
    assert!(gamma(&sys, &q, &pm).abs() < 1e-14, "Γ = 0 on registration");
    // The rates are Γ's derivatives, by central differences.
    let mut flow = sys;
    flow.e += de;
    let (dq, dp) = flow.rates(&q, &pm);
    let h = 1e-6;
    let bump_p = |i: usize, s: f64| {
        let mut x = pm;
        if i < 2 {
            x.p1[i] += s
        } else {
            x.p2[i - 2] += s
        }
        x
    };
    let bump_q = |i: usize, s: f64| {
        let mut x = q;
        if i < 2 {
            x.u1[i] += s
        } else {
            x.u2[i - 2] += s
        }
        x
    };
    let du = [dq.u1[0], dq.u1[1], dq.u2[0], dq.u2[1]];
    let dpv = [dp.p1[0], dp.p1[1], dp.p2[0], dp.p2[1]];
    for i in 0..4 {
        let dg_dp = (gamma(&sys, &q, &bump_p(i, h)) - gamma(&sys, &q, &bump_p(i, -h))) / (2.0 * h);
        let dg_du =
            (gamma(&sys, &bump_q(i, h), &pm) - gamma(&sys, &bump_q(i, -h), &pm)) / (2.0 * h);
        assert!(
            (du[i] - dg_dp).abs() < 1e-6,
            "u̇[{i}] = ∂Γ/∂p: {} against {}",
            du[i],
            dg_dp
        );
        assert!(
            (dpv[i] + dg_du).abs() < 1e-6,
            "ṗ[{i}] = −∂Γ/∂u: {} against {}",
            dpv[i],
            -dg_du
        );
    }
    let (a, b) = AzSystem::ab(&q);
    assert_eq!(dq.t, a * b, "dt/dτ = A·B");
    // One interval keeps Γ near zero.
    let out = march::<Rk4, f64, Counting, ExplicitTest>(
        &s,
        0,
        1,
        AzCount::new(),
        &p,
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    let (sys1, q1, p1) = register(&out.state, p.tiny, p.r3_floor);
    assert!(gamma(&sys1, &q1, &p1).abs() < 1e-14);
    assert!(
        (sys1.e - sys.e).abs() < 1e-9 * sys.e.abs(),
        "the energy across the interval: {} → {}",
        sys.e,
        sys1.e
    );
}

#[test]
fn az_energy_held() {
    check_energy(0.0);
}

negative_control!(
    az_energy_held,
    "the flow's energy moved by 0.1 from the state's",
    expected = "ṗ[",
    check_energy(0.1)
);

// ── The step's signals and the march's counts ──────────────────────────────────────────────────────────────────

/// A step-size rule, [`step_size`]'s signature.
type Size = fn(&AzSystem<f64>, &AzQ<f64>, &AzP<f64>, f64, &AzParams<f64>) -> StepSize<f64>;

/// The step's signals at their edges (Part 2b, steps 3 and 4; Part 6): at the interval's entry `c₀` is the entry size,
/// not a cap hit, and below the entry size it is; `A` or `B` at `TINY` is not floored and just below it is; a tiny
/// predictive `f` sets the size and an `f` of 0 turns the limit off; the clamp takes the remainder `(Δ_sync − t)/(A·B)`,
/// and none past the boundary.
fn check_signals(size: Size) {
    let mut p = params::<f64>();
    p.clamp = false;
    let s = burrau::<f64>();
    let (sys, q, pm) = register(&s, p.tiny, p.r3_floor);
    let entry = entry_size(&q, &p);
    let at_entry = size(&sys, &q, &pm, entry, &p);
    assert!(
        !at_entry.cap_hit,
        "at the entry c₀ is the entry size, not above it"
    );
    assert!(!at_entry.floored && !at_entry.by_limit, "{at_entry:?}");
    let capped = size(&sys, &q, &pm, entry / 2.0, &p);
    assert!(
        capped.cap_hit && capped.h == entry / 2.0,
        "below the entry size: {capped:?}"
    );
    let (a, b) = AzSystem::ab(&q);
    let mut t = p;
    t.tiny = a.min(b);
    assert!(
        !size(&sys, &q, &pm, entry, &t).floored,
        "A or B at TINY is not below it"
    );
    t.tiny = a.min(b).next_up();
    assert!(
        size(&sys, &q, &pm, entry, &t).floored,
        "just above it, floored"
    );
    let mut f = p;
    f.predictive_f = 1e-9;
    let limited = size(&sys, &q, &pm, entry, &f);
    assert!(
        limited.by_limit && limited.h < entry,
        "a tiny f sets the size: {limited:?}"
    );
    f.predictive_f = 0.0;
    assert!(
        !size(&sys, &q, &pm, entry, &f).by_limit,
        "f = 0: the limit is off"
    );
    let mut c = p;
    c.clamp = true;
    let mut near = q;
    near.t = c.delta_sync - 1e-9;
    let clamped = size(&sys, &near, &pm, entry, &c);
    assert_eq!(
        clamped.h,
        (c.delta_sync - near.t) / (a * b),
        "the clamp takes the remainder"
    );
    near.t = c.delta_sync + 1e-9;
    assert_eq!(
        size(&sys, &near, &pm, entry, &c).h,
        0.0,
        "past the boundary, no step"
    );
}

#[test]
fn az_step_signals() {
    check_signals(step_size);
}

negative_control!(
    az_step_signals,
    "a rule that counts every step a cap hit",
    expected = "at the entry c₀ is the entry size, not above it",
    check_signals(|s, q, p, e, a| StepSize {
        cap_hit: true,
        ..step_size(s, q, p, e, a)
    })
);

/// A march over `[k, k + n)` from `s` carrying `c`: its state, its counts.
type Over<'a> = &'a dyn Fn(&State<f64>, u32, u32, AzCount<f64>) -> AzAdvanced<f64>;

/// Burrau over 80 sync intervals (`t = 10`) in one march, against the same marched one interval at a time: the same
/// state, bit for bit; the same steps; one registration per interval; the switches of reference body counted between
/// registrations, not at the first; the largest interval's steps; `dt_max` positive and at most `Δ_sync`. Without the
/// clamp or the predictive limit and with a large `η`, a step past `2·Δ_sync` is an overshoot.
fn check_counts(over: Over<'_>) {
    let p = params::<f64>();
    let s = burrau::<f64>();
    let n = 80;
    let whole = over(&s, 0, n, AzCount::new());
    let (mut st, mut c) = (s, AzCount::new());
    let (mut switches, mut largest, mut last) = (0, 0, 3);
    for k in 0..n {
        let before = c.steps;
        let out = over(&st, k, k + 1, c);
        largest = largest.max(out.count.steps - before);
        if last != 3 && out.count.reference != last {
            switches += 1;
        }
        last = out.count.reference;
        (st, c) = (out.state, out.count);
    }
    assert_eq!(
        bits(&whole.state),
        bits(&st),
        "one march or one interval at a time, the same state"
    );
    assert_eq!(whole.count.steps, c.steps);
    assert_eq!(
        whole.count.re_registrations, n,
        "one registration per interval"
    );
    assert!(switches > 0, "the fixture switches reference body");
    assert_eq!(
        whole.count.switches, switches,
        "the switches between registrations"
    );
    assert_eq!(
        whole.count.largest_interval, largest,
        "the largest interval"
    );
    match whole.count.dt_max.report() {
        kernel::regularisation::dt_max::DtMaxReport::Largest { dt_max, skipped } => {
            assert!(
                dt_max > 0.0 && dt_max <= p.delta_sync && skipped == 0,
                "dt_max {dt_max:e}"
            );
        }
        other => panic!("no usable step: {other:?}"),
    }
    let mut loose = p;
    loose.clamp = false;
    loose.secant_tries = 0;
    loose.eta = 50.0;
    loose.predictive_f = 0.0;
    let wild = march::<Rk4, f64, Counting, ExplicitTest>(
        &s,
        0,
        1,
        AzCount::new(),
        &loose,
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    assert!(
        wild.count.overshoots > 0,
        "a step past 2·Δ_sync: {:?}",
        wild.count
    );
}

/// The occupant's march over `[k, k + n)`.
fn marched(s: &State<f64>, k: u32, n: u32, c: AzCount<f64>) -> AzAdvanced<f64> {
    march::<Rk4, f64, Counting, ExplicitTest>(
        s,
        k,
        n,
        c,
        &params(),
        &mut Counting::default(),
        &mut ExplicitTest,
    )
}

#[test]
fn az_counts() {
    check_counts(&marched);
}

negative_control!(
    az_counts,
    "a march that counts the first registration a switch",
    expected = "the switches between registrations",
    check_counts(&|s, k, n, c| {
        let mut out = marched(s, k, n, c);
        if k == 0 {
            out.count.switches += 1;
        }
        out
    })
);
