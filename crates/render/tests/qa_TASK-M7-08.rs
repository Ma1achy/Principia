//! QA's tests for TASK-M7-08, written from the requirements it closes, not from the implementation:
//! - REQ-COL-018 (`categorical_palette_qa_*`): the generated palette is Okabe & Ito's published colours, cycling, for
//!   `n ≤ 8`, and for `n > 8` the golden-angle hue `θ_i = 2π·frac(i·φ_g)`, `φ_g = (√5 − 1)/2`, of each class, measured
//!   back in OKLab with Ottosson's published matrices written here again; `state` and `detail` take §1.4's palette,
//!   every other categorical ledger field the generated palette of its class count, and no Chazy field a generated
//!   palette (R-442).
//! - REQ-COL-040 with REQ-COL-051's hook (`golden_angle_adjacency_qa_*`): dd_colouring §5 unit test 9, consecutive
//!   classes ≈ 137.5° apart (dd_colouring §3.7) and, as the display shows them (8-bit), at least the calibrated minimum
//!   for every `n` from 9 to 96; the minimum lies below every achieved separation (REQ-COL-051's verify).
//! - REQ-COL-019, REQ-COL-058, REQ-COL-162 (`categorical_filter_qa_*`): the state map rendered on the GPU with
//!   R-449's filters ("just collisions", "just body 3's escape") and the escaper view (the escape classes): a shown
//!   class's pixels are bit-identical to the unfiltered render, which is §1.4's table; a muted pixel is the §1.4 mute
//!   of its class colour (OKLab lightness `L_mute`, chroma `k_mute` of its own at its own hue), at least τ from the
//!   invalid colour `#141418` and from every shown class; a pixel with no class is the background whatever the mask;
//!   for both swatch sets. The mute's values are the calibration hook REQ-COL-162, read from `palette::MUTE_L` and
//!   `palette::MUTE_CHROMA`, never written here; the WGSL's constants are checked to be the same hook.
//!
//! Each check takes what it checks as an argument, so its control runs it on a wrong one and shows it fails (R-176).

use render::assemble::{Kind, Node, Occupant, Stain};
use render::bind::Context;
use render::colour::outcome;
use render::colour::palette::{self, Palette, ALL, MIN_HUE_SEPARATION_DEG, MUTE_CHROMA, MUTE_L};
use render::present::Rgb;
use render::raster::Grid;
use render::registry::ZERO_SOURCE;
use validation::golden_scene::{render_stain_with, Inputs, STATE_DECODE_FAILED, STATE_SIM_FAILED};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::synthetic::Synthetic;

// ── Independent references ───────────────────────────────────────────────────────────────────────────────────────

/// Okabe & Ito (2002), "Color Universal Design", the eight colours in their published order: black, orange, sky blue,
/// bluish green, yellow, blue, vermillion, reddish purple.
const OKABE_ITO_PUBLISHED: [&str; 8] = [
    "#000000", "#E69F00", "#56B4E9", "#009E73", "#F0E442", "#0072B2", "#D55E00", "#CC79A7",
];

/// colour_composition §1.4's table, in its order (bounded / nothing yet first, triple ejection last), transcribed from
/// the doc.
const TABLE: [&str; 9] = [
    "#141418", "#DE2D2D", "#2EBC4E", "#3462E0", "#F29620", "#F0DE32", "#E034C6", "#30C8DC",
    "#611597",
];

/// τ, the corpus's OKLab collision distance (R-429), the floor REQ-COL-162's verify sets for the mute's separations.
const TAU: f64 = 0.101;

/// The highest Fibonacci-lattice count dd_colouring §5's unit test 9 reaches (REQ-COL-040, REQ-COL-051: 96).
const FIBONACCI_MAX: u32 = 96;

/// The 8-bit sRGB code `#RRGGBB` decoded to linear RGB by the sRGB transfer function (IEC 61966-2-1), written here.
fn hex(s: &str) -> Rgb {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).expect("a hex colour");
    [16, 8, 0].map(|sh| {
        let c = f64::from((v >> sh) & 0xff) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// Linear RGB rounded to 8-bit sRGB and decoded again: the colour as the display shows it.
fn quantise(c: Rgb) -> Rgb {
    let code = c.map(|x| {
        let x = x.clamp(0.0, 1.0);
        let e = if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        };
        (e * 255.0).round() as u32
    });
    hex(&format!("#{:02X}{:02X}{:02X}", code[0], code[1], code[2]))
}

/// Linear sRGB to OKLab, Ottosson (2020), "A perceptual color space for image processing", the published matrices
/// written here again, independent of the crate's.
fn oklab(c: Rgb) -> [f64; 3] {
    let lms = [
        0.4122214708 * c[0] + 0.5363325363 * c[1] + 0.0514459929 * c[2],
        0.2119034982 * c[0] + 0.6806995451 * c[1] + 0.1073969566 * c[2],
        0.0883024619 * c[0] + 0.2817188376 * c[1] + 0.6299787005 * c[2],
    ]
    .map(f64::cbrt);
    [
        0.2104542553 * lms[0] + 0.7936177850 * lms[1] - 0.0040720468 * lms[2],
        1.9779984951 * lms[0] - 2.4285922050 * lms[1] + 0.4505937099 * lms[2],
        0.0259040371 * lms[0] + 0.7827717662 * lms[1] - 0.8086757660 * lms[2],
    ]
}

/// The OKLab hue of `c`, in degrees in [0, 360).
fn hue_deg(c: Rgb) -> f64 {
    let [_, a, b] = oklab(c);
    b.atan2(a).to_degrees().rem_euclid(360.0)
}

/// The separation of two hues on the circle, in degrees in [0, 180].
fn circ(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Euclidean distance in OKLab, as REQ-COL-055 and R-429 measure.
fn dist(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (oklab(a), oklab(b));
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

fn phi_g() -> f64 {
    (5f64.sqrt() - 1.0) / 2.0
}

// ── REQ-COL-018: the generators and which palette a field takes ──────────────────────────────────────────────────

/// Checks `generated(i, n)`: for `n ≤ 8`, Okabe & Ito's published colours in order, cycling past 8; for `n > 8`, the
/// OKLab hue of class `i` is `360°·frac(i·φ_g)`, and the colour is the same for every `n` (it depends on `i` alone).
fn check_generators(generated: fn(u32, u32) -> Rgb) {
    for n in 1..=8u32 {
        for i in 0..24u32 {
            let want = hex(OKABE_ITO_PUBLISHED[(i % 8) as usize]);
            let got = generated(i, n);
            assert!(
                (0..3).all(|c| (got[c] - want[c]).abs() < 1e-9),
                "n = {n}: class {i} is {got:?}, not Okabe–Ito's {}",
                OKABE_ITO_PUBLISHED[(i % 8) as usize]
            );
        }
    }
    for n in [9u32, 10, 13, 21, 34, 55, 89, FIBONACCI_MAX, 500] {
        for i in 0..n.min(300) {
            let got = generated(i, n);
            let want = 360.0 * (f64::from(i) * phi_g()).fract();
            assert!(
                circ(hue_deg(got), want) < 1e-4,
                "n = {n}: class {i}'s OKLab hue is {:.6}°, not 2π·frac(i·φ_g) = {want:.6}°",
                hue_deg(got)
            );
            let same = generated(i, 9);
            assert!(
                (0..3).all(|k| (got[k] - same[k]).abs() < 1e-12),
                "class {i}'s golden-angle colour depends on n ({n} against 9)"
            );
        }
    }
}

#[test]
fn categorical_palette_qa_generators_from_their_definitions() {
    check_generators(palette::generated);
}

negative_control!(
    categorical_palette_qa_generators_from_their_definitions,
    "a golden angle written with φ_g = (√5 + 1)/2 − 1 rounded to 0.618 fails",
    expected = "OKLab hue is",
    check_generators(|i, n| if n <= 8 {
        palette::generated(i, n)
    } else {
        render::present::hue_wheel((f64::from(i) * 0.618).fract())
    })
);

#[cfg(feature = "controls")]
mod categorical_palette_qa_generators_spread_by_n {
    use super::*;
    negative_control!(
        categorical_palette_qa_generators_from_their_definitions,
        "an index palette spreading n hues evenly, i/n, not the golden angle, fails",
        expected = "OKLab hue is",
        check_generators(|i, n| if n <= 8 {
            palette::generated(i, n)
        } else {
            render::present::hue_wheel(f64::from(i) / f64::from(n))
        })
    );
}

#[cfg(feature = "controls")]
mod categorical_palette_qa_generators_okabe_ito_order {
    use super::*;
    negative_control!(
        categorical_palette_qa_generators_from_their_definitions,
        "an Okabe–Ito cycle starting one colour late fails",
        expected = "class 0 is",
        check_generators(|i, n| if n <= 8 {
            palette::generated(i + 1, n)
        } else {
            palette::generated(i, n)
        })
    );
}

/// Checks `resolve` against the ledger, enumerated here: `state` and `detail` take §1.4's outcome palette (R-77;
/// dd_colouring §3.7: the outcome palette is `state ⊕ detail`); every other categorical field `categorical(n)` the
/// generated palette of its `n` classes; a non-categorical field none; and no Chazy final-motion field ever a generated
/// index palette (REQ-COL-018, R-442; a guard for when those fields join the ledger).
fn check_resolve(resolve: fn(&str) -> Result<Palette, String>) {
    let l = ledger::payload::ledger();
    let entries = ledger::gen::validate(&l).unwrap_or_else(|e| panic!("{e}"));
    let mut others = 0;
    for e in &entries {
        let name = e.name.to_string();
        let got = resolve(&name);
        match e.scale {
            ledger::schema::Scale::Categorical(_) if name == "state" || name == "detail" => {
                assert_eq!(
                    got,
                    Ok(Palette::Outcome),
                    "`{name}` is not on §1.4's palette"
                );
            }
            ledger::schema::Scale::Categorical(n) => {
                others += 1;
                assert_eq!(
                    got,
                    Ok(Palette::Generated { classes: n }),
                    "`{name}`, categorical({n}), is not on the generated palette of {n} classes"
                );
            }
            _ => assert!(
                got.is_err(),
                "`{name}` is not categorical, yet takes {got:?}"
            ),
        }
    }
    assert!(
        others > 0,
        "the ledger has no categorical field but the outcome's"
    );
    for chazy in [
        "chazy_final",
        "chazy_status",
        "chazy_structure",
        "chazy_escaper",
    ] {
        assert!(
            !matches!(resolve(chazy), Ok(Palette::Generated { .. })),
            "`{chazy}` takes a generated index palette, not R-442's"
        );
    }
}

#[test]
fn categorical_palette_qa_state_and_detail_take_section_1_4() {
    check_resolve(palette::resolve);
}

negative_control!(
    categorical_palette_qa_state_and_detail_take_section_1_4,
    "a resolver giving `detail` its own index palette fails",
    expected = "`detail` is not on §1.4's palette",
    check_resolve(|f| if f == "detail" {
        Ok(Palette::Generated { classes: 4 })
    } else {
        palette::resolve(f)
    })
);

#[cfg(feature = "controls")]
mod categorical_palette_qa_chazy_guard {
    use super::*;
    negative_control!(
        categorical_palette_qa_state_and_detail_take_section_1_4,
        "a resolver giving a Chazy field a generated palette fails",
        expected = "`chazy_final` takes a generated index palette",
        check_resolve(|f| if f == "chazy_final" {
            Ok(Palette::Generated { classes: 7 })
        } else {
            palette::resolve(f)
        })
    );
}

// ── REQ-COL-040, REQ-COL-051: golden-angle adjacency ─────────────────────────────────────────────────────────────

/// dd_colouring §5 unit test 9 on `generated`: for every `n` from 9 to 96, consecutive classes are ≈ 137.5° apart in
/// OKLab hue unquantised (dd_colouring §3.7: 360°·(1 − φ_g)), and at least `min_deg` apart as the display shows them
/// (8-bit sRGB). Returns the least achieved 8-bit separation.
fn check_adjacency(generated: fn(u32, u32) -> Rgb, min_deg: f64) -> f64 {
    let golden = 360.0 * (1.0 - phi_g());
    let mut least = f64::INFINITY;
    for n in 9..=FIBONACCI_MAX {
        for i in 0..n - 1 {
            let (a, b) = (generated(i, n), generated(i + 1, n));
            let exact = circ(hue_deg(a), hue_deg(b));
            assert!(
                (exact - golden).abs() < 1e-4,
                "n = {n}: classes {i}, {} are {exact:.5}° apart, not the golden angle's {golden:.5}°",
                i + 1
            );
            let shown = circ(hue_deg(quantise(a)), hue_deg(quantise(b)));
            assert!(
                shown >= min_deg,
                "n = {n}: classes {i}, {} are {shown:.3}° apart as shown, below the minimum {min_deg}°",
                i + 1
            );
            least = least.min(shown);
        }
    }
    least
}

#[test]
fn golden_angle_adjacency_qa_consecutive_classes_up_to_96() {
    let least = check_adjacency(palette::generated, MIN_HUE_SEPARATION_DEG);
    // REQ-COL-051's verify: the minimum is chosen below the achieved separations.
    assert!(
        MIN_HUE_SEPARATION_DEG > 0.0 && MIN_HUE_SEPARATION_DEG <= least,
        "the calibrated minimum {MIN_HUE_SEPARATION_DEG}° is not below the least achieved {least:.3}°"
    );
}

negative_control!(
    golden_angle_adjacency_qa_consecutive_classes_up_to_96,
    "a palette stepping a half turn less 1/n, consecutive hues near each other at large n, fails",
    expected = "not the golden angle's",
    {
        check_adjacency(
            |i, n| render::present::hue_wheel((f64::from(i) * (0.5 - 1.0 / f64::from(n))).fract()),
            MIN_HUE_SEPARATION_DEG,
        );
    }
);

#[cfg(feature = "controls")]
mod golden_angle_adjacency_qa_minimum_bites {
    use super::*;
    negative_control!(
        golden_angle_adjacency_qa_consecutive_classes_up_to_96,
        "a minimum above the golden angle's separation fails on the real palette",
        expected = "below the minimum",
        {
            check_adjacency(palette::generated, 138.0);
        }
    );
}

// ── REQ-COL-162: the calibration hook is one value in both twins ─────────────────────────────────────────────────

/// The value of `const <name>: f32 = <v>;` in `wgsl`.
fn wgsl_const(wgsl: &str, name: &str) -> f64 {
    let line = wgsl
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("const {name}:")))
        .unwrap_or_else(|| panic!("the WGSL has no `const {name}`"));
    let v = line.split('=').nth(1).expect("an initialiser");
    v.trim()
        .trim_end_matches(';')
        .trim()
        .parse()
        .expect("a number")
}

/// Checks that the WGSL's mute constants are `(l_mute, k_mute)`, the CPU twin's calibration hook, as f32, and that both are a lightness and a fraction: `L_mute` in (0, 1), `k_mute` in [0, 1) (a mute, not an identity).
fn check_hook(wgsl: &str, (l_mute, k_mute): (f64, f64)) {
    for (name, want) in [("CAT_MUTE_L", l_mute), ("CAT_MUTE_CHROMA", k_mute)] {
        let got = wgsl_const(wgsl, name);
        assert_eq!(
            got as f32, want as f32,
            "the WGSL's {name} is {got}, not the hook's {want}"
        );
    }
    assert!(
        l_mute > 0.0 && l_mute < 1.0,
        "L_mute {l_mute} is no lightness"
    );
    assert!(
        (0.0..1.0).contains(&k_mute),
        "k_mute {k_mute} is not a fraction below 1"
    );
}

#[test]
fn categorical_filter_qa_mute_values_are_one_hook() {
    check_hook(palette::WGSL, (MUTE_L, MUTE_CHROMA));
}

negative_control!(
    categorical_filter_qa_mute_values_are_one_hook,
    "a WGSL whose mute lightness drifted from the CPU twin's fails",
    expected = "the WGSL's CAT_MUTE_L is",
    check_hook(
        &palette::WGSL.replace(
            &format!("const CAT_MUTE_L: f32 = {MUTE_L};"),
            "const CAT_MUTE_L: f32 = 0.5;"
        ),
        (MUTE_L, MUTE_CHROMA)
    )
);

#[cfg(feature = "controls")]
mod categorical_filter_qa_identity_mute {
    use super::*;
    negative_control!(
        categorical_filter_qa_mute_values_are_one_hook,
        "a hook whose k_mute is 1, keeping every chroma, fails",
        expected = "is not a fraction below 1",
        check_hook(
            &palette::WGSL.replace(
                &format!("const CAT_MUTE_CHROMA: f32 = {MUTE_CHROMA};"),
                "const CAT_MUTE_CHROMA: f32 = 1.0;"
            ),
            (MUTE_L, 1.0)
        )
    );
}

// ── REQ-COL-058: the mute's structure, on the CPU ────────────────────────────────────────────────────────────────

/// Checks `mute` on every class of the palettes colour_composition §1.4 names (the outcome table, Okabe–Ito, the
/// golden angle's 96): OKLab lightness `L_mute`; `a`, `b` scaled by `k_mute`, so the hue is kept and the chroma is the
/// fraction `k_mute`; at least τ from the invalid colour `#141418` and from every other class of its palette.
fn check_mute(mute: fn(Rgb) -> Rgb) {
    let invalid = hex("#141418");
    let sets: Vec<(&str, Vec<Rgb>)> = vec![
        ("outcome", TABLE.iter().map(|h| hex(h)).collect()),
        (
            "Okabe–Ito",
            OKABE_ITO_PUBLISHED.iter().map(|h| hex(h)).collect(),
        ),
        (
            "golden angle 96",
            (0..FIBONACCI_MAX)
                .map(|i| palette::generated(i, FIBONACCI_MAX))
                .collect(),
        ),
    ];
    for (name, set) in &sets {
        for (i, &c) in set.iter().enumerate() {
            let m = mute(c);
            let ([_, a, b], [lm, am, bm]) = (oklab(c), oklab(m));
            assert!(
                (lm - MUTE_L).abs() < 1e-6,
                "{name} class {i}'s mute has L {lm}, not L_mute {MUTE_L}"
            );
            assert!(
                (am - MUTE_CHROMA * a).abs() < 1e-6 && (bm - MUTE_CHROMA * b).abs() < 1e-6,
                "{name} class {i}'s mute has (a, b) ({am}, {bm}), not k_mute·({a}, {b})"
            );
            assert!(
                dist(m, invalid) >= TAU,
                "{name} class {i}'s mute is {:.4} from the invalid colour, within τ",
                dist(m, invalid)
            );
            for (j, &s) in set.iter().enumerate().filter(|&(j, _)| j != i) {
                assert!(
                    dist(m, s) >= TAU,
                    "{name} class {i}'s mute is {:.4} from shown class {j}, within τ",
                    dist(m, s)
                );
            }
        }
    }
}

#[test]
fn categorical_filter_qa_mute_structure() {
    check_mute(palette::mute);
}

negative_control!(
    categorical_filter_qa_mute_structure,
    "a mute that greys every class to one colour, losing the hue, fails",
    expected = "not k_mute",
    check_mute(|_| render::present::oklab_to_linear([MUTE_L, 0.0, 0.0]))
);

// ── REQ-COL-019, REQ-COL-058: the filter on the rendered state map ───────────────────────────────────────────────

const TILE: u32 = 4;
/// payload §2's `state` codes (the kernel's generated `STATE_*`).
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;

/// The background the renders take, a value of its own.
const BG: [f32; 3] = [0.5, 0.0, 0.25];

/// `(state, detail, forbidden, class)`: each sample and its row of §1.4's table (`None`: no class, the background).
const SAMPLES: [(u32, u32, bool, Option<usize>); 14] = [
    (BOUNDED, 0, false, Some(0)),
    (RUNNING, 0, false, Some(0)),
    (STATE_SIM_FAILED, 2, false, Some(0)),
    (COLLISION, 2, false, Some(1)),
    (COLLISION, 1, false, Some(2)),
    (COLLISION, 0, false, Some(3)),
    (COLLISION, 3, false, Some(4)),
    (ESCAPE, 0, false, Some(5)),
    (ESCAPE, 1, false, Some(6)),
    (ESCAPE, 2, false, Some(7)),
    (ESCAPE, 3, false, Some(8)),
    (STATE_DECODE_FAILED, 0, false, None),
    (ESCAPE, 2, true, None),
    (COLLISION, 2, true, None),
];

/// R-449's filters and the escaper view, as bits of the table's rows, plus no class shown, and only bits no outcome
/// class has.
fn filters() -> Vec<(&'static str, u32)> {
    vec![
        ("just collisions", 0b1_1110),
        ("just body 3's escape", 1 << 7),
        ("the escape classes", 0b1_1110_0000),
        ("none", 0),
        ("bits past the classes", 0xFFFF_FE00),
    ]
}

fn grid() -> Grid {
    Grid::new([SAMPLES.len() as u32, 1], 1, 0, TILE).expect("the grid")
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

fn stain(colour: Occupant) -> Stain {
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

/// Each sample's colour, its tile's pixels checked uniform, over the scene with `mask` and swatch set `set`.
fn render(h: &GpuHarness, colour: &Occupant, mask: u32, set: u32) -> Vec<Rgb> {
    let g = grid();
    let mut s = Synthetic::flat(g, 3);
    for (x, &(state, detail, forbidden, _)) in SAMPLES.iter().enumerate() {
        let i = g.cell(x as u32 * TILE + TILE / 2, TILE / 2).sample;
        s.sample(i).state(state).detail(detail).times(40, 0);
        s.set_forbidden(i, forbidden);
    }
    let px = render_stain_with(
        h.device(),
        h.queue(),
        &stain(colour.clone()),
        &s,
        &context(),
        &|_, name| match name {
            n if n == palette::MASK_PARAM => Some(vec![f64::from(mask)]),
            n if n == outcome::SET_PARAM => Some(vec![f64::from(set)]),
            _ => None,
        },
        Inputs {
            background: BG,
            ..Inputs::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
    .pixels;
    let width = SAMPLES.len() as u32 * TILE;
    (0..SAMPLES.len() as u32)
        .map(|x| {
            let first = px[(x * TILE) as usize];
            for py in 0..TILE {
                for pxx in x * TILE..(x + 1) * TILE {
                    assert_eq!(
                        px[(py * width + pxx) as usize],
                        first,
                        "sample {x}'s tile is not one colour"
                    );
                }
            }
            [0, 1, 2].map(|c| f64::from(first[c]))
        })
        .collect()
}

/// f32 evaluation on the GPU against this file's f64 OKLab: a bound well above f32 round-off through the cube roots.
const GPU_TOL: f64 = 1e-4;

/// Checks the state map through `colour`: unfiltered, set 0 is §1.4's table (and a sample with no class the
/// background); under each filter, for both swatch sets, a shown class is bit-identical to its unfiltered pixel, a
/// muted class is the §1.4 mute of its unfiltered colour (L_mute; k_mute·(a, b)) at least τ from `#141418` and from
/// every shown class of the render, and a sample with no class is the background exactly.
fn check_filter(colour: Occupant) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let bg = BG.map(f64::from);
    let plain0 = render(&h, &colour, ALL, 0);
    for (x, &(_, _, _, class)) in SAMPLES.iter().enumerate() {
        let want = class.map_or(bg, |k| hex(TABLE[k]));
        assert!(
            (0..3).all(|c| (plain0[x][c] - want[c]).abs() < 1e-6),
            "unfiltered: sample {x} is {:?}, not {}",
            plain0[x],
            class.map_or("the background", |k| TABLE[k])
        );
    }
    let invalid = hex("#141418");
    for set in [0u32, 1] {
        let plain = render(&h, &colour, ALL, set);
        for (name, mask) in filters() {
            let got = render(&h, &colour, mask, set);
            let shown: Vec<Rgb> = SAMPLES
                .iter()
                .enumerate()
                .filter(|(_, s)| s.3.is_some_and(|k| (mask >> k) & 1 == 1))
                .map(|(x, _)| got[x])
                .collect();
            for (x, &(state, detail, _, class)) in SAMPLES.iter().enumerate() {
                let at = format!("set {set}, {name}: sample {x} (state {state}, detail {detail})");
                let Some(k) = class else {
                    assert_eq!(got[x], bg, "{at}: no class, yet not the background");
                    continue;
                };
                if (mask >> k) & 1 == 1 {
                    assert_eq!(got[x], plain[x], "{at}: shown, yet changed by the filter");
                    continue;
                }
                let ([_, a, b], [l, am, bm]) = (oklab(plain[x]), oklab(got[x]));
                assert!(
                    (l - MUTE_L).abs() < GPU_TOL,
                    "{at}: muted, yet OKLab L {l}, not L_mute {MUTE_L}"
                );
                assert!(
                    (am - MUTE_CHROMA * a).abs() < GPU_TOL
                        && (bm - MUTE_CHROMA * b).abs() < GPU_TOL,
                    "{at}: muted, yet (a, b) ({am}, {bm}), not k_mute·({a}, {b})"
                );
                assert!(
                    dist(got[x], invalid) >= TAU,
                    "{at}: muted, yet {:.4} from the invalid colour",
                    dist(got[x], invalid)
                );
                for s in &shown {
                    assert!(
                        dist(got[x], *s) >= TAU,
                        "{at}: muted, yet {:.4} from a shown class",
                        dist(got[x], *s)
                    );
                }
            }
        }
    }
}

#[test]
fn categorical_filter_qa_state_map_filters() {
    check_filter(Occupant::BuiltIn(outcome::ID.to_owned()));
}

#[cfg(feature = "controls")]
fn altered(from: &str, to: &str) -> Occupant {
    assert!(outcome::WGSL.contains(from), "the occupant has no `{from}`");
    Occupant::Custom(outcome::WGSL.replace(from, to))
}

negative_control!(
    categorical_filter_qa_state_map_filters,
    "an occupant whose 'mute' only dims each class to a fifth of its light fails",
    expected = "muted, yet OKLab L",
    check_filter(altered(
        "return cat_filter(rgb, u32(k), uniforms.class_mask);",
        "return select(rgb * 0.2, rgb, cat_shown(u32(k), uniforms.class_mask));"
    ))
);

#[cfg(feature = "controls")]
mod categorical_filter_qa_reversed_bits {
    use super::*;
    negative_control!(
        categorical_filter_qa_state_map_filters,
        "an occupant reading the mask's bits in reverse class order fails",
        expected = "set 0, just collisions: sample 0",
        check_filter(altered(
            "return cat_filter(rgb, u32(k), uniforms.class_mask);",
            "return cat_filter(rgb, 8u - u32(k), uniforms.class_mask);"
        ))
    );
}

#[cfg(feature = "controls")]
mod categorical_filter_qa_muted_background {
    use super::*;
    negative_control!(
        categorical_filter_qa_state_map_filters,
        "an occupant that filters a sample with no class as class 0 fails",
        expected = "no class, yet not the background",
        check_filter(altered(
            "if (k < 0) {\n        return rgb;\n    }",
            "if (k < -1) {\n        return rgb;\n    }"
        ))
    );
}
