//! Heggie (`kernel::regularisation::heggie`; integrator_contract Part 2b § "Heggie: the equations, the time
//! transformations, the maps and the step control (transcribed)", § "The time-reversal maps (REQ-INT-095)", Part 1,
//! Part 6; R-160, R-161, R-500, R-561), TASK-M3-08:
//! - `heggie_pairs`: Heggie × every stepper under Eq. 20 and Eq. 22 marches a fixture state across a sync boundary as a
//!   direct march, registered once at `t = 0`, its state finite and moved and its profile its form's; the default time
//!   transformation is Eq. 22 (R-161). Controls: a stepper that takes no step does not land, and a default of Eq. 20.
//! - `heggie_march_*`: the transcription's checks. The round trip Cartesian → regularised → Cartesian at unequal
//!   masses, the registered energy and Eq. 7's velocities against the Cartesian state (control: a quarter-turned `Q₀`);
//!   `Γ* = (E − h)·R₀R₁R₂` anchored on the Cartesian energy of the image (control: two reduced masses swapped); the
//!   march registered once, `h` held, landing on each boundary at `k·Δ_sync` (control: a march re-registered at a
//!   boundary); the rates against a finite difference of the Hamiltonian (control: a sign flipped); the explicit
//!   finiteness exit at entry and mid-interval, the state latched finite (control: the test removed); R-604's budget
//!   ending the march latched (control: a budget that does not bind); the floor signal on `dt/dτ < TINY` or `Rᵢ < TINY`
//!   at its edges (control: the reference's `Rᵢ` arm alone); the entry-cap hit, the predictive limit and the clamp at
//!   their edges; the explicit test on every component.
//! - `regularised_step_reversibility`: dd_integrator §5's test 3 on Heggie × RK4 (Eq. 20 and Eq. 22) and Aarseth–Zare +
//!   RK4, each through its time-reversal map, on its full state, at REQ-VAL-191's criterion inside its domain: each
//!   residual and verdict printed, each verdict against the pair's profile; at the same `dt`, Cartesian RK4 and
//!   REQ-VAL-032's asymmetric Yoshida-4 read not reversible. Controls: a profile with one flag flipped, and a
//!   measurement set with one built occupant left out (REQ-VAL-190's Heggie and Aarseth–Zare arms; R-500).

use std::cell::RefCell;

use kernel::driver::advance::Cadence;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, Geom, State, Stepper};
use kernel::regularisation::az::{self, AzP, AzQ, AzReal, AzStepper, AzSystem, End, ExplicitTest};
use kernel::regularisation::dt_max::DtMaxReport;
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::heggie::{
    self as hg, march, register, reversed, step_size, HgAdvanced, HgCount, HgP, HgParams, HgQ,
    HgStepper, HgSystem, HgWatch, DEFAULT_TIME,
};
use kernel::regularisation::pairs::HeggieTime;
use validation::{
    negative_control, STEP_REVERSAL_EDGE_F32, STEP_REVERSAL_EDGE_F64, STEP_REVERSAL_EDGE_MAX,
    STEP_REVERSAL_TOL,
};

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// `x` at `R`'s width.
fn v<R: AzReal>(x: f64) -> R {
    R::at_width(x as f32, x)
}

/// Test values for the step control: `η = 0.01`, `Δ_sync = 1/8`, the reference's `TINY` and landing tolerance, R-160's
/// `f = 0.02`, a budget that does not bind, the clamp and the secant on, under `time`. Test inputs, not calibrations.
fn params<R: AzReal>(time: HeggieTime) -> HgParams<R> {
    HgParams {
        eta: v(0.01),
        delta_sync: v(0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: v(0.02),
        land_eps_rel: R::at_width(1e-5, 1e-14),
        step_budget_rate: v(1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        time,
    }
}

/// A generic bound triple at unequal masses `(1, 0.8, 1.3)`, moved to its centre of mass and momentum.
fn triple<R: AzReal>() -> State<R> {
    let m = [1.0, 0.8, 1.3];
    let r = [[-0.6, 0.1], [0.5, -0.35], [0.15, 0.6]];
    let vel = [[0.05, 0.3], [-0.25, -0.1], [0.1, -0.2]];
    let total: f64 = m.iter().sum();
    let mut rc = [0.0; 2];
    let mut pc = [0.0; 2];
    for i in 0..3 {
        for c in 0..2 {
            rc[c] += m[i] * r[i][c] / total;
            pc[c] += m[i] * vel[i][c];
        }
    }
    State {
        m: m.map(v),
        r: [0, 1, 2].map(|i| [0, 1].map(|c| v(r[i][c] - rc[c]))),
        p: [0, 1, 2].map(|i| [0, 1].map(|c| v(m[i] * vel[i][c] - m[i] * pc[c] / total))),
    }
}

/// A cadence that keeps nothing and never ends the march.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// The physical energy of `s` at f64.
fn energy<R: AzReal + Into<f64>>(s: &State<R>) -> f64 {
    let m: [f64; 3] = s.m.map(Into::into);
    let r: [[f64; 2]; 3] = s.r.map(|x| x.map(Into::into));
    let p: [[f64; 2]; 3] = s.p.map(|x| x.map(Into::into));
    let kinetic: f64 = (0..3)
        .map(|i| (p[i][0].powi(2) + p[i][1].powi(2)) / (2.0 * m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| m[i] * m[j] / (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]))
        .sum();
    kinetic - potential
}

/// The largest component of `a − b` over positions and momenta, relative to the largest of `b`'s.
fn state_diff(a: &State<f64>, b: &State<f64>) -> f64 {
    let flat =
        |s: &State<f64>| -> Vec<f64> { s.r.iter().chain(s.p.iter()).flatten().copied().collect() };
    let (x, y) = (flat(a), flat(b));
    let scale = y.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    x.iter()
        .zip(&y)
        .fold(0.0f64, |m, (p, q)| m.max((p - q).abs()))
        / scale
}

/// Whether every position and momentum of `s` is finite.
fn finite_state(s: &State<f64>) -> bool {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .all(|x| x.is_finite())
}

// ── heggie_pairs ───────────────────────────────────────────────────────────────────────────────────────────────

/// What one pair's march gave.
struct PairRun {
    name: &'static str,
    out: HgAdvanced<f64>,
    profile: CapabilityProfile,
    form: CapabilityProfile,
}

/// Heggie × `S` under `time` from [`triple`] to the first boundary.
fn pair_run<S: HgStepper>(name: &'static str, time: HeggieTime, form: Form) -> PairRun {
    let mut p = params::<f64>(time);
    p.n_min = 5_000;
    p.step_budget_rate = 0.0;
    PairRun {
        name,
        out: march::<S, f64, Quiet, ExplicitTest>(&triple(), 1, &p, &mut Quiet, &mut ExplicitTest),
        profile: hg::profile::<S>(),
        form: form.pair_profile(),
    }
}

/// Each run landed on boundary 1 at its time exactly, still marching, its state finite and moved from the start, the
/// cadence having followed its steps, and its profile its form's.
fn check_pairs(runs: &[PairRun]) {
    let s0 = triple::<f64>();
    for r in runs {
        assert_eq!(
            r.out.count.end,
            End::Marching,
            "{}: did not land: {:?}",
            r.name,
            r.out.count.end
        );
        assert_eq!(
            (r.out.boundary, r.out.t),
            (1, 0.125),
            "{}: did not land on boundary 1",
            r.name
        );
        assert!(
            finite_state(&r.out.state),
            "{}: the state is not finite",
            r.name
        );
        assert!(
            state_diff(&r.out.state, &s0) > 1e-3,
            "{}: the state did not move",
            r.name
        );
        assert!(
            r.out.count.accepted > 0,
            "{}: the cadence followed no step",
            r.name
        );
        assert_eq!(
            r.profile, r.form,
            "{}: the profile is not its form's",
            r.name
        );
        println!(
            "heggie_pairs: {}: {} steps, |ΔE_reg/E| {:.3e}",
            r.name,
            r.out.count.steps,
            ((r.out.regularised() - r.out.system.h) / r.out.system.h).abs()
        );
    }
}

/// Heggie's default time transformation is Eq. 22 at n = 3/2 (R-161).
fn check_default(time: HeggieTime) {
    assert_eq!(
        time,
        HeggieTime::Eq22,
        "the default time transformation is not Eq. 22"
    );
}

/// Every stepper's run under `time`.
fn every_pair(time: HeggieTime) -> Vec<PairRun> {
    vec![
        pair_run::<Euler>("Euler", time, Form::Euler),
        pair_run::<Kdk>("KDK", time, Form::Kdk),
        pair_run::<Yoshida4>("Yoshida-4", time, Form::Yoshida4),
        pair_run::<Yoshida6>("Yoshida-6", time, Form::Yoshida6),
        pair_run::<Rk4>("RK4", time, Form::Rk4),
    ]
}

#[test]
fn heggie_pairs() {
    check_pairs(&every_pair(HeggieTime::Eq20));
    check_pairs(&every_pair(HeggieTime::Eq22));
    check_default(DEFAULT_TIME);
    check_default(params::<f64>(DEFAULT_TIME).time);
}

/// A stepper that takes no step: the control's.
struct NoStep;

impl HgStepper for NoStep {
    fn pair_profile() -> CapabilityProfile {
        Form::Rk4.pair_profile()
    }

    fn regularised_step<R: AzReal>(
        _system: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        _h: R,
    ) -> (HgQ<R>, HgP<R>) {
        (*q, *p)
    }
}

negative_control!(
    heggie_pairs,
    "a stepper that takes no step, which never lands",
    expected = "did not land",
    check_pairs(&[pair_run::<NoStep>("no step", DEFAULT_TIME, Form::Rk4)])
);

negative_control!(
    heggie_pairs_default,
    "a default of Eq. 20",
    expected = "the default time transformation is not Eq. 22",
    check_default(HeggieTime::Eq20)
);

// ── heggie_march: the maps and the energy ──────────────────────────────────────────────────────────────────────

/// The round trip of `s` through registration and back, at unequal masses, within `1e-14` relative; the registered `h`
/// and the regularised energy of the registration both the Cartesian energy within `1e-14`; and Eq. 7's relative
/// velocities the image's `vⱼ − vₖ` within `1e-14`. `turn` quarter-turns `Q₀` first, the control's.
fn check_round_trip(turn: bool) {
    let s = triple::<f64>();
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let (system, mut q, p) = register(&s, 1e-300, time);
        if turn {
            q.q[0] = [-q.q[0][1], q.q[0][0]];
        }
        let back = system.image(&q, &p);
        let d = state_diff(&back, &s);
        assert!(d < 1e-14, "the round trip misses the state by {d:.3e}");
        let e = energy(&s);
        assert!(
            ((system.h - e) / e).abs() < 1e-14,
            "h is not the Cartesian energy"
        );
        assert!(
            ((system.energy(&q, &p) - e) / e).abs() < 1e-14,
            "the regularised energy is not the Cartesian"
        );
        let (_, pv) = system.phys(&q, &p);
        let vel = system.q_dot(&pv);
        let w = |i: usize| [back.p[i][0] / back.m[i], back.p[i][1] / back.m[i]];
        for (i, (j, k)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            let rel = [w(j)[0] - w(k)[0], w(j)[1] - w(k)[1]];
            let dv = (vel[i][0] - rel[0]).hypot(vel[i][1] - rel[1]) / rel[0].hypot(rel[1]);
            assert!(dv < 1e-14, "Eq. 7's q̇{i} misses vⱼ − vₖ by {dv:.3e}");
        }
    }
}

#[test]
fn heggie_march_round_trip() {
    check_round_trip(false);
}

negative_control!(
    heggie_march_round_trip,
    "Q₀ quarter-turned, which maps q₀ to −q₀",
    expected = "the round trip misses the state",
    check_round_trip(true)
);

/// `Γ*` against `(E − h)·R₀R₁R₂` at a state a boundary into the march and an `h` off the energy, `E` the Cartesian
/// energy of the image (an anchor independent of the kernel's Eq. 6), within `1e-12` of the largest term; `swap` swaps
/// `μ₀` and `μ₁` in the system first, the control's.
fn check_gamma_anchor(swap: bool) {
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let out = march::<Rk4, f64, Quiet, ExplicitTest>(
            &triple(),
            1,
            &params(time),
            &mut Quiet,
            &mut ExplicitTest,
        );
        let mut system = HgSystem {
            h: out.system.h + 0.3,
            ..out.system
        };
        if swap {
            system.mu = [system.mu[1], system.mu[0], system.mu[2]];
        }
        let r: Vec<f64> = out.q.q.iter().map(|x| x[0] * x[0] + x[1] * x[1]).collect();
        let product = r[0] * r[1] * r[2];
        let e = energy(&system.image(&out.q, &out.p));
        let want = (e - system.h) * product;
        let got = system.gamma(&out.q, &out.p);
        assert!(
            (got - want).abs() <= 1e-12 * (system.h * product).abs(),
            "Γ* {got:.6e} is not (E − h)·R₀R₁R₂ {want:.6e}"
        );
    }
}

#[test]
fn heggie_march_gamma_anchor() {
    check_gamma_anchor(false);
}

negative_control!(
    heggie_march_gamma_anchor,
    "μ₀ and μ₁ swapped, a mispaired reduced mass",
    expected = "is not (E − h)·R₀R₁R₂",
    check_gamma_anchor(true)
);

/// The rates of Heggie under each time transformation against a central difference of its Hamiltonian over 32 random
/// states at step `1e-6`, within `1e-8`, REQ-VAL-171's proposed step and tolerance (`validation::heggie`'s check;
/// REQ-VAL-170's Heggie arm); `flip` flips a sign.
fn check_rates(flip: bool) {
    use validation::heggie::{discrepancy, FdOccupant};
    for o in [
        FdOccupant::HeggieEq20,
        FdOccupant::HeggieEq22,
        FdOccupant::AarsethZare,
    ] {
        let d = discrepancy(o, 1e-6, 32, 7, flip);
        println!("heggie_march_rates_fd: {o:?}: discrepancy {d:.3e}");
        assert!(
            d < 1e-8,
            "{o:?}'s rates miss the finite difference by {d:.3e}"
        );
    }
}

#[test]
fn heggie_march_rates_fd() {
    check_rates(false);
}

negative_control!(
    heggie_march_rates_fd,
    "a momentum rate's sign flipped",
    expected = "rates miss the finite difference",
    check_rates(true)
);

// ── heggie_march: registration once, the landings, the ends ────────────────────────────────────────────────────

/// The march to boundary 4 from `s0` as `run` gives it: registered once (`h` bitwise the registration's), landing on
/// boundary 4 at `4·Δ_sync` exactly, still marching, its regularised energy held within `1e-8`.
fn check_once(out: &HgAdvanced<f64>, s0: &State<f64>) {
    let (registered, _, _) = register(s0, 1e-300, DEFAULT_TIME);
    assert_eq!(
        out.system.h.to_bits(),
        registered.h.to_bits(),
        "h changed: the march re-registered"
    );
    assert_eq!(
        (out.boundary, out.t, out.count.end),
        (4, 0.5, End::Marching),
        "the march did not land on 4"
    );
    let drift = ((out.regularised() - out.system.h) / out.system.h).abs();
    assert!(drift < 1e-8, "the regularised energy drifted {drift:.3e}");
}

#[test]
fn heggie_march_registered_once() {
    let s0 = triple::<f64>();
    let out = march::<Rk4, f64, Quiet, ExplicitTest>(
        &s0,
        4,
        &params(DEFAULT_TIME),
        &mut Quiet,
        &mut ExplicitTest,
    );
    check_once(&out, &s0);
    let mid = march::<Rk4, f64, Quiet, ExplicitTest>(
        &s0,
        2,
        &params(DEFAULT_TIME),
        &mut Quiet,
        &mut ExplicitTest,
    );
    assert_eq!((mid.boundary, mid.t), (2, 0.25), "boundary 2 at 2·Δ_sync");
}

negative_control!(
    heggie_march_registered_once,
    "a march re-registered at boundary 2, its second half from the Cartesian state there",
    expected = "h changed",
    {
        let s0 = triple::<f64>();
        let p = params(DEFAULT_TIME);
        let half =
            march::<Rk4, f64, Quiet, ExplicitTest>(&s0, 2, &p, &mut Quiet, &mut ExplicitTest);
        let mut out = march::<Rk4, f64, Quiet, ExplicitTest>(
            &half.state,
            2,
            &p,
            &mut Quiet,
            &mut ExplicitTest,
        );
        out.boundary = 4;
        out.t = 0.5;
        check_once(&out, &s0)
    }
);

/// A watch that reports every state finite: the control's, the explicit test removed.
struct NoTest;

impl<R> HgWatch<R> for NoTest {
    fn finite(&mut self, _q: &HgQ<R>, _p: &HgP<R>) -> bool {
        true
    }
}

/// The march of a state whose second body has no mass, so its first step is not finite, with the watch `X` and a budget
/// of 5 steps.
fn massless<X: HgWatch<f64>>(watch: &mut X) -> HgAdvanced<f64> {
    let mut s = triple::<f64>();
    s.m[1] = 0.0;
    let mut p = params::<f64>(DEFAULT_TIME);
    p.n_min = 5;
    p.step_budget_rate = 0.0;
    march::<Rk4, f64, Quiet, X>(&s, 1, &p, &mut Quiet, watch)
}

/// The march ended at the explicit finiteness test, its state latched finite, after `steps` steps.
fn check_non_finite(out: &HgAdvanced<f64>, steps: u32) {
    assert!(
        finite_state(&out.state),
        "a non-finite state was carried past the finiteness test"
    );
    assert_eq!(
        out.count.end,
        End::NonFinite,
        "the march did not end at the finiteness test"
    );
    assert_eq!(
        out.count.steps, steps,
        "the march took {} steps",
        out.count.steps
    );
}

#[test]
fn heggie_march_non_finite() {
    // Mid-interval: the first step is not finite, the state latched at the registration's image.
    let out = massless(&mut ExplicitTest);
    check_non_finite(&out, 1);
    assert_eq!(out.count.non_finite_entries, 0, "a finite entry counted");
    // At entry: two bodies coincide, so dt/dτ is 0 there and the march ends before any step.
    let mut s = triple::<f64>();
    s.r[1] = s.r[0];
    let out = march::<Rk4, f64, Quiet, ExplicitTest>(
        &s,
        1,
        &params(DEFAULT_TIME),
        &mut Quiet,
        &mut ExplicitTest,
    );
    check_non_finite(&out, 0);
    assert_eq!(
        (out.count.non_finite_entries, out.boundary),
        (1, 0),
        "the entry is counted, the boundary not reached"
    );
}

negative_control!(
    heggie_march_non_finite,
    "the march without its finiteness test, which carries the non-finite state on",
    expected = "a non-finite state was carried past the finiteness test",
    check_non_finite(&massless(&mut NoTest), 1)
);

/// The march from [`triple`] with a budget of `n_min` steps at rate 0: spent after `n_min + 1` steps, ended
/// [`End::Exhausted`], its state latched short of the boundary.
fn check_exhausted(n_min: u32) {
    let mut p = params::<f64>(DEFAULT_TIME);
    p.n_min = n_min;
    p.step_budget_rate = 0.0;
    let out =
        march::<Rk4, f64, Quiet, ExplicitTest>(&triple(), 1, &p, &mut Quiet, &mut ExplicitTest);
    assert_eq!(out.count.end, End::Exhausted, "the budget was not spent");
    assert_eq!(
        (out.count.steps, out.count.accepted, out.boundary),
        (4, 4, 0),
        "the budget's steps"
    );
    assert!(
        out.t > 0.0 && out.t < 0.125,
        "the time short of the boundary: {}",
        out.t
    );
}

#[test]
fn heggie_march_exhausted() {
    check_exhausted(3);
}

negative_control!(
    heggie_march_exhausted,
    "a budget that does not bind",
    expected = "the budget was not spent",
    check_exhausted(1_000_000)
);

// ── heggie_march: the step's size ──────────────────────────────────────────────────────────────────────────────

/// A state with `Rᵢ = Qᵢₓ²` set by `q = [Q₀ₓ, Q₁ₓ, Q₂ₓ]` on the x axis, unit masses, `h = −1`, the momenta
/// `P = [(0.5, 0), 0, 0]`: at `Qᵢ = (1, 0)` the relative momenta are `(0.25, 0), 0, 0` and Eq. 7's velocities
/// `(0.5, 0), (−0.25, 0), (−0.25, 0)`, so `v_max = 0.5` exactly.
fn at_r(q: [f64; 3], time: HeggieTime, tiny: f64) -> (HgSystem<f64>, HgQ<f64>, HgP<f64>) {
    (
        HgSystem::new([1.0; 3], -1.0, tiny, time),
        HgQ {
            q: q.map(|x| [x, 0.0]),
            t: 0.0,
        },
        HgP {
            p: [[0.5, 0.0], [0.0; 2], [0.0; 2]],
        },
    )
}

/// [`params`] at the system's floor and time.
fn params_at(s: &HgSystem<f64>) -> HgParams<f64> {
    HgParams {
        tiny: s.tiny,
        ..params(s.time)
    }
}

/// The floor signal at `TINY = 2⁻¹⁰` against `signal`: on `dt/dτ < TINY` with every `Rᵢ ≥ TINY`, on `R₀ < TINY` alone,
/// off with `R₀` exactly `TINY` and `dt/dτ` above, off with every value above, under Eq. 20 and Eq. 22; and off at
/// `dt/dτ` exactly `TINY` under Eq. 20.
fn check_floor(signal: impl Fn(&HgSystem<f64>, &HgQ<f64>, &HgP<f64>) -> bool) {
    let tiny = 2f64.powi(-10);
    let edge = 2f64.powi(-5); // edge² = TINY.
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let cases = [
            (
                "dt/dτ below TINY, every Rᵢ above",
                [2.0 * edge, 0.25, 0.25],
                true,
            ),
            ("R₀ below TINY alone", [0.5 * edge, 16.0, 16.0], true),
            ("R₀ at TINY, dt/dτ above", [edge, 16.0, 16.0], false),
            ("every value above", [0.5, 1.0, 1.0], false),
        ];
        for (what, q, want) in cases {
            let (s, q, p) = at_r(q, time, tiny);
            assert_eq!(
                signal(&s, &q, &p),
                want,
                "the floor signal, {what}, under {time:?}"
            );
        }
    }
    let (s, q, p) = at_r([0.25, 0.25, 0.5], HeggieTime::Eq20, tiny);
    assert_eq!(s.dt_dtau(&q), tiny, "the case's dt/dτ");
    assert!(
        !signal(&s, &q, &p),
        "the floor signal, dt/dτ at TINY, under Eq20"
    );
}

#[test]
fn heggie_march_floor_signal() {
    check_floor(|s, q, p| step_size(s, q, p, 1.0, &params_at(s)).floored);
}

negative_control!(
    heggie_march_floor_signal,
    "the reference's signal, Rᵢ < TINY alone, which misses a floored dt/dτ",
    expected = "the floor signal, dt/dτ below TINY",
    check_floor(|_, q, _| q.q.iter().any(|x| x[0] * x[0] < 2f64.powi(-10)))
);

/// The step's size from [`at_r`] at `Qᵢ = (1, 0)`, `dt/dτ = 1`: the per-step size `c₀ = η·Δ/d` against the entry size
/// at, above and below it (the cap hit only below); the predictive limit `f·d_min/(v_max·dt/dτ) = 2f`, off at `f = 0`,
/// `f < 0` and a denominator of 0, cutting where it is smaller than `c₀` and not where it equals or exceeds it; the
/// clamp at the interval's end and past it, and with the clamp off.
fn check_step_size() {
    let (s, q, p) = at_r([1.0, 1.0, 1.0], HeggieTime::Eq20, 1e-300);
    let mut pr = params::<f64>(HeggieTime::Eq20);
    pr.predictive_f = 0.0;
    let c0 = pr.eta * pr.delta_sync;
    for (entry, h, cap) in [
        (c0, c0, false),
        (2.0 * c0, c0, false),
        (0.5 * c0, 0.5 * c0, true),
    ] {
        let z = step_size(&s, &q, &p, entry, &pr);
        assert_eq!(
            (z.h, z.cap_hit, z.by_limit, z.floored),
            (h, cap, false, false),
            "the size at entry {entry}"
        );
    }
    assert_eq!(
        hg::predictive(&s, &q, &p, 1.0),
        (2.0, true),
        "the limit at f = 1"
    );
    assert!(!hg::predictive(&s, &q, &p, 0.0).1, "the limit on at f = 0");
    assert!(!hg::predictive(&s, &q, &p, -1.0).1, "the limit on at f < 0");
    let still = HgP { p: [[0.0; 2]; 3] };
    assert!(
        !hg::predictive(&s, &q, &still, 1.0).1,
        "the limit on at a denominator of 0"
    );
    let far = HgQ {
        q: [[f64::INFINITY, 0.0], [1.0, 0.0], [1.0, 0.0]],
        t: 0.0,
    };
    assert!(
        !hg::predictive(&s, &far, &p, 1.0).1,
        "the limit on at a denominator not finite"
    );
    for (f, cut) in [(0.25 * c0, true), (0.5 * c0, false), (c0, false)] {
        pr.predictive_f = f;
        let z = step_size(&s, &q, &p, 1.0, &pr);
        assert_eq!(z.by_limit, cut, "the limit's cut at f = {f}");
        assert_eq!(z.h, if cut { 2.0 * f } else { c0 }, "the size at f = {f}");
    }
    pr.predictive_f = 0.0;
    let near = HgQ {
        t: pr.delta_sync - 1e-4,
        ..q
    };
    assert_eq!(
        step_size(&s, &near, &p, 1.0, &pr).h,
        pr.delta_sync - near.t,
        "the clamp"
    );
    let past = HgQ {
        t: 2.0 * pr.delta_sync,
        ..q
    };
    assert_eq!(
        step_size(&s, &past, &p, 1.0, &pr).h,
        0.0,
        "the clamp past the boundary"
    );
    pr.clamp = false;
    assert_eq!(
        step_size(&s, &near, &p, 1.0, &pr).h,
        c0,
        "the size with no clamp"
    );
}

#[test]
fn heggie_march_step_size() {
    check_step_size();
}

negative_control!(
    heggie_march_step_size,
    "the step's size read at a state whose dt/dτ is 2, the per-step size halved",
    expected = "the size at entry",
    {
        let (s, q, p) = at_r([1.0, 2f64.sqrt(), 1.0], HeggieTime::Eq20, 1e-300);
        let pr = HgParams {
            predictive_f: 0.0,
            ..params::<f64>(HeggieTime::Eq20)
        };
        let c0 = pr.eta * pr.delta_sync;
        let z = step_size(&s, &q, &p, c0, &pr);
        assert_eq!(
            (z.h, z.cap_hit, z.by_limit, z.floored),
            (c0, false, false, false),
            "the size at entry {c0}"
        )
    }
);

/// `dt/dτ`: `R₀R₁R₂` under Eq. 20, `R₀R₁R₂/S^{3/2}` under Eq. 22, `S` floored at `TINY` below it.
fn check_dt_dtau(time22: HeggieTime) {
    let (s20, q, _) = at_r([1.0, 2.0, 0.5], HeggieTime::Eq20, 1e-300);
    let (s22, _, _) = at_r([1.0, 2.0, 0.5], time22, 1e-300);
    let sum: f64 = 1.0 + 4.0 + 0.25;
    assert_eq!(s20.dt_dtau(&q), 1.0, "dt/dτ under Eq. 20");
    assert_eq!(
        s22.dt_dtau(&q),
        1.0 / (sum * sum.sqrt()),
        "dt/dτ under Eq. 22"
    );
    let (floor, low, _) = at_r([0.1, 0.1, 0.1], HeggieTime::Eq22, 0.25);
    let r = 0.1 * 0.1;
    assert_eq!(
        floor.dt_dtau(&low),
        r * r * r / (0.25 * 0.5),
        "S below TINY floored to it"
    );
}

#[test]
fn heggie_march_dt_dtau() {
    check_dt_dtau(HeggieTime::Eq22);
}

negative_control!(
    heggie_march_dt_dtau,
    "Eq. 20's dt/dτ read for Eq. 22's",
    expected = "dt/dτ under Eq. 22",
    check_dt_dtau(HeggieTime::Eq20)
);

/// The explicit finiteness test refuses a NaN in each of the thirteen components and passes a finite state; `skip`
/// leaves component `skip` finite, the control's.
fn check_explicit(skip: Option<usize>) {
    let (_, q, p) = register(&triple::<f64>(), 1e-300, DEFAULT_TIME);
    assert!(
        HgWatch::finite(&mut ExplicitTest, &q, &p),
        "a finite state refused"
    );
    for i in 0..13 {
        let (mut q1, mut p1) = (q, p);
        let nan = if Some(i) == skip { 0.5 } else { f64::NAN };
        match i {
            0..=5 => q1.q[i / 2][i % 2] = nan,
            6 => q1.t = nan,
            _ => p1.p[(i - 7) / 2][(i - 7) % 2] = nan,
        }
        assert!(
            !HgWatch::finite(&mut ExplicitTest, &q1, &p1),
            "a NaN in component {i} passed"
        );
    }
}

#[test]
fn heggie_march_explicit_test() {
    check_explicit(None);
}

negative_control!(
    heggie_march_explicit_test,
    "the clock's component left finite",
    expected = "a NaN in component 6 passed",
    check_explicit(Some(6))
);

/// A stepper whose step advances only the clock, by the physical step `h·dt/dτ` times a scripted factor (1 once the
/// script runs out).
struct Scripted;

thread_local! {
    /// The factors [`Scripted`] reads, one per step (each test thread its own).
    static FACTORS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

impl HgStepper for Scripted {
    fn pair_profile() -> CapabilityProfile {
        Form::Rk4.pair_profile()
    }

    fn regularised_step<R: AzReal>(
        system: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        h: R,
    ) -> (HgQ<R>, HgP<R>) {
        let f = FACTORS.with_borrow_mut(|f| if f.is_empty() { 1.0 } else { f.remove(0) });
        (
            HgQ {
                t: q.t + h * system.dt_dtau(q) * v::<R>(f),
                ..*q
            },
            *p,
        )
    }
}

/// One interval's march of [`Scripted`] reading `factors`, from an equilateral triangle of unit masses at rest, with
/// `η = 1` and `Δ_sync = d/8`, `d` the registration's `dt/dτ` under Eq. 20, so that the first step's size is `1/8`
/// exactly and the clock's sizes are exact multiples of `Δ_sync`; no predictive limit, the secant's `tries` and the
/// clamp `clamp`. Its counts and its `Δ_sync`.
fn scripted(factors: &[f64], tries: u32, clamp: bool) -> (HgCount<f64>, f64) {
    FACTORS.with_borrow_mut(|f| *f = factors.to_vec());
    let y = 3f64.sqrt() / 6.0;
    let s = State {
        m: [1.0; 3],
        r: [[0.0, 2.0 * y], [-0.5, -y], [0.5, -y]],
        p: [[0.0; 2]; 3],
    };
    let (system, q, _) = register(&s, 1e-300, HeggieTime::Eq20);
    let mut pr = params::<f64>(HeggieTime::Eq20);
    pr.eta = 1.0;
    pr.delta_sync = system.dt_dtau(&q) / 8.0;
    pr.predictive_f = 0.0;
    pr.secant_tries = tries;
    pr.clamp = clamp;
    pr.n_min = 20;
    pr.step_budget_rate = 0.0;
    let out =
        march::<Scripted, f64, Quiet, ExplicitTest>(&s, 1, &pr, &mut Quiet, &mut ExplicitTest);
    FACTORS.with_borrow_mut(|f| f.clear());
    (out.count, pr.delta_sync)
}

/// The landing secant, its stops and the overshoot's edge; `tries` the secant's (the control turns it off).
fn check_secant(tries: u32) {
    let (c, _) = scripted(&[0.5, 1.0], tries, true);
    assert_eq!(
        (c.steps, c.end),
        (2, End::Marching),
        "short by half, then a landing"
    );
    let (c, _) = scripted(&[1.1, 1.1], tries, true);
    assert_eq!(
        (c.steps, c.end),
        (2, End::Marching),
        "10% over, one retake that lands"
    );
    let (c, _) = scripted(&[1.1, 0.0], tries, true);
    assert_eq!(
        (c.steps, c.end),
        (3, End::Marching),
        "a retake whose clock stands still stops the secant"
    );
    let (c, _) = scripted(&[1.1], 0, true);
    assert_eq!(c.steps, 1, "the secant off takes no retake");
    let (c, _) = scripted(&[2.5], 0, true);
    assert_eq!(c.overshoots, 1, "past twice the interval is an overshoot");
    let (c, _) = scripted(&[2.5], 0, false);
    assert_eq!(c.overshoots, 0, "an overshoot counted with the clamp off");
    let (c, delta) = scripted(&[2.0], 0, true);
    assert_eq!(c.overshoots, 0, "twice the interval is not past it");
    assert_eq!(
        c.dt_max.report(),
        DtMaxReport::Largest {
            dt_max: 2.0 * delta,
            skipped: 0
        },
        "dt_max the physical step"
    );
}

#[test]
fn heggie_march_landing_secant() {
    check_secant(4);
}

negative_control!(
    heggie_march_landing_secant,
    "the secant off",
    expected = "10% over, one retake that lands",
    check_secant(0)
);

// ── regularised_step_reversibility ─────────────────────────────────────────────────────────────────────────────

/// A precision test 3 runs at.
trait P: AzReal + Into<f64> {
    const EPS: f64;
    /// The domain's lower edge (REQ-VAL-191).
    fn edge() -> f64;
}

impl P for f64 {
    const EPS: f64 = f64::EPSILON;
    fn edge() -> f64 {
        STEP_REVERSAL_EDGE_F64.number()
    }
}

impl P for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn edge() -> f64 {
        STEP_REVERSAL_EDGE_F32.number()
    }
}

/// A flow that records the pair separations and pair masses at every evaluation, for the step's `τ`.
struct Recording<'a, F: Flow<f64>> {
    flow: &'a F,
    pairs: fn(&F, &<F as Flow<f64>>::Q) -> [(f64, f64); 3],
    taus: RefCell<Vec<f64>>,
}

impl<F: Flow<f64>> Recording<'_, F> {
    fn record(&self, q: &F::Q) {
        for (d, m) in (self.pairs)(self.flow, q) {
            self.taus.borrow_mut().push((d * d * d / m).sqrt());
        }
    }
}

impl<F: Flow<f64>> Flow<f64> for Recording<'_, F> {
    type Q = F::Q;
    type P = F::P;
    fn q_rate(&self, q: &F::Q, p: &F::P) -> F::Q {
        self.record(q);
        self.flow.q_rate(q, p)
    }
    fn p_rate(&self, q: &F::Q, p: &F::P) -> F::P {
        self.record(q);
        self.flow.p_rate(q, p)
    }
    fn rates(&self, q: &F::Q, p: &F::P) -> (F::Q, F::P) {
        self.record(q);
        self.flow.rates(q, p)
    }
    fn q_add(q: &F::Q, h: f64, dq: &F::Q) -> F::Q {
        F::q_add(q, h, dq)
    }
    fn p_add(p: &F::P, h: f64, dp: &F::P) -> F::P {
        F::p_add(p, h, dp)
    }
    fn q_weighted(two: f64, k: [&F::Q; 4]) -> F::Q {
        F::q_weighted(two, k)
    }
    fn p_weighted(two: f64, k: [&F::P; 4]) -> F::P {
        F::p_weighted(two, k)
    }
}

/// Heggie's three pairs: `Rᵢ = |qᵢ|` and `mⱼ + mₖ`.
fn hg_pairs(s: &HgSystem<f64>, q: &HgQ<f64>) -> [(f64, f64); 3] {
    let r = |i: usize| q.q[i][0] * q.q[i][0] + q.q[i][1] * q.q[i][1];
    let [m0, m1, m2] = s.m;
    [(r(0), m1 + m2), (r(1), m2 + m0), (r(2), m0 + m1)]
}

/// Aarseth–Zare's three pairs: `A`, `B`, `|R₃|` and their masses.
fn az_pairs(s: &AzSystem<f64>, q: &AzQ<f64>) -> [(f64, f64); 3] {
    let (a, b) = AzSystem::ab(q);
    let (r1, r2) = (az::lc(q.u1), az::lc(q.u2));
    let r3 = (r2[0] - r1[0]).hypot(r2[1] - r1[1]);
    let [ma, mb, mc] = s.mabc;
    [(a, ma + mb), (b, ma + mc), (r3, mb + mc)]
}

/// One measurement: the occupant, `dt/τ` (the step's `τ`), `dt`, the residual in `ε`, and the verdict.
#[derive(Clone, Debug)]
struct Measured {
    name: &'static str,
    ratio: f64,
    dt: f64,
    residual: f64,
    verdict: bool,
}

/// The test's target `dt/τ` at `R`, `τ` the start's: `0.1` at f64 and `0.6` at f32, inside each domain (the measured
/// ratio, with the step's `τ`, is checked to be).
fn target<R: P>() -> f64 {
    if R::EPS == f64::EPSILON {
        0.1
    } else {
        0.6
    }
}

/// `x` relative distance between two flat vectors over `y`'s norm.
fn rel(x: &[f64], y: &[f64]) -> f64 {
    let n = y.iter().map(|a| a * a).sum::<f64>().sqrt();
    x.iter()
        .zip(y)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
        / n
}

/// Test 3 on Heggie × RK4 under `time` at `R`, from [`triple`]: a fictitious step `h` sized so the physical step reads
/// `dt/τ` near [`target`], `τ` the step's at f64; `STEP(h)`, `Θ`, `STEP(h)`, `Θ`, the residual on the full state.
fn measure_heggie<R: P>(name: &'static str, time: HeggieTime) -> Measured {
    let flat = |q: &HgQ<R>, p: &HgP<R>| -> Vec<f64> {
        q.q.iter()
            .flatten()
            .chain(std::iter::once(&q.t))
            .chain(p.p.iter().flatten())
            .map(|&x| x.into())
            .collect()
    };
    let (s64, q64, p64) = register(&triple::<f64>(), 1e-300, time);
    let tau0 = hg_pairs(&s64, &q64)
        .iter()
        .map(|&(d, m)| (d * d * d / m).sqrt())
        .fold(f64::INFINITY, f64::min);
    let h64 = target::<R>() * tau0 / s64.dt_dtau(&q64);
    let rec = Recording {
        flow: &s64,
        pairs: hg_pairs,
        taus: RefCell::new(Vec::new()),
    };
    let (q1, p1) = Form::Rk4.step(&rec, &q64, &p64, h64);
    let (qr, pr) = reversed(&q1, &p1);
    let _ = Form::Rk4.step(&rec, &qr, &pr, h64);
    let tau = rec
        .taus
        .borrow()
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let dt = q1.t;
    let (s, q, p) = register(&triple::<R>(), R::at_width(1e-37, 1e-300), time);
    let h: R = v(h64);
    let (a, b) = <Rk4 as HgStepper>::regularised_step(&s, &q, &p, h);
    let (a, b) = reversed(&a, &b);
    let (a, b) = <Rk4 as HgStepper>::regularised_step(&s, &a, &b, h);
    let (a, b) = reversed(&a, &b);
    let residual = rel(&flat(&a, &b), &flat(&q, &p)) / R::EPS;
    Measured {
        name,
        ratio: dt / tau,
        dt,
        residual,
        verdict: residual <= STEP_REVERSAL_TOL.number(),
    }
}

/// Aarseth–Zare's time-reversal map (Part 2b § "The time-reversal maps (REQ-INT-095)"): `(u, p, t) → (u, −p, −t)`.
fn az_reversed<R: AzReal>(q: &AzQ<R>, p: &AzP<R>) -> (AzQ<R>, AzP<R>) {
    let neg = |x: [R; 2]| [-x[0], -x[1]];
    (
        AzQ { t: -q.t, ..*q },
        AzP {
            p1: neg(p.p1),
            p2: neg(p.p2),
        },
    )
}

/// Test 3 on Aarseth–Zare + RK4 at `R`, as [`measure_heggie`].
fn measure_az<R: P>(name: &'static str) -> Measured {
    let flat = |q: &AzQ<R>, p: &AzP<R>| -> Vec<f64> {
        [
            q.u1[0], q.u1[1], q.u2[0], q.u2[1], q.t, p.p1[0], p.p1[1], p.p2[0], p.p2[1],
        ]
        .map(Into::into)
        .to_vec()
    };
    let (s64, q64, p64) = az::register(&triple::<f64>(), 1e-300, 1e-300);
    let tau0 = az_pairs(&s64, &q64)
        .iter()
        .map(|&(d, m)| (d * d * d / m).sqrt())
        .fold(f64::INFINITY, f64::min);
    let (a0, b0) = AzSystem::ab(&q64);
    let h64 = target::<R>() * tau0 / (a0 * b0);
    let rec = Recording {
        flow: &s64,
        pairs: az_pairs,
        taus: RefCell::new(Vec::new()),
    };
    let (q1, p1) = Form::Rk4.step(&rec, &q64, &p64, h64);
    let (qr, pr) = az_reversed(&q1, &p1);
    let _ = Form::Rk4.step(&rec, &qr, &pr, h64);
    let tau = rec
        .taus
        .borrow()
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let dt = q1.t;
    let (s, q, p) = az::register(
        &triple::<R>(),
        R::at_width(1e-37, 1e-300),
        R::at_width(0.0, 1e-300),
    );
    let h: R = v(h64);
    let (a, b) = <Rk4 as AzStepper>::regularised_step(&s, &q, &p, h);
    let (a, b) = az_reversed(&a, &b);
    let (a, b) = <Rk4 as AzStepper>::regularised_step(&s, &a, &b, h);
    let (a, b) = az_reversed(&a, &b);
    let residual = rel(&flat(&a, &b), &flat(&q, &p)) / R::EPS;
    Measured {
        name,
        ratio: dt / tau,
        dt,
        residual,
        verdict: residual <= STEP_REVERSAL_TOL.number(),
    }
}

/// The built regularised occupants and their profiles' `step_reversible`.
fn built() -> Vec<(&'static str, bool)> {
    vec![
        (
            "Heggie × RK4 (Eq. 20)",
            hg::profile::<Rk4>().step_reversible,
        ),
        (
            "Heggie × RK4 (Eq. 22)",
            hg::profile::<Rk4>().step_reversible,
        ),
        (
            "Aarseth–Zare + RK4",
            <Rk4 as AzStepper>::pair_profile().step_reversible,
        ),
    ]
}

/// Every built occupant is measured, inside the criterion's domain, and its profile's `step_reversible` is its
/// verdict; a verdict other than R-500's listed one (Aarseth–Zare + RK4 false) is printed for the review queue.
fn check_measured(measured: &[Measured], profiles: &[(&'static str, bool)], edge: f64) {
    for &(name, flag) in profiles {
        let m = measured
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{name} is missing from the measurement"));
        assert!(
            edge <= m.ratio && m.ratio <= STEP_REVERSAL_EDGE_MAX.number(),
            "{name}: dt/τ {:.3e} is outside the criterion's domain",
            m.ratio
        );
        assert_eq!(
            flag, m.verdict,
            "{name}: its profile's step_reversible differs from its measured verdict"
        );
        if name == "Aarseth–Zare + RK4" && m.verdict {
            println!("REVIEW QUEUE: Aarseth–Zare + RK4 measured step-reversible, against R-500's listed false");
        }
    }
}

/// The Cartesian control at `dt` from [`triple`]: test 3's residual in `ε` of `step`, momentum negation for the map.
fn cartesian<R: P>(step: impl Fn(&State<R>, R, &Geom<R>) -> (State<R>, Geom<R>), dt: f64) -> f64 {
    let s = triple::<R>();
    let neg = |s: &State<R>| State {
        p: s.p.map(|x| [-x[0], -x[1]]),
        ..*s
    };
    let flat = |s: &State<R>| -> Vec<f64> {
        s.r.iter()
            .chain(s.p.iter())
            .flatten()
            .map(|&x| x.into())
            .collect()
    };
    let dt: R = v(dt);
    let (a, _) = step(&s, dt, &Geom::new(&s.m, &s.r));
    let a = neg(&a);
    let (b, _) = step(&a, dt, &Geom::new(&a.m, &a.r));
    rel(&flat(&neg(&b)), &flat(&s)) / R::EPS
}

/// REQ-VAL-032's asymmetric Yoshida-4: the KDK steps at `w₁, w₁, w₀`.
fn asymmetric<R: P>(s: &State<R>, dt: R, g: &Geom<R>) -> (State<R>, Geom<R>) {
    let (s, g) = Kdk::step(s, R::YOSHIDA4_W1 * dt, g);
    let (s, g) = Kdk::step(&s, R::YOSHIDA4_W1 * dt, &g);
    Kdk::step(&s, R::YOSHIDA4_W0 * dt, &g)
}

/// The measurements at `R`, printed, with the Cartesian controls at each one's `dt`.
fn measure_all<R: P>() -> Vec<Measured> {
    let m = vec![
        measure_heggie::<R>("Heggie × RK4 (Eq. 20)", HeggieTime::Eq20),
        measure_heggie::<R>("Heggie × RK4 (Eq. 22)", HeggieTime::Eq22),
        measure_az::<R>("Aarseth–Zare + RK4"),
    ];
    let tol = STEP_REVERSAL_TOL.number();
    for x in &m {
        let rk4 = cartesian::<R>(Rk4::step, x.dt);
        let asym = cartesian::<R>(asymmetric, x.dt);
        println!(
            "regularised_step_reversibility at {}: {}: dt {:.4e} (dt/τ {:.3e}), residual {:.3e} ε, {}; at that dt \
             Cartesian RK4 {rk4:.3e} ε, the asymmetric Yoshida-4 {asym:.3e} ε (criterion {tol} ε)",
            std::any::type_name::<R>(),
            x.name,
            x.dt,
            x.ratio,
            x.residual,
            if x.verdict { "reversible" } else { "not reversible" }
        );
        assert!(
            rk4 > tol,
            "Cartesian RK4 reads reversible at {}'s dt",
            x.name
        );
        assert!(
            asym > tol,
            "the asymmetric Yoshida-4 reads reversible at {}'s dt",
            x.name
        );
    }
    m
}

#[test]
fn regularised_step_reversibility() {
    check_measured(&measure_all::<f64>(), &built(), f64::edge());
    check_measured(&measure_all::<f32>(), &built(), f32::edge());
}

negative_control!(
    regularised_step_reversibility_flag,
    "a profile with Heggie × RK4 (Eq. 22)'s step_reversible flipped",
    expected = "its profile's step_reversible differs from its measured verdict",
    {
        let mut profiles = built();
        profiles[1].1 = !profiles[1].1;
        check_measured(&measure_all::<f64>(), &profiles, f64::edge())
    }
);

negative_control!(
    regularised_step_reversibility_missing,
    "a measurement set with Aarseth–Zare + RK4 left out",
    expected = "is missing from the measurement",
    {
        let mut m = measure_all::<f64>();
        m.pop();
        check_measured(&m, &built(), f64::edge())
    }
);
