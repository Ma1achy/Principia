//! logH (`kernel::regularisation::logh`; integrator_contract Part 2b § "logH and TTL: the equations, the time
//! transformations, the map and the step control (transcribed)"). Each criterion below was written before its first
//! run.
//! - `logh_rates_fd`, `logh_ttl_rates_fd`: the equations of motion against a central difference, over random states
//!   with `B` (logH) and `W` (TTL) drawn independently of the state, so the two denominators differ. logH's against
//!   `Λ = ln(K + B) − ln U`, `dt/ds` against `∂Λ/∂B`; TTL's, which has no Hamiltonian of the state alone, against
//!   `K − U` under its two denominators and `dW/ds` against `Ω` differenced along the velocities. Controls: a momentum
//!   rate's sign flipped, and `dW/ds`'s.
//! - `logh_leapfrog_denominators`: the leapfrog's drift and kick move `(r, t)` and `p` at the rates above, on the same
//!   states off the solution, so each divides by its own denominator. Control: the two denominators swapped.
//! - `logh_reversed_involution`: the time-reversal map negates `p` and `t`, keeps `r` and `W`, and is an involution.
//!   Control: a map that negates `W` too.
//! - `logh_step_reversible`: dd_integrator §5's test 3 on logH's leapfrog in each time mode, through the map, the
//!   residual on the full state `(r, t, p, W)`, at the step-reversal criterion inside its domain, at f64 and f32; each
//!   pair's profile flag against its measured verdict. Control: the same measurement on RK4 reads not reversible.
//! - `logh_march_figure_eight`, `logh_march_close_passage`: a direct march lands on the requested physical time, its
//!   state's own clock the boundary's, holds the energy, and its state is the known one at that time (the figure-eight
//!   after one period; an `e = 0.99` binary after one period). Control: a stepper whose clock runs twice as fast.
//! - `logh_landing_state`: every landing leaves the state's own clock on the boundary, bitwise. Control: the
//!   clock-only landing, the retakes off.
//! - `logh_clamped_step_lands`: the leapfrog's clamped step from a clock one to four ulps short of a boundary ends on
//!   the boundary, bitwise. Control: the leapfrog with the clock advanced by each half-drift in turn.
//! - `logh_march_non_finite`, `logh_march_exhausted`: the march ends not finite (at entry and mid-interval, its state
//!   latched finite) and exhausted apart. Controls: the finiteness test removed, and a budget that does not bind.

use kernel::driver::advance::Cadence;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::physics::PairGeom;
use kernel::regularisation::az::{AzReal, End, ExplicitTest};
use kernel::regularisation::form::Flow;
use kernel::regularisation::logh::{
    drift, kick, leapfrog, march, profile, register, reversed, step_size, LhAdvanced, LhP,
    LhParams, LhQ, LhRegistered, LhStepper, LhSystem, LhWatch,
};
use kernel::regularisation::pairs::LogHMode;
use validation::drift::{FIGURE_EIGHT_V3, FIGURE_EIGHT_X1};
use validation::{
    negative_control, STEP_REVERSAL_EDGE_F32, STEP_REVERSAL_EDGE_F64, STEP_REVERSAL_EDGE_MAX,
    STEP_REVERSAL_TOL,
};

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// `x` at `R`'s width.
fn v<R: AzReal>(x: f64) -> R {
    R::at_width(x as f32, x)
}

/// The floor the tests run at, far below any value they meet.
fn tiny<R: AzReal>() -> R {
    R::at_width(1e-37, 1e-300)
}

/// Test values for the step control: `η`, `Δ_sync`, the predictive limit's `f`, a budget that does not bind, and the
/// landing in force (the clamp, the secant's 4 retakes and the bisection's 64 halvings), under `mode`. Test inputs,
/// not calibrations.
fn params(mode: LogHMode, eta: f64, delta_sync: f64, predictive_f: f64) -> LhParams<f64> {
    LhParams {
        eta,
        delta_sync,
        tiny: tiny(),
        predictive_f,
        step_budget_rate: 1e7,
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        mode,
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

/// Whether every position and momentum of `s` is finite.
fn finite_state(s: &State<f64>) -> bool {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .all(|x| x.is_finite())
}

/// The largest component of `a − b` over the positions and momenta of `bodies`, relative to the largest of `b`'s.
fn miss(a: &State<f64>, b: &State<f64>, bodies: &[usize]) -> f64 {
    let flat = |s: &State<f64>| -> Vec<f64> {
        bodies
            .iter()
            .flat_map(|&i| [s.r[i][0], s.r[i][1], s.p[i][0], s.p[i][1]])
            .collect()
    };
    let (x, y) = (flat(a), flat(b));
    let scale = y.iter().fold(0.0f64, |m, c| m.max(c.abs()));
    x.iter()
        .zip(&y)
        .fold(0.0f64, |m, (p, q)| m.max((p - q).abs()))
        / scale
}

/// logH × `S` from `s0` to boundary `k`.
fn run<S: LhStepper>(s0: &State<f64>, k: u32, p: &LhParams<f64>) -> LhAdvanced<f64> {
    march::<S, f64, Quiet, ExplicitTest>(s0, k, p, &mut Quiet, &mut ExplicitTest)
}

// ── The rates against a finite difference ──────────────────────────────────────────────────────────────────────

/// A fixed sequence of uniform draws.
struct Draws(u64);

impl Draws {
    fn next(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        lo + (hi - lo) * ((self.0 >> 11) as f64 / (1u64 << 53) as f64)
    }

    /// A random state off the solution: unequal masses in `[0.3, 3]`, the flat state `x` with positions in
    /// `[−1.2, 1.2]`, every pair at least `0.3` apart, and momenta in `[−1, 1]`, and `B` (logH) or `W` (TTL) in
    /// `[0.5, 3]`, drawn independently of the state.
    fn state(&mut self) -> ([f64; 3], Vec<f64>, f64) {
        let m = [
            self.next(0.3, 3.0),
            self.next(0.3, 3.0),
            self.next(0.3, 3.0),
        ];
        let mut x: Vec<f64> = (0..6).map(|_| self.next(-1.2, 1.2)).collect();
        while separations(&x).iter().any(|&s| s < 0.3) {
            x = (0..6).map(|_| self.next(-1.2, 1.2)).collect();
        }
        x.extend((0..6).map(|_| self.next(-1.0, 1.0)));
        let extra = self.next(0.5, 3.0);
        (m, x, extra)
    }
}

/// The three pair separations of the flat state `x = (r₀, r₁, r₂, p₀, p₁, p₂)`, pairs `(0, 1)`, `(0, 2)`, `(1, 2)`.
fn separations(x: &[f64]) -> [f64; 3] {
    let d = |i: usize, j: usize| (x[2 * i] - x[2 * j]).hypot(x[2 * i + 1] - x[2 * j + 1]);
    [d(0, 1), d(0, 2), d(1, 2)]
}

/// `K = Σ |pᵢ|²/(2mᵢ)` of the flat state, written here, not read from the kernel.
fn kinetic(x: &[f64], m: [f64; 3]) -> f64 {
    (0..3)
        .map(|i| (x[6 + 2 * i].powi(2) + x[7 + 2 * i].powi(2)) / (2.0 * m[i]))
        .sum()
}

/// `U = Σ mᵢmⱼ/rᵢⱼ` of the flat state.
fn potential(x: &[f64], m: [f64; 3]) -> f64 {
    let d = separations(x);
    m[0] * m[1] / d[0] + m[0] * m[2] / d[1] + m[1] * m[2] / d[2]
}

/// `Ω = m̄²·Σ 1/rᵢⱼ` of the flat state.
fn omega(x: &[f64], m: [f64; 3]) -> f64 {
    let mean = (m[0] + m[1] + m[2]) / 3.0;
    separations(x).iter().map(|d| mean * mean / d).sum()
}

/// The system and state at the flat state `x` with masses `m`: `extra` is `B` in the logH mode and `W` in the TTL
/// mode, whichever the mode's drift divides by.
fn at(x: &[f64], m: [f64; 3], extra: f64, mode: LogHMode) -> (LhSystem<f64>, LhQ<f64>, LhP<f64>) {
    let pair = |c: usize| [x[c], x[c + 1]];
    (
        LhSystem::new(m, extra, tiny(), mode),
        LhQ {
            r: [pair(0), pair(2), pair(4)],
            t: 0.0,
        },
        LhP {
            p: [pair(6), pair(8), pair(10)],
            w: extra,
        },
    )
}

/// The central difference of `f` along component `c` of `x`, at the step `1e-5·max(|x_c|, 1)`.
fn central(f: impl Fn(&[f64]) -> f64, x: &[f64], c: usize) -> f64 {
    let h = 1e-5 * x[c].abs().max(1.0);
    let (mut up, mut down) = (x.to_vec(), x.to_vec());
    up[c] += h;
    down[c] -= h;
    (f(&up) - f(&down)) / (up[c] - down[c])
}

/// The largest discrepancy, over 32 random states under `mode` ([`Draws::state`]), between the analytic rates and the
/// central differences, over the largest analytic rate. `flip` flips one rate's sign, the control's: `dp₀ₓ/ds` in the
/// logH mode, `dW/ds` in the TTL mode.
fn rates_discrepancy(mode: LogHMode, flip: bool) -> f64 {
    let mut d = Draws(11);
    let mut worst = 0.0f64;
    for _ in 0..32 {
        let (m, x, extra) = d.state();
        let (s, q, p) = at(&x, m, extra, mode);
        let (dq, mut dp) = s.rates_of(&q, &p);
        assert_eq!(
            s.rates(&q, &p),
            (dq, dp),
            "the flow's rates are not rates_of"
        );
        assert_eq!(
            (s.q_rate(&q, &p), s.p_rate(&q, &p)),
            (dq, dp),
            "the flow's rates apart"
        );
        if flip {
            match mode {
                LogHMode::LogH => dp.p[0][0] = -dp.p[0][0],
                LogHMode::Ttl => dp.w = -dp.w,
            }
        }
        // (dr/ds, dp/ds, dt/ds, dW/ds), fourteen numbers.
        let mut analytic: Vec<f64> = dq.r.iter().flatten().copied().collect();
        analytic.extend(dp.p.iter().flatten());
        analytic.push(dq.t);
        analytic.push(dp.w);
        let fd: Vec<f64> = match mode {
            LogHMode::LogH => {
                // Λ(r, p; B) = ln(K + B) − ln U: dr/ds = ∂Λ/∂p, dp/ds = −∂Λ/∂r, dt/ds = ∂Λ/∂B, and W is held.
                let lambda = |y: &[f64], b: f64| (kinetic(y, m) + b).ln() - potential(y, m).ln();
                let mut fd: Vec<f64> = (6..12)
                    .map(|c| central(|y| lambda(y, extra), &x, c))
                    .collect();
                fd.extend((0..6).map(|c| -central(|y| lambda(y, extra), &x, c)));
                let h = 1e-5 * extra;
                fd.push(
                    (lambda(&x, extra + h) - lambda(&x, extra - h)) / ((extra + h) - (extra - h)),
                );
                fd.push(0.0);
                fd
            }
            LogHMode::Ttl => {
                // The physical H = K − U under the two denominators: dr/ds = (∂K/∂p)/W, dp/ds = (∂U/∂r)/Ω,
                // dt/ds = 1/W, and dW/ds = (Σ ∂Ω/∂r·v)/Ω, v = ∂K/∂p.
                let om = omega(&x, m);
                let vel: Vec<f64> = (6..12).map(|c| central(|y| kinetic(y, m), &x, c)).collect();
                let mut fd: Vec<f64> = vel.iter().map(|u| u / extra).collect();
                fd.extend((0..6).map(|c| central(|y| potential(y, m), &x, c) / om));
                fd.push(1.0 / extra);
                fd.push(
                    (0..6)
                        .map(|c| central(|y| omega(y, m), &x, c) * vel[c])
                        .sum::<f64>()
                        / om,
                );
                fd
            }
        };
        let scale = analytic.iter().fold(0.0f64, |a, u| a.max(u.abs()));
        let off = analytic
            .iter()
            .zip(&fd)
            .fold(0.0f64, |a, (u, w)| a.max((u - w).abs()));
        worst = worst.max(off / scale);
    }
    worst
}

/// The criterion, written before the first run: the mode's rates within `1e-7` of the central difference, relative to
/// the largest rate. At the step `1e-5` the difference's truncation is of order `h²/d² ≈ 10⁻⁹` at the least separation
/// `0.3` and its rounding of order `ε/h ≈ 10⁻¹⁰`, so `1e-7` is above both; a flipped sign reads of order 1.
fn check_rates(mode: LogHMode, flip: bool) {
    let d = rates_discrepancy(mode, flip);
    println!("logh rates_fd: {mode:?}: discrepancy {d:.3e} (criterion 1e-7)");
    assert!(
        d < 1e-7,
        "{mode:?}'s rates miss the finite difference by {d:.3e}"
    );
}

#[test]
fn logh_rates_fd() {
    check_rates(LogHMode::LogH, false);
}

negative_control!(
    logh_rates_fd,
    "a momentum rate's sign flipped",
    expected = "rates miss the finite difference",
    check_rates(LogHMode::LogH, true)
);

#[test]
fn logh_ttl_rates_fd() {
    check_rates(LogHMode::Ttl, false);
}

negative_control!(
    logh_ttl_rates_fd,
    "dW/ds's sign flipped",
    expected = "rates miss the finite difference",
    check_rates(LogHMode::Ttl, true)
);

// ── The leapfrog's two denominators ────────────────────────────────────────────────────────────────────────────

/// A drift or a kick: `(system, q, p, h)` to the positions with the clock, or to the momenta with `W`.
type Part<T> = fn(&LhSystem<f64>, &LhQ<f64>, &LhP<f64>, f64) -> T;

/// The drift with the kick's denominator under it: the control's.
#[cfg(feature = "controls")]
fn swapped_drift(s: &LhSystem<f64>, q: &LhQ<f64>, p: &LhP<f64>, h: f64) -> LhQ<f64> {
    drift(
        s,
        q,
        p,
        h * s.drift_raw(p) / s.kick_raw(&PairGeom::new(&q.r)),
    )
}

/// The kick with the drift's denominator under it: the control's.
#[cfg(feature = "controls")]
fn swapped_kick(s: &LhSystem<f64>, q: &LhQ<f64>, p: &LhP<f64>, h: f64) -> LhP<f64> {
    kick(
        s,
        q,
        p,
        h * s.kick_raw(&PairGeom::new(&q.r)) / s.drift_raw(p),
    )
}

/// The criterion, written before the first run: over the 32 states off the solution ([`Draws::state`]) in each mode,
/// a drift of `h = 10⁻³` moves `(r, t)` and a kick of `h` moves `p` by `h` times the equations' rates, within `1e-9` of
/// the largest rate. Both are linear in `h`, so the only difference is the rounding of `(x₁ − x)/h`, of order
/// `ε/h ≈ 2·10⁻¹³`. On the solution the two denominators are one number and a swap is invisible; off it, where `B` and
/// `W` are drawn apart from the state, a swap reads of order 1. `W`'s increment is not read here: it is taken at the
/// midpoint velocity, which the rate at `(r, p)` is not.
///
/// And the leapfrog is these parts: on every state its `(r, p, W)` after a step `h` are those of `drift(h/2)`,
/// `kick(h)`, `drift(h/2)`, bitwise (its clock is not: it adds the two drift times once, the parts in turn). The
/// leapfrog forms its drifts itself, so without this the check above would guard a drift the march does not run.
fn check_parts(drift: Part<LhQ<f64>>, kick: Part<LhP<f64>>) {
    let h = 1e-3;
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let mut d = Draws(11);
        let mut worst = 0.0f64;
        let mut apart = 0;
        for _ in 0..32 {
            let (m, x, extra) = d.state();
            let (s, q, p) = at(&x, m, extra, mode);
            let q_half = drift(&s, &q, &p, h / 2.0);
            let p_kicked = kick(&s, &q_half, &p, h);
            let q_parts = drift(&s, &q_half, &p_kicked, h / 2.0);
            let (q_leap, p_leap) = leapfrog(&s, &q, &p, h);
            apart += u32::from((q_leap.r, p_leap) != (q_parts.r, p_kicked));
            assert!(
                ((s.drift_raw(&p) / s.kick_raw(&PairGeom::new(&q.r))) - 1.0).abs() > 1e-3,
                "the state is on the solution, where a swap cannot be seen"
            );
            let (dq, dp) = s.rates_of(&q, &p);
            let (q1, p1) = (drift(&s, &q, &p, h), kick(&s, &q, &p, h));
            let mut want: Vec<f64> = dq.r.iter().flatten().copied().collect();
            want.extend(dp.p.iter().flatten());
            want.push(dq.t);
            let mut got: Vec<f64> = (0..3)
                .flat_map(|i| [0, 1].map(|c| (q1.r[i][c] - q.r[i][c]) / h))
                .collect();
            got.extend((0..3).flat_map(|i| [0, 1].map(|c| (p1.p[i][c] - p.p[i][c]) / h)));
            got.push((q1.t - q.t) / h);
            let scale = want.iter().fold(0.0f64, |a, u| a.max(u.abs()));
            let off = want
                .iter()
                .zip(&got)
                .fold(0.0f64, |a, (u, w)| a.max((u - w).abs()));
            worst = worst.max(off / scale);
        }
        println!("logh leapfrog_denominators: {mode:?}: discrepancy {worst:.3e} (criterion 1e-9)");
        assert!(
            worst < 1e-9,
            "{mode:?}'s drift and kick miss the rates by {worst:.3e}"
        );
        assert_eq!(
            apart, 0,
            "{mode:?}: the leapfrog is not its drift and kick composed on {apart} of 32 states"
        );
    }
}

#[test]
fn logh_leapfrog_denominators() {
    check_parts(drift, kick);
}

negative_control!(
    logh_leapfrog_denominators,
    "the drift dividing by the kick's denominator and the kick by the drift's",
    expected = "drift and kick miss the rates",
    check_parts(swapped_drift, swapped_kick)
);

// ── The time-reversal map ──────────────────────────────────────────────────────────────────────────────────────

/// `map` on a TTL state one leapfrog step from [`triple`] (so `t ≠ 0` and `W` has moved): applied twice it is the
/// identity, bitwise; applied once it keeps `r` and `W` and negates `p` and `t`.
fn check_map(map: impl Fn(&LhQ<f64>, &LhP<f64>) -> (LhQ<f64>, LhP<f64>)) {
    let LhRegistered { system, q, p } = register(&triple::<f64>(), tiny(), LogHMode::Ttl);
    let (q, p) = <Kdk as LhStepper>::regularised_step(&system, &q, &p, 1e-2);
    assert!(q.t > 0.0 && p.w > 0.0, "the fixture's t and W");
    let (q1, p1) = map(&q, &p);
    assert_eq!(map(&q1, &p1), (q, p), "the map is not an involution");
    assert_eq!(q1.r, q.r, "the map does not keep r");
    assert_eq!(q1.t, -q.t, "the map does not negate t");
    assert_eq!(
        p1.p,
        p.p.map(|x| [-x[0], -x[1]]),
        "the map does not negate p"
    );
    assert_eq!(p1.w, p.w, "the map does not keep W");
}

#[test]
fn logh_reversed_involution() {
    check_map(reversed);
}

negative_control!(
    logh_reversed_involution,
    "a map that negates W too",
    expected = "the map does not keep W",
    check_map(|q, p| {
        let (q1, mut p1) = reversed(q, p);
        p1.w = -p1.w;
        (q1, p1)
    })
);

// ── Step reversibility ─────────────────────────────────────────────────────────────────────────────────────────

/// A precision test 3 runs at.
trait P: AzReal + Into<f64> {
    const EPS: f64;
    /// The criterion's domain's lower edge.
    fn edge() -> f64;
    /// The `dt/τ` the measurement is taken at, inside the domain: `0.1` at f64 and `0.6` at f32.
    fn target() -> f64;
}

impl P for f64 {
    const EPS: f64 = f64::EPSILON;
    fn edge() -> f64 {
        STEP_REVERSAL_EDGE_F64.number()
    }
    fn target() -> f64 {
        0.1
    }
}

impl P for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn edge() -> f64 {
        STEP_REVERSAL_EDGE_F32.number()
    }
    fn target() -> f64 {
        0.6
    }
}

/// One measurement: the pair, its precision, `dt/τ`, the residual in `ε`, and its profile's `step_reversible`.
#[derive(Clone, Debug)]
struct Measured {
    name: &'static str,
    precision: &'static str,
    ratio: f64,
    residual: f64,
    flag: bool,
}

/// Test 3 on logH × `S` in `mode` at `R`, from [`triple`]: `STEP(h)`, `Θ`, `STEP(h)`, `Θ`, the residual the relative
/// distance from the start over the full state `(r, t, p, W)`, in `ε`. `h` is set so that the step's `dt/τ` reads
/// `R`'s target, `τ = √(d³/(mᵢ + mⱼ))` the shortest two-body time over the pairs at the step's two ends.
fn measure<R: P, S: LhStepper>(name: &'static str, mode: LogHMode) -> Measured {
    let LhRegistered { system, q, p } = register(&triple::<R>(), tiny(), mode);
    let flat = |q: &LhQ<R>, p: &LhP<R>| -> Vec<f64> {
        q.r.iter()
            .flatten()
            .chain(std::iter::once(&q.t))
            .chain(p.p.iter().flatten())
            .chain(std::iter::once(&p.w))
            .map(|&x| x.into())
            .collect()
    };
    let masses: [f64; 3] = system.m.map(|x| x.into());
    let tau = |q: &LhQ<R>| -> f64 {
        [(1, 2), (2, 0), (0, 1)]
            .iter()
            .map(|&(i, j)| {
                let (a, b): ([f64; 2], [f64; 2]) =
                    (q.r[i].map(|x| x.into()), q.r[j].map(|x| x.into()));
                let d = (a[0] - b[0]).hypot(a[1] - b[1]);
                (d * d * d / (masses[i] + masses[j])).sqrt()
            })
            .fold(f64::INFINITY, f64::min)
    };
    let once = |h: f64| -> (f64, f64) {
        let h: R = v(h);
        let (q1, p1) = S::regularised_step(&system, &q, &p, h);
        let (qr, pr) = reversed(&q1, &p1);
        let (q2, p2) = S::regularised_step(&system, &qr, &pr, h);
        let (qb, pb) = reversed(&q2, &p2);
        let (x, y) = (flat(&qb, &pb), flat(&q, &p));
        let n = y.iter().map(|a| a * a).sum::<f64>().sqrt();
        let off = x
            .iter()
            .zip(&y)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        let dt: f64 = (q1.t - q.t).into();
        (dt / tau(&q).min(tau(&q1)), off / n / R::EPS)
    };
    let rate: f64 = system.dt_ds(&p).into();
    let mut h = R::target() * tau(&q) / rate;
    for _ in 0..8 {
        h *= R::target() / once(h).0;
    }
    let (ratio, residual) = once(h);
    Measured {
        name,
        precision: std::any::type_name::<R>(),
        ratio,
        residual,
        flag: profile::<S>(mode).step_reversible,
    }
}

/// The criterion, the register's (written before the first run): inside the domain `edge ≤ dt/τ ≤ 1`, the residual at
/// most `STEP_REVERSAL_TOL` (16) `ε` reads reversible.
fn check_reversible(m: &Measured, edge: f64) {
    let tol = STEP_REVERSAL_TOL.number();
    println!(
        "logh_step_reversible at {}: {}: dt/τ {:.3e}, residual {:.3e} ε (criterion {tol} ε)",
        m.precision, m.name, m.ratio, m.residual
    );
    assert!(
        edge <= m.ratio && m.ratio <= STEP_REVERSAL_EDGE_MAX.number(),
        "{}: dt/τ {:.3e} is outside the criterion's domain",
        m.name,
        m.ratio
    );
    assert!(
        m.residual <= tol,
        "{} reads not reversible: {:.3e} ε",
        m.name,
        m.residual
    );
}

/// The leapfrog in each time mode reads reversible, RK4 on the same state at the same `dt/τ` does not, and each
/// pair's profile flag is its measured verdict.
fn reversibility_at<R: P>() {
    let tol = STEP_REVERSAL_TOL.number();
    for (mode, leap, rk4) in [
        (LogHMode::LogH, "logH × KDK", "logH × RK4"),
        (LogHMode::Ttl, "logH-TTL × KDK", "logH-TTL × RK4"),
    ] {
        let m = measure::<R, Kdk>(leap, mode);
        check_reversible(&m, R::edge());
        assert!(m.flag, "{leap}'s profile does not say step-reversible");
        let c = measure::<R, Rk4>(rk4, mode);
        println!(
            "logh_step_reversible at {}: {}: dt/τ {:.3e}, residual {:.3e} ε",
            c.precision, c.name, c.ratio, c.residual
        );
        assert!(
            c.residual > tol,
            "{rk4} reads reversible: {:.3e} ε",
            c.residual
        );
        assert!(!c.flag, "{rk4}'s profile says step-reversible");
    }
}

#[test]
fn logh_step_reversible() {
    reversibility_at::<f64>();
    reversibility_at::<f32>();
}

negative_control!(
    logh_step_reversible,
    "the same measurement on RK4 in the TTL mode",
    expected = "reads not reversible",
    check_reversible(
        &measure::<f64, Rk4>("logH-TTL × RK4", LogHMode::Ttl),
        f64::edge()
    )
);

// ── The march on known fixtures ────────────────────────────────────────────────────────────────────────────────

/// `S`'s step with its clock run twice as fast as its state moves: the control's, the time transformation's clock
/// broken. The step's own increment is read from a copy whose clock is `0` (the rates do not read the clock, so the
/// state is the same): read as `q1.t − q.t` it is a whole number of the clock's ulps, its double cannot close a gap of
/// one ulp, and the control's march stands short of a boundary until its budget is spent, which is not the failure
/// the control is for (measured: both fast-clock controls ended exhausted before this).
#[cfg(feature = "controls")]
fn fast_step<S: LhStepper, R: AzReal>(
    system: &LhSystem<R>,
    q: &LhQ<R>,
    p: &LhP<R>,
    h: R,
) -> (LhQ<R>, LhP<R>) {
    let from_zero = LhQ {
        r: q.r,
        t: R::default(),
    };
    let (mut q1, p1) = S::regularised_step(system, &from_zero, p, h);
    q1.t = q.t + (q1.t + q1.t);
    (q1, p1)
}

/// RK4 with a fast clock ([`fast_step`]).
#[cfg(feature = "controls")]
struct FastRk4;

#[cfg(feature = "controls")]
impl LhStepper for FastRk4 {
    fn pair_profile(mode: LogHMode) -> kernel::occupant::CapabilityProfile {
        Rk4::pair_profile(mode)
    }

    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        fast_step::<Rk4, R>(system, q, p, h)
    }
}

/// The leapfrog with a fast clock ([`fast_step`]).
#[cfg(feature = "controls")]
struct FastKdk;

#[cfg(feature = "controls")]
impl LhStepper for FastKdk {
    fn pair_profile(mode: LogHMode) -> kernel::occupant::CapabilityProfile {
        Kdk::pair_profile(mode)
    }

    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        fast_step::<Kdk, R>(system, q, p, h)
    }
}

/// The march `out` landed on boundary `k` at `k·Δ_sync` with its state's own clock the boundary's, still marching; its
/// energy within `energy_tol` of the registration's, relatively; and its state within `closure_tol` of `known`, the
/// known state at that time, over `bodies`.
fn check_fixture(
    name: &str,
    out: &LhAdvanced<f64>,
    (k, delta): (u32, f64),
    (known, bodies): (&State<f64>, &[usize]),
    (energy_tol, closure_tol): (f64, f64),
) {
    assert_eq!(
        out.count.end,
        End::Marching,
        "{name}: the march did not land: boundary {}, clock {:e}, {:?}",
        out.boundary,
        out.q.t,
        out.count
    );
    assert_eq!(
        (out.boundary, out.t),
        (k, f64::from(k) * delta),
        "{name}: the march is not at the requested time"
    );
    assert_eq!(
        out.q.t.to_bits(),
        delta.to_bits(),
        "{name}: the state's own clock is not the boundary's"
    );
    let e0 = -out.system.b;
    let drift = ((out.energy() - e0) / e0).abs();
    let off = miss(&out.state, known, bodies);
    println!(
        "logh march: {name}: {} steps, |ΔE/E| {drift:.3e} (criterion {energy_tol:.0e}), the state off the known one \
         by {off:.3e} (criterion {closure_tol:.0e}), residual (K+B−U)/U {:.3e}",
        out.count.steps,
        out.system.residual(&out.q, &out.p)
    );
    assert!(drift < energy_tol, "{name}: the energy drifted {drift:.3e}");
    assert!(
        off < closure_tol,
        "{name}: the state at the requested time misses the known one by {off:.3e}"
    );
}

/// The figure-eight's period in `G = 1`, `mᵢ = 1`, the published one (`ledger::fixtures::figure_eight().period`).
const FIGURE_EIGHT_PERIOD: f64 = 6.325_913_98;

/// The figure-eight in its published units, `mᵢ = 1`.
fn figure_eight() -> State<f64> {
    let [x, y] = FIGURE_EIGHT_X1;
    let [vx, vy] = FIGURE_EIGHT_V3;
    State {
        m: [1.0; 3],
        r: [[x, y], [-x, -y], [0.0, 0.0]],
        p: [[-vx / 2.0, -vy / 2.0], [-vx / 2.0, -vy / 2.0], [vx, vy]],
    }
}

/// The figure-eight over one period on `Δ_sync = T/32`, 32 boundaries, the predictive limit off so that the step is
/// the time transformation's alone, under the stepper `S` at `eta`, in each time mode (at equal masses `Ω = U`, so the
/// TTL mode differs from the logH mode only in carrying `W`).
fn figure_eight_runs<S: LhStepper>(name: &str, eta: f64, tols: (f64, f64)) {
    let s0 = figure_eight();
    let delta = FIGURE_EIGHT_PERIOD / 32.0;
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let out = run::<S>(&s0, 32, &params(mode, eta, delta, 0.0));
        check_fixture(
            &format!("figure-eight, {name}, {mode:?}"),
            &out,
            (32, delta),
            (&s0, &[0, 1, 2]),
            tols,
        );
    }
}

/// The criteria, written before the first run. The published initial conditions have eight digits, so the orbit they
/// start returns to its start after `T` to about `10⁻⁷` to `10⁻⁶`, the floor of any closure here.
/// - RK4 at `η = 0.005` (`dt ≈ 10⁻³`, a fourth-order error far below that floor): closure within `1e-5`, energy
///   within `1e-8`.
/// - The leapfrog at `η = 0.002` (`dt ≈ 4·10⁻⁴`, a second-order phase error of order `(ω dt)²·ωT/24 ≈ 10⁻⁶`): closure
///   within `1e-3`, energy within `1e-5`.
///
/// A clock running twice as fast leaves the state at `T/2`, where the bodies are mirrored, a miss of order 1.
#[test]
fn logh_march_figure_eight() {
    figure_eight_runs::<Rk4>("RK4", 0.005, (1e-8, 1e-5));
    figure_eight_runs::<Kdk>("KDK", 0.002, (1e-5, 1e-3));
}

negative_control!(
    logh_march_figure_eight,
    "a stepper whose clock runs twice as fast as its state moves",
    expected = "the state at the requested time misses the known one",
    figure_eight_runs::<FastRk4>("RK4 with a fast clock", 0.005, (1e-8, 1e-5))
);

/// A binary of masses `0.7` and `0.3`, `a = 1`, `e = 0.99` (pericentre separation `0.01`), started at apocentre on
/// the x axis about its centre of mass; the third body, of mass `10⁻¹²`, at rest at distance `10⁴`, so the pair is
/// Keplerian to far below the criteria. Its period is `2π`.
fn binary() -> State<f64> {
    let (a, e) = (1.0f64, 0.99f64);
    let d = a * (1.0 + e);
    let speed = ((1.0 - e) / (a * (1.0 + e))).sqrt();
    State {
        m: [0.7, 0.3, 1e-12],
        r: [[-0.3 * d, 0.0], [0.7 * d, 0.0], [0.0, 1e4]],
        p: [[0.0, -0.21 * speed], [0.0, 0.21 * speed], [0.0, 0.0]],
    }
}

/// The binary over one period on `Δ_sync = T/16`, 16 boundaries, under logH's leapfrog at `η = 10⁻³`, the predictive
/// limit off (the reference records that the limit spends the budget at a collision, and the passage is the time
/// transformation's to resolve), in each time mode; the pair, bodies 0 and 1, against its start.
fn binary_runs<S: LhStepper>(name: &str) {
    let s0 = binary();
    let delta = std::f64::consts::TAU / 16.0;
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let out = run::<S>(&s0, 16, &params(mode, 1e-3, delta, 0.0));
        check_fixture(
            &format!("e = 0.99 binary, {name}, {mode:?}"),
            &out,
            (16, delta),
            (&s0, &[0, 1]),
            (1e-6, 1e-4),
        );
    }
}

/// The criteria, written before the first run: the leapfrog on logH follows a two-body orbit exactly but for a
/// second-order error in time, here at about `3·10⁴` steps a period, so the pair returns to its start within `1e-4`
/// (the time error, of order `10⁻⁸`, moves the apocentre state far less) and its energy holds within `1e-6`. A clock
/// running twice as fast leaves the pair at pericentre, a miss of order 1.
///
/// Measured: the logH mode holds the energy to rounding (`10⁻¹⁴`), the TTL mode to `3·10⁻⁸`. The difference is the
/// third body's: `Ω` weights every pair alike, so its two far pairs add `2w/10⁴` to `w/r₀₁`, and `Ω` is no longer a
/// multiple of `U`; with the third body at `10⁶` and `10⁸` the TTL figure fell to `3·10⁻¹⁰` and `3·10⁻¹²`, as `1/d`,
/// which is how `2w/d` falls.
#[test]
fn logh_march_close_passage() {
    binary_runs::<Kdk>("KDK");
}

negative_control!(
    logh_march_close_passage,
    "a stepper whose clock runs twice as fast as its state moves",
    expected = "the state at the requested time misses the known one",
    binary_runs::<FastKdk>("KDK with a fast clock")
);

// ── The landing ────────────────────────────────────────────────────────────────────────────────────────────────

/// logH × KDK and × RK4 in each time mode from [`triple`] to each of boundaries 1 to 8 (`Δ_sync = 1/8`, `η = 0.01`,
/// `f = 0.02`), the landing's secant at `tries` retakes and its bisection at `halvings`: each lands with the time
/// `k·Δ_sync` and the state's own clock the boundary's, bitwise.
fn check_landings(tries: u32, halvings: u32) {
    fn one<S: LhStepper>(name: &str, mode: LogHMode, k: u32, tries: u32, halvings: u32) {
        let mut p = params(mode, 0.01, 0.125, 0.02);
        p.secant_tries = tries;
        p.bisect_tries = halvings;
        let out = run::<S>(&triple(), k, &p);
        assert_eq!(
            (out.count.end, out.boundary, out.t),
            (End::Marching, k, f64::from(k) * 0.125),
            "{name}, {mode:?}: did not land on boundary {k}"
        );
        assert_eq!(
            out.q.t.to_bits(),
            0.125f64.to_bits(),
            "{name}, {mode:?}, boundary {k}: the state's own clock is not the boundary's: {:e} past",
            out.q.t - 0.125
        );
    }
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        for k in 1..=8 {
            one::<Kdk>("KDK", mode, k, tries, halvings);
            one::<Rk4>("RK4", mode, k, tries, halvings);
        }
    }
}

#[test]
fn logh_landing_state() {
    check_landings(4, 64);
}

negative_control!(
    logh_landing_state,
    "the clock-only landing: no retake, a step past the boundary kept",
    expected = "the state's own clock is not the boundary's",
    check_landings(0, 0)
);

// ── The clamped step at a boundary ─────────────────────────────────────────────────────────────────────────────

/// The leapfrog as the reference composes it, each half-drift adding its own time to the clock: the control's.
#[cfg(feature = "controls")]
struct SplitKdk;

#[cfg(feature = "controls")]
impl LhStepper for SplitKdk {
    fn pair_profile(mode: LogHMode) -> kernel::occupant::CapabilityProfile {
        Kdk::pair_profile(mode)
    }

    fn regularised_step<R: AzReal>(
        system: &LhSystem<R>,
        q: &LhQ<R>,
        p: &LhP<R>,
        h: R,
    ) -> (LhQ<R>, LhP<R>) {
        let half = h / (R::one() + R::one());
        let q_half = drift(system, q, p, half);
        let p1 = kick(system, &q_half, p, h);
        (drift(system, &q_half, &p1, half), p1)
    }
}

/// The criterion, written before the first run: from [`triple`] with its clock set 1, 2, 3 and 4 ulps short of the
/// boundary, the step the march's own clamp sizes ([`step_size`]) ends `S`'s clock on the boundary, bitwise, in each
/// time mode and at two boundaries whose last bits differ (`T/32` of the figure-eight, where the march was measured
/// to stand one ulp short for its whole budget, and the next number up). A clock advanced by two half-increments
/// loses both to rounding at one ulp short where the clock's last bit is even.
fn check_clamped_lands<S: LhStepper>() {
    let boundary = FIGURE_EIGHT_PERIOD / 32.0;
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        for delta in [boundary, f64::from_bits(boundary.to_bits() + 1)] {
            for gap in 1..=4u64 {
                let LhRegistered { system, mut q, p } = register(&triple::<f64>(), tiny(), mode);
                q.t = f64::from_bits(delta.to_bits() - gap);
                let sized = step_size(
                    &system,
                    &q,
                    &p,
                    f64::INFINITY,
                    &params(mode, 0.01, delta, 0.0),
                );
                assert!(sized.h > 0.0, "the clamp's step");
                let (q1, _) = S::regularised_step(&system, &q, &p, sized.h);
                assert_eq!(
                    q1.t.to_bits(),
                    delta.to_bits(),
                    "{mode:?}: the clamped step from {gap} ulp short did not land on the boundary {delta:e}: it \
                     ended {} ulp short",
                    delta.to_bits().wrapping_sub(q1.t.to_bits()) as i64
                );
            }
        }
    }
}

#[test]
fn logh_clamped_step_lands() {
    check_clamped_lands::<Kdk>();
    check_clamped_lands::<Rk4>();
}

negative_control!(
    logh_clamped_step_lands,
    "the leapfrog with the clock advanced by each half-drift in turn",
    expected = "did not land on the boundary",
    check_clamped_lands::<SplitKdk>()
);

// ── The march's ends ───────────────────────────────────────────────────────────────────────────────────────────

/// A watch that reports every state finite: the control's, the explicit test removed.
#[cfg(feature = "controls")]
struct NoTest;

#[cfg(feature = "controls")]
impl<R> LhWatch<R> for NoTest {
    fn finite(&mut self, _q: &LhQ<R>, _p: &LhP<R>) -> bool {
        true
    }
}

/// The march of [`triple`] with its second body's mass set to 0 under `mode`, with the watch `X` and a budget of 5
/// steps. In the TTL mode the entry is finite (`W = Ω`) and the first step's velocity `p/m` is not; in the logH mode
/// `K + B` is not a number at entry.
fn massless<X: LhWatch<f64>>(mode: LogHMode, watch: &mut X) -> LhAdvanced<f64> {
    let mut s = triple::<f64>();
    s.m[1] = 0.0;
    let mut p = params(mode, 0.01, 0.125, 0.02);
    p.n_min = 5;
    p.step_budget_rate = 0.0;
    march::<Kdk, f64, Quiet, X>(&s, 1, &p, &mut Quiet, watch)
}

/// The march ended at the explicit finiteness test, its state latched finite, after `steps` steps.
fn check_non_finite(out: &LhAdvanced<f64>, steps: u32) {
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
fn logh_march_non_finite() {
    // Mid-interval: the first step is not finite, the state latched at the registration's.
    let out = massless(LogHMode::Ttl, &mut ExplicitTest);
    check_non_finite(&out, 1);
    assert_eq!(out.count.non_finite_entries, 0, "a finite entry counted");
    // At entry: dt/ds is not a number, and the march ends before any step.
    let out = massless(LogHMode::LogH, &mut ExplicitTest);
    check_non_finite(&out, 0);
    assert_eq!(
        (out.count.non_finite_entries, out.boundary),
        (1, 0),
        "the entry is counted, the boundary not reached"
    );
}

negative_control!(
    logh_march_non_finite,
    "the march without its finiteness test, which carries the non-finite state on",
    expected = "a non-finite state was carried past the finiteness test",
    check_non_finite(&massless(LogHMode::Ttl, &mut NoTest), 1)
);

/// The march from [`triple`] with a budget of `n_min` steps at rate 0: spent after `n_min + 1` steps, ended
/// [`End::Exhausted`], which is not [`End::NonFinite`], its state finite and latched short of the boundary.
fn check_exhausted(n_min: u32) {
    for mode in [LogHMode::LogH, LogHMode::Ttl] {
        let mut p = params(mode, 0.01, 0.125, 0.02);
        p.n_min = n_min;
        p.step_budget_rate = 0.0;
        let out = run::<Kdk>(&triple(), 1, &p);
        assert_eq!(
            out.count.end,
            End::Exhausted,
            "{mode:?}: the budget was not spent"
        );
        assert_eq!(
            (out.count.steps, out.count.accepted, out.boundary),
            (4, 4, 0),
            "{mode:?}: the budget's steps"
        );
        assert!(
            finite_state(&out.state) && out.t > 0.0 && out.t < 0.125,
            "{mode:?}: the state short of the boundary: t = {}",
            out.t
        );
    }
}

#[test]
fn logh_march_exhausted() {
    check_exhausted(3);
}

negative_control!(
    logh_march_exhausted,
    "a budget that does not bind",
    expected = "the budget was not spent",
    check_exhausted(1_000_000)
);
