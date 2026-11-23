//! `cargo xtask gate convergence --propose` (TASK-M3-34): the convergence gate's threshold on the finest `r_k`
//! (REQ-VAL-135) and its region minimum (REQ-VAL-168), from aggregates of the M3 march.
//!
//! **The refinement.** R-171's strides 32, 4, 1, 0 were prin-rs's escape-sampling strides, a cadence the M3 march no
//! longer has: escape is read at the window's marks, one grid (integrator contract Part 2a). The plan is unclear here;
//! chose the pixel grid as the refinement, strides 8, 4, 2 and 0 (every pixel) of the re-validation set's 64 × 64
//! sample, each a subset of the next, because it refines the one sampling an aggregate over a slice still has and it
//! reuses the same marches (pitfalls §3: "a diagnostic must reuse the original discretisation").
//!
//! **The aggregates** (the regions, one series each): per population, the fraction of pixels labelled escape, and
//! labelled collision, by the production horizon, the detector as it is.
//!
//! **The proposal, written before the measurement.** The threshold stays R-171's 0.1 where it fails the pitfalls §3
//! record and every series of the march whose `r_k` strictly decreases has its finest `r_k` below it; otherwise the
//! report says which series and the threshold is not proposed. The region minimum is proposed as the number of
//! series the march declares where all of them pass, and not proposed otherwise.

use super::gate::Marched;
use super::march::Stop;
use super::revalidation::revalidation_slices;
use super::tau_gap::Slice;
use crate::convergence::{first_non_decrease, passes, relative_steps};

/// The pixel strides, coarse to fine; 0 is every pixel.
pub const STRIDES: [u32; 4] = [8, 4, 2, 0];

/// R-171's provisional threshold.
pub const THRESHOLD: f64 = 0.1;

/// The pitfalls §3 record, coarse to fine (strides 32, 4, 1, 0).
pub const PITFALL: [f64; 4] = [0.2153, 0.4423, 0.5494, 0.0947];

/// Whether pixel `i` of an `n × n` grid is in the sample at `stride`.
fn sampled(i: usize, n: usize, stride: u32) -> bool {
    let s = stride.max(1) as usize;
    (i % n).is_multiple_of(s) && (i / n).is_multiple_of(s)
}

/// One aggregate over `m` at each stride: the fraction of sampled pixels `pick` holds for.
fn series(m: &Marched, n: usize, pick: &dyn Fn(usize) -> bool) -> Vec<f64> {
    STRIDES
        .iter()
        .map(|&s| {
            let idx: Vec<usize> = (0..m.runs.len()).filter(|&i| sampled(i, n, s)).collect();
            idx.iter().filter(|&&i| pick(i)).count() as f64 / idx.len() as f64
        })
        .collect()
}

/// The proposal (the module's note).
pub fn propose() -> Result<String, String> {
    let mut out = format!(
        "REQ-VAL-135 and REQ-VAL-168, from aggregates of the M3 march (pixel strides {STRIDES:?} of the re-validation \
         set, Heggie × RK4, the detector as it is, labels by the production horizon):\n"
    );
    let r_pit = relative_steps(&PITFALL)?;
    out += &format!(
        "- the pitfalls §3 record {PITFALL:?}: r = {r_pit:.4?}; strictly decreasing: {}; passes at {THRESHOLD}: {}\n",
        first_non_decrease(&r_pit).is_none(),
        passes(&r_pit, THRESHOLD)
    );
    let mut decreasing = Vec::new();
    let mut verdicts = Vec::new();
    for s in revalidation_slices() {
        let m = Slice::marched(&s)?;
        let horizon = |i: usize| m.pop.horizon(i);
        let esc = |i: usize| m.runs[i].firing.is_some_and(|f| f.t <= horizon(i));
        let col = |i: usize| m.runs[i].stop == Stop::Collision && m.runs[i].t_end <= horizon(i);
        for (what, pick) in [
            ("escape", &esc as &dyn Fn(usize) -> bool),
            ("collision", &col),
        ] {
            let x = series(&m, s.n, pick);
            let line = match relative_steps(&x) {
                Ok(r) => {
                    let ok = passes(&r, THRESHOLD);
                    verdicts.push(ok);
                    if first_non_decrease(&r).is_none() {
                        decreasing.push((format!("{} {what}", m.pop.name), r[r.len() - 1]));
                    }
                    format!(
                        "x = {x:.4?}, r = {r:.4?}, strictly decreasing {}, passes at {THRESHOLD}: {ok}",
                        first_non_decrease(&r).is_none()
                    )
                }
                Err(e) => format!("x = {x:.4?}: {e}"),
            };
            out += &format!("- {} {what} fraction: {line}\n", m.pop.name);
        }
    }
    let below = decreasing.iter().all(|(_, f)| *f < THRESHOLD);
    let fails_pitfall = !passes(&r_pit, THRESHOLD);
    out += &format!(
        "\nThreshold: {}. The strictly decreasing series' finest r_k: {decreasing:.4?}.\n",
        if fails_pitfall && below && !decreasing.is_empty() {
            format!(
                "{THRESHOLD} proposed: it fails the pitfalls record and passes every converging aggregate of the march"
            )
        } else {
            format!("{THRESHOLD} NOT proposed: a converging aggregate's finest r_k is at or above it, or none converges")
        }
    );
    let passing = verdicts.iter().filter(|&&v| v).count();
    out += &format!(
        "Region minimum: the march declares {} judged series, {passing} of them passing; {}.\n",
        verdicts.len(),
        if passing > 0 && passing == verdicts.len() {
            format!("all pass, so {} is proposed", verdicts.len())
        } else {
            "no minimum is proposed: a minimum is the count at which a passing conclusion stops holding, and here no \
             conclusion passes"
                .to_owned()
        }
    );
    Ok(out)
}
