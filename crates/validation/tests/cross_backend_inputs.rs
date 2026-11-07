//! What the kernel cross-backend check's inputs are (TASK-M4-27; REQ-VAL-347): each builder of
//! `validation::cross_backend`, through what `build_inputs` gives, on every platform.
//!
//! The comparison runs every backend on whatever states it is given, so a builder that drifts still agrees with itself,
//! and the one test that held them bit for bit runs on macOS alone: the builders call the platform's `sin_cos` and `powf`
//! (`cross_backend_read_back_fixture_is_the_builders`). So nothing else on CI told a state from any other. These tests
//! say what the states are:
//! - first the property each state is built to have, as the file it is copied from states it
//!   (`crates/engine/tests/canon_gpu.rs`, `crates/engine/tests/decode_cross_backend.rs`,
//!   `crates/validation/tests/qa_TASK-M2-25_gpu.rs`). A mirror tie's `λ̃_y` is `−a` exactly; a `τ_I` state's `I_in` is its
//!   target exactly; the hierarchy's `λ̃_y` is `−k·δ_λ`; a crossing's two points are adjacent floats either side of
//!   `M01_TINY`. Where the file defines a state by a formula on the platform's own `sin_cos`, the test evaluates that
//!   formula on the same platform, so it holds bit for bit there too.
//! - then the fixture itself, which the Deliverables make the builders' output, written once. What no platform call
//!   reaches is held bit for bit on every platform ([`check_fixture_exact`]). What one reaches is held to [`NEAR`]
//!   ([`check_fixture_near`]), which is a test's reach, not a measurement's: no rate reads it (R-649).
//!
//! No test here needs a GPU. Each registers the control that must make it fail (R-176).

use std::f64::consts::SQRT_2;
use std::path::Path;
use std::sync::OnceLock;

use kernel::canon::{canonicalise, lambda_t_y, scale_step};
use kernel::chart::{Latent, ShapeMap, ShapeSphere};
use kernel::decode::{decode, Ic};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{Defaults, Link, StickBreaking};
use kernel::real::IeeeBits;
use validation::cross_backend::{build_inputs, chart_of, with_points, Inputs};
use validation::negative_control;

const EPS32: f64 = f32::EPSILON as f64;

/// A word of what a platform's `sin_cos`, `powf` or `exp` reaches, against the fixture's: the fraction of its scale it
/// may move. The platforms round those calls apart by an ulp or so, which a saturated decode or a `θ = 0.7` edge
/// bisection amplifies; every builder's change moves a word by a percent or more.
const NEAR: f64 = 1e-3;

const TIE: &str = "canonicaliser_mirror_tie";
const TURNED: &str = "canonicaliser_mirror_tie turned";
const SEAM: &str = "canonicaliser_seam_sweep seam";
const TAU: &str = "canonicaliser_seam_sweep tau";
const SUBNORMAL: &str = "canonicaliser_seam_sweep subnormal";
const RANGE: &str = "canonicaliser_seam_sweep range";
const SCALE: &str = "canonicaliser_scale_invariance";
const HIERARCHY: &str = "canonicaliser_hierarchy";
const EDGE: &str = "canonicaliser_hierarchy_edge";
const ORDER: &str = "canonicaliser_canonicalise_order";
const COINCIDENT: &str = "canonicaliser_coincident_inner_pair";
const SIGNED_ZERO: &str = "canonicaliser_phi_signed_zero";
const NOOP: &str = "canonicaliser_noop_gpu";
const NOOP_WIDE: &str = "canonicaliser_noop_gpu wider";

/// `M₀₁`'s crossings' batch, #393's batches and the sphere's pole batch.
const CROSSINGS: &str = "M₀₁ onset crossings";
const BATCH_393: &str = "#393";
const POLE_BATCH: &str = "n₀ = ±1 poles";

/// `decode_cross_backend.rs`' `η_E` targets.
const TARGETS: [f32; 4] = [0.0, -0.3, -40.0, 1.5];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// What the builders give, built once per test process.
fn built() -> &'static Inputs {
    static BUILT: OnceLock<Inputs> = OnceLock::new();
    BUILT.get_or_init(build_inputs)
}

/// The states of `name`.
fn part<'a>(inputs: &'a Inputs, name: &str) -> &'a [Ic<f32>] {
    let mut at = 0;
    for p in &inputs.parts {
        if p.name == name {
            return &inputs.canon[at..at + p.count];
        }
        at += p.count;
    }
    panic!("no part named {name}");
}

fn bits(x: &Ic<f32>) -> Vec<u32> {
    let r = x.r.iter().flatten();
    let p = x.p.iter().flatten();
    x.m.iter().chain(r).chain(p).map(|q| q.to_bits()).collect()
}

fn widen(x: &Ic<f32>) -> Ic<f64> {
    Ic {
        m: x.m.map(f64::from),
        r: x.r.map(|v| v.map(f64::from)),
        p: x.p.map(|v| v.map(f64::from)),
    }
}

fn narrow(x: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: x.m.map(|q| q as f32),
        r: x.r.map(|v| v.map(|q| q as f32)),
        p: x.p.map(|v| v.map(|q| q as f32)),
    }
}

/// The masses of a generic state: each of the weights' share, `lo` and `hi` the shares' bounds, and a sum of 1.
fn is_simplex(m: [f32; 3], lo: f32, hi: f32) -> bool {
    let sum = m.iter().map(|q| f64::from(*q)).sum::<f64>();
    (sum - 1.0).abs() <= 4.0 * EPS32 && m.iter().all(|q| (lo..=hi).contains(q))
}

fn between(v: &[[f32; 2]; 3], lo: f32, hi: f32) -> bool {
    v.iter().flatten().all(|q| (lo..=hi).contains(q))
}

// ── The parts ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Each test's states, in the order of `build_inputs`, and how many, from the test's own loops.
fn check_parts(inputs: &Inputs) {
    let want = [
        (TIE, 10),
        (TURNED, 10 * 3 * 2),
        (SEAM, 20_000),
        (TAU, 6 * 8),
        (SUBNORMAL, 2000),
        (RANGE, 8 + 4),
        (SCALE, (1 + 8 + 6) * 2000),
        (HIERARCHY, 6 * 3 * 2000),
        (EDGE, 4 * 2 * 40 * 61),
        (ORDER, 3 * 3 * 3 * 2),
        (COINCIDENT, 24 + 4 + 4),
        (SIGNED_ZERO, 4),
        (NOOP, 16 * 64 + 8 * 4 + 4 * 4),
        (NOOP_WIDE, 64 * 1024),
    ];
    let got: Vec<(&str, usize)> = inputs
        .parts
        .iter()
        .map(|p| (p.name.as_str(), p.count))
        .collect();
    assert!(got == want, "the parts are not the tests': {got:?}");
    let total: usize = want.iter().map(|w| w.1).sum();
    assert_eq!(inputs.canon.len(), total, "the parts are not the states");
    assert_eq!(inputs.len(), total + inputs.points.len());
}

#[test]
fn cross_backend_inputs_are_the_tests_parts() {
    check_parts(built());
}

negative_control!(
    cross_backend_inputs_are_the_tests_parts,
    "a part short of a state",
    expected = "the parts are not the tests'",
    {
        let mut inputs = build_inputs();
        inputs.parts[3].count -= 1;
        check_parts(&inputs);
    }
);

// ── canonicaliser_mirror_tie ─────────────────────────────────────────────────────────────────────────────────────

/// `tie_cases`' `a`: `±δ_λ`, one step either side of each, `±2δ_λ` and `±0`.
fn tie_values<R: ChartConstants + std::ops::Neg<Output = R> + std::ops::Add<Output = R> + Copy>(
    up: fn(R) -> R,
    down: fn(R) -> R,
    zero: R,
) -> [R; 10] {
    let d = R::DELTA_LAMBDA;
    [
        d,
        up(d),
        down(d),
        d + d,
        -d,
        -up(d),
        -down(d),
        -(d + d),
        zero,
        -zero,
    ]
}

/// The tie: masses `(¼, ¼, ½)`, `r₀ = r₁ = (−1, a)`, `r₂ = (1, −a)`, at rest, and from those `λ̃_y = −a` exactly and
/// `I = 1` exactly, which is what makes its comparison one rounding on every backend.
fn check_ties(inputs: &Inputs) {
    let states = part(inputs, TIE);
    for (x, a) in states
        .iter()
        .zip(tie_values(f32::next_up, f32::next_down, 0.0))
    {
        let want = Ic {
            m: [0.25, 0.25, 0.5],
            r: [[-1.0, a], [-1.0, a], [1.0, -a]],
            p: [[0.0; 2]; 3],
        };
        assert!(bits(x) == bits(&want), "a tie is not the tie at a = {a:e}");
        assert!(
            lambda_t_y(x.m, x.r) == -a && scale_step(x).i_in == 1.0,
            "a tie's branch inputs are not −a and 1, at a = {a:e}"
        );
    }
}

#[test]
fn cross_backend_inputs_ties_are_the_deadband_edge() {
    check_ties(built());
}

negative_control!(
    cross_backend_inputs_ties_are_the_deadband_edge,
    "a tie's third body on the first's side",
    expected = "a tie is not the tie",
    {
        let mut inputs = build_inputs();
        inputs.canon[0].r[2][1] = -inputs.canon[0].r[2][1];
        check_ties(&inputs);
    }
);

/// `x` rotated by `θ`, translated by `(0.75, −1.25)` and boosted by `(0.3, −0.2)`, as `turned` in `canon_gpu.rs` has it,
/// on this platform's `sin_cos`.
fn turned(x: &Ic<f64>, theta: f64) -> Ic<f32> {
    let (s, c) = theta.sin_cos();
    let rot = |v: [f64; 2]| [c * v[0] - s * v[1], s * v[0] + c * v[1]];
    narrow(&Ic {
        m: x.m,
        r: x.r.map(|v| {
            let v = rot(v);
            [v[0] + 0.75, v[1] - 1.25]
        }),
        p: [0, 1, 2].map(|i| {
            let v = rot(x.p[i]);
            [v[0] + 0.3 * x.m[i], v[1] - 0.2 * x.m[i]]
        }),
    })
}

/// Each tie and its companion off the pole, `r = [(−1, a), (1, a), (0, −a)]` at `a/√2`, so that `λ̃_y` is near `−a`, each
/// turned at the test's three angles.
fn check_turned(inputs: &Inputs) {
    let mut states = part(inputs, TURNED).iter();
    for a in tie_values(f64::next_up, f64::next_down, 0.0) {
        for theta in [0.3, 1.9, -2.6] {
            let tie = Ic {
                m: [0.25, 0.25, 0.5],
                r: [[-1.0, a], [-1.0, a], [1.0, -a]],
                p: [[0.0; 2]; 3],
            };
            let b = a / SQRT_2;
            let axis = Ic {
                r: [[-1.0, b], [1.0, b], [0.0, -b]],
                ..tie
            };
            for base in [tie, axis] {
                let x = states.next().expect("a state for each turned tie");
                let want = turned(&base, theta);
                assert!(
                    bits(x) == bits(&want),
                    "a turned tie is not its tie's rigid image, at a = {a:e}, θ = {theta}"
                );
            }
        }
    }
}

#[test]
fn cross_backend_inputs_turned_ties_are_rigid_images() {
    check_turned(built());
}

negative_control!(
    cross_backend_inputs_turned_ties_are_rigid_images,
    "a turned tie moved",
    expected = "a turned tie is not its tie's rigid image",
    {
        let mut inputs = build_inputs();
        let at = 10 + 3;
        inputs.canon[at].r[0][0] += 0.125;
        check_turned(&inputs);
    }
);

// ── canonicaliser_seam_sweep ─────────────────────────────────────────────────────────────────────────────────────

/// Collinear states through a point of `[−½, ½]²`, so `λ̃_y` is 0 in the reals, every eighth with `r₁ = r₀` bit for bit,
/// at all angles.
fn check_seam(inputs: &Inputs) {
    let states = part(inputs, SEAM);
    let mut quarters = [0usize; 4];
    for (n, x) in states.iter().enumerate() {
        assert!(
            is_simplex(x.m, 0.2 / 3.8, 1.2 / 1.6),
            "a seam state's masses are off the simplex, state {n}"
        );
        assert!(
            between(&x.r, -1.5, 1.5) && between(&x.p, -0.5, 0.5),
            "a seam state is out of its box, state {n}"
        );
        assert_eq!(
            x.r[0] == x.r[1],
            n % 8 == 0,
            "every eighth seam state and no other has r₁ = r₀, state {n}"
        );
        let w = widen(x);
        let [d, e] = [1, 2].map(|i| [w.r[i][0] - w.r[0][0], w.r[i][1] - w.r[0][1]]);
        let cross = d[0] * e[1] - d[1] * e[0];
        assert!(
            cross.abs() <= 16.0 * EPS32,
            "a seam state is not collinear, state {n}: {cross:e}"
        );
        let far = if n % 8 == 0 { e } else { d };
        let line = far[1].atan2(far[0]).rem_euclid(std::f64::consts::PI);
        quarters[(line / std::f64::consts::FRAC_PI_4) as usize % 4] += 1;
    }
    assert!(
        quarters.iter().all(|q| *q > states.len() / 8),
        "the seam's lines do not take every angle: {quarters:?}"
    );
}

#[test]
fn cross_backend_inputs_seam_states_are_collinear() {
    check_seam(built());
}

negative_control!(
    cross_backend_inputs_seam_states_are_collinear,
    "a seam state's third body off its line",
    expected = "a seam state is not collinear",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 5;
        inputs.canon[at].r[2] = [-inputs.canon[at].r[2][1], inputs.canon[at].r[2][0] + 0.25];
        check_seam(&inputs);
    }
);

/// `τ_I`'s edge, either side of 1, and one ulp of `I_in` either side of it, each `at_inertia` target's eight states:
/// `I_in` as `C` forms it is the target exactly, and step 0 runs iff `|I_in − 1| > τ_I`.
fn check_tau(inputs: &Inputs) {
    let (e, tau) = (EPS32, f64::from(f32::TAU_I));
    let targets = [
        1.0 + tau - e,
        1.0 + tau,
        1.0 + tau + e,
        1.0 - tau + e / 2.0,
        1.0 - tau,
        1.0 - tau - e / 2.0,
    ];
    let states = part(inputs, TAU);
    assert_eq!(states.len(), 6 * 8);
    for (run, target) in states.chunks(8).zip(targets) {
        for x in run {
            let i = f64::from(scale_step(x).i_in);
            assert!(
                i == target,
                "a τ_I state's I_in is {i}, not its target {target}"
            );
            assert!(
                canonicalise(x).rescaled == ((i - 1.0).abs() > tau),
                "a τ_I state's skip is not its I_in's, at {i}"
            );
        }
        assert!(
            run.windows(2).all(|w| w[0].m != w[1].m),
            "a τ_I state repeats"
        );
    }
    let skips = states.iter().filter(|x| !canonicalise(x).rescaled).count();
    assert_eq!(skips, 4 * 8, "τ_I's edge is not either side of its states");
}

#[test]
fn cross_backend_inputs_tau_states_are_at_their_targets() {
    check_tau(built());
}

negative_control!(
    cross_backend_inputs_tau_states_are_at_their_targets,
    "a τ_I state's positions one step out",
    expected = "a τ_I state's I_in is",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 9;
        inputs.canon[at].r = inputs.canon[at].r.map(|v| v.map(|q| q * 1.0625));
        check_tau(&inputs);
    }
);

/// `canon_gpu.rs`' subnormal states: `r₀` near `1e-36`, the inner pair thousands of steps apart in `x`, `y` or both, or
/// `1e-37` apart in `x`, or coincident; every seventh third body thousands of steps from `r₀`.
fn check_subnormal(inputs: &Inputs) {
    let steps = |a: f32, b: f32| b.to_bits() as i64 - a.to_bits() as i64;
    let near = |a: f32, b: f32| (1..=4000).contains(&steps(a, b));
    for (n, x) in part(inputs, SUBNORMAL).iter().enumerate() {
        let [r0, r1, r2] = x.r;
        let fine = is_simplex(x.m, 0.2 / 3.8, 1.2 / 1.6)
            && between(&x.p, -0.5, 0.5)
            && r0.iter().all(|q| (0.5e-36..=2.0e-36).contains(q));
        let (dx, dy) = (near(r0[0], r1[0]), near(r0[1], r1[1]));
        let (same_x, same_y) = (r0[0] == r1[0], r0[1] == r1[1]);
        let tenth = (f64::from(r1[0]) - f64::from(r0[0]) - 1e-37).abs() < 2e-43;
        let inner = match n % 5 {
            0 => dx && same_y,
            1 => same_x && dy,
            2 => dx && dy,
            3 => tenth && dy,
            _ => same_x && same_y,
        };
        let third = if n % 7 == 0 {
            near(r0[0], r2[0]) && near(r0[1], r2[1])
        } else {
            // Both from `r₀`'s `x`: the test's `[0; 2].map(|j| r0[j] + …)` reads `j` as the element, 0 each time.
            r2.iter().all(|q| {
                let d = f64::from(*q) - f64::from(r0[0]);
                (0.19e-36..=1.51e-36).contains(&d)
            })
        };
        assert!(
            fine && inner && third,
            "a subnormal state is not one of its classes, state {n}"
        );
    }
}

#[test]
fn cross_backend_inputs_subnormal_states_are_their_classes() {
    check_subnormal(built());
}

negative_control!(
    cross_backend_inputs_subnormal_states_are_their_classes,
    "a coincident inner pair parted",
    expected = "a subnormal state is not one of its classes",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 4;
        inputs.canon[at].r[1][0] *= 2.0;
        check_subnormal(&inputs);
    }
);

/// The bodies apart at each `k`, and the three coincident at each point.
fn check_range(inputs: &Inputs) {
    let states = part(inputs, RANGE);
    let p = [[0.1, 0.0], [-0.3, 0.2], [0.1, -0.1]];
    let ks = [1e-37f32, 1e-30, 1e-25, 1e-20, 1e19, 1e20, 1e30, 1e37];
    let far = ks.map(|k| Ic {
        m: [0.25, 0.25, 0.5],
        r: [[-k, 0.0], [k, 0.0], [0.0, k]],
        p,
    });
    let points = [[0.0f32, 0.0], [0.3, -1.7], [-2.5e-3, 7.0], [1e-36, 3e-36]];
    let coincident = points.map(|q| Ic {
        m: [0.2, 0.3, 0.5],
        r: [q; 3],
        p,
    });
    let want: Vec<Vec<u32>> = far.iter().chain(&coincident).map(bits).collect();
    let got: Vec<Vec<u32>> = states.iter().map(bits).collect();
    assert!(
        got == want,
        "the range states are not the far and coincident"
    );
}

#[test]
fn cross_backend_inputs_range_states_are_far_and_coincident() {
    check_range(built());
}

negative_control!(
    cross_backend_inputs_range_states_are_far_and_coincident,
    "a far state moved",
    expected = "the range states are not the far and coincident",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000;
        inputs.canon[at].m[0] = 0.5;
        check_range(&inputs);
    }
);

// ── canonicaliser_scale_invariance ───────────────────────────────────────────────────────────────────────────────

/// Generic states, then those positions at each power of two, which scales the f32 words exactly, and at each decimal,
/// where they round anew; masses and momenta untouched.
fn check_scale(inputs: &Inputs) {
    let states = part(inputs, SCALE);
    let (base, rest) = states.split_at(2000);
    let powers = [-60, -40, -30, -27, 27, 30, 40, 60].map(|e| (2f64.powi(e), true));
    let decimals = [1e-8, 1e-9, 1e-10, 1e-12, 1e16, 1e18].map(|k| (k, false));
    let scales = powers.iter().chain(&decimals);
    for x in base {
        assert!(
            is_simplex(x.m, 0.2 / 3.8, 1.2 / 1.6)
                && between(&x.r, -1.0, 1.0)
                && between(&x.p, -0.5, 0.5),
            "a generic state is out of its box"
        );
    }
    for (run, (k, exact)) in rest.chunks(2000).zip(scales) {
        for (x, b) in run.iter().zip(base) {
            assert!(
                x.m == b.m && x.p == b.p,
                "a scale moved a mass or a momentum"
            );
            for (v, w) in x.r.iter().flatten().zip(b.r.iter().flatten()) {
                let want = f64::from(*w) * k;
                let off = (f64::from(*v) - want).abs();
                let tolerance = if *exact { 0.0 } else { EPS32 * want.abs() };
                assert!(off <= tolerance, "a state is not at its scale {k:e}");
            }
        }
    }
}

#[test]
fn cross_backend_inputs_scale_states_are_at_each_scale() {
    check_scale(built());
}

negative_control!(
    cross_backend_inputs_scale_states_are_at_each_scale,
    "a state left at the wrong scale",
    expected = "is not at its scale",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 2000 + 5;
        inputs.canon[at].r[1][0] *= 2.0;
        check_scale(&inputs);
    }
);

// ── canonicaliser_hierarchy ──────────────────────────────────────────────────────────────────────────────────────

/// `‖ρ‖` over the outer distance.
const RATIOS: [f64; 6] = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8];

/// Hierarchical states, each ratio's edge on `+x`, its edge at a random angle and `−2δ_λ` on `+x`: `r₀ = 0`, `‖ρ‖` the
/// ratio, `r₂` at distance `0.5…2` from the pair and, on `+x`, where the offset survives the rounding to f32, `λ̃_y` is
/// `−k·δ_λ` with `k = 1 ± 10⁻⁶` or 2, the latter mirroring.
fn check_hierarchy(inputs: &Inputs) {
    for (n, x) in part(inputs, HIERARCHY).iter().enumerate() {
        let (ratio, block) = (RATIOS[n / 6000], (n / 2000) % 3);
        let w = widen(x);
        let rho = w.r[1][0].hypot(w.r[1][1]);
        let m01 = w.m[0] + w.m[1];
        let pair = [0, 1].map(|j| w.r[2][j] - w.m[1] * w.r[1][j] / m01);
        let far = pair[0].hypot(pair[1]);
        let shape = is_simplex(x.m, 0.01 / 2.01, 1.0 / 1.02)
            && x.r[0] == [0.0, 0.0]
            && (rho / ratio - 1.0).abs() <= EPS32
            && (0.49..=2.01).contains(&far)
            && x.p == [[0.1, 0.0], [0.0, -0.2], [0.05, 0.1]];
        assert!(shape, "a hierarchical state is not its shape, state {n}");
        if block == 1 {
            continue;
        }
        let axis = x.r[1][1] == 0.0 && x.r[1][0] > 0.0;
        assert!(axis, "an axis state's pair is off +x, state {n}");
        let k = lambda_t_y(w.m, w.r) / scale_step(&w).i_in.sqrt() / -f64::DELTA_LAMBDA;
        let want = if block == 0 { 1.0 } else { 2.0 };
        assert!(
            (k / want - 1.0).abs() <= 2e-6,
            "a hierarchical state's λ̃_y is {k}·δ_λ, not ±{want}, state {n}"
        );
        assert!(
            block == 0 || canonicalise(x).mirrored,
            "a state at −2δ_λ does not mirror, state {n}"
        );
    }
}

#[test]
fn cross_backend_inputs_hierarchy_states_are_at_their_offset() {
    check_hierarchy(built());
}

negative_control!(
    cross_backend_inputs_hierarchy_states_are_at_their_offset,
    "a hierarchical state's offset doubled",
    expected = "a hierarchical state's λ̃_y is",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 3;
        inputs.canon[at].r[2][1] *= 2.0;
        check_hierarchy(&inputs);
    }
);

// ── canonicaliser_hierarchy_edge ─────────────────────────────────────────────────────────────────────────────────

/// `EDGE_RATIOS`, the inner pair's half-separation over the outer distance as powers of two.
const EDGE_RATIOS: [i32; 4] = [-12, -15, -20, -40];

/// The edge's sweeps, 61 consecutive f32 `a`, 40 draws at each ratio and angle.
const SWEEP: usize = 61;

/// One sweep on `+x`: `r = [(−u, a), (u, a), (x₂, −a)]` at rest, `a` stepping one float at a time across the deadband's
/// edge, so that the first state does not mirror and the last does.
fn check_edge_sweep(run: &[Ic<f32>], j: i32) {
    let a0 = run[0].r[0][1];
    let (u, x2) = (run[0].r[1][0], run[0].r[2][0]);
    assert!(
        (1.0..=2.0).contains(&(f64::from(u) / 2f64.powi(j))) && (0.5..=1.5).contains(&x2),
        "a sweep's pair or third body is not at its ratio"
    );
    let mut a = a0;
    for x in run {
        let want = [[-u, a], [u, a], [x2, -a]];
        assert!(
            x.r == want && x.m == run[0].m && x.p == [[0.0; 2]; 3],
            "a sweep is not one float of a at a time"
        );
        a = a.next_up();
    }
    assert!(
        !canonicalise(&run[0]).mirrored && canonicalise(&run[SWEEP - 1]).mirrored,
        "a sweep does not cross the deadband's edge, at 2^{j}"
    );
}

/// The states of each ratio and angle: `+x`'s sweeps, and the same draws (the same seed) turned by 0.7, whose `a` the
/// turn's bisection finds again to within its own rounding.
fn check_edge(inputs: &Inputs) {
    let states = part(inputs, EDGE);
    let (cs, sn) = (0.7f64.cos(), 0.7f64.sin());
    let turn = |v: [f64; 2]| [v[0] * cs - v[1] * sn, v[0] * sn + v[1] * cs];
    for (k, block) in states.chunks(2 * 40 * SWEEP).enumerate() {
        let (flat, turned) = block.split_at(40 * SWEEP);
        for (run, other) in flat.chunks(SWEEP).zip(turned.chunks(SWEEP)) {
            check_edge_sweep(run, EDGE_RATIOS[k]);
            let (x, y) = (widen(&run[SWEEP / 2]), widen(&other[SWEEP / 2]));
            assert!(y.m == x.m, "a turned sweep is not the same draw's");
            let size = x.r[1][0].max(x.r[0][1].abs());
            for i in 0..3 {
                let reach = if i == 2 { 2.0 * EPS32 } else { 0.01 * size };
                let want = turn(x.r[i]);
                assert!(
                    (0..2).all(|j| (want[j] - y.r[i][j]).abs() <= reach + EPS32 * want[j].abs()),
                    "a turned sweep is not its draw's turn"
                );
            }
        }
    }
}

#[test]
fn cross_backend_inputs_edge_states_sweep_the_deadband() {
    check_edge(built());
}

negative_control!(
    cross_backend_inputs_edge_states_sweep_the_deadband,
    "a sweep's state moved a float",
    expected = "a sweep is not one float of a at a time",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 36_000 + 7;
        let a = inputs.canon[at].r[0][1];
        inputs.canon[at].r[0][1] = a.next_up();
        check_edge(&inputs);
    }
);

// ── canonicaliser_canonicalise_order ─────────────────────────────────────────────────────────────────────────────

/// `representatives()` of `canon_gpu.rs`: decodes well outside the deadband.
fn representatives() -> [Ic<f64>; 3] {
    [
        [0.4f64, -0.3, 0.5, -0.2, 0.3, 0.1, 0.6, -0.4],
        [-0.8, 1.1, -0.4, 0.7, -0.6, 0.2, -0.3, 0.9],
        [1.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ]
    .map(|z| decode::<Defaults, f64>(&kernel::decode::Latent::new(z)).ic)
}

/// dd_encode §3.3's gauge at the test's values: `rᵢ' = s·R(θ)·Π·rᵢ + a`, `pᵢ' = s^{−1/2}·R(θ)·Π·pᵢ + mᵢu`, on this
/// platform's `sin_cos` and `powf`.
fn gauged(x: &Ic<f64>, a: [f64; 2], theta: f64, s: f64, mirror: bool) -> Ic<f32> {
    let (sn, cs) = theta.sin_cos();
    let map = |v: [f64; 2], k: f64| {
        let y = if mirror { -v[1] } else { v[1] };
        [k * (cs * v[0] - sn * y), k * (sn * v[0] + cs * y)]
    };
    let u = [0.4, -1.1];
    narrow(&Ic {
        m: x.m,
        r: x.r.map(|v| {
            let w = map(v, s);
            [w[0] + a[0], w[1] + a[1]]
        }),
        p: [0, 1, 2].map(|i| {
            let w = map(x.p[i], s.powf(-0.5));
            [w[0] + x.m[i] * u[0], w[1] + x.m[i] * u[1]]
        }),
    })
}

/// Each representative under each gauge: three scales, three angles, both mirrors, a translation of several times the
/// state's size; `C` of it mirrors iff `Π` does and reads `I_in = s²`.
fn check_order(inputs: &Inputs) {
    let mut states = part(inputs, ORDER).iter();
    for rep in representatives() {
        for (k, s) in [0.5, 3.0, 40.0].into_iter().enumerate() {
            for theta in [0.7, 2.9, -1.3] {
                for mirror in [false, true] {
                    let x = states.next().expect("a state for each gauge");
                    let a = [3.0 * s, -2.0 * s - k as f64];
                    assert!(
                        bits(x) == bits(&gauged(&rep, a, theta, s, mirror)),
                        "an order state is not its gauge of its representative"
                    );
                    let c = canonicalise(x);
                    let off = (f64::from(c.i_in) / (s * s) - 1.0).abs();
                    assert!(
                        c.mirrored == mirror && off <= 64.0 * EPS32,
                        "C of an order state does not read its gauge"
                    );
                }
            }
        }
    }
}

#[test]
fn cross_backend_inputs_order_states_are_the_gauges() {
    check_order(built());
}

negative_control!(
    cross_backend_inputs_order_states_are_the_gauges,
    "a gauge's boost moved",
    expected = "an order state is not its gauge of its representative",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 36_000 + 19_520 + 4;
        inputs.canon[at].p[1][0] += 0.25;
        check_order(&inputs);
    }
);

// ── canonicaliser_coincident_inner_pair, canonicaliser_phi_signed_zero, canonicaliser_noop_gpu ───────────────────────

/// `r₀ = r₁` bit for bit, the third body elsewhere; then every body at the origin, `r₁` each sign of 0; then the first
/// state's `r₁` moved `2⁻²⁰` along each axis.
fn check_coincident(inputs: &Inputs) {
    let states = part(inputs, COINCIDENT);
    let (bases, rest) = states.split_at(24);
    for x in bases {
        assert!(
            x.r[0] == x.r[1]
                && x.r[0] != x.r[2]
                && is_simplex(x.m, 0.1 / 2.1, 1.0 / 1.2)
                && between(&x.r, -3.0, 3.0)
                && between(&x.p, -1.0, 1.0),
            "a coincident state's pair is not coincident"
        );
    }
    let (m, p) = (
        [0.25, 0.375, 0.375],
        [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]],
    );
    for (x, y) in rest[..4]
        .iter()
        .zip([[0.0, 0.0], [-0.0, 0.0], [-0.0, -0.0], [0.0, -0.0]])
    {
        let want = Ic {
            m,
            r: [[0.0; 2], y, [0.0; 2]],
            p,
        };
        assert!(bits(x) == bits(&want), "a signed pole is not its zeros");
    }
    let step = 2f64.powi(-20);
    let offs: [[f64; 2]; 4] = [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]];
    for (x, off) in rest[4..].iter().zip(offs) {
        let b = &bases[0];
        assert!(
            x.m == b.m && x.p == b.p && x.r[0] == b.r[0] && x.r[2] == b.r[2],
            "a parted pair is not the first state's"
        );
        for j in 0..2 {
            let moved = f64::from(x.r[1][j]) - f64::from(b.r[0][j]);
            assert!(
                (moved - off[j] * step).abs() <= 0.25 * step,
                "a parted pair is not 2⁻²⁰ along its axis"
            );
        }
    }
}

#[test]
fn cross_backend_inputs_coincident_states_are_the_pair() {
    check_coincident(built());
}

negative_control!(
    cross_backend_inputs_coincident_states_are_the_pair,
    "a coincident pair parted",
    expected = "a coincident state's pair is not coincident",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 36_000 + 19_520 + 54 + 2;
        inputs.canon[at].r[1][0] += 1.0;
        check_coincident(&inputs);
    }
);

/// `canonicaliser_phi_signed_zero`'s states: `ρ_x` a signed zero, `ρ` along `y`, each way.
fn check_signed_zero(inputs: &Inputs) {
    let (m, p) = (
        [0.25, 0.375, 0.375],
        [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]],
    );
    let tiny = (2f64.powi(-20)) as f32;
    let want = [
        [[0.0, -1.0], [-0.0, 1.0], [0.7, 0.2]],
        [[0.0, 1.0], [-0.0, -1.0], [-0.7, 0.2]],
        [[0.0, 0.0], [-0.0, tiny], [0.5, -0.3]],
        [[-0.0, 1.0], [0.0, -1.0], [0.4, 0.6]],
    ];
    let states = part(inputs, SIGNED_ZERO);
    assert_eq!(states.len(), want.len());
    for (x, r) in states.iter().zip(want) {
        let want = Ic { m, r, p };
        assert!(bits(x) == bits(&want), "a signed-zero state is not its own");
    }
}

#[test]
fn cross_backend_inputs_signed_zero_states_are_the_tests() {
    check_signed_zero(built());
}

negative_control!(
    cross_backend_inputs_signed_zero_states_are_the_tests,
    "a zero's sign flipped",
    expected = "a signed-zero state is not its own",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 36_000 + 19_520 + 54 + 32 + 1;
        inputs.canon[at].r[0][0] = -0.0;
        check_signed_zero(&inputs);
    }
);

/// dd_decoder §5 test 5 at f32: `C` of a decode takes no mirror and turns nothing.
fn check_noop(inputs: &Inputs) {
    for name in [NOOP, NOOP_WIDE] {
        for (n, x) in part(inputs, name).iter().enumerate() {
            let c = canonicalise(x);
            assert!(
                !c.mirrored && c.phi == 0.0,
                "C moved a decode of {name}, state {n}"
            );
        }
    }
}

#[test]
fn cross_backend_inputs_noop_states_are_canonical() {
    check_noop(built());
}

negative_control!(
    cross_backend_inputs_noop_states_are_canonical,
    "a decode turned over",
    expected = "C moved a decode of",
    {
        let mut inputs = build_inputs();
        let at = 10 + 60 + 20_000 + 48 + 2000 + 12 + 30_000 + 36_000 + 19_520 + 54 + 32 + 4;
        inputs.canon[at].r = inputs.canon[at].r.map(|v| [v[1], v[0]]);
        check_noop(&inputs);
    }
);

// ── The decode's batches ─────────────────────────────────────────────────────────────────────────────────────────

/// Each chart's run in `run`: `η_E` off, then each of [`TARGETS`] at the same chart and points, then, on the variants
/// that allow it, every eighth point alone toward `V₀`.
fn check_runs(inputs: &Inputs) {
    let batches = with_points(inputs);
    let mut at = 0;
    let own = [
        "stage_decode_latent_defaults",
        "stage_decode_mass_simplex_defaults",
        "stage_decode_euclid_plane_defaults",
        "stage_decode_burrau_theta_delta_m_defaults",
        "stage_decode_shape_sphere_defaults",
    ];
    while at < batches.len() {
        let (_, b, uv) = &batches[at];
        at += 1;
        assert!(b.e_star.is_none(), "a run does not start with η_E off");
        if b.label == CROSSINGS || b.label == BATCH_393 {
            continue;
        }
        for target in TARGETS {
            let (_, t, tv) = &batches[at];
            assert!(
                t.e_star == Some(target.to_bits())
                    && (&t.entry, &t.label, &t.uniform) == (&b.entry, &b.label, &b.uniform)
                    && tv == uv,
                "a run's targets are not TARGETS at its chart and points"
            );
            at += 1;
        }
        let mut next = 0;
        while at < batches.len()
            && batches[at].1.e_star.is_some()
            && batches[at].1.uniform == b.uniform
        {
            let (_, e, ev) = &batches[at];
            let k = (next..uv.len())
                .find(|k| uv[*k] == ev[0] && k % 8 == 0)
                .filter(|_| ev.len() == 1 && e.points == 1);
            let k = k.expect("a run's own point is every eighth");
            assert!(
                own.contains(&b.entry.as_str()),
                "a variant that forbids η_E has an own V₀"
            );
            next = k + 1;
            at += 1;
        }
        assert!(
            !own.contains(&b.entry.as_str()) || next > 0,
            "a variant that allows η_E has no own V₀"
        );
    }
}

#[test]
fn cross_backend_inputs_batches_run_each_target() {
    check_runs(built());
}

negative_control!(
    cross_backend_inputs_batches_run_each_target,
    "a target changed",
    expected = "a run's targets are not TARGETS at its chart and points",
    {
        let mut inputs = build_inputs();
        inputs.batches[2].e_star = Some(1.25f32.to_bits());
        check_runs(&inputs);
    }
);

fn is_lattice(tail: &[[f32; 2]]) -> bool {
    let want = [0.0, 0.5, 1.0].map(|u| [0.0, 0.5, 1.0].map(|v| [u, v]));
    tail == want.as_flattened()
}

fn on_square(uv: &[[f32; 2]], lo: f32, hi: f32) -> bool {
    uv.iter()
        .all(|p| (lo..=hi).contains(&p[0]) && (0.0..=1.0).contains(&p[1]))
}

/// The points of a batch with `η_E` off, by what `decode_cross_backend.rs` draws for its label: `points()`' draws and
/// its nine lattice points; `inside()`; the edge columns; the points past the square; and the sphere's, off and on its
/// poles.
fn check_points(inputs: &Inputs) {
    let poles = [[0.5, 0.5], [0.0, 0.5], [1.0, 0.5]];
    let fractions = |n: usize| (0..=n).map(move |j| j as f32 / n as f32);
    for (_, b, uv) in with_points(inputs) {
        if b.e_star.is_some() {
            continue;
        }
        let invariant = b.entry.contains("invariant");
        let sphere = b.entry.contains("shape_sphere");
        let fine = match b.label.as_str() {
            CROSSINGS | BATCH_393 => true,
            POLE_BATCH => uv == poles,
            "edge columns" => {
                let column = |s: f32, half: &[[f32; 2]]| {
                    let (even, drawn) = half.split_at(33);
                    even.iter().zip(fractions(32)).all(|(p, t)| *p == [s, t])
                        && on_square(drawn, s, s)
                };
                let (a, c) = uv.split_at(49);
                uv.len() == 98 && column(0.0, a) && column(1.0, c)
            }
            "past the square" => {
                let (even, drawn) = uv.split_at(4 * 17);
                let rows = [-0.5, -0.25, 1.25, 1.5]
                    .into_iter()
                    .flat_map(|s| fractions(16).map(move |t| [s, t]));
                uv.len() == 100
                    && even.iter().copied().eq(rows)
                    && drawn.iter().enumerate().all(|(k, p)| {
                        let (lo, hi) = if k % 2 == 0 {
                            (-0.75, -0.01)
                        } else {
                            (1.01, 1.75)
                        };
                        (lo..=hi).contains(&p[0])
                    })
                    && on_square(drawn, -0.75, 1.75)
            }
            "fuzz" if invariant => uv.len() == 64 && on_square(&uv, 0.01, 0.99),
            "fuzz" if sphere => {
                uv.len() == 64 + 9 - 3
                    && on_square(&uv, 0.0, 1.0)
                    && !uv.iter().any(|p| poles.contains(p))
            }
            // The golden point and REQ-DEC-043's interior draw no lattice.
            "defaults" if uv.len() != 64 + 9 => on_square(&uv, 0.0, 1.0),
            _ => on_square(&uv, 0.0, 1.0) && is_lattice(&uv[uv.len() - 9..]),
        };
        assert!(fine, "a batch's points are not its label's: {}", b.label);
    }
}

#[test]
fn cross_backend_inputs_batches_hold_their_labels_points() {
    check_points(built());
}

negative_control!(
    cross_backend_inputs_batches_hold_their_labels_points,
    "a batch of past points back on the square",
    expected = "a batch's points are not its label's",
    {
        let mut inputs = build_inputs();
        let mut at = 0;
        let past = inputs
            .batches
            .iter()
            .find(|b| {
                at += b.points;
                b.label == "past the square"
            })
            .map(|b| b.points)
            .unwrap();
        inputs.points[at - past] = [0.25f32.to_bits(), 0.0f32.to_bits()];
        check_points(&inputs);
    }
);

/// #393's six batches: the exponential map about `(1, ±0, ±0)`, each sign of the pole's zeros at phase 0, and `+0` at
/// phases 0.3 and 2, frame `y`, `z`, extent `10⁻⁴`; at the pole `(½, ½)` first, then `(½ ± δ, ½)` and `(½, ½ ± δ)`, `2δ`
/// from 1 down to f32's step below ½.
fn check_393(inputs: &Inputs) {
    let z = [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]];
    let charts = z
        .map(|z| (z, 0.0))
        .into_iter()
        .chain([([0.0, 0.0], 0.3f32), ([0.0, 0.0], 2.0)]);
    let mut uv = vec![[0.5f32, 0.5]];
    let steps = (0..8)
        .map(|k| 0.5 * 10f32.powi(-k))
        .chain([0.5 - 0.5f32.next_down()]);
    for d in steps {
        for u in [0.5 + d, 0.5 - d] {
            uv.extend([[u, 0.5], [0.5, u]]);
        }
    }
    let all = with_points(inputs);
    let batches: Vec<_> = all.iter().filter(|b| b.1.label == BATCH_393).collect();
    assert_eq!(batches.len(), 6, "#393 is not six batches");
    for ((_, b, points), (z, phase)) in batches.into_iter().zip(charts) {
        let c: ShapeSphere<f32> = chart_of(b);
        let n0 = [1.0, z[0], z[1]].map(f32::to_bits);
        assert!(
            c.map == ShapeMap::Exponential
                && c.n0.map(f32::to_bits) == n0
                && c.e1 == [0.0, 1.0, 0.0]
                && c.e2 == [0.0, 0.0, 1.0]
                && c.scale == 1e-4
                && c.phase == phase,
            "a #393 chart is not its pole and phase"
        );
        assert!(*points == uv, "#393's points are not the pole's approach");
    }
}

#[test]
fn cross_backend_inputs_batches_of_393_approach_the_pole() {
    check_393(built());
}

negative_control!(
    cross_backend_inputs_batches_of_393_approach_the_pole,
    "the pole's point moved",
    expected = "#393's points are not the pole's approach",
    {
        let mut inputs = build_inputs();
        let n = inputs.points.len();
        inputs.points[n - 37] = [0.25f32.to_bits(), 0.5f32.to_bits()];
        check_393(&inputs);
    }
);

/// `M01_TINY` at `(z₀, z₁)` on `stick_breaking`'s masses.
fn tiny(z0: f32, z1: f32) -> bool {
    let m = <StickBreaking as Link<2, 3>>::forward([z0, z1]);
    m[0] + m[1] < kernel::decode::mass::m01_epsilon::<f32>()
}

/// TASK-M2-02's 40 000 points: on each of 20 000 lines `z₁ = 6.3 … 20.3`, the two adjacent floats of `z₀` either side of
/// the onset, on the chart whose plane is `(z₆, z₇)`, so that `(u, v) = (z₀, z₁)/2`.
fn check_crossings(inputs: &Inputs) {
    let all = with_points(inputs);
    let (_, b, uv) = all
        .iter()
        .find(|b| b.1.label == CROSSINGS)
        .expect("the crossings' batch");
    assert_eq!(b.entry, "stage_decode_latent_stick_breaking");
    let c: Latent<f32> = chart_of(b);
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0 } else { 0.0 });
    let mut z0 = [0.0; 8];
    (z0[6], z0[7]) = (1.0, 1.0);
    assert!(
        c.z0 == z0 && c.q1 == e(6) && c.q2 == e(7),
        "the crossings' chart is not the (z₆, z₇) plane"
    );
    assert_eq!(
        uv.len(),
        40_000,
        "the crossings are not 20 000 lines' pairs"
    );
    for (k, pair) in uv.chunks(2).enumerate() {
        let [[lo, z1], [hi, z2]] = [pair[0], pair[1]].map(|p| [p[0] * 2.0, p[1] * 2.0]);
        let line = (6.3 + 14.0 * k as f64 / 20_000.0) as f32;
        assert!(
            z1 == z2 && (z1 - line).abs() <= 2.0 * f32::EPSILON * line,
            "a crossing is not on its line, {k}"
        );
        assert!(
            lo.next_up() == hi && !tiny(lo, z1) && tiny(hi, z1),
            "a crossing's points are not either side of the onset, {k}"
        );
    }
}

#[test]
fn cross_backend_inputs_crossings_straddle_the_onset() {
    check_crossings(built());
}

negative_control!(
    cross_backend_inputs_crossings_straddle_the_onset,
    "a crossing's pair one float further apart",
    expected = "a crossing's points are not either side of the onset",
    {
        let mut inputs = build_inputs();
        let n = inputs.points.len();
        let at = n - 222 - 2 * 700;
        let lo = f32::from_bits(inputs.points[at][0]);
        inputs.points[at][0] = lo.next_down().to_bits();
        check_crossings(&inputs);
    }
);

// ── The fixture ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Which words of a state of `part`, `n`-th, no platform call reaches: all of them, or the masses and momenta alone.
fn exact_words(name: &str, n: usize) -> usize {
    const ALL: usize = 15;
    const MASSES_AND_MOMENTA: usize = 3;
    match name {
        TIE | TAU | SUBNORMAL | RANGE | SCALE | COINCIDENT | SIGNED_ZERO => ALL,
        // `+x`'s blocks take `θ = 0`, whose sine and cosine every platform gives alike.
        HIERARCHY if (n / 2000) % 3 != 1 => ALL,
        EDGE if (n / (40 * SWEEP)) % 2 == 0 => ALL,
        TURNED | SEAM | HIERARCHY | EDGE => MASSES_AND_MOMENTA,
        _ => 0,
    }
}

/// The fixture is what the builders give, bit for bit, in every word no platform call reaches, on every platform:
/// `+`, `−`, `×`, `/`, `√`, `powi`, `fma` and conversions round alike everywhere, so any difference is a builder's. The
/// decode's charts and points are drawn from the seeds alone, but for the crossings, which bisect on the platform's
/// `exp`, and each own `E*`, a decode's `V₀`.
fn check_fixture_exact(inputs: &Inputs, held: &Inputs) {
    let mut at = 0;
    for p in &held.parts {
        for n in 0..p.count {
            let (a, b) = (bits(&inputs.canon[at + n]), bits(&held.canon[at + n]));
            let k = exact_words(&p.name, n);
            let momenta = if k == 3 { 9..15 } else { 15..15 };
            assert!(
                a[..k] == b[..k] && a[momenta.clone()] == b[momenta],
                "a word no platform call reaches is not the fixture's: {} {n}",
                p.name
            );
        }
        at += p.count;
    }
    let targets = TARGETS.map(f32::to_bits);
    let pairs = with_points(inputs).into_iter().zip(with_points(held));
    for ((_, a, ua), (_, b, ub)) in pairs {
        let own = |e: Option<u32>| e.filter(|e| !targets.contains(e)).map(|_| 0);
        let e_star = |e: Option<u32>| own(e).or(e);
        let same = (&a.entry, &a.label, &a.uniform, a.points, e_star(a.e_star))
            == (&b.entry, &b.label, &b.uniform, b.points, e_star(b.e_star));
        let free = b.label != CROSSINGS;
        assert!(
            same && (!free || ua == ub),
            "a batch's charts or points are not the fixture's: {} {}",
            b.entry,
            b.label
        );
    }
}

#[test]
fn cross_backend_inputs_fixture_holds_every_word_no_platform_call_reaches() {
    let held = Inputs::load(root()).expect("the fixture");
    check_fixture_exact(built(), &held);
}

negative_control!(
    cross_backend_inputs_fixture_holds_every_word_no_platform_call_reaches,
    "a builder's chart moved by one bit",
    expected = "a batch's charts or points are not the fixture's",
    {
        let held = Inputs::load(root()).expect("the fixture");
        let mut inputs = build_inputs();
        inputs.batches[40].uniform[0] ^= 1;
        check_fixture_exact(&inputs, &held);
    }
);

fn is_near(a: f32, b: f32, scale: f64) -> bool {
    (f64::from(a) - f64::from(b)).abs() <= NEAR * scale
}

/// The fixture is what the builders give, to [`NEAR`], in the words a platform call reaches: the turned seam and
/// hierarchy positions, on their own scale, and a decode's masses, which its links alone give. A decode's positions and
/// momenta are not held: saturated, they are f32's conditioning, 10⁶ eps and more.
fn check_fixture_near(inputs: &Inputs, held: &Inputs) {
    let positions = |x: &Ic<f32>, y: &Ic<f32>, name: &str, scale: [f64; 3]| {
        for i in 0..3 {
            for j in 0..2 {
                assert!(
                    is_near(x.r[i][j], y.r[i][j], scale[i]),
                    "a position a platform call reaches is off the fixture's: {name}"
                );
            }
        }
    };
    for (x, y) in part(inputs, SEAM).iter().zip(part(held, SEAM)) {
        positions(x, y, SEAM, [1.0; 3]);
    }
    let pairs = part(inputs, HIERARCHY).iter().zip(part(held, HIERARCHY));
    for (n, (x, y)) in pairs.enumerate() {
        positions(x, y, HIERARCHY, [1.0, RATIOS[n / 6000], 1.0]);
    }
    for name in [NOOP, NOOP_WIDE] {
        for (x, y) in part(inputs, name).iter().zip(part(held, name)) {
            assert!(
                (0..3).all(|i| is_near(x.m[i], y.m[i], 1.0)),
                "a decode's masses are off the fixture's: {name}"
            );
        }
    }
    let pairs = with_points(inputs).into_iter().zip(with_points(held));
    for ((_, a, ua), (_, _, ub)) in pairs {
        if a.label == CROSSINGS {
            let close = |(p, q): (&[f32; 2], &[f32; 2])| (0..2).all(|j| is_near(p[j], q[j], 1.0));
            assert!(
                ua.iter().zip(&ub).all(close),
                "a crossing is off the fixture's"
            );
        }
    }
}

#[test]
fn cross_backend_inputs_fixture_holds_what_a_platform_call_reaches_to_near() {
    let held = Inputs::load(root()).expect("the fixture");
    check_fixture_near(built(), &held);
}

negative_control!(
    cross_backend_inputs_fixture_holds_what_a_platform_call_reaches_to_near,
    "a decode's masses swapped",
    expected = "a decode's masses are off the fixture's",
    {
        let held = Inputs::load(root()).expect("the fixture");
        let mut inputs = build_inputs();
        let at = inputs.canon.len() - 1;
        inputs.canon[at].m.swap(0, 2);
        check_fixture_near(&inputs, &held);
    }
);
