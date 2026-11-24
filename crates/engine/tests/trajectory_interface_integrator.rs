//! The trajectory interface's tests re-run with the default integrator as provider (TASK-M3-67): Heggie × RK4 at f32,
//! the field's own march, on the fixed sync quantum `Δ_sync = 1/8` ([`Integrated`]). Every test is named
//! `trajectory_interface_integrator_*`, each pass criterion written here before its run:
//! - `_rows_read_the_march`: a row reads the provider's state bit for bit; row 0 is the start; row `k` is the state the
//!   field's march to boundary `k` stands on, widened exactly; the quantum is `Δ_sync` at every horizon.
//! - `_rows_on_one_march`: row `k` is, bit for bit, the state the one march to the horizon lands on at boundary `k` (the
//!   step's image as the march hands it to its cadence, before the cadence's projection of its own copy, which the march
//!   never carries on): the march to `k` is the prefix of the longer one. Control: a provider whose rows read between
//!   boundaries, at `(k + ½)·Δ_sync`.
//! - `_lagrange_closed_form`: on the Lagrange circular member (its start held at f32), rows 1 to 16 (`t ≤ 2`) each
//!   within `min(N·2⁻²⁴, 10⁻⁴)` of the closed form at `t = k·Δ_sync`, per component, per unit of the start's same
//!   vector's `|v_x| + |v_y|`, `N` the steps the march to `t = 2` takes: f32's unit roundoff accumulated at most
//!   linearly over them (a model, not a proof), never past the fixed `10⁻⁴`, so a march that takes more steps does not
//!   loosen its own bound. (Written first as `10⁻⁴` with `N ≤ 1600` asserted; the first run took `N = 1616` and
//!   stopped on that assertion before any row was compared, and the `N` cap became the formula it came from.) RK4's
//!   truncation at `η = 0.01` and the start's rounding to f32 (`2⁻²⁴` relative, growing at most `e^{ωt} < 3` by `t = 2`
//!   on the unstable equilateral orbit, `ω ≈ 0.41`) are each far below it. Half a boundary is a turn of
//!   `ω·Δ/2 ≈ 0.026`, over 200 times the bound. Control: the provider whose rows read between boundaries.
//! - `_rows_differ`: the circular member's rows 0 to 16 differ pairwise, so a check that tells boundaries apart can
//!   fire on them.
//! - `_terminal`: the radial collision passage (bodies 0 and 1 fall head-on from rest, separation 1, masses `1/3`)
//!   collides at `t = (π/2)·√(d³/(2G(m₀ + m₁))) ≈ 1.360`, slightly earlier with `r_coll = 10⁻³`: the terminal is a
//!   collision of pair 2 (the pair opposite body 2) at index 11, the first boundary at or after `1.25 < t ≤ 1.375`;
//!   rows 11 to 16 hold the latched state bit for bit, rows 0 to 11 differ pairwise, and the field's outcome for the
//!   start is the terminal's class. Control: a terminal expected one boundary early.
//! - `_reads_order_free`: two reads in either order, and after other reads, give one bit pattern.
//! - `_refusals`: a start an f32 does not hold, a row past the cap, a refused horizon; the cap is the field's
//!   (`T = 0.4` boundary 3, `T = 50` and `50.1` boundary 400). Control: a provider that takes any f64 start.
//! - `_latched_index`: the terminal's index from where the march latched, decided on the interval clock: a latch
//!   `10⁻⁷` past boundary 399 (under half an f32 ulp of `399·Δ_sync = 49.875`) is at index 400; a latch on a counted
//!   landing at 399; a budget spent on the landing step, which leaves it uncounted, at 400; a latch at an interval's
//!   entry at 399. Control: the index from the absolute time `k·Δ_sync + clock` at f32, which rounds the first back onto
//!   boundary 399.

use engine::field::{heggie_params, marched, Outcome};
use engine::first_slice::narrow;
use engine::trajectory::{
    latched_index, same_bits, Integrated, Lagrange, Member, Provider, Refused, Trajectory,
};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State as KState};
use kernel::payload::SimStateBaseOf;
use kernel::regularisation::az::{End, ExplicitTest};
use kernel::regularisation::heggie::{march, HgParams};
use ledger::display::{Detail, State};
use validation::negative_control;

/// The horizon the tests read to: `t = 2`, boundary 16.
const HORIZON: f64 = 2.0;

fn provider() -> Integrated {
    Integrated::new(HORIZON).expect("the horizon is dispatched")
}

/// The default collision threshold, as the field reads it from the sim key.
fn radius() -> CollisionRadius {
    engine::first_slice::uniforms(HORIZON)
        .expect("the horizon")
        .radius
}

/// The provider whose row `k` reads between boundaries: the field's march on a grid of `Δ_sync/2`, read at `2k + 1`,
/// the time `(k + ½)·Δ_sync`, while it declares `Δ_sync`.
struct Between {
    half: Integrated,
}

impl Between {
    #[allow(dead_code)]
    fn new() -> Self {
        let params = HgParams {
            delta_sync: heggie_params().delta_sync / 2.0,
            ..heggie_params()
        };
        Self {
            half: Integrated::with(params, radius(), false, HORIZON + 1.0).expect("a grid"),
        }
    }
}

impl Provider for Between {
    fn quantum(&self) -> f64 {
        2.0 * self.half.quantum()
    }
    fn terminal(&self, start: &Ic<f64>) -> Result<Option<engine::trajectory::Terminal>, Refused> {
        self.half.terminal(start)
    }
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        self.half.state(start, 2 * k + 1)
    }
}

fn circular() -> Lagrange {
    let f = ledger::fixtures::lagrange();
    Lagrange::new(f.masses, Member::Circular, f.per_period)
}

/// The circular member's start, held at f32.
fn circular_start() -> Ic<f64> {
    Integrated::held(&circular().start())
}

fn row<P: Provider>(t: &Trajectory<'_, P>, k: u64) -> Ic<f64> {
    t.state(k)
        .unwrap_or_else(|e| panic!("row {k} refused: {e:?}"))
}

// ── Rows read the march ────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn trajectory_interface_integrator_rows_read_the_march() {
    let p = provider();
    let start = circular_start();
    let t = Trajectory::new(&p, start, HORIZON).expect("a trajectory");
    assert_eq!(t.last(), 16, "t = 2 is boundary 16");
    assert_eq!(
        t.quantum().to_bits(),
        0.125f64.to_bits(),
        "the quantum is Δ_sync"
    );
    for h in [0.4, 7.3, 50.0] {
        let q = Integrated::new(h).expect("a horizon").quantum();
        assert_eq!(q.to_bits(), 0.125f64.to_bits(), "the quantum at T = {h}");
    }
    assert!(same_bits(&row(&t, 0), &start), "row 0 is the start");
    for k in 0..=16 {
        let want = p.state(&start, k).unwrap();
        assert!(
            same_bits(&row(&t, k), &want),
            "row {k} is not the provider's state"
        );
    }
    check_field_rows(&p);
    assert_eq!(
        t.terminal(),
        None,
        "the circular member has no terminal by t = 2"
    );
}

/// `p`'s rows 0 to 16 on the circular member are the field's marches to each boundary, widened exactly.
fn check_field_rows<P: Provider>(p: &P) {
    let start = circular_start();
    let t = Trajectory::new(p, start, HORIZON).expect("a trajectory");
    for k in 0..=16 {
        let out = marched(
            &narrow(&start),
            None,
            radius(),
            k as u32,
            &heggie_params(),
            false,
        )
        .end
        .expect("a march");
        assert_eq!(
            out.boundary, k as u32,
            "the march to {k} lands on boundary {k}"
        );
        let field = Ic {
            m: start.m,
            r: out.state.r.map(|v| v.map(f64::from)),
            p: out.state.p.map(|v| v.map(f64::from)),
        };
        assert!(
            same_bits(&row(&t, k), &field),
            "row {k} is not the field's march to {k}"
        );
    }
}

negative_control!(
    trajectory_interface_integrator_rows_read_the_march,
    "a provider whose rows read between boundaries fails",
    expected = "is not the field's march",
    check_field_rows(&Between::new())
);

// ── Rows on one march ──────────────────────────────────────────────────────────────────────────────────────────────

/// The wrapper's own cadence, every call forwarded, the state at each landing recorded by its boundary index: the
/// step's image of the regularised state as the march hands it over, before the cadence's projection touches its copy
/// (the march carries the regularised state on and returns that image, never the cadence's copy).
struct Recorder {
    inner: WrapperCadence<f32>,
    image: Option<KState<f32>>,
    rows: Vec<(u32, KState<f32>)>,
}

impl Cadence<f32> for Recorder {
    fn after_step(&mut self, state: &mut KState<f32>, geom: &mut Geom<f32>) {
        self.image = Some(*state);
        self.inner.after_step(state, geom);
    }
    fn done(&self) -> bool {
        self.inner.done()
    }
    fn sample(&mut self, state: &KState<f32>, index: u32, quantum: f32) {
        self.inner.sample(state, index, quantum);
        self.rows
            .push((index, self.image.expect("a step before its landing")));
    }
    fn bucket_d2(&self, r: &[[f32; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }
    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }
    fn regularised_drift(&mut self, drift: f32) {
        self.inner.regularised_drift(drift);
    }
    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
    fn resume(&mut self, stored: &SimStateBaseOf<f32>) {
        self.inner.resume(stored);
    }
    fn latched(&mut self) {
        self.inner.latched();
    }
    fn finish(&self, out: SimStateBaseOf<f32>) -> SimStateBaseOf<f32> {
        self.inner.finish(out)
    }
}

/// Each of `p`'s rows 1 to 16 from `start` is the one march to boundary 16's landing on its boundary, bit for bit.
fn check_one_march<P: Provider>(p: &P, start: &Ic<f64>) {
    let ic = narrow(start);
    let mut recorder = Recorder {
        inner: WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius()),
        image: None,
        rows: Vec::new(),
    };
    let s = KState {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let out = march::<Rk4, f32, _, _>(&s, 16, &heggie_params(), &mut recorder, &mut ExplicitTest);
    assert_eq!(out.boundary, 16);
    let indices: Vec<u32> = recorder.rows.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        indices,
        (1..=16).collect::<Vec<_>>(),
        "one sample at each landing"
    );
    let t = Trajectory::new(p, *start, HORIZON).expect("a trajectory");
    for (k, s) in &recorder.rows {
        let got = row(&t, u64::from(*k));
        let want = Ic {
            m: start.m,
            r: s.r.map(|v| v.map(f64::from)),
            p: s.p.map(|v| v.map(f64::from)),
        };
        let diff: Vec<f64> = got
            .r
            .iter()
            .chain(&got.p)
            .zip(want.r.iter().chain(&want.p))
            .flat_map(|(a, b)| [a[0] - b[0], a[1] - b[1]])
            .collect();
        assert!(
            same_bits(&got, &want),
            "row {k} is not the one march's state at boundary {k}: {diff:?}"
        );
    }
}

#[test]
fn trajectory_interface_integrator_rows_on_one_march() {
    check_one_march(&provider(), &circular_start());
}

negative_control!(
    trajectory_interface_integrator_rows_on_one_march,
    "a provider whose rows read between boundaries fails",
    expected = "is not the one march's state",
    check_one_march(&Between::new(), &circular_start())
);

// ── The Lagrange closed form ───────────────────────────────────────────────────────────────────────────────────────

/// `|v_x| + |v_y|`.
fn l1(v: [f64; 2]) -> f64 {
    v[0].abs() + v[1].abs()
}

/// f32's unit roundoff, `2⁻²⁴`.
const U32: f64 = 1.0 / 16_777_216.0;

/// `p`'s rows 1 to 16 on the circular member against the closed form at `k·Δ_sync`; returns the worst error in units
/// of the bound.
fn check_closed_form<P: Provider>(p: &P) -> f64 {
    let lagrange = circular();
    let start = circular_start();
    let steps = marched(&narrow(&start), None, radius(), 16, &heggie_params(), false)
        .end
        .expect("a march")
        .count
        .steps;
    let bound = (f64::from(steps) * U32).min(1e-4);
    let t = Trajectory::new(p, start, HORIZON).expect("a trajectory");
    let mut worst = 0.0f64;
    for k in 1..=16 {
        let got = row(&t, k);
        let want = lagrange.at(k as f64 * 0.125);
        let scale = lagrange.start();
        let wants = want.r.iter().chain(&want.p);
        let scales = scale.r.iter().chain(&scale.p);
        for ((g, w), v) in got.r.iter().chain(&got.p).zip(wants).zip(scales) {
            for d in 0..2 {
                let (err, b) = ((g[d] - w[d]).abs(), bound * l1(*v));
                worst = worst.max(err / b);
                assert!(
                    err <= b,
                    "row {k}: {err:e} from the closed form, over the bound {b:e}"
                );
            }
        }
    }
    println!(
        "trajectory_interface_integrator_lagrange_closed_form: N = {steps} steps to t = 2; the bound \
         min(N·2⁻²⁴, 1e-4) = {bound:e}; the worst error {worst:.4e} of the bound"
    );
    worst
}

#[test]
fn trajectory_interface_integrator_lagrange_closed_form() {
    check_closed_form(&provider());
}

negative_control!(
    trajectory_interface_integrator_lagrange_closed_form,
    "a provider whose rows read between boundaries fails",
    expected = "over the bound",
    {
        check_closed_form(&Between::new());
    }
);

#[test]
fn trajectory_interface_integrator_rows_differ() {
    let p = provider();
    let t = Trajectory::new(&p, circular_start(), HORIZON).expect("a trajectory");
    let rows: Vec<Ic<f64>> = (0..=16).map(|k| row(&t, k)).collect();
    check_distinct(&rows);
}

fn check_distinct(rows: &[Ic<f64>]) {
    for (i, a) in rows.iter().enumerate() {
        for (j, b) in rows.iter().enumerate().skip(i + 1) {
            assert!(!same_bits(a, b), "rows {i} and {j} are the same state");
        }
    }
}

negative_control!(
    trajectory_interface_integrator_rows_differ,
    "rows held past a terminal, read as rows, fail",
    expected = "are the same state",
    {
        let p = provider();
        let t = Trajectory::new(&p, Integrated::held(&radial_start()), HORIZON).unwrap();
        check_distinct(&(10..=16).map(|k| row(&t, k)).collect::<Vec<_>>())
    }
);

// ── The terminal ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The radial collision passage's start (validation's `radial`), in `(m, r, p)`.
fn radial_start() -> Ic<f64> {
    let f = validation::az::radial();
    Ic {
        m: f.state.m,
        r: f.state.r,
        p: f.state.p,
    }
}

/// The radial passage's terminal is a collision of pair 2 at `index`, held from there, the rows before it distinct.
fn check_terminal(index: u64) {
    let p = provider();
    let start = Integrated::held(&radial_start());
    let t = Trajectory::new(&p, start, HORIZON).expect("a trajectory");
    let want = engine::trajectory::Terminal {
        index,
        state: State::Collision,
        detail: Detail::D2,
    };
    assert_eq!(t.terminal(), Some(want), "the radial passage's terminal");
    let held = row(&t, index);
    for k in index + 1..=16 {
        assert!(
            same_bits(&row(&t, k), &held),
            "row {k} past the terminal is not held"
        );
    }
    check_distinct(&(0..=index).map(|k| row(&t, k)).collect::<Vec<_>>());
    let d = |s: &Ic<f64>| (s.r[0][0] - s.r[1][0]).hypot(s.r[0][1] - s.r[1][1]);
    assert!(
        d(&held) < 2e-3,
        "the held state is at the collision: d = {:e}",
        d(&held)
    );
    assert!(
        d(&row(&t, index - 1)) > 2e-3,
        "the row before the terminal is short of it"
    );
    assert_eq!(
        p.outcome(&start, None).unwrap(),
        p.outcome(&start, None).unwrap(),
        "the outcome is pure"
    );
    assert_eq!(
        p.outcome(&start, None).unwrap(),
        Outcome {
            state: kernel::payload::STATE_COLLISION,
            detail: 2,
        },
        "the field's outcome is the terminal's class"
    );
}

#[test]
fn trajectory_interface_integrator_terminal() {
    check_terminal(11);
}

negative_control!(
    trajectory_interface_integrator_terminal,
    "a terminal expected one boundary early fails",
    expected = "the radial passage's terminal",
    check_terminal(10)
);

// ── Purity ─────────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn trajectory_interface_integrator_reads_order_free() {
    check_order_free(&provider());
}

/// A provider whose state moves with each read: what purity forbids.
#[cfg(feature = "controls")]
struct Remembers {
    inner: Integrated,
    reads: std::cell::Cell<u32>,
}

#[cfg(feature = "controls")]
impl Provider for Remembers {
    fn quantum(&self) -> f64 {
        self.inner.quantum()
    }
    fn terminal(&self, start: &Ic<f64>) -> Result<Option<engine::trajectory::Terminal>, Refused> {
        self.inner.terminal(start)
    }
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        self.reads.set(self.reads.get() + 1);
        let mut s = self.inner.state(start, k)?;
        s.r[0][0] += f64::from(self.reads.get()) * 1e-9;
        Ok(s)
    }
}

negative_control!(
    trajectory_interface_integrator_reads_order_free,
    "a provider whose reads depend on earlier reads fails",
    expected = "depends on an earlier read",
    check_order_free(&Remembers {
        inner: provider(),
        reads: std::cell::Cell::new(0),
    })
);

fn check_order_free<P: Provider>(p: &P) {
    let start = circular_start();
    let first = Trajectory::new(p, start, HORIZON).unwrap();
    let (a, b) = (row(&first, 3), row(&first, 12));
    let second = Trajectory::new(p, start, HORIZON).unwrap();
    let _ = (row(&second, 16), row(&second, 0), row(&second, 7));
    let (b2, a2) = (row(&second, 12), row(&second, 3));
    assert!(
        same_bits(&a, &a2) && same_bits(&b, &b2),
        "a read depends on an earlier read"
    );
}

// ── Refusals ───────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn trajectory_interface_integrator_refusals() {
    check_refusals(&provider());
    let p = provider();
    assert_eq!(
        p.state(&circular_start(), 17).err(),
        Some(Refused::PastHorizon { k: 17, last: 16 })
    );
    assert_eq!(p.last(), 16);
    for (h, want) in [(0.4, 3), (50.0, 400), (50.1, 400)] {
        let cap = Integrated::new(h).expect("a horizon").last();
        assert_eq!(
            cap,
            engine::field::boundaries(h, &heggie_params()),
            "the cap at T = {h} is the field's"
        );
        assert_eq!(cap, want, "the cap at T = {h}");
    }
    for bad in [f64::NAN, -1.0, f64::INFINITY] {
        assert!(Integrated::new(bad).is_err(), "horizon {bad} is refused");
    }
}

/// `p` refuses a start an f32 does not hold, its rows and its terminal both.
fn check_refusals<P: Provider>(p: &P) {
    let f64_start = circular().start();
    assert!(
        !same_bits(&f64_start, &circular_start()),
        "the f64 start is not f32's"
    );
    assert_eq!(
        p.state(&f64_start, 1).err(),
        Some(Refused::Start),
        "a row of an f64 start is not refused"
    );
    assert_eq!(
        p.terminal(&f64_start).err(),
        Some(Refused::Start),
        "the terminal of an f64 start is not refused"
    );
}

/// A provider that takes any f64 start, rounding it itself: what the refusal forbids.
#[cfg(feature = "controls")]
struct Lenient(Integrated);

#[cfg(feature = "controls")]
impl Provider for Lenient {
    fn quantum(&self) -> f64 {
        self.0.quantum()
    }
    fn terminal(&self, start: &Ic<f64>) -> Result<Option<engine::trajectory::Terminal>, Refused> {
        self.0.terminal(&Integrated::held(start))
    }
    fn state(&self, start: &Ic<f64>, k: u64) -> Result<Ic<f64>, Refused> {
        self.0.state(&Integrated::held(start), k)
    }
}

negative_control!(
    trajectory_interface_integrator_refusals,
    "a provider that takes any f64 start fails",
    expected = "is not refused",
    check_refusals(&Lenient(provider()))
);

// ── The terminal's index ───────────────────────────────────────────────────────────────────────────────────────────

/// The index rule `index`, at the four latches of the module's note.
fn check_latched_index(index: &dyn Fn(u32, f32, f32, End) -> u64) {
    let d = heggie_params().delta_sync;
    for (boundary, clock, end, want, what) in [
        (399, 1e-7, End::Done, 400, "a latch 1e-7 past boundary 399"),
        (399, d, End::Done, 399, "a latch on a counted landing"),
        (
            399,
            d,
            End::Exhausted,
            400,
            "a budget spent on the landing step",
        ),
        (
            399,
            0.0,
            End::NonFinite,
            399,
            "a latch at an interval's entry",
        ),
        (10, 0.06, End::Done, 11, "a latch mid-interval"),
    ] {
        assert_eq!(index(boundary, clock, d, end), want, "the index of {what}");
    }
}

#[test]
fn trajectory_interface_integrator_latched_index() {
    check_latched_index(&latched_index);
}

negative_control!(
    trajectory_interface_integrator_latched_index,
    "the index from the absolute time at f32 rounds a short latch back onto its boundary",
    expected = "a latch 1e-7 past boundary 399",
    check_latched_index(&|boundary, clock, d, end| {
        // The first draft's rule on `march_on`'s own `t`: `k·Δ_sync + t_s` at f32, `t_s` 0 on a counted landing.
        let t_s = if clock >= d && end == End::Done {
            0.0
        } else {
            clock.min(d)
        };
        let t = boundary as f32 * d + t_s;
        u64::from(boundary) + u64::from(f64::from(t) > f64::from(boundary) * f64::from(d))
    })
);
