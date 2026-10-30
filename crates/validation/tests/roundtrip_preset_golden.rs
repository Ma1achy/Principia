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
//! - `roundtrip_preset_phys_of_is_phys`: `phys_of`, which render's tests hold the preset's residual twin to, is
//!   `validation::roundtrip::phys` on the same states, bit for bit, on states that differ in one value at a time.
//!   Control: its states swapped.
//! - `roundtrip_preset_twin_invalid`: a trip from a state with an infinite or NaN momentum is invalid, its ratio 0; a
//!   finite one keeps its tag and its small ratio; an invalid sample's colour is the hatch at its pixel's centre.
//!   Control: a finite momentum.
//! - `roundtrip_preset_on_section_is_the_t2_reference`: each fragment chart's `FragmentChart::ON_SECTION`, which picks
//!   the preset's reference (`D(z)` on `C`'s section, `C(D(z))` off it), is its T2 reference
//!   (`validation::roundtrip::Reference`): `D(z)` on every chart but the shape sphere (R-544). Control: the flag held to
//!   the other reference.
//! - `roundtrip_preset_clamp_band`: on the clamp-framed latent chart, `clamp_ambiguous` finds exactly the samples with
//!   a control within `CLAMP_BAND` (a quarter `ε`) of its clamp, some and fewer than a quarter of them; `at_clamp`'s
//!   band is open, its edges outside. Control: a band twice as wide.

use std::fs;
use std::path::{Path, PathBuf};

use kernel::decode::Ic;
use render::presets::roundtrip::{Class, State};
use render::presets::DecodeChart;
use validation::decode_preset::{quantise, GOLDEN_MARGIN};
use validation::golden_scene::tie_margin;
use validation::negative_control;
use validation::roundtrip_preset::{
    at_clamp, clamp_ambiguous, clamp_frame, colours, expected, golden, golden_name, phys_of,
    twin_of, twins, CHARTS, CLAMP_BAND, GOLDEN_DEPTH, GOLDEN_SIDE, GOLDEN_SUITE,
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

fn check_phys_of(of: fn(&State, &State) -> f64) {
    for scale in [0.5, 3.0] {
        let x = state(scale);
        for y in moved(&x, 0.25) {
            let want = validation::roundtrip::phys(&ic(&x), &ic(&y));
            assert!(want > 0.0);
            assert_eq!(
                of(&x, &y).to_bits(),
                want.to_bits(),
                "phys_of is not phys at scale {scale}"
            );
        }
    }
}

#[test]
fn roundtrip_preset_phys_of_is_phys() {
    check_phys_of(phys_of);
}

negative_control!(
    roundtrip_preset_phys_of_is_phys,
    "phys_of with its states swapped must be found",
    expected = "phys_of is not phys",
    check_phys_of(|x, y| phys_of(y, x))
);

// ── The twin's invalid trip and its colour ──────────────────────────────────────────────────────────────────────────

/// A decoded state with one momentum `bad`, whose trip `twin_of` must class invalid.
fn check_invalid(bad: f32) {
    let x = Ic {
        m: [0.2f32, 0.3, 0.5],
        r: [[-0.4, 0.1], [0.9, -0.2], [-0.3, 0.05]],
        p: [[bad, -0.6], [-0.35, 0.2], [0.2, 0.4]],
    };
    for on_section in [true, false] {
        let t = twin_of(&x, 0, on_section);
        assert_eq!(
            t.class,
            Class::Invalid,
            "a trip from momentum {bad} (on the section {on_section}) is not invalid"
        );
        assert_eq!(t.ratio(), 0.0, "an invalid trip's ratio");
    }
}

#[test]
fn roundtrip_preset_twin_invalid() {
    for bad in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        check_invalid(bad);
    }
    // A decoded state, on C's section, so its trip is within ε_phys; tagged as an infeasible first decode would be.
    let z = [0.3f32, -0.2, 0.1, -0.4, 0.25, 0.05, 0.2, -0.35];
    let decoded =
        kernel::decode::decode::<validation::Defaults, f32>(&kernel::decode::Latent { z }).ic;
    let fine = twin_of(&decoded, 4, true);
    assert_eq!((fine.class, fine.tags & 0xff), (Class::Feasibility, 4));
    assert!(
        fine.ratio() > 0.0 && fine.ratio() <= 1.0,
        "ratio {}",
        fine.ratio()
    );
    // An invalid sample draws the hatch at its pixel's centre, every other its class's colour.
    let scene = golden(DecodeChart::Latent).unwrap();
    let mut t = twins(&scene).unwrap();
    t[5].class = Class::Invalid;
    let c = colours(&scene, &t);
    let (w, h) = scene.size();
    for y in 0..h {
        for x in 0..w {
            let q = scene.context.grid.cell(x, y).quad as usize;
            let want = if q == 5 {
                render::present::debug_invalid([f64::from(x) + 0.5, f64::from(y) + 0.5])
            } else {
                [0.0; 3]
            };
            assert_eq!(c[(y * w + x) as usize], want, "pixel ({x}, {y})");
        }
    }
}

negative_control!(
    roundtrip_preset_twin_invalid,
    "a finite momentum must not be invalid",
    expected = "is not invalid",
    check_invalid(1.0)
);

// ── The clamp band ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The clamp-framed latent chart's samples within `band` of a clamp, by their margins, against `clamp_ambiguous`'s.
fn check_band(band: f64) {
    let scene = {
        let mut s =
            validation::decode_preset::Scene::lattice(DecodeChart::Latent, [64, 64], 20).unwrap();
        s.uniforms = clamp_frame();
        s
    };
    let found = clamp_ambiguous(&scene).unwrap();
    assert!(
        !found.is_empty(),
        "the clamp frame has no sample at a clamp boundary"
    );
    let chart = <kernel::chart::Latent<f32> as kernel::stage::decode::UniformChart>::from_uniform(
        &scene.uniforms.chart,
    );
    let mut want = Vec::new();
    for q in 0..scene.context.grid.quad_count() {
        let p = scene.point(q);
        let x = kernel::stage::decode::sample_with::<_, validation::Defaults, f32>(
            &chart, p[0], p[1], None,
        )
        .ic;
        let m = validation::roundtrip::margins(&validation::roundtrip::widen(&x));
        if m.iter().any(|m| (m - 1.0).abs() < band) {
            want.push((q, m));
        }
    }
    assert_eq!(
        found, want,
        "clamp_ambiguous is not the samples within {band} of a clamp"
    );
    assert!(
        want.len() < (64 * 64) / 4,
        "half the frame is at a clamp boundary"
    );
}

#[test]
fn roundtrip_preset_clamp_band() {
    assert_eq!(CLAMP_BAND, 0.25);
    check_band(CLAMP_BAND);
    // The band's edges, exact in f64, are outside it; just inside them, and the clamp itself, inside.
    let at = |k: usize, v: f64| {
        let mut m = [1.0e6; 8];
        m[k] = v;
        at_clamp(&m)
    };
    for k in 0..8 {
        assert!(
            !at(k, 1.25) && !at(k, 0.75),
            "control {k}: a band edge is inside the band"
        );
        assert!(
            at(k, 1.2499) && at(k, 0.7501) && at(k, 1.0),
            "control {k}: inside the band is outside it"
        );
        assert!(
            !at(k, 2.0) && !at(k, 0.0),
            "control {k}: far from the clamp is inside the band"
        );
    }
}

negative_control!(
    roundtrip_preset_clamp_band,
    "a band twice as wide must find other samples",
    expected = "clamp_ambiguous is not the samples",
    check_band(2.0 * CLAMP_BAND)
);

// ── The section flag ────────────────────────────────────────────────────────────────────────────────────────────────

/// Each fragment chart's `FragmentChart::ON_SECTION`, which picks the preset's reference, against its T2 reference
/// (`validation::roundtrip::Reference`), `decoded` standing for `Against::Decoded`.
fn check_on_section(decoded: validation::roundtrip::Against) {
    use kernel::chart::{
        BurrauThetaK, EuclidPlane, InvariantLE, InvariantLK, Latent, MassSimplex, ShapeSphere,
    };
    use kernel::stage::fragment::FragmentChart;
    use validation::roundtrip::Reference;
    fn one<C: FragmentChart + Reference>(name: &str, decoded: validation::roundtrip::Against) {
        assert_eq!(
            C::ON_SECTION,
            C::AGAINST == decoded,
            "{name}: the fragment's section flag is not its T2 reference's"
        );
    }
    one::<Latent<f32>>("latent", decoded);
    one::<ShapeSphere<f32>>("shape sphere", decoded);
    one::<InvariantLE<f32>>("(L_z, E)", decoded);
    one::<InvariantLK<f32>>("(L_z, K)", decoded);
    one::<MassSimplex<f32>>("mass simplex", decoded);
    one::<EuclidPlane<f32>>("Euclid plane", decoded);
    one::<BurrauThetaK<f32>>("θ × K", decoded);
}

#[test]
fn roundtrip_preset_on_section_is_the_t2_reference() {
    check_on_section(validation::roundtrip::Against::Decoded);
}

negative_control!(
    roundtrip_preset_on_section_is_the_t2_reference,
    "the flag held to the other reference must be found",
    expected = "the fragment's section flag is not its T2 reference's",
    check_on_section(validation::roundtrip::Against::Canonical)
);
