//! The Chazy classifier's reference and its fixture states (chazy drill-down §3.7, §12.1, §12.2).
//!
//! **The Fewbody reference** ([`fewbody_resolved`]; Fregeau, Cheung, Portegies Zwart & Rasio 2004, MNRAS 352, 1,
//! §3.3.5), transcribed for three bodies from prin-rs PR #32's `fewbody_resolved`: resolved iff every top-level pair
//! recedes, the tidal ratio on every binary is below `δ`, every tree is Mardling–Aarseth stable and the top-level energy
//! is positive. It is the reference the classifier is measured against, never a candidate (REQ-VAL-184), so it reads
//! none of the kernel's classifier: its two-body energies, Kepler elements and Mardling–Aarseth test are its own,
//! written here at f64 from the drill-down's definitions (§3.1, §3.5). Its `Unresolved` is the reference's alone.
//!
//! **The fixture states** (§12.1, transcribed from prin-rs PR #32's `tests/chazy.rs`; §12.2 items 10 and 11 and R-595's
//! arm, written here): equal masses, a third each, `G = M = 1`, every state shifted to the centre-of-mass frame. The
//! kernel's tests classify them with `R_0 = 1` and a fresh latch, `I_max = I`, as §12.1 says, unless a test says
//! otherwise.

/// A fixture state: masses, positions and velocities, in the centre-of-mass frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub m: [f64; 3],
    pub r: [[f64; 2]; 3],
    pub v: [[f64; 2]; 3],
}

/// Equal masses, a third each (§12.1).
pub const THIRDS: [f64; 3] = [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];

impl State {
    /// The state of masses `m`, positions `r` and velocities `v`, shifted to its centre-of-mass frame.
    pub fn com(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> Self {
        let total = m[0] + m[1] + m[2];
        let mean =
            |q: &[[f64; 2]; 3], c: usize| (0..3).map(|k| m[k] * q[k][c]).sum::<f64>() / total;
        let (rc, vc) = ([mean(&r, 0), mean(&r, 1)], [mean(&v, 0), mean(&v, 1)]);
        State {
            m,
            r: r.map(|q| [q[0] - rc[0], q[1] - rc[1]]),
            v: v.map(|q| [q[0] - vc[0], q[1] - vc[1]]),
        }
    }

    /// Momenta `p = m v`.
    pub fn p(&self) -> [[f64; 2]; 3] {
        [0, 1, 2].map(|k| [self.m[k] * self.v[k][0], self.m[k] * self.v[k][1]])
    }

    /// The total energy, `Σ ½ m v² − Σ m_i m_j / d_ij` (`G = 1`, unsoftened).
    pub fn energy(&self) -> f64 {
        let kinetic: f64 = (0..3)
            .map(|k| 0.5 * self.m[k] * (self.v[k][0].powi(2) + self.v[k][1].powi(2)))
            .sum();
        let potential: f64 = PAIRS
            .iter()
            .map(|&(i, j)| self.m[i] * self.m[j] / dist(self.r[i], self.r[j]))
            .sum();
        kinetic - potential
    }

    /// The moment of inertia about the centre of mass, `Σ m |r − R_com|²`.
    pub fn inertia(&self) -> f64 {
        let total: f64 = self.m.iter().sum();
        let c = [0, 1].map(|a| (0..3).map(|k| self.m[k] * self.r[k][a]).sum::<f64>() / total);
        (0..3)
            .map(|k| self.m[k] * ((self.r[k][0] - c[0]).powi(2) + (self.r[k][1] - c[1]).powi(2)))
            .sum()
    }

    /// Pair `(i, j)`'s two-body energies: `K = ½ μ |Δv|²`, `U = m_i m_j / d`, `v_r = Δr·Δv / d`.
    pub fn pair(&self, i: usize, j: usize) -> TwoBody {
        let mu = self.m[i] * self.m[j] / (self.m[i] + self.m[j]);
        two_body(
            sub(self.r[j], self.r[i]),
            sub(self.v[j], self.v[i]),
            mu,
            self.m[i] * self.m[j],
        )
    }

    /// Body `b`'s two-body energies against the centre of mass of `(i, j)`.
    pub fn outer(&self, b: usize, i: usize, j: usize) -> TwoBody {
        let (c, w, mp) = self.centre(i, j);
        let mu = self.m[b] * mp / (self.m[b] + mp);
        two_body(sub(self.r[b], c), sub(self.v[b], w), mu, self.m[b] * mp)
    }

    fn centre(&self, i: usize, j: usize) -> ([f64; 2], [f64; 2], f64) {
        let (m, mp) = (self.m, self.m[i] + self.m[j]);
        let at = |q: &[[f64; 2]; 3]| [0, 1].map(|a| (m[i] * q[i][a] + m[j] * q[j][a]) / mp);
        (at(&self.r), at(&self.v), mp)
    }

    /// The Kepler elements of pair `(i, j)`'s relative orbit.
    pub fn pair_kepler(&self, i: usize, j: usize) -> Kepler {
        kepler(
            sub(self.r[j], self.r[i]),
            sub(self.v[j], self.v[i]),
            self.m[i] + self.m[j],
        )
    }

    /// The Kepler elements of body `k`'s orbit about the centre of mass of `(i, j)`.
    pub fn outer_kepler(&self, k: usize, i: usize, j: usize) -> Kepler {
        let (c, w, mp) = self.centre(i, j);
        kepler(sub(self.r[k], c), sub(self.v[k], w), mp + self.m[k])
    }
}

/// The three pairs, `(0, 1)`, `(0, 2)`, `(1, 2)`.
pub const PAIRS: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = sub(a, b);
    d[0].hypot(d[1])
}

/// A two-body kinetic energy, potential magnitude, radial velocity and separation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBody {
    pub k: f64,
    pub u: f64,
    pub v_r: f64,
    pub d: f64,
}

fn two_body(dr: [f64; 2], dv: [f64; 2], mu: f64, mm: f64) -> TwoBody {
    let d = dr[0].hypot(dr[1]);
    TwoBody {
        k: 0.5 * mu * (dv[0] * dv[0] + dv[1] * dv[1]),
        u: mm / d,
        v_r: (dr[0] * dv[0] + dr[1] * dv[1]) / d,
        d,
    }
}

/// Kepler elements: `a` (negative if unbound), `e`, `h`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kepler {
    pub a: f64,
    pub e: f64,
    pub h: f64,
}

/// The Kepler elements of the relative orbit `(dr, dv)` of total mass `m` (§3.5).
pub fn kepler(dr: [f64; 2], dv: [f64; 2], m: f64) -> Kepler {
    let eps = 0.5 * (dv[0] * dv[0] + dv[1] * dv[1]) - m / dr[0].hypot(dr[1]);
    let h = dr[0] * dv[1] - dr[1] * dv[0];
    Kepler {
        a: -m / (2.0 * eps),
        e: (1.0 + 2.0 * eps * h * h / (m * m)).max(0.0).sqrt(),
        h,
    }
}

/// Mardling & Aarseth (2001), planar (§3.5): the inner pair `(i, j)` and the outer body `k`; false where it is not a
/// hierarchy (`a_in ≤ 0`, `a_out ≤ 0` or `e_out ≥ 1`).
pub fn ma_stable(s: &State, i: usize, j: usize, k: usize) -> bool {
    let inner = s.pair_kepler(i, j);
    let outer = s.outer_kepler(k, i, j);
    if !(inner.a > 0.0 && outer.a > 0.0 && outer.e < 1.0) {
        return false;
    }
    let q_out = s.m[k] / (s.m[i] + s.m[j]);
    let factor = if inner.h * outer.h >= 0.0 { 1.0 } else { 0.7 };
    let bound = 2.8 * ((1.0 + q_out) * (1.0 + outer.e) / (1.0 - outer.e).sqrt()).powf(0.4) * factor;
    outer.a * (1.0 - outer.e) / inner.a > bound
}

/// The reference's verdicts (§3.7): H, HE, B-stable or unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    H,
    He,
    BStable,
    Unresolved,
}

/// The tidal ratio Fewbody reads on the bound pair `(i, j)` from body `k`: `2 m_k / (m_i + m_j) · (a (1 + e) / d_out)³`.
pub fn fewbody_tidal(s: &State, i: usize, j: usize, k: usize) -> f64 {
    let inner = s.pair_kepler(i, j);
    let x = inner.a * (1.0 + inner.e) / s.outer(k, i, j).d;
    2.0 * s.m[k] / (s.m[i] + s.m[j]) * x.powi(3)
}

/// Fewbody's termination test for three bodies (§3.7; prin-rs PR #32's `fewbody_resolved`), at tidal tolerance `delta`:
/// the pair with positive Kepler `a` and the smallest `a`; with none, H where every pair recedes and `E > 0`; with a bound
/// pair `(i, j)` and third body `k`, where `k` is bound to it (`K_out < U_out`) B-stable if Mardling–Aarseth holds, and
/// where it is not, HE if `k` recedes and the tidal ratio is below `delta`; unresolved otherwise.
pub fn fewbody_resolved(s: &State, delta: f64) -> Reference {
    let bound = PAIRS
        .iter()
        .map(|&(i, j)| (i, j, s.pair_kepler(i, j).a))
        .filter(|&(_, _, a)| a > 0.0)
        .fold(None, |best: Option<(usize, usize, f64)>, c| match best {
            Some(b) if b.2 <= c.2 => Some(b),
            _ => Some(c),
        });
    let Some((i, j, _)) = bound else {
        let receding = PAIRS.iter().all(|&(i, j)| s.pair(i, j).v_r > 0.0);
        return if receding && s.energy() > 0.0 {
            Reference::H
        } else {
            Reference::Unresolved
        };
    };
    let k = 3 - i - j;
    let outer = s.outer(k, i, j);
    if outer.k < outer.u {
        if ma_stable(s, i, j, k) {
            Reference::BStable
        } else {
            Reference::Unresolved
        }
    } else if outer.v_r > 0.0 && fewbody_tidal(s, i, j, k) < delta {
        Reference::He
    } else {
        Reference::Unresolved
    }
}

/// The equilateral directions `(1, 0)`, `(−½, √3/2)`, `(−½, −√3/2)` (§12.1).
pub fn equilateral() -> [[f64; 2]; 3] {
    let h = 3f64.sqrt() / 2.0;
    [[1.0, 0.0], [-0.5, h], [-0.5, -h]]
}

/// `binary_plus_escaper(a, e, d, q_out)` (§12.1): bodies 0 and 1 on a Kepler orbit of semimajor axis `a` and
/// eccentricity `e`, at pericentre, `r_p = a (1 − e)`, `v_p = √(M_b (1 + e) / r_p)`, at `(0, −f_0 r_p)` and
/// `(0, f_1 r_p)` moving `(f_0 v_p, 0)` and `(−f_1 v_p, 0)`; body 2 at `(d, 0)` receding radially with outer
/// `K/U = q_out`, `v_out = √(2 q_out (m_2 M_b / d) / μ_out)`.
pub fn binary_plus_escaper(a: f64, e: f64, d: f64, q_out: f64) -> State {
    let m = THIRDS;
    let mb = m[0] + m[1];
    let (f0, f1) = (m[1] / mb, m[0] / mb);
    let rp = a * (1.0 - e);
    let vp = (mb * (1.0 + e) / rp).sqrt();
    let mu_out = m[2] * mb / (m[2] + mb);
    let v_out = (2.0 * q_out * (m[2] * mb / d) / mu_out).sqrt();
    State::com(
        m,
        [[0.0, -f0 * rp], [0.0, f1 * rp], [d, 0.0]],
        [[f0 * vp, 0.0], [-f1 * vp, 0.0], [v_out, 0.0]],
    )
}

/// The two-stage break-up (§12.1): bodies 0 and 1 at `(0, ∓0.5)` moving `(0, ∓½ v_rel)` with pair `K/U = 2`; body 2 at
/// `(50, 0)` receding with outer `K/U = 2`.
pub fn two_stage() -> State {
    let m = THIRDS;
    let mu = m[0] * m[1] / (m[0] + m[1]);
    // K/U = 2 at d = 1: ½ μ v_rel² = 2 m_0 m_1.
    let v_rel = (4.0 * m[0] * m[1] / mu).sqrt();
    let mb = m[0] + m[1];
    let mu_out = m[2] * mb / (m[2] + mb);
    let v_out = (2.0 * 2.0 * (m[2] * mb / 50.0) / mu_out).sqrt();
    State::com(
        m,
        [[0.0, -0.5], [0.0, 0.5], [50.0, 0.0]],
        [[0.0, -0.5 * v_rel], [0.0, 0.5 * v_rel], [v_out, 0.0]],
    )
}

/// The equilateral directions scaled: positions `pos ×` and velocities `vel ×` them (§12.1's flying-apart, transient
/// and at-rest states).
pub fn equilateral_state(pos: f64, vel: f64) -> State {
    let u = equilateral();
    State::com(
        THIRDS,
        u.map(|q| [pos * q[0], pos * q[1]]),
        u.map(|q| [vel * q[0], vel * q[1]]),
    )
}

/// The Mardling–Aarseth hierarchy (§12.1): a circular inner pair, `a_in = 0.1`, as [`binary_plus_escaper`]'s pair at
/// `e = 0` (`v_in = √(M_b / a_in)`), and body 2 at `(10, 0)` moving `(v_r, √(1/10))`, prograde. §12.1's is `v_r = 0`;
/// §12.2 item 10's adds a radial velocity that keeps body 2 bound to the pair and Mardling–Aarseth true.
pub fn ma_hierarchy(v_r: f64) -> State {
    let m = THIRDS;
    let mb = m[0] + m[1];
    let (f0, f1) = (m[1] / mb, m[0] / mb);
    let a = 0.1;
    let vp = (mb / a).sqrt();
    State::com(
        m,
        [[0.0, -f0 * a], [0.0, f1 * a], [10.0, 0.0]],
        [[f0 * vp, 0.0], [-f1 * vp, 0.0], [v_r, (0.1f64).sqrt()]],
    )
}

/// §12.2 item 11, the review's homothetic counter-example to an unguarded rule: equal masses on the equilateral
/// directions at radius `scale` (initial radius 1, so `R_0 = 1`), each moving radially outward at the speed the
/// homothetic expansion with `E = −0.1 U_0` has there, `U_0` the initial potential's magnitude. Energy conservation
/// gives that speed: the kinetic energy is `½ v²` (three bodies of a third) and the potential `U_0 / scale`, so
/// `½ v² = E + U_0 / scale = U_0 (1/scale − 0.1)`; the expansion turns back at `scale = 10`.
pub fn homothetic(scale: f64) -> State {
    let u = equilateral();
    // At radius 1 every side is √3, and U_0 = 3 · (1/9) / √3.
    let u_0 = 3.0 * (1.0 / 9.0) / 3f64.sqrt();
    let v = (2.0 * u_0 * (1.0 / scale - 0.1)).sqrt();
    State::com(
        THIRDS,
        u.map(|q| [scale * q[0], scale * q[1]]),
        u.map(|q| [v * q[0], v * q[1]]),
    )
}

/// The initial potential's magnitude of [`homothetic`], `U_0`.
pub fn homothetic_u0() -> f64 {
    3.0 * (1.0 / 9.0) / 3f64.sqrt()
}

/// R-595's arm (written here): an `E < 0` state that passes the dispersal step's geometric tests (every pair farther
/// than `h_far · R_0 = 5`, every pair receding), with a bound pair inside the parabolic band and the hierarchy gate
/// passing. Bodies 0 and 1 are a weakly bound pair, `a = 100`, `e = 0.95`, at separation 6 on the outgoing leg, along
/// the x axis, so `K/U = 1 − d/(2a) = 0.97`: bound by the sign (`K < U`), parabolic by the band. Body 2 is at `(33, 0)`
/// from the pair's centre, `ρ = 5.5`, receding radially with outer `K/U = q_out`. At `q_out = 1.06` the outer band is
/// hyperbolic and `E ≈ −2·10⁻⁴`; the outer speed (`√(2 q_out / 33)` relative to the pair) is above the pair's members'
/// own radial speed, so every pair recedes.
pub fn r595(q_out: f64) -> State {
    let m = THIRDS;
    let mb = m[0] + m[1];
    let (f0, f1) = (m[1] / mb, m[0] / mb);
    let (a, e, d) = (100.0, 0.95, 6.0);
    let v2 = mb * (2.0 / d - 1.0 / a);
    let h = (mb * a * (1.0 - e * e)).sqrt();
    let v_t = h / d;
    let v_rad = (v2 - v_t * v_t).sqrt();
    let d_out = 33.0;
    let mu_out = m[2] * mb / (m[2] + mb);
    let v_out = (2.0 * q_out * (m[2] * mb / d_out) / mu_out).sqrt();
    State::com(
        m,
        [[-f0 * d, 0.0], [f1 * d, 0.0], [d_out, 0.0]],
        [
            [-f0 * v_rad, -f0 * v_t],
            [f1 * v_rad, f1 * v_t],
            [v_out, 0.0],
        ],
    )
}
