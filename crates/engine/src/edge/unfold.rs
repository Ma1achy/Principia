//! Unfold: past a fold the chart shows its second sheet (chart_reference §5.4 and §2.2 (iv)).
//!
//! The unfolded coordinate `s̃` (the warp's `s`; under the linear option `(1 + L_z/L_max)/2` at the pixel's own `L_max`)
//! has period 2: `s̃ ∈ [0, 1]` is sheet 1, `s̃ > 1` shows sheet 2 at `2 − s̃`, `s̃ < 0` sheet 2 at `−s̃`. The period's
//! representative is `[−½, 3/2)`, centred on the square; a pixel outside it repeats one inside.

use super::{inside, Count, Kind, Resolved, Sheet};
use kernel::chart::invariant::{decode_on_sheet, InvariantChart};
use kernel::decode::invariant::Sheet as KernelSheet;
use kernel::decode::Decoded;
use kernel::generated::links::Defaults;

/// `s̃` folded onto its period `[−½, 3/2)`: exact where `s̃` is dyadic, so `s̃ + 2` and `s̃` are the same bits.
#[inline]
fn period(s: f64) -> f64 {
    s - 2.0 * ((s + 0.5) / 2.0).floor()
}

/// The sheet and the canonical coordinate of `s̃`, and whether `s̃` lies in the period `[−½, 3/2)` (its pixels are
/// counted; one outside is a repeat of one inside).
pub fn fold(s: f64) -> (f64, Sheet, bool) {
    if inside(s) {
        return (s, Sheet::One, true);
    }
    let x = period(s);
    let primary = (-0.5..1.5).contains(&s);
    if x > 1.0 {
        (2.0 - x, Sheet::Two, primary)
    } else if x < 0.0 {
        (-x, Sheet::Two, primary)
    } else {
        (x, Sheet::One, primary)
    }
}

/// `s̃` for a linear axis: `L_z` against the pixel's own `L_max = √(2·I·K*)`, `(1 + L_z/L_max)/2`. A pixel with
/// `K* ≤ 0` has no sheet to unfold onto: it is the clamp below the floor's ([`super::clamp::below_floor`]).
pub fn linear_s(lz: f64, k_star: f64, inertia: f64) -> Option<f64> {
    let l_max = (2.0 * inertia * k_star).sqrt();
    (k_star > 0.0 && l_max.is_finite()).then(|| (1.0 + lz / l_max) / 2.0)
}

/// The pixel at `at`, whose coordinate `axis` (0 or 1) is `s̃`, under `kind`: unfolded onto sheet 2 past the fold; a
/// clamp lands on the edge `s̃ ∈ {0, 1}`, whose construction is the rigid rotation at `±L_max`; blank is blank.
/// Past the period a pixel is a repeat only where the tilt's normal leaves the hidden offset unchanged along the
/// unfolded axis (`normal_zero[axis]`, `N_⊥ = 0`, decided exactly on the stored angles); else it is its own system,
/// counted, on the same sheet.
pub fn resolve(at: [f64; 2], axis: usize, kind: Kind, normal_zero: [bool; 2]) -> Resolved {
    let s = at[axis];
    if inside(s) {
        return Resolved::own(at);
    }
    let with = |x: f64| {
        let mut p = at;
        p[axis] = x;
        p
    };
    match kind {
        Kind::Blank => Resolved::blank(at),
        Kind::Clamp => Resolved {
            at: with(if s > 1.0 { 1.0 } else { 0.0 }),
            sheet: Sheet::One,
            applied: Some(Kind::Clamp),
            count: Count::Clamped,
        },
        Kind::Unfold => {
            let (x, sheet, primary) = fold(s);
            let repeat = !primary && normal_zero[axis];
            Resolved {
                at: with(x),
                sheet,
                applied: Some(Kind::Unfold),
                count: if repeat { Count::Repeat } else { Count::System },
            }
        }
        _ => Resolved::own(at),
    }
}

/// The decode of a warped invariant chart at a resolved pixel, on the pixel's sheet.
pub fn decode<C>(chart: &C, r: &Resolved) -> Decoded<f64>
where
    C: InvariantChart<Param = f64>,
{
    let sheet = match r.sheet {
        Sheet::One => KernelSheet::One,
        Sheet::Two => KernelSheet::Two,
    };
    decode_on_sheet::<Defaults, f64, C>(chart, r.at[0], r.at[1], sheet).0
}
