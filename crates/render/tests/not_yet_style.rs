//! The "not yet" style (R-399; REQ-TOOL-162, REQ-COL-064's proposal, confirmed with its dots at `#909090`, R-434;
//! TASK-M1-17): an unset `d_min` and a running sample are drawn by `debug_not_yet(frag_xy)`, the grey `DBG_NOT_YET`
//! with a fine dot stipple from the pixel position, distinct from the invalid hatch's stripes, so no ramp's flat colour
//! reads as it:
//! - the presentation layer's dot colour is its CPU mirror's, and `debug_not_yet` on the GPU is the mirror at every
//!   pixel (`not_yet_style_mirror_*`);
//! - over any 4 × 4 pixel block, at every phase, the style takes at least two colours, its mean, in linear light, is
//!   within [`MEAN_BOUND`] of `DBG_NOT_YET` in OKLab, and it differs from `debug_invalid` at every pixel
//!   (`not_yet_style_blocks_*`);
//! - REQ-COL-064's evidence, as confirmed (R-434): the pattern, its lightness contrast over the grey, its distance from
//!   the hatch's two colours and from the pending hatch's blue, and that no flat colour of any palette (the outcome
//!   palette, the `dbg_*` palettes, every LUT of colour_composition §7.1, viridis, twilight, magma and cool-warm among
//!   them, the grey ramp and the hue circle) comes within [`FLAT_BOUND`], the hatch's τ, of every pixel of a 4 × 4
//!   block (`not_yet_style_proposal_*`). Run with `--nocapture` for the table the PR attaches.
//!
//! Each test registers its negative control (R-176).

use std::path::Path;

use ledger::gen::prelude;
use render::present::{self, Rgb};
use validation::gpu::GpuHarness;
use validation::negative_control;

#[path = "support/palettes.rs"]
mod palettes;

/// The bound on the OKLab distance between a 4 × 4 block's mean colour, averaged in linear light, and `DBG_NOT_YET`: a
/// quarter of each block is the lighter dot, which lifts the mean 0.079 in OKLab lightness with R-434's `#909090` dots
/// (0.057 with the proposal's `#808080`, under its bound of 0.06; applied per R-369, the bound follows the dot).
const MEAN_BOUND: f64 = 0.08;

/// The bound below which no flat colour may come of every pixel of a 4 × 4 block: τ, 0.101, the OKLab distance from
/// `#FF00FF` to `#E034C6` that the hatch is held to (R-429), the bar R-434 sets for the stipple, in place of the
/// proposal's 0.08 (half the OKLab distance between the grey and the `#808080` dot, 0.176, less a margin).
const FLAT_BOUND: f64 = 0.101;

/// The rendered square's side, in pixels: four of the stipple's 4 px cells, so every phase of a 4 × 4 block fits.
const SIDE: u32 = 16;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The OKLab distance between two linear colours.
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (present::linear_to_oklab(a), present::linear_to_oklab(b));
    a.iter()
        .zip(&b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The constant `name`'s three channels, as `present.wgsl` declares it.
fn wgsl_constant(name: &str) -> [f32; 3] {
    let source = read("crates/render/shaders/wgsl/lib/present.wgsl");
    let head = format!("const {name}: vec3<f32> = vec3<f32>(");
    let line = source
        .lines()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| panic!("present.wgsl declares no {name}"));
    let v: Vec<f32> = line[head.len()..]
        .trim_end_matches(");")
        .split(',')
        .map(|x| x.trim().parse().unwrap_or_else(|e| panic!("`{x}`: {e}")))
        .collect();
    [v[0], v[1], v[2]]
}

/// Checks that the WGSL channels `wgsl` are each the f32 nearest `mirror`'s.
fn check_constant(wgsl: [f32; 3], mirror: Rgb) {
    for (w, m) in wgsl.iter().zip(&mirror) {
        assert_eq!(*w, *m as f32, "{wgsl:?} is not the CPU mirror {mirror:?}");
    }
}

/// `debug_not_yet` at every pixel of a `SIDE` × `SIDE` target, rows from the top, rendered on the GPU after the
/// prelude and the presentation layer, `from` replaced by `to` in the presentation layer: a control's one mutation.
fn rendered(from: &str, to: &str) -> Vec<Rgb> {
    let present = read("crates/render/shaders/wgsl/lib/present.wgsl");
    assert!(present.contains(from), "the mutation's target `{from}`");
    let module = format!(
        "{}\n{}\n@fragment\nfn t_not_yet(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<u32> {{\n    \
         return vec4<u32>(bitcast<vec3<u32>>(debug_not_yet(pos.xy)), 0u);\n}}\n",
        read(prelude::PATH),
        present.replace(from, to)
    );
    let gpu = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let kernel = gpu
        .fragment(&module, "t_not_yet", &[], SIDE, SIDE)
        .unwrap_or_else(|e| panic!("{e}"));
    kernel
        .draw(&[])
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [0, 1, 2].map(|c| f64::from(f32::from_bits(p[c]))))
        .collect()
}

/// The fragment position of pixel `k` of the target: its centre.
fn frag(k: usize) -> [f64; 2] {
    [
        (k % SIDE as usize) as f64 + 0.5,
        (k / SIDE as usize) as f64 + 0.5,
    ]
}

/// Checks that each pixel of `image` is the CPU mirror's `debug_not_yet` there.
fn check_mirror(image: &[Rgb]) {
    for (k, got) in image.iter().enumerate() {
        let want = present::debug_not_yet(frag(k));
        assert!(
            got.iter().zip(&want).all(|(g, w)| (g - w).abs() < 1e-7),
            "pixel {:?}: the shader's {got:?} is not the mirror's {want:?}",
            frag(k)
        );
    }
}

#[test]
fn not_yet_style_mirror_is_present_wgsl() {
    check_constant(wgsl_constant("DBG_NOT_YET_DOT"), present::not_yet_dot());
    check_constant(wgsl_constant("DBG_NOT_YET"), present::not_yet());
    let [r, g, b] = present::NOT_YET_DOT_SRGB8;
    assert!(r == g && g == b, "the dot is neutral");
    check_mirror(&rendered("", ""));
}

negative_control!(
    not_yet_style_mirror_is_present_wgsl,
    "dots one pixel wide, not two, are not the mirror's",
    expected = "is not the mirror's",
    check_mirror(&rendered(
        "((p.x | p.y) & 2) == 0",
        "((p.x | p.y) & 3) == 0"
    ))
);

/// The pixels of the 4 × 4 block at `(x, y)`, as indices into a `SIDE`-wide image.
fn block(x: usize, y: usize) -> impl Iterator<Item = usize> {
    (0..16).map(move |j| (y + j / 4) * SIDE as usize + x + j % 4)
}

/// Checks every 4 × 4 block of `image`, at every phase: at least two colours, a mean within [`MEAN_BOUND`] of the grey,
/// and every pixel apart from `debug_invalid`'s there.
fn check_blocks(image: &[Rgb]) {
    let grey = present::not_yet();
    for y in 0..=(SIDE as usize - 4) {
        for x in 0..=(SIDE as usize - 4) {
            let pixels: Vec<usize> = block(x, y).collect();
            let first = image[pixels[0]];
            assert!(
                pixels.iter().any(|&k| distance(image[k], first) > 0.0),
                "the block at ({x}, {y}) is one flat colour"
            );
            let mean = [0, 1, 2].map(|c| pixels.iter().map(|&k| image[k][c]).sum::<f64>() / 16.0);
            let d = distance(mean, grey);
            assert!(
                d <= MEAN_BOUND,
                "the block at ({x}, {y}) averages {d:.3} from DBG_NOT_YET, beyond {MEAN_BOUND}"
            );
            for &k in &pixels {
                let hatch = present::debug_invalid(frag(k));
                assert!(
                    distance(image[k], hatch) > FLAT_BOUND,
                    "pixel {:?} is the hatch's colour",
                    frag(k)
                );
            }
        }
    }
}

#[test]
fn not_yet_style_blocks_are_stippled_grey_never_the_hatch() {
    check_blocks(&rendered("", ""));
}

negative_control!(
    not_yet_style_blocks_are_stippled_grey_never_the_hatch,
    "the flat grey TASK-M1-09 drew, before R-399",
    expected = "is one flat colour",
    check_blocks(&rendered(
        "return select(DBG_NOT_YET, DBG_NOT_YET_DOT, ((p.x | p.y) & 2) == 0);",
        "return DBG_NOT_YET;"
    ))
);

/// The largest OKLab distance from the flat colour `c` to a pixel of the 4 × 4 block at the origin of `style`: how far
/// `c` is from reproducing the block.
fn block_reach(style: fn([f64; 2]) -> Rgb, c: Rgb) -> f64 {
    (0..16)
        .map(|j| style([f64::from(j % 4) + 0.5, f64::from(j / 4) + 0.5]))
        .map(|p| distance(c, p))
        .fold(0.0, f64::max)
}

/// Checks that no colour of `palette` comes within [`FLAT_BOUND`] of every pixel of a 4 × 4 block of `style`, and
/// returns the nearest.
fn check_no_flat(style: fn([f64; 2]) -> Rgb, palette: &[(String, Rgb)]) -> (f64, String) {
    let (reach, name) = palette
        .iter()
        .map(|(name, c)| (block_reach(style, *c), name.clone()))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or((f64::INFINITY, String::new()));
    assert!(
        reach >= FLAT_BOUND,
        "the flat colour {name} comes within {reach:.3} of every pixel of a 4 × 4 block"
    );
    (reach, name)
}

#[test]
fn not_yet_style_proposal_evidence() {
    let (grey, dot) = (present::not_yet(), present::not_yet_dot());
    let lab = |c| present::linear_to_oklab(c)[0];
    let contrast = lab(dot) - lab(grey);
    println!(
        "REQ-COL-064 proposal: #{:02X}{:02X}{:02X} dots, {} × {} px in each {} × {} px cell, at p = ⌊frag_xy⌋ in \
         framebuffer (physical) pixels, over the grey #{:02X}{:02X}{:02X} (REQ-COL-053)",
        present::NOT_YET_DOT_SRGB8[0],
        present::NOT_YET_DOT_SRGB8[1],
        present::NOT_YET_DOT_SRGB8[2],
        present::NOT_YET_DOT,
        present::NOT_YET_DOT,
        present::NOT_YET_CELL,
        present::NOT_YET_CELL,
        present::NOT_YET_SRGB8[0],
        present::NOT_YET_SRGB8[1],
        present::NOT_YET_SRGB8[2]
    );
    println!(
        "  OKLab lightness: grey {:.3}, dot {:.3}, contrast {contrast:.3}; the hatch's two colours are {:.3} apart",
        lab(grey),
        lab(dot),
        {
            let [a, b] = prelude::hatch().colours.map(present::srgb8);
            lab(b) - lab(a)
        }
    );
    let mean = [0, 1, 2].map(|c| (3.0 * grey[c] + dot[c]) / 4.0);
    println!(
        "  a 4 × 4 block's mean, in linear light: OKLab L {:.3}, {:.3} from the grey",
        lab(mean),
        distance(mean, grey)
    );
    for c in prelude::hatch().colours {
        let h = present::srgb8(c);
        println!(
            "  hatch #{:02X}{:02X}{:02X}: {:.3} from the grey, {:.3} from the dot",
            c[0],
            c[1],
            c[2],
            distance(h, grey),
            distance(h, dot)
        );
    }
    let pending = palettes::hex(0x0000FF);
    println!(
        "  pending hatch #0000FF (REQ-TOOL-124): {:.3} from the grey, {:.3} from the dot",
        distance(pending, grey),
        distance(pending, dot)
    );
    let mut palette = palettes::entries();
    palette.extend((0..=4096).map(|k| {
        let t = f64::from(k) / 4096.0;
        (format!("ramp_coolwarm({t:.4})"), present::ramp_coolwarm(t))
    }));
    palette.extend((0..=4096).map(|k| {
        let t = f64::from(k) / 4096.0;
        (format!("ramp_magma({t:.4})"), present::ramp_magma(t))
    }));
    let (reach, name) = check_no_flat(present::debug_not_yet, &palette);
    println!(
        "  the nearest flat colour to a 4 × 4 block, over {} palette colours: {name}, {reach:.3} from its farthest \
         pixel",
        palette.len()
    );
    assert_eq!(
        format!("{contrast:.3}"),
        "0.229",
        "the contrast the docs give"
    );
    assert_eq!(
        format!("{:.3}", distance(mean, grey)),
        "0.079",
        "the mean the docs give"
    );
    assert_eq!(
        format!("{reach:.3}"),
        "0.115",
        "the nearest flat colour's reach R-434 gives"
    );
}

negative_control!(
    not_yet_style_proposal_evidence,
    "the flat grey, before R-399, is reproduced by the grey ramp, RQ-259's collision",
    expected = "comes within",
    {
        check_no_flat(|_| present::not_yet(), &palettes::entries());
    }
);
