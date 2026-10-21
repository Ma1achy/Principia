//! The Inspector's gates, `inspector-handles` and `inspector-roundtrip` (TASK-M2-27; REQ-VAL-021, REQ-VAL-022,
//! REQ-VAL-121, PIT-3): the draws, the handles, the leaks and what they localise, the exact CoM, the verdict at and past
//! its bound, the refusals, and the runner on both gates' fixtures, where every control fails.

use std::path::{Path, PathBuf};

use kernel::decode::Ic;
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use validation::decode_f32_eps::SplitMix;
use validation::eps_phys::EncodeKind;
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::gauge::Gauge;
use validation::inspector::{
    ball, com, encode_leaky, handle_gaps, handle_samples, held_shapes, latents, radius_refusal,
    round_trip, verdict, worst, z_gap, Handle, HandlesInput, Held, InspectorHandlesGate,
    InspectorRoundtripGate, Leak, RoundtripInput, COM_BOUND, HANDLES, ROUNDTRIP,
};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn handles_input(ics: u32, leak: Leak) -> HandlesInput {
    HandlesInput {
        description: "a test input".to_owned(),
        seed: 3,
        ics,
        radius: 4.0,
        span: 1.0,
        octaves: 4.0,
        leak,
    }
}

fn roundtrip_input(samples: u32, encode: EncodeKind, shift: f64) -> RoundtripInput {
    RoundtripInput {
        description: "a test input".to_owned(),
        seed: 3,
        samples,
        radius: 4.0,
        encode,
        shift,
        shapes: 0,
    }
}

fn config(name: &str) -> GateConfig {
    gate::config(&root(), name).expect("the gate's gate.json")
}

// ── The draws ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Every draw lies in the ball, and the draws reach across it: some component past `0.8·radius` in magnitude, some
/// vector past `0.8·radius` in norm.
fn check_ball(draw: fn(&mut SplitMix, f64) -> [f64; 8]) {
    let mut rng = SplitMix::new(7);
    let z: Vec<[f64; 8]> = (0..4000).map(|_| draw(&mut rng, 4.0)).collect();
    let norm = |v: &[f64; 8]| v.iter().map(|x| x * x).sum::<f64>().sqrt();
    assert!(z.iter().all(|v| norm(v) < 4.0), "a draw outside the ball");
    assert!(
        z.iter().any(|v| norm(v) > 3.2) && z.iter().flatten().any(|x| x.abs() > 3.2),
        "the draws do not reach across the ball"
    );
}

#[test]
fn inspector_ball() {
    check_ball(ball);
}

negative_control!(
    inspector_ball,
    "draws from the whole cube must fail",
    expected = "a draw outside the ball",
    check_ball(|rng, r| std::array::from_fn(|_| validation::gauge::uniform(rng, -r, r)))
);

/// `handle_samples` gives `ics` ICs, the same from one seed and others from another; each handle moves only its own
/// component of the gauge element, at a size in proportion to the IC's own scale, reaching both ends of its range.
fn check_handles(draw: fn(Handle, &mut SplitMix, f64) -> Gauge) {
    let mut rng = SplitMix::new(11);
    let id = Gauge::identity();
    for s0 in [1.0 / 16.0, 1.0, 16.0] {
        for _ in 0..200 {
            let g = draw(Handle::Translate, &mut rng, s0);
            assert!(
                g.a.iter().all(|a| a.abs() < s0) && Gauge { a: id.a, ..g } == id,
                "translate-all's offset out of proportion to the state, or another component moved: {g:?}"
            );
            let g = draw(Handle::Boost, &mut rng, s0);
            assert!(
                g.u.iter().all(|u| u.abs() < 1.0 / s0.sqrt()) && Gauge { u: id.u, ..g } == id,
                "boost-all's velocity out of proportion to the state, or another component moved: {g:?}"
            );
            let g = draw(Handle::Rotate, &mut rng, s0);
            assert!(
                g.theta.abs() <= std::f64::consts::PI && Gauge { theta: 0.0, ..g } == id,
                "rotate-all's turn out of range, or another component moved: {g:?}"
            );
            let g = draw(Handle::Scale, &mut rng, s0);
            assert!(
                g.s.log2().abs() <= 4.0 && Gauge { s: 1.0, ..g } == id,
                "scale-all's factor out of range, or another component moved: {g:?}"
            );
        }
    }
    let spread = |h: Handle, pick: fn(&Gauge) -> f64, lo: f64| {
        let mut rng = SplitMix::new(13);
        let v: Vec<f64> = (0..400).map(|_| pick(&draw(h, &mut rng, 1.0))).collect();
        v.iter().any(|&x| x > lo) && v.iter().any(|&x| x < -lo)
    };
    assert!(
        spread(Handle::Translate, |g| g.a[0], 0.8)
            && spread(Handle::Translate, |g| g.a[1], 0.8)
            && spread(Handle::Boost, |g| g.u[0], 0.8)
            && spread(Handle::Boost, |g| g.u[1], 0.8)
            && spread(Handle::Rotate, |g| g.theta, 2.5)
            && spread(Handle::Scale, |g| g.s.log2(), 3.2),
        "the handles' sizes do not reach across their ranges"
    );
}

#[test]
fn inspector_handles_draw() {
    check_handles(|h, rng, s0| h.draw(rng, s0, 1.0, 4.0));
    let i = handles_input(64, Leak::None);
    let list = handle_samples(&i);
    assert_eq!(list.len(), 64, "the IC count");
    assert_eq!(list, handle_samples(&i), "one seed, one list");
    assert_ne!(
        list,
        handle_samples(&HandlesInput { seed: 4, ..i }),
        "another seed draws another list"
    );
    assert!(
        list.iter()
            .all(|s| (s.ic.m.iter().sum::<f64>() - 1.0).abs() < 1e-15),
        "an IC's masses do not sum to 1"
    );
}

/// Each IC is `g₀·D(z)` with `g₀`'s offset and boost in proportion to its scale `s₀ = √I_in` (`D(z)` is at `I = 1`):
/// the CoM within `span·s₀` and the total momentum within `span·s₀^{−1/2}` on each axis, each reaching both ends, and
/// `s₀` reaching both ends of `2^[−octaves, octaves)`.
fn check_ics(list: &[validation::inspector::HandleSample], span: f64, octaves: f64) {
    let mut seen = Vec::new();
    for s in list {
        let x = s.ic;
        let c = kernel::canon::centre_of_mass(x.m, x.r);
        let centred = x.r.map(|v| [v[0] - c[0], v[1] - c[1]]);
        let s0 = kernel::canon::inertia(x.m, centred).sqrt();
        let total = [0, 1].map(|k| x.p.iter().map(|p| p[k]).sum::<f64>());
        let (a, u) = (c.map(|w| w / s0), total.map(|w| w * s0.sqrt()));
        let slack = span * (1.0 + 1e-9);
        assert!(
            a.iter().all(|w| w.abs() <= slack),
            "the IC's offset out of proportion: {a:?}"
        );
        assert!(
            u.iter().all(|w| w.abs() <= slack),
            "the IC's boost out of proportion: {u:?}"
        );
        assert!(
            s0.log2().abs() <= octaves * (1.0 + 1e-9),
            "the IC's scale {s0} out of range"
        );
        seen.push([a[0], a[1], u[0], u[1], s0.log2() * span / octaves]);
    }
    for k in 0..5 {
        assert!(
            seen.iter().any(|v| v[k] > 0.8 * span) && seen.iter().any(|v| v[k] < -0.8 * span),
            "the ICs' gauge does not reach across its range (component {k})"
        );
    }
}

#[test]
fn inspector_handles_ics() {
    check_ics(&handle_samples(&handles_input(400, Leak::None)), 1.0, 4.0);
}

negative_control!(
    inspector_handles_ics,
    "ICs boosted past their own scale must fail",
    expected = "the IC's boost out of proportion",
    {
        let mut list = handle_samples(&handles_input(400, Leak::None));
        for s in &mut list {
            s.ic = Gauge {
                u: [5.0, 0.0],
                ..Gauge::identity()
            }
            .act(&s.ic);
        }
        check_ics(&list, 1.0, 4.0)
    }
);

negative_control!(
    inspector_handles_draw,
    "an offset that ignores the state's scale must fail",
    expected = "translate-all's offset out of proportion",
    check_handles(|h, rng, s0| match h {
        Handle::Translate => h.draw(rng, 1.0, 1.0, 4.0),
        _ => h.draw(rng, s0, 1.0, 4.0),
    })
);

#[test]
fn inspector_handle_words() {
    let words: Vec<&str> = Handle::ALL.iter().map(|h| h.word()).collect();
    assert_eq!(
        words,
        ["translate-all", "boost-all", "rotate-all", "scale-all"],
        "the handles' names and order"
    );
}

negative_control!(
    inspector_handle_words,
    "a reversed order must fail",
    expected = "the handles' names and order",
    {
        let words: Vec<&str> = Handle::ALL.iter().rev().map(|h| h.word()).collect();
        assert_eq!(
            words,
            ["translate-all", "boost-all", "rotate-all", "scale-all"],
            "the handles' names and order"
        );
    }
);

// ── The norm and the worst ───────────────────────────────────────────────────────────────────────────────────────

/// `z_gap` is the Euclidean norm of the difference: `‖(3, 4, 0, …)‖ = 5`, and 0 on equal vectors.
fn check_gap(gap: fn([f64; 8], [f64; 8]) -> f64) {
    let a = [1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 0.0, -1.0];
    let b = [4.0, 6.0, 0.5, 0.0, 0.0, 0.0, 0.0, -1.0];
    assert_eq!(gap(a, b), 5.0, "‖(3, 4, 0, …)‖ is not 5");
    assert_eq!(gap(a, a), 0.0, "a vector's gap to itself");
}

#[test]
fn inspector_z_gap() {
    check_gap(z_gap);
}

negative_control!(
    inspector_z_gap,
    "the max-abs norm must fail",
    expected = "is not 5",
    check_gap(|a, b| a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max))
);

/// `worst` is the largest value and its first index; a NaN anywhere gives NaN at its index; an empty list `(0, 0)`.
fn check_worst(pick: fn(&[f64]) -> (f64, usize)) {
    assert_eq!(pick(&[1.0, 3.0, 2.0, 3.0]), (3.0, 1), "the first of equals");
    assert_eq!(pick(&[]), (0.0, 0), "an empty list");
    let (w, at) = pick(&[1.0, f64::NAN, 5.0]);
    assert!(w.is_nan() && at == 1, "a NaN dropped: {w} at {at}");
}

#[test]
fn inspector_worst() {
    check_worst(worst);
}

negative_control!(
    inspector_worst,
    "a fold that drops NaN must fail",
    expected = "a NaN dropped",
    check_worst(|v| {
        let w = v.iter().copied().fold(0.0, f64::max);
        (w, v.iter().position(|&x| x == w).unwrap_or(0))
    })
);

/// The verdict passes at the bound itself and fails one ulp past it, and on NaN.
fn check_verdict(judge: fn(f64, f64) -> Verdict) {
    let b = 1.6e-14;
    assert_eq!(judge(b, b), Verdict::Pass, "at the bound");
    assert_eq!(judge(0.0, b), Verdict::Pass, "at 0");
    assert_eq!(
        judge(f64::from_bits(b.to_bits() + 1), b),
        Verdict::Fail,
        "one ulp past the bound"
    );
    assert_eq!(judge(f64::NAN, b), Verdict::Fail, "a NaN");
}

#[test]
fn inspector_verdict() {
    check_verdict(verdict);
}

negative_control!(
    inspector_verdict,
    "a strict verdict must fail at the bound",
    expected = "at the bound",
    check_verdict(|w, b| if w < b { Verdict::Pass } else { Verdict::Fail })
);

// ── The leaks ────────────────────────────────────────────────────────────────────────────────────────────────────

/// With `leak`'s step dropped, `own` moves `z` by order 1 on the typical IC and every other handle only by rounding:
/// the first handle that nudges `z` localises the leak. With no leak, `encode_leaky` is `E` itself, bit for bit.
fn check_localises(leak: Leak, own: Handle) {
    let gaps: Vec<[f64; 4]> = handle_samples(&handles_input(200, leak))
        .iter()
        .map(|s| handle_gaps(s, leak))
        .collect();
    for (k, h) in Handle::ALL.iter().enumerate() {
        let mut v: Vec<f64> = gaps.iter().map(|g| g[k]).collect();
        v.sort_by(f64::total_cmp);
        let median = v[v.len() / 2];
        if *h == own {
            assert!(
                median > 0.1,
                "{leak:?}: its own {} does not move z (median {median:e})",
                h.word()
            );
        } else {
            assert!(
                median < 1e-14,
                "{leak:?}: {} moves z (median {median:e})",
                h.word()
            );
        }
    }
}

#[test]
fn inspector_leaks_localise() {
    check_localises(Leak::Com, Handle::Translate);
    check_localises(Leak::Boost, Handle::Boost);
    check_localises(Leak::Rotate, Handle::Rotate);
    check_localises(Leak::Scale, Handle::Scale);
    for s in handle_samples(&handles_input(64, Leak::None)) {
        assert_eq!(
            encode_leaky(&s.ic, Leak::None),
            encode::<Defaults, f64>(&s.ic).z.z,
            "no leak is not E"
        );
    }
}

negative_control!(
    inspector_leaks_localise,
    "a leak paired with another step's handle must fail",
    expected = "its own translate-all does not move z",
    check_localises(Leak::Boost, Handle::Translate)
);

/// The block inverses of `x` under the defaults.
fn blocks(x: &Ic<f64>) -> [f64; 8] {
    kernel::encode::blocks::invert::<Defaults, f64>(x).z
}

/// `x` with every momentum scaled by `k`.
fn momenta_times(x: &Ic<f64>, k: f64) -> Ic<f64> {
    Ic {
        p: x.p.map(|v| v.map(|w| w * k)),
        ..*x
    }
}

/// Each leak's state, known from a canonical `x = D(z)` moved by a gauge element `g` that `C` undoes: the CoM leak is
/// `x` with its momenta by `(I₀/I_in)^{1/4}`, `I₀ = s² + ‖a‖²` about the origin and `I_in = s²`; the boost leak `x`
/// boosted by the `u` that `g` carries into its own frame; the rotation leak `R(θ)·x`; the scale leak `x` with its
/// momenta by `s^{−1/2}`. Each within `10⁻¹⁰` of `encode`'s, over turns, parities and scales.
fn check_leak_states(leaky: fn(&Ic<f64>, Leak) -> [f64; 8]) {
    let mut rng = SplitMix::new(19);
    let near = |a: [f64; 8], b: [f64; 8], what: &str| {
        let d = z_gap(a, b);
        assert!(d <= 1e-10, "the {what} leak's state is off by {d:e}");
    };
    for k in 0..200 {
        let x = validation::inspector::decode_z(ball(&mut rng, 1.5));
        let theta = validation::gauge::uniform(&mut rng, -3.0, 3.0);
        let s = validation::gauge::uniform(&mut rng, -2.0, 2.0).exp2();
        let a = [validation::gauge::uniform(&mut rng, -1.0, 1.0) * s, 0.7 * s];
        let mirror = k % 2 == 1;
        let id = Gauge::identity();
        let moved = Gauge { a, theta, s, ..id };
        let i0 = s * s + a[0] * a[0] + a[1] * a[1];
        near(
            leaky(&moved.act(&x), Leak::Com),
            blocks(&momenta_times(&x, (i0 / (s * s)).sqrt().sqrt())),
            "Com",
        );
        let u = [0.3, -0.2];
        let boosted = Gauge { u, ..id }.act(&x);
        let y = Gauge {
            theta,
            s,
            mirror,
            ..id
        }
        .act(&boosted);
        near(leaky(&y, Leak::Boost), blocks(&boosted), "Boost");
        let turned = Gauge { theta, ..id }.act(&x);
        near(
            leaky(&Gauge { theta, s, a, ..id }.act(&x), Leak::Rotate),
            blocks(&turned),
            "Rotate",
        );
        near(
            leaky(&Gauge { s, a, ..id }.act(&x), Leak::Scale),
            blocks(&momenta_times(&x, 1.0 / s.sqrt())),
            "Scale",
        );
    }
}

#[test]
fn inspector_leak_states() {
    check_leak_states(encode_leaky);
}

negative_control!(
    inspector_leak_states,
    "a boost leak that leaves the momentum in, unturned, must fail",
    expected = "the Boost leak's state",
    check_leak_states(|x, leak| match leak {
        Leak::Boost => {
            let c = kernel::canon::canonicalise(x);
            let total = [0, 1].map(|k| x.p.iter().map(|p| p[k]).sum::<f64>());
            let mut ic = c.ic;
            for (p, m) in ic.p.iter_mut().zip(x.m) {
                *p = [p[0] + m * total[0], p[1] + m * total[1]];
            }
            blocks(&ic)
        }
        _ => encode_leaky(x, leak),
    })
);

/// Under `E` itself every handle holds `z` within the gate's bound on a small sample, and a leaky `E` does not.
fn check_holds(leak: Leak) {
    let worst_gap = handle_samples(&handles_input(500, leak))
        .iter()
        .flat_map(|s| handle_gaps(s, leak))
        .fold(0.0, f64::max);
    assert!(
        worst_gap <= 1.6e-14,
        "{leak:?}: a handle moves z by {worst_gap:e}"
    );
}

#[test]
fn inspector_handles_hold() {
    check_holds(Leak::None);
}

negative_control!(
    inspector_handles_hold,
    "the scale leak must fail",
    expected = "a handle moves z",
    check_holds(Leak::Scale)
);

// ── The CoM and the round trip ───────────────────────────────────────────────────────────────────────────────────

/// `n/2⁵⁴`, exactly.
fn dyadic(n: i64) -> f64 {
    n as f64 * (-54f64).exp2()
}

/// `com` reads the state's own CoM, not its reading's rounding: on dyadic masses and positions of 54-bit numerators,
/// whose products need 108 bits, against the exact sum in integers, rounded once; and a known CoM, `2⁻⁵⁰` off the
/// origin, exactly.
fn check_com(read: fn(&Ic<f64>) -> f64) {
    let mut rng = SplitMix::new(17);
    let mut num = || (rng.next_u64() >> 11) as i64 | 1 << 52;
    let mut worst_rel = 0.0f64;
    for _ in 0..500 {
        let m: [i64; 3] = std::array::from_fn(|_| num() >> 2);
        let rx: [i64; 3] = std::array::from_fn(|_| num());
        let sign = [1i128, -1, 1];
        let exact: i128 = (0..3).map(|i| sign[i] * m[i] as i128 * rx[i] as i128).sum();
        let x = Ic {
            m: m.map(dyadic),
            r: std::array::from_fn(|i| [sign[i] as f64 * dyadic(rx[i]), 0.0]),
            p: [[0.0; 2]; 3],
        };
        let want = (exact as f64 * (-108f64).exp2()).abs();
        let got = read(&x);
        worst_rel = worst_rel.max((got - want).abs() / want);
    }
    assert!(
        worst_rel <= 1e-25,
        "the CoM read is not the state's own: relative error {worst_rel:e}"
    );
    let known = Ic {
        m: [0.5, 0.25, 0.25],
        r: [[-0.5, 0.0], [0.5, 0.5], [0.5, -0.5]].map(|v| [v[0] + (-50f64).exp2(), v[1]]),
        p: [[0.0; 2]; 3],
    };
    assert_eq!(read(&known), (-50f64).exp2(), "a CoM of 2⁻⁵⁰");
}

#[test]
fn inspector_com() {
    check_com(com);
}

negative_control!(
    inspector_com,
    "a plain sum, each product rounded, must fail",
    expected = "the CoM read is not the state's own",
    check_com(|x| {
        let axis = |k: usize| (0..3).map(|i| x.m[i] * x.r[i][k]).sum::<f64>();
        axis(0).hypot(axis(1))
    })
);

/// The round trip: under the defaults within the gate's bound with the CoM within [`COM_BOUND`]; a shift moves the CoM
/// to `√2·shift` and leaves the round trip's bits alone; the swapped links move `z` by order 1.
fn check_trips(kind: EncodeKind, bound: f64) {
    let i = roundtrip_input(500, kind, 0.0);
    let z = latents(&i);
    assert_eq!(z.len(), 500, "the sample count");
    assert_eq!(z, latents(&i), "one seed, one list");
    for &v in &z {
        let (gap, c) = round_trip(v, kind, 0.0);
        assert!(
            gap <= bound,
            "a round trip of {gap:e} past {bound:e} at {v:?}"
        );
        assert!(c <= COM_BOUND, "a canonical CoM of {c:e} at {v:?}");
        let shift = (-50f64).exp2();
        let (moved, c) = round_trip(v, kind, shift);
        assert_eq!(moved, gap, "the shift moves the round trip");
        assert!(
            (c - std::f64::consts::SQRT_2 * shift).abs() <= 2e-16,
            "a shift of {shift:e} reads a CoM of {c:e}"
        );
    }
}

#[test]
fn inspector_round_trip() {
    check_trips(EncodeKind::Defaults, 1.2e-13);
    let far = latents(&roundtrip_input(64, EncodeKind::Mismatched, 0.0))
        .into_iter()
        .map(|v| round_trip(v, EncodeKind::Mismatched, 0.0).0)
        .fold(f64::INFINITY, f64::min);
    assert!(far > 1e-3, "a mismatched round trip within {far:e}");
}

negative_control!(
    inspector_round_trip,
    "the swapped links must fail",
    expected = "a round trip of",
    check_trips(EncodeKind::Mismatched, 1.2e-13)
);

/// The held families: `n` of each; every collinear state's `z_β` at its clamp image with its bit set, and no
/// equilateral one's; every equilateral `|w − 1|` and collinear `|w|` at rounding (below 10⁻¹²), reported, not judged
/// (RQ-475).
fn check_held(n: u32, held: &Held) {
    let n = n as usize;
    assert!(
        held.pole.len() == n && held.equator.len() == n,
        "the held families' counts"
    );
    assert_eq!(held.beta_bit, [0, n], "z_β's clamp bits by family");
    assert_eq!(held.beta_edge, [0, n], "z_β at its clamp image by family");
    assert!(
        held.pole.iter().chain(&held.equator).all(|&w| w < 1e-12),
        "a held w past rounding"
    );
    let spread = |k: usize| {
        let r: Vec<f64> = held.masses.iter().map(|m| (m[k] / m[0]).ln()).collect();
        r.iter().any(|&x| x > 2.0) && r.iter().any(|&x| x < -2.0)
    };
    assert!(
        held.masses.len() == n
            && held
                .masses
                .iter()
                .all(|m| (m.iter().sum::<f64>() - 1.0).abs() < 1e-15)
            && spread(1)
            && spread(2),
        "the collinear masses do not spread across the mass block"
    );
}

#[test]
fn inspector_held_shapes() {
    check_held(200, &held_shapes(5, 200));
}

negative_control!(
    inspector_held_shapes,
    "a family one short must fail",
    expected = "the held families' counts",
    {
        let mut held = held_shapes(5, 200);
        held.pole.pop();
        check_held(200, &held)
    }
);

// ── The gates ────────────────────────────────────────────────────────────────────────────────────────────────────

/// Each gate refuses an empty input and an input that is not its own, naming why.
fn check_refusals(handles_ics: u32, roundtrip_samples: u32) {
    let h = InspectorHandlesGate::judge(
        &config(HANDLES),
        "t",
        &handles_input(handles_ics, Leak::None),
    );
    assert!(
        h.as_ref().is_err_and(|e| e.contains("no ICs")),
        "an empty handles input is not refused: {:?}",
        h.map(|r| r.verdict)
    );
    let r = InspectorRoundtripGate::judge(
        &config(ROUNDTRIP),
        "t",
        &roundtrip_input(roundtrip_samples, EncodeKind::Defaults, 0.0),
    );
    assert!(
        r.as_ref().is_err_and(|e| e.contains("no samples")),
        "an empty round-trip input is not refused: {:?}",
        r.map(|r| r.verdict)
    );
    let bad = serde_json::json!({ "description": "x", "seed": 1 });
    let got = InspectorHandlesGate.run(&config(HANDLES), "bad.json", &bad);
    assert!(
        got.as_ref()
            .is_err_and(|e| e.contains("is not an inspector-handles input")),
        "a bad handles input is not refused"
    );
    let got = InspectorRoundtripGate.run(&config(ROUNDTRIP), "bad.json", &bad);
    assert!(
        got.as_ref()
            .is_err_and(|e| e.contains("is not an inspector-roundtrip input")),
        "a bad round-trip input is not refused"
    );
}

#[test]
fn inspector_refusals() {
    check_refusals(0, 0);
}

negative_control!(
    inspector_refusals,
    "a non-empty input must not be refused",
    expected = "an empty handles input is not refused",
    check_refusals(1, 0)
);

/// A radius is refused unless finite and positive, `ball` never returning otherwise: 0, −0, a negative, NaN and ±∞
/// are refused, the least positive normal and 4 are not; and both judges refuse a radius of 0 and of NaN before drawing.
fn check_radius(refusal: fn(f64) -> Option<String>) {
    for bad in [0.0, -0.0, -4.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            refusal(bad).is_some_and(|e| e.contains("is not finite and positive")),
            "the radius {bad} is not refused"
        );
    }
    for good in [f64::MIN_POSITIVE, 4.0] {
        assert_eq!(refusal(good), None, "the radius {good} is refused");
    }
    for bad in [0.0, f64::NAN] {
        let h = InspectorHandlesGate::judge(
            &config(HANDLES),
            "t",
            &HandlesInput {
                radius: bad,
                ..handles_input(4, Leak::None)
            },
        );
        let r = InspectorRoundtripGate::judge(
            &config(ROUNDTRIP),
            "t",
            &RoundtripInput {
                radius: bad,
                ..roundtrip_input(4, EncodeKind::Defaults, 0.0)
            },
        );
        assert!(
            h.is_err_and(|e| e.contains("radius")) && r.is_err_and(|e| e.contains("radius")),
            "a gate draws at the radius {bad}"
        );
    }
}

#[test]
fn inspector_radius_refused() {
    check_radius(radius_refusal);
}

negative_control!(
    inspector_radius_refused,
    "a refusal that admits an infinite radius must fail",
    expected = "the radius inf is not refused",
    check_radius(|r| if r > 0.0 { None } else { radius_refusal(r) })
);

/// The judges' verdicts and lines: under `E` a pass naming each handle and the margin; a leak or a shift a fail naming
/// the failing check and the control; held lines only where the input draws shapes.
fn check_reports(tight: f64) {
    let mut c = config(HANDLES);
    c.threshold.value = tight;
    let pass = InspectorHandlesGate::judge(&c, "t", &handles_input(200, Leak::None)).unwrap();
    assert_eq!(pass.verdict, Verdict::Pass, "E fails the handles gate");
    assert_eq!(pass.scatter.len(), 4, "a line per handle");
    for (line, h) in pass.scatter.iter().zip(Handle::ALL) {
        assert!(
            line.starts_with(&format!("{}: ‖Δz‖: max ", h.word()))
                && line.contains("; median ")
                && line.ends_with("over 200 samples"),
            "a handle's scatter line: {line}"
        );
    }
    assert!(
        pass.negative_results
            .iter()
            .all(|l| l.contains("0 of 200 beyond") && l.contains("margin")),
        "a pass's lines: {:?}",
        pass.negative_results
    );
    let fail = InspectorHandlesGate::judge(&c, "t", &handles_input(200, Leak::Rotate)).unwrap();
    assert_eq!(fail.verdict, Verdict::Fail, "a rotation leak passes");
    assert!(
        fail.negative_results[2].starts_with("rotate-all: 200 of 200 beyond")
            && fail
                .negative_results
                .last()
                .unwrap()
                .contains("Rotate step dropped"),
        "a leak's lines: {:?}",
        fail.negative_results
    );
    let rc = config(ROUNDTRIP);
    let mut i = roundtrip_input(200, EncodeKind::Defaults, 0.0);
    let pass = InspectorRoundtripGate::judge(&rc, "t", &i).unwrap();
    assert_eq!(pass.verdict, Verdict::Pass, "E fails the round-trip gate");
    assert_eq!(pass.scatter.len(), 2, "no held lines without shapes");
    i.shapes = 9;
    let held = InspectorRoundtripGate::judge(&rc, "t", &i).unwrap();
    let h = held_shapes(i.seed, 9);
    // An odd count, so a count of the inexact cannot pass for the exact one.
    let exact = |v: &[f64]| v.iter().filter(|&&x| x == 0.0).count();
    let counts = |k: usize| {
        format!(
            "clamp bit set in {}, at its clamp image in {}",
            h.beta_bit[k], h.beta_edge[k]
        )
    };
    assert!(
        exact(&h.pole) > 0
            && held.scatter[2].contains(&format!("w = 1 exactly in {} of 9", exact(&h.pole)))
            && held.scatter[2].ends_with(&counts(0))
            && held.scatter[3].contains(&format!("w = 0 exactly in {} of 9", exact(&h.equator)))
            && held.scatter[3].ends_with(&counts(1)),
        "the held counts: {:?}",
        held.scatter
    );
    assert!(
        held.scatter.len() == 4
            && held.scatter[2..]
                .iter()
                .all(|l| l.starts_with("held (RQ-475")),
        "the held lines: {:?}",
        held.scatter
    );
    i.shapes = 0;
    i.shift = (-50f64).exp2();
    let shifted = InspectorRoundtripGate::judge(&rc, "t", &i).unwrap();
    assert_eq!(shifted.verdict, Verdict::Fail, "a shifted CoM passes");
    assert!(
        shifted.negative_results[0].contains("0 of 200 beyond")
            && shifted.negative_results[1].contains("200 of 200 beyond")
            && shifted
                .negative_results
                .last()
                .unwrap()
                .contains("the CoM check must fail"),
        "a shift's lines: {:?}",
        shifted.negative_results
    );
    i.shift = 0.0;
    i.encode = EncodeKind::Mismatched;
    let swapped = InspectorRoundtripGate::judge(&rc, "t", &i).unwrap();
    assert_eq!(swapped.verdict, Verdict::Fail, "the swapped links pass");
    assert!(
        swapped
            .negative_results
            .last()
            .unwrap()
            .contains("every link of the defaults swapped"),
        "a swap's lines: {:?}",
        swapped.negative_results
    );
}

#[test]
fn inspector_reports() {
    check_reports(1.6e-14);
}

negative_control!(
    inspector_reports,
    "a threshold at rounding's own scale must fail E",
    expected = "E fails the handles gate",
    check_reports(1e-16)
);

/// The runner on `root`'s fixtures gives every input of both gates its expected outcome.
fn check_fixtures(root: &Path) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("inspector_gates");
    for name in [HANDLES, ROUNDTRIP] {
        let result = gate::run(root, name, &out);
        assert!(
            result.is_ok(),
            "{name}'s fixtures do not give their expected outcomes: {result:?}"
        );
    }
}

#[test]
fn inspector_fixtures() {
    check_fixtures(&root());
}

negative_control!(
    inspector_fixtures,
    "a leak's fixture expected to pass must fail the run",
    expected = "fixtures do not give their expected outcomes",
    {
        let copy = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("inspector_copy");
        std::fs::create_dir_all(copy.join("plan")).unwrap();
        std::fs::copy(
            root().join("plan/requirements.yaml"),
            copy.join("plan/requirements.yaml"),
        )
        .unwrap();
        for name in [HANDLES, ROUNDTRIP] {
            let dir = copy.join("fixtures/gates").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            for entry in std::fs::read_dir(root().join("fixtures/gates").join(name)).unwrap() {
                let path = entry.unwrap().path();
                std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
            }
        }
        let leak = copy
            .join("fixtures/gates")
            .join(HANDLES)
            .join("leak_scale.json");
        let text = std::fs::read_to_string(&leak)
            .unwrap()
            .replace(r#""expected": "fail""#, r#""expected": "pass""#);
        std::fs::write(&leak, text).unwrap();
        check_fixtures(&copy)
    }
);
