//! The ROUNDTRIP preset on the GPU (TASK-M2-26; REQ-RENDER-025; inverse-encode contract Part 8; R-297): its trip
//! `z → D → E → D`, recomputed wholly in the fragment and compiled as the backend compiles it (fast-math on Metal), read
//! back as words over three scenes, and its WGSL class and colour against the CPU twins.
//!
//! - `roundtrip_preset_residual_within_eps_phys`: over the latent chart's interior, a latent chart framed by clamp
//!   boundaries (`validation::roundtrip_preset::clamp_frame`) and the `(L_z, E)` chart past its edge `s = 1`, 64 × 64
//!   samples each, every untagged residual is within `ε_phys` (REQ-ENC-024), and each sample's class is the CPU
//!   trip's, read off the trip's own reports; a sample at a clamp boundary, where rounding decides the clamp bit, may
//!   be untagged or clamped. The framed chart shows clamps and the `(L_z, E)` chart feasibility tags. Control: a trip
//!   whose free decode reads the encode's `z` with one momentum control moved by 0.05 is past `ε_phys`.
//! - `roundtrip_preset_shape_sphere_both_hemispheres`: over the shape sphere chart's two hemispheres (R-544), each
//!   sample's class is the CPU trip's, none is a mirror tie, and every lower-hemisphere sample the encode mirrors, its
//!   decode off `C`'s section, is untagged and within `ε_phys` against `C(D(z))` (inverse-encode Part 4, Part 8's T2).
//!   Control: the trip measured against `D(z)` with every mirror tagged, as before; the lower hemisphere is tagged.
//! - `roundtrip_preset_golden_renders_its_reference`: each golden scene's render, through the harness and through the
//!   `golden_harness` binary, quantised as the golden runner quantises, is its CPU trip's image, pixel for pixel, and its
//!   words view's classes are the CPU trip's, sample for sample; and
//!   each scene's name finds its chart and no other's. Control: one chart's render held to the other's CPU trip.
//! - `roundtrip_preset_wgsl_is_the_twin`: the view's `roundtrip_class` on every case of the class table, its notices
//!   read through `roundtrip_tie_notices` on a chart on `C`'s section and off it, and its `roundtrip_colour` at the
//!   brightness's breakpoints, each class, are the CPU twins' (`Class::of`, `tie_notices`, `colour`).
//!   Control: a class function with the clamp test cut.

use render::bind::ViewOutput;
use render::presets::roundtrip::{self, colour, Class};
use render::presets::DecodeChart;
use validation::decode_preset::quantise;
use validation::decode_preset::Scene;
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::roundtrip_preset::{
    clamp_ambiguous, clamp_frame, expected, golden, golden_chart, golden_name, render, trips,
    trips_view, twins, CHARTS,
};

/// A scene of `side` × `side` one-pixel samples over `chart` at `uniforms`, its `u` extent `extent`.
fn scene(
    chart: DecodeChart,
    uniforms: Option<kernel::stage::decode::DecodeUniforms>,
    extent: f64,
) -> Scene {
    let side = 64u32;
    let depth = 20u32;
    let cells: Vec<[u64; 2]> = (0..side)
        .flat_map(|j| (0..side).map(move |i| [i, j]))
        .map(|ij| {
            let span = [extent, 1.0];
            [0, 1].map(|k| {
                ((f64::from(ij[k]) + 0.5) * span[k] * f64::from(depth).exp2() / f64::from(side))
                    as u64
            })
        })
        .collect();
    let mut s = Scene::new(chart, [side, side], 1, depth, &cells).unwrap();
    if let Some(u) = uniforms {
        s.uniforms = u;
    }
    s
}

fn check_trips(view: &str) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let scenes = [
        ("latent", scene(DecodeChart::Latent, None, 1.0), None),
        (
            "latent framed by clamps",
            scene(DecodeChart::Latent, Some(clamp_frame()), 1.0),
            Some(Class::Clamp),
        ),
        (
            "(L_z, E) past s = 1",
            scene(DecodeChart::InvariantLE, None, 1.5),
            Some(Class::Feasibility),
        ),
    ];
    let mut report = Vec::new();
    for (name, s, shows) in &scenes {
        let gpu_trips = trips_view(s, gpu.device(), gpu.queue(), view).unwrap();
        let cpu = twins(s).unwrap();
        let ambiguous: Vec<u32> = clamp_ambiguous(s)
            .unwrap()
            .into_iter()
            .map(|a| a.0)
            .collect();
        let mut worst = 0.0f64;
        let mut counts = [0usize; 5];
        for (q, (g, c)) in gpu_trips.iter().zip(&cpu).enumerate() {
            counts[g.class as usize] += 1;
            if g.class == Class::Untagged {
                assert!(
                    g.ratio <= 1.0,
                    "{name}: sample {q} at {:?} is {} ε_phys, past ε_phys, untagged",
                    s.point(q as u32),
                    g.ratio
                );
                worst = worst.max(g.ratio);
            }
            if ambiguous.contains(&(q as u32)) {
                assert!(
                    matches!(g.class, Class::Untagged | Class::Clamp),
                    "{name}: sample {q} at a clamp boundary is {:?}",
                    g.class
                );
            } else {
                assert_eq!(
                    (g.class, g.notices, g.tags),
                    (c.class, c.notices, c.tags),
                    "{name}: sample {q} at {:?}: the fragment's trip is not the CPU's",
                    s.point(q as u32)
                );
            }
        }
        if let Some(tag) = shows {
            assert!(counts[*tag as usize] > 0, "{name} shows no {tag:?}");
        }
        report.push(format!(
            "{name}: classes {counts:?}, worst untagged {worst:.4} ε_phys"
        ));
    }
    println!(
        "roundtrip_preset_residual_within_eps_phys: {}",
        report.join("; ")
    );
}

#[test]
fn roundtrip_preset_residual_within_eps_phys() {
    check_trips(roundtrip::words().view());
}

negative_control!(
    roundtrip_preset_residual_within_eps_phys,
    "a free decode of the encode's z with a momentum control moved by 0.05 must be past ε_phys",
    expected = "past ε_phys",
    check_trips(&{
        let view = roundtrip::words().view().to_owned();
        let moved = view.replace(
            "let back = frag_decode_free(e.z);",
            "var zm = e.z;\n    zm[2] = zm[2] + 0.05;\n    let back = frag_decode_free(zm);",
        );
        assert_ne!(moved, view, "the control's edit did not apply");
        moved
    })
);

// ── The shape sphere's two hemispheres (R-544) ──────────────────────────────────────────────────────────────────────

/// The shape sphere chart over both hemispheres, 64 × 64 samples of the spherical map, `v < ½` the lower: every trip is
/// the CPU twin's, no sample is tagged mirror tie, every untagged residual is within `ε_phys`, and every lower-hemisphere
/// sample the encode mirrors, its decode off `C`'s section, is untagged and within `ε_phys`, measured against `C(D(z))`.
fn check_shape_sphere(view: &str) {
    use roundtrip::MIRRORED;
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let s = scene(DecodeChart::ShapeSphere, None, 1.0);
    let gpu_trips = trips_view(&s, gpu.device(), gpu.queue(), view).unwrap();
    let cpu = twins(&s).unwrap();
    let ambiguous: Vec<u32> = clamp_ambiguous(&s)
        .unwrap()
        .into_iter()
        .map(|a| a.0)
        .collect();
    let (mut lower_mirrored, mut upper, mut worst) = (0usize, 0usize, 0.0f64);
    let mut counts = [0usize; 5];
    for (q, (g, c)) in gpu_trips.iter().zip(&cpu).enumerate() {
        let p = s.point(q as u32);
        counts[g.class as usize] += 1;
        if p[1] < 0.5 && g.notices & MIRRORED != 0 && g.class != Class::Clamp {
            lower_mirrored += 1;
            assert_eq!(
                g.class,
                Class::Untagged,
                "lower hemisphere sample {q} at {p:?}: mirrored by C and tagged, not held to ε_phys"
            );
            assert!(
                g.ratio <= 1.0,
                "lower hemisphere sample {q} at {p:?}: {} ε_phys against C(D(z)), past ε_phys",
                g.ratio
            );
        }
        if p[1] > 0.5 {
            upper += 1;
        }
        assert_ne!(
            g.class,
            Class::MirrorTie,
            "sample {q} at {p:?}: a mirror tie on the shape sphere, off R-82's band"
        );
        if g.class == Class::Untagged {
            worst = worst.max(g.ratio);
            assert!(
                g.ratio <= 1.0,
                "sample {q} at {p:?} is {} ε_phys, past ε_phys, untagged",
                g.ratio
            );
        }
        if !ambiguous.contains(&(q as u32)) {
            assert_eq!(
                (g.class, g.notices, g.tags),
                (c.class, c.notices, c.tags),
                "sample {q} at {p:?}: the fragment's trip is not the CPU's"
            );
        }
    }
    assert!(
        lower_mirrored * 4 > gpu_trips.len() && upper * 4 > gpu_trips.len(),
        "the scene covers too little of either hemisphere: {lower_mirrored} lower mirrored, {upper} upper"
    );
    println!(
        "roundtrip_preset_shape_sphere_both_hemispheres: classes {counts:?}, {lower_mirrored} lower-hemisphere samples \
         mirrored by C, worst untagged {worst:.4} ε_phys"
    );
}

#[test]
fn roundtrip_preset_shape_sphere_both_hemispheres() {
    check_shape_sphere(roundtrip::words().view());
}

negative_control!(
    roundtrip_preset_shape_sphere_both_hemispheres,
    "measured against D(z) with every mirror tagged, as before R-544's reference, the lower hemisphere is tagged",
    expected = "lower hemisphere",
    check_shape_sphere(&{
        let view = roundtrip::words().view().to_owned();
        let undone = view
            .replace("if (!d.on_section) {", "if (false) {")
            .replace(
                "roundtrip_tie_notices(e.notices, d.on_section)",
                "e.notices",
            );
        assert_ne!(view, undone, "the control's edit did not apply");
        undone
    })
);

// ── The golden scenes on the GPU ───────────────────────────────────────────────────────────────────────────────────

/// Each golden scene's render against `against`'s chart's CPU trip (its own but in the control).
fn check_golden(against: fn(DecodeChart) -> DecodeChart) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    for chart in CHARTS {
        assert_eq!(
            golden_chart(&golden_name(chart)),
            Some(chart),
            "{chart:?}'s scene name"
        );
        let scene = golden(chart).unwrap();
        let classes: Vec<Class> = trips(&scene, gpu.device(), gpu.queue())
            .unwrap()
            .iter()
            .map(|t| t.class)
            .collect();
        let cpu: Vec<Class> = twins(&golden(against(chart)).unwrap())
            .unwrap()
            .iter()
            .map(|t| t.class)
            .collect();
        assert_eq!(
            classes, cpu,
            "{chart:?}: the rendered golden is not the CPU trip's, its classes differ"
        );
        let pixels = render(&scene, gpu.device(), gpu.queue()).unwrap();
        let got = quantise(
            &pixels
                .iter()
                .map(|p| [0, 1, 2].map(|k| f64::from(p[k])))
                .collect::<Vec<_>>(),
        );
        let want = quantise(&expected(&golden(against(chart)).unwrap()).unwrap());
        assert_eq!(got.len(), want.len());
        let differ = got.iter().zip(&want).filter(|(a, b)| a != b).count();
        assert_eq!(
            differ, 0,
            "{chart:?}: the rendered golden is not the CPU trip's, {differ} pixels differ"
        );
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_golden_harness"))
            .args(["--scene", &golden_name(chart)])
            .output()
            .expect("run golden_harness");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let (w, h) = scene.size();
        assert_eq!(
            out.stdout,
            validation::golden_scene::encode_image(w, h, &pixels),
            "{chart:?}: golden_harness's image is not the harness's render"
        );
    }
    for name in [
        "roundtrip-preset-none",
        "decode-preset-latent",
        "roundtrip-preset-shape_sphere",
    ] {
        assert_eq!(golden_chart(name), None, "`{name}` names a ROUNDTRIP scene");
    }
}

#[test]
fn roundtrip_preset_golden_renders_its_reference() {
    check_golden(|c| c);
}

negative_control!(
    roundtrip_preset_golden_renders_its_reference,
    "a render held to the other chart's CPU trip must differ",
    expected = "the rendered golden is not the CPU trip's",
    check_golden(|c| match c {
        DecodeChart::Latent => DecodeChart::InvariantLE,
        _ => DecodeChart::Latent,
    })
);

// ── The class and colour in WGSL ───────────────────────────────────────────────────────────────────────────────────

/// `(invalid, tag, tag_free, notices)` the class table covers, each class's tests at their bits and neighbours; each
/// read on a chart on `C`'s section (pixels 0 to 14) and off it (15 to 29), through `roundtrip_tie_notices`.
const CLASS_CASES: [(bool, u32, u32, u32); 15] = [
    (true, 4, 4, 0xffff),
    (true, 0, 0, 0),
    (false, 4, 0, 0xff03),
    (false, 0, 1, 0xff03),
    (false, 0, 0, 0x0003),
    (false, 0, 0, 0x0102),
    (false, 0, 0, 0x8000),
    (false, 0, 0, 0x0001),
    (false, 0, 0, 0x0002),
    (false, 0, 0, 0x0006),
    (false, 0, 0, 0x0004),
    (false, 0, 0, 0x00fc),
    (false, 0, 0, 0x0001_0000),
    (false, 0, 0, 0x0302),
    (false, 0, 0, 0),
];

/// The ratios the colour is checked at: within, at and just past `ε_phys`, mid-span, full scale and past it.
const RATIOS: [f32; 6] = [0.5, 1.0, 1.001, 4096.0, 16_777_216.0, 1.0e9];

/// A view of pixel `x`: its class case's `roundtrip_class` and its colour case's `roundtrip_colour`, by `x`, as words.
fn class_view(snippet: &str) -> String {
    let cases: Vec<String> = CLASS_CASES
        .iter()
        .map(|(i, t, f, n)| format!("vec4<u32>({}u, {t}u, {f}u, {n}u)", u32::from(*i)))
        .collect();
    let ratios: Vec<String> = RATIOS.iter().map(|r| format!("{r:?}")).collect();
    format!(
        "{snippet}
const CLASS_CASES = array<vec4<u32>, {n}>({cases});
const RATIOS = array<f32, {m}>({ratios});
fn view(rc: RenderContext, l: Lanes) -> vec4<u32> {{
    let x = u32(l.screen.pixel.x);
    let c = CLASS_CASES[x % {n}u];
    let cls = roundtrip_class(c.x != 0u, c.y, c.z, roundtrip_tie_notices(c.w, x < {n}u));
    var t: RoundTrip;
    t.kind = x % 5u;
    t.ratio = RATIOS[(x / 5u) % {m}u];
    let frag_xy = vec2<f32>(l.screen.pixel) + vec2<f32>(0.5);
    let rgb = roundtrip_colour(t, frag_xy);
    return vec4<u32>(cls, bitcast<u32>(rgb.x), bitcast<u32>(rgb.y), bitcast<u32>(rgb.z));
}}
",
        n = CLASS_CASES.len(),
        cases = cases.join(", "),
        m = RATIOS.len(),
        ratios = ratios.join(", "),
    )
}

fn check_wgsl(snippet: &str) {
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let width = 30u32;
    let s = Scene::lattice(DecodeChart::Latent, [width, 1], 10).unwrap();
    let image = s
        .draw_view(
            gpu.device(),
            gpu.queue(),
            &class_view(snippet),
            ViewOutput::Words,
        )
        .unwrap();
    for x in 0..width {
        let w = image.words(x, 0);
        let (invalid, tag, tag_free, notices) = CLASS_CASES[x as usize % CLASS_CASES.len()];
        let on_section = (x as usize) < CLASS_CASES.len();
        let want = Class::of(
            !invalid,
            tag,
            tag_free,
            roundtrip::tie_notices(notices, on_section),
        );
        assert_eq!(
            Class::from_code(w[0]),
            Some(want),
            "the WGSL class of (invalid {invalid}, tags {tag}, {tag_free}, notices {notices:#x}, on the section \
             {on_section})"
        );
        let class = Class::from_code(x % 5).unwrap();
        let ratio = f64::from(RATIOS[(x as usize / 5) % RATIOS.len()]);
        let want = colour(class, ratio, [f64::from(x) + 0.5, 0.5]);
        let got = [w[1], w[2], w[3]].map(|b| f64::from(f32::from_bits(b)));
        assert!(
            got.iter().zip(&want).all(|(g, c)| (g - c).abs() <= 1e-5),
            "the WGSL colour of {class:?} at ratio {ratio}: {got:?}, not {want:?}"
        );
    }
}

#[test]
fn roundtrip_preset_wgsl_is_the_twin() {
    check_wgsl(&roundtrip::snippet());
}

negative_control!(
    roundtrip_preset_wgsl_is_the_twin,
    "a WGSL class with the clamp test cut must be found",
    expected = "the WGSL class",
    check_wgsl(&{
        let s = roundtrip::snippet();
        let cut = s.replace("if ((notices & 0xff01u) != 0u) {", "if (false) {");
        assert_ne!(cut, s, "the control's edit did not apply");
        cut
    })
);
