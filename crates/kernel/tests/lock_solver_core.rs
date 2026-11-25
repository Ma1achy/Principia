//! The lock's solver core (design `principia_chart_composition.md` § 17.8 and § 10.6): a least-squares solve
//! over a map with a recorded branch rule, convergence reported per call, failures tagged, and the measurement along a
//! 64-frame morph.
//!
//! The plan is unclear here; chose to run these as `cargo test -p kernel lock_solver_core` because the solver is a pure
//! kernel function, so its tests need no engine.
//!
//! The plan is unclear here; chose two test-side morphs because the blend chart is not built: (1) a
//! constructed one, `F_s(u, v) = (u − t(s), v, u² − 1)`, whose target `0` has two local minimisers, one in each well,
//! the nearer changing side at `t = 0` (a cut locus) and the left one ending at the fold `t = 2/(3√6)`; and (2) a state
//! blend of two latent charts' decoded states, `(1 − s)·D(Φ_A) + s·D(Φ_B)` over the fifteen `(m, r, p)` words, the
//! anchor `D(Φ_A(0.37, 0.61))`, so the anchor leaves the image as `s` grows. The solver is the same whatever the map.
//! Morph (2) has no exact slopes yet: its slopes are central differences, the stand-in, and its weight
//! is 1 per word (the metric is placement's to define).
//!
//! - `lock_solver_core_solves`: on the image the solve converges `OnImage` at the point; off it, `Stationary` at the
//!   foot, its residual the distance; f64 and f32. Control: the slopes' sign flipped (a wrong Jacobian).
//! - `lock_solver_core_failures_tagged`: each failure its tag, the report present. Control: a filter that drops the
//!   failed reports.
//! - `lock_solver_core_branch_rule`: the restart keeps the converged seed of least residual, the lowest index on a tie,
//!   recorded; the same call twice is bitwise the same. Control: a rule keeping the highest index on a tie.
//! - `lock_solver_core_follow_ends`: past the fold the continued solve stops, tagged `Ended`, at the last frame where
//!   its minimiser exists. Control: the continuity test switched off runs past the end.
//! - `lock_solver_core_follow_smooth`: minimisers that move smoothly but unevenly (from rest as `s²`, `s^2.5` and `s³`,
//!   one turning back inside a frame, and 28 hairpins turning inside frame 33) are followed through every frame,
//!   never tagged `Ended`. Control: the test at depth 2, which tags the `s^2.5` path's first frame.
//! - `lock_solver_core_failures_labelled`: the failures' cases again, its control on the tags: a stall expected as the
//!   iteration cap.
//! - `lock_solver_core_terminates_ceiling_max` and `lock_solver_core_terminates_steep_f32`: the code review's overflow
//!   route, `ceiling·scale` overflowing to ∞ (a ceiling of `f64::MAX`; a map of slope 10¹⁴ at f32), returns. Controls:
//!   the solver with its exit on a non-finite μ dropped (`Spec::unguarded`, under `controls`), caught by the timeout.
//! - `lock_solver_core_terminates`: every call returns, whatever the damping: an initial damping of 0 is refused,
//!   tagged; a `decrease` that underflows `μ` to 0 at f64 and f32 still returns. Control: the unguarded update
//!   `μ ← μ·increase` from `μ = 0`, caught by the same ten-second timeout.
//! - `lock_solver_core_measurement`: the measurement (below). Control: the restart chain in place of the continued one.
//!
//! **The measurement** (criteria written before the first run). Along a 64-frame morph, `s_k = k/64`: the share of
//! frames whose four-seed restart converges to another minimiser than the continued solve (the two converged points
//! more than `AGREE` apart), reported, not asserted, but for morph (1), built with two basins, where a share of 0 means
//! the measurement cannot fire. Continuity (G8's criterion, design § 17.8's continuous slide): the continued solve's largest step between
//! frames, in chart coordinates, over 64, 128 and 256 frames, `L₁₂₈ ≤ 0.6·L₆₄` and `L₂₅₆ ≤ 0.6·L₁₂₈`. Every continued
//! frame must converge (no failure, no `Ended`) on both morphs.

mod decode_support;

use decode_support::P;
use kernel::chart::lock_solve::{
    distance, follow, restart, solve, Branch, Continuity, Criterion, Damping, Failure, Family, Map,
    Report, Spec, Status, Target,
};
use kernel::chart::{self, Latent};
use kernel::generated::links::Defaults;
use validation::negative_control;

// ── Settings, the test's (written before the first run) ───────────────────────────────────────────────────────────────

/// The f64 solve's settings.
fn spec64() -> Spec<f64> {
    Spec {
        max_iterations: 200,
        residual_tolerance: 1e-13,
        optimality_tolerance: 1e-10,
        conditioning_floor: 1e-12,
        damping: Damping {
            initial: 1e-3,
            increase: 10.0,
            decrease: 0.1,
            ceiling: 1e12,
        },
        #[cfg(feature = "controls")]
        unguarded: false,
        #[cfg(feature = "controls")]
        polish_unscaled: false,
    }
}

/// The f32 solve's settings.
fn spec32() -> Spec<f32> {
    Spec {
        max_iterations: 200,
        residual_tolerance: 1e-5,
        optimality_tolerance: 1e-3,
        conditioning_floor: 1e-6,
        damping: Damping {
            initial: 1e-3,
            increase: 10.0,
            decrease: 0.1,
            ceiling: 1e12,
        },
        #[cfg(feature = "controls")]
        unguarded: false,
        #[cfg(feature = "controls")]
        polish_unscaled: false,
    }
}

/// The settings at `R`.
fn spec<R: P>() -> Spec<R> {
    if R::eps() < 1e-10 {
        let s = spec64();
        Spec {
            max_iterations: s.max_iterations,
            residual_tolerance: R::of(s.residual_tolerance),
            optimality_tolerance: R::of(s.optimality_tolerance),
            conditioning_floor: R::of(s.conditioning_floor),
            damping: Damping {
                initial: R::of(s.damping.initial),
                increase: R::of(s.damping.increase),
                decrease: R::of(s.damping.decrease),
                ceiling: R::of(s.damping.ceiling),
            },
            #[cfg(feature = "controls")]
            unguarded: false,
            #[cfg(feature = "controls")]
            polish_unscaled: false,
        }
    } else {
        let s = spec32();
        Spec {
            max_iterations: s.max_iterations,
            residual_tolerance: R::of(s.residual_tolerance as f64),
            optimality_tolerance: R::of(s.optimality_tolerance as f64),
            conditioning_floor: R::of(s.conditioning_floor as f64),
            damping: Damping {
                initial: R::of(s.damping.initial as f64),
                increase: R::of(s.damping.increase as f64),
                decrease: R::of(s.damping.decrease as f64),
                ceiling: R::of(s.damping.ceiling as f64),
            },
            #[cfg(feature = "controls")]
            unguarded: false,
            #[cfg(feature = "controls")]
            polish_unscaled: false,
        }
    }
}

/// How close two converged points must be to be one minimiser, in chart coordinates: f64.
const AGREE: f64 = 1e-6;

/// The continuity test: G8's ratio, a rounding-size floor, `AGREE`.
fn continuity() -> Continuity<f64> {
    Continuity {
        ratio: 0.6,
        floor: 1e-9,
        agree: AGREE,
        depth: 32,
    }
}

/// G8's ratio for the whole path.
const G8_RATIO: f64 = 0.6;

// ── Maps ──────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A map given by its value and exact slopes; `flip` negates the slopes (the controls' wrong Jacobian).
struct Exact<V, J> {
    value: V,
    jacobian: J,
    flip: bool,
}

impl<
        R: P,
        const N: usize,
        V: Fn([R; 2]) -> Option<[R; N]>,
        J: Fn([R; 2]) -> Option<[[R; N]; 2]>,
    > Map<R, N> for Exact<V, J>
{
    fn value(&self, x: [R; 2]) -> Option<[R; N]> {
        (self.value)(x)
    }
    fn jacobian(&self, x: [R; 2]) -> Option<[[R; N]; 2]> {
        let j = (self.jacobian)(x)?;
        Some(if self.flip {
            j.map(|c| c.map(|v| -v))
        } else {
            j
        })
    }
}

/// The paraboloid `F(u, v) = (u, v, u² + v²)`: every target `(a, b, a² + b²)` is on its image.
fn paraboloid<R: P>(flip: bool) -> impl Map<R, 3> {
    Exact {
        value: |x: [R; 2]| Some([x[0], x[1], x[0] * x[0] + x[1] * x[1]]),
        jacobian: |x: [R; 2]| {
            let two = R::of(2.0);
            Some([
                [R::one(), R::zero(), two * x[0]],
                [R::zero(), R::one(), two * x[1]],
            ])
        },
        flip,
    }
}

/// The plane `F(u, v) = (u, v, 0)`: the target `(a, b, h)` is off its image, its foot `(a, b)`, distance `|h|`.
fn plane<R: P>(flip: bool) -> impl Map<R, 3> {
    Exact {
        value: |x: [R; 2]| Some([x[0], x[1], R::zero()]),
        jacobian: |_x: [R; 2]| {
            Some([
                [R::one(), R::zero(), R::zero()],
                [R::zero(), R::one(), R::zero()],
            ])
        },
        flip,
    }
}

/// Morph (1), the double well: `F_s(u, v) = (u − t(s), v, u² − 1)`, `t(s) = t₀ + (t₁ − t₀)s`, target 0.
struct Wells<R> {
    t0: R,
    t1: R,
}

impl<R: P> Wells<R> {
    fn t(&self, s: R) -> R {
        self.t0 + (self.t1 - self.t0) * s
    }
}

impl<R: P> Family<R, 3> for Wells<R> {
    fn value(&self, s: R, x: [R; 2]) -> Option<[R; 3]> {
        Some([x[0] - self.t(s), x[1], x[0] * x[0] - R::one()])
    }
    fn jacobian(&self, _s: R, x: [R; 2]) -> Option<[[R; 3]; 2]> {
        Some([
            [R::one(), R::zero(), R::of(2.0) * x[0]],
            [R::zero(), R::one(), R::zero()],
        ])
    }
}

/// The double well's left minimiser exists while `t < 2/(3√6)`, the fold.
fn fold() -> f64 {
    2.0 / (3.0 * 6f64.sqrt())
}

/// The fifteen `(m, r, p)` words of a decoded state.
fn words(ic: &kernel::decode::Ic<f64>) -> [f64; 15] {
    let mut w = [0.0; 15];
    w[..3].copy_from_slice(&ic.m);
    for b in 0..3 {
        w[3 + 2 * b] = ic.r[b][0];
        w[4 + 2 * b] = ic.r[b][1];
        w[9 + 2 * b] = ic.p[b][0];
        w[10 + 2 * b] = ic.p[b][1];
    }
    w
}

/// Morph (2), the state blend of two latent charts' decodes, slopes by central differences at step `H`.
struct Blend {
    a: Latent<f64>,
    b: Latent<f64>,
}

/// Morph (2)'s central-difference step.
const H: f64 = 1e-5;

impl Blend {
    fn fixture() -> Self {
        // Chart A: the shape plane (z_α, z_β) about z₀. Chart B: the same z₀ and horizontal, its vertical z_q0, so the
        // anchor's β is not on B's image.
        let z0 = [0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.1, -0.1];
        let e = |k: usize| {
            let mut v = [0.0; 8];
            v[k] = 1.5;
            v
        };
        Blend {
            a: Latent {
                z0,
                q1: e(0),
                q2: e(1),
            },
            b: Latent {
                z0,
                q1: e(0),
                q2: e(2),
            },
        }
    }
    fn state(c: &Latent<f64>, x: [f64; 2]) -> [f64; 15] {
        words(&chart::decode::<Defaults, f64, _>(c, x[0], x[1]).ic)
    }
    fn anchor(&self) -> [f64; 15] {
        Self::state(&self.a, [0.37, 0.61])
    }
}

impl Family<f64, 15> for Blend {
    fn value(&self, s: f64, x: [f64; 2]) -> Option<[f64; 15]> {
        let (fa, fb) = (Self::state(&self.a, x), Self::state(&self.b, x));
        let mut f = [0.0; 15];
        for i in 0..15 {
            f[i] = (1.0 - s) * fa[i] + s * fb[i];
        }
        f.iter().all(|v| v.is_finite()).then_some(f)
    }
    fn jacobian(&self, s: f64, x: [f64; 2]) -> Option<[[f64; 15]; 2]> {
        kernel::chart::lock_solve::central_difference(|p| self.value(s, p), x, H)
    }
}

// ── Checks ────────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The report converged by `criterion` within `tol` of `want`, with residual `residual` within `tol`.
fn check_converged<R: P>(
    r: &Report<R>,
    criterion: Criterion,
    want: [f64; 2],
    residual: f64,
    tol: f64,
) {
    assert_eq!(
        r.status,
        Status::Converged(criterion),
        "did not converge {criterion:?}: {r:?}"
    );
    let off = distance([r.x[0].f(), r.x[1].f()], want);
    assert!(off <= tol, "converged {off:e} from {want:?}: {r:?}");
    assert!(
        (r.residual.f() - residual).abs() <= tol,
        "residual {} is not {residual}: {r:?}",
        r.residual.f()
    );
}

/// On the image and off it, at `R`, with the slopes flipped or not.
fn solves<R: P>(flip: bool) {
    let tol = if R::eps() < 1e-10 { 1e-9 } else { 1e-3 };
    let (a, b) = (0.3, -0.4);
    let on = Target {
        value: [R::of(a), R::of(b), R::of(a * a + b * b)],
        weight: [R::one(); 3],
    };
    let r = solve(
        &paraboloid::<R>(flip),
        &on,
        [R::of(1.0), R::of(0.7)],
        &spec(),
    );
    check_converged(&r, Criterion::OnImage, [a, b], 0.0, tol);
    assert!(r.iterations > 0 && r.evaluations > r.iterations, "{r:?}");
    let off = Target {
        value: [R::of(a), R::of(b), R::of(0.25)],
        weight: [R::one(); 3],
    };
    let r = solve(&plane::<R>(flip), &off, [R::of(-1.0), R::of(2.0)], &spec());
    check_converged(&r, Criterion::Stationary, [a, b], 0.25, tol);
    assert!(
        r.optimality.f() <= spec::<R>().optimality_tolerance.f(),
        "{r:?}"
    );
    println!(
        "solves at {}: off-image residual {:e}, optimality {:e}, iterations {}",
        R::NAME,
        r.residual.f(),
        r.optimality.f(),
        r.iterations
    );
}

/// `follow` and `restart` at `R` along morph (1), 16 frames: every continued frame converges on the left well, and
/// every restart converges, the right well past `t = 0`.
fn follows<R: P>() {
    let (floor, agree) = if R::eps() < 1e-10 {
        (1e-9, AGREE)
    } else {
        (1e-4, 1e-2)
    };
    let c = Continuity {
        ratio: R::of(0.6),
        floor: R::of(floor),
        agree: R::of(agree),
        depth: 32,
    };
    let t = Target {
        value: [R::zero(); 3],
        weight: [R::one(); 3],
    };
    let wells = Wells {
        t0: R::of(-0.2),
        t1: R::of(0.2),
    };
    let seeds = WELL_SEEDS.map(|p| p.map(R::of));
    let start = chart::lock_solve::Frame {
        family: &wells,
        s: R::zero(),
    };
    let mut x = solve(&start, &t, [R::of(-1.0), R::zero()], &spec()).x;
    let frames = 16;
    for k in 1..=frames {
        let (s0, s1) = (
            R::of((k - 1) as f64 / frames as f64),
            R::of(k as f64 / frames as f64),
        );
        let r = follow(&wells, &t, s0, x, s1, &spec(), &c);
        assert!(
            r.converged() && r.x[0] < R::zero(),
            "{} frame {k} not followed: {r:?}",
            R::NAME
        );
        let fresh = restart(
            &chart::lock_solve::Frame {
                family: &wells,
                s: s1,
            },
            &t,
            &seeds,
            &spec(),
        );
        assert!(
            fresh.converged(),
            "{} frame {k}'s restart did not converge: {fresh:?}",
            R::NAME
        );
        x = r.x;
    }
}

#[test]
fn lock_solver_core_solves() {
    solves::<f64>(false);
    solves::<f32>(false);
    follows::<f64>();
    follows::<f32>();
}

negative_control!(
    lock_solver_core_solves,
    "the slopes' sign flipped (a wrong Jacobian) must not converge at the point",
    expected = "did not converge",
    solves::<f64>(true)
);

/// Each failure's tag, from cases built to fail; `drop` filters out the failed reports (the control).
/// How the failure check is broken, for its controls: not at all, the failed reports dropped, or one case's expected
/// tag swapped with another's.
#[derive(Clone, Copy, PartialEq)]
enum Fault {
    None,
    Drop,
    Mislabel,
}

fn failures_tagged(fault: Fault) {
    let s = spec::<f64>();
    let w = [1.0; 3];
    let mut cases: Vec<(&str, Failure, Report<f64>)> = Vec::new();
    // The map not finite at the seed.
    let holed = Exact {
        value: |x: [f64; 2]| (x[0] < 1.0).then_some([x[0], x[1], 0.0]),
        jacobian: |_x: [f64; 2]| Some([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
        flip: false,
    };
    let t = Target {
        value: [0.0; 3],
        weight: w,
    };
    // Settings that cannot terminate: no initial damping (a rejection would leave μ at 0).
    let no_damping = Spec {
        damping: Damping {
            initial: 0.0,
            ..s.damping
        },
        ..s
    };
    cases.push((
        "settings",
        Failure::Settings,
        solve(&plane::<f64>(false), &t, [1.0, 1.0], &no_damping),
    ));
    cases.push((
        "non-finite seed",
        Failure::NonFinite,
        solve(&holed, &t, [2.0, 0.0], &s),
    ));
    // No accepted step allowed.
    let capped = Spec {
        max_iterations: 0,
        ..s
    };
    cases.push((
        "iteration cap",
        Failure::IterationCap,
        solve(&plane::<f64>(false), &t, [1.0, 1.0], &capped),
    ));
    // Slopes of the wrong sign: no damping lowers the cost.
    cases.push((
        "stall",
        if fault == Fault::Mislabel {
            Failure::IterationCap
        } else {
            Failure::Stalled
        },
        solve(&plane::<f64>(true), &t, [1.0, 1.0], &s),
    ));
    // A rank-one map: F(u, v) = (u + v, u + v, 0), its minimisers a line.
    let line = Exact {
        value: |x: [f64; 2]| Some([x[0] + x[1], x[0] + x[1], 0.0]),
        jacobian: |_x: [f64; 2]| Some([[1.0, 1.0, 0.0], [1.0, 1.0, 0.0]]),
        flip: false,
    };
    cases.push((
        "ill-conditioned",
        Failure::IllConditioned,
        solve(&line, &t, [1.0, 0.5], &s),
    ));
    // A frame across the fold: the followed minimiser ends.
    let wells = Wells { t0: 0.0, t1: 0.6 };
    let seed = solve(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &t,
        [-1.0, 0.0],
        &s,
    );
    assert!(seed.converged(), "{seed:?}");
    cases.push((
        "ended",
        Failure::Ended,
        follow(&wells, &t, 0.0, seed.x, 1.0, &s, &continuity()),
    ));
    let kept: Vec<_> = cases
        .into_iter()
        .filter(|(_, _, r)| fault != Fault::Drop || r.converged())
        .collect();
    assert_eq!(
        kept.len(),
        6,
        "a failed report was dropped: {} of 6 kept",
        kept.len()
    );
    for (what, tag, r) in &kept {
        assert_eq!(
            r.status,
            Status::Failed(*tag),
            "{what} is not tagged {tag:?}: {r:?}"
        );
        assert!(
            r.x.iter().all(|v| v.is_finite()),
            "{what}'s report holds no point: {r:?}"
        );
        if *tag == Failure::Ended {
            // At the frame's start, (−1/√2, 0) at t = 0: JᵀJ = diag(1 + 4u², 1) = diag(3, 1), conditioning 1/3.
            assert!(
                (r.conditioning - 1.0 / 3.0).abs() < 1e-9 && r.travel == 0.0,
                "the ended report's numbers are not its point's: {r:?}"
            );
        }
        println!(
            "{what}: {:?}, x {:?}, iterations {}, conditioning {:e}",
            r.status, r.x, r.iterations, r.conditioning
        );
    }
}

#[test]
fn lock_solver_core_failures_tagged() {
    failures_tagged(Fault::None);
}

negative_control!(
    lock_solver_core_failures_tagged,
    "failed reports dropped must fail the count",
    expected = "a failed report was dropped",
    failures_tagged(Fault::Drop)
);

/// The same cases as `lock_solver_core_failures_tagged`, its control on the tags themselves.
#[test]
fn lock_solver_core_failures_labelled() {
    failures_tagged(Fault::None);
}

negative_control!(
    lock_solver_core_failures_labelled,
    "a stall expected as the iteration cap must fail the tag check",
    expected = "stall is not tagged IterationCap",
    failures_tagged(Fault::Mislabel)
);

/// Rosenbrock's valley, `F(u, v) = (10(v − u²), 1 − u)`, target 0, at `R`.
fn rosenbrock<R: P>() -> impl Map<R, 2> {
    Exact {
        value: |x: [R; 2]| Some([R::of(10.0) * (x[1] - x[0] * x[0]), R::one() - x[0]]),
        jacobian: |x: [R; 2]| Some([[R::of(-20.0) * x[0], -R::one()], [R::of(10.0), R::zero()]]),
        flip: false,
    }
}

/// Runs `call` on a thread and returns its result, or panics if it has not returned within ten seconds.
fn within_ten_seconds<T: Send + 'static>(
    what: &str,
    call: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(call());
    });
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("{what} did not return within 10 s"))
}

/// A change to the damping.
type Change = fn(Damping<f64>) -> Damping<f64>;

/// Every call returns a report, whatever the damping: settings that cannot terminate are refused; a `decrease` that
/// underflows `μ` to 0 at f64 and at f32 still returns, by the floor. Rosenbrock from `(−1.2, 1)`, whose first full
/// Gauss–Newton steps are rejected.
fn terminates() {
    let seed = [-1.2, 1.0];
    let zero = Target {
        value: [0.0; 2],
        weight: [1.0; 2],
    };
    let refused = within_ten_seconds("initial damping 0", move || {
        let s = Spec {
            damping: Damping {
                initial: 0.0,
                ..spec64().damping
            },
            ..spec64()
        };
        solve(&rosenbrock::<f64>(), &zero, seed, &s)
    });
    assert_eq!(
        refused.status,
        Status::Failed(Failure::Settings),
        "{refused:?}"
    );
    // The code review's other settings that could not terminate, on the stall fixture (every trial rejected).
    let bad: [(&str, Change); 3] = [
        ("increase 1", |d| Damping { increase: 1.0, ..d }),
        ("ceiling infinite", |d| Damping {
            ceiling: f64::INFINITY,
            ..d
        }),
        ("initial NaN", |d| Damping {
            initial: f64::NAN,
            ..d
        }),
    ];
    for (what, change) in bad {
        let r = within_ten_seconds(what, move || {
            let s = Spec {
                damping: change(spec64().damping),
                ..spec64()
            };
            let zero3 = Target {
                value: [0.0; 3],
                weight: [1.0; 3],
            };
            solve(&plane::<f64>(true), &zero3, [1.0, 1.0], &s)
        });
        assert_eq!(r.status, Status::Failed(Failure::Settings), "{what}: {r:?}");
    }
    let f64_underflow = within_ten_seconds("f64 with μ underflowing", move || {
        let s = Spec {
            damping: Damping {
                decrease: 1e-300,
                ..spec64().damping
            },
            ..spec64()
        };
        solve(&rosenbrock::<f64>(), &zero, seed, &s)
    });
    let f32_underflow = within_ten_seconds("f32 with μ underflowing", move || {
        let zero = Target {
            value: [0.0f32; 2],
            weight: [1.0; 2],
        };
        let s = Spec {
            damping: Damping {
                decrease: 1e-30,
                ..spec32().damping
            },
            ..spec32()
        };
        solve(&rosenbrock::<f32>(), &zero, [-1.2, 1.0], &s)
    });
    println!("terminates: initial 0 {:?}; f64 underflow {:?} in {} iterations at {:?}; f32 underflow {:?} in {} iterations at {:?}",
        refused.status, f64_underflow.status, f64_underflow.iterations, f64_underflow.x,
        f32_underflow.status, f32_underflow.iterations, f32_underflow.x);
}

#[test]
fn lock_solver_core_terminates() {
    terminates();
}

negative_control!(
    lock_solver_core_terminates,
    "the unguarded update, μ ← μ·increase from μ = 0, never passes the ceiling, and the timeout catches it",
    expected = "did not return within 10 s",
    {
        within_ten_seconds("the unguarded update", || {
            let d = spec64().damping;
            let mut mu = 0.0 * d.initial;
            while mu <= d.ceiling {
                mu *= d.increase;
            }
            mu
        });
    }
);

#[cfg_attr(not(feature = "controls"), allow(dead_code))]
/// A restart that keeps the highest index on a tie (`<=`): the control's broken rule.
fn restart_last_on_tie<M: Map<f64, 3>>(
    map: &M,
    t: &Target<f64, 3>,
    seeds: &[[f64; 2]; 4],
) -> Report<f64> {
    let mut best: Option<(u32, Report<f64>)> = None;
    for (k, seed) in seeds.iter().enumerate() {
        let r = solve(map, t, *seed, &spec());
        if r.converged() && best.as_ref().is_none_or(|(_, b)| r.residual <= b.residual) {
            best = Some((k as u32, r));
        }
    }
    let (index, mut r) = best.expect("a seed converges");
    r.branch = Branch::Restart {
        seed: seeds[index as usize],
        index,
        count: 4,
    };
    r
}

/// The four seeds of the double well's window `[−1.5, 1.5]²`, its quarter points.
const WELL_SEEDS: [[f64; 2]; 4] = [[-0.75, -0.75], [0.75, -0.75], [-0.75, 0.75], [0.75, 0.75]];

/// The restart's rule on the double well: at `t = 0.1` the right well is nearer; seeds 1 and 3 (mirror images in `v`)
/// both reach it, a tie, and the lower index, 1, is kept. At `t = 0` the wells tie too (mirror images in `u`) and the
/// lowest index, seed 0's left well, is kept.
fn branch_rule(rule: impl Fn(&Frame64, &Target<f64, 3>, &[[f64; 2]; 4]) -> Report<f64>) {
    let t = Target {
        value: [0.0; 3],
        weight: [1.0; 3],
    };
    let wells = Wells { t0: 0.0, t1: 0.1 };
    let right = rule(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 1.0,
        },
        &t,
        &WELL_SEEDS,
    );
    assert!(
        right.converged() && right.x[0] > 0.0,
        "the nearer, right, well not kept: {right:?}"
    );
    assert!(
        matches!(
            right.branch,
            Branch::Restart {
                index: 1,
                count: 4,
                ..
            }
        ),
        "the branch is not recorded as seed 1 of 4: {:?}",
        right.branch
    );
    let tie = rule(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &t,
        &WELL_SEEDS,
    );
    let mirror = solve(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &t,
        WELL_SEEDS[1],
        &spec(),
    );
    assert_eq!(
        tie.residual.to_bits(),
        mirror.residual.to_bits(),
        "the fixture's wells do not tie bitwise"
    );
    assert!(
        matches!(tie.branch, Branch::Restart { index: 0, .. }) && tie.x[0] < 0.0,
        "on a tie the lowest index is not kept: {:?} at {:?}",
        tie.branch,
        tie.x
    );
    let again = rule(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &t,
        &WELL_SEEDS,
    );
    assert_eq!(
        format!("{tie:?}"),
        format!("{again:?}"),
        "the restart is not a pure function"
    );
    assert_eq!(
        tie.x.map(f64::to_bits),
        again.x.map(f64::to_bits),
        "the restart is not bitwise repeatable"
    );
}

type Frame64<'a> = chart::lock_solve::Frame<'a, f64, Wells<f64>>;

#[test]
fn lock_solver_core_branch_rule() {
    branch_rule(|m, t, seeds| restart(m, t, seeds, &spec()));
}

negative_control!(
    lock_solver_core_branch_rule,
    "a rule keeping the highest index on a tie must fail",
    expected = "the branch is not recorded as seed 1 of 4",
    branch_rule(|m, t, seeds| restart_last_on_tie(m, t, seeds))
);

/// The double well from `t = −0.2` to `0.5`, 64 frames: the continued solve stops, `Ended`, at the last frame whose
/// `t` is below the fold, within two frames of it; `c` is the continuity test (the control's switched off).
fn follow_ends(c: Continuity<f64>) {
    let t = Target {
        value: [0.0; 3],
        weight: [1.0; 3],
    };
    let wells = Wells { t0: -0.2, t1: 0.5 };
    let frames = 64;
    let mut x = solve(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &t,
        [-1.0, 0.0],
        &spec(),
    )
    .x;
    let mut stop = None;
    for k in 1..=frames {
        let (s0, s1) = ((k - 1) as f64 / frames as f64, k as f64 / frames as f64);
        let r = follow(&wells, &t, s0, x, s1, &spec(), &c);
        match r.status {
            Status::Converged(_) => x = r.x,
            Status::Failed(Failure::Ended) => {
                assert_eq!(
                    r.x, x,
                    "an ended frame does not keep the last frame's point"
                );
                stop = Some(s0);
                break;
            }
            other => panic!("frame {k} failed otherwise: {other:?}"),
        }
    }
    let stop = stop.unwrap_or_else(|| {
        panic!("the continued solve ran past the end of its minimiser, at x = {x:?}")
    });
    let dt = (wells.t1 - wells.t0) / frames as f64;
    let t_stop = wells.t(stop);
    println!("follow ends: stopped at s = {stop}, t = {t_stop:.6}, the fold at t = {:.6}, frame dt = {dt:.6}", fold());
    assert!(t_stop < fold(), "stopped past the fold: t = {t_stop}");
    assert!(
        t_stop >= fold() - 2.0 * dt,
        "stopped more than two frames before the fold: t = {t_stop}"
    );
    assert!(
        x[0] < 0.0,
        "the stop is not on the followed, left, minimiser: {x:?}"
    );
}

#[test]
fn lock_solver_core_follow_ends() {
    follow_ends(continuity());
}

negative_control!(
    lock_solver_core_follow_ends,
    "the continuity test switched off must run past the end",
    expected = "ran past the end",
    follow_ends(Continuity {
        ratio: f64::MAX,
        ..continuity()
    })
);

/// Morph (1) eased from rest: `t(s) = t₀ + (t₁ − t₀)s²`.
struct EasedWells(Wells<f64>);

impl Family<f64, 3> for EasedWells {
    fn value(&self, s: f64, x: [f64; 2]) -> Option<[f64; 3]> {
        self.0.value(s * s, x)
    }
    fn jacobian(&self, s: f64, x: [f64; 2]) -> Option<[[f64; 3]; 2]> {
        self.0.jacobian(s * s, x)
    }
}

/// A minimiser's path `g`, `u = g(s)`.
type Path = fn(f64) -> f64;

/// A minimiser moving along `u = g(s)`: `F_s(u, v) = (u − g(s), v)`, target 0.
struct Along(Path);

impl Family<f64, 2> for Along {
    fn value(&self, s: f64, x: [f64; 2]) -> Option<[f64; 2]> {
        Some([x[0] - (self.0)(s), x[1]])
    }
    fn jacobian(&self, _s: f64, _x: [f64; 2]) -> Option<[[f64; 2]; 2]> {
        Some([[1.0, 0.0], [0.0, 1.0]])
    }
}

/// The code review's hairpin: the minimiser `p(s) = (v₀·s, (s − c)²)` of `F_s(x) = x − p(s)`, target 0, turning at
/// `s = c`; its speed never falls below `v₀`.
struct Hairpin {
    v0: f64,
    c: f64,
}

impl Family<f64, 2> for Hairpin {
    fn value(&self, s: f64, x: [f64; 2]) -> Option<[f64; 2]> {
        Some([x[0] - self.v0 * s, x[1] - (s - self.c) * (s - self.c)])
    }
    fn jacobian(&self, _s: f64, _x: [f64; 2]) -> Option<[[f64; 2]; 2]> {
        Some([[1.0, 0.0], [0.0, 1.0]])
    }
}

/// Follows `family` over 64 frames from `x`: every frame converges, none `Ended`; returns the most iterations a frame.
fn follow_all<const N: usize, Fam: Family<f64, N>>(
    name: &str,
    family: &Fam,
    t: &Target<f64, N>,
    mut x: [f64; 2],
    c: &Continuity<f64>,
) -> u32 {
    let frames = 64;
    let mut most = 0;
    for k in 1..=frames {
        let (s0, s1) = ((k - 1) as f64 / frames as f64, k as f64 / frames as f64);
        let r = follow(family, t, s0, x, s1, &spec(), c);
        assert!(
            r.converged(),
            "{name}: a smooth path's frame {k} is tagged: {:?} at {:?}",
            r.status,
            r.x
        );
        most = most.max(r.iterations);
        x = r.x;
    }
    most
}

/// Smooth minimisers that move unevenly inside a frame, followed by `c` over 64 frames, none tagged `Ended`: the eased
/// double well (`t ∝ s²` from rest), paths `s^2.5` and `s³` from rest, one that turns back at 0.4 of frame 1,
/// `0.05(64s − 0.4)²` (the physics review's), and the code review's 28 hairpins.
fn follow_smooth(c: Continuity<f64>) {
    let t = Target {
        value: [0.0; 3],
        weight: [1.0; 3],
    };
    let eased = EasedWells(Wells { t0: -0.2, t1: 0.2 });
    let start = solve(
        &chart::lock_solve::Frame {
            family: &eased,
            s: 0.0,
        },
        &t,
        [-1.0, 0.0],
        &spec(),
    )
    .x;
    let most = follow_all("the eased wells", &eased, &t, start, &c);
    println!("follow smooth: the eased wells, 64 of 64 frames followed, at most {most} iterations a frame");
    let zero = Target {
        value: [0.0; 2],
        weight: [1.0; 2],
    };
    let paths: [(&str, Path); 3] = [
        ("s^2.5", |s| s.powf(2.5)),
        ("s^3", |s| s * s * s),
        ("turning in frame 1", |s| {
            0.05 * (64.0 * s - 0.4) * (64.0 * s - 0.4)
        }),
    ];
    for (name, g) in paths {
        let most = follow_all(name, &Along(g), &zero, [g(0.0), 0.0], &c);
        println!(
            "follow smooth: {name}, 64 of 64 frames followed, at most {most} iterations a frame"
        );
    }
    // The code review's sweep: a hairpin turning inside frame 33, at each of seven places in it, at four speeds.
    let mut most = 0;
    for v0 in [0.0, 1e-3, 3e-3, 1e-2] {
        for place in [0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9] {
            let hairpin = Hairpin {
                v0,
                c: (32.0 + place) / 64.0,
            };
            let name = format!("hairpin v0 {v0}, turning at {place} of frame 33");
            let start = [0.0, hairpin.c * hairpin.c];
            most = most.max(follow_all(&name, &hairpin, &zero, start, &c));
        }
    }
    println!(
        "follow smooth: 28 of 28 hairpins, 64 of 64 frames each, at most {most} iterations a frame"
    );
}

#[test]
fn lock_solver_core_follow_smooth() {
    follow_smooth(continuity());
}

negative_control!(
    lock_solver_core_follow_smooth,
    "the continuity test at depth 2 tags the first frame of a path from rest as s^2.5",
    expected = "s^2.5: a smooth path's frame 1 is tagged",
    follow_smooth(Continuity {
        depth: 2,
        ..continuity()
    })
);

// ── The measurement ───────────────────────────────────────────────────────────────────────────────────────────────────

/// One morph's measurement: the continued points over `frames` frames (by `follow`, or by the restart for the
/// control), the largest step, and, at 64 frames, the basin-change share.
struct Run {
    largest: f64,
    most_iterations: u32,
    /// The largest step between frames in the state, `‖F_s(x_k) − F_s′(x_{k−1})‖` over the words, reported only.
    largest_state: f64,
    changes: usize,
    restarts_converged: usize,
}

fn run<const N: usize, Fam: Family<f64, N>>(
    family: &Fam,
    t: &Target<f64, N>,
    x0: [f64; 2],
    seeds: &[[f64; 2]; 4],
    frames: usize,
    restart_chain: bool,
) -> Run {
    let mut x = x0;
    let mut largest: f64 = 0.0;
    let mut most_iterations = 0;
    let mut largest_state: f64 = 0.0;
    let state = |s: f64, x: [f64; 2]| {
        family
            .value(s, x)
            .expect("a converged frame's state is finite")
    };
    let mut previous = state(0.0, x0);
    let mut changes = 0;
    let mut restarts_converged = 0;
    for k in 1..=frames {
        let (s0, s1) = ((k - 1) as f64 / frames as f64, k as f64 / frames as f64);
        let frame = chart::lock_solve::Frame { family, s: s1 };
        let fresh = restart(&frame, t, seeds, &spec());
        let next = if restart_chain {
            fresh
        } else {
            follow(family, t, s0, x, s1, &spec(), &continuity())
        };
        assert!(
            next.converged(),
            "frame {k} of {frames} did not converge: {next:?}"
        );
        largest = largest.max(distance(next.x, x));
        most_iterations = most_iterations.max(next.iterations);
        let now = state(s1, next.x);
        let step: f64 = now
            .iter()
            .zip(&previous)
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        largest_state = largest_state.max(step.sqrt());
        previous = now;
        if fresh.converged() {
            restarts_converged += 1;
            if distance(fresh.x, next.x) > AGREE {
                changes += 1;
            }
        }
        x = next.x;
    }
    Run {
        largest,
        most_iterations,
        largest_state,
        changes,
        restarts_converged,
    }
}

/// The measurement on one morph; `restart_chain` puts the restart in the continued solve's place (the control).
fn measure<const N: usize, Fam: Family<f64, N>>(
    name: &str,
    family: &Fam,
    t: &Target<f64, N>,
    x0: [f64; 2],
    seeds: &[[f64; 2]; 4],
    restart_chain: bool,
) -> f64 {
    let runs: Vec<Run> = [64, 128, 256]
        .iter()
        .map(|&n| run(family, t, x0, seeds, n, restart_chain))
        .collect();
    let share = runs[0].changes as f64 / 64.0;
    println!(
        "{name}: basin changes {} of 64 frames (share {share:.4}), restarts converged {} of 64, at most {} iterations a \
         continued frame",
        runs[0].changes, runs[0].restarts_converged, runs[0].most_iterations
    );
    println!(
        "{name}: largest step L64 {:.6e}, L128 {:.6e}, L256 {:.6e}; ratios {:.4}, {:.4} (criterion ≤ {G8_RATIO})",
        runs[0].largest,
        runs[1].largest,
        runs[2].largest,
        runs[1].largest / runs[0].largest,
        runs[2].largest / runs[1].largest
    );
    println!(
        "{name}: in the state, reported only: largest step S64 {:.6e}, S128 {:.6e}, S256 {:.6e}; ratios {:.4}, {:.4}",
        runs[0].largest_state,
        runs[1].largest_state,
        runs[2].largest_state,
        runs[1].largest_state / runs[0].largest_state,
        runs[2].largest_state / runs[1].largest_state
    );
    assert!(
        runs[1].largest <= G8_RATIO * runs[0].largest
            && runs[2].largest <= G8_RATIO * runs[1].largest,
        "{name}: continuity fails G8: L64 {:e}, L128 {:e}, L256 {:e}",
        runs[0].largest,
        runs[1].largest,
        runs[2].largest
    );
    share
}

fn measurement(restart_chain: bool) {
    let zero = Target {
        value: [0.0; 3],
        weight: [1.0; 3],
    };
    let wells = Wells { t0: -0.2, t1: 0.2 };
    let x0 = solve(
        &chart::lock_solve::Frame {
            family: &wells,
            s: 0.0,
        },
        &zero,
        [-1.0, 0.0],
        &spec(),
    );
    assert!(x0.converged(), "{x0:?}");
    let share = measure(
        "morph (1), double well",
        &wells,
        &zero,
        x0.x,
        &WELL_SEEDS,
        restart_chain,
    );
    assert!(
        share > 0.0,
        "morph (1) has two basins and no change was seen: the measurement cannot fire"
    );

    let blend = Blend::fixture();
    let anchor = Target {
        value: blend.anchor(),
        weight: [1.0; 15],
    };
    let pin = [0.37, 0.61];
    let at0 = solve(
        &chart::lock_solve::Frame {
            family: &blend,
            s: 0.0,
        },
        &anchor,
        pin,
        &spec(),
    );
    assert_eq!(
        at0.status,
        Status::Converged(Criterion::OnImage),
        "the anchor is not on A's image at its pin: {at0:?}"
    );
    let quarters = [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75], [0.75, 0.75]];
    measure(
        "morph (2), latent state blend",
        &blend,
        &anchor,
        at0.x,
        &quarters,
        restart_chain,
    );
    let end = solve(
        &chart::lock_solve::Frame {
            family: &blend,
            s: 1.0,
        },
        &anchor,
        pin,
        &spec(),
    );
    println!(
        "morph (2): at s = 1 from the pin, {:?}, residual {:.6e}",
        end.status, end.residual
    );
}

#[test]
fn lock_solver_core_measurement() {
    measurement(false);
}

negative_control!(
    lock_solver_core_measurement,
    "the restart chain in the continued solve's place jumps basins and must fail G8",
    expected = "continuity fails G8",
    measurement(true)
);

/// The code review's overflow route: a finite ceiling whose product with the map's scale overflows, so μ reaches ∞
/// with it, every trial rejected (slopes negated). At f64, `ceiling: f64::MAX` on `F = (2u, 2v)` (scale 4); it must
/// return, tagged. `unguarded` (the control) drops the solver's exit on a non-finite μ.
fn terminates_ceiling_max(unguarded: bool) {
    let r = within_ten_seconds("f64 with ceiling f64::MAX", move || {
        #[allow(unused_mut)]
        let mut s = Spec {
            damping: Damping {
                ceiling: f64::MAX,
                ..spec64().damping
            },
            ..spec64()
        };
        #[cfg(feature = "controls")]
        {
            s.unguarded = unguarded;
        }
        #[cfg(not(feature = "controls"))]
        let _ = unguarded;
        let map = Exact {
            value: |x: [f64; 2]| Some([2.0 * x[0], 2.0 * x[1]]),
            jacobian: |_x: [f64; 2]| Some([[2.0, 0.0], [0.0, 2.0]]),
            flip: true,
        };
        let one = Target {
            value: [1.0; 2],
            weight: [1.0; 2],
        };
        solve(&map, &one, [1.0, 1.0], &s)
    });
    assert!(!r.converged(), "f64 ceiling f64::MAX: {r:?}");
    println!("terminates: f64 ceiling f64::MAX {:?}", r.status);
}

#[test]
fn lock_solver_core_terminates_ceiling_max() {
    terminates_ceiling_max(false);
}

negative_control!(
    lock_solver_core_terminates_ceiling_max,
    "the solver without its exit on a non-finite μ hangs once ceiling·scale overflows, and the timeout catches it",
    expected = "f64 with ceiling f64::MAX did not return within 10 s",
    terminates_ceiling_max(true)
);

/// The same route at f32 with the tests' own settings: a map of slope 10¹⁴ (scale 10²⁸, so ceiling·scale overflows),
/// slopes negated; it must return, tagged. `unguarded` is the control's, as above.
fn terminates_steep_f32(unguarded: bool) {
    let r = within_ten_seconds("f32 on a map of slope 1e14", move || {
        #[allow(unused_mut)]
        let mut s = spec32();
        #[cfg(feature = "controls")]
        {
            s.unguarded = unguarded;
        }
        #[cfg(not(feature = "controls"))]
        let _ = unguarded;
        let k = 1e14f32;
        let map = Exact {
            value: move |x: [f32; 2]| Some([k * x[0], k * x[1]]),
            jacobian: move |_x: [f32; 2]| Some([[k, 0.0], [0.0, k]]),
            flip: true,
        };
        let one = Target {
            value: [1.0f32; 2],
            weight: [1.0; 2],
        };
        solve(&map, &one, [1.0, 1.0], &s)
    });
    assert!(!r.converged(), "f32 slope 1e14: {r:?}");
    println!("terminates: f32 slope 1e14 {:?}", r.status);
}

#[test]
fn lock_solver_core_terminates_steep_f32() {
    terminates_steep_f32(false);
}

negative_control!(
    lock_solver_core_terminates_steep_f32,
    "the solver without its exit on a non-finite μ hangs on a steep map at f32, and the timeout catches it",
    expected = "f32 on a map of slope 1e14 did not return within 10 s",
    terminates_steep_f32(true)
);
