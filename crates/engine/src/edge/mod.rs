//! Chart edges in the CPU evaluator: what a pixel past an edge of `[0,1]²` shows.
//!
//! Each edge of a chart declares a [`Kind`]; [`Edge`] holds the default and the kinds the edge may be switched to.
//! The kinds act on the pixel's chart point `(s, t)` before `Φ`, in this order: the pixel is resolved to the point
//! whose decode it shows (a canonical point, a sheet-2 point, an edge system, a squashed point), and the
//! [`Resolved`] record says which sheet, which kind acted, and how the pixel counts. A pixel inside `[0,1]²` that no
//! true edge crosses comes back with its input bits, never recomputed.
//!
//! The kinds act on the chart's own `(s, t)` only, never on a tilt's hidden coordinates, so on a tilted view a
//! repeat is the same system only where the move leaves the hidden offset unchanged. That is decided exactly on
//! the stored angles ([`kernel::chart::TiltSpec::normal_zero`]), never on a computed value; where it fails the edge
//! is affine and the pixel is its own system, counted.
//!
//! This module records the count of one pixel and nothing more: the area statistics, the navigation stop, the sim
//! key and the view state's per-edge override are not built here.

pub mod clamp;
pub mod saturate;
pub mod unfold;
pub mod wrap;

/// What an edge shows past itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The formula continues.
    Affine,
    /// The coordinate repeats by its period.
    Wrap,
    /// Continues over the pole, the partner coordinate shifted by half its period.
    PoleCrossing,
    /// A declared map onto the point that is the same system (the exponential map past `r = π`).
    Repeat,
    /// Onto the second sheet.
    Unfold,
    /// The nearest point of the primary range in the edge's metric.
    Clamp,
    /// The coordinate squashed toward a true end it never reaches.
    Saturate,
    /// Not a system.
    Blank,
}

/// The sheet of a construction a pixel shows: sheet 1 as written, sheet 2 the time reversal ([`unfold`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    /// The chart's own system.
    One,
    /// The time-reversed system.
    Two,
}

/// How one pixel counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// Its own system, counted.
    System,
    /// A repeat of a canonical point, not counted.
    Repeat,
    /// An edge system shown again by a clamp, not counted.
    Clamped,
    /// A squashed pixel: counted where its system is not shown inside the square, which the edge declares.
    Saturated,
    /// Not a system: blank.
    Blank,
}

/// One pixel after its edges have acted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    /// The chart point to decode.
    pub at: [f64; 2],
    /// The sheet the decode runs on.
    pub sheet: Sheet,
    /// The kind that moved the pixel: `None` where nothing did.
    pub applied: Option<Kind>,
    /// How the pixel counts.
    pub count: Count,
}

impl Resolved {
    /// The pixel as itself: sheet 1, counted, no kind acted.
    pub fn own(at: [f64; 2]) -> Self {
        Resolved {
            at,
            sheet: Sheet::One,
            applied: None,
            count: Count::System,
        }
    }

    /// The pixel blank, at its own point.
    pub fn blank(at: [f64; 2]) -> Self {
        Resolved {
            at,
            sheet: Sheet::One,
            applied: Some(Kind::Blank),
            count: Count::Blank,
        }
    }
}

/// An edge's declaration: its default kind and the kinds it may be switched to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// The kind the edge takes unless overridden.
    pub default: Kind,
    /// The kinds allowed beside the default; [`Kind::Blank`] is allowed on every edge.
    pub also: &'static [Kind],
}

/// A kind the edge does not allow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAllowed(pub Kind);

impl Edge {
    /// The kind in force: the override if the edge allows it, else the default.
    pub fn kind(&self, over: Option<Kind>) -> Result<Kind, NotAllowed> {
        match over {
            None => Ok(self.default),
            Some(k) if k == self.default || k == Kind::Blank || self.also.contains(&k) => Ok(k),
            Some(k) => Err(NotAllowed(k)),
        }
    }
}

/// Whether `x` lies in `[0, 1]`.
#[inline]
pub(crate) fn inside(x: f64) -> bool {
    (0.0..=1.0).contains(&x)
}
