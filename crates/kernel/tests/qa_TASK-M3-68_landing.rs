//! QA tests for TASK-M3-68's terminal on a landing step (PR #502), written from integrator contract Part 2a § "The sync
//! grid is a fixed quantum of physical time (R-565)": "a terminal event … still latches at the step it fires on, and
//! the copy's march ends there", not from the implementation:
//! - `qa_m368_done_on_each_landing`: Aarseth–Zare + RK4 marched over four sync boundaries, with a terminal that fires on
//!   the landing step of the first, second and third interval in turn, at f32 and f64. The march ends at that step: its
//!   state is bitwise the landing state of the same march run to that boundary with no terminal, its accepted steps are
//!   that march's, its time is that boundary's (within the landing tolerance), the end is `Done`, and the march never
//!   registered past the landing (re-registrations equal to the boundaries landed on, so no registration of the next
//!   interval was made).
//!
//! The negative control (R-176) runs the same check with a terminal whose `done` is first seen one read late, the
//! landing step's own read missed: the march then registers the next interval before it stops, and the check fails.

#[cfg(feature = "controls")]
use std::cell::Cell;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{march, AzCount, AzParams, AzReal, End, ExplicitTest};
use validation::negative_control;

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

/// The landing tolerance's relative width at `R` (the test values below).
const LAND_EPS: (f32, f64) = (1e-5, 1e-14);

/// Step control values for the test: `η = 0.01`, `Δ_sync = 1/8` (exact in binary), `TINY` `1e-37` / `1e-300`, landing
/// tolerance `1e-5` / `1e-14`, `f = 0.02`, a budget no march here comes near, the landing clamp on.
fn params<R: AzReal>() -> AzParams<R> {
    AzParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(0.125, 0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::at_width(0.02, 0.02),
        r3_floor: R::at_width(0.0, 1e-300),
        land_eps_rel: R::at_width(LAND_EPS.0, LAND_EPS.1),
        step_budget_rate: R::at_width(1e7, 1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
    }
}

/// Three bodies of masses 3, 4, 5 (over 12) on Burrau's triangle (lengths over 5), with small momenta summing to zero.
fn burrau<R: P>() -> State<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    State {
        m: m.map(R::of),
        r: r.map(|x| x.map(R::of)),
        p: p.map(|x| x.map(R::of)),
    }
}

/// The bits of a state's positions and momenta.
fn bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| Into::<f64>::into(x).to_bits())
        .collect()
}

/// A cadence that is never done.
struct Never;

impl<R> Cadence<R> for Never {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// A terminal that fires once `n` steps have been kept: done from the read after the `n`-th step on.
struct FiresAt {
    n: u32,
    kept: u32,
}

impl<R> Cadence<R> for FiresAt {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.kept += 1;
    }
    fn done(&self) -> bool {
        self.kept >= self.n
    }
}

/// A terminal that fires once `n` steps have been kept, but whose first read after firing reports not done (a
/// control's: the landing step's own read missed).
#[cfg(feature = "controls")]
struct SeenLate {
    n: u32,
    kept: u32,
    missed: Cell<bool>,
}

#[cfg(feature = "controls")]
impl<R> Cadence<R> for SeenLate {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.kept += 1;
    }
    fn done(&self) -> bool {
        if self.kept < self.n {
            false
        } else if !self.missed.get() {
            self.missed.set(true);
            false
        } else {
            true
        }
    }
}

/// The terminal cadence under test, built to fire at `n` kept steps.
type Terminal<C> = fn(u32) -> C;

/// For each boundary `j` in 1..=3 at `R`: the march from boundary 0 to 4 with a terminal that fires on interval `j`'s
/// landing step ends at that step, latched (see the module's note).
fn check_each_landing<R: P, C: Cadence<R>>(terminal: Terminal<C>) {
    let s = burrau::<R>();
    let prm = params::<R>();
    let delta: f64 = prm.delta_sync.into();
    let eps = if core::mem::size_of::<R>() == 4 {
        f64::from(LAND_EPS.0)
    } else {
        LAND_EPS.1
    };
    for j in 1..=3u32 {
        let reference = march::<Rk4, R, Never, ExplicitTest>(
            &s,
            0,
            j,
            AzCount::new(),
            &prm,
            &mut Never,
            &mut ExplicitTest,
        );
        assert_eq!(
            (reference.boundary, reference.count.end),
            (j, End::Marching),
            "the fixture lands on boundary {j} with no terminal"
        );
        let n = reference.count.accepted;
        let mut c = terminal(n);
        let out = march::<Rk4, R, C, ExplicitTest>(
            &s,
            0,
            4,
            AzCount::new(),
            &prm,
            &mut c,
            &mut ExplicitTest,
        );
        assert_eq!(
            out.count.re_registrations, j,
            "a terminal on boundary {j}'s landing step ends the march there, no later interval registered"
        );
        assert_eq!(out.count.end, End::Done, "the march ended on the terminal");
        assert_eq!(
            out.count.accepted, n,
            "no step is kept past the landing step the terminal fired on (boundary {j})"
        );
        assert_eq!(
            bits(&out.state),
            bits(&reference.state),
            "the latched state is boundary {j}'s landing state"
        );
        let t: f64 = out.t.into();
        let at = f64::from(j) * delta;
        assert!(
            (t - at).abs() <= delta * eps * 2.0,
            "the latched time {t} is boundary {j}'s, {at}, within the landing tolerance"
        );
    }
}

#[test]
fn qa_m368_done_on_each_landing() {
    check_each_landing::<f64, FiresAt>(|n| FiresAt { n, kept: 0 });
    check_each_landing::<f32, FiresAt>(|n| FiresAt { n, kept: 0 });
}

negative_control!(
    qa_m368_done_on_each_landing,
    "the terminal's done seen one read late, the landing step's read missed",
    expected = "ends the march there, no later interval registered",
    check_each_landing::<f64, SeenLate>(|n| SeenLate {
        n,
        kept: 0,
        missed: Cell::new(false),
    })
);
