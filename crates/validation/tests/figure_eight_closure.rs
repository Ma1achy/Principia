//! The figure-eight closure gate (TASK-M3-28; REQ-VAL-041, REQ-VAL-133): the anchor's ladder, run on every merge.
//!
//! - `figure_eight_closure_gate`: the gate on its fixtures, every input its expected outcome (the occupants pass, the
//!   landing-off and Euler controls fail). Its control: the landing-off input relabelled `pass`.
//! - `figure_eight_closure_default_occupant`: Heggie (Eq. 22) × RK4, the shipping occupant, passes; its control, the
//!   same occupant with the landing off (dd_validation_orbits §0's overshoot), fails on order and on closure.
//! - `figure_eight_closure_fit`: the order is read from the successive differences between rungs; its control reads it
//!   from the closure `|dr|`, which flattens at the published digits' own non-closure and reads a fourth-order
//!   occupant as order 0.
//! - `figure_eight_closure_resolution`: the fit's arithmetic at its edges: a slope just above the floor passes and one
//!   just below fails (3.501 and 3.49 against 4 − 0.5), and fewer than three resolved differences never pass; its control counts unresolved ones.
//!
//! **Pass criteria**, written in the gate before its first run (`validation::gate::figure_eight`'s note): every run
//! marching on the period's boundary; at least 3 differences at least `1e-11`; the fitted order at least the stated one
//! less the input's tolerance (0.5); the closure at `η = 0.01` within the input's bound. The tolerance and the threshold's
//! rule were fixed before any measurement; the ladder, the fit to the differences and their floor after the first run
//! and a probe (the gate's note says which and why).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use kernel::regularisation::az::End;
use kernel::regularisation::pairs::{Choice, HeggieTime};
use validation::gate::figure_eight::{
    self, differences, judge, ladder, order, resolved, start, Ladder, Named, Rung,
    DIFFERENCE_FLOOR, MIN_RESOLVED, NAME, ORDER_TOLERANCE,
};
use validation::gate::{self, Verdict};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh scratch directory: one per call, so a test and its control, which run in one process, never share one.
fn out(name: &str) -> PathBuf {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("figure-eight-{name}-{}-{call}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The default occupant, with the landing on or off.
fn default_occupant(landing: bool) -> Named {
    Named {
        stepper: figure_eight::stepper("rk4").unwrap(),
        regularisation: Choice::Heggie(HeggieTime::Eq22),
        landing,
    }
}

// ── The runner on the fixtures ───────────────────────────────────────────────────────────────────────────────────

/// The gate run on a copy of its fixtures, with `relabel` applied to the input files' text; panics unless every input
/// gives its expected outcome.
fn check_gate(relabel: fn(&str, String) -> String) {
    let copy = out("root");
    let dir = copy.join("fixtures/gates").join(NAME);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(copy.join("docs/history")).unwrap();
    std::fs::copy(
        root().join("docs/history/requirements.yaml"),
        copy.join("docs/history/requirements.yaml"),
    )
    .unwrap();
    for entry in std::fs::read_dir(root().join("fixtures/gates").join(NAME)).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(dir.join(&name), relabel(&name, text)).unwrap();
    }
    gate::run(&copy, NAME, &out("reports")).expect("each input its expected outcome");
}

#[test]
fn figure_eight_closure_gate() {
    check_gate(|_, text| text);
}

negative_control!(
    figure_eight_closure_gate,
    "the landing-off control relabelled `pass`",
    expected = "each input its expected outcome",
    check_gate(|name, text| if name == "control_overshoot.json" {
        text.replace("\"expected\": \"fail\"", "\"expected\": \"pass\"")
    } else {
        text
    })
);

// ── The shipping occupant ────────────────────────────────────────────────────────────────────────────────────────

/// The default occupant's closure bound, from its input, `heggie_rk4.json`, and the gate's ceiling over it.
fn threshold() -> f64 {
    let path = root()
        .join("fixtures/gates")
        .join(NAME)
        .join("heggie_rk4.json");
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let bound = v["closure_bound"]
        .as_f64()
        .expect("the input's closure bound");
    assert!(bound <= gate::config(&root(), NAME).unwrap().threshold.value);
    bound
}

fn check_default_occupant(landing: bool) {
    let l = ladder(&default_occupant(landing), &start().unwrap()).unwrap();
    for line in l.table() {
        println!("{line}");
    }
    let (verdict, lines) = judge(&l, threshold(), ORDER_TOLERANCE);
    for line in &lines {
        println!("- {line}");
    }
    assert_eq!(
        verdict,
        Verdict::Pass,
        "the figure-eight closure gate fails for {}: {lines:?}",
        l.named.describe()
    );
}

#[test]
fn figure_eight_closure_default_occupant() {
    check_default_occupant(true);
}

negative_control!(
    figure_eight_closure_default_occupant,
    "the landing off: each interval's last step overshoots its boundary (dd_validation_orbits §0)",
    expected = "the figure-eight closure gate fails",
    check_default_occupant(false)
);

// ── What the order is fitted to ──────────────────────────────────────────────────────────────────────────────────

/// The default occupant's fitted order read by `fit` is at least 4 less the tolerance.
fn check_fit(fit: fn(&Ladder) -> f64) {
    let l = ladder(&default_occupant(true), &start().unwrap()).unwrap();
    let p = fit(&l);
    println!("the default occupant's fitted order: {p:.3}");
    assert!(
        p >= 4.0 - ORDER_TOLERANCE,
        "the fit reads the fourth-order default occupant as order {p:.3}"
    );
}

#[test]
fn figure_eight_closure_fit() {
    check_fit(Ladder::order);
}

negative_control!(
    figure_eight_closure_fit,
    "the order fitted to the closure |dr|, which flattens at the published digits' non-closure",
    expected = "the fit reads the fourth-order default occupant as order",
    check_fit(|l| {
        let points: Vec<(f64, f64)> = l
            .rungs
            .iter()
            .map(|r| (r.eta.ln(), r.closure.ln()))
            .collect();
        figure_eight::slope(&points)
    })
);

// ── The fit's arithmetic at its edges ────────────────────────────────────────────────────────────────────────────

/// A synthetic ladder whose differences are `c·η^p` at `etas` (each rung's state offset along x by the running sum, so
/// neighbouring rungs differ by exactly the difference), the gate run marching on boundary 1 with closure 0.
fn synthetic(etas: &[f64], p: f64, c: f64) -> Ladder {
    let s = start().unwrap();
    let grid = figure_eight::grid(s.period, 1).unwrap();
    let mut x = 0.0;
    let mut rungs = Vec::new();
    for (k, &eta) in etas.iter().enumerate() {
        let mut state = s.state;
        state.r[0][0] += x;
        rungs.push(Rung {
            eta,
            state,
            closure: 0.0,
            error: 0.0,
            steps: 1,
            end: End::Marching,
            boundary: 1,
        });
        if k + 1 < etas.len() {
            x += c * eta.powf(p);
        }
    }
    let gate = rungs[0];
    Ladder {
        named: default_occupant(true),
        stated: 4,
        period: s.period,
        grid,
        truth: figure_eight::truth(&s, &grid),
        rungs,
        gate,
    }
}

fn check_resolution(count: fn(&[(f64, f64)]) -> usize) {
    let etas = [0.32, 0.16, 0.08, 0.04];
    // Just above 3.5 passes against 4 − 0.5; just below fails.
    let at = synthetic(&etas, 3.501, 1e-4);
    assert_eq!(
        judge(&at, 1.0, 0.5).0,
        Verdict::Pass,
        "order 3.501 fails against 4 − 0.5"
    );
    let below = synthetic(&etas, 3.49, 1e-4);
    assert_eq!(
        judge(&below, 1.0, 0.5).0,
        Verdict::Fail,
        "order 3.49 passes"
    );
    // Two differences above the floor and one below: too few, whatever the slope.
    let c = DIFFERENCE_FLOOR * 1.5 / 0.16f64.powi(4);
    let few = synthetic(&etas, 4.0, c);
    let d = differences(&few.rungs);
    assert!(
        d[0].1 >= DIFFERENCE_FLOOR && d[2].1 < DIFFERENCE_FLOOR,
        "{d:?}"
    );
    assert!(
        count(&d) < MIN_RESOLVED,
        "{} resolved differences counted where two are above the floor",
        count(&d)
    );
    assert_eq!(
        judge(&few, 1.0, 0.5).0,
        Verdict::Fail,
        "a two-difference fit passes"
    );
    assert!((order(&resolved(&d)) - 4.0).abs() < 1e-9);
}

#[test]
fn figure_eight_closure_resolution() {
    check_resolution(|d| resolved(d).len());
}

negative_control!(
    figure_eight_closure_resolution,
    "every difference counted, resolved or not",
    expected = "resolved differences counted where two are above the floor",
    check_resolution(|d| d.len())
);
