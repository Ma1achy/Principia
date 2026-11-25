//! Chazy's final-motion classifier (chazy drill-down §3; R-442): at every playhead, a label for what kind of final
//! motion the three bodies are in, as far as can be told at `t`, from the live state, the masses, `E_0`, the initial
//! hyperradius `R_0 = √(I_0/M)` and one running latch, `I_max`, the greatest moment of inertia met so far (§6). It
//! reads nothing after the playhead (the live-playhead test, §3.1), writes nothing to the payload and changes no event
//! class: the label is derived at read (§5.1; payload §5), through the read side's `chazy_class`.
//!
//! **Transcribed, not re-derived.** Every definition and the rule's order are the drill-down's transcription of prin-rs
//! PR #32 (`src/chazy.rs::classify`, `pair_kup`, `out_kup`, `kepler`, `ma_stable`), with R-548's three changes and
//! R-595's reading of them: transient before B-stable, B-stable requiring `E < 0` (step 4); B-stable before excursion
//! inside the hierarchy step (step 2); H, HP, P and the two-stage H only at `E ≥ 0` (steps 1 and 2), an `E < 0` state
//! that passes the dispersal step's geometric tests going on to the hierarchy step, where it may read HE, PE, B-stable
//! or excursion.
//!
//! **The two physics choices** (§3.4): a pair's binding is a sign, `K < U`, never the band, so an eccentric binary at
//! pericentre (`K/U = (1 + e)/2`) is still bound; and parabolic is a band, `|K/U − 1| ≤ ε`, scored with its hyperbolic
//! neighbour ([`Motion::coarse`]).
//!
//! **Softening.** §3.1 softens both two-body potentials at `ε_soft ≠ 0` (R-792). The kernel's force carries no
//! softening, so `U = m_i m_j / d` here, the drill-down's `ε_soft = 0` case.
//!
//! **One rule at every width.** Generic over the kernel's `Real` with `Float`'s functions (R-265), so the CPU at f64 and
//! the GPU at f32 instantiate the one rule; the parameters are the constants register's, read at the classifier's width
//! ([`Width`]). The labels may differ between widths within ulps of a sign change: a label on a real trajectory,
//! reported, not asserted (§8; R-548).
//!
//! **The latch's storage** ([`i_max_code`], [`i_max_decode`]; R-545): `I_max` is stored in the descriptor's bits 10–14
//! as its level in half-octaves above `I_0`, 32 levels to `2¹⁶`. The plan is unclear here: the ruling gives "log,
//! half-octave, 32 levels to 2¹⁶" without saying what the levels count from. Chose `I_max / I_0`, because `I_max ≥ I_0`
//! by construction (the latch starts at `I_0`), so level 0 is the fresh latch and decodes to `I_0` exactly, and the
//! read side recovers `I_0 = M·R_0²` from the inputs the classifier already reads, with no new stored field.

use spirv_std::num_traits::Float;

use crate::driver::advance::Width;
use crate::generated::constants::{CHAZY_DELTA, CHAZY_EPSILON, CHAZY_H_FAR, CHAZY_OS_RATIO};
use crate::shape::shape;
use crate::Real;

/// The float type the classifier runs at: the kernel's `Real` with `Float`'s functions, its constants read at its width.
/// f32 and f64.
pub trait ChazyReal: Real + Float + Width {}

impl<R: Real + Float + Width> ChazyReal for R {}

/// The classifier's ten labels (§3.8), with the codes the read side's `chazy_class` gives them. `Unresolved` is the
/// Fewbody reference's alone (§3.7), and the classifier never returns it.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motion {
    /// All three separate hyperbolically.
    H = 0,
    /// All three separate, one or two pairs parabolically.
    Hp = 1,
    /// All three separate parabolically.
    P = 2,
    /// One body escapes hyperbolically from a bound pair.
    He = 3,
    /// One body escapes parabolically from a bound pair.
    Pe = 4,
    /// A bound hierarchy that passes Mardling–Aarseth (2001): the one bound label that is a prediction.
    BStable = 5,
    /// Bound so far: `E < 0` and nothing above.
    BSoFar = 6,
    /// `E > 0` and nobody has left yet.
    Transient = 7,
    /// Bound, and has been at least `os_ratio` times as spread out as it is now.
    OsCandidate = 8,
    /// Far out and receding, but bound to its pair and not certified stable.
    Excursion = 9,
}

impl Motion {
    /// The ten labels in code order.
    pub const ALL: [Motion; 10] = [
        Motion::H,
        Motion::Hp,
        Motion::P,
        Motion::He,
        Motion::Pe,
        Motion::BStable,
        Motion::BSoFar,
        Motion::Transient,
        Motion::OsCandidate,
        Motion::Excursion,
    ];

    /// The label's code, `0..10`: what the read side's `chazy_class` reads.
    #[inline]
    pub fn code(self) -> u32 {
        self as u32
    }

    /// The label of `code`, or `None` for a code above 9.
    #[inline]
    pub fn from_code(code: u32) -> Option<Motion> {
        if code < 10 {
            Some(Motion::ALL[code as usize])
        } else {
            None
        }
    }

    /// Whether the label is committed (prin-rs's `Motion::committed`): H, HP, P, HE, PE and B-stable. The other four
    /// are the working states (§5.4).
    #[inline]
    pub fn committed(self) -> bool {
        matches!(
            self,
            Motion::H | Motion::Hp | Motion::P | Motion::He | Motion::Pe | Motion::BStable
        )
    }

    /// The label scored with its hyperbolic neighbour (§3.4): HP and P → H, PE → HE, every other label itself.
    /// Parabolic has zero measure, so a finite run cannot certify it.
    #[inline]
    pub fn coarse(self) -> Motion {
        match self {
            Motion::Hp | Motion::P => Motion::H,
            Motion::Pe => Motion::He,
            other => other,
        }
    }

    /// The label as the drill-down writes it.
    pub fn name(self) -> &'static str {
        match self {
            Motion::H => "H",
            Motion::Hp => "HP",
            Motion::P => "P",
            Motion::He => "HE",
            Motion::Pe => "PE",
            Motion::BStable => "B-stable",
            Motion::BSoFar => "B-so-far",
            Motion::Transient => "transient",
            Motion::OsCandidate => "OS-candidate",
            Motion::Excursion => "excursion",
        }
    }
}

/// Which test commits a separation (§3.6; prin-rs's `Gate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// The default: dispersal needs every pair farther than `h_far · R_0`, a hierarchy needs `ρ_b > h_far`.
    Hierarchy,
    /// Fewbody's isolation: dispersal needs `E > 0`, a hierarchy needs the tidal ratio below `δ`.
    Tidal,
}

/// The classifier's settings (§3.3; prin-rs's `ChazyCfg`). [`ChazyCfg::defaults`] is the register's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChazyCfg<R> {
    /// The commit gate.
    pub gate: Gate,
    /// The hierarchy ratio a body passes to commit, and the dispersal distance in units of `R_0`.
    pub h_far: R,
    /// The parabolic band's half-width in `K/U`.
    pub eps: R,
    /// `I_max / I` above which a bound system reads OS-candidate.
    pub os_ratio: R,
    /// The tidal tolerance, read by the tidal gate alone.
    pub delta: R,
    /// Whether the settling gate is on (§3.6). Off by default: it was measured as pure delay (§4.1). Off is a flag,
    /// never `τ = ∞`, which a shader cannot hold.
    pub settling: bool,
    /// The settling gate's bound on the shape-sphere chord over one snapshot; read only where `settling` is on.
    pub tau: R,
    /// Whether Mardling–Aarseth may certify B-stable.
    pub mardling_aarseth: bool,
}

impl<R: ChazyReal> ChazyCfg<R> {
    /// `ChazyCfg::default()` (§3.3): the hierarchy gate, the register's `h_far`, `ε`, `os_ratio` and `δ`, settling off,
    /// Mardling–Aarseth on.
    #[inline]
    pub fn defaults() -> Self {
        ChazyCfg {
            gate: Gate::Hierarchy,
            h_far: R::at_width(CHAZY_H_FAR as f32, CHAZY_H_FAR),
            eps: R::at_width(CHAZY_EPSILON as f32, CHAZY_EPSILON),
            os_ratio: R::at_width(CHAZY_OS_RATIO as f32, CHAZY_OS_RATIO),
            delta: R::at_width(CHAZY_DELTA as f32, CHAZY_DELTA),
            settling: false,
            tau: R::default(),
            mardling_aarseth: true,
        }
    }
}

/// What the classifier reads at a playhead (§3.1): the masses, the live positions and velocities, `E = E_0`, `R_0` and
/// the latch `I_max`. Velocities, not momenta: [`Playhead::from_momenta`] converts a stored state's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playhead<R> {
    pub m: [R; 3],
    pub r: [[R; 2]; 3],
    pub v: [[R; 2]; 3],
    /// The total energy at `t = 0`, `E_0`.
    pub e: R,
    /// The initial hyperradius `√(I_0/M)`.
    pub r_0: R,
    /// The running maximum of `I`.
    pub i_max: R,
}

impl<R: ChazyReal> Playhead<R> {
    /// The playhead of a stored state: masses `m`, positions `r`, momenta `p` (`v = p/m`).
    #[inline]
    pub fn from_momenta(m: [R; 3], r: [[R; 2]; 3], p: [[R; 2]; 3], e: R, r_0: R, i_max: R) -> Self {
        let v = [
            [p[0][0] / m[0], p[0][1] / m[0]],
            [p[1][0] / m[1], p[1][1] / m[1]],
            [p[2][0] / m[2], p[2][1] / m[2]],
        ];
        Playhead {
            m,
            r,
            v,
            e,
            r_0,
            i_max,
        }
    }
}

/// The settling gate's previous snapshot (§3.6): the shape vector `n_prev`, held in a second latch, and whether there is
/// one. With settling off it is never read; [`Previous::none`] is then all a caller passes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Previous<R> {
    pub n: [R; 3],
    pub held: bool,
}

impl<R: ChazyReal> Previous<R> {
    /// No previous snapshot: under settling, nothing is settled yet.
    #[inline]
    pub fn none() -> Self {
        Previous {
            n: [R::default(); 3],
            held: false,
        }
    }

    /// The previous snapshot's shape vector `n`.
    #[inline]
    pub fn at(n: [R; 3]) -> Self {
        Previous { n, held: true }
    }
}

/// A two-body kinetic and potential energy and radial velocity (§3.1): `K = ½ μ |Δv|²`, `U` the potential's magnitude,
/// `v_r = Δr·Δv / d` (receding when positive) and the separation `d`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kup<R> {
    pub k: R,
    pub u: R,
    pub v_r: R,
    pub d: R,
}

/// The two-body `K`, `U`, `v_r` and `d` of a relative motion `(dr, dv)` with reduced mass `mu` and potential product
/// `mm` (`U = mm / d`).
#[inline]
fn kup<R: ChazyReal>(dr: [R; 2], dv: [R; 2], mu: R, mm: R) -> Kup<R> {
    let half = R::one() / (R::one() + R::one());
    let d = (dr[0] * dr[0] + dr[1] * dr[1]).square_root();
    Kup {
        k: half * mu * (dv[0] * dv[0] + dv[1] * dv[1]),
        u: mm / d,
        v_r: (dr[0] * dv[0] + dr[1] * dv[1]) / d,
        d,
    }
}

/// Pair `(i, j)`'s two-body energies (`pair_kup`, §3.1): `μ = m_i m_j / (m_i + m_j)`, `Δr = r_j − r_i`,
/// `Δv = v_j − v_i`, `U = m_i m_j / d`.
#[inline]
pub fn pair_kup<R: ChazyReal>(ph: &Playhead<R>, i: usize, j: usize) -> Kup<R> {
    let (m, r, v) = (&ph.m, &ph.r, &ph.v);
    let dr = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
    let dv = [v[j][0] - v[i][0], v[j][1] - v[i][1]];
    kup(dr, dv, m[i] * m[j] / (m[i] + m[j]), m[i] * m[j])
}

/// The centre of mass of bodies `i` and `j`, its velocity and their mass `M_p`.
#[inline]
fn centre<R: ChazyReal>(ph: &Playhead<R>, i: usize, j: usize) -> ([R; 2], [R; 2], R) {
    let (m, r, v) = (&ph.m, &ph.r, &ph.v);
    let mp = m[i] + m[j];
    let c = [
        (m[i] * r[i][0] + m[j] * r[j][0]) / mp,
        (m[i] * r[i][1] + m[j] * r[j][1]) / mp,
    ];
    let w = [
        (m[i] * v[i][0] + m[j] * v[j][0]) / mp,
        (m[i] * v[i][1] + m[j] * v[j][1]) / mp,
    ];
    (c, w, mp)
}

/// The outer body `b`'s two-body energies against the centre of mass of `(i, j)` (`out_kup`, §3.1):
/// `M_p = m_i + m_j`, `Δr = r_b − R_c`, `Δv = v_b − V_c`, `μ = m_b M_p / (m_b + M_p)`, `U = m_b M_p / d`.
#[inline]
pub fn out_kup<R: ChazyReal>(ph: &Playhead<R>, b: usize, i: usize, j: usize) -> Kup<R> {
    let (m, r, v) = (&ph.m, &ph.r, &ph.v);
    let (c, w, mp) = centre(ph, i, j);
    let dr = [r[b][0] - c[0], r[b][1] - c[1]];
    let dv = [v[b][0] - w[0], v[b][1] - w[1]];
    kup(dr, dv, m[b] * mp / (m[b] + mp), m[b] * mp)
}

/// The parabolic band of a ratio `q = K/U` (§3.1, `band`): `+1` (hyperbolic) where `q > 1 + ε`, `−1` (bound) where
/// `q < 1 − ε`, `0` (parabolic) otherwise, `|q − 1| ≤ ε`.
#[inline]
pub fn band<R: ChazyReal>(k: R, u: R, eps: R) -> i32 {
    let q = k / u;
    if q > R::one() + eps {
        1
    } else if q < R::one() - eps {
        -1
    } else {
        0
    }
}

/// Kepler elements of a relative orbit (§3.5, `kepler`): `a` (negative if unbound), `e` and the angular momentum `h`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kepler<R> {
    pub a: R,
    pub e: R,
    pub h: R,
}

/// The Kepler elements of the relative orbit `(dr, dv)` of total mass `m` (§3.5): specific energy
/// `ε_K = ½|Δv|² − m/|Δr|`, `h = Δr_x Δv_y − Δr_y Δv_x`, `a = −m / (2 ε_K)`, `e = √max(0, 1 + 2 ε_K h² / m²)`.
#[inline]
pub fn kepler<R: ChazyReal>(dr: [R; 2], dv: [R; 2], m: R) -> Kepler<R> {
    let two = R::one() + R::one();
    let d = (dr[0] * dr[0] + dr[1] * dr[1]).square_root();
    let eps_k = (dv[0] * dv[0] + dv[1] * dv[1]) / two - m / d;
    let h = dr[0] * dv[1] - dr[1] * dv[0];
    let e2 = R::one() + two * eps_k * h * h / (m * m);
    Kepler {
        a: -m / (two * eps_k),
        e: if e2 > R::default() {
            e2.square_root()
        } else {
            R::default()
        },
        h,
    }
}

/// Mardling & Aarseth (2001), planar (§3.5, `ma_stable`), for the inner pair `(i, j)` and the outer body `k`:
/// stable iff `a_out (1 − e_out) / a_in > 2.8 [(1 + q_out)(1 + e_out)/√(1 − e_out)]^0.4 × f`, `q_out = m_k / M_b`,
/// `f` 1 prograde (`h_in · h_out ≥ 0`) and 0.7 retrograde. Not a hierarchy, so false, unless `a_in > 0`, `a_out > 0`
/// and `e_out < 1`.
#[inline]
pub fn ma_stable<R: ChazyReal>(ph: &Playhead<R>, i: usize, j: usize, k: usize) -> bool {
    let (m, r, v) = (&ph.m, &ph.r, &ph.v);
    let mb = m[i] + m[j];
    let inner = kepler(
        [r[j][0] - r[i][0], r[j][1] - r[i][1]],
        [v[j][0] - v[i][0], v[j][1] - v[i][1]],
        mb,
    );
    let (c, w, _) = centre(ph, i, j);
    let outer = kepler(
        [r[k][0] - c[0], r[k][1] - c[1]],
        [v[k][0] - w[0], v[k][1] - w[1]],
        mb + m[k],
    );
    let zero = R::default();
    if !(inner.a > zero && outer.a > zero && outer.e < R::one()) {
        return false;
    }
    let q_out = m[k] / mb;
    let factor = if inner.h * outer.h >= zero {
        R::one()
    } else {
        R::at_width(0.7, 0.7)
    };
    let base = (R::one() + q_out) * (R::one() + outer.e) / (R::one() - outer.e).square_root();
    let bound = R::at_width(2.8, 2.8) * base.power(R::at_width(0.4, 0.4)) * factor;
    outer.a * (R::one() - outer.e) / inner.a > bound
}

/// The tidal ratio on pair `(i, j)` from body `b` (§3.6): `2 m_b / (m_i + m_j) · (r_pair / d_out)³`, `r_pair` the pair's
/// apocentre `a_in (1 + e_in)` where its Kepler `a_in > 0`, its separation otherwise.
#[inline]
pub fn tidal_ratio<R: ChazyReal>(ph: &Playhead<R>, i: usize, j: usize, b: usize) -> R {
    let (m, r, v) = (&ph.m, &ph.r, &ph.v);
    let mb = m[i] + m[j];
    let pair = pair_kup(ph, i, j);
    let inner = kepler(
        [r[j][0] - r[i][0], r[j][1] - r[i][1]],
        [v[j][0] - v[i][0], v[j][1] - v[i][1]],
        mb,
    );
    let r_pair = if inner.a > R::default() {
        inner.a * (R::one() + inner.e)
    } else {
        pair.d
    };
    let x = r_pair / out_kup(ph, b, i, j).d;
    (R::one() + R::one()) * m[b] / mb * x * x * x
}

/// The other two bodies of `b`, in index order: `b`'s pair.
#[inline]
pub fn others(b: usize) -> (usize, usize) {
    match b {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// The hierarchy ratio of body `b` (§3.1): its distance from its pair's centre of mass over the pair's separation,
/// `ρ_b = d_out(b) / d_pair(i, j)`.
#[inline]
pub fn hierarchy_ratio<R: ChazyReal>(ph: &Playhead<R>, b: usize) -> R {
    let (i, j) = others(b);
    out_kup(ph, b, i, j).d / pair_kup(ph, i, j).d
}

/// The body most separated from its pair (§3.1): the largest `ρ_b`, the lowest index on a tie (prin-rs's fold keeps the
/// first strictly greater).
#[inline]
pub fn most_separated<R: ChazyReal>(ph: &Playhead<R>) -> usize {
    let rho = [
        hierarchy_ratio(ph, 0),
        hierarchy_ratio(ph, 1),
        hierarchy_ratio(ph, 2),
    ];
    let mut best = 0;
    if rho[1] > rho[best] {
        best = 1;
    }
    if rho[2] > rho[best] {
        best = 2;
    }
    best
}

/// The moment of inertia about the centre of mass, `I = Σ m_k |r_k − R_com|²` (§3.1; prin-rs's `energy::inertia`).
#[inline]
pub fn inertia<R: ChazyReal>(m: &[R; 3], r: &[[R; 2]; 3]) -> R {
    let total = m[0] + m[1] + m[2];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0] + m[2] * r[2][0]) / total,
        (m[0] * r[0][1] + m[1] * r[1][1] + m[2] * r[2][1]) / total,
    ];
    let mut total_i = R::default();
    let mut k = 0;
    while k < 3 {
        let (x, y) = (r[k][0] - c[0], r[k][1] - c[1]);
        total_i = total_i + m[k] * (x * x + y * y);
        k += 1;
    }
    total_i
}

/// The initial hyperradius `R_0 = √(I_0 / M)` of masses `m` at positions `r` (§3.1; prin-rs's `energy::hyperradius`).
#[inline]
pub fn hyperradius<R: ChazyReal>(m: &[R; 3], r: &[[R; 2]; 3]) -> R {
    (inertia(m, r) / (m[0] + m[1] + m[2])).square_root()
}

/// Whether the shape has settled (§3.6): always, with settling off; otherwise there is a previous snapshot and the
/// chord `|n − n_prev|` is below `τ`, compared squared.
#[inline]
fn settled<R: ChazyReal>(ph: &Playhead<R>, cfg: &ChazyCfg<R>, prev: &Previous<R>) -> bool {
    if !cfg.settling {
        return true;
    }
    let n = shape(ph.r, ph.m);
    let d = [n[0] - prev.n[0], n[1] - prev.n[1], n[2] - prev.n[2]];
    prev.held && d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < cfg.tau * cfg.tau
}

/// The label at a playhead (§3.2, the rule in order, as R-548 and R-595 rule it). `prev` is the settling gate's
/// previous snapshot, read only with settling on.
///
/// 1. **Dispersal.** If `E ≥ 0`, every pair is farther than `h_far · R_0` and the shape has settled (under the tidal
///    gate, both replaced by `E > 0`), and every pair recedes: with no pair bound by its band, H when no pair is in the band, HP when one or
///    two are, P when all three are.
/// 2. **Hierarchy.** For the body `b` most separated from its pair, the pair's binding is a sign (`K < U`, else its
///    band); the gate is `ρ_b > h_far` and settled (under the tidal gate, both replaced by the tidal ratio below `δ`:
///    settling is the hierarchy gate's alone, as prin-rs PR #32's `classify` has it). Where it passes and `b` recedes:
///    a bound pair reads HE, PE, or (outer bound) B-stable where step 4's test holds and excursion where not; an
///    unbound pair with `b` not bound reads H, the two-stage break-up, at `E ≥ 0`. Otherwise go on.
/// 3. **Transient**, at `E > 0`.
/// 4. **B-stable**, at `E < 0`, with Mardling–Aarseth on, the pair bound, `b` bound to it and `ma_stable`.
/// 5. **OS-candidate**, where `I_max / I > os_ratio`.
/// 6. Otherwise **B-so-far**.
#[inline]
pub fn classify<R: ChazyReal>(ph: &Playhead<R>, cfg: &ChazyCfg<R>, prev: &Previous<R>) -> Motion {
    let zero = R::default();
    let settled = settled(ph, cfg, prev);

    // 1. Dispersal, at E ≥ 0 only (R-548): all three separating needs non-negative energy.
    let pairs = [pair_kup(ph, 0, 1), pair_kup(ph, 0, 2), pair_kup(ph, 1, 2)];
    let far_limit = cfg.h_far * ph.r_0;
    let far = match cfg.gate {
        Gate::Hierarchy => {
            pairs[0].d > far_limit && pairs[1].d > far_limit && pairs[2].d > far_limit && settled
        }
        Gate::Tidal => ph.e > zero,
    };
    let receding = pairs[0].v_r > zero && pairs[1].v_r > zero && pairs[2].v_r > zero;
    if ph.e >= zero && far && receding {
        let bands = [
            band(pairs[0].k, pairs[0].u, cfg.eps),
            band(pairs[1].k, pairs[1].u, cfg.eps),
            band(pairs[2].k, pairs[2].u, cfg.eps),
        ];
        if bands[0] != -1 && bands[1] != -1 && bands[2] != -1 {
            let in_band =
                u32::from(bands[0] == 0) + u32::from(bands[1] == 0) + u32::from(bands[2] == 0);
            return match in_band {
                0 => Motion::H,
                3 => Motion::P,
                _ => Motion::Hp,
            };
        }
    }

    // 2. Hierarchy: the body most separated from its pair.
    let b = most_separated(ph);
    let (i, j) = others(b);
    let pair = pair_kup(ph, i, j);
    let outer = out_kup(ph, b, i, j);
    // §3.4: a pair's binding is a sign, not a band.
    let pair_sign = if pair.k < pair.u {
        -1
    } else {
        band(pair.k, pair.u, cfg.eps)
    };
    let outer_band = band(outer.k, outer.u, cfg.eps);
    let gate = match cfg.gate {
        Gate::Hierarchy => hierarchy_ratio(ph, b) > cfg.h_far && settled,
        Gate::Tidal => tidal_ratio(ph, i, j, b) < cfg.delta,
    };
    // Step 4's test, read in step 2 too: B-stable before excursion (R-548).
    let stable = ph.e < zero
        && cfg.mardling_aarseth
        && pair_sign == -1
        && outer.k < outer.u
        && ma_stable(ph, i, j, b);
    if gate && outer.v_r > zero {
        if pair_sign == -1 {
            return match outer_band {
                1 => Motion::He,
                0 => Motion::Pe,
                _ if stable => Motion::BStable,
                _ => Motion::Excursion,
            };
        }
        if outer_band >= 0 && ph.e >= zero {
            return Motion::H;
        }
    }

    // 3. Transient: E > 0 cannot stay bound (Lagrange–Jacobi).
    if ph.e > zero {
        return Motion::Transient;
    }
    // 4. B-stable.
    if stable {
        return Motion::BStable;
    }
    // 5. OS-candidate.
    if ph.i_max / inertia(&ph.m, &ph.r) > cfg.os_ratio {
        return Motion::OsCandidate;
    }
    // 6.
    Motion::BSoFar
}

/// `I_max`'s stored level (R-545; REQ-PAY-106): the number of whole half-octaves `I_max` lies above `I_0`, the `c` with
/// `2^(c/2) ≤ I_max / I_0 < 2^((c+1)/2)`, saturating at 31 (`2^15.5` and up, 32 levels to `2¹⁶`), 0 at or below `I_0`.
/// Decided on `(I_max / I_0)²` against the exact powers of two `2^c`, so no logarithm, `√2` or non-finite constant is
/// read; a NaN ratio reads 0. Monotone in `I_max`, so a latch's level is the greatest of its segments' levels.
#[inline]
pub fn i_max_code<R: ChazyReal>(i_max: R, i_0: R) -> u32 {
    let ratio = i_max / i_0;
    let square = ratio * ratio;
    let two = R::one() + R::one();
    // The field's five bits hold levels 0–31 (R-545).
    let mut code = 0;
    let mut edge = two;
    while code < 31 && square >= edge {
        code += 1;
        edge = edge * two;
    }
    code
}

/// The stored level `code`'s value (R-545): the level's lower edge, `I_0 · 2^(code/2)`, so level 0 is `I_0` exactly
/// and a decoded latch is never above the latch it was stored from: OS-candidate, `I_max / I > os_ratio`, can then only
/// fire less often than on the unquantised latch, never where it would not.
#[inline]
pub fn i_max_decode<R: ChazyReal>(code: u32, i_0: R) -> R {
    let two = R::one() + R::one();
    let mut scale = R::one();
    let mut k = 0;
    while k < code {
        scale = scale * two;
        k += 1;
    }
    i_0 * scale.square_root()
}

/// The running maximum of `I` (§6; REQ-PAY-105): forward-only, starting at `I_0`, raised at every step by the step's
/// `I`, a NaN `I` never latched. A resumed march takes up the stored level as a floor ([`ImaxLatch::reload`]) and the
/// stored level is the greater of the floor and its own maximum's level: the level is monotone, so a march split at any
/// persist boundary stores the uncut march's level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImaxLatch<R> {
    /// `I_0`, the IC's moment of inertia, the levels' reference.
    i_0: R,
    /// The greatest `I` met by this march so far, `I_0` at the start of a run from its IC.
    max: R,
    /// A resumed march's stored level; 0 for a run from its IC.
    floor: u32,
}

impl<R: ChazyReal> ImaxLatch<R> {
    /// The latch of a run from its IC, of moment of inertia `i_0`: `I_max = I_0`.
    #[inline]
    pub fn new(i_0: R) -> Self {
        ImaxLatch {
            i_0,
            max: i_0,
            floor: 0,
        }
    }

    /// Takes up a stored level `code` as this march's floor.
    #[inline]
    pub fn reload(&mut self, code: u32) {
        self.floor = code;
    }

    /// Latches the moment of inertia `i` of a step's state: the maximum raised where `i` is above it. A NaN fails the
    /// comparison and is never latched.
    #[inline]
    pub fn observe(&mut self, i: R) {
        if i > self.max {
            self.max = i;
        }
    }

    /// The live `I_max` of this march: the greatest `I` it met, `I_0` included. A resumed march's starts again at
    /// `I_0`: its stored part is [`ImaxLatch::code`]'s floor.
    #[inline]
    pub fn live(&self) -> R {
        self.max
    }

    /// `I_0`.
    #[inline]
    pub fn i_0(&self) -> R {
        self.i_0
    }

    /// The stored level: the greater of the floor and the live maximum's level ([`i_max_code`]).
    #[inline]
    pub fn code(&self) -> u32 {
        let own = i_max_code(self.max, self.i_0);
        if own > self.floor {
            own
        } else {
            self.floor
        }
    }
}

/// The read side's `chazy_class` (§5.1; payload §5): the label's code of a stored sample, from its masses `m`, its live
/// positions `r` and momenta `p`, `E_0`, `R_0` and the stored level of `I_max`, decoded against `I_0 = M·R_0²`, under the
/// defaults with no previous snapshot (settling off). It reads nothing else.
#[inline]
pub fn chazy_class<R: ChazyReal>(
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> u32 {
    label_of(m, r, p, e_0, r_0, i_max_code).code()
}

// ── The read-side fields (§5.2–§5.6) ──────────────────────────────────────────────────────────────────
//
// `chazy_final`, `chazy_status`, `chazy_structure` and `chazy_q` are folds of the label at the playhead; `chazy_escaper`
// and `chazy_qk` read the live state and the masses alone. Every one is derived at read, nothing stored (§5.1).

/// `chazy_final`'s eight values (§5.2, §5.3): Chazy's seven classes and collision, with the codes the read side gives.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Final {
    /// Bounded, or nobody has left yet at the playhead (B-stable, B-so-far, excursion and transient).
    B = 0,
    H = 1,
    Hp = 2,
    P = 3,
    He = 4,
    Pe = 5,
    /// Oscillatory: the OS-candidate label.
    Os = 6,
    /// A collision-terminated sample: a pair, a triple or a collision at `t = 0` (§7).
    Collision = 7,
}

impl Final {
    /// The eight values in code order.
    pub const ALL: [Final; 8] = [
        Final::B,
        Final::H,
        Final::Hp,
        Final::P,
        Final::He,
        Final::Pe,
        Final::Os,
        Final::Collision,
    ];

    /// The value's code, `0..8`.
    #[inline]
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// `chazy_status`'s seven values (§5.2, §5.4), with the codes the read side gives.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// H, HP, P, HE or PE.
    Committed = 0,
    /// Committed, but shown in its own colour: the one bound label that is a prediction.
    BStable = 1,
    BSoFar = 2,
    Excursion = 3,
    Transient = 4,
    OsCandidate = 5,
    /// A collision-terminated sample (§7).
    Collision = 6,
}

impl Status {
    /// The seven values in code order.
    pub const ALL: [Status; 7] = [
        Status::Committed,
        Status::BStable,
        Status::BSoFar,
        Status::Excursion,
        Status::Transient,
        Status::OsCandidate,
        Status::Collision,
    ];

    /// The value's code, `0..7`.
    #[inline]
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// `chazy_structure`'s four values (§5.2, §5.5): which bodies are separating from which.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Structure {
    Dispersal = 0,
    Hierarchy = 1,
    Interacting = 2,
    Oscillatory = 3,
}

impl Structure {
    /// The four values in code order.
    pub const ALL: [Structure; 4] = [
        Structure::Dispersal,
        Structure::Hierarchy,
        Structure::Interacting,
        Structure::Oscillatory,
    ];

    /// The value's code, `0..4`.
    #[inline]
    pub fn code(self) -> u32 {
        self as u32
    }
}

// The folds: begin. Each is a match over every label with no wildcard arm, so a label added to `Motion` fails to build
// until each fold maps it (§12.2 item 3); `kernel/tests/chazy_fields.rs` compiles this region against an eleventh label
// to hold that. Nothing between the markers reads anything but `Motion`, `Final`, `Status` and `Structure`.

/// The label folded onto Chazy's seven (§5.3): H, HP, P, HE and PE themselves; B-stable, B-so-far, excursion (bound to
/// its pair) and transient (nobody has left yet: a display choice, not a physics claim) to B; OS-candidate to OS.
#[inline]
pub fn fold_final(label: Motion) -> Final {
    match label {
        Motion::H => Final::H,
        Motion::Hp => Final::Hp,
        Motion::P => Final::P,
        Motion::He => Final::He,
        Motion::Pe => Final::Pe,
        Motion::BStable => Final::B,
        Motion::BSoFar => Final::B,
        Motion::Excursion => Final::B,
        Motion::Transient => Final::B,
        Motion::OsCandidate => Final::Os,
    }
}

/// The label's status (§5.4): committed for H, HP, P, HE and PE; B-stable in its own value; the four working states
/// themselves.
#[inline]
pub fn fold_status(label: Motion) -> Status {
    match label {
        Motion::H => Status::Committed,
        Motion::Hp => Status::Committed,
        Motion::P => Status::Committed,
        Motion::He => Status::Committed,
        Motion::Pe => Status::Committed,
        Motion::BStable => Status::BStable,
        Motion::BSoFar => Status::BSoFar,
        Motion::Excursion => Status::Excursion,
        Motion::Transient => Status::Transient,
        Motion::OsCandidate => Status::OsCandidate,
    }
}

/// The label's structure row (§5.5, the precedence written there): the row is the label's, never the hierarchy ratio's
/// alone. Dispersal for H, HP and P, the two-stage H included; hierarchy for the hierarchy step's own labels, HE, PE
/// and excursion, and for B-stable, which Mardling–Aarseth certifies a bound hierarchy (§10.5.2's "bound hierarchy");
/// oscillatory for OS-candidate; interacting for transient and B-so-far, whatever their most separated body's ratio.
///
/// The plan is unclear here: §5.5 suggests the hierarchy row for HE, PE and excursion alone and interacting for "the
/// rest". Chose B-stable in the hierarchy row because §10.5.2 draws a bound hierarchy dark in the hierarchy hue and a
/// B-stable state is one by its own test; its `q` is then the outer `K/U`, below 1 by that test (`K_out < U_out`), so
/// the review's property (a bound label below the parabolic mid-point) holds on either row.
#[inline]
pub fn fold_structure(label: Motion) -> Structure {
    match label {
        Motion::H => Structure::Dispersal,
        Motion::Hp => Structure::Dispersal,
        Motion::P => Structure::Dispersal,
        Motion::He => Structure::Hierarchy,
        Motion::Pe => Structure::Hierarchy,
        Motion::Excursion => Structure::Hierarchy,
        Motion::BStable => Structure::Hierarchy,
        Motion::Transient => Structure::Interacting,
        Motion::BSoFar => Structure::Interacting,
        Motion::OsCandidate => Structure::Oscillatory,
    }
}

// The folds: end.

/// The label of a stored sample at the playhead ([`chazy_class`]'s): masses `m`, live positions `r` and momenta `p`,
/// `E_0`, `R_0` and the stored level of `I_max`, under the defaults with no previous snapshot.
#[inline]
pub fn label_of<R: ChazyReal>(
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> Motion {
    let i_0 = (m[0] + m[1] + m[2]) * r_0 * r_0;
    let ph = Playhead::from_momenta(m, r, p, e_0, r_0, i_max_decode(i_max_code, i_0));
    classify(&ph, &ChazyCfg::defaults(), &Previous::none())
}

/// The read side's `chazy_final` (§5.3): a collision-terminated sample (`state` is `collision`: a pair, a triple or a
/// collision at `t = 0` alike, §7) reads collision, every other sample its label folded ([`fold_final`]). An
/// escape-terminated sample's stored state is its fire-time state, so it reads the class there (§7).
#[inline]
pub fn chazy_final<R: ChazyReal>(
    state: u32,
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> u32 {
    if state == crate::payload::STATE_COLLISION {
        Final::Collision.code()
    } else {
        fold_final(label_of(m, r, p, e_0, r_0, i_max_code)).code()
    }
}

/// The read side's `chazy_status` (§5.4): collision for a collision-terminated sample, as [`chazy_final`]; otherwise
/// the label's status ([`fold_status`]).
#[inline]
pub fn chazy_status<R: ChazyReal>(
    state: u32,
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> u32 {
    if state == crate::payload::STATE_COLLISION {
        Status::Collision.code()
    } else {
        fold_status(label_of(m, r, p, e_0, r_0, i_max_code)).code()
    }
}

/// The read side's `chazy_structure` (§5.5): the label's row ([`fold_structure`]). A collision-terminated sample has
/// no structure to read; its row is its stored state's, which the stains do not draw (they draw `chazy_final`'s
/// collision, §7).
#[inline]
pub fn chazy_structure<R: ChazyReal>(
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> u32 {
    fold_structure(label_of(m, r, p, e_0, r_0, i_max_code)).code()
}

/// The smaller of `a` and `b`, `b` where they compare false.
#[inline]
fn least<R: ChazyReal>(a: R, b: R) -> R {
    if a < b {
        a
    } else {
        b
    }
}

/// `q` of the label's row at the playhead `ph` (§5.5; [`chazy_q`]).
///
/// - **Dispersal**: the smallest pair `K/U`, where no pair is bound by its band (step 1's H, HP and P, and a two-stage H
///   with every pair unbound); otherwise, a two-stage H with a bound pair `(b, i)` or `(b, j)`, the smaller of the inner
///   pair's `K/U` and the outer body's, the two ratios the rule banded to commit it. Either way `q ≥ 1 − ε`.
/// - **Hierarchy**: the most separated body's outer `K/U` against its pair.
/// - **Interacting** and **oscillatory**: the total `K/|U|` with `K = E_0 + |U|`, `q = 1 + E_0/|U|`, held at 0 from below.
///
/// The plan is unclear here on two points. "The smallest pair `K/U` (the least unbound pair)" presumes every pair
/// unbound, which a two-stage H need not have (a body co-moving with the escaper reads `K/U` near 0); chose the two
/// ratios the rule committed on, so an H, HP or P sample never draws below the parabolic mid-point (§10.5.2: dispersal
/// dark "not reachable"). And "total `K/|U|` (> 1 exactly when E > 0)" holds for the live energy only up to the march's
/// drift; chose `K = E_0 + |U|`, the energy the label reads, so `q ≥ 1` exactly when `E_0 ≥ 0` and `q ≤ 1` exactly
/// when `E_0 ≤ 0` (rounding is monotone and 1 is exact), and a transient sample is never drawn below a bound one by a
/// drift. Strictly above 1 wherever `E_0/|U|` is above half an ulp of 1.
#[inline]
pub fn q_of<R: ChazyReal>(ph: &Playhead<R>, row: Structure, eps: R) -> R {
    let zero = R::default();
    let ratio = |x: Kup<R>| x.k / x.u;
    let pairs = [pair_kup(ph, 0, 1), pair_kup(ph, 0, 2), pair_kup(ph, 1, 2)];
    let b = most_separated(ph);
    let (i, j) = others(b);
    match row {
        Structure::Dispersal => {
            let three = least(least(ratio(pairs[0]), ratio(pairs[1])), ratio(pairs[2]));
            if three >= R::one() - eps {
                three
            } else {
                least(ratio(pair_kup(ph, i, j)), ratio(out_kup(ph, b, i, j)))
            }
        }
        Structure::Hierarchy => ratio(out_kup(ph, b, i, j)),
        Structure::Interacting | Structure::Oscillatory => {
            let u = pairs[0].u + pairs[1].u + pairs[2].u;
            let q = R::one() + ph.e / u;
            if q < zero {
                zero
            } else {
                q
            }
        }
    }
}

/// The read side's `chazy_q` (§5.5): the label's row's `K/|U|` ([`q_of`]), ≥ 0, 1 parabolic; for display
/// `b = q / (1 + q)`.
#[inline]
pub fn chazy_q<R: ChazyReal>(
    m: [R; 3],
    r: [[R; 2]; 3],
    p: [[R; 2]; 3],
    e_0: R,
    r_0: R,
    i_max_code: u32,
) -> R {
    let i_0 = (m[0] + m[1] + m[2]) * r_0 * r_0;
    let ph = Playhead::from_momenta(m, r, p, e_0, r_0, i_max_decode(i_max_code, i_0));
    let cfg = ChazyCfg::defaults();
    let row = fold_structure(classify(&ph, &cfg, &Previous::none()));
    q_of(&ph, row, cfg.eps)
}

/// The read side's `chazy_escaper` (§5.6): the body most separated from its pair, the largest hierarchy ratio
/// `ρ_b`, the lowest index on a tie ([`most_separated`], `classify`'s own choice). A body index, 0-based. From
/// the live positions and the masses alone.
#[inline]
pub fn chazy_escaper<R: ChazyReal>(m: [R; 3], r: [[R; 2]; 3]) -> u32 {
    let ph = Playhead {
        m,
        r,
        v: [[R::default(); 2]; 3],
        e: R::default(),
        r_0: R::one(),
        i_max: R::default(),
    };
    match most_separated(&ph) {
        0 => 0,
        1 => 1,
        _ => 2,
    }
}

/// The read side's `chazy_qk` (§5.6): body `k`'s `K/|U|` against the centre of mass of the other two
/// (`out_kup(k)`), `k = 0, 1, 2`, each ≥ 0, 1 parabolic. From the live positions and momenta and the masses alone; on a
/// HE or PE sample `chazy_qk[chazy_escaper]` is `chazy_q`.
#[inline]
pub fn chazy_qk<R: ChazyReal>(m: [R; 3], r: [[R; 2]; 3], p: [[R; 2]; 3]) -> [R; 3] {
    let ph = Playhead::from_momenta(m, r, p, R::default(), R::one(), R::default());
    let q = |b: usize| {
        let (i, j) = others(b);
        let x = out_kup(&ph, b, i, j);
        x.k / x.u
    };
    [q(0), q(1), q(2)]
}
