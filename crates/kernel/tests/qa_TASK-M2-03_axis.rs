//! QA tests for TASK-M2-03's `rho_angle` on the axis and at the pole (REQ-PAY-088), written from dd_decoder §3.6's
//! case-by-case definition and R-417's `π̂`, not from the implementation:
//! - `π̂` is §3.2's image of π: π rounded toward zero at f32 (f32's nearest π lies above π, so it is that value's
//!   predecessor) and π's nearest value at f64 (R-417);
//! - **antiparallel** (`ρ × λ = 0`, `ρ · λ < 0`): exactly `π̂`, whatever the sign of the zero `ρ × λ`
//!   (`qa_m2_03_axis_antiparallel`);
//! - **parallel, or the pole** (`ρ × λ = 0`, `ρ · λ ≥ 0`): exactly 0, the poles where a −0 `ρ · λ` would make the CPU's
//!   `atan2` read π included (`qa_m2_03_axis_parallel_and_pole`);
//! - **elsewhere**: `atan2(ρ × λ, ρ · λ)` on `(−π̂, π̂]` at each precision, a value just above the axis taken down to
//!   `π̂` and one just below it, which stands for −π, read as `π̂`; every value of a sweep through the half turn lies
//!   in `(−π̂, π̂]` and within the round-off of the angle (`qa_m2_03_rho_angle_half_turn`).
//!
//! Each check runs at f64 and f32 (R-265). Each negative control (R-176) runs the same check on a descriptor whose
//! `rho_angle` is the bare `atan2(ρ × λ, ρ · λ)` at the precision, the value §3.6 says is not taken on the axis.

use kernel::decode::descriptor::describe;
use kernel::decode::Ic;
use kernel::generated::links::LinkReal;
use kernel::payload::{ICDescriptorOf, PayloadReal};
use validation::negative_control;

trait Fp: LinkReal + PayloadReal + std::fmt::Debug + Copy + PartialEq + 'static {
    const EPS: f64;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn bits(self) -> u64;
    /// `π̂` (R-417), written here from the rule, not read from the code.
    fn pi_hat() -> Self;
}

impl Fp for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn pi_hat() -> Self {
        // π's nearest f64.
        std::f64::consts::PI
    }
}

impl Fp for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn bits(self) -> u64 {
        self.to_bits() as u64
    }
    fn pi_hat() -> Self {
        // f32's nearest π lies above π (3.14159274… > 3.14159265…), so π rounded toward zero is its predecessor.
        let nearest = std::f32::consts::PI;
        assert!(nearest as f64 > std::f64::consts::PI);
        f32::from_bits(nearest.to_bits() - 1)
    }
}

type Describe<R> = fn(&Ic<R>) -> ICDescriptorOf<R>;

/// An IC at rest with masses `m` and positions `r`, each written exactly at `R` (the zeros keep their signs).
fn ic<R: Fp>(m: [f64; 3], r: [[f64; 2]; 3]) -> Ic<R> {
    Ic {
        m: m.map(R::of),
        r: r.map(|v| v.map(R::of)),
        p: [[R::of(0.0); 2]; 3],
    }
}

/// Equal inner masses with `r₀ = −r₁`: the inner pair's centre of mass is exactly the origin, so `λ = r₂` exactly.
const M: [f64; 3] = [0.25, 0.25, 0.5];

/// The descriptor's `rho_angle` on `(m, r)` is exactly `want`'s bits.
fn check_exact<R: Fp>(desc: Describe<R>, m: [f64; 3], r: [[f64; 2]; 3], want: R, case: &str) {
    let got = desc(&ic::<R>(m, r)).rho_angle;
    assert!(
        got.bits() == want.bits(),
        "rho_angle = {got:?}, not {want:?} ({case}) at m = {m:?}, r = {r:?}, {}",
        R::NAME
    );
}

// ── Antiparallel: π̂ whatever the sign of the zero cross product ─────────────────────────────────────────────────

/// Collinear ICs on the x axis, `λ` along `−ρ`; the `y` zeros written `+0` and `−0`, so `ρ × λ` is `+0` or `−0`.
fn antiparallel<R: Fp>(desc: Describe<R>) {
    let pi = R::pi_hat();
    for (y0, y1, y2) in [
        (0.0, 0.0, 0.0),
        (-0.0, -0.0, -0.0),
        (0.0, -0.0, 0.0),
        (-0.0, 0.0, -0.0),
        (0.0, 0.0, -0.0),
    ] {
        // ρ = (1, ±0), λ = (−1, ±0); and mirrored in x, ρ = (−1, ±0), λ = (1, ±0).
        check_exact(
            desc,
            M,
            [[-0.5, y0], [0.5, y1], [-1.0, y2]],
            pi,
            "antiparallel",
        );
        check_exact(
            desc,
            M,
            [[0.5, y0], [-0.5, y1], [1.0, y2]],
            pi,
            "antiparallel",
        );
    }
    // Unequal masses, the bodies anywhere on a line through the origin at y = 0: λ antiparallel to ρ.
    check_exact(
        desc,
        [0.25, 0.35, 0.4],
        [[1.0, 0.0], [-1.0, 0.0], [2.0, 0.0]],
        pi,
        "antiparallel",
    );
    // Along the y axis: ρ = (±0, 1), λ = (±0, −1).
    check_exact(
        desc,
        M,
        [[0.0, -0.5], [-0.0, 0.5], [0.0, -1.0]],
        pi,
        "antiparallel",
    );
    check_exact(
        desc,
        M,
        [[-0.0, -0.5], [0.0, 0.5], [-0.0, -1.0]],
        pi,
        "antiparallel",
    );
}

#[test]
fn qa_m2_03_axis_antiparallel() {
    antiparallel::<f64>(describe::<f64>);
    antiparallel::<f32>(describe::<f32>);
}

/// The descriptor with `rho_angle` the bare `atan2(ρ × λ, ρ · λ)`, its operands computed at `R` from §3.6's formulas.
#[cfg(feature = "controls")]
fn bare_atan2<R: Fp>(c: &Ic<R>) -> ICDescriptorOf<R> {
    let mut d = describe(c);
    let [m0, m1, _] = c.m;
    let m01 = m0 + m1;
    let rho = [c.r[1][0] - c.r[0][0], c.r[1][1] - c.r[0][1]];
    let lam = [
        c.r[2][0] - (m0 * c.r[0][0] + m1 * c.r[1][0]) / m01,
        c.r[2][1] - (m0 * c.r[0][1] + m1 * c.r[1][1]) / m01,
    ];
    let cross = rho[0] * lam[1] - rho[1] * lam[0];
    let dot = rho[0] * lam[0] + rho[1] * lam[1];
    // atan2 of the R values, correctly signed zeros kept, rounded once to R.
    d.rho_angle = R::of(cross.f().atan2(dot.f()));
    d
}

negative_control!(
    qa_m2_03_axis_antiparallel,
    "rho_angle the bare atan2, −π on a −0 cross product",
    expected = "(antiparallel) at m",
    antiparallel::<f64>(bare_atan2::<f64>)
);

// ── Parallel and the poles: exactly 0 ────────────────────────────────────────────────────────────────────────────

fn parallel_and_pole<R: Fp>(desc: Describe<R>) {
    let zero = R::of(0.0);
    // Parallel: ρ = (1, ±0), λ = (1, ±0), so ρ × λ = ±0 and ρ · λ > 0.
    for (y0, y1, y2) in [(0.0, 0.0, 0.0), (-0.0, -0.0, -0.0), (0.0, -0.0, -0.0)] {
        let r = [[-0.5, y0], [0.5, y1], [1.0, y2]];
        let got = desc(&ic::<R>(M, r)).rho_angle;
        assert!(
            got == zero,
            "rho_angle = {got:?}, not 0 (parallel) at r = {r:?}, {}",
            R::NAME
        );
    }
    // The pole ρ = 0 (r₀ = r₁), with λ in the third quadrant: ρ · λ = (+0)(−1) + (+0)(−1) = −0 and ρ × λ = +0, where
    // the CPU's atan2(+0, −0) is π.
    let poles = [
        [[0.3, 0.2], [0.3, 0.2], [-1.0, -1.0]],
        [[0.0, 0.0], [0.0, 0.0], [-1.0, -2.0]],
        [[0.0, 0.0], [0.0, 0.0], [-1.0, 0.5]],
        [[0.0, 0.0], [0.0, 0.0], [1.0, 1.0]],
        // The pole λ = 0 (r₂ at the inner pair's centre of mass, the origin, written −0): λ = (−0, −0), ρ = (1, 0),
        // so ρ · λ = −0.
        [[-0.5, 0.0], [0.5, 0.0], [-0.0, -0.0]],
        [[-0.5, 0.0], [0.5, 0.0], [0.0, 0.0]],
        // Both: every body at the origin.
        [[0.0, 0.0], [0.0, 0.0], [-0.0, -0.0]],
    ];
    for r in poles {
        let got = desc(&ic::<R>(M, r)).rho_angle;
        assert!(
            got == zero,
            "rho_angle = {got:?}, not 0 (the pole) at r = {r:?}, {}",
            R::NAME
        );
    }
}

#[test]
fn qa_m2_03_axis_parallel_and_pole() {
    parallel_and_pole::<f64>(describe::<f64>);
    parallel_and_pole::<f32>(describe::<f32>);
}

negative_control!(
    qa_m2_03_axis_parallel_and_pole,
    "rho_angle the bare atan2, π at the pole with a −0 dot product",
    expected = "not 0 (the pole)",
    parallel_and_pole::<f64>(bare_atan2::<f64>)
);

// ── Off the axis: (−π̂, π̂] at each precision ─────────────────────────────────────────────────────────────────────

fn half_turn<R: Fp>(desc: Describe<R>) {
    let pi = R::pi_hat();
    // Just off the antiparallel axis, on either side: the angle is within a hair of ±π, nearer π or −π than any other
    // value of R, so §3.6 gives π̂ on both sides (above: atan2 taken down to π̂; below: at or below −π̂, read as π̂).
    let hair = if R::NAME == "f32" { 1e-10 } else { 1e-20 };
    for y in [hair, -hair] {
        check_exact(
            desc,
            M,
            [[-0.5, 0.0], [0.5, 0.0], [-1.0, y]],
            pi,
            "next to the half turn",
        );
    }
    // A sweep through the circle, densest near ±π: λ = (cos θ, sin θ), ρ = (1, 0), so the angle is θ.
    let mut thetas = Vec::new();
    for k in 0..=200 {
        thetas.push(-std::f64::consts::PI + std::f64::consts::TAU * k as f64 / 200.0);
    }
    for k in 1..=60 {
        let d = 2f64.powi(-k);
        thetas.extend([std::f64::consts::PI - d, -std::f64::consts::PI + d]);
    }
    for th in thetas {
        let lam = [R::of(th.cos()), R::of(th.sin())];
        let r = [[-0.5, 0.0], [0.5, 0.0], [lam[0].f(), lam[1].f()]];
        let got = desc(&ic::<R>(M, r)).rho_angle;
        let (g, p) = (got.f(), pi.f());
        assert!(
            g > -p && g <= p,
            "rho_angle = {g} outside (−π̂, π̂] = (−{p}, {p}] at θ = {th}, {}",
            R::NAME
        );
        // The angle of the vector as written at R, in f64; compared on the circle.
        let want = lam[1].f().atan2(lam[0].f());
        let diff = (g - want).abs();
        let diff = diff.min(std::f64::consts::TAU - diff);
        assert!(
            diff <= 8.0 * R::EPS * std::f64::consts::PI,
            "rho_angle = {g}, not the angle {want} at θ = {th}, {}",
            R::NAME
        );
    }
}

#[test]
fn qa_m2_03_rho_angle_half_turn() {
    half_turn::<f64>(describe::<f64>);
    half_turn::<f32>(describe::<f32>);
}

negative_control!(
    qa_m2_03_rho_angle_half_turn,
    "rho_angle the bare atan2 at f32, whose nearest π lies above π̂",
    expected = "(next to the half turn) at m",
    half_turn::<f32>(bare_atan2::<f32>)
);
