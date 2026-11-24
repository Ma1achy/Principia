//! The escape rules measured against a continued-run truth (TASK-M3-34): every rule on the same production march,
//! each read at the escape window's marks as production reads it, scored by its first commit against prin-rs's
//! continued-run truth. The pass criteria and the default's pick are the PR's, written before the run of record.
//!
//! **One march, every rule, both modes.** [`RuleHarness`] wraps the production cadence (`WrapperCadence`, the default
//! rule) and forwards every hook, so the march is production's to the step; it never ends on an escape. At every
//! sampling point it hands the state to each rule's own kernel type (`kernel::detect::escape_rule`) through the trait
//! production calls, and records the rule's label each time it changes ([`Measured::labels`]): the live label, the
//! default mode's. The first label is the stop option's: under it production ends there and the label stays, and the
//! march up to that point is the same march, so a rule's first label here ([`Measured::commits`]) is the one a
//! production march under the option ends on. [`same_as_production`] checks both against
//! `engine::precision::integrate_ic_rule`. Fewbody is the kernel's port; validation's reference
//! (`crate::chazy::fewbody_resolved`, [`fewbody_reference`]) is read beside it at every mark and every mark where the
//! two differ is counted ([`Measured::fewbody_check`]).
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
//! - otherwise no escape, split as prin-rs splits it into B-stable (Mardling–Aarseth on the last state, the body most
//!   separated from its pair outside, `kernel::chazy::ma_stable`) and unresolved ([`Measured::b_stable`]). #49's table
//!   ([`score`]) scores both as wrong and counts the unresolved apart; the live scoring (`super::live`) reports a label
//!   on an unresolved truth apart, neither right nor wrong.
//!
//! A case whose truth is known by construction (a periodic orbit, a Mardling–Aarseth stable triple) is
//! [`Truth::Bound`], set by its case and not read from the record.
//!
//! A march a collision, a failure or a spent budget ended is censored.
//!
//! **Outcome codes.** Truth H/HP/P is a triple ejection (`detail` 3), HE/PE an escape of body `b` (`detail` b). A commit
//! is correct where it names the same; otherwise wrong, a class mismatch counted apart too. A commit on a censored truth
//! is unjudged.

use kernel::chazy::{ma_stable, most_separated, others, Playhead};
use kernel::decode::Ic;
use kernel::detect::escape::{is_mark, Verdict};
use kernel::detect::escape_rule::{
    pair_bound, Chazy, Closure, DistanceFloor, EnergyFloor, EscapeRule, Fewbody,
};
use kernel::driver::advance::{Advance, Cadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::integrate_with;
use kernel::generated::constants::CHAZY_DELTA;
use kernel::occupant::{Geom, State};
use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;
use kernel::payload::{DETAIL_ESCAPE_BODY_0, DETAIL_ESCAPE_BODY_1, DETAIL_ESCAPE_BODY_2};

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
    /// prin-rs's Chazy final-motion classifier, live (`kernel::detect::escape_rule::Chazy`).
    Chazy,
    /// Fewbody's termination test (`kernel::detect::escape_rule::Fewbody`, the port of the reference).
    Fewbody,
}

impl RuleId {
    /// Every rule, in table order.
    pub const ALL: [RuleId; 5] = [
        RuleId::Closure,
        RuleId::EnergyFloor,
        RuleId::DistanceFloor,
        RuleId::Chazy,
        RuleId::Fewbody,
    ];

    /// The rule `kernel::detect::escape_rule::DefaultRule` names (kept equal to it by `escape_rules_default_is_the_kernels`).
    pub const DEFAULT: RuleId = RuleId::Fewbody;

    /// The rule's name in the table.
    pub fn name(self) -> &'static str {
        match self {
            RuleId::Closure => "closure",
            RuleId::EnergyFloor => "energy floor",
            RuleId::DistanceFloor => "distance floor",
            RuleId::Chazy => "chazy",
            RuleId::Fewbody => "fewbody",
        }
    }

    /// Whether the rule may be the default: every rule is a kernel rule since Fewbody's port.
    pub fn eligible(self) -> bool {
        true
    }
}

/// A fired verdict on `escaper`, a triple ejection where `triple`.
fn verdict_of(escaper: usize, triple: bool) -> Verdict {
    let body = [
        DETAIL_ESCAPE_BODY_0,
        DETAIL_ESCAPE_BODY_1,
        DETAIL_ESCAPE_BODY_2,
    ][escaper];
    Verdict {
        fired: true,
        escaper: body,
        detail: if triple {
            DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
        } else {
            body
        },
    }
}

/// The reference's verdict on the state `(m, r, p)`: `crate::chazy::fewbody_resolved` at the register's `δ`, its H a
/// triple ejection of the body most separated from its pair, its HE the escape of the third body `k` (the one outside
/// its least-`a` bound pair), B-stable and unresolved `None`. The kernel's [`Fewbody`] is its port.
pub fn fewbody_reference(m: &[f64; 3], r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]) -> Option<Verdict> {
    fewbody_reference_at(m, r, p, CHAZY_DELTA)
}

/// [`fewbody_reference`] at the tidal tolerance `delta` in place of the register's.
pub fn fewbody_reference_at(
    m: &[f64; 3],
    r: &[[f64; 2]; 3],
    p: &[[f64; 2]; 3],
    delta: f64,
) -> Option<Verdict> {
    use crate::chazy::{fewbody_resolved, Reference, PAIRS};
    let s = crate::chazy::State {
        m: *m,
        r: *r,
        v: std::array::from_fn(|k| [p[k][0] / m[k], p[k][1] / m[k]]),
    };
    match fewbody_resolved(&s, delta) {
        Reference::H => {
            let ph = Playhead::from_momenta(*m, *r, *p, s.energy(), 1.0, 1.0);
            Some(verdict_of(most_separated(&ph), true))
        }
        Reference::He => {
            // The third body of the least-`a` bound pair, as `fewbody_resolved` chooses it.
            let (i, j) = PAIRS
                .iter()
                .map(|&(i, j)| (i, j, s.pair_kepler(i, j).a))
                .filter(|&(_, _, a)| a > 0.0)
                .fold(None, |best: Option<(usize, usize, f64)>, c| match best {
                    Some(b) if b.2 <= c.2 => Some(b),
                    _ => Some(c),
                })
                .map(|(i, j, _)| (i, j))
                .expect("an HE has a bound pair");
            Some(verdict_of(3 - i - j, false))
        }
        Reference::BStable | Reference::Unresolved => None,
    }
}

/// Every rule's detector, started on the IC.
#[derive(Clone, Copy, Debug)]
struct Rules {
    closure: Closure<f64>,
    energy: EnergyFloor<f64>,
    distance: DistanceFloor<f64>,
    chazy: Chazy<f64>,
    fewbody: Fewbody<f64>,
}

impl Rules {
    fn start(ic: &Ic<f64>, e_0: f64) -> Self {
        Rules {
            closure: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            energy: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            distance: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            chazy: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
            fewbody: EscapeRule::start(&ic.m, &ic.r, &ic.p, e_0),
        }
    }

    /// Each rule at the sampling point with the cadence's `I_max`, and each rule's label after it, by rule: live, every
    /// rule re-reading its label at every mark.
    fn sample(
        &mut self,
        s: &State<f64>,
        index: u32,
        quantum: f64,
        i_max: f64,
    ) -> [Option<Verdict>; 5] {
        fn one<E: EscapeRule<f64>>(
            e: &mut E,
            s: &State<f64>,
            i: u32,
            q: f64,
            i_max: f64,
        ) -> Option<Verdict> {
            e.sample(&s.m, &s.r, &s.p, i, q, i_max);
            e.fired().then(|| e.verdict())
        }
        [
            one(&mut self.closure, s, index, quantum, i_max),
            one(&mut self.energy, s, index, quantum, i_max),
            one(&mut self.distance, s, index, quantum, i_max),
            one(&mut self.chazy, s, index, quantum, i_max),
            one(&mut self.fewbody, s, index, quantum, i_max),
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

/// A rule's label changes: each the time it took the label and the label, `None` no escape; the label at `t = 0` is
/// `None` and not recorded.
pub type Labels = Vec<(f64, Option<Commit>)>;

/// Whether two labels are the same label: both none, or both an escape with the same `detail` (the escaper's body, or
/// 3, a triple ejection, which names no body). A change of body is a change of label.
pub fn same_label(a: Option<Commit>, b: Option<Commit>) -> bool {
    a.map(|c| c.detail) == b.map(|c| c.detail)
}

/// The production cadence with escape ending nothing, every rule read at every sampling point, and the truth's record
/// (the module's note).
pub struct RuleHarness {
    inner: WrapperCadence<f64>,
    rules: Rules,
    commits: [Option<(f64, Commit)>; 5],
    labels: [Labels; 5],
    fewbody_check: (u32, u32),
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
            commits: [None; 5],
            labels: Default::default(),
            fewbody_check: (0, 0),
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
        let fired = self
            .rules
            .sample(state, index, quantum, self.inner.i_max_now());
        for (k, v) in fired.into_iter().enumerate() {
            let label = v.map(|v| Commit {
                index,
                escaper: v.escaper,
                detail: v.detail,
            });
            let last = self.labels[k].last().and_then(|(_, l)| *l);
            if !same_label(last, label) {
                self.labels[k].push((t, label));
            }
            if let (None, Some(c)) = (self.commits[k], label) {
                self.commits[k] = Some((t, c));
            }
        }
        if is_mark(index, quantum) {
            let port = self.rules.fewbody.verdict();
            let port = port.fired.then_some(port.detail);
            let reference = fewbody_reference(&state.m, &state.r, &state.p).map(|v| v.detail);
            self.fewbody_check.0 += 1;
            self.fewbody_check.1 += u32::from(port != reference);
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
    /// Bound by construction (a periodic orbit, a Mardling–Aarseth stable triple): set by its case.
    Bound,
    /// A collision, a failure or a spent budget ended it.
    Censored,
}

/// The record's last entry and the second half's growth exponents `β`, by the body each pair leaves out, as [`truth`]
/// reads them; `None` on a record too short to judge.
pub fn exponents(series: &[Record]) -> Option<(Record, [f64; 3])> {
    let end = *series.last()?;
    if series.len() < 4 {
        return None;
    }
    let start = series.iter().find(|x| x.t >= 0.5 * end.t)?;
    Some((
        end,
        std::array::from_fn(|k| (end.d[k] / start.d[k]).ln() / (end.t / start.t).ln()),
    ))
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
    pub commits: [Option<(f64, Commit)>; 5],
    pub t_eval: f64,
    pub e_0: f64,
    pub end: End,
    pub t_end: f64,
    /// Where the truth is no escape: whether Mardling–Aarseth certifies the last state (B-stable), else unresolved.
    pub b_stable: bool,
    /// The record's last entry and its growth exponents ([`exponents`]), for the per-trajectory listing.
    pub last_record: Option<(Record, [f64; 3])>,
    /// Each rule's live label changes over the whole continued run ([`Labels`]).
    pub labels: [Labels; 5],
    /// The marks at which the kernel's Fewbody was read beside the reference, and how many of them the two differ on.
    pub fewbody_check: (u32, u32),
    /// The record of the continued run ([`record`] every [`SERIES_EVERY`]), for the cases that read it.
    pub series: Vec<Record>,
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
    let b_stable = truth == Truth::NoEscape
        && h.last.is_some_and(|last| {
            let ph = Playhead::from_momenta(last.m, last.r, last.p, e_0, 1.0, 1.0);
            let b = most_separated(&ph);
            let (i, j) = others(b);
            ma_stable(&ph, i, j, b)
        });
    Measured {
        truth,
        commits,
        t_eval,
        e_0,
        end,
        t_end,
        b_stable,
        last_record: exponents(&h.series),
        labels: h.labels,
        fewbody_check: h.fewbody_check,
        series: h.series,
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
    /// Of those, the ones on an unresolved truth (no escape, not B-stable).
    pub on_unresolved: usize,
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
        self.on_unresolved += other.on_unresolved;
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
        Truth::NoEscape | Truth::Bound => Some(false),
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
                        s.on_unresolved += usize::from(!r.b_stable);
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
        "{name:<16} wrong {:>4} (class {:>3}, on no-escape {:>4}, of them unresolved {:>4})  correct {:>5}  never {:>5}  unjudged {:>4}  \
         first correct p50 {:>8} p90 {:>8}",
        s.wrong,
        s.wrong_class,
        s.on_no_escape,
        s.on_unresolved,
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
        "{} trajectories ({} with E < 0): truth dispersal {}, single escape {}, no escape {} (B-stable {}, unresolved \
         {}), bound by construction {}, censored {}; continued runs ended asymptotic {}, at T_truth {}",
        runs.len(),
        neg,
        count(&|t| *t == Truth::Dispersal),
        count(&|t| matches!(t, Truth::Escape(_))),
        count(&|t| *t == Truth::NoEscape),
        runs.iter().filter(|r| r.truth == Truth::NoEscape && r.b_stable).count(),
        runs.iter().filter(|r| r.truth == Truth::NoEscape && !r.b_stable).count(),
        count(&|t| *t == Truth::Bound),
        count(&|t| *t == Truth::Censored),
        runs.iter().filter(|r| r.end == End::Asymptotic).count(),
        runs.iter().filter(|r| r.end == End::Horizon).count(),
    )
}

/// The production march of `ic` under `key` with the escape rule `rule`: `engine::precision::integrate_ic_rule` with
/// that rule's kernel type.
pub fn production<A: Advance>(
    ic: &Ic<f64>,
    key: &MarchKey64,
    rule: RuleId,
) -> engine::precision::SimState64 {
    use engine::precision::integrate_ic_rule;
    match rule {
        RuleId::Closure => integrate_ic_rule::<A, Closure<f64>>(ic, None, key),
        RuleId::EnergyFloor => integrate_ic_rule::<A, EnergyFloor<f64>>(ic, None, key),
        RuleId::DistanceFloor => integrate_ic_rule::<A, DistanceFloor<f64>>(ic, None, key),
        RuleId::Chazy => integrate_ic_rule::<A, Chazy<f64>>(ic, None, key),
        RuleId::Fewbody => integrate_ic_rule::<A, Fewbody<f64>>(ic, None, key),
    }
}

/// Whether the harness under `rule` on `ic` under `key` is the production march under `production` (the same rule,
/// unless a control says otherwise), to the horizon `horizon_steps`, in both modes. Under the stop option: production
/// ends in escape on the macro-step holding the harness's first label with the same `detail`, or neither labels an
/// escape. Live, the default: production's label at the horizon is the harness's last label by then, an escape with the
/// same `detail` or none, a collision where the harness's march met one. `Err` says where they part.
pub fn same_as_production<A: Advance>(
    ic: &Ic<f64>,
    key: &MarchKey64,
    rule: RuleId,
    production: RuleId,
    horizon_steps: u32,
) -> Result<(), String> {
    use kernel::payload::{sd_detail, sd_state, tm_t_end_step, STATE_COLLISION, STATE_ESCAPE};
    let mut k = *key;
    k.loop_values.horizon_steps = horizon_steps;
    let t_h = f64::from(horizon_steps) * k.loop_values.dt_macro;
    let m = measure::<A>(ic, &k, t_h, horizon_steps);
    let idx = RuleId::ALL.iter().position(|&r| r == rule).expect("a rule");
    let stopped = self::production::<A>(
        ic,
        &MarchKey64 {
            stop_on_escape: true,
            ..k
        },
        production,
    );
    let first = (sd_state(stopped.packed_a) == STATE_ESCAPE).then(|| {
        (
            f64::from(tm_t_end_step(stopped.times)) * k.loop_values.dt_macro,
            sd_detail(stopped.packed_a),
        )
    });
    let harness = m.commits[idx].map(|(t, c)| (t, c.detail));
    match (first, harness) {
        (None, None) => {}
        (Some((tp, dp)), Some((th, dh)))
            if dp == dh && (tp - th).abs() <= k.loop_values.dt_macro => {}
        (p, h) => {
            return Err(format!(
                "{}: under the stop option production ends in {p:?} (time, detail), the harness's first label is {h:?}",
                rule.name()
            ))
        }
    }
    let live = self::production::<A>(
        ic,
        &MarchKey64 {
            stop_on_escape: false,
            ..k
        },
        production,
    );
    let state = sd_state(live.packed_a);
    let label = (state == STATE_ESCAPE).then(|| sd_detail(live.packed_a));
    let collided = m.end == End::Censored && state == STATE_COLLISION;
    let last = m.labels[idx]
        .iter()
        .take_while(|(t, _)| *t <= t_h)
        .last()
        .and_then(|(_, l)| l.map(|c| c.detail));
    if collided || label == last {
        Ok(())
    } else {
        Err(format!(
            "{}: live, production's label at the horizon is {label:?} (state {state}), the harness's last label {last:?}",
            rule.name()
        ))
    }
}

/// The per-trajectory listing of `runs` of the population `name`: one tab-separated line each, pixel, truth (with
/// B-stable or unresolved), how the continued run ended and when, `E_0`, the last record's hierarchy ratio, its body,
/// least separation and `β`, then each rule's commit by `T_eval` (time, escaper, detail) and its verdict.
pub fn listing(name: &str, runs: &[Measured]) -> String {
    let mut out = String::new();
    for (i, r) in runs.iter().enumerate() {
        let truth = match r.truth {
            Truth::NoEscape if r.b_stable => "no-escape/B-stable".to_owned(),
            Truth::NoEscape => "no-escape/unresolved".to_owned(),
            t => format!("{t:?}"),
        };
        let rec = r.last_record.map_or("-\t-\t-\t-".to_owned(), |(e, b)| {
            let least = e.d.iter().copied().fold(f64::INFINITY, f64::min);
            format!(
                "{:.4e}\t{}\t{:.4e}\t{:.3},{:.3},{:.3}",
                e.ratio, e.body, least, b[0], b[1], b[2]
            )
        });
        out += &format!(
            "{name}\t{i}\t{truth}\t{:?}\t{:.2}\t{:.5}\t{rec}",
            r.end, r.t_end, r.e_0
        );
        for c in r.commits {
            out += &match c {
                Some((t, c)) => format!(
                    "\t{t:.2},{},{},{}",
                    c.escaper,
                    c.detail,
                    match judge(&c, r.truth) {
                        Some(true) => "correct",
                        Some(false) => "WRONG",
                        None => "unjudged",
                    }
                ),
                None => "\t-".to_owned(),
            };
        }
        out.push('\n');
    }
    out
}
