//! QA tests for TASK-M2-25's R-628 change, written from the ruling and dd_decoder §3.6, not from the implementation:
//! the descriptor's `rho_angle` is read from the configuration block's own `(ρ̃, λ̃)`, `Φ`'s vectors, through §3.6's
//! cases; the other descriptor fields keep their positional forms; `η_E` moves only the momenta, so it leaves
//! `rho_angle` as it was.
//!
//! - `qa_m225_rho_angle_is_phis_on_every_chart`: on every M2 chart at the harness's held parameters
//!   (`validation::decode_preset::uniforms`), on a 9 × 9 lattice of chart points and the square's centre, at f32 and
//!   f64, the decode stage's own source (`kernel::stage::decode::sample_with`, `η_E` off and toward a target) and the
//!   chart's decode (`ChartDecode::decode_at`) each store as `rho_angle` exactly, bit for bit, §3.6's case value
//!   computed here from the chart decode's configuration block `ρ̃ = config.rho_t`, `λ̃ = config.lambda_t`: on the axis
//!   (`ρ̃ × λ̃ = 0`) `π̂` where `ρ̃ · λ̃ < 0` and 0 otherwise; elsewhere `atan2(ρ̃ × λ̃, ρ̃ · λ̃)`, taken down to `π̂`, a value
//!   at or below `−π̂` read as `π̂`; `π̂` π rounded toward zero at f32 and π's nearest value at f64 (R-417). Every other
//!   descriptor field is the positional form of the stored `(m, r, p)` (`descriptor::describe`), bit for bit. The
//!   η_E-rescaled descriptor's `rho_angle` is the un-rescaled one's, bit for bit.
//! - `qa_m225_rho_angle_at_and_near_the_lambda_pole`: the shape sphere on its exponential map at `(½, ½)`, so the point
//!   is `n₀`: at the exact `λ̃ = 0` pole `n₀ = (+1, ±0, ±0)`, every sign of the zeros, phases `φ_f ∈ {0, 0.3, 2}`, the
//!   configuration's `λ̃` is zero and `rho_angle` is `+0`'s bits ("the value is the constant 0") on the decode stage's
//!   source at f32 and f64, with `η_E` off and on; near it, `n₀ = (+1, ε, 0)` and `(+1, 0, ε)`, both signs, `|ε|` from
//!   `10⁻¹²` to `10⁻⁴`, the f32 `rho_angle` is within REQ-DEC-043's factor (`decode-f32-eps`'s gate threshold, the
//!   tolerance REQ-COL-006's verify detail names) of the f64 angle of `Φ`'s vectors, a whole turn apart counted as none.
//!
//! Controls (R-176): the expectation read from the positions' Jacobi vectors (the form R-628 replaced), and the
//! expectation with `λ̃` mirrored in `ρ̃`'s axis (the sign a mirror would turn, which no decode takes): each is found.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; each chart at the harness's held parameters.

use std::path::{Path, PathBuf};

use kernel::chart::invariant::WarpReal;
use kernel::chart::{
    BurrauThetaK, ChartDecode, EuclidPlane, InvariantLE, InvariantLK, Latent, MassSimplex,
    ShapeMap, ShapeSphere,
};
use kernel::decode::descriptor::describe;
use kernel::decode::Ic;
use kernel::generated::links::{Defaults, LinkReal};
use kernel::payload::{ICDescriptorOf, PayloadReal};
use kernel::stage::decode::{sample_with, UniformChart};
use render::presets::DecodeChart;
use validation::decode_preset as dp;
use validation::negative_control;

// ── The widths and §3.6's cases, written here ───────────────────────────────────────────────────────────────────────

trait W:
    LinkReal + PayloadReal + WarpReal + From<f32> + Copy + PartialEq + std::fmt::Debug + 'static
{
    const LABEL: &'static str;
    /// `π̂`, §3.2's image of π (R-417).
    const PI_HAT: Self;
    fn of(x: f64) -> Self;
    fn f(self) -> f64;
    fn bits(self) -> u64;
    fn nil() -> Self;
    fn angle2(y: Self, x: Self) -> Self;
}

impl W for f32 {
    const LABEL: &'static str = "f32";
    // f32's nearest π, 0x40490fdb, lies above π; rounded toward zero it is the value below.
    const PI_HAT: f32 = f32::from_bits(0x4049_0fda);
    fn of(x: f64) -> Self {
        x as f32
    }
    fn f(self) -> f64 {
        f64::from(self)
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn nil() -> Self {
        0.0
    }
    fn angle2(y: Self, x: Self) -> Self {
        y.atan2(x)
    }
}

impl W for f64 {
    const LABEL: &'static str = "f64";
    const PI_HAT: f64 = std::f64::consts::PI;
    fn of(x: f64) -> Self {
        x
    }
    fn f(self) -> f64 {
        self
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn nil() -> Self {
        0.0
    }
    fn angle2(y: Self, x: Self) -> Self {
        y.atan2(x)
    }
}

/// dd_decoder §3.6's `rho_angle` of `ρ̃ = r`, `λ̃ = l`, case by case.
fn case_angle<R: W>(r: [R; 2], l: [R; 2]) -> R {
    let cross = r[0] * l[1] - r[1] * l[0];
    let dot = r[0] * l[0] + r[1] * l[1];
    let zero = R::nil();
    if cross == zero {
        return if dot < zero { R::PI_HAT } else { zero };
    }
    let a = R::angle2(cross, dot);
    if a > R::PI_HAT || a <= zero - R::PI_HAT {
        R::PI_HAT
    } else {
        a
    }
}

/// The positions' Jacobi vectors `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, the form R-628 replaced (the controls).
#[cfg(feature = "controls")]
fn positional<R: W>(ic: &Ic<R>) -> ([R; 2], [R; 2]) {
    let [m0, m1, _] = ic.m;
    let [r0, r1, r2] = ic.r;
    let m01 = m0 + m1;
    let inner = [0, 1].map(|k| (m0 * r0[k] + m1 * r1[k]) / m01);
    (
        [r1[0] - r0[0], r1[1] - r0[1]],
        [r2[0] - inner[0], r2[1] - inner[1]],
    )
}

/// Which expectation: R-628's, or a control's.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Expect {
    /// `Φ`'s `(ρ̃, λ̃)`, the configuration block's.
    Phi,
    /// The positions' Jacobi vectors (control).
    #[cfg(feature = "controls")]
    Positional,
    /// `Φ`'s vectors, `λ̃` mirrored in `ρ̃`'s axis (control).
    #[cfg(feature = "controls")]
    Mirrored,
}

fn expected<R: W>(expect: Expect, rho_t: [R; 2], lambda_t: [R; 2], _ic: &Ic<R>) -> R {
    match expect {
        Expect::Phi => case_angle(rho_t, lambda_t),
        #[cfg(feature = "controls")]
        Expect::Positional => {
            let (r, l) = positional(_ic);
            case_angle(r, l)
        }
        #[cfg(feature = "controls")]
        Expect::Mirrored => {
            // λ̃ reflected in ρ̃'s direction: λ̃' = 2(λ̃·ρ̂)ρ̂ − λ̃, formed in f64.
            let (r, l) = (rho_t.map(W::f), lambda_t.map(W::f));
            let n = r[0] * r[0] + r[1] * r[1];
            let s = 2.0 * (l[0] * r[0] + l[1] * r[1]) / n;
            case_angle(rho_t, [R::of(s * r[0] - l[0]), R::of(s * r[1] - l[1])])
        }
    }
}

fn words<R: W>(d: &ICDescriptorOf<R>) -> [R; 12] {
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

const NAMES: [&str; 12] = [
    "m0",
    "m1",
    "m2",
    "q_mass",
    "rho_mag",
    "lambda_mag",
    "rho_ratio",
    "rho_angle",
    "K_0",
    "V_0",
    "virial_ratio",
    "r_min_pair_0",
];
const RHO_ANGLE: usize = 7;

// ── Every chart ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// One point of `chart` at width `R`, against `expect`.
fn check_point<C, R>(chart: &C, uv: [f32; 2], expect: Expect, who: &str)
where
    C: ChartDecode<Param = f32>,
    R: W,
{
    let (u, v) = (<R as From<f32>>::from(uv[0]), <R as From<f32>>::from(uv[1]));
    let at = format!("{who} at {uv:?}, {}", R::LABEL);
    let d = chart.decode_at::<Defaults, R>(u, v);
    let want = expected(expect, d.config.rho_t, d.config.lambda_t, &d.ic);
    let off = sample_with::<C, Defaults, R>(chart, u, v, None);
    let v0 = off.descriptor.V_0.f();
    let target = if v0.is_finite() {
        R::of(v0 + 0.25 * v0.abs().max(1.0))
    } else {
        R::of(0.0)
    };
    let on = sample_with::<C, Defaults, R>(chart, u, v, Some(target));
    for (name, desc) in [
        ("chart decode's", &d.descriptor),
        ("decode stage's, η_E off,", &off.descriptor),
        ("decode stage's, η_E on,", &on.descriptor),
    ] {
        let got = desc.rho_angle;
        assert!(
            got.bits() == want.bits(),
            "{at}: the {name} rho_angle {got:?} is not §3.6's angle of Φ's (ρ̃, λ̃), {want:?}"
        );
    }
    for (name, out) in [("η_E off", &off), ("η_E on", &on)] {
        let have = words(&out.descriptor);
        let positional = words(&describe(&out.ic));
        for k in (0..12).filter(|k| *k != RHO_ANGLE) {
            assert!(
                have[k].bits() == positional[k].bits(),
                "{at}, {name}: {} {:?} is not its positional form {:?} of the stored (m, r, p)",
                NAMES[k],
                have[k],
                positional[k]
            );
        }
    }
    assert!(
        on.descriptor.rho_angle.bits() == off.descriptor.rho_angle.bits(),
        "{at}: η_E moved rho_angle"
    );
}

fn lattice() -> Vec<[f32; 2]> {
    let mut out = vec![[0.5, 0.5]];
    for j in 0..9 {
        for i in 0..9 {
            out.push([(i as f32 + 0.5) / 9.0, (j as f32 + 0.5) / 9.0]);
        }
    }
    out
}

fn check_chart<C: UniformChart>(which: DecodeChart, expect: Expect) {
    let chart = C::from_uniform(&dp::uniforms(which).chart);
    for uv in lattice() {
        check_point::<C, f32>(&chart, uv, expect, &format!("{which:?}"));
        check_point::<C, f64>(&chart, uv, expect, &format!("{which:?}"));
    }
}

fn check_charts(expect: Expect) {
    for which in DecodeChart::ALL {
        match which {
            DecodeChart::Latent => check_chart::<Latent<f32>>(which, expect),
            DecodeChart::ShapeSphere => check_chart::<ShapeSphere<f32>>(which, expect),
            DecodeChart::InvariantLE => check_chart::<InvariantLE<f32>>(which, expect),
            DecodeChart::InvariantLK => check_chart::<InvariantLK<f32>>(which, expect),
            DecodeChart::MassSimplex => check_chart::<MassSimplex<f32>>(which, expect),
            DecodeChart::EuclidPlane => check_chart::<EuclidPlane<f32>>(which, expect),
            DecodeChart::BurrauThetaK => check_chart::<BurrauThetaK<f32>>(which, expect),
        }
    }
}

#[test]
fn qa_m225_rho_angle_is_phis_on_every_chart() {
    check_charts(Expect::Phi);
}

negative_control!(
    qa_m225_rho_angle_is_phis_on_every_chart,
    "rho_angle expected from the positions' Jacobi vectors must be found",
    expected = "is not §3.6's angle of Φ's",
    check_charts(Expect::Positional)
);

negative_control!(
    qa_m225_rho_angle_is_phis_on_every_chart_mirror,
    "rho_angle expected with λ̃ mirrored must be found",
    expected = "is not §3.6's angle of Φ's",
    check_charts(Expect::Mirrored)
);

// ── The λ̃ = 0 pole ─────────────────────────────────────────────────────────────────────────────────────────────────

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// REQ-DEC-043's factor, in f32 eps.
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

/// The shape sphere on its exponential map at the point `n0` (at `(½, ½)`), phase `phase`, else the held parameters.
fn sphere(n0: [f32; 3], phase: f32) -> ShapeSphere<f32> {
    let held = ShapeSphere::<f32>::from_uniform(&dp::uniforms(DecodeChart::ShapeSphere).chart);
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1: [0.0, 1.0, 0.0],
        e2: [0.0, 0.0, 1.0],
        phase,
        ..held
    }
}

const PHASES: [f32; 3] = [0.0, 0.3, 2.0];

/// The f64 angle of `Φ`'s vectors at `c`'s `(½, ½)`, §3.6's cases on the f64 decode's configuration block.
fn phi64(c: &ShapeSphere<f32>) -> f64 {
    let k = c.decode_at::<Defaults, f64>(0.5, 0.5).config;
    case_angle(k.rho_t, k.lambda_t)
}

fn check_pole(expect: Expect) {
    for phase in PHASES {
        for zeros in [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]] {
            let c = sphere([1.0, zeros[0], zeros[1]], phase);
            let at = format!("n₀ = {:?}, φ_f = {phase}", c.n0);
            let d32 = c.decode_at::<Defaults, f32>(0.5, 0.5);
            let d64 = c.decode_at::<Defaults, f64>(0.5, 0.5);
            assert!(
                d32.config.lambda_t.iter().all(|x| *x == 0.0)
                    && d64.config.lambda_t.iter().all(|x| *x == 0.0),
                "{at}: Φ's λ̃ is not 0, the pole is not reached"
            );
            let want32 = expected(expect, d32.config.rho_t, d32.config.lambda_t, &d32.ic);
            let want64 = expected(expect, d64.config.rho_t, d64.config.lambda_t, &d64.ic);
            for e in [None, Some(0.0)] {
                let o32 = sample_with::<_, Defaults, f32>(&c, 0.5, 0.5, e.map(|x| x as f32));
                let o64 = sample_with::<_, Defaults, f64>(&c, 0.5, 0.5, e);
                assert!(
                    o32.descriptor.rho_angle.to_bits() == want32.to_bits()
                        && o64.descriptor.rho_angle.to_bits() == want64.to_bits(),
                    "{at}, E* {e:?}: rho_angle ({:?}, {:?}) at the λ̃ = 0 pole is not §3.6's angle of Φ's (ρ̃, λ̃)",
                    o32.descriptor.rho_angle,
                    o64.descriptor.rho_angle
                );
                assert_eq!(
                    (
                        o32.descriptor.rho_angle.to_bits(),
                        o64.descriptor.rho_angle.to_bits()
                    ),
                    (0, 0),
                    "{at}, E* {e:?}: rho_angle at the λ̃ = 0 pole is not the constant +0"
                );
            }
        }
    }
    let f = factor();
    let tau = std::f64::consts::TAU;
    for phase in PHASES {
        for k in 0..=8 {
            let e = 10f32.powi(-12 + k);
            for eps in [e, -e] {
                for n0 in [[1.0, eps, 0.0], [1.0, 0.0, eps]] {
                    let c = sphere(n0, phase);
                    let d = c.decode_at::<Defaults, f32>(0.5, 0.5);
                    let want = expected(expect, d.config.rho_t, d.config.lambda_t, &d.ic);
                    let got = sample_with::<_, Defaults, f32>(&c, 0.5, 0.5, None)
                        .descriptor
                        .rho_angle;
                    assert!(
                        got.to_bits() == want.to_bits(),
                        "n₀ = {n0:?}, φ_f = {phase}: rho_angle {got:e} is not §3.6's angle of Φ's (ρ̃, λ̃), {want:e}"
                    );
                    let truth = phi64(&c);
                    let diff = (f64::from(got) - truth).rem_euclid(tau);
                    let diff = diff.min(tau - diff);
                    let err = diff / (f64::from(f32::EPSILON) * truth.abs().max(1.0));
                    assert!(
                        err <= f,
                        "n₀ = {n0:?}, φ_f = {phase}: the f32 rho_angle {got:e} is {err} eps from Φ's f64 angle \
                         {truth:e}, over REQ-DEC-043's {f}"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_m225_rho_angle_at_and_near_the_lambda_pole() {
    check_pole(Expect::Phi);
}

negative_control!(
    qa_m225_rho_angle_at_and_near_the_lambda_pole,
    "rho_angle expected from the positions' Jacobi vectors at the pole must be found",
    expected = "is not §3.6's angle of Φ's",
    check_pole(Expect::Positional)
);
