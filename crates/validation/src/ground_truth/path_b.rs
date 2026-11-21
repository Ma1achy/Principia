//! Path B (validation_ground_truth_note, "Two entry paths"): the lower harness entry for an exactly degenerate
//! analytic configuration, a physical `(m, r, p)` built from its closed form, handed to the march and the shape map
//! directly.
//!
//! ```text
//! inject::<A>(m, r, p, &key) → Result<PathB, Refused>
//! ```
//!
//! - **`m`, `r`, `p`**: the masses, positions and momenta in the sim's units (`G = 1`, `Σmᵢ = 1`) and in the canonical
//!   gauge, the centre of mass at the origin and at rest (`Σmᵢrᵢ = 0`, `Σpᵢ = 0`). The gauge is checked, never imposed:
//!   a start outside it is [`Refused`], so the march sees exactly the closed form's numbers. The scale gauge `I = 1` is
//!   not required: the march is scale-covariant, and a closed form is free to choose its scale.
//! - **`A`**: the occupant (a type, as `engine::precision`'s entry points take it).
//! - **`key`**: the march's values, its horizon and macro-step, and its collision radius (`super::march_key`).
//!
//! **What it bypasses**: the chart (no `(u, v)`, no window), the link selection's decode `D` (no `z`), the encode `E`,
//! and canonicalisation `C` (the closed form is built in the canonical gauge, and only checked there). **What it keeps**:
//! the dispatch-time label of the start (`kernel::detect::t0`), the wrapper's march at f64 with its cadence, detectors,
//! monitor and shape readout (`engine::precision::integrate_ic`), and the shape map `n(r, m)` (`kernel::shape::shape`)
//! read on the start and on the state the march stopped at.
//!
//! **Why a degenerate configuration needs it.** Euler's collinear configuration with the middle body labelled 2 (the
//! third body at the inner pair's centre of mass) has `λ̃ = 0`, the `α = 0` pole, which the encode reaches only by
//! clamping (`lookup_clamped`; the test `entry_paths_euler_path_b` measures it). The chart approaches the pole as a limit
//! and never lands on it, so the exact configuration enters here, below the chart.

use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::shape::shape;

use engine::precision::{integrate_ic, MarchKey64, SimState64};

use super::Provenance;

/// Why a start is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refused {
    /// A value is not finite.
    NotFinite,
    /// A mass is not positive, or the masses do not sum to 1 within 4 ulp.
    Masses,
    /// The centre of mass is off the origin, or not at rest, by more than [`GAUGE_TOL`] of the state's own extent:
    /// `|Σmᵢrᵢ|` against `max‖rᵢ‖`, `|Σpᵢ|` against `max‖pᵢ‖`.
    Gauge {
        /// `|Σmᵢrᵢ| / max‖rᵢ‖`.
        com: f64,
        /// `|Σpᵢ| / max‖pᵢ‖`, 0 at rest.
        momentum: f64,
    },
}

/// The gauge check's tolerance, relative to the state's own extent: 16 ulp of f64, a closed form's rounding.
pub const GAUGE_TOL: f64 = 16.0 * f64::EPSILON;

/// A start through Path B.
#[derive(Clone, Debug, PartialEq)]
pub struct PathB {
    /// The start as injected.
    pub start: Ic<f64>,
    /// The march's `SimState`.
    pub state: SimState64,
    /// The shape point `n` of the start.
    pub shape_start: [f64; 3],
    /// The shape point `n` where the march stopped.
    pub shape_end: [f64; 3],
}

/// Checks `(m, r, p)` against the canonical gauge (the module's note).
pub fn gauge(m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> Result<Ic<f64>, Refused> {
    let finite = m
        .iter()
        .chain(r.iter().flatten())
        .chain(p.iter().flatten())
        .all(|x| x.is_finite());
    if !finite {
        return Err(Refused::NotFinite);
    }
    let total = m[0] + m[1] + m[2];
    if m.iter().any(|&x| x <= 0.0) || (total - 1.0).abs() > 4.0 * f64::EPSILON {
        return Err(Refused::Masses);
    }
    let norm = |v: [f64; 2]| v[0].hypot(v[1]);
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|&w| norm(w)).fold(0.0f64, f64::max);
    let sum = |w: [f64; 3], v: &[[f64; 2]; 3]| {
        [0, 1].map(|k| w[0] * v[0][k] + w[1] * v[1][k] + w[2] * v[2][k])
    };
    let relative = |s: [f64; 2], e: f64| if e > 0.0 { norm(s) / e } else { norm(s) };
    let com = relative(sum(m, &r), extent(&r));
    let momentum = relative(sum([1.0; 3], &p), extent(&p));
    if com > GAUGE_TOL || momentum > GAUGE_TOL {
        return Err(Refused::Gauge { com, momentum });
    }
    Ok(Ic { m, r, p })
}

/// The Path B entry: `(m, r, p)` checked against the gauge, then marched by `A` under `key` (the module's note).
pub fn inject<A: Advance>(
    m: [f64; 3],
    r: [[f64; 2]; 3],
    p: [[f64; 2]; 3],
    key: &MarchKey64,
) -> Result<PathB, Refused> {
    let start = gauge(m, r, p)?;
    let state = integrate_ic::<A>(&start, None, key);
    Ok(PathB {
        start,
        state,
        shape_start: shape(start.r, start.m),
        shape_end: shape(state.r, start.m),
    })
}

/// The provenance of a Path B run of the fixture `fixture` by the occupant named `occupant` under `key` to `horizon`.
pub fn provenance(
    fixture: &str,
    occupant: &'static str,
    key: &MarchKey64,
    horizon: f64,
) -> Provenance {
    Provenance {
        fixture: fixture.to_string(),
        path: "B",
        occupant,
        selection: "none (no decode)",
        horizon,
        dt_macro: key.loop_values.dt_macro,
        r_coll: f64::from(key.radius.root()),
    }
}
