//! The escape rule replayed on a harness march at any `tau` and any window (TASK-M3-34): the measurement under the tau
//! gap, the precision gate and the window ladder.
//!
//! **What it replays.** The detector's own test (`kernel::detect::escape::test`): at each window mark, the chord
//! between the shape point now and the one held from the previous mark, compared squared against `tau²` and refused
//! where not finite, and the farthest body's `E_rel > 0` decided as the detector decides it. The harness keeps, at
//! every sampling point, exactly what that test reads ([`super::march::Point`]: `n̂`, the candidate, the ionisation
//! gate), so the replay at the register's `tau` and window is the detector itself, firing for firing
//! ([`reproduces`]), and at any other `(tau, window)` it is the detector the register would build with those values.
//! The marks of another window follow the kernel's rule ([`marks_at_or_before`]) with the window as a parameter.
//!
//! **One march for every rung.** A replay reads the march; it never re-integrates. Every `(tau, window)` is scored on
//! the same trajectories, so a difference between rungs is the rule's and not the integration's (pitfalls §3, "a
//! diagnostic must reuse the original discretisation").

use kernel::decode::Ic;
use kernel::detect::escape::TAU_PROVISIONAL;
use kernel::driver::welford::chord2;
use kernel::generated::constants::ESCAPE_WINDOW;
use kernel::payload::DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION;
use kernel::shape::shape;

use super::march::{Firing, PostEscape};

/// A rule: the settling threshold and the window, physical time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule {
    /// `tau`, a chord on the unit shape sphere.
    pub tau: f64,
    /// The window `|Δn̂|` is taken over.
    pub window: f64,
}

impl Rule {
    /// The register's rule: its `tau` and its window, the detector as it is built.
    pub fn register() -> Self {
        Rule {
            tau: TAU_PROVISIONAL,
            window: ESCAPE_WINDOW,
        }
    }

    /// The register's window at `tau`.
    pub fn at(tau: f64) -> Self {
        Rule {
            tau,
            ..Self::register()
        }
    }
}

/// The marks of `window` at or before the sampling point `index` of a grid of `quantum`: the kernel's
/// `marks_at_or_before`, its window a parameter. The quotient proposes `k`; the two products decide it.
pub fn marks_at_or_before(index: u32, quantum: f64, window: f64) -> u32 {
    let t = f64::from(index) * quantum;
    let mut k = (t / window) as u32;
    if k > 0 && f64::from(k) * window > t {
        k -= 1;
    }
    if k < u32::MAX && f64::from(k + 1) * window <= t {
        k += 1;
    }
    k
}

/// Whether the sampling point `index` is the first at or after a mark of `window` (the kernel's `is_mark`).
pub fn is_mark(index: u32, quantum: f64, window: f64) -> bool {
    index > 0
        && marks_at_or_before(index, quantum, window)
            > marks_at_or_before(index - 1, quantum, window)
}

/// One window mark of a replay: its time, the chord there, and the detector's candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chord {
    /// The index into the march's points.
    pub point: usize,
    /// The physical time.
    pub t: f64,
    /// `|n̂ − n̂_lag|`; not finite where the shape point was not.
    pub chord: f64,
    /// Whether the farthest body is unbound there: the energy arm, armed.
    pub armed: bool,
    /// The farthest body.
    pub body: u32,
    /// The escape's `detail` were it to fire there.
    pub detail: u32,
}

/// Every mark of `window` along `run` from the IC `ic`, with its chord: the register set to `n̂(0)` and taking `n̂` at
/// each mark, as the detector's does, but never stopping.
pub fn chords(run: &PostEscape, ic: &Ic<f64>, window: f64) -> Vec<Chord> {
    let mut lag = shape(ic.r, ic.m);
    let mut out = Vec::new();
    for (i, p) in run.points.iter().enumerate() {
        if !is_mark(p.index, run.quantum, window) {
            continue;
        }
        let c2 = chord2(p.n, lag);
        out.push(Chord {
            point: i,
            t: p.t,
            chord: if c2.is_finite() { c2.sqrt() } else { f64::NAN },
            armed: p.candidate.found,
            body: p.candidate.body,
            detail: if p.ionised {
                DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
            } else {
                p.candidate.body
            },
        });
        lag = p.n;
    }
    out
}

/// The firing of `rule` on `run` from `ic`: the first mark at which the squared chord is finite and below `tau²` and
/// the farthest body is unbound, the detector's test; `None` where it never fires on the march.
pub fn replay(run: &PostEscape, ic: &Ic<f64>, rule: Rule) -> Option<Firing> {
    let mut lag = shape(ic.r, ic.m);
    for p in &run.points {
        if !is_mark(p.index, run.quantum, rule.window) {
            continue;
        }
        let c2 = chord2(p.n, lag);
        if c2.is_finite() && c2 < rule.tau * rule.tau && p.candidate.found {
            return Some(Firing {
                index: p.index,
                t: p.t,
                escaper: p.candidate.body,
                detail: if p.ionised {
                    DETAIL_ESCAPE_ALL_THREE_TRIPLE_EJECTION
                } else {
                    p.candidate.body
                },
                r: [[f64::NAN; 2]; 3],
                p: [[f64::NAN; 2]; 3],
            });
        }
        lag = p.n;
    }
    None
}

/// `run` with its firing replaced by `rule`'s, for the scorers that read a march's firing ([`super::check2::score`]).
/// The replayed firing keeps no state: nothing compares it.
pub fn under(run: &PostEscape, ic: &Ic<f64>, rule: Rule) -> PostEscape {
    PostEscape {
        firing: replay(run, ic, rule),
        ..run.clone()
    }
}

/// Every run of a population under `rule`.
pub fn all_under(runs: &[PostEscape], ics: &[Ic<f64>], rule: Rule) -> Vec<PostEscape> {
    runs.iter()
        .zip(ics)
        .map(|(r, ic)| under(r, ic, rule))
        .collect()
}

/// Whether the replay at the register's rule is the detector's own firing on `run`: the same sampling point, escaper
/// and `detail`, or neither fired. `Err` says where they part.
pub fn reproduces(run: &PostEscape, ic: &Ic<f64>) -> Result<(), String> {
    let own = run.firing.map(|f| (f.index, f.escaper, f.detail));
    let again = replay(run, ic, Rule::register()).map(|f| (f.index, f.escaper, f.detail));
    if own == again {
        Ok(())
    } else {
        Err(format!(
            "the replay at the register's rule fires {again:?} (index, escaper, detail), the detector {own:?}"
        ))
    }
}
