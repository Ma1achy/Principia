//! The trajectory interface's exact fixtures' constants (chart_composition § 18.7; R-918, R-919 F18; TASK-M2-82), in a
//! crate below `engine` and `validation` so that `engine`'s providers and tests and `validation`'s `figure_eight()`
//! read the same numbers.
//!
//! Every value here is fixture data, the inputs of exactly known trajectories that tests assert on, not a settled
//! default or threshold of the instrument, so not a constants-register entry (REQ-SYS-082; `engine::fixtures` holds
//! its harness values the same way).

/// The figure-eight's published constants (Chenciner & Montgomery 2000; Simó's digits), in `G = 1`, `mᵢ = 1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigureEight {
    /// `x₁`; `x₂ = −x₁`, `x₃ = 0` (dd_integrator § 5, test 2).
    pub x1: [f64; 2],
    /// `v₃`; `v₁ = v₂ = −v₃/2` (dd_integrator § 5, test 2).
    pub v3: [f64; 2],
    /// The period `T₈` (dd_validation_orbits § 0, the period § 1.2 points to).
    pub period: f64,
}

/// The figure-eight's constants (dd_integrator § 5, test 2; dd_validation_orbits § 0 and § 1.2).
pub const fn figure_eight() -> FigureEight {
    FigureEight {
        x1: [0.970_004_36, -0.243_087_53],
        v3: [-0.932_407_37, -0.864_731_46],
        period: 6.325_913_98,
    }
}

/// The Lagrange equilateral homographic fixture's inputs (dd_validation_orbits § 1.3: "valid for any mass ratios"):
/// unequal masses, so a test of "any masses" is one; the elliptic member's eccentricity; and each member's quantum as
/// a division of its period or, for the homothetic one, of its collapse time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lagrange {
    /// The masses, dyadic so that `Σmᵢ = 1` exactly.
    pub masses: [f64; 3],
    /// The elliptic member's eccentricity, started at pericentre.
    pub eccentricity: f64,
    /// The circular and elliptic members' boundaries per period: `Δ = P/8`, so boundaries `0` to `7` lie within one
    /// period, `kΔ < 2π/ω`, and boundary 8 is the period itself (§ 18.7).
    pub per_period: f64,
    /// The homothetic member's boundaries per collapse time, `Δ = t_c/5.5`: the collision falls between boundaries 5
    /// and 6.
    pub per_collapse: f64,
    /// The homothetic member at `Δ = t_c/4`: the collision falls on boundary 4.
    pub per_collapse_on_boundary: f64,
}

/// The Lagrange fixture's inputs.
pub const fn lagrange() -> Lagrange {
    Lagrange {
        masses: [0.5, 0.3125, 0.1875],
        eccentricity: 0.5,
        per_period: 8.0,
        per_collapse: 5.5,
        per_collapse_on_boundary: 4.0,
    }
}
