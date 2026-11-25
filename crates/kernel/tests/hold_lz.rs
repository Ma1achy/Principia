//! Hold `L_z` and both held (`kernel::decode::hold`; chart_composition § 5.3), each check at the f64 and the f32
//! instantiation natively:
//! - `hold_lz_sets_target`: `L_z` alone lands on `L*`, the velocity change is a multiple of `Jr` (the nearest point),
//!   and `Σp = 0`; with both held, `E` is `E*` too.
//! - `hold_lz_near_rigid`: at f32, from `‖δv‖/‖v‖ = 10⁻¹` to `10⁻⁶`, both held lands on `L*` to `1e-6` of `a`, the
//!   sphere's radius. A pixel within `τ` of rigid is tagged, momenta kept, and is not in this range.
//! - `hold_lz_off_bitwise`: with `L_z` off the stage's decode is `η_E`'s, bit for bit, and the uniform's row is
//!   `η_E`'s bytes.
//! - `hold_lz_chart_flags`: a chart with `L_z` as an axis forbids the hold, one that holds it by construction runs no
//!   rotation, every other chart's hold runs.
//!
//! **The tolerances** (written before the runs): the held `L_z` and `E` are formed from `O(1)` terms in a few tens of
//! roundings, so `|L_z − L*|`, `|E − E*|` and `|Σp|` are at most `40·eps·scale`, `scale = max(1, |L*|, √(2K*))`. The
//! near-rigid bound is the review's worst, `1.8e-7` of `a` at f32, with a margin of 5.

mod decode_support;
mod hold_support;

use decode_support::P;
use hold_support::*;
use kernel::chart::invariant::WarpReal;
use kernel::chart::{Chart, ChartDecode, Latent, LzHold};
use kernel::decode::energy_norm::Outcome;
use kernel::decode::hold::{hold, Held};
use kernel::decode::invariant::ExactSign;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{energy, hold as row, sample_held, sample_with};
use validation::negative_control;

/// A hold under test: `hold`'s own signature.
type Hold<R> = fn([R; 3], &[[R; 2]; 3], &[[R; 2]; 3], R, R, R, bool, R) -> Held<R>;

/// The worst of the draws, for the printed figure.
#[derive(Default)]
struct Worst(std::cell::Cell<f64>);

impl Worst {
    fn see(&self, x: f64) {
        self.0.set(self.0.get().max(x));
    }
}

/// `E*` reachable from `K* = L*²/2 + a²/2` at the state's `U`.
fn target(u: f64, l: f64, a2: f64) -> f64 {
    u + 0.5 * l * l + 0.5 * a2
}

/// `L*` alone, then both, over `n` draws: the checks of the module's note.
fn check_sets_target<R: P + ExactSign>(f: Hold<R>, n: usize, worst: &Worst) {
    let mut rng = Rng(11);
    let tol = 40.0 * R::eps();
    for k in 0..n {
        let (m, r) = state(&mut rng);
        let v = velocity(&m, &mut rng);
        let l = rng.between(-1.5, 1.5);
        let a2 = rng.between(0.05, 4.0);
        let u = potential(&m, &r);
        let e = target(u, l, a2);
        let scale = 1.0_f64.max(l.abs()).max((l * l + a2).sqrt());
        for both in [false, true] {
            let out = f(
                m.map(R::of),
                &to::<R>(&r),
                &to::<R>(&v),
                R::of(u),
                R::of(1.0),
                R::of(l),
                both,
                R::of(if both { e } else { 0.0 }),
            );
            assert_eq!(out.outcome, Outcome::Held, "draw {k}");
            let (mf, rf, vf) = (
                m.map(|x| R::of(x).f()),
                from::<R>(&to::<R>(&r)),
                from::<R>(&out.v),
            );
            let dl = (lz(&mf, &rf, &vf) - R::of(l).f()).abs() / scale;
            let de = (kinetic(&mf, &vf) + potential(&mf, &rf) - R::of(e).f()).abs() / scale;
            let dp = momentum_sum(&mf, &vf) / scale;
            worst.see(dl.max(if both { de } else { 0.0 }).max(dp));
            assert!(dl <= tol, "L_z is off by {dl} (draw {k}, both {both})");
            assert!(dp <= tol, "Σp is {dp} (draw {k}, both {both})");
            if both {
                assert!(de <= tol, "E is off by {de} (draw {k})");
            } else {
                // The nearest point: the change is a multiple of Jr.
                let vin = from::<R>(&to::<R>(&v));
                let w = jr(&rf);
                let c = (lz(&mf, &rf, &vf) - lz(&mf, &rf, &vin)) / ip(&mf, &w, &w);
                for b in 0..3 {
                    for q in 0..2 {
                        let off = (vf[b][q] - vin[b][q] - c * w[b][q]).abs();
                        assert!(
                            off <= tol * scale,
                            "the change is not along Jr: {off} (draw {k})"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn hold_lz_sets_target() {
    let w = Worst::default();
    check_sets_target::<f64>(hold::<f64>, 4000, &w);
    println!("worst f64: {:e} eps·scale-normalised", w.0.get());
    let w = Worst::default();
    check_sets_target::<f32>(hold::<f32>, 4000, &w);
    println!("worst f32: {:e}", w.0.get());
}

/// Both held with `η_E` alone, the hold's step replaced by `p ← η_E·p` (what the step took the place of).
#[cfg(feature = "controls")]
#[allow(clippy::too_many_arguments)] // Reason: `hold`'s own signature.
fn eta_only<R: P + ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    u: R,
    r_min: R,
    lz: R,
    e_on: bool,
    e_star: R,
) -> Held<R> {
    let mut h = hold(m, r, v, u, r_min, lz, e_on, e_star);
    let k0 = R::of(0.5)
        * (0..3)
            .map(|i| m[i] * (v[i][0] * v[i][0] + v[i][1] * v[i][1]))
            .fold(R::of(0.0), |a, b| a + b);
    let eta = ((e_star - u) / k0).sqrt();
    h.v = v.map(|b| b.map(|x| eta * x));
    h
}

negative_control!(
    hold_lz_sets_target,
    "η_E applied alone with both on breaks the held L_z",
    expected = "L_z is off by",
    check_sets_target::<f64>(eta_only::<f64>, 50, &Worst::default())
);

/// The both-held step with one projection of `Jr` out of `δ̂`, not two (the control's mutant).
#[cfg(feature = "controls")]
#[allow(clippy::too_many_arguments)] // Reason: `hold`'s own signature.
fn one_projection<R: P + ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    u: R,
    _r_min: R,
    lz: R,
    _e_on: bool,
    e_star: R,
) -> Held<R> {
    let zero = R::of(0.0);
    let half = R::of(0.5);
    let w = [
        [-r[0][1], r[0][0]],
        [-r[1][1], r[1][0]],
        [-r[2][1], r[2][0]],
    ];
    let ip = |a: &[[R; 2]; 3], b: &[[R; 2]; 3]| {
        (0..3)
            .map(|i| m[i] * (a[i][0] * b[i][0] + a[i][1] * b[i][1]))
            .fold(zero, |x, y| x + y)
    };
    let i = ip(&w, &w);
    let l0 = ip(&w, v);
    let dv = [0, 1, 2].map(|b| [0, 1].map(|c| v[b][c] - (l0 / i) * w[b][c]));
    let s = dv.iter().flatten().fold(zero, |a, x| a.max(x.abs()));
    let unit = dv.map(|b| b.map(|x| x / s));
    let k_hat = half * ip(&unit, &unit);
    let spare = (e_star - u - lz * lz / (i + i)).max(zero);
    let eta = (spare / k_hat).sqrt();
    Held {
        v: [0, 1, 2].map(|b| [0, 1].map(|c| (lz / i) * w[b][c] + eta * unit[b][c])),
        outcome: Outcome::Held,
    }
}

/// Near-rigid decodes at f32 held on both: `|L_z − L*|` as a fraction of `a`.
fn check_near_rigid(f: Hold<f32>, ratios: &[f64], bound: f64) {
    let mut rng = Rng(5);
    let (l, a2) = (0.7, 1.0);
    for &ratio in ratios {
        let mut worst = 0.0_f64;
        let mut tagged = 0;
        for _ in 0..2000 {
            let (m, r) = state(&mut rng);
            let v = near_rigid(&m, &r, ratio, &mut rng);
            let u = potential(&m, &r);
            let e = target(u, l, a2);
            let (mf, rf) = (m.map(|x| x as f32), to::<f32>(&r));
            let out = f(
                mf,
                &rf,
                &to::<f32>(&v),
                u as f32,
                1.0,
                l as f32,
                true,
                e as f32,
            );
            if out.outcome != Outcome::Held {
                tagged += 1;
                continue;
            }
            let m64 = mf.map(f64::from);
            let dl =
                (lz(&m64, &from::<f32>(&rf), &from::<f32>(&out.v)) - f64::from(l as f32)).abs();
            worst = worst.max(dl / a2.sqrt());
        }
        println!("ratio {ratio:e}: worst |ΔL_z|/a {worst:e}, tagged {tagged}");
        assert!(
            worst <= bound,
            "|ΔL_z|/a is {worst:e} at ratio {ratio:e}, past {bound:e}"
        );
        if ratio >= 1e-4 {
            assert_eq!(tagged, 0, "a pixel at ratio {ratio:e} is tagged");
        }
    }
}

const RATIOS: [f64; 6] = [1e-1, 1e-2, 1e-3, 1e-4, 1e-5, 1e-6];

#[test]
fn hold_lz_near_rigid() {
    check_near_rigid(hold::<f32>, &RATIOS, 1e-6);
}

negative_control!(
    hold_lz_near_rigid,
    "one projection of Jr instead of two fails the near-rigid fixture",
    expected = "|ΔL_z|/a is",
    check_near_rigid(one_projection::<f32>, &[1e-3, 1e-5], 1e-6)
);

/// The two decodes' bits equal, by their printed forms (a float's shortest round-trip form is its bits').
fn same<T: std::fmt::Debug>(a: &T, b: &T, what: &str) {
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "{what}");
}

/// The cases of the off-switch checks: latent `z₀`, one moving, one near rest, one with a heavy third mass.
const CASES: [[f64; 8]; 3] = [
    [0.0, 0.0, 1.0, -0.5, 0.3, 0.7, 0.2, -0.4],
    [0.4, -0.2, 0.2, 0.9, -0.6, 0.1, 0.5, 0.3],
    [-0.9, 0.8, 0.0, 0.0, 0.0, 0.0, 0.1, -0.1],
];

/// With `L_z` off (`None`, or `lz_star`'s reading of it), the stage's decode is `sample_with`'s, bit for bit, for each
/// `E*`.
fn check_off<R>(lz_star: fn() -> Option<R>)
where
    R: P + WarpReal,
    Latent<R>: ChartDecode<Param = R>,
{
    for z0 in CASES {
        let chart = Latent {
            z0: z0.map(R::of),
            q1: [R::of(0.0); 8],
            q2: [R::of(0.0); 8],
        };
        for e in [None, Some(0.0), Some(-0.4), Some(0.9)] {
            let e = e.map(R::of);
            let old = sample_with::<_, Defaults, R>(&chart, R::of(0.0), R::of(0.0), e);
            let new = sample_held::<_, Defaults, R>(&chart, R::of(0.0), R::of(0.0), e, lz_star());
            same(&new.ic, &old.ic, "the decode's IC moved with L_z off");
            same(
                &new.descriptor,
                &old.descriptor,
                "the descriptor moved with L_z off",
            );
            same(
                &new.degenerate,
                &old.degenerate,
                "the tag moved with L_z off",
            );
        }
    }
}

#[test]
fn hold_lz_off_bitwise() {
    check_off::<f64>(|| None);
    check_off::<f32>(|| None);
    // The uniform's row is η_E's bytes with `L_z` off, and carries `L*` beside it when on.
    for e in [None, Some(0.0_f32), Some(-0.3), Some(2.5)] {
        assert_eq!(
            row(e, None),
            energy(e),
            "the row with L_z off is the energy row"
        );
        let on = row(e, Some(0.7));
        assert_eq!(on.x & 2, 2);
        assert_eq!(f32::from_bits(on.z), 0.7);
        assert_eq!((on.x & 1, on.y), (energy(e).x & 1, energy(e).y));
    }
}

negative_control!(
    hold_lz_off_bitwise,
    "an off switch read as L* = 0 moves the decode",
    expected = "moved with L_z off",
    check_off::<f64>(|| Some(0.0))
);

/// A chart's flag decides: `L_z` an axis forbids, a construction runs no rotation, a free chart's runs; through the
/// stage, a hold on a construction chart is the decode it would be without one.
/// The stage's decode under a hold on `L_z` of a free chart: `decode` is `sample_held`'s own signature.
type Stage = fn(&Latent<f64>, Option<f64>) -> kernel::stage::decode::DecodeOut<f64>;

fn check_flags(decode: Stage) {
    use kernel::chart::invariant::Invariant;
    use kernel::chart::{InvariantLE, InvariantLK};
    let free = Latent::<f64> {
        z0: [0.0; 8],
        q1: [0.0; 8],
        q2: [0.0; 8],
    };
    assert_eq!(free.lz_hold(), LzHold::Free);
    assert!(!free.forbids_lz_hold());
    let inv = Invariant::<f64> {
        z_cfg: [0.0; 2],
        z_mass: [0.0; 2],
        k_max: 1.0,
        gamma_k: 1.0,
    };
    assert_eq!(InvariantLE(inv).lz_hold(), LzHold::Axis);
    assert!(InvariantLK(inv).forbids_lz_hold());
    for z0 in CASES {
        let chart = Latent {
            z0: z0.map(f64::from),
            q1: [0.0; 8],
            q2: [0.0; 8],
        };
        let off = decode(&chart, None);
        let on = decode(&chart, Some(0.9));
        assert_ne!(
            format!("{off:?}"),
            format!("{on:?}"),
            "a free chart's hold did not run"
        );
    }
}

fn stage(c: &Latent<f64>, lz: Option<f64>) -> kernel::stage::decode::DecodeOut<f64> {
    sample_held::<_, Defaults, f64>(c, 0.0, 0.0, None, lz)
}

#[test]
fn hold_lz_chart_flags() {
    check_flags(stage);
}

/// A decode that drops the hold: a free chart's flag read as "forbidden".
#[cfg(feature = "controls")]
fn dropped(c: &Latent<f64>, _: Option<f64>) -> kernel::stage::decode::DecodeOut<f64> {
    stage(c, None)
}

negative_control!(
    hold_lz_chart_flags,
    "a free chart whose hold is dropped did not run it",
    expected = "a free chart's hold did not run",
    check_flags(dropped)
);
