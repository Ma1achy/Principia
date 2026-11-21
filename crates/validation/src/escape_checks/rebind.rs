//! The re-bind test (pitfalls §1.5 and §2.4 check 1's analogue): every trajectory that fires escape, followed past the
//! firing by the harness march ([`super::march`]), is checked at five physical offsets past its firing, and none may
//! re-bind.
//!
//! **The offsets** ([`offsets`]) are the times prin-rs's +1, +2, +3, +4 and +8 sync boundaries stood at in the
//! measurement that found the 0 of 895: `deep interior`, `t_max = 13` and `n_sync = 32` in Burrau's units
//! (`docs/reference/prin-rs/src/integrate/az/driver.rs`, `escape_confirm`'s note: "In `deep interior`, of the 895
//! trajectories that escape under `escape_every = 1` …, 0.000 are still unbound one boundary later — and 0.000 at +2,
//! +3, +4 and +8"; its grid, the integrator contract Part 2a's "production grid (`t_max / n_sync = 13/32`)"). So the
//! offsets are `k·13/32` Burrau time units, `k ∈ {1, 2, 3, 4, 8}`: 0.40625, 0.8125, 1.21875, 1.625 and 3.25, each
//! multiplied by the pixel's canonical time per Burrau unit. The harness marches on `Δ_sync`, so each is read at the
//! first sync boundary at or after `t_fire + offset`, the escape window's own rule for a time between boundaries.
//!
//! **Re-bound** is the energy arm alone (prin-rs's "the persistence question is 'does it RE-BIND', and that is the
//! energy arm alone"): the escaper's `E_rel ≤ 0` at an offset. The closure arm jitters above `tau` on a settled escape,
//! so reading persistence off full candidacy would score jitter as re-binding. A march a collision or a failure ended
//! before an offset is **censored** there: reported, not counted as unbound. A triple ejection's escaper is the body the
//! detector named.
//!
//! **The pass criterion**: none re-bind (REQ-EVT-013), over at least [`super::check2::MIN_DECIDED_FIRINGS`] firings, so
//! the test is not passed vacuously by a slice that does not fire.

use super::march::{Point, PostEscape};
use super::{spread, Population};

/// prin-rs's measurement grid, Burrau's units: `t_max` and `n_sync` (the module's note).
pub const PRIN_RS_T_MAX: f64 = 13.0;
/// prin-rs's `n_sync`.
pub const PRIN_RS_N_SYNC: f64 = 32.0;
/// The sync boundaries past firing the measurement checked.
pub const BOUNDARIES: [u32; 5] = [1, 2, 3, 4, 8];

/// The five offsets in Burrau's time units, `k·t_max/n_sync`.
pub fn offsets_burrau() -> [f64; 5] {
    BOUNDARIES.map(|k| f64::from(k) * PRIN_RS_T_MAX / PRIN_RS_N_SYNC)
}

/// The five offsets in canonical time at a pixel whose canonical time per Burrau unit is `scale`.
pub fn offsets(scale: f64) -> [f64; 5] {
    offsets_burrau().map(|o| o * scale)
}

/// One offset's reading on a fired trajectory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reading {
    /// The escaper is still unbound: its `E_rel`.
    Unbound(f64),
    /// It re-bound: its `E_rel ≤ 0`.
    Rebound(f64),
    /// The march ended before the offset.
    Censored,
}

/// A fired trajectory's five readings, from its firing at `t_fire`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rebind {
    /// The pixel.
    pub pixel: usize,
    /// The firing's time.
    pub t_fire: f64,
    /// Whether it fired by the slice's own horizon.
    pub by_horizon: bool,
    /// The escaper.
    pub escaper: u32,
    /// The readings at the five offsets.
    pub readings: [Reading; 5],
}

/// The readings of `run` at `offsets` past its firing, where it fired with its last offset inside the harness horizon
/// `harness`; `None` otherwise.
pub fn readings(pixel: usize, run: &PostEscape, harness: f64, offsets: [f64; 5]) -> Option<Rebind> {
    readings_with(pixel, run, harness, offsets, at_or_after)
}

/// How a reading picks its sampling point for a time between boundaries.
pub type Pick = for<'a> fn(&'a [Point], f64) -> Option<&'a Point>;

/// The first sampling point at or after `t` (the module's note), `None` past the march's end.
pub fn at_or_after(points: &[Point], t: f64) -> Option<&Point> {
    points.iter().find(|p| p.t >= t)
}

/// [`readings`] with the sampling point of each offset picked by `pick`.
pub fn readings_with(
    pixel: usize,
    run: &PostEscape,
    harness: f64,
    offsets: [f64; 5],
    pick: Pick,
) -> Option<Rebind> {
    let f = run.firing.filter(|f| f.t + offsets[4] <= harness)?;
    let k = f.escaper as usize;
    let readings = offsets.map(|o| match pick(&run.points, f.t + o) {
        Some(p) if p.e_rel[k] > 0.0 => Reading::Unbound(p.e_rel[k]),
        Some(p) => Reading::Rebound(p.e_rel[k]),
        None => Reading::Censored,
    });
    Some(Rebind {
        pixel,
        t_fire: f.t,
        by_horizon: false,
        escaper: f.escaper,
        readings,
    })
}

/// Every firing of a population read at its pixel's offsets.
pub fn rebinds(pop: &Population, runs: &[PostEscape]) -> Vec<Rebind> {
    runs.iter()
        .enumerate()
        .filter_map(|(i, r)| {
            readings(i, r, pop.harness_horizon(i), offsets(pop.scale[i])).map(|x| Rebind {
                by_horizon: x.t_fire <= pop.horizon(i),
                ..x
            })
        })
        .collect()
}

/// The verdict on `rebinds` (the module's note): `Ok` with its line, or `Err` saying what failed.
pub fn judge(rebinds: &[Rebind]) -> Result<String, String> {
    let rebound: Vec<&Rebind> = rebinds
        .iter()
        .filter(|r| r.readings.iter().any(|x| matches!(x, Reading::Rebound(_))))
        .collect();
    let line = format!(
        "{} of {} firings re-bind at an offset (criterion: none, over ≥ {} firings)",
        rebound.len(),
        rebinds.len(),
        super::check2::MIN_DECIDED_FIRINGS
    );
    if rebinds.len() < super::check2::MIN_DECIDED_FIRINGS {
        return Err(format!("{line}: too few firings to read"));
    }
    if rebound.is_empty() {
        Ok(line)
    } else {
        let first: Vec<String> = rebound
            .iter()
            .take(5)
            .map(|r| {
                format!(
                    "pixel {} fired at {:.3}: {:?}",
                    r.pixel, r.t_fire, r.readings
                )
            })
            .collect();
        Err(format!("{line}; first: {}", first.join("; ")))
    }
}

/// The report of a population's readings: per offset, the unbound, re-bound and censored counts and the escaper's
/// `E_rel` there.
pub fn report(name: &str, rebinds: &[Rebind]) -> String {
    let mut out = format!(
        "{name}: {} firings ({} by the slice's own horizon); offsets k·13/32 Burrau units, k = {BOUNDARIES:?} ({:?}), each at the first sync boundary \
         at or after it\n",
        rebinds.len(),
        rebinds.iter().filter(|r| r.by_horizon).count(),
        offsets_burrau()
    );
    for (j, k) in BOUNDARIES.iter().enumerate() {
        let col: Vec<Reading> = rebinds.iter().map(|r| r.readings[j]).collect();
        let count = |f: fn(&Reading) -> bool| col.iter().filter(|x| f(x)).count();
        let e: Vec<f64> = col
            .iter()
            .filter_map(|x| match x {
                Reading::Unbound(e) | Reading::Rebound(e) => Some(*e),
                Reading::Censored => None,
            })
            .collect();
        out += &format!(
            "- +{k}: unbound {}, re-bound {}, censored {}; escaper E_rel {}\n",
            count(|x| matches!(x, Reading::Unbound(_))),
            count(|x| matches!(x, Reading::Rebound(_))),
            count(|x| matches!(x, Reading::Censored)),
            spread(&e)
        );
    }
    out
}
