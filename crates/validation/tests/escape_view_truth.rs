//! The capture view's escape labels scored against continued runs.
//!
//! **The view** is the app's capture view (`gui::real::capture_view`): the latent chart `z_α × z_β`, untilted, every
//! slice offset 0, 64 × 64 samples, the horizon 50, computed as the app's CPU field backend computes it
//! (`engine::field::Cpu`): decoded at f64, narrowed once to f32, the dispatch-time label, then Heggie × RK4 at f32 over
//! the sync grid with the wrapper's cadence watching every step. Validation cannot import the GUI, so [`capture_view`]
//! rebuilds the view; the field backend reads only its plane, window, horizon and kernel variant.
//!
//! **One variable at a time.** [`outcome_rule`] is `engine::field::outcome_stopping` with the escape rule a type
//! parameter: at the default rule it is the field backend's own sample, bit for bit (`view_replica_is_the_field_backend`).
//! The field before PR #67 is Closure with the stop option on; after it, Fewbody live. The other two corners separate
//! the rule from the stop option.
//!
//! **The truth** is #49's continued-run truth (`validation::escape_checks::rules::truth`) on the app's own march
//! continued: the same f32 march, the same cadence, the same sync grid, marched on past the horizon with a record every
//! 2 time units until the motion is asymptotic, never before `T_eval = 3 × 50`, at the latest `T_truth = 10 × T_eval`
//! ([`continued`]). The march to the horizon does not depend on how far it is marched (the step budget reads elapsed
//! time), so its label at `t = 50` is the field's (`view_continued_run_is_the_field`). The f64 run of the same IC by the
//! same code is a second column.
//!
//! **The measurements** run in release (`cargo test --release -p validation --test escape_view_truth -- --ignored
//! --nocapture`); their pass criteria are written in each before it ran. The guards run in every build.
//!
//! **Domain**: this view, its horizon and the field backend's step-control values; the default escape rule's register.

use std::collections::BTreeMap;

use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use engine::decode_dispatch::decode_only_with;
use engine::field::{boundaries, heggie_params, latent, Field, Outcome};
use engine::first_slice::{narrow, uniforms, Quad};
use kernel::chart::Latent;
use kernel::chazy::{
    kepler, ma_stable, most_separated, others, out_kup, pair_kup, tidal_ratio, Playhead,
};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape::is_mark;
use kernel::detect::escape::EscapeReal;
use kernel::detect::escape_rule::{fewbody_test, Closure, DefaultRule, EscapeRule, Fewbody};
use kernel::detect::t0::t0;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{monitored, WrapperReal};
use kernel::generated::constants::CHAZY_DELTA;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, set_detail, set_state, SimStateBaseOf, STATE_BOUNDED, STATE_COLLISION,
    STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{AzReal, End, ExplicitTest};
use kernel::regularisation::heggie::{march, march_on, HgConfig, HgParams, HgStanding};
use render::colour::outcome::{shown, Shown};

use validation::escape_checks::march::parallel;
use validation::escape_checks::rules::{self, Record, Truth};
use validation::ground_truth::FieldHeggie;
use validation::negative_control;

// ── The view ────────────────────────────────────────────────────────────────────────────────────────────────────

/// The samples across and up (`gui::real::FIELD_SIZE`).
const SIZE: [usize; 2] = [64, 64];

/// The horizon (`gui::mock::MOCK_T_MAX`).
const HORIZON: f64 = 50.0;

/// The evaluation horizon of the continued run, `3 × HORIZON`, as #49 set it (`T_eval = 3 · t_max`).
const T_EVAL: f64 = 3.0 * HORIZON;

/// The truth's horizon, `rules::TRUTH_FACTOR × T_EVAL`.
const T_TRUTH: f64 = rules::TRUTH_FACTOR * T_EVAL;

/// The app's capture view (`gui::real::capture_view`): `gui::mock::state(0.0)` with `z₀` at the origin. The fields the
/// field backend reads are the plane, the window, the horizon and the kernel variant.
fn capture_view() -> SimConfig {
    let mut q1 = [0.0; 8];
    let mut q2 = [0.0; 8];
    q1[0] = 1.0;
    q2[1] = 1.0;
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: HORIZON },
        collision: Collision::default(),
        quality: Quality {},
    }
}

/// One sample of the view: its IC at f64 as decoded, narrowed to f32 as the field backend marches it, and its tag.
#[derive(Clone, Copy, Debug)]
struct Cell {
    ic64: Ic<f64>,
    ic: Ic<f32>,
    degenerate: Option<Degenerate>,
}

/// The samples of `sim`'s chart over its window, `size`, decoded as `engine::field::Cpu::field_on` decodes them, row by
/// row from the bottom left.
fn cells(sim: &SimConfig, size: [usize; 2]) -> Vec<Cell> {
    let quad = Quad {
        chart: latent(sim),
        window: sim.chart.window,
        size,
    };
    let key = SimConfig {
        kernel_variant: sim.kernel_variant,
        ..quad.sim_key()
    };
    (0..quad.len())
        .map(|i| {
            let out =
                decode_only_with::<Latent<f32>, Defaults>(&quad.chart, quad.uv(i), &key, None)
                    .expect("the capture view decodes");
            Cell {
                ic64: out.ic,
                ic: narrow(&out.ic),
                degenerate: out.degenerate,
            }
        })
        .collect()
}

/// The field backend's march values for `sim`: the collision threshold, the horizon's sync boundary and Heggie's values.
fn march_values(sim: &SimConfig) -> (CollisionRadius, u32, HgParams<f32>) {
    let key = uniforms(sim.horizon.t).expect("the horizon dispatches");
    let params = heggie_params();
    (key.radius, boundaries(sim.horizon.t, &params), params)
}

/// `sim_failed`'s detail for a march that ended at `s` (`engine::field`'s own).
fn failed_detail<R: WrapperReal>(s: &State<R>) -> u32 {
    let values = || s.r.iter().chain(s.p.iter()).flatten();
    if values().any(|x| x.is_nan()) {
        0
    } else if values().any(|x| x.is_infinite()) {
        1
    } else {
        2
    }
}

/// `engine::field::outcome_stopping` under the escape rule `E`: the field backend's sample, its rule a parameter.
fn outcome_rule<E: EscapeRule<f32>>(
    c: &Cell,
    radius: CollisionRadius,
    k_target: u32,
    params: &HgParams<f32>,
    stop_on_escape: bool,
) -> Outcome {
    let ic = &c.ic;
    let start = t0(ic, c.degenerate, &radius);
    if start.state != STATE_RUNNING {
        return Outcome {
            state: start.state,
            detail: start.detail,
        };
    }
    let mut cadence = WrapperCadence::<f32, E>::with_rule(&ic.m, &ic.r, &ic.p, radius)
        .stopping_on_escape(stop_on_escape);
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let out = march::<Rk4, f32, _, _>(&state, k_target, params, &mut cadence, &mut ExplicitTest);
    let (code, detail) = match out.count.end {
        End::NonFinite => (STATE_SIM_FAILED, failed_detail(&out.state)),
        End::Exhausted => (STATE_RUNNING, 0),
        End::Done | End::Marching if out.boundary >= k_target => (STATE_BOUNDED, 0),
        End::Done | End::Marching => (STATE_RUNNING, 0),
    };
    let base = SimStateBaseOf::<f32> {
        r: out.state.r,
        p: out.state.p,
        packed_a: set_detail(set_state(0, code), detail),
        ..SimStateBaseOf::default()
    };
    let labelled = monitored(base, &cadence);
    Outcome {
        state: sd_state(labelled.packed_a),
        detail: sd_detail(labelled.packed_a),
    }
}

/// The field of `cells` under the rule `E` and the stop option, on every core.
fn field_rule<E: EscapeRule<f32>>(cells: &[Cell], sim: &SimConfig, stop: bool) -> Field {
    let (radius, k, params) = march_values(sim);
    let outcomes = parallel(cells.len(), |i| {
        Ok(outcome_rule::<E>(&cells[i], radius, k, &params, stop))
    })
    .expect("the field");
    Field {
        size: SIZE,
        outcomes,
    }
}

/// The image row (0 at the top) and column of sample `i`.
fn row_col(i: usize) -> (usize, usize) {
    let (x, y) = (i % SIZE[0], i / SIZE[0]);
    (SIZE[1] - 1 - y, x)
}

/// The palette class a sample is drawn in: its swatch, `None` for the canvas background.
fn class(o: Outcome) -> Option<usize> {
    match shown(o.state, o.detail, false) {
        Shown::Swatch(k) => Some(k),
        _ => None,
    }
}

/// Whether the palette draws `o` as an escape.
fn is_escape(o: Outcome) -> bool {
    o.state == STATE_ESCAPE
}

/// Whether the palette draws `o` as bounded / nothing yet (`bounded`, `running`, `sim_failed`, the reserved codes).
fn is_nothing_yet(o: Outcome) -> bool {
    class(o) == Some(0)
}

// ── The continued run ───────────────────────────────────────────────────────────────────────────────────────────

/// The width the continued run marches at: f32, the field's, or f64, the second column.
trait Width: WrapperReal + AzReal + Into<f64> + Send + Sync {}

impl Width for f32 {}
impl Width for f64 {}

/// A window mark of a march: its time, the state there and the rule's label.
type MarkAt<R> = (f64, State<R>, Option<u32>);

/// The wrapper's cadence under the rule `E`, live, every hook forwarded, so the march is the field backend's to the
/// step; at every sampling point it keeps the rule's label when it changes, the record every [`rules::SERIES_EVERY`]
/// and, up to the boundary `k_path`, the state at every boundary.
struct Continued<R, E> {
    inner: WrapperCadence<R, E>,
    /// The horizon's boundary, the label and the state there.
    k_h: u32,
    at_h: Option<(Option<u32>, State<R>)>,
    /// The label changes: time and `detail`, `None` no escape.
    labels: Vec<(f64, Option<u32>)>,
    series: Vec<Record>,
    next_record: f64,
    asymptotic: bool,
    last: Option<State<R>>,
    k_path: u32,
    path: Vec<State<R>>,
    /// At every window mark up to `t_marks`, the time, the state and the rule's label there (the drop diagnostic).
    t_marks: f64,
    marks: Vec<MarkAt<R>>,
}

impl<R: Width, E: EscapeRule<R>> Cadence<R> for Continued<R, E> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
    }

    /// A collision or a failure ends it, as the field's march; or the motion turned asymptotic after `T_eval`.
    fn done(&self) -> bool {
        self.inner.done() || self.asymptotic
    }

    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.inner.sample(state, index, quantum);
        let t = f64::from(index) * quantum.into();
        let v = self.inner.escape().verdict();
        let label = v.fired.then_some(v.detail);
        if self
            .labels
            .last()
            .map_or(label.is_some(), |(_, l)| *l != label)
        {
            self.labels.push((t, label));
        }
        if index == self.k_h {
            self.at_h = Some((label, *state));
        }
        if index <= self.k_path {
            self.path.push(*state);
        }
        if t <= self.t_marks && is_mark(index, quantum) {
            self.marks.push((t, *state, label));
        }
        if t >= self.next_record {
            let (m, r) = widen(state);
            let rec = rules::record(&m, &r, t);
            self.next_record += rules::SERIES_EVERY;
            let least = rec.d.iter().copied().fold(f64::INFINITY, f64::min);
            if t >= T_EVAL && (rec.ratio > rules::ASYMPTOTIC || least > rules::ASYMPTOTIC) {
                self.asymptotic = true;
            }
            self.series.push(rec);
        }
        self.last = Some(*state);
    }

    fn signals(&mut self, floored: bool, cap_hit: bool) {
        self.inner.signals(floored, cap_hit);
    }

    fn regularised_drift(&mut self, drift: R) {
        self.inner.regularised_drift(drift);
    }

    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn escape_final(&self) -> bool {
        self.inner.escape_final()
    }

    fn latched(&mut self) {
        self.inner.latched();
    }

    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    fn failure(&self) -> kernel::detect::label::Fired {
        self.inner.failure()
    }
}

/// `(m, r)` of `s` at f64.
fn widen<R: Width>(s: &State<R>) -> ([f64; 3], [[f64; 2]; 3]) {
    (s.m.map(Into::into), s.r.map(|v| v.map(Into::into)))
}

/// `s` at f64.
fn widen_state<R: Width>(s: &State<R>) -> State<f64> {
    State {
        m: s.m.map(Into::into),
        r: s.r.map(|v| v.map(Into::into)),
        p: s.p.map(|v| v.map(Into::into)),
    }
}

/// How a continued run ended (`rules::End`'s reading).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ended {
    /// Asymptotic after `T_eval`.
    Asymptotic,
    /// `T_truth` reached.
    Horizon,
    /// A collision at this time: censored.
    Collision(u32),
    /// A failure or a spent budget: censored.
    Failed,
}

/// One continued run.
#[derive(Clone, Debug)]
struct Run {
    /// The label at the horizon's boundary (`None` no escape; also `None` where the run ended before it).
    label_h: Option<u32>,
    /// Whether the run reached the horizon's boundary.
    reached_h: bool,
    /// The state at the horizon's boundary, at f64.
    state_h: Option<State<f64>>,
    labels: Vec<(f64, Option<u32>)>,
    truth: Truth,
    /// Where the truth is no escape: Mardling–Aarseth on the last state.
    b_stable: bool,
    ended: Ended,
    t_end: f64,
    /// The record's last entry and its exponents.
    last_record: Option<(Record, [f64; 3])>,
    /// The states at every boundary up to `k_path` (empty unless asked).
    path: Vec<State<f64>>,
    /// The march's outcome code at the end: collision's pair, where one ended it.
    collision_detail: Option<u32>,
}

/// The field backend's march of `ic` at the width `R` under the rule `E`, live, with Heggie's values `params`, marched
/// on to `t_stop` (`T_TRUTH` for the truth: the module's note); the states at every boundary up to `k_path` kept.
fn continued<R: Width, E: EscapeRule<R>>(
    ic: &Ic<R>,
    radius: CollisionRadius,
    params: &HgParams<R>,
    k_path: u32,
    t_stop: f64,
) -> Run {
    let delta: f64 = params.delta_sync.into();
    let k_h = (HORIZON / delta).floor() as u32;
    let k_truth = (t_stop / delta).floor() as u32;
    let inner = WrapperCadence::<R, E>::with_rule(&ic.m, &ic.r, &ic.p, radius);
    let e_0: f64 = inner.monitor_state().e_0().into();
    let mut h = Continued {
        inner,
        k_h,
        at_h: None,
        labels: Vec::new(),
        series: Vec::new(),
        next_record: rules::SERIES_EVERY,
        asymptotic: false,
        last: None,
        k_path,
        path: Vec::new(),
        t_marks: -1.0,
        marks: Vec::new(),
    };
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let out = march::<Rk4, R, _, _>(&state, k_truth, params, &mut h, &mut ExplicitTest);
    let events = h.inner.events();
    let t_end = f64::from(out.boundary) * delta;
    let ended = if events.collision.fired {
        Ended::Collision(out.boundary)
    } else if events.failed.fired || matches!(out.count.end, End::NonFinite | End::Exhausted) {
        Ended::Failed
    } else if h.asymptotic {
        Ended::Asymptotic
    } else if out.boundary >= k_truth {
        Ended::Horizon
    } else {
        Ended::Failed
    };
    let last = h.last.map(|s| widen_state(&s));
    let truth = match (ended, last) {
        (Ended::Asymptotic | Ended::Horizon, Some(last)) => rules::truth(&h.series, &last.m, &last),
        _ => Truth::Censored,
    };
    let b_stable = truth == Truth::NoEscape
        && last.is_some_and(|l| {
            let ph = Playhead::from_momenta(l.m, l.r, l.p, e_0, 1.0, 1.0);
            let b = most_separated(&ph);
            let (i, j) = others(b);
            ma_stable(&ph, i, j, b)
        });
    let collision_detail = events.collision.fired.then(|| {
        let base = SimStateBaseOf::<R> {
            r: out.state.r,
            p: out.state.p,
            packed_a: set_detail(set_state(0, STATE_BOUNDED), 0),
            ..SimStateBaseOf::default()
        };
        sd_detail(monitored(base, &h.inner).packed_a)
    });
    Run {
        label_h: h.at_h.and_then(|(l, _)| l),
        reached_h: h.at_h.is_some(),
        state_h: h.at_h.map(|(_, s)| widen_state(&s)),
        labels: h.labels,
        truth,
        b_stable,
        ended,
        t_end,
        last_record: rules::exponents(&h.series),
        path: h.path.iter().map(widen_state).collect(),
        collision_detail,
    }
}

/// Heggie's values the field backend marches with, at `R` (`FieldHeggie`, `engine::field::heggie_params` at f32), its
/// `η` scaled by `eta_scale`.
fn params_at<R: Width>(eta_scale: f64) -> HgParams<R> {
    let p = FieldHeggie::params::<R>();
    let eta: f64 = p.eta.into();
    HgParams {
        eta: R::from(eta * eta_scale).expect("η at R"),
        ..p
    }
}

/// `ic` at f64.
fn widen_ic(ic: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: ic.m.map(f64::from),
        r: ic.r.map(|v| v.map(f64::from)),
        p: ic.p.map(|v| v.map(f64::from)),
    }
}

// ── Scoring ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A horizon label's verdict against the continued run's truth, the escaping body counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
    /// The truth's class and, for a single escape, its body.
    Right,
    /// A single escape against a dispersal, or the reverse.
    WrongClass,
    /// A single escape of another body.
    WrongBody,
    /// An escape on a Mardling–Aarseth stable truth: wrong.
    WrongOnBStable,
    /// An escape on a truth that is no escape and not certified stable by `T_truth`: neither right nor wrong.
    Unresolved,
    /// The continued run was censored (a later collision, a failure): unjudged.
    Censored,
}

/// The verdict of the escape label `detail` against `run`'s truth.
fn verdict(detail: u32, run: &Run) -> Verdict {
    let triple = detail == 3;
    match run.truth {
        Truth::Dispersal if triple => Verdict::Right,
        Truth::Dispersal => Verdict::WrongClass,
        Truth::Escape(_) if triple => Verdict::WrongClass,
        Truth::Escape(b) if b == detail => Verdict::Right,
        Truth::Escape(_) => Verdict::WrongBody,
        Truth::NoEscape | Truth::Bound if run.b_stable => Verdict::WrongOnBStable,
        Truth::NoEscape | Truth::Bound => Verdict::Unresolved,
        Truth::Censored => Verdict::Censored,
    }
}

/// The escape `detail`'s name: the body (0-based, the payload's), or the triple ejection.
fn label_name(detail: u32) -> String {
    if detail == 3 {
        "triple ejection".into()
    } else {
        format!("escape body {detail}")
    }
}

/// The truth's name, B-stable or unresolved where it is no escape.
fn truth_name(run: &Run) -> String {
    match run.truth {
        Truth::NoEscape if run.b_stable => "no escape (B-stable)".into(),
        Truth::NoEscape => "no escape (unresolved)".into(),
        Truth::Escape(b) => format!("escape body {b}"),
        Truth::Dispersal => "dispersal (triple ejection)".into(),
        t => format!("{t:?}"),
    }
}

// ── Guards ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// Cells of the capture view, by index, row by row from the bottom left, that the guards march (cheap in a debug build).
/// The plan is unclear on how many cells a guard should march in every build; chose, for each label (bodies 0, 1, 2),
/// the first of the 370 cells that went from nothing yet to escape at #67 whose mirror pair agrees (so not a cell where
/// f32 rounding decides the outcome), which Closure and Fewbody label apart, so the Closure controls fail on them,
/// because the whole view takes minutes unoptimised.
const GUARD_CELLS: [usize; 3] = [160, 46, 188];

/// The replica under the rule `E` against the field backend's own sample, both stop settings, on `GUARD_CELLS`.
fn check_replica<E: EscapeRule<f32>>() {
    let sim = capture_view();
    let all = cells(&sim, SIZE);
    let (radius, k, params) = march_values(&sim);
    for i in GUARD_CELLS {
        for stop in [false, true] {
            let ours = outcome_rule::<E>(&all[i], radius, k, &params, stop);
            let theirs = engine::field::outcome_stopping(
                &all[i].ic,
                all[i].degenerate,
                radius,
                k,
                &params,
                stop,
            );
            assert_eq!(
                ours, theirs,
                "cell {i}, stop {stop}: the replica is not the field backend"
            );
        }
    }
}

#[test]
fn view_replica_is_the_field_backend() {
    check_replica::<DefaultRule<f32>>();
}

negative_control!(
    view_replica_is_the_field_backend,
    "the replica under Closure against the field backend's default rule",
    expected = "the replica is not the field backend",
    check_replica::<Closure<f32>>()
);

/// The continued run under the rule `E` read at the horizon's boundary against the field backend's default-rule sample
/// on `GUARD_CELLS`: the same escape label, or both bounded.
fn check_continued<E: EscapeRule<f32>>() {
    let sim = capture_view();
    let all = cells(&sim, SIZE);
    let (radius, k, params) = march_values(&sim);
    for i in GUARD_CELLS {
        let field = engine::field::outcome(&all[i].ic, all[i].degenerate, radius, k, &params);
        // Past the horizon by a few windows only: the guard reads the horizon, which does not depend on how far it goes.
        let run = continued::<f32, E>(&all[i].ic, radius, &params_at::<f32>(1.0), 0, HORIZON + 2.0);
        assert!(
            run_is_cell(&run, field),
            "cell {i}: the continued run is not the field's march ({:?} against {field:?})",
            run.label_h
        );
    }
}

#[test]
fn view_continued_run_is_the_field() {
    check_continued::<DefaultRule<f32>>();
}

negative_control!(
    view_continued_run_is_the_field,
    "the continued run under Closure against the field's default rule",
    expected = "the continued run is not the field's march",
    check_continued::<Closure<f32>>()
);

/// The decoded ICs of every mirror pair in the column `col` against `map` applied to the lower cell's: the greatest
/// component difference, positions and momenta, at f64.
fn mirror_ic_gap(col: usize, map: fn(&State<f64>) -> State<f64>) -> f64 {
    let sim = capture_view();
    let quad = Quad {
        chart: latent(&sim),
        window: sim.chart.window,
        size: SIZE,
    };
    let key = SimConfig {
        kernel_variant: sim.kernel_variant,
        ..quad.sim_key()
    };
    let ic = |i: usize| {
        let ic = decode_only_with::<Latent<f32>, Defaults>(&quad.chart, quad.uv(i), &key, None)
            .expect("decodes")
            .ic;
        State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        }
    };
    (0..SIZE[1] / 2)
        .map(|y| {
            let i = y * SIZE[0] + col;
            let (a, b) = (ic(i), map(&ic(mirror_index(i))));
            let mut g = 0.0f64;
            for k in 0..3 {
                for x in 0..2 {
                    g = g.max((a.r[k][x] - b.r[k][x]).abs());
                    g = g.max((a.p[k][x] - b.p[k][x]).abs());
                }
            }
            g
        })
        .fold(0.0, f64::max)
}

/// The mirror `z_β → −z_β` of the decoded IC is `x → −x` with bodies 0 and 1 swapped, to f64 rounding: the map
/// [`mirror_outcome`] relabels the field by. Criterion: every pair in three columns within `1e-14` (a few ulps of the
/// positions' unit scale). Control: the reflection without the relabelling.
fn check_mirror_map(map: fn(&State<f64>) -> State<f64>) {
    for col in [5, 18, 40] {
        let g = mirror_ic_gap(col, map);
        assert!(
            g <= 1e-14,
            "column {col}: the mirror's IC is not the relabelled reflection ({g:.3e})"
        );
    }
}

#[test]
fn view_mirror_map_is_the_relabelled_reflection() {
    check_mirror_map(mirror_state);
}

negative_control!(
    view_mirror_map_is_the_relabelled_reflection,
    "the reflection with the bodies left in place",
    expected = "the mirror's IC is not the relabelled reflection",
    check_mirror_map(|s| State {
        m: s.m,
        r: s.r.map(|v| [-v[0], v[1]]),
        p: s.p.map(|v| [-v[0], v[1]]),
    })
);

/// A run whose truth is `truth`, for the verdict's guard.
fn run_with(truth: Truth, b_stable: bool) -> Run {
    Run {
        label_h: None,
        reached_h: true,
        state_h: None,
        labels: Vec::new(),
        truth,
        b_stable,
        ended: Ended::Horizon,
        t_end: T_TRUTH,
        last_record: None,
        path: Vec::new(),
        collision_detail: None,
    }
}

/// The verdict counts the body (the owner's rule: right class, wrong body is wrong), keeps an unresolved truth apart
/// from wrong and a censored one unjudged, on constructed truths.
fn check_verdict(verdict: fn(u32, &Run) -> Verdict) {
    let cases = [
        (1, run_with(Truth::Escape(1), false), Verdict::Right),
        (0, run_with(Truth::Escape(1), false), Verdict::WrongBody),
        (3, run_with(Truth::Escape(1), false), Verdict::WrongClass),
        (2, run_with(Truth::Dispersal, false), Verdict::WrongClass),
        (3, run_with(Truth::Dispersal, false), Verdict::Right),
        (2, run_with(Truth::NoEscape, true), Verdict::WrongOnBStable),
        (2, run_with(Truth::NoEscape, false), Verdict::Unresolved),
        (2, run_with(Truth::Censored, false), Verdict::Censored),
    ];
    for (detail, run, want) in cases {
        assert_eq!(
            verdict(detail, &run),
            want,
            "the verdict of {detail} on {:?}: the scoring is not the owner's",
            run.truth
        );
    }
}

#[test]
fn view_verdict_counts_the_body() {
    check_verdict(verdict);
}

negative_control!(
    view_verdict_counts_the_body,
    "a verdict that reads only the class",
    expected = "the scoring is not the owner's",
    check_verdict(|d, run| match (verdict(d, run), run.truth) {
        (Verdict::WrongBody, _) => Verdict::Right,
        (v, _) => v,
    })
);

// ── Measurements ────────────────────────────────────────────────────────────────────────────────────────────────

/// The four corners of the two changes PR #67 made: the rule (Closure, Fewbody) and the stop option (on, off).
struct Corners {
    cells: Vec<Cell>,
    /// Before #67: Closure, the first label stops the march and stays.
    closure_stop: Field,
    /// The stop option alone changed: Closure, live.
    closure_live: Field,
    /// The rule alone changed: Fewbody, the first label stops.
    fewbody_stop: Field,
    /// After #67: Fewbody, live.
    fewbody_live: Field,
}

fn corners() -> Corners {
    let sim = capture_view();
    let cells = cells(&sim, SIZE);
    Corners {
        closure_stop: field_rule::<Closure<f32>>(&cells, &sim, true),
        closure_live: field_rule::<Closure<f32>>(&cells, &sim, false),
        fewbody_stop: field_rule::<Fewbody<f32>>(&cells, &sim, true),
        fewbody_live: field_rule::<Fewbody<f32>>(&cells, &sim, false),
        cells,
    }
}

/// The mirror `z_β → −z_β` of sample `i`: the same column, the row reflected.
fn mirror_index(i: usize) -> usize {
    let (x, y) = (i % SIZE[0], i / SIZE[0]);
    (SIZE[1] - 1 - y) * SIZE[0] + x
}

/// The body permutation of the mirror: bodies 0 and 1 (the payload's indices; the owner's 1 and 2) swap, body 2 stays.
const MIRROR_BODY: [usize; 3] = [1, 0, 2];

/// An outcome relabelled by the mirror: an escape's body and a collision's pair (named by the body it leaves out)
/// through [`MIRROR_BODY`]; a triple, and every other state, unchanged.
fn mirror_outcome(o: Outcome) -> Outcome {
    let relabel = (o.state == STATE_ESCAPE || o.state == STATE_COLLISION) && o.detail < 3;
    Outcome {
        detail: if relabel {
            MIRROR_BODY[o.detail as usize] as u32
        } else {
            o.detail
        },
        ..o
    }
}

/// The state mirrored: `x → −x`, bodies relabelled through [`MIRROR_BODY`].
fn mirror_state(s: &State<f64>) -> State<f64> {
    State {
        m: std::array::from_fn(|b| s.m[MIRROR_BODY[b]]),
        r: std::array::from_fn(|b| [-s.r[MIRROR_BODY[b]][0], s.r[MIRROR_BODY[b]][1]]),
        p: std::array::from_fn(|b| [-s.p[MIRROR_BODY[b]][0], s.p[MIRROR_BODY[b]][1]]),
    }
}

/// A map from a state to its mirror.
type Mirror = fn(&State<f64>) -> State<f64>;

/// The greatest component difference between `a` and `map`'s mirror of `b`, positions and momenta.
fn mirror_gap(a: &State<f64>, b: &State<f64>, map: Mirror) -> f64 {
    let mb = map(b);
    let mut g = 0.0f64;
    for k in 0..3 {
        for x in 0..2 {
            g = g.max((a.r[k][x] - mb.r[k][x]).abs());
            g = g.max((a.p[k][x] - mb.p[k][x]).abs());
        }
    }
    g
}

/// The pairs `(i, mirror)`, `i` in the lower half, whose palette classes disagree under the mirror in `field`.
fn mirror_disagreements(field: &Field) -> Vec<usize> {
    (0..SIZE[0] * SIZE[1] / 2)
        .filter(|&i| {
            let j = mirror_index(i);
            class(field.outcomes[i]) != class(mirror_outcome(field.outcomes[j]))
        })
        .collect()
}

/// The anchors the Mac measured on this view (the brief): 74 escapes before #67 and 444 after; 370 cells from
/// bounded / nothing yet to escape; no collision cell changed and no escape changed body; the mirror disagreeing on 128
/// of 2048 pairs before and 133 after ([`MAC`]). Pass criterion, written before the run: each count equal, and the replica's
/// after-field equal to `engine::field::field` on every cell. A difference says this is not the view the Mac measured,
/// and is reported, not adjusted.
///
/// Then item 1: over the 370, which change alone already labels the cell an escape. No threshold: a measurement, its
/// four-way split summing to 370 the only check.
#[test]
#[ignore = "the four 64 × 64 fields: about a minute in release"]
fn view_corners_separate_the_rule_from_the_stop_option() {
    let c = corners();
    let sim = capture_view();
    let app = engine::field::field(&sim, sim.chart.window, SIZE).expect("the app's field");
    let differ = (0..app.outcomes.len())
        .filter(|&i| app.outcomes[i] != c.fewbody_live.outcomes[i])
        .count();
    println!(
        "the replica (Fewbody, live) against engine::field::field: {differ} of 4096 cells differ"
    );

    let escapes = |f: &Field| f.outcomes.iter().filter(|&&o| is_escape(o)).count();
    println!(
        "escapes: closure+stop {}, closure live {}, fewbody+stop {}, fewbody live {}",
        escapes(&c.closure_stop),
        escapes(&c.closure_live),
        escapes(&c.fewbody_stop),
        escapes(&c.fewbody_live)
    );
    for (name, f) in [
        ("closure+stop", &c.closure_stop),
        ("closure live", &c.closure_live),
        ("fewbody+stop", &c.fewbody_stop),
        ("fewbody live", &c.fewbody_live),
    ] {
        println!(
            "  {name}: raw (state, detail) histogram {:?}",
            f.histogram()
        );
    }

    // Every cell's change of palette class, before → after.
    let mut moves: BTreeMap<(Option<usize>, Option<usize>), usize> = BTreeMap::new();
    for i in 0..c.cells.len() {
        let (a, b) = (
            class(c.closure_stop.outcomes[i]),
            class(c.fewbody_live.outcomes[i]),
        );
        if a != b {
            *moves.entry((a, b)).or_default() += 1;
        }
    }
    println!("palette class changes before → after (swatch index): {moves:?}");
    let flipped: Vec<usize> = (0..c.cells.len())
        .filter(|&i| {
            is_nothing_yet(c.closure_stop.outcomes[i]) && is_escape(c.fewbody_live.outcomes[i])
        })
        .collect();
    let collision_changed = (0..c.cells.len())
        .filter(|&i| {
            let (a, b) = (c.closure_stop.outcomes[i], c.fewbody_live.outcomes[i]);
            (a.state == STATE_COLLISION || b.state == STATE_COLLISION) && class(a) != class(b)
        })
        .count();
    let escape_changed_body = (0..c.cells.len())
        .filter(|&i| {
            let (a, b) = (c.closure_stop.outcomes[i], c.fewbody_live.outcomes[i]);
            is_escape(a) && is_escape(b) && a.detail != b.detail
        })
        .count();
    let escape_lost = (0..c.cells.len())
        .filter(|&i| {
            is_escape(c.closure_stop.outcomes[i]) && !is_escape(c.fewbody_live.outcomes[i])
        })
        .count();
    let before = mirror_disagreements(&c.closure_stop);
    let after = mirror_disagreements(&c.fewbody_live);
    println!(
        "bounded → escape {}; collision cells changed {collision_changed}; escapes that changed body {escape_changed_body}; \
         escapes lost {escape_lost}; mirror disagreements before {} after {} (of 2048 pairs)",
        flipped.len(),
        before.len(),
        after.len()
    );

    // Item 1: the 370 by which change alone labels them.
    let mut split: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for &i in &flipped {
        let rule_alone = if is_escape(c.fewbody_stop.outcomes[i]) {
            "rule alone: escape"
        } else {
            "rule alone: not escape"
        };
        let stop_alone = if is_escape(c.closure_live.outcomes[i]) {
            "stop option alone: escape"
        } else {
            "stop option alone: not escape"
        };
        split.entry((rule_alone, stop_alone)).or_default().push(i);
    }
    println!(
        "item 1, the {} cells by which change alone labels them an escape:",
        flipped.len()
    );
    let mut total = 0;
    for ((a, b), v) in &split {
        total += v.len();
        println!("  {a:<24} {b:<30} {:>4}", v.len());
    }
    // How the 370's labels sit in the two middle corners, and whether a middle corner names the same body as after.
    let same_body = |f: &Field| {
        flipped
            .iter()
            .filter(|&&i| {
                is_escape(f.outcomes[i])
                    && f.outcomes[i].detail == c.fewbody_live.outcomes[i].detail
            })
            .count()
    };
    println!(
        "  of them, the same label as after #67: under the rule alone {}, under the stop option alone {}",
        same_body(&c.fewbody_stop),
        same_body(&c.closure_live)
    );
    let mut middle: BTreeMap<(Outcome, Outcome), usize> = BTreeMap::new();
    for &i in &flipped {
        *middle
            .entry((c.fewbody_stop.outcomes[i], c.closure_live.outcomes[i]))
            .or_default() += 1;
    }
    println!("  (fewbody+stop, closure live) outcome pairs over the 370: {middle:?}");
    let same_stop = (0..c.cells.len())
        .filter(|&i| c.fewbody_stop.outcomes[i] == c.fewbody_live.outcomes[i])
        .count();
    println!("  Fewbody with the stop option against Fewbody live: the same outcome on {same_stop} of 4096 cells");
    // Guard cells: the first of the 370 with each label.
    let guard: Vec<usize> = (0..3)
        .filter_map(|d| {
            flipped
                .iter()
                .copied()
                .find(|&i| c.fewbody_live.outcomes[i].detail == d)
        })
        .collect();
    println!("  the first of the 370 per label: {guard:?}");
    let in_pairs = |i: usize| {
        let lower = if i < SIZE[0] * SIZE[1] / 2 {
            i
        } else {
            mirror_index(i)
        };
        before.contains(&lower) || after.contains(&lower)
    };
    let candidates: Vec<Vec<usize>> = (0..3)
        .map(|d| {
            flipped
                .iter()
                .copied()
                .filter(|&i| c.fewbody_live.outcomes[i].detail == d && !in_pairs(i))
                .take(4)
                .collect()
        })
        .collect();
    println!("  of the 370, per label, the first four whose mirror pair agrees: {candidates:?}");
    println!(
        "  the guards' cells {GUARD_CELLS:?} in a disagreeing mirror pair: {:?}",
        GUARD_CELLS.map(in_pairs)
    );

    assert_eq!(total, flipped.len());
    check_anchors(&Anchors {
        replica_differs: differ,
        escapes_before: escapes(&c.closure_stop),
        escapes_after: escapes(&c.fewbody_live),
        flipped: flipped.len(),
        collision_changed,
        escape_changed_body,
        mirror_before: before.len(),
        mirror_after: after.len(),
    });
}

/// The counts the anchors compare.
#[derive(Clone, Copy, Debug)]
struct Anchors {
    replica_differs: usize,
    escapes_before: usize,
    escapes_after: usize,
    flipped: usize,
    collision_changed: usize,
    escape_changed_body: usize,
    mirror_before: usize,
    mirror_after: usize,
}

/// The Mac's numbers (the brief). The mirror counts were first given as 131 and 136, which counted the cells the app's
/// home button covers in the screenshot (image rows 1–2, columns 61–62); recounted on the Mac without them, 128 and 133.
const MAC: Anchors = Anchors {
    replica_differs: 0,
    escapes_before: 74,
    escapes_after: 444,
    flipped: 370,
    collision_changed: 0,
    escape_changed_body: 0,
    mirror_before: 128,
    mirror_after: 133,
};

/// The anchors' criterion (the test's note): each count the Mac's; the escape counts first, the mirror's last.
fn check_anchors(a: &Anchors) {
    assert_eq!(a.replica_differs, 0, "the replica is not the app's field");
    assert_eq!(
        a.escapes_before, MAC.escapes_before,
        "not the Mac's before-field"
    );
    assert_eq!(
        a.escapes_after, MAC.escapes_after,
        "not the Mac's after-field"
    );
    assert_eq!(a.flipped, MAC.flipped, "not the Mac's 370");
    assert_eq!(a.collision_changed, 0, "a collision cell changed");
    assert_eq!(a.escape_changed_body, 0, "an escape changed body");
    assert_eq!(
        a.mirror_before, MAC.mirror_before,
        "not the Mac's mirror disagreements before #67"
    );
    assert_eq!(
        a.mirror_after, MAC.mirror_after,
        "not the Mac's mirror disagreements after #67"
    );
}

negative_control!(
    view_corners_separate_the_rule_from_the_stop_option,
    "the fields before and after #67 swapped",
    expected = "not the Mac's before-field",
    check_anchors(&Anchors {
        escapes_before: MAC.escapes_after,
        escapes_after: MAC.escapes_before,
        ..MAC
    })
);

/// The two-body energy of body `b` about the centre of mass of the other two (pitfalls §2.2's `E_rel`, the total mass).
fn e_rel(s: &State<f64>, b: usize) -> f64 {
    let (i, j) = others(b);
    let mp = s.m[i] + s.m[j];
    let com =
        |v: [[f64; 2]; 3], w: [f64; 3]| [0, 1].map(|x| (w[i] * v[i][x] + w[j] * v[j][x]) / mp);
    let vel: [[f64; 2]; 3] = std::array::from_fn(|k| [s.p[k][0] / s.m[k], s.p[k][1] / s.m[k]]);
    let rc = com(s.r, s.m);
    let vc = com(vel, s.m);
    let d = (s.r[b][0] - rc[0]).hypot(s.r[b][1] - rc[1]);
    let dv = [vel[b][0] - vc[0], vel[b][1] - vc[1]];
    0.5 * (dv[0] * dv[0] + dv[1] * dv[1]) - (mp + s.m[b]) / d
}

/// The continued runs of every cell, at f32 (the field's march) and f64 (the second column).
fn runs_all(c: &[Cell]) -> (Vec<Run>, Vec<Run>) {
    let sim = capture_view();
    let (radius, _, _) = march_values(&sim);
    let (p32, p64) = (params_at::<f32>(1.0), params_at::<f64>(1.0));
    let r32 = parallel(c.len(), |i| {
        Ok(match t0(&c[i].ic, c[i].degenerate, &radius).state {
            STATE_RUNNING => Some(continued::<f32, Fewbody<f32>>(
                &c[i].ic, radius, &p32, 0, T_TRUTH,
            )),
            _ => None,
        })
    })
    .expect("f32 runs");
    let r64 = parallel(c.len(), |i| {
        Ok(match t0(&c[i].ic, c[i].degenerate, &radius).state {
            STATE_RUNNING => Some(continued::<f64, Fewbody<f64>>(
                &widen_ic(&c[i].ic),
                radius,
                &p64,
                0,
                T_TRUTH,
            )),
            _ => None,
        })
    })
    .expect("f64 runs");
    let none = || Run {
        label_h: None,
        reached_h: false,
        state_h: None,
        labels: Vec::new(),
        truth: Truth::Censored,
        b_stable: false,
        ended: Ended::Collision(0),
        t_end: 0.0,
        last_record: None,
        path: Vec::new(),
        collision_detail: None,
    };
    (
        r32.into_iter().map(|r| r.unwrap_or_else(none)).collect(),
        r64.into_iter().map(|r| r.unwrap_or_else(none)).collect(),
    )
}

/// Whether the continued run's horizon reads as the field's cell: an escape label the field's escape, the same `detail`;
/// bounded the field's bounded, reached with no label; a collision a run that collided by the horizon.
fn run_is_cell(run: &Run, o: Outcome) -> bool {
    let k_h = (HORIZON / f64::from(heggie_params().delta_sync)).floor() as u32;
    match o.state {
        STATE_ESCAPE => run.label_h == Some(o.detail),
        STATE_BOUNDED => run.reached_h && run.label_h.is_none(),
        STATE_COLLISION => matches!(run.ended, Ended::Collision(k) if k <= k_h),
        _ => true,
    }
}

/// Item 2: every escape cell of the after-field scored against the continued-run truth, the body counted.
///
/// **Pass criterion, written before the run:** the field's escape label at the horizon is the continued f32 run's on
/// every cell (the continued run is the field's march), and no escape cell is wrong: no wrong class, no wrong body, no
/// escape on a Mardling–Aarseth stable truth. Unresolved truths (no escape and not certified stable by `T_truth`) are
/// reported apart and never counted wrong; censored continued runs (a later collision, a failure) are unjudged.
/// The f64 run of the same IC is a second column, reported, not a criterion.
#[test]
#[ignore = "every cell continued to t = 1500 at f32 and f64: minutes in release"]
fn view_escape_labels_scored_against_continued_runs() {
    let c = corners();
    let (r32, r64) = runs_all(&c.cells);
    let field = &c.fewbody_live;
    let not_field: Vec<usize> = (0..field.outcomes.len())
        .filter(|&i| !run_is_cell(&r32[i], field.outcomes[i]))
        .collect();
    println!(
        "the continued f32 run's horizon against the field: {} of 4096 cells differ {:?}",
        not_field.len(),
        not_field
    );
    let escapes: Vec<usize> = (0..field.outcomes.len())
        .filter(|&i| is_escape(field.outcomes[i]))
        .collect();
    let same64 = escapes
        .iter()
        .filter(|&&i| r64[i].label_h == Some(field.outcomes[i].detail))
        .count();
    println!(
        "the f64 run's label at t = 50 equals the field's on {same64} of {} escape cells",
        escapes.len()
    );

    let mut tally: BTreeMap<Verdict, usize> = BTreeMap::new();
    let mut tally64: BTreeMap<Verdict, usize> = BTreeMap::new();
    let mut by_label: BTreeMap<(u32, Verdict), usize> = BTreeMap::new();
    for &i in &escapes {
        let d = field.outcomes[i].detail;
        let v = verdict(d, &r32[i]);
        *tally.entry(v).or_default() += 1;
        *tally64.entry(verdict(d, &r64[i])).or_default() += 1;
        *by_label.entry((d, v)).or_default() += 1;
    }
    println!(
        "item 2, the {} escape cells against the f32 continued run: {tally:?}",
        escapes.len()
    );
    println!("  by label (detail, verdict): {by_label:?}");
    println!("  against the f64 continued run of the same IC (second column): {tally64:?}");
    let mut truths: BTreeMap<String, usize> = BTreeMap::new();
    for &i in &escapes {
        *truths.entry(truth_name(&r32[i])).or_default() += 1;
    }
    println!("  the truths of the escape cells: {truths:?}");
    let ends: BTreeMap<String, usize> = escapes.iter().fold(BTreeMap::new(), |mut m, &i| {
        *m.entry(
            format!("{:?}", r32[i].ended)
                .split('(')
                .next()
                .unwrap()
                .to_owned(),
        )
        .or_default() += 1;
        m
    });
    println!("  how their continued runs ended: {ends:?}");

    println!("  every escape cell not right (row from the top, column; label; truth; how the run ended; f64 truth):");
    for &i in &escapes {
        let d = field.outcomes[i].detail;
        let v = verdict(d, &r32[i]);
        if v != Verdict::Right {
            let (row, col) = row_col(i);
            let rec = r32[i].last_record.map_or("-".to_owned(), |(e, b)| {
                format!(
                    "ratio {:.3e} body {} least sep {:.3e} beta {:.2},{:.2},{:.2}",
                    e.ratio,
                    e.body,
                    e.d.iter().copied().fold(f64::INFINITY, f64::min),
                    b[0],
                    b[1],
                    b[2]
                )
            });
            println!(
                "    {v:?}: row {row:>2} col {col:>2} (cell {i}): label {}; truth {}; {:?} at t = {:.1}; {rec}; \
                 f64 truth {}; label changes {:?}",
                label_name(d),
                truth_name(&r32[i]),
                r32[i].ended,
                r32[i].t_end,
                truth_name(&r64[i]),
                r32[i].labels
            );
        }
    }
    println!("  the f64 column's escape cells not right and not censored:");
    for &i in &escapes {
        let d = field.outcomes[i].detail;
        let v = verdict(d, &r64[i]);
        if v != Verdict::Right && v != Verdict::Censored {
            let (row, col) = row_col(i);
            println!(
                "    {v:?}: row {row:>2} col {col:>2} (cell {i}): field label {}; f64 label at 50 {:?}; f64 truth {}; f32 truth {}; \
                 f64 label changes {:?}",
                label_name(d),
                r64[i].label_h,
                truth_name(&r64[i]),
                truth_name(&r32[i]),
                r64[i].labels
            );
        }
    }
    // The label's history past the horizon: how many escape cells change label after t = 50.
    let changed_after = escapes
        .iter()
        .filter(|&&i| {
            r32[i]
                .labels
                .iter()
                .any(|(t, l)| *t > HORIZON && *l != r32[i].label_h)
        })
        .count();
    println!("  escape cells whose live label changes after t = 50 in the continued run: {changed_after}");
    // Every label change after the first label, up to T_eval and to the end, by kind: back to no escape, or to another
    // escape label (a change of body or class).
    let flips = |runs: &[Run], until: f64| {
        let (mut to_none, mut to_other, mut cells) = (0, 0, 0);
        for &i in &escapes {
            let mut seen: Option<u32> = None;
            let mut any = false;
            for &(t, l) in &runs[i].labels {
                if t > until {
                    break;
                }
                match (seen, l) {
                    (Some(_), None) => {
                        to_none += 1;
                        any = true;
                    }
                    (Some(a), Some(b)) if a != b => {
                        to_other += 1;
                        any = true;
                    }
                    _ => {}
                }
                if l.is_some() {
                    seen = l;
                }
            }
            cells += usize::from(any);
        }
        (to_none, to_other, cells)
    };
    println!(
        "  label changes after the first label, (back to no escape, to another escape label, cells): by T_eval {:?}, \
         by the end of the continued run {:?}",
        flips(&r32, T_EVAL),
        flips(&r32, T_TRUTH)
    );
    println!(
        "  the same on the f64 runs of the same ICs: by T_eval {:?}, by the end {:?}",
        flips(&r64, T_EVAL),
        flips(&r64, T_TRUTH)
    );
    for i in GUARD_CELLS {
        println!(
            "  guard cell {i}: field {:?}, verdict {:?}, truth {}",
            field.outcomes[i],
            verdict(field.outcomes[i].detail, &r32[i]),
            truth_name(&r32[i])
        );
    }

    assert!(
        not_field.is_empty(),
        "the continued run is not the field's march"
    );
    judge_labels(&tally);
}

/// Item 2's criterion on a tally of verdicts: no wrong class, no wrong body, no escape on a B-stable truth.
fn judge_labels(tally: &BTreeMap<Verdict, usize>) {
    let wrong = [
        Verdict::WrongClass,
        Verdict::WrongBody,
        Verdict::WrongOnBStable,
    ]
    .iter()
    .map(|v| tally.get(v).copied().unwrap_or(0))
    .sum::<usize>();
    assert_eq!(
        wrong, 0,
        "escape labels at the horizon are wrong against the continued run: {tally:?}"
    );
}

/// The guards' cells marched as the field marches them, their escape labels given another body, scored against their
/// continued runs: the control of item 2's criterion, on the criterion's own scoring.
#[cfg(feature = "controls")]
fn relabelled_guards_scored() {
    let sim = capture_view();
    let all = cells(&sim, SIZE);
    let (radius, k, params) = march_values(&sim);
    let mut tally: BTreeMap<Verdict, usize> = BTreeMap::new();
    for i in GUARD_CELLS {
        let o = outcome_rule::<DefaultRule<f32>>(&all[i], radius, k, &params, false);
        let run =
            continued::<f32, Fewbody<f32>>(&all[i].ic, radius, &params_at::<f32>(1.0), 0, T_TRUTH);
        assert!(
            is_escape(o) && o.detail < 3,
            "cell {i}: a guard cell is a single escape"
        );
        *tally.entry(verdict((o.detail + 1) % 3, &run)).or_default() += 1;
    }
    judge_labels(&tally);
}

negative_control!(
    view_escape_labels_scored_against_continued_runs,
    "the guards' escape labels given another body",
    expected = "escape labels at the horizon are wrong",
    relabelled_guards_scored()
);

/// Item 3: the black slot, image rows 31–32 (from the top), columns 14–22, which stays bounded / nothing yet inside the
/// escape region; with rows 30 and 33 beside it for comparison.
///
/// **Pass criterion, written before the run**, per cell of the slot: bounded at the horizon is **right** where the
/// continued run's truth is no escape, or where it is an escape of body `b` whose two-body energy about the other two
/// (`E_rel`, pitfalls §2.2) is still negative at `t = 50`, so no rule could have named the escape by then; it is
/// **late** where the truth is an escape of `b` and `b` is already unbound (`E_rel > 0`) at `t = 50`; a censored truth
/// is unjudged. A late cell is a missed label, reported; the test fails on any late cell in the slot.
///
/// **Amended after the run, and said so.** As written, the criterion read `E_rel` of the truth's escaper whatever its
/// place at `t = 50`, and called 4 of the slot's cells late. In all 4 the escaper is at `t = 50` a member of the inner
/// binary, whose `E_rel` about the other two is positive from its orbital speed alone (pitfalls §1.5's transient), and
/// the body out is the third body, on a bound excursion (`E_rel < 0`), which falls back before the
/// escape. That contradicts the criterion's own intent ("no rule could have named the escape by then"). Amended: late
/// only where the truth's escaper is the body out at `t = 50` (the record's most separated body) and unbound there. Both
/// readings are printed; the test asserts the amended one.
#[test]
#[ignore = "eighteen cells and their neighbours continued to t = 1500"]
fn view_black_slot_continued() {
    check_slot(
        &[30, 31, 32, 33],
        12..=24,
        |row, col| (31..=32).contains(&row) && (14..=22).contains(&col),
        None,
    );
}

negative_control!(
    view_black_slot_continued,
    "an escape cell beside the slot, its label taken away, judged as a slot cell",
    expected = "a missed escape label",
    check_slot(
        &[30],
        17..=17,
        |_, _| true,
        Some(Outcome {
            state: STATE_BOUNDED,
            detail: 0
        })
    )
);

/// Item 3 on the image's `rows` over `cols`: each cell marched as the field marches it (or, under `force`, given that
/// outcome), continued, and judged where `slot` says it is in the slot and the field does not label it an escape.
fn check_slot(
    rows: &[usize],
    cols: std::ops::RangeInclusive<usize>,
    slot: fn(usize, usize) -> bool,
    force: Option<Outcome>,
) {
    let sim = capture_view();
    let all = cells(&sim, SIZE);
    let (radius, k, params) = march_values(&sim);
    let p32 = params_at::<f32>(1.0);
    let at = |row: usize, col: usize| (SIZE[1] - 1 - row) * SIZE[0] + col;
    let mut late = 0;
    let mut late_amended = 0;
    for &row in rows {
        for col in cols.clone() {
            let i = at(row, col);
            let o = force.unwrap_or_else(|| {
                outcome_rule::<DefaultRule<f32>>(&all[i], radius, k, &params, false)
            });
            let run = continued::<f32, Fewbody<f32>>(&all[i].ic, radius, &p32, 0, T_TRUTH);
            let first = run.labels.iter().find(|(_, l)| l.is_some()).copied();
            let s = run.state_h;
            let (ratio, e) = s.map_or((f64::NAN, [f64::NAN; 3]), |s| {
                let rec = rules::record(&s.m, &s.r, HORIZON);
                (rec.ratio, std::array::from_fn(|b| e_rel(&s, b)))
            });
            let in_slot = slot(row, col) && !is_escape(o);
            // The criterion as written before the run (the test's note).
            let written = if !in_slot {
                "-"
            } else {
                match run.truth {
                    Truth::NoEscape | Truth::Bound => "right (no escape)",
                    Truth::Escape(b) if e[b as usize] <= 0.0 => "right (escaper still bound at 50)",
                    Truth::Escape(_) => {
                        late += 1;
                        "LATE (escaper unbound at 50)"
                    }
                    Truth::Dispersal if e.iter().all(|&x| x <= 0.0) => "right (all bound at 50)",
                    Truth::Dispersal => {
                        late += 1;
                        "LATE (a body unbound at 50)"
                    }
                    Truth::Censored => "unjudged",
                }
            };
            // The criterion amended after the run (the test's note): the escaper must be the body out at t = 50.
            let out = s.map(|s| rules::record(&s.m, &s.r, HORIZON).body);
            let amended = if !in_slot {
                "-"
            } else {
                match run.truth {
                    Truth::NoEscape | Truth::Bound => "right (no escape)",
                    Truth::Escape(b) if out == Some(b as usize) && e[b as usize] > 0.0 => {
                        late_amended += 1;
                        "LATE (the escaper is out and unbound at 50)"
                    }
                    Truth::Escape(_) => "right (the escaper is not out, or not unbound, at 50)",
                    Truth::Dispersal if out.is_some_and(|b| e[b] > 0.0) => {
                        late_amended += 1;
                        "LATE (the body out is unbound at 50)"
                    }
                    Truth::Dispersal => "right (the body out is bound at 50)",
                    Truth::Censored => "unjudged",
                }
            };
            let judged =
                format!("as written: {written}; amended: {amended}; the body out at 50: {out:?}");
            println!(
                "row {row} col {col:>2}: field {:?} ({}); first label {first:?}; truth {}; {:?} t_end {:.1}; at t = 50 \
                 ratio {ratio:.2}, E_rel by body {:.3},{:.3},{:.3}; {judged}",
                o,
                if is_escape(o) { "escape" } else if is_nothing_yet(o) { "nothing yet" } else { "collision" },
                truth_name(&run),
                run.ended,
                run.t_end,
                e[0],
                e[1],
                e[2]
            );
        }
    }
    println!("the slot: late as written {late}, late as amended {late_amended}");
    assert_eq!(
        late_amended, 0,
        "bounded at the horizon in the slot is a missed escape label"
    );
}

/// How one mirror pair's trajectories compare at one width and step: when they part, and their outcomes.
#[derive(Clone, Copy, Debug)]
struct Parting {
    /// The first sync boundary's time where the gap exceeds [`PART`], `None` where it never does before the shorter
    /// run ends (or the horizon).
    t_part: Option<f64>,
    /// The gap at the IC.
    gap_0: f64,
    /// The outcomes at the horizon (Fewbody, live), the mirror's relabelled, and whether they agree.
    agree: bool,
}

/// The gap, in canonical units of position and momentum, past which two mirror trajectories have parted: ten
/// thousand times f32's rounding of the unit, far above both widths' rounding and far below the system's scale (1).
const PART: f64 = 1e-3;

/// The outcome of a run at the horizon under Fewbody live: the collision, else the label, else bounded.
fn run_outcome(run: &Run) -> Outcome {
    let k_h = (HORIZON / f64::from(heggie_params().delta_sync)).floor() as u32;
    match (run.ended, run.collision_detail) {
        (Ended::Collision(k), Some(d)) if k <= k_h => Outcome {
            state: STATE_COLLISION,
            detail: d,
        },
        _ => match run.label_h {
            Some(d) => Outcome {
                state: STATE_ESCAPE,
                detail: d,
            },
            None => Outcome {
                state: STATE_BOUNDED,
                detail: 0,
            },
        },
    }
}

/// The pair `(i, mirror_index(i))` marched at `R` with `η` scaled by `eta_scale`, from the IC at that width.
fn parting<R: Width>(a: &Ic<R>, b: &Ic<R>, eta_scale: f64, map: Mirror) -> Parting {
    let sim = capture_view();
    let (radius, k_h, _) = march_values(&sim);
    let p = params_at::<R>(eta_scale);
    let ra = continued::<R, Fewbody<R>>(a, radius, &p, k_h, HORIZON);
    let rb = continued::<R, Fewbody<R>>(b, radius, &p, k_h, HORIZON);
    let ia = State {
        m: a.m.map(Into::into),
        r: a.r.map(|v| v.map(Into::into)),
        p: a.p.map(|v| v.map(Into::into)),
    };
    let ib = State {
        m: b.m.map(Into::into),
        r: b.r.map(|v| v.map(Into::into)),
        p: b.p.map(|v| v.map(Into::into)),
    };
    let delta = f64::from(heggie_params().delta_sync);
    let n = ra.path.len().min(rb.path.len());
    let t_part = (0..n)
        .find(|&k| mirror_gap(&ra.path[k], &rb.path[k], map) > PART)
        .map(|k| (k + 1) as f64 * delta);
    Parting {
        t_part,
        gap_0: mirror_gap(&ia, &ib, map),
        agree: run_outcome(&ra) == mirror_outcome(run_outcome(&rb)),
    }
}

/// What a disagreeing mirror pair is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PairKind {
    /// The field's march never parts from its mirror, yet the labels differ: the labelling is not mirror-symmetric.
    LabelDefect,
    /// The trajectories part, and later at f64 than at f32 (or not at all): rounding amplified by chaos.
    Chaos,
    /// The trajectories part, and no later at f64 than at f32: the asymmetry does not shrink with the rounding.
    Persists,
}

/// The margin by which the parting must move later at f64 to count as moved, time units.
const MOVED: f64 = 1.0;

/// Item 4: every pair the mirror disagrees on, before #67 (Closure, stop) or after (Fewbody, live).
///
/// **Pass criterion, written before the run**, per pair, from the field's own march (f32, the field's `η`), then one
/// knob at a time: the width (f64 at the same `η`, the IC decoded at f64), then the step (f64, `η/4`); the step refined
/// at f32 (`η/4`) is reported beside them, added after the first run because the brief asks about the step:
/// - **label defect**: at f32 the gap between the pair's states (positions and momenta, one mirrored) stays at or below
///   [`PART`] at every sync boundary to the shorter run's end, yet the field's labels disagree;
/// - **chaos amplifying rounding**: the gap exceeds [`PART`] at f32 by `t_f32`, and at f64 it does so later than
///   `t_f32 + MOVED`, or never by the horizon;
/// - **persists**: the gap exceeds [`PART`] at f32 by `t_f32`, and at f64 no later than `t_f32 + MOVED`.
///
/// The test fails on any label defect or persisting pair. Whether the outcomes agree at f64 and at `η/4` is reported
/// beside it, not judged: a chaotic pair can still part by the horizon at f64.
#[test]
#[ignore = "eight marches per disagreeing pair: a minute in release"]
fn view_mirror_disagreements_under_refinement() {
    let c = corners();
    let before = mirror_disagreements(&c.closure_stop);
    let after = mirror_disagreements(&c.fewbody_live);
    let mut pairs: Vec<usize> = before.iter().chain(after.iter()).copied().collect();
    pairs.sort_unstable();
    pairs.dedup();
    println!(
        "pairs: {} before, {} after, {} in both, {} in either",
        before.len(),
        after.len(),
        before.iter().filter(|i| after.contains(i)).count(),
        pairs.len()
    );
    let results = parallel(pairs.len(), |k| {
        let i = pairs[k];
        let j = mirror_index(i);
        let (a, b) = (&c.cells[i], &c.cells[j]);
        Ok((
            parting::<f32>(&a.ic, &b.ic, 1.0, mirror_state),
            parting::<f64>(&a.ic64, &b.ic64, 1.0, mirror_state),
            parting::<f64>(&a.ic64, &b.ic64, 0.25, mirror_state),
            parting::<f32>(&a.ic, &b.ic, 0.25, mirror_state),
        ))
    })
    .expect("the pairs");
    let mut kinds: BTreeMap<PairKind, usize> = BTreeMap::new();
    let mut agree64 = (0, 0);
    let mut fine32 = (0, 0, 0);
    let mut map = vec![['.'; 64]; 32];
    for (k, &i) in pairs.iter().enumerate() {
        let (f32r, f64r, fine, f32_fine) = results[k];
        fine32.0 += usize::from(f32_fine.agree);
        fine32.1 += usize::from(f32_fine.t_part.is_none());
        fine32.2 += usize::from(
            matches!((f32_fine.t_part, f32r.t_part), (Some(a), Some(b)) if a > b + MOVED),
        );
        let kind = kind_of(&f32r, &f64r);
        *kinds.entry(kind).or_default() += 1;
        agree64.0 += usize::from(f64r.agree);
        agree64.1 += usize::from(fine.agree);
        let (row, col) = row_col(i);
        let j = mirror_index(i);
        map[row - 32][col] = match kind {
            PairKind::LabelDefect => 'L',
            PairKind::Chaos => 'c',
            PairKind::Persists => 'P',
        };
        println!(
            "row {row:>2}/{:>2} col {col:>2}: before {:?} / {:?}, after {:?} / {:?} (mirror relabelled); IC gap f32 \
             {:.1e} f64 {:.1e}; parts f32 {:?} f64 {:?} f64 η/4 {:?} f32 η/4 {:?}; agree at f32 {} f64 {} f64 η/4 {} f32 \
             η/4 {}; {kind:?}",
            63 - row,
            class(c.closure_stop.outcomes[i]),
            class(mirror_outcome(c.closure_stop.outcomes[j])),
            class(c.fewbody_live.outcomes[i]),
            class(mirror_outcome(c.fewbody_live.outcomes[j])),
            f32r.gap_0,
            f64r.gap_0,
            f32r.t_part,
            f64r.t_part,
            fine.t_part,
            f32_fine.t_part,
            f32r.agree,
            f64r.agree,
            fine.agree,
            f32_fine.agree
        );
    }
    println!("item 4, by kind: {kinds:?}");
    println!(
        "  outcomes agree under the mirror at f64: {} of {}; at f64 η/4: {} of {}",
        agree64.0,
        pairs.len(),
        agree64.1,
        pairs.len()
    );
    println!(
        "  with the step refined at f32 (η/4): outcomes agree under the mirror on {} of {}; never part by t = 50 on {}; \
         part more than {MOVED} later than at η on {}",
        fine32.0,
        pairs.len(),
        fine32.1,
        fine32.2
    );
    let shifts: Vec<f64> = results
        .iter()
        .filter_map(|(a, b, _, _)| Some(b.t_part.unwrap_or(HORIZON) - a.t_part?))
        .collect();
    let mut s = shifts.clone();
    s.sort_by(f64::total_cmp);
    if !s.is_empty() {
        println!(
            "  parting moved later at f64 by (min, p10, p50, p90, max): {:.2} {:.2} {:.2} {:.2} {:.2} time units",
            s[0],
            s[s.len() / 10],
            s[s.len() / 2],
            s[s.len() * 9 / 10],
            s[s.len() - 1]
        );
    }
    println!("  the lower half's map (row 32 at the top, the image's rows 32–63; c chaos, P persists, L label defect):");
    for line in &map {
        println!("    {}", line.iter().collect::<String>());
    }
    judge_pairs(&kinds);
}

/// The kind of a disagreeing pair from its f32 and f64 partings (the test's note).
fn kind_of(f32r: &Parting, f64r: &Parting) -> PairKind {
    match f32r.t_part {
        None => PairKind::LabelDefect,
        Some(t) => match f64r.t_part {
            Some(t64) if t64 <= t + MOVED => PairKind::Persists,
            _ => PairKind::Chaos,
        },
    }
}

/// Item 4's criterion: no label defect, no persisting pair.
fn judge_pairs(kinds: &BTreeMap<PairKind, usize>) {
    assert_eq!(
        kinds.get(&PairKind::LabelDefect),
        None,
        "a mirror pair's labels differ on mirror trajectories"
    );
    assert_eq!(
        kinds.get(&PairKind::Persists),
        None,
        "a mirror pair's asymmetry does not shrink with the rounding"
    );
}

/// The pair at image row 63, column 33 (sample 33, a disagreeing pair of both fields), judged under the mirror with
/// the bodies left in place: an asymmetry no refinement removes.
#[cfg(feature = "controls")]
fn unrelabelled_pair_judged() {
    let sim = capture_view();
    let all = cells(&sim, SIZE);
    let (a, b) = (&all[33], &all[mirror_index(33)]);
    let unrelabelled: Mirror = |s| State {
        m: s.m,
        r: s.r.map(|v| [-v[0], v[1]]),
        p: s.p.map(|v| [-v[0], v[1]]),
    };
    let kind = kind_of(
        &parting::<f32>(&a.ic, &b.ic, 1.0, unrelabelled),
        &parting::<f64>(&a.ic64, &b.ic64, 1.0, unrelabelled),
    );
    judge_pairs(&BTreeMap::from([(kind, 1)]));
}

negative_control!(
    view_mirror_disagreements_under_refinement,
    "a disagreeing pair judged under the mirror without the relabelling",
    expected = "a mirror pair's asymmetry does not shrink with the rounding",
    unrelabelled_pair_judged()
);

// ── Why the f32 live label drops back ───────────────────────────────────────────────────────────────────────────
//
// On this view's f32 field, 36 of the 444 escape cells drop back from escape to nothing yet 70 times by T_eval; the
// f64 runs never do (#73's item 6). These two measurements find which term of Fewbody's test flips at each drop, and
// whether the flip is the test's arithmetic, the state it is handed, or the march.

/// Every term of `kernel::detect::escape_rule::fewbody_test`, evaluated at one width and widened to f64 to print.
#[derive(Clone, Copy, Debug)]
struct Terms {
    /// The Kepler `a` of the pairs `(0, 1)`, `(0, 2)`, `(1, 2)`.
    a: [f64; 3],
    /// The third body of the least-`a` bound pair, 3 where no pair is bound.
    k: usize,
    /// The outer body's `K`, `U` and `v_r` against the pair, and the pair's tidal ratio (all 0 where `k = 3`).
    out_k: f64,
    out_u: f64,
    out_vr: f64,
    tidal: f64,
    /// The pairs' `v_r`, `(0, 1)`, `(0, 2)`, `(1, 2)` (the triple-ejection leg).
    vr: [f64; 3],
    e_0: f64,
    /// The verdict: the escaper's `detail`, 3 a triple ejection, `None` not fired.
    fired: Option<u32>,
}

/// `fewbody_test`'s terms at the width `W`, transcribed step for step ([`check_transcription`] holds it to the
/// kernel's verdict).
fn fewbody_terms<W: EscapeReal + Into<f64>>(
    m: &[W; 3],
    r: &[[W; 2]; 3],
    p: &[[W; 2]; 3],
    e_0: W,
) -> Terms {
    let ph = Playhead::from_momenta(*m, *r, *p, e_0, W::one(), W::one());
    let a_of = |i: usize, j: usize| {
        let (ri, rj, vi, vj) = (r[i], r[j], ph.v[i], ph.v[j]);
        kepler(
            [rj[0] - ri[0], rj[1] - ri[1]],
            [vj[0] - vi[0], vj[1] - vi[1]],
            m[i] + m[j],
        )
        .a
    };
    let zero = W::zero();
    let (a01, a02, a12) = (a_of(0, 1), a_of(0, 2), a_of(1, 2));
    let mut k = 3usize;
    let mut least = a01;
    if a01 > zero {
        k = 2;
    }
    if a02 > zero && (k == 3 || a02 < least) {
        k = 1;
        least = a02;
    }
    if a12 > zero && (k == 3 || a12 < least) {
        k = 0;
    }
    let vr = [
        pair_kup(&ph, 0, 1).v_r,
        pair_kup(&ph, 0, 2).v_r,
        pair_kup(&ph, 1, 2).v_r,
    ];
    let delta = W::at_width(CHAZY_DELTA as f32, CHAZY_DELTA);
    let (out, tidal, fired) = if k == 3 {
        let receding = vr.iter().all(|&v| v > zero);
        (None, 0.0, (receding && e_0 > zero).then_some(3u32))
    } else {
        let (i, j) = others(k);
        let out = out_kup(&ph, k, i, j);
        let tidal = tidal_ratio(&ph, i, j, k);
        let fired = (out.k >= out.u && out.v_r > zero && tidal < delta).then_some(k as u32);
        (Some(out), tidal.into(), fired)
    };
    Terms {
        a: [a01.into(), a02.into(), a12.into()],
        k,
        out_k: out.map_or(0.0, |o| o.k.into()),
        out_u: out.map_or(0.0, |o| o.u.into()),
        out_vr: out.map_or(0.0, |o| o.v_r.into()),
        tidal,
        vr: vr.map(Into::into),
        e_0: e_0.into(),
        fired,
    }
}

impl Terms {
    /// One line of every term.
    fn line(&self) -> String {
        format!(
            "a {:+.6e} {:+.6e} {:+.6e} k {} | K {:.9e} U {:.9e} v_r {:+.3e} tidal/δ {:.6e} | pair v_r {:+.2e} {:+.2e} \
             {:+.2e} E0 {:+.4e} | {:?}",
            self.a[0],
            self.a[1],
            self.a[2],
            self.k,
            self.out_k,
            self.out_u,
            self.out_vr,
            self.tidal / CHAZY_DELTA,
            self.vr[0],
            self.vr[1],
            self.vr[2],
            self.e_0,
            self.fired
        )
    }
}

/// Which of the test's legs differ from `a` to `b`. Where the bound pair changes, its third body is the flip (the
/// outer legs of `k = 3` are not computed, so they are not compared); the triple leg is compared where either has no
/// bound pair.
fn flipped(a: &Terms, b: &Terms) -> String {
    let mut s = Vec::new();
    if a.k != b.k {
        s.push(format!("bound pair's third body {}→{}", a.k, b.k));
    } else if a.k < 3 {
        let legs = |t: &Terms| (t.out_k >= t.out_u, t.out_vr > 0.0, t.tidal < CHAZY_DELTA);
        let (x, y) = (legs(a), legs(b));
        if x.0 != y.0 {
            s.push(format!("K≥U {}→{}", x.0, y.0));
        }
        if x.1 != y.1 {
            s.push(format!("v_r>0 {}→{}", x.1, y.1));
        }
        if x.2 != y.2 {
            s.push(format!("tidal<δ {}→{}", x.2, y.2));
        }
    }
    if b.k == 3 {
        let recede = b.vr.iter().all(|&v| v > 0.0);
        s.push(format!("then all recede {recede}, E0>0 {}", b.e_0 > 0.0));
    }
    s.join(", ")
}

/// The transcription's verdict against the kernel's on every state `(s, e_0)`.
///
/// **Pass criterion:** they agree on every state, or the terms printed are not the test's.
fn check_transcription(terms: fn(&State<f32>, f32) -> Option<u32>, states: &[(State<f32>, f32)]) {
    let differ = states
        .iter()
        .filter(|(s, e_0)| {
            let v = fewbody_test(&s.m, &s.r, &s.p, *e_0);
            terms(s, *e_0) != v.fired.then_some(v.detail)
        })
        .count();
    assert_eq!(
        differ,
        0,
        "the transcription is not the kernel's test on {differ} of {} states",
        states.len()
    );
}

/// The transcription's verdict.
fn transcribed(s: &State<f32>, e_0: f32) -> Option<u32> {
    fewbody_terms(&s.m, &s.r, &s.p, e_0).fired
}

/// A broken transcription for the control: the tidal leg left out.
#[cfg(feature = "controls")]
fn transcribed_without_tidal(s: &State<f32>, e_0: f32) -> Option<u32> {
    let t = fewbody_terms(&s.m, &s.r, &s.p, e_0);
    (t.k < 3 && t.out_k >= t.out_u && t.out_vr > 0.0)
        .then_some(t.k as u32)
        .or(t.fired)
}

/// A hierarchical triple, equal masses: the pair `(1, 2)` bound at separation 1, body 0 at 7.5 from the pair receding
/// unbound, so the pair's tidal ratio, about 2e-3, is far above `δ` and the tidal leg alone decides "not fired".
#[cfg(feature = "controls")]
fn tidal_decides() -> (State<f32>, f32) {
    let m = [1.0 / 3.0; 3];
    let w = 0.4f32;
    let s = State {
        m,
        r: [[5.0, 0.0], [-2.5, -0.5], [-2.5, 0.5]],
        p: [
            [m[0] * 2.0, 0.0],
            [m[1] * (-1.0 + w), 0.0],
            [m[2] * (-1.0 - w), 0.0],
        ],
    };
    (s, -0.1)
}

/// One window mark of a march: the time, the Cartesian image the rule is handed, the march's own relative positions
/// `qᵢ = rⱼ − rₖ` (`HgSystem::phys`) and the rule's label there.
struct RelMark<R> {
    t: f64,
    image: State<R>,
    q: [[R; 2]; 3],
    label: Option<u32>,
}

/// A march of the view to `T_eval` read at every window mark, and, where a collision ended it, the time, the march's
/// least `|qᵢ|` and the image's least pair separation on the collision's step.
struct RelRun<R> {
    e_0: R,
    marks: Vec<RelMark<R>>,
    collision: Option<(f64, f64, f64)>,
}

/// The field backend's march of `ic` at the width `R` under Fewbody live (the continued run's cadence, every hook
/// forwarded), to `T_eval`, cut at every sync boundary (`march_on` from where it stands is the uncut march step for
/// step; `cut_differs` measures it) so the regularised state is read at each mark beside the image the rule is handed.
fn marks_rel<R: Width>(ic: &Ic<R>, radius: CollisionRadius, params: &HgParams<R>) -> RelRun<R> {
    let delta: f64 = params.delta_sync.into();
    let inner = WrapperCadence::<R, Fewbody<R>>::with_rule(&ic.m, &ic.r, &ic.p, radius);
    let e_0 = inner.monitor_state().e_0();
    let mut h = Continued {
        inner,
        k_h: (HORIZON / delta).floor() as u32,
        at_h: None,
        labels: Vec::new(),
        series: Vec::new(),
        next_record: rules::SERIES_EVERY,
        asymptotic: false,
        last: None,
        k_path: 0,
        path: Vec::new(),
        t_marks: T_EVAL,
        marks: Vec::new(),
    };
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let k_eval = (T_EVAL / delta).floor() as u32;
    let mut adv = march::<Rk4, R, _, _>(&state, 1, params, &mut h, &mut ExplicitTest);
    let mut marks = Vec::new();
    loop {
        if h.marks.len() > marks.len() {
            let (t, image, label) = h.marks[h.marks.len() - 1];
            let (q, _) = adv.system.phys(&adv.q, &adv.p);
            marks.push(RelMark { t, image, q, label });
        }
        if adv.boundary >= k_eval || h.done() || adv.count.end != End::Marching {
            break;
        }
        let from = HgStanding {
            q: adv.q,
            p: adv.p,
            boundary: adv.boundary,
            count: adv.count,
        };
        adv = march_on::<Rk4, R, _, _>(
            &adv.system,
            &adv.state,
            from,
            adv.boundary + 1,
            params,
            &mut h,
            &mut ExplicitTest,
        );
    }
    let collision = h.inner.events().collision.fired.then(|| {
        let (q, _) = adv.system.phys(&adv.q, &adv.p);
        let q = q.map(|v| v.map(Into::into));
        let s = widen_state(&adv.state);
        let least_q = q.iter().map(|v| norm(*v)).fold(f64::INFINITY, f64::min);
        let least_r = [(1, 2), (2, 0), (0, 1)]
            .iter()
            .map(|&(i, j)| norm([s.r[j][0] - s.r[i][0], s.r[j][1] - s.r[i][1]]))
            .fold(f64::INFINITY, f64::min);
        (adv.t.into(), least_q, least_r)
    });
    RelRun {
        e_0,
        marks,
        collision,
    }
}

/// `|v|`.
fn norm(v: [f64; 2]) -> f64 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

/// The closure `q₀ + q₁ + q₂` of the march's relative positions: zero in exact arithmetic (`qᵢ = rⱼ − rₖ`), but Heggie
/// marches all three, so the march's rounding can open it.
fn closure_gap(q: &[[f64; 2]; 3]) -> [f64; 2] {
    [q[0][0] + q[1][0] + q[2][0], q[0][1] + q[1][1] + q[2][1]]
}

/// The closure gap `|q₀ + q₁ + q₂|` at a mark.
fn gap_of<R: Width>(mk: &RelMark<R>) -> f64 {
    norm(closure_gap(&mk.q.map(|v| v.map(Into::into))))
}

/// The f32 march of `ic` under Fewbody live to `T_eval`, uncut, with its time, state and label at every window mark:
/// what [`marks_rel`]'s cut march is held to.
fn marks_f32(
    ic: &Ic<f32>,
    radius: CollisionRadius,
    params: &HgParams<f32>,
) -> (f32, Vec<MarkAt<f32>>) {
    let delta: f64 = params.delta_sync.into();
    let inner = WrapperCadence::<f32, Fewbody<f32>>::with_rule(&ic.m, &ic.r, &ic.p, radius);
    let e_0 = inner.monitor_state().e_0();
    let mut h = Continued {
        inner,
        k_h: (HORIZON / delta).floor() as u32,
        at_h: None,
        labels: Vec::new(),
        series: Vec::new(),
        next_record: rules::SERIES_EVERY,
        asymptotic: false,
        last: None,
        k_path: 0,
        path: Vec::new(),
        t_marks: T_EVAL,
        marks: Vec::new(),
    };
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let k_eval = (T_EVAL / delta).floor() as u32;
    march::<Rk4, f32, _, _>(&state, k_eval, params, &mut h, &mut ExplicitTest);
    (e_0, h.marks)
}

/// The pair the march holds closest: the `i` of the least `|qᵢ|`, the pair of the other two bodies.
fn closest<R: Width>(q: &[[R; 2]; 3]) -> usize {
    let n = q.map(|v| {
        let v: [f64; 2] = v.map(Into::into);
        norm(v)
    });
    if n[0] <= n[1] && n[0] <= n[2] {
        0
    } else if n[1] <= n[2] {
        1
    } else {
        2
    }
}

/// The Cartesian state at the width `R` rebuilt from the march's relative positions `q`, the closest pair's `qᵢ` kept
/// as the march has it and the closure gap split between the other two, then `HgSystem::cartesian`'s
/// `rᵢ = (mⱼqₖ − mₖqⱼ)/M`, with the image's momenta: the image puts the gap on every position, this puts it on the two
/// long separations only.
fn rebuild<R: Width>(image: &State<R>, q: &[[R; 2]; 3]) -> State<R> {
    let m = image.m;
    let mut q = *q;
    let i = closest(&q);
    let half = R::one() / (R::one() + R::one());
    let c = [q[0][0] + q[1][0] + q[2][0], q[0][1] + q[1][1] + q[2][1]];
    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
    for a in 0..2 {
        q[j][a] = q[j][a] - c[a] * half;
        q[k][a] = q[k][a] - c[a] * half;
    }
    let total = m[0] + m[1] + m[2];
    let r = |mj: R, qk: [R; 2], mk: R, qj: [R; 2]| {
        [
            (mj * qk[0] - mk * qj[0]) / total,
            (mj * qk[1] - mk * qj[1]) / total,
        ]
    };
    State {
        m,
        r: [
            r(m[1], q[2], m[2], q[1]),
            r(m[2], q[0], m[0], q[2]),
            r(m[0], q[1], m[1], q[0]),
        ],
        p: image.p,
    }
}

/// The Kepler `a` of the pair other than body `i` in the state `s`.
fn pair_a_of(s: &State<f64>, i: usize) -> f64 {
    let (j, k) = others(i);
    let v = |b: usize| [s.p[b][0] / s.m[b], s.p[b][1] / s.m[b]];
    let (vj, vk) = (v(j), v(k));
    kepler(
        [s.r[k][0] - s.r[j][0], s.r[k][1] - s.r[j][1]],
        [vk[0] - vj[0], vk[1] - vj[1]],
        s.m[j] + s.m[k],
    )
    .a
}

/// The drops of a label sequence: every mark whose label is none after a labelled one, the first label on.
fn drops(labels: &[Option<u32>]) -> Vec<usize> {
    (1..labels.len())
        .filter(|&i| labels[i - 1].is_some() && labels[i].is_none())
        .collect()
}

/// **Pass criterion:** no cell's label drops back to nothing yet after its first label.
fn check_no_drops(what: &str, cells: &[(usize, Vec<Option<u32>>)]) {
    let total: usize = cells.iter().map(|(_, l)| drops(l).len()).sum();
    let on: Vec<usize> = cells
        .iter()
        .filter(|(_, l)| !drops(l).is_empty())
        .map(|(c, _)| *c)
        .collect();
    assert_eq!(
        total, 0,
        "the label {what} drops back {total} times, on cells {on:?}"
    );
}

/// The view's escape cells (the field under Fewbody live) and their IC.
fn view_escapes() -> (Vec<Cell>, Vec<usize>, CollisionRadius) {
    let sim = capture_view();
    let cells = cells(&sim, SIZE);
    let field = field_rule::<Fewbody<f32>>(&cells, &sim, false);
    let escapes: Vec<usize> = (0..field.outcomes.len())
        .filter(|&i| is_escape(field.outcomes[i]))
        .collect();
    let (radius, _, _) = march_values(&sim);
    (cells, escapes, radius)
}

/// The first measurement: at every drop of the f32 live label on the view's escape cells by `T_eval`, each term of the
/// Fewbody test at the mark before, the drop's first unlabelled mark and the mark it returns at, at f32 and at f64 on
/// the same f32 state.
///
/// **Pass criterion, added after the first run had printed these numbers** (it guards the measurement, it does not
/// judge the label): the transcription is the kernel's test at every mark, so the terms printed are the ones that
/// decided, and the count reproduces #73's: 70 drops on 36 cells by `T_eval`.
///
/// **Prediction, written before the run, falsified by it:** the leg that flips is the tidal ratio against `δ`, through
/// the inner pair's Kepler `a`, whose specific energy `½|Δv|² − m/d` cancels at f32 near pericentre; so the f64
/// evaluation of the same f32 state still fires. **Measured:** 57 drops find no bound pair at all and 13 find the pair
/// with its tidal ratio over `δ` because its `a` jumped; the f64 evaluation of the same f32 state drops on all 70. So
/// the test's arithmetic is not the cause: the state it is handed is (the second measurement).
#[test]
#[ignore = "the view's escape cells marched to T_eval: seconds in release"]
fn view_label_drops_by_term() {
    let (cells, escapes, radius) = view_escapes();
    let p32 = params_at::<f32>(1.0);
    let runs = parallel(escapes.len(), |n| {
        Ok(marks_rel(&cells[escapes[n]].ic, radius, &p32))
    })
    .expect("the escape cells' marches");
    let states: Vec<(State<f32>, f32)> = runs
        .iter()
        .flat_map(|run| run.marks.iter().map(|mk| (mk.image, run.e_0)))
        .collect();
    let march_disagrees = runs
        .iter()
        .flat_map(|run| run.marks.iter().map(move |mk| (mk, run.e_0)))
        .filter(|(mk, e_0)| transcribed(&mk.image, *e_0) != mk.label)
        .count();
    let mut n_drops = 0;
    let mut n_cells = 0;
    let mut by_leg: BTreeMap<String, usize> = BTreeMap::new();
    let mut f64_also = 0;
    for (n, run) in runs.iter().enumerate() {
        let labels: Vec<Option<u32>> = run.marks.iter().map(|mk| mk.label).collect();
        let at = drops(&labels);
        n_drops += at.len();
        n_cells += usize::from(!at.is_empty());
        for &d in &at {
            let back = (d..labels.len())
                .find(|&j| labels[j].is_some())
                .unwrap_or(labels.len() - 1);
            let (row, col) = row_col(escapes[n]);
            println!(
                "cell {} (row {row}, col {col}), drop at t = {}, back at t = {}",
                escapes[n], run.marks[d].t, run.marks[back].t
            );
            let mut t32 = Vec::new();
            let mut t64 = Vec::new();
            for (w, j) in ["before", "drop", "after"].iter().zip([d - 1, d, back]) {
                let s = &run.marks[j].image;
                let a = fewbody_terms(&s.m, &s.r, &s.p, run.e_0);
                let w64 = widen_state(s);
                let b = fewbody_terms(&w64.m, &w64.r, &w64.p, f64::from(run.e_0));
                println!("  {w:<6} t = {:>8.3} f32 {}", run.marks[j].t, a.line());
                println!("  {w:<6} t = {:>8.3} f64 {}", run.marks[j].t, b.line());
                t32.push(a);
                t64.push(b);
            }
            let leg = flipped(&t32[0], &t32[1]);
            println!(
                "  flips at f32: [{leg}]; at f64 on the f32 state: [{}]",
                flipped(&t64[0], &t64[1])
            );
            *by_leg.entry(leg).or_default() += 1;
            f64_also += usize::from(t64[1].fired.is_none());
        }
    }
    println!(
        "escape cells {}; drops by T_eval {n_drops} on {n_cells} cells; marks where the march's label is not the \
         transcription's: {march_disagrees}",
        escapes.len()
    );
    println!("the legs that flip at the drop, at f32: {by_leg:#?}");
    println!(
        "drops where the f64 evaluation of the same f32 state also reads no escape: {f64_also} of {n_drops}"
    );
    check_transcription(transcribed, &states);
    assert_eq!(
        march_disagrees, 0,
        "the march's label is not the transcription's"
    );
    assert_eq!((n_drops, n_cells), (70, 36), "not #73's drops");
}

negative_control!(
    view_label_drops_by_term,
    "a transcription without the tidal leg must not pass as the kernel's test",
    expected = "the transcription is not the kernel's test",
    check_transcription(transcribed_without_tidal, &[tidal_decides()])
);

/// The second measurement: the same test on the march's own relative positions, against the image it is handed; and
/// the closure gap the image spreads, by width and step.
///
/// **First prediction, written before its run, falsified by it:** the image rounds each position at the pair's distance
/// from the centre of mass, so the pair's separation the test forms from it loses digits near pericentre, and a
/// rebuild from the three `qᵢ` as the image builds it would not drop. **Measured:** at the drops the image's closest
/// pair separation differs from the march's `|qᵢ|` by far more than f32's ulp at the positions' scale (the run prints
/// the range over every drop), and that rebuild drops 70 times too: the difference is the closure gap
/// `|q₀ + q₁ + q₂|`, which the image spreads over every position.
///
/// **Pass criteria.** Written before its run: the test read on [`rebuild`]'s state (the march's closest `qᵢ` kept, the
/// gap on the two long separations) drops 0 times by `T_eval` at f64, and at f32 alike (so a kernel form at f32 could
/// get it). Added after a run had printed it, as a guard on the measurement: the cut march is the uncut march at every
/// mark (`cut_differs` 0, against [`marks_f32`]'s). The whole view's comparison at the end is printed, not judged.
#[test]
#[ignore = "the view's escape cells marched to T_eval at three settings: a minute in release"]
fn view_label_drops_image_against_march() {
    let (cells, escapes, radius) = view_escapes();
    let p32 = params_at::<f32>(1.0);
    let runs = parallel(escapes.len(), |n| {
        let ic = &cells[escapes[n]].ic;
        Ok((marks_f32(ic, radius, &p32), marks_rel(ic, radius, &p32)))
    })
    .expect("the escape cells' marches");
    let cut_differs = runs
        .iter()
        .filter(|((_, a), b)| {
            a.len() != b.marks.len()
                || a.iter()
                    .zip(&b.marks)
                    .any(|(x, y)| x.0 != y.t || x.1 != y.image || x.2 != y.label)
        })
        .count();
    println!("cells whose cut march differs from the uncut at any mark: {cut_differs}");
    let mut image = Vec::new();
    let mut on32 = Vec::new();
    let mut on64 = Vec::new();
    let mut lengths: BTreeMap<usize, usize> = BTreeMap::new();
    let mut never_back = 0;
    // Over every drop: the march's |q_i| of the closest pair, the image's relative error on it, and the closure gap.
    let mut at_drops: Vec<[f64; 3]> = Vec::new();
    println!(
        "every drop (cell, row, col; drop and return; the leg that flips; the closest pair `i`, the march's |q_i| and \
         the image's separation; the closure gap; the pair's Kepler a from the image and from the rebuild at f32, at \
         the marks before, at and after the drop; the test on the rebuild at f32, f64):"
    );
    for (n, (_, run)) in runs.iter().enumerate() {
        let e_0 = run.e_0;
        let li: Vec<Option<u32>> = run.marks.iter().map(|mk| mk.label).collect();
        let reb32: Vec<State<f32>> = run
            .marks
            .iter()
            .map(|mk| rebuild(&mk.image, &mk.q))
            .collect();
        let reb64: Vec<State<f64>> = run
            .marks
            .iter()
            .map(|mk| rebuild(&widen_state(&mk.image), &mk.q.map(|v| v.map(f64::from))))
            .collect();
        let l32: Vec<Option<u32>> = reb32
            .iter()
            .map(|s| fewbody_terms(&s.m, &s.r, &s.p, e_0).fired)
            .collect();
        let l64: Vec<Option<u32>> = reb64
            .iter()
            .map(|s| fewbody_terms(&s.m, &s.r, &s.p, f64::from(e_0)).fired)
            .collect();
        for d in drops(&li) {
            let back = (d..li.len()).find(|&j| li[j].is_some());
            match back {
                Some(b) => *lengths.entry(b - d).or_default() += 1,
                None => never_back += 1,
            }
            let after = back.unwrap_or(li.len() - 1);
            let mk = &run.marks[d];
            let i = closest(&mk.q);
            let (j, k) = others(i);
            let s = widen_state(&mk.image);
            let img_sep = norm([s.r[k][0] - s.r[j][0], s.r[k][1] - s.r[j][1]]);
            let q_sep = norm(mk.q[i].map(f64::from));
            let a_img = [d - 1, d, after].map(|x| pair_a_of(&widen_state(&run.marks[x].image), i));
            let a_reb = [d - 1, d, after].map(|x| pair_a_of(&widen_state(&reb32[x]), i));
            let t = |x: usize| {
                let s = &run.marks[x].image;
                fewbody_terms(&s.m, &s.r, &s.p, e_0)
            };
            at_drops.push([q_sep, (img_sep - q_sep).abs() / q_sep, gap_of(mk)]);
            let (row, col) = row_col(escapes[n]);
            println!(
                "  cell {:>4} (row {row:>2}, col {col:>2}) t {:>7.3}→{:>7.3}{} | {} | pair of body {i}: |q| {:.4e} image {:.4e} \
                 gap {:.2e} | a image {:+.4e} {:+.4e} {:+.4e} | a rebuilt {:+.4e} {:+.4e} {:+.4e} | rebuilt reads {:?} {:?}",
                escapes[n],
                mk.t,
                run.marks[after].t,
                if back.is_none() { " (not back)" } else { "" },
                flipped(&t(d - 1), &t(d)),
                q_sep,
                img_sep,
                gap_of(mk),
                a_img[0],
                a_img[1],
                a_img[2],
                a_reb[0],
                a_reb[1],
                a_reb[2],
                l32[d],
                l64[d]
            );
        }
        image.push((escapes[n], li));
        on32.push((escapes[n], l32));
        on64.push((escapes[n], l64));
    }
    let count = |v: &[(usize, Vec<Option<u32>>)]| {
        let d: Vec<usize> = v.iter().map(|(_, l)| drops(l).len()).collect();
        (
            d.iter().sum::<usize>(),
            d.iter().filter(|&&x| x > 0).count(),
        )
    };
    println!(
        "drops by T_eval (drops, cells): on the image, the rule's {:?}; on the rebuild at f32 {:?}; at f64 {:?}",
        count(&image),
        count(&on32),
        count(&on64)
    );
    for (w, name) in [
        "the march's |q_i| of the closest pair",
        "the image's relative error on that separation",
        "the closure gap",
    ]
    .iter()
    .enumerate()
    {
        let v: Vec<f64> = at_drops.iter().map(|x| x[w]).collect();
        println!(
            "at the {} drops, {name}: min {:.2e} max {:.2e}",
            v.len(),
            v.iter().copied().fold(f64::INFINITY, f64::min),
            v.iter().copied().fold(0.0, f64::max)
        );
    }
    println!(
        "drop lengths in window marks (marks unlabelled: drops): {lengths:?}; not back by T_eval: {never_back}"
    );
    // A collision ends the march on the image's separations too: at each escape cell's collision by T_eval, the march's
    // least |q_i| against the image's least separation and r_coll.
    println!("collisions by T_eval among the escape cells (collision radius {radius:?}):");
    for (n, (_, run)) in runs.iter().enumerate() {
        if let Some((t, least_q, least_r)) = run.collision {
            let (row, col) = row_col(escapes[n]);
            println!(
                "  cell {:>4} (row {row:>2}, col {col:>2}) t {t:>8.3}: the march's least |q| {least_q:.4e}, the image's least \
                 separation {least_r:.4e}",
                escapes[n]
            );
        }
    }
    // Is the closure gap the march's f32 rounding or its step? The dropping cells' gap at their last mark by T_eval:
    // the field's f32 at η, f32 at η/4 (four times the steps) and f64 at η.
    let dropping: Vec<usize> = image
        .iter()
        .filter(|(_, l)| !drops(l).is_empty())
        .map(|(c, _)| *c)
        .collect();
    let p32q = params_at::<f32>(0.25);
    let p64 = params_at::<f64>(1.0);
    let gaps = parallel(dropping.len(), |n| {
        let ic = &cells[dropping[n]].ic;
        let last = |v: Vec<f64>| v.last().copied().unwrap_or(f64::NAN);
        Ok([
            last(
                marks_rel(ic, radius, &p32)
                    .marks
                    .iter()
                    .map(gap_of)
                    .collect(),
            ),
            last(
                marks_rel(ic, radius, &p32q)
                    .marks
                    .iter()
                    .map(gap_of)
                    .collect(),
            ),
            last(
                marks_rel(&widen_ic(ic), radius, &p64)
                    .marks
                    .iter()
                    .map(gap_of)
                    .collect(),
            ),
        ])
    })
    .expect("the gaps");
    for (w, name) in ["f32 at η", "f32 at η/4", "f64 at η"].iter().enumerate() {
        let mut g: Vec<f64> = gaps.iter().map(|x| x[w]).collect();
        g.sort_by(f64::total_cmp);
        println!(
            "the closure gap at the last mark by T_eval on the {} dropping cells, {name}: min {:.2e} median {:.2e} max \
             {:.2e}",
            g.len(),
            g[0],
            g[g.len() / 2],
            g[g.len() - 1]
        );
    }
    whole_view(&cells, radius, &p32);
    assert_eq!(cut_differs, 0, "the cut march is not the march");
    check_no_drops("on the rebuild at f64", &on64);
    check_no_drops("on the rebuild at f32", &on32);
}

negative_control!(
    view_label_drops_image_against_march,
    "a label that drops back and returns must be counted",
    expected = "drops back 1 times",
    check_no_drops("of the control", &[(0, vec![None, Some(0), None, Some(0)])])
);

/// The whole view, every cell the field marches, at f32 to `T_eval`: the label the rule reads on the image against the
/// test on [`rebuild`]'s state, at the horizon and by `T_eval`, and the collisions the image ends a march with against
/// the march's own least `|qᵢ|`. Printed, not judged: it sizes what option 1 of the PR would change, and counts the cells
/// sitting in a drop at the horizon, which the escape cells leave out.
fn whole_view(cells: &[Cell], radius: CollisionRadius, p32: &HgParams<f32>) {
    let runs = parallel(cells.len(), |i| {
        Ok(
            (t0(&cells[i].ic, cells[i].degenerate, &radius).state == STATE_RUNNING)
                .then(|| marks_rel(&cells[i].ic, radius, p32)),
        )
    })
    .expect("the view's marches");
    let r_coll = 1e-3;
    let mut at_h: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut d_img, mut d_reb, mut c_img, mut c_reb) = (0, 0, 0, 0);
    let mut collisions: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut gained = Vec::new();
    for (i, run) in runs.iter().enumerate() {
        let Some(run) = run else { continue };
        let li: Vec<Option<u32>> = run.marks.iter().map(|mk| mk.label).collect();
        let lr: Vec<Option<u32>> = run
            .marks
            .iter()
            .map(|mk| {
                let s = rebuild(&mk.image, &mk.q);
                fewbody_terms(&s.m, &s.r, &s.p, run.e_0).fired
            })
            .collect();
        let (a, b) = (drops(&li).len(), drops(&lr).len());
        d_img += a;
        d_reb += b;
        c_img += usize::from(a > 0);
        c_reb += usize::from(b > 0);
        let h = run.marks.iter().position(|mk| mk.t == HORIZON);
        let kind = match h.map(|h| (li[h], lr[h])) {
            None => "the march ended before the horizon",
            Some((x, y)) if x == y => "the same",
            Some((Some(_), None)) => "image escape, rebuild none",
            Some((None, Some(_))) => {
                gained.push(i);
                "image none, rebuild escape"
            }
            Some(_) => "escape of another body",
        };
        *at_h.entry(kind).or_default() += 1;
        if let Some((t, least_q, _)) = run.collision {
            let when = if t <= HORIZON {
                "by the horizon"
            } else {
                "after it"
            };
            let x = least_q / r_coll;
            let real = if x < 1.0 {
                "march |q| < r_coll"
            } else if x < 1.1 {
                "march |q| 1-1.1 r_coll"
            } else if x < 2.0 {
                "march |q| 1.1-2 r_coll"
            } else if x < 10.0 {
                "march |q| 2-10 r_coll"
            } else {
                "march |q| > 10 r_coll"
            };
            *collisions.entry((when, real)).or_default() += 1;
        }
    }
    println!(
        "the whole view, {} cells marched: at the horizon, the rule's label on the image against the rebuild's: {at_h:?}",
        runs.iter().flatten().count()
    );
    println!(
        "  the cells labelled on the rebuild but not on the image at the horizon (row, col): {:?}",
        gained.iter().map(|&i| row_col(i)).collect::<Vec<_>>()
    );
    println!(
        "  drops by T_eval after the first label (drops, cells): on the image ({d_img}, {c_img}); on the rebuild \
         ({d_reb}, {c_reb})"
    );
    println!("  collisions by T_eval that end a march (when, the march's own least |q| on that step): {collisions:?}");
}
