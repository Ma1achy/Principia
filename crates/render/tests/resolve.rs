//! The colour resolve (`render::resolve`): each copy's records drawn through the same stain, the colours averaged into
//! the pixel, last, in linear RGB.
//!
//! The pass criteria, fixed before the first run:
//! - **the 75/25 blend**: a pixel whose four copies are three `escape` (body 0) and one `collision` (pair 0) resolves,
//!   through the canonical outcome stain, to the 8-bit sRGB code of `0.75·lin(#F0DE32) + 0.25·lin(#3462E0)` in each
//!   channel, `lin` the sRGB transfer decoded, rounded once to 8 bits: exactly, no tolerance. A pixel whose four copies
//!   are three `collision` and one `decode_failed` resolves the same way with the canvas background `#0A0A0A` as the
//!   failed copy's colour, on an sRGB-typed target and on the GUI's plain `Rgba8Unorm` one. Controls: the class index
//!   averaged (the copies' mean `state`, rounded, coloured by the CPU palette), and the records averaged before
//!   colouring (one record set holding each pixel's mean `state`, drawn through the stain once): both land on another
//!   class's colour (`bounded` for the first pixel);
//! - **one copy is the stain's picture**: the resolve of one copy returns every 8-bit code unchanged (all 256 in every
//!   channel and alpha), and `resolve::draw` of one copy equals the view's own render byte for byte. Controls: an
//!   encode that truncates instead of rounding, and a second, different copy added.
//!
//! Each test registers its negative control, which breaks its property and shows the check fails.

use ledger::gen::read::Tier;
use render::colour::outcome::{self, Shown};
use render::pipeline_cache::{FrameInputs, NodeKey};
use render::present::{self, linear_to_srgb, srgb8};
use render::raster::Grid;
use render::resolve::{self, resolve_srgb8};
use render::stain_view::{Records, StainView};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::stain::{NodeId, NodeKind, Occupant, StainGraph};
use validation::synthetic::{Bytes, Synthetic};

/// Two pixels across, one up.
const W: u32 = 2;
const H: u32 = 1;

const INPUTS: FrameInputs = FrameInputs {
    ensemble: 0,
    ensemble_spread: 0.0,
    masses: [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
    dt_macro: 0.01,
    delta_0: 1e-6,
    n_renorm: 16,
    horizon_steps: 1000,
};

const ESCAPE: u32 = 0;
const COLLISION: u32 = 2;
const DECODE_FAILED: u32 = 5;

/// Each copy's `state` at each pixel, detail 0 throughout: pixel 0 is three `escape` and one `collision`, pixel 1 three
/// `collision` and one `decode_failed`.
const STATES: [[u32; 2]; 4] = [
    [ESCAPE, COLLISION],
    [COLLISION, DECODE_FAILED],
    [ESCAPE, COLLISION],
    [ESCAPE, COLLISION],
];

/// A field of `W × H` samples holding `states`, detail 0.
fn field(states: [u32; 2]) -> Bytes {
    let grid = Grid::new([W * H, 1], 1, 0, 1).unwrap_or_else(|e| panic!("{e}"));
    let mut set = Synthetic::flat(grid, 0);
    for (i, s) in states.iter().enumerate() {
        set.sample(i as u32).state(*s).detail(0).times(40, 0);
    }
    set.bytes()
}

fn records(b: &Bytes) -> Records<'_> {
    Records {
        simstate: &b.simstate,
        word: &b.word,
        ic: &b.ic,
    }
}

/// A view over `first`'s records with the canonical outcome stain compiled.
fn view(h: &GpuHarness, first: &Bytes, format: wgpu::TextureFormat) -> StainView {
    let mut v = StainView::new(
        h.device(),
        h.queue(),
        format,
        [W, H],
        records(first),
        Tier::FULL,
        INPUTS,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut g = StainGraph::new();
    let source = g
        .add(NodeKind::Source, Occupant::Field("ftle".into()))
        .unwrap_or_else(|e| panic!("{e}"));
    let colour = g
        .add(NodeKind::Colour, Occupant::Builtin(outcome::name().into()))
        .unwrap_or_else(|e| panic!("{e}"));
    g.connect(source, colour, 0)
        .unwrap_or_else(|e| panic!("{e}"));
    g.connect(colour, NodeId(0), 0)
        .unwrap_or_else(|e| panic!("{e}"));
    let stain = g
        .canonical()
        .unwrap_or_else(|e| panic!("{e}"))
        .stain()
        .clone();
    let keys: Vec<NodeKey> = (0..stain.nodes().len() as u64).map(|k| 100 + k).collect();
    v.set_stain(&stain, &keys).unwrap_or_else(|e| panic!("{e}"));
    v.wait();
    assert!(
        v.errors().is_empty(),
        "the outcome stain failed: {:?}",
        v.errors()
    );
    v
}

/// The colour the outcome palette draws `state` (detail 0) in, 8-bit sRGB.
fn palette(state: u32) -> [u8; 3] {
    match outcome::shown(state, 0, false) {
        Shown::Swatch(k) => outcome::SWATCHES[k].srgb8,
        Shown::Forbidden | Shown::DecodeFailed => present::CANVAS_BACKGROUND_SRGB8,
    }
}

/// The pass criterion's colour: `0.75·lin(a) + 0.25·lin(b)`, encoded and rounded once.
fn blend(a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
    let (a, b) = (srgb8(a), srgb8(b));
    core::array::from_fn(|c| (linear_to_srgb(0.75 * a[c] + 0.25 * b[c]) * 255.0).round() as u8)
}

/// The picture's pixels, left to right (one row).
fn pixels(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.chunks(4).map(|t| [t[0], t[1], t[2], t[3]]).collect()
}

fn check_blend(picture: &[u8]) {
    let want = [
        blend(palette(ESCAPE), palette(COLLISION)),
        blend(palette(COLLISION), present::CANVAS_BACKGROUND_SRGB8),
    ];
    let got = pixels(picture);
    println!("resolved {got:?}, want {want:?}");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(
            g[..3] == w && g[3] == 255,
            "pixel {i} resolves to {g:?}, not the 75/25 blend {w:?}"
        );
    }
}

fn copies() -> Vec<Bytes> {
    STATES.iter().map(|s| field(*s)).collect()
}

/// The resolve under test into `format` targets: every copy drawn through the stain, then averaged.
fn resolved(format: wgpu::TextureFormat) -> Vec<u8> {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let copies = copies();
    let mut v = view(&h, &copies[0], format);
    let all: Vec<Records> = copies.iter().map(records).collect();
    resolve::draw(&mut v, &all)
        .unwrap_or_else(|e| panic!("{e}"))
        .bytes
}

/// Each pixel's mean `state` over the copies, rounded: an averaged class index.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn mean_states() -> [u32; 2] {
    core::array::from_fn(|i| {
        let sum: u32 = STATES.iter().map(|s| s[i]).sum();
        (f64::from(sum) / STATES.len() as f64).round() as u32
    })
}

/// The two target formats: sRGB-typed, and the GUI's plain 8-bit one, which stores the codes unconverted.
const FORMATS: [wgpu::TextureFormat; 2] = [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
];

#[test]
fn a_three_to_one_pixel_resolves_to_the_75_25_blend() {
    for format in FORMATS {
        check_blend(&resolved(format));
    }
}

negative_control!(
    a_three_to_one_pixel_resolves_to_the_75_25_blend,
    "the class index averaged and then coloured is another class's colour",
    expected = "not the 75/25 blend",
    {
        let picture: Vec<u8> = mean_states()
            .iter()
            .flat_map(|s| {
                let [r, g, b] = palette(*s);
                [r, g, b, 255]
            })
            .collect();
        check_blend(&picture)
    }
);

/// The same criterion, guarding that the records are coloured before they are averaged.
#[test]
fn the_copies_are_coloured_before_they_are_averaged() {
    for format in FORMATS {
        check_blend(&resolved(format));
    }
}

negative_control!(
    the_copies_are_coloured_before_they_are_averaged,
    "the records averaged before colouring draw another class's colour",
    expected = "not the 75/25 blend",
    {
        let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
        let averaged = field(mean_states());
        let mut v = view(&h, &averaged, wgpu::TextureFormat::Rgba8UnormSrgb);
        check_blend(&v.render(W, H).unwrap_or_else(|e| panic!("{e}")).bytes)
    }
);

// --- one copy is the stain's picture -------------------------------------------------------------------------------

fn check_codes_round_trip(resolve: impl Fn(&[&[u8]]) -> Vec<u8>) {
    let all: Vec<u8> = (0..=255u8).flat_map(|c| [c, c, c, c]).collect();
    let got = resolve(&[&all]);
    for (i, (g, w)) in got.iter().zip(&all).enumerate() {
        assert_eq!(g, w, "one copy's code {w} (byte {i}) resolves to {g}");
    }
}

#[test]
fn one_copy_round_trips_every_code() {
    check_codes_round_trip(|c| resolve_srgb8(c).unwrap_or_else(|e| panic!("{e}")));
}

negative_control!(
    one_copy_round_trips_every_code,
    "an encode that truncates moves codes",
    expected = "resolves to",
    check_codes_round_trip(|c| {
        c[0].iter()
            .map(|&x| {
                let l = present::srgb_to_linear(f64::from(x) / 255.0);
                (linear_to_srgb(l) * 255.0 * (1.0 - 1e-12)) as u8
            })
            .collect()
    })
);

fn check_one_copy_is_the_render(extra: bool) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let copies = copies();
    let mut v = view(&h, &copies[0], wgpu::TextureFormat::Rgba8UnormSrgb);
    let own = v.render(W, H).unwrap_or_else(|e| panic!("{e}"));
    let mut given = vec![records(&copies[0])];
    if extra {
        given.push(records(&copies[1]));
    }
    let drawn = resolve::draw(&mut v, &given).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        drawn.bytes == own.bytes && drawn.format == own.format,
        "one copy's resolve {:?} is not the view's own picture {:?}",
        drawn.bytes,
        own.bytes
    );
}

#[test]
fn one_copy_is_the_views_picture() {
    check_one_copy_is_the_render(false);
}

negative_control!(
    one_copy_is_the_views_picture,
    "a second, different copy changes the picture",
    expected = "is not the view's own picture",
    check_one_copy_is_the_render(true)
);
