//! The Inspector's RK45 reference (`engine::precision::rk45`): Dormand–Prince 5(4) in f64, adaptive, held
//! outside the occupant system.
//!
//! Pass criteria, fixed before the first run:
//! - **The figure-eight closes** (the repository's anchor orbit; `engine::trajectory::FigureEight`, its start and its
//!   period `T₈` in the canonical gauge): at `rtol = atol = 10⁻¹²`, the run reaches `T₈`, and the largest position
//!   component's distance from the start is at most `10⁻⁶`, the relative energy error at most `10⁻⁹`. The bound is
//!   the fixture's, not the integrator's: the start and the period are published to 8 and 9 significant digits, so the
//!   orbit they give misses closing by about `10⁻⁸` before any step is taken, and the orbit is linearly stable, so that
//!   miss is not amplified by more than a decade or two in one period.
//! - **The tableau is fifth order with a fourth-order embedded pair**: fixed steps from the figure-eight's start over
//!   `t = 0.5` at `h = 0.025` and `0.0125`, against the same scheme at `h/32`: the propagated solution's measured
//!   order `log₂(err(h)/err(h/2))` at least 4.5, the embedded one's at least 3.5; and the rows sum to the nodes and both
//!   weight sets to 1 within `4ε`.
//! - **A landing clipped to a sliver is taken, not called an underflow**: asked for `[0.5, 0.5 + 4 ulp]` from the
//!   figure-eight's start, the second step is clipped to 4 ulp (`4.4·10⁻¹⁶`, below the underflow floor `16ε`); the
//!   run ends `Reached` with both states, the second within `10⁻¹⁴` of the first.
//! - **A step the control shrank is still an underflow**: the drop below asked only for `t = 3`, far past the
//!   collision, so no step near the collision is a landing; the run ends `Underflow`, with no state and its step budget
//!   (10⁵) unspent.
//! - **It stops at a collision and reports only what it reached**: two bodies dropped from rest collide at
//!   `t_c = (π/2)·√(d³/(2M))` (≈ 1.36 here); asked for `[0.5, 3]`, the run ends short of 3 (`Underflow` or
//!   `Exhausted`) with exactly one state, the one at 0.5.

use kernel::occupant::State;

use engine::precision::rk45::{
    self, fixed_step, integrate, integrate_with_rule, underflowed, Rk45End, Rk45Params, Tableau,
};
use engine::trajectory::FigureEight;
use validation::negative_control;

fn start() -> State<f64> {
    let ic = FigureEight::new().start();
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}

fn tight() -> Rk45Params {
    Rk45Params {
        rtol: 1e-12,
        atol: 1e-12,
        max_steps: 1_000_000,
    }
}

fn position_distance(a: &State<f64>, b: &State<f64>) -> f64 {
    a.r.iter()
        .flatten()
        .zip(b.r.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

fn distance(a: &State<f64>, b: &State<f64>) -> f64 {
    a.r.iter()
        .chain(&a.p)
        .flatten()
        .zip(b.r.iter().chain(&b.p).flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

// ── The figure-eight closes ──────────────────────────────────────────────────────────────────────────────────────

fn check_closure(fraction: f64) {
    let f = FigureEight::new();
    let s0 = start();
    let t = fraction * engine::trajectory::Provider::quantum(&f);
    let run = integrate(&s0, &[t], &tight());
    assert_eq!(
        run.end,
        Rk45End::Reached,
        "the reference did not reach the period"
    );
    let end = &run.states[0];
    let closure = position_distance(end, &s0);
    let e0 = rk45::energy(&s0);
    let drift = ((rk45::energy(end) - e0) / e0).abs();
    println!(
        "figure-eight over {fraction}·T₈ = {t}: closure {closure:.3e}, |ΔE/E| {drift:.3e}, {} accepted, {} rejected",
        run.accepted, run.rejected
    );
    assert!(
        drift <= 1e-9,
        "the energy drifted {drift:.3e} over the period"
    );
    assert!(
        closure <= 1e-6,
        "the figure-eight does not close: {closure:.3e} from its start"
    );
}

#[test]
fn rk45_figure_eight_closure() {
    check_closure(1.0);
}

negative_control!(
    rk45_figure_eight_closure,
    "one per cent short of the period the orbit is visibly open",
    expected = "does not close",
    check_closure(0.99)
);

// ── The order of the tableau ─────────────────────────────────────────────────────────────────────────────────────

/// `n` fixed steps of `h`, the propagated and the embedded solutions each marched on their own.
fn fixed(tableau: &Tableau, h: f64, n: u32) -> (State<f64>, State<f64>) {
    let (mut high, mut low) = (start(), start());
    for _ in 0..n {
        high = fixed_step(tableau, &high, h).0;
        low = fixed_step(tableau, &low, h).1;
    }
    (high, low)
}

fn check_order(tableau: &Tableau) {
    for (i, row) in tableau.a.iter().enumerate() {
        let sum: f64 = row.iter().sum();
        assert!(
            (sum - tableau.c[i]).abs() <= 4.0 * f64::EPSILON,
            "row {i} sums to {sum}, not its node {}",
            tableau.c[i]
        );
    }
    for (name, w) in [("propagated", tableau.b), ("embedded", tableau.e)] {
        let sum: f64 = w.iter().sum();
        assert!(
            (sum - 1.0).abs() <= 4.0 * f64::EPSILON,
            "the {name} weights sum to {sum}"
        );
    }
    let span = 0.5;
    let h = 0.025;
    let n = (span / h) as u32;
    let (fine_high, fine_low) = fixed(tableau, h / 32.0, n * 32);
    let (h1, l1) = fixed(tableau, h, n);
    let (h2, l2) = fixed(tableau, h / 2.0, 2 * n);
    let high = (distance(&h1, &fine_high) / distance(&h2, &fine_high)).log2();
    let low = (distance(&l1, &fine_low) / distance(&l2, &fine_low)).log2();
    println!("tableau: propagated order {high:.2}, embedded order {low:.2}");
    assert!(
        high >= 4.5,
        "the propagated solution's order {high:.2} is below 4.5"
    );
    assert!(
        low >= 3.5,
        "the embedded solution's order {low:.2} is below 3.5"
    );
}

#[test]
fn rk45_dormand_prince_order() {
    assert!(
        Tableau::dormand_prince().fsal(),
        "Dormand–Prince is first-same-as-last"
    );
    check_order(&Tableau::dormand_prince());
}

negative_control!(
    rk45_dormand_prince_order,
    "a propagated weight moved by 10⁻³ and the last one moved back, so the weights still sum to 1",
    expected = "propagated solution's order",
    {
        let mut t = Tableau::dormand_prince();
        t.b[2] += 1e-3;
        t.b[3] -= 1e-3;
        check_order(&t)
    }
);

// ── A collision stops it ─────────────────────────────────────────────────────────────────────────────────────────

/// Bodies 0 and 1 at rest a unit apart, body 2 at rest ten units off: 0 and 1 collide at about 1.36.
fn drop() -> State<f64> {
    let third = 1.0 / 3.0;
    State {
        m: [third; 3],
        r: [[-0.5, 0.0], [0.5, 0.0], [0.0, 10.0]],
        p: [[0.0; 2]; 3],
    }
}

fn check_stops(times: &[f64]) {
    let run = integrate(&drop(), times, &tight());
    println!(
        "drop to {times:?}: {:?} after {} accepted, {} rejected, {} states",
        run.end,
        run.accepted,
        run.rejected,
        run.states.len()
    );
    assert!(
        matches!(run.end, Rk45End::Underflow | Rk45End::Exhausted),
        "the reference ran through the collision: {:?}",
        run.end
    );
    assert_eq!(
        run.states.len(),
        1,
        "the reference reports a time past the collision"
    );
}

#[test]
fn rk45_stops_at_collision() {
    check_stops(&[0.5, 3.0]);
}

negative_control!(
    rk45_stops_at_collision,
    "asked only for times before the collision, it reaches them",
    expected = "ran through the collision",
    check_stops(&[0.5, 1.0])
);

// ── The underflow rule ───────────────────────────────────────────────────────────────────────────────────────────

/// The rule before `a7a5a11`: any step at or below the floor, a clipped landing included.
#[cfg(feature = "controls")]
fn old_rule(h_try: f64, t: f64, _landing: bool) -> bool {
    h_try <= 16.0 * f64::EPSILON * t.abs().max(1.0)
}

/// A rule that never fires.
#[cfg(feature = "controls")]
fn never(_h_try: f64, _t: f64, _landing: bool) -> bool {
    false
}

/// `t` moved up by `n` ulps.
fn ulps_above(t: f64, n: u32) -> f64 {
    (0..n).fold(t, |x, _| f64::from_bits(x.to_bits() + 1))
}

fn check_sliver_landing(rule: fn(f64, f64, bool) -> bool) {
    let t1 = 0.5;
    let t2 = ulps_above(t1, 4);
    assert!(
        t2 - t1 <= 16.0 * f64::EPSILON,
        "the sliver is not below the underflow floor"
    );
    let run = integrate_with_rule(
        &Tableau::dormand_prince(),
        &start(),
        &[t1, t2],
        &tight(),
        rule,
    );
    println!(
        "sliver landing [{t1}, {t2:e}] (step {:.2e}): {:?}, {} states, {} accepted",
        t2 - t1,
        run.end,
        run.states.len(),
        run.accepted
    );
    assert_eq!(
        run.end,
        Rk45End::Reached,
        "a landing clipped to a sliver ends the run as {:?}",
        run.end
    );
    assert_eq!(
        run.states.len(),
        2,
        "the run reports {} of its 2 times",
        run.states.len()
    );
    let d = distance(&run.states[0], &run.states[1]);
    assert!(d <= 1e-14, "the sliver moved the state by {d:.3e}");
}

#[test]
fn rk45_sliver_landing_is_taken() {
    check_sliver_landing(underflowed);
}

negative_control!(
    rk45_sliver_landing_is_taken,
    "the old rule calls the sliver landing an underflow",
    expected = "a landing clipped to a sliver ends the run as Underflow",
    check_sliver_landing(old_rule)
);

fn check_shrunk_underflow(rule: fn(f64, f64, bool) -> bool) {
    let params = Rk45Params {
        max_steps: 100_000,
        ..tight()
    };
    let run = integrate_with_rule(&Tableau::dormand_prince(), &drop(), &[3.0], &params, rule);
    println!(
        "drop to [3.0]: {:?} after {} accepted, {} rejected",
        run.end, run.accepted, run.rejected
    );
    assert_eq!(
        run.end,
        Rk45End::Underflow,
        "a step the control shrank ends the run as {:?}",
        run.end
    );
    assert!(
        run.states.is_empty(),
        "the run reports a time past the collision"
    );
    assert!(
        run.accepted + run.rejected < params.max_steps,
        "the budget, not the underflow, stopped the run"
    );
}

#[test]
fn rk45_shrunk_step_underflows() {
    check_shrunk_underflow(underflowed);
}

negative_control!(
    rk45_shrunk_step_underflows,
    "a rule that never fires runs the collision to the budget",
    expected = "a step the control shrank ends the run as Exhausted",
    check_shrunk_underflow(never)
);
