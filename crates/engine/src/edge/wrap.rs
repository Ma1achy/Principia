//! Wrap, pole-crossing and the exponential map's repeat (chart_reference §5.4's shape-sphere rows).
//!
//! The spherical map's `θ` has period 1 in `s`; `φ` crosses the poles with period 2 in `t`, the partner `s` shifted by
//! half its period at each crossing. Both fold a pixel onto the canonical point in `[0,1]²` by exact subtraction of
//! the period, so dyadic `s` and `s + 1` resolve to the same bits; don't resolve through the map's trig, which returns
//! the same point to rounding only. The exponential map's `(r, ê) ↦ (2π − r, −ê)` is the repeat past `r = π`.

use super::{inside, Count, Kind, Resolved, Sheet};
use core::f64::consts::PI;

/// Each axis's kind and whether the tilt's normal leaves a repeat along it the same system: `[N_a = 0, N_b = 0]`,
/// decided exactly on the stored angles, `[true, true]` untilted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fold {
    /// The kind on `s` (θ's edges).
    pub s: Kind,
    /// The kind on `t` (the poles).
    pub t: Kind,
    /// `[N_a = 0, N_b = 0]`.
    pub normal_zero: [bool; 2],
}

/// `x` less its whole periods, `[0, period)`: exact where `x` and `period` are dyadic.
#[inline]
fn modulo(x: f64, period: f64) -> f64 {
    x - period * (x / period).floor()
}

/// The spherical map's pixel at `(s, t)`: past the square folded onto the canonical point where its kind allows and
/// the tilt keeps the hidden offset (θ's period iff `N_a = 0`, the pole maps iff `N_a = N_b = 0`), else its own
/// system, counted; blank where its kind is blank.
pub fn spherical(at: [f64; 2], fold: Fold) -> Resolved {
    let [s, t] = at;
    if inside(s) && inside(t) {
        return Resolved::own(at);
    }
    let [na, nb] = fold.normal_zero;
    let (mut s_out, mut t_out) = (s, t);
    let mut moved = None;
    if !inside(t) {
        match fold.t {
            Kind::Blank => return Resolved::blank(at),
            Kind::PoleCrossing if na && nb => {
                let tm = modulo(t, 2.0);
                let over = tm > 1.0;
                t_out = if over { 2.0 - tm } else { tm };
                if over {
                    s_out += 0.5;
                }
                moved = Some(Kind::PoleCrossing);
            }
            _ => {}
        }
    }
    if !inside(s_out) {
        match fold.s {
            Kind::Blank => return Resolved::blank(at),
            Kind::Wrap if na => {
                s_out = modulo(s_out, 1.0);
                moved = moved.or(Some(Kind::Wrap));
            }
            _ => {}
        }
    }
    match moved {
        Some(kind) => Resolved {
            at: [s_out, t_out],
            sheet: Sheet::One,
            applied: Some(kind),
            count: Count::Repeat,
        },
        None => Resolved::own(at),
    }
}

/// The exponential map's pixel at `(u, v)` with extent `scale` (`d = ((2u − 1)·scale, (2v − 1)·scale)`, `r = ‖d‖`):
/// inside `r ≤ π` as written; past it the repeat map `(r, ê) ↦ (2π − r, −ê)` (reduced mod `2π`) iff
/// `N_a = N_b = 0`, else affine, the formula continuing over the sphere again, counted; a clamp lands on `−n₀`.
pub fn exponential(at: [f64; 2], scale: f64, kind: Kind, normal_zero: [bool; 2]) -> Resolved {
    let [u, v] = at;
    let d = [(2.0 * u - 1.0) * scale, (2.0 * v - 1.0) * scale];
    let r = libm::hypot(d[0], d[1]);
    if r <= PI {
        return Resolved::own(at);
    }
    let back = |rad: f64, sign: f64| {
        let k = sign * rad / r;
        [
            (d[0] * k / scale + 1.0) / 2.0,
            (d[1] * k / scale + 1.0) / 2.0,
        ]
    };
    match kind {
        Kind::Blank => Resolved::blank(at),
        Kind::Clamp => Resolved {
            at: back(PI, 1.0),
            sheet: Sheet::One,
            applied: Some(Kind::Clamp),
            count: Count::Clamped,
        },
        Kind::Repeat if normal_zero == [true, true] => {
            let rm = modulo(r, 2.0 * PI);
            let (rad, sign) = if rm > PI {
                (2.0 * PI - rm, -1.0)
            } else {
                (rm, 1.0)
            };
            Resolved {
                at: back(rad, sign),
                sheet: Sheet::One,
                applied: Some(Kind::Repeat),
                count: Count::Repeat,
            }
        }
        _ => Resolved::own(at),
    }
}
