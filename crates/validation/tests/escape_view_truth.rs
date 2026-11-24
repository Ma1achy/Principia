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
use kernel::chazy::{ma_stable, most_separated, others, Playhead};
use kernel::decode::degenerate::Degenerate;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::escape_rule::{Closure, DefaultRule, EscapeRule, Fewbody};
use kernel::detect::t0::t0;
use kernel::driver::advance::Cadence;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{monitored, WrapperReal};
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    sd_detail, sd_state, set_detail, set_state, SimStateBaseOf, STATE_BOUNDED, STATE_COLLISION,
    STATE_ESCAPE, STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{AzReal, End, ExplicitTest};
use kernel::regularisation::heggie::{march, HgConfig, HgParams};
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
