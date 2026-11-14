//! QA's tests for TASK-M3-77 (REQ-INT-123): Heggie's step control and landing built to integrator_contract Part 2b
//! § "Heggie: the equations, the time transformations, the maps and the step control (transcribed)", steps 1 to 10,
//! with the landing under R-998's rule as R-1007 applies it (no tolerance, the state landing with the clock), R-604's
//! step budget, and Part 6's floor signal and entry-cap hits (R-160, R-561, R-565, R-604, R-998, R-1007, R-1013).
//!
//! Written from the requirement and the contract, not from the implementation. Every anchor is computed here:
//! - `qa_m377_landing`: Heggie × RK4 under Eq. 20 and Eq. 22, at f64 and f32, from a fixture triple of its own, lands
//!   on each boundary 1 to 8: the stored state's own clock is `Δ_sync` bitwise, the stored Cartesian state is that
//!   regularised state's image, the time reported `k·Δ_sync` bitwise, and no state the march keeps (every one its
//!   finiteness test reads) has its clock past the boundary. Control: the landing off (the clamp off, both caps 0).
//! - `qa_m377_clamp_off`: the clamp off alone does not turn the landing off (step 8, "The setting that turns it off"):
//!   still no kept clock past the boundary. Control: the clamp off with both caps 0.
//! - `qa_m377_registered_once`: the march registers once (one `registered` call) for any number of boundaries, `h` is
//!   the registration's bitwise, and each boundary passes the landed `(Q, P)` into the next interval untouched, its clock
//!   reset to `0` (steps 1, 2, 10). Control: a march re-registered at every boundary.
//! - `qa_m377_dt_dtau`: `dt/dτ` is `R₀R₁R₂` (Eq. 20) or `R₀R₁R₂/S^{3/2}` (Eq. 22), against `Rᵢ = |Qᵢ|²` computed here,
//!   and the clamp's `dt/dτ` is bitwise the rate at which the step advances `t` (step 5). Control: Eq. 20's `dt/dτ` read
//!   under Eq. 22.
//! - `qa_m377_predictive`: step 6's limit `f·d_min/(v_max·dt/dτ)` at random registered states, against the pair
//!   separations and relative velocities of the Cartesian image computed here (`qᵢ = rⱼ − rₖ`, `q̇ᵢ = vⱼ − vₖ`), off
//!   for `f ≤ 0` and at rest. Control: the relative velocities read as `vⱼ + vₖ`.
//! - `qa_m377_step_size`: step 5's rule at its edges, against the rule written out here: the entry cap's strict `>`,
//!   the predictive cut, the clamp `max(dt_left − t, 0)/d` with the same floored `d`, and the floor signal on
//!   `dt/dτ < TINY` and on `Rᵢ < TINY` each alone, strict. Control: the clamp read with the unfloored `dt/dτ`.
//! - `qa_m377_signals_counted`: in a march, the floor signals and entry-cap hits counted equal those tallied here from
//!   each step's pre-floor `dt/dτ`, `S` and least `Rᵢ`, both nonzero. Control: the cap hit tallied with `≥`.
//! - `qa_m377_budget`: R-604's helper, over when the running total exceeds `max(N_min, rate·t)`, at its edges at f64
//!   and f32; the march ends `Exhausted`, latched, with no step after, and its allowance reads the time since `t = 0`,
//!   not the interval clock. Control: a helper over at equality.
//! - `qa_m377_non_finite`: a state with two bodies coincident ends the march at entry (`dt/dτ = 0`), and a NaN in a
//!   regularised position, momentum or the clock mid-interval ends it `NonFinite`, latched at the last finite state.
//!   Control: a finiteness test that reads positions and momenta but not the clock.
//!
//! The values marked "test value" are fixtures, not calibrations (REQ-INT-080, REQ-INT-103, REQ-INT-105, REQ-INT-108
//! are the calibrations). The identities are checked bitwise where the contract says bitwise, else at `ROUND`.

use std::cell::Cell;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{CapabilityProfile, Geom, State};
use kernel::regularisation::az::{over_allowance, AzReal, End, ExplicitTest};
use kernel::regularisation::form::Form;
use kernel::regularisation::heggie::{
    march, predictive, register, step_size, HgAdvanced, HgP, HgParams, HgQ, HgStepper, HgSystem,
    HgWatch, DEFAULT_TIME,
};
use kernel::regularisation::pairs::HeggieTime;
use validation::negative_control;

/// The f64 round-off bound for identities between two orderings of the same arithmetic (relative, O(1) quantities).
const ROUND: f64 = 1e-12;

// ── Fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A bound triple of qa's own at masses `(1, 0.6, 0.3)`, moved to its centre of mass and momentum (test value).
fn fixture() -> ([f64; 3], [[f64; 2]; 3], [[f64; 2]; 3]) {
    let m = [1.0, 0.6, 0.3];
    let r = [[-0.45, 0.05], [0.55, -0.1], [0.05, 1.1]];
    let v = [[0.02, -0.18], [-0.05, 0.3], [0.1, 0.0]];
    let total: f64 = m.iter().sum();
    let mut rc = [0.0; 2];
    let mut vc = [0.0; 2];
    for i in 0..3 {
        for c in 0..2 {
            rc[c] += m[i] * r[i][c] / total;
            vc[c] += m[i] * v[i][c] / total;
        }
    }
    let r = [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c]));
    let v = [0, 1, 2].map(|i| [0, 1].map(|c| v[i][c] - vc[c]));
    (m, r, v)
}

fn fixture64() -> State<f64> {
    let (m, r, v) = fixture();
    State {
        m,
        r,
        p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * v[i][c])),
    }
}

fn fixture32() -> State<f32> {
    let s = fixture64();
    State {
        m: s.m.map(|x| x as f32),
        r: s.r.map(|v| v.map(|x| x as f32)),
        p: s.p.map(|v| v.map(|x| x as f32)),
    }
}

/// The occupant's landing in force (the clamp, the secant's 4 retakes, the bisection's 64 halvings; Part 2b step 8),
/// R-160's `f = 0.02`, and test values for `η`, `Δ_sync`, `TINY` and a budget that does not bind.
fn params64(time: HeggieTime) -> HgParams<f64> {
    HgParams {
        eta: 0.02,
        delta_sync: 0.1,
        tiny: 1e-300,
        predictive_f: 0.02,
        step_budget_rate: 0.0,
        n_min: 4_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time,
    }
}

fn params32(time: HeggieTime) -> HgParams<f32> {
    HgParams {
        eta: 0.02,
        delta_sync: 0.1,
        tiny: 1e-37,
        predictive_f: 0.02,
        step_budget_rate: 0.0,
        n_min: 4_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time,
    }
}

/// A cadence that counts the steps it follows and never ends the march.
#[derive(Default)]
struct Counting(u32);

impl<R> Cadence<R> for Counting {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.0 += 1;
    }
    fn done(&self) -> bool {
        false
    }
}

/// A watch that records what the march tells it, its finiteness test the explicit one unless `skip_clock`, which
/// leaves the clock unread (the control's).
struct Recorder<R> {
    registrations: u32,
    kept: Vec<(HgQ<R>, HgP<R>)>,
    sized: Vec<(R, R, R)>,
    skip_clock: bool,
}

impl<R> Recorder<R> {
    fn new() -> Self {
        Recorder {
            registrations: 0,
            kept: Vec::new(),
            sized: Vec::new(),
            skip_clock: false,
        }
    }
}

impl<R: AzReal> HgWatch<R> for Recorder<R> {
    fn finite(&mut self, q: &HgQ<R>, p: &HgP<R>) -> bool {
        let ok = if self.skip_clock {
            let probe = HgQ {
                t: R::default(),
                ..*q
            };
            HgWatch::<R>::finite(&mut ExplicitTest, &probe, p)
        } else {
            HgWatch::<R>::finite(&mut ExplicitTest, q, p)
        };
        if ok {
            self.kept.push((*q, *p));
        }
        ok
    }
    fn registered(&mut self, _q: &HgQ<R>) {
        self.registrations += 1;
    }
    fn sized(&mut self, dt_dtau: R, s: R, r_least: R) {
        self.sized.push((dt_dtau, s, r_least));
    }
}

/// `Rᵢ = |Qᵢ|²`, computed here.
fn rs(q: &HgQ<f64>) -> [f64; 3] {
    q.q.map(|u| u[0] * u[0] + u[1] * u[1])
}

/// `dt/dτ` written from Part 2b: `R₀R₁R₂`, or `R₀R₁R₂/S^{3/2}` with `S` floored at `TINY`.
fn dt_dtau_here(q: &HgQ<f64>, time: HeggieTime, tiny: f64) -> f64 {
    let r = rs(q);
    let product = r[0] * r[1] * r[2];
    match time {
        HeggieTime::Eq20 => product,
        HeggieTime::Eq22 => product / r.iter().sum::<f64>().max(tiny).powf(1.5),
    }
}

/// The pair separations `|rⱼ − rₖ|` and relative velocities `|vⱼ − vₖ|` of a Cartesian state, `vᵢ = pᵢ/mᵢ`; with
/// `wrong`, the velocities read as `vⱼ + vₖ` (the control's).
fn pairs_here(s: &State<f64>, wrong: bool) -> ([f64; 3], [f64; 3]) {
    let v = [0, 1, 2].map(|i| [s.p[i][0] / s.m[i], s.p[i][1] / s.m[i]]);
    let mut d = [0.0; 3];
    let mut w = [0.0; 3];
    for i in 0..3 {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        d[i] = (s.r[j][0] - s.r[k][0]).hypot(s.r[j][1] - s.r[k][1]);
        w[i] = if wrong {
            (v[j][0] + v[k][0]).hypot(v[j][1] + v[k][1])
        } else {
            (v[j][0] - v[k][0]).hypot(v[j][1] - v[k][1])
        };
    }
    (d, w)
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(f64::MIN_POSITIVE)
}

/// A xorshift generator, so the draws are fixed.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn sym(&mut self, a: f64) -> f64 {
        a * (2.0 * self.unit() - 1.0)
    }
}

/// A random regularised state: `Qᵢ` and `Pᵢ` components in `[−1.5, 1.5]`, the clock `0`.
fn random_state(rng: &mut Rng) -> (HgQ<f64>, HgP<f64>) {
    let mut v = || [rng.sym(1.5), rng.sym(1.5)];
    let q = HgQ {
        q: [v(), v(), v()],
        t: 0.0,
    };
    let p = HgP { p: [v(), v(), v()] };
    (q, p)
}

/// A random Cartesian state at masses `(1, 0.6, 0.3)`, positions in `[−1, 1]` and velocities in `[−0.5, 0.5]`, moved to
/// its centre of mass and momentum, registered under `time`: a regularised state whose `qᵢ` close (`Σqᵢ = 0`).
fn random_registered(rng: &mut Rng, time: HeggieTime) -> (HgSystem<f64>, HgQ<f64>, HgP<f64>) {
    let m = [1.0, 0.6, 0.3];
    let total: f64 = m.iter().sum();
    let r: [[f64; 2]; 3] = [0, 1, 2].map(|_| [rng.sym(1.0), rng.sym(1.0)]);
    let v: [[f64; 2]; 3] = [0, 1, 2].map(|_| [rng.sym(0.5), rng.sym(0.5)]);
    let rc = [0, 1].map(|c| (0..3).map(|i| m[i] * r[i][c]).sum::<f64>() / total);
    let vc = [0, 1].map(|c| (0..3).map(|i| m[i] * v[i][c]).sum::<f64>() / total);
    let s = State {
        m,
        r: [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c])),
        p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * (v[i][c] - vc[c]))),
    };
    let reg = register(&s, 1e-300, time);
    (reg.system, reg.q, reg.p)
}

// ── qa_m377_landing ──────────────────────────────────────────────────────────────────────────────────────────────

/// Heggie × RK4 at f64 under `time` lands on each boundary 1 to 8 with the state's own clock the boundary's, under
/// the landing settings `(clamp, secant, bisect)`.
fn check_landing64(time: HeggieTime, landing: (bool, u32, u32)) {
    let s0 = fixture64();
    let mut p = params64(time);
    (p.clamp, p.secant_tries, p.bisect_tries) = landing;
    let delta = p.delta_sync;
    for k in 1..=8u32 {
        let mut watch = Recorder::new();
        let out: HgAdvanced<f64> = march::<Rk4, f64, Counting, Recorder<f64>>(
            &s0,
            k,
            &p,
            &mut Counting::default(),
            &mut watch,
        );
        let past = watch.kept.iter().map(|(q, _)| q.t).fold(0.0f64, f64::max);
        assert!(
            past <= delta,
            "{time:?}, k = {k}: a state kept past the boundary (clock {past:e}, Δ {delta:e})"
        );
        assert_eq!(
            (out.count.end, out.boundary),
            (End::Marching, k),
            "{time:?}: did not land on {k}"
        );
        assert_eq!(
            out.q.t.to_bits(),
            delta.to_bits(),
            "{time:?}, k = {k}: the stored state's own clock {:e} is not the boundary's",
            out.q.t
        );
        assert_eq!(
            out.t.to_bits(),
            (f64::from(k) * delta).to_bits(),
            "{time:?}, k = {k}: the time reported {:e} is not k·Δ_sync",
            out.t
        );
        assert_eq!(
            out.state,
            out.system.image(&out.q, &out.p),
            "{time:?}, k = {k}: the stored Cartesian state is not the image of the landed regularised state"
        );
        let landings = watch.kept.iter().filter(|(q, _)| q.t == delta).count();
        assert_eq!(landings, k as usize, "{time:?}: one landing per boundary");
    }
}

fn check_landing32(time: HeggieTime) {
    let s0 = fixture32();
    let p = params32(time);
    let delta = p.delta_sync;
    for k in 1..=8u32 {
        let mut watch = Recorder::new();
        let out = march::<Rk4, f32, Counting, Recorder<f32>>(
            &s0,
            k,
            &p,
            &mut Counting::default(),
            &mut watch,
        );
        let past = watch.kept.iter().map(|(q, _)| q.t).fold(0.0f32, f32::max);
        assert!(
            past <= delta,
            "f32 {time:?}, k = {k}: a state kept past the boundary"
        );
        assert_eq!(
            (
                out.count.end,
                out.boundary,
                out.q.t.to_bits(),
                out.t.to_bits()
            ),
            (
                End::Marching,
                k,
                delta.to_bits(),
                (k as f32 * delta).to_bits()
            ),
            "f32 {time:?}: did not land on {k} with the state's own clock the boundary's"
        );
        assert_eq!(out.state, out.system.image(&out.q, &out.p));
    }
}

#[test]
fn qa_m377_landing() {
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        check_landing64(time, (true, 4, 64));
        check_landing32(time);
    }
}

negative_control!(
    qa_m377_landing,
    "the landing off: the clamp off and both caps 0, a step past the boundary kept",
    expected = "a state kept past the boundary",
    check_landing64(DEFAULT_TIME, (false, 0, 0))
);

// ── qa_m377_clamp_off ────────────────────────────────────────────────────────────────────────────────────────────

/// With the clamp off and the retakes `(secant, bisect)`, no state the march keeps has its clock past the boundary; the
/// march either lands with the state's own clock the boundary's or ends on its budget, latched.
fn check_clamp_off(secant: u32, bisect: u32) {
    let s0 = fixture64();
    let mut p = params64(DEFAULT_TIME);
    p.clamp = false;
    p.secant_tries = secant;
    p.bisect_tries = bisect;
    p.n_min = 20_000;
    let mut watch = Recorder::new();
    let out = march::<Rk4, f64, Counting, Recorder<f64>>(
        &s0,
        2,
        &p,
        &mut Counting::default(),
        &mut watch,
    );
    let past = watch.kept.iter().map(|(q, _)| q.t).fold(0.0f64, f64::max);
    assert!(
        past <= p.delta_sync,
        "the clamp off: a state kept past the boundary (clock {past:e})"
    );
    assert!(
        out.q.t <= p.delta_sync,
        "the clamp off: the stored clock past the boundary"
    );
    if out.count.end == End::Marching {
        assert_eq!(out.q.t.to_bits(), p.delta_sync.to_bits());
    }
}

#[test]
fn qa_m377_clamp_off() {
    check_clamp_off(4, 64);
    check_clamp_off(0, 64);
}

negative_control!(
    qa_m377_clamp_off,
    "the clamp off and both caps 0",
    expected = "a state kept past the boundary",
    check_clamp_off(0, 0)
);

// ── qa_m377_registered_once ──────────────────────────────────────────────────────────────────────────────────────

/// What a march to `k` registered and kept; with `reregister`, the march to `k` run as `k` one-boundary marches, each
/// registered from the last one's Cartesian state (the control's).
fn run_to(k: u32, reregister: bool) -> (HgAdvanced<f64>, Recorder<f64>) {
    let p = params64(DEFAULT_TIME);
    let mut watch = Recorder::new();
    if reregister {
        let mut s = fixture64();
        let mut out = None;
        for _ in 0..k {
            let o = march::<Rk4, f64, Counting, Recorder<f64>>(
                &s,
                1,
                &p,
                &mut Counting::default(),
                &mut watch,
            );
            s = o.state;
            out = Some(o);
        }
        (out.expect("k > 0"), watch)
    } else {
        let out = march::<Rk4, f64, Counting, Recorder<f64>>(
            &fixture64(),
            k,
            &p,
            &mut Counting::default(),
            &mut watch,
        );
        (out, watch)
    }
}

fn check_once(reregister: bool) {
    let registered = register(&fixture64(), 1e-300, DEFAULT_TIME).system;
    let (out, watch) = run_to(5, reregister);
    assert_eq!(
        watch.registrations, 1,
        "registered {} times, not once",
        watch.registrations
    );
    assert_eq!(
        out.system.h.to_bits(),
        registered.h.to_bits(),
        "h is not the registration's"
    );
    // Each landing is followed by the next interval's entry: the same (Q, P) bitwise, the clock reset to 0.
    let delta = params64(DEFAULT_TIME).delta_sync;
    let mut carried = 0;
    for w in watch.kept.windows(2) {
        if w[0].0.t == delta {
            assert_eq!(
                w[1].0.t.to_bits(),
                0f64.to_bits(),
                "the clock not reset at the boundary"
            );
            assert_eq!(
                (w[1].0.q, w[1].1.p),
                (w[0].0.q, w[0].1.p),
                "the boundary touched the regularised state"
            );
            carried += 1;
        }
    }
    assert_eq!(carried, 4, "four boundaries carried into a next interval");
}

#[test]
fn qa_m377_registered_once() {
    check_once(false);
}

negative_control!(
    qa_m377_registered_once,
    "a march re-registered at every boundary",
    expected = "times, not once",
    check_once(true)
);

// ── qa_m377_dt_dtau ──────────────────────────────────────────────────────────────────────────────────────────────

fn check_dt_dtau(read_as: Option<HeggieTime>) {
    let mut rng = Rng(0x3377_0001);
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let sys = HgSystem::new([1.0, 0.6, 0.3], -0.7, 1e-300, time);
        for _ in 0..200 {
            let (q, p) = random_state(&mut rng);
            let here = dt_dtau_here(&q, read_as.unwrap_or(time), 1e-300);
            let d = sys.dt_dtau(&q);
            assert!(
                rel(d, here) < ROUND,
                "{time:?}: dt/dτ {d:e} against {here:e} written from Part 2b"
            );
            let rate = sys.rates_of(&q, &p).0.t;
            assert_eq!(
                d.to_bits(),
                rate.to_bits(),
                "{time:?}: the clamp's dt/dτ {d:e} is not bitwise the rate of t {rate:e}"
            );
        }
    }
}

#[test]
fn qa_m377_dt_dtau() {
    check_dt_dtau(None);
}

negative_control!(
    qa_m377_dt_dtau,
    "Eq. 20's dt/dτ read under Eq. 22",
    expected = "written from Part 2b",
    check_dt_dtau(Some(HeggieTime::Eq20))
);

// ── qa_m377_predictive ───────────────────────────────────────────────────────────────────────────────────────────

fn check_predictive(wrong: bool) {
    let mut rng = Rng(0x3377_0002);
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        for _ in 0..200 {
            let (sys, q, p) = random_registered(&mut rng, time);
            let f = 0.02;
            let (d, w) = pairs_here(&sys.image(&q, &p), wrong);
            let d_min = d.iter().copied().fold(f64::INFINITY, f64::min);
            let v_max = w.iter().copied().fold(0.0, f64::max);
            let want = f * d_min / (v_max * dt_dtau_here(&q, time, 1e-300));
            let (lim, on) = predictive(&sys, &q, &p, f);
            assert!(on, "{time:?}: the limit off at a moving state");
            assert!(
                rel(lim, want) < 1e-10,
                "{time:?}: the predictive limit {lim:e} against f·d_min/(v_max·dt/dτ) {want:e}"
            );
            assert!(!predictive(&sys, &q, &p, 0.0).1, "the limit on at f = 0");
            assert!(!predictive(&sys, &q, &p, -0.02).1, "the limit on at f < 0");
        }
        // At rest, v_max = 0: no bound.
        let (sys, q, _) = random_registered(&mut rng, time);
        let rest = HgP { p: [[0.0; 2]; 3] };
        assert!(
            !predictive(&sys, &q, &rest, 0.02).1,
            "{time:?}: the limit on at rest"
        );
    }
}

#[test]
fn qa_m377_predictive() {
    check_predictive(false);
}

negative_control!(
    qa_m377_predictive,
    "the relative velocities read as vⱼ + vₖ",
    expected = "the predictive limit",
    check_predictive(true)
);

// ── qa_m377_step_size ────────────────────────────────────────────────────────────────────────────────────────────

/// Step 5's rule, written out here: `d = max(dt/dτ, TINY)`; `min(η·dt_left/d, entry)`; the predictive cut; the clamp
/// `min(·, max(dt_left − t, 0)/d)`. With `unfloored_clamp`, the clamp divides by the unfloored `dt/dτ` (the control's).
fn rule(
    sys: &HgSystem<f64>,
    q: &HgQ<f64>,
    p: &HgP<f64>,
    entry: f64,
    pr: &HgParams<f64>,
    unfloored_clamp: bool,
) -> f64 {
    let dt = sys.dt_dtau(q);
    let d = dt.max(pr.tiny);
    let mut h = (pr.eta * pr.delta_sync / d).min(entry);
    let (lim, on) = predictive(sys, q, p, pr.predictive_f);
    if on && lim < h {
        h = lim;
    }
    if pr.clamp {
        let c = if unfloored_clamp { dt } else { d };
        h = h.min((pr.delta_sync - q.t).max(0.0) / c);
    }
    h
}

fn check_step_size(unfloored_clamp: bool) {
    let s0 = fixture64();
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let reg = register(&s0, 1e-300, time);
        let (sys, q0, p0) = (reg.system, reg.q, reg.p);
        let mut pr = params64(time);
        pr.predictive_f = 0.0;
        let d = sys.dt_dtau(&q0);
        let c0 = pr.eta * pr.delta_sync / d;
        // The entry cap: a hit only where η·dt_left/d > entry, strictly.
        for (entry, hit) in [(0.5 * c0, true), (c0, false), (2.0 * c0, false)] {
            let st = step_size(&sys, &q0, &p0, entry, &pr);
            assert_eq!(
                (st.h.to_bits(), st.cap_hit, st.by_limit, st.floored),
                (c0.min(entry).to_bits(), hit, false, false),
                "{time:?}: the entry cap at entry {entry:e}, c₀ {c0:e}"
            );
        }
        // The clamp: the room left over d, never negative; off, the step is c₀.
        for frac in [0.25, 0.0, -0.5] {
            let q = HgQ {
                t: pr.delta_sync - frac * c0 * d,
                ..q0
            };
            let st = step_size(&sys, &q, &p0, 2.0 * c0, &pr);
            assert_eq!(
                st.h.to_bits(),
                rule(&sys, &q, &p0, 2.0 * c0, &pr, false).to_bits(),
                "{time:?}: the clamp at {frac} of a step left"
            );
            assert!(
                st.h >= 0.0 && st.h <= c0,
                "{time:?}: the clamped step out of [0, c₀]"
            );
            let mut off = pr;
            off.clamp = false;
            assert_eq!(
                step_size(&sys, &q, &p0, 2.0 * c0, &off).h.to_bits(),
                c0.to_bits()
            );
        }
        // The predictive cut.
        let mut cut = pr;
        cut.predictive_f = 1e-9;
        let st = step_size(&sys, &q0, &p0, 2.0 * c0, &cut);
        let (lim, on) = predictive(&sys, &q0, &p0, 1e-9);
        assert!(
            on && lim < c0,
            "the fixture's limit at f = 1e-9 is below c₀"
        );
        assert_eq!(
            (st.h.to_bits(), st.by_limit),
            (lim.to_bits(), true),
            "{time:?}: the predictive cut"
        );
        cut.predictive_f = 1e9;
        let st = step_size(&sys, &q0, &p0, 2.0 * c0, &cut);
        assert_eq!(
            (st.h.to_bits(), st.by_limit),
            (c0.to_bits(), false),
            "{time:?}: a limit above c₀ cuts"
        );
    }
    // The floor signal, each arm alone, strict, on synthetic positions; and the clamp with the floored d.
    let zero_p = HgP { p: [[0.0; 2]; 3] };
    let a = 0.3f64.sqrt();
    // dt/dτ < TINY with every Rᵢ ≥ TINY: Rᵢ = 0.3, dt/dτ ≤ 0.027.
    let q = HgQ {
        q: [[a, 0.0], [0.0, a], [a, 0.0]],
        t: 0.0,
    };
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let sys = HgSystem::new([1.0, 0.6, 0.3], -0.7, 0.05, time);
        let mut pr = params64(time);
        pr.tiny = 0.05;
        pr.predictive_f = 0.0;
        let dt = sys.dt_dtau(&q);
        assert!(
            dt < pr.tiny && rs(&q).iter().all(|&r| r >= pr.tiny),
            "fixture: the dt/dτ arm alone"
        );
        let st = step_size(&sys, &q, &zero_p, 1e9, &pr);
        assert!(st.floored, "{time:?}: dt/dτ < TINY not signalled");
        // Near the boundary the clamp divides by the floored d = TINY.
        let near = HgQ {
            t: pr.delta_sync - 1e-6,
            ..q
        };
        let st = step_size(&sys, &near, &zero_p, 1e9, &pr);
        assert_eq!(
            st.h.to_bits(),
            rule(&sys, &near, &zero_p, 1e9, &pr, unfloored_clamp).to_bits(),
            "{time:?}: the clamp with the floored d"
        );
    }
    // Rᵢ < TINY with dt/dτ ≥ TINY: R₀ = 1e-4, R₁ = R₂ = 100 under Eq. 20; and R₀ = TINY exactly is not a signal.
    let q = HgQ {
        q: [[0.01, 0.0], [10.0, 0.0], [0.0, 10.0]],
        t: 0.0,
    };
    let r0 = rs(&q)[0];
    for (tiny, signal) in [(2.0 * r0, true), (r0, false)] {
        let sys = HgSystem::new([1.0, 0.6, 0.3], -0.7, tiny, HeggieTime::Eq20);
        let mut pr = params64(HeggieTime::Eq20);
        pr.tiny = tiny;
        pr.predictive_f = 0.0;
        assert!(sys.dt_dtau(&q) >= tiny, "fixture: the Rᵢ arm alone");
        assert_eq!(
            step_size(&sys, &q, &zero_p, 1e9, &pr).floored,
            signal,
            "Rᵢ against TINY {tiny:e}: the floor signal"
        );
    }
}

#[test]
fn qa_m377_step_size() {
    check_step_size(false);
}

negative_control!(
    qa_m377_step_size,
    "the clamp read with the unfloored dt/dτ",
    expected = "the clamp with the floored d",
    check_step_size(true)
);

// ── qa_m377_signals_counted ──────────────────────────────────────────────────────────────────────────────────────

/// One interval of Heggie × RK4 (Eq. 20) from the Pythagorean triple (masses 3, 4, 5 at rest) scaled by 0.1, with
/// `TINY` the median pre-floor `dt/dτ` of a run at a negligible floor, so the floor acts on some steps and not others.
/// The counts against those tallied here from the steps' pre-floor values; with `ge`, the cap hit tallied with `≥`.
fn check_signals(ge: bool) {
    let m = [3.0, 4.0, 5.0];
    let r0 = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]];
    let total: f64 = m.iter().sum();
    let c = [0, 1].map(|k| (0..3).map(|i| m[i] * r0[i][k]).sum::<f64>() / total);
    let s = State {
        m,
        r: r0.map(|r| [0.1 * (r[0] - c[0]), 0.1 * (r[1] - c[1])]),
        p: [[0.0; 2]; 3],
    };
    let mut pr = params64(HeggieTime::Eq20);
    pr.delta_sync = 0.004;
    pr.eta = 0.05;
    let mut probe = Recorder::new();
    march::<Rk4, f64, Counting, Recorder<f64>>(&s, 1, &pr, &mut Counting::default(), &mut probe);
    let mut ds: Vec<f64> = probe.sized.iter().map(|x| x.0).collect();
    ds.sort_by(f64::total_cmp);
    pr.tiny = ds[ds.len() / 2];
    // The floor also enters the conditioned inverse; it stays below every Levi-Civita half-norm here.
    assert!(
        pr.tiny < 0.1,
        "fixture: TINY {:e} small beside the separations",
        pr.tiny
    );
    let mut watch = Recorder::new();
    let out = march::<Rk4, f64, Counting, Recorder<f64>>(
        &s,
        1,
        &pr,
        &mut Counting::default(),
        &mut watch,
    );
    assert_eq!(out.boundary, 1, "the fixture interval lands");
    let entry = pr.eta * pr.delta_sync / watch.sized[0].0;
    let mut floors = 0;
    let mut hits = 0;
    let mut tallied = 0;
    for &(dt, _s, r_least) in &watch.sized {
        floors += u32::from(dt < pr.tiny || r_least < pr.tiny);
        let c0 = pr.eta * pr.delta_sync / dt.max(pr.tiny);
        hits += u32::from(c0 > entry);
        tallied += u32::from(if ge { c0 >= entry } else { c0 > entry });
    }
    let n = watch.sized.len() as u32;
    assert!(
        floors > 0 && floors < n,
        "fixture: the floor on some steps only ({floors} of {n})"
    );
    assert!(
        hits > 0 && hits < n,
        "fixture: cap hits on some steps only ({hits} of {n})"
    );
    assert_eq!(out.count.floored, floors, "the floor signals counted");
    assert_eq!(out.count.cap_hits, tallied, "the entry-cap hits counted");
}

#[test]
fn qa_m377_signals_counted() {
    check_signals(false);
}

negative_control!(
    qa_m377_signals_counted,
    "the cap hit tallied with ≥ (the entry step itself counted)",
    expected = "the entry-cap hits counted",
    check_signals(true)
);

// ── qa_m377_budget ───────────────────────────────────────────────────────────────────────────────────────────────

fn check_allowance(
    f64_over: impl Fn(u32, f64, f64, u32) -> bool,
    f32_over: impl Fn(u32, f32, f32, u32) -> bool,
) {
    // max(N_min, rate·t): N_min = 10 binding at t = 0 and below rate·t = 10.
    for (steps, t, rate, n_min, over) in [
        (10, 0.0, 0.0, 10, false),
        (11, 0.0, 0.0, 10, true),
        (0, 0.0, 0.0, 0, false),
        (1, 0.0, 0.0, 0, true),
        (100, 0.25, 400.0, 10, false),
        (101, 0.25, 400.0, 10, true),
        (200, 0.25, 400.0, 200, false),
        (201, 0.25, 400.0, 200, true),
    ] {
        assert_eq!(
            f64_over(steps, t, rate, n_min),
            over,
            "f64: {steps} steps against max({n_min}, {rate}·{t})"
        );
        assert_eq!(
            f32_over(steps, t as f32, rate as f32, n_min),
            over,
            "f32: {steps} steps against max({n_min}, {rate}·{t})"
        );
    }
}

/// A march under the budget `(n_min, rate)` to `k`, and the cadence's count.
fn budget_run(k: u32, n_min: u32, rate: f64) -> (HgAdvanced<f64>, u32) {
    let mut p = params64(DEFAULT_TIME);
    p.n_min = n_min;
    p.step_budget_rate = rate;
    let mut cad = Counting::default();
    let out =
        march::<Rk4, f64, Counting, ExplicitTest>(&fixture64(), k, &p, &mut cad, &mut ExplicitTest);
    (out, cad.0)
}

#[test]
fn qa_m377_budget() {
    check_allowance(over_allowance::<f64>, over_allowance::<f32>);
    let delta = params64(DEFAULT_TIME).delta_sync;
    // A budget of 0 ends the march after its first step, latched there.
    let (out, followed) = budget_run(8, 0, 0.0);
    assert_eq!(
        (
            out.count.end,
            out.count.steps,
            out.boundary,
            followed,
            out.count.accepted
        ),
        (End::Exhausted, 1, 0, 1, 1),
        "a budget of 0: one step, then Exhausted"
    );
    assert_eq!(
        out.state,
        out.system.image(&out.q, &out.p),
        "the exhausted state latched"
    );
    assert_eq!(
        out.t.to_bits(),
        out.q.t.min(delta).to_bits(),
        "the exhausted time"
    );
    // The steps to boundary 1 and to boundary 2 under a budget that does not bind.
    let (one, _) = budget_run(1, 4_000_000, 0.0);
    let (two, _) = budget_run(2, 4_000_000, 0.0);
    let (s1, s2) = (one.count.steps, two.count.steps);
    assert_eq!((one.boundary, two.boundary), (1, 2));
    // N_min at half of interval 1's steps ends the march inside interval 1, latched; nothing follows it.
    let (out, followed) = budget_run(8, s1 / 2, 0.0);
    assert_eq!(
        (out.count.end, out.boundary),
        (End::Exhausted, 0),
        "N_min = s₁/2 binds in interval 1"
    );
    assert_eq!(
        followed, out.count.accepted,
        "a step followed after the budget ended the march"
    );
    assert!(
        out.count.steps > s1 / 2 && out.count.steps <= s1 / 2 + 1 + 4 + 64,
        "exhausted at its first excess"
    );
    // The allowance reads the time since t = 0: at boundary 1 it is rate·Δ_sync = 2·s₁, above the s₁ steps taken, so
    // interval 2 runs; read on the interval clock it would be N_min = s₁/2 there, and end the march.
    let rate = 2.0 * f64::from(s1) / delta;
    assert!(
        s2 <= 3 * s1,
        "fixture: interval 2 takes no more than twice interval 1's steps"
    );
    let (out, _) = budget_run(2, s1 / 2, rate);
    assert_eq!(
        (out.count.end, out.boundary),
        (End::Marching, 2),
        "the allowance read on the interval clock, not the time since t = 0"
    );
}

negative_control!(
    qa_m377_budget,
    "a helper over at equality",
    expected = "steps against max",
    check_allowance(
        |s, t, r, n| f64::from(s) >= (r * t).max(f64::from(n)),
        |s, t, r, n| s as f32 >= (r * t).max(n as f32),
    )
);

// ── qa_m377_non_finite ───────────────────────────────────────────────────────────────────────────────────────────

/// Which component the poisoned stepper sets to NaN.
#[derive(Clone, Copy, Debug)]
enum Poison {
    Q,
    P,
    Clock,
}

thread_local! {
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static POISON: Cell<Option<Poison>> = const { Cell::new(None) };
}

/// RK4 under Heggie, whose sixth step and every later one puts a NaN in the poisoned component.
struct Poisoned;

impl HgStepper for Poisoned {
    fn pair_profile() -> CapabilityProfile {
        Form::Rk4.pair_profile()
    }

    fn regularised_step<R: AzReal>(
        sys: &HgSystem<R>,
        q: &HgQ<R>,
        p: &HgP<R>,
        h: R,
    ) -> (HgQ<R>, HgP<R>) {
        let (mut q1, mut p1) = <Rk4 as HgStepper>::regularised_step(sys, q, p, h);
        let n = CALLS.get() + 1;
        CALLS.set(n);
        if n > 5 {
            let nan = R::default() / R::default();
            match POISON.get() {
                Some(Poison::Q) => q1.q[1][0] = nan,
                Some(Poison::P) => p1.p[2][1] = nan,
                Some(Poison::Clock) => q1.t = nan,
                None => {}
            }
        }
        (q1, p1)
    }
}

fn finite64(s: &State<f64>) -> bool {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .all(|x| x.is_finite())
}

fn check_non_finite(skip_clock: bool) {
    // Two bodies coincident: R₂ = 0, so the entry dt/dτ is 0 and the march ends before a step.
    let mut s = fixture64();
    s.r[1] = s.r[0];
    for time in [HeggieTime::Eq20, HeggieTime::Eq22] {
        let mut p = params64(time);
        p.n_min = 50;
        let out = march::<Rk4, f64, Counting, ExplicitTest>(
            &s,
            3,
            &p,
            &mut Counting::default(),
            &mut ExplicitTest,
        );
        assert_eq!(
            (
                out.count.end,
                out.count.non_finite_entries,
                out.count.steps,
                out.boundary
            ),
            (End::NonFinite, 1, 0, 0),
            "{time:?}: a zero entry dt/dτ"
        );
    }
    // A NaN mid-interval in each component: the march ends NonFinite, latched at the last finite state.
    for poison in [Poison::Q, Poison::P, Poison::Clock] {
        CALLS.set(0);
        POISON.set(Some(poison));
        let mut watch = Recorder::new();
        watch.skip_clock = skip_clock;
        let mut p = params64(DEFAULT_TIME);
        p.n_min = 50;
        let mut cad = Counting::default();
        let out = march::<Poisoned, f64, Counting, Recorder<f64>>(
            &fixture64(),
            3,
            &p,
            &mut cad,
            &mut watch,
        );
        POISON.set(None);
        assert_eq!(
            (out.count.end, out.boundary),
            (End::NonFinite, 0),
            "a NaN in {poison:?}: not ended by the finiteness test"
        );
        assert!(
            finite64(&out.state) && out.q.t.is_finite(),
            "{poison:?}: the latched state not finite"
        );
        let last = watch.kept.last().expect("a kept state");
        assert_eq!(
            (out.q, out.p),
            *last,
            "{poison:?}: latched at a state other than the last finite one"
        );
        assert_eq!(
            cad.0, 5,
            "{poison:?}: the cadence followed a step after the NaN"
        );
    }
}

#[test]
fn qa_m377_non_finite() {
    check_non_finite(false);
}

negative_control!(
    qa_m377_non_finite,
    "a finiteness test that does not read the clock",
    expected = "not ended by the finiteness test",
    check_non_finite(true)
);
