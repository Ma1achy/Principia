//! The `delta-dep` measurement's judgement (`validation::measure::delta_dep`; REQ-VAL-055; R-37), on synthetic
//! populations, so its edges are exact:
//! - `delta_dep_judge`: a `δ` strictly inside the gap that is the rule's proposal passes; `δ` at either edge of the gap,
//!   or inside it but not the proposal, fails; with no gap there is no proposal; the rule rounds the geometric middle to
//!   two significant figures. The ledger's departed row and the moved schema version are the real ledger's.
//!
//! - `delta_dep_helpers`: a run's greatest chord, greatest step change and least chord in a window (both ends
//!   inclusive, the first of equal minima kept); the populations' edges; the report's proposal and no-gap lines; the
//!   fixture lists; the ledger's departed row; the figure-eight's period.
//! - `delta_dep_run`: a measured run's chords are the wrapper's, the shape chord of the state each horizon returns.
//! - `delta_dep_sample`, `delta_dep_nan_reductions`, `delta_dep_judge_failed`, `delta_dep_failed_report` and
//!   `delta_dep_run_fails`: a failed run (a macro-step not completed, a state or chord not finite) is stated, never
//!   read: the sample's arms, a NaN carried through every reduction, the judgement failing on a failed run below or
//!   above and naming it, the report flagging it, and a real march that goes non-finite under Yoshida4 at
//!   `r_coll = 0` (two bodies coincident at `t = 0`) recorded failed at its step with no non-finite chord kept.
//!
//! Each test registers its negative control (R-176).

use kernel::decode::Ic;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::welford::chord2;
use kernel::driver::wrapper::{integrate_with, WrapperUniforms};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::shape::shape;
use validation::drift::sim_key;
use validation::gate::Verdict;
use validation::measure::delta_dep::{
    beside, departed_row, eight, eight_period, eight_shape_period, generic, judge, report, sample,
    two_figures, Cause, Failure, Fixture, Measured, Run,
};
use validation::negative_control;

/// A run whose chords are `chords`, at `dt = 1`.
fn run(chords: Vec<f64>) -> Run {
    Run {
        name: "synthetic",
        width: "f64",
        dt: 1.0,
        horizon_steps: chords.len() as u32,
        chords,
        failed: None,
    }
}

/// A measurement whose largest first return is `below` and whose smallest generic excursion is `above`.
fn measured(below: f64, above: f64) -> Measured {
    Measured {
        returns: vec![
            (run(vec![0.3, below, 0.4]), below, 2.0),
            (run(vec![below / 2.0]), below / 2.0, 1.0),
        ],
        generic: vec![run(vec![0.1, above]), run(vec![2.0 * above])],
        beside: vec![],
        shape_period: 4.0,
    }
}

/// A judgement under check: the measurement's ([`judge`]), or a control's.
type Judge = fn(&Measured, f64) -> Verdict;

fn kernel_judge(m: &Measured, delta: f64) -> Verdict {
    judge(m, delta).0
}

/// Checks `judge` on the gap `(10⁻³, 0.5)`: its proposal, 0.022, passes; the edges and a value inside but not the
/// proposal fail; a measurement with no gap proposes nothing.
fn check_judge(judge: Judge) {
    let m = measured(1e-3, 0.5);
    assert_eq!(
        m.proposal(),
        Some(0.022),
        "the proposal of the gap (1e-3, 0.5)"
    );
    assert_eq!(judge(&m, 0.022), Verdict::Pass, "the proposal failed");
    assert_eq!(
        judge(&m, 1e-3),
        Verdict::Fail,
        "the gap's lower edge passed"
    );
    assert_eq!(judge(&m, 0.5), Verdict::Fail, "the gap's upper edge passed");
    assert_eq!(
        judge(&m, 0.023),
        Verdict::Fail,
        "a value inside the gap that is not the proposal passed"
    );
    assert_eq!(
        measured(0.5, 0.5).proposal(),
        None,
        "a gap of zero width proposed"
    );
    assert_eq!(measured(0.6, 0.5).proposal(), None, "no gap proposed");
    for (x, want) in [(0.022_36, 0.022), (123.4, 120.0), (0.0995, 0.1), (5.0, 5.0)] {
        let got = two_figures(x);
        assert!(
            (got - want).abs() <= 1e-12 * want,
            "two figures of {x} is {got}, not {want}"
        );
    }
}

#[test]
fn delta_dep_judge() {
    check_judge(kernel_judge);
}

negative_control!(
    delta_dep_judge,
    "a judgement that reads only the upper edge passes δ at the lower one",
    expected = "the gap's lower edge passed",
    check_judge(|m, delta| if delta < m.above().lo() && delta != 0.023 {
        Verdict::Pass
    } else {
        Verdict::Fail
    })
);

/// A window search under check: [`Run::least_in`], or a control's.
type Least = fn(&Run, f64, f64) -> Option<(f64, f64)>;

/// Checks the run helpers on a synthetic run (`dt = 1`, and `dt = 1/4` for the window's times), `least` the window
/// search; the populations, the report, the fixture lists, the departed row and the figure-eight's period.
fn check_helpers(least: Least) {
    let r = run(vec![0.5, 0.2, 0.9, 0.2, 0.4]);
    assert_eq!(r.max(), 0.9, "the greatest chord");
    assert_eq!(r.step(), 0.7, "the greatest step change");
    assert_eq!(
        run(vec![0.6]).step(),
        0.6,
        "a one-sample run's step is its chord"
    );
    assert_eq!(run(vec![]).step(), 0.0, "an empty run's step");
    assert_eq!(
        least(&r, 2.0, 4.0),
        Some((0.2, 2.0)),
        "the first of equal minima, both ends inclusive"
    );
    assert_eq!(
        least(&r, 3.0, 3.0),
        Some((0.9, 3.0)),
        "a window of one sample"
    );
    assert_eq!(
        least(&r, 4.0, 5.0),
        Some((0.2, 4.0)),
        "the window's lower end inclusive"
    );
    assert_eq!(
        least(&r, 1.0, 1.0),
        Some((0.5, 1.0)),
        "the first sample is at t = dt"
    );
    assert_eq!(least(&r, 5.5, 9.0), None, "a window past the run");
    let quarter = Run {
        dt: 0.25,
        ..run(vec![0.5, 0.2, 0.9, 0.1])
    };
    assert_eq!(
        least(&quarter, 0.5, 0.75),
        Some((0.2, 0.5)),
        "the sample `k` is at t = k·dt, at dt = 1/4"
    );
    assert_eq!(
        least(&quarter, 1.0, 4.0),
        Some((0.1, 1.0)),
        "the last sample at dt = 1/4 is at t = 1"
    );
    let m = measured(1e-3, 0.5);
    assert_eq!(
        (m.below().lo(), m.below().hi()),
        (5e-4, 1e-3),
        "the population below"
    );
    assert_eq!(
        (m.above().lo(), m.above().hi()),
        (0.5, 1.0),
        "the population above"
    );
    let text = report(&m);
    assert!(
        text.contains("**Proposed:** `δ_dep = 0.022`"),
        "the report's proposal: {text}"
    );
    assert!(
        text.contains("Below: 2 values in [5.000e-4, 1.000e-3]"),
        "the report's gap: {text}"
    );
    assert!(
        report(&measured(0.6, 0.5)).contains("**No gap:**"),
        "the report with no gap"
    );
    let names: Vec<&str> = generic().iter().map(|f| f.name).collect();
    assert_eq!(names.len(), 8, "the generic fixtures: {names:?}");
    assert_eq!(
        (names[0], names[7]),
        ("Burrau", "figure-eight"),
        "the generic fixtures' ends"
    );
    let beside: Vec<&str> = beside().iter().map(|f| f.name).collect();
    assert_eq!(beside.len(), 4, "the fixtures beside: {beside:?}");
    assert!(
        beside.iter().take(3).all(|n| n.starts_with("Lagrange")) && beside[3] == "Kepler-embedded"
    );
    assert!(
        departed_row(),
        "the ledger's departed row is not packed_a bit 15"
    );
    let p = eight_period();
    assert!(
        (p - 14.850_886).abs() < 1e-5,
        "the figure-eight's canonical period {p}"
    );
    assert_eq!(eight_shape_period(), 0.5 * p, "the shape period");
}

#[test]
fn delta_dep_helpers() {
    check_helpers(Run::least_in);
}

negative_control!(
    delta_dep_helpers,
    "a window search that keeps the last of equal minima picks the later sample",
    expected = "the first of equal minima",
    check_helpers(|r, from, to| {
        r.chords
            .iter()
            .enumerate()
            .map(|(i, &c)| ((i + 1) as f64 * r.dt, c))
            .filter(|&(t, _)| t >= from && t <= to)
            .fold(None, |best: Option<(f64, f64)>, (t, c)| match best {
                Some((b, _)) if b < c => best,
                _ => Some((c, t)),
            })
    })
);

/// A measured run under check: [`run`] at f64, or a control's.
type Measure = fn(f64) -> Run;

/// Checks `measure`'s chords of the figure-eight over `horizon` against the wrapper's: after each horizon `k`, the
/// chord of the state a `k`-step run returns to the IC's shape.
fn check_run(measure: Measure) {
    let horizon = 0.03;
    let r = measure(horizon);
    let f = eight();
    let key = sim_key::<f64>(horizon);
    assert_eq!(r.dt, 1e-3, "the run's dt_macro");
    assert_eq!(r.failed, None, "the figure-eight's run failed");
    assert!(r.horizon_steps >= 30, "the run's horizon");
    assert_eq!(
        r.chords.len() as u32,
        r.horizon_steps,
        "a chord a completed step"
    );
    let n_0 = shape(f.ic.r, f.ic.m);
    for (k, &c) in r.chords.iter().enumerate() {
        let u = WrapperUniforms {
            horizon_steps: k as u32 + 1,
            ..key
        };
        let s = integrate_with::<Yoshida4, f64, _>(
            &f.ic,
            &u,
            &mut WrapperCadence::new(&f.ic.m, &f.ic.r, &f.ic.p),
        );
        let want = chord2(shape(s.r, f.ic.m), n_0).sqrt();
        assert_eq!(
            c.to_bits(),
            want.to_bits(),
            "step {}: the chord {c} is not the wrapper's {want}",
            k + 1
        );
    }
}

#[test]
fn delta_dep_run() {
    check_run(|h| validation::measure::delta_dep::run::<f64>(&eight(), h, "f64"));
}

negative_control!(
    delta_dep_run,
    "a run whose chords lag a step behind is not the wrapper's",
    expected = "is not the wrapper's",
    check_run(|h| {
        let mut r = validation::measure::delta_dep::run::<f64>(&eight(), h, "f64");
        r.chords.insert(0, 0.0);
        r.chords.pop();
        r
    })
);

// ── A failed run is stated, never read ─────────────────────────────────────────────────────────────────────────

/// A sample's classification under check: [`sample`], or a control's.
type Sample = fn(u32, u32, bool, f64) -> Result<f64, Failure>;

/// Checks `sample`'s arms, in their order (a step not reached, then the state, then the chord), and [`Run::fault`]'s:
/// a recorded failure, else the first non-finite chord, else a run short of (or past) its horizon.
fn check_sample(sample: Sample) {
    let fail = |step, cause| Err(Failure { step, cause });
    assert_eq!(sample(3, 3, true, 0.25), Ok(0.5), "a sound sample's chord");
    assert_eq!(sample(3, 3, true, 0.0), Ok(0.0), "a zero chord is a sample");
    assert_eq!(
        sample(3, 2, true, 0.25),
        fail(3, Cause::Short),
        "a step not reached"
    );
    assert_eq!(
        sample(3, 4, true, 0.25),
        fail(3, Cause::Short),
        "a step overrun"
    );
    assert_eq!(
        sample(3, 2, false, f64::NAN),
        fail(3, Cause::Short),
        "a step not reached is tested first"
    );
    assert_eq!(
        sample(3, 3, false, 0.25),
        fail(3, Cause::State),
        "a non-finite state"
    );
    assert_eq!(
        sample(3, 3, false, f64::NAN),
        fail(3, Cause::State),
        "the state is tested before the chord"
    );
    for d2 in [f64::NAN, f64::INFINITY, -1.0] {
        assert_eq!(
            sample(3, 3, true, d2),
            fail(3, Cause::Chord),
            "a non-finite chord from {d2}"
        );
    }
    let r = run(vec![0.1, 0.2, 0.3]);
    assert_eq!(r.fault(), None, "a sound run's fault");
    let at = |step, cause| Some(Failure { step, cause });
    assert_eq!(
        run(vec![0.1, f64::NAN, 0.3]).fault(),
        at(2, Cause::Chord),
        "a NaN chord's step"
    );
    assert_eq!(
        run(vec![f64::INFINITY]).fault(),
        at(1, Cause::Chord),
        "an infinite chord's step"
    );
    assert_eq!(
        Run {
            horizon_steps: 4,
            ..r.clone()
        }
        .fault(),
        at(4, Cause::Short),
        "a run short of its horizon"
    );
    assert_eq!(
        Run {
            horizon_steps: 2,
            ..r.clone()
        }
        .fault(),
        at(4, Cause::Short),
        "a run past its horizon"
    );
    let recorded = Run {
        horizon_steps: 5,
        failed: Some(Failure {
            step: 4,
            cause: Cause::State,
        }),
        ..r
    };
    assert_eq!(
        recorded.fault(),
        at(4, Cause::State),
        "a recorded failure is the run's"
    );
    assert_eq!(
        recorded.fault_line().as_deref(),
        Some("run synthetic f64 dt 1.0000e0: failed at step 4, the state is not finite"),
        "a failure's line"
    );
    assert_eq!(run(vec![0.1]).fault_line(), None, "a sound run's line");
    for cause in [Cause::Short, Cause::State, Cause::Chord] {
        assert!(!cause.word().is_empty(), "{cause:?} has no words");
    }
}

#[test]
fn delta_dep_sample() {
    check_sample(sample);
}

negative_control!(
    delta_dep_sample,
    "a sample that never tests the state keeps a chord from a non-finite state",
    expected = "a non-finite state",
    check_sample(|k, reached, _state, d2| sample(k, reached, true, d2))
);

/// A run's greatest chord under check: [`Run::max`], or a control's.
type Max = fn(&Run) -> f64;

/// Checks that every reduction carries a NaN through, wherever in the run it stands, `max` the greatest chord.
fn check_reductions(max: Max) {
    for chords in [
        vec![0.1, f64::NAN, 1.9],
        vec![f64::NAN, 0.1, 1.9],
        vec![0.1, 1.9, f64::NAN],
    ] {
        let r = run(chords.clone());
        assert!(
            max(&r).is_nan(),
            "{chords:?}: the greatest chord dropped the NaN"
        );
        assert!(
            r.step().is_nan(),
            "{chords:?}: the greatest step dropped the NaN"
        );
        let (least, _) = r.least_in(1.0, 3.0).expect("the window holds the run");
        assert!(
            least.is_nan(),
            "{chords:?}: the least chord dropped the NaN"
        );
        let p = validation::measure::delta_dep::Population {
            values: chords.iter().map(|&v| ("x".to_owned(), v)).collect(),
        };
        assert!(
            p.lo().is_nan() && p.hi().is_nan(),
            "{chords:?}: the population's edges dropped the NaN"
        );
    }
    let r = run(vec![0.1, f64::NAN, 1.9]);
    assert_eq!(
        r.least_in(2.0, 2.0).map(|(c, t)| (c.is_nan(), t)),
        Some((true, 2.0)),
        "the NaN's time"
    );
    assert_eq!(
        r.least_in(3.0, 3.0),
        Some((1.9, 3.0)),
        "a window past the NaN"
    );
    let sound = run(vec![0.3, 0.1, 0.2]);
    assert_eq!(max(&sound), 0.3, "a sound run's greatest chord");
    assert_eq!(run(vec![]).max(), 0.0, "an empty run's greatest chord");
}

#[test]
fn delta_dep_nan_reductions() {
    check_reductions(Run::max);
}

negative_control!(
    delta_dep_nan_reductions,
    "a greatest chord by f64::max drops the NaN",
    expected = "the greatest chord dropped the NaN",
    check_reductions(|r| r.chords.iter().fold(0.0, |m, &c| m.max(c)))
);

/// A judgement with its lines under check: [`judge`], or a control's.
type FullJudge = fn(&Measured, f64) -> (Verdict, Vec<String>);

/// A measurement with the gap `(10⁻³, 0.5)` (proposal 0.022), the first below run's chords `below` read over the
/// window `[P_s/2, 3P_s/2] = [2, 6]`, the first generic run's chords `above`.
fn with_runs(below: Run, above: Run) -> Measured {
    let (least, at) = below.least_in(2.0, 6.0).expect("the window holds the run");
    Measured {
        returns: vec![(below, least, at), (run(vec![0.3, 5e-4]), 5e-4, 2.0)],
        generic: vec![above, run(vec![1.0])],
        beside: vec![],
        shape_period: 4.0,
    }
}

/// Checks that `judge` passes the proposal on sound runs and fails it, naming the run, its width, `dt_macro` and step,
/// on a run below or above that has a NaN chord, stopped short or recorded a failure.
fn check_judge_failed(judge: FullJudge) {
    let below = || run(vec![0.3, 2e-3, 1e-3]);
    let above = || run(vec![0.1, 0.2, 0.5]);
    let sound = with_runs(below(), above());
    assert_eq!(
        sound.proposal(),
        Some(0.022),
        "the sound measurement's proposal"
    );
    let (v, lines) = judge(&sound, 0.022);
    assert_eq!(v, Verdict::Pass, "the sound measurement failed: {lines:?}");
    let nan = |c: &[f64]| run(c.to_vec());
    let short = Run {
        horizon_steps: 4,
        ..above()
    };
    let recorded = Run {
        horizon_steps: 4,
        failed: Some(Failure {
            step: 4,
            cause: Cause::State,
        }),
        ..above()
    };
    for (what, m, line) in [
        (
            "a run above with a NaN chord",
            with_runs(below(), nan(&[0.1, f64::NAN, 0.5])),
            "- fails: run synthetic f64 dt 1.0000e0: failed at step 2, the chord is not finite",
        ),
        (
            "a run below with a NaN chord inside [P_s/2, 3P_s/2]",
            with_runs(nan(&[0.3, f64::NAN, 1e-3]), above()),
            "- fails: run synthetic f64 dt 1.0000e0: failed at step 2, the chord is not finite",
        ),
        (
            "a run above short of its horizon",
            with_runs(below(), short),
            "- fails: run synthetic f64 dt 1.0000e0: failed at step 4, the march stopped before its horizon",
        ),
        (
            "a run above with a recorded failure",
            with_runs(below(), recorded),
            "- fails: run synthetic f64 dt 1.0000e0: failed at step 4, the state is not finite",
        ),
    ] {
        let (v, lines) = judge(&m, 0.022);
        assert_eq!(v, Verdict::Fail, "{what} passed: {lines:?}");
        assert!(lines.iter().any(|l| l == line), "{what}: no line `{line}` in {lines:?}");
        assert_eq!(m.proposal(), None, "{what}: a proposal");
    }
}

#[test]
fn delta_dep_judge_failed() {
    check_judge_failed(judge);
}

/// The least chord over `[from, to]` by the fold before failed runs were stated, which drops a NaN.
#[cfg(feature = "controls")]
fn least_dropping(r: &Run, from: f64, to: f64) -> f64 {
    r.chords
        .iter()
        .enumerate()
        .map(|(i, &c)| ((i + 1) as f64 * r.dt, c))
        .filter(|&(t, _)| t >= from && t <= to)
        .fold(f64::INFINITY, |m, (_, c)| m.min(c))
}

negative_control!(
    delta_dep_judge_failed,
    "a judgement with no finiteness check, by NaN-dropping folds, passes a run with a NaN chord",
    expected = "a run above with a NaN chord passed",
    check_judge_failed(|m, delta| {
        let p = m.shape_period;
        let below = m
            .returns
            .iter()
            .map(|(r, _, _)| least_dropping(r, 0.5 * p, 1.5 * p))
            .fold(0.0, f64::max);
        let above = m
            .generic
            .iter()
            .map(|r| r.chords.iter().fold(0.0, |a: f64, &c| a.max(c)))
            .fold(f64::INFINITY, f64::min);
        let pass = delta > below && delta < above && two_figures((below * above).sqrt()) == delta;
        (if pass { Verdict::Pass } else { Verdict::Fail }, vec![])
    })
);

/// A report under check: [`report`], or a control's.
type Report = fn(&Measured) -> String;

/// Checks that `report` names every failed run, below, above and beside, flags it in its table and makes no proposal;
/// and says no run failed where none did.
fn check_failed_report(report: Report) {
    let sound = with_runs(run(vec![0.3, 2e-3, 1e-3]), run(vec![0.1, 0.2, 0.5]));
    let text = report(&sound);
    assert!(
        text.contains("None: every run is finite and full-length."),
        "the sound report: {text}"
    );
    assert!(
        !text.contains("failed at step"),
        "the sound report flags a run: {text}"
    );
    let mut m = with_runs(run(vec![0.3, 2e-3, 1e-3]), run(vec![0.1, f64::NAN, 0.5]));
    m.beside = vec![Run {
        name: "beside",
        horizon_steps: 3,
        failed: Some(Failure {
            step: 2,
            cause: Cause::State,
        }),
        ..run(vec![0.1])
    }];
    let text = report(&m);
    for want in [
        "- run synthetic f64 dt 1.0000e0: failed at step 2, the chord is not finite",
        "- run beside f64 dt 1.0000e0: failed at step 2, the state is not finite",
        "| synthetic | f64 (failed at step 2: the chord is not finite) |",
        "| beside | f64 (failed at step 2: the state is not finite) |",
        "**No proposal:** runs below or above failed: 1 (Failed runs).",
    ] {
        assert!(text.contains(want), "the report has no `{want}`: {text}");
    }
    let mut below = with_runs(run(vec![0.3, f64::NAN, 1e-3]), run(vec![0.1, 0.2, 0.5]));
    below.beside = vec![];
    assert!(
        report(&below).contains("| f64 (failed at step 2: the chord is not finite) |"),
        "the first-return table flags no failed run"
    );
}

#[test]
fn delta_dep_failed_report() {
    check_failed_report(report);
}

negative_control!(
    delta_dep_failed_report,
    "a report that ignores a recorded failure on a run beside does not name it",
    expected = "run beside f64",
    check_failed_report(|m| {
        let mut m = m.clone();
        for r in &mut m.beside {
            r.failed = None;
            r.horizon_steps = r.chords.len() as u32;
        }
        report(&m)
    })
);

/// Two bodies coincident at `t = 0` (bodies 0 and 1 at `(1/2, 0)`, body 2 at `(−1, 0)`, at rest, the centre of mass at
/// the origin): Yoshida4's first force evaluation divides by a zero separation, so the march goes non-finite at once,
/// and at `r_coll = 0` nothing ends it.
fn coincident() -> Fixture {
    Fixture {
        name: "coincident pair",
        ic: Ic {
            m: [1.0 / 3.0; 3],
            r: [[0.5, 0.0], [0.5, 0.0], [-1.0, 0.0]],
            p: [[0.0; 2]; 3],
        },
        horizon: 0.01,
    }
}

/// A measured run under check, of a fixture over a horizon at f64: [`run`], or a control's.
type MeasureAt = fn(&Fixture, f64) -> Run;

/// The march before failed runs were recorded: every completed step's chord kept, finite or not, and no failure.
#[cfg(feature = "controls")]
fn run_unchecked(f: &Fixture, horizon: f64) -> Run {
    use kernel::driver::wrapper::integrate_resumed_with;
    use kernel::payload::tm_t_end_step;
    let ic = f.ic;
    let key = sim_key::<f64>(horizon);
    let n_0 = shape(ic.r, ic.m);
    let zero = WrapperUniforms {
        horizon_steps: 0,
        ..key
    };
    let mut s = integrate_with::<Yoshida4, f64, _>(
        &ic,
        &zero,
        &mut WrapperCadence::new(&ic.m, &ic.r, &ic.p),
    );
    let mut chords = Vec::new();
    for k in 1..=key.horizon_steps {
        let mut cadence = WrapperCadence::new(&ic.m, &s.r, &s.p);
        s = integrate_resumed_with::<Yoshida4, f64, _>(&ic, &key, &mut cadence, &s, 1);
        if tm_t_end_step(s.times) != k {
            break;
        }
        chords.push(chord2(shape(s.r, ic.m), n_0).sqrt());
    }
    Run {
        name: f.name,
        width: "f64",
        dt: key.dt_macro,
        chords,
        horizon_steps: key.horizon_steps,
        failed: None,
    }
}

/// Checks that `measure` records the coincident pair's run failed at its first step, its state not finite, keeping no
/// chord, and the figure-eight's over the same horizon not failed.
fn check_run_fails(measure: MeasureAt) {
    let r = measure(&coincident(), 0.01);
    assert_eq!(r.horizon_steps, 10, "the coincident pair's horizon");
    assert_eq!(
        r.failed,
        Some(Failure {
            step: 1,
            cause: Cause::State
        }),
        "the coincident pair's run is not failed at step 1, its state not finite"
    );
    assert_eq!(r.chords.len(), 0, "a chord kept at or past the failure");
    assert!(
        r.chords.iter().all(|c| c.is_finite()),
        "a non-finite chord kept"
    );
    assert_eq!(r.fault(), r.failed, "the run's fault is its failure");
    let e = measure(&eight(), 0.01);
    assert_eq!(e.failed, None, "the figure-eight's run failed");
    assert_eq!(e.fault(), None, "the figure-eight's run has a fault");
    assert_eq!(
        e.chords.len(),
        10,
        "the figure-eight's run is not full-length"
    );
}

#[test]
fn delta_dep_run_fails() {
    check_run_fails(|f, h| validation::measure::delta_dep::run::<f64>(f, h, "f64"));
}

negative_control!(
    delta_dep_run_fails,
    "a march that keeps every completed step's chord, finite or not, records no failure",
    expected = "the coincident pair's run is not failed",
    check_run_fails(run_unchecked)
);
