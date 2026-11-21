//! Path A (validation_ground_truth_note, "Two entry paths"; inverse_encode_contract Part 7): a generic known start,
//! through the whole front end, then the march a pixel runs.
//!
//! ```text
//! x (physical, G = 1, Σm = 1)
//!   → C, canonicalise                          (kernel::canon, inside kernel::encode)
//!   → E, encode → z                            (kernel::encode::ingest)
//!   → CHECK 1: ‖D(z) − C(x)‖_phys and κ(x)     ([`Check1`])
//!   → the latent chart through z, decoded at its pixel (½, ½), then the wrapper's march at f64
//!                                              (engine::precision: decode_only, integrate)
//!   → CHECK 2: the behaviour the tier asserts  (crate::ground_truth::tiers)
//! ```
//!
//! **The pixel.** The latent chart `z(u, v) = z₀ + (2u − 1)·q₁ + (2v − 1)·q₂` with `z₀ = z` and `q₁ = q₂ = 0`, read at
//! `(½, ½)`, where both factors are exactly 0, so the chart places `z` itself, bit for bit. The decode is the decode
//! stage's source at f64 (`engine::precision::decode_only`), not `kernel::decode` called directly: the same entry a
//! pixel's decode takes on the CPU. [`PathA::forward_matches_check1`] holds the two decodes to each other.
//!
//! **A failed round trip is a located bug** (the note's "Everything is representable in principle"): CHECK 1's
//! residual is a bug signal, never a statement that the start is out of reach. A start the encode can only clamp
//! (`lookup_clamped`, an `α`-pole) is Path B's, not Path A's: [`PathA::clamped`] reports it.

use kernel::decode::{Degenerate, Ic};
use kernel::driver::advance::Advance;
use kernel::encode::ingest::ingest;
use kernel::encode::SimplexClamp;
use kernel::generated::links::Selection;

use engine::compute::ComputeError;
use engine::precision::{decode_only, integrate_ic, MarchKey64, SimState64};

use super::{latent_sim_key, Provenance};
use crate::roundtrip::phys;

/// CHECK 1: what the front end made of the start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Check1 {
    /// `C(x)`, the start in the canonical gauge.
    pub canonical: Ic<f64>,
    /// `C`'s length scale `λ = I_in^{−1/2}`: lengths are multiplied by it, times by `λ^{3/2}`.
    pub lambda: f64,
    /// `z = E(x)`.
    pub z: [f64; 8],
    /// `D(z)`, the free decode of `z`.
    pub decoded: Ic<f64>,
    /// `‖D(z) − C(x)‖_phys` (inverse_encode_contract Part 4), `C(x)` the reference.
    pub residual: f64,
    /// `κ(x)`, the encode's weighted Jacobian's largest singular value at the start.
    pub kappa: f64,
    /// Whether the encode clamped (`lookup_clamped`): the start sits on a pole the chart reaches only by clamping.
    pub clamped: bool,
}

/// CHECK 1 of the start `x` (`G = 1`, `Σm = 1`) under the link selection `S`.
pub fn check1<S>(x: &Ic<f64>) -> Check1
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let g = ingest::<S, f64, _>(x, phys);
    Check1 {
        canonical: g.encoded.canonical.ic,
        lambda: g.encoded.canonical.lambda,
        z: g.encoded.z.z,
        decoded: g.decoded,
        residual: g.residual,
        kappa: g.kappa,
        clamped: g.encoded.report.lookup_clamped(),
    }
}

/// The forward leg: the pixel at `z` decoded under `S` at f64, then marched by the occupant `A` under `key`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forward {
    /// The pixel's decoded start.
    pub start: Ic<f64>,
    /// The decoder's `DEGENERATE` tag, if any.
    pub degenerate: Option<Degenerate>,
    /// The march's `SimState`.
    pub state: SimState64,
}

/// The forward leg from `z` (the module's note).
pub fn forward<S, A>(z: &[f64; 8], key: &MarchKey64) -> Result<Forward, ComputeError>
where
    S: Selection,
    A: Advance,
{
    let chart = kernel::chart::Latent::<f64> {
        z0: *z,
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    let out = decode_only::<_, S>(&chart, [0.5, 0.5], &latent_sim_key(*z))?;
    Ok(Forward {
        start: out.ic,
        degenerate: out.degenerate,
        state: integrate_ic::<A>(&out.ic, out.degenerate, key),
    })
}

/// A start through Path A: CHECK 1 and the forward leg, with the run's provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct PathA {
    /// CHECK 1.
    pub check1: Check1,
    /// The forward leg.
    pub forward: Forward,
    /// What the march ran under.
    pub provenance: Provenance,
}

impl PathA {
    /// Whether the forward leg's decode is CHECK 1's `D(z)`, bit for bit: the pixel's decode and the free decode are
    /// the one decoder.
    pub fn forward_matches_check1(&self) -> bool {
        format!("{:?}", self.forward.start) == format!("{:?}", self.check1.decoded)
    }
}

/// `x` through Path A under the link selection `S`, marched by the occupant `A` under `key`; `provenance` names the
/// fixture, the occupant and the links (its horizon, step and `r_coll` are read from `key` and `horizon` here).
pub fn run<S, A>(
    x: &Ic<f64>,
    key: &MarchKey64,
    horizon: f64,
    names: (&str, &'static str, &'static str),
) -> Result<PathA, ComputeError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
    A: Advance,
{
    let check1 = check1::<S>(x);
    let forward = forward::<S, A>(&check1.z, key)?;
    Ok(PathA {
        check1,
        forward,
        provenance: Provenance {
            fixture: names.0.to_string(),
            path: "A",
            occupant: names.1,
            selection: names.2,
            horizon,
            dt_macro: key.loop_values.dt_macro,
            r_coll: f64::from(key.radius.root()),
        },
    })
}
