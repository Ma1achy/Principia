//! QA tests for TASK-M3-68, the Aarseth–Zare occupant, written from the requirements and integrator contract Part 2b
//! § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)",
//! not from the implementation. Every expected value comes from a model written here from the contract's text:
//! - REQ-INT-044 (`qa_m368_lc_inverse_any_orientation`, `qa_m368_unstable_form_loses`): the conditioned inverse's
//!   round trip `R → u → L(u)u` within a few unit round-offs at every orientation, at f32 and f64, `u₀ ≥ 0`; and the
//!   unstable form at 179.9° within a decade of the reference's own table (`2.2e-2` at f32, `4.0e-12` at f64).
//! - Part 2b's reference-body rule (`qa_m368_reference_body`): the body not in the longest of `(0,1), (0,2), (1,2)`,
//!   the first maximum winning a tie, on integer triangles whose distances tie exactly (R-953).
//! - Part 2b's registration (`qa_m368_registration`): the pairs `(a,b)`, `(a,c)` with `b < c`, the mass factors, `E`
//!   the Cartesian energy, `Γ = A·B·(H − E) = 0` on the registered state, and the return to the centre-of-mass state.
//! - Part 2b's equations of motion (`qa_m368_equations_of_motion`): the rates against central differences of `Γ` as
//!   the contract writes it, and `dt/dτ = A·B`.
//! - REQ-INT-049 and Part 2b steps 2–4 (`qa_m368_step_control`): the entry size, the least usable constraint, the
//!   predictive limit on the relative **momenta** `P = K·V` (R-953), the entry-cap hit, the floor signal, and the
//!   landing clamp `dτ_eff = min(dτ, max(dt_left − t_s, 0)/(A·B))`.
//! - REQ-INT-105's finiteness arm (`qa_m368_doubly_degenerate`): an f32 state with both `A` and `B` below `TINY` at
//!   entry (subnormal separations) ends at the explicit finiteness test, its state finite; degenerate only
//!   mid-interval, its step is finite, a counted cap hit.
//! - REQ-INT-054 (`qa_m368_tau_schedule_bound`): over fuzzed ICs, the steps actually taken (counted by a stepper
//!   wrapper written here, not by the occupant's own counter) are at every boundary `t_k` at most
//!   `max(N_min, rate·t_k)`; a spent copy is never stepped again; the budget can fire.
//! - R-565 (`qa_m368_boundary_by_products`, `qa_m368_advance_lands_on_the_grid`): the boundary at or before `t` by the
//!   products `k·Δ_sync`; `ADVANCE` ends at exactly `k·Δ_sync`, and to a target between boundaries at the one before it.
//! - REQ-INT-024 (`qa_m368_slot_profile`): AZ + RK4's profile is Part 2a's row (order 4, 4 evaluations, not
//!   symplectic, not step- or run-reversible, owns its time mapping), and the slot accepts a stepper type that is not a
//!   Cartesian `Stepper` (the Mikkola–Tanikawa leapfrog's place), with its own row.
//!
//! Each check takes the code under test as a function, so its negative control (R-176) runs the same check on a faulty
//! one.

use std::cell::Cell;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
#[cfg(feature = "controls")]
use kernel::occupant::Stepper;
use kernel::occupant::{CapabilityProfile, Geom, State};
use kernel::regularisation::az::{
    boundary_at_or_before, entry_size, lc, lc_inverse, lc_inverse_unstable, march, reference_body,
    register, step_size, AarsethZare, AzAdvanced, AzConfig, AzCount, AzP, AzParams, AzQ, AzReal,
    AzStepper, AzSystem, End, ExplicitTest, StepSize, Watch,
};
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::Regularisation;
use proptest::prelude::*;
use validation::{negative_control, prop};

type V2 = [f64; 2];

/// A cadence that keeps nothing and never ends the march.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// Step-control values for these tests at f64 (test inputs, not the calibrations): `η`, `Δ_sync = 1/8`, the
/// reference's `TINY` (`1e-300`, R-953), R-160's `f = 0.02`, the `|R₃|` floor `1e-300`, a landing tolerance `1e-14`,
/// and a budget that does not bind.
fn params64(eta: f64) -> AzParams<f64> {
    AzParams {
        eta,
        delta_sync: 0.125,
        tiny: 1e-300,
        predictive_f: 0.02,
        r3_floor: 1e-300,
        land_eps_rel: 1e-14,
        step_budget_rate: 1e9,
        n_min: u32::MAX / 2,
        clamp: true,
        secant_tries: 4,
    }
}

fn state64(m: [f64; 3], r: [V2; 3], v: [V2; 3]) -> State<f64> {
    State {
        m,
        r,
        p: [0, 1, 2].map(|i| [m[i] * v[i][0], m[i] * v[i][1]]),
    }
}

/// Burrau-like masses 3, 4, 5 (unequal, so momenta and velocities differ), a scalene triangle, generic velocities with
/// a non-zero centre-of-mass motion.
fn generic() -> State<f64> {
    state64(
        [3.0, 4.0, 5.0],
        [[1.0, 3.0], [-2.0, -1.0], [1.3, -0.7]],
        [[0.3, -0.2], [-0.1, 0.45], [0.05, 0.12]],
    )
}

fn norm(a: V2) -> f64 {
    a[0].hypot(a[1])
}

fn sub(a: V2, b: V2) -> V2 {
    [a[0] - b[0], a[1] - b[1]]
}

/// `L(u)w`.
fn lmul(u: V2, w: V2) -> V2 {
    [u[0] * w[0] - u[1] * w[1], u[1] * w[0] + u[0] * w[1]]
}

/// The Cartesian energy `Σ|pᵢ|²/(2mᵢ) − Σ mᵢmⱼ/|rᵢ − rⱼ|`.
fn cartesian_energy(s: &State<f64>) -> f64 {
    let kin: f64 = (0..3)
        .map(|i| (s.p[i][0].powi(2) + s.p[i][1].powi(2)) / (2.0 * s.m[i]))
        .sum();
    let pot: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| s.m[i] * s.m[j] / norm(sub(s.r[i], s.r[j])))
        .sum();
    kin - pot
}

/// The other two bodies, `b < c`.
fn others(a: u32) -> (usize, usize) {
    match a {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// The relative momenta `P₁ = K₁₁V₁ + K₁₂V₂`, `P₂ = K₁₂V₁ + K₂₂V₂` from the Cartesian state, about body `a`
/// (Part 2b, "Coordinates and mass factors").
fn momenta(s: &State<f64>, a: u32) -> (V2, V2) {
    let (b, c) = others(a);
    let a = a as usize;
    let v = |i: usize| [s.p[i][0] / s.m[i], s.p[i][1] / s.m[i]];
    let (ma, mb, mc) = (s.m[a], s.m[b], s.m[c]);
    let total = ma + mb + mc;
    let (k11, k22, k12) = (
        mb * (ma + mc) / total,
        mc * (ma + mb) / total,
        -mb * mc / total,
    );
    let v1 = sub(v(b), v(a));
    let v2 = sub(v(c), v(a));
    (
        [k11 * v1[0] + k12 * v2[0], k11 * v1[1] + k12 * v2[1]],
        [k12 * v1[0] + k22 * v2[0], k12 * v1[1] + k22 * v2[1]],
    )
}

#[cfg(feature = "controls")]
/// The relative velocities `V₁ = v_b − v_a`, `V₂ = v_c − v_a`: what R-953 says the limit does **not** read.
fn velocities(s: &State<f64>, a: u32) -> (V2, V2) {
    let (b, c) = others(a);
    let a = a as usize;
    let v = |i: usize| [s.p[i][0] / s.m[i], s.p[i][1] / s.m[i]];
    (sub(v(b), v(a)), sub(v(c), v(a)))
}

// ── qa_m368_lc_inverse_any_orientation ───────────────────────────────────────────────────────────────────────────

/// The round trip's relative error `|L(u)u − R|/|R|` at `R`'s width, `R` the vector of length `scale` at `deg`
/// degrees rounded to the width, the error taken in f64; and `u₀`.
fn trip<R: AzReal + Into<f64>>(
    inverse: fn([R; 2], R) -> [R; 2],
    deg: f64,
    scale: f64,
    tiny: R,
) -> (f64, f64) {
    let th = deg.to_radians();
    let r: [R; 2] = [
        R::at_width((scale * th.cos()) as f32, scale * th.cos()),
        R::at_width((scale * th.sin()) as f32, scale * th.sin()),
    ];
    let u = inverse(r, tiny);
    let back = lc(u);
    let (x, y): (f64, f64) = (r[0].into(), r[1].into());
    let (bx, by): (f64, f64) = (back[0].into(), back[1].into());
    ((bx - x).hypot(by - y) / x.hypot(y), u[0].into())
}

/// REQ-INT-044: the conditioned inverse at every orientation (0° to 360° in 0.1° steps, and 179.9°, 179.99°, 180°)
/// and over scales `2^-30 .. 2^30`: relative round-trip error at most 4 unit round-offs everywhere, at most 2 at 179.9°
/// (the requirement's "≈ 5.96e-8 at f32, ≈ 1e-16 at f64"), and `u₀ ≥ 0` (Part 2b, the sign choice). The bounds were
/// fixed before the first run.
fn check_any_orientation(i32_: fn([f32; 2], f32) -> [f32; 2], i64_: fn([f64; 2], f64) -> [f64; 2]) {
    let u32_ = f64::from(f32::EPSILON) / 2.0;
    let u64_ = f64::EPSILON / 2.0;
    let mut angles: Vec<f64> = (0..3600).map(|i| f64::from(i) * 0.1).collect();
    angles.extend([179.9, 179.99, 180.0, -179.9, 359.9]);
    for &deg in &angles {
        for e in [-30, -7, 0, 5, 30] {
            let scale = 2f64.powi(e);
            let (e32, u0_32) = trip::<f32>(i32_, deg, scale, 1e-37);
            let (e64, u0_64) = trip::<f64>(i64_, deg, scale, 1e-300);
            assert!(
                e32 <= 4.0 * u32_ && e64 <= 4.0 * u64_,
                "orientation-dependent accuracy at {deg}° (scale 2^{e}): f32 {e32:.3e}, f64 {e64:.3e}"
            );
            assert!(u0_32 >= 0.0 && u0_64 >= 0.0, "u₀ < 0 at {deg}°");
        }
    }
    let (e32, _) = trip::<f32>(i32_, 179.9, 1.0, 1e-37);
    let (e64, _) = trip::<f64>(i64_, 179.9, 1.0, 1e-300);
    println!("conditioned at 179.9°: f32 {e32:.3e}, f64 {e64:.3e}");
    assert!(
        e32 <= 2.0 * u32_ && e64 <= 2.0 * u64_,
        "orientation-dependent accuracy at 179.9°: f32 {e32:.3e}, f64 {e64:.3e}"
    );
}

#[test]
fn qa_m368_lc_inverse_any_orientation() {
    check_any_orientation(lc_inverse::<f32>, lc_inverse::<f64>);
}

negative_control!(
    qa_m368_lc_inverse_any_orientation,
    "the unstable form's accuracy depends on orientation",
    expected = "orientation-dependent accuracy",
    check_any_orientation(lc_inverse_unstable::<f32>, lc_inverse_unstable::<f64>)
);

// ── qa_m368_unstable_form_loses ──────────────────────────────────────────────────────────────────────────────────

/// REQ-INT-044's verify: the unstable form at 179.9° loses as the reference's table has it, `2.2e-2` at f32 and
/// `4.0e-12` at f64 (R-927); read as "within a decade", fixed before the first run. The measurement can fire.
fn check_unstable_loses(i32_: fn([f32; 2], f32) -> [f32; 2], i64_: fn([f64; 2], f64) -> [f64; 2]) {
    let (e32, _) = trip::<f32>(i32_, 179.9, 1.0, 1e-37);
    let (e64, _) = trip::<f64>(i64_, 179.9, 1.0, 1e-300);
    println!("unstable at 179.9°: f32 {e32:.3e} (table 2.2e-2), f64 {e64:.3e} (table 4.0e-12)");
    let within = |e: f64, table: f64| e >= table / 10.0 && e <= table * 10.0;
    assert!(
        within(e32, 2.2e-2) && within(e64, 4.0e-12),
        "the unstable form does not lose as tabulated: f32 {e32:.3e}, f64 {e64:.3e}"
    );
}

#[test]
fn qa_m368_unstable_form_loses() {
    check_unstable_loses(lc_inverse_unstable::<f32>, lc_inverse_unstable::<f64>);
}

negative_control!(
    qa_m368_unstable_form_loses,
    "the conditioned form does not lose there, so the measurement can tell them apart",
    expected = "does not lose as tabulated",
    check_unstable_loses(lc_inverse::<f32>, lc_inverse::<f64>)
);

// ── qa_m368_reference_body ───────────────────────────────────────────────────────────────────────────────────────

/// Part 2b: `a = [2, 1, 0][argmax]` over `(0,1), (0,2), (1,2)`, the first maximum winning a tie. Integer triangles,
/// so the ties are exact: 3-4-5 and 2-4-√20 triangles.
fn check_reference(at64: fn(&[V2; 3]) -> u32, at32: fn(&[[f32; 2]; 3]) -> u32) {
    let cases: [([V2; 3], u32, &str); 6] = [
        ([[0.0, 0.0], [4.0, 0.0], [2.0, 1.0]], 2, "longest (0,1)"),
        ([[0.0, 0.0], [1.0, 1.0], [4.0, 0.0]], 1, "longest (0,2)"),
        ([[2.0, 1.0], [0.0, 0.0], [4.0, 0.0]], 0, "longest (1,2)"),
        (
            [[0.0, 0.0], [5.0, 0.0], [3.0, 4.0]],
            2,
            "tie (0,1) and (0,2)",
        ),
        (
            [[0.0, 0.0], [5.0, 0.0], [2.0, 4.0]],
            2,
            "tie (0,1) and (1,2)",
        ),
        (
            [[5.0, 0.0], [3.0, 4.0], [0.0, 0.0]],
            1,
            "tie (0,2) and (1,2)",
        ),
    ];
    for (r, want, what) in cases {
        for shift in [[0.0, 0.0], [0.5, -0.25], [-64.0, 32.0]] {
            let r = r.map(|p| [p[0] + shift[0], p[1] + shift[1]]);
            let r32 = r.map(|p| p.map(|c| c as f32));
            assert_eq!(at64(&r), want, "wrong reference body, f64, {what}");
            assert_eq!(at32(&r32), want, "wrong reference body, f32, {what}");
        }
    }
}

#[cfg(feature = "controls")]
/// The last maximum: the tie-break the contract rules out.
fn last_maximum<R: AzReal>(r: &[[R; 2]; 3]) -> u32 {
    let d2 = |i: usize, j: usize| {
        let dx = r[i][0] - r[j][0];
        let dy = r[i][1] - r[j][1];
        dx * dx + dy * dy
    };
    let d = [d2(0, 1), d2(0, 2), d2(1, 2)];
    let mut k = 0;
    for i in 1..3 {
        if d[i] >= d[k] {
            k = i;
        }
    }
    [2, 1, 0][k]
}

#[test]
fn qa_m368_reference_body() {
    check_reference(reference_body::<f64>, reference_body::<f32>);
}

negative_control!(
    qa_m368_reference_body,
    "the last maximum wins a tie",
    expected = "wrong reference body",
    check_reference(last_maximum::<f64>, last_maximum::<f32>)
);

// ── qa_m368_registration ─────────────────────────────────────────────────────────────────────────────────────────

/// `Γ` as Part 2b writes it, at the system's masses (its reference body `a`) and energy `e`.
fn gamma(m: [f64; 3], a: u32, e: f64, u1: V2, p1: V2, u2: V2, p2: V2) -> f64 {
    let (b, c) = others(a);
    let (ma, mb, mc) = (m[a as usize], m[b], m[c]);
    let mu1 = ma * mb / (ma + mb);
    let mu2 = ma * mc / (ma + mc);
    let aa = u1[0] * u1[0] + u1[1] * u1[1];
    let bb = u2[0] * u2[0] + u2[1] * u2[1];
    let l1p1 = lmul(u1, p1);
    let l2p2 = lmul(u2, p2);
    let r3 = sub(lmul(u2, u2), lmul(u1, u1));
    bb * (p1[0] * p1[0] + p1[1] * p1[1]) / (8.0 * mu1)
        + aa * (p2[0] * p2[0] + p2[1] * p2[1]) / (8.0 * mu2)
        + (l1p1[0] * l2p2[0] + l1p1[1] * l2p2[1]) / (4.0 * ma)
        - ma * mb * bb
        - ma * mc * aa
        - aa * bb * mb * mc / norm(r3)
        - e * aa * bb
}

/// The state shifted to its centre of mass, positions and velocities.
fn centred(s: &State<f64>) -> State<f64> {
    let total: f64 = s.m.iter().sum();
    let rc = [0, 1].map(|k| (0..3).map(|i| s.m[i] * s.r[i][k]).sum::<f64>() / total);
    let vc = [0, 1].map(|k| (0..3).map(|i| s.p[i][k]).sum::<f64>() / total);
    State {
        m: s.m,
        r: s.r.map(|p| sub(p, rc)),
        p: [0, 1, 2].map(|i| {
            [
                s.m[i] * (s.p[i][0] / s.m[i] - vc[0]),
                s.m[i] * (s.p[i][1] / s.m[i] - vc[1]),
            ]
        }),
    }
}

/// A map from the registered `(q, p)` back to Cartesian.
type Back = fn(&AzSystem<f64>, &AzQ<f64>, &AzP<f64>) -> State<f64>;

/// Part 2b, "Registration and return": the pairs, the mass factors, `E`, `Γ = 0` on the registered state, and the
/// return to the centre-of-mass state. `back` maps the registered `(q, p)` to Cartesian.
fn check_registration(back: Back) {
    for s in [
        generic(),
        state64(
            [1.0, 1.0, 1.0],
            [[-0.97, 0.24], [0.97, -0.24], [0.0, 0.0]],
            [[0.466, 0.432], [0.466, 0.432], [-0.932, -0.865]],
        ),
        state64(
            [0.2, 0.7, 0.1],
            [[0.0, 0.0], [-3.0, 0.5], [0.05, 0.02]],
            [[0.0, 0.1], [0.02, -0.03], [1.5, -2.0]],
        ),
    ] {
        let (sys, q, p) = register(&s, 1e-300, 1e-300);
        let a = sys.a;
        assert_eq!(a, reference_body(&s.r), "registered about another body");
        let (b, c) = others(a);
        let ai = a as usize;
        let close = |x: V2, y: V2, what: &str| {
            assert!(
                norm(sub(x, y)) <= 1e-13 * (1.0 + norm(y)),
                "registration: {what}: {x:?} against {y:?}"
            );
        };
        close(lc(q.u1), sub(s.r[b], s.r[ai]), "R₁ = r_b − r_a");
        close(lc(q.u2), sub(s.r[c], s.r[ai]), "R₂ = r_c − r_a");
        assert_eq!(q.t, 0.0, "the interval clock starts at 0");
        // E is the energy of the relative motion: the Cartesian energy in the centre-of-mass frame.
        let h = cartesian_energy(&centred(&s));
        assert!(
            (sys.e - h).abs() <= 1e-13 * h.abs(),
            "registration: E {} against the Cartesian energy {h}",
            sys.e
        );
        let g = gamma(s.m, a, h, q.u1, p.p1, q.u2, p.p2);
        let ab = (q.u1[0].powi(2) + q.u1[1].powi(2)) * (q.u2[0].powi(2) + q.u2[1].powi(2));
        assert!(
            g.abs() <= 1e-12 * (1.0 + ab * h.abs()),
            "registration: Γ = A·B·(H − E) is {g:.3e}, not 0"
        );
        // P₁ = L₁p₁/(2A), P₂ = L₂p₂/(2B) are the momenta K·V.
        let (pp1, pp2) = momenta(&s, a);
        let [_, _, got1, got2] = sys.phys(&q, &p);
        close(got1, pp1, "P₁ = K·V");
        close(got2, pp2, "P₂ = K·V");
        let want = centred(&s);
        let got = back(&sys, &q, &p);
        for i in 0..3 {
            close(got.r[i], want.r[i], "the return's positions");
            close(got.p[i], want.p[i], "the return's momenta");
        }
    }
}

#[cfg(feature = "controls")]
fn swapped_back(sys: &AzSystem<f64>, q: &AzQ<f64>, p: &AzP<f64>) -> State<f64> {
    let q = AzQ {
        u1: q.u2,
        u2: q.u1,
        t: q.t,
    };
    let p = AzP { p1: p.p2, p2: p.p1 };
    sys.image(&q, &p)
}

#[test]
fn qa_m368_registration() {
    check_registration(|sys, q, p| sys.image(q, p));
}

negative_control!(
    qa_m368_registration,
    "the two pairs exchanged on the way back",
    expected = "the return's",
    check_registration(swapped_back)
);

// ── qa_m368_equations_of_motion ──────────────────────────────────────────────────────────────────────────────────

/// Part 2b's equations of motion: `(du₁, du₂)/dτ = ∂Γ/∂(p₁, p₂)`, `(dp₁, dp₂)/dτ = −∂Γ/∂(u₁, u₂)`, `dt/dτ = A·B`,
/// against central differences of `Γ` (step `1e-6`: truncation about `1e-12`, rounding about `1e-10`; a wrong term is
/// of order one, so `1e-6` relative separates them). `g` is `Γ` with the system's masses, body and energy.
fn check_rates(g: fn(&AzSystem<f64>, V2, V2, V2, V2) -> f64) {
    let base = generic();
    let (sys, q0, p0) = register(&base, 1e-300, 1e-300);
    // Away from registration too: perturbed `(u, p)` with the same system.
    for (du, dp) in [(0.0, 0.0), (0.03, -0.02), (-0.05, 0.07)] {
        let q = AzQ {
            u1: [q0.u1[0] + du, q0.u1[1] - du],
            u2: [q0.u2[0] - 0.5 * du, q0.u2[1] + du],
            t: 0.0,
        };
        let p = AzP {
            p1: [p0.p1[0] + dp, p0.p1[1]],
            p2: [p0.p2[0], p0.p2[1] - dp],
        };
        let d = 1e-6;
        let at = |k: usize, sign: f64| {
            let mut x = [q.u1, p.p1, q.u2, p.p2];
            x[k / 2][k % 2] += sign * d;
            g(&sys, x[0], x[1], x[2], x[3])
        };
        let grad: Vec<f64> = (0..8)
            .map(|k| (at(k, 1.0) - at(k, -1.0)) / (2.0 * d))
            .collect();
        let dq = sys.q_rate(&q, &p);
        let dpr = sys.p_rate(&q, &p);
        // Order of `grad`: u1x u1y p1x p1y u2x u2y p2x p2y.
        let pairs = [
            (dq.u1[0], grad[2], "du₁/dτ = ∂Γ/∂p₁"),
            (dq.u1[1], grad[3], "du₁/dτ = ∂Γ/∂p₁"),
            (dq.u2[0], grad[6], "du₂/dτ = ∂Γ/∂p₂"),
            (dq.u2[1], grad[7], "du₂/dτ = ∂Γ/∂p₂"),
            (dpr.p1[0], -grad[0], "dp₁/dτ = −∂Γ/∂u₁"),
            (dpr.p1[1], -grad[1], "dp₁/dτ = −∂Γ/∂u₁"),
            (dpr.p2[0], -grad[4], "dp₂/dτ = −∂Γ/∂u₂"),
            (dpr.p2[1], -grad[5], "dp₂/dτ = −∂Γ/∂u₂"),
        ];
        for (got, want, what) in pairs {
            assert!(
                (got - want).abs() <= 1e-6 * (1.0 + want.abs()),
                "equations of motion: {what}: {got} against {want}"
            );
        }
        let (aa, bb) = AzSystem::ab(&q);
        assert_eq!(dq.t, aa * bb, "dt/dτ is not A·B");
    }
}

fn gamma_of(sys: &AzSystem<f64>, u1: V2, p1: V2, u2: V2, p2: V2) -> f64 {
    gamma(sys.m, sys.a, sys.e, u1, p1, u2, p2)
}

#[cfg(feature = "controls")]
/// `Γ` with the cross term's sign flipped: not the contract's `Γ`.
fn gamma_flipped(sys: &AzSystem<f64>, u1: V2, p1: V2, u2: V2, p2: V2) -> f64 {
    let ma = sys.m[sys.a as usize];
    let l1p1 = lmul(u1, p1);
    let l2p2 = lmul(u2, p2);
    gamma_of(sys, u1, p1, u2, p2) - 2.0 * (l1p1[0] * l2p2[0] + l1p1[1] * l2p2[1]) / (4.0 * ma)
}

#[test]
fn qa_m368_equations_of_motion() {
    check_rates(gamma_of);
}

negative_control!(
    qa_m368_equations_of_motion,
    "a Γ whose cross term has the wrong sign",
    expected = "equations of motion",
    check_rates(gamma_flipped)
);

// ── qa_m368_step_control ─────────────────────────────────────────────────────────────────────────────────────────

/// The relative vectors' limit Part 2b step 4 reads, from the Cartesian state: `(P₁, P₂)` (R-953), or, for the
/// control, the velocities.
type Limit = fn(&State<f64>, u32) -> (V2, V2);

/// REQ-INT-049 and Part 2b steps 2–4, from the formulas, on a registered state at unequal masses.
fn check_step_control(limit: Limit) {
    let s = generic();
    let (sys, q, p) = register(&s, 1e-300, 1e-300);
    let (aa, bb) = AzSystem::ab(&q);
    let ab = aa * bb;
    let dt = 0.125;
    let rel = |x: f64, y: f64| (x - y).abs() <= 1e-12 * y.abs();

    // Step 2: the entry size.
    let pr = params64(0.01);
    let entry = entry_size(&q, &pr);
    assert!(
        rel(entry, 0.01 * dt / ab),
        "entry size {entry} is not η·dt_left/(A·B)"
    );

    // Step 4: the predictive limit binds where η is large: c₂ = f·d_min/(P_max·A·B).
    let (v1, v2) = limit(&s, sys.a);
    let (r1, r2) = (lc(q.u1), lc(q.u2));
    let d_min = norm(r1).min(norm(r2)).min(norm(sub(r2, r1)));
    let p_max = norm(v1).max(norm(v2)).max(norm(sub(v2, v1)));
    let c2 = 0.02 * d_min / (p_max * ab);
    let big = params64(1e6);
    let e_big = entry_size(&q, &big);
    let sz: StepSize<f64> = step_size(&sys, &q, &p, e_big, &big);
    let (m1, m2) = momenta(&s, sys.a);
    let binding = 0.02 * d_min / (norm(m1).max(norm(m2)).max(norm(sub(m2, m1))) * ab);
    assert!(
        binding < dt / ab,
        "fixture: the limit does not bind below the clamp"
    );
    assert!(
        sz.by_limit && rel(sz.h, c2),
        "predictive limit: step {} (by the limit: {}) against f·d_min/(P_max·A·B) = {c2}",
        sz.h,
        sz.by_limit
    );

    // Step 3: the least usable constraint, with the limit off (f = 0): c₀ = c₁ at entry.
    let mut off = params64(0.01);
    off.predictive_f = 0.0;
    let sz = step_size(&sys, &q, &p, entry, &off);
    assert!(rel(sz.h, 0.01 * dt / ab) && !sz.by_limit && !sz.cap_hit && !sz.floored);
    // The entry-cap hit: c₀ > c₁ takes c₁, counted.
    let sz = step_size(&sys, &q, &p, entry / 2.0, &off);
    assert!(
        sz.cap_hit && sz.h == entry / 2.0,
        "the entry cap does not bind"
    );
    // The floor signal and the floored product: TINY above A.
    let mut fl = off;
    fl.tiny = 2.0 * aa.max(bb);
    let sz = step_size(&sys, &q, &p, entry, &fl);
    assert!(sz.floored, "A, B < TINY not signalled");
    assert!(
        rel(sz.h, (0.01 * dt / (fl.tiny * fl.tiny)).min(entry)),
        "the floored product is not max(A, TINY)·max(B, TINY)"
    );

    // REQ-INT-049: dτ_eff = min(dτ, max(dt_left − t_s, 0)/(A·B)).
    for t_s in [dt - 1e-6, dt - 1e-9, 0.5 * dt] {
        let qt = AzQ { t: t_s, ..q };
        let sz = step_size(&sys, &qt, &p, entry, &off);
        let want = (0.01 * dt / ab).min((dt - t_s) / ab);
        assert!(
            rel(sz.h, want),
            "landing clamp at t_s = {t_s}: {} against {want}",
            sz.h
        );
    }
    for t_s in [dt, dt + 1e-3] {
        let qt = AzQ { t: t_s, ..q };
        assert_eq!(
            step_size(&sys, &qt, &p, entry, &off).h,
            0.0,
            "the clamp past the boundary is not 0"
        );
    }
    let mut free = off;
    free.clamp = false;
    let qt = AzQ { t: dt - 1e-6, ..q };
    assert!(
        rel(step_size(&sys, &qt, &p, entry, &free).h, 0.01 * dt / ab),
        "the clamp acts while off"
    );
}

#[test]
fn qa_m368_step_control() {
    check_step_control(momenta);
}

negative_control!(
    qa_m368_step_control,
    "a predictive limit on the velocities, not the momenta (R-953)",
    expected = "predictive limit",
    check_step_control(velocities)
);

// ── qa_m368_doubly_degenerate ────────────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "controls")]
/// A watch that never finds a state non-finite: the march without the explicit test.
struct Blind;

#[cfg(feature = "controls")]
impl Watch<f32> for Blind {
    fn finite(&mut self, _q: &AzQ<f32>, _p: &AzP<f32>) -> bool {
        true
    }
}

fn params32() -> AzParams<f32> {
    AzParams {
        eta: 0.01,
        delta_sync: 0.125,
        tiny: 1e-37,
        predictive_f: 0.02,
        r3_floor: 1e-37,
        land_eps_rel: 1e-5,
        step_budget_rate: 1e6,
        n_min: 10_000,
        clamp: true,
        secant_tries: 4,
    }
}

/// Three bodies within `1e-40` (subnormal at f32) of one another: `A` and `B` both below `TINY = 1e-37`, `TINY·TINY`
/// underflows to `0`, the entry size is infinite.
fn degenerate32() -> State<f32> {
    State {
        m: [1.0, 1.0, 1.0],
        r: [[0.0, 0.0], [1e-40, 0.0], [-1e-40, 0.0]],
        p: [[0.0, 0.0], [0.5, 0.25], [-0.5, -0.25]],
    }
}

/// REQ-INT-105's finiteness arm: the march ends at the explicit test, no non-finite state carried past it.
fn check_degenerate<X: Watch<f32>>(mut watch: X) {
    let s = degenerate32();
    let (_, q, _) = register(&s, 1e-37, 1e-37);
    let (aa, bb) = AzSystem::ab(&q);
    assert!(
        aa < 1e-37 && bb < 1e-37,
        "fixture: A = {aa:e}, B = {bb:e} not both below TINY"
    );
    assert!(
        !entry_size(&q, &params32()).is_finite(),
        "fixture: the entry size is finite"
    );
    let out: AzAdvanced<f32> = march::<Rk4, f32, Quiet, X>(
        &s,
        0,
        4,
        AzCount::new(),
        &params32(),
        &mut Quiet,
        &mut watch,
    );
    let all = out
        .state
        .r
        .iter()
        .chain(out.state.p.iter())
        .flatten()
        .all(|x| x.is_finite());
    assert!(
        all,
        "a non-finite state carried past the test: {:?}",
        out.state
    );
    assert_eq!(
        out.count.end,
        End::NonFinite,
        "the march did not end at the finiteness test"
    );
    assert_eq!(out.boundary, 0, "a degenerate copy reached a boundary");
    assert!(
        out.count.non_finite_entries >= 1,
        "the non-finite entry is not counted"
    );
}

#[test]
fn qa_m368_doubly_degenerate() {
    check_degenerate(ExplicitTest);
    // Degenerate only mid-interval: a finite entry size; the per-step rule skips the non-finite c₀ and the step is
    // finite, a counted cap hit (Part 2b, step 3; driver.rs:640).
    let s = degenerate32();
    let (sys, q, p) = register(&s, 1e-37, 1e-37);
    let entry = 1e-3_f32;
    let sz = step_size(&sys, &q, &p, entry, &params32());
    assert!(
        sz.h.is_finite() && sz.h > 0.0,
        "mid-interval step {} not finite",
        sz.h
    );
    assert_eq!(sz.h, entry, "mid-interval step is not the entry size");
    assert!(
        sz.cap_hit && sz.floored,
        "mid-interval: cap hit {} floored {}",
        sz.cap_hit,
        sz.floored
    );
}

negative_control!(
    qa_m368_doubly_degenerate,
    "the march without the explicit finiteness test",
    expected = "non-finite state carried past",
    check_degenerate(Blind)
);

// ── qa_m368_tau_schedule_bound ───────────────────────────────────────────────────────────────────────────────────

thread_local! {
    static TAKEN: Cell<u64> = const { Cell::new(0) };
}

/// RK4 in the slot, each step tallied here, independently of the occupant's counter.
struct Tally;

impl AzStepper for Tally {
    fn pair_profile() -> CapabilityProfile {
        <Rk4 as AzStepper>::pair_profile()
    }
    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        TAKEN.with(|c| c.set(c.get() + 1));
        <Rk4 as AzStepper>::regularised_step(system, q, p, h)
    }
}

/// One boundary's march from `(s, k, count)`.
type Interval = fn(&State<f64>, u32, AzCount<f64>, &AzParams<f64>) -> AzAdvanced<f64>;

fn carried(s: &State<f64>, k: u32, c: AzCount<f64>, pr: &AzParams<f64>) -> AzAdvanced<f64> {
    march::<Tally, f64, Quiet, ExplicitTest>(s, k, k + 1, c, pr, &mut Quiet, &mut ExplicitTest)
}

#[cfg(feature = "controls")]
/// The control: the running total reset at each boundary (a cap per interval read as the budget), the rest of the
/// copy's record carried.
fn reset(s: &State<f64>, k: u32, c: AzCount<f64>, pr: &AzParams<f64>) -> AzAdvanced<f64> {
    march::<Tally, f64, Quiet, ExplicitTest>(
        s,
        k,
        k + 1,
        AzCount { steps: 0, ..c },
        pr,
        &mut Quiet,
        &mut ExplicitTest,
    )
}

/// The budget: `N_min = 150` and 600 steps per unit time, against about 100 steps per interval of `1/8` at `η = 0.01`
/// (about 800 per unit time): one or two boundaries are reached and the bound read there, and then the cumulative
/// total outruns `rate·t` and the budget fires, while no single interval's count reaches `N_min`.
fn budget() -> AzParams<f64> {
    let mut p = params64(0.01);
    p.n_min = 150;
    p.step_budget_rate = 600.0;
    p
}

/// REQ-INT-054 on one IC: returns whether the budget fired, and the most steps taken in one interval.
fn schedule(s: &State<f64>, step: Interval) -> Result<(bool, u64), TestCaseError> {
    let pr = budget();
    TAKEN.with(|c| c.set(0));
    let mut s = *s;
    let mut count = AzCount::new();
    let mut k = 0;
    let mut largest = 0;
    let mut fired = false;
    for _ in 0..16 {
        let before = TAKEN.with(Cell::get);
        let out = step(&s, k, count, &pr);
        let taken = TAKEN.with(Cell::get);
        largest = largest.max(taken - before);
        if out.count.end == End::Marching {
            prop_assert_eq!(out.boundary, k + 1);
            let t_k = f64::from(out.boundary) * pr.delta_sync;
            let bound = f64::from(pr.n_min).max(pr.step_budget_rate * t_k);
            prop_assert!(
                taken as f64 <= bound,
                "the running total {} at boundary t_k = {} exceeds max(N_min, rate·t_k) = {}",
                taken,
                t_k,
                bound
            );
        } else if out.count.end == End::Exhausted {
            fired = true;
            // Never run on: a later call takes no step and leaves the state.
            let again = step(&out.state, out.boundary, out.count, &pr);
            prop_assert_eq!(
                TAKEN.with(Cell::get),
                taken,
                "a spent copy was stepped again"
            );
            prop_assert_eq!(again.state.r, out.state.r);
            prop_assert_eq!(again.count.end, End::Exhausted);
            break;
        } else {
            break;
        }
        s = out.state;
        count = out.count;
        k = out.boundary;
    }
    Ok((fired, largest))
}

fn ic() -> impl Strategy<Value = State<f64>> {
    let m = 0.3f64..2.0;
    let x = -1.0f64..1.0;
    let v = -0.5f64..0.5;
    (
        [m.clone(), m.clone(), m],
        [x.clone(), x.clone(), x.clone(), x.clone()],
        [v.clone(), v.clone(), v.clone(), v.clone(), v.clone(), v],
    )
        .prop_filter("separated", |(_, x, _)| {
            let r = [[0.0, 0.0], [x[0], x[1]], [x[2], x[3]]];
            [(0, 1), (0, 2), (1, 2)]
                .iter()
                .all(|&(i, j)| norm(sub(r[i], r[j])) > 0.3)
        })
        .prop_map(|(m, x, v)| {
            state64(
                m,
                [[0.0, 0.0], [x[0], x[1]], [x[2], x[3]]],
                [[v[0], v[1]], [v[2], v[3]], [v[4], v[5]]],
            )
        })
}

fn check_schedule(step: Interval) {
    // The budget can fire: a fixed IC that spends it.
    let (fired, largest) = schedule(&generic(), step).expect("the fixed IC");
    println!(
        "fixed IC: budget fired {fired}; largest count in one interval {largest} (for the record)"
    );
    assert!(
        fired,
        "the running total exceeded the boundary bound without the budget firing"
    );
    prop::run(&ic(), |s| {
        let (_, largest) = schedule(&s, step)?;
        println!("largest count in one interval: {largest} (for the record)");
        Ok(())
    });
}

#[test]
fn qa_m368_tau_schedule_bound() {
    check_schedule(carried);
}

negative_control!(
    qa_m368_tau_schedule_bound,
    "a counter reset at each boundary misses the cumulative total",
    expected = "the running total",
    check_schedule(reset)
);

// ── qa_m368_boundary_by_products ─────────────────────────────────────────────────────────────────────────────────

/// R-565: the latest boundary `k` at or before `t`, by the products the grid's times are: `k·Δ ≤ t < (k + 1)·Δ`, at
/// `t` on, just below and just above a boundary's product, at f64 and f32 (`k < 2²⁰`).
fn check_products(b64: fn(f64, f64) -> u32, b32: fn(f32, f32) -> u32) {
    let s = (1u32..1 << 20, 1e-3f64..2.0, 0u8..3);
    prop::run(&s, |(k, delta, nudge)| {
        let on = f64::from(k) * delta;
        let t = match nudge {
            0 => on,
            1 => f64::from_bits(on.to_bits() - 1),
            _ => f64::from_bits(on.to_bits() + 1),
        };
        let got = b64(t, delta);
        prop_assert!(
            f64::from(got) * delta <= t && f64::from(got + 1) * delta > t,
            "boundary {} is not the one at or before {} (Δ = {})",
            got,
            t,
            delta
        );
        let d32 = delta as f32;
        let on32 = (k as f32) * d32;
        let t32 = match nudge {
            0 => on32,
            1 => f32::from_bits(on32.to_bits() - 1),
            _ => f32::from_bits(on32.to_bits() + 1),
        };
        let got = b32(t32, d32);
        prop_assert!(
            (got as f32) * d32 <= t32 && ((got + 1) as f32) * d32 > t32,
            "boundary {} is not the one at or before {} (Δ = {}, f32)",
            got,
            t32,
            d32
        );
        Ok(())
    });
}

#[test]
fn qa_m368_boundary_by_products() {
    check_products(boundary_at_or_before::<f64>, boundary_at_or_before::<f32>);
}

negative_control!(
    qa_m368_boundary_by_products,
    "the truncated quotient t/Δ is not the boundary by products",
    expected = "is not the one at or before",
    check_products(|t, d| (t / d) as u32, |t, d| (t / d) as u32)
);

// ── qa_m368_advance_lands_on_the_grid ────────────────────────────────────────────────────────────────────────────

struct Cfg;

impl AzConfig for Cfg {
    fn params<R: AzReal>() -> AzParams<R> {
        AzParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            r3_floor: R::at_width(1e-37, 1e-300),
            land_eps_rel: R::at_width(1e-5, 1e-14),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
        }
    }
}

/// `ADVANCE` to a physical target from `t = 0`.
type Reach = fn(&State<f64>, f64) -> AzAdvanced<f64>;

/// R-565: `ADVANCE` to a boundary ends at exactly its time; to a target between boundaries, at the boundary before it,
/// the same state bit for bit.
fn check_grid(reach: Reach) {
    let s = generic();
    let delta = 0.125;
    for k in [1u32, 3, 6] {
        let on = reach(&s, f64::from(k) * delta);
        assert_eq!(on.count.end, End::Marching);
        assert_eq!(
            on.t,
            f64::from(k) * delta,
            "ADVANCE to boundary {k} ends at {}",
            on.t
        );
        for frac in [0.25, 0.5, 0.999] {
            let between = reach(&s, (f64::from(k) + frac) * delta);
            assert_eq!(
                between.t,
                f64::from(k) * delta,
                "ADVANCE between boundaries does not stop at the boundary before it"
            );
            assert_eq!(
                (between.state.r, between.state.p),
                (on.state.r, on.state.p),
                "ADVANCE between boundaries does not stop at the boundary before it (state)"
            );
        }
    }
}

fn reach(s: &State<f64>, t: f64) -> AzAdvanced<f64> {
    AarsethZare::<Cfg>::advance_to::<Rk4, f64, Quiet>(s, 0.0, t, AzCount::new(), &mut Quiet)
}

#[cfg(feature = "controls")]
/// The control: the boundary after an off-grid target.
fn reach_after(s: &State<f64>, t: f64) -> AzAdvanced<f64> {
    reach(s, (t / 0.125).ceil() * 0.125)
}

#[test]
fn qa_m368_advance_lands_on_the_grid() {
    check_grid(reach);
}

negative_control!(
    qa_m368_advance_lands_on_the_grid,
    "an ADVANCE that runs on to the boundary after the target",
    expected = "does not stop at the boundary before it",
    check_grid(reach_after)
);

// ── qa_m368_slot_profile ─────────────────────────────────────────────────────────────────────────────────────────

/// A stepper type that is not a Cartesian `Stepper`, filling the slot as the Mikkola–Tanikawa leapfrog would, with
/// Part 2a's row for the pair; its step is a placeholder (the leapfrog's equations are not in the corpus, R-162).
struct MtSlot;

impl AzStepper for MtSlot {
    fn pair_profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 2,
            force_evals: 1,
            symplectic: true,
            step_reversible: true,
            run_reversible: false,
            owns_time_mapping: true,
        }
    }
    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        Form::Rk4.step(system, q, p, h)
    }
}

/// REQ-INT-024: AZ + RK4 is Part 2a's row: order 4, 4 evaluations, not symplectic, not step- or run-reversible, owning
/// its time mapping.
fn check_profile(p: CapabilityProfile) {
    let row = CapabilityProfile {
        order: 4,
        force_evals: 4,
        symplectic: false,
        step_reversible: false,
        run_reversible: false,
        owns_time_mapping: true,
    };
    assert_eq!(p, row, "AZ + RK4's profile is not Part 2a's row");
}

#[test]
fn qa_m368_slot_profile() {
    check_profile(<AarsethZare<Cfg> as Regularisation>::profile::<Rk4>());
    check_profile(<Rk4 as AzStepper>::pair_profile());
    // The slot takes a type of its own, and the march runs it to the boundary.
    let out = AarsethZare::<Cfg>::advance_to::<MtSlot, f64, Quiet>(
        &generic(),
        0.0,
        0.25,
        AzCount::new(),
        &mut Quiet,
    );
    assert_eq!(out.t, 0.25);
    assert!(<MtSlot as AzStepper>::pair_profile().symplectic);
}

negative_control!(
    qa_m368_slot_profile,
    "the stepper's own row reported for the pair",
    expected = "not Part 2a's row",
    check_profile(Rk4::profile())
);
