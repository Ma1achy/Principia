//! The compass's Jacobi and Shape views and the physical steer (render_gui_spec § "The compass: look and views (R-451,
//! R-452)"; deep_zoom §2, "The latent Jacobian"; R-452, R-550, R-633).
//!
//! `J` is the latent Jacobian: the eight-column, f64, central-difference `∂x/∂z` of the decoder's output in the view's
//! coordinates, read after canonicalisation, the scale gauge and `η_E`, each row divided by its fixed reference range,
//! at `J_D`'s step ([`latent_jacobian`]). It is never `J_D`, and the linear path never reads it. A view pushes the
//! plane's orthonormalised pair through it and draws the Plücker grid of the result over its eight coordinates
//! ([`Pushforward`]), with the area stretch on that pair, so the stretch does not move with the zoom or the tilts' shear.
//!
//! The steer to cell `(i, j)` is `q₁ ∝ J⁺ e_i`, `q₂ ∝ J⁺ e_j`, Gram–Schmidt orthonormalised, `J⁺` from `J`'s singular
//! value decomposition with every singular value below `c·σ₁` dropped ([`CUT`]), so `J⁺ = J⁻¹` where nothing is dropped.
//! It is refused where the smaller singular value of `[P e_i, P e_j]`, `P = J J⁺`, is below [`TOLERANCE`], the reason
//! named ([`Refusal`]).

use engine::contract::decoded::{self, Sphere, State};
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, Links, Plane, SimConfig, LATENT_DIM};

use super::views::{self, upper, Grid};
use crate::explore::manifold_view::{basis_edit, dot, norm};

/// `J⁺`'s cut `c` (REQ-GUI-410, an R-71 calibration confirmed at the M6 gate): inside the measured gap, at the order of
/// its geometric middle, about `2.9e-9`.
pub const CUT: f64 = 1e-9;
/// The refusal tolerance on `[P e_i, P e_j]`'s smaller singular value (REQ-GUI-346, an R-71 calibration confirmed at the
/// M6 gate; the measurement is render_gui_spec's).
pub const TOLERANCE: f64 = 0.1;
/// Where `sin φ` is below this, the Shape view's state is near the chart's pole, for naming a refusal's reason and for
/// the readout's line (REQ-GUI-411, an R-71 calibration confirmed at the M6 gate; the measurement is render_gui_spec's).
pub const POLE_SIN: f64 = 0.1;

/// The singular values `J` keeps on an `E*` chart with nothing else rank-deficient: the shell drops one (R-550).
const SHELL_KEPT: usize = LATENT_DIM - 1;

/// A matrix, rows first.
pub type Matrix = [[f64; LATENT_DIM]; LATENT_DIM];

/// A physical view's coordinate set (R-550).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set {
    /// `α, β, p_ρₓ, p_ρᵧ, p_λₓ, p_λᵧ, m₀/M, m₁/M`, each named as the decoder writes it.
    Jacobi,
    /// `θ, φ, θ̇, φ̇, L_z, Ṙ, m₀/M, m₁/M`, in R-14's convention, `φ` the polar angle from `+w`.
    Shape,
}

/// The Jacobi view's coordinate names, as drawn: no font of the theme's proportional family (Ubuntu Light, egui's emoji
/// fonts, Hack) has `ₓ`, `ᵧ`, `Ṙ` or a combining dot, so `p_ρₓ` reads `p_ρx` and a rate `θ̇` reads `dθ/dt` (a look
/// choice, R-390).
pub const JACOBI_NAMES: [&str; LATENT_DIM] =
    ["α", "β", "p_ρx", "p_ρy", "p_λx", "p_λy", "m₀/M", "m₁/M"];
/// The Shape view's coordinate names.
pub const SHAPE_NAMES: [&str; LATENT_DIM] =
    ["θ", "φ", "dθ/dt", "dφ/dt", "L_z", "dR/dt", "m₀/M", "m₁/M"];
/// The Shape set's row of `θ`, the only periodic coordinate: its difference is taken on the circle.
const THETA: usize = 0;

/// `√21`, the reference factor of `L_z` and `Ṙ` (REQ-GUI-347): `|ρ̃̇|² + |λ̃̇|²` at the mass simplex's centre, where
/// `μ_ρ = 1/6` and `μ_λ = 2/9`, with each momentum component at `q_max`, is `(12 + 9)·q_max²`.
fn rate_factor() -> f64 {
    21.0_f64.sqrt()
}

impl Set {
    /// Its coordinates' names, in its rows' order.
    pub fn names(self) -> [&'static str; LATENT_DIM] {
        match self {
            Set::Jacobi => JACOBI_NAMES,
            Set::Shape => SHAPE_NAMES,
        }
    }

    /// Each coordinate's fixed reference range at the momentum saturation bound `q_max` (RQ-346 (c), REQ-GUI-347):
    /// angles by the length of their domain, masses by 1, momenta by `q_max`; `L_z` and `Ṙ` by `√21·q_max`, `θ̇` and `φ̇`
    /// by `2√21·q_max`. Each is an expression in the decoder's bounds, never a spread over the image.
    pub fn ranges(self, q_max: f64) -> [f64; LATENT_DIM] {
        use std::f64::consts::{FRAC_PI_2, PI, TAU};
        match self {
            Set::Jacobi => [FRAC_PI_2, PI, q_max, q_max, q_max, q_max, 1.0, 1.0],
            Set::Shape => {
                let rate = rate_factor() * q_max;
                [TAU, PI, 2.0 * rate, 2.0 * rate, rate, rate, 1.0, 1.0]
            }
        }
    }

    /// The state's coordinates in this set, before any range.
    pub fn coordinates(self, s: &State) -> [f64; LATENT_DIM] {
        match self {
            Set::Jacobi => [
                s.alpha,
                s.beta,
                s.p_rho[0],
                s.p_rho[1],
                s.p_lambda[0],
                s.p_lambda[1],
                s.m[0],
                s.m[1],
            ],
            Set::Shape => {
                let sh = Sphere::of(s);
                [
                    sh.theta,
                    sh.phi,
                    sh.theta_dot,
                    sh.phi_dot,
                    s.l_z,
                    sh.r_dot,
                    s.m[0],
                    s.m[1],
                ]
            }
        }
    }
}

/// What a physical view differentiates (REQ-DEC-049): the decoder's CPU f64 path under the energy target `e_star`, or a
/// known linear map, the tests' fixture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decoder {
    /// The decoder, with `E*` (`None` is off, R-25). `SimConfig` holds no `E*` yet, so the running mock's is `None`.
    Real {
        /// The energy target.
        e_star: Option<f64>,
    },
    /// `x = A z` in the view's coordinates (a known `J`), `shell` marking it an `E*` chart.
    Linear {
        /// `A`.
        a: &'static Matrix,
        /// Whether the fixture stands for an `E*`-normalised chart, for a refusal's reason.
        shell: bool,
    },
}

impl Default for Decoder {
    fn default() -> Self {
        Decoder::Real { e_star: None }
    }
}

impl Decoder {
    /// The coordinates of `set` at `z`, before any range.
    pub fn coordinates(&self, set: Set, links: &Links, z: &Latent) -> [f64; LATENT_DIM] {
        match self {
            Decoder::Real { e_star } => set.coordinates(&decoded::state(z, links, *e_star)),
            Decoder::Linear { a, .. } => a.map(|row| dot(&row, z)),
        }
    }

    /// Whether this is an `E*`-normalised chart.
    pub fn on_shell(&self) -> bool {
        match self {
            Decoder::Real { e_star } => e_star.is_some(),
            Decoder::Linear { shell, .. } => *shell,
        }
    }
}

/// `b − a` for row `row` of `set`: on the circle for `θ`, so a difference across the seam is the short way round.
pub fn difference(set: Set, row: usize, a: f64, b: f64) -> f64 {
    use std::f64::consts::{PI, TAU};
    let d = b - a;
    if set == Set::Shape && row == THETA {
        d - TAU * ((d + PI) / TAU).floor()
    } else {
        d
    }
}

/// Column `k`'s central difference of `f` at `z` (deep_zoom §2, `J_D`'s step rule; R-633): `h = ε^{1/3}·max(1, |z_k|)`,
/// divided by the step actually taken, `(z + h) − (z − h)`, `d` the row's difference.
pub fn central_column(
    f: &impl Fn(&Latent) -> [f64; LATENT_DIM],
    d: &impl Fn(usize, f64, f64) -> f64,
    z: &Latent,
    k: usize,
) -> [f64; LATENT_DIM] {
    let h = f64::EPSILON.cbrt() * z[k].abs().max(1.0);
    let (mut up, mut down) = (*z, *z);
    up[k] = z[k] + h;
    down[k] = z[k] - h;
    let taken = up[k] - down[k];
    let (a, b) = (f(&down), f(&up));
    std::array::from_fn(|row| d(row, a[row], b[row]) / taken)
}

/// The latent Jacobian at `z` (REQ-DEC-049) in `set`'s coordinates, each row divided by its reference range at `links`'
/// `q_max`.
pub fn latent_jacobian(decoder: &Decoder, set: Set, links: &Links, z: &Latent) -> Matrix {
    let f = |at: &Latent| decoder.coordinates(set, links, at);
    let d = |row: usize, a: f64, b: f64| difference(set, row, a, b);
    let columns: [[f64; LATENT_DIM]; LATENT_DIM] =
        std::array::from_fn(|k| central_column(&f, &d, z, k));
    let ranges = set.ranges(links.link_params().q_max);
    std::array::from_fn(|row| std::array::from_fn(|k| columns[k][row] / ranges[row]))
}

/// `a x`, rows first.
pub fn apply(a: &Matrix, x: &Latent) -> Latent {
    a.map(|row| dot(&row, x))
}

/// `|u₁ ∧ u₂|`, the Gram factor `√(|u₁|²|u₂|² − (u₁·u₂)²)`, computed from the pair's Plücker coordinates so it keeps
/// its accuracy where the pair is nearly parallel.
pub fn gram(u1: &Latent, u2: &Latent) -> f64 {
    upper()
        .map(|(i, j)| {
            let p = u1[i] * u2[j] - u1[j] * u2[i];
            p * p
        })
        .sum::<f64>()
        .sqrt()
}

/// A singular value decomposition `J = U Σ Vᵀ`, the singular values in descending order: `u[k]` and `v[k]` are the left
/// and right vectors of `sigma[k]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Svd {
    /// The left singular vectors.
    pub u: [Latent; LATENT_DIM],
    /// The singular values, largest first.
    pub sigma: Latent,
    /// The right singular vectors.
    pub v: [Latent; LATENT_DIM],
}

/// The most sweeps [`svd`] takes; one-sided Jacobi on an 8 × 8 converges in well under this.
const SWEEPS: usize = 40;

/// `j`'s decomposition by one-sided Jacobi rotations, which keep a small singular value's relative accuracy: the columns
/// of `j` are rotated until orthogonal, a bounded number of sweeps.
pub fn svd(j: &Matrix) -> Svd {
    let mut c: [Latent; LATENT_DIM] = std::array::from_fn(|k| j.map(|row| row[k]));
    let mut v: [Latent; LATENT_DIM] =
        std::array::from_fn(|k| std::array::from_fn(|n| if n == k { 1.0 } else { 0.0 }));
    for _ in 0..SWEEPS {
        let mut rotated = false;
        for (p, q) in upper() {
            let (a, b, g) = (dot(&c[p], &c[p]), dot(&c[q], &c[q]), dot(&c[p], &c[q]));
            if g.abs() <= f64::EPSILON * (a * b).sqrt() {
                continue;
            }
            rotated = true;
            let zeta = (b - a) / (2.0 * g);
            let t = zeta.signum() / (zeta.abs() + zeta.hypot(1.0));
            let cs = 1.0 / (1.0 + t * t).sqrt();
            let sn = cs * t;
            for m in [&mut c, &mut v] {
                let (mut x, mut y) = (m[p], m[q]);
                for (a, b) in x.iter_mut().zip(&mut y) {
                    let (s, t) = (*a, *b);
                    *a = cs * s - sn * t;
                    *b = sn * s + cs * t;
                }
                m[p] = x;
                m[q] = y;
            }
        }
        if !rotated {
            break;
        }
    }
    let mut order: [usize; LATENT_DIM] = std::array::from_fn(|k| k);
    let lengths = c.map(|col| norm(&col));
    order.sort_by(|&a, &b| lengths[b].total_cmp(&lengths[a]));
    Svd {
        u: order.map(|k| {
            let s = lengths[k];
            c[k].map(|x| if s > 0.0 { x / s } else { 0.0 })
        }),
        sigma: order.map(|k| lengths[k]),
        v: order.map(|k| v[k]),
    }
}

/// The pseudo-inverse's action and the projector, `J⁺` with each singular value below `cut·σ₁` dropped (R-633).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inverse {
    /// `J`'s decomposition.
    pub svd: Svd,
    /// How many singular values are kept, the first of [`Svd::sigma`]'s.
    pub kept: usize,
}

impl Inverse {
    /// `J⁺` of `j` at the cut `cut`.
    pub fn of(j: &Matrix, cut: f64) -> Self {
        let svd = svd(j);
        let bound = cut * svd.sigma[0];
        let kept = svd
            .sigma
            .iter()
            .filter(|s| **s > 0.0 && **s >= bound)
            .count();
        Self { svd, kept }
    }

    /// `J⁺ e_i`.
    pub fn preimage(&self, i: usize) -> Latent {
        let mut out = [0.0; LATENT_DIM];
        for k in 0..self.kept {
            let weight = self.svd.u[k][i] / self.svd.sigma[k];
            for (o, v) in out.iter_mut().zip(&self.svd.v[k]) {
                *o += weight * v;
            }
        }
        out
    }

    /// `P e_i`, `P = J J⁺`, the projector onto `J`'s kept range.
    pub fn projected(&self, i: usize) -> Latent {
        let mut out = [0.0; LATENT_DIM];
        for k in 0..self.kept {
            let weight = self.svd.u[k][i];
            for (o, u) in out.iter_mut().zip(&self.svd.u[k]) {
                *o += weight * u;
            }
        }
        out
    }

    /// The smaller singular value of `[P e_i, P e_j]` (RQ-344's (a2) test): in `[0, 1]` and scale-free.
    pub fn smaller(&self, i: usize, j: usize) -> f64 {
        smaller_singular(&self.projected(i), &self.projected(j))
    }
}

/// The smaller singular value of the two-column matrix `[x, y]`, from its Gram matrix's eigenvalue.
pub fn smaller_singular(x: &Latent, y: &Latent) -> f64 {
    let (a, b, c) = (dot(x, x), dot(y, y), dot(x, y));
    let root = ((a - b) * (a - b) + 4.0 * c * c).sqrt();
    (((a + b - root) / 2.0).max(0.0)).sqrt()
}

/// Why a physical steer is refused (R-550 J6), with the reason the console logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The cell's plane needs the direction normal to the energy shell, on an `E*`-normalised chart.
    EnergyShell,
    /// The state is within the pole's radius of the Shape view's sphere, the chart's singularity.
    Pole,
    /// `J` cannot reach the cell's plane: a saturated link.
    Saturated,
}

impl Refusal {
    /// The reason, as logged.
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::EnergyShell => "normal to the energy shell",
            Refusal::Pole => "the chart's pole",
            Refusal::Saturated => "a saturated link",
        }
    }

    /// Why cell `(i, j)`, refused by `inverse` at `z`, is refused. On a shell chart the shell accounts for one dropped
    /// value, so seven kept leaves it the only cause, inside the pole's radius as well: the pole costs `J` nothing more
    /// there (R-550; REQ-GUI-411's record). Otherwise the pole comes first, a state with no chart there. Off a shell chart
    /// the other cause is `J`'s own rank. On one, fewer kept means something else is rank-deficient too, and the cell is
    /// told apart by whether it is still refused with `η_E` off, the shell gone; a fixture has no such chart, and names
    /// the other cause.
    fn at(
        decoder: &Decoder,
        set: Set,
        links: &Links,
        z: &Latent,
        inverse: &Inverse,
        (i, j): (usize, usize),
    ) -> Self {
        if decoder.on_shell() && inverse.kept >= SHELL_KEPT {
            return Refusal::EnergyShell;
        }
        if at_pole(decoder, set, links, z) {
            return Refusal::Pole;
        }
        if !decoder.on_shell() {
            return Refusal::Saturated;
        }
        match decoder {
            Decoder::Real { .. } => {
                let off = Decoder::Real { e_star: None };
                let off = Inverse::of(&latent_jacobian(&off, set, links, z), CUT);
                if refuses(off.smaller(i, j)) {
                    Refusal::Saturated
                } else {
                    Refusal::EnergyShell
                }
            }
            Decoder::Linear { .. } => Refusal::Saturated,
        }
    }
}

/// Whether `sin φ` is below the pole's radius, [`POLE_SIN`].
pub fn near_pole(sin_phi: f64) -> bool {
    sin_phi < POLE_SIN
}

/// Whether RQ-344's (a2) value refuses a cell: below [`TOLERANCE`], and not at it.
pub fn refuses(smaller: f64) -> bool {
    smaller < TOLERANCE
}

/// Whether `z` is within the pole's radius of the Shape view's sphere, on the decoder.
pub fn at_pole(decoder: &Decoder, set: Set, links: &Links, z: &Latent) -> bool {
    match (set, decoder) {
        (Set::Shape, Decoder::Real { e_star }) => {
            near_pole(Sphere::of(&decoded::state(z, links, *e_star)).sin_phi)
        }
        _ => false,
    }
}

/// The pair `(q₁, q₂)` Gram–Schmidt orthonormalised, each at the length `lengths` gives.
fn gram_schmidt(q1: &Latent, q2: &Latent, lengths: (f64, f64)) -> (Latent, Latent) {
    let n1 = norm(q1);
    let e1 = q1.map(|x| x / n1);
    let along = dot(q2, &e1);
    let w: Latent = std::array::from_fn(|k| q2[k] - along * e1[k]);
    let n2 = norm(&w);
    (e1.map(|x| x * lengths.0), w.map(|x| x * lengths.1 / n2))
}

/// The physical steer to cell `(i, j)` from the chart `sim` at `z₀` (REQ-GUI-200), or why it is refused: one `SetField`
/// on the basis, each axis at the length it had, as after the Chart builder's Apply (§G7). `None` for a cell on or below
/// the diagonal, which is empty.
pub fn steer(
    decoder: &Decoder,
    set: Set,
    sim: &SimConfig,
    i: usize,
    j: usize,
) -> Option<Result<SetField, Refusal>> {
    if i >= j || j >= LATENT_DIM {
        return None;
    }
    let plane = &sim.plane;
    let inverse = Inverse::of(&latent_jacobian(decoder, set, &sim.links, &plane.z0), CUT);
    if refuses(inverse.smaller(i, j)) {
        let why = Refusal::at(decoder, set, &sim.links, &plane.z0, &inverse, (i, j));
        return Some(Err(why));
    }
    let basis = gram_schmidt(
        &inverse.preimage(i),
        &inverse.preimage(j),
        (norm(&plane.q1), norm(&plane.q2)),
    );
    Some(Ok(basis_edit(basis)))
}

/// The console's line for a refused steer to cell `(i, j)` of `set`.
pub fn refusal_line(set: Set, i: usize, j: usize, refusal: Refusal) -> String {
    let names = set.names();
    format!(
        "steer to {} × {} refused: {}",
        names[i],
        names[j],
        refusal.reason()
    )
}

/// The plane pushed through `J` at a latent point: the grid over the view's coordinates and the area stretch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pushforward {
    /// The Plücker grid of `u₁ ∧ u₂`, normalised; all zero where `degenerate`.
    pub grid: Grid,
    /// The area stretch on the orthonormalised pair.
    pub stretch: f64,
    /// Whether `J q̂₁ ∧ J q̂₂` vanishes, at `J⁺`'s own cut relative to `σ₁`, or the plane has no pair.
    pub degenerate: bool,
}

/// `plane`'s pair pushed through `j` (R-550; REQ-GUI-199).
pub fn pushforward(j: &Matrix, plane: &Plane) -> Pushforward {
    let mut out = Pushforward {
        grid: [[0.0; LATENT_DIM]; LATENT_DIM],
        stretch: 0.0,
        degenerate: true,
    };
    let Some((e1, e2)) = views::orthonormal(plane) else {
        return out;
    };
    let (u1, u2) = (apply(j, &e1), apply(j, &e2));
    let stretch = gram(&u1, &u2);
    out.stretch = stretch;
    let sigma1 = svd(j).sigma[0];
    if stretch <= CUT * sigma1 * norm(&u1).max(norm(&u2)) {
        return out;
    }
    out.degenerate = false;
    for (a, b) in upper() {
        out.grid[a][b] = (u1[a] * u2[b] - u1[b] * u2[a]) / stretch;
    }
    out
}

/// The latent point `J` is taken at: the hovered pixel's on the affine chart while the pointer is over the figure,
/// otherwise `z₀` (R-452 C4; R-550 J2).
pub fn latent_point(plane: &Plane, hover: Option<[f64; 2]>) -> Latent {
    match hover {
        Some([s, t]) => crate::explore::manifold_view::point(plane, s, t),
        None => plane.z0,
    }
}

/// The physical views' help text: the metric named, and why they change as the slice moves (REQ-GUI-199).
pub const HELP: &str = "changes as the slice moves, J depending on z₀; Plücker does not. Each coordinate over its fixed \
reference range. J at the hovered pixel; nonlinear chart, z₀. Click to steer";

/// The readout's line within the pole's radius of the sphere (RQ-343 (b)): named as the chart's singularity. Near, not at:
/// the radius runs to about 5.7°, where the grid still draws and steers are accepted. Short enough for one line under
/// the grid.
pub const POLE_LINE: &str = "near the chart's pole: θ, φ singular";

/// The message where the pushforward vanishes (REQ-GUI-199): its values said, not drawn, under the empty cells it asks
/// to be clicked. `q₁`, `q₂` span the plane `q̂₁`, `q̂₂` do, and no font of the theme has the combining hat.
pub const DEGENERATE: &str = "J q₁ ∧ J q₂ = 0: this plane has no image here. Click a cell to steer";

/// The readout of `push` in `set`: the dominant pair and the area stretch.
pub fn readout(set: Set, push: &Pushforward) -> String {
    let names = set.names();
    let strongest = match views::dominant(&push.grid) {
        Some((i, j)) => format!(
            "{} × {} {}",
            names[i],
            names[j],
            views::value(push.grid[i][j])
        ),
        None => "—".to_owned(),
    };
    format!(
        "strongest: {strongest} · stretch {}",
        stretch_text(push.stretch)
    )
}

/// From here up the readout's stretch is in scientific form. Measured at the least panel and at 1024 by 768, with each
/// view's widest pairs and the focus on the compass element: the whole column shows in one stop with plain digits up to
/// `99999.999`, not at `999999.999`. The stretch grows past `1e18` toward the Shape sphere's pole, and from here up the
/// plain digits are no shorter than `1.699e18` (a look choice, R-390).
pub const STRETCH_SCIENTIFIC_FROM: f64 = 1e4;

/// The area stretch as the readout reads it: three decimals, or three in scientific form from
/// [`STRETCH_SCIENTIFIC_FROM`] up, whose widest value, `1.798e308`, is whole in the same measure.
fn stretch_text(stretch: f64) -> String {
    if stretch < STRETCH_SCIENTIFIC_FROM {
        format!("{stretch:.3}")
    } else {
        format!("{stretch:.3e}")
    }
}

/// The hover text of cell `(i, j)` of `grid` in `set`.
pub fn hover_text(set: Set, grid: &Grid, i: usize, j: usize) -> String {
    let names = set.names();
    format!("{} × {}  {}", names[i], names[j], views::value(grid[i][j]))
}
