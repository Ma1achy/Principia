//! The generated fragment decode at the shape sphere's exact poles (R-437, R-436; REQ-COL-006; dd_decoder §3.2 "Every
//! `atan2` at a possible pole", §3.3 "`φ` at the pole"; pitfalls §11), against the CPU's f32 decode of the same source.
//!
//! - `decode_preset_shape_poles`: the shape-sphere chart on its exponential map (`ShapeMap::Exponential`) at `(½, ½)`,
//!   where the offset is 0 and `sin r/r` is selected as 1, so the point is `n₀` bit for bit, its zero components
//!   keeping their signs where the frame's zeros are `−0` (`d = −0`) and turning `+0` where they are `+0`. `n₀ = (−1,
//!   ±0, ±0)` is the `ρ̃ = 0` collision pole (`BC₀₁`): the inverse's `atan2(q, p)` sees `p = q = ±0` and takes R-437's
//!   selected 0, `C`'s rotation at `ρ = 0` takes `φ = 0` (§3.3), and the descriptor's `V` takes the section's `−∞`
//!   select at the zero separation. `n₀ = (+1, ±0, ±0)` is the `λ̃ = 0` pole, the same `atan2` at `±0` operands. Over
//!   every sign of the two zeros, under both frames, the fragment's words in the probe's layout
//!   (`render::presets::decode::probe`: `(m, r, p)`, the tag, `z`, the latent flag and the descriptor), read in one
//!   draw of a 4 × 4 tile, one shader build per case, are held to `kernel::stage::fragment::decode_at`, the fragment
//!   decode's own Rust source, run on the CPU at f32 on the same uniforms: a non-finite word bit for bit (`V₀ = −∞`'s
//!   bits at `n₀ = −1`, also asserted directly), a finite one within REQ-DEC-043's factor, the tag and flag exactly; at
//!   the `λ̃ = 0` pole, where `λ` is `C`'s rounding residue, whose size and sign each backend rounds its own way,
//!   `rho_ratio` as its reciprocal and `rho_angle` as one of the axis's two values, 0 or `π̂` (`residue_agrees`). The
//!   CPU's point is asserted to be the pole, and the cases to reach all four signs of `(p, q)` at each pole. Control: a
//!   CPU reference that takes `atan2`'s own value at `(+0, −0)`, `π`, in place of the select (the chart's phase `φ_f =
//!   π`, which at `ρ̃ = 0` moves only `ψ`), is far outside the factor.
//!
//! **Domain** (pitfalls §10): the decode alone at one point, `(½, ½)` on a depth-0 quad, inputs held fixed; the chart's
//! held values `validation::decode_preset::uniforms`'s, its frame the poles'. The spherical map's `(½, ½)` is not this
//! domain: at f32 it lies about `5·10⁻⁸` off `BC₀₁` (`render`'s `decode_preset_poles`).

use kernel::chart::{ShapeMap, ShapeSphere};
use kernel::decode::descriptor::pi_image;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{DecodeUniforms, UniformChart};
use kernel::stage::fragment::{
    decode_at, DECODE_DESCRIPTOR, DECODE_IC, DECODE_TAG, DECODE_UV, DECODE_WORDS, DECODE_Z,
    DECODE_Z_LATENT,
};
use render::bind::ViewOutput;
use render::presets::{self, agreement, DecodeChart};
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

/// The descriptor's `lambda_mag`, `rho_ratio` and `rho_angle`, by their places in its twelve words.
const LAMBDA_MAG: usize = 5;
const RHO_RATIO: usize = 6;
const RHO_ANGLE: usize = 7;

/// The view's words against the CPU decode's, `rho_ratio` and `rho_angle` apart ([`residue_agrees`]): `(view word,
/// decode word, name)`.
fn pairs() -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    for k in 0..15 {
        out.push((k, DECODE_IC + k, format!("(m, r, p) word {k}")));
    }
    for k in 0..8 {
        out.push((16 + k, DECODE_Z + k, format!("z{k}")));
    }
    for k in (0..12).filter(|k| *k != RHO_RATIO && *k != RHO_ANGLE) {
        out.push((
            25 + k,
            DECODE_DESCRIPTOR + k,
            format!("descriptor word {k}"),
        ));
    }
    out
}

/// One fragment word against the CPU's at f32: bit for bit where either is non-finite (R-436), else within
/// REQ-DEC-043's factor of the CPU's.
fn agree(gpu: u32, cpu: u32) -> bool {
    let (g, c) = (f32::from_bits(gpu), f32::from_bits(cpu));
    if presets::non_finite_bits(g) || presets::non_finite_bits(c) {
        return gpu == cpu;
    }
    let bound =
        f64::from(agreement::FACTOR) * f64::from(f32::EPSILON) * f64::from(c.abs()).max(1.0);
    (f64::from(g) - f64::from(c)).abs() <= bound
}

/// `rho_ratio = ‖ρ̃‖/‖λ̃‖` and `rho_angle`, the fragment's against the CPU's. Where the CPU's `‖λ̃‖` is within
/// REQ-DEC-043's factor of 0, at the `λ̃ = 0` pole, `λ` is `C`'s rounding residue (`Φ`'s `λ̃` is 0; the CoM shift's
/// rounding is not), whose size and sign each compilation rounds its own way: Metal's and the CPU's are nonzero,
/// lavapipe's sits on the other side of 0 or at 0. So there `rho_ratio` is held as its reciprocal `‖λ̃‖/‖ρ̃‖`, well
/// conditioned, within the factor (`1/+∞ = 0` where a backend's `λ` is 0 and the ratio's `+∞` select is taken), and
/// `rho_angle`, every body on the `x` axis so `ρ × λ = 0` exactly, as one of the axis's two values, 0 or `π̂`
/// (dd_decoder §3.6), the residue's sign choosing which. Elsewhere both are held as every word is.
fn residue_agrees(gpu: &[u32], cpu: &[u32; DECODE_WORDS]) -> Result<(), String> {
    let at = |k: usize| (gpu[25 + k], cpu[DECODE_DESCRIPTOR + k]);
    let ((gr, cr), (ga, ca)) = (at(RHO_RATIO), at(RHO_ANGLE));
    let lambda = f64::from(f32::from_bits(cpu[DECODE_DESCRIPTOR + LAMBDA_MAG]));
    let residue = lambda <= f64::from(agreement::FACTOR) * f64::from(f32::EPSILON);
    let ratio = if residue {
        let inverse = |w: u32| (1.0 / f32::from_bits(w)).to_bits();
        agree(inverse(gr), inverse(cr))
    } else {
        agree(gr, cr)
    };
    let angle = if residue {
        [0, pi_image::<f32>().to_bits()].contains(&ga)
    } else {
        agree(ga, ca)
    };
    match (ratio, angle) {
        (true, true) => Ok(()),
        (false, _) => Err(format!("rho_ratio {gr:#010x} is not the CPU's {cr:#010x}")),
        (_, false) => Err(format!("rho_angle {ga:#010x} is not the CPU's {ca:#010x}")),
    }
}

/// The chart at the pole `n₀ = (sign, zeros)`, its frame's zero components `+0` or `−0` by `negative_frame`.
fn chart(sign: f32, zeros: [f32; 2], negative_frame: bool) -> ShapeSphere<f32> {
    let held = ShapeSphere::<f32>::from_uniform(&dp::uniforms(DecodeChart::ShapeSphere).chart);
    let (e1, e2) = if negative_frame {
        ([-0.0, -1.0, -0.0], [-0.0, -0.0, -1.0])
    } else {
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0])
    };
    ShapeSphere {
        map: ShapeMap::Exponential,
        n0: [sign, zeros[0], zeros[1]],
        e1,
        e2,
        phase: 0.0,
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

/// The poles checked; `reference_phase` is the CPU reference's `φ_f`: 0, the chart's own, or `π`, `atan2`'s value at
/// `(+0, −0)` in place of the select (the control).
fn check(reference_phase: f32) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let mut scene = Scene::new(DecodeChart::ShapeSphere, [1, 1], 4, 0, &[[0, 0]]).unwrap();
    let mut reached = Vec::new();
    for sign in [-1.0f32, 1.0] {
        for zeros in [[0.0f32, 0.0], [0.0, -0.0], [-0.0, 0.0], [-0.0, -0.0]] {
            for negative_frame in [false, true] {
                let c = chart(sign, zeros, negative_frame);
                let case = format!("n₀ = {:?}, frame zeros negative {negative_frame}", c.n0);
                let n = c.point(0.5f32, 0.5f32);
                assert!(
                    n[0] == sign && n[1] == 0.0 && n[2] == 0.0,
                    "{case}: the CPU's point {n:?} is not the pole"
                );
                reached.push((sign, n[1].is_sign_negative(), n[2].is_sign_negative()));
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
                    phase: reference_phase,
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
                if let Err(e) = residue_agrees(&w, &reference) {
                    panic!("{case}: the fragment's {e}");
                }
                if sign < 0.0 {
                    assert_eq!(
                        w[34], 0xff80_0000,
                        "{case}: the fragment's V_0 at the ρ̃ = 0 pole is not −∞'s bits"
                    );
                }
            }
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

#[test]
fn decode_preset_shape_poles() {
    check(0.0);
}

negative_control!(
    decode_preset_shape_poles,
    "a reference taking atan2's π at the pole in place of the select must be found",
    expected = "is not the CPU's",
    check(std::f32::consts::PI)
);
