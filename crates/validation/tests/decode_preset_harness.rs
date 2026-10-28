//! The DECODE preset's harness (`validation::decode_preset`; TASK-M2-25) held to what it stands for:
//!
//! - `decode_preset_harness_raster`: the chart point and within-tile coordinate the harness's CPU twin decodes each
//!   pixel at (`Scene::pixel_point`, `Scene::tile_uv`) are the fragment's own `ctx.chart.slice_uv` and `ctx.tile.uv`, read
//!   back from the GPU, within one f32 ulp. Control: a twin a pixel off.
//! - `decode_preset_harness_probe`: the probe reads back, per sample, its chart point, the f64 reference's tag and its
//!   values within REQ-DEC-043's factor, `Φ`'s `z` and its latent flag. Control: the reference taken at another sample.
//! - `decode_preset_harness_render`: `Scene::render` against `Scene::expected` on one golden scene. Control: another
//!   chart's twin.
//! - `decode_preset_harness_cpu`: the f32 state's `Ic`, the descriptor's and encode's words in their orders, the values'
//!   order, the tag words, a degenerate tag, the shape sphere's held momenta, and the golden names. Control: a state
//!   read with its bodies swapped.

use render::bind::ViewOutput;
use render::presets::DecodeChart;
use validation::decode_f32_eps::error;
use validation::decode_preset::{self as dp, Scene, State};
use validation::gpu::GpuHarness;
use validation::negative_control;

/// A view writing the fragment's `slice_uv` and `tile.uv` bits.
const RASTER: &str = "
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {
    return vec4<u32>(bitcast<u32>(l.chart.slice_uv.x), bitcast<u32>(l.chart.slice_uv.y),
                     bitcast<u32>(l.tile.uv.x), bitcast<u32>(l.tile.uv.y));
}
";

/// Whether two f32 are within one ulp.
fn ulp(a: f32, b: f32) -> bool {
    a == b || (a.to_bits() as i64 - b.to_bits() as i64).abs() <= 1
}

/// The twin's raster against the GPU's; `dx` moves the twin's pixel (the control).
fn check_raster(dx: u32) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let scene = dp::golden(DecodeChart::Latent).unwrap();
    let image = scene
        .draw_view(gpu.device(), gpu.queue(), RASTER, ViewOutput::Words)
        .unwrap();
    let (width, height) = scene.size();
    for y in 0..height {
        for x in 0..width - dx {
            let w = image.words(x, y);
            let p = scene.pixel_point(x + dx, y);
            let t = scene.tile_uv(x + dx, y);
            for k in 0..2 {
                assert!(
                    ulp(f32::from_bits(w[k]), p[k]),
                    "pixel ({x}, {y}): the twin's chart point {p:?} is not the fragment's slice_uv"
                );
                assert!(
                    ulp(f32::from_bits(w[2 + k]), t[k] as f32),
                    "pixel ({x}, {y}): the twin's tile uv {t:?} is not the fragment's"
                );
            }
        }
    }
}

#[test]
fn decode_preset_harness_raster() {
    check_raster(0);
}

negative_control!(
    decode_preset_harness_raster,
    "a twin a pixel off must be found",
    expected = "is not the fragment's",
    check_raster(1)
);

/// The probe against the references; `other` takes each reference at the next sample (the control).
fn check_probe(other: bool) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for chart in DecodeChart::ALL {
        let scene = Scene::lattice(chart, [2, 2], 3).unwrap();
        let words = scene.probe(gpu.device(), gpu.queue()).unwrap();
        assert_eq!(words.len(), 4, "{chart:?}: the probe's samples");
        // The lattice's cells: sample (i, j) at cell ⌊(i + ½)·2³/2⌋, so its point is (cell + ½)/8.
        assert_eq!(
            scene.point(0),
            [0.3125, 0.3125],
            "{chart:?}: the lattice's first point"
        );
        assert_eq!(
            scene.point(3),
            [0.8125, 0.8125],
            "{chart:?}: the lattice's last point"
        );
        for (q, w) in (0u32..).zip(&words) {
            let point = scene.point(q);
            assert_eq!(
                [w[37], w[38]],
                point.map(f32::to_bits),
                "{chart:?} {q}: the probe's point"
            );
            let at = if other {
                scene.point((q + 1) % 4)
            } else {
                point
            };
            let r = dp::reference(chart, &scene.uniforms, at).unwrap();
            assert_eq!(w[15], dp::tag(&r.out), "{chart:?} {q}: the tag");
            assert_eq!(w[24] != 0, r.z_latent, "{chart:?} {q}: the latent flag");
            let v = dp::values(&r.out);
            for k in 0..15 {
                let e = error(dp::f(w[k]), v[k]);
                assert!(
                    e <= 256.0,
                    "{chart:?} {q}: value {k} is {e} eps off the reference"
                );
            }
            for k in 0..8 {
                assert!(
                    error(dp::f(w[16 + k]), r.z[k]) <= 256.0,
                    "{chart:?} {q}: z{k} is off the reference"
                );
            }
        }
    }
}

#[test]
fn decode_preset_harness_probe() {
    check_probe(false);
}

negative_control!(
    decode_preset_harness_probe,
    "a reference taken at another sample must be found",
    expected = "off the reference",
    check_probe(true)
);

/// `Scene::render` against `Scene::expected`, the twin taken of `twin`'s scene.
fn check_render(twin: DecodeChart) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let scene = dp::golden(DecodeChart::MassSimplex).unwrap();
    let got = scene.render(gpu.device(), gpu.queue()).unwrap();
    let want = dp::golden(twin).unwrap().expected().unwrap();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g[3], 1.0);
        for k in 0..3 {
            assert!(
                (f64::from(g[k]) - w[k]).abs() <= 1e-5,
                "the render's {g:?} is not the twin's {w:?}"
            );
        }
    }
}

#[test]
fn decode_preset_harness_render() {
    check_render(DecodeChart::MassSimplex);
}

negative_control!(
    decode_preset_harness_render,
    "another chart's twin must differ",
    expected = "is not the twin's",
    check_render(DecodeChart::Latent)
);

/// The CPU helpers' orders; `swap` swaps bodies 0 and 1 in the state read (the control).
fn check_cpu(swap: bool) {
    let s = State {
        m: [0.2, 0.3, 0.5],
        r: [[-0.9, 0.11], [0.7, -0.23], [0.31, 0.17]],
        p: [[0.13, -0.41], [0.29, 0.07], [-0.42, 0.34]],
    };
    let mut ic = s.ic();
    if swap {
        ic.m.swap(0, 1);
        ic.r.swap(0, 1);
        ic.p.swap(0, 1);
    }
    assert_eq!(
        (ic.m, ic.r, ic.p),
        (s.m, s.r, s.p),
        "the state's Ic is not its (m, r, p)"
    );
    let d = kernel::decode::descriptor::describe(&ic);
    assert_eq!(
        dp::describe32(&s),
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
            d.r_min_pair_0
        ]
        .map(f32::to_bits)
    );
    let e = kernel::encode::encode::<kernel::generated::links::Defaults, f32>(&ic);
    let w = dp::encode32(&s);
    assert_eq!(w[..8], e.z.z.map(f32::to_bits));
    assert_eq!(
        w[8..],
        [
            e.blocks.alpha.to_bits(),
            e.blocks.beta.to_bits(),
            e.report.lambda.to_bits(),
            kernel::stage::encode::notices(&e.report)
        ]
    );
    let r = dp::reference(
        DecodeChart::Latent,
        &dp::uniforms(DecodeChart::Latent),
        [0.3, 0.6],
    )
    .unwrap();
    let v = dp::values(&r.out);
    assert_eq!(v[..3], r.out.ic.m);
    assert_eq!(v[3..5], r.out.ic.r[0]);
    assert_eq!(v[9..11], r.out.ic.p[0]);
    assert_eq!(v[23], r.out.descriptor.K_0);
    assert_eq!(dp::tag(&r.out), 0, "a sane decode is tagged");
    let mut degenerate = r.out;
    degenerate.degenerate = Some(kernel::decode::degenerate::Degenerate::EnergyInfeasible);
    assert_eq!(dp::tag(&degenerate), 2, "a degenerate decode's tag");
    assert_eq!(dp::f(0.25f32.to_bits()), 0.25);
    use kernel::stage::decode::UniformChart;
    let sphere = kernel::chart::ShapeSphere::<f32>::from_uniform(
        &dp::uniforms(DecodeChart::ShapeSphere).chart,
    );
    assert_eq!(
        sphere.z_mom,
        [0.2, -0.1, 0.3, 0.15],
        "the shape sphere's held momenta"
    );
    for chart in DecodeChart::ALL {
        assert_eq!(dp::golden_chart(&dp::golden_name(chart)), Some(chart));
        assert!(dp::golden_name(chart).starts_with("decode-preset-"));
    }
    assert_eq!(dp::golden_chart("decode-preset-none"), None);
    assert_eq!(
        dp::uniform_bytes(&dp::uniforms(DecodeChart::Latent)).len(),
        128
    );
}

#[test]
fn decode_preset_harness_cpu() {
    check_cpu(false);
}

negative_control!(
    decode_preset_harness_cpu,
    "a state read with its bodies swapped must be found",
    expected = "the state's Ic is not its (m, r, p)",
    check_cpu(true)
);
