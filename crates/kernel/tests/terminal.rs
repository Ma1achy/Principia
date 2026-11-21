//! Terminal states, their precedence and the terminal latch (`kernel::detect::label`, `kernel::driver::terminal`,
//! the wrapper's cadence; integrator_contract Part 1, Part 7; payload §2, §5; plan/tasks/M3/TASK-M3-17.md).
//!
//! The fixtures are built here, in the centre-of-mass frame, at `G = 1` and unit masses unless stated:
//! - **expanding**: an equilateral triangle of side 10, each body moving radially outward at speed 1 (the escape
//!   fixtures' `three_way_unbound`). Its shape is fixed, every pair is unbound and separating and `E_0 = 1.2 > 0`, so
//!   escape fires at the first window mark with `detail` 3, a triple ejection.
//! - **head-on**: bodies 0 and 1 at rest at `(∓1, 0)`, body 2 at rest at `(0, 8)`: the pair falls together and meets
//!   `r_coll = 0.1` near `t = 2.2`, a collision of pair 2 (the side opposite body 2).
//! - **bound**: a binary of separation 1 with a third body at distance 5 moving tangentially at speed² 0.3, bound
//!   (`E_rel = 0.15 − 3/5 < 0`), run for 2 time units: no event, `bounded` at the horizon.
//! - **ejection**: the escape fixtures' `hyperbolic_ejection`, a binary of separation 1 and a third body at distance
//!   20 moving radially outward at speed² 1.3, `E_rel = +0.5`: escape fires near `t = 529`.
//! - **near-singular**: bodies 0 and 1 at rest at `(∓1, ±0.002)`, body 2 at rest at `(0, 6)`: the pair falls almost
//!   head-on and passes at a separation far inside the substep law's cap, with no collision threshold.
//!
//! Pass criteria are written in each test before it was run.

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::is_mark;
use kernel::detect::label::{precedence, Events, Fired, Label};
use kernel::driver::advance::Cadence;
use kernel::driver::benettin::{completed_renorms, ftle_read, Absolute, Benettin, BenettinParams};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::carried::Carried;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::substep::{NsubThresholds, Quantise};
use kernel::driver::terminal::{finished, latch, redispatch};
use kernel::driver::wrapper::{
    integrate_detecting, integrate_resumed_with, integrate_with, integrate_with_co, monitored,
    WrapperReal, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_dmin_pair, sd_saturated, sd_state, tm_t_dmin_step, tm_t_end_step, PayloadReal,
    SimStateBaseOf, SimStateFTLEOf, DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION,
    DETAIL_COLLISION_PAIR_0, DETAIL_COLLISION_PAIR_1, DETAIL_COLLISION_PAIR_2,
    DETAIL_DECODE_FAILED_DEGENERATE_CONFIGURATION, DETAIL_DECODE_FAILED_INVALID_MASS_CONSTRUCTION,
    DETAIL_DECODE_FAILED_NON_FINITE_DECODE_OUTPUT, DETAIL_DECODE_FAILED_OTHER_RESERVED,
    DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION, DETAIL_ESCAPE_BODY_0, DETAIL_ESCAPE_BODY_1,
    DETAIL_ESCAPE_BODY_2, STATE_BOUNDED, STATE_COLLISION, STATE_DECODE_FAILED, STATE_ESCAPE,
    STATE_RUNNING, STATE_SIM_FAILED,
};
use spirv_std::glam::UVec4;
use spirv_std::num_traits::NumCast;
use std::cell::Cell;
use validation::negative_control;

// ── Precision, keys and fixtures ────────────────────────────────────────────────────────────────────────────────────

trait P: WrapperReal + Quantise + PayloadReal + Into<f64> + NumCast {}

impl<R: WrapperReal + Quantise + PayloadReal + Into<f64> + NumCast> P for R {}

fn real<R: P>(x: f64) -> R {
    <R as NumCast>::from(x).expect("a finite f64")
}

/// A sim key with every threshold at `f32::MAX`, so `N_sub` is `n_max` on every macro-step.
fn flat<R: P>(dt: f64, horizon_steps: u32, n_max: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: real(dt),
        horizon_steps,
        n_max,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// A sim key under the substep law's own table, `r_sub = 0.05`, `γ_sub = 1.5`, capped at `n_max`.
fn law<R: P>(dt: f64, horizon_steps: u32, n_max: u32) -> WrapperUniforms<R> {
    let config = NsubConfig {
        n_max,
        ..NsubConfig::DEFAULT
    };
    WrapperUniforms {
        dt_macro: real(dt),
        horizon_steps,
        n_max,
        thresholds: thresholds(&config).expect("the key generates"),
    }
}

/// The state of masses `m`, positions `r` and velocities `v`, moved to its centre of mass and momentum, at `R`.
fn centred<R: P>(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<R> {
    let total: f64 = m.iter().sum();
    let mean = |x: &[[f64; 2]; 3], c: usize| (0..3).map(|b| m[b] * x[b][c]).sum::<f64>() / total;
    let (rc, vc) = ([mean(&r, 0), mean(&r, 1)], [mean(&v, 0), mean(&v, 1)]);
    State {
        m: m.map(real),
        r: core::array::from_fn(|b| [real(r[b][0] - rc[0]), real(r[b][1] - rc[1])]),
        p: core::array::from_fn(|b| {
            [
                real(m[b] * (v[b][0] - vc[0])),
                real(m[b] * (v[b][1] - vc[1])),
            ]
        }),
    }
}

/// An equilateral triangle of side `side`, each body moving radially outward at `speed`.
fn equilateral<R: P>(side: f64, speed: f64) -> State<R> {
    let rho = side / 3f64.sqrt();
    let dir = |k: usize| {
        let a = core::f64::consts::FRAC_PI_2 + k as f64 * 2.0 * core::f64::consts::FRAC_PI_3;
        [a.cos(), a.sin()]
    };
    centred(
        [1.0; 3],
        core::array::from_fn(|k| [rho * dir(k)[0], rho * dir(k)[1]]),
        core::array::from_fn(|k| [speed * dir(k)[0], speed * dir(k)[1]]),
    )
}

fn expanding<R: P>() -> State<R> {
    equilateral(10.0, 1.0)
}

fn head_on<R: P>() -> State<R> {
    centred(
        [1.0; 3],
        [[-1.0, 0.0], [1.0, 0.0], [0.0, 8.0]],
        [[0.0; 2]; 3],
    )
}

/// A binary of separation `a` with a third body at distance `d` from its centre of mass, moving at `third`.
fn binary_third<R: P>(a: f64, d: f64, third: [f64; 2]) -> State<R> {
    let vb = (2.0 / a).sqrt();
    centred(
        [1.0; 3],
        [[-a / 2.0, 0.0], [a / 2.0, 0.0], [d, 0.0]],
        [[0.0, -vb / 2.0], [0.0, vb / 2.0], third],
    )
}

fn bound<R: P>() -> State<R> {
    binary_third(1.0, 5.0, [0.0, 0.3f64.sqrt()])
}

fn ejection<R: P>() -> State<R> {
    binary_third(1.0, 20.0, [1.3f64.sqrt(), 0.0])
}

fn near_singular<R: P>() -> State<R> {
    centred(
        [1.0; 3],
        [[-1.0, 0.002], [1.0, -0.002], [0.0, 6.0]],
        [[0.0; 2]; 3],
    )
}

fn ic<R: P>(s: &State<R>) -> Ic<R> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// The bytes of a stored state, for a bitwise comparison of every word.
fn bytes<T: Copy>(x: &T) -> Vec<u8> {
    // SAFETY: `T` is a generated `#[repr(C)]` payload struct of plain integers and floats with its tail padding
    // declared as a member (payload §1), so every byte is initialised.
    unsafe { std::slice::from_raw_parts((x as *const T).cast::<u8>(), size_of::<T>()).to_vec() }
}

fn name(state: u32) -> &'static str {
    match state {
        STATE_ESCAPE => "escape",
        STATE_BOUNDED => "bounded",
        STATE_COLLISION => "collision",
        STATE_RUNNING => "running",
        STATE_SIM_FAILED => "sim_failed",
        STATE_DECODE_FAILED => "decode_failed",
        _ => "reserved",
    }
}

// ── A recording cadence ─────────────────────────────────────────────────────────────────────────────────────────────

/// The wrapper's cadence, every hook passed on, with what a test reads beside it: the least quantised `d²` of each
/// state the cadence saw (the IC's first), the call count at each sampling point (each a completed macro-step under a
/// stepper) and the sampling point escape fired at. `latched: false` reads `done` as never set, a loop that does not
/// exit on a terminal; `clocked: false` drops the wrapper's macro-step clock.
struct Recording<R> {
    inner: WrapperCadence<R>,
    d2: Vec<f32>,
    samples: Vec<(u32, u32)>,
    escaped_at: Option<u32>,
    latched: bool,
    clocked: bool,
}

impl<R: P> Recording<R> {
    fn new(s: &State<R>, radius: CollisionRadius) -> Self {
        let inner = WrapperCadence::with_radius(&s.m, &s.r, &s.p, radius);
        Recording {
            d2: vec![inner.detector().last().min()],
            inner,
            samples: Vec::new(),
            escaped_at: None,
            latched: true,
            clocked: true,
        }
    }
}

impl<R: P> Cadence<R> for Recording<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.d2.push(self.inner.detector().last().min());
    }

    fn done(&self) -> bool {
        self.latched && self.inner.done()
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        let before = self.inner.escape().fired();
        self.inner.sample(state, index, quantum);
        self.samples.push((index, self.inner.invocations()));
        if self.inner.escape().fired() && !before {
            self.escaped_at = Some(index);
        }
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        if self.clocked {
            self.inner.clock(completed);
        }
    }

    fn resume(&mut self, stored: &SimStateBaseOf<R>) {
        self.inner.resume(stored);
    }

    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        self.inner.finish(out)
    }
}

/// The run of `s` under `key` and `radius` through a [`Recording`] cadence, `latched` and `clocked` as given, labelled
/// by the cadence's write.
fn recorded<R: P>(
    s: &State<R>,
    key: &WrapperUniforms<R>,
    radius: CollisionRadius,
    latched: bool,
    clocked: bool,
) -> (SimStateBaseOf<R>, Recording<R>) {
    let mut c = Recording::new(s, radius);
    c.latched = latched;
    c.clocked = clocked;
    let out = integrate_with::<Kdk, R, _>(&ic(s), key, &mut c);
    (c.finish(out), c)
}

// ── state_enum ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The wrapper's cadence with a NaN put into the first body's momentum before the `at`-th call (never, where `at` is
/// 0): the state goes non-finite mid-march.
struct Poisoned<R> {
    inner: WrapperCadence<R>,
    calls: u32,
    at: u32,
}

impl<R: P> Cadence<R> for Poisoned<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.calls += 1;
        if self.calls == self.at {
            state.p[0][0] = R::nan();
        }
        self.inner.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.inner.done()
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
    }

    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        self.inner.finish(out)
    }
}

/// The horizon reached with no event is `bounded`, at `t_end_step = horizon_steps`.
fn check_bounded(horizon_steps: u32, cut: u32) {
    let s = bound::<f64>();
    let zero = integrate_detecting::<Kdk, f64>(&ic(&s), &flat(0.01, 0, 1), CollisionRadius::none());
    let mut c = WrapperCadence::new(&s.m, &s.r, &s.p);
    let out = integrate_resumed_with::<Kdk, f64, _>(
        &ic(&s),
        &flat(0.01, horizon_steps, 1),
        &mut c,
        &zero,
        cut,
    );
    let out = c.finish(out);
    assert_eq!(
        (name(sd_state(out.packed_a)), tm_t_end_step(out.times)),
        ("bounded", horizon_steps),
        "the horizon reached with no event is not `bounded` at the horizon"
    );
}

/// A NaN put into the state mid-march is `sim_failed`, `detail` 0 (a NaN in the state), whatever else fired.
fn check_nan(at: u32) {
    for (s, radius) in [
        (bound::<f64>(), CollisionRadius::none()),
        (head_on::<f64>(), CollisionRadius::from_r_coll(0.1)),
        (expanding::<f64>(), CollisionRadius::none()),
    ] {
        let mut c = Poisoned {
            inner: WrapperCadence::with_radius(&s.m, &s.r, &s.p, radius),
            calls: 0,
            at,
        };
        let out = integrate_with::<Kdk, f64, _>(&ic(&s), &flat(0.01, 300, 1), &mut c);
        let out = c.finish(out);
        assert_eq!(
            (name(sd_state(out.packed_a)), sd_detail(out.packed_a)),
            ("sim_failed", 0),
            "a NaN injected mid-march is not `sim_failed` with detail 0"
        );
    }
}

/// `N_sub` capped at `N_max` on every macro-step is not terminal: the run goes to the horizon, `saturated`.
fn check_cap(stop_on_cap: bool) {
    let s = bound::<f64>();
    let key = flat::<f64>(0.01, 200, 2);
    let mut c = CapStop {
        inner: WrapperCadence::new(&s.m, &s.r, &s.p),
        table: key.thresholds,
        n_max: key.n_max,
        stop: stop_on_cap,
        capped: Cell::new(false),
    };
    let out = integrate_with::<Kdk, f64, _>(&ic(&s), &key, &mut c);
    let out = c.inner.finish(out);
    assert!(
        sd_saturated(out.packed_a),
        "the capped run is not saturated"
    );
    assert_eq!(
        (name(sd_state(out.packed_a)), tm_t_end_step(out.times)),
        ("bounded", 200),
        "the substep cap ended the march"
    );
}

#[test]
fn state_enum() {
    check_bounded(200, 200);
    // Step 20, before the expanding triangle's escape at its first mark (step 40) and the head-on collision (~220).
    check_nan(20);
    check_cap(false);
}

negative_control!(
    state_enum,
    "the run stopped one macro-step short of its horizon",
    expected = "is not `bounded` at the horizon",
    check_bounded(200, 199)
);

mod no_nan {
    use super::*;

    negative_control!(
        state_enum,
        "no NaN injected",
        expected = "is not `sim_failed` with detail 0",
        check_nan(0)
    );
}

mod cap_terminal {
    use super::*;

    negative_control!(
        state_enum,
        "the substep cap made terminal",
        expected = "the substep cap ended the march",
        check_cap(true)
    );
}

// ── nmax_cap_continues ──────────────────────────────────────────────────────────────────────────────────────────────

/// The wrapper's cadence, with `done` set where the substep law's bucket for a macro-step exceeds `n_max` (`stop`):
/// a cap made terminal.
struct CapStop<R> {
    inner: WrapperCadence<R>,
    table: NsubThresholds,
    n_max: u32,
    stop: bool,
    capped: Cell<bool>,
}

impl<R: P> Cadence<R> for CapStop<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.inner.done() || (self.stop && self.capped.get())
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        let d2 = self.inner.bucket_d2(r);
        if self.table.bucket(d2) > self.n_max {
            self.capped.set(true);
        }
        d2
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }
}

/// The near-singular encounter under the law capped at `N_max = 4`, with no collision threshold: the bucket exceeds
/// the cap at the closest approach, the run is `saturated`, and it marches on past the encounter to an outcome,
/// `bounded` at the horizon or an `escape`, never `running` and never failed.
fn check_nmax_cap(stop_on_cap: bool) {
    let s = near_singular::<f64>();
    let key = law::<f64>(0.01, 2000, 4);
    let mut c = CapStop {
        inner: WrapperCadence::new(&s.m, &s.r, &s.p),
        table: key.thresholds,
        n_max: key.n_max,
        stop: stop_on_cap,
        capped: Cell::new(false),
    };
    let out = integrate_with::<Kdk, f64, _>(&ic(&s), &key, &mut c);
    let out = c.inner.finish(out);
    let (t_end, t_dmin) = (tm_t_end_step(out.times), tm_t_dmin_step(out.times));
    let d_min = c.inner.detector().latch().d2().sqrt();
    println!(
        "nmax_cap_continues: d_min {d_min:.3e} at macro-step {t_dmin}, the bucket there {}, N_max 4; {} at \
         t_end_step {t_end} of 2000, saturated {}",
        key.thresholds.bucket(c.inner.detector().latch().d2()),
        name(sd_state(out.packed_a)),
        sd_saturated(out.packed_a)
    );
    assert!(
        key.thresholds.bucket(c.inner.detector().latch().d2()) > 4,
        "the encounter does not reach the cap"
    );
    assert!(
        sd_saturated(out.packed_a),
        "the capped run is not saturated"
    );
    let s = sd_state(out.packed_a);
    assert!(
        (s == STATE_BOUNDED && t_end == 2000) || s == STATE_ESCAPE,
        "the capped march did not continue to a dynamical outcome: {} at step {t_end}",
        name(s)
    );
    assert!(t_end > t_dmin, "the march did not go on past the encounter");
}

#[test]
fn nmax_cap_continues() {
    check_nmax_cap(false);
}

negative_control!(
    nmax_cap_continues,
    "the cap made terminal",
    expected = "did not continue to a dynamical outcome",
    check_nmax_cap(true)
);

// ── terminal_precedence ─────────────────────────────────────────────────────────────────────────────────────────────

type Rule = fn(&Events) -> Label;

fn fired_escape(at: u32, detail: u32) -> Events {
    Events {
        escape: Fired::at(at, detail),
        ..Events::default()
    }
}

/// The pure cases: every order of collision and escape, a same-step tie, a failure over each, triple ejection as
/// escape's detail.
fn check_rule(rule: Rule) {
    let both = |c: u32, e: u32| Events {
        collision: Fired::at(c, DETAIL_COLLISION_PAIR_2),
        escape: Fired::at(e, DETAIL_ESCAPE_BODY_1),
        ..Events::default()
    };
    let cases = [
        (
            "escape before collision",
            both(9, 5),
            (STATE_ESCAPE, DETAIL_ESCAPE_BODY_1),
        ),
        (
            "collision before escape",
            both(5, 9),
            (STATE_COLLISION, DETAIL_COLLISION_PAIR_2),
        ),
        (
            "a same-step tie",
            both(7, 7),
            (STATE_COLLISION, DETAIL_COLLISION_PAIR_2),
        ),
        (
            "sim_failed with a collision and an escape",
            Events {
                failed: Fired::at(0, 1),
                ..both(3, 4)
            },
            (STATE_SIM_FAILED, 1),
        ),
        (
            "sim_failed with an escape",
            Events {
                failed: Fired::at(0, 2),
                ..fired_escape(4, DETAIL_ESCAPE_BODY_0)
            },
            (STATE_SIM_FAILED, 2),
        ),
        (
            "sim_failed at the horizon",
            Events {
                failed: Fired::at(0, 0),
                horizon: true,
                ..Events::default()
            },
            (STATE_SIM_FAILED, 0),
        ),
        (
            "triple ejection",
            fired_escape(8, DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION),
            (STATE_ESCAPE, DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION),
        ),
        (
            "an escape on the horizon",
            Events {
                horizon: true,
                ..fired_escape(8, DETAIL_ESCAPE_BODY_2)
            },
            (STATE_ESCAPE, DETAIL_ESCAPE_BODY_2),
        ),
        (
            "the horizon with no event",
            Events {
                horizon: true,
                ..Events::default()
            },
            (STATE_BOUNDED, 0),
        ),
        (
            "short of the horizon",
            Events::default(),
            (STATE_RUNNING, 0),
        ),
    ];
    for (what, events, (state, detail)) in cases {
        let l = rule(&events);
        assert!(
            (l.state, l.detail) == (state, detail),
            "{what}: {} detail {}, not {} detail {detail}",
            name(l.state),
            l.detail,
            name(state)
        );
    }
}

/// The labels of the cadence's own events at `R` (the module's fixtures): a collision and an escape on one state, a
/// tie; an escape a step before a collision; each through `rule` and through the wrapper's write ([`monitored`]); and
/// a march's failure over the tie. Returns the labels, for the comparison across precisions.
fn cadence_labels<R: P>(rule: Rule) -> Vec<(u32, u32)> {
    let radius = CollisionRadius::from_r_coll(2.0);
    let s0 = expanding::<R>();
    let close = equilateral::<R>(1.0, 10.0);
    let apart = equilateral::<R>(5.0, 10.0);
    let step = |c: &mut WrapperCadence<R>, s: &State<R>| {
        let mut s = *s;
        let mut g = Geom::new(&s.m, &s.r);
        c.after_step(&mut s, &mut g);
        s
    };
    let quantum: R = real(0.05);
    assert!(is_mark(8, quantum));
    let mut labels = Vec::new();
    let mut tie = WrapperCadence::with_radius(&s0.m, &s0.r, &s0.p, radius);
    let s = step(&mut tie, &close);
    tie.sample(&s, 8, quantum);
    let e = tie.events();
    assert!(
        e.collision.fired && e.escape.fired && e.collision.at == e.escape.at,
        "the cadence did not read the tie: {e:?}"
    );
    let mut later = WrapperCadence::with_radius(&s0.m, &s0.r, &s0.p, radius);
    let s = step(&mut later, &apart);
    later.sample(&s, 8, quantum);
    assert!(later.done(), "escape did not set `done`");
    // The march would stop here; one more step is driven by hand so that both events are recorded, the escape first.
    step(&mut later, &close);
    let e2 = later.events();
    assert!(e2.collision.fired && e2.escape.fired && e2.escape.at < e2.collision.at);
    let out = SimStateBaseOf::<R> {
        packed_a: Label {
            state: STATE_RUNNING,
            detail: 0,
        }
        .write(0),
        ..SimStateBaseOf::default()
    };
    let failed_out = SimStateBaseOf::<R> {
        packed_a: Label {
            state: STATE_SIM_FAILED,
            detail: 1,
        }
        .write(0),
        ..out
    };
    for (what, cadence, out, expected) in [
        (
            "a same-step tie",
            &tie,
            out,
            (STATE_COLLISION, DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION),
        ),
        (
            "escape before collision",
            &later,
            out,
            (STATE_ESCAPE, DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION),
        ),
        (
            "sim_failed over the tie",
            &tie,
            failed_out,
            (STATE_SIM_FAILED, 1),
        ),
    ] {
        let events = Events {
            failed: if sd_state(out.packed_a) == STATE_SIM_FAILED {
                Fired::at(0, sd_detail(out.packed_a))
            } else {
                Fired::none()
            },
            ..cadence.events()
        };
        let l = rule(&events);
        assert!(
            (l.state, l.detail) == expected,
            "{what} at {}: {} detail {}",
            core::any::type_name::<R>(),
            name(l.state),
            l.detail
        );
        let written = monitored(out, cadence);
        assert_eq!(
            (sd_state(written.packed_a), sd_detail(written.packed_a)),
            expected,
            "{what}: the wrapper's write is not the precedence's"
        );
        labels.push(expected);
    }
    // Through the wrapper: the expanding triangle escapes at the first mark, the head-on pair collides.
    for (s, radius) in [
        (expanding::<R>(), CollisionRadius::none()),
        (head_on::<R>(), CollisionRadius::from_r_coll(0.1)),
    ] {
        let out = integrate_detecting::<Kdk, R>(&ic(&s), &flat(0.05, 200, 4), radius);
        labels.push((sd_state(out.packed_a), sd_detail(out.packed_a)));
        labels.push((tm_t_end_step(out.times), 0));
    }
    labels
}

fn check_precedence(rule: Rule) {
    check_rule(rule);
    let (a, b) = (cadence_labels::<f32>(rule), cadence_labels::<f64>(rule));
    println!("terminal_precedence: f32 {a:?}, f64 {b:?}");
    assert_eq!(a, b, "the labels differ between f32 and f64");
    assert_eq!(
        &a[3..],
        &[
            (STATE_ESCAPE, DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION),
            (8, 0),
            (STATE_COLLISION, DETAIL_COLLISION_PAIR_2),
            (a[6].0, 0)
        ],
        "the wrapper's fixtures"
    );
}

#[test]
fn terminal_precedence() {
    check_precedence(precedence);
}

#[cfg(feature = "controls")]
fn collision_always_first(e: &Events) -> Label {
    if e.collision.fired && !e.failed.fired {
        Label {
            state: STATE_COLLISION,
            detail: e.collision.detail,
        }
    } else {
        precedence(e)
    }
}

#[cfg(feature = "controls")]
fn tie_to_escape(e: &Events) -> Label {
    if e.collision.fired && e.escape.fired && e.collision.at == e.escape.at && !e.failed.fired {
        Label {
            state: STATE_ESCAPE,
            detail: e.escape.detail,
        }
    } else {
        precedence(e)
    }
}

#[cfg(feature = "controls")]
fn failure_last(e: &Events) -> Label {
    precedence(&Events {
        failed: if e.collision.fired || e.escape.fired {
            Fired::none()
        } else {
            e.failed
        },
        ..*e
    })
}

negative_control!(
    terminal_precedence,
    "collision over escape whatever their times",
    expected = "escape before collision: collision",
    check_precedence(collision_always_first)
);

mod tie_to_escape {
    use super::*;

    negative_control!(
        terminal_precedence,
        "a same-step tie to escape",
        expected = "a same-step tie: escape",
        check_precedence(tie_to_escape)
    );
}

mod failure_last {
    use super::*;

    negative_control!(
        terminal_precedence,
        "sim_failed only where nothing else fired",
        expected = "sim_failed with a collision and an escape: collision",
        check_precedence(failure_last)
    );
}

// ── escape_latch ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The expanding triangle by KDK, two substeps a macro-step: escape fires at the first window mark, step 8; the loop
/// ends there with `done` set, `t_end_step = 8`, and no step after it. The head-on pair under `r_coll = 0.1`: the
/// collision stops the march on the step it fired on.
fn check_escape_latch(latched: bool) {
    let s = expanding::<f64>();
    let (out, c) = recorded(
        &s,
        &flat(0.05, 200, 2),
        CollisionRadius::none(),
        latched,
        true,
    );
    assert_eq!(c.escaped_at, Some(8), "escape fired off the first mark");
    assert_eq!(name(sd_state(out.packed_a)), "escape");
    assert!(c.inner.done(), "`done` is not set");
    assert!(
        tm_t_end_step(out.times) == 8 && c.inner.invocations() == 16,
        "the production loop did not end at the firing step: t_end_step {}, {} steps",
        tm_t_end_step(out.times),
        c.inner.invocations()
    );
    let s = head_on::<f64>();
    let (out, c) = recorded(
        &s,
        &flat(0.01, 1000, 2),
        CollisionRadius::from_r_coll(0.1),
        latched,
        true,
    );
    let e = c.inner.events();
    println!(
        "escape_latch: the collision fired on step {} (t_end_step {}), {} steps taken",
        e.collision.at,
        tm_t_end_step(out.times),
        c.inner.invocations()
    );
    assert_eq!(name(sd_state(out.packed_a)), "collision");
    assert!(
        c.inner.invocations() == e.collision.at && tm_t_end_step(out.times) == e.collision.at / 2,
        "a collision did not stop the march"
    );
}

#[test]
fn escape_latch() {
    check_escape_latch(true);
}

negative_control!(
    escape_latch,
    "`done` never read",
    expected = "the production loop did not end at the firing step",
    check_escape_latch(false)
);

// ── terminal_latch ──────────────────────────────────────────────────────────────────────────────────────────────────

/// A later dispatch of `stored` with `latched` the terminal latch ([`redispatch`]) or none (the march resumed and the
/// cadence's write applied regardless): a fresh cadence from the stored state, `carried` the buffer.
fn dispatch_again<R: P>(
    s: &State<R>,
    key: &WrapperUniforms<R>,
    radius: CollisionRadius,
    stored: &SimStateBaseOf<R>,
    carried: &mut Carried<R>,
    latched: bool,
) -> (SimStateBaseOf<R>, u32) {
    let mut c = WrapperCadence::with_radius(&s.m, &stored.r, &stored.p, radius);
    let out = if latched {
        redispatch::<Kdk, R, _>(&ic(s), key, &mut c, stored, carried, 100)
    } else {
        // The latch bypassed: the march from the stored state, as if it were still running.
        let resumed = Ic {
            m: s.m,
            r: stored.r,
            p: stored.p,
        };
        let out = integrate_with::<Kdk, R, _>(&resumed, key, &mut c);
        c.finish(out)
    };
    (out, c.invocations())
}

/// Synthetic samples run to a collision, to an escape and to the horizon: the loop ends with `done` set at the step
/// that terminated it (the horizon's `bounded` has no event), and every `SimState` word is bit-identical on three
/// later dispatches, under longer horizons for the two events, the carried buffer and the cadence untouched.
fn check_terminal_latch<R: P>(latched: bool) {
    let cases = [
        ("escape", expanding::<R>(), CollisionRadius::none(), 0.05),
        (
            "collision",
            head_on::<R>(),
            CollisionRadius::from_r_coll(0.1),
            0.01,
        ),
        ("bounded", bound::<R>(), CollisionRadius::none(), 0.01),
    ];
    for (what, s, radius, dt) in cases {
        let key = flat::<R>(dt, 300, 2);
        let (stored, c) = recorded(&s, &key, radius, true, true);
        assert_eq!(name(sd_state(stored.packed_a)), what);
        assert!(finished(stored.packed_a, stored.times, 300));
        if what != "bounded" {
            let e = c.inner.events();
            let at = if what == "escape" {
                e.escape.at
            } else {
                e.collision.at
            };
            assert!(
                c.inner.done() && c.inner.invocations() == at,
                "{what}: the loop did not end at the terminating step"
            );
        }
        let mut carried = Carried::<R>::unregistered();
        let before = carried;
        for k in 0..3u32 {
            let horizon = if what == "bounded" {
                300
            } else {
                300 + 400 * k
            };
            let key = flat::<R>(dt, horizon, 2);
            let (again, steps) = dispatch_again(&s, &key, radius, &stored, &mut carried, latched);
            let changed: Vec<&str> = [
                ("r", bytes(&again.r) != bytes(&stored.r)),
                ("p", bytes(&again.p) != bytes(&stored.p)),
                ("packed_a", again.packed_a != stored.packed_a),
                ("packed_b", again.packed_b != stored.packed_b),
                ("times", again.times != stored.times),
                (
                    "total_substeps",
                    again.total_substeps != stored.total_substeps,
                ),
                ("E_0", bytes(&again.E_0) != bytes(&stored.E_0)),
                ("theta", bytes(&again.theta) != bytes(&stored.theta)),
            ]
            .iter()
            .filter(|(_, d)| *d)
            .map(|(n, _)| *n)
            .collect();
            assert!(
                bytes(&again) == bytes(&stored),
                "{what} at {}: dispatch {k} changed the stored words ({changed:?}, {steps} steps taken)",
                core::any::type_name::<R>()
            );
            assert_eq!(steps, 0, "{what}: a later dispatch called the cadence");
            assert_eq!(sd_state(again.packed_a), sd_state(stored.packed_a));
        }
        assert!(carried == before, "{what}: the carried buffer was touched");
    }
}

#[test]
fn terminal_latch() {
    check_terminal_latch::<f32>(true);
    check_terminal_latch::<f64>(true);
}

negative_control!(
    terminal_latch,
    "a later dispatch with no latch",
    expected = "changed the stored words",
    check_terminal_latch::<f64>(false)
);

// ── detail_union ────────────────────────────────────────────────────────────────────────────────────────────────────

/// What `detail` means under `state` (payload §2), from the generated codes.
fn meaning(state: u32, detail: u32) -> String {
    let d = detail;
    match state {
        STATE_COLLISION => match d {
            DETAIL_COLLISION_PAIR_0 => "collision of pair 0".into(),
            DETAIL_COLLISION_PAIR_1 => "collision of pair 1".into(),
            DETAIL_COLLISION_PAIR_2 => "collision of pair 2".into(),
            DETAIL_COLLISION_ALL_THREE_TRIPLE_COLLISION => "triple collision".into(),
            _ => format!("collision with no detail {d}"),
        },
        STATE_ESCAPE => match d {
            DETAIL_ESCAPE_BODY_0 => "escape of body 0".into(),
            DETAIL_ESCAPE_BODY_1 => "escape of body 1".into(),
            DETAIL_ESCAPE_BODY_2 => "escape of body 2".into(),
            DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION => "triple ejection".into(),
            _ => format!("escape with no detail {d}"),
        },
        STATE_DECODE_FAILED => match d {
            DETAIL_DECODE_FAILED_NON_FINITE_DECODE_OUTPUT => "non-finite decode output".into(),
            DETAIL_DECODE_FAILED_DEGENERATE_CONFIGURATION => "degenerate configuration".into(),
            DETAIL_DECODE_FAILED_INVALID_MASS_CONSTRUCTION => "invalid mass construction".into(),
            DETAIL_DECODE_FAILED_OTHER_RESERVED => "other or reserved".into(),
            _ => format!("decode_failed with no detail {d}"),
        },
        // payload §2's `failed_detail`, which has no generated names.
        STATE_SIM_FAILED => match d {
            0 => "a NaN in the state".into(),
            1 => "an infinity in the state".into(),
            2 => "a non-finite derived quantity".into(),
            _ => format!("sim_failed with no detail {d}"),
        },
        STATE_BOUNDED => "bounded".into(),
        STATE_RUNNING => "running".into(),
        _ => format!("reserved state {state}"),
    }
}

type Write = fn(Label, u32) -> u32;

/// For each state and each of its detail codes, the label written over a word with every other bit set and over
/// zero: it reads back as written, with the meaning written, and every other bit is kept.
fn check_detail_union(write: Write) {
    let codes: [(u32, &[u32]); 6] = [
        (STATE_ESCAPE, &[0, 1, 2, 3]),
        (STATE_BOUNDED, &[0]),
        (STATE_COLLISION, &[0, 1, 2, 3]),
        (STATE_RUNNING, &[0]),
        (STATE_SIM_FAILED, &[0, 1, 2]),
        (STATE_DECODE_FAILED, &[0, 1, 2, 3]),
    ];
    let mask = 0b1_1111u32; // `state` bits 0–2 and `detail` bits 3–4 (payload §2).
    let unders = [0u32, !0, 0xA5A5_5A5A];
    for (state, details) in codes {
        for &detail in details {
            assert!(
                !meaning(state, detail).contains("no detail"),
                "an unnamed code"
            );
            for under in unders {
                let w = write(Label { state, detail }, under);
                let read = (sd_state(w), sd_detail(w));
                assert!(
                    read == (state, detail) && meaning(read.0, read.1) == meaning(state, detail),
                    "{} read back as {} under {under:#x}",
                    meaning(state, detail),
                    meaning(read.0, read.1)
                );
            }
        }
    }
    for (state, details) in codes {
        for &detail in details {
            for under in unders {
                let w = write(Label { state, detail }, under);
                assert_eq!(w & !mask, under & !mask, "the write touched other bits");
            }
        }
    }
    // The detail a march writes means what happened: the expanding triangle's triple ejection, the head-on pair's
    // collision of pair 2 (the side opposite body 2), and the collision's `detail` is the pair `dmin_pair` names.
    let out = integrate_detecting::<Kdk, f64>(
        &ic(&expanding()),
        &flat(0.05, 200, 2),
        CollisionRadius::none(),
    );
    assert_eq!(
        meaning(sd_state(out.packed_a), sd_detail(out.packed_a)),
        "triple ejection"
    );
    let out = integrate_detecting::<Kdk, f64>(
        &ic(&head_on()),
        &flat(0.01, 1000, 2),
        CollisionRadius::from_r_coll(0.1),
    );
    assert_eq!(
        meaning(sd_state(out.packed_a), sd_detail(out.packed_a)),
        "collision of pair 2"
    );
    assert_eq!(sd_dmin_pair(out.packed_a), sd_detail(out.packed_a));
}

#[test]
fn detail_union() {
    check_detail_union(|l, w| l.write(w));
}

negative_control!(
    detail_union,
    "the detail written into `dmin_pair`'s bits",
    expected = "read back as",
    check_detail_union(|l, w| kernel::payload::set_dmin_pair(
        Label {
            state: l.state,
            detail: 0
        }
        .write(w),
        l.detail
    ))
);

// ── t_end_step_counter ──────────────────────────────────────────────────────────────────────────────────────────────

/// A small deterministic generator for the fuzzed ICs.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// A fuzzed IC: masses in `[0.3, 1]` scaled to total 1, positions in the unit square, velocities up to `v`.
fn fuzzed<R: P>(g: &mut Lcg, v: f64) -> State<R> {
    let m: [f64; 3] = core::array::from_fn(|_| g.range(0.3, 1.0));
    let total: f64 = m.iter().sum();
    let m = m.map(|x| x / total);
    let r = core::array::from_fn(|_| [g.range(-1.0, 1.0), g.range(-1.0, 1.0)]);
    let u = core::array::from_fn(|_| [g.range(-v, v), g.range(-v, v)]);
    centred(m, r, u)
}

/// The macro-step index of the call ordinal `j` (0 the IC) from the sampling points' call counts: the last completed
/// macro-step's end at or before it, a grid point reached exactly counting as that point.
fn grid_index(j: u32, samples: &[(u32, u32)]) -> u32 {
    samples
        .iter()
        .filter(|(_, at)| *at <= j)
        .map(|(i, _)| *i)
        .max()
        .unwrap_or(0)
}

/// For fuzzed ICs and the three fixtures, run to the horizon or cut at a fuzzed step: `t_end_step` is the count of
/// completed macro-steps (each a sampling point under KDK), an escape's is the macro-step it fired at, no step is taken
/// after a terminal, and `t_dmin_step` is the macro-step index of the least `d²` the cadence saw (a later state
/// replacing an earlier only where strictly below), 0 for a failed sample. Returns the cases whose minimum is not the
/// IC's.
fn check_t_end_step(latched: bool, clocked: bool) {
    let mut g = Lcg(0x005E_ED17);
    let mut states: Vec<(State<f64>, CollisionRadius, f64)> = (0..40)
        .map(|i| {
            (
                fuzzed(&mut g, if i % 2 == 0 { 0.4 } else { 1.5 }),
                CollisionRadius::from_r_coll(0.02),
                0.01,
            )
        })
        .collect();
    states.push((expanding(), CollisionRadius::none(), 0.05));
    states.push((head_on(), CollisionRadius::from_r_coll(0.1), 0.01));
    states.push((near_singular(), CollisionRadius::none(), 0.01));
    let mut tally = std::collections::BTreeMap::new();
    let mut later_min = 0;
    for (n, (s, radius, dt)) in states.iter().enumerate() {
        let horizon = 400;
        let cut = 1 + (g.next() * horizon as f64) as u32;
        let key = law::<f64>(*dt, horizon, 16);
        // The march from the IC cut at `cut` steps: a resumed call from the IC's stored state at step 0.
        let zero = integrate_detecting::<Kdk, f64>(&ic(s), &law(*dt, 0, 16), *radius);
        let mut c = Recording::new(s, *radius);
        c.latched = latched;
        c.clocked = clocked;
        let out = integrate_resumed_with::<Kdk, f64, _>(&ic(s), &key, &mut c, &zero, cut);
        let out = c.finish(out);
        let state = sd_state(out.packed_a);
        *tally.entry(name(state)).or_insert(0) += 1;
        let t_end = tm_t_end_step(out.times);
        assert_eq!(
            t_end as usize,
            c.samples.len(),
            "case {n}: t_end_step is not the completed macro-steps"
        );
        let last = c.samples.last().map(|&(_, at)| at).unwrap_or(0);
        match state {
            STATE_ESCAPE => assert!(
                c.escaped_at == Some(t_end) && c.inner.invocations() == last,
                "case {n}: t_end_step {t_end} is not latched at the escape's firing step {:?}",
                c.escaped_at
            ),
            STATE_COLLISION => assert_eq!(
                c.inner.invocations(),
                c.inner.events().collision.at,
                "case {n}: a step was taken after the collision"
            ),
            STATE_RUNNING => assert_eq!(t_end, cut, "case {n}: the cut"),
            STATE_BOUNDED => assert_eq!(t_end, horizon),
            _ => {}
        }
        // The argmin over the states the cadence saw, the earliest of a tie.
        let mut j_min = 0;
        for (j, &d2) in c.d2.iter().enumerate() {
            if d2 < c.d2[j_min] {
                j_min = j;
            }
        }
        let expected = if state == STATE_SIM_FAILED {
            0
        } else {
            grid_index(j_min as u32, &c.samples)
        };
        if expected > 0 {
            later_min += 1;
        }
        assert_eq!(
            tm_t_dmin_step(out.times),
            expected,
            "case {n} ({}): t_dmin_step is not the argmin step (call {j_min})",
            name(state)
        );
        assert!(tm_t_dmin_step(out.times) <= t_end);
    }
    println!("t_end_step_counter: {tally:?}; {later_min} minima after the IC");
    assert!(
        tally.len() >= 3 && later_min >= 5,
        "the fuzzed set does not exercise the counters"
    );
}

#[test]
fn t_end_step_counter() {
    check_t_end_step(true, true);
}

negative_control!(
    t_end_step_counter,
    "`done` never read",
    expected = "a step was taken after the collision",
    check_t_end_step(false, true)
);

mod no_clock {
    use super::*;

    negative_control!(
        t_end_step_counter,
        "the macro-step clock dropped",
        expected = "t_dmin_step is not the argmin step",
        check_t_end_step(true, false)
    );
}

// ── totality_labels ─────────────────────────────────────────────────────────────────────────────────────────────────

/// Fuzzed ICs at `R`, slow and fast, under the law capped at `N_max = 3` and `r_coll = 0.01`, run to the horizon: every
/// sample's `state` is a finished label, one of the six (never `running`, never a reserved code), and it is the
/// precedence of the cadence's own events; every saturated sample reaches a dynamical outcome. `labelled: false` reads
/// the march's label alone, before the cadence's write.
fn check_totality<R: P>(labelled: bool) {
    let mut g = Lcg(0x707A_1171);
    let key = law::<R>(0.02, 1500, 3);
    let radius = CollisionRadius::from_r_coll(0.01);
    let mut tally = std::collections::BTreeMap::new();
    let mut saturated = 0;
    for n in 0..48 {
        let s = fuzzed::<R>(&mut g, [0.2, 0.8, 2.0][n % 3]);
        let mut c = WrapperCadence::with_radius(&s.m, &s.r, &s.p, radius);
        let out = integrate_with::<Kdk, R, _>(&ic(&s), &key, &mut c);
        let out = if labelled { c.finish(out) } else { out };
        let state = sd_state(out.packed_a);
        *tally.entry(name(state)).or_insert(0) += 1;
        assert!(
            matches!(
                state,
                STATE_ESCAPE | STATE_BOUNDED | STATE_COLLISION | STATE_SIM_FAILED
            ),
            "case {n} at {}: `{}` is not one of the finished labels",
            core::any::type_name::<R>(),
            name(state)
        );
        let events = Events {
            failed: if c.monitor_state().non_finite() {
                Fired::at(0, sd_detail(out.packed_a))
            } else {
                Fired::none()
            },
            horizon: tm_t_end_step(out.times) == 1500,
            ..c.events()
        };
        assert_eq!(
            precedence(&events).state,
            state,
            "case {n}: the label is not its events' precedence"
        );
        if sd_saturated(out.packed_a) {
            saturated += 1;
            assert!(
                matches!(state, STATE_ESCAPE | STATE_BOUNDED | STATE_COLLISION),
                "case {n}: a saturated sample did not reach a dynamical outcome: {}",
                name(state)
            );
        }
    }
    println!(
        "totality_labels at {}: {tally:?}, {saturated} saturated",
        core::any::type_name::<R>()
    );
    assert!(
        saturated >= 1 && tally.len() >= 2,
        "the fuzzed set does not exercise the labels"
    );
}

#[test]
fn totality_labels() {
    check_totality::<f32>(true);
    check_totality::<f64>(true);
}

negative_control!(
    totality_labels,
    "the march's label without the detectors' write",
    expected = "is not one of the finished labels",
    check_totality::<f64>(false)
);

// ── escape_freezes_accumulators ─────────────────────────────────────────────────────────────────────────────────────

/// The wrapper's cadence with escape never tested: the march past escape, as a dispatch that ignored the latch would
/// run it (validation's harness march is of this kind, on its own state).
struct PastEscape<R>(WrapperCadence<R>);

impl<R: P> Cadence<R> for PastEscape<R> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.0.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.0.done()
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.0.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.0.clock(completed);
    }
}

const DT: f64 = 0.05;
const N_RENORM: u32 = 1000;

fn params() -> BenettinParams<f64> {
    BenettinParams {
        delta_0: 1e-6,
        n_renorm: N_RENORM,
    }
}

/// The FTLE tier's run of the ejection to `horizon` macro-steps: the sample's cadence `Recording` (or, past escape,
/// one that never tests it), its shadow beside it, labelled and written as `integrate_ftle_with` does.
fn ftle_run(horizon: u32, past_escape: bool) -> (SimStateFTLEOf<f64>, Recording<f64>) {
    let s = ejection::<f64>();
    let key = flat::<f64>(DT, horizon, 1);
    let mut shadow = Benettin::<f64, Absolute>::new(&ic(&s), CollisionRadius::none(), params());
    let mut c = Recording::new(&s, CollisionRadius::none());
    let out = if past_escape {
        let mut p = PastEscape(c.inner);
        let out = integrate_with_co::<Kdk, f64, _, _>(&ic(&s), &key, &mut p, &mut shadow);
        c.inner = p.0;
        out
    } else {
        integrate_with_co::<Kdk, f64, _, _>(&ic(&s), &key, &mut c, &mut shadow)
    };
    (shadow.write(monitored(out, &c.inner)), c)
}

/// `(S + ln(δ/δ₀)) / (n·dt)` from the stored fields, by hand in f64: the partial interval finalised from the stored
/// shadow's `δ` at the stored step.
fn by_hand(s: &SimStateFTLEOf<f64>) -> f64 {
    let mut d2 = 0.0;
    for b in 0..3 {
        for k in 0..2 {
            d2 += (s.r_sh[b][k] - s.r[b][k]).powi(2) + (s.p_sh[b][k] - s.p[b][k]).powi(2);
        }
    }
    let n = tm_t_end_step(s.times) as f64;
    (s.S + (d2.sqrt() / params().delta_0).ln()) / (n * DT)
}

type Read = fn(&SimStateFTLEOf<f64>) -> f64;

fn read_ftle(s: &SimStateFTLEOf<f64>) -> f64 {
    ftle_read(s, &params(), DT)
}

/// The ejection at the FTLE tier, KDK at `dt = 0.05`, `n_renorm = 1000`, `‖δ₀‖ = 10⁻⁶`: escape fires off a
/// renormalisation boundary after at least one, with the partial interval's growth `|ln(δ/δ₀)| ≥ 0.01`; the loop ends
/// at the firing step; `S` advanced while the sample ran; three later dispatches under longer horizons leave every
/// word, and so `S`, `t_end_step`, the latches and the derived FTLE, as they were; and the derived FTLE read by `read`
/// is `(S + ln(δ/δ₀))/(t_esc)` by hand, to `10⁻¹²` relative, which plain `S/t` misses by more than `10⁻⁶`.
fn check_freeze(read: Read, latched: bool) {
    let horizon = 14000;
    let (stored, c) = ftle_run(horizon, false);
    let t_esc = tm_t_end_step(stored.times);
    assert_eq!(name(sd_state(stored.packed_a)), "escape");
    assert!(
        c.escaped_at == Some(t_esc)
            && c.inner.invocations() == t_esc
            && c.samples.last().map(|s| s.0) == Some(t_esc),
        "the production loop did not end at the firing step"
    );
    let partial = {
        let d: f64 = (by_hand(&stored) * t_esc as f64 * DT) - stored.S;
        d
    };
    let renorms = completed_renorms(t_esc, N_RENORM);
    println!(
        "escape_freezes_accumulators: escape at step {t_esc} (t = {:.2}), {renorms} renormalisations, {} steps past \
         the last; S = {:.6}, partial ln(δ/δ₀) = {partial:.6}",
        t_esc as f64 * DT,
        t_esc % N_RENORM,
        stored.S
    );
    assert!(
        renorms >= 1 && !t_esc.is_multiple_of(N_RENORM) && partial.abs() >= 0.01,
        "the fixture does not fire off a renormalisation boundary after one, with a clearly nonzero partial interval"
    );
    // `S` advanced while the sample ran: it differs between the first renormalisation and the escape.
    let (early, _) = ftle_run(N_RENORM + 1, false);
    assert_eq!(sd_state(early.packed_a), STATE_BOUNDED);
    assert!(
        early.S != 0.0 && early.S != stored.S,
        "S did not advance while the sample ran"
    );
    // The derived FTLE, finalised at the firing step.
    let ftle = read(&stored);
    let expected = by_hand(&stored);
    let plain = stored.S / (t_esc as f64 * DT);
    println!(
        "escape_freezes_accumulators: FTLE {ftle:.9}, by hand {expected:.9}, plain S/t {plain:.9}; the march past \
         escape reads {:.9} at step {horizon}",
        read_ftle(&ftle_run(horizon, true).0)
    );
    assert!(
        (plain - expected).abs() > 1e-6 * expected.abs(),
        "plain S/t cannot tell the finalised FTLE apart here"
    );
    assert!(
        (ftle - expected).abs() <= 1e-12 * expected.abs(),
        "the derived FTLE {ftle} is not S_final/(t_esc) at the firing step, {expected}"
    );
    // Later dispatches: the latch hands the stored sample back; without it, the march past escape.
    for k in 1..=3u32 {
        let later = horizon + 2000 * k;
        let again = if latched {
            latch(&stored, later, |_| ftle_run(later, true).0)
        } else {
            ftle_run(later, true).0
        };
        assert!(
            bytes(&again) == bytes(&stored),
            "dispatch {k} changed the frozen sample: state {}, t_end_step {} → {}, S {} → {}",
            name(sd_state(again.packed_a)),
            t_esc,
            tm_t_end_step(again.times),
            stored.S,
            again.S
        );
        assert_eq!(read(&again).to_bits(), ftle.to_bits());
    }
}

#[test]
fn escape_freezes_accumulators() {
    check_freeze(read_ftle, true);
}

#[cfg(feature = "controls")]
fn plain_s_over_t(s: &SimStateFTLEOf<f64>) -> f64 {
    s.S / (tm_t_end_step(s.times) as f64 * DT)
}

negative_control!(
    escape_freezes_accumulators,
    "plain S/t, the partial interval left unfinalised",
    expected = "is not S_final/(t_esc) at the firing step",
    check_freeze(plain_s_over_t, true)
);

mod unlatched {
    use super::*;

    negative_control!(
        escape_freezes_accumulators,
        "later dispatches with no latch",
        expected = "changed the frozen sample",
        check_freeze(read_ftle, false)
    );
}
