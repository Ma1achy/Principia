//! The escape label live by default, every rule re-scored with the body counted (escape-live-label):
//! `cargo xtask gate escape-rules --live [--n <n>] [--dump <path>]`.
//!
//! The owner, 23 Nov: "I wouldn't by default stop integration at the first label; I'd make that an option", and "don't
//! copy how prin-rs doesn't consider the body that collided or escaped". So the default label is the selected rule's
//! label at the playhead, read from the live state and the rule's own latches, free to change as the march goes on;
//! stopping at the first label, which then stays, is the option (`stop_on_escape`). Every rule is scored here in both
//! modes, side by side, from one march per trajectory ([`super::rules::RuleHarness`]): the live label is the rule's
//! label changes ([`super::rules::Measured::labels`]); the option's is the first of them, since under it production ends
//! there and the march up to that point is the same march.
//!
//! # The criteria, written before the run of record
//!
//! **The cases.**
//! - #49's three fixtures, 64 × 64 pixel centres each, at #49's collision radii (`config_stability` its own 0.005,
//!   `config_basin` and `deep_interior` 0), and the Burrau anchor (`r_coll` 0), with #49's evaluation horizon
//!   `T_eval = 3·t_max` (Burrau: 300 of Szebehely & Peters' units) and its continued-run truth to `10·T_eval`.
//! - **Periodic orbits that must never escape**, truth bound by construction ([`Truth::Bound`]): the figure-eight
//!   (`ground_truth::tiers::figure_eight`, the repo's anchor), and Broucke A7 and Broucke R3, the two Broucke orbits in
//!   the repo's records (the Belgrade gallery, `docs/incoming/2026-10-26_belgrade_gallery/belgrade_gallery.csv`; its
//!   terms: copy and use with credit to `http://three-body.ipb.ac.rs/` and Šuvakov & Dmitrašinović 2013). No Hénon
//!   orbit is in the repo: the gallery's Hénon pages were not archived. Each runs to `T_eval = 200` canonical. Its truth
//!   holds while the orbit stays on its path: until the first record (every 2 time units) whose greatest hierarchy
//!   ratio exceeds [`PERIODIC_RATIO`] times the greatest on its records to `t = `[`PERIODIC_EARLY`]; a playhead past that
//!   is unjudged, and the time is reported.
//! - **Stable hierarchical triples**, truth bound by construction: equal masses, an inner circular binary of separation
//!   1 and a third body on a circular orbit about it at `d = 5, 8, 12, 20` binary separations, prograde and retrograde
//!   (eight ICs, `G = 1`, unit masses, then to the canonical gauge). Each must be Mardling–Aarseth stable at `t = 0`
//!   (`kernel::chazy::ma_stable`); one that is not is left out and reported. `T_eval = 200` canonical.
//! - **A positive-energy slice**, where triple ejection occurs: the Burrau configuration in the canonical gauge (`I = 1`,
//!   `E = −U = −0.199` at rest), each body given `p_i = m_i (a r_i + b ẑ × r_i)`, `a` (expansion) over `[0.2, 1.0]`
//!   and `b` (rotation) over `[0.7, 1.1]`, 32 × 32 pixel centres, so `E = ½(a² + b²) − U > 0` at every pixel; `r_coll`
//!   0, `t_max` 50, so `T_eval` 150 and the truth to 1500.
//!
//! **The playheads**: `T` = 5, 10, 20, 50, 100, 200 and `T_eval`, canonical time (Burrau's in its own units). The
//! label at `T` is the rule's label at the last window mark at or before `T`, live; under the option, the first label
//! if it came at or before `T`, else none. A trajectory is judged at `T` where its truth is judged (not censored) and,
//! for a periodic orbit, its path still holds at `T`. A continued run that stopped as asymptotic before `T` holds its
//! last label, as the motion did. The number judged is reported at each `T`.
//!
//! **A label at `T` against the truth** (the body counts):
//! - truth an escape of body `b` (HE, PE): correct where the label is `b`'s escape; **wrong (body)** where it is
//!   another body's escape; **wrong (class)** where it is a triple ejection; **pending** where there is no label yet.
//! - truth a triple ejection (H, HP, P), which names no body: correct where the label is a triple ejection; wrong
//!   (class) where it is a single body's escape; pending where none.
//! - truth bound (B-stable, or bound by construction): correct where there is no label; **wrong (bound)** where there
//!   is one.
//! - truth unresolved (no escape and not B-stable by `T_truth`): a label is counted apart, **on unresolved**, neither
//!   right nor wrong.
//!
//! **Per trajectory, over `[0, T_eval]`**, each rule and each mode:
//! - **wrong commit**: any label shown that is wrong at its time, by the rules above (wrong body, wrong class or on a
//!   bound truth), each kind also counted apart; a label shown on an unresolved truth is counted apart;
//! - **time to the first correct label** (escape truths), p50 and p90;
//! - **time to a label that stays correct** (escape truths): the time of the last label change, where the label from
//!   then to the end of the continued run is correct, counted where that time is at most `T_eval`; p50 and p90, the
//!   count, and the escape truths with none (never);
//! - **flips after the first commit**: label changes after the first label, by `T_eval`, a change of body, of class,
//!   or back to no label each one flip; the total and the trajectories that flip.
//!   Under the option a label never changes, so its flips are 0 and its stays-correct time is its first label's.
//!
//! **The pick of the default rule** (live, the default mode): among the five rules, those with zero wrong commits
//! pooled over every case (the three fixtures, Burrau, the periodic orbits, the stable triples and the positive-energy
//! slice), wrong body included; of those, the smallest pooled median time to a label that stays correct, **an escape
//! truth with none by `T_eval` counted as `+∞`**; a tie (equal medians, both infinite included) to the most labels that
//! stay correct by `T_eval`. With none at zero wrong commits, the current default (closure) stays.
//!
//! **Amended before the run of record, after the 8 × 8 smoke run** (its harness check), and said so on the PR: the pick
//! first took the median over the trajectories that reached a staying label alone, which the smoke showed ranks a rule
//! whose label stays correct on 17 of 121 escape truths above one whose label does on 83 (survivorship, not the time to
//! a label that stays correct); and the periodic orbits' path first held while the ratio stayed at most 3, which Broucke
//! R3's own path exceeds (3.085). Both statistics and both picks are printed. Burrau, the periodic orbits and the triples
//! do not depend on the fixtures' size, so their numbers were seen in the smoke run before the run of record. Whether the pick puts Burrau in [50, 100] is not part of the rule; it is reported: the two Burrau
//! label tests are un-ignored only where the picked rule's first label (the option's label time) lies in [50, 100] of
//! Szebehely & Peters' units naming the lightest body, and its live label at `T_eval` is that same escape.
//!
//! **Fewbody's port** is measured, and validation's reference is read beside it at every mark of every trajectory; the
//! marks where they differ are reported (a measurement, not a gate).

use std::sync::OnceLock;

use kernel::canon::canonicalise;
use kernel::chazy::{ma_stable, Playhead};
use kernel::decode::Ic;
use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;

use engine::precision::MarchKey64;

use crate::ground_truth::{march_key, tiers, BENCHMARK_R_COLL};
use crate::import::{unit_state, Start};

use super::march::parallel;
use super::rules::{self, Commit, End, Measured, RuleId, Truth, TRUTH_FACTOR};
use super::GroundTruthOccupant;

/// The playheads, canonical time (Burrau's in Szebehely & Peters' units); `T_eval` is added per trajectory.
pub const PLAYHEADS: [f64; 6] = [5.0, 10.0, 20.0, 50.0, 100.0, 200.0];

/// The periodic orbits' and the stable triples' evaluation horizon, canonical time.
pub const BOUND_T_EVAL: f64 = 200.0;

/// A periodic orbit's path holds while every record's greatest hierarchy ratio stays at most this times the greatest on
/// its records to [`PERIODIC_EARLY`], its own early extent.
pub const PERIODIC_RATIO: f64 = 2.0;

/// The time to which a periodic orbit's records set its own extent, canonical: several periods of each of the three.
pub const PERIODIC_EARLY: f64 = 20.0;

/// The two modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The default: the label at the playhead, free to change.
    Live,
    /// The option: the first label stops the march and stays.
    Stop,
}

impl Mode {
    /// Both, live first.
    pub const BOTH: [Mode; 2] = [Mode::Live, Mode::Stop];

    /// Its name in the tables.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Live => "live",
            Mode::Stop => "stop",
        }
    }
}

/// One case's trajectory: its measurement, the time its truth holds until (a periodic orbit's path; infinite
/// otherwise), and its time unit in canonical time (Burrau's scale; 1 otherwise).
#[derive(Clone, Debug)]
pub struct Run {
    pub m: Measured,
    pub valid_until: f64,
    pub unit: f64,
}

/// A case: its name and its runs.
#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    pub runs: Vec<Run>,
    /// What the case reports beside its tables (the orbits' path times, the triples' Mardling–Aarseth checks).
    pub notes: Vec<String>,
}

/// The label of rule `k` at time `t` (canonical) in `mode` (the module's note).
pub fn label_at(m: &Measured, k: usize, mode: Mode, t: f64) -> Option<Commit> {
    match mode {
        Mode::Live => m.labels[k]
            .iter()
            .take_while(|(s, _)| *s <= t)
            .last()
            .and_then(|(_, l)| *l),
        Mode::Stop => first_label(m, k).filter(|(s, _)| *s <= t).map(|(_, c)| c),
    }
}

/// Rule `k`'s first label over the whole run, with its time.
pub fn first_label(m: &Measured, k: usize) -> Option<(f64, Commit)> {
    m.labels[k].iter().find_map(|(t, l)| l.map(|c| (*t, c)))
}

/// A label against a truth (the module's note).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judged {
    Correct,
    WrongBody,
    WrongClass,
    WrongBound,
    Pending,
    OnUnresolved,
    /// No label on an unresolved truth.
    Quiet,
    Unjudged,
}

impl Judged {
    /// Whether it is wrong.
    pub fn wrong(self) -> bool {
        matches!(
            self,
            Judged::WrongBody | Judged::WrongClass | Judged::WrongBound
        )
    }
}

/// The label `label` against `m`'s truth (the module's note).
pub fn judge(m: &Measured, label: Option<Commit>) -> Judged {
    let triple = |c: Commit| c.detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;
    match (m.truth, label) {
        (Truth::Censored, _) => Judged::Unjudged,
        (Truth::Bound, None) => Judged::Correct,
        (Truth::Bound, Some(_)) => Judged::WrongBound,
        (Truth::NoEscape, None) if m.b_stable => Judged::Correct,
        (Truth::NoEscape, Some(_)) if m.b_stable => Judged::WrongBound,
        (Truth::NoEscape, None) => Judged::Quiet,
        (Truth::NoEscape, Some(_)) => Judged::OnUnresolved,
        (Truth::Dispersal | Truth::Escape(_), None) => Judged::Pending,
        (Truth::Dispersal, Some(c)) if triple(c) => Judged::Correct,
        (Truth::Dispersal, Some(_)) => Judged::WrongClass,
        (Truth::Escape(_), Some(c)) if triple(c) => Judged::WrongClass,
        (Truth::Escape(b), Some(c)) if c.escaper == b && c.detail == b => Judged::Correct,
        (Truth::Escape(_), Some(_)) => Judged::WrongBody,
    }
}

/// Whether `run` is judged at `t` (canonical): its truth judged and, for a periodic orbit, its path still holding.
fn judged_at(run: &Run, t: f64) -> bool {
    run.m.truth != Truth::Censored && t <= run.valid_until
}

/// The counts at one playhead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtT {
    pub n: usize,
    pub correct: usize,
    pub wrong_body: usize,
    pub wrong_class: usize,
    pub wrong_bound: usize,
    pub pending: usize,
    pub on_unresolved: usize,
}

impl AtT {
    /// The wrong labels.
    pub fn wrong(&self) -> usize {
        self.wrong_body + self.wrong_class + self.wrong_bound
    }

    fn add(&mut self, o: &AtT) {
        self.n += o.n;
        self.correct += o.correct;
        self.wrong_body += o.wrong_body;
        self.wrong_class += o.wrong_class;
        self.wrong_bound += o.wrong_bound;
        self.pending += o.pending;
        self.on_unresolved += o.on_unresolved;
    }

    fn count(&mut self, j: Judged) {
        if j == Judged::Unjudged {
            return;
        }
        self.n += 1;
        match j {
            Judged::Correct => self.correct += 1,
            Judged::WrongBody => self.wrong_body += 1,
            Judged::WrongClass => self.wrong_class += 1,
            Judged::WrongBound => self.wrong_bound += 1,
            Judged::Pending => self.pending += 1,
            Judged::OnUnresolved => self.on_unresolved += 1,
            Judged::Quiet | Judged::Unjudged => {}
        }
    }
}

/// One rule's score in one mode over a set of runs (the module's note).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Score {
    /// At each of [`PLAYHEADS`], then at `T_eval`.
    pub at: [AtT; 7],
    /// Trajectories with a wrong label shown by `T_eval`, and of them those showing each kind.
    pub wrong: usize,
    pub wrong_body: usize,
    pub wrong_class: usize,
    pub wrong_bound: usize,
    /// Trajectories with an unresolved truth that showed a label by `T_eval`, and those with an unresolved truth.
    pub on_unresolved: usize,
    pub unresolved: usize,
    /// The times of the first correct label, by `T_eval`.
    pub first_correct: Vec<f64>,
    /// The times of a label that stays correct, by `T_eval`.
    pub stays: Vec<f64>,
    /// Escape truths judged, and those with no label that stays correct by `T_eval`.
    pub escapes: usize,
    pub never: usize,
    /// Label changes after the first label by `T_eval`, and the trajectories with one or more.
    pub flips: usize,
    pub flipped: usize,
    /// Censored trajectories (a collision, a failure) that showed a label by `T_eval`: unjudged. Under the stop option
    /// their sample is that escape; live, the collision takes it.
    pub unjudged: usize,
}

/// The quantile `q` of `v`, `None` where empty.
pub fn quantile(v: &[f64], q: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    Some(v[((v.len() - 1) as f64 * q).round() as usize])
}

impl Score {
    /// Adds `o` in.
    pub fn add(&mut self, o: &Score) {
        for (a, b) in self.at.iter_mut().zip(&o.at) {
            a.add(b);
        }
        self.wrong += o.wrong;
        self.wrong_body += o.wrong_body;
        self.wrong_class += o.wrong_class;
        self.wrong_bound += o.wrong_bound;
        self.on_unresolved += o.on_unresolved;
        self.unresolved += o.unresolved;
        self.first_correct.extend_from_slice(&o.first_correct);
        self.stays.extend_from_slice(&o.stays);
        self.escapes += o.escapes;
        self.never += o.never;
        self.flips += o.flips;
        self.flipped += o.flipped;
        self.unjudged += o.unjudged;
    }

    /// The pick's statistic: the median time to a label that stays correct over every escape truth, one with none by
    /// `T_eval` counted as `+∞`.
    pub fn stays_median(&self) -> f64 {
        let mut v = self.stays.clone();
        v.extend(std::iter::repeat_n(f64::INFINITY, self.never));
        quantile(&v, 0.5).unwrap_or(f64::INFINITY)
    }
}

/// The labels rule `k` shows in `mode` over `[0, t_eval]` (canonical), each with its time: live, every change; under
/// the option, the first label alone.
fn shown(m: &Measured, k: usize, mode: Mode, t_eval: f64) -> Vec<(f64, Option<Commit>)> {
    match mode {
        Mode::Live => m.labels[k]
            .iter()
            .copied()
            .take_while(|(t, _)| *t <= t_eval)
            .collect(),
        Mode::Stop => first_label(m, k)
            .filter(|(t, _)| *t <= t_eval)
            .map(|(t, c)| vec![(t, Some(c))])
            .unwrap_or_default(),
    }
}

/// Rule `k`'s score in `mode` over `runs` (the module's note).
pub fn score(runs: &[Run], k: usize, mode: Mode) -> Score {
    let mut s = Score::default();
    for run in runs {
        let m = &run.m;
        let t_eval = m.t_eval;
        for (i, &t) in PLAYHEADS.iter().chain(std::iter::once(&t_eval)).enumerate() {
            let tc = if i < PLAYHEADS.len() { t * run.unit } else { t };
            if judged_at(run, tc) {
                s.at[i].count(judge(m, label_at(m, k, mode, tc)));
            }
        }
        if m.truth == Truth::Censored {
            s.unjudged += usize::from(shown(m, k, mode, t_eval).iter().any(|(_, l)| l.is_some()));
            continue;
        }
        let horizon = t_eval.min(run.valid_until);
        let labels = shown(m, k, mode, horizon);
        let kinds: Vec<Judged> = labels.iter().map(|(_, l)| judge(m, *l)).collect();
        if kinds.iter().any(|j| j.wrong()) {
            s.wrong += 1;
        }
        s.wrong_body += usize::from(kinds.contains(&Judged::WrongBody));
        s.wrong_class += usize::from(kinds.contains(&Judged::WrongClass));
        s.wrong_bound += usize::from(kinds.contains(&Judged::WrongBound));
        if m.truth == Truth::NoEscape && !m.b_stable {
            s.unresolved += 1;
            s.on_unresolved += usize::from(kinds.contains(&Judged::OnUnresolved));
        }
        if matches!(m.truth, Truth::Escape(_) | Truth::Dispersal) {
            s.escapes += 1;
            if let Some((t, _)) = labels
                .iter()
                .zip(&kinds)
                .find(|(_, j)| **j == Judged::Correct)
                .map(|(l, _)| l)
            {
                s.first_correct.push(*t);
            }
            let stays = match mode {
                Mode::Live => m.labels[k]
                    .last()
                    .filter(|(t, l)| *t <= t_eval && judge(m, *l) == Judged::Correct)
                    .map(|(t, _)| *t),
                Mode::Stop => first_label(m, k)
                    .filter(|(t, c)| *t <= t_eval && judge(m, Some(*c)) == Judged::Correct)
                    .map(|(t, _)| t),
            };
            match stays {
                Some(t) => s.stays.push(t),
                None => s.never += 1,
            }
        }
        let first = labels.iter().position(|(_, l)| l.is_some());
        let flips = first.map_or(0, |f| labels.len() - f - 1);
        s.flips += flips;
        s.flipped += usize::from(flips > 0);
    }
    s
}

/// A score's rows: the playheads, then the per-trajectory figures.
pub fn rows(name: &str, mode: Mode, s: &Score) -> String {
    let q = |v: &[f64], x: f64| quantile(v, x).map_or("-".into(), |x| format!("{x:.2}"));
    let mut out = format!(
        "  {name:<15} {:<4} wrong commits {:>4} (body {:>4}, class {:>4}, on bound {:>4}); on unresolved {:>4} of {:>4}; \
         first correct p50 {:>7} p90 {:>7}; stays correct {:>5} p50 {:>7} p90 {:>7}, never {:>5} of {:>5}, median with never as ∞ {:>7}; \
         flips {:>5} on {:>5}; unjudged (censored) labelled {:>5}\n",
        mode.name(),
        s.wrong,
        s.wrong_body,
        s.wrong_class,
        s.wrong_bound,
        s.on_unresolved,
        s.unresolved,
        q(&s.first_correct, 0.5),
        q(&s.first_correct, 0.9),
        s.stays.len(),
        q(&s.stays, 0.5),
        q(&s.stays, 0.9),
        s.never,
        s.escapes,
        format!("{:.2}", s.stays_median()),
        s.flips,
        s.flipped,
        s.unjudged,
    );
    for (i, a) in s.at.iter().enumerate() {
        let t = PLAYHEADS
            .get(i)
            .map_or("T_eval".to_owned(), |t| format!("T={t}"));
        out += &format!(
            "      {t:<7} n {:>5}  correct {:>5}  wrong {:>4} (body {:>4}, class {:>4}, bound {:>4})  pending {:>5}  on unresolved {:>4}\n",
            a.n, a.correct, a.wrong(), a.wrong_body, a.wrong_class, a.wrong_bound, a.pending, a.on_unresolved
        );
    }
    out
}

// ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// A start in its paper's units to the canonical gauge (`C`), as the IC constructor takes a built state
/// (`engine::precision::built_ic`).
fn canonical(start: &Start) -> Result<Ic<f64>, String> {
    Ok(canonicalise(&unit_state(start)?).ic)
}

/// Broucke A7 (the Belgrade gallery's `sol.php`; Broucke 1975), `G = 1`, unit masses.
pub fn broucke_a7() -> Start {
    Start {
        name: "broucke_a7".into(),
        citation: "Broucke 1975, via http://three-body.ipb.ac.rs/ (Šuvakov & Dmitrašinović 2013, PRL 110, 114301)"
            .into(),
        g: 1.0,
        m: [1.0; 3],
        r: [[-1.0, 0.0], [0.5, -0.647584], [0.5, 0.647584]],
        v: [[0.0, -0.939325], [-0.505328, 0.4696663], [0.505328, 0.4696663]],
    }
}

/// Broucke R3 (the Belgrade gallery's `bsol.php?id=18`; Broucke 1975), `G = 1`, unit masses.
pub fn broucke_r3() -> Start {
    Start {
        name: "broucke_r3".into(),
        citation: "Broucke 1975, via http://three-body.ipb.ac.rs/ (Šuvakov & Dmitrašinović 2013, PRL 110, 114301)"
            .into(),
        g: 1.0,
        m: [1.0; 3],
        r: [[0.8920281421, 0.0], [-0.6628498947, 0.0], [-0.2291782474, 0.0]],
        v: [[0.0, 0.9957939373], [0.0, -1.6191613336], [0.0, 0.6233673964]],
    }
}

/// A stable hierarchical triple: unit masses, bodies 0 and 1 on a circular orbit of separation 1, body 2 on a circular
/// orbit about their centre of mass at distance `d`, prograde or retrograde; `G = 1`.
pub fn hierarchical(d: f64, prograde: bool) -> Start {
    let w = (2.0f64).sqrt() / 2.0;
    let u = (3.0 / d).sqrt() * if prograde { 1.0 } else { -1.0 };
    Start {
        name: format!("triple_d{d}_{}", if prograde { "pro" } else { "retro" }),
        citation: "constructed".into(),
        g: 1.0,
        m: [1.0; 3],
        r: [[-0.5, 0.0], [0.5, 0.0], [0.0, d]],
        v: [[0.0, -w], [0.0, w], [-u, 0.0]],
    }
}

/// The run of `ic` with its truth bound by construction, to [`BOUND_T_EVAL`] and no further, at `r_coll` 0.
fn bound_run(ic: &Ic<f64>) -> Result<Measured, String> {
    let key = march_key(BOUND_T_EVAL, BENCHMARK_R_COLL).map_err(|e| e.0)?;
    let steps = (BOUND_T_EVAL / key.loop_values.dt_macro).round() as u32;
    let mut m = rules::measure::<GroundTruthOccupant>(ic, &key, BOUND_T_EVAL, steps);
    if m.end != End::Censored {
        m.truth = Truth::Bound;
    }
    Ok(m)
}

/// The periodic orbits (the module's note): each with the time its path held until.
pub fn periodic() -> Result<Case, String> {
    let starts = [tiers::figure_eight(), broucke_a7(), broucke_r3()];
    let measured = parallel(starts.len(), |i| {
        let ic = canonical(&starts[i])?;
        bound_run(&ic)
    })?;
    let mut notes = Vec::new();
    let runs = measured
        .into_iter()
        .zip(&starts)
        .map(|(m, s)| {
            let early = m
                .series
                .iter()
                .take_while(|r| r.t <= PERIODIC_EARLY)
                .map(|r| r.ratio)
                .fold(0.0, f64::max);
            let left = m
                .series
                .iter()
                .find(|r| r.ratio > PERIODIC_RATIO * early)
                .map(|r| r.t);
            let most = m.series.iter().map(|r| r.ratio).fold(0.0, f64::max);
            notes.push(format!(
                "{}: E_0 {:.5}, end {:?} at {:.1}; greatest hierarchy ratio on the record {most:.3}, to t = {PERIODIC_EARLY} \
                 {early:.3}; path held {}",
                s.name,
                m.e_0,
                m.end,
                m.t_end,
                left.map_or("to T_eval".into(), |t| format!("until {t:.1}"))
            ));
            Run {
                valid_until: left.unwrap_or(f64::INFINITY),
                m,
                unit: 1.0,
            }
        })
        .collect();
    Ok(Case {
        name: "periodic".into(),
        runs,
        notes,
    })
}

/// The stable hierarchical triples (the module's note), those Mardling–Aarseth stable at `t = 0`.
pub fn triples() -> Result<Case, String> {
    let mut notes = Vec::new();
    let mut ics = Vec::new();
    for d in [5.0, 8.0, 12.0, 20.0] {
        for pro in [true, false] {
            let s = hierarchical(d, pro);
            let ic = canonical(&s)?;
            let ph = Playhead::from_momenta(ic.m, ic.r, ic.p, 0.0, 1.0, 1.0);
            // Body 2 is the outer body; the canonical gauge relabels nothing.
            let stable = ma_stable(&ph, 0, 1, 2);
            notes.push(format!(
                "{}: Mardling–Aarseth stable at t = 0: {stable}",
                s.name
            ));
            if stable {
                ics.push(ic);
            }
        }
    }
    let runs = parallel(ics.len(), |i| bound_run(&ics[i]))?
        .into_iter()
        .map(|m| Run {
            m,
            valid_until: f64::INFINITY,
            unit: 1.0,
        })
        .collect();
    Ok(Case {
        name: "stable triples".into(),
        runs,
        notes,
    })
}

/// The positive-energy slice's ICs at `n × n` pixel centres (the module's note), with each pixel's `(a, b)`.
pub fn positive_energy_ics(n: usize) -> Result<Vec<Ic<f64>>, String> {
    let base = canonical(&tiers::burrau())?;
    let mut out = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let a = 0.2 + 0.8 * (i as f64 + 0.5) / n as f64;
            let b = 0.7 + 0.4 * (j as f64 + 0.5) / n as f64;
            let p = std::array::from_fn(|k| {
                let [x, y] = base.r[k];
                [base.m[k] * (a * x - b * y), base.m[k] * (a * y + b * x)]
            });
            out.push(Ic { p, ..base });
        }
    }
    Ok(out)
}

/// The positive-energy slice (the module's note), `n × n`.
pub fn positive_energy(n: usize) -> Result<Case, String> {
    let ics = positive_energy_ics(n)?;
    let key = march_key(50.0, BENCHMARK_R_COLL).map_err(|e| e.0)?;
    let t_eval = 3.0 * 50.0;
    let steps = (TRUTH_FACTOR * t_eval / key.loop_values.dt_macro).round() as u32;
    let runs = parallel(ics.len(), |i| {
        Ok::<_, String>(Run {
            m: rules::measure::<GroundTruthOccupant>(&ics[i], &key, t_eval, steps),
            valid_until: f64::INFINITY,
            unit: 1.0,
        })
    })?;
    let positive = runs.iter().filter(|r| r.m.e_0 > 0.0).count();
    Ok(Case {
        name: format!("positive energy ({n} × {n})"),
        notes: vec![format!("E_0 > 0 on {positive} of {} pixels", runs.len())],
        runs,
    })
}

/// #49's fixtures at `n × n`, each a case.
pub fn fixtures(n: usize) -> Result<Vec<Case>, String> {
    super::rules_gate::fixtures()
        .into_iter()
        .map(|mut slice| {
            slice.n = n;
            let runs = super::rules_gate::measured(&slice)?.as_ref().clone();
            Ok(Case {
                name: format!("{} ({n} × {n})", slice.slice),
                runs: runs
                    .into_iter()
                    .map(|m| Run {
                        m,
                        valid_until: f64::INFINITY,
                        unit: 1.0,
                    })
                    .collect(),
                notes: Vec::new(),
            })
        })
        .collect()
}

/// The Burrau anchor as #49 measures it (`super::rules_gate::burrau`), its playheads in Szebehely & Peters' units.
pub fn burrau() -> Result<Case, String> {
    let (m, scale) = super::rules_gate::burrau()?;
    let mut notes = Vec::new();
    for (k, rule) in RuleId::ALL.iter().enumerate() {
        let first = first_label(m, k);
        let last = label_at(m, k, Mode::Live, m.t_eval);
        notes.push(format!(
            "{:<15} first label {:?} (S&P units, escaper, detail): {:?}; live label at T_eval {:?}; label changes {} \
             (Burrau's rows below: playheads in S&P units, times in canonical, scale {scale})",
            rule.name(),
            first.map(|(t, c)| (format!("{:.2}", t / scale), c.escaper, c.detail)),
            first.map(|(_, c)| judge(m, Some(c))),
            last.map(|c| (c.escaper, c.detail)),
            m.labels[k].len()
        ));
    }
    Ok(Case {
        name: format!("Burrau (truth {:?})", m.truth),
        runs: vec![Run {
            m: m.clone(),
            valid_until: f64::INFINITY,
            unit: *scale,
        }],
        notes,
    })
}

/// Every case at fixture size `n` (64 for the run of record), once per process.
pub fn cases(n: usize) -> Result<&'static Vec<Case>, String> {
    static ONCE: OnceLock<Result<Vec<Case>, String>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let mut out = fixtures(n)?;
        out.push(burrau()?);
        out.push(periodic()?);
        out.push(triples()?);
        out.push(positive_energy(n / 2)?);
        Ok(out)
    })
    .as_ref()
    .map_err(Clone::clone)
}

/// `--live`: every case's tables in both modes, the pooled tables, the Fewbody cross-check and the pick (the module's
/// note).
pub fn report(n: usize) -> Result<String, String> {
    let cases = cases(n)?;
    let mut out = String::from(
        "The escape rules, live (the default) and under the stop option, at the playheads, the body counted (criteria in \
         validation::escape_checks::live, written before the run):\n",
    );
    let mut pooled: Vec<[Score; 2]> = vec![Default::default(); RuleId::ALL.len()];
    let (mut marks, mut differ) = (0u64, 0u64);
    for case in cases {
        let ms: Vec<Measured> = case.runs.iter().map(|r| r.m.clone()).collect();
        out += &format!("\n{}: {}\n", case.name, rules::census(&ms));
        for note in &case.notes {
            out += &format!("  {note}\n");
        }
        for (k, rule) in RuleId::ALL.iter().enumerate() {
            for (j, mode) in Mode::BOTH.iter().enumerate() {
                let s = score(&case.runs, k, *mode);
                out += &rows(rule.name(), *mode, &s);
                pooled[k][j].add(&s);
            }
        }
        for r in &case.runs {
            marks += u64::from(r.m.fewbody_check.0);
            differ += u64::from(r.m.fewbody_check.1);
        }
    }
    out += "\nPooled over every case:\n";
    for (k, rule) in RuleId::ALL.iter().enumerate() {
        for (j, mode) in Mode::BOTH.iter().enumerate() {
            out += &rows(rule.name(), *mode, &pooled[k][j]);
        }
    }
    out += &format!(
        "\nFewbody's kernel port against validation's reference: {differ} of {marks} marks differ.\n"
    );
    let live: Vec<Score> = pooled.iter().map(|p| p[0].clone()).collect();
    let name = |p: Option<RuleId>| match p {
        Some(r) => r.name().to_owned(),
        None => format!(
            "none has zero wrong commits, so {} stays",
            RuleId::DEFAULT.name()
        ),
    };
    out += &format!(
        "\nThe pick (live, zero wrong commits pooled, then the least median time to a label that stays correct, none by \
         T_eval counted as +∞): {}.\nThe pick by the first-written statistic (the median over the labels that stay \
         correct alone; see the amendment): {}.\n",
        name(pick(&live, Score::stays_median)),
        name(pick(&live, |s| quantile(&s.stays, 0.5).unwrap_or(f64::INFINITY)))
    );
    Ok(out)
}

/// The pick from each rule's pooled live score by the statistic `stat` (the module's note): a tie where the two are
/// equal, both infinite included, or within 10⁻⁹.
pub fn pick(live: &[Score], stat: impl Fn(&Score) -> f64) -> Option<RuleId> {
    RuleId::ALL
        .iter()
        .enumerate()
        .filter(|(k, _)| live[*k].wrong == 0)
        .min_by(|(a, _), (b, _)| {
            let (x, y) = (stat(&live[*a]), stat(&live[*b]));
            if x == y || (x - y).abs() <= 1e-9 {
                live[*b].stays.len().cmp(&live[*a].stays.len())
            } else {
                x.total_cmp(&y)
            }
        })
        .map(|(_, r)| *r)
}

/// `--dump <path>`: one line per trajectory and rule, every label change by the end of the continued run (time,
/// detail; `-` for no label), with the truth.
pub fn dump(n: usize) -> Result<String, String> {
    let mut out = String::from(
        "# case\ttrajectory\ttruth\tb_stable\tend\tt_end\tvalid_until\tunit\trule\tlabels\n",
    );
    for case in cases(n)? {
        for (i, r) in case.runs.iter().enumerate() {
            for (k, rule) in RuleId::ALL.iter().enumerate() {
                let labels: Vec<String> = r.m.labels[k]
                    .iter()
                    .map(|(t, l)| {
                        format!(
                            "{t:.2}:{}",
                            l.map_or("-".to_owned(), |c| c.detail.to_string())
                        )
                    })
                    .collect();
                out += &format!(
                    "{}\t{i}\t{:?}\t{}\t{:?}\t{:.2}\t{}\t{}\t{}\t{}\n",
                    case.name,
                    r.m.truth,
                    r.m.b_stable,
                    r.m.end,
                    r.m.t_end,
                    r.valid_until,
                    r.unit,
                    rule.name(),
                    labels.join(" ")
                );
            }
        }
    }
    Ok(out)
}

/// The key a case's IC marches under, for a test: `t_max` canonical at `r_coll` 0.
pub fn key(t_max: f64) -> Result<MarchKey64, String> {
    march_key(t_max, BENCHMARK_R_COLL).map_err(|e| e.0)
}
