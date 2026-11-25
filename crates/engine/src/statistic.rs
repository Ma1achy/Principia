//! A statistic's measure (design `principia_chart_composition.md` § 6.4): a fraction over a chart says what it counts.
//!
//! **A flat grid, never the quadtree's leaves.** The quadtree's leaf density follows the boundaries, not any physical
//! density (the scheduler contract's Part 2), so a statistic takes the flat grid: one sample at each pixel's centre,
//! `((i + ½)/w, (j + ½)/h)`, row by row. Anything else, a list of leaves among them, is refused ([`Refused::NotFlatGrid`]).
//!
//! **Weighted or labelled, no third.** A warped or composed chart's pixels are not equal in any physical measure, so a
//! fraction is one of:
//! - **weighted** ([`Weighting::Named`]): each counted pixel carries the chart's area factor into a reference measure the
//!   statistic names, `√det(JᵀJ)`, `J = ∂Y/∂(u, v)`; where the chart has no such factor (`ray-degenerate`, the factor
//!   `None`) the share is undefined and reads "—", never 0%;
//! - **labelled** ([`Weighting::PerPixel`], the default): the count, `per pixel of this chart`, claiming nothing physical.
//!
//! A mid-morph frame admits no area statistic ([`Weighting::MidMorph`], [`Refused::MidMorph`]); the two end charts keep
//! their own. Pixels that leave the count (`counted` false) leave the total too.
//!
//! [`warp_area`] is the area factor of a chart whose two axes are warped independently, `|x′(u)·y′(v)|`, into the
//! axes' own values: a paper's measure where the axes are the paper's coordinates.

use std::fmt;

use kernel::chart::warp::AxisWarp;

/// One pixel's sample: its chart point, whether it counts (pixels that leave the count leave the total), and whether it
/// is in the set whose share is asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The chart point.
    pub u: f64,
    /// The chart point.
    pub v: f64,
    /// Whether the pixel counts.
    pub counted: bool,
    /// Whether it is in the set.
    pub inside: bool,
}

/// The flat grid a statistic is taken on: `width × height` pixels of the unit square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatGrid {
    /// Pixels across.
    pub width: usize,
    /// Pixels down.
    pub height: usize,
}

impl FlatGrid {
    /// The centre of pixel `k`, row by row.
    pub fn centre(&self, k: usize) -> (f64, f64) {
        let (i, j) = (k % self.width, k / self.width);
        (
            (i as f64 + 0.5) / self.width as f64,
            (j as f64 + 0.5) / self.height as f64,
        )
    }

    /// Every pixel centre, row by row.
    pub fn centres(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        (0..self.width * self.height).map(|k| self.centre(k))
    }
}

/// An axis's warp and its range.
pub type WarpedAxis = (AxisWarp<f64>, [f64; 2]);

/// A chart's area factor at `(u, v)`; `None` where it has none.
pub type Area<'a> = &'a dyn Fn(f64, f64) -> Option<f64>;

/// How a statistic weighs its pixels (the module's note).
pub enum Weighting<'a> {
    /// Into the reference measure `name`: each pixel's area factor at its chart point, `None` where the chart has none.
    Named {
        /// The measure's name, stated with the statistic.
        name: &'a str,
        /// The area factor at `(u, v)`.
        area: Area<'a>,
    },
    /// The count, `per pixel of this chart`: the default.
    PerPixel,
    /// A mid-morph frame: no area statistic.
    MidMorph,
}

/// What a fraction counts in.
#[derive(Clone, Debug, PartialEq)]
pub enum Measure {
    /// Weighted into the named measure.
    Named(String),
    /// `per pixel of this chart`.
    PerPixel,
}

impl fmt::Display for Measure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Measure::Named(name) => write!(f, "measure: {name}"),
            Measure::PerPixel => f.write_str("per pixel of this chart"),
        }
    }
}

/// A fraction and what it counts in; `share` `None` reads "—": undefined, never 0%.
#[derive(Clone, Debug, PartialEq)]
pub struct Fraction {
    /// The share, or `None`.
    pub share: Option<f64>,
    /// Its measure.
    pub measure: Measure,
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.share {
            Some(s) => write!(f, "{:.4}% ({})", 100.0 * s, self.measure),
            None => write!(f, "— ({})", self.measure),
        }
    }
}

/// Why a statistic is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The samples are not the flat grid's pixel centres, row by row: a count of leaves, or any other sampling.
    NotFlatGrid,
    /// A weight with no measure named: neither weighted into a named measure nor labelled.
    NoMeasure,
    /// A mid-morph frame admits no area statistic.
    MidMorph,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refused::NotFlatGrid => "refused: the samples are not a flat grid's pixel centres",
            Refused::NoMeasure => "refused: no weight and no label: a weight names its measure",
            Refused::MidMorph => "refused: a mid-morph frame admits no area statistic",
        })
    }
}

/// The share of `samples` inside the set, over `grid`, under `weighting` (the module's note).
pub fn fraction(
    grid: FlatGrid,
    samples: &[Sample],
    weighting: Weighting<'_>,
) -> Result<Fraction, Refused> {
    fraction_guarded(grid, samples, weighting, true)
}

/// [`fraction`], with the guard that a weight names its measure on or off (its control turns it off).
pub fn fraction_guarded(
    grid: FlatGrid,
    samples: &[Sample],
    weighting: Weighting<'_>,
    named: bool,
) -> Result<Fraction, Refused> {
    let flat = samples.len() == grid.width * grid.height
        && samples
            .iter()
            .enumerate()
            .all(|(k, s)| (s.u, s.v) == grid.centre(k));
    if !flat {
        return Err(Refused::NotFlatGrid);
    }
    let (measure, area): (Measure, Option<Area<'_>>) = match weighting {
        Weighting::MidMorph => return Err(Refused::MidMorph),
        Weighting::Named { name, .. } if named && name.trim().is_empty() => {
            return Err(Refused::NoMeasure)
        }
        Weighting::Named { name, area } => (Measure::Named(name.to_owned()), Some(area)),
        Weighting::PerPixel => (Measure::PerPixel, None),
    };
    let (mut inside, mut total) = (0.0f64, 0.0f64);
    let mut undefined = false;
    for s in samples.iter().filter(|s| s.counted) {
        let w = match area {
            Some(a) => match a(s.u, s.v) {
                Some(w) if w.is_finite() && w >= 0.0 => w,
                _ => {
                    undefined = true;
                    0.0
                }
            },
            None => 1.0,
        };
        total += w;
        if s.inside {
            inside += w;
        }
    }
    let share = (!undefined && total > 0.0).then(|| inside / total);
    Ok(Fraction { share, measure })
}

/// The area factor of a chart whose axes are warped independently, `|x′(u)·y′(v)|`, into the axes' own values (the
/// module's note): each axis its warp and range.
pub fn warp_area(h: WarpedAxis, v: WarpedAxis) -> impl Fn(f64, f64) -> Option<f64> {
    move |u, t| Some((h.0.rate(h.1, u) * v.0.rate(v.1, t)).abs())
}
