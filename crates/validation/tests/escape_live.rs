//! The escape label live by default and under the stop option, scored by `escape_checks::live` (the pass criteria are
//! written here, before the run):
//! - `escape_live_judge`: a label against a truth. Escape of body 1: label body 1 correct, body 0 wrong (body), triple
//!   ejection wrong (class), none pending. Dispersal: triple ejection correct, a body wrong (class). Bound by
//!   construction: no label correct, any label wrong (bound). Censored: unjudged. Control: the wrong body named correct.
//! - `escape_live_modes`: a rule labelling body 0 at t = 10, body 1 at 20 and nothing at 30 reads body 1 at t = 25
//!   live and body 0 under the option, nothing before 10 in both, and under the option never changes. Control: the
//!   option's label read as live.
//! - `escape_live_score`: on that trajectory against an escape of body 1 live: one wrong commit (body), 2 flips, no
//!   label that stays correct; under the option: one wrong commit, no flips. Against an escape of body 0 under the
//!   option the label stays correct at t = 10. A periodic orbit's path ending at t = 15 leaves its playheads past 15
//!   unjudged. Control: the option's flips counted as live.
//! - `escape_live_ties`: equal medians, and medians within 1e-9, go to the rule with more stays, the later in table order
//!   (so a missing tie-break fails); medians 1e-6 apart do not tie. Control: the near tie's rule with no more stays.
//! - `escape_live_stays`, `escape_live_horizon`, `escape_live_inf_tie`, `escape_live_never_is_inf`: under the option a first
//!   label stays correct at its time, live a final none label does not; a wrong label after a periodic orbit's path end
//!   is not a wrong commit; equal infinite medians go to the rule with more stays; a trajectory with no staying label
//!   counts as +inf in the median. Each has a control.
//! - `escape_live_pick`: among rules with zero wrong commits, the least median time to a label that stays correct, an
//!   escape truth with none counted as +inf; a tie to the most stays; none at zero wrong keeps `None`. Control: the
//!   fastest rule with a wrong commit allowed.

use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION as TRIPLE;
use validation::escape_checks::live::{
    judge, label_at, pick, score, Judged, Mode, Run, Score, PLAYHEADS,
};
use validation::escape_checks::rules::{Commit, End, Measured, RuleId, Truth};
use validation::negative_control;

fn commit(detail: u32) -> Commit {
    Commit {
        index: 0,
        escaper: if detail == TRIPLE { 0 } else { detail },
        detail,
    }
}

fn measured(truth: Truth, labels: Vec<(f64, Option<Commit>)>) -> Measured {
    let mut ls: [Vec<(f64, Option<Commit>)>; 5] = Default::default();
    ls[0] = labels;
    Measured {
        truth,
        commits: [None; 5],
        t_eval: 150.0,
        e_0: 0.1,
        end: End::Asymptotic,
        t_end: 150.0,
        b_stable: false,
        last_record: None,
        labels: ls,
        fewbody_check: (0, 0),
        series: Vec::new(),
    }
}

fn run(m: Measured, valid_until: f64) -> Run {
    Run {
        m,
        valid_until,
        unit: 1.0,
    }
}

fn flicker() -> Vec<(f64, Option<Commit>)> {
    vec![
        (10.0, Some(commit(0))),
        (20.0, Some(commit(1))),
        (30.0, None),
    ]
}

fn check_judge(wrong_body_is: Judged) {
    let esc1 = measured(Truth::Escape(1), vec![]);
    assert_eq!(judge(&esc1, Some(commit(1))), Judged::Correct);
    assert_eq!(judge(&esc1, Some(commit(0))), wrong_body_is, "wrong body");
    assert_eq!(judge(&esc1, Some(commit(TRIPLE))), Judged::WrongClass);
    assert_eq!(judge(&esc1, None), Judged::Pending);
    let disp = measured(Truth::Dispersal, vec![]);
    assert_eq!(judge(&disp, Some(commit(TRIPLE))), Judged::Correct);
    assert_eq!(judge(&disp, Some(commit(2))), Judged::WrongClass);
    let bound = measured(Truth::Bound, vec![]);
    assert_eq!(judge(&bound, None), Judged::Correct);
    assert_eq!(judge(&bound, Some(commit(2))), Judged::WrongBound);
    assert_eq!(judge(&disp, None), Judged::Pending);
    let named_other = Commit {
        index: 0,
        escaper: 1,
        detail: 0,
    };
    assert_eq!(judge(&esc1, Some(named_other)), wrong_body_is, "wrong body");
    let mut stable = measured(Truth::NoEscape, vec![]);
    stable.b_stable = true;
    assert_eq!(judge(&stable, None), Judged::Correct);
    assert_eq!(judge(&stable, Some(commit(2))), Judged::WrongBound);
    let unresolved = measured(Truth::NoEscape, vec![]);
    assert_eq!(judge(&unresolved, None), Judged::Quiet);
    assert_eq!(judge(&unresolved, Some(commit(2))), Judged::OnUnresolved);
    let cens = measured(Truth::Censored, vec![]);
    assert_eq!(judge(&cens, Some(commit(2))), Judged::Unjudged);
}

#[test]
fn escape_live_judge() {
    check_judge(Judged::WrongBody);
}

negative_control!(
    escape_live_judge,
    "the wrong body named correct",
    expected = "wrong body",
    check_judge(Judged::Correct)
);

fn check_modes(stop_reads: Mode) {
    let m = measured(Truth::Escape(1), flicker());
    assert_eq!(label_at(&m, 0, Mode::Live, 25.0), Some(commit(1)));
    assert_eq!(label_at(&m, 0, stop_reads, 25.0), Some(commit(0)), "stop");
    assert_eq!(label_at(&m, 0, Mode::Live, 5.0), None);
    assert_eq!(label_at(&m, 0, stop_reads, 5.0), None);
    assert_eq!(label_at(&m, 0, Mode::Live, 35.0), None);
    assert_eq!(label_at(&m, 0, stop_reads, 35.0), Some(commit(0)), "stop");
}

#[test]
fn escape_live_modes() {
    check_modes(Mode::Stop);
}

negative_control!(
    escape_live_modes,
    "the option's label read as live",
    expected = "stop",
    check_modes(Mode::Live)
);

fn check_score(stop_mode: Mode, orbit_valid_until: f64) {
    let wrong = [run(measured(Truth::Escape(1), flicker()), f64::INFINITY)];
    let live = score(&wrong, 0, Mode::Live);
    assert_eq!(
        (live.wrong, live.wrong_body, live.flips, live.flipped),
        (1, 1, 2, 1)
    );
    assert_eq!((live.stays.len(), live.never, live.escapes), (0, 1, 1));
    assert_eq!(live.first_correct, vec![20.0]);
    let stop = score(&wrong, 0, stop_mode);
    assert_eq!((stop.wrong, stop.flips), (1, 0), "stop flips");

    // The path of a periodic orbit ends at t = 15: playheads 20 and later are unjudged.
    let orbit = [run(measured(Truth::Bound, vec![]), orbit_valid_until)];
    let s = score(&orbit, 0, Mode::Live);
    let judged: Vec<usize> = s.at.iter().map(|a| a.n).collect();
    assert_eq!(PLAYHEADS[..2], [5.0, 10.0]);
    assert_eq!(judged, vec![1, 1, 0, 0, 0, 0, 0], "path end");
}

fn check_stays(mode: Mode) {
    let right = [run(measured(Truth::Escape(0), flicker()), f64::INFINITY)];
    let s = score(&right, 0, mode);
    assert_eq!(s.stays, vec![10.0], "stays");
}

#[test]
fn escape_live_stays() {
    // Under the option the first label (body 0, t = 10) stays correct. Live, the last label (t = 30) is none, so none does.
    check_stays(Mode::Stop);
    assert_eq!(
        score(
            &[run(measured(Truth::Escape(0), flicker()), f64::INFINITY)],
            0,
            Mode::Live
        )
        .never,
        1
    );
}

negative_control!(
    escape_live_stays,
    "the live label read as staying correct",
    expected = "stays",
    check_stays(Mode::Live)
);

/// A wrong label past the path's end is not a wrong commit; before it, it is.
fn check_horizon(valid_until: f64) {
    let labels = vec![(10.0, Some(commit(1))), (30.0, Some(commit(0)))];
    let r = [run(measured(Truth::Escape(1), labels), valid_until)];
    assert_eq!(score(&r, 0, Mode::Live).wrong, 0, "path horizon");
}

#[test]
fn escape_live_horizon() {
    check_horizon(20.0);
}

negative_control!(
    escape_live_horizon,
    "the path never ending",
    expected = "path horizon",
    check_horizon(f64::INFINITY)
);

#[test]
fn escape_live_score() {
    check_score(Mode::Stop, 15.0);
}

negative_control!(
    escape_live_score,
    "the option's flips counted as live",
    expected = "stop flips",
    check_score(Mode::Live, 15.0)
);

#[test]
fn escape_live_path_end() {
    check_score(Mode::Stop, 15.0);
}

negative_control!(
    escape_live_path_end,
    "the orbit's path never ending",
    expected = "path end",
    check_score(Mode::Stop, f64::INFINITY)
);

fn rule_score(stays: &[f64], never: usize, wrong: usize) -> Score {
    Score {
        stays: stays.to_vec(),
        never,
        escapes: stays.len() + never,
        wrong,
        ..Score::default()
    }
}

fn check_pick(fast_wrong: usize) {
    let mut live = vec![Score::default(); 5];
    // Closure (index 0): fastest, but a wrong commit.
    live[0] = rule_score(&[1.0, 1.0, 1.0], 0, fast_wrong);
    // Energy floor: median +inf (never on most).
    live[1] = rule_score(&[2.0], 2, 0);
    // Distance floor: median 50.
    live[2] = rule_score(&[40.0, 50.0, 60.0], 0, 0);
    // Chazy: a wrong commit.
    live[3] = rule_score(&[5.0, 5.0, 5.0], 0, 2);
    // Fewbody: median 30.
    live[4] = rule_score(&[20.0, 30.0, 70.0], 0, 0);
    assert_eq!(
        pick(&live, Score::stays_median),
        Some(RuleId::Fewbody),
        "zero wrong"
    );
    // No rule at zero wrong commits: no pick (the current default stays).
    let all_wrong = vec![rule_score(&[1.0], 0, 1); 5];
    assert_eq!(pick(&all_wrong, Score::stays_median), None);
}

/// Closure (table order first) has 3 stays at median 30. Fewbody (table order last) has `fewbody_stays` stays at a median
/// of `30 + gap`. Rules between are out on wrong commits. A tie goes to the rule with more stays; `min_by` alone would
/// return Closure, the first.
fn check_tie(fewbody_stays: usize, gap: f64, expect: RuleId, what: &str) {
    let mut live = vec![rule_score(&[1.0], 0, 1); 5];
    live[0] = rule_score(&[30.0, 30.0, 30.0], 0, 0);
    live[4] = rule_score(&vec![30.0 + gap; fewbody_stays], 0, 0);
    assert_eq!(pick(&live, Score::stays_median), Some(expect), "{what}");
}

/// Equal infinite medians tie too (`inf - inf` is NaN, so the tolerance cannot decide it): Closure 1 stay, 3 never;
/// Fewbody `n` stays, 3 never.
fn check_inf_tie(fewbody_stays: usize) {
    let mut live = vec![rule_score(&[1.0], 0, 1); 5];
    live[0] = rule_score(&[5.0], 3, 0);
    live[4] = rule_score(&vec![5.0; fewbody_stays], 3, 0);
    assert!(live[0].stays_median().is_infinite() && live[4].stays_median().is_infinite());
    assert_eq!(
        pick(&live, Score::stays_median),
        Some(RuleId::Fewbody),
        "inf tie"
    );
}

#[test]
fn escape_live_inf_tie() {
    check_inf_tie(2);
}

negative_control!(
    escape_live_inf_tie,
    "the later rule with no more stays",
    expected = "inf tie",
    check_inf_tie(1)
);

fn check_never(never: usize) {
    let s = rule_score(&[10.0], never, 0);
    assert_eq!(s.stays_median(), f64::INFINITY, "never as inf");
}

#[test]
fn escape_live_never_is_inf() {
    check_never(3);
}

negative_control!(
    escape_live_never_is_inf,
    "no trajectory without a staying label",
    expected = "never as inf",
    check_never(0)
);

fn check_ties(fewbody_stays: usize) {
    check_tie(4, 0.0, RuleId::Fewbody, "tie: most stays");
    check_tie(
        fewbody_stays,
        5e-10,
        RuleId::Fewbody,
        "near tie: most stays",
    );
    // Past the tolerance the medians differ and the lesser median wins whatever the stays.
    check_tie(4, 1e-6, RuleId::Closure, "no tie: least median");
}

#[test]
fn escape_live_ties() {
    check_ties(4);
}

negative_control!(
    escape_live_ties,
    "the near tie's rule with no more stays than the first",
    expected = "near tie",
    check_ties(3)
);

#[test]
fn escape_live_pick() {
    check_pick(1);
}

negative_control!(
    escape_live_pick,
    "the fastest rule's wrong commit allowed",
    expected = "zero wrong",
    check_pick(0)
);
