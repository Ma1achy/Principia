//! A segment's timing (R-901; the chart-space model's Theorem 5; export contract Part 3): when its frames are shown,
//! never what they show. A frame at playback fraction `u` is the segment's frame at `s = ease(u)`, and the family never
//! sees `u`, so easing cannot move a keyframe or change a frame's content.

use serde::{Deserialize, Serialize};

use super::PathError;

/// A monotone easing curve with `e(0) = 0` and `e(1) = 1` exactly. Which curves are offered, and the default, are
/// TASK-M8-31's (REQ-TOOL-197). The plan is unclear here; chose `Linear` and a pinned monotone table, the form any such
/// curve is sampled into, because Theorem 5's test needs one curve that is not the identity and no curve is chosen yet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    /// `e(u) = u`.
    #[default]
    Linear,
    /// Piecewise linear through the table's knots.
    Table(MonotoneTable),
}

/// The knots `(u, e)` of a piecewise-linear easing: `(0, 0)` first and `(1, 1)` last, `u` strictly increasing, `e`
/// non-decreasing, every value finite. Built only by [`MonotoneTable::new`], which refuses any other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonotoneTable {
    knots: Vec<(f64, f64)>,
}

impl MonotoneTable {
    /// The table of `knots`, or [`PathError::Easing`] where they are not pinned, monotone and finite.
    pub fn new(knots: Vec<(f64, f64)>) -> Result<Self, PathError> {
        let pinned = knots.first() == Some(&(0.0, 0.0)) && knots.last() == Some(&(1.0, 1.0));
        let finite = knots.iter().all(|(u, e)| u.is_finite() && e.is_finite());
        let monotone = knots
            .windows(2)
            .all(|w| w[0].0 < w[1].0 && w[0].1 <= w[1].1);
        match pinned && finite && monotone {
            true => Ok(Self { knots }),
            false => Err(PathError::Easing),
        }
    }

    /// The knots.
    pub fn knots(&self) -> &[(f64, f64)] {
        &self.knots
    }
}

impl Easing {
    /// `e(u)` for `u` in `[0, 1]`: exactly 0 at 0 and exactly 1 at 1, compared exactly, never by arithmetic.
    pub fn ease(&self, u: f64) -> f64 {
        if u == 0.0 {
            return 0.0;
        }
        if u == 1.0 {
            return 1.0;
        }
        match self {
            Easing::Linear => u,
            Easing::Table(table) => {
                let k = &table.knots;
                let i = k.partition_point(|(x, _)| *x <= u).clamp(1, k.len() - 1);
                let ((u0, e0), (u1, e1)) = (k[i - 1], k[i]);
                e0 + (u - u0) / (u1 - u0) * (e1 - e0)
            }
        }
    }
}

/// A segment's timing: its easing. The retime table (§ 17.9) is stage 4's, typed by its hash and not built.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    /// The easing.
    pub easing: Easing,
    /// The retime table's hash: not built (stage 4); a segment carrying one refuses to be shown.
    pub retime: Option<u64>,
}
