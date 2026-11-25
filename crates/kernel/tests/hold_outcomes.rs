//! The outcomes of hold `L_z` with `E` (`kernel::decode::hold`; chart_composition § 5.3, "The outcomes, in this order"),
//! each at the f64 and the f32 instantiation natively, on inputs `(m, r, v)` whose values are exact at both:
//! - `hold_outcomes_in_order`: each of the six outcomes on an exact fixture; a tagged decode keeps its velocities bit
//!   for bit; a rest start on the floor becomes `(L*/I)·Jr`; the collision pole beats `E* < U`, which beats `2K* < L*²`.
//! - `hold_outcomes_state_at_the_floor`: at f32, the state the second projection leaves at `K̂ = 0` (masses
//!   `(¼, ¼, ½)`, positions `(∓√2, 0)`, `(0, 0)`, `v = 0.005·Jr`), above the floor and on it: its outcome is the exact
//!   one, and every float is finite.
//! - `hold_outcomes_tags_agree`: the outcome tags are bit-identical at f32 and f64 on identical `(m, r, v)` across a
//!   grid on both sides of `τ = 2⁻²²`, the state above and exactly rigid draws included, and no arm has a NaN.
//! - `hold_outcomes_rigid_exact`: [`rigid`](kernel::decode::hold::rigid) equals the exact rational comparison, `v` on
//!   both sides of `τ`, at both precisions.
//! - `hold_outcomes_floor_order`: [`floor_order`](kernel::decode::hold::floor_order) is the exact sign of `2K* − L*²`
//!   and its `< 0` is `infeasible`.
//!
//! **Pass criteria, written before the runs.** Every outcome in the table on its fixture; `rigid` and `floor_order`
//! equal the exact answer on every draw and on every grid point, no exceptions; the tags equal at f32 and f64 on every
//! draw; no NaN or infinity in any velocity. The controls: the order swapped at 2 and 3 fails the table; a tag formed
//! from `s = 0` and `K̂ = 0` differs between f32 and f64 on exactly rigid draws (16.5 % of 6,000 in the design
//! document's measurement); a plain float evaluation of the wedge is not `rigid`.

mod decode_support;
mod hold_support;

use decode_support::P;
use hold_support::*;
use kernel::chart::invariant::WarpReal;
use kernel::decode::energy_norm::Outcome;
use kernel::decode::hold::{floor_order, hold, rigid, Held};
use kernel::decode::invariant::{infeasible, ExactSign};
use num_bigint::BigInt;
use num_rational::BigRational;
use validation::negative_control;

/// The hold under test: `hold`'s own signature.
type Hold<R> = fn([R; 3], &[[R; 2]; 3], &[[R; 2]; 3], R, R, R, bool, R) -> Held<R>;

/// The canonical fixture state: masses `(¼, ¼, ½)`, `r = (−1, 1), (1, −1), (0, 0)`, so `I = 1` exactly, and `w = Jr`.
const M: [f64; 3] = [0.25, 0.25, 0.5];
const R0: Bodies = [[-1.0, 1.0], [1.0, -1.0], [0.0, 0.0]];

/// A held state: `hold` at `R` on `(M, R0, v)` with the potential `u`, `L*` and `E*`.
fn run<R: P + ExactSign>(f: Hold<R>, v: &Bodies, r_min: f64, u: f64, l: f64, e: f64) -> Held<R> {
    f(
        M.map(R::of),
        &to::<R>(&R0),
        &to::<R>(v),
        R::of(u),
        R::of(r_min),
        R::of(l),
        true,
        R::of(e),
    )
}

/// `v` as bits, to compare "untouched" exactly.
fn bits<R: P>(v: &[[R; 2]; 3]) -> Vec<u64> {
    v.iter().flatten().map(|x| x.bits()).collect()
}

/// A moving velocity with `Σmv = 0`, off the rigid line by a lot.
const MOVING: Bodies = [[0.5, 0.25], [-0.5, 0.25], [0.0, -0.25]];

/// The six outcomes on their fixtures, `U = −0.25`: the floor is `K* = L*²/2`.
fn check_outcomes<R: P + ExactSign>(f: Hold<R>) {
    let w = jr(&R0);
    let rest = [[0.0; 2]; 3];
    let rigid_v = w.map(|b| b.map(|x| 0.5 * x));
    let u = -0.25;

    // 1: the collision pole, whatever E* and L*: untouched, ENERGY_UNSCALABLE. E* < U would be 2.
    let h = run::<R>(f, &MOVING, 0.0, 0.0, 0.5, -3.0);
    assert_eq!(
        h.outcome,
        Outcome::Unscalable,
        "outcome 1: the collision pole"
    );
    assert_eq!(
        bits(&h.v),
        bits(&to::<R>(&MOVING)),
        "outcome 1 moved the velocities"
    );

    // 2: E* < U, though 2K* < L*² as well: ENERGY_INFEASIBLE, the order's 2 before 3.
    let h = run::<R>(f, &MOVING, 1.0, u, 0.5, -1.0);
    assert_eq!(h.outcome, Outcome::Infeasible, "outcome 2: E* < U");
    assert_eq!(
        bits(&h.v),
        bits(&to::<R>(&MOVING)),
        "outcome 2 moved the velocities"
    );

    // 3: E* ≥ U and 2K* < L*²: K* = 0.0625, L*² = 0.25.
    let h = run::<R>(f, &MOVING, 1.0, u, 0.5, -0.1875);
    assert_eq!(h.outcome, Outcome::BelowFloor, "outcome 3: 2K* < L*²");
    assert_eq!(
        bits(&h.v),
        bits(&to::<R>(&MOVING)),
        "outcome 3 moved the velocities"
    );

    // 4: a rest start, and a rigid one, on the floor (K* = 0.125, L* = 0.5): untagged, the rotation applied.
    for v in [&rest, &rigid_v] {
        let h = run::<R>(f, v, 1.0, u, 0.5, -0.125);
        assert_eq!(h.outcome, Outcome::OnFloor, "outcome 4: rigid on the floor");
        assert_eq!(h.outcome.tag(), None, "outcome 4 is tagged");
        let want = to::<R>(&w.map(|b| b.map(|x| 0.5 * x)));
        // Compared as values: the rotation's zero components are +0, and `0.5·(−0)` is −0.
        assert_eq!(
            from::<R>(&h.v),
            from::<R>(&want),
            "a start on the floor is not (L*/I)·Jr"
        );
    }

    // 5: rest and rigid above the floor (K* = 0.25): ENERGY_UNSCALABLE, untouched.
    for v in [&rest, &rigid_v] {
        let h = run::<R>(f, v, 1.0, u, 0.5, 0.0);
        assert_eq!(
            h.outcome,
            Outcome::Unscalable,
            "outcome 5: rigid above the floor"
        );
        assert_eq!(
            bits(&h.v),
            bits(&to::<R>(v)),
            "outcome 5 moved the velocities"
        );
    }

    // 6: otherwise applied, on both targets (K* = 0.25 → E* = 0).
    let h = run::<R>(f, &MOVING, 1.0, u, 0.5, 0.0);
    assert_eq!(h.outcome, Outcome::Held, "outcome 6");
    let (m, r, v) = (M, from::<R>(&to::<R>(&R0)), from::<R>(&h.v));
    assert!((lz(&m, &r, &v) - 0.5).abs() < 1e-5, "outcome 6 misses L*");
    assert!((kinetic(&m, &v) - 0.25).abs() < 1e-5, "outcome 6 misses K*");
    // Not rigid, on the floor: η = 0, the rigid rotation.
    let h = run::<R>(f, &MOVING, 1.0, u, 0.5, -0.125);
    assert_eq!(h.outcome, Outcome::Held, "outcome 6 on the floor");
    let v = from::<R>(&h.v);
    assert!(
        (kinetic(&m, &v) - 0.125).abs() < 1e-6,
        "the floor is not the rigid rotation"
    );
}

#[test]
fn hold_outcomes_in_order() {
    check_outcomes::<f64>(hold::<f64>);
    check_outcomes::<f32>(hold::<f32>);
}

/// `hold` with outcomes 2 and 3 exchanged: the order swapped, since `E* < U` implies `2K* < L*²`, so the swap changes
/// only the name the first of the two gives.
#[cfg(feature = "controls")]
#[allow(clippy::too_many_arguments)] // Reason: `hold`'s own signature.
fn swapped<R: P + ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    u: R,
    r_min: R,
    lz: R,
    e_on: bool,
    e: R,
) -> Held<R> {
    let mut h = hold(m, r, v, u, r_min, lz, e_on, e);
    h.outcome = match h.outcome {
        Outcome::Infeasible => Outcome::BelowFloor,
        Outcome::BelowFloor => Outcome::Infeasible,
        o => o,
    };
    h
}

negative_control!(
    hold_outcomes_in_order,
    "the order swapped at 2 and 3 names the wrong outcome",
    expected = "outcome 2: E* < U",
    check_outcomes::<f64>(swapped::<f64>)
);

/// The state the second projection leaves at `K̂ = 0` (the module's note), at the precision `R`, with its `L*`: above
/// the floor (`a² = 0.01`) and on it.
fn check_state_at_the_floor<R: P + ExactSign>(f: Hold<R>) {
    let m = [0.25, 0.25, 0.5];
    let s = 2.0_f64.sqrt() as f32 as f64;
    let r = [[-s, 0.0], [s, 0.0], [0.0, 0.0]];
    let v = jr(&r).map(|b| b.map(|x| (0.005 * x) as f32 as f64));
    // `L* = 1/16`, so the floor `K* = L*²/2` is a dyadic and `E*` forms it exactly at both precisions.
    let (l, u) = (0.0625, -0.5);
    for (what, e, want) in [
        ("above the floor", u + 0.0078125, Outcome::Unscalable),
        ("on the floor", u + 0.001953125, Outcome::OnFloor),
    ] {
        let h = f(
            m.map(R::of),
            &to::<R>(&r),
            &to::<R>(&v),
            R::of(u),
            R::of(1.0),
            R::of(l),
            true,
            R::of(e),
        );
        assert_eq!(h.outcome, want, "the state at K̂ = 0, {what}");
        assert!(
            h.v.iter().flatten().all(|x| x.f().is_finite()),
            "a non-finite velocity at the state, {what}"
        );
    }
}

#[test]
fn hold_outcomes_state_at_the_floor() {
    check_state_at_the_floor::<f64>(hold::<f64>);
    check_state_at_the_floor::<f32>(hold::<f32>);
}

/// `hold` with a start on the floor read as above it: outcome 4 named 5.
#[cfg(feature = "controls")]
#[allow(clippy::too_many_arguments)] // Reason: `hold`'s own signature.
fn floor_as_above<R: P + ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    u: R,
    r_min: R,
    lz: R,
    e_on: bool,
    e: R,
) -> Held<R> {
    let mut h = hold(m, r, v, u, r_min, lz, e_on, e);
    if h.outcome == Outcome::OnFloor {
        h.outcome = Outcome::Unscalable;
    }
    h
}

negative_control!(
    hold_outcomes_state_at_the_floor,
    "the floor read as above it names the wrong outcome",
    expected = "the state at K̂ = 0, on the floor",
    check_state_at_the_floor::<f64>(floor_as_above::<f64>)
);

/// The velocities with `s = 0` or `K̂ = 0` as the test: the tag read from the divisions at the precision `R`, in the
/// outcomes' order (the control's mutant).
#[cfg(feature = "controls")]
#[allow(clippy::too_many_arguments)] // Reason: `hold`'s own signature.
fn tag_from_s_and_k_hat<R: P + ExactSign>(
    m: [R; 3],
    r: &[[R; 2]; 3],
    v: &[[R; 2]; 3],
    u: R,
    r_min: R,
    lz: R,
    e_on: bool,
    e: R,
) -> Held<R> {
    let mut h = hold(m, r, v, u, r_min, lz, e_on, e);
    if !matches!(
        h.outcome,
        Outcome::Held | Outcome::OnFloor | Outcome::Unscalable
    ) {
        return h;
    }
    let zero = R::of(0.0);
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
    let no_direction = if s == zero {
        true
    } else {
        let d = dv.map(|b| b.map(|x| x / s));
        let c = ip(&w, &d) / i;
        let d = [0, 1, 2].map(|b| [0, 1].map(|q| d[b][q] - c * w[b][q]));
        R::of(0.5) * ip(&d, &d) == zero
    };
    let on_floor = floor_order(e - u, lz) == 0;
    h.outcome = if no_direction {
        if on_floor {
            Outcome::OnFloor
        } else {
            Outcome::Unscalable
        }
    } else {
        Outcome::Held
    };
    h
}

/// The draws: exactly rigid starts (`v` the rotation scaled by a power of two or a decade, rounded to f32) and
/// starts on both sides of `τ`, each as f32 values, so f32 and f64 see identical `(m, r, v)`.
fn grid() -> Vec<([f64; 3], Bodies, Bodies)> {
    let mut rng = Rng(77);
    let mut out = Vec::new();
    // The fixture state, rigid and off it.
    for ratio in [0.0, 1e-8, 1e-7, 2e-7, 3e-7, 5e-7, 1e-6, 1e-5, 1e-3, 1e-1] {
        for _ in 0..200 {
            let (m, r) = state(&mut rng);
            let (m, r) = (to_f32(&m), r.map(|b| b.map(|x| x as f32 as f64)));
            let c = 10f64.powf(rng.between(-3.0, 0.0));
            let w = jr(&r);
            let mut v = w.map(|b| b.map(|x| c * x));
            if ratio > 0.0 {
                let big = v.iter().flatten().fold(0.0_f64, |a, x| a.max(x.abs()));
                for body in &mut v {
                    for x in body.iter_mut() {
                        *x += ratio * big * rng.between(-1.0, 1.0);
                    }
                }
            }
            out.push((m, r, v.map(|b| b.map(|x| x as f32 as f64))));
        }
    }
    out
}

/// `m` rounded to f32, as f64.
fn to_f32(m: &[f64; 3]) -> [f64; 3] {
    m.map(|x| x as f32 as f64)
}

/// The tags at f32 and f64 on identical inputs, over [`grid`], with the targets `u`, `L*` and `E*` giving both sides
/// of the floor; no non-finite velocity.
fn check_tags_agree(f32_hold: Hold<f32>, f64_hold: Hold<f64>) {
    let mut checked = 0;
    let mut disagreeing = 0;
    for (m, r, v) in grid() {
        for (u, l, e) in [
            (-0.25, 0.5, -0.125),
            (-0.25, 0.5, 0.0),
            (-0.25, 0.25, -0.125),
            (-0.5, 0.0, -0.25),
        ] {
            let a = f32_hold(
                m.map(|x| x as f32),
                &to::<f32>(&r),
                &to::<f32>(&v),
                u as f32,
                1.0,
                l as f32,
                true,
                e as f32,
            );
            let b = f64_hold(m, &r, &v, u, 1.0, l, true, e);
            assert!(
                a.v.iter().flatten().all(|x| x.is_finite())
                    && b.v.iter().flatten().all(|x| x.is_finite()),
                "a non-finite velocity"
            );
            checked += 1;
            if a.outcome != b.outcome {
                disagreeing += 1;
            }
        }
    }
    println!("{checked} draws, {disagreeing} disagree");
    assert_eq!(
        disagreeing, 0,
        "the tags differ between f32 and f64 on {disagreeing} of {checked} identical inputs"
    );
}

#[test]
fn hold_outcomes_tags_agree() {
    check_tags_agree(hold::<f32>, hold::<f64>);
}

negative_control!(
    hold_outcomes_tags_agree,
    "a tag from s = 0 and K̂ = 0 differs between f32 and f64 on exactly rigid draws",
    expected = "the tags differ between f32 and f64",
    check_tags_agree(tag_from_s_and_k_hat::<f32>, tag_from_s_and_k_hat::<f64>)
);

/// `rigid(v, w)` by the definition, in exact rationals.
fn rigid_exact(v: &Bodies, w: &Bodies) -> bool {
    let q = |x: f64| BigRational::from_float(x).expect("finite");
    let v: Vec<BigRational> = v.iter().flatten().map(|&x| q(x)).collect();
    let w: Vec<BigRational> = w.iter().flatten().map(|&x| q(x)).collect();
    let abs = |x: &BigRational| {
        if *x < BigRational::from_integer(0.into()) {
            -x.clone()
        } else {
            x.clone()
        }
    };
    let big_v = v
        .iter()
        .map(abs)
        .fold(BigRational::from_integer(0.into()), |a, x| {
            if x > a {
                x
            } else {
                a
            }
        });
    let big_w = w
        .iter()
        .map(abs)
        .fold(BigRational::from_integer(0.into()), |a, x| {
            if x > a {
                x
            } else {
                a
            }
        });
    let p = w.iter().position(|x| abs(x) == big_w).unwrap();
    let tau = BigRational::new(BigInt::from(1), BigInt::from(1u64 << 22));
    (0..6).all(|b| abs(&(&v[b] * &w[p] - &v[p] * &w[b])) <= &tau * &big_v * abs(&w[p]))
}

/// A plain float evaluation of the same wedge at `R`: what `rigid` is not.
#[cfg(feature = "controls")]
fn rigid_floats<R: P + ExactSign>(v: &[[R; 2]; 3], w: &[[R; 2]; 3]) -> bool {
    let (v, w): (Vec<R>, Vec<R>) = (
        v.iter().flatten().copied().collect(),
        w.iter().flatten().copied().collect(),
    );
    let zero = R::of(0.0);
    let big_v = v.iter().fold(zero, |a, x| a.max(x.abs()));
    let big_w = w.iter().fold(zero, |a, x| a.max(x.abs()));
    let p = w.iter().position(|x| x.abs() == big_w).unwrap();
    let tau = R::of(2.0f64.powi(-22));
    (0..6).all(|b| (v[b] * w[p] - v[p] * w[b]).abs() <= tau * big_v * w[p].abs())
}

/// A velocity predicate under test: `rigid`'s own signature.
type Rigid<R> = fn(&[[R; 2]; 3], &[[R; 2]; 3]) -> bool;

/// `rigid` against the exact comparison on [`grid`] and on draws within a factor 2 of `τ`.
fn check_rigid_exact<R: P + ExactSign>(rigid: Rigid<R>) {
    let mut rng = Rng(3);
    let (mut yes, mut no) = (0, 0);
    let mut cases: Vec<(Bodies, Bodies)> = grid().into_iter().map(|(_, r, v)| (r, v)).collect();
    for _ in 0..3000 {
        let (m, r) = state(&mut rng);
        let r = r.map(|b| b.map(|x| R::of(x).f()));
        let ratio = 2.0f64.powf(rng.between(-24.0, -20.0));
        cases.push((r, near_rigid(&m, &r, ratio, &mut rng)));
    }
    for (r, v) in cases {
        let (r, v) = (to::<R>(&r), to::<R>(&v));
        let (rf, vf) = (from::<R>(&r), from::<R>(&v));
        let w = jr(&rf);
        let want = rigid_exact(&vf, &w);
        let wr = to::<R>(&w);
        let got = rigid(&vf.map(|b| b.map(R::of)), &wr);
        if want {
            yes += 1;
        } else {
            no += 1;
        }
        assert_eq!(
            got, want,
            "rigid is not the exact comparison: got {got}, exact {want}"
        );
    }
    println!("{yes} rigid, {no} not");
    assert!(yes > 100 && no > 100, "the draws are on one side of τ");
}

#[test]
fn hold_outcomes_rigid_exact() {
    check_rigid_exact::<f64>(rigid::<f64>);
    check_rigid_exact::<f32>(rigid::<f32>);
}

negative_control!(
    hold_outcomes_rigid_exact,
    "a plain float evaluation of the wedge is not the exact comparison",
    expected = "rigid is not the exact comparison",
    check_rigid_exact::<f32>(rigid_floats::<f32>)
);

/// The exact sign of `2K* − L*²` from rationals.
fn order_exact(k: f64, l: f64) -> i32 {
    let q = |x: f64| BigRational::from_float(x).expect("finite");
    let d = q(k) * BigRational::from_integer(2.into()) - q(l) * q(l);
    let zero = BigRational::from_integer(0.into());
    (d > zero) as i32 - (d < zero) as i32
}

/// `floor_order` against the exact sign, and `infeasible` against its `< 0`, over values on both sides of the floor.
fn check_floor_order<R: P + WarpReal>(order: fn(R, R) -> i32) {
    let mut rng = Rng(21);
    let mut seen = [0usize; 3];
    for n in 0..20_000 {
        let l = R::of(rng.between(-2.0, 2.0) * if n % 7 == 0 { 0.0 } else { 1.0 });
        let on = R::of(0.5 * l.f() * l.f());
        // Values on the floor to the bit, one ulp each side, and anywhere.
        let k = match n % 4 {
            0 => on,
            1 => on.above(),
            2 => on.below(),
            _ => R::of(rng.between(-1.0, 3.0)),
        };
        // `on` is `L*²/2` rounded, so its floor is only exact where the rounding is.
        let want = order_exact(k.f(), l.f());
        let got = order(k, l);
        seen[(want + 1) as usize] += 1;
        assert_eq!(
            got,
            want,
            "floor_order is not the exact sign at K* = {:e}, L* = {:e}",
            k.f(),
            l.f()
        );
        assert_eq!(
            infeasible(R::of(1.0), k, l),
            want < 0,
            "infeasible and the floor's sign differ at K* = {:e}, L* = {:e}",
            k.f(),
            l.f()
        );
    }
    println!("below {} on {} above {}", seen[0], seen[1], seen[2]);
    assert!(
        seen.iter().all(|&n| n > 100),
        "a side of the floor is not drawn"
    );
}

#[test]
fn hold_outcomes_floor_order() {
    check_floor_order::<f64>(floor_order::<f64>);
    check_floor_order::<f32>(floor_order::<f32>);
}

/// The sign read from the rounded `2K* − L*²`, which is what the exact routine is there to avoid.
#[cfg(feature = "controls")]
fn floor_rounded<R: P + WarpReal>(k: R, l: R) -> i32 {
    let d = k + k - l * l;
    (d > R::of(0.0)) as i32 - (d < R::of(0.0)) as i32
}

negative_control!(
    hold_outcomes_floor_order,
    "the sign of the rounded 2K* − L*² misreads the floor",
    expected = "floor_order is not the exact sign",
    check_floor_order::<f32>(floor_rounded::<f32>)
);
