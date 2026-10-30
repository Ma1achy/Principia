//! The custom-stop ramp (colour_composition §9.4, "The custom-stop ramp"; R-72, R-546; REQ-COL-140): the gradient
//! editor's "custom stops, cyclic" (render_gui_spec Part II §9), a `colour` node's ramp of `t ∈ [0, 1]` to a colour.
//!
//! - **Stops.** Two or more, each a position in `[0, 1]` and an OKLab colour `(L, a, b)`, `L` in `[0, 1]`, every
//!   number finite; in order of position, never decreasing. Two stops may share a position, which draws a hard edge.
//! - **Interpolation space.** OKLab: between two neighbouring stops each of `L`, `a` and `b` is interpolated linearly
//!   in `t`; the result is converted to linear sRGB, the `vec3` a `colour` node outputs.
//! - **Ends.** Not cyclic, `t` is clamped to `[0, 1]`: below the first stop's position the first stop's colour, at or
//!   past the last stop's the last's. Where stops share a position, `t` there takes the later stop's colour.
//! - **`cyclic`.** `t` is taken modulo 1, so `t` and `t + 1` are one colour, and the ends wrap: past the last stop the
//!   ramp runs on from the last stop's colour to the first stop's, reached at the first stop's position plus 1.
//!
//! It is a `colour` node's param, the node's `ramp` (`Node::ramp`), and serialises with the node in `RenderState`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// One stop: its position in `[0, 1]` and its OKLab colour `(L, a, b)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub position: f64,
    pub oklab: [f64; 3],
}

/// A custom-stop ramp (colour_composition §9.4), checked at construction and when read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Parts", into = "Parts")]
pub struct Ramp {
    stops: Vec<Stop>,
    cyclic: bool,
}

/// The serialised form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parts {
    stops: Vec<Stop>,
    cyclic: bool,
}

impl TryFrom<Parts> for Ramp {
    type Error = RampError;

    fn try_from(p: Parts) -> Result<Ramp, RampError> {
        Ramp::new(p.stops, p.cyclic)
    }
}

impl From<Ramp> for Parts {
    fn from(r: Ramp) -> Parts {
        Parts {
            stops: r.stops,
            cyclic: r.cyclic,
        }
    }
}

/// Why a ramp was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RampError(pub String);

impl fmt::Display for RampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RampError {}

impl Ramp {
    /// The ramp of `stops`, cyclic or not; refused unless there are two or more stops, each position finite and in
    /// `[0, 1]`, never below the one before it, and each colour finite with `L` in `[0, 1]`.
    pub fn new(stops: Vec<Stop>, cyclic: bool) -> Result<Ramp, RampError> {
        if stops.len() < 2 {
            return Err(RampError(format!(
                "a ramp has two or more stops; this one has {}",
                stops.len()
            )));
        }
        let mut before = 0.0;
        for (k, s) in stops.iter().enumerate() {
            if !(0.0..=1.0).contains(&s.position) {
                return Err(RampError(format!(
                    "stop {k}'s position {} is not in [0, 1]",
                    s.position
                )));
            }
            if s.position < before {
                return Err(RampError(format!(
                    "stop {k}'s position {} is below the stop before it, at {before}",
                    s.position
                )));
            }
            before = s.position;
            let [l, a, b] = s.oklab;
            if !(0.0..=1.0).contains(&l) || !a.is_finite() || !b.is_finite() {
                return Err(RampError(format!(
                    "stop {k}'s colour {:?} is not an OKLab colour with L in [0, 1]",
                    s.oklab
                )));
            }
        }
        Ok(Ramp { stops, cyclic })
    }

    /// The stops, in order of position.
    pub fn stops(&self) -> &[Stop] {
        &self.stops
    }

    /// Whether the ramp is cyclic.
    pub fn cyclic(&self) -> bool {
        self.cyclic
    }

    /// The ramp's OKLab colour at `t`, or `None` where `t` is not finite: a non-finite field takes the invalid-pixel
    /// treatment (colour_composition §1.2), not a colour of the ramp.
    pub fn oklab_at(&self, t: f64) -> Option<[f64; 3]> {
        if !t.is_finite() {
            return None;
        }
        let (first, last) = (self.stops[0], self.stops[self.stops.len() - 1]);
        let t = if self.cyclic {
            // `t − ⌊t⌋` rounds to 1 for a `t` just below an integer; that `t` is the integer's, so 0.
            let f = t - t.floor();
            if f < 1.0 {
                f
            } else {
                0.0
            }
        } else {
            t.clamp(0.0, 1.0)
        };
        if t < first.position || t >= last.position {
            if !self.cyclic {
                return Some(if t < first.position { first } else { last }.oklab);
            }
            // The wrap: from the last stop to the first, one period on. Its span and `t`'s offset into it are summed
            // from their parts, never shifted by 1, which would round a first stop a hair past 0 away. The span is
            // positive, since `t` lies in `[0, 1)` outside `[first, last)`, so not both first = 0 and last = 1; and
            // the offset is at most the span, rounding being monotone, so `u` is in `[0, 1]`.
            let span = (1.0 - last.position) + first.position;
            let offset = if t < first.position {
                (1.0 - last.position) + t
            } else {
                t - last.position
            };
            return Some(lerp(last.oklab, first.oklab, offset / span));
        }
        // The last stop at or before `t`, and the next, past `t`.
        let k = self.stops.iter().rposition(|s| s.position <= t)?;
        let (a, b) = (self.stops[k], self.stops[k + 1]);
        let u = (t - a.position) / (b.position - a.position);
        Some(lerp(a.oklab, b.oklab, u))
    }

    /// The ramp's colour at `t` in linear sRGB, the `vec3` a `colour` node outputs; `None` where `t` is not finite.
    pub fn linear_at(&self, t: f64) -> Option<[f64; 3]> {
        self.oklab_at(t).map(oklab_to_linear)
    }
}

/// `a + u (b − a)`, per component.
fn lerp(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + u * (b[i] - a[i]))
}

/// OKLab to linear sRGB (dd_colouring §3.1; R-51): delegates to the render crate's one source of the conversion,
/// `render::colour::space::oklab_to_linear` over its `OKLAB` coefficients, so the two cannot drift apart.
pub fn oklab_to_linear(lab: [f64; 3]) -> [f64; 3] {
    render::colour::space::oklab_to_linear(lab)
}
