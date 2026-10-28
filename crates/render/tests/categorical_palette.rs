//! The categorical palettes and the categorical filter (TASK-M7-08; colour_composition §1.4; dd_colouring §3.7, §5):
//! - REQ-COL-018 (`categorical_palette_*`): the generated palette for `n ≤ 8` is the Okabe–Ito cycle, for `n > 8` the
//!   golden-angle hue `θ_i = 2π·frac(i·φ_g)` of every class; the outcome `state` (and its `detail` union) resolves to
//!   §1.4's palette, the occupant `outcome_state`, never a generated index palette; the WGSL is the CPU twin's.
//! - REQ-COL-040, REQ-COL-051 (`golden_angle_adjacency_*`): dd_colouring §5 unit test 9 for `n` up to 96, each class's
//!   8-bit sRGB colour measured in OKLab, consecutive classes at least the proposed minimum hue separation apart; the
//!   achieved separations are printed for the R-71 proposal (`--nocapture`).
//! - REQ-COL-019, REQ-COL-058 (`categorical_filter_*`): the state map filtered to the escape classes draws them
//!   unchanged and mutes every other class, the background staying the background; the mute transform as
//!   colour_composition §1.4 defines it, clear of the invalid colour and of the shown classes; the class mask's bit
//!   boundaries; and no `escaper` field in the ledger nor `escaper` mode in the registry.
//!
//! Each check takes the function, text or list it checks, so its control runs it on a wrong one and shows it fails
//! (R-176).

use render::assemble::{Kind, Node, Occupant, Stain};
use render::bind::Context;
use render::colour::outcome::{self, SWATCHES};
use render::colour::palette::{self, Palette, ALL, MIN_HUE_SEPARATION_DEG, MUTE_CHROMA, MUTE_L};
use render::colour::space;
use render::present::{self, Rgb};
use render::raster::Grid;
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain_with, Inputs, STATE_DECODE_FAILED, STATE_SIM_FAILED};
use validation::gpu::{BindingKind, GpuHarness};
use validation::negative_control;
use validation::synthetic::Synthetic;

/// The Okabe–Ito palette, 8-bit sRGB, in its published order (Okabe & Ito 2002), written here again.
const OKABE_ITO: [u32; 8] = [
    0x000000, 0xE69F00, 0x56B4E9, 0x009E73, 0xF0E442, 0x0072B2, 0xD55E00, 0xCC79A7,
];

/// The hue wheel's OKLCH lightness and chroma, the register's (render contract Part 5).
const WHEEL: (f64, f64) = (0.75, 0.12);

/// The Fibonacci-lattice counts dd_colouring §5's unit test 9 reaches (REQ-COL-040).
const FIBONACCI_COUNT: u32 = 96;

/// τ, the corpus's collision distance in OKLab (R-429; REQ-COL-055).
const TAU: f64 = 0.101;

fn hex(h: u32) -> Rgb {
    present::srgb8([(h >> 16) as u8, (h >> 8) as u8, h as u8])
}

fn close(a: Rgb, b: Rgb, tol: f64) -> bool {
    (0..3).all(|c| (a[c] - b[c]).abs() <= tol)
}

fn distance(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (space::linear_to_oklab(a), space::linear_to_oklab(b));
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

// ── REQ-COL-018: the generators and the state palette ────────────────────────────────────────────────────────────

/// Checks `generated` against the definitions: Okabe–Ito, cycling, for `n` from 1 to 8; the golden-angle hue of every
/// class on the hue wheel for `n` from 9, its OKLab hue `2π·frac(i·φ_g)`.
fn check_generator(generated: fn(u32, u32) -> Rgb) {
    for n in 1..=8 {
        for i in 0..20 {
            let want = hex(OKABE_ITO[(i % 8) as usize]);
            assert!(
                close(generated(i, n), want, 1e-12),
                "class {i} of {n} is not Okabe–Ito's #{:06X}",
                OKABE_ITO[(i % 8) as usize]
            );
        }
    }
    let phi = (5f64.sqrt() - 1.0) / 2.0;
    for n in [9, 10, 13, 21, 55, FIBONACCI_COUNT, 1000] {
        for i in 0..n.min(200) {
            let turns = (f64::from(i) * phi).fract();
            let want = present::oklch_to_linear(WHEEL.0, WHEEL.1, turns);
            let got = generated(i, n);
            assert!(
                close(got, want, 1e-12),
                "class {i} of {n} is not the golden angle's"
            );
            let [l, c, h] = space::oklab_to_oklch(space::linear_to_oklab(got));
            // Measured back through dd_colouring §3.1's published matrices, inverse to about 1e-8.
            let dh = (h / std::f64::consts::TAU - turns).rem_euclid(1.0);
            assert!(
                (l - WHEEL.0).abs() < 1e-6 && (c - WHEEL.1).abs() < 1e-6 && dh.min(1.0 - dh) < 1e-6,
                "class {i} of {n} is OKLCH ({l}, {c}, {h}), not hue 2π·frac(i·φ_g)"
            );
        }
    }
}

#[test]
fn categorical_palette_generator_is_okabe_ito_then_golden_angle() {
    check_generator(palette::generated);
}

negative_control!(
    categorical_palette_generator_is_okabe_ito_then_golden_angle,
    "a generator that turns to the golden angle at n = 8, not past it, fails",
    expected = "class 0 of 8 is not Okabe–Ito's",
    check_generator(|i, n| if n < 8 {
        palette::generated(i, n)
    } else {
        palette::generated(i, 9)
    })
);

/// Checks `resolve` on the ledger's fields: `state` and `detail` take the outcome palette, every other categorical
/// field the generated palette of its class count, and a field that is not categorical, or not in the ledger, none.
fn check_resolution(resolve: fn(&str) -> Result<Palette, String>) {
    for field in ["state", "detail"] {
        assert_eq!(
            resolve(field),
            Ok(Palette::Outcome),
            "`{field}` does not resolve to the outcome palette"
        );
    }
    for (field, classes) in [("dmin_pair", 3), ("last_symbol", 4)] {
        assert_eq!(
            resolve(field),
            Ok(Palette::Generated { classes }),
            "`{field}` does not resolve to the generated palette of {classes} classes"
        );
    }
    for field in ["t_end_step", "escaper"] {
        assert!(resolve(field).is_err(), "`{field}` resolves to a palette");
    }
    assert_eq!(
        render::assemble::builtin(render::assemble::Kind::Colour, outcome::ID),
        Some(outcome::WGSL),
        "the outcome palette is not the built-in colour `{}`",
        outcome::ID
    );
}

#[test]
fn categorical_palette_state_resolves_to_the_outcome_palette() {
    check_resolution(palette::resolve);
}

negative_control!(
    categorical_palette_state_resolves_to_the_outcome_palette,
    "a resolver giving `state` a generated index palette fails",
    expected = "`state` does not resolve to the outcome palette",
    check_resolution(|field| match field {
        "state" => Ok(Palette::Generated { classes: 6 }),
        _ => palette::resolve(field),
    })
);

// ── The WGSL against its CPU twin ────────────────────────────────────────────────────────────────────────────────

/// The test entry: pixel `x` evaluates `cases[x]`, word 0 its operation, the rest its arguments.
const ENTRY: &str = r"
@group(2) @binding(0) var<storage, read> cases: array<vec4<u32>>;

fn rgb(c: vec3<f32>) -> vec4<u32> { return vec4<u32>(bitcast<vec3<u32>>(c), 0u); }

@fragment
fn t_palette(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {
    let a = cases[u32(pos.x)];
    switch a.x {
        case 0u: { return rgb(cat_palette(a.y, a.z)); }
        case 1u: { return rgb(cat_mute(bitcast<vec3<f32>>(a.yzw))); }
        case 2u: { return rgb(cat_ramp(a.y, a.z, a.w)); }
        case 3u: { return vec4<u32>(select(0u, 1u, cat_shown(a.y, a.z)), 0u, 0u, 0u); }
        default: { return vec4<u32>(0xffffffffu); }
    }
}
";

fn read(rel: &str) -> String {
    let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The prelude, the colour-space maps, the presentation layer and `palette`, the palette WGSL, then [`ENTRY`].
fn module(palette: &str) -> String {
    format!(
        "{}\n{}\n{}\n{palette}\n{ENTRY}",
        read(ledger::gen::prelude::PATH),
        read("crates/render/shaders/wgsl/lib/colour_space.wgsl"),
        read("crates/render/shaders/wgsl/lib/present.wgsl"),
    )
}

/// One case and the CPU twin's answer, with its bound.
enum Want {
    Colour(Rgb, f64),
    Shown(bool),
}

/// The cases: the generator either side of n = 8 and far past it; the mute of each outcome swatch and of each
/// Okabe–Ito colour; the ramp through a mask; the mask's bits about 31.
fn cases() -> Vec<([u32; 4], String, Want)> {
    let mut out = Vec::new();
    for (i, n) in [
        (0, 1),
        (7, 8),
        (9, 8),
        (0, 9),
        (1, 9),
        (8, 12),
        (95, 96),
        (1000, 1001),
    ] {
        // Okabe–Ito decodes within the transfer's f32 bound; the golden angle adds `hue_wheel`'s f32 cos and sin.
        let bound = if n <= 8 { 1e-5 } else { 1e-3 };
        out.push((
            [0, i, n, 0],
            format!("cat_palette({i}, {n})"),
            Want::Colour(palette::generated(i, n), bound),
        ));
    }
    let colours = SWATCHES
        .iter()
        .map(|s| present::srgb8(s.srgb8))
        .chain(OKABE_ITO.iter().map(|&h| hex(h)));
    for c in colours {
        let f = c.map(|v| (v as f32).to_bits());
        out.push((
            [1, f[0], f[1], f[2]],
            format!("cat_mute({c:?})"),
            Want::Colour(palette::mute(c.map(|v| f64::from(v as f32))), 1e-4),
        ));
    }
    let mask = palette::mask(&[1, 3]);
    for i in 0..5 {
        out.push((
            [2, i, 5, mask],
            format!("cat_ramp({i}, 5, {mask:#b})"),
            Want::Colour(palette::ramp(i, 5, mask), 1e-4),
        ));
    }
    for (class, mask) in [
        (0, 1),
        (1, 1),
        (30, 1 << 31),
        (31, 1 << 31),
        (32, 1 << 31),
        (u32::MAX, 1 << 31),
        (32, !(1 << 31)),
        (5, ALL),
        (5, 0),
    ] {
        out.push((
            [3, class, mask, 0],
            format!("cat_shown({class}, {mask:#x})"),
            Want::Shown(palette::shown(class, mask)),
        ));
    }
    out
}

/// Checks `palette`, the palette WGSL, evaluated on the GPU, against the CPU twin on every case.
fn check_wgsl(h: &GpuHarness, palette: &str) {
    let cases = cases();
    let words: Vec<u32> = cases.iter().flat_map(|c| c.0).collect();
    let kernel = h
        .fragment(
            &module(palette),
            "t_palette",
            &[&[], &[], &[BindingKind::Storage]],
            cases.len() as u32,
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let pixels = kernel
        .draw(&[&[], &[], &[&words]])
        .unwrap_or_else(|e| panic!("{e}"));
    for ((_, what, want), p) in cases.iter().zip(pixels) {
        match want {
            Want::Colour(w, bound) => {
                let got = [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c])));
                assert!(
                    close(got, *w, *bound),
                    "{what} is {got:?} on the GPU, not {w:?} within {bound}"
                );
            }
            Want::Shown(w) => assert_eq!(p[0] == 1, *w, "{what} on the GPU"),
        }
    }
}

#[test]
fn categorical_palette_wgsl_is_the_cpu_twin() {
    check_wgsl(&gpu(), palette::WGSL);
}

#[cfg(feature = "controls")]
fn altered_palette(from: &str, to: &str) -> String {
    assert!(palette::WGSL.contains(from), "the WGSL has no `{from}`");
    palette::WGSL.replace(from, to)
}

negative_control!(
    categorical_palette_wgsl_is_the_cpu_twin,
    "WGSL that clamps the class to 30, not 31, fails",
    expected = "cat_shown(31, 0x80000000) on the GPU",
    check_wgsl(&gpu(), &altered_palette("min(k, 31u)", "min(k, 30u)"))
);

#[cfg(feature = "controls")]
mod categorical_palette_wgsl_generator {
    use super::*;
    negative_control!(
        categorical_palette_wgsl_is_the_cpu_twin,
        "WGSL that keeps Okabe–Ito past 8 classes fails",
        expected = "cat_palette(0, 9) is",
        check_wgsl(&gpu(), &altered_palette("n <= 8u", "n <= 9u"))
    );
}

#[cfg(feature = "controls")]
mod categorical_palette_wgsl_mute {
    use super::*;
    negative_control!(
        categorical_palette_wgsl_is_the_cpu_twin,
        "WGSL whose mute keeps the class's own lightness fails",
        expected = "cat_mute(",
        check_wgsl(
            &gpu(),
            &altered_palette("vec3<f32>(CAT_MUTE_L,", "vec3<f32>(lab.x,")
        )
    );
}

// ── REQ-COL-040, REQ-COL-051: golden-angle adjacency ─────────────────────────────────────────────────────────────

/// `c` as the display shows it, each channel rounded to 8-bit sRGB, decoded to linear.
fn quantised(c: Rgb) -> Rgb {
    present::srgb8(c.map(|v| (space::linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8))
}

/// The OKLab hue separation of two colours, in degrees, on the circle: in [0, 180].
fn hue_separation(a: Rgb, b: Rgb) -> f64 {
    let ha = space::oklab_to_oklch(space::linear_to_oklab(a))[2];
    let hb = space::oklab_to_oklch(space::linear_to_oklab(b))[2];
    let d = (ha - hb).to_degrees().rem_euclid(360.0);
    d.min(360.0 - d)
}

/// The least OKLab hue separation of consecutive classes of `generated`'s `n`-class palette, 8-bit quantised, and the
/// class `i` whose pair `(i, i + 1)` has it.
fn least_consecutive(generated: fn(u32, u32) -> Rgb, n: u32) -> (f64, u32) {
    (0..n - 1)
        .map(|i| {
            let s = hue_separation(quantised(generated(i, n)), quantised(generated(i + 1, n)));
            (s, i)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("two classes at least")
}

/// dd_colouring §5 unit test 9: for every `n` from 9, where the golden angle starts, to 96, consecutive classes of
/// `generated`'s palette are at least [`MIN_HUE_SEPARATION_DEG`] apart in OKLab hue, each colour as the display shows
/// it; the achieved separations, printed, are the R-71 proposal's evidence. Returns the least over every `n`.
fn check_adjacency(generated: fn(u32, u32) -> Rgb) -> f64 {
    let mut least = f64::INFINITY;
    println!("golden-angle adjacency: n, least consecutive OKLab hue separation (8-bit sRGB), at classes (i, i + 1)");
    for n in 9..=FIBONACCI_COUNT {
        let (s, i) = least_consecutive(generated, n);
        println!("  n = {n:3}: {s:.3}° at ({i}, {})", i + 1);
        assert!(
            s >= MIN_HUE_SEPARATION_DEG,
            "n = {n}: classes {i} and {} are {s:.3}° apart, below {MIN_HUE_SEPARATION_DEG}°",
            i + 1
        );
        least = least.min(s);
    }
    println!(
        "least over n ≤ {FIBONACCI_COUNT}: {least:.3}°; proposed minimum {MIN_HUE_SEPARATION_DEG}°"
    );
    least
}

#[test]
fn golden_angle_adjacency_up_to_the_fibonacci_counts() {
    let least = check_adjacency(palette::generated);
    // The proposal's figure (palette.rs, the PR): the least achieved separation, 136.68°.
    assert!(
        (least - 136.68).abs() < 0.005,
        "the least achieved separation is {least:.3}°, not the proposal's 136.68°"
    );
}

negative_control!(
    golden_angle_adjacency_up_to_the_fibonacci_counts,
    "an evenly spaced hue palette, 360°/n between neighbours, fails",
    expected = "n = 9: classes",
    {
        check_adjacency(|i, n| {
            if n <= 8 {
                palette::generated(i, n)
            } else {
                present::hue_wheel(f64::from(i) / f64::from(n))
            }
        });
    }
);

// ── REQ-COL-058: the mute transform ──────────────────────────────────────────────────────────────────────────────

/// The palettes the mute is measured on: the outcome palette's two sets, Okabe–Ito, and the golden angle's 96.
fn palettes() -> Vec<(&'static str, Vec<Rgb>)> {
    vec![
        (
            "outcome sRGB",
            SWATCHES.iter().map(|s| present::srgb8(s.srgb8)).collect(),
        ),
        (
            "outcome OKLab-uniform",
            outcome::OKLAB_UNIFORM
                .iter()
                .map(|s| present::srgb8(s.srgb8))
                .collect(),
        ),
        ("Okabe–Ito", OKABE_ITO.iter().map(|&h| hex(h)).collect()),
        (
            "golden angle, 96",
            (0..FIBONACCI_COUNT)
                .map(|i| palette::generated(i, FIBONACCI_COUNT))
                .collect(),
        ),
    ]
}

/// Checks `mute` against colour_composition §1.4's definition, OKLab `(MUTE_L, MUTE_CHROMA·a, MUTE_CHROMA·b)` where in
/// gamut, and that every muted class of every palette is at least τ from every other class of its palette, any of
/// which may be shown, from the presentation views' invalid colour and from the "not yet" grey; prints the nearest of
/// each palette. Returns the least distance over every palette to a shown class, to the invalid colour and to the grey.
fn check_mute(mute: fn(Rgb) -> Rgb) -> [f64; 3] {
    let invalid = present::present_invalid();
    let not_yet = present::not_yet();
    let mut least = [f64::INFINITY; 3];
    for (name, set) in palettes() {
        let mut nearest = (f64::INFINITY, String::new());
        for (i, &c) in set.iter().enumerate() {
            let m = mute(c);
            let [_, a, b] = space::linear_to_oklab(c);
            let want = space::oklab_to_linear([MUTE_L, MUTE_CHROMA * a, MUTE_CHROMA * b]);
            assert!(
                want.iter().all(|v| (0.0..=1.0).contains(v)),
                "{name} class {i}'s mute is out of gamut"
            );
            assert!(
                close(m, want, 1e-12),
                "{name} class {i}'s mute is {m:?}, not OKLab ({MUTE_L}, {MUTE_CHROMA}·a, {MUTE_CHROMA}·b)"
            );
            let others = set
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(j, &s)| (0, distance(m, s), format!("class {j}")));
            for (kind, d, what) in others.chain([
                (
                    1,
                    distance(m, invalid),
                    "the invalid colour #141418".to_owned(),
                ),
                (
                    2,
                    distance(m, not_yet),
                    "the not-yet grey #4E4E4E".to_owned(),
                ),
            ]) {
                assert!(
                    d >= TAU,
                    "{name} class {i}'s mute is {d:.3} from {what}, within τ = {TAU}"
                );
                least[kind] = least[kind].min(d);
                if d < nearest.0 {
                    nearest = (d, format!("class {i}'s mute to {what}"));
                }
            }
        }
        println!("mute, {name}: nearest {:.3}, {}", nearest.0, nearest.1);
    }
    println!(
        "mute, every palette: {:.3} from a shown class, {:.3} from #141418, {:.3} from #4E4E4E",
        least[0], least[1], least[2]
    );
    least
}

#[test]
fn categorical_filter_mute_is_the_section_1_4_transform() {
    let least = check_mute(palette::mute);
    // colour_composition §1.4's figures (RQ-607): 0.119 from a shown class, 0.117 from #141418, 0.114 from #4E4E4E.
    for (got, want, what) in [
        (least[0], 0.119, "a shown class"),
        (least[1], 0.117, "the invalid colour"),
        (least[2], 0.114, "the not-yet grey"),
    ] {
        assert!(
            (got - want).abs() < 0.0005,
            "the mute's least distance to {what} is {got:.4}, not §1.4's {want}"
        );
    }
}

negative_control!(
    categorical_filter_mute_is_the_section_1_4_transform,
    "a mute that only dims, keeping each class's own colour at a fifth of its light, fails",
    expected = "outcome sRGB class 0's mute is",
    {
        check_mute(|c| c.map(|v| v * 0.2));
    }
);

// ── REQ-COL-019: the categorical filter on the state map ─────────────────────────────────────────────────────────

/// Each sample's tile, in pixels.
const TILE: u32 = 4;

/// The `state` codes (payload §2).
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;

/// The canvas background the renders take: a value of its own, exact in f32.
const BG: [f32; 3] = [0.25, 0.125, 0.0625];

/// The filter scene's samples: `(state, detail, forbidden)`, every class of the outcome palette, then running,
/// `sim_failed`, `decode_failed` and a forbidden escape.
const SAMPLES: [(u32, u32, bool); 13] = [
    (BOUNDED, 0, false),
    (COLLISION, 2, false),
    (COLLISION, 1, false),
    (COLLISION, 0, false),
    (COLLISION, 3, false),
    (ESCAPE, 0, false),
    (ESCAPE, 1, false),
    (ESCAPE, 2, false),
    (ESCAPE, 3, false),
    (RUNNING, 0, false),
    (STATE_SIM_FAILED, 1, false),
    (STATE_DECODE_FAILED, 1, false),
    (ESCAPE, 0, true),
];

/// The escape classes' indices in the outcome palette: escape body 1, 2, 3 and triple ejection.
const ESCAPES: [u32; 4] = [5, 6, 7, 8];

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

fn grid() -> Grid {
    Grid::new([SAMPLES.len() as u32, 1], 1, 0, TILE).expect("the scene's grid")
}

fn context() -> Context {
    Context {
        dt_macro: 0.01,
        delta_0: 1e-6,
        n_renorm: 16,
        horizon_steps: 1000,
        z: [0.0; 8],
        grid: grid(),
        chart_id: 0,
        time: 0.0,
        ensemble_spread: 0.0,
        out_of_chart: false,
        valid_sample_count: 1,
    }
}

/// Event mode's stain with `colour` in the colour slot: source → colour, brightness None → Replace-L → OUT.
fn event(colour: Occupant) -> Stain {
    let node = |kind, occupant, inputs| Node {
        kind,
        occupant,
        inputs,
    };
    Stain::new(vec![
        node(
            Kind::Source,
            Occupant::Custom(ZERO_SOURCE.to_owned()),
            vec![],
        ),
        node(Kind::Colour, colour, vec![Some(0)]),
        node(Kind::Brightness, Occupant::None, vec![Some(0)]),
        node(
            Kind::Combiner,
            Occupant::BuiltIn("replace_l".to_owned()),
            vec![Some(1), Some(2)],
        ),
        node(Kind::Out, Occupant::None, vec![Some(3)]),
    ])
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The state map, the outcome occupant `colour` in event mode, over the filter scene, its class mask at `mask`.
fn render(h: &GpuHarness, colour: Occupant, mask: u32) -> Vec<[f32; 4]> {
    let g = grid();
    let mut set = Synthetic::flat(g, 3);
    for (x, &(state, detail, forbidden)) in SAMPLES.iter().enumerate() {
        let i = g.cell(x as u32 * TILE + TILE / 2, TILE / 2).sample;
        set.sample(i).state(state).detail(detail).times(40, 0);
        set.set_forbidden(i, forbidden);
    }
    render_stain_with(
        h.device(),
        h.queue(),
        &event(colour),
        &set,
        &context(),
        &|_, name| (name == palette::MASK_PARAM).then(|| vec![f64::from(mask)]),
        Inputs {
            background: BG,
            ..Inputs::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
    .pixels
}

/// Checks the state map through `colour` filtered to the escape classes: an escape draws its swatch exactly, every
/// other class its swatch's mute, the `decode_failed` and forbidden samples the background exactly; and unfiltered,
/// every class its swatch.
fn check_filter(colour: Occupant) {
    let h = gpu();
    let width = SAMPLES.len() as u32 * TILE;
    for (mask, name) in [(palette::mask(&ESCAPES), "filtered"), (ALL, "unfiltered")] {
        let image = render(&h, colour.clone(), mask);
        for (x, &(state, detail, forbidden)) in SAMPLES.iter().enumerate() {
            let k = outcome::class(state, detail);
            let swatch = present::srgb8(SWATCHES[k].srgb8);
            let (want, tol, what) = if forbidden || state == STATE_DECODE_FAILED {
                (BG.map(f64::from), 0.0, "the background")
            } else if palette::shown(k as u32, mask) {
                (swatch.map(|v| f64::from(v as f32)), 0.0, "its swatch")
            } else {
                (palette::mute(swatch), 1e-4, "muted")
            };
            for py in 0..TILE {
                for px in x as u32 * TILE..(x as u32 + 1) * TILE {
                    let p = image[(py * width + px) as usize];
                    let got = [0, 1, 2].map(|c| f64::from(p[c]));
                    assert!(
                        close(got, want, tol),
                        "{name}: sample {x} (state {state}, detail {detail}) at ({px}, {py}) is {got:?}, \
                         not {what} {want:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn categorical_filter_state_map_to_the_escape_classes() {
    check_filter(Occupant::BuiltIn(outcome::ID.to_owned()));
}

#[cfg(feature = "controls")]
fn altered_outcome(from: &str, to: &str) -> Occupant {
    assert!(outcome::WGSL.contains(from), "the occupant has no `{from}`");
    Occupant::Custom(outcome::WGSL.replace(from, to))
}

negative_control!(
    categorical_filter_state_map_to_the_escape_classes,
    "an occupant that ignores its class mask fails",
    expected = "filtered: sample 0 (state 1, detail 0)",
    check_filter(altered_outcome(
        "return cat_filter(rgb, u32(k), uniforms.class_mask);",
        "return rgb;"
    ))
);

#[cfg(feature = "controls")]
mod categorical_filter_class_order {
    use super::*;
    negative_control!(
        categorical_filter_state_map_to_the_escape_classes,
        "an occupant whose escape classes are one index off fails",
        expected = "filtered: sample 5 (state 0, detail 0)",
        check_filter(altered_outcome(
            "return select(5 + detail, 8, detail == 3);",
            "return select(4 + detail, 8, detail == 3);"
        ))
    );
}

#[cfg(feature = "controls")]
mod categorical_filter_background {
    use super::*;
    negative_control!(
        categorical_filter_state_map_to_the_escape_classes,
        "an occupant that mutes the background of a sample with no class fails",
        expected = "filtered: sample 11 (state 5, detail 1)",
        check_filter(altered_outcome("if (k < 0) {", "if (k < -1) {"))
    );
}

/// Checks the class mask's bits: class `i` shown by bit `min(i, 31)`; [`ALL`] shows every class and 0 none; `mask`
/// sets exactly the bits of its classes.
fn check_mask(shown: fn(u32, u32) -> bool) {
    for (class, mask, want) in [
        (0, 1, true),
        (1, 1, false),
        (0, 2, false),
        (30, 1 << 31, false),
        (31, 1 << 31, true),
        (32, 1 << 31, true),
        (u32::MAX, 1 << 31, true),
        (31, !(1 << 31), false),
        (32, !(1 << 31), false),
    ] {
        assert_eq!(shown(class, mask), want, "class {class} under {mask:#x}");
    }
    for class in [0, 8, 31, 32, 1000] {
        assert!(shown(class, ALL), "every bit set hides class {class}");
        assert!(!shown(class, 0), "no bit set shows class {class}");
    }
    assert_eq!(palette::mask(&ESCAPES), 0b1_1110_0000, "the escape mask");
    assert_eq!(
        palette::mask(&[31, 40]),
        1 << 31,
        "classes past 31 share bit 31"
    );
    assert_eq!(palette::mask(&[]), 0, "the empty mask");
    let c = palette::generated(3, 5);
    assert_eq!(
        palette::filter(c, 3, 1 << 3),
        c,
        "a shown class is unchanged"
    );
    assert_eq!(
        palette::filter(c, 3, 0),
        palette::mute(c),
        "a hidden class is muted"
    );
    assert_eq!(
        palette::ramp(3, 5, 0),
        palette::mute(c),
        "the ramp is the filter's"
    );
    assert_eq!(palette::ramp(3, 5, ALL), c, "the ramp is the generator's");
}

#[test]
fn categorical_filter_mask_bits() {
    check_mask(palette::shown);
}

negative_control!(
    categorical_filter_mask_bits,
    "a mask that shows a class past 31 only when every bit is set fails",
    expected = "class 32 under 0x80000000",
    check_mask(|class, mask| if class < 32 {
        palette::shown(class, mask)
    } else {
        mask == ALL
    })
);

/// Checks that no field of `fields` is `escaper` and no mode of `modes`, registry ids, is one: the escaper view is the
/// outcome palette filtered (colour_composition §1.4).
fn check_no_escaper(fields: &[String], modes: &[String]) {
    assert!(
        !fields.iter().any(|f| f == "escaper"),
        "the ledger has an `escaper` field"
    );
    assert!(
        !modes
            .iter()
            .any(|m| m.rsplit('/').next() == Some("escaper")),
        "the registry has an `escaper` mode"
    );
    assert!(
        fields.iter().any(|f| f == "state") && modes.iter().any(|m| m == "colour/outcome_state"),
        "the lists are not the ledger's and the registry's"
    );
}

fn ledger_fields() -> Vec<String> {
    let l = ledger::payload::ledger();
    ledger::gen::validate(&l)
        .unwrap_or_else(|e| panic!("{e}"))
        .iter()
        .map(|e| e.name.to_string())
        .collect()
}

fn registry_modes() -> Vec<String> {
    render::registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|e| e.id)
        .collect()
}

#[test]
fn categorical_filter_no_escaper_field_or_mode() {
    check_no_escaper(&ledger_fields(), &registry_modes());
}

negative_control!(
    categorical_filter_no_escaper_field_or_mode,
    "a registry with an escaper mode fails",
    expected = "the registry has an `escaper` mode",
    check_no_escaper(
        &ledger_fields(),
        &[registry_modes(), vec!["colour/escaper".to_owned()]].concat()
    )
);
