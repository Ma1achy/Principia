//! The generated fragment decode at and near the shape sphere's exact poles (R-437, R-436, R-628; REQ-COL-006;
//! dd_decoder §3.2 "Every `atan2` at a possible pole", §3.3 "`φ` at the pole", §3.6; pitfalls §11), against the CPU's
//! f32 decode of the same source and, for `rho_angle`, `Φ`'s f64 angle.
//!
//! - `decode_preset_shape_poles`: the shape-sphere chart on its exponential map (`ShapeMap::Exponential`) at `(½, ½)`,
//!   where the offset is 0 and `sin r/r` is selected as 1, so the point is `n₀` bit for bit, its zero components
//!   keeping their signs where the frame's zeros are `−0` (`d = −0`) and turning `+0` where they are `+0`. `n₀ = (−1,
//!   ±0, ±0)` is the `ρ̃ = 0` collision pole (`BC₀₁`): the inverse's `atan2(q, p)` sees `p = q = ±0` and takes R-437's
//!   selected 0, `C`'s rotation at `ρ = 0` takes `φ = 0` (§3.3), and the descriptor's `V` takes the section's `−∞`
//!   select at the zero separation. `n₀ = (+1, ±0, ±0)` is the `λ̃ = 0` pole, the same `atan2` at `±0` operands. Over
//!   every sign of the two zeros, under both frames, at the chart's phase `φ_f = 0`, and at the `λ̃ = 0` pole at the
//!   generic phase `φ_f = 0.3` too; and near that pole, `n₀ = (+1, ε, +0)` and `(+1, +0, ε)`, `‖λ̃‖ = |ε|/2`, over
//!   both signs of `ε` from `10⁻¹²` to `5·10⁻⁵`, at `φ_f = 0` and `0.3` (the sweep of PR #303's measurement). In each
//!   case the fragment's words in the probe's layout (`render::presets::decode::probe`: `(m, r, p)`, the tag, `z`, the
//!   latent flag and the descriptor), read in one draw of a 4 × 4 tile, one shader build per case, are held to
//!   `kernel::stage::fragment::decode_at`, the fragment decode's own Rust source, run on the CPU at f32 on the same
//!   uniforms: a non-finite word bit for bit (`V₀ = −∞`'s bits at `n₀ = −1`, also asserted directly), a finite one
//!   within REQ-DEC-043's factor, the tag and flag exactly. `rho_angle` (descriptor word 7) is held so too, strictly:
//!   it is read from the configuration block's own `(ρ̃, λ̃)` (R-628), not from `C`'s re-centred positions, whose
//!   rounding residue each backend rounds its own way; the fragment's and the CPU's are each held to `Φ`'s f64 angle
//!   within the factor, and at an exact pole each is 0's bits. `rho_ratio` is held as dd_decoder §3.6 defines it at
//!   the `α = 0` pole, on each backend's own words: `+∞`'s bits exactly where its `lambda_mag` is 0, else its own
//!   quotient `rho_mag/lambda_mag` within the factor (`ratio_own`); between backends it divides two residues. The
//!   CPU's point is asserted to be `n₀`, and the cases to reach all four signs of `(p, q)` at each pole.
//! - Controls: a CPU reference that takes `atan2`'s own value at `(+0, −0)`, `π`, in place of the select (the chart's
//!   phase `φ_f = π`, which at `ρ̃ = 0` moves only `ψ`), is far outside the factor; and `rho_angle` read from the
//!   CPU decode's positions (`descriptor::describe`, the form before R-628) is far from `Φ`'s f64 angle near the
//!   `λ̃ = 0` pole (CPU only).
//!
//! **Domain** (pitfalls §10): the decode alone at one point, `(½, ½)` on a depth-0 quad, inputs held fixed; the chart's
//! held values `validation::decode_preset::uniforms`'s, its frame the poles'. The spherical map's `(½, ½)` is not this
//! domain: at f32 it lies about `5·10⁻⁸` off `BC₀₁` (`render`'s `decode_preset_poles`).

use kernel::chart::{Chart, ShapeMap, ShapeSphere};
use kernel::decode::descriptor::angle;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{DecodeUniforms, UniformChart};
use kernel::stage::fragment::{
    decode_at, DECODE_DESCRIPTOR, DECODE_IC, DECODE_TAG, DECODE_UV, DECODE_WORDS, DECODE_Z,
    DECODE_Z_LATENT,
};
use render::bind::ViewOutput;
use render::presets::{self, DecodeChart};
use validation::decode_preset::{self as dp, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// The view: the generated decode at `(½, ½)`, its words in the probe's layout (`render::presets::decode::probe`),
/// group `k` (words `4k` to `4k + 3`) at the tile's pixel `k`, row-major from the top-left, so one draw of a 4 × 4 tile
/// reads every word, one shader build per case.
const VIEW: &str = "
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    let d = frag_decode(vec2<f32>(0.5, 0.5));
    var w: array<u32, 40>;
    w[0] = bitcast<u32>(d.m.x);
    w[1] = bitcast<u32>(d.m.y);
    w[2] = bitcast<u32>(d.m.z);
    for (var b = 0u; b < 3u; b++) {
        w[3u + 2u * b] = bitcast<u32>(d.r[b].x);
        w[4u + 2u * b] = bitcast<u32>(d.r[b].y);
        w[9u + 2u * b] = bitcast<u32>(d.p[b].x);
        w[10u + 2u * b] = bitcast<u32>(d.p[b].y);
    }
    w[15] = d.tag;
    for (var k = 0u; k < 8u; k++) {
        w[16u + k] = bitcast<u32>(d.z[k]);
    }
    w[24] = select(0u, 1u, d.z_latent);
    w[25] = d.d.m0;
    w[26] = d.d.m1;
    w[27] = d.d.m2;
    w[28] = d.d.q_mass;
    w[29] = d.d.rho_mag;
    w[30] = d.d.lambda_mag;
    w[31] = d.d.rho_ratio;
    w[32] = d.d.rho_angle;
    w[33] = d.d.K_0;
    w[34] = d.d.V_0;
    w[35] = d.d.virial_ratio;
    w[36] = d.d.r_min_pair_0;
    w[39] = 0x600du;
    let px = vec2<u32>(l.screen.pixel);
    let k = min(px.y * 4u + px.x, 9u);
    return vec4<u32>(w[4u * k], w[4u * k + 1u], w[4u * k + 2u], w[4u * k + 3u]);
}
";

/// The view's words: 40, the last a marker that the groups were read in place.
const WORDS: usize = 40;

/// The descriptor's `rho_mag`, `lambda_mag`, `rho_ratio` and `rho_angle`, by their places in its twelve words.
const RHO_MAG: usize = 4;
const LAMBDA_MAG: usize = 5;
const RHO_RATIO: usize = 6;
const RHO_ANGLE: usize = 7;

/// Where the view's descriptor starts.
const VIEW_DESCRIPTOR: usize = 25;

/// `+∞`'s bits at f32.
const POS_INF: u32 = 0x7f80_0000;

/// The phases: the chart's `φ_f = 0`, and a generic one.
const PHASES: [f32; 2] = [0.0, 0.3];

/// The sweep's `|ε|`, `‖λ̃‖ = |ε|/2`: from where `C`'s residue swamps `λ̃` to where the positions' angle agrees again.
const EPSILONS: [f32; 5] = [1e-12, 1.6e-9, 3.2e-8, 1e-6, 5e-5];

/// The view's words against the CPU decode's, `rho_ratio` apart ([`ratio_own`]): `(view word, decode word, name)`.
fn pairs() -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    for k in 0..15 {
        out.push((k, DECODE_IC + k, format!("(m, r, p) word {k}")));
    }
    for k in 0..8 {
        out.push((16 + k, DECODE_Z + k, format!("z{k}")));
    }
    for k in (0..12).filter(|k| *k != RHO_RATIO) {
        out.push((
            VIEW_DESCRIPTOR + k,
            DECODE_DESCRIPTOR + k,
            format!("descriptor word {k}"),
        ));
    }
    out
}

/// REQ-DEC-043's factor, in f32 eps: `decode-f32-eps`'s threshold, read once.
fn factor() -> f64 {
    static FACTOR: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *FACTOR.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        validation::gate::config(&root, "decode-f32-eps")
            .expect("decode-f32-eps's gate.json")
            .threshold
            .value
    })
}

/// REQ-DEC-043's factor of f32's ε, relative to `c`, floored at 1.
fn bound(c: f64) -> f64 {
    factor() * f64::from(f32::EPSILON) * c.abs().max(1.0)
}

/// One fragment word against the CPU's at f32: bit for bit where either is non-finite (R-436), else within
/// REQ-DEC-043's factor of the CPU's.
fn agree(gpu: u32, cpu: u32) -> bool {
    let (g, c) = (f32::from_bits(gpu), f32::from_bits(cpu));
    if presets::non_finite_bits(g) || presets::non_finite_bits(c) {
        return gpu == cpu;
    }
    (f64::from(g) - f64::from(c)).abs() <= bound(f64::from(c))
}

/// `rho_ratio` against the descriptor's own `rho_mag` and `lambda_mag`, `d` its twelve words (dd_decoder §3.6): `+∞`'s
/// bits exactly where `lambda_mag` is 0, else the quotient `rho_mag/lambda_mag` within the factor.
fn ratio_own(d: &[u32]) -> Result<(), String> {
    let (rho, lambda) = (f32::from_bits(d[RHO_MAG]), f32::from_bits(d[LAMBDA_MAG]));
    let ratio = d[RHO_RATIO];
    let ok = if lambda == 0.0 {
        ratio == POS_INF
    } else {
        ratio != POS_INF && agree(ratio, (rho / lambda).to_bits())
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "rho_ratio {ratio:#010x} is not its own rho_mag {rho:e} over lambda_mag {lambda:e}"
        ))
    }
}

/// The signed difference `a − b` of two angles, folded onto `[−π, π]`.
fn turn(a: f64, b: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let d = (a - b).rem_euclid(tau);
    if d > std::f64::consts::PI {
        d - tau
    } else {
        d
    }
}

/// `rho_angle`'s bits `got`, `who`'s, against `Φ`'s f64 angle `truth`, within the factor, a whole turn apart counted as
/// none.
fn check_phi(case: &str, who: &str, got: u32, truth: f64) {
    let g = f64::from(f32::from_bits(got));
    assert!(
        turn(g, truth).abs() <= bound(truth),
        "{case}: the {who} rho_angle {g:e} is not Φ's f64 angle {truth:e}"
    );
}

/// The chart at the point `n0`, its frame's zero components `+0` or `−0` by `negative_frame`, at the phase `phase`.
fn chart(n0: [f32; 3], negative_frame: bool, phase: f32) -> ShapeSphere<f32> {
    let held = ShapeSphere::<f32>::from_uniform(&dp::uniforms(DecodeChart::ShapeSphere).chart);
    let (e1, e2) = if negative_frame {
        ([-0.0, -1.0, -0.0], [-0.0, -0.0, -1.0])
    } else {
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0])
    };
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0,
        e1,
        e2,
        phase,
        ..held
    }
}

/// The CPU's f32 fragment decode of `chart` at `(½, ½)`.
fn cpu(chart: &ShapeSphere<f32>) -> [u32; DECODE_WORDS] {
    let u = DecodeUniforms {
        chart: chart.uniform(),
        ..dp::uniforms(DecodeChart::ShapeSphere)
    };
    let mut words = [0u32; DECODE_WORDS];
    words[DECODE_UV] = 0.5f32.to_bits();
    words[DECODE_UV + 1] = 0.5f32.to_bits();
    decode_at::<ShapeSphere<f32>, Defaults>(&u, &mut words);
    words
}

/// `Φ`'s angle at f64: `chart`'s decode at `(½, ½)` at f64, [`angle`] of its configuration block's `ρ̃ × λ̃` and
/// `ρ̃ · λ̃`, formed here.
fn phi_angle(chart: &ShapeSphere<f32>) -> f64 {
    let c = chart.decode::<Defaults, f64>(0.5, 0.5).config;
    let (r, l) = (c.rho_t, c.lambda_t);
    angle(r[0] * l[1] - r[1] * l[0], r[0] * l[0] + r[1] * l[1])
}

/// The CPU decode's `(m, r, p)`, from its words.
#[cfg(feature = "controls")]
fn ic_of(words: &[u32; DECODE_WORDS]) -> kernel::decode::Ic<f32> {
    let w = |k: usize| f32::from_bits(words[DECODE_IC + k]);
    kernel::decode::Ic {
        m: [w(0), w(1), w(2)],
        r: [[w(3), w(4)], [w(5), w(6)], [w(7), w(8)]],
        p: [[w(9), w(10)], [w(11), w(12)], [w(13), w(14)]],
    }
}

/// The fragment's words of `scene`'s decode at `(½, ½)`, in one draw ([`VIEW`]).
fn gpu_words(gpu: &GpuHarness, scene: &Scene) -> [u32; WORDS] {
    let image = scene
        .draw_view(gpu.device(), gpu.queue(), VIEW, ViewOutput::Words)
        .unwrap();
    let mut w = [0u32; WORDS];
    for k in 0..WORDS / 4 {
        let px = image.words(k as u32 % 4, k as u32 / 4);
        w[4 * k..4 * k + 4].copy_from_slice(&px[..4]);
    }
    w
}

/// A case: its chart, and whether its `n₀` is an exact pole (`rho_angle` 0's bits there).
struct Case {
    chart: ShapeSphere<f32>,
    pole: bool,
}

/// The exact poles: both, every sign of the two zeros, both frames, at `φ_f = 0`; and the `λ̃ = 0` pole likewise at
/// the generic phase.
fn poles() -> Vec<Case> {
    let mut out = Vec::new();
    for (sign, phase) in [(-1.0f32, 0.0f32), (1.0, PHASES[0]), (1.0, PHASES[1])] {
        for zeros in [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]] {
            for negative_frame in [false, true] {
                out.push(Case {
                    chart: chart([sign, zeros[0], zeros[1]], negative_frame, phase),
                    pole: true,
                });
            }
        }
    }
    out
}

/// The sweep near the `λ̃ = 0` pole: `n₀ = (+1, ε, +0)` and `(+1, +0, ε)`, both signs of each [`EPSILONS`], at each
/// of [`PHASES`].
fn sweep() -> Vec<Case> {
    let mut out = Vec::new();
    for phase in PHASES {
        for e in EPSILONS {
            for eps in [e, -e] {
                for n0 in [[1.0, eps, 0.0], [1.0, 0.0, eps]] {
                    out.push(Case {
                        chart: chart(n0, false, phase),
                        pole: false,
                    });
                }
            }
        }
    }
    out
}

/// The cases checked; `reference_phase` is the CPU reference's `φ_f` where `Some`: `atan2`'s value at `(+0, −0)`, `π`,
/// in place of the select (the control); `None`, each case's own.
fn check(reference_phase: Option<f32>) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let mut scene = Scene::new(DecodeChart::ShapeSphere, [1, 1], 4, 0, &[[0, 0]]).unwrap();
    let mut reached = Vec::new();
    for Case { chart: c, pole } in poles().into_iter().chain(sweep()) {
        let case = format!("n₀ = {:?}, φ_f = {}, frame e1 {:?}", c.n0, c.phase, c.e1);
        let n = c.point(0.5f32, 0.5f32);
        assert!(
            n.map(f32::to_bits) == c.n0.map(f32::to_bits)
                || (pole && n[1] == 0.0 && n[2] == 0.0 && n[0] == c.n0[0]),
            "{case}: the CPU's point {n:?} is not n₀"
        );
        if pole {
            reached.push((c.n0[0], n[1].is_sign_negative(), n[2].is_sign_negative()));
        }
        scene.uniforms = DecodeUniforms {
            chart: c.uniform(),
            ..dp::uniforms(DecodeChart::ShapeSphere)
        };
        let w = gpu_words(&gpu, &scene);
        assert_eq!(
            w[39], 0x600d,
            "{case}: the view's groups were not read in place"
        );
        let reference = cpu(&ShapeSphere {
            phase: reference_phase.unwrap_or(c.phase),
            ..c
        });
        assert_eq!(w[15], reference[DECODE_TAG], "{case}: the tag differs");
        assert_eq!(
            w[24], reference[DECODE_Z_LATENT],
            "{case}: the latent flag differs"
        );
        for (g, r, name) in pairs() {
            assert!(
                agree(w[g], reference[r]),
                "{case}: the fragment's {name} {:#010x} is not the CPU's {:#010x}",
                w[g],
                reference[r]
            );
        }
        let theirs = &reference[DECODE_DESCRIPTOR..DECODE_DESCRIPTOR + 12];
        for (who, d) in [
            ("fragment's", &w[VIEW_DESCRIPTOR..VIEW_DESCRIPTOR + 12]),
            ("CPU's", theirs),
        ] {
            if let Err(e) = ratio_own(d) {
                panic!("{case}: the {who} {e}");
            }
        }
        let truth = phi_angle(&c);
        let ours = w[VIEW_DESCRIPTOR + RHO_ANGLE];
        check_phi(&case, "fragment's", ours, truth);
        check_phi(&case, "CPU's", theirs[RHO_ANGLE], truth);
        if pole {
            assert_eq!(
                [ours, theirs[RHO_ANGLE]],
                [0, 0],
                "{case}: rho_angle at the exact pole is not 0's bits on the fragment and the CPU"
            );
        }
        if c.n0[0] < 0.0 {
            assert_eq!(
                w[34], 0xff80_0000,
                "{case}: the fragment's V_0 at the ρ̃ = 0 pole is not −∞'s bits"
            );
        }
    }
    for sign in [-1.0f32, 1.0] {
        for (p, q) in [(false, false), (false, true), (true, false), (true, true)] {
            assert!(
                reached.contains(&(sign, p, q)),
                "n₀ = {sign}: no case reaches p negative {p}, q negative {q}"
            );
        }
    }
}

/// `rho_angle` read from the CPU decode's positions (`descriptor::describe`, the form before R-628), held to `Φ`'s
/// f64 angle over the sweep by [`check_phi`]: the control of that check, on the CPU alone.
#[cfg(feature = "controls")]
fn positional_against_phi() {
    for Case { chart: c, .. } in sweep() {
        let case = format!("n₀ = {:?}, φ_f = {}", c.n0, c.phase);
        let positional = kernel::decode::descriptor::describe(&ic_of(&cpu(&c))).rho_angle;
        check_phi(&case, "positional", positional.to_bits(), phi_angle(&c));
    }
}

#[test]
fn decode_preset_shape_poles() {
    check(None);
}

negative_control!(
    decode_preset_shape_poles,
    "a reference taking atan2's π at the pole in place of the select must be found",
    expected = "is not the CPU's",
    check(Some(std::f32::consts::PI))
);

negative_control!(
    decode_preset_shape_poles_phi,
    "rho_angle read from the re-centred positions, not Φ's vectors, must be found near the λ̃ = 0 pole",
    expected = "is not Φ's f64 angle",
    positional_against_phi()
);
