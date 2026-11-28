//! Slopes of a chart's decode in three tiers, and the flag at each kink site.
//!
//! A chart declares one [`Tier`]: **exact**, its `J_D` columns by the one decode source run at [`Dual`]; **implicit**,
//! its columns by the implicit-function rule where the decode solves for its state (a slope from `J_D`, never from a
//! difference); or **none**, no slope, every sample evaluated on the CPU, still a valid chart. Central differences are
//! the check of an exact or implicit slope, never its source.
//!
//! A kink site is where the decode changes branch and the slope jumps or is undefined: the feasibility boundary, `k = 0`,
//! `warped_k` at `d = 0`, `normalise`'s edges, the exponential map's `r = 0`, the poles, the seed switches, and the
//! ties and clamps of the registry rows' forwards. A tier-1 or
//! tier-2 chart lists its sites in [`Declaration::kinks`], and [`flag`] reads a footprint: it returns the site the
//! footprint meets, so a consumer need not ask whether a point lies exactly on one.
//!
//! The plan is unclear here: it gives the kink sites by name and no geometry. A chart's [`Site`] gives one where it has
//! it, and [`flag`] reads those first, so a kink whose slope jumps and stays finite is found. Otherwise [`flag`] takes a
//! site to be met where, in the footprint's nine points (corners, edge midpoints and centre), the chart's own `validate`
//! leaves `Valid`, the decode is `degenerate`, an exact slope is not finite, or a slope is larger than `1/h` (a slope
//! grows like 1 over the distance to a pole). A kink with finite slopes on both sides and no [`Site`] is not seen.
//!
//! The encode's canonical-form shortcuts (the scale step skipped within `τ_I` of `I = 1`, a zero centre of mass or `ρ_y`)
//! are not kinks: a dual takes its slope through the smooth form (`crate::canon::canonicalise`).
//!
//! Host-only: the SPIR-V build does not take this module.

use crate::chart::{Chart, ValidationResult, REGISTRY};
use crate::dual::Dual;
use crate::generated::links::Selection;

/// How a chart's slopes are obtained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// `J_D` by the decode at [`Dual`].
    Exact,
    /// `J_D` by the implicit-function rule.
    Implicit,
    /// No slope: each sample is evaluated on the CPU.
    None,
}

/// A place the decode changes branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kink {
    /// The edge of the feasible set of the targets a chart asks for.
    FeasibilityBoundary,
    /// `k = 0`, where the momentum's scale is a square root of `k`.
    KZero,
    /// `warped_k` at `d = 0`.
    WarpedKAtZeroD,
    /// An edge of the energy normalisation.
    NormaliseEdge,
    /// The exponential map's `r = 0`.
    ExpMapRZero,
    /// A pole of the shape sphere or of an angle's chart.
    Pole,
    /// A switch between two seeds of a construction.
    SeedSwitch,
    /// A `min` or `max` of two values, where they tie.
    OrderSwitch,
    /// A clamp's edge.
    ClampEdge,
}

/// A kink site's own geometry, in the chart's `(u, v)`: whether the footprint of half-width `h` about `(u, v)` meets it.
/// Where a chart has one, [`flag`] reads it first, so a kink whose slope jumps and stays finite is flagged too.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    /// Which kink this is.
    pub kink: Kink,
    /// Whether the square `[u − h, u + h] × [v − h, v + h]` meets the site.
    pub meets: fn(f64, f64, f64) -> bool,
}

/// What a chart declares: its tier and the kink sites it flags. `tier` is `None` until a chart declares one.
#[derive(Clone, Copy, Debug)]
pub struct Declaration {
    /// The chart's registry name ([`REGISTRY`]).
    pub chart: &'static str,
    /// Its slope tier; `None` for a chart that has not declared one.
    pub tier: Option<Tier>,
    /// The kink sites it flags; empty for a tier-3 chart.
    pub kinks: &'static [Kink],
    /// The geometry of those of `kinks` that have one.
    pub sites: &'static [Site],
}

/// The shape sphere's pole and the exponential map's `r = 0`, one point `(½, ½)` of the chart's square: `n = −n₀` on the
/// spherical map, `n = n₀` on the exponential one.
fn sphere_centre(u: f64, v: f64, h: f64) -> bool {
    (u - 0.5).abs() <= h && (v - 0.5).abs() <= h
}

use Kink::*;

/// Every chart of [`REGISTRY`], in its order, with its declaration.
pub const DECLARATIONS: [Declaration; 11] = [
    Declaration {
        chart: "Latent",
        tier: Some(Tier::Exact),
        kinks: &[],
        sites: &[],
    },
    Declaration {
        chart: "ShapeSphere",
        tier: Some(Tier::Exact),
        kinks: &[Pole, ExpMapRZero],
        sites: &[
            Site {
                kink: Pole,
                meets: sphere_centre,
            },
            Site {
                kink: ExpMapRZero,
                meets: sphere_centre,
            },
        ],
    },
    Declaration {
        chart: "Burrau",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, KZero, SeedSwitch],
        sites: &[],
    },
    Declaration {
        chart: "EuclidPlane",
        tier: Some(Tier::Exact),
        kinks: &[SeedSwitch, FeasibilityBoundary],
        sites: &[],
    },
    Declaration {
        chart: "BurrauThetaK",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, KZero, SeedSwitch],
        sites: &[],
    },
    Declaration {
        chart: "BurrauThetaLz",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, SeedSwitch],
        sites: &[],
    },
    Declaration {
        chart: "BurrauThetaDeltaM",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, SeedSwitch],
        sites: &[],
    },
    Declaration {
        chart: "InvariantLE",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, NormaliseEdge, WarpedKAtZeroD],
        sites: &[],
    },
    Declaration {
        chart: "InvariantLK",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary, KZero, WarpedKAtZeroD],
        sites: &[],
    },
    Declaration {
        chart: "MassSimplex",
        tier: Some(Tier::Exact),
        kinks: &[FeasibilityBoundary],
        sites: &[],
    },
    Declaration {
        chart: "Mixed",
        tier: Some(Tier::Exact),
        kinks: &[Pole, KZero, NormaliseEdge],
        sites: &[],
    },
];

/// The declaration of the chart named `name`.
pub fn declaration(name: &str) -> Option<&'static Declaration> {
    DECLARATIONS.iter().find(|d| d.chart == name)
}

/// The registry names that do not declare exactly one tier in `table`: none when every chart declares one.
pub fn undeclared_in(table: &[Declaration]) -> impl Iterator<Item = &'static str> + '_ {
    REGISTRY.iter().copied().filter(|n| {
        table
            .iter()
            .filter(|d| d.chart == *n && d.tier.is_some())
            .count()
            != 1
    })
}

/// `(m, r, p)` of a decode as 15 numbers: the masses, the positions, the momenta.
pub type Components = [f64; 15];

/// The 15 components of a decoded `(m, r, p)`.
fn components(ic: &crate::decode::Ic<Dual>) -> [Dual; 15] {
    let mut c = [Dual::default(); 15];
    for (i, m) in ic.m.iter().enumerate() {
        c[i] = *m;
    }
    for (i, r) in ic.r.iter().enumerate() {
        c[3 + 2 * i] = r[0];
        c[4 + 2 * i] = r[1];
    }
    for (i, p) in ic.p.iter().enumerate() {
        c[9 + 2 * i] = p[0];
        c[10 + 2 * i] = p[1];
    }
    c
}

/// The decode's value and exact slopes at `(u, v)`: `(value, ∂/∂u, ∂/∂v)`, each of the 15 components, the decode run
/// twice at [`Dual`], one seed per column.
pub fn exact<S, C>(chart: &C, u: f64, v: f64) -> (Components, Components, Components)
where
    S: Selection,
    C: Chart<Param = f64>,
{
    let cu = components(
        &chart
            .decode::<S, Dual>(Dual::variable(u), Dual::constant(v))
            .ic,
    );
    let cv = components(
        &chart
            .decode::<S, Dual>(Dual::constant(u), Dual::variable(v))
            .ic,
    );
    (cu.map(|c| c.v), cu.map(|c| c.d), cv.map(|c| c.d))
}

/// The kink site the footprint of half-width `h` about `(u, v)` meets, if any (the module's note). A declared site's own
/// geometry is read first, so a kink whose slope jumps and stays finite is flagged. Past those, a branch change
/// (`validate` leaves `Valid`, or the decode is degenerate) is the feasibility boundary where the chart lists it; a slope
/// that is not finite or above `1/h` is the first of the chart's listed sites that divides by a quantity going to zero.
pub fn flag<S, C>(chart: &C, declared: &Declaration, u: f64, v: f64, h: f64) -> Option<Kink>
where
    S: Selection,
    C: Chart<Param = f64>,
{
    if let Some(site) = declared.sites.iter().find(|s| (s.meets)(u, v, h)) {
        return Some(site.kink);
    }
    let kinks = declared.kinks;
    let listed = |k: Kink| kinks.contains(&k).then_some(k);
    let mut hit = None;
    for du in [-h, 0.0, h] {
        for dv in [-h, 0.0, h] {
            let (pu, pv) = (u + du, v + dv);
            let left = !matches!(chart.validate(pu, pv), ValidationResult::Valid);
            if left || chart.decode::<S, f64>(pu, pv).degenerate.is_some() {
                hit = hit
                    .or(listed(FeasibilityBoundary))
                    .or(kinks.first().copied());
            }
            let (_, su, sv) = exact::<S, C>(chart, pu, pv);
            if su
                .iter()
                .chain(&sv)
                .any(|x| !x.is_finite() || x.abs() > 1.0 / h)
            {
                hit = hit
                    .or(listed(ExpMapRZero))
                    .or(listed(Pole))
                    .or(listed(KZero))
                    .or(listed(WarpedKAtZeroD))
                    .or(kinks.first().copied());
            }
        }
    }
    hit
}
