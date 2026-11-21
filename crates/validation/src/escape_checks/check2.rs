//! Check 2 (pitfalls §2.4): **independent ground truth**, separation growing without bound, not an energy sign. The
//! old ground truth (unbound and receding at `t = 30`) shares the energy term with the criterion it scores; this one
//! reads only distances, from the harness's unstopped march ([`super::march`]), so the detector's `E_rel` cannot
//! agree with it by construction.
//!
//! **The ground truth** ([`truth`]), on one trajectory marched to the harness horizon `T_h`, read on its sampling
//! points in the second half of the march, `[T_h/2, T_h]`: body `k` escapes where
//! 1. `k` is the body farthest from the other two's centre of mass at every point,
//! 2. its distance `d_k` grows strictly from each point to the next, and
//! 3. it grows at least as fast as the slowest unbounded growth: `d_k(T_h) − d_k(3T_h/4) ≥ ρ·(d_k(3T_h/4) − d_k(T_h/2))`,
//!    `ρ = 2^{2/3} − 1` ([`RHO`]).
//!
//! Otherwise it is bound (no body escapes). A march a collision or a failure ended is **censored**: counted and
//! reported, never called bound. `ρ` is not tuned: a parabolic escape, the slowest that is unbounded, grows as
//! `(t − t₀)^{2/3}`, and over the segment's two quarters its increments' ratio is `2^{2/3} − 1` where it starts at the
//! segment's start, `t₀ = T_h/2`, and larger for any earlier start; a hyperbolic escape tends to `d ∝ t`, ratio 1. A
//! bound orbit decelerates faster than the parabola through the same points and turns back at its apocentre, which
//! condition 2 catches where it falls in the segment. One whose apocentre lies past `T_h` is not decidable on the
//! horizon: that is what the horizon's calibration is for, and it would read as an escape.
//!
//! **The score** ([`score`]): the detector's firings by the production horizon against the ground truth. A firing is
//! a true positive where the truth's escaper is the fired escaper (any escaper for `detail = 3`, a triple ejection);
//! precision is over firings whose truth is decided; recall is over the truth's escapers. Late firings, after the
//! production horizon and before the harness's, are reported beside them: late, not wrong (pitfalls §2.3).
//!
//! **The pass criterion** ([`judge`]), proposed (REQ-VAL-123): over the judged firings, precision at least
//! [`PRECISION_PROPOSED`], over at least [`MIN_DECIDED_FIRINGS`] decided firings. Recall is reported, not judged: the label is known to fire late
//! (PR #29) and the next task re-validates `tau` on these numbers.

use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;

use super::march::{PostEscape, Stop};
use super::{spread, Population};

/// The slowest unbounded growth's increment ratio, `2^{2/3} − 1` (the module's note).
pub fn rho() -> f64 {
    2f64.powf(2.0 / 3.0) - 1.0
}

/// [`rho`]'s value, to the digits a report prints.
pub const RHO: f64 = 0.587_401_051_968_199_5;

/// The proposed precision a check must reach (REQ-VAL-123, REQ-VAL-124; provisional until confirmed): pitfalls §2.2's
/// table measured 100.0% on the config chart against its energy ground truth, so a criterion that fires on one
/// non-escaper in a hundred has lost what made it the criterion.
pub const PRECISION_PROPOSED: f64 = 0.99;

/// The fewest decided firings a check's verdict is read over, so that a precision of 1 over three firings does not pass
/// (pitfalls §3, "check the measurement can fire before reading it").
pub const MIN_DECIDED_FIRINGS: usize = 20;

/// The ground truth of one trajectory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    /// Body `k`'s separation grows without bound.
    Escapes(u32),
    /// No body's does.
    Bound,
    /// A collision or a failure ended the march: undecided.
    Censored,
}

/// The second half's growth of the farthest body, where its distance is increasing throughout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Growth {
    /// The body.
    pub body: u32,
    /// Whether it is the farthest at every point of the segment.
    pub farthest: bool,
    /// Whether its distance grows strictly between consecutive points of the segment.
    pub increasing: bool,
    /// The increments' ratio, late quarter over the third quarter.
    pub ratio: f64,
}

/// The point index of the first sampling point at or after `t`, or the last.
fn at_or_after(run: &PostEscape, t: f64) -> usize {
    run.points
        .iter()
        .position(|p| p.t >= t)
        .unwrap_or(run.points.len().saturating_sub(1))
}

/// The farthest body's growth over `[T_h/2, T_h]` (the module's note); `None` where the march took too few points.
pub fn growth(run: &PostEscape) -> Option<Growth> {
    let last = run.points.last()?;
    let t_h = last.t;
    let (a, b) = (at_or_after(run, t_h / 2.0), at_or_after(run, 0.75 * t_h));
    let z = run.points.len() - 1;
    if !(a < b && b < z) {
        return None;
    }
    let far = |d: &[f64; 3]| {
        (0..3)
            .max_by(|&i, &j| d[i].total_cmp(&d[j]))
            .expect("three bodies")
    };
    let k = far(&last.d);
    let seg = &run.points[a..];
    let farthest = seg.iter().all(|p| far(&p.d) == k);
    let increasing = seg.windows(2).all(|w| w[1].d[k] > w[0].d[k]);
    let (da, db, dz) = (run.points[a].d[k], run.points[b].d[k], run.points[z].d[k]);
    Some(Growth {
        body: k as u32,
        farthest,
        increasing,
        ratio: (dz - db) / (db - da),
    })
}

/// The ground truth of `run` (the module's note).
pub fn truth(run: &PostEscape) -> Truth {
    if run.stop != Stop::Horizon {
        return Truth::Censored;
    }
    match growth(run) {
        Some(g) if g.farthest && g.increasing && g.ratio >= RHO => Truth::Escapes(g.body),
        Some(_) => Truth::Bound,
        None => Truth::Censored,
    }
}

/// The geometric ejection time of body `k`: the earliest sampling point from which `d_k` grows strictly to the end.
pub fn ejection_time(run: &PostEscape, k: u32) -> Option<f64> {
    let k = k as usize;
    let mut first = run.points.len().checked_sub(1)?;
    while first > 0 && run.points[first - 1].d[k] < run.points[first].d[k] {
        first -= 1;
    }
    Some(run.points[first].t)
}

/// One trajectory scored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scored {
    /// The ground truth.
    pub truth: Truth,
    /// The firing by the judged time `T_h/2`: its time, escaper and `detail`.
    pub fired: Option<(f64, u32, u32)>,
    /// Whether that firing is also by the slice's own horizon.
    pub by_horizon: bool,
    /// A firing after the judged time and by the harness horizon: its time, and whether it names the truth's escaper.
    pub late: Option<(f64, bool)>,
    /// The smallest window chord over the marks by the judged time.
    pub chord_min: f64,
    /// The window chord at the firing's mark.
    pub chord_at_firing: Option<f64>,
    /// The firing's time less the truth escaper's geometric ejection time.
    pub lag: Option<f64>,
}

/// Whether a firing with `escaper` and `detail` names the truth's escaper `k`.
fn names(escaper: u32, detail: u32, k: u32) -> bool {
    detail == DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION || escaper == k
}

/// `run` scored against its ground truth: firings by `judged` judged, `horizon` the slice's own.
pub fn scored(run: &PostEscape, judged: f64, horizon: f64) -> Scored {
    let truth = truth(run);
    let fired = run
        .firing
        .filter(|f| f.t <= judged)
        .map(|f| (f.t, f.escaper, f.detail));
    let late = run.firing.filter(|f| f.t > judged).map(|f| {
        let named = matches!(truth, Truth::Escapes(k) if names(f.escaper, f.detail, k));
        (f.t, named)
    });
    let chord_min = run
        .marks
        .iter()
        .filter(|m| m.t <= judged)
        .map(|m| m.chord)
        .fold(f64::INFINITY, f64::min);
    let chord_at_firing = run
        .firing
        .and_then(|f| run.marks.iter().find(|m| m.t == f.t).map(|m| m.chord));
    let lag = match (truth, run.firing) {
        (Truth::Escapes(k), Some(f)) => ejection_time(run, k).map(|t| f.t - t),
        _ => None,
    };
    Scored {
        truth,
        fired,
        by_horizon: run.firing.is_some_and(|f| f.t <= horizon),
        late,
        chord_min,
        chord_at_firing,
        lag,
    }
}

/// A population's confusion against the ground truth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Trajectories.
    pub n: usize,
    /// Firings whose truth escapes, the fired escaper.
    pub tp: usize,
    /// Firings whose truth is bound, or escapes another body.
    pub fp: usize,
    /// Firings whose march was censored.
    pub fired_censored: usize,
    /// The truth's escapers the detector had not fired on.
    pub fn_: usize,
    /// Of those, the ones it fired on later, by the harness horizon.
    pub fn_late: usize,
    /// The truth's bound trajectories it did not fire on.
    pub tn: usize,
    /// Censored trajectories with no firing.
    pub censored: usize,
}

impl Tally {
    /// `TP / (TP + FP)`, over decided firings; `None` with none.
    pub fn precision(&self) -> Option<f64> {
        let d = self.tp + self.fp;
        (d > 0).then(|| self.tp as f64 / d as f64)
    }

    /// `TP / (TP + FN)`, over the truth's escapers; `None` with none.
    pub fn recall(&self) -> Option<f64> {
        let d = self.tp + self.fn_;
        (d > 0).then(|| self.tp as f64 / d as f64)
    }
}

/// The confusion of `scores` over the judged firings, or, with `by_horizon`, over the firings by the slice's own
/// horizon alone.
pub fn tally(scores: &[Scored], by_horizon: bool) -> Tally {
    let mut t = Tally {
        n: scores.len(),
        ..Tally::default()
    };
    for s in scores {
        let fired = s.fired.filter(|_| !by_horizon || s.by_horizon);
        match (fired, s.truth) {
            (Some((_, e, d)), Truth::Escapes(k)) if names(e, d, k) => t.tp += 1,
            (Some(_), Truth::Censored) => t.fired_censored += 1,
            (Some(_), _) => t.fp += 1,
            (None, Truth::Escapes(_)) => {
                t.fn_ += 1;
                t.fn_late += usize::from(s.late.is_some());
            }
            (None, Truth::Bound) => t.tn += 1,
            (None, Truth::Censored) => t.censored += 1,
        }
    }
    t
}

/// A population scored: each pixel at its own judged time and horizon.
pub fn score(pop: &Population, runs: &[PostEscape]) -> Vec<Scored> {
    runs.iter()
        .enumerate()
        .map(|(i, r)| scored(r, pop.judged(i), pop.horizon(i)))
        .collect()
}

/// The verdict of a check on `tally` (the module's note): `Ok` with its line, or `Err` saying what failed.
pub fn judge(tally: &Tally, precision: f64) -> Result<String, String> {
    let decided = tally.tp + tally.fp;
    let p = tally.precision();
    let line = format!(
        "precision {} over {decided} decided firings (criterion ≥ {precision}, over ≥ {MIN_DECIDED_FIRINGS})",
        p.map_or("none".into(), |p| format!("{p:.4}"))
    );
    if decided < MIN_DECIDED_FIRINGS {
        return Err(format!("{line}: too few decided firings to read"));
    }
    match p {
        Some(p) if p >= precision => Ok(line),
        _ => Err(line),
    }
}

/// The report of a scored population: the confusion, precision and recall, the firing times, the lag behind the
/// geometric ejection, and the window chord's distributions for the truth's escapers and bound runs.
pub fn report(name: &str, pop: &Population, runs: &[PostEscape], scores: &[Scored]) -> String {
    let t = tally(scores, false);
    let h = tally(scores, true);
    let pick = |f: &dyn Fn(&Scored) -> Option<f64>| scores.iter().filter_map(f).collect::<Vec<_>>();
    let fired_t = pick(&|s| s.fired.map(|f| f.0));
    let late_t = pick(&|s| s.late.map(|l| l.0));
    let late_named = scores
        .iter()
        .filter(|s| s.late.is_some_and(|l| l.1))
        .count();
    let lag = pick(&|s| s.lag);
    let chord_esc = pick(&|s| matches!(s.truth, Truth::Escapes(_)).then_some(s.chord_min));
    let chord_bound = pick(&|s| (s.truth == Truth::Bound).then_some(s.chord_min));
    let chord_fire = pick(&|s| s.chord_at_firing);
    let ratios: Vec<f64> = runs.iter().filter_map(growth).map(|g| g.ratio).collect();
    let fmt = |x: Option<f64>| x.map_or("none".into(), |x| format!("{x:.4}"));
    let horizons: Vec<f64> = (0..pop.ics.len()).map(|i| pop.horizon(i)).collect();
    let harness: Vec<f64> = (0..pop.ics.len()).map(|i| pop.harness_horizon(i)).collect();
    let stops = |s: Stop| runs.iter().filter(|r| r.stop == s).count();
    format!(
        "{name} ({} pixels, {} left out; canonical time: the slice's horizon {}, harness horizon T_h {}; marches \
         ended by collision {}, failure {}, spent budget {}):\n\
         - judged, firings by T_h/2: TP {} FP {} TN {} FN {} (of which fired after T_h/2: {}); censored: {} fired, \
         {} not fired\n\
         - judged: precision {}, recall {}\n\
         - by the slice's own horizon: TP {} FP {} FN {}; precision {}, recall {}\n\
         - firing times, judged firings: {}\n\
         - late firings, after T_h/2: {}; of them naming the truth's escaper: {late_named}\n\
         - lag of the firing behind the geometric ejection, truth escapers: {}\n\
         - least window chord |Δn̂| by T_h/2, truth escapers: {}\n\
         - least window chord |Δn̂| by T_h/2, truth bound: {}\n\
         - window chord at the firing: {}\n\
         - growth ratio, late over third quarter (ρ = {RHO:.4}): {}",
        pop.ics.len(),
        pop.skipped.len(),
        spread(&horizons),
        spread(&harness),
        stops(Stop::Collision),
        stops(Stop::Failed),
        stops(Stop::Short),
        t.tp,
        t.fp,
        t.tn,
        t.fn_,
        t.fn_late,
        t.fired_censored,
        t.censored,
        fmt(t.precision()),
        fmt(t.recall()),
        h.tp,
        h.fp,
        h.fn_,
        fmt(h.precision()),
        fmt(h.recall()),
        spread(&fired_t),
        spread(&late_t),
        spread(&lag),
        spread(&chord_esc),
        spread(&chord_bound),
        spread(&chord_fire),
        spread(&ratios),
    )
}
