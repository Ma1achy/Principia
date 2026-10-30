//! The ROUNDTRIP preset's golden suite, `fixtures/golden/roundtrip-preset/` (TASK-M2-26; REQ-RENDER-025), and its
//! residual twin: each chart's reference image is the CPU trip (`validation::roundtrip_preset::expected`), the f32
//! decode, encode and free decode of the one Rust source at every pixel's chart point coloured by the preset's CPU twin
//! (`render::presets::roundtrip::colour`) and quantised as the golden runner quantises (half to even). `cargo xtask
//! golden roundtrip-preset` renders the preset on the GPU through `golden_harness` and holds it to these references
//! exactly (REQ-VAL-138).
//!
//! - `roundtrip_preset_golden_references_are_the_cpu_trip`: each case's `case.json` names its harness scene and size,
//!   and its `reference.png` is the CPU trip's image, every channel at least `GOLDEN_MARGIN` of an 8-bit step from a
//!   rounding tie and no sample at a clamp boundary. Control: one chart's reference held to the other's CPU trip
//!   differs. A missing or stale reference is written under the test's target directory, named in the failure, for
//!   review; the suite's references change only with a row in `fixtures/golden/BASELINES.md` (R-110).
//! - `roundtrip_preset_golden_scenes_show_the_view`: the latent scene is untagged and within `ε_phys` throughout, the
//!   view dark everywhere; the `(L_z, E)` scene holds untagged samples within `ε_phys` and tagged feasibility samples,
//!   the edge between them at `s = 1`. Control: the `(L_z, E)` chart over the unit square alone shows no tag.
//! - `roundtrip_preset_residual_is_phys`: the preset's residual twin (`render::presets::roundtrip::residual`) is
//!   `validation::roundtrip::phys`, bit for bit, on states that differ in one value at a time, each body's mass,
//!   position and momentum, at extents above and below 1; `State::finite` finds a non-finite mass, position or
//!   momentum. Control: the residual measured against the other state's extents.

use std::fs;
use std::path::{Path, PathBuf};

use kernel::decode::Ic;
use render::presets::roundtrip::{residual, Class, State};
use render::presets::DecodeChart;
use validation::decode_preset::{quantise, GOLDEN_MARGIN};
use validation::golden_scene::tie_margin;
use validation::negative_control;
use validation::roundtrip_preset::{
    clamp_ambiguous, expected, golden, golden_name, twins, CHARTS, GOLDEN_DEPTH, GOLDEN_SIDE,
    GOLDEN_SUITE,
};

fn suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(GOLDEN_SUITE)
}

/// The 8-bit RGB pixels of the PNG at `path`, with its size.
fn read_png(path: &Path) -> Result<(u32, u32, Vec<[u8; 3]>), String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if info.color_type != png::ColorType::Rgb || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not 8-bit RGB", path.display()));
    }
    let pixels = buf[..info.buffer_size()]
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    Ok((info.width, info.height, pixels))
}

/// Writes `pixels`, `width` × `height`, as an 8-bit RGB PNG at `path`.
fn write_png(path: &Path, width: u32, height: u32, pixels: &[[u8; 3]]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels.as_flattened()).unwrap();
}

/// `case`'s reference held to `chart`'s CPU trip.
fn check_reference(case: DecodeChart, chart: DecodeChart) {
    let scene = golden(chart).unwrap_or_else(|e| panic!("{e}"));
    let colours = expected(&scene).unwrap();
    let margin = tie_margin(&colours);
    assert!(
        margin >= GOLDEN_MARGIN,
        "{chart:?}: the CPU trip's image is {margin} of an 8-bit step from a tie"
    );
    let ambiguous = clamp_ambiguous(&scene).unwrap();
    assert!(
        ambiguous.is_empty(),
        "{chart:?}: samples at a clamp boundary: {ambiguous:?}"
    );
    let (width, height) = scene.size();
    assert_eq!((width, height), (GOLDEN_SIDE[0], GOLDEN_SIDE[1]));
    let want = quantise(&colours);
    let dir = suite().join(case.section());
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    assert_eq!(config["render"]["harness"], golden_name(case));
    assert_eq!(config["render"]["width"], width);
    assert_eq!(config["render"]["height"], height);
    assert_eq!(config["tolerance"], "REQ-VAL-138");
    let got = read_png(&dir.join("reference.png"));
    if got.as_ref().ok() != Some(&(width, height, want.clone())) {
        let candidate = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(GOLDEN_SUITE)
            .join(chart.section())
            .join("reference.png");
        write_png(&candidate, width, height, &want);
        panic!(
            "{case:?}'s reference is not {chart:?}'s CPU trip ({}); the CPU trip's image is written to {}",
            match got {
                Ok((w, h, p)) => format!(
                    "{w} × {h}, {} pixels differ",
                    p.iter().zip(&want).filter(|(a, b)| a != b).count()
                ),
                Err(e) => e,
            },
            candidate.display()
        );
    }
}

#[test]
fn roundtrip_preset_golden_references_are_the_cpu_trip() {
    for chart in CHARTS {
        check_reference(chart, chart);
    }
}

negative_control!(
    roundtrip_preset_golden_references_are_the_cpu_trip,
    "a reference held to another chart's CPU trip must differ",
    expected = "'s reference is not",
    check_reference(DecodeChart::InvariantLE, DecodeChart::Latent)
);

/// The classes of `chart`'s golden scene, its `u` extent replaced by `extent` where given, and its worst untagged ratio.
fn classes(chart: DecodeChart, extent: Option<f64>) -> (Vec<Class>, f64) {
    let mut scene = golden(chart).unwrap();
    if let Some(e) = extent {
        let n = GOLDEN_SIDE[0] * GOLDEN_SIDE[1];
        for q in 0..n {
            let [i, j] = [q % GOLDEN_SIDE[0], q / GOLDEN_SIDE[0]];
            let cell = [
                ((f64::from(i) + 0.5) * e * f64::from(GOLDEN_DEPTH).exp2()
                    / f64::from(GOLDEN_SIDE[0])) as u64,
                ((f64::from(j) + 0.5) * f64::from(GOLDEN_DEPTH).exp2() / f64::from(GOLDEN_SIDE[1]))
                    as u64,
            ];
            scene.set.place(q, GOLDEN_DEPTH, cell);
        }
    }
    let t = twins(&scene).unwrap();
    let worst = t
        .iter()
        .filter(|t| t.class == Class::Untagged)
        .map(|t| t.ratio())
        .fold(0.0f64, f64::max);
    (t.iter().map(|t| t.class).collect(), worst)
}

fn check_scenes(le_extent: Option<f64>) {
    let (latent, worst) = classes(DecodeChart::Latent, None);
    assert!(
        latent.iter().all(|c| *c == Class::Untagged),
        "the latent scene holds a tag"
    );
    assert!(worst <= 1.0, "the latent scene is {worst} ε_phys");
    let (le, worst) = classes(DecodeChart::InvariantLE, le_extent);
    assert!(worst <= 1.0, "the (L_z, E) scene is {worst} ε_phys");
    let w = GOLDEN_SIDE[0] as usize;
    let scale = le_extent.unwrap_or(1.5);
    for (q, c) in le.iter().enumerate() {
        let s = (((q % w) as f64) + 0.5) * scale / w as f64;
        let want = if s > 1.0 {
            Class::Feasibility
        } else {
            Class::Untagged
        };
        assert_eq!(*c, want, "the (L_z, E) scene at s = {s}");
    }
    assert!(
        le.contains(&Class::Feasibility),
        "the (L_z, E) scene shows no tag"
    );
}

#[test]
fn roundtrip_preset_golden_scenes_show_the_view() {
    check_scenes(None);
}

negative_control!(
    roundtrip_preset_golden_scenes_show_the_view,
    "the (L_z, E) chart over the unit square alone must show no tag",
    expected = "shows no tag",
    check_scenes(Some(1.0))
);

// ── The residual ────────────────────────────────────────────────────────────────────────────────────────────────────

fn state(scale: f64) -> State {
    State {
        m: [0.2, 0.3, 0.5],
        r: [
            [-0.4 * scale, 0.1 * scale],
            [0.9 * scale, -0.2 * scale],
            [-0.3 * scale, 0.05 * scale],
        ],
        p: [
            [0.15 * scale, -0.6 * scale],
            [-0.35 * scale, 0.2 * scale],
            [0.2 * scale, 0.4 * scale],
        ],
    }
}

fn ic(s: &State) -> Ic<f64> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// Every state that differs from `x` in one value by `d`.
fn moved(x: &State, d: f64) -> Vec<State> {
    let mut out = Vec::new();
    for i in 0..3 {
        let mut y = *x;
        y.m[i] += d;
        out.push(y);
        for k in 0..2 {
            let mut y = *x;
            y.r[i][k] += d;
            out.push(y);
            let mut y = *x;
            y.p[i][k] -= d;
            out.push(y);
        }
    }
    out
}

fn check_residual(res: fn(&State, &State) -> f64) {
    for scale in [0.5, 3.0] {
        let x = state(scale);
        assert_eq!(res(&x, &x), 0.0, "a state's residual to itself");
        for y in moved(&x, 0.25) {
            let want = validation::roundtrip::phys(&ic(&x), &ic(&y));
            let got = res(&x, &y);
            assert!(want > 0.0);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "residual {got} is not phys's {want} at scale {scale}"
            );
            assert_eq!(residual(&x, &y), want);
        }
    }
}

#[test]
fn roundtrip_preset_residual_is_phys() {
    check_residual(residual);
    let x = state(1.0);
    assert!(x.finite());
    for (k, bad) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
        .into_iter()
        .enumerate()
    {
        let mut y = x;
        y.m[k] = bad;
        assert!(!y.finite(), "a non-finite mass");
        let mut y = x;
        y.r[k][1] = bad;
        assert!(!y.finite(), "a non-finite position");
        let mut y = x;
        y.p[2 - k][0] = bad;
        assert!(!y.finite(), "a non-finite momentum");
    }
}

negative_control!(
    roundtrip_preset_residual_is_phys,
    "the residual against the other state's extents must be found",
    expected = "is not phys's",
    check_residual(|x, y| residual(y, x))
);
