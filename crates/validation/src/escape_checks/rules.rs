//! The escape rules measured against a continued-run truth (TASK-M3-34): every rule on the same production march,
//! each read at the escape window's marks as production reads it, scored by its first commit against prin-rs's
//! continued-run truth. The pass criteria and the default's pick are the PR's, written before the run of record.
//!
//! **One march, every rule.** [`RuleHarness`] wraps the production cadence (`WrapperCadence`, the closure rule) and
//! forwards every hook, so the march is production's to the step; it never ends on an escape. At every sampling point
//! it hands the state to each rule's own detector, the kernel's type for a kernel rule (`kernel::detect::escape_rule`),
//! through the trait production calls, so a rule's commit here is the one a production march under that rule would
//! end on. [`same_as_production`] checks that per rule against `engine::precision::integrate_ic_rule`.
//!
//! **The truth** ([`truth`]; prin-rs `examples/chazy_probe.rs::truth` at `57a7f5e`, ported). The march goes on past the
//! evaluation horizon `T_eval`, a record kept every [`SERIES_EVERY`] time units, until the motion is unambiguously
//! asymptotic (hierarchy ratio or least pair separation above [`ASYMPTOTIC`]` · R_0`, `R_0 = 1`) but never before
//! `T_eval`, and at the latest at `T_truth`. Over the record's second half, each pair separation's growth exponent
//! `β = ln(d₂/d₁)/ln(t₂/t₁)`:
//! - all three `β > 0.5` at least separation `> 50`: a dispersal, H, HP or P by how many `β < 0.85`;
//! - at hierarchy ratio `> 100`: with the pair bound (its two-body energy negative, the Kepler `a > 0` prin-rs reads,
//!   decided as the kernel decides it, `kernel::detect::escape_rule::pair_bound`) and the escaper's two separations'
//!   least `β > 0.5`, HE (`> 0.85`) or PE, escaper the most separated body;
//! - otherwise no escape. prin-rs splits that into B-stable (Mardling–Aarseth) and unresolved; both score the same here,
//!   since a commit on either is wrong, and the split waits for the classifier's Mardling–Aarseth test (TASK-M7-35).
//!
//! A march a collision, a failure or a spent budget ended is censored.
//!
//! **Outcome codes.** Truth H/HP/P is a triple ejection (`detail` 3), HE/PE an escape of body `b` (`detail` b). A commit
//! is correct where it names the same; otherwise wrong, a class mismatch counted apart too. A commit on a censored truth
//! is unjudged.

use kernel::decode::Ic;
use kernel::detect::escape::Verdict;
use kernel::detect::escape_rule::{pair_bound, Closure, DistanceFloor, EnergyFloor, EscapeRule};
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::integrate_with;
use kernel::occupant::{Geom, State};
use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;

use engine::precision::MarchKey64;

use super::march::parallel;

/// The record's spacing, time units (prin-rs records every fifth 0.4 snapshot: 2).
pub const SERIES_EVERY: f64 = 2.0;

/// The asymptotic stop's reach, in `R_0` (prin-rs: hierarchy ratio or least separation over `1e3`).
pub const ASYMPTOTIC: f64 = 1e3;

/// The truth's horizon as a multiple of the evaluation horizon.
pub const TRUTH_FACTOR: f64 = 10.0;

/// The rules measured, in table order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleId {
    /// Closure and `E_rel` (today's).
    Closure,
    /// prin-rs's naive energy baseline.
    EnergyFloor,
    /// prin-rs's naive distance baseline.
    DistanceFloor,
}

impl RuleId {
    /// Every rule, in table order.
    pub const ALL: [RuleId; 3] = [RuleId::Closure, RuleId::EnergyFloor, RuleId::DistanceFloor];

    /// The rule's name in the table.
    pub fn name(self) -> &'static str {
        match self {
            RuleId::Closure => "closure",
            RuleId::EnergyFloor => "energy floor",
            RuleId::DistanceFloor => "distance floor",
        }
    }

    /// Whether the rule may be the default (the PR's pick rule): a kernel rule.
    pub fn eligible(self) -> bool {
        true
    }
}

/// Every rule's detector, started on the IC.
#[derive(Clone, Copy, Debug)]
struct Rules {
    closure: Closure<f64>,
    energy: EnergyFloor<f64>,
    distance: DistanceFloor<f64>,
}

impl Rules {
    fn start(ic: &Ic<f64>, e_0: f64) -> Self {
        Rules {
            closure: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            energy: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            distance: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
        }
    }

    /// Each rule at the sampling point, and the verdicts that fired on it, by rule.
    fn sample(&mut self, s: &State<f64>, index: u32, quantum: f64) -> [Option<Verdict>; 3] {
        fn one<E: EscapeRule<f64>>(e: &mut E, s: &State<f64>, i: u32, q: f64) -> Option<Verdict> {
            if e.fired() {
                return None;
            }
            e.sample(&s.m, &s.r, &s.p, i, q);
            e.fired().then(|| e.verdict())
        }
        [
            one(&mut self.closure, s, index, quantum),
            one(&mut self.energy, s, index, quantum),
            one(&mut self.distance, s, index, quantum),
        ]
    }
}

/// A rule's commit: its time and label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The sampling point's index.
    pub index: u32,
    /// The escaper's body.
    pub escaper: u32,
    /// The `detail`: the escaper, or 3 for a triple ejection.
    pub detail: u32,
}

/// One record of the series: time, the pair separations (by the body each leaves out), the greatest hierarchy ratio
/// and its body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub t: f64,
    pub d: [f64; 3],
    pub ratio: f64,
    pub body: usize,
}

/// The pair separations of `r`, by the body each leaves out.
pub fn separations(r: &[[f64; 2]; 3]) -> [f64; 3] {
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    [d(r[1], r[2]), d(r[2], r[0]), d(r[0], r[1])]
}

/// The record of the state `(m, r)` at `t`: each body's distance from the other two's centre of mass over their
/// separation, the greatest and its body (prin-rs's `record`).
pub fn record(m: &[f64; 3], r: &[[f64; 2]; 3], t: f64) -> Record {
    let d = separations(r);
    let out = super::march::distances(m, r);
    let (mut body, mut ratio) = (0, f64::NEG_INFINITY);
    for b in 0..3 {
        let x = out[b] / d[b];
        if x > ratio {
            (body, ratio) = (b, x);
        }
    }
    Record { t, d, ratio, body }
}

/// What ended the continued run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Asymptotic after `T_eval`.
    Asymptotic,
    /// `T_truth` reached.
    Horizon,
    /// A collision, a failure, or the march ended short: censored.
    Censored,
}

/// The production cadence with escape ending nothing, every rule read at every sampling point, and the truth's record
/// (the module's note).
pub struct RuleHarness {
    inner: WrapperCadence<f64>,
    rules: Rules,
    commits: [Option<(f64, Commit)>; 3],
    series: Vec<Record>,
    next_record: f64,
    t_eval: f64,
    asymptotic: bool,
    last: Option<State<f64>>,
}

impl RuleHarness {
    /// The harness of a run from `ic` under `key`'s collision radius, its evaluation horizon `t_eval`.
    pub fn new(ic: &Ic<f64>, key: &MarchKey64, t_eval: f64) -> Self {
        let inner = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, key.radius);
        let e_0 = inner.monitor_state().e_0();
        RuleHarness {
            inner,
            rules: Rules::start(ic, e_0),
            commits: [None; 3],
            series: Vec::new(),
            next_record: SERIES_EVERY,
            t_eval,
            asymptotic: false,
            last: None,
        }
    }
}

impl Cadence<f64> for RuleHarness {
    fn after_step(&mut self, state: &mut State<f64>, geom: &mut Geom<f64>) {
        self.inner.after_step(state, geom);
    }

    /// A collision or a failure ends the march, or the motion turned asymptotic after `T_eval`; an escape does not.
    fn done(&self) -> bool {
        let events = self.inner.events();
        events.failed.fired || events.collision.fired || self.asymptotic
    }

    fn sample(&mut self, state: &State<f64>, index: u32, quantum: f64) {
        self.inner.sample(state, index, quantum);
        let t = f64::from(index) * quantum;
        let fired = self.rules.sample(state, index, quantum);
        for (k, v) in fired.into_iter().enumerate() {
            if let (None, Some(v)) = (self.commits[k], v) {
                self.commits[k] = Some((
                    t,
                    Commit {
                        index,
                        escaper: v.escaper,
                        detail: v.detail,
                    },
                ));
            }
        }
        if t >= self.next_record {
            let rec = record(&state.m, &state.r, t);
            self.next_record += SERIES_EVERY;
            let least = rec.d.iter().copied().fold(f64::INFINITY, f64::min);
            if t >= self.t_eval && (rec.ratio > ASYMPTOTIC || least > ASYMPTOTIC) {
                self.asymptotic = true;
            }
            self.series.push(rec);
        }
        self.last = Some(*state);
    }

    fn bucket_d2(&self, r: &[[f64; 2]; 3]) -> f32 {
        self.inner.bucket_d2(r)
    }

    fn clock(&mut self, completed: u32) {
        self.inner.clock(completed);
    }

    fn non_finite(&mut self, detail: u32) {
        self.inner.non_finite(detail);
    }

    fn failure(&self) -> kernel::detect::label::Fired {
        self.inner.failure()
    }
}

/// The truth of a continued run (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    /// All three separate: H, HP or P (`detail` 3).
    Dispersal,
    /// Body `b` escapes a bound pair: HE or PE.
    Escape(u32),
    /// No escape by `T_truth`: B-stable or unresolved.
    NoEscape,
    /// A collision, a failure or a spent budget ended it.
    Censored,
}

/// prin-rs's truth on the record `series` and the last state `(m, r, p)` (the module's note).
pub fn truth(series: &[Record], m: &[f64; 3], last: &State<f64>) -> Truth {
    let n = series.len();
    if n < 4 {
        return Truth::Censored;
    }
    let end = series[n - 1];
    let Some(start) = series.iter().find(|x| x.t >= 0.5 * end.t) else {
        return Truth::Censored;
    };
    let beta: [f64; 3] =
        std::array::from_fn(|k| (end.d[k] / start.d[k]).ln() / (end.t / start.t).ln());
    let least = end.d.iter().copied().fold(f64::INFINITY, f64::min);
    if least > 50.0 && beta.iter().all(|&x| x > 0.5) {
        return Truth::Dispersal;
    }
    if end.ratio > 100.0 {
        let b = end.body;
        // The escaper's two separations, the pairs that include it: every pair but the one leaving it out.
        let g = (0..3)
            .filter(|&q| q != b)
            .map(|q| beta[q])
            .fold(f64::INFINITY, f64::min);
        let (i, j) = ((b + 1) % 3, (b + 2) % 3);
        let bound = pair_bound((m[i], last.r[i], last.p[i]), (m[j], last.r[j], last.p[j]));
        if bound && g > 0.5 {
            return Truth::Escape(b as u32);
        }
    }
    Truth::NoEscape
}

/// One trajectory measured: its truth, each rule's commit by `T_eval` with its time, `E_0`'s sign, and how the run
/// ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub truth: Truth,
    pub commits: [Option<(f64, Commit)>; 3],
    pub t_eval: f64,
    pub e_0: f64,
    pub end: End,
    pub t_end: f64,
}

/// The run of `ic` by `A` under `key` with its horizon extended to `truth_steps` macro-steps, its evaluation horizon
/// `t_eval` (the module's note).
pub fn measure<A: Advance>(
    ic: &Ic<f64>,
    key: &MarchKey64,
    t_eval: f64,
    truth_steps: u32,
) -> Measured {
    let mut uniforms = key.loop_values;
    uniforms.horizon_steps = truth_steps;
    let mut h = RuleHarness::new(ic, key, t_eval);
    let e_0 = h.inner.monitor_state().e_0();
    integrate_with::<A, f64, RuleHarness>(ic, &uniforms, &mut h);
    let events = h.inner.events();
    let t_end = h.series.last().map_or(0.0, |r| r.t);
    let reached = h
        .series
        .last()
        .is_some_and(|r| r.t + SERIES_EVERY >= f64::from(truth_steps) * uniforms.dt_macro);
    let end = if events.failed.fired || events.collision.fired {
        End::Censored
    } else if h.asymptotic {
        End::Asymptotic
    } else if reached {
        End::Horizon
    } else {
        End::Censored
    };
    let truth = match (end, h.last) {
        (End::Censored, _) | (_, None) => Truth::Censored,
        (_, Some(last)) => truth(&h.series, &ic.m, &last),
    };
    let commits = h.commits.map(|c| c.filter(|(t, _)| *t <= t_eval));
    Measured {
        truth,
        commits,
        t_eval,
        e_0,
        end,
        t_end,
    }
}

/// Every pixel of a population measured, on every core: evaluation horizon `T_eval` its harness horizon,
/// `T_truth = `[`TRUTH_FACTOR`]` · T_eval`.
pub fn measure_population<A: Advance>(pop: &super::Population) -> Result<Vec<Measured>, String> {
    parallel(pop.ics.len(), |i| {
        let t_eval = pop.harness_horizon(i);
        let steps = (TRUTH_FACTOR * f64::from(pop.harness_steps[i])).round() as u32;
        Ok(measure::<A>(&pop.ics[i], &pop.keys[i], t_eval, steps))
    })
}

/// A rule's score against the truth.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Score {
    /// Correct commits: the truth's escaper or class.
    pub correct: usize,
    /// Wrong commits.
    pub wrong: usize,
    /// Of the wrong, those naming an escape where the truth is another kind of escape (single against triple).
    pub wrong_class: usize,
    /// Of the wrong, those on a truth with no escape.
    pub on_no_escape: usize,
    /// Commits on censored truths.
    pub unjudged: usize,
    /// Truth escapes with no correct commit by `T_eval`.
    pub never: usize,
    /// The times of the correct commits.
    pub times: Vec<f64>,
}

impl Score {
    /// The quantile `q` of the correct commits' times, `None` with none.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.times.is_empty() {
            return None;
        }
        let mut v = self.times.clone();
        v.sort_by(f64::total_cmp);
        Some(v[((v.len() - 1) as f64 * q).round() as usize])
    }

    /// Adds `other` in.
    pub fn add(&mut self, other: &Score) {
        self.correct += other.correct;
        self.wrong += other.wrong;
        self.wrong_class += other.wrong_class;
        self.on_no_escape += other.on_no_escape;
        self.unjudged += other.unjudged;
        self.never += other.never;
        self.times.extend_from_slice(&other.times);
    }
}

/// The verdict of one commit against `truth` (the module's note): `Some(true)` correct, `Some(false)` wrong, `None`
/// unjudged.
pub fn judge(commit: &Commit, truth: Truth) -> Option<bool> {
    let triple = commit.detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;
    match truth {
        Truth::Censored => None,
        Truth::NoEscape => Some(false),
        Truth::Dispersal => Some(triple),
        Truth::Escape(b) => Some(!triple && commit.escaper == b),
    }
}

/// Rule `k`'s score over `runs`.
pub fn score(runs: &[Measured], k: usize) -> Score {
    let mut s = Score::default();
    for r in runs {
        let escapes = matches!(r.truth, Truth::Dispersal | Truth::Escape(_));
        match r.commits[k] {
            Some((t, c)) => match judge(&c, r.truth) {
                Some(true) => {
                    s.correct += 1;
                    s.times.push(t);
                }
                Some(false) => {
                    s.wrong += 1;
                    if escapes {
                        s.wrong_class += usize::from(
                            (c.detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION)
                                != (r.truth == Truth::Dispersal),
                        );
                    } else {
                        s.on_no_escape += 1;
                    }
                    s.never += usize::from(escapes);
                }
                None => s.unjudged += 1,
            },
            None => s.never += usize::from(escapes),
        }
    }
    s
}

/// A score's table row.
pub fn row(name: &str, s: &Score) -> String {
    let q = |x: Option<f64>| x.map_or("-".into(), |x| format!("{x:.2}"));
    format!(
        "{name:<16} wrong {:>4} (class {:>3}, on no-escape {:>4})  correct {:>5}  never {:>5}  unjudged {:>4}  \
         first correct p50 {:>8} p90 {:>8}",
        s.wrong,
        s.wrong_class,
        s.on_no_escape,
        s.correct,
        s.never,
        s.unjudged,
        q(s.quantile(0.5)),
        q(s.quantile(0.9))
    )
}

/// The truth's census over `runs`.
pub fn census(runs: &[Measured]) -> String {
    let count = |f: &dyn Fn(&Truth) -> bool| runs.iter().filter(|r| f(&r.truth)).count();
    let neg = runs.iter().filter(|r| r.e_0 < 0.0).count();
    format!(
        "{} trajectories ({} with E < 0): truth dispersal {}, single escape {}, no escape {}, censored {}; continued runs \
         ended asymptotic {}, at T_truth {}",
        runs.len(),
        neg,
        count(&|t| *t == Truth::Dispersal),
        count(&|t| matches!(t, Truth::Escape(_))),
        count(&|t| *t == Truth::NoEscape),
        count(&|t| *t == Truth::Censored),
        runs.iter().filter(|r| r.end == End::Asymptotic).count(),
        runs.iter().filter(|r| r.end == End::Horizon).count(),
    )
}

/// Whether the harness's commit under `rule` on `ic` under `key` is the production march's under `production` (the
/// same rule, unless a control says otherwise), to the horizon `horizon_steps`: production fires escape on the macro-step
/// holding the harness's sampling point with the same `detail`, or neither fires. `Err` says where they part.
pub fn same_as_production<A: Advance>(
    ic: &Ic<f64>,
    key: &MarchKey64,
    rule: RuleId,
    production: RuleId,
    horizon_steps: u32,
) -> Result<(), String> {
    use kernel::payload::{sd_detail, sd_state, tm_t_end_step, STATE_ESCAPE};
    let mut k = *key;
    k.loop_values.horizon_steps = horizon_steps;
    let t_h = f64::from(horizon_steps) * k.loop_values.dt_macro;
    let out = match production {
        RuleId::Closure => engine::precision::integrate_ic_rule::<A, Closure<f64>>(ic, None, &k),
        RuleId::EnergyFloor => {
            engine::precision::integrate_ic_rule::<A, EnergyFloor<f64>>(ic, None, &k)
        }
        RuleId::DistanceFloor => {
            engine::precision::integrate_ic_rule::<A, DistanceFloor<f64>>(ic, None, &k)
        }
    };
    let m = measure::<A>(ic, &k, t_h, horizon_steps);
    let idx = RuleId::ALL.iter().position(|&r| r == rule).expect("a rule");
    let production = (sd_state(out.packed_a) == STATE_ESCAPE).then(|| {
        (
            f64::from(tm_t_end_step(out.times)) * k.loop_values.dt_macro,
            sd_detail(out.packed_a),
        )
    });
    let harness = m.commits[idx].map(|(t, c)| (t, c.detail));
    match (production, harness) {
        (None, None) => Ok(()),
        (Some((tp, dp)), Some((th, dh)))
            if dp == dh && (tp - th).abs() <= k.loop_values.dt_macro =>
        {
            Ok(())
        }
        (p, h) => Err(format!(
            "{}: production ends in {p:?} (time, detail), the harness commits {h:?}",
            rule.name()
        )),
    }
}
