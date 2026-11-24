//! The Precision ring's state-in entry, `computeICState(state, simKey)`, beside `computeIC`, and the frame the IC
//! constructor integrates (ic_save §3.4; parity_contract §1).
//!
//! Pass criteria, fixed before the first run:
//! - **A picked IC**: `computeICState` on the state `computeIC` decodes (no `DEGENERATE` tag) gives a `SimState` equal
//!   to `computeIC`'s bit for bit. Control: an entry that runs `C` again on the state it is given.
//! - **A built IC in the lower hemisphere**, off every slice (its centre of mass off the origin and moving, `I_in` far
//!   from 1, `ρ` off `+x`, `λ̃_y < 0` and `L_z ≠ 0`, each asserted of the fixture first), inspected through the
//!   constructor's entry: the state integrated (the inspection's IC and the first state of its path) is
//!   - in its hemisphere: the sign of `λ̃_y` and of `L_z`, each read off the integrated state's own coordinates, are
//!     `as_built`'s, each read in `as_built`'s centre-of-mass frame. Control: the mirror folded, the twin integrated.
//!   - on the canonical section: `|Σmᵢrᵢ| ≤ 10⁻¹⁴`, `|I − 1| ≤ 10⁻¹³`, `ρ̃_x > 0` and `|ρ̃_y| ≤ 10⁻¹⁴·‖ρ̃‖`. Rounding
//!     in `C` leaves residues of a few `10⁻¹⁶` on coordinates of order 1; the controls' errors are of order 1.
//!     Control: the rescale skipped.
//!   - at rest: `|Σpᵢ| ≤ 10⁻¹⁴`. Control: the boost skipped.
//!   - bitwise `C(as_built)`, the shared `C` at f64. The test first asserts that decode(encode(`as_built`)) is not
//!     `C(as_built)` bit for bit, so its control can fail. Control: the state re-decoded from its `z`.
//!   - and its march is that state's: the `SimState` bitwise `computeICState`'s on `C(as_built)` and the constructor's
//!     entry's (`compute_built`), and the RK45 reference started from `C(as_built)`, bit for bit, reaching every time
//!     of the path. Control: the mirror twin's trajectory.
//!
//! The march is RK4 at f64 to `T = 1`: short, so the debug build stays quick.
//!
//! **Domain** (pitfalls §10): one built state, one picked chart point, the sim key, the occupant and the reference's
//! tolerances held fixed.

use kernel::canon::canonicalise;
#[cfg(feature = "controls")]
use kernel::canon::{reflect, Canonical};
use kernel::chart::Latent;
use kernel::decode::{decode, Ic};
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::first_slice::Quad;
use engine::inspector::{Inspection, Inspector};
#[cfg(feature = "controls")]
use engine::precision::built_ic;
use engine::precision::rk45::{Rk45End, Rk45Params};
use engine::precision::{
    compute_built, compute_ic, compute_ic_state, decode_only, state_of, SimState64,
};
use validation::negative_control;

/// A generic latent chart (as `precision_ring`'s).
fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

const T: f64 = 1.0;

fn key() -> SimConfig {
    let quad = Quad {
        chart: latent(),
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: T },
        ..quad.sim_key()
    }
}

fn inspector() -> Inspector<Latent<f32>> {
    Inspector {
        chart: latent(),
        sim_key: key(),
        reference: Rk45Params {
            rtol: 1e-12,
            atol: 1e-12,
            max_steps: 1_000_000,
        },
    }
}

/// Every field's bits, as text: equal texts are equal bits for every non-NaN value.
fn bits<T: core::fmt::Debug>(x: &T) -> String {
    format!("{x:?}")
}

// ── A picked IC ──────────────────────────────────────────────────────────────────────────────────────────────────

const UV: [f64; 2] = [0.3, 0.6];

/// `entry` given the state `computeIC` decodes at `UV` gives `computeIC`'s `SimState`, bit for bit.
fn check_picked(entry: impl Fn(&Ic<f64>) -> SimState64) {
    let out = decode_only::<Latent<f32>, Defaults>(&latent(), UV, &key()).unwrap();
    assert!(
        out.degenerate.is_none(),
        "the picked fixture is tagged DEGENERATE, which computeICState does not take"
    );
    let picked = compute_ic::<Latent<f32>, Defaults, Rk4>(&latent(), UV, &key()).unwrap();
    assert_eq!(
        bits(&entry(&out.ic)),
        bits(&picked),
        "computeICState on computeIC's decoded state is not computeIC's SimState, bit for bit"
    );
}

#[test]
fn compute_ic_state_picked() {
    check_picked(|ic| compute_ic_state::<Rk4>(ic, &key()).unwrap());
}

negative_control!(
    compute_ic_state_picked,
    "an entry that runs C again on the state it is given",
    expected = "is not computeIC's SimState",
    check_picked(|ic| {
        let again = canonicalise(ic).ic;
        assert_ne!(
            bits(&again),
            bits(ic),
            "C of the decoded state is the decoded state, bit for bit"
        );
        compute_ic_state::<Rk4>(&again, &key()).unwrap()
    })
);

// ── A built IC in the lower hemisphere ───────────────────────────────────────────────────────────────────────────

/// A state as the constructor builds it, off every slice: its centre of mass off the origin and moving, `I_in` about
/// 7, `ρ` off `+x`, body 2 below the line through bodies 0 and 1 (`λ̃_y < 0`), and turning (`L_z ≠ 0`).
fn as_built() -> Ic<f64> {
    Ic {
        m: [0.2, 0.3, 0.5],
        r: [[3.9, 2.1], [6.3, 5.7], [7.2, 0.6]],
        p: [[0.3, -0.1], [0.05, 0.4], [0.9, 0.6]],
    }
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn com(m: &[f64; 3], r: &[[f64; 2]; 3]) -> [f64; 2] {
    let s = |k: usize| m[0] * r[0][k] + m[1] * r[1][k] + m[2] * r[2][k];
    [s(0), s(1)]
}

fn total(p: &[[f64; 2]; 3]) -> [f64; 2] {
    [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]]
}

/// `ρ = r₁ − r₀` and `λ = r₂ − (m₀r₀ + m₁r₁)/(m₀ + m₁)`.
fn jacobi(ic: &Ic<f64>) -> ([f64; 2], [f64; 2]) {
    let (m, r) = (ic.m, ic.r);
    let m01 = m[0] + m[1];
    let inner = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    (sub(r[1], r[0]), sub(r[2], inner))
}

/// `L_z` in `ic`'s own centre-of-mass frame, at rest.
fn l_z(ic: &Ic<f64>) -> f64 {
    let (c, q) = (com(&ic.m, &ic.r), total(&ic.p));
    (0..3)
        .map(|i| {
            let r = sub(ic.r[i], c);
            let p = [ic.p[i][0] - ic.m[i] * q[0], ic.p[i][1] - ic.m[i] * q[1]];
            cross(r, p)
        })
        .sum()
}

fn inertia(ic: &Ic<f64>) -> f64 {
    let c = com(&ic.m, &ic.r);
    (0..3)
        .map(|i| {
            let d = sub(ic.r[i], c);
            ic.m[i] * (d[0] * d[0] + d[1] * d[1])
        })
        .sum()
}

/// The fixture is what the test needs it to be: every step of `C` has work to do, and the hemisphere and the turning
/// are clear of their seams.
fn check_fixture(b: &Ic<f64>) {
    assert_eq!(b.m.iter().sum::<f64>(), 1.0, "the masses do not sum to 1");
    let c = com(&b.m, &b.r);
    assert!(
        c[0].hypot(c[1]) > 1.0,
        "the centre of mass is near the origin"
    );
    let q = total(&b.p);
    assert!(q[0].hypot(q[1]) > 0.1, "the total momentum is near 0");
    assert!((inertia(b) - 1.0).abs() > 1.0, "I_in is near 1");
    let (rho, lambda) = jacobi(b);
    assert!(rho[1].abs() > 0.1 * rho[0].abs(), "ρ is near +x");
    assert!(
        cross(rho, lambda) < -0.1 * rho[0].hypot(rho[1]) * lambda[0].hypot(lambda[1]),
        "the fixture is not clearly in the lower hemisphere"
    );
    assert!(l_z(b).abs() > 0.1, "L_z is near 0");
    assert_eq!(
        encode::<Defaults, f64>(b).report.clamped_controls,
        0,
        "the fixture's encode clamps a control"
    );
}

/// The state integrated: the inspection's IC, and the first state of its path, bit for bit.
fn integrated(got: &Inspection) -> Ic<f64> {
    assert_eq!(
        bits(&got.path[0]),
        bits(&state_of(&got.ic)),
        "the path does not start at the inspection's IC"
    );
    got.ic
}

/// `λ̃_y`'s sign and `L_z`'s, read off the state integrated, are `as_built`'s.
fn check_hemisphere(b: &Ic<f64>, got: &Inspection) {
    let s = integrated(got);
    let (rho, lambda) = jacobi(b);
    let (rho_t, lambda_t) = jacobi(&s);
    eprintln!(
        "measured: as_built ρ×λ = {:e}, L_z = {:e}; integrated λ̃_y = {:e}, L_z = {:e}",
        cross(rho, lambda),
        l_z(b),
        lambda_t[1],
        l_z(&s)
    );
    assert!(rho_t[0] > 0.0, "ρ̃ is not on +x");
    assert_eq!(
        lambda_t[1].signum(),
        cross(rho, lambda).signum(),
        "λ̃_y's sign is not as_built's: the hemisphere was not kept"
    );
    assert_eq!(
        l_z(&s).signum(),
        l_z(b).signum(),
        "L_z's sign is not as_built's"
    );
}

/// The state integrated is on the canonical section: its centre of mass at the origin, `I = 1`, `ρ̃` on `+x`.
fn check_section(got: &Inspection) {
    let s = integrated(got);
    let c = com(&s.m, &s.r);
    assert!(
        c[0].abs() <= 1e-14 && c[1].abs() <= 1e-14,
        "the centre of mass is not at the origin: {c:?}"
    );
    let i = inertia(&s);
    let (rho, _) = jacobi(&s);
    eprintln!(
        "measured: CoM {c:?} (≤ 1e-14), I − 1 = {:e} (≤ 1e-13), ρ̃_y/‖ρ̃‖ = {:e} (≤ 1e-14)",
        i - 1.0,
        rho[1] / rho[0].hypot(rho[1])
    );
    assert!(
        (i - 1.0).abs() <= 1e-13,
        "I is not 1: I − 1 = {:e}",
        i - 1.0
    );
    let (rho, _) = jacobi(&s);
    assert!(
        rho[0] > 0.0 && rho[1].abs() <= 1e-14 * rho[0].hypot(rho[1]),
        "ρ̃ is not on +x: {rho:?}"
    );
}

/// The state integrated is at rest: `Σpᵢ = 0`.
fn check_at_rest(got: &Inspection) {
    let q = total(&integrated(got).p);
    eprintln!("measured: Σp = {q:?} (≤ 1e-14)");
    assert!(
        q[0].abs() <= 1e-14 && q[1].abs() <= 1e-14,
        "the total momentum is not 0: {q:?}"
    );
}

/// The state integrated is `C(as_built)` bit for bit.
fn check_bitwise(b: &Ic<f64>, got: &Inspection) {
    assert_eq!(
        bits(&integrated(got)),
        bits(&canonicalise(b).ic),
        "the state integrated is not C(as_built), bit for bit"
    );
}

/// The march is of `C(as_built)`: the `SimState` is `computeICState`'s on it and the constructor entry's, and the RK45
/// reference starts from it and reaches every time of the path.
fn check_march(b: &Ic<f64>, got: &Inspection) {
    let c = canonicalise(b).ic;
    assert_eq!(
        bits(&got.sim_state),
        bits(&compute_ic_state::<Rk4>(&c, &key()).unwrap()),
        "the SimState is not computeICState's on C(as_built)"
    );
    assert_eq!(
        bits(&got.sim_state),
        bits(&compute_built::<Rk4>(b, &key()).unwrap()),
        "the SimState is not the constructor entry's"
    );
    assert_eq!(
        got.reference.end,
        Rk45End::Reached,
        "the RK45 reference did not reach the path's times"
    );
    assert_eq!(got.reference.states.len(), got.times.len());
    assert_eq!(
        bits(&got.reference.states[0]),
        bits(&state_of(&c)),
        "the RK45 reference does not start at C(as_built)"
    );
}

/// The constructor's entry on `b`.
fn built(b: &Ic<f64>) -> Inspection {
    inspector().inspect_built::<Rk4>(b).unwrap()
}

#[cfg(feature = "controls")]
/// A state given as it is, through the state-in entry: the controls' seam.
fn given(s: &Ic<f64>) -> Inspection {
    inspector().inspect_state::<Rk4>(s).unwrap()
}

#[cfg(feature = "controls")]
/// `C(b)` with step 0 undone: `r·√I_in`, `p/I_in^{1/4}`, so the state is at the input's scale.
fn rescale_skipped(b: &Ic<f64>) -> Ic<f64> {
    let Canonical { ic, i_in, .. } = canonicalise(b);
    let (k, j) = (i_in.sqrt(), i_in.sqrt().sqrt());
    Ic {
        m: ic.m,
        r: ic.r.map(|[x, y]| [x * k, y * k]),
        p: ic.p.map(|[x, y]| [x / j, y / j]),
    }
}

#[cfg(feature = "controls")]
/// `C(b)` with step 1b undone: each `pᵢ` given back `mᵢ` of the input's total momentum, scaled by step 0 and turned
/// by step 2.
fn boost_skipped(b: &Ic<f64>) -> Ic<f64> {
    let Canonical { ic, i_in, phi, .. } = canonicalise(b);
    let q = total(&b.p);
    let j = i_in.sqrt().sqrt();
    let (c, s) = (phi.cos(), phi.sin());
    let q = [j * (c * q[0] + s * q[1]), j * (-s * q[0] + c * q[1])];
    Ic {
        m: ic.m,
        r: ic.r,
        p: core::array::from_fn(|i| [ic.p[i][0] + ic.m[i] * q[0], ic.p[i][1] + ic.m[i] * q[1]]),
    }
}

#[test]
fn compute_ic_state_built_keeps_its_hemisphere() {
    let b = as_built();
    check_fixture(&b);
    check_hemisphere(&b, &built(&b));
}

negative_control!(
    compute_ic_state_built_keeps_its_hemisphere,
    "an entry that folds the mirror, integrating the twin",
    expected = "the hemisphere was not kept",
    {
        let b = as_built();
        check_hemisphere(&b, &given(&reflect(&built_ic(&b), true)))
    }
);

#[test]
fn compute_ic_state_built_on_the_section() {
    let b = as_built();
    check_fixture(&b);
    check_section(&built(&b));
}

negative_control!(
    compute_ic_state_built_on_the_section,
    "an entry that skips the rescale",
    expected = "I is not 1",
    {
        let b = as_built();
        check_section(&given(&rescale_skipped(&b)))
    }
);

#[test]
fn compute_ic_state_built_at_rest() {
    let b = as_built();
    check_fixture(&b);
    check_at_rest(&built(&b));
}

negative_control!(
    compute_ic_state_built_at_rest,
    "an entry that skips the boost",
    expected = "the total momentum is not 0",
    {
        let b = as_built();
        check_at_rest(&given(&boost_skipped(&b)))
    }
);

#[test]
fn compute_ic_state_built_is_c_bitwise() {
    let b = as_built();
    check_fixture(&b);
    let c = canonicalise(&b).ic;
    let redecoded = decode::<Defaults, f64>(&encode::<Defaults, f64>(&b).z).ic;
    assert_ne!(
        bits(&redecoded),
        bits(&c),
        "decode(encode(as_built)) is C(as_built) bit for bit, so the re-decode control cannot fail"
    );
    check_bitwise(&b, &built(&b));
}

negative_control!(
    compute_ic_state_built_is_c_bitwise,
    "an entry that re-decodes the state from its z",
    expected = "the state integrated is not C(as_built), bit for bit",
    {
        let b = as_built();
        let redecoded = decode::<Defaults, f64>(&encode::<Defaults, f64>(&b).z).ic;
        let got = given(&redecoded);
        // The re-decoded state is canonical too: only its bits give it away.
        check_hemisphere(&b, &got);
        check_section(&got);
        check_at_rest(&got);
        check_bitwise(&b, &got)
    }
);

#[test]
fn compute_ic_state_built_march() {
    let b = as_built();
    check_fixture(&b);
    check_march(&b, &built(&b));
}

negative_control!(
    compute_ic_state_built_march,
    "an entry that integrates the mirror twin's trajectory",
    expected = "the SimState is not computeICState's on C(as_built)",
    {
        let b = as_built();
        check_march(&b, &given(&reflect(&built_ic(&b), true)))
    }
);
