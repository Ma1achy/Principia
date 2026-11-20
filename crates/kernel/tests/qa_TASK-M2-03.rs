//! QA tests for TASK-M2-03 (the free momentum block, the ordered decode entry and the `ICDescriptor`), written from
//! the requirements and dd_decoder §3.2, §3.4, §3.6 and §5, chart_reference §0.3 and R-28, R-417, R-421, not from the
//! implementation. Each check runs at the f64 and the f32 instantiation (R-265), behind function pointers or a
//! `Blocks` implementation, so each negative control (R-176) runs the same check on a deliberately broken decode.
//!
//! - REQ-DEC-001, REQ-DEC-004: the entry calls mass → config → momentum → invariants, once each, each block handed its
//!   own `z` slice and the outputs of the blocks before it, and the invariants the joined `(m, r, p)`.
//! - REQ-DEC-003: a `z` with one block non-zero moves only that block's physical outputs (config α, β; momentum
//!   `p_ρ`, `p_λ`; mass `m`).
//! - REQ-DEC-026: perturbing one block's controls leaves the other blocks' intermediates bit-identical (fuzzed).
//! - REQ-DEC-019, REQ-DEC-030, REQ-DEC-045: `qₖ = q_max(2σ(z) − 1)` with `q_max = 2` (R-10), `p_ρ = (q₀, q₁)`,
//!   `p_λ = (q₂, q₃)`, the Jacobi-to-particle map with no unweighting; `Σpᵢ = 0` within REQ-DEC-045's proposal; the
//!   kinetic energy of the particles equals the Jacobi form `‖p_ρ‖²/2μ_ρ + ‖p_λ‖²/2μ_λ`, which holds only for the
//!   conjugate map and so catches the factor swap `Σpᵢ = 0` cannot (PIT-9: swapping both factors keeps `Σpᵢ = 0`);
//!   `z_q = 0` gives rest; `|qₖ| ≤ q_max`, reached at saturation (R-417).
//! - REQ-DEC-027: decoder test 3 over fuzzed `z` with saturation: α, β compared in f64 at the exact `π/2` and `π`, the
//!   lower poles reached at underflow, `|qₖ| = q_max` at saturation, masses in the open simplex.
//! - REQ-VAL-018, REQ-PAY-034: the golden IC at f64 and f32 against values written here from their closed forms
//!   (exact decimals, each rounded once to the precision, R-421), `K_0` and `virial_ratio` bit-exact, the others within
//!   REQ-PAY-095's proposal; `E₀ = K_0 + V_0`; the struct's members, with no `E₀`; and `fixtures/golden_ic/z0.json`
//!   cross-checked against the same closed forms.
//! - REQ-VAL-020: `z = 0` is equilateral; `z_β` past underflow gives `β = 0` exactly and a collinear IC; `z_β → +∞`
//!   approaches `π` from below, monotone, never past it.
//! - REQ-PAY-088: the six defined fields on ICs out of the canonical frame (rotated, mirrored, unequal masses) against
//!   dd_decoder §3.6's formulas evaluated here in f64; their stated ranges, `rho_angle ∈ (−π, π]` included on the
//!   collinear antiparallel IC; on canonical decodes `rho_mag = cos α`, `lambda_mag = sin α`, `rho_angle = β`.
//!
//! **Tolerances.** REQ-PAY-095 (calibrated; the PR's proposal, pending the M2 gate): 4 ulps from the exact value
//! rounded to the precision. REQ-DEC-045 (calibrated; the PR's proposal, pending the M2 gate): `|Σpᵢ|` per component
//! within `4·eps·Σ‖pᵢ‖`. REQ-DEC-044 (TASK-M2-02's proposal): `I − 1` within 8 eps. The checks of a formula against
//! an f64 evaluation have no calibration requirement: each bound is the round-off of the operations the formula names,
//! derived beside its constant.

use std::fmt::Debug;

use kernel::decode::config::Config;
use kernel::decode::descriptor::{describe, energy};
use kernel::decode::latent::Latent;
use kernel::decode::mass::{self, Masses};
use kernel::decode::momentum::{self, Momentum};
use kernel::decode::{decode, decode_with, Blocks, Decoded, Free, Ic};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::{ICDescriptorOf, PayloadReal};
use proptest::prelude::*;
use validation::negative_control;

// ── Precisions ──────────────────────────────────────────────────────────────────────────────────────────────────

trait Fp: LinkReal + PayloadReal + Debug + Copy + PartialEq + 'static {
    const EPS: f64;
    const MAX: f64;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    /// The value nearest the decimal string `s` (Rust's parse rounds correctly).
    fn parse(s: &str) -> Self;
    fn bits(self) -> u64;
    /// The order-preserving integer of the bits, for a ulp distance.
    fn ordered(self) -> i128;
    fn next_down(self) -> Self;
}

impl Fp for f64 {
    const EPS: f64 = f64::EPSILON;
    const MAX: f64 = f64::MAX;
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn ordered(self) -> i128 {
        let b = self.to_bits() as i64;
        (if b < 0 { i64::MIN - b } else { b }) as i128
    }
    fn next_down(self) -> Self {
        if self > 0.0 {
            f64::from_bits(self.to_bits() - 1)
        } else {
            f64::from_bits(self.to_bits() + 1)
        }
    }
}

impl Fp for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const MAX: f64 = f32::MAX as f64;
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        self as f64
    }
    fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }
    fn bits(self) -> u64 {
        self.to_bits() as u64
    }
    fn ordered(self) -> i128 {
        let b = self.to_bits() as i32;
        (if b < 0 { i32::MIN - b } else { b }) as i128
    }
    fn next_down(self) -> Self {
        if self > 0.0 {
            f32::from_bits(self.to_bits() - 1)
        } else {
            f32::from_bits(self.to_bits() + 1)
        }
    }
}

fn ulps<R: Fp>(a: R, b: R) -> i128 {
    (a.ordered() - b.ordered()).abs()
}

fn at<R: Fp, const N: usize>(z: [f64; N]) -> [R; N] {
    z.map(R::of)
}

// ── Constants from the corpus ───────────────────────────────────────────────────────────────────────────────────

/// `q_max = 2` (R-10).
const Q_MAX: f64 = 2.0;
/// `μ_max = 5` (R-10).
const MU_MAX: f64 = 5.0;
/// REQ-PAY-095's proposal: 4 ulps.
const TOL_ULPS: i128 = 4;
/// REQ-DEC-045's proposal: `|Σpᵢ|` within `4·eps·Σ‖pᵢ‖`.
const TOL_SUM_P: f64 = 4.0;
/// REQ-DEC-044's proposal: `I − 1` within 8 eps.
const TOL_I: f64 = 8.0;

/// Exact decimals of the golden IC's closed forms, to 50 digits.
const THIRD: &str = "0.33333333333333333333333333333333333333333333333333";
const INV_SQRT2: &str = "0.70710678118654752440084436210484903928483593768847";
const SQRT3: &str = "1.7320508075688772935274463415058723669428052538104";
/// `V₀ = −3·(1/9)/√3 = −√3/9`.
const NEG_SQRT3_OVER_9: &str = "-0.19245008972987525483638292683398581854920058375671";
const HALF_PI: &str = "1.5707963267948966192313216916397514420985846996876";
const PI: &str = "3.1415926535897932384626433832795028841971693993751";

/// The largest `R` not above the exact `s` (R-417: the images of `π/2` and `π` are rounded toward zero).
fn toward_zero<R: Fp>(s: &str) -> R {
    let n = R::parse(s);
    // The f64 nearest of π and π/2 lie below them (dd_decoder §3.2), and an f32 rounding error is far above the f64
    // one, so the f64 comparison decides the f32 case.
    if R::EPS > f64::EPSILON && n.f() > s.parse::<f64>().unwrap() {
        n.next_down()
    } else {
        n
    }
}

// ── The domain ──────────────────────────────────────────────────────────────────────────────────────────────────

/// One control: the centre, the transition, saturation, log-uniform magnitudes to the precision's largest value.
fn control(max: f64) -> BoxedStrategy<f64> {
    let top = max.log10();
    prop_oneof![
        -3.0f64..=3.0,
        -40.0f64..=40.0,
        (any::<bool>(), -3.0f64..=top)
            .prop_map(|(n, e)| if n { -1.0 } else { 1.0 } * 10f64.powf(e)),
        Just(max),
        Just(-max),
        Just(0.0),
    ]
    .boxed()
}

fn latent(max: f64) -> BoxedStrategy<[f64; 8]> {
    proptest::array::uniform8(control(max)).boxed()
}

/// A latent whose mass controls give unequal inner masses (`z_μ1 ≠ 0` under softmax_tanh), PIT-9.
fn latent_unequal(max: f64) -> BoxedStrategy<[f64; 8]> {
    (latent(max), prop_oneof![0.05f64..=3.0, -3.0f64..=-0.05])
        .prop_map(|(mut z, m)| {
            z[6] = m;
            z
        })
        .boxed()
}

// ── REQ-DEC-001, REQ-DEC-004: the call order and what each block reads ──────────────────────────────────────────

/// One recorded block call: its name, the `z` slice it was handed, and the bits of the earlier outputs it read.
#[derive(Clone, Debug, PartialEq)]
struct Call {
    name: &'static str,
    z: Vec<u64>,
    reads: Vec<u64>,
}

fn mass_bits<R: Fp>(m: &Masses<R>) -> Vec<u64> {
    m.m.iter().chain([&m.m01]).map(|x| x.bits()).collect()
}
fn config_bits<R: Fp>(c: &Config<R>) -> Vec<u64> {
    let mut v = vec![c.alpha.bits(), c.beta.bits()];
    v.extend(c.r.iter().flatten().map(|x| x.bits()));
    v
}
fn ic_bits<R: Fp>(ic: &Ic<R>) -> Vec<u64> {
    ic.m.iter()
        .chain(ic.r.iter().flatten())
        .chain(ic.p.iter().flatten())
        .map(|x| x.bits())
        .collect()
}

/// The free decoder under the defaults, recording each call.
struct Recorder<R: Fp> {
    inner: Free<Defaults>,
    calls: Vec<Call>,
    _r: std::marker::PhantomData<R>,
}

impl<R: Fp> Blocks<R> for Recorder<R> {
    fn mass(&mut self, z: [R; 2]) -> Masses<R> {
        self.calls.push(Call {
            name: "mass",
            z: z.iter().map(|x| x.bits()).collect(),
            reads: vec![],
        });
        <Free<Defaults> as Blocks<R>>::mass(&mut self.inner, z)
    }
    fn config(&mut self, z: [R; 2], masses: &Masses<R>) -> Config<R> {
        self.calls.push(Call {
            name: "config",
            z: z.iter().map(|x| x.bits()).collect(),
            reads: mass_bits(masses),
        });
        <Free<Defaults> as Blocks<R>>::config(&mut self.inner, z, masses)
    }
    fn momentum(&mut self, z: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R> {
        let mut reads = mass_bits(masses);
        reads.extend(config_bits(config));
        self.calls.push(Call {
            name: "momentum",
            z: z.iter().map(|x| x.bits()).collect(),
            reads,
        });
        <Free<Defaults> as Blocks<R>>::momentum(&mut self.inner, z, masses, config)
    }
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        self.calls.push(Call {
            name: "invariants",
            z: vec![],
            reads: ic_bits(ic),
        });
        <Free<Defaults> as Blocks<R>>::invariants(&mut self.inner, ic)
    }
}

/// The calls the contract requires for `z` and the decode's own outputs `d`: mass on `z[6:8]`, config on `z[0:2]`
/// reading the masses, momentum on `z[2:6]` reading the masses and the configuration, the invariants on the joined
/// `(m, r, p)`.
fn want_calls<R: Fp>(z: &[R; 8], d: &Decoded<R>) -> Vec<Call> {
    let b = |s: &[R]| s.iter().map(|x| x.bits()).collect::<Vec<_>>();
    let mut mom_reads = mass_bits(&d.masses);
    mom_reads.extend(config_bits(&d.config));
    vec![
        Call {
            name: "mass",
            z: b(&z[6..8]),
            reads: vec![],
        },
        Call {
            name: "config",
            z: b(&z[0..2]),
            reads: mass_bits(&d.masses),
        },
        Call {
            name: "momentum",
            z: b(&z[2..6]),
            reads: mom_reads,
        },
        Call {
            name: "invariants",
            z: vec![],
            reads: ic_bits(&Ic {
                m: d.masses.m,
                r: d.config.r,
                p: d.momentum.p,
            }),
        },
    ]
}

fn check_calls<R: Fp>(got: &[Call], z: &[R; 8], d: &Decoded<R>) {
    let names: Vec<_> = got.iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        ["mass", "config", "momentum", "invariants"],
        "the decode order is not mass → config → momentum → invariants at {}",
        R::NAME
    );
    assert_eq!(
        got,
        want_calls(z, d).as_slice(),
        "a block did not read its own z slice and the earlier blocks' outputs at {}",
        R::NAME
    );
    // The output is the join of the blocks', and the descriptor is the one of the joined IC.
    assert_eq!(
        (d.ic.m, d.ic.r, d.ic.p),
        (d.masses.m, d.config.r, d.momentum.p),
        "the decoded (m, r, p) is not the blocks' join at {}",
        R::NAME
    );
    assert_eq!(
        d.descriptor,
        describe(&d.ic),
        "the descriptor is not the joined IC's at {}",
        R::NAME
    );
    assert_eq!(
        d.degenerate, d.masses.degenerate,
        "the mass tag was dropped"
    );
}

fn run_calls<R: Fp>(z: [f64; 8]) -> (Vec<Call>, [R; 8], Decoded<R>) {
    let z = at::<R, 8>(z);
    let mut rec = Recorder::<R> {
        inner: Free::new(),
        calls: vec![],
        _r: std::marker::PhantomData,
    };
    let d = decode_with(&mut rec, &Latent::new(z));
    (rec.calls, z, d)
}

/// Distinct controls in every slot, so a slice handed to the wrong block is seen.
const Z_DISTINCT: [f64; 8] = [0.11, -0.22, 0.33, -0.44, 0.55, -0.66, 0.77, -0.88];

#[test]
fn qa_m2_03_call_order() {
    for z in [
        Z_DISTINCT,
        [0.0; 8],
        [-3.0, 40.0, 1e30, -1e30, 2.0, -2.0, 12.0, -0.3],
    ] {
        let (c, z64, d) = run_calls::<f64>(z);
        check_calls(&c, &z64, &d);
        let (c, z32, d) = run_calls::<f32>(z);
        check_calls(&c, &z32, &d);
    }
}

negative_control!(
    qa_m2_03_call_order,
    "the momentum block recorded before the configuration block",
    expected = "the decode order is not",
    {
        let (mut c, z, d) = run_calls::<f64>(Z_DISTINCT);
        c.swap(1, 2);
        check_calls(&c, &z, &d);
    }
);

mod call_order_slice {
    use super::*;
    negative_control!(
        qa_m2_03_call_order,
        "the configuration block handed the mass slice",
        expected = "did not read its own z slice",
        {
            let (mut c, z, d) = run_calls::<f64>(Z_DISTINCT);
            c[1].z = c[0].z.clone();
            check_calls(&c, &z, &d);
        }
    );
}

// ── REQ-DEC-003: one block at a time ────────────────────────────────────────────────────────────────────────────

type Decode<R> = fn(&Latent<R>) -> Decoded<R>;

/// The physical outputs per block: config `(α, β)`, momentum `(p_ρ, p_λ)`, mass `m`.
fn physical<R: Fp>(d: &Decoded<R>) -> [Vec<u64>; 3] {
    [
        vec![d.config.alpha.bits(), d.config.beta.bits()],
        d.momentum
            .p_rho
            .iter()
            .chain(&d.momentum.p_lambda)
            .map(|x| x.bits())
            .collect(),
        d.masses.m.iter().map(|x| x.bits()).collect(),
    ]
}

const BLOCKS: [(&str, std::ops::Range<usize>); 3] =
    [("config", 0..2), ("momentum", 2..6), ("mass", 6..8)];

fn check_one_block<R: Fp>(dec: Decode<R>) {
    let base = physical(&dec(&Latent::new([R::of(0.0); 8])));
    for (name, slots) in BLOCKS.iter() {
        for v in [0.7, -1.3, 4.0] {
            let mut z = [0.0; 8];
            for (j, s) in slots.clone().enumerate() {
                z[s] = v * (1.0 + 0.25 * j as f64);
            }
            let out = physical(&dec(&Latent::new(at::<R, 8>(z))));
            let moved: Vec<&str> = (0..3)
                .filter(|&b| out[b] != base[b])
                .map(|b| BLOCKS[b].0)
                .collect();
            assert_eq!(
                moved,
                vec![*name],
                "z with only the {name} block set (z = {z:?}) moved {moved:?} at {}",
                R::NAME
            );
        }
    }
}

#[test]
fn qa_m2_03_one_block_at_a_time() {
    check_one_block::<f64>(decode::<Defaults, f64>);
    check_one_block::<f32>(decode::<Defaults, f32>);
}

/// The decode with the layout read wrong: the mass block's controls taken from `z[0:2]`, the config's from `z[6:8]`.
#[cfg(feature = "controls")]
fn swapped_layout<R: Fp>(l: &Latent<R>) -> Decoded<R> {
    let mut z = l.z;
    z.swap(0, 6);
    z.swap(1, 7);
    decode::<Defaults, R>(&Latent::new(z))
}

negative_control!(
    qa_m2_03_one_block_at_a_time,
    "a decode reading the mass controls from z[0:2]",
    expected = "with only the config block set",
    check_one_block::<f64>(swapped_layout::<f64>)
);

// ── REQ-DEC-026: blocks couple only through the join ────────────────────────────────────────────────────────────

/// Each block's intermediates: mass `(m, M₀₁, tag)`, config `(α, β, ρ̃, λ̃)`, momentum `(q, p_ρ, p_λ)`.
fn intermediates<R: Fp>(d: &Decoded<R>) -> [Vec<u64>; 3] {
    let b = |v: &mut dyn Iterator<Item = &R>| v.map(|x| x.bits()).collect::<Vec<_>>();
    let c = &d.config;
    let m = &d.momentum;
    let mut mass = b(&mut d.masses.m.iter().chain([&d.masses.m01]));
    mass.push(d.masses.degenerate.is_some() as u64);
    [
        b(&mut [c.alpha, c.beta].iter().chain(&c.rho_t).chain(&c.lambda_t)),
        b(&mut m.q.iter().chain(&m.p_rho).chain(&m.p_lambda)),
        mass,
    ]
}

fn check_independent<R: Fp>(dec: Decode<R>, z: [f64; 8], other: [f64; 8]) {
    let base = intermediates(&dec(&Latent::new(at::<R, 8>(z))));
    for (k, (name, slots)) in BLOCKS.iter().enumerate() {
        let mut y = z;
        for s in slots.clone() {
            y[s] = other[s];
        }
        let out = intermediates(&dec(&Latent::new(at::<R, 8>(y))));
        for b in (0..3).filter(|&b| b != k) {
            assert_eq!(
                out[b], base[b],
                "perturbing the {name} block changed the {} block's intermediates at {}, z = {z:?} → {y:?}",
                BLOCKS[b].0,
                R::NAME
            );
        }
    }
}

fn check_independence(d64: Decode<f64>, d32: Decode<f32>) {
    validation::prop::run(
        &(latent(f32::MAX as f64), latent(f32::MAX as f64)),
        |(z, o)| {
            check_independent(d64, z, o);
            check_independent(d32, z, o);
            Ok(())
        },
    );
    validation::prop::run(&(latent(f64::MAX), latent(f64::MAX)), |(z, o)| {
        check_independent(d64, z, o);
        Ok(())
    });
}

#[test]
fn qa_m2_03_block_independence() {
    check_independence(decode::<Defaults, f64>, decode::<Defaults, f32>);
}

/// A momentum block that reads the masses into `q` (scaling `p_ρ` by `m₀ + m₁`), coupling momentum to mass.
#[cfg(feature = "controls")]
struct Coupled;

#[cfg(feature = "controls")]
impl<R: Fp> Blocks<R> for Coupled {
    fn mass(&mut self, z: [R; 2]) -> Masses<R> {
        mass::decode::<Defaults, R>(z)
    }
    fn config(&mut self, z: [R; 2], m: &Masses<R>) -> Config<R> {
        <Free<Defaults> as Blocks<R>>::config(&mut Free::<Defaults>::new(), z, m)
    }
    fn momentum(&mut self, z: [R; 4], m: &Masses<R>, _: &Config<R>) -> Momentum<R> {
        let mut out = momentum::decode::<Defaults, R>(z, m);
        out.p_rho = out.p_rho.map(|x| x * m.m[0]);
        out
    }
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        describe(ic)
    }
}

#[cfg(feature = "controls")]
fn coupled<R: Fp>(l: &Latent<R>) -> Decoded<R> {
    decode_with(&mut Coupled, l)
}

negative_control!(
    qa_m2_03_block_independence,
    "a momentum block that reads the masses",
    expected = "perturbing the mass block changed the momentum block",
    check_independent::<f64>(
        coupled::<f64>,
        [0.0, 0.0, 1.0, -0.5, 0.2, 0.3, 0.4, -0.1],
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 1.5]
    )
);

// ── REQ-DEC-019, REQ-DEC-030, REQ-DEC-045: the momentum block ───────────────────────────────────────────────────

type MomBlock<R> = fn([R; 4], &Masses<R>) -> Momentum<R>;

/// `σ(z) = 1/(1 + e^−z)`, in f64.
fn sigma(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// Checks `mom` at the controls `zq` and the masses `masses` against dd_decoder §3.4: the controls, the map, `Σpᵢ = 0`
/// and conjugacy.
fn check_momentum<R: Fp>(mom: MomBlock<R>, zq: [R; 4], masses: &Masses<R>) {
    let out = mom(zq, masses);
    check_q(&out, zq);
    check_map(&out, masses);
    check_sum_p(&out, masses);
    check_conjugate(&out, masses);
}

/// `qₖ = q_max(2σ(z_qk) − 1)`, `|qₖ| ≤ q_max`, `p_ρ = (q₀, q₁)`, `p_λ = (q₂, q₃)`.
fn check_q<R: Fp>(out: &Momentum<R>, zq: [R; 4]) {
    let q = out.q.map(Fp::f);
    for k in 0..4 {
        let want = Q_MAX * (2.0 * sigma(zq[k].f()) - 1.0);
        // σ is exp, an add and a divide (≤ 3 roundings and exp's ulp, relative to σ ≤ 1), doubled and scaled by
        // q_max: within 8 eps of q_max.
        assert!(
            (q[k] - want).abs() <= 8.0 * R::EPS * Q_MAX,
            "q{k} = {} at z = {}, not q_max(2σ(z) − 1) = {want} at {}",
            q[k],
            zq[k].f(),
            R::NAME
        );
        assert!(
            q[k].abs() <= Q_MAX,
            "|q{k}| = {} > q_max at {}",
            q[k],
            R::NAME
        );
    }
    assert_eq!(
        (out.p_rho, out.p_lambda),
        ([out.q[0], out.q[1]], [out.q[2], out.q[3]]),
        "p_ρ is not (q₀, q₁) or p_λ not (q₂, q₃) at {}",
        R::NAME
    );
}

/// `|p_ρ| + |p_λ|`, componentwise, from the block's own `q`.
fn jacobi_scale<R: Fp>(out: &Momentum<R>) -> f64 {
    out.q.iter().map(|x| x.f().abs()).sum()
}

/// The map `p₀ = −p_ρ − (m₀/M₀₁)p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`, `p₂ = p_λ`, from the block's `q`, unweighted (R-28).
fn check_map<R: Fp>(out: &Momentum<R>, masses: &Masses<R>) {
    let q = out.q.map(Fp::f);
    let [m0, m1, m2] = masses.m.map(Fp::f);
    let m01 = m0 + m1;
    let (pr, pl) = ([q[0], q[1]], [q[2], q[3]]);
    let want = [
        [-pr[0] - m0 / m01 * pl[0], -pr[1] - m0 / m01 * pl[1]],
        [pr[0] - m1 / m01 * pl[0], pr[1] - m1 / m01 * pl[1]],
        pl,
    ];
    let p = out.p.map(|v| v.map(Fp::f));
    let jac = jacobi_scale(out);
    for i in 0..3 {
        for c in 0..2 {
            // The map is a divide, a multiply and an add (≤ 1.5 eps of |p_ρ| + |p_λ|) in the block and again in
            // this f64 evaluation: 4 eps.
            assert!(
                (p[i][c] - want[i][c]).abs() <= 4.0 * R::EPS * jac,
                "p{i}[{c}] = {}, not dd_decoder §3.4's {} at m = {:?}, q = {q:?}, {}",
                p[i][c],
                want[i][c],
                [m0, m1, m2],
                R::NAME
            );
        }
    }
}

/// `Σpᵢ = 0` within REQ-DEC-045's proposal.
fn check_sum_p<R: Fp>(out: &Momentum<R>, masses: &Masses<R>) {
    let p = out.p.map(|v| v.map(Fp::f));
    let scale: f64 = p.iter().map(|v| v[0].hypot(v[1])).sum();
    let sum = [0, 1].map(|c| p[0][c] + p[1][c] + p[2][c]);
    for (c, s) in sum.into_iter().enumerate() {
        assert!(
            s.abs() <= TOL_SUM_P * R::EPS * scale,
            "Σp[{c}] = {s}, beyond {TOL_SUM_P} eps of Σ‖p‖ = {scale} at m = {:?}, {}",
            masses.m,
            R::NAME
        );
    }
}

/// The momenta are conjugate to `ρ` and `λ`: `Σ‖pᵢ‖²/2mᵢ = ‖p_ρ‖²/2μ_ρ + ‖p_λ‖²/2μ_λ`, `μ_ρ = m₀m₁/M₀₁`,
/// `μ_λ = m₂M₀₁/(m₂ + M₀₁)`. Each `pᵢ` carries ≤ 2 eps of `|p_ρ| + |p_λ|`, so the particle sum carries
/// ≤ Σ 2‖pᵢ‖·2 eps·jac/2mᵢ; the f64 sums add 8 f64 eps of the totals.
fn check_conjugate<R: Fp>(out: &Momentum<R>, masses: &Masses<R>) {
    let q = out.q.map(Fp::f);
    let m = masses.m.map(Fp::f);
    let p = out.p.map(|v| v.map(Fp::f));
    let (pr, pl) = ([q[0], q[1]], [q[2], q[3]]);
    let m01 = m[0] + m[1];
    let k_part: f64 = (0..3)
        .map(|i| (p[i][0].powi(2) + p[i][1].powi(2)) / (2.0 * m[i]))
        .sum();
    let mu_r = m[0] * m[1] / m01;
    let mu_l = m[2] * m01 / (m[2] + m01);
    let k_jac = (pr[0].powi(2) + pr[1].powi(2)) / (2.0 * mu_r)
        + (pl[0].powi(2) + pl[1].powi(2)) / (2.0 * mu_l);
    let jac = jacobi_scale(out);
    let bound = (0..3)
        .map(|i| 4.0 * R::EPS * p[i][0].hypot(p[i][1]) * jac / m[i])
        .sum::<f64>()
        + 8.0 * f64::EPSILON * (k_part + k_jac);
    assert!(
        (k_part - k_jac).abs() <= bound,
        "Σ‖pᵢ‖²/2mᵢ = {k_part} ≠ ‖p_ρ‖²/2μ_ρ + ‖p_λ‖²/2μ_λ = {k_jac}: the momenta are not conjugate to ρ and λ at \
         m = {m:?}, {}",
        R::NAME
    );
}

/// `z_q = 0` gives rest, every `pᵢ` exactly 0.
fn check_rest<R: Fp>(mom: MomBlock<R>, masses: &Masses<R>) {
    let out = mom([R::of(0.0); 4], masses);
    assert!(
        out.p.iter().flatten().all(|x| x.f() == 0.0),
        "z_q = 0 does not give rest: {:?} at {}",
        out.p,
        R::NAME
    );
}

/// `|qₖ| = q_max` at saturation (R-417), with sign.
fn check_saturation<R: Fp>(mom: MomBlock<R>, masses: &Masses<R>) {
    for z in [1e3, 1e30, R::MAX] {
        let out = mom(at::<R, 4>([z, -z, z, -z]), masses);
        assert_eq!(
            out.q.map(Fp::f),
            [Q_MAX, -Q_MAX, Q_MAX, -Q_MAX],
            "q does not reach ±q_max at z = ±{z:e}, {}",
            R::NAME
        );
    }
}

/// Unequal-mass fixtures: the masses `(c, b, a)/(a + b + c)` of the Burrau triple 3, 4, 5 (dd_decoder §3.1's Burrau
/// point), a skewed decode, and directly set simplex points.
fn mass_points<R: Fp>() -> Vec<Masses<R>> {
    let burrau = [(4.0f64 / 5.0).ln() / MU_MAX, (3.0f64 / 5.0).ln() / MU_MAX].map(f64::atanh);
    let mut v = vec![
        mass::decode::<Defaults, R>(at(burrau)),
        mass::decode::<Defaults, R>(at([2.5, -0.4])),
        mass::decode::<Defaults, R>(at([-1.0, 1.0])),
    ];
    for m in [[0.7, 0.2, 0.1], [0.01, 0.6, 0.39], [1e-4, 0.5, 0.4999]] {
        v.push(mass::from_masses::<R>(at(m)));
    }
    v
}

fn check_momentum_all(m64: MomBlock<f64>, m32: MomBlock<f32>) {
    for (ms64, ms32) in mass_points::<f64>().iter().zip(mass_points::<f32>()) {
        assert_ne!(
            ms64.m[0], ms64.m[1],
            "the fixture's inner masses are equal (PIT-9)"
        );
        check_rest(m64, ms64);
        check_rest(m32, &ms32);
        check_saturation(m64, ms64);
        check_saturation(m32, &ms32);
        for zq in [
            [0.3, -1.2, 2.0, -0.7],
            [5.0, 0.1, -3.0, 9.0],
            [-0.01, 0.02, 1.5, 1.5],
        ] {
            check_momentum(m64, at(zq), ms64);
            check_momentum(m32, at(zq), &ms32);
        }
    }
    validation::prop::run(&latent_unequal(f32::MAX as f64), |z| {
        let ms64 = mass::decode::<Defaults, f64>([z[6], z[7]]);
        let ms32 = mass::decode::<Defaults, f32>(at([z[6], z[7]]));
        assert_ne!(
            ms64.m[0], ms64.m[1],
            "the fuzz drew equal inner masses (PIT-9)"
        );
        check_momentum(m64, [z[2], z[3], z[4], z[5]], &ms64);
        check_momentum(m32, at([z[2], z[3], z[4], z[5]]), &ms32);
        Ok(())
    });
}

#[test]
fn qa_m2_03_momentum() {
    check_momentum_all(
        momentum::decode::<Defaults, f64>,
        momentum::decode::<Defaults, f32>,
    );
}

/// The positions' factors on the momenta, `p₀` taking `m₁/M₀₁` and `p₁` `m₀/M₀₁` (chart_reference §0.3's hazard):
/// `Σpᵢ` stays 0.
#[cfg(feature = "controls")]
fn swapped<R: Fp>(z: [R; 4], m: &Masses<R>) -> Momentum<R> {
    let mut out = momentum::decode::<Defaults, R>(z, m);
    let swap = Masses {
        m: [m.m[1], m.m[0], m.m[2]],
        ..*m
    };
    out.p = momentum::to_particles(&swap, out.p_rho, out.p_lambda);
    out
}

/// One factor wrong, `p₁` taking `m₀/M₀₁` too: `Σpᵢ ≠ 0` at unequal masses.
#[cfg(feature = "controls")]
fn one_sided<R: Fp>(z: [R; 4], m: &Masses<R>) -> Momentum<R> {
    let mut out = momentum::decode::<Defaults, R>(z, m);
    let w = m.m[0] / m.m01;
    out.p[1] = [
        out.p_rho[0] - w * out.p_lambda[0],
        out.p_rho[1] - w * out.p_lambda[1],
    ];
    out
}

/// R-28's forbidden step: `p_ρ` and `p_λ` unweighted by `√μ_ρ`, `√μ_λ` before the map.
#[cfg(feature = "controls")]
fn unweighted<R: Fp>(z: [R; 4], m: &Masses<R>) -> Momentum<R> {
    let mut out = momentum::decode::<Defaults, R>(z, m);
    let [m0, m1, m2] = m.m;
    let (a, b) = ((m0 * m1 / m.m01).sqrt(), (m2 * m.m01).sqrt());
    let (pr, pl) = (out.p_rho.map(|x| x * a), out.p_lambda.map(|x| x * b));
    out.p = momentum::to_particles(m, pr, pl);
    out
}

negative_control!(
    qa_m2_03_momentum,
    "both factors swapped, as the positions take them: Σpᵢ = 0 holds, conjugacy fails",
    expected = "the momenta are not conjugate",
    {
        let m = mass_points::<f64>()[0];
        let out = swapped::<f64>(at([0.3, -1.2, 2.0, -0.7]), &m);
        check_sum_p(&out, &m);
        check_conjugate(&out, &m);
    }
);

mod momentum_one_sided {
    use super::*;
    negative_control!(
        qa_m2_03_momentum,
        "p₁ taking m₀/M₀₁: Σpᵢ ≠ 0 at unequal masses",
        expected = "beyond 4 eps of Σ‖p‖",
        {
            let m = mass_points::<f32>()[0];
            check_sum_p(&one_sided::<f32>(at([0.3, -1.2, 2.0, -0.7]), &m), &m);
        }
    );
}

mod momentum_swapped_map {
    use super::*;
    negative_control!(
        qa_m2_03_momentum,
        "both factors swapped fail the formula",
        expected = "not dd_decoder §3.4's",
        check_momentum_all(swapped::<f64>, swapped::<f32>)
    );
}

mod momentum_unweighted {
    use super::*;
    negative_control!(
        qa_m2_03_momentum,
        "R-28's unweighting step fails the formula",
        expected = "not dd_decoder §3.4's",
        check_momentum_all(unweighted::<f64>, unweighted::<f32>)
    );
}

// ── REQ-DEC-027: ranges ─────────────────────────────────────────────────────────────────────────────────────────

fn check_ranges<R: Fp>(d: &Decoded<R>, z: &[f64; 8]) {
    let (a, b) = (d.config.alpha.f(), d.config.beta.f());
    // Compared in f64 at the exact bounds: the f64 π/2 and π lie below the exact values (dd_decoder §3.2), so an f32
    // value above them is above π/2 and π.
    assert!(
        (0.0..=std::f64::consts::FRAC_PI_2).contains(&a),
        "α = {a} outside [0, π/2] at z = {z:?}, {}",
        R::NAME
    );
    assert!(
        (0.0..=std::f64::consts::PI).contains(&b),
        "β = {b} outside [0, π] at z = {z:?}, {}",
        R::NAME
    );
    for q in d.momentum.q {
        assert!(
            q.f().abs() <= Q_MAX,
            "|q| = {} > q_max at z = {z:?}, {}",
            q.f(),
            R::NAME
        );
    }
    for m in d.masses.m {
        assert!(
            m.f() > 0.0 && m.f() < 1.0,
            "m = {:?} leaves the open simplex at z = {z:?}, {}",
            d.masses.m,
            R::NAME
        );
    }
}

fn check_poles<R: Fp>(dec: Decode<R>) {
    for z in [-1e3, -1e30, -R::MAX] {
        let d = dec(&Latent::new(at([z, z, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])));
        assert_eq!(
            (d.config.alpha.f(), d.config.beta.f()),
            (0.0, 0.0),
            "α = 0 and β = 0 are not reached past underflow at z = {z:e}, {}",
            R::NAME
        );
        let d = dec(&Latent::new(at([-z, -z, -z, z, -z, z, 0.0, 0.0])));
        assert_eq!(
            d.momentum.q.map(Fp::f),
            [Q_MAX, -Q_MAX, Q_MAX, -Q_MAX],
            "|q| = q_max not reached at z = {:e}, {}",
            -z,
            R::NAME
        );
        // The upper images: the generated π/2 and π rounded toward zero, times σ = 1.
        assert_eq!(
            (d.config.alpha, d.config.beta),
            (toward_zero::<R>(HALF_PI), toward_zero::<R>(PI)),
            "α, β at saturation are not the images of π/2 and π rounded toward zero (R-417), {}",
            R::NAME
        );
    }
}

#[test]
fn qa_m2_03_ranges() {
    check_poles::<f64>(decode::<Defaults, f64>);
    check_poles::<f32>(decode::<Defaults, f32>);
    validation::prop::run(&latent(f32::MAX as f64), |z| {
        check_ranges(&decode::<Defaults, f64>(&Latent::new(z)), &z);
        check_ranges(&decode::<Defaults, f32>(&Latent::new(at(z))), &z);
        Ok(())
    });
    validation::prop::run(&latent(f64::MAX), |z| {
        check_ranges(&decode::<Defaults, f64>(&Latent::new(z)), &z);
        Ok(())
    });
}

negative_control!(
    qa_m2_03_ranges,
    "an f32 β at π rounded to nearest, above π",
    expected = "outside [0, π]",
    {
        let mut d = decode::<Defaults, f32>(&Latent::new([0.0; 8]));
        d.config.beta = f32::parse(PI);
        check_ranges(&d, &[0.0; 8]);
    }
);

// ── REQ-VAL-018, REQ-PAY-034: the golden IC ─────────────────────────────────────────────────────────────────────

/// The descriptor's members by name (the ledger's, dd_generation_root §3.6), with the exact value at the golden IC
/// as a decimal, and whether it is 0 at rest.
const GOLDEN: [(&str, &str, bool); 12] = [
    ("m0", THIRD, false),
    ("m1", THIRD, false),
    ("m2", THIRD, false),
    ("q_mass", "1", false),
    ("rho_mag", INV_SQRT2, false),
    ("lambda_mag", INV_SQRT2, false),
    ("rho_ratio", "1", false),
    ("rho_angle", HALF_PI, false),
    ("K_0", "0", true),
    ("V_0", NEG_SQRT3_OVER_9, false),
    ("virial_ratio", "0", true),
    ("r_min_pair_0", SQRT3, false),
];

fn fields<R: Fp>(d: &ICDescriptorOf<R>) -> [R; 12] {
    [
        d.m0,
        d.m1,
        d.m2,
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.K_0,
        d.V_0,
        d.virial_ratio,
        d.r_min_pair_0,
    ]
}

fn check_descriptor<R: Fp>(d: &ICDescriptorOf<R>) {
    for ((name, exact, zero), got) in GOLDEN.iter().zip(fields(d)) {
        let want = R::parse(exact);
        if *zero {
            assert_eq!(
                got.bits(),
                want.bits(),
                "{name} = {got:?} is not bit-exactly 0 at rest (R-421), {}",
                R::NAME
            );
        } else {
            let u = ulps(got, want);
            assert!(
                u <= TOL_ULPS,
                "{name} = {got:?} is {u} ulps from the exact value rounded, {want:?} (REQ-PAY-095: {TOL_ULPS}), {}",
                R::NAME
            );
        }
    }
    // E₀ = K_0 + V_0, derived at read.
    let e0 = energy(d);
    assert_eq!(
        e0.bits(),
        (d.K_0 + d.V_0).bits(),
        "E₀ is not K_0 + V_0 at {}",
        R::NAME
    );
    let u = ulps(e0, R::parse(NEG_SQRT3_OVER_9));
    assert!(
        u <= TOL_ULPS,
        "E₀ = {e0:?} is {u} ulps from −√3/9 at {}",
        R::NAME
    );
}

fn check_golden<R: Fp>(d: &Decoded<R>) {
    let third = R::parse(THIRD);
    for m in d.masses.m {
        assert!(
            ulps(m, third) <= TOL_ULPS,
            "m = {:?} is not (⅓, ⅓, ⅓) at {}",
            d.masses.m,
            R::NAME
        );
    }
    // σ(0) = ½ exactly, so α and β are the halves of the images of π/2 and π (R-417).
    let half = R::of(0.5);
    assert_eq!(
        (d.config.alpha.bits(), d.config.beta.bits()),
        (
            (toward_zero::<R>(HALF_PI) * half).bits(),
            (toward_zero::<R>(PI) * half).bits()
        ),
        "α, β at z = 0 are not π/4, π/2 (the images of π/2 and π, halved) at {}",
        R::NAME
    );
    assert!(
        d.momentum
            .q
            .iter()
            .chain(d.ic.p.iter().flatten())
            .all(|x| x.f() == 0.0),
        "z = 0 is not at rest at {}",
        R::NAME
    );
    let r = d.ic.r.map(|v| v.map(Fp::f));
    let m = d.ic.m.map(Fp::f);
    let i: f64 = (0..3)
        .map(|k| m[k] * (r[k][0].powi(2) + r[k][1].powi(2)))
        .sum();
    assert!(
        (i - 1.0).abs() <= TOL_I * R::EPS,
        "I = {i} ≠ 1 at {}",
        R::NAME
    );
    check_equilateral(&d.ic);
    check_descriptor(&d.descriptor);
}

/// Every pair distance within REQ-PAY-095's `r_min_pair_0` tolerance of √3: the Lagrange equilateral triangle.
fn check_equilateral<R: Fp>(ic: &Ic<R>) {
    let r = ic.r.map(|v| v.map(Fp::f));
    let sqrt3 = R::parse(SQRT3);
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let d = R::of((r[a][0] - r[b][0]).hypot(r[a][1] - r[b][1]));
        assert!(
            ulps(d, sqrt3) <= TOL_ULPS,
            "‖r{a} − r{b}‖ = {d:?}, not √3: z = 0 is not the equilateral triangle at {}",
            R::NAME
        );
    }
}

#[test]
fn qa_m2_03_golden_ic() {
    check_golden(&decode::<Defaults, f64>(&Latent::new([0.0; 8])));
    check_golden(&decode::<Defaults, f32>(&Latent::new([0.0; 8])));
}

negative_control!(
    qa_m2_03_golden_ic,
    "rho_ratio 5 ulps off",
    expected = "rho_ratio = ",
    {
        let mut d = decode::<Defaults, f32>(&Latent::new([0.0; 8]));
        d.descriptor.rho_ratio = f32::from_bits(d.descriptor.rho_ratio.to_bits() + 5);
        check_golden(&d);
    }
);

mod golden_zero {
    use super::*;
    negative_control!(
        qa_m2_03_golden_ic,
        "K_0 written as −0, not bit-exact",
        expected = "is not bit-exactly 0 at rest",
        {
            let mut d = decode::<Defaults, f64>(&Latent::new([0.0; 8]));
            d.descriptor.K_0 = -0.0;
            check_golden(&d);
        }
    );
}

/// The struct's members, by its `Debug` form: §3.6's twelve, the declared padding, no `E₀`; 64 B at f32.
fn check_members(debug: &str, size: usize) {
    let names: Vec<&str> = debug
        .split(['{', ','])
        .skip(1)
        .filter(|s| s.contains(':'))
        .filter_map(|s| s.split(':').next())
        .map(str::trim)
        .collect();
    let mut want: Vec<&str> = GOLDEN.iter().map(|g| g.0).collect();
    want.extend(["_pad", "_tail"]);
    assert_eq!(names, want, "ICDescriptor's members are not the ledger's");
    assert!(
        !names
            .iter()
            .any(|n| n.eq_ignore_ascii_case("e_0") || n.eq_ignore_ascii_case("e0")),
        "ICDescriptor stores E₀"
    );
    assert_eq!(size, 64, "ICDescriptor is not 64 B at f32");
}

#[test]
fn qa_m2_03_descriptor_members() {
    let d = ICDescriptorOf::<f32>::default();
    check_members(
        &format!("{d:?}"),
        std::mem::size_of::<ICDescriptorOf<f32>>(),
    );
}

negative_control!(
    qa_m2_03_descriptor_members,
    "a struct with an E_0 member",
    expected = "are not the ledger's",
    check_members(
        &format!("{:?}", ICDescriptorOf::<f32>::default()).replacen("K_0", "E_0", 1),
        64
    )
);

/// `fixtures/golden_ic/z0.json`'s descriptor values, and `E_0`, against the closed forms here, at both precisions.
fn check_fixture(json: &serde_json::Value) {
    assert!(
        json["z"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v.as_f64() == Some(0.0)),
        "the fixture's z is not 0"
    );
    let values = json["values"].as_array().unwrap();
    let find = |name: &str| {
        values
            .iter()
            .find(|v| v["name"] == name)
            .unwrap_or_else(|| panic!("the fixture has no `{name}`"))
    };
    let hex = |v: &serde_json::Value, k: &str| {
        u64::from_str_radix(v[k].as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
    };
    let mut rows: Vec<(String, &str, bool)> = GOLDEN
        .iter()
        .map(|(n, e, z)| (format!("descriptor.{n}"), *e, *z))
        .collect();
    rows.push(("E_0".into(), NEG_SQRT3_OVER_9, false));
    for (name, exact, zero) in rows {
        let v = find(&name);
        assert_eq!(
            (hex(v, "f64"), hex(v, "f32")),
            (f64::parse(exact).bits(), f32::parse(exact).bits()),
            "the fixture's `{name}` is not its exact value rounded to f64 and f32"
        );
        assert_eq!(
            v["zero_at_rest"], zero,
            "the fixture's `{name}` zero_at_rest is wrong"
        );
    }
}

fn fixture() -> serde_json::Value {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden_ic/z0.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap()
}

#[test]
fn qa_m2_03_golden_fixture() {
    check_fixture(&fixture());
}

negative_control!(
    qa_m2_03_golden_fixture,
    "V_0 rounded one f32 ulp off",
    expected = "is not its exact value rounded",
    {
        let mut j = fixture();
        for v in j["values"].as_array_mut().unwrap() {
            if v["name"] == "descriptor.V_0" {
                v["f32"] = serde_json::json!("0xBE4511A4");
            }
        }
        check_fixture(&j);
    }
);

// ── REQ-VAL-020: the Lagrange and Euler landmarks ───────────────────────────────────────────────────────────────

/// `|ρ × λ|/(‖ρ‖‖λ‖)` from the positions, in f64: 0 on a collinear IC.
fn sine<R: Fp>(ic: &Ic<R>) -> f64 {
    let r = ic.r.map(|v| v.map(Fp::f));
    let m = ic.m.map(Fp::f);
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let c = [
        (m[0] * r[0][0] + m[1] * r[1][0]) / (m[0] + m[1]),
        (m[0] * r[0][1] + m[1] * r[1][1]) / (m[0] + m[1]),
    ];
    let lam = [r[2][0] - c[0], r[2][1] - c[1]];
    (rho[0] * lam[1] - rho[1] * lam[0]).abs() / (rho[0].hypot(rho[1]) * lam[0].hypot(lam[1]))
}

fn check_landmarks<R: Fp>(dec: Decode<R>) {
    check_equilateral(&dec(&Latent::new([R::of(0.0); 8])).ic);
    for za in [-2.0, 0.0, 1.5] {
        for zm in [[0.0, 0.0], [0.8, -1.1]] {
            // β = 0 past underflow: every body on the x axis exactly.
            for zb in [-1e3, -R::MAX] {
                let d = dec(&Latent::new(at([za, zb, 0.0, 0.0, 0.0, 0.0, zm[0], zm[1]])));
                assert_eq!(d.config.beta.f(), 0.0, "β ≠ 0 at z_β = {zb:e}, {}", R::NAME);
                assert!(
                    d.ic.r.iter().all(|v| v[1].f() == 0.0),
                    "z_β = {zb:e} is not collinear: r = {:?} at {}",
                    d.ic.r,
                    R::NAME
                );
            }
            // z_β → +∞: β rises to π from below, the IC approaching collinear.
            let mut prev: Option<f64> = None;
            for zb in [1.0, 2.0, 4.0, 8.0, 16.0, 30.0, 1e3, R::MAX] {
                let d = dec(&Latent::new(at([za, zb, 0.0, 0.0, 0.0, 0.0, zm[0], zm[1]])));
                let b = d.config.beta.f();
                assert!(
                    b <= std::f64::consts::PI,
                    "β = {b} passes π at z_β = {zb:e}, {}",
                    R::NAME
                );
                if let Some(pb) = prev {
                    assert!(
                        b >= pb,
                        "β = {b} falls from {pb} at z_β = {zb:e}: π not approached from below, {}",
                        R::NAME
                    );
                }
                // The IC is as collinear as β says: the positions' sine of the ρ–λ angle is sin β to the rounding
                // of the positions (≤ 16 eps of Σ‖rᵢ‖² over ‖ρ‖‖λ‖).
                let r = d.ic.r.map(|v| v.map(Fp::f));
                let sc: f64 = r.iter().map(|v| v[0].hypot(v[1])).sum();
                let rho = (r[1][0] - r[0][0]).hypot(r[1][1] - r[0][1]);
                let lam = r[2][0].hypot(r[2][1]) / d.masses.m01.f();
                let noise = 16.0 * R::EPS * sc * sc / (rho * lam);
                assert!(
                    sine(&d.ic) <= b.sin() + noise,
                    "the IC at β = {b} is not collinear to sin β: sine {} at z_β = {zb:e}, {}",
                    sine(&d.ic),
                    R::NAME
                );
                prev = Some(b);
            }
            assert_eq!(
                prev.unwrap(),
                toward_zero::<R>(PI).f(),
                "z_β → ∞ does not reach the image of π, the Euler collinear limit, {}",
                R::NAME
            );
        }
    }
}

#[test]
fn qa_m2_03_landmarks() {
    check_landmarks::<f64>(decode::<Defaults, f64>);
    check_landmarks::<f32>(decode::<Defaults, f32>);
}

/// The decode with `β` pushed up by `(π − β)·2`: past π as β → π.
#[cfg(feature = "controls")]
fn past_pi<R: Fp>(l: &Latent<R>) -> Decoded<R> {
    let mut d = decode::<Defaults, R>(l);
    if d.config.beta.f() > 1.0 {
        d.config.beta = R::of(2.0 * std::f64::consts::PI - d.config.beta.f() + 1e-3);
    }
    d
}

negative_control!(
    qa_m2_03_landmarks,
    "β reflected past π",
    expected = "passes π",
    check_landmarks::<f64>(past_pi::<f64>)
);

// ── REQ-PAY-088: the six definitions on ICs out of the canonical frame ──────────────────────────────────────────

/// dd_decoder §3.6's six fields, evaluated in f64 from `(m, r)`.
fn reference(m: [f64; 3], r: [[f64; 2]; 3]) -> [f64; 6] {
    let m01 = m[0] + m[1];
    let rho = [r[1][0] - r[0][0], r[1][1] - r[0][1]];
    let lam = [
        r[2][0] - (m[0] * r[0][0] + m[1] * r[1][0]) / m01,
        r[2][1] - (m[0] * r[0][1] + m[1] * r[1][1]) / m01,
    ];
    let rm = (m[0] * m[1] / m01).sqrt() * rho[0].hypot(rho[1]);
    let lm = (m[2] * m01).sqrt() * lam[0].hypot(lam[1]);
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    [
        m[0].min(m[1]) / m[0].max(m[1]),
        rm,
        lm,
        rm / lm,
        (rho[0] * lam[1] - rho[1] * lam[0]).atan2(rho[0] * lam[0] + rho[1] * lam[1]),
        d(r[0], r[1]).min(d(r[1], r[2])).min(d(r[2], r[0])),
    ]
}

fn six<R: Fp>(d: &ICDescriptorOf<R>) -> [f64; 6] {
    [
        d.q_mass,
        d.rho_mag,
        d.lambda_mag,
        d.rho_ratio,
        d.rho_angle,
        d.r_min_pair_0,
    ]
    .map(Fp::f)
}

const SIX: [&str; 6] = [
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "r_min_pair_0",
];

type Describe<R> = fn(&Ic<R>) -> ICDescriptorOf<R>;

/// Checks `desc` on `(m, r)` at `R` against [`reference`] on the same `R` values. The bound per field is the
/// round-off of the formula's ≤ 8 operations on its inputs' scale, 16 eps of: `q_mass` itself; `rho_mag` √μ_ρ·S;
/// `lambda_mag` √μ_λ·S; `r_min_pair_0` S, with `S = Σ‖rᵢ‖`; `rho_ratio` the ratio's sensitivity,
/// `(√μ_ρ + rho_ratio·√μ_λ)·S/lambda_mag`; `rho_angle` `S/‖ρ‖ + S/‖λ‖`, compared on the circle.
fn check_definitions<R: Fp>(desc: Describe<R>, m: [f64; 3], r: [[f64; 2]; 3]) {
    let ic = Ic {
        m: at(m),
        r: r.map(|v| at(v)),
        p: [[R::of(0.0); 2]; 3],
    };
    let mf = ic.m.map(Fp::f);
    let rf = ic.r.map(|v| v.map(Fp::f));
    let want = reference(mf, rf);
    let got = six(&desc(&ic));
    let s: f64 = rf.iter().map(|v| v[0].hypot(v[1])).sum();
    let m01 = mf[0] + mf[1];
    let (sr, sl) = ((mf[0] * mf[1] / m01).sqrt(), (mf[2] * m01).sqrt());
    let (nr, nl) = (want[1] / sr, want[2] / sl);
    let scale = [
        want[0],
        sr * s,
        sl * s,
        (sr + want[3] * sl) * s / want[2],
        s / nr + s / nl,
        s,
    ];
    for k in 0..6 {
        let mut diff = (got[k] - want[k]).abs();
        if k == 4 {
            diff = diff.min(2.0 * std::f64::consts::PI - diff);
        }
        assert!(
            diff <= 16.0 * R::EPS * scale[k],
            "{} = {}, not dd_decoder §3.6's {} at m = {mf:?}, r = {rf:?}, {}",
            SIX[k],
            got[k],
            want[k],
            R::NAME
        );
    }
    // The stated ranges (dd_decoder §3.6).
    assert!(
        got[0] > 0.0 && got[0] <= 1.0,
        "q_mass = {} outside (0, 1]",
        got[0]
    );
    assert!(
        got[1] >= 0.0 && got[2] >= 0.0 && got[3] >= 0.0 && got[5] >= 0.0,
        "a magnitude < 0: {got:?}"
    );
    assert!(
        got[4] > -std::f64::consts::PI && got[4] <= std::f64::consts::PI,
        "rho_angle = {} outside (−π, π] at m = {mf:?}, r = {rf:?}, {}",
        got[4],
        R::NAME
    );
}

/// Rotates `r` by `phi`, and mirrors it (`y → −y`) if `mirror`.
fn moved(r: [[f64; 2]; 3], phi: f64, mirror: bool) -> [[f64; 2]; 3] {
    let (s, c) = phi.sin_cos();
    r.map(|[x, y]| {
        let y = if mirror { -y } else { y };
        [c * x - s * y, s * x + c * y]
    })
}

fn ic_strategy() -> impl Strategy<Value = ([f64; 3], [[f64; 2]; 3], f64, bool)> {
    (
        [0.02f64..=1.0, 0.02f64..=1.0, 0.02f64..=1.0],
        [
            [-2.0f64..=2.0, -2.0f64..=2.0],
            [-2.0f64..=2.0, -2.0f64..=2.0],
            [-2.0f64..=2.0, -2.0f64..=2.0],
        ],
        -4.0f64..=4.0,
        any::<bool>(),
    )
        .prop_map(|(w, r, phi, mir)| {
            let t: f64 = w.iter().sum();
            (w.map(|x| x / t), r, phi, mir)
        })
        // Off the collision and the α = 0 pole, where the fields are 0, ∞ or 0/0 by definition: every pair and
        // body 2's distance from the inner pair's CoM at least 0.05.
        .prop_filter("a collision or a pole", |(m, r, ..)| {
            let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
            let c = [
                (m[0] * r[0][0] + m[1] * r[1][0]) / (m[0] + m[1]),
                (m[0] * r[0][1] + m[1] * r[1][1]) / (m[0] + m[1]),
            ];
            d(r[0], r[1]) > 0.05
                && d(r[1], r[2]) > 0.05
                && d(r[2], r[0]) > 0.05
                && d(r[2], c) > 0.05
        })
}

fn check_definitions_all(d64: Describe<f64>, d32: Describe<f32>) {
    validation::prop::run(&ic_strategy(), |(m, r, phi, mir)| {
        let r = moved(r, phi, mir);
        check_definitions(d64, m, r);
        check_definitions(d32, m, r);
        Ok(())
    });
    // Rotation and mirror move nothing but rho_angle's sign; q_mass is symmetric in bodies 0 and 1.
    let m = [0.5, 0.2, 0.3];
    let r = [[-0.6, 0.1], [0.9, -0.2], [0.05, 0.8]];
    let base = six(&d64(&Ic {
        m,
        r,
        p: [[0.0; 2]; 3],
    }));
    for (phi, mir) in [(0.7, false), (2.9, true), (-1.3, true)] {
        let got = six(&d64(&Ic {
            m,
            r: moved(r, phi, mir),
            p: [[0.0; 2]; 3],
        }));
        for k in [0, 1, 2, 3, 5] {
            assert!(
                (got[k] - base[k]).abs() <= 16.0 * f64::EPSILON * base[k],
                "{} moved under a rotation by {phi} (mirror {mir}): {} → {}",
                SIX[k],
                base[k],
                got[k]
            );
        }
        let want = if mir { -base[4] } else { base[4] };
        assert!(
            (got[4] - want).abs() <= 16.0 * f64::EPSILON * 8.0,
            "rho_angle {} is not {want} under a rotation by {phi} (mirror {mir})",
            got[4]
        );
    }
    let swapped = d64(&Ic {
        m: [m[1], m[0], m[2]],
        r: [r[1], r[0], r[2]],
        p: [[0.0; 2]; 3],
    });
    assert_eq!(
        swapped.q_mass, base[0],
        "q_mass changes when bodies 0 and 1 swap labels"
    );
}

#[test]
fn qa_m2_03_definitions() {
    check_definitions_all(describe::<f64>, describe::<f32>);
}

/// The collinear antiparallel ICs: every body on the x axis (each `y` written `+0`), `λ` along `−ρ`, so the angle from
/// `ρ` to `λ` is π, which dd_decoder §3.6's range `(−π, π]` includes and `−π` does not; and the same IC mirrored.
fn check_antiparallel<R: Fp>(desc: Describe<R>) {
    let m = [0.25, 0.35, 0.4];
    for r in [
        [[1.0, 0.0], [-1.0, 0.0], [2.0, 0.0]],
        [[1.0, -0.0], [-1.0, -0.0], [2.0, -0.0]],
        [[-1.0, 0.0], [1.0, 0.0], [-2.0, 0.0]],
    ] {
        check_definitions(desc, m, r);
    }
}

#[test]
fn qa_m2_03_rho_angle_range() {
    check_antiparallel::<f64>(describe::<f64>);
    check_antiparallel::<f32>(describe::<f32>);
}

/// A descriptor whose `rho_angle` is reported at `−π` for an angle of π.
#[cfg(feature = "controls")]
fn minus_pi<R: Fp>(ic: &Ic<R>) -> ICDescriptorOf<R> {
    let mut d = describe(ic);
    if d.rho_angle.f().abs() > 3.0 {
        d.rho_angle = R::of(-std::f64::consts::PI);
    }
    d
}

negative_control!(
    qa_m2_03_rho_angle_range,
    "rho_angle reported at −π",
    expected = "outside (−π, π]",
    check_antiparallel::<f64>(minus_pi::<f64>)
);

/// `rho_mag` and `lambda_mag` as the unweighted ‖ρ‖, ‖λ‖, the norms dd_decoder §3.6 does not define.
#[cfg(feature = "controls")]
fn unweighted_norms<R: Fp>(ic: &Ic<R>) -> ICDescriptorOf<R> {
    let mut d = describe(ic);
    let [m0, m1, m2] = ic.m;
    let m01 = m0 + m1;
    d.rho_mag = d.rho_mag / (m0 * m1 / m01).sqrt();
    d.lambda_mag = d.lambda_mag / (m2 * m01).sqrt();
    d
}

negative_control!(
    qa_m2_03_definitions,
    "rho_mag as the unweighted ‖ρ‖",
    expected = "rho_mag = ",
    check_definitions_all(unweighted_norms::<f64>, unweighted_norms::<f32>)
);

/// On canonical decodes: `rho_mag = cos α`, `lambda_mag = sin α`, `rho_angle = β`, `rho_mag² + lambda_mag² = 1`.
fn check_canonical<R: Fp>(d: &Decoded<R>) {
    let (a, b) = (d.config.alpha.f(), d.config.beta.f());
    let [_, rm, lm, _, ang, _] = six(&d.descriptor);
    // Each field is ≤ 8 roundings from the emitted positions, which carry REQ-DEC-044's 8 eps of the unit scale.
    let tol = 32.0 * R::EPS;
    assert!(
        (rm - a.cos()).abs() <= tol,
        "rho_mag = {rm} ≠ cos α = {} at {}",
        a.cos(),
        R::NAME
    );
    assert!(
        (lm - a.sin()).abs() <= tol,
        "lambda_mag = {lm} ≠ sin α = {} at {}",
        a.sin(),
        R::NAME
    );
    assert!(
        (rm * rm + lm * lm - 1.0).abs() <= tol,
        "rho_mag² + lambda_mag² ≠ 1 at {}",
        R::NAME
    );
    if lm > 0.01 && rm > 0.01 {
        assert!(
            (ang - b).abs() <= tol / lm.min(rm),
            "rho_angle = {ang} ≠ β = {b} at {}",
            R::NAME
        );
    }
}

#[test]
fn qa_m2_03_canonical_definitions() {
    validation::prop::run(&latent(40.0), |z| {
        check_canonical(&decode::<Defaults, f64>(&Latent::new(z)));
        check_canonical(&decode::<Defaults, f32>(&Latent::new(at(z))));
        Ok(())
    });
}

negative_control!(
    qa_m2_03_canonical_definitions,
    "rho_mag and lambda_mag exchanged",
    expected = "rho_mag = ",
    {
        let mut d = decode::<Defaults, f64>(&Latent::new([0.9, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        std::mem::swap(&mut d.descriptor.rho_mag, &mut d.descriptor.lambda_mag);
        check_canonical(&d);
    }
);
