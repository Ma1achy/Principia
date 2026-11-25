//! A warped virial axis with an asked bottom `Q₀ > 0` at a held `L_z ≠ 0` puts the floor under the rise only
//! (chart_reference §2.1; `kernel::chart::physical`): `K* = max(Q₀·|U|/2, L_z²/2I) + (Q_hi − Q₀)·t^γ_K·|U|/2`, the
//! pixel's own `U` and `I`, so the bottom row is the asked `Q₀` on a column where `Q₀ ≥ Q_min(U) = L_z²/(I·|U|)` and the
//! floor, the rigid rotation, on one where it is not. Every arm runs at f64 and at f32, the chart built at that
//! precision, on a 65 × 65 grid over `[0,1]²`, every decode read back through the physics recomputed here in f64 from
//! `(m, r, p)` ([`phys`]).
//!
//! `virial_floor_under_rise`, its criteria written before the first run:
//! - **Unchanged arms, bit for bit.** At `Q₀ = 0` and a held `L_z ≠ 0`, the decode is the additive form's,
//!   `K* = floor + Q(t)·|U|/2`; at `Q₀ > 0` and a held `L_z = 0`, it is `K* = Q(t)·|U|/2`. The reference is the additive
//!   rule at every `Q₀`, through `decode_rule` on the chart's own latent point ([`additive`]).
//! - **Under the rise.** At `Q₀ = 1`, `Q_hi = 2.5`, a held `L_z` chosen so that `Q_min(U)` crosses `Q₀` at the column
//!   `u = ½` (`|U|` runs from 0.031 to 0.082 across the window), and the example's below: every pixel validates and is a system; `|K − K*| ≤ (TOL_K + RND)·eps·K*` with `K*` the line above on
//!   the recomputed `U` and `I`; `|L_z − held| ≤ TOL_L·eps·(|held| + √(2K)) + RND·eps·Σ|r ∧ p|`.
//! - **The bottom row.** On a column with `Q₀ ≥ Q_min(U)·(1 + MARGIN)` the pixel's own ratio is `Q₀`,
//!   `|2K/|U| − Q₀| ≤ 2(TOL_K + RND)·eps·Q₀`; on one with `Q_min(U) ≥ Q₀·(1 + MARGIN)` it is the floor:
//!   `|K − L_z²/2I| ≤ (TOL_K + RND)·eps·K` and the kinetic energy beside the rigid rotation within the same. At the
//!   crossing held value, at least 8 columns of each kind, so both arms can fire.
//! - **The example.** Held so that `Q_min(U) = 0.25` at the column `u = ½` (`Q₀ = 1`): the pixel `(½, 0)` reads its own
//!   ratio 1 within `2(TOL_K + RND)·eps`.
//!
//! Control: the additive form decoded on the `Q₀ > 0` axis at the held `L_z ≠ 0`.
//!
//! `TOL_K` and `TOL_L` are the construction's (8 and 4 eps, as `energy_axes.rs`), `RND` this file's re-evaluation's
//! rounding (16 eps, as `qa_TASK-M2-37_floor.rs`). The slice, the ranges and the held values are test data.

use std::fmt::Debug;

use kernel::chart::energy::{decode_rule, Mapping, Rule};
use kernel::chart::invariant::{self, WarpReal};
use kernel::chart::mixed::ZAlpha;
use kernel::chart::physical::{AxisVirial, MixedEnergy};
use kernel::chart::{Chart, ValidationResult};
use kernel::decode::Decoded;
use kernel::generated::links::{Defaults, LinkParams, LinkReal};
use kernel::payload::PayloadReal;
use validation::{negative_control, GAMMA_K};

/// The grid side: 65 points `i/64` per axis, edges included.
const N: u32 = 65;
/// `|K(p) − K*|` in units of `eps·K*`, the construction's.
const TOL_K: f64 = 8.0;
/// `|L_z(p) − L_z|` in units of `eps·(|L_z| + √(2K*))`, the construction's.
const TOL_L: f64 = 4.0;
/// This file's re-evaluation of `K`, `U`, `I` and `L_z` from the state, in eps.
const RND: f64 = 16.0;
/// How far from `Q₀ = Q_min(U)` a column must be to be judged on one side, relative.
const MARGIN: f64 = 1e-3;
/// The slice centre (test data).
const Z0: [f64; 8] = [-0.2, 0.3, 0.1, -0.2, 0.05, 0.15, 0.5, -0.3];
/// `z_α`'s range: wide enough that `U` moves by a factor of several across the window (test data).
const RANGE: [f64; 2] = [-1.5, 1.5];
/// The asked range with a bottom `Q₀ = 1` (test data).
const ASKED: [f64; 2] = [1.0, 2.5];
/// `Q_min(U)` at the column `u = ½` that the held `L_z` is chosen for.
const Q_MIN_MID: f64 = 0.25;

/// The scalar a chart is built and decoded at.
trait Prec: WarpReal + LinkReal + PayloadReal + Debug + Copy + 'static
where
    f64: From<Self>,
{
    const EPS: f64;
    fn of(x: f64) -> Self;
}

impl Prec for f64 {
    const EPS: f64 = f64::EPSILON;
    fn of(x: f64) -> Self {
        x
    }
}

impl Prec for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    fn of(x: f64) -> Self {
        x as f32
    }
}

type Virial<R> = MixedEnergy<ZAlpha, AxisVirial, Defaults, R>;

/// The warped virial axis beside `z_α` over `asked`, at the held `held`.
fn chart<R: Prec>(asked: [f64; 2], held: f64) -> Virial<R>
where
    f64: From<R>,
{
    Virial::<R>::virial(
        Z0.map(R::of),
        RANGE.map(R::of),
        asked.map(R::of),
        R::of(GAMMA_K.number()),
        R::of(held),
        Mapping::Warped,
    )
    .expect("an offered range")
}

/// A decoded system's physics, recomputed in f64 about the centre of mass.
struct Phys {
    u: f64,
    i: f64,
    k: f64,
    lz: f64,
    lz_scale: f64,
    /// The kinetic energy of the velocities' difference from the rigid rotation `ω = L_z/I`.
    non_rigid: f64,
}

fn phys<R: Prec>(d: &Decoded<R>) -> Phys
where
    f64: From<R>,
{
    let m = d.ic.m.map(f64::from);
    let r = d.ic.r.map(|v| v.map(f64::from));
    let p = d.ic.p.map(|v| v.map(f64::from));
    let total: f64 = m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|j| m[j] * r[j][a]).sum::<f64>() / total);
    let rel = |j: usize| [r[j][0] - c[0], r[j][1] - c[1]];
    let sum = |f: &dyn Fn(usize) -> f64| (0..3).map(f).sum::<f64>();
    let i = sum(&|j| m[j] * (rel(j)[0].powi(2) + rel(j)[1].powi(2)));
    let lz = sum(&|j| rel(j)[0] * p[j][1] - rel(j)[1] * p[j][0]);
    let lz_scale = sum(&|j| (rel(j)[0] * p[j][1]).abs() + (rel(j)[1] * p[j][0]).abs());
    let k = sum(&|j| (p[j][0].powi(2) + p[j][1].powi(2)) / (2.0 * m[j]));
    let omega = lz / i;
    let non_rigid = sum(&|j| {
        let dv = [
            p[j][0] / m[j] + omega * rel(j)[1],
            p[j][1] / m[j] - omega * rel(j)[0],
        ];
        m[j] * (dv[0] * dv[0] + dv[1] * dv[1]) / 2.0
    });
    let mut u = 0.0;
    for a in 0..3 {
        for b in (a + 1)..3 {
            let s = ((r[a][0] - r[b][0]).powi(2) + (r[a][1] - r[b][1]).powi(2)).sqrt();
            u -= m[a] * m[b] / s;
        }
    }
    Phys {
        u,
        i,
        k,
        lz,
        lz_scale,
        non_rigid,
    }
}

/// The additive form at every `Q₀`, `K* = floor + Q(t)·|U|/2`, through
/// `decode_rule` on the chart's own latent point, the rule as `MixedEnergy::decode_from` builds it at `Q₀ = 0`.
fn additive<R: Prec>(c: &Virial<R>, u: R, t: R) -> Decoded<R>
where
    f64: From<R>,
{
    let z = c.map(u, t).z;
    let two = R::one() + R::one();
    let rule = Rule {
        k0: invariant::k_floor(c.held) + R::zero(),
        weight: c.scale(t) / two,
        bottom_weight: R::zero(),
        u_ref: R::zero(),
        s2: R::zero(),
        lz: c.held,
        lz_on_warp: false,
    };
    decode_rule::<Defaults, R>(rule, &z, &LinkParams::DEFAULT).0
}

/// The chart's own decode.
fn own<R: Prec>(c: &Virial<R>, u: R, t: R) -> Decoded<R>
where
    f64: From<R>,
{
    c.decode::<Defaults, R>(u, t)
}

fn grid() -> Vec<[f64; 2]> {
    let at = |i: u32| f64::from(i) / f64::from(N - 1);
    (0..N)
        .flat_map(|j| (0..N).map(move |i| [at(i), at(j)]))
        .collect()
}

/// `|U|` at the column `u = ½`, on the bottom row (the configuration does not read the momenta).
fn mid_u<R: Prec>() -> f64
where
    f64: From<R>,
{
    let c = chart::<R>([0.0, 1.0], 0.0);
    phys(&own(&c, R::of(0.5), R::zero())).u.abs()
}

/// The arms at the precision `R` (the module's note), the floor-under-rise arm decoded by `decode`.
fn arms<R: Prec>(decode: fn(&Virial<R>, R, R) -> Decoded<R>)
where
    f64: From<R>,
{
    let name = core::any::type_name::<R>();
    // Unchanged arms, bit for bit.
    for (asked, held) in [
        ([0.0, 2.5], 0.3),
        ([0.0, 2.5], -0.7),
        (ASKED, 0.0),
        ([0.4, 1.5], 0.0),
    ] {
        let c = chart::<R>(asked, held);
        for [u, t] in grid() {
            let (u, t) = (R::of(u), R::of(t));
            let (got, want) = (own(&c, u, t), additive(&c, u, t));
            assert!(
                format!("{got:?}") == format!("{want:?}"),
                "{name}: Q₀ = {}, held {held}, ({u:?}, {t:?}) is not the additive form bit for bit",
                asked[0]
            );
        }
    }
    // Under the rise, held so that Q_min(U) at u = ½ is Q₀ (the columns cross there; I = 1), then the example's 0.25.
    for q_mid in [ASKED[0], Q_MIN_MID] {
        under_rise(decode, (q_mid * mid_u::<R>()).sqrt(), q_mid == ASKED[0]);
    }
}

/// The floor-under-rise arm at the held `held`, decoded by `decode`; `crossing` where the bottom row must hold 8 columns
/// on each side of `Q₀ = Q_min(U)`, and else the example's pixel `(½, 0)` reads 1.
fn under_rise<R: Prec>(decode: fn(&Virial<R>, R, R) -> Decoded<R>, held: f64, crossing: bool)
where
    f64: From<R>,
{
    let eps = R::EPS;
    let name = core::any::type_name::<R>();
    let c = chart::<R>(ASKED, held);
    let held = f64::from(c.held);
    let (q0, hi, g) = (ASKED[0], ASKED[1], GAMMA_K.number());
    let (mut feasible, mut floored, mut worst) = (0, 0, 0.0f64);
    for [u, t] in grid() {
        let d = decode(&c, R::of(u), R::of(t));
        let p = phys(&d);
        let rise = (hi - q0) * f64::from(R::of(t)).powf(g);
        let want = (q0 * p.u.abs() / 2.0).max(held * held / (2.0 * p.i)) + rise * p.u.abs() / 2.0;
        assert!(
            c.validate(u, t) == ValidationResult::Valid && d.degenerate.is_none(),
            "{name}: ({u}, {t}) is not a system"
        );
        worst = worst.max((p.k - want).abs() / (eps * want));
        assert!(
            (p.k - want).abs() <= (TOL_K + RND) * eps * want,
            "{name}: ({u}, {t}) has K = {}, which is not max(Q₀·|U|/2, floor) + rise = {want}",
            p.k
        );
        assert!(
            (p.lz - held).abs()
                <= TOL_L * eps * (held.abs() + (2.0 * p.k).sqrt()) + RND * eps * p.lz_scale,
            "{name}: ({u}, {t}) has L_z = {}, not the held {held}",
            p.lz
        );
        if t != 0.0 {
            continue;
        }
        let q_min = held * held / (p.i * p.u.abs());
        let ratio = 2.0 * p.k / p.u.abs();
        if q0 >= q_min * (1.0 + MARGIN) {
            feasible += 1;
            assert!(
                (ratio - q0).abs() <= 2.0 * (TOL_K + RND) * eps * q0,
                "{name}: the bottom row at u = {u}, Q_min = {q_min} below Q₀, reads {ratio}, not Q₀"
            );
        } else if q_min >= q0 * (1.0 + MARGIN) {
            floored += 1;
            let tol = (TOL_K + RND) * eps * p.k;
            assert!(
                (p.k - held * held / (2.0 * p.i)).abs() <= tol && p.non_rigid <= tol,
                "{name}: the bottom row at u = {u}, Q_min = {q_min} above Q₀, is not the floor: K = {}",
                p.k
            );
        }
    }
    println!(
        "{name}: held {held}: worst |K − K*| = {worst:.2} eps·K* (criterion {})",
        TOL_K + RND
    );
    if crossing {
        assert!(
            feasible >= 8 && floored >= 8,
            "{name}: the bottom row has {feasible} columns where Q₀ is feasible and {floored} on the floor, not 8 of \
             each"
        );
        return;
    }
    // The example: Q₀ = 1 on the column where Q_min(U) = 0.25 reads 1 at t = 0.
    let p = phys(&decode(&c, R::of(0.5), R::zero()));
    let q_min = held * held / (p.i * p.u.abs());
    let ratio = 2.0 * p.k / p.u.abs();
    assert!(
        (q_min - Q_MIN_MID).abs() <= 1e-3 && (ratio - 1.0).abs() <= 2.0 * (TOL_K + RND) * eps,
        "{name}: at Q_min = {q_min} the pixel (½, 0) reads {ratio}, not 1"
    );
}

fn check(control: bool) {
    if control {
        arms::<f64>(additive);
        arms::<f32>(additive);
    } else {
        arms::<f64>(own);
        arms::<f32>(own);
    }
}

#[test]
fn virial_floor_under_rise() {
    check(false);
}

negative_control!(
    virial_floor_under_rise,
    "the additive form on a Q₀ > 0 axis at a held L_z ≠ 0 stands Q₀ on the floor, not on the larger of the two",
    expected = "which is not max(Q₀·|U|/2, floor) + rise",
    check(true)
);

// ── The baked entry ──────────────────────────────────────────────────────────────────────────────────────────────────

type Entry = fn(
    spirv_std::glam::UVec3,
    &kernel::stage::decode::DecodeUniforms,
    &[f32],
    &mut [u32],
    &mut [u32],
    &mut [u32],
);

/// The virial pair's entry, called natively (the GPU's translation is the Mac's to re-emit): its uniform round-trips
/// the chart, warped and linear, and at every point of a 9 × 9 grid, and three invocations past it, the entry writes
/// the stage's `sample` of the chart bit for bit: its `(m, r, p)`, label and descriptor. Criterion written first: exact.
fn check_virial_entry(entry: Entry) {
    use kernel::chart::mixed::Alpha;
    use kernel::stage::decode::{
        descriptor_words, flags, ic_words, read_descriptor, read_ic, sample, untag, DecodeMode,
        DecodeUniforms, DecodeVariant, MixedAlphaVirialDefaults, UniformChart,
    };
    type Pair = <MixedAlphaVirialDefaults as DecodeVariant>::Chart;
    assert_eq!(
        MixedAlphaVirialDefaults::ENTRY,
        "stage_decode_mixed_alpha_virial_defaults"
    );
    for (asked, held, mapping) in [
        (ASKED, 0.15f32, Mapping::Warped),
        ([0.0, 2.0], 0.15, Mapping::Warped),
        ([-0.5, 2.0], 0.2, Mapping::Linear),
    ] {
        let chart: Pair = MixedEnergy::<Alpha, AxisVirial, Defaults, f32>::virial(
            Z0.map(|z| z as f32),
            [0.2, 1.4],
            asked.map(|q| q as f32),
            2.0,
            held,
            mapping,
        )
        .expect("an offered range");
        let rows = chart.uniform();
        assert_eq!(
            Pair::from_uniform(&rows),
            chart,
            "the uniform does not round-trip the chart"
        );
        let uv: Vec<f32> = (0..81)
            .flat_map(|i| [(i % 9) as f32 / 8.0, (i / 9) as f32 / 8.0])
            .collect();
        let n = uv.len() / 2;
        let uniforms = DecodeUniforms {
            chart: rows,
            flags: [flags(DecodeMode::Full), 0, 0, 0].into(),
            energy: Default::default(),
        };
        let mut ic = vec![0; n * ic_words()];
        let mut tags = vec![u32::MAX; n];
        let mut descriptors = vec![0; n * descriptor_words()];
        for i in 0..n as u32 + 3 {
            entry(
                spirv_std::glam::UVec3::new(i, 0, 0),
                &uniforms,
                &uv,
                &mut ic,
                &mut tags,
                &mut descriptors,
            );
        }
        for i in 0..n {
            let want = sample::<Pair, Defaults, f32>(&chart, uv[2 * i], uv[2 * i + 1]);
            let got = (
                read_ic(&ic, i),
                untag(tags[i]),
                read_descriptor(&descriptors, i),
            );
            assert!(
                got.0 == want.ic && got.1 == Ok(want.degenerate) && got.2 == want.descriptor,
                "sample {i}: the entry point's decode is not the virial pair's"
            );
        }
    }
}

#[test]
fn virial_floor_under_rise_entry() {
    check_virial_entry(kernel::stage::decode::mixed_alpha_virial_defaults);
}

negative_control!(
    virial_floor_under_rise_entry,
    "the (z_μ1, α) entry point on the virial pair's uniform",
    expected = "the entry point's decode is not the virial pair's",
    check_virial_entry(kernel::stage::decode::mixed_mu1_alpha_defaults)
);
