//! The DECODE preset's decode against the f64 `decodeOnly()` (REQ-TOOL-029; R-133), on every M2 chart (REQ-RENDER-027),
//! through the generated fragment decode (R-116; REQ-COL-006): `render::presets::decode`'s probe, the preset's own
//! decode written as words, read back per sample and held to `engine::decode_dispatch::decode_only` at f64 on the same
//! f32 chart point, widened.
//!
//! - `decode_preset_vs_decode_only`: on each chart, 8 × 8 samples spread across the slice, each of the 27 continuous
//!   values (`(m, r, p)` and the descriptor) within REQ-DEC-043's calibrated f32 decode factor by its measure
//!   (`validation::decode_f32_eps::error`), the tag word equal, `Φ`'s `z` within the factor and its latent flag the
//!   chart's; and the identities `Σm = 1`, `Σmᵢrᵢ = 0` and `I = Σmᵢ‖rᵢ‖² = 1` within the same factor. Control: the
//!   reference decoded `10⁻³` off the fragment's point fails.
//! - `decode_preset_poles` (R-437, R-436, R-438): at a coincident pair (`ρ̃ = 0`), at `λ̃ = 0` and on the `−x` half-axis
//!   of the inner pair's centre, every sign of the zeros, the fragment's descriptor (`frag_describe`) and encode
//!   (`frag_encode`) against the CPU's at f32: a non-finite value bit for bit (`V₀`'s `−∞` at the coincident pair, which
//!   the test also asserts directly), a finite one within REQ-DEC-043's factor (the two compilations may differ in the
//!   last bits, and a pole's branch taken the other way, `α = π` for `−π`, is far outside it), the notices word
//!   exactly; and the shape sphere's decode at its `n₀` pole, `(½, ½)`, is finite with the CPU's tag. Control: a finite
//!   `V₀` at the coincident pair fails.
//!
//! **Domain** (pitfalls §10): the decode alone, its inputs held fixed; the factor is REQ-DEC-043's, provisional until the
//! M2 gate, on the domains its record lists (`validation::decode_f32_eps`), each chart at the parameters
//! `validation::decode_preset::uniforms` gives, inside its square.

use std::path::{Path, PathBuf};

use render::presets::{self, agreement, DecodeChart};
use validation::decode_preset::{self as dp, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// REQ-DEC-043's factor, in f32 eps: `decode-f32-eps`'s threshold.
fn factor() -> f64 {
    validation::gate::config(&root(), "decode-f32-eps")
        .expect("decode-f32-eps's gate.json")
        .threshold
        .value
}

/// The f32 `eps`, `2⁻²³`.
const EPS: f64 = f32::EPSILON as f64;

/// The worst of one chart's comparison, in eps, and where.
#[derive(Debug, Default)]
struct Worst {
    value: f64,
    what: String,
}

impl Worst {
    fn note(&mut self, e: f64, what: impl FnOnce() -> String) {
        if e > self.value || e.is_nan() {
            self.value = e;
            self.what = what();
        }
    }
}

/// One chart's probe against its f64 references, each taken `offset` off the fragment's point (0 but in the control):
/// the worst error over the values, `z` and the identities, in eps; panics on a tag or latent flag that differs.
fn compare(gpu: &GpuHarness, chart: DecodeChart, offset: f32) -> Worst {
    let scene = Scene::lattice(chart, [8, 8], 10).unwrap();
    let probe = scene.probe(gpu.device(), gpu.queue()).unwrap();
    let mut worst = Worst::default();
    for (q, w) in probe.iter().enumerate() {
        let uv = [f32::from_bits(w[37]), f32::from_bits(w[38])];
        assert_eq!(
            uv,
            scene.point(q as u32),
            "{chart:?} sample {q}: the fragment decoded another point"
        );
        let r = dp::reference(chart, &scene.uniforms, [uv[0] + offset, uv[1]]).unwrap();
        assert_eq!(
            w[15],
            dp::tag(&r.out),
            "{chart:?} at {uv:?}: the tag differs"
        );
        assert_eq!(
            w[24] != 0,
            r.z_latent,
            "{chart:?}: ctx.chart.z's latent flag is not the chart's"
        );
        let names = validation::decode_f32_eps::QUANTITIES;
        let cpu = dp::values(&r.out);
        let frag: Vec<f64> = (0..15)
            .map(|k| dp::f(w[k]))
            .chain((25..37).map(|k| dp::f(w[k])))
            .collect();
        for k in 0..27 {
            let e = validation::decode_f32_eps::error(frag[k], cpu[k]);
            worst.note(e, || format!("{} at {uv:?}", names[k]));
        }
        for k in 0..8 {
            let e = validation::decode_f32_eps::error(dp::f(w[16 + k]), r.z[k]);
            worst.note(e, || format!("z{k} at {uv:?}"));
        }
        let m = [0, 1, 2].map(|k| frag[k]);
        let rr = [0, 1, 2].map(|b| [frag[3 + 2 * b], frag[4 + 2 * b]]);
        let sum = m.iter().sum::<f64>();
        worst.note((sum - 1.0).abs() / EPS, || format!("Σm − 1 at {uv:?}"));
        // Each axis's mass-weighted coordinates, `mᵢ·rᵢ`, for the CoM.
        let weighted = [0, 1].map(|a| [0, 1, 2].map(|b| m[b] * rr[b][a]));
        for (a, w) in weighted.iter().enumerate() {
            let com: f64 = w.iter().sum();
            let scale: f64 = w.iter().map(|x| x.abs()).sum::<f64>().max(1.0);
            worst.note(com.abs() / (EPS * scale), || format!("CoM_{a} at {uv:?}"));
        }
        let inertia: f64 = (0..3)
            .map(|b| m[b] * (rr[b][0] * rr[b][0] + rr[b][1] * rr[b][1]))
            .sum();
        worst.note((inertia - 1.0).abs() / EPS, || format!("I − 1 at {uv:?}"));
    }
    worst
}

fn check_vs_decode_only(offset: f32) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let f = factor();
    let mut report = Vec::new();
    for chart in DecodeChart::ALL {
        let w = compare(&gpu, chart, offset);
        report.push(format!("{chart:?} {:.2} ({})", w.value, w.what));
        assert!(
            w.value <= f,
            "{chart:?}: the fragment decode is {:.2} eps from decodeOnly at {}, over REQ-DEC-043's {f}",
            w.value,
            w.what
        );
    }
    println!(
        "decode_preset_vs_decode_only: worst in eps (REQ-DEC-043's {f}): {}",
        report.join("; ")
    );
}

#[test]
fn decode_preset_vs_decode_only() {
    check_vs_decode_only(0.0);
}

negative_control!(
    decode_preset_vs_decode_only,
    "a reference decoded 10⁻³ off the fragment's point must fail REQ-DEC-043's factor",
    expected = "over REQ-DEC-043's",
    check_vs_decode_only(1e-3)
);

/// The preset library's DECODE and agreement presets are locked, and a fork leaves them unchanged.
#[test]
fn decode_preset_locked() {
    let library = presets::library();
    let names: Vec<&str> = library.iter().map(|p| p.name()).collect();
    assert_eq!(
        names,
        [presets::decode::NAME, presets::agreement::NAME],
        "the library's fragment-recompute presets"
    );
    for p in &library {
        assert!(p.locked(), "{} ships unlocked", p.name());
        let before = p.clone();
        let mut fork = p.fork("mine");
        fork.view.push_str("\n// edited\n");
        assert_eq!(p, &before, "editing a fork of {} changed it", p.name());
        assert_eq!(fork.name, "mine");
    }
}

// ── The poles (R-436, R-437, R-438) ─────────────────────────────────────────────────────────────────────────────────

use validation::decode_preset::State;

/// The WGSL `f32` of `x`, by its bits, so a `−0` stays `−0`.
fn lit(x: f32) -> String {
    format!("bitcast<f32>({:#010x}u)", x.to_bits())
}

/// A view calling `call` (`frag_describe` or `frag_encode`) on state `x` of `states`, the pixel's column, and writing
/// its twelve result words, group `group` of three, as `pick` lists them.
fn state_view(states: &[State], call: &str, pick: &[&str; 12], group: usize) -> String {
    let mut arms = String::new();
    for (i, s) in states.iter().enumerate() {
        let v2 = |v: [f32; 2]| format!("vec2<f32>({}, {})", lit(v[0]), lit(v[1]));
        arms.push_str(&format!(
            "        case {i}u: {{ m = vec3<f32>({}, {}, {}); r = array<vec2<f32>, 3>({}, {}, {}); \
             p = array<vec2<f32>, 3>({}, {}, {}); }}\n",
            lit(s.m[0]),
            lit(s.m[1]),
            lit(s.m[2]),
            v2(s.r[0]),
            v2(s.r[1]),
            v2(s.r[2]),
            v2(s.p[0]),
            v2(s.p[1]),
            v2(s.p[2]),
        ));
    }
    let g = |k: usize| pick[4 * group + k];
    format!(
        "
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{
    var m: vec3<f32>;
    var r: array<vec2<f32>, 3>;
    var p: array<vec2<f32>, 3>;
    switch (u32(l.screen.pixel.x)) {{
{arms}        default: {{}}
    }}
    let o = {call}(m, r, p);
    return vec4<u32>({}, {}, {}, {});
}}
",
        g(0),
        g(1),
        g(2),
        g(3)
    )
}

/// The twelve result words of `call` on each of `states` in the fragment, the snippet `snippet` placed before it.
fn run_states(snippet: &str, states: &[State], call: &str, pick: &[&str; 12]) -> Vec<[u32; 12]> {
    use render::bind::{self, ViewOutput};
    use render::headless::{Draw, Target};
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let n = states.len() as u32;
    let grid = render::raster::Grid::new([n, 1], 1, 0, 1).unwrap();
    let set = validation::synthetic::Synthetic::flat(grid, 0);
    let context = render::bind::Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid,
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    };
    let bytes = set.bytes();
    let bound = bind::upload(gpu.device(), &bytes.payload(), &context);
    let mut out = vec![[0u32; 12]; states.len()];
    for group in 0..3 {
        let view = format!("{snippet}{}", state_view(states, call, pick, group));
        let module = bind::module(&view, ViewOutput::Words).unwrap();
        let image = render::headless::render(
            gpu.device(),
            gpu.queue(),
            &Draw {
                module: &module,
                entry: bind::ENTRY,
                layouts: &bound.layout_refs(),
                groups: &bound.group_refs(),
            },
            Target {
                width: n,
                height: 1,
                format: wgpu::TextureFormat::Rgba32Uint,
            },
        )
        .unwrap();
        for (x, o) in out.iter_mut().enumerate() {
            for (k, w) in image.words(x as u32, 0).into_iter().enumerate() {
                o[4 * group + k] = w;
            }
        }
    }
    out
}

/// The descriptor's fields as `frag_describe` returns them, as words.
const DESCRIBE_PICK: [&str; 12] = [
    "o.m0",
    "o.m1",
    "o.m2",
    "o.q_mass",
    "o.rho_mag",
    "o.lambda_mag",
    "o.rho_ratio",
    "o.rho_angle",
    "o.K_0",
    "o.V_0",
    "o.virial_ratio",
    "o.r_min_pair_0",
];

/// The encode's words as `frag_encode` returns them (`kernel::stage::encode`'s order).
const ENCODE_PICK: [&str; 12] = [
    "bitcast<u32>(o.z[0])",
    "bitcast<u32>(o.z[1])",
    "bitcast<u32>(o.z[2])",
    "bitcast<u32>(o.z[3])",
    "bitcast<u32>(o.z[4])",
    "bitcast<u32>(o.z[5])",
    "bitcast<u32>(o.z[6])",
    "bitcast<u32>(o.z[7])",
    "bitcast<u32>(o.alpha)",
    "bitcast<u32>(o.beta)",
    "bitcast<u32>(o.lambda)",
    "o.notices",
];

const MASSES: [f32; 3] = [0.25, 0.25, 0.5];
const P: [[f32; 2]; 3] = [[0.1, -0.2], [0.3, 0.1], [-0.2, 0.05]];

/// States at the poles, every sign of the zeros: bodies 0 and 1 coincident (`ρ̃ = 0`, the collision pole); body 2 at
/// the inner pair's centre (`λ̃ = 0`, the `α = 0` pole); and body 2 on the `−x` half-axis of the inner pair's centre
/// (`λ̃_y = ±0`, `λ̃_x < 0`).
fn pole_states() -> Vec<State> {
    let mut out = Vec::new();
    for z in [0.0f32, -0.0] {
        out.push(State {
            m: MASSES,
            r: [[z, 0.5], [z, 0.5], [0.0, -0.5]],
            p: P,
        });
        out.push(State {
            m: MASSES,
            r: [[-1.0, z], [1.0, -z], [z, -z]],
            p: P,
        });
        out.push(State {
            m: MASSES,
            r: [[-1.0, 0.0], [1.0, 0.0], [-0.75, z]],
            p: P,
        });
    }
    out
}

/// One fragment word against the CPU's at f32: bit for bit where either is non-finite (the pole's value, R-436), else
/// within REQ-DEC-043's factor of the CPU's (the two compilations of one formula may differ in their last bits).
fn agree(gpu: u32, cpu: u32) -> bool {
    let (g, c) = (f32::from_bits(gpu), f32::from_bits(cpu));
    if presets::non_finite_bits(g) || presets::non_finite_bits(c) {
        return gpu == cpu;
    }
    let bound =
        f64::from(agreement::FACTOR) * f64::from(f32::EPSILON) * f64::from(c.abs()).max(1.0);
    (f64::from(g) - f64::from(c)).abs() <= bound
}

/// The pole checks; `alter` stores a finite `V_0` in the fragment's descriptor at the coincident pair, the control.
fn check_poles(alter: bool) {
    let states = pole_states();
    let describe = run_states(
        &presets::describe_snippet().unwrap(),
        &states,
        "frag_describe",
        &DESCRIBE_PICK,
    );
    for (s, gpu) in states.iter().zip(&describe) {
        let mut gpu = *gpu;
        let coincident = s.r[0] == s.r[1];
        if alter && coincident {
            gpu[9] = 0;
        }
        let cpu = dp::describe32(s);
        for k in 0..12 {
            assert!(
                agree(gpu[k], cpu[k]),
                "{s:?}: the fragment's {} is {:#010x}, the CPU's pole value {:#010x}",
                DESCRIBE_PICK[k],
                gpu[k],
                cpu[k]
            );
        }
        if coincident {
            assert_eq!(
                gpu[9], 0xff80_0000,
                "{s:?}: the fragment's V_0 at a coincident pair is not −∞'s bits"
            );
        }
    }
    let encode = run_states(presets::ENCODE_WGSL, &states, "frag_encode", &ENCODE_PICK);
    for (s, gpu) in states.iter().zip(&encode) {
        let cpu = dp::encode32(s);
        for (k, what) in [(8, "α"), (9, "β"), (10, "λ")] {
            assert!(
                agree(gpu[k], cpu[k]),
                "{s:?}: the fragment encode's {what} {:#010x} is not the CPU's {:#010x}",
                gpu[k],
                cpu[k]
            );
        }
        assert_eq!(
            gpu[11], cpu[11],
            "{s:?}: the fragment encode's notices {:#010x} are not the CPU's {:#010x}",
            gpu[11], cpu[11]
        );
    }
}

#[test]
fn decode_preset_poles() {
    check_poles(false);
    // The shape sphere's n₀ pole, (½, ½), the depth-0 cell: finite, with the CPU's tag.
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let scene = Scene::new(DecodeChart::ShapeSphere, [1, 1], 1, 0, &[[0, 0]]).unwrap();
    let w = scene.probe(gpu.device(), gpu.queue()).unwrap()[0];
    let r = dp::reference(DecodeChart::ShapeSphere, &scene.uniforms, [0.5, 0.5]).unwrap();
    assert_eq!(w[15], dp::tag(&r.out), "the pole's tag differs");
    for (k, word) in w[..15].iter().enumerate() {
        assert!(
            !presets::non_finite_bits(f32::from_bits(*word)),
            "the shape sphere's pole decodes word {k} non-finite"
        );
    }
}

negative_control!(
    decode_preset_poles,
    "a V_0 at the collision pole stored as anything but −∞'s bits must be found",
    expected = "the CPU's pole value",
    check_poles(true)
);
