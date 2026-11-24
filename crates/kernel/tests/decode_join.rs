//! The decoder's free path joined (`kernel::decode`; dd_decoder §2, §3.4, §5), each check at the f64 and the f32
//! instantiation natively:
//! - `decode_factorised`: the entry calls mass → config → momentum → derived invariants, each stage reading its own
//!   `z` slice and the stages before it, and no block's output moves with a later block's controls.
//! - `decode_block_layout`: `z` with one block non-zero moves only that block's physical outputs.
//! - `decode_momentum`: `Σpᵢ = 0` on fuzzed `z` with unequal masses, the particle momenta conjugate to `ρ` and `λ`;
//!   `z_q = 0` gives rest; `|qₖ| ≤ q_max` (PIT-9).
//! - `decode_no_mass_unweighting`: an unequal-mass fixture matches dd_decoder §3.4's formula, with no unweighting.
//! - `decode_block_independence`: decoder test 2, each block's controls perturbed alone.
//! - `decode_ranges`: decoder test 3, the closed ranges at f32 and f64, every registered link.
//! - `golden_ic`: `z = 0` decodes to the golden IC, every value against `fixtures/golden_ic/z0.json` (the
//!   offset link `β = z_β + c` keeps the golden IC at `z = 0`).
//! - `chart_landmarks_lagrange_euler`: Lagrange's equilateral triangle at `z = 0`, Euler's collinear states at
//!   `z_β = −π/2` (`β = 0`, body 2 on the `+x` side) and `z_β = π/2` (`β = π̂`, on the `−x` side), interior points of
//!   the periodic `β` reached exactly.
//!
//! **The proposed tolerance** (unconfirmed until the M2 gate): dd_decoder test 1's `Σpᵢ = 0` holds to
//! [`SUM_P_EPS`] = 4 `eps`, componentwise, relative to `Σᵢ‖pᵢ‖`, at both precisions (`eps` = 2.2×10⁻¹⁶ at f64, 1.2×10⁻⁷
//! at f32), over every `z` of the domain, saturation included, every registered link. The evidence, 200 runs at the
//! fixed seeds `PROPTEST_RNG_SEED` = 1 … 200 (the fixed grid in each, and 256 fuzzed `z` drawn with unequal masses), as
//! the range of each run's worst residual and the worst after 50, 100 and 200 runs:
//!
//! | quantity | f64: range, worst at 50/100/200 | f32: range, worst at 50/100/200 | proposed |
//! |---|---|---|---|
//! | \|Σpᵢ\|/Σ‖pᵢ‖ | 0.30–0.65; 0.65/0.65/0.65 | 0.25–0.48; 0.46/0.48/0.48 | 4 |
//! | conjugacy: μ_ρ(p₁/m₁ − p₀/m₀) − p_ρ | 0.57–1.35; 1.28/1.28/1.35 | 0.20–0.38; 0.37/0.37/0.38 | 4, not calibrated |
//! | conjugacy: p₀ + p₁ + p_λ | 0.55–0.89; 0.89/0.89/0.89 | 0.43–0.87; 0.87/0.87/0.87 | 4, not calibrated |
//!
//! Re-measured by `for s in $(seq 1 200); do PROPTEST_RNG_SEED=$s cargo test -p kernel --test decode_join --
//! decode_momentum --nocapture; done`.

mod decode_support;

use decode_support::*;
use kernel::decode::config::Config;
use kernel::decode::mass::Masses;
use kernel::decode::momentum::{self, Momentum};
use kernel::decode::{self, Blocks, Decoded, Free, Ic, Latent};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::*;
use kernel::payload::ICDescriptorOf;
use validation::negative_control;

// ── decode_factorised ───────────────────────────────────────────────────────────────────────────────────────────

/// A stage of the decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Mass,
    Config,
    Momentum,
    Invariants,
}

/// The free decoder under the defaults, recording each stage's call and inputs.
struct Recorder<R: P> {
    inner: Free<Defaults>,
    calls: Vec<Stage>,
    z_mass: Option<[R; 2]>,
    z_cfg: Option<[R; 2]>,
    z_mom: Option<[R; 4]>,
    config_masses: Option<Masses<R>>,
    momentum_masses: Option<Masses<R>>,
    momentum_config: Option<Config<R>>,
    invariants_ic: Option<Ic<R>>,
}

impl<R: P> Recorder<R> {
    fn new() -> Self {
        Recorder {
            inner: Free::new(),
            calls: Vec::new(),
            z_mass: None,
            z_cfg: None,
            z_mom: None,
            config_masses: None,
            momentum_masses: None,
            momentum_config: None,
            invariants_ic: None,
        }
    }
}

impl<R: P> Blocks<R> for Recorder<R> {
    fn mass(&mut self, z_mass: [R; 2]) -> Masses<R> {
        self.calls.push(Stage::Mass);
        self.z_mass = Some(z_mass);
        Blocks::<R>::mass(&mut self.inner, z_mass)
    }
    fn config(&mut self, z_cfg: [R; 2], masses: &Masses<R>) -> Config<R> {
        self.calls.push(Stage::Config);
        self.z_cfg = Some(z_cfg);
        self.config_masses = Some(*masses);
        self.inner.config(z_cfg, masses)
    }
    fn momentum(&mut self, z_mom: [R; 4], masses: &Masses<R>, config: &Config<R>) -> Momentum<R> {
        self.calls.push(Stage::Momentum);
        self.z_mom = Some(z_mom);
        self.momentum_masses = Some(*masses);
        self.momentum_config = Some(*config);
        self.inner.momentum(z_mom, masses, config)
    }
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        self.calls.push(Stage::Invariants);
        self.invariants_ic = Some(*ic);
        self.inner.invariants(ic)
    }
}

/// An entry through the recorder.
type Recorded<R> = fn(&mut Recorder<R>, &Latent<R>) -> Decoded<R>;

/// The entry under test.
fn entry<R: P>(b: &mut Recorder<R>, z: &Latent<R>) -> Decoded<R> {
    decode::decode_with(b, z)
}

/// Checks, at `z`, that `run` calls the stages in the order mass → config → momentum → invariants, each once, each
/// block reading its own slice (`z[6:8]`, `z[0:2]`, `z[2:6]`) and the outputs of the stages before it, and the
/// invariants the joined `(m, r, p)`; and that no block's output moves when a later block's controls do.
fn check_factorised<R: P>(run: Recorded<R>, z: [f64; 8]) {
    let zl = latent::<R>(z);
    let mut b = Recorder::new();
    let out = run(&mut b, &zl);
    assert_eq!(
        b.calls,
        [
            Stage::Mass,
            Stage::Config,
            Stage::Momentum,
            Stage::Invariants
        ],
        "the decode order is not mass → config → momentum → invariants at {}",
        R::NAME
    );
    assert_eq!(
        b.z_mass,
        Some(zl.mass()),
        "the mass block did not read z[6:8]"
    );
    assert_eq!(
        b.z_cfg,
        Some(zl.config()),
        "the config block did not read z[0:2]"
    );
    assert_eq!(
        b.z_mom,
        Some(zl.momentum()),
        "the momentum block did not read z[2:6]"
    );
    assert_eq!(
        b.config_masses,
        Some(out.masses),
        "the config block did not read the decoded masses"
    );
    assert_eq!(
        (b.momentum_masses, b.momentum_config),
        (Some(out.masses), Some(out.config)),
        "the momentum block did not read the decoded masses and configuration"
    );
    let joined = Ic {
        m: out.masses.m,
        r: out.config.r,
        p: out.momentum.p,
    };
    assert_eq!(
        (b.invariants_ic, out.ic),
        (Some(joined), joined),
        "the invariants did not read the joined (m, r, p)"
    );
    // A block's output does not move with a later block's controls: the masses with z[0:6], the configuration with
    // z[2:6].
    for k in 0..6 {
        let mut moved = z;
        moved[k] = if z[k] == 0.7 { -0.3 } else { 0.7 };
        let mut b2 = Recorder::new();
        let other = run(&mut b2, &latent::<R>(moved));
        assert_eq!(
            other.masses,
            out.masses,
            "the masses moved with z[{k}] at {}",
            R::NAME
        );
        if k >= 2 {
            assert_eq!(
                other.config,
                out.config,
                "the configuration moved with z[{k}] at {}",
                R::NAME
            );
        }
    }
}

fn check_factorised_both(run64: Recorded<f64>, run32: Recorded<f32>) {
    over(any_z(), |z| {
        check_factorised(run64, z);
        check_factorised(run32, z);
    });
}

#[test]
fn decode_factorised() {
    check_factorised_both(entry::<f64>, entry::<f32>);
}

/// An entry that evaluates the invariants first, on the zero state, then the blocks.
#[cfg(feature = "controls")]
fn invariants_first<R: P>(b: &mut Recorder<R>, z: &Latent<R>) -> Decoded<R> {
    let zero = Ic {
        m: [R::zero(); 3],
        r: [[R::zero(); 2]; 3],
        p: [[R::zero(); 2]; 3],
    };
    let _ = b.invariants(&zero);
    decode::decode_with(b, z)
}

negative_control!(
    decode_factorised,
    "an entry that evaluates the invariants before the blocks must fail",
    expected = "the decode order is not mass → config → momentum → invariants",
    check_factorised_both(invariants_first::<f64>, invariants_first::<f32>)
);

/// An entry that hands the config block the momentum block's first two controls.
#[cfg(feature = "controls")]
fn config_from_momentum<R: P>(b: &mut Recorder<R>, z: &Latent<R>) -> Decoded<R> {
    let mut swapped = z.z;
    swapped.swap(0, 2);
    swapped.swap(1, 3);
    decode::decode_with(b, &Latent::new(swapped))
}

mod config_slice {
    use super::*;

    negative_control!(
        decode_factorised,
        "an entry that reads the config block from z[2:4] must fail",
        expected = "the config",
        check_factorised_both(config_from_momentum::<f64>, config_from_momentum::<f32>)
    );
}

// ── decode_block_layout ─────────────────────────────────────────────────────────────────────────────────────────

/// The block a latent index belongs to: config `z[0:2]`, momentum `z[2:6]`, mass `z[6:8]`.
fn block_of(k: usize) -> &'static str {
    match k {
        0 | 1 => "config",
        2..=5 => "momentum",
        _ => "mass",
    }
}

/// Each block's physical outputs: config `(α, β)`, momentum `(p_ρ, p_λ)`, mass `m`, as f64 for comparison.
fn physical<R: P>(d: &Decoded<R>) -> [(&'static str, Vec<f64>); 3] {
    let f = |v: &[R]| v.iter().map(|x| x.f()).collect::<Vec<_>>();
    [
        ("config", f(&[d.config.alpha, d.config.beta])),
        (
            "momentum",
            f(&[
                d.momentum.p_rho[0],
                d.momentum.p_rho[1],
                d.momentum.p_lambda[0],
                d.momentum.p_lambda[1],
            ]),
        ),
        ("mass", f(&d.masses.m)),
    ]
}

/// Checks that `z` with only `z[k]` non-zero moves exactly the physical outputs of `z[k]`'s block from `z = 0`'s.
fn check_layout<R: P>(run: Entry<R>) {
    let base = physical(&run(&latent::<R>([0.0; 8])));
    for k in 0..8 {
        for v in [1.3, -0.6, 40.0] {
            let mut z = [0.0; 8];
            z[k] = v;
            let got = physical(&run(&latent::<R>(z)));
            for ((block, before), (_, after)) in base.iter().zip(&got) {
                if *block == block_of(k) {
                    assert_ne!(
                        before,
                        after,
                        "z[{k}] = {v} does not move the {block} block's outputs at {}",
                        R::NAME
                    );
                } else {
                    assert_eq!(
                        before,
                        after,
                        "z[{k}] = {v} moves the {block} block's outputs at {}",
                        R::NAME
                    );
                }
            }
        }
    }
}

#[test]
fn decode_block_layout() {
    for (_, run) in selections::<f64>() {
        check_layout(run);
    }
    for (_, run) in selections::<f32>() {
        check_layout(run);
    }
}

/// The defaults with the mass controls read from `z[0:2]` and the config controls from `z[6:8]`.
#[cfg(feature = "controls")]
fn mass_first<R: P>(z: &Latent<R>) -> Decoded<R> {
    let mut swapped = z.z;
    swapped.swap(0, 6);
    swapped.swap(1, 7);
    decode::decode::<Defaults, R>(&Latent::new(swapped))
}

negative_control!(
    decode_block_layout,
    "a layout with the mass block at z[0:2] must fail",
    expected = "z[0] = 1.3 does not move the config block's outputs",
    check_layout::<f64>(mass_first::<f64>)
);

// ── decode_momentum ─────────────────────────────────────────────────────────────────────────────────────────────

/// The proposed tolerance: `|Σpᵢ| ≤ SUM_P_EPS · eps · Σᵢ‖pᵢ‖` (the module's header).
const SUM_P_EPS: f64 = 4.0;

/// The conjugacy residuals' bound, in `eps` relative to `‖p_ρ‖ + ‖p_λ‖`: `μ_ρ(p₁/m₁ − p₀/m₀) = p_ρ` and
/// `p₀ + p₁ = −p_λ`, the two relations dd_decoder §3.4's map encodes (a tolerance of the same kind as dd_decoder
/// test 1's identities: a few roundings of the map, the divisions and the reduced mass).
const CONJUGATE_EPS: f64 = 4.0;

/// A momentum block at `R`, behind a function pointer, so a control runs the same check on a broken one.
type MomentumBlock<R> = fn([R; 4], &Masses<R>) -> Momentum<R>;

/// The momentum block under the defaults.
fn block<R: P>() -> MomentumBlock<R> {
    momentum::decode::<Defaults, R>
}

/// Checks `D_mom` = `mom` at `z`'s masses (both mass links) and momentum controls: `Σpᵢ = 0` within [`SUM_P_EPS`];
/// the momenta conjugate to `ρ` and `λ` within [`CONJUGATE_EPS`]; `p_ρ = (q₀, q₁)`, `p_λ = (q₂, q₃) = p₂` exactly;
/// `|qₖ| ≤ q_max`; and, PIT-9, the masses unequal under `softmax_tanh`.
fn check_momentum<R: P>(mom: MomentumBlock<R>, z: [f64; 8], worst: &Worst) {
    // `softmax_tanh` gives unequal inner masses wherever `z_μ1 ≠ 0`; `stick_breaking`'s corner can give equal ones.
    for (unequal, masses) in [
        (
            true,
            kernel::decode::mass::decode::<Defaults, R>(at::<R, 2>([z[6], z[7]])),
        ),
        (
            false,
            kernel::decode::mass::decode::<Stick, R>(at::<R, 2>([z[6], z[7]])),
        ),
    ] {
        let [m0, m1, m2] = masses.m.map(|m| m.f());
        assert!(
            !unequal || m0 != m1,
            "the fuzz drew equal inner masses at z = {z:?} (PIT-9)"
        );
        let out = mom(at::<R, 4>([z[2], z[3], z[4], z[5]]), &masses);
        let q = out.q.map(|x| x.f());
        let q_max = <f64 as ChartConstants>::Q_MAX;
        assert!(
            q.iter().all(|x| x.abs() <= q_max),
            "|q| > q_max: {q:?} at {}",
            R::NAME
        );
        assert_eq!(
            (out.p_rho, out.p_lambda, out.p[2]),
            (
                [out.q[0], out.q[1]],
                [out.q[2], out.q[3]],
                [out.q[2], out.q[3]]
            ),
            "p_ρ ≠ (q₀, q₁), or p_λ ≠ (q₂, q₃) = p₂ at {}",
            R::NAME
        );
        let p = out.p.map(|v| v.map(|x| x.f()));
        let scale: f64 = p.iter().map(|v| v[0].hypot(v[1])).sum();
        for ((a, b), d) in p[0].iter().zip(p[1]).zip(p[2]) {
            let sum = a + b + d;
            let err = if scale == 0.0 {
                sum.abs()
            } else {
                sum.abs() / scale / R::eps()
            };
            worst.see(&format!("|Σp|/Σ‖p‖ at {}", R::NAME), err);
            assert!(
                err <= SUM_P_EPS,
                "Σp = {sum} ({err} eps of Σ‖p‖ = {scale}) at m = {:?}, q = {q:?}, {}",
                [m0, m1, m2],
                R::NAME
            );
        }
        let (p_rho, p_lambda) = ([q[0], q[1]], [q[2], q[3]]);
        let jacobi = p_rho[0].hypot(p_rho[1]) + p_lambda[0].hypot(p_lambda[1]);
        let mu_rho = m0 * m1 / (m0 + m1);
        for c in 0..2 {
            let rel = mu_rho * (p[1][c] / m1 - p[0][c] / m0) - p_rho[c];
            let com = p[0][c] + p[1][c] + p_lambda[c];
            for (what, r) in [("μ_ρ(p₁/m₁ − p₀/m₀) − p_ρ", rel), ("p₀ + p₁ + p_λ", com)]
            {
                let err = if jacobi == 0.0 {
                    r.abs()
                } else {
                    r.abs() / jacobi / R::eps()
                };
                worst.see(&format!("{what} at {}", R::NAME), err);
                assert!(
                    err <= CONJUGATE_EPS,
                    "{what} = {r} ({err} eps) at m = {:?}, q = {q:?}, {}: the momenta are not conjugate to ρ and λ",
                    [m0, m1, m2],
                    R::NAME
                );
            }
        }
    }
}

/// `z_q = 0` gives rest, every `pᵢ` exactly 0, at every mass.
fn check_rest<R: P>(mom: MomentumBlock<R>) {
    for zm in FIXED {
        let masses = kernel::decode::mass::decode::<Defaults, R>(at::<R, 2>([zm, -zm]));
        let out = mom([R::zero(); 4], &masses);
        assert!(
            out.p.iter().flatten().all(|x| x.f() == 0.0),
            "z_q = 0 does not give rest: p = {:?} at {}",
            out.p,
            R::NAME
        );
    }
}

fn check_momentum_both(m64: MomentumBlock<f64>, m32: MomentumBlock<f32>) {
    let worst = Worst::default();
    check_rest(m64);
    check_rest(m32);
    let fixed = [
        [0.0, 0.0, 1.0, -2.0, 0.5, 3.0, 1.3, -0.4],
        [0.0, 0.0, -1e38, 1e38, 1e38, -1e38, -40.0, 40.0],
        [0.0, 0.0, 8.0, 0.0, -8.0, 0.0, 0.3, 0.0],
    ];
    for z in fixed {
        check_momentum(m64, z, &worst);
        check_momentum(m32, z, &worst);
    }
    validation::prop::run(&unequal_z(), |z| {
        check_momentum(m64, z, &worst);
        check_momentum(m32, z, &worst);
        Ok(())
    });
    worst.print("decode_momentum");
}

#[test]
fn decode_momentum() {
    check_momentum_both(block::<f64>(), block::<f32>());
}

/// The map with one factor swapped: `p₁` takes `m₀/M₀₁`, as `p₀` does. At equal masses it is the right map (PIT-9).
#[cfg(feature = "controls")]
fn one_factor<R: P>(z: [R; 4], masses: &Masses<R>) -> Momentum<R> {
    let mut out = momentum::decode::<Defaults, R>(z, masses);
    let w = masses.m[0] / masses.m01;
    out.p[1] = [
        out.p_rho[0] - w * out.p_lambda[0],
        out.p_rho[1] - w * out.p_lambda[1],
    ];
    out
}

negative_control!(
    decode_momentum,
    "p₁ taking m₀/M₀₁ breaks Σpᵢ = 0 at unequal masses",
    expected = "Σp = ",
    check_momentum_both(one_factor::<f64>, one_factor::<f32>)
);

/// The map with both factors swapped, as the positions take them: `p₀` takes `m₁/M₀₁`, `p₁` takes `m₀/M₀₁`. `Σpᵢ` is
/// still 0, so only the conjugacy check catches it.
#[cfg(feature = "controls")]
fn positions_factors<R: P>(z: [R; 4], masses: &Masses<R>) -> Momentum<R> {
    let mut out = momentum::decode::<Defaults, R>(z, masses);
    let (w0, w1) = (masses.m[1] / masses.m01, masses.m[0] / masses.m01);
    out.p[0] = [
        -out.p_rho[0] - w0 * out.p_lambda[0],
        -out.p_rho[1] - w0 * out.p_lambda[1],
    ];
    out.p[1] = [
        out.p_rho[0] - w1 * out.p_lambda[0],
        out.p_rho[1] - w1 * out.p_lambda[1],
    ];
    out
}

mod swapped_factors {
    use super::*;

    negative_control!(
        decode_momentum,
        "the positions' factors on the momenta keep Σpᵢ = 0 but break conjugacy",
        expected = "the momenta are not conjugate to ρ and λ",
        check_momentum_both(positions_factors::<f64>, positions_factors::<f32>)
    );
}

// ── decode_no_mass_unweighting ──────────────────────────────────────────────────────────────────────────────────

/// The fixture's tolerance on each particle momentum, in `eps` relative to `‖p_ρ‖ + ‖p_λ‖` (a tolerance of the same
/// kind as dd_decoder test 1's identities: the link's and the map's round-off, and `z`'s own rounding to `R`).
const FIXTURE_EPS: f64 = 8.0;

/// The fixture: the Burrau masses `(3, 4, 5)`, body 0 the reference, so `m = (5, 4, 3)/12`, and four momentum controls.
const BURRAU: (f64, f64, f64) = (3.0, 4.0, 5.0);
const Z_Q: [f64; 4] = [0.3, -1.2, 2.0, -0.7];

/// Checks the momentum block `mom` against dd_decoder §3.4's formula at the fixture, computed here in f64 from the
/// exact masses and `qₖ = q_max·tanh(z_qk/2)` (`= q_max·(2σ(z_qk) − 1)`), with no unweighting.
fn check_fixture<R: P>(mom: MomentumBlock<R>) {
    let (a, b, c) = BURRAU;
    let mu_max = <f64 as ChartConstants>::MU_MAX;
    let q_max = <f64 as ChartConstants>::Q_MAX;
    let zm = [f64::ln(b / c) / mu_max, f64::ln(a / c) / mu_max].map(f64::atanh);
    let masses = kernel::decode::mass::decode::<Defaults, R>(at::<R, 2>(zm));
    let m = [c, b, a].map(|x| x / (a + b + c));
    let q = Z_Q.map(|z| q_max * (z / 2.0).tanh());
    let (p_rho, p_lambda) = ([q[0], q[1]], [q[2], q[3]]);
    let m01 = m[0] + m[1];
    let want = [
        [
            -p_rho[0] - m[0] / m01 * p_lambda[0],
            -p_rho[1] - m[0] / m01 * p_lambda[1],
        ],
        [
            p_rho[0] - m[1] / m01 * p_lambda[0],
            p_rho[1] - m[1] / m01 * p_lambda[1],
        ],
        p_lambda,
    ];
    let scale = p_rho[0].hypot(p_rho[1]) + p_lambda[0].hypot(p_lambda[1]);
    let got = mom(at::<R, 4>(Z_Q), &masses).p;
    for i in 0..3 {
        for k in 0..2 {
            let err = (got[i][k].f() - want[i][k]).abs() / scale / R::eps();
            assert!(
                err <= FIXTURE_EPS,
                "p{i}[{k}] = {} at {}, not dd_decoder §3.4's {} ({err} eps)",
                got[i][k].f(),
                R::NAME,
                want[i][k]
            );
        }
    }
}

#[test]
fn decode_no_mass_unweighting() {
    check_fixture(block::<f64>());
    check_fixture(block::<f32>());
}

/// A block that unweights the Jacobi momenta by the reduced masses, `p_ρ√μ_ρ` and `p_λ√μ_λ`, before the map: the step
/// dd_decoder §3.4's formula does not take.
#[cfg(feature = "controls")]
fn unweighted<R: P>(z: [R; 4], masses: &Masses<R>) -> Momentum<R> {
    let q = momentum::controls::<Defaults, R>(z);
    let [m0, m1, m2] = masses.m;
    let (s_rho, s_lambda) = ((m0 * m1 / masses.m01).sqrt(), (m2 * masses.m01).sqrt());
    let p_rho = [q[0] * s_rho, q[1] * s_rho];
    let p_lambda = [q[2] * s_lambda, q[3] * s_lambda];
    Momentum {
        q,
        p_rho,
        p_lambda,
        p: momentum::to_particles(masses, p_rho, p_lambda),
    }
}

negative_control!(
    decode_no_mass_unweighting,
    "a momentum block that unweights by √μ must fail the fixture",
    expected = "not dd_decoder §3.4's",
    check_fixture(unweighted::<f64>)
);

// ── decode_block_independence ───────────────────────────────────────────────────────────────────────────────────

/// Each block's intermediate values, compared bit for bit: mass `(m, M₀₁)`, config `(α, β, ρ̃, λ̃)`, momentum
/// `(q, p_ρ, p_λ)`.
fn intermediates<R: P>(d: &Decoded<R>) -> [(&'static str, Vec<u64>); 3] {
    let b = |v: &[R]| v.iter().map(|x| x.bits()).collect::<Vec<_>>();
    let c = &d.config;
    let m = &d.momentum;
    [
        (
            "mass",
            b(&[d.masses.m[0], d.masses.m[1], d.masses.m[2], d.masses.m01]),
        ),
        (
            "config",
            b(&[
                c.alpha,
                c.beta,
                c.rho_t[0],
                c.rho_t[1],
                c.lambda_t[0],
                c.lambda_t[1],
            ]),
        ),
        (
            "momentum",
            b(&[
                m.q[0],
                m.q[1],
                m.q[2],
                m.q[3],
                m.p_rho[0],
                m.p_rho[1],
                m.p_lambda[0],
                m.p_lambda[1],
            ]),
        ),
    ]
}

/// The slice of each block in `z`.
const SLICES: [(&str, std::ops::Range<usize>); 3] =
    [("mass", 6..8), ("config", 0..2), ("momentum", 2..6)];

/// Checks that perturbing one block's controls, `z` against `z` with that block's slice from `other`, leaves the other
/// blocks' intermediate values bit-identical.
fn check_independence<R: P>(run: Entry<R>, z: [f64; 8], other: [f64; 8]) {
    let base = intermediates(&run(&latent::<R>(z)));
    for (perturbed, slice) in SLICES.iter().cloned() {
        let mut moved = z;
        moved[slice.clone()].copy_from_slice(&other[slice]);
        let got = intermediates(&run(&latent::<R>(moved)));
        for ((block, before), (_, after)) in base.iter().zip(&got) {
            if *block != perturbed {
                assert_eq!(
                    before,
                    after,
                    "perturbing the {perturbed} block moved the {block} block at z = {z:?}, {}",
                    R::NAME
                );
            }
        }
    }
}

fn check_independence_all(runs64: &[Entry<f64>], runs32: &[Entry<f32>]) {
    let pairs = (any_z(), any_z());
    let check = |(z, other): ([f64; 8], [f64; 8])| {
        runs64
            .iter()
            .for_each(|run| check_independence(*run, z, other));
        runs32
            .iter()
            .for_each(|run| check_independence(*run, z, other));
    };
    for z in fixed_z() {
        check((z, [0.9, -1.1, 0.4, -0.2, 1e3, -40.0, 1.3, -0.8]));
    }
    validation::prop::run(&pairs, |zs| {
        check(zs);
        Ok(())
    });
}

#[test]
fn decode_block_independence() {
    let r64 = selections::<f64>().map(|(_, r)| r);
    let r32 = selections::<f32>().map(|(_, r)| r);
    check_independence_all(&r64, &r32);
}

/// The defaults with the momentum controls scaled by `3m₀`: the mass block coupled into the momentum block.
#[cfg(feature = "controls")]
struct Coupled;

#[cfg(feature = "controls")]
impl<R: P> Blocks<R> for Coupled {
    fn mass(&mut self, z: [R; 2]) -> Masses<R> {
        kernel::decode::mass::decode::<Defaults, R>(z)
    }
    fn config(&mut self, z: [R; 2], masses: &Masses<R>) -> Config<R> {
        kernel::decode::config::decode::<Defaults, R>(z, masses)
    }
    fn momentum(&mut self, z: [R; 4], masses: &Masses<R>, _: &Config<R>) -> Momentum<R> {
        let three = R::one() + R::one() + R::one();
        let mut out = momentum::decode::<Defaults, R>(z, masses);
        out.p_rho = out.p_rho.map(|x| x * three * masses.m[0]);
        out
    }
    fn invariants(&mut self, ic: &Ic<R>) -> ICDescriptorOf<R> {
        kernel::decode::descriptor::describe(ic)
    }
}

#[cfg(feature = "controls")]
fn coupled<R: P>(z: &Latent<R>) -> Decoded<R> {
    decode::decode_with(&mut Coupled, z)
}

negative_control!(
    decode_block_independence,
    "a momentum block that reads the masses into p_ρ must fail",
    expected = "perturbing the mass block moved the momentum block",
    check_independence_all(&[coupled::<f64>], &[coupled::<f32>])
);

// ── decode_ranges ───────────────────────────────────────────────────────────────────────────────────────────────

/// Checks decoder test 3 at `z` for the entry `run`: `α ∈ [0, π/2]`, compared in f64 at the exact bound
/// (`FRAC_PI_2` is the greatest f64 at or below it, so `≤` against it is `≤` against the exact bound); `β = z_β + c`
/// bit for bit at `R`, `c` the offset ([`offset`]), the periodic link having no range to close
/// (dd_decoder §3.2); `|qₖ| ≤ q_max`; every mass in the open simplex `(0, 1)`.
fn check_ranges<R: P>(name: &str, run: Entry<R>, z: [f64; 8]) {
    let zl = latent::<R>(z);
    let d = run(&zl);
    let (alpha, beta) = (d.config.alpha.f(), d.config.beta.f());
    let at = format!("{name} at z = {z:?}, {}", R::NAME);
    assert!(
        (0.0..=std::f64::consts::FRAC_PI_2).contains(&alpha),
        "α = {alpha} is outside [0, π/2]: {at}"
    );
    let want = (zl.z[1] + offset::<R>()).f();
    assert!(
        beta.to_bits() == want.to_bits(),
        "β = {beta} is not z_β + c = {want}: {at}"
    );
    let q_max = <f64 as ChartConstants>::Q_MAX;
    for q in d.momentum.q {
        assert!(q.f().abs() <= q_max, "|q| = {} > q_max: {at}", q.f().abs());
    }
    for m in d.masses.m {
        assert!(
            m.f() > 0.0 && m.f() < 1.0,
            "m = {} is outside the open simplex: {at}",
            m.f()
        );
    }
}

/// Past the underflow threshold the lower `α`-pole is reached exactly, `β = z_β + c` far out as near,
/// and saturated `qₖ` reach `±q_max`.
fn check_saturation<R: P>(name: &str, run: Entry<R>) {
    let q_max = <f64 as ChartConstants>::Q_MAX;
    for far in [1e30, 1e38] {
        let d = run(&latent::<R>([-far, -far, far, -far, far, -far, 0.0, 0.0]));
        assert_eq!(
            (d.config.alpha.f(), d.config.beta.f()),
            (0.0, (R::of(-far) + offset::<R>()).f()),
            "{name}: α past the underflow threshold is not 0, or β not z_β + c, at {}",
            R::NAME
        );
        assert_eq!(
            d.momentum.q.map(|q| q.f()),
            [q_max, -q_max, q_max, -q_max],
            "{name}: saturated q do not reach ±q_max at {}",
            R::NAME
        );
    }
}

fn check_ranges_all(runs64: &[(&str, Entry<f64>)], runs32: &[(&str, Entry<f32>)]) {
    for (name, run) in runs64 {
        check_saturation(name, *run);
    }
    for (name, run) in runs32 {
        check_saturation(name, *run);
    }
    over(any_z(), |z| {
        for (name, run) in runs64 {
            check_ranges(name, *run, z);
        }
        for (name, run) in runs32 {
            check_ranges(name, *run, z);
        }
    });
}

#[test]
fn decode_ranges() {
    check_ranges_all(&selections::<f64>(), &selections::<f32>());
}

/// `β = π̂·σ(z)` with `π̂` the f32 nearest to π, `0x4049_0FDB`, above π: a round-to-nearest constant, the
/// control's defect.
#[cfg(feature = "controls")]
struct NearestPi;

#[cfg(feature = "controls")]
impl Link<1, 1> for NearestPi {
    const NAME: &'static str = "nearest_pi";
    type Codomain = Beta;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        let pi: R = num_traits_cast(f32::from_bits(0x4049_0FDB) as f64);
        [pi / (R::one() + (-v[0]).exp())]
    }
    fn inverse<R: LinkReal>(_: [R; 1]) -> [R; 1] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 1]) -> R {
        unreachable!()
    }
}

/// `x` at `R`.
#[cfg(feature = "controls")]
fn num_traits_cast<R: LinkReal>(x: f64) -> R {
    <R as spirv_std::num_traits::NumCast>::from(x).expect("finite")
}

#[cfg(feature = "controls")]
type WithNearestPi =
    Select<SoftmaxTanh, SigmoidAlpha, NearestPi, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

negative_control!(
    decode_ranges,
    "a σ-type β link, β = π̂·σ(z_β), is not the periodic link's offset and must fail",
    expected = "not z_β + c",
    check_ranges_all(
        &[("nearest_pi", decode::decode::<WithNearestPi, f64>)],
        &[("nearest_pi", decode::decode::<WithNearestPi, f32>)]
    )
);

// ── golden_ic ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The golden IC's non-descriptor values' tolerance, in `eps` relative to `max(1, |v|)`, the decoder's unit scale
/// (`I = 1`): a tolerance of the same kind as dd_decoder test 1's identities, the few roundings of the blocks from
/// `z = 0`.
const GOLDEN_EPS: f64 = 4.0;

/// Each fixture value but the descriptor's, read from `d`, by the fixture's name.
fn golden_value<R: P>(d: &Decoded<R>, name: &str) -> Option<f64> {
    let c = &d.config;
    let ic = &d.ic;
    let v2 = |v: [R; 2], axis: &str| if axis == "x" { v[0] } else { v[1] };
    let dist = |i: usize, j: usize| {
        let (a, b) = (ic.r[i].map(|x| x.f()), ic.r[j].map(|x| x.f()));
        (a[0] - b[0]).hypot(a[1] - b[1])
    };
    let f = |x: R| Some(x.f());
    let m = ic.m.map(|x| x.f());
    let r = ic.r.map(|v| v.map(|x| x.f()));
    let p = ic.p.map(|v| v.map(|x| x.f()));
    match name.rsplit_once('_') {
        Some((v, axis)) if ["x", "y"].contains(&axis) => match v {
            "rho_t" => f(v2(c.rho_t, axis)),
            "lambda_t" => f(v2(c.lambda_t, axis)),
            "rho" => f(v2(c.rho, axis)),
            "lambda" => f(v2(c.lambda, axis)),
            "r0" => f(v2(ic.r[0], axis)),
            "r1" => f(v2(ic.r[1], axis)),
            "r2" => f(v2(ic.r[2], axis)),
            "p0" => f(v2(ic.p[0], axis)),
            "p1" => f(v2(ic.p[1], axis)),
            "p2" => f(v2(ic.p[2], axis)),
            "com" => {
                let k = usize::from(axis == "y");
                Some(m[0] * r[0][k] + m[1] * r[1][k] + m[2] * r[2][k])
            }
            "ptot" => {
                let k = usize::from(axis == "y");
                Some(p[0][k] + p[1][k] + p[2][k])
            }
            _ => None,
        },
        _ => match name {
            "m0" | "m1" | "m2" => f(ic.m[name[1..].parse::<usize>().expect("an index")]),
            "M01" => f(d.masses.m01),
            "alpha" => f(c.alpha),
            "beta" => f(c.beta),
            "q0" | "q1" | "q2" | "q3" => f(d.momentum.q[name[1..].parse::<usize>().expect("q")]),
            "I" => Some(
                (0..3)
                    .map(|i| m[i] * (r[i][0].powi(2) + r[i][1].powi(2)))
                    .sum(),
            ),
            "M" => Some(m.iter().sum()),
            "pair_01" => Some(dist(0, 1)),
            "pair_12" => Some(dist(1, 2)),
            "pair_20" => Some(dist(2, 0)),
            _ => None,
        },
    }
}

/// Checks `run`'s decode of the golden `z` ([`golden_z`]) against every value of the fixture at `R`: the descriptor's
/// fields within their proposed ulp tolerances (bit-exact for those 0 at rest), its derived `E₀` as `V_0`, the rest within
/// [`GOLDEN_EPS`].
fn check_golden<R: P>(run: Entry<R>, worst: &Worst) {
    let g = golden();
    let d = run(&golden_z::<R>());
    for v in golden_values(&g) {
        let n = name(&v);
        let want = R::from_fixture(&v);
        let exact = v["decimal"]
            .as_str()
            .expect("a decimal")
            .parse::<f64>()
            .expect("a number");
        if let Some(f) = n.strip_prefix("descriptor.") {
            let got = field(&d.descriptor, f);
            check_field(f, got, want, worst);
        } else if n == "E_0" {
            let got = kernel::decode::descriptor::energy(&d.descriptor);
            check_field("V_0", got, want, worst);
        } else {
            let got = golden_value(&d, n).unwrap_or_else(|| panic!("no golden value `{n}`"));
            let err = (got - want.f()).abs() / exact.abs().max(1.0) / R::eps();
            worst.see(&format!("{} at {}", v["group"], R::NAME), err);
            assert!(
                err <= GOLDEN_EPS,
                "the golden IC's {n} = {got} at {}, not {} ({err} eps)",
                R::NAME,
                want.f()
            );
        }
    }
}

#[test]
fn golden_ic() {
    let worst = Worst::default();
    check_golden(decode::decode::<Defaults, f64>, &worst);
    check_golden(decode::decode::<Defaults, f32>, &worst);
    worst.print("golden_ic");
}

/// `softmax(0, μ₁, μ₂)` with the reference's logit 1, not 0: `z = 0` no longer gives equal masses.
#[cfg(feature = "controls")]
struct ReferenceOne;

#[cfg(feature = "controls")]
impl Link<2, 3> for ReferenceOne {
    const NAME: &'static str = "reference_one";
    type Codomain = Mass;
    fn forward<R: LinkReal>(v: [R; 2]) -> [R; 3] {
        let e = [
            R::one().exp(),
            (R::MU_MAX * v[0].tanh()).exp(),
            (R::MU_MAX * v[1].tanh()).exp(),
        ];
        let s = e[0] + e[1] + e[2];
        [e[0] / s, e[1] / s, e[2] / s]
    }
    fn inverse<R: LinkReal>(_: [R; 3]) -> [R; 2] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 2]) -> R {
        unreachable!()
    }
}

negative_control!(
    golden_ic,
    "a mass link whose reference logit is 1 is not the golden IC",
    expected = "the golden IC's m0",
    check_golden(
        decode::decode::<
            Select<
                ReferenceOne,
                SigmoidAlpha,
                PeriodicBeta,
                SigmoidQ,
                SigmoidQ,
                SigmoidQ,
                SigmoidQ,
            >,
            f64,
        >,
        &Worst::default()
    )
);

// ── chart_landmarks_lagrange_euler ──────────────────────────────────────────────────────────────────────────────

/// `(r₁ − r₀) × (r₂ − r₀)`, twice the triangle's signed area, in f64.
fn area2<R: P>(ic: &Ic<R>) -> f64 {
    let r = ic.r.map(|v| v.map(|x| x.f()));
    let (a, b) = (
        [r[1][0] - r[0][0], r[1][1] - r[0][1]],
        [r[2][0] - r[0][0], r[2][1] - r[0][1]],
    );
    a[0] * b[1] - a[1] * b[0]
}

/// The collinear landmark at `z_β` under `run`: `β` bit for bit `want`, body 2 on the `side` of the inner pair's
/// centre of mass (`+1` for `+x`, `−1` for `−x`), and the triangle's area, relative to `r_min²`, at most `bound`.
fn check_euler<R: P>(run: Entry<R>, z_beta: R, want: R, side: f64, bound: f64) {
    let mut z = latent::<R>([0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.4, -0.2]);
    z.z[1] = z_beta;
    let d = run(&z);
    let at = format!("z_β = {} at {}", z_beta.f(), R::NAME);
    assert!(
        d.config.beta.f().to_bits() == want.f().to_bits(),
        "{at} does not give β = {}: β = {}",
        want.f(),
        d.config.beta.f()
    );
    let r = d.ic.r.map(|v| v.map(|x| x.f()));
    let m = d.ic.m.map(|x| x.f());
    let inner = (m[0] * r[0][0] + m[1] * r[1][0]) / (m[0] + m[1]);
    assert!(
        side * (r[2][0] - inner) > 0.0,
        "{at}: body 2 is not on the {} side: r = {r:?}",
        if side > 0.0 { "+x" } else { "−x" }
    );
    let rmin = d.descriptor.r_min_pair_0.f();
    let flat = area2(&d.ic).abs() / (rmin * rmin);
    assert!(flat <= bound, "{at} is not collinear: area/r_min² = {flat}");
}

/// Checks the landmarks under `run`: `z = 0` is Lagrange's equilateral triangle, its three sides
/// `√3` within [`GOLDEN_EPS`]; `z_β = −c` gives `β = 0` exactly, body 2 on the `+x` side, every body on the x axis,
/// collinear exactly; `z_β = c` gives `β = π̂` exactly (`c + c = π̂`), body 2 on the `−x` side, collinear to `sin π̂`,
/// π's rounding at `R`, its area relative to `r_min²` within [`GOLDEN_EPS`] `eps` of 0; `c` the offset ([`offset`]),
/// so each is an interior point of the periodic `β`, reached with no underflow and no edge.
fn check_landmarks<R: P>(run: Entry<R>) {
    // The collinear states first: a σ-type link, or the offset's nearest f32, the controls, fail there.
    let c = offset::<R>();
    check_euler(run, -c, R::zero(), 1.0, 0.0);
    let mut z = latent::<R>([0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.4, -0.2]);
    z.z[1] = -c;
    let d = run(&z);
    assert!(
        d.ic.r.iter().all(|v| v[1].f() == 0.0),
        "β = 0 is not on the x axis at {}: r = {:?}",
        R::NAME,
        d.ic.r
    );
    check_euler(run, c, R::LIT_7, -1.0, GOLDEN_EPS * R::eps());
    let d = run(&golden_z::<R>());
    let r = d.ic.r.map(|v| v.map(|x| x.f()));
    let side = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    for (i, j) in [(0, 1), (1, 2), (2, 0)] {
        let err = (side(i, j) - 3f64.sqrt()).abs() / 3f64.sqrt() / R::eps();
        assert!(
            err <= GOLDEN_EPS,
            "z = 0 is not equilateral at {}: side {i}{j} = {} ({err} eps from √3)",
            R::NAME,
            side(i, j)
        );
    }
}

#[test]
fn chart_landmarks_lagrange_euler() {
    check_landmarks(decode::decode::<Defaults, f64>);
    check_landmarks(decode::decode::<Defaults, f32>);
}

negative_control!(
    chart_landmarks_lagrange_euler,
    "a σ-type β link, β = π̂·σ(z_β), takes z_β = −π/2 to β > 0, not the collinear β = 0, and must fail",
    expected = "does not give β = 0",
    check_landmarks(decode::decode::<WithNearestPi, f32>)
);

/// The offset link with f32's nearest `π/2`, `0x3FC9_0FDB`, in place of `fl↓(π/2)`: the constant the definition does
/// not take (physics review 5944675263 of PR #560).
#[cfg(feature = "controls")]
struct NearestOffset;

#[cfg(feature = "controls")]
impl Link<1, 1> for NearestOffset {
    const NAME: &'static str = "nearest_offset";
    type Codomain = Beta;
    fn forward<R: LinkReal>(v: [R; 1]) -> [R; 1] {
        [v[0] + num_traits_cast::<R>(f32::from_bits(0x3FC9_0FDB) as f64)]
    }
    fn inverse<R: LinkReal>(_: [R; 1]) -> [R; 1] {
        unreachable!()
    }
    fn log_det<R: LinkReal>(_: [R; 1]) -> R {
        unreachable!()
    }
}

#[cfg(feature = "controls")]
type WithNearestOffset =
    Select<SoftmaxTanh, SigmoidAlpha, NearestOffset, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

mod nearest_offset {
    use super::*;

    negative_control!(
        chart_landmarks_lagrange_euler,
        "the offset with f32's nearest π/2 takes z_β = −fl↓(π/2) to one ulp above 0, not β = 0, and must fail",
        expected = "does not give β = 0",
        check_landmarks(decode::decode::<WithNearestOffset, f32>)
    );
}
