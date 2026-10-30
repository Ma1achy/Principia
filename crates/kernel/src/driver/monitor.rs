//! The invariant monitor (integrator_contract Part 1, "COM projection and invariant monitoring"; dd_integrator §3.5;
//! REQ-INT-012): the energy and `L_z` are monitored, never restored (REQ-INT-011).
//!
//! **Before the loop** ([`Monitor::new`]) it evaluates and stores, on the IC,
//!
//! ```text
//! E₀   = Σᵢ ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/‖rᵢ − rⱼ‖        L_{z,0} = Σᵢ (xᵢ p_{y,i} − yᵢ p_{x,i})
//! ```
//!
//! **After each `STEP`, post-projection** ([`Monitor::update`]) it evaluates `E(t)` and `L_z(t)` and accumulates
//! `ΔE_final = E(t) − E₀`, `max|ΔE|`, `ΔL_z final = L_z(t) − L_{z,0}` and `max|ΔL_z|`, the worst deviation so far. The
//! energy reads the force evaluation's pairwise distances ([`crate::physics::energy`]; REQ-PERF-001), so its marginal
//! cost is the kinetic term and two comparisons.
//!
//! **The relative forms** ([`relative`]) divide by a floor, `δ_E = ΔE / max(ε_E, |E₀|)` and
//! `δ_L = ΔL_z / max(ε_L, |L_{z,0}|)`, so a near-zero initial invariant (a rest start's `L_{z,0} = 0`) does not blow up.
//! `ε_E` and `ε_L` are the sim key's (integrator_contract Part 3; 10⁻⁶ by default, dd_integrator §3.5), passed in where a
//! relative form is read, not held here: the march latches the absolute drifts, which the payload stores (`E_0`, `Lz_0`
//! and `packed_b`'s `dE_max`, `dLz_max`; payload §1, §2), and the relative energy drift the energy-suspect predicate
//! reads is formed at read from them (payload §5; the generated read side's `suspect_energy`).
//!
//! **Final and max, both** (dd_integrator §3.5): the shape of the drift is the diagnostic. A spike that recovers before
//! the run ends, close-encounter stress, shows as `max|ΔE| ≫ |ΔE_final|` (dd_integrator §5 test 7; REQ-VAL-033); a
//! secular drift as the two near equal.
//!
//! **Latches in the working precision.** The maxima are held at `Real` during the march, the live values, and packed to
//! the payload's f16 halves once, when the run's `SimState` is written (payload §1, the precision rule's option (a)).

use crate::occupant::Geom;
use crate::physics::energy::{angular_momentum, energy};
use crate::Real;

/// `|x|`, by a comparison with zero.
#[inline]
pub fn magnitude<R: Real>(x: R) -> R {
    let zero = R::default();
    if x < zero {
        -x
    } else {
        x
    }
}

/// The larger of `a` and `b`; `a` where they are equal or either is NaN, so a latch keeps its value past a NaN.
#[inline]
pub fn larger<R: Real>(a: R, b: R) -> R {
    if b > a {
        b
    } else {
        a
    }
}

/// The relative form of a drift `delta` of an invariant whose initial value is `initial`: `delta / max(eps, |initial|)`
/// (dd_integrator §3.5), `eps > 0` the invariant's floor, `ε_E` or `ε_L`.
#[inline]
pub fn relative<R: Real>(delta: R, initial: R, eps: R) -> R {
    delta / larger(eps, magnitude(initial))
}

/// The run's monitored invariants: the references stored before the loop and the drifts accumulated after each step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monitor<R> {
    /// `E₀`, the energy of the IC.
    e_0: R,
    /// `L_{z,0}`, the angular momentum of the IC.
    lz_0: R,
    /// `ΔE` after the last step taken, `E(t) − E₀`; 0 before the first.
    de_final: R,
    /// `max|ΔE|` over the steps taken; 0 before the first.
    de_max: R,
    /// `ΔL_z` after the last step taken; 0 before the first.
    dlz_final: R,
    /// `max|ΔL_z|` over the steps taken; 0 before the first.
    dlz_max: R,
}

impl<R: Real> Monitor<R> {
    /// The monitor of a run starting from masses `m`, positions `r`, momenta `p`, whose force evaluation is `geom`: `E₀`
    /// and `L_{z,0}` evaluated and stored, before the loop, and every drift 0.
    #[inline]
    pub fn new(m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3], geom: &Geom<R>) -> Self {
        let zero = R::default();
        Monitor {
            e_0: energy(m, p, &geom.pair),
            lz_0: angular_momentum(r, p),
            de_final: zero,
            de_max: zero,
            dlz_final: zero,
            dlz_max: zero,
        }
    }

    /// Accumulates the state after one projected step: masses `m`, positions `r`, momenta `p` and the force
    /// evaluation `geom` at `r`. `E(t)` and `L_z(t)` are evaluated; the finals are replaced and the maxima raised.
    #[inline]
    pub fn update(&mut self, m: &[R; 3], r: &[[R; 2]; 3], p: &[[R; 2]; 3], geom: &Geom<R>) {
        let de = energy(m, p, &geom.pair) - self.e_0;
        let dlz = angular_momentum(r, p) - self.lz_0;
        self.de_final = de;
        self.de_max = larger(self.de_max, magnitude(de));
        self.dlz_final = dlz;
        self.dlz_max = larger(self.dlz_max, magnitude(dlz));
    }

    /// `E₀`.
    #[inline]
    pub fn e_0(&self) -> R {
        self.e_0
    }

    /// `L_{z,0}`.
    #[inline]
    pub fn lz_0(&self) -> R {
        self.lz_0
    }

    /// `ΔE_final`, signed.
    #[inline]
    pub fn de_final(&self) -> R {
        self.de_final
    }

    /// `max|ΔE|`.
    #[inline]
    pub fn de_max(&self) -> R {
        self.de_max
    }

    /// `ΔL_z` final, signed.
    #[inline]
    pub fn dlz_final(&self) -> R {
        self.dlz_final
    }

    /// `max|ΔL_z|`.
    #[inline]
    pub fn dlz_max(&self) -> R {
        self.dlz_max
    }

    /// `δ_E` of the final drift, `ΔE_final / max(ε_E, |E₀|)`.
    #[inline]
    pub fn delta_e_final(&self, eps_e: R) -> R {
        relative(self.de_final, self.e_0, eps_e)
    }

    /// The relative form of `max|ΔE|`, `max|ΔE| / max(ε_E, |E₀|)`.
    #[inline]
    pub fn delta_e_max(&self, eps_e: R) -> R {
        relative(self.de_max, self.e_0, eps_e)
    }

    /// `δ_L` of the final drift, `ΔL_z final / max(ε_L, |L_{z,0}|)`.
    #[inline]
    pub fn delta_l_final(&self, eps_l: R) -> R {
        relative(self.dlz_final, self.lz_0, eps_l)
    }

    /// The relative form of `max|ΔL_z|`, `max|ΔL_z| / max(ε_L, |L_{z,0}|)`.
    #[inline]
    pub fn delta_l_max(&self, eps_l: R) -> R {
        relative(self.dlz_max, self.lz_0, eps_l)
    }
}
