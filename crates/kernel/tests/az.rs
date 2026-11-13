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
//! - `az_landing_secant`: on a scripted clock, the landing's secant and bisection landing the clock on the boundary
//!   exactly, a retake that leaves the clock where it was, the budget's physical time, a copy spent as it lands, and
//!   the overshoot's edge, no kept clock past the boundary; its control turns the retakes off (R-998).
//! - `az_boundary_corrected`, `az_seam_index`: the boundary's corrections of the quotient both ways and before `0`,
//!   and the seam's `ADVANCE` from a later macro index; their controls the uncorrected quotient and a seam from 0.
//! - `az_form_orders`: each stepper's form at its own order on the harmonic oscillator (KDK second order there, being
//!   separable); its control takes Yoshida-4's form for Yoshida-6.
//! - `az_stepper_forms`: each stepper type names its own form (`Stepper::FORM`, fixed at compile time); its control
//!   has Yoshida-6 name Yoshida-4's.
//! - `az_whole_nan`, `az_whole_negative`, `az_whole_above`: `Count::whole` clamps in the float type before the cast
//!   (gpu_determinism_note § "The discipline", rule 3): a NaN on its bits to `0`, a negative to `0`, at or above `2³²`
//!   to `u32::MAX`; their controls a NaN sent to the top, a negative by its magnitude, and a clamp with no select.
//! - `az_landing_no_tolerance`: no landing tolerance, with the clamp on or off: a clock short of the boundary never
//!   lands, one past it is never kept; its control keeps a step past the boundary (R-998).
//! - `az_landing_state_with_clock`: under RK4, an interval whose clamped step overshoots stores the state whose own
//!   clock is the boundary, bitwise, its time the boundary's; its control is the clock-only correction (R-998;
//!   REQ-INT-119).
//! - `az_r3_floor_tiny`: the equations of motion read `|R₃|` floored at `TINY`, `1e-37` at f32 and `1e-300` at f64; its
//!   control a floor of 0 at f32 (R-999; REQ-INT-120).
//! - `az_seam_non_finite_fails`, `az_seam_completes`, `az_seam_exhausted_stops`, `az_seam_done_short`: through the
//!   wrapper (`integrate_with`), the seam's end (Part 2a, Cost 2; § "The sync grid is a fixed quantum of physical time
//!   (R-565)"): a march ended non-finite fails the run, `sim_failed` at the macro-step it ended in; every macro-step
//!   that reached its boundary is completed, `t_end_step` and the Welford samples counting it; a spent budget stops
//!   the copy, `running` as it stood; a march stopped on `done` is not completed. Their controls drop the end (the
//!   stepper's rule) or count a short call completed.
//! - `az_done_on_landing`: a seam call spanning several sync boundaries with a terminal on a landing step ends there,
//!   latched; its control leaves the terminal's `done` unread.
//! - `az_done_on_target_landing`: a terminal on the landing step of the call's own target boundary, at
//!   `Δ_sync = dt_macro` and at the last of four boundaries, registers the landing at its exact time, reaches the
//!   target, and through the wrapper completes the macro-step with its Welford sample; its control reads it short.
//! - `regularisation_owning_default_end`, `regularisation_none_keeps_count`: the default end of an occupant owning
//!   its time mapping, and `none` bitwise the stepper's own run; their controls swap the two rules.

use std::cell::Cell;

use kernel::decode::Ic;
#[cfg(feature = "controls")]
use kernel::driver::advance::AdvanceReal;
use kernel::driver::advance::{Advance, AdvanceParams, Advanced, Cadence, Count, Reached};
use kernel::driver::substep::NsubThresholds;
use kernel::driver::welford::{self, Welford};
use kernel::driver::wrapper::{integrate, integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{
    CapabilityProfile, DebugOccupant, Geom, Occupant, Selection, State, Stepper,
};
use kernel::payload::{
    sd_detail, sd_dmin_pair, sd_state, tm_t_end_step, SimStateBaseOf, SD_DMIN_PAIR_SENTINEL,
    STATE_BOUNDED, STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{
    boundary_at_or_before, entry_size, lc, lc_inverse, lc_inverse_unstable, march, over_budget,
    reference_body, register, step_size, AarsethZare, AzAdvanced, AzConfig, AzCount, AzP, AzParams,
    AzQ, AzReal, AzStepper, AzSystem, End, ExplicitTest, StepSize, Watch,
};
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::pairs::{Choice, Pair, PairTable, Status};
use kernel::regularisation::{Composed, Kind, NoRegularisation, Regularisation};
use kernel::shape::shape;
use spirv_std::glam::UVec4;
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
/// f32, `1e-300` at f64, R-953), which also floors `|R₃|` (R-999), R-160's `f = 0.02`, a budget that does not bind, the
/// clamp on, the secant's 4 retakes and the bisection's 64 halvings (Part 2b, step 6; R-998). Test inputs, not the
/// calibrations.
fn params<R: AzReal>() -> AzParams<R> {
    AzParams {
        eta: v(0.01),
        delta_sync: v(0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: v(0.02),
        step_budget_rate: v(1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
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
    let (system, q, pm) = register(&degenerate(), p.tiny);
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

/// Checks that each stepper type names its own form (`Stepper::FORM`, fixed at compile time): `named` pairs each
/// stepper's expected form with the one it names.
fn check_forms(named: [(Form, Form); 5]) {
    for (want, got) in named {
        assert_eq!(got, want, "each stepper's form is its own");
    }
}

#[test]
fn az_stepper_forms() {
    check_forms([
        (Form::Euler, Euler::FORM),
        (Form::Kdk, Kdk::FORM),
        (Form::Yoshida4, Yoshida4::FORM),
        (Form::Yoshida6, Yoshida6::FORM),
        (Form::Rk4, Rk4::FORM),
    ]);
}

negative_control!(
    az_stepper_forms,
    "Yoshida-6 naming Yoshida-4's form",
    expected = "each stepper's form is its own",
    check_forms([
        (Form::Euler, Euler::FORM),
        (Form::Kdk, Kdk::FORM),
        (Form::Yoshida4, Yoshida4::FORM),
        (Form::Yoshida6, Yoshida4::FORM),
        (Form::Rk4, Rk4::FORM),
    ])
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
    // At TINY exactly the guard takes the other root: on y < 0, b where y/(2a) is −b; and, on x < 0 with y = −0, a = +0
    // where y/(2u₁) is −0.
    assert_eq!(inverse([0.0, -2.0], 1.0), [1.0, 1.0], "a = TINY: u₁ = b");
    assert_eq!(
        lc_inverse_unstable([0.0, -2.0], 1.0),
        [1.0, 1.0],
        "u₀ = TINY: u₁ = b"
    );
    let u = inverse([-4.0, -0.0], 2.0);
    assert_eq!(
        (u[0].to_bits(), u[1]),
        (0.0f64.to_bits(), 2.0),
        "|u₁| = TINY: u₀ = a = +0"
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
        unnormalised(),
    ] {
        let (system, q, pm) = register(&s, p.tiny);
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
    let (sys, q, pm) = register(&s, p.tiny);
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
    let (sys1, q1, p1) = register(&out.state, p.tiny);
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
    let (sys, q, pm) = register(&s, p.tiny);
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
    // A's own floor test: with u₁ and u₂ swapped, A and B swap.
    let swapped = AzQ {
        u1: q.u2,
        u2: q.u1,
        ..q
    };
    let (sa, sb) = AzSystem::ab(&swapped);
    t.tiny = sa.min(sb);
    assert!(
        !size(&sys, &swapped, &pm, entry, &t).floored,
        "swapped: at TINY, not below it"
    );
    t.tiny = sa.min(sb).next_up();
    assert!(
        size(&sys, &swapped, &pm, entry, &t).floored,
        "swapped: just above it, floored"
    );
    // The least of the usable constraints, the predictive limit off (f = 0): an entry size not usable (0, NaN,
    // negative) is skipped for c₀; one above c₀ loses to it.
    let mut off = p;
    off.predictive_f = 0.0;
    let c0 = p.eta * p.delta_sync / (a * b);
    for bad in [0.0, f64::NAN, -1.0, 2.0 * c0] {
        let sized = size(&sys, &q, &pm, bad, &off);
        assert_eq!(sized.h, c0, "entry {bad}: c₀ is the least usable");
        assert!(!sized.by_limit);
    }
    // The predictive limit's value, f·d_min/(P_max·A·B), and a tie with the entry size, which it does not set.
    let mut lim = p;
    lim.predictive_f = 1e-9;
    let alone = size(&sys, &q, &pm, 1e300, &lim);
    let [r1, r2, p1, p2] = sys.phys(&q, &pm);
    let n = |v: [f64; 2]| (v[0] * v[0] + v[1] * v[1]).sqrt();
    let d = |x: [f64; 2], y: [f64; 2]| [x[0] - y[0], x[1] - y[1]];
    let d_min = n(r1).min(n(r2)).min(n(d(r2, r1)));
    let p_max = n(p1).max(n(p2)).max(n(d(p2, p1)));
    let want = 1e-9 * d_min / (p_max * a * b);
    assert!(
        alone.by_limit && (alone.h - want).abs() <= 1e-12 * want,
        "c₂ {} against {want}",
        alone.h
    );
    let tie = size(&sys, &q, &pm, alone.h, &lim);
    assert!(
        tie.h == alone.h && !tie.by_limit,
        "a tie with the entry size: {tie:?}"
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
    assert!(
        whole.count.cap_hits > 0 && whole.count.predictive_bound > 0,
        "the cap hits and the predictive limit's steps counted: {:?}",
        whole.count
    );
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
    loose.bisect_tries = 0;
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

/// The latest boundary at or before `t` at f32 as at f64 (the width's own whole count, `Count::whole`): `boundary` the
/// rule under test.
fn check_boundary_f32(boundary: fn(f32, f32) -> u32) {
    for (t, k) in [(0.8f32, 6), (0.75, 6), (2.0, 16), (0.1, 0), (10.0, 80)] {
        assert_eq!(
            boundary(t, 0.125),
            k,
            "the boundary at or before {t} on 1/8, at f32"
        );
    }
    assert_eq!(
        boundary(1e9, 0.125),
        u32::MAX,
        "beyond u32's range, its last"
    );
}

#[test]
fn az_boundary_f32() {
    check_boundary_f32(boundary_at_or_before::<f32>);
}

negative_control!(
    az_boundary_f32,
    "a rule one boundary short",
    expected = "the boundary at or before",
    check_boundary_f32(|t, d| boundary_at_or_before::<f32>(t, d).saturating_sub(1))
);

// ── The whole count, clamped before the cast (gpu_determinism_note § "The discipline", rule 3) ─────────────────

/// `Count::whole` at f32 and at f64, the rules under test.
struct Whole {
    single: fn(f32) -> u32,
    double: fn(f64) -> u32,
}

/// The kernel's `Count::whole`.
const WHOLE: Whole = Whole {
    single: Count::whole,
    double: Count::whole,
};

/// A NaN, of either sign and any payload, is sent to `0` (tested on its bits before the cast).
fn check_whole_nan(w: &Whole) {
    for bits in [0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFFFF_FFFF] {
        assert_eq!(
            (w.single)(f32::from_bits(bits)),
            0,
            "a NaN at f32 ({bits:#x}) is 0"
        );
    }
    for bits in [
        0x7FF8_0000_0000_0000u64,
        0xFFF8_0000_0000_0000,
        0x7FF0_0000_0000_0001,
        0xFFFF_FFFF_FFFF_FFFF,
    ] {
        assert_eq!(
            (w.double)(f64::from_bits(bits)),
            0,
            "a NaN at f64 ({bits:#x}) is 0"
        );
    }
}

/// A value below `0` is `0`, `−∞` and `−0` among them; one at or just above `0` truncates.
fn check_whole_negative(w: &Whole) {
    for x in [-1.5f32, -0.5, -0.0, -1e30, f32::NEG_INFINITY, f32::MIN] {
        assert_eq!((w.single)(x), 0, "a negative at f32 ({x}) is 0");
    }
    for x in [-1.5f64, -0.5, -0.0, -1e300, f64::NEG_INFINITY, f64::MIN] {
        assert_eq!((w.double)(x), 0, "a negative at f64 ({x}) is 0");
    }
    assert_eq!((w.single)(0.0), 0);
    assert_eq!((w.single)(2.75), 2, "in range, truncated, at f32");
    assert_eq!((w.double)(0.0), 0);
    assert_eq!((w.double)(2.75), 2, "in range, truncated, at f64");
}

/// A value at or above `2³²` is `u32::MAX`; the greatest float below it truncates to itself (`2³² − 2⁸` at f32,
/// `u32::MAX` at f64, and `u32::MAX + ½` truncates to `u32::MAX`).
fn check_whole_above(w: &Whole) {
    for x in [4_294_967_296.0f32, 1e30, f32::MAX, f32::INFINITY] {
        assert_eq!(
            (w.single)(x),
            u32::MAX,
            "above range at f32 ({x}) is u32::MAX"
        );
    }
    assert_eq!(
        (w.single)(4_294_967_040.0),
        4_294_967_040,
        "the greatest f32 below 2^32 is itself"
    );
    for x in [
        4_294_967_296.0f64,
        4_294_967_295.5,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        assert_eq!(
            (w.double)(x),
            u32::MAX,
            "above range at f64 ({x}) is u32::MAX"
        );
    }
    assert_eq!((w.double)(4_294_967_295.0), u32::MAX);
    assert_eq!(
        (w.double)(4_294_967_294.5),
        u32::MAX - 1,
        "just below, truncated, at f64"
    );
}

#[test]
fn az_whole_nan() {
    check_whole_nan(&WHOLE);
}

negative_control!(
    az_whole_nan,
    "a NaN sent to the top of the range",
    expected = "a NaN at f32",
    check_whole_nan(&Whole {
        single: |x| if x.is_nan() { u32::MAX } else { x.whole() },
        double: |x| if x.is_nan() { u32::MAX } else { x.whole() },
    })
);

#[test]
fn az_whole_negative() {
    check_whole_negative(&WHOLE);
}

negative_control!(
    az_whole_negative,
    "a negative taken by its magnitude",
    expected = "a negative at f32",
    check_whole_negative(&Whole {
        single: |x| x.abs().whole(),
        double: |x| x.abs().whole(),
    })
);

#[test]
fn az_whole_above() {
    check_whole_above(&WHOLE);
}

negative_control!(
    az_whole_above,
    "a value above range clamped to the greatest float below 2^32 and cast, with no select of u32::MAX",
    expected = "above range at f32",
    check_whole_above(&Whole {
        single: |x| x.min(4_294_967_040.0).whole(),
        double: Count::whole,
    })
);

/// Masses that do not sum to one, with momenta, at f64.
fn unnormalised() -> State<f64> {
    state(
        [0.3, 0.5, 0.9],
        [[0.4, -0.2], [-0.7, 0.1], [0.2, 0.9]],
        [[0.1, 0.2], [-0.3, 0.05], [0.0, -0.1]],
    )
}

/// The Cartesian energy of `s` in its centre-of-momentum frame, computed apart from the occupant.
fn com_energy(s: &State<f64>) -> f64 {
    let mt: f64 = s.m.iter().sum();
    let pc = [0, 1].map(|j| s.p.iter().map(|p| p[j]).sum::<f64>());
    let kinetic: f64 = (0..3)
        .map(|i| {
            let q = [0, 1].map(|j| s.p[i][j] - s.m[i] / mt * pc[j]);
            (q[0] * q[0] + q[1] * q[1]) / (2.0 * s.m[i])
        })
        .sum();
    let d = |i: usize, j: usize| {
        ((s.r[i][0] - s.r[j][0]).powi(2) + (s.r[i][1] - s.r[j][1]).powi(2)).sqrt()
    };
    kinetic - s.m[0] * s.m[1] / d(0, 1) - s.m[0] * s.m[2] / d(0, 2) - s.m[1] * s.m[2] / d(1, 2)
}

/// Positions shifted to their centre of mass.
fn centred(s: &State<f64>) -> [[f64; 2]; 3] {
    let mt: f64 = s.m.iter().sum();
    let rc = [0, 1].map(|j| (0..3).map(|i| s.m[i] * s.r[i][j]).sum::<f64>() / mt);
    s.r.map(|r| [r[0] - rc[0], r[1] - rc[1]])
}

/// The registration against the Cartesian state, computed apart: its energy `E` is the state's centre-of-momentum
/// energy; and Aarseth–Zare's march over one interval (`t = 1/8`) lands where Cartesian RK4 at `dt = 1/65536` does,
/// within `1e-9`, at unit and other mass totals.
fn check_against_cartesian(e_of: fn(&State<f64>) -> f64) {
    let p = params::<f64>();
    for s in [burrau::<f64>(), unnormalised()] {
        let (sys, _, _) = register(&s, p.tiny);
        assert!(
            (sys.e - e_of(&s)).abs() < 1e-13,
            "E on registration: {} against {}",
            sys.e,
            e_of(&s)
        );
        let out = march::<Rk4, f64, Counting, ExplicitTest>(
            &s,
            0,
            1,
            AzCount::new(),
            &p,
            &mut Counting::default(),
            &mut ExplicitTest,
        );
        let mut c = s;
        let mut g = Geom::new(&c.m, &c.r);
        for _ in 0..8192 {
            (c, g) = Rk4::step(&c, 0.125 / 8192.0, &g);
        }
        let (a, b) = (centred(&out.state), centred(&c));
        for i in 0..3 {
            for j in 0..2 {
                assert!(
                    (a[i][j] - b[i][j]).abs() < 1e-9,
                    "r[{i}][{j}]: {} against {}",
                    a[i][j],
                    b[i][j]
                );
            }
        }
    }
}

#[test]
fn az_against_cartesian() {
    check_against_cartesian(com_energy);
}

negative_control!(
    az_against_cartesian,
    "the energy without its potential",
    expected = "E on registration",
    check_against_cartesian(|s| com_energy(s) + 1.0)
);

/// The budget at its edge, `steps` against `max(N_min, rate·t)`, exceeded only above it, and a NaN allowance spent;
/// the explicit finiteness test false on a NaN in any one of the nine components.
fn check_guards(test: &mut dyn FnMut(&AzQ<f64>, &AzP<f64>) -> bool) {
    let mut p = params::<f64>();
    p.n_min = 100;
    p.step_budget_rate = 1000.0;
    assert!(!over_budget(100, 0.05, &p), "at N_min, not over");
    assert!(over_budget(101, 0.05, &p), "past N_min");
    assert!(
        !over_budget(200, 0.2, &p) && over_budget(201, 0.2, &p),
        "at and past rate·t"
    );
    assert!(over_budget(0, f64::NAN, &p), "a NaN allowance is spent");
    let s = burrau::<f64>();
    let (_, q, pm) = register(&s, p.tiny);
    assert!(test(&q, &pm), "a finite state is finite");
    for i in 0..9 {
        let (mut q1, mut p1) = (q, pm);
        let slot = match i {
            0 => &mut q1.u1[0],
            1 => &mut q1.u1[1],
            2 => &mut q1.u2[0],
            3 => &mut q1.u2[1],
            4 => &mut q1.t,
            5 => &mut p1.p1[0],
            6 => &mut p1.p1[1],
            7 => &mut p1.p2[0],
            _ => &mut p1.p2[1],
        };
        *slot = f64::NAN;
        assert!(!test(&q1, &p1), "a NaN in component {i} is not finite");
    }
}

#[test]
fn az_guards() {
    check_guards(&mut |q, p| ExplicitTest.finite(q, p));
}

negative_control!(
    az_guards,
    "a test that reads only u₁",
    expected = "is not finite",
    check_guards(&mut |q, _| q.u1.iter().all(|x| x.is_finite()))
);

// ── The interval's control on a scripted clock ─────────────────────────────────────────────────────────────────

/// How [`Scripted`] moves the interval clock, `x = A·B·h` being the step's physical size to first order; the
/// regularised positions and momenta stay as they are, so that `A·B` and the step sizes are the registration's.
#[derive(Clone, Copy)]
enum Clock {
    /// `t + κ·x`: a clock running `κ` times as fast as the step control reckons.
    Scale(f64),
    /// `t + x + c`: every step jumps `c` besides.
    Jump(f64),
    /// `T`, whatever the step.
    To(f64),
    /// `t + 2x` where `x ≥ 3Δ_sync/4`, else `t`: a short step leaves the clock where it was.
    Stall,
    /// One float short of `Δ_sync` where `x < Δ_sync/2`, else one float past it: no step lands the clock on it.
    Gap,
}

thread_local! {
    /// The clock [`Scripted`] runs (each test thread its own).
    static CLOCK: Cell<Clock> = const { Cell::new(Clock::Scale(1.0)) };
}

/// A stepper whose step moves only the interval clock, by [`CLOCK`]'s rule: the step control's landing, secant,
/// budget and counts, on values worked by hand.
struct Scripted;

impl AzStepper for Scripted {
    fn pair_profile() -> CapabilityProfile {
        <Rk4 as AzStepper>::pair_profile()
    }

    fn regularised_step<R: AzReal>(
        _system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        let (a, b) = AzSystem::ab(q);
        let x = a * b * h;
        let t = match CLOCK.with(Cell::get) {
            Clock::Scale(k) => q.t + v::<R>(k) * x,
            Clock::Jump(c) => q.t + x + v::<R>(c),
            Clock::To(t) => v(t),
            Clock::Gap => {
                let d = 0.125f64;
                let side = if x < v(0.0625) {
                    f64::from_bits(d.to_bits() - 1)
                } else {
                    f64::from_bits(d.to_bits() + 1)
                };
                v(side)
            }
            Clock::Stall => {
                if x >= v(0.75 * 0.125) {
                    q.t + v::<R>(2.0) * x
                } else {
                    q.t
                }
            }
        };
        (AzQ { t, ..*q }, *p)
    }
}

/// A finiteness test that records the interval clock of every state it is shown.
#[derive(Default)]
struct Clocks {
    t: Vec<f64>,
}

impl Watch<f64> for Clocks {
    fn finite(&mut self, q: &AzQ<f64>, p: &AzP<f64>) -> bool {
        self.t.push(q.t);
        ExplicitTest.finite(q, p)
    }
}

/// The scripted tests' values: [`params`] with `η = 10`, so that the clamp sets the first step, `x = Δ_sync`, and the
/// predictive limit off.
fn scripted() -> AzParams<f64> {
    let mut p = params::<f64>();
    p.eta = 10.0;
    p.predictive_f = 0.0;
    p
}

/// A march of Burrau's state over the one interval from boundary `k` under `clock` with `params`: its end, and the
/// clocks the finiteness test was shown.
type ScriptRun<'a> = &'a dyn Fn(Clock, u32, AzParams<f64>) -> (AzAdvanced<f64>, Vec<f64>);

/// The scripted march.
fn script(clock: Clock, k: u32, params: AzParams<f64>) -> (AzAdvanced<f64>, Vec<f64>) {
    CLOCK.with(|c| c.set(clock));
    let mut watch = Clocks::default();
    let out = march::<Scripted, f64, Counting, Clocks>(
        &burrau(),
        k,
        k + 1,
        AzCount::new(),
        &params,
        &mut Counting::default(),
        &mut watch,
    );
    (out, watch.t)
}

/// The landing and what the interval counts around it (Part 2b, steps 5 to 7; R-604, R-998), `Δ_sync = 1/8`:
/// - a clock twice as fast: the clamped step reaches `2Δ`, past, and the retakes land the clock on `Δ` exactly, the
///   march then at boundary 1;
/// - a clock that jumps `Δ/2` each step: the secant's 3 retakes end at `31Δ/30` by hand, still past, and the bisection
///   lands the clock on `Δ` exactly;
/// - a retake that leaves the clock where it was stops the secant at once, its state short at `t = 0`, and the
///   interval starts again (a budget of 4 ends it: three tries of two steps);
/// - a copy whose budget is spent as it lands is not counted landed: it stays at boundary 3, its time `3Δ` plus its
///   clock, `Δ`;
/// - the budget reads the physical time `k·Δ + t`: from boundary 4 at 8 steps per unit time the allowance is
///   `8·(1/2 + t)`, and the interval lands;
/// - with the retakes off, a step to exactly `2Δ` is not an overshoot, and one to `0.3` is.
///
/// No clock the march keeps is past the boundary.
fn check_secant(run: ScriptRun<'_>) {
    let never_past = |t: &[f64]| {
        assert!(
            t.iter().all(|&t| t <= 0.125),
            "a kept clock is past the boundary: {t:?}"
        );
    };
    let (out, t) = run(Clock::Scale(2.0), 0, scripted());
    assert_eq!((out.boundary, out.count.end), (1, End::Marching));
    assert_eq!(
        t.last().copied(),
        Some(0.125),
        "the retakes land the clock on Δ exactly ({t:?})"
    );
    assert_eq!(
        (
            out.count.steps,
            out.count.most_bisections,
            out.count.closings
        ),
        (2, 0, 0),
        "the clamped step and one retake at half its size, no halving, no closing step"
    );
    never_past(&t);

    // From `t = 0.6Δ`: `η = 0.3` puts the first step at `0.3Δ` (the clock `0.6Δ`), the clamped second reaches `1.4Δ`,
    // and the secant's `want` is the remainder `0.4Δ`, so one retake at half the step lands.
    let mut p = scripted();
    p.eta = 0.3;
    let (out, t) = run(Clock::Scale(2.0), 0, p);
    assert_eq!(
        (out.boundary, t.last().copied(), out.count.most_bisections),
        (1, Some(0.125), 0),
        "a landing from t_before > 0 takes the remainder as its want ({t:?}, {} steps)",
        out.count.steps
    );

    // The bisection's cap: a clock every step puts past the boundary, the secant off, 3 halvings.
    let mut p = scripted();
    p.secant_tries = 0;
    p.bisect_tries = 3;
    p.n_min = 3;
    p.step_budget_rate = 0.0;
    let (out, _) = run(Clock::To(0.25), 0, p);
    assert_eq!(
        (out.count.most_bisections, out.count.steps, out.count.end),
        (3, 4, End::Exhausted),
        "the bisection stops at its cap"
    );

    // A clock with no landing step: the bracket closes to adjacent step sizes before the cap of 64, the kept clock
    // one float short, and the copy, unable to close it, is spent unlanded.
    let mut p = scripted();
    p.n_min = 200;
    p.step_budget_rate = 0.0;
    let (out, t) = run(Clock::Gap, 0, p);
    assert!(
        out.count.most_bisections > 1 && out.count.most_bisections < 64,
        "the bracket closes before the cap: {} halvings",
        out.count.most_bisections
    );
    assert_eq!((out.count.end, out.boundary), (End::Exhausted, 0));
    never_past(&t);

    let mut p = scripted();
    p.secant_tries = 3;
    p.n_min = 200;
    p.step_budget_rate = 0.0;
    let (out, t) = run(Clock::Jump(0.0625), 0, p);
    assert_eq!(
        (out.boundary, out.count.end, t.last().copied()),
        (1, End::Marching, Some(0.125)),
        "the bisection lands the clock on Δ ({t:?})"
    );
    assert!(
        out.count.most_bisections >= 1 && out.count.steps >= 5,
        "the clamped step, the secant's 3 retakes and a halving: {} steps, {} halvings",
        out.count.steps,
        out.count.most_bisections
    );
    never_past(&t);

    let mut p = scripted();
    p.n_min = 4;
    p.step_budget_rate = 0.0;
    let (out, t) = run(Clock::Stall, 0, p);
    assert_eq!(
        (out.count.end, out.count.steps, out.count.accepted),
        (End::Exhausted, 6, 3),
        "a retake that leaves the clock stops the secant ({t:?})"
    );
    assert_eq!(out.count.closings, 1, "the short landing is counted once");
    never_past(&t);

    let mut p = scripted();
    p.n_min = 1;
    p.step_budget_rate = 0.0;
    let (out, _) = run(Clock::Scale(2.0), 3, p);
    assert_eq!(
        (out.count.end, out.boundary),
        (End::Exhausted, 3),
        "a copy spent as it lands stays at its boundary"
    );
    assert_eq!(out.t, 0.5, "its time the boundary's plus its clock");

    let mut p = scripted();
    p.n_min = 0;
    p.step_budget_rate = 8.0;
    let (out, _) = run(Clock::Scale(2.0), 4, p);
    assert_eq!(
        (out.count.end, out.boundary),
        (End::Marching, 5),
        "the allowance at k·Δ + t"
    );

    let mut p = scripted();
    p.secant_tries = 0;
    p.bisect_tries = 0;
    let (out, _) = run(Clock::To(0.25), 0, p);
    assert_eq!(
        out.count.overshoots, 0,
        "a step to exactly 2Δ is no overshoot"
    );
    let (out, _) = run(Clock::To(0.3), 0, p);
    assert_eq!(out.count.overshoots, 1, "a step past 2Δ is an overshoot");
}

#[test]
fn az_landing_secant() {
    check_secant(&script);
}

negative_control!(
    az_landing_secant,
    "a march without the landing's retakes, the clock set to the boundary over an overshot state",
    expected = "the retakes land the clock on Δ exactly",
    check_secant(&|c, k, mut p| {
        p.secant_tries = 0;
        p.bisect_tries = 0;
        script(c, k, p)
    })
);

/// No landing tolerance (R-998): a clock that stops `Δ·5e-15` short of the boundary never lands, with the clamp on or
/// off, the copy stepping on until its budget (3 steps) is spent; a clock that reaches the boundary exactly lands
/// either way; and a clock that every step and retake puts past the boundary is never kept: the copy is spent at
/// `t = 0`, unlanded. `run` is the scripted march under test.
fn check_no_tolerance(run: ScriptRun<'_>) {
    let short = 0.125 - 0.125 * 0.5e-14;
    let mut p = scripted();
    p.n_min = 3;
    p.step_budget_rate = 0.0;
    for clamp in [true, false] {
        p.clamp = clamp;
        let (out, _) = run(Clock::To(short), 0, p);
        assert_eq!(
            (out.count.end, out.boundary),
            (End::Exhausted, 0),
            "a clock short of the boundary never lands (clamp {clamp})"
        );
        let (out, _) = run(Clock::To(0.125), 0, p);
        assert_eq!(
            (out.count.end, out.boundary),
            (End::Marching, 1),
            "the boundary itself lands (clamp {clamp})"
        );
    }
    p.clamp = true;
    let past = f64::from_bits(0.125f64.to_bits() + 1);
    let (out, t) = run(Clock::To(past), 0, p);
    assert_eq!(
        (out.count.end, out.boundary),
        (End::Exhausted, 0),
        "a clock past the boundary is never kept ({t:?})"
    );
    assert!(t.iter().all(|&t| t == 0.0), "only t = 0 kept: {t:?}");
}

#[test]
fn az_landing_no_tolerance() {
    check_no_tolerance(&script);
}

negative_control!(
    az_landing_no_tolerance,
    "a march that keeps a step past the boundary, its retakes off",
    expected = "a clock past the boundary is never kept",
    check_no_tolerance(&|c, k, mut p| {
        p.secant_tries = 0;
        p.bisect_tries = 0;
        script(c, k, p)
    })
);

/// The state lands on the boundary with the clock (R-998; REQ-INT-119), under RK4 on Burrau's state at f64: at
/// `η = 10` with the predictive limit off each interval is one clamped step, whose first-order prediction of the
/// landing overshoots the boundary (asserted, on the first of the sync quanta tried that does). The march's stored
/// state is the image of the regularised state whose own clock is the boundary, bitwise, and its time reads the
/// boundary; that state is nearer the state at the boundary (a march at `η = 0.001`) than the overshot one is; and no
/// landing tolerance is read (`AzParams` has none). `retakes` sets the landing's retakes on `params`.
fn check_state_with_clock(retakes: fn(&mut AzParams<f64>)) {
    let s = burrau::<f64>();
    let quanta = [0.125, 0.25, 0.0625, 0.5, 0.03125];
    let mut p = params::<f64>();
    p.eta = 10.0;
    p.predictive_f = 0.0;
    let (sys, q, pm) = register(&s, p.tiny);
    let delta = quanta.into_iter().find(|&d| {
        let mut p = p;
        p.delta_sync = d;
        let h = step_size(&sys, &q, &pm, entry_size(&q, &p), &p).h;
        <Rk4 as AzStepper>::regularised_step(&sys, &q, &pm, h).0.t > d
    });
    let delta = delta.expect("a sync quantum whose clamped step overshoots");
    p.delta_sync = delta;
    let h = step_size(&sys, &q, &pm, entry_size(&q, &p), &p).h;
    let (oq, op) = <Rk4 as AzStepper>::regularised_step(&sys, &q, &pm, h);
    let overshot = sys.image(&oq, &op);
    let mut fine = p;
    fine.eta = 0.001;
    let at_boundary = march::<Rk4, f64, Counting, ExplicitTest>(
        &s,
        0,
        1,
        AzCount::new(),
        &fine,
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    retakes(&mut p);
    let mut watch = Clocks::default();
    let out = march::<Rk4, f64, Counting, Clocks>(
        &s,
        0,
        1,
        AzCount::new(),
        &p,
        &mut Counting::default(),
        &mut watch,
    );
    let gap = |a: &State<f64>, b: &State<f64>| {
        a.r.iter()
            .flatten()
            .zip(b.r.iter().flatten())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    };
    let (stored, overshoot) = (
        gap(&out.state, &at_boundary.state),
        gap(&overshot, &at_boundary.state),
    );
    eprintln!(
        "az_landing_state_with_clock: Δ_sync {delta}, the clamped step's clock {:e} (Δ + {:e}); the stored state {stored:e}          from the state at the boundary, the overshot one {overshoot:e}; {} steps, {} closings, {} halvings at most",
        oq.t,
        oq.t - delta,
        out.count.steps,
        out.count.closings,
        out.count.most_bisections
    );
    assert_eq!((out.boundary, out.count.end), (1, End::Marching));
    assert_eq!(out.t, delta, "the clock reads the boundary");
    assert_eq!(
        watch.t.last().copied(),
        Some(delta),
        "the stored state's own clock is the boundary, bitwise: {:?}",
        watch.t
    );
    assert!(
        stored < overshoot,
        "the stored state is nearer the state at the boundary ({stored:e}) than the overshot one ({overshoot:e})"
    );
}

#[test]
fn az_landing_state_with_clock() {
    check_state_with_clock(|_| {});
}

negative_control!(
    az_landing_state_with_clock,
    "the clock-only correction: the retakes off, the clock set to the boundary over the overshot state",
    expected = "the stored state's own clock is the boundary, bitwise",
    check_state_with_clock(|p| {
        p.secant_tries = 0;
        p.bisect_tries = 0;
    })
);

// ── The floor on |R₃| ──────────────────────────────────────────────────────────────────────────────────────────

/// The equations of motion read `|R₃|` floored at `TINY` at each working precision (R-999; REQ-INT-120): with
/// `u₂ = u₁`, so that `|R₃| = 0`, the term reads `TINY`, `1e-37` at f32 (not 0) and `1e-300` at f64; above the floor
/// it reads `|R₂ − R₁|` itself. `tiny32` is the f32 floor registered.
fn check_r3_floor(tiny32: f32) {
    let (sys, q, _) = register(&burrau::<f32>(), tiny32);
    let on = AzQ { u2: q.u1, ..q };
    assert_eq!(
        sys.r3(&on).to_bits(),
        1e-37f32.to_bits(),
        "at f32 |R₃| = 0 reads TINY, 1e-37, not {:e}",
        sys.r3(&on)
    );
    let r3 = {
        let d = [lc(q.u2)[0] - lc(q.u1)[0], lc(q.u2)[1] - lc(q.u1)[1]];
        (d[0] * d[0] + d[1] * d[1]).sqrt()
    };
    assert_eq!(
        sys.r3(&q),
        r3,
        "above TINY at f32 the term reads |R₃| itself"
    );
    let p = params::<f64>();
    let (sys, q, _) = register(&burrau::<f64>(), p.tiny);
    let on = AzQ { u2: q.u1, ..q };
    assert_eq!(sys.r3(&on), 1e-300, "at f64 |R₃| = 0 reads TINY, 1e-300");
    let r3 = {
        let d = [lc(q.u2)[0] - lc(q.u1)[0], lc(q.u2)[1] - lc(q.u1)[1]];
        (d[0] * d[0] + d[1] * d[1]).sqrt()
    };
    assert_eq!(
        sys.r3(&q),
        r3,
        "above TINY at f64 the term reads |R₃| itself"
    );
}

#[test]
fn az_r3_floor_tiny() {
    check_r3_floor(params::<f32>().tiny);
}

negative_control!(
    az_r3_floor_tiny,
    "a floor of 0 at f32",
    expected = "at f32 |R₃| = 0 reads TINY",
    check_r3_floor(0.0)
);

// ── The boundary's corrections and the seam's index ────────────────────────────────────────────────────────────

/// Where the quotient's whole count misses the boundary, the products correct it: `25.7/0.1` gives 257 and
/// `257 × 0.1` is above `25.7`; `18.7/0.1` gives 186 and `187 × 0.1` is `18.7`; and a time before `0` has the first
/// boundary.
fn check_boundary_corrected(boundary: fn(f64, f64) -> u32) {
    assert_eq!(
        boundary(25.7, 0.1),
        256,
        "one boundary back from the quotient's"
    );
    assert_eq!(
        boundary(18.7, 0.1),
        187,
        "one boundary on from the quotient's"
    );
    assert_eq!(boundary(-0.05, 0.125), 0, "before 0, the first boundary");
}

#[test]
fn az_boundary_corrected() {
    check_boundary_corrected(boundary_at_or_before::<f64>);
}

negative_control!(
    az_boundary_corrected,
    "the quotient's whole count, uncorrected",
    expected = "one boundary back from the quotient's",
    check_boundary_corrected(|t, d| (t / d) as u32)
);

/// The seam's `ADVANCE` from macro index 32 to 64 at `dt_macro = 1/128` (`t = 1/4` to `1/2`): the march from
/// boundary 2 to 4, bitwise, the cadence's steps its kept ones. `seam(t_now, t_target)` is the seam under test.
fn check_seam_index(seam: &dyn Fn(u32, u32) -> Advanced<f64>) {
    let out = march::<Rk4, f64, Counting, ExplicitTest>(
        &burrau(),
        2,
        4,
        AzCount::new(),
        &params(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    let via = seam(32, 64);
    assert_eq!(
        bits(&via.state),
        bits(&out.state),
        "the seam from index 32 at dt_macro = 1/128 marches boundary 2 to 4"
    );
    assert_eq!(via.substeps, out.count.accepted);
}

/// The seam's `ADVANCE` of Burrau's state at `dt_macro = 1/128`.
fn seam_at(t_now: u32, t_target: u32) -> Advanced<f64> {
    let s = burrau::<f64>();
    <Composed<Rk4, Az> as Advance>::advance(
        &s,
        &Geom::new(&s.m, &s.r),
        t_now,
        t_target,
        &AdvanceParams {
            dt_macro: 1.0 / 128.0,
            n_sub: 1,
        },
        &mut Counting::default(),
    )
}

#[test]
fn az_seam_index() {
    check_seam_index(&seam_at);
}

negative_control!(
    az_seam_index,
    "a seam that starts from index 0",
    expected = "the seam from index 32",
    check_seam_index(&|_, t| seam_at(0, t))
);

// ── Each form's order ──────────────────────────────────────────────────────────────────────────────────────────

/// The harmonic oscillator `q̇ = p`, `ṗ = −q`: separable, so that the KDK form is Störmer–Verlet, second order, and
/// the Yoshida compositions reach their own orders.
struct Spring;

impl Flow<f64> for Spring {
    type Q = f64;
    type P = f64;

    fn q_rate(&self, _q: &f64, p: &f64) -> f64 {
        *p
    }

    fn p_rate(&self, q: &f64, _p: &f64) -> f64 {
        -*q
    }

    fn q_add(q: &f64, h: f64, dq: &f64) -> f64 {
        q + h * dq
    }

    fn p_add(p: &f64, h: f64, dp: &f64) -> f64 {
        p + h * dp
    }

    fn q_weighted(two: f64, k: [&f64; 4]) -> f64 {
        k[0] + two * k[1] + two * k[2] + k[3]
    }

    fn p_weighted(two: f64, k: [&f64; 4]) -> f64 {
        k[0] + two * k[1] + two * k[2] + k[3]
    }
}

/// `form`'s error at `t = 1` from `(1, 0)` in `n` steps, against `(cos 1, −sin 1)`.
fn spring_error(form: Form, n: u32) -> f64 {
    let h = 1.0 / f64::from(n);
    let (mut q, mut p) = (1.0, 0.0);
    for _ in 0..n {
        (q, p) = form.step(&Spring, &q, &p, h);
    }
    (q - 1f64.cos()).hypot(p + 1f64.sin())
}

/// Each form's order on the oscillator, from 8 and 16 steps, within a quarter of its own: Euler 1, KDK 2, Yoshida-4 4,
/// Yoshida-6 6, RK4 4 (`forms` the form taken for each).
fn check_orders(forms: [(Form, &str, f64); 5]) {
    for (form, name, order) in forms {
        let measured = (spring_error(form, 8) / spring_error(form, 16)).log2();
        assert!(
            (measured - order).abs() <= 0.25,
            "the order of {name} on the oscillator: {measured:.3}, not {order}"
        );
    }
}

/// The forms and their orders.
const FORMS: [(Form, &str, f64); 5] = [
    (Form::Euler, "Euler", 1.0),
    (Form::Kdk, "KDK", 2.0),
    (Form::Yoshida4, "Yoshida-4", 4.0),
    (Form::Yoshida6, "Yoshida-6", 6.0),
    (Form::Rk4, "RK4", 4.0),
];

#[test]
fn az_form_orders() {
    check_orders(FORMS);
}

negative_control!(
    az_form_orders,
    "Yoshida-4's form taken for Yoshida-6",
    expected = "the order of Yoshida-6",
    check_orders({
        let mut forms = FORMS;
        forms[3].0 = Form::Yoshida4;
        forms
    })
);

// ── The seam's end, through the wrapper ────────────────────────────────────────────────────────────────────────

/// A sim key for the wrapper's loop: `dt_macro`, `horizon_steps`, and `N_sub = n_max` throughout (every threshold the
/// largest finite f32, so every finite `d²` is below them all), so that an occupant's step count is never `N_sub` by
/// accident.
fn key<R: AzReal>(dt_macro: f64, horizon_steps: u32, n_max: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: v(dt_macro),
        horizon_steps,
        n_max,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// The IC of the state `s`.
fn ic_of<R: Copy>(s: &State<R>) -> Ic<R> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// The wrapper's run of `ic` under `uniforms` by the occupant under test, with the cadence `C`.
type WrapperRun<R, C> = fn(&Ic<R>, &WrapperUniforms<R>, &mut C) -> SimStateBaseOf<R>;

/// The wrapper's run by `A`.
fn run_by<A: Advance, R: WrapperReal, C: Cadence<R>>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut C,
) -> SimStateBaseOf<R> {
    integrate_with::<A, R, C>(ic, uniforms, cadence)
}

/// A cadence that counts its calls and is done from its `n`-th on.
struct DoneAfter {
    n: u32,
    calls: u32,
}

impl DoneAfter {
    fn new(n: u32) -> Self {
        DoneAfter { n, calls: 0 }
    }
}

impl<R> Cadence<R> for DoneAfter {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        self.calls >= self.n
    }
}

/// The occupant `A` read by the stepper's rule alone, its end dropped: a macro-step completed when the steps taken are
/// `N_sub`, and no march ever ended (a control's; the seam before PR #502's fix).
#[cfg(feature = "controls")]
struct Dropping<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Dropping<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }
}

/// The occupant `A` with a call that stopped short counted as its target reached (a control's).
#[cfg(feature = "controls")]
struct Overclaiming<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Overclaiming<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }

    fn advance_reaching<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, Reached) {
        let (out, reached) = A::advance_reaching(state, geom, t_now, t_target, params, cadence);
        let reached = if reached == Reached::Short {
            Reached::Target
        } else {
            reached
        };
        (out, reached)
    }
}

/// The occupant `A` read by the rule of an occupant that owns its time mapping, short only on the cadence's `done`
/// (a control's, for `none`).
#[cfg(feature = "controls")]
struct Shorting<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Shorting<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }

    fn advance_reaching<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, Reached) {
        let out = A::advance(state, geom, t_now, t_target, params, cadence);
        let reached = if cadence.done() {
            Reached::Short
        } else {
            Reached::Target
        };
        (out, reached)
    }
}

/// Aarseth–Zare + RK4, the composed occupant through the seam.
type AzRk4 = Composed<Rk4, Az>;

/// The degenerate fixture through the wrapper at `dt_macro = 1/16` (`Δ_sync = 1/8`), eight macro-steps, `N_sub = 4`:
/// the first macro-step's target is before the first boundary, so it takes no step and is completed; the second's
/// march registers the degenerate state and ends at the explicit finiteness test. The run is `sim_failed`, never
/// `bounded`: latched at the macro-step it ended in, which is not completed (`t_end_step = 1`), its state the march's
/// latched finite one, its detail 2 (a non-finite derived quantity on a finite state, payload §2), and, through
/// [`integrate`]'s monitored fields, `dmin_pair` its sentinel (Part 2a § "The sync grid is a fixed quantum of physical
/// time (R-565)": "a terminal event … still latches at the step it fires on").
fn check_non_finite_fails(
    run: WrapperRun<f32, Counting>,
    monitored: fn(&Ic<f32>) -> SimStateBaseOf<f32>,
) {
    let ic = ic_of(&degenerate());
    let out = run(&ic, &key(1.0 / 16.0, 8, 4), &mut Counting::default());
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "a march ended non-finite fails the run: state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "the run latches at the macro-step its march ended in"
    );
    assert_eq!(
        sd_detail(out.packed_a),
        2,
        "a non-finite step from a finite state"
    );
    assert_eq!(out.total_substeps, 0, "no non-finite step is kept");
    let latched = degenerate_march(ExplicitTest).state;
    assert_eq!(
        bits(&State {
            m: latched.m,
            r: out.r,
            p: out.p
        }),
        bits(&latched),
        "the state is the march's latched finite one"
    );
    let full = monitored(&ic);
    assert_eq!(
        sd_state(full.packed_a),
        STATE_SIM_FAILED,
        "monitored keeps sim_failed"
    );
    assert_eq!(sd_detail(full.packed_a), 2, "monitored keeps the detail");
    assert_eq!(
        sd_dmin_pair(full.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a failed run's dmin_pair is its sentinel"
    );
}

/// The wrapper's own run of `ic` by Aarseth–Zare + RK4, its monitored fields written, at `r_coll = 0`.
fn monitored_az(ic: &Ic<f32>) -> SimStateBaseOf<f32> {
    integrate::<AzRk4, f32>(ic, &key(1.0 / 16.0, 8, 4))
}

#[test]
fn az_seam_non_finite_fails() {
    check_non_finite_fails(run_by::<AzRk4, f32, Counting>, monitored_az);
}

negative_control!(
    az_seam_non_finite_fails,
    "a seam that drops the march's end, the run labelled bounded",
    expected = "a march ended non-finite fails the run",
    check_non_finite_fails(run_by::<Dropping<AzRk4>, f32, Counting>, |ic| {
        integrate::<Dropping<AzRk4>, f32>(ic, &key(1.0 / 16.0, 8, 4))
    })
);

/// The seam's calls of a run of Burrau's state at `dt_macro = 1/16`, eight macro-steps, replayed one by one: the
/// state each returns, the steps taken, and the Welford co-moments after each as a completed macro-step (payload §4).
fn replay() -> (State<f64>, u32, Welford<f64>) {
    let s0 = burrau::<f64>();
    let n_0 = shape(s0.r, s0.m);
    let (mut s, mut steps, mut w) = (s0, 0, Welford::default());
    let mut cadence = Counting::default();
    for k in 0..8 {
        let out = <AzRk4 as Advance>::advance(
            &s,
            &Geom::new(&s.m, &s.r),
            k,
            k + 1,
            &AdvanceParams {
                dt_macro: 1.0 / 16.0,
                n_sub: 4,
            },
            &mut cadence,
        );
        s = out.state;
        steps += out.substeps;
        w = w.update(welford::y(s.r, s.m, n_0), k + 1, 1.0 / 16.0);
    }
    (s, steps, w)
}

/// Burrau's state through the wrapper at `dt_macro = 1/16` (`Δ_sync = 1/8`), eight macro-steps (`t = 1/2`, four
/// boundaries), `N_sub = 4`, which Aarseth–Zare's step count never is: every macro-step reaches its boundary at or
/// before its target (R-565) and is completed, whatever the steps it took (Part 2a, Cost 2), so the run is `bounded`
/// with `t_end_step = 8` and eight Welford samples, each on the state the seam returned (payload §2, §4), bitwise the
/// replay's.
fn check_completes(run: WrapperRun<f64, Counting>) {
    let out = run(
        &ic_of(&burrau::<f64>()),
        &key(1.0 / 16.0, 8, 4),
        &mut Counting::default(),
    );
    let (s, steps, w) = replay();
    assert_eq!(
        tm_t_end_step(out.times),
        8,
        "every macro-step that reached its boundary is completed"
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_BOUNDED,
        "a run to the horizon is bounded"
    );
    assert!(steps > 0, "the march stepped");
    assert_eq!(out.total_substeps, steps, "the steps are the seam's");
    assert_eq!(
        (out.mean_y.to_bits(), out.C_ty.to_bits()),
        (w.mean_y.to_bits(), w.c_ty.to_bits()),
        "the diffusion takes one sample per completed macro-step: {} {} against {} {}",
        out.mean_y,
        out.C_ty,
        w.mean_y,
        w.c_ty
    );
    assert!(w.mean_y > 0.0, "the samples see the shape move");
    assert_eq!(
        (bits(&State {
            m: s.m,
            r: out.r,
            p: out.p
        })),
        bits(&s),
        "the state is the seam's"
    );
}

#[test]
fn az_seam_completes() {
    check_completes(run_by::<AzRk4, f64, Counting>);
}

negative_control!(
    az_seam_completes,
    "a seam read by the stepper's rule, a macro-step completed only at N_sub steps",
    expected = "every macro-step that reached its boundary is completed",
    check_completes(run_by::<Dropping<AzRk4>, f64, Counting>)
);

/// The test values with a budget of 4 steps, which the first interval's march spends.
struct Spent;

impl AzConfig for Spent {
    fn params<R: AzReal>() -> AzParams<R> {
        let mut p = params::<R>();
        p.n_min = 4;
        p.step_budget_rate = R::default();
        p
    }
}

/// Aarseth–Zare + RK4 on a budget of 4 steps.
type SpentRk4 = Composed<Rk4, AarsethZare<Spent>>;

/// The budget here is `N_min = 4` at rate 0, so one seam call's fresh count spends it whatever the time: the cumulative
/// budget across seam calls at `step_budget_rate > 0` (R-604) is TASK-M3-41's, with the count carried through the seam
/// (code review 5880407311 of PR #502, F1).
///
/// Burrau's state through the wrapper as in [`check_completes`] on a budget of 4 steps: the first macro-step takes no
/// step and is completed, the second's march spends its budget, and the copy is never run on (R-604): the run stops
/// there, `running` as it stood (Part 7; R-603's flag being TASK-M3-41's), at `t_end_step = 1`, with the state and
/// steps of that one march.
fn check_exhausted_stops(run: WrapperRun<f64, Counting>) {
    let out = run(
        &ic_of(&burrau::<f64>()),
        &key(1.0 / 16.0, 8, 4),
        &mut Counting::default(),
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_RUNNING,
        "a spent copy stops as it stood: state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "the spent macro-step is not completed"
    );
    let s = burrau::<f64>();
    let once = <SpentRk4 as Advance>::advance(
        &s,
        &Geom::new(&s.m, &s.r),
        1,
        2,
        &AdvanceParams {
            dt_macro: 1.0 / 16.0,
            n_sub: 4,
        },
        &mut Counting::default(),
    );
    assert_eq!(out.total_substeps, once.substeps, "one march, never run on");
    assert_eq!(
        (bits(&State {
            m: s.m,
            r: out.r,
            p: out.p
        })),
        bits(&once.state),
        "the state the march latched"
    );
}

#[test]
fn az_seam_exhausted_stops() {
    check_exhausted_stops(run_by::<SpentRk4, f64, Counting>);
}

negative_control!(
    az_seam_exhausted_stops,
    "a seam that drops the spent budget, the copy run on",
    expected = "a spent copy stops as it stood",
    check_exhausted_stops(run_by::<Dropping<SpentRk4>, f64, Counting>)
);

/// Burrau's state through the wrapper as in [`check_completes`] with a cadence done from its first call: the first
/// macro-step takes no step and is completed, the second's march stops on `done` after one step, short of its
/// boundary, and is not completed: `running` at `t_end_step = 1`.
fn check_done_short(run: WrapperRun<f64, DoneAfter>) {
    let mut cadence = DoneAfter::new(1);
    let out = run(
        &ic_of(&burrau::<f64>()),
        &key(1.0 / 16.0, 8, 4),
        &mut cadence,
    );
    assert_eq!(cadence.calls, 1, "the march stopped on done");
    assert_eq!(sd_state(out.packed_a), STATE_RUNNING);
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "a march stopped on done is not completed"
    );
}

#[test]
fn az_seam_done_short() {
    check_done_short(run_by::<AzRk4, f64, DoneAfter>);
}

negative_control!(
    az_seam_done_short,
    "a seam that counts a march stopped on done as completed",
    expected = "a march stopped on done is not completed",
    check_done_short(run_by::<Overclaiming<AzRk4>, f64, DoneAfter>)
);

/// A stand-in regularisation that owns its time mapping and keeps the default end ([`Regularisation`]'s
/// `advance_reaching`): each call one step that leaves the state as it was, the cadence after it.
struct Held;

impl Regularisation for Held {
    const KIND: Kind = Kind::Heggie;

    fn profile<S: Stepper>() -> CapabilityProfile {
        Az::profile::<S>()
    }

    fn advance<S: Stepper, R: kernel::driver::advance::AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        _t_now: u32,
        _t_target: u32,
        _params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let (mut s, mut g) = (*state, *geom);
        cadence.after_step(&mut s, &mut g);
        Advanced {
            state: s,
            geom: g,
            substeps: 1,
        }
    }
}

/// The default end of an occupant that owns its time mapping, through the wrapper at `N_sub = 4` (its one step never
/// `N_sub`): every call reaches its target, eight macro-steps completed and `bounded`, unless the cadence's `done`
/// stopped it, when none is (`running`, `t_end_step = 0`).
fn check_owning_default(run: WrapperRun<f64, Counting>, run_done: WrapperRun<f64, DoneAfter>) {
    let ic = ic_of(&burrau::<f64>());
    let out = run(&ic, &key(1.0 / 16.0, 8, 4), &mut Counting::default());
    assert_eq!(
        (tm_t_end_step(out.times), sd_state(out.packed_a)),
        (8, STATE_BOUNDED),
        "an occupant owning its time mapping completes each macro-step it reaches"
    );
    let out = run_done(&ic, &key(1.0 / 16.0, 8, 4), &mut DoneAfter::new(1));
    assert_eq!(
        (tm_t_end_step(out.times), sd_state(out.packed_a)),
        (0, STATE_RUNNING),
        "a call stopped on done completes nothing"
    );
}

#[test]
fn regularisation_owning_default_end() {
    check_owning_default(
        run_by::<Composed<Rk4, Held>, f64, Counting>,
        run_by::<Composed<Rk4, Held>, f64, DoneAfter>,
    );
}

negative_control!(
    regularisation_owning_default_end,
    "the stepper's rule for an occupant owning its time mapping",
    expected = "an occupant owning its time mapping completes each macro-step it reaches",
    check_owning_default(
        run_by::<Dropping<Composed<Rk4, Held>>, f64, Counting>,
        run_by::<Composed<Rk4, Held>, f64, DoneAfter>,
    )
);

negative_control!(
    regularisation_owning_default_end_done,
    "a call stopped on done counted as completed",
    expected = "a call stopped on done completes nothing",
    check_owning_default(
        run_by::<Composed<Rk4, Held>, f64, Counting>,
        run_by::<Overclaiming<Composed<Rk4, Held>>, f64, DoneAfter>,
    )
);

/// `none` keeps the stepper's rule, bitwise the stepper's own run: under a cadence done from its fourth call at
/// `N_sub = 4`, the first macro-step's fourth substep fires `done` and the macro-step, all its substeps taken, is
/// completed (`t_end_step = 1`), where the rule of an occupant owning its time mapping would count it short.
fn check_none_keeps_count(run: WrapperRun<f64, DoneAfter>) {
    let ic = ic_of(&burrau::<f64>());
    let out = run(&ic, &key(1.0 / 16.0, 8, 4), &mut DoneAfter::new(4));
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "none completes a macro-step whose N_sub substeps were taken"
    );
    let own =
        integrate_with::<Rk4, f64, DoneAfter>(&ic, &key(1.0 / 16.0, 8, 4), &mut DoneAfter::new(4));
    assert_eq!(
        format!("{out:?}"),
        format!("{own:?}"),
        "none is the stepper's own run"
    );
    let out = run(&ic, &key(1.0 / 16.0, 8, 4), &mut DoneAfter::new(u32::MAX));
    let own = integrate_with::<Rk4, f64, DoneAfter>(
        &ic,
        &key(1.0 / 16.0, 8, 4),
        &mut DoneAfter::new(u32::MAX),
    );
    assert_eq!(
        format!("{out:?}"),
        format!("{own:?}"),
        "none is the stepper's own run to the horizon"
    );
    assert_eq!(tm_t_end_step(own.times), 8);
}

#[test]
fn regularisation_none_keeps_count() {
    check_none_keeps_count(run_by::<Composed<Rk4, NoRegularisation>, f64, DoneAfter>);
}

negative_control!(
    regularisation_none_keeps_count,
    "none read by the rule of an occupant owning its time mapping",
    expected = "none completes a macro-step whose N_sub substeps were taken",
    check_none_keeps_count(run_by::<Shorting<Composed<Rk4, NoRegularisation>>, f64, DoneAfter>)
);

/// A cadence that is never done, whatever it is told: a terminal's `done` left unread (a control's).
#[cfg(feature = "controls")]
struct Unread(DoneAfter);

#[cfg(feature = "controls")]
impl<R> Cadence<R> for Unread {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.0.after_step(state, geom);
    }
    fn done(&self) -> bool {
        false
    }
}

/// One seam call spanning four sync boundaries (`dt_macro = 1/2`, `Δ_sync = 1/8`) with a terminal that fires on the
/// first interval's landing step (the cadence done from the call that follows that step): the march ends there, its
/// state the landing's, latched, never marched on into the next interval, and the call is short of its target (Part
/// 2a § "The sync grid is a fixed quantum of physical time (R-565)": "a terminal event … still latches at the step it
/// fires on"; code review 5880407311 of PR #502, F2). `cadence(n)` is the cadence done from its `n`-th call.
fn check_done_on_landing<C: Cadence<f64>>(cadence: fn(u32) -> C) {
    let s = burrau::<f64>();
    let first = march::<Rk4, f64, Counting, ExplicitTest>(
        &s,
        0,
        1,
        AzCount::new(),
        &params(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    assert_eq!(first.boundary, 1, "the first interval lands");
    let n = first.count.accepted;
    let mut c = cadence(n);
    let (out, reached) = <AzRk4 as Advance>::advance_reaching(
        &s,
        &Geom::new(&s.m, &s.r),
        0,
        1,
        &AdvanceParams {
            dt_macro: 0.5,
            n_sub: 4,
        },
        &mut c,
    );
    assert_eq!(
        (out.substeps, bits(&out.state)),
        (n, bits(&first.state)),
        "the march stops at the landing step its terminal fired on"
    );
    assert_eq!(
        reached,
        Reached::Short,
        "a call ended by a terminal is short"
    );
}

#[test]
fn az_done_on_landing() {
    check_done_on_landing(DoneAfter::new);
}

negative_control!(
    az_done_on_landing,
    "the terminal's done left unread, the march carried on past it",
    expected = "the march stops at the landing step its terminal fired on",
    check_done_on_landing(|n| Unread(DoneAfter::new(n)))
);

/// The seam call from macro index 0 to 1 at `dt_macro` by `A`, from Burrau's state, with `cadence`.
fn reach_by<A: Advance>(dt_macro: f64, cadence: &mut DoneAfter) -> (Advanced<f64>, Reached) {
    let s = burrau::<f64>();
    A::advance_reaching(
        &s,
        &Geom::new(&s.m, &s.r),
        0,
        1,
        &AdvanceParams { dt_macro, n_sub: 4 },
        cadence,
    )
}

/// The seam's call under test, [`reach_by`] for an occupant.
type Reach = fn(f64, &mut DoneAfter) -> (Advanced<f64>, Reached);

/// A terminal that fires on the landing step of the call's own target boundary, `boundaries` sync intervals away
/// (`dt_macro = boundaries·Δ_sync`): the landing is registered, the boundary reached at its time exactly, then the march
/// ends (`Done`), and the call reports its target reached, as a stepper's terminal on its last substep completes the
/// macro-step; through the wrapper the macro-step is completed (`t_end_step = 1`) with its Welford sample on the
/// landing state, the run `running` for the terminal's label (physics review 5880652590 of PR #502, P3; code review
/// 5880701680, F3).
fn check_done_on_target(boundaries: u32, reach: Reach, run: WrapperRun<f64, DoneAfter>) {
    let s = burrau::<f64>();
    let dt_macro = 0.125 * f64::from(boundaries);
    let free = march::<Rk4, f64, Counting, ExplicitTest>(
        &s,
        0,
        boundaries,
        AzCount::new(),
        &params(),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    assert_eq!(free.boundary, boundaries, "the free march lands");
    let n = free.count.accepted;
    let (out, reached) = reach(dt_macro, &mut DoneAfter::new(n));
    assert_eq!(
        reached,
        Reached::Target,
        "a terminal on the target boundary's landing step reaches the target ({boundaries} boundaries)"
    );
    assert_eq!(
        (out.substeps, bits(&out.state)),
        (n, bits(&free.state)),
        "the landing state, latched"
    );
    let latched = march::<Rk4, f64, DoneAfter, ExplicitTest>(
        &s,
        0,
        boundaries,
        AzCount::new(),
        &params(),
        &mut DoneAfter::new(n),
        &mut ExplicitTest,
    );
    assert_eq!(
        (latched.count.end, latched.boundary),
        (End::Done, boundaries),
        "the landing is registered, then the march ends"
    );
    assert_eq!(
        latched.t.to_bits(),
        (0.125 * f64::from(boundaries)).to_bits(),
        "the time is the boundary's exactly: {}",
        latched.t
    );
    let got = run(&ic_of(&s), &key(dt_macro, 8, 4), &mut DoneAfter::new(n));
    assert_eq!(
        (tm_t_end_step(got.times), sd_state(got.packed_a)),
        (1, STATE_RUNNING),
        "the macro-step is completed"
    );
    let y = welford::y(free.state.r, s.m, shape(s.r, s.m));
    assert_eq!(
        (got.mean_y.to_bits(), got.C_ty.to_bits()),
        (y.to_bits(), 0f64.to_bits()),
        "its Welford sample is taken on the landing state"
    );
}

#[test]
fn az_done_on_target_landing() {
    check_done_on_target(1, reach_by::<AzRk4>, run_by::<AzRk4, f64, DoneAfter>);
    check_done_on_target(4, reach_by::<AzRk4>, run_by::<AzRk4, f64, DoneAfter>);
}

negative_control!(
    az_done_on_target_landing,
    "a terminal on the target boundary's landing step read as short, the macro-step dropped",
    expected = "a terminal on the target boundary's landing step reaches the target",
    check_done_on_target(
        1,
        reach_by::<Shorting<AzRk4>>,
        run_by::<Shorting<AzRk4>, f64, DoneAfter>,
    )
);
