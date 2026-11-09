//! QA tests for TASK-M3-05, from the requirements and integrator_contract Part 1, dd_integrator §3.4, §3.5, §5, not from
//! the implementation. Every energy and `L_z` here is written out in this file from dd §3.5's formula; the kernel's
//! `physics::energy` is never the reference.
//! - REQ-INT-010: a worked projection in exact dyadics, and `R_com`, `P_com` zero to rounding after every `STEP` of a
//!   whole run, each occupant at f32 and f64 (`qa_com_projection_*`).
//! - REQ-INT-011: the projection changes `E` by exactly `−|P|²/2M` and `L_z` by exactly `−R × P`, the removed modes'
//!   own terms, on states carrying a large drift, so nothing restores either (`qa_projection_restores_nothing`).
//! - REQ-INT-012: `E₀`, `L_{z,0}` by hand before the loop; the accumulation is post-projection, at every substep,
//!   signed final, unsigned max; the relative forms' floors (`qa_invariant_monitor_*`).
//! - REQ-INT-040: the predicates' relative energy and absolute `L_z`, read from the latches alone, and the `symplectic`
//!   flag carried (`qa_drift_suspect_read_time_*`); Euler lighting on bound states with momentum, the domain RQ-626
//!   names (`qa_drift_suspect_euler_on_bound_states`).
//!
//! **Domain** (pitfalls §10): `qa_com_projection_every_step` holds over the first steps of dd test 7's fixture; the
//! rounding bounds are multiples of the unit roundoff at the state's own scale.

use kernel::decode::{self, Ic, Latent};
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::monitor::{relative, Monitor};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::project::ComProjection;
use kernel::driver::wrapper::{integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State, Stepper};
use kernel::payload::{
    suspect_energy, suspect_lz, DriftSuspect, SimState, SUSPECT_ENERGY_THRESHOLD,
    SUSPECT_LZ_THRESHOLD,
};
use validation::decode_f32_eps::SplitMix;
use validation::drift::{close_encounter, figure_eight, ic_at, integrate_fixture, AsR, Fixture};
use validation::negative_control;

trait P: WrapperReal + Into<f64> + std::fmt::Debug {
    const U: f64;
}
impl P for f32 {
    const U: f64 = f32::EPSILON as f64 / 2.0;
}
impl P for f64 {
    const U: f64 = f64::EPSILON / 2.0;
}

type V = [[f64; 2]; 3];

/// dd §3.5's `E = Σ ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/rᵢⱼ`, written out.
fn hand_energy(m: &[f64; 3], r: &V, p: &V) -> f64 {
    let kin: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let pot: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let (dx, dy) = (r[i][0] - r[j][0], r[i][1] - r[j][1]);
            m[i] * m[j] / (dx * dx + dy * dy).sqrt()
        })
        .sum();
    kin - pot
}

/// dd §3.5's `L_z = Σ (xᵢ p_{y,i} − yᵢ p_{x,i})`, written out.
fn hand_lz(r: &V, p: &V) -> f64 {
    (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum()
}

/// `(R_com, P_com)`, written out from the requirement's statement.
fn hand_com(m: &[f64; 3], r: &V, p: &V) -> ([f64; 2], [f64; 2]) {
    let total: f64 = m.iter().sum();
    let mean = |k: usize| (0..3).map(|i| m[i] * r[i][k]).sum::<f64>() / total;
    let sum = |k: usize| (0..3).map(|i| p[i][k]).sum::<f64>();
    ([mean(0), mean(1)], [sum(0), sum(1)])
}

fn wide<R: P>(v: &[[R; 2]; 3]) -> V {
    v.map(|a| a.map(Into::into))
}

fn wide_m<R: P>(m: &[R; 3]) -> [f64; 3] {
    m.map(Into::into)
}

/// Worked in exact dyadics: `m = (1, 2, 5)`, `M = 8`.
const M: [f64; 3] = [1.0, 2.0, 5.0];
const R0: V = [[1.0, 0.0], [0.0, 1.0], [3.0, -1.0]];
const P0: V = [[1.0, 2.0], [3.0, -4.0], [4.0, 2.0]];

/// `S = (16, −3)`, so `R_com = (2, −3/8)`; `P_com = (8, 0)`, so `pᵢ − (mᵢ/8)·(8, 0) = pᵢ − (mᵢ, 0)`.
const R1: V = [[-1.0, 0.375], [-2.0, 1.375], [1.0, -0.625]];
const P1: V = [[0.0, 2.0], [1.0, -4.0], [-1.0, 2.0]];

fn check_worked(project: fn(&[f64; 3], &V, &V) -> (V, V)) {
    let (r, p) = project(&M, &R0, &P0);
    assert_eq!((r, p), (R1, P1), "the projection is not the worked one");
}

fn the_projection(m: &[f64; 3], r: &V, p: &V) -> (V, V) {
    ComProjection::new(m).project(r, p)
}

#[test]
fn qa_com_projection_worked() {
    check_worked(the_projection);
    let f32_case = ComProjection::new(&M.map(|x| x as f32));
    let (r, p) = f32_case.project(
        &R0.map(|v| v.map(|x| x as f32)),
        &P0.map(|v| v.map(|x| x as f32)),
    );
    assert_eq!(
        (wide(&r), wide(&p)),
        (R1, P1),
        "f32: the projection is not the worked one"
    );
    assert_eq!(ComProjection::new(&M).total_mass(), 8.0);
}

negative_control!(
    qa_com_projection_worked,
    "a momentum projection that subtracts P/3, equal weights, not mᵢ/M",
    expected = "not the worked one",
    check_worked(|m, r, p| {
        let (rr, _) = the_projection(m, r, p);
        let q = hand_com(m, r, p).1;
        (
            rr,
            [0, 1, 2].map(|i| [p[i][0] - q[0] / 3.0, p[i][1] - q[1] / 3.0]),
        )
    })
);

/// The velocity form, worked: `V_com = Σ mᵢvᵢ/M`, the masses `(1, 2, 5)`, the velocities `R0`.
fn check_velocity(project: fn(&V) -> V) {
    assert_eq!(project(&R0), R1, "vᵢ − V_com is not the worked one");
}

#[test]
fn qa_com_projection_velocity_worked() {
    check_velocity(|v| ComProjection::new(&M).project_velocities(v));
}

negative_control!(
    qa_com_projection_velocity_worked,
    "an unweighted mean is not V_com",
    expected = "is not the worked one",
    check_velocity(|v| ComProjection::new(&[1.0, 1.0, 1.0]).project_velocities(v))
);

/// A cadence around the wrapper's own, checking after every internal step: `R_com`, `P_com` zero to rounding, and the
/// monitor's drifts equal to the ones written out here on the state the step leaves, with its own latches.
struct Audited<R> {
    inner: WrapperCadence<R>,
    e_0: f64,
    lz_0: f64,
    de_max: f64,
    dlz_max: f64,
    steps: u32,
}

impl<R: P> Audited<R> {
    fn new(ic: &Ic<R>) -> Self {
        let (m, r, p) = (wide_m(&ic.m), wide(&ic.r), wide(&ic.p));
        Audited {
            inner: WrapperCadence::new(&ic.m, &ic.r, &ic.p),
            e_0: hand_energy(&m, &r, &p),
            lz_0: hand_lz(&r, &p),
            de_max: 0.0,
            dlz_max: 0.0,
            steps: 0,
        }
    }
}

impl<R: P> Cadence<R> for Audited<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.steps += 1;
        let (m, r, p) = (wide_m(&state.m), wide(&state.r), wide(&state.p));
        let (c, q) = hand_com(&m, &r, &p);
        let reach = |v: &V| v.iter().flatten().fold(0f64, |a, x| a.max(x.abs()));
        let (rs, ps) = (reach(&r), reach(&p));
        // The sums' rounding: a few units at each term's scale; the projected state is formed at `R`.
        let bound = 16.0 * R::U;
        assert!(
            c[0].abs().max(c[1].abs()) <= bound * rs,
            "step {}: R_com {c:?} is not zero to rounding at scale {rs:e}",
            self.steps
        );
        assert!(
            q[0].abs().max(q[1].abs()) <= bound * ps,
            "step {}: P_com {q:?} is not zero to rounding at scale {ps:e}",
            self.steps
        );
        let (de, dlz) = (
            hand_energy(&m, &r, &p) - self.e_0,
            hand_lz(&r, &p) - self.lz_0,
        );
        self.de_max = self.de_max.max(de.abs());
        self.dlz_max = self.dlz_max.max(dlz.abs());
    }
    fn done(&self) -> bool {
        false
    }
}

fn key<R: P>(dt: f64, steps: u32) -> WrapperUniforms<R>
where
    f64: AsR<R>,
{
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: dt.to_r(),
        horizon_steps: steps,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default sim key generates"),
    }
}

/// `ic` with a bulk offset and momentum, so the first projection has something to remove.
fn off_com<R: P>(fixture: &Fixture) -> Ic<R>
where
    f64: AsR<R>,
{
    let mut ic = fixture.ic;
    for i in 0..3 {
        ic.r[i] = [ic.r[i][0] + 0.75, ic.r[i][1] - 0.5];
        ic.p[i] = [ic.p[i][0] + ic.m[i] * 0.25, ic.p[i][1] - ic.m[i] * 0.125];
    }
    ic_at::<R>(&ic)
}

fn audit<A: Stepper, R: P>(steps: u32)
where
    f64: AsR<R>,
{
    let ic = off_com::<R>(&close_encounter());
    let mut cadence = Audited::new(&ic);
    integrate_with::<A, R, _>(&ic, &key::<R>(1e-3, steps), &mut cadence);
    assert!(
        cadence.steps >= steps,
        "the cadence ran {} steps",
        cadence.steps
    );
    // The monitor latched what the state after each projected step gave: to the arithmetic's difference, the rounding
    // of `E` and `L_z` at `R` against the same sums at f64.
    let live = cadence.inner.monitor_state();
    let scale = 1.0 + cadence.e_0.abs();
    let tol = 64.0 * R::U * scale;
    let (de, dlz): (f64, f64) = (live.de_max().into(), live.dlz_max().into());
    assert!(
        (de - cadence.de_max).abs() <= tol,
        "max|ΔE|: the monitor's {de:e}, the state's after each projected step {:e}",
        cadence.de_max
    );
    assert!(
        (dlz - cadence.dlz_max).abs() <= tol,
        "max|ΔL_z|: the monitor's {dlz:e}, the state's after each projected step {:e}",
        cadence.dlz_max
    );
}

#[test]
fn qa_com_projection_every_step() {
    audit::<Euler, f64>(2000);
    audit::<Kdk, f64>(2000);
    audit::<Yoshida4, f64>(2000);
    audit::<Yoshida6, f64>(2000);
    audit::<Rk4, f64>(2000);
    audit::<Euler, f32>(2000);
    audit::<Kdk, f32>(2000);
    audit::<Yoshida4, f32>(2000);
    audit::<Yoshida6, f32>(2000);
    audit::<Rk4, f32>(2000);
}

/// A march that doesn't project: its `R_com` stays at the offset the audit must catch.
#[cfg(feature = "controls")]
struct Unprojected<R> {
    audit: Audited<R>,
}

#[cfg(feature = "controls")]
impl<R: P> Cadence<R> for Unprojected<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        // The audit's cadence projects a copy, so the audit's check reads this one's state unprojected.
        let (m, r, p) = (wide_m(&state.m), wide(&state.r), wide(&state.p));
        let (c, _) = hand_com(&m, &r, &p);
        let _ = geom;
        assert!(
            c[0].abs() <= 16.0 * R::U,
            "R_com {c:?} is not zero to rounding"
        );
        self.audit.steps += 1;
    }
    fn done(&self) -> bool {
        false
    }
}

negative_control!(
    qa_com_projection_every_step,
    "a march with the projection left out keeps its offset",
    expected = "is not zero to rounding",
    {
        let ic = off_com::<f64>(&close_encounter());
        let mut cadence = Unprojected {
            audit: Audited::new(&ic),
        };
        integrate_with::<Kdk, f64, _>(&ic, &key::<f64>(1e-3, 2), &mut cadence);
    }
);

/// A state drawn from `rng`, with a bulk offset and momentum.
fn drawn(rng: &mut SplitMix) -> (State<f64>, f64) {
    let mut d = |lo: f32, hi: f32| f64::from(rng.uniform(lo, hi));
    let m = [d(0.1, 2.0), d(0.1, 2.0), d(0.1, 2.0)];
    let r = [
        [d(-3.0, 3.0), d(-3.0, 3.0)],
        [d(-3.0, 3.0), d(-3.0, 3.0)],
        [d(-3.0, 3.0), d(-3.0, 3.0)],
    ];
    let p = [
        [d(-3.0, 3.0), d(-3.0, 3.0)],
        [d(-3.0, 3.0), d(-3.0, 3.0)],
        [d(-3.0, 3.0), d(-3.0, 3.0)],
    ];
    let total = m.iter().sum();
    (State { m, r, p }, total)
}

/// Removing the bulk modes changes `E` by `−|P|²/2M` and `L_z` by `−R × P`, nothing more (Σwᵢ = 1, Σpᵢ = P): the
/// projection neither restores nor rescales either.
fn check_restores_nothing(project: fn(&State<f64>) -> State<f64>) {
    let mut rng = SplitMix::new(0x5305);
    for _ in 0..512 {
        let (s, total) = drawn(&mut rng);
        let out = project(&s);
        let (c, q) = hand_com(&s.m, &s.r, &s.p);
        let (e0, e1) = (
            hand_energy(&s.m, &s.r, &s.p),
            hand_energy(&out.m, &out.r, &out.p),
        );
        let (l0, l1) = (hand_lz(&s.r, &s.p), hand_lz(&out.r, &out.p));
        let want_e = e0 - (q[0] * q[0] + q[1] * q[1]) / (2.0 * total);
        let want_l = l0 - (c[0] * q[1] - c[1] * q[0]);
        // Each sum has a dozen terms of up to the scale below, so a few hundred units of its largest.
        let scale = 1.0 + e0.abs() + l0.abs() + q[0].abs() * q[0].abs() + q[1].abs() * q[1].abs();
        let tol = 512.0 * f64::EPSILON * scale * (1.0 + 1.0 / s.m[0].min(s.m[1]).min(s.m[2]));
        assert!(
            (e1 - want_e).abs() <= tol,
            "E after the projection is {e1}, not E − |P|²/2M = {want_e}"
        );
        assert!(
            (l1 - want_l).abs() <= tol,
            "L_z after the projection is {l1}, not L_z − R × P = {want_l}"
        );
    }
}

fn through_the_cadence(s: &State<f64>) -> State<f64> {
    let mut out = *s;
    let mut geom = Geom::new(&s.m, &s.r);
    WrapperCadence::new(&s.m, &s.r, &s.p).project(&mut out, &mut geom);
    out
}

#[test]
fn qa_projection_restores_nothing() {
    check_restores_nothing(through_the_cadence);
}

negative_control!(
    qa_projection_restores_nothing,
    "a projection that then rescales the momenta to put the kinetic energy back",
    expected = "E after the projection",
    check_restores_nothing(|s| {
        let mut out = through_the_cadence(s);
        let kin = |p: &V| -> f64 {
            (0..3)
                .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * s.m[i]))
                .sum()
        };
        let k = (kin(&s.p) / kin(&out.p).max(1e-300)).sqrt();
        out.p = out.p.map(|v| v.map(|x| k * x));
        out
    })
);

/// Stored before the loop, on the IC as given (with its bulk momentum, which the first projection removes), by hand.
fn check_references(r: &V, p: &V) {
    let cadence = WrapperCadence::new(&M, r, p);
    let m = cadence.monitor_state();
    // `E = 5/2 + 25/4 + 20/10 − (2/√2 + 5/√5 + 10/√13)`, `L_z = 2 − 3 + 10`, each term by hand.
    let e = 2.5 + 6.25 + 2.0 - (2.0 / 2f64.sqrt() + 5.0 / 5f64.sqrt() + 10.0 / 13f64.sqrt());
    assert!(
        (m.e_0() - e).abs() <= 16.0 * f64::EPSILON * e.abs(),
        "E₀ {} ≠ {e}",
        m.e_0()
    );
    assert_eq!(m.lz_0(), 9.0, "L_{{z,0}}");
    assert_eq!(
        (m.de_final(), m.de_max(), m.dlz_final(), m.dlz_max()),
        (0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn qa_invariant_monitor_references_by_hand() {
    check_references(&R0, &P0);
}

negative_control!(
    qa_invariant_monitor_references_by_hand,
    "references taken after the projection lose the bulk term",
    expected = "≠",
    check_references(&R1, &P1)
);

/// Post-projection: from a state with a bulk momentum, the first step's `ΔE` holds `−|P|²/2M`; a monitor reading
/// before the projection would show about 0 there. `dt` is small enough that the step itself adds far less.
fn check_post_projection(read_before_projection: bool) {
    let dt = 1e-9;
    let (m, r, p) = (M, R0, P0);
    let geom = Geom::new(&m, &r);
    let mut cadence = WrapperCadence::new(&m, &r, &p);
    let (mut state, mut g) = Kdk::step(&State { m, r, p }, dt, &geom);
    let before = hand_energy(&m, &state.r, &state.p) - hand_energy(&m, &r, &p);
    cadence.after_step(&mut state, &mut g);
    let got = if read_before_projection {
        before
    } else {
        cadence.monitor_state().de_final()
    };
    let (_, q) = hand_com(&m, &r, &p);
    let want = -(q[0] * q[0] + q[1] * q[1]) / (2.0 * 8.0);
    assert!(
        (got - want).abs() <= 1e-6,
        "ΔE after the first step is {got}, not the post-projection {want}"
    );
    assert_eq!(cadence.invocations(), 1);
}

#[test]
fn qa_invariant_monitor_is_post_projection() {
    check_post_projection(false);
}

negative_control!(
    qa_invariant_monitor_is_post_projection,
    "the drift read before the projection misses the removed bulk term",
    expected = "not the post-projection",
    check_post_projection(true)
);

/// A trace of momentum scales from a state with no bulk momentum: `ΔE` and `ΔL_z` are signed, the maxima unsigned, whichever
/// way the excursion went, and each final the last one.
fn check_signed(read_final: fn(&Monitor<f64>) -> f64) {
    // `Σp = 0`, so the projection leaves `E` and `L_z` but for rounding.
    let m = [1.0, 1.0, 1.0];
    let r: V = [[-1.0, 0.0], [1.0, 0.0], [0.0, 2.0]];
    let p: V = [[0.25, 0.5], [0.25, -1.0], [-0.5, 0.5]];
    let mut cadence = WrapperCadence::new(&m, &r, &p);
    let (e0, l0) = (hand_energy(&m, &r, &p), hand_lz(&r, &p));
    let mut worst = (0f64, 0f64);
    let mut last = (0f64, 0f64);
    // Scales below 1 lower E and move L_z toward 0, so ΔE < 0 and ΔL_z has L₀'s opposite sign.
    for s in [0.5, 0.25, 0.75, 0.875] {
        let q = p.map(|v| v.map(|x| s * x));
        let mut state = State { m, r, p: q };
        let mut g = Geom::new(&m, &r);
        cadence.after_step(&mut state, &mut g);
        last = (hand_energy(&m, &r, &q) - e0, hand_lz(&r, &q) - l0);
        worst = (worst.0.max(last.0.abs()), worst.1.max(last.1.abs()));
    }
    let live = cadence.monitor_state();
    let close = |a: f64, b: f64| (a - b).abs() <= 64.0 * f64::EPSILON * (1.0 + b.abs());
    assert!(
        last.0 < 0.0,
        "the fixture's final ΔE must be negative: {}",
        last.0
    );
    assert!(
        close(read_final(live), last.0),
        "ΔE_final {} ≠ {}",
        read_final(live),
        last.0
    );
    assert!(
        close(live.dlz_final(), last.1),
        "ΔL_z final {} ≠ {}",
        live.dlz_final(),
        last.1
    );
    assert!(
        close(live.de_max(), worst.0),
        "max|ΔE| {} ≠ {}",
        live.de_max(),
        worst.0
    );
    assert!(
        close(live.dlz_max(), worst.1),
        "max|ΔL_z| {} ≠ {}",
        live.dlz_max(),
        worst.1
    );
    assert!(live.de_max() > live.de_final().abs());
}

#[test]
fn qa_invariant_monitor_signed_final_unsigned_max() {
    check_signed(|m| m.de_final());
}

negative_control!(
    qa_invariant_monitor_signed_final_unsigned_max,
    "an unsigned final is not ΔE_final",
    expected = "ΔE_final",
    check_signed(|m| m.de_final().abs())
);

/// `δ = Δ / max(ε, |X₀|)`: the sign of `Δ` kept, `|X₀|` for a negative reference, the floor where `|X₀| < ε`.
fn check_floors(relative: fn(f64, f64, f64) -> f64) {
    let eps = 1e-6f64;
    assert_eq!(relative(-3.0, -2.0, eps), -1.5, "|E₀| over the floor");
    assert_eq!(relative(3.0, 2.0, eps), 1.5);
    assert_eq!(
        relative(1e-6, 0.0, eps),
        1.0,
        "a rest start's L_z,0 = 0 reads the floor"
    );
    assert_eq!(
        relative(1e-6, -1e-9, eps),
        1.0,
        "|X₀| under the floor reads the floor"
    );
    assert_eq!(relative(-1e-6, 1e-9, eps), -1.0);
    assert!(relative(1.0, 0.0, eps).is_finite());
}

#[test]
fn qa_invariant_monitor_relative_floors() {
    check_floors(relative);
}

negative_control!(
    qa_invariant_monitor_relative_floors,
    "no floor: a rest start divides by zero",
    expected = "reads the floor",
    check_floors(|d, i, _| d / i.abs())
);

// ── The predicates (REQ-INT-040) ────────────────────────────────────────────────────────────────────────────────

fn tau_e() -> f32 {
    SUSPECT_ENERGY_THRESHOLD
}

fn tau_l() -> f32 {
    SUSPECT_LZ_THRESHOLD
}

/// Energy-suspect on the **relative** drift: one latch lights a small `|E₀|` and not a large one, either sign, and a
/// rest-start `E₀ = 0` reads the floor.
fn check_energy_relative(predicate: fn(f32, f32, f32) -> bool) {
    let (t, eps) = (tau_e(), 1e-6f32);
    let de = 8.0 * t;
    assert!(predicate(de, 1.0, eps), "δE = 8τ at E₀ = 1 is not lit");
    assert!(predicate(de, -1.0, eps), "δE = 8τ at E₀ = −1 is not lit");
    assert!(!predicate(de, 64.0, eps), "δE = τ/8 at E₀ = 64 is lit");
    assert!(!predicate(de, -64.0, eps), "δE = τ/8 at E₀ = −64 is lit");
    assert!(
        predicate(8.0 * t * eps, 0.0, eps),
        "δE = 8τ at the floor is not lit"
    );
    assert!(
        !predicate(t * eps / 8.0, 0.0, eps),
        "δE = τ/8 at the floor is lit"
    );
    assert!(!predicate(t, 0.0, 1024.0), "a raised ε_E must lower δE");
    assert!(!predicate(f32::NAN, 1.0, eps), "a NaN latch reads lit");
}

#[test]
fn qa_drift_suspect_read_time_energy_is_relative() {
    check_energy_relative(suspect_energy);
}

negative_control!(
    qa_drift_suspect_read_time_energy_is_relative,
    "an energy predicate on the absolute drift",
    expected = "is lit",
    check_energy_relative(|de, _, _| de > tau_e())
);

/// `L_z`-suspect on the **absolute** drift: the latch alone decides, in the read side's predicate.
fn check_lz_absolute(predicate: fn(f32) -> bool) {
    let t = tau_l();
    assert!(predicate(2.0 * t), "2τ is not lit");
    assert!(!predicate(t / 2.0), "τ/2 is lit");
    assert!(!predicate(f32::NAN), "a NaN latch is lit");
}

#[test]
fn qa_drift_suspect_read_time_lz_is_absolute() {
    check_lz_absolute(suspect_lz);
}

negative_control!(
    qa_drift_suspect_read_time_lz_is_absolute,
    "a threshold 8 times higher leaves 2τ unlit",
    expected = "is not lit",
    check_lz_absolute(|d| d > 8.0 * tau_l())
);

/// The read side's whole predicate, from the stored state: it reads `dE_max`, `E_0`, `dLz_max`, not `r`, `p`, `Lz_0`, so
/// moving those moves nothing; and the flag it carries is the one it is given, the occupant's.
fn check_latches(read: fn(&SimState, f32, bool) -> DriftSuspect) {
    let t = tau_e();
    let eps = 1e-6f32;
    let mut s = read_state();
    s.E_0 = 1.0;
    s.Lz_0 = 1.0;
    s.dE_max = 8.0 * t;
    s.dLz_max = tau_l() / 2.0;
    let want = read(&s, eps, true);
    assert_eq!(
        (want.energy, want.lz, want.reads_dynamics),
        (true, false, true)
    );
    let mut moved = s;
    moved.r = [[9.0, 9.0]; 3];
    moved.p = [[-7.0, 7.0]; 3];
    moved.Lz_0 = 1e6;
    assert_eq!(
        read(&moved, eps, true),
        want,
        "the predicate moved with r, p or Lz_0"
    );
    let lit = kernel::payload::SimState {
        dLz_max: 2.0 * tau_l(),
        ..s
    };
    assert!(read(&lit, eps, true).lz);
    assert!(!read(&s, eps, false).reads_dynamics);
    // integrator contract Part 2: the flag the consumer reads is the profile's.
    let flags = [
        Euler::profile().symplectic,
        Rk4::profile().symplectic,
        Kdk::profile().symplectic,
        Yoshida4::profile().symplectic,
        Yoshida6::profile().symplectic,
    ];
    assert_eq!(flags, [false, false, true, true, true]);
}

#[test]
fn qa_drift_suspect_read_time_reads_only_the_latches() {
    check_latches(kernel::payload::drift_suspect);
}

negative_control!(
    qa_drift_suspect_read_time_reads_only_the_latches,
    "an L_z predicate that reads Lz_0 moves with it",
    expected = "moved with r, p or Lz_0",
    check_latches(|s, eps, symplectic| DriftSuspect {
        lz: s.dLz_max * s.Lz_0.abs() > tau_l(),
        ..kernel::payload::drift_suspect(s, eps, symplectic)
    })
);

fn read_state() -> kernel::payload::SimState {
    let fixture = figure_eight();
    let short = Fixture {
        horizon: 1.0,
        ..fixture
    };
    let (out, _) = integrate_fixture::<Kdk, f32>(&short);
    let params = kernel::payload::ReadParams {
        dt_macro: 1e-3,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
    };
    kernel::payload::sim_state_from_base(
        &out,
        [0; 4],
        false,
        kernel::payload::canonical_nan(),
        false,
        ic_at::<f32>(&short.ic).m,
        &params,
    )
}

/// RQ-626's option (a) names "bound, interacting states". Draws from the latent cube near rest, keeping those with
/// `E₀ < 0` whose bodies are all still within `SEPARATION` of each other at the end of the run: Euler at f32 lights
/// `SUSPECT_ENERGY` on each.
fn check_lights_bound_states<A: Stepper>() {
    const SEPARATION: f64 = 8.0;
    let mut rng = SplitMix::new(0x0626);
    let (mut kept, mut lit, mut least) = (0u32, 0u32, f64::MAX);
    for k in 0..1024 {
        // The momentum controls within a band about rest: a full-range draw is almost never bound.
        let band = [0.05, 0.1, 0.2, 0.4][k % 4];
        let z: [f64; 8] = core::array::from_fn(|i| {
            let reach = if i < 2 { 1.5 } else { band };
            f64::from(rng.uniform(-reach, reach))
        });
        let ic = decode::decode::<Defaults, f64>(&Latent::new(z)).ic;
        if hand_energy(&ic.m, &ic.r, &ic.p) >= 0.0 {
            continue;
        }
        let fixture = Fixture {
            name: "a bound draw",
            ic,
            horizon: 50.0,
        };
        let (out, _) = integrate_fixture::<A, f32>(&fixture);
        let r = wide(&out.r);
        let far = [(0, 1), (0, 2), (1, 2)].iter().fold(0f64, |a, &(i, j)| {
            let (dx, dy) = (r[i][0] - r[j][0], r[i][1] - r[j][1]);
            a.max((dx * dx + dy * dy).sqrt())
        });
        if far > SEPARATION {
            continue;
        }
        kept += 1;
        let de = kernel::payload::pb_dE_max(out.packed_b);
        let rel = f64::from(de) / 1e-6f64.max(f64::from(out.E_0.abs()));
        least = least.min(rel);
        lit += u32::from(suspect_energy(de, out.E_0, 1e-6));
    }
    println!("bound draws kept {kept}, lit {lit}, least relative stored dE_max {least:e}");
    assert!(kept >= 16, "too few bound draws to say anything: {kept}");
    assert_eq!(
        lit, kept,
        "the occupant left some bound draws unlit, least δE {least:e}"
    );
}

#[test]
fn qa_drift_suspect_euler_on_bound_states() {
    check_lights_bound_states::<Euler>();
}

negative_control!(
    qa_drift_suspect_euler_on_bound_states,
    "KDK's energy error stays under the threshold on most of them",
    expected = "left some bound draws unlit",
    check_lights_bound_states::<Kdk>()
);
