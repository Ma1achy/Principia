//! The supersampled picture of the CPU field: each copy's records (`engine::field::ensemble`) drawn through the
//! canonical outcome stain and resolved in linear RGB (`render::resolve`).
//!
//! The pass criteria, fixed before the first run:
//! - **E = 0 is the un-jittered picture**: on the zero view at 8 × 8, the resolve of the field's E = 0 copies equals,
//!   byte for byte, the stain view's own picture of `Cpu::stored_on`'s records and the field's outcome image
//!   (`Field::rgba8`, what the GUI's field uploads). Control: copy 0 jittered (copy 1 of
//!   E = 1 drawn as the only copy).
//! - **E = 15 is closer to the reference than E = 0** (a measurement, run by hand in release: `cargo test --release -p
//!   validation --test supersample_picture -- --ignored --nocapture`): the zero view at 64 × 64 (`z_α × z_β`, no tilt,
//!   no offset, horizon 50). The reference is each pixel's 16 × 16 regular grid of samples inside its footprint, at
//!   `((i + ½)/16 − ½, (j + ½)/16 − ½)` footprints from its centre, each drawn through the same stain and box-averaged
//!   in linear RGB by the same resolve. A pixel's error is the RMS over its three channels of the difference of the
//!   two pictures' 8-bit codes decoded to linear RGB. **Criterion: the mean error over the 4096 pixels at E = 15 is
//!   below the mean at E = 0.** Reported for E = 0, 1, 3, 7 and 15: the mean, the share of exact pixels, the
//!   quantiles and the whole distribution in bins, the pixels better and worse than E = 0, and the wall time of each
//!   E's own march. If it fails, it is reported, not moved.

use std::num::NonZeroUsize;
use std::time::Instant;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::record::TIER;
use engine::field::{Cpu, RecordBytes};
use engine::first_slice::Quad;
use engine::pool::Pool;
use kernel::chart::Latent;
use kernel::payload::ReadParams;
use render::colour::outcome;
use render::headless::Image;
use render::pipeline_cache::{FrameInputs, NodeKey};
use render::present::srgb_to_linear;
use render::resolve::{self, resolve_srgb8};
use render::stain_view::{Records, StainView};
use validation::gpu::GpuHarness;
use validation::negative_control;
use validation::stain::{NodeId, NodeKind, Occupant, StainGraph};

const HORIZON: f64 = 50.0;

/// The app's zero view at its horizon of 50 (`engine/tests/field_record.rs`'s).
fn zero_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let quad = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: HORIZON },
        ..quad.sim_key()
    }
}

fn pool() -> Pool {
    Pool::with_threads(NonZeroUsize::new(3).expect("workers")).expect("the pool")
}

fn records(b: &RecordBytes) -> Records<'_> {
    Records {
        simstate: &b.simstate,
        word: &b.word,
        ic: &b.ic,
    }
}

/// The read side's uniforms for records read at `read`: each copy is drawn as a plain field, no ensemble bound.
fn inputs(read: ReadParams) -> FrameInputs {
    FrameInputs {
        ensemble: 0,
        ensemble_spread: 0.0,
        masses: [1.0 / 3.0; 3],
        dt_macro: read.dt_macro,
        delta_0: read.delta_0,
        n_renorm: read.n_renorm,
        horizon_steps: read.horizon_steps,
    }
}

/// A view of a `size` field over `first`'s records with the canonical outcome stain compiled.
fn view(h: &GpuHarness, size: [u32; 2], first: &RecordBytes, read: ReadParams) -> StainView {
    let mut v = StainView::new(
        h.device(),
        h.queue(),
        wgpu::TextureFormat::Rgba8UnormSrgb,
        size,
        records(first),
        TIER,
        inputs(read),
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

/// The supersampled picture of `size` at `e`: every copy drawn and resolved.
fn picture(h: &GpuHarness, size: [usize; 2], e: u32) -> Image {
    let s = Cpu
        .supersampled_on(&pool(), &zero_view(), Window::SQUARE, size, e)
        .expect("the supersampled field");
    let bytes = s.bytes();
    let mut v = view(h, size.map(|n| n as u32), &bytes[0], s.read);
    let all: Vec<Records> = bytes.iter().map(records).collect();
    resolve::draw(&mut v, &all).unwrap_or_else(|e| panic!("{e}"))
}

// --- E = 0 is the un-jittered picture ----------------------------------------------------------------------------

const SMALL: [usize; 2] = [8, 8];

/// The pixels of `a` and `b`, RGBA8 rows from the top, that differ.
fn differing(a: &[u8], b: &[u8]) -> usize {
    assert_eq!(a.len(), b.len(), "the pictures differ in size");
    a.chunks(4).zip(b.chunks(4)).filter(|(a, b)| a != b).count()
}

/// The resolve of copy `copy` of the field at `e`, drawn as the only copy, against the field before supersampling:
/// the stain view's own picture of `stored_on`'s records, and the outcome image the GUI's field uploads
/// (`Field::rgba8`), a decode-failed sample's transparent texel read as the canvas background the stain draws there.
fn check_e0_is_unjittered_picture(copy: u32, e: u32) {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let plain = Cpu
        .stored_on(&pool(), &zero_view(), Window::SQUARE, SMALL)
        .expect("the field's records");
    let mut v = view(&h, SMALL.map(|n| n as u32), &plain.bytes(), plain.read);
    let own = v
        .render(SMALL[0] as u32, SMALL[1] as u32)
        .unwrap_or_else(|e| panic!("{e}"));
    let [r, g, b] = render::present::CANVAS_BACKGROUND_SRGB8;
    let gui: Vec<u8> = plain
        .field
        .rgba8()
        .chunks(4)
        .flat_map(|t| {
            if t[3] == 0 {
                [r, g, b, 255]
            } else {
                [t[0], t[1], t[2], t[3]]
            }
        })
        .collect();
    let s = Cpu
        .supersampled_on(&pool(), &zero_view(), Window::SQUARE, SMALL, e)
        .expect("the supersampled field");
    let copy = s.copy(copy).bytes();
    let mut w = view(&h, SMALL.map(|n| n as u32), &copy, s.read);
    let drawn = resolve::draw(&mut w, &[records(&copy)]).unwrap_or_else(|e| panic!("{e}"));
    let (by_view, by_gui) = (
        differing(&own.bytes, &drawn.bytes),
        differing(&gui, &drawn.bytes),
    );
    assert!(
        by_view == 0 && by_gui == 0,
        "E = 0's picture is not the un-jittered field's: {by_view} of 64 pixels differ from the view's picture, \
         {by_gui} from the field's outcome image"
    );
}

#[test]
fn e0_is_the_unjittered_picture() {
    check_e0_is_unjittered_picture(0, 0);
}

negative_control!(
    e0_is_the_unjittered_picture,
    "a jittered copy drawn as copy 0 is not the un-jittered picture",
    expected = "is not the un-jittered field's",
    check_e0_is_unjittered_picture(1, 1)
);

// --- the measurement -----------------------------------------------------------------------------------------------

const SIZE: [usize; 2] = [64, 64];
const GRID: usize = 16;

/// The reference: each pixel's `GRID × GRID` regular grid inside its footprint, each sub-grid point's whole field a
/// 64 × 64 field over the window moved by that point's offset, drawn through the stain and resolved.
fn reference(h: &GpuHarness) -> Image {
    let half = Window::SQUARE.half;
    let footprint = [0, 1].map(|a| 2.0 * half[a] / SIZE[a] as f64);
    let mut pictures = Vec::with_capacity(GRID * GRID);
    let mut view_of: Option<StainView> = None;
    for j in 0..GRID {
        for i in 0..GRID {
            let o = [i, j].map(|n| (n as f64 + 0.5) / GRID as f64 - 0.5);
            let window = Window {
                centre: [0, 1].map(|a| Window::SQUARE.centre[a] + o[a] * footprint[a]),
                half,
            };
            let s = Cpu
                .stored_on(&pool(), &zero_view(), window, SIZE)
                .expect("a reference field");
            let bytes = s.bytes();
            let v = view_of.get_or_insert_with(|| view(h, SIZE.map(|n| n as u32), &bytes, s.read));
            v.set_records(records(&bytes))
                .unwrap_or_else(|e| panic!("{e}"));
            pictures.push(
                v.render(SIZE[0] as u32, SIZE[1] as u32)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .bytes,
            );
        }
    }
    let all: Vec<&[u8]> = pictures.iter().map(Vec::as_slice).collect();
    Image {
        width: SIZE[0] as u32,
        height: SIZE[1] as u32,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        bytes: resolve_srgb8(&all).unwrap_or_else(|e| panic!("{e}")),
    }
}

/// Each pixel's error against `reference`: the RMS over RGB of the linear difference.
fn errors(picture: &Image, reference: &Image) -> Vec<f64> {
    picture
        .bytes
        .chunks(4)
        .zip(reference.bytes.chunks(4))
        .map(|(a, b)| {
            let sq: f64 = (0..3)
                .map(|c| {
                    let d = srgb_to_linear(f64::from(a[c]) / 255.0)
                        - srgb_to_linear(f64::from(b[c]) / 255.0);
                    d * d
                })
                .sum();
            (sq / 3.0).sqrt()
        })
        .collect()
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

fn save(path: &std::path::Path, image: &Image) {
    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("{e}"));
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    let mut w = enc.write_header().unwrap_or_else(|e| panic!("{e}"));
    w.write_image_data(&image.bytes)
        .unwrap_or_else(|e| panic!("{e}"));
}

/// The bins of the error distribution's report: upper edges, in linear RGB.
const BINS: [f64; 9] = [0.0, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.4, 1.0];

#[test]
#[ignore = "about 1.1 M marches: about 70 minutes in release on three workers"]
fn e15_is_closer_to_the_reference_than_e0() {
    let h = GpuHarness::new().unwrap_or_else(|e| panic!("{e}"));
    let out = std::env::var_os("SUPERSAMPLE_PICTURES").map(std::path::PathBuf::from);
    let started = Instant::now();
    let reference = reference(&h);
    println!(
        "reference: {GRID}×{GRID} per pixel, {} marches, {:.1} s",
        GRID * GRID * SIZE[0] * SIZE[1],
        started.elapsed().as_secs_f64()
    );
    if let Some(dir) = &out {
        save(&dir.join("reference.png"), &reference);
    }
    let mut means = Vec::new();
    let mut base: Option<Vec<f64>> = None;
    for e in [0, 1, 3, 7, 15] {
        let started = Instant::now();
        let p = picture(&h, SIZE, e);
        let seconds = started.elapsed().as_secs_f64();
        if let Some(dir) = &out {
            save(&dir.join(format!("e{e}.png")), &p);
        }
        let err = errors(&p, &reference);
        let mean = err.iter().sum::<f64>() / err.len() as f64;
        let mut sorted = err.clone();
        sorted.sort_by(f64::total_cmp);
        let exact = err.iter().filter(|&&x| x == 0.0).count();
        let mut bins = [0usize; BINS.len()];
        for &x in &err {
            bins[BINS.iter().position(|&b| x <= b).unwrap_or(BINS.len() - 1)] += 1;
        }
        let (better, worse) = base.as_ref().map_or((0, 0), |b| {
            (
                err.iter().zip(b).filter(|(x, y)| x < y).count(),
                err.iter().zip(b).filter(|(x, y)| x > y).count(),
            )
        });
        println!(
            "E = {e:2}: {seconds:6.1} s; mean {mean:.5}; exact {exact}/4096; p50 {:.5} p75 {:.5} p90 {:.5} p99 {:.5} max \
             {:.5}; vs E = 0 better {better} worse {worse}",
            quantile(&sorted, 0.5),
            quantile(&sorted, 0.75),
            quantile(&sorted, 0.9),
            quantile(&sorted, 0.99),
            sorted[sorted.len() - 1],
        );
        println!("        bins (≤ edge) {BINS:?}: {bins:?}");
        means.push((e, mean));
        if e == 0 {
            base = Some(err);
        }
    }
    check_closer(&means);
}

/// The criterion on the measured means, `(E, mean error)` from E = 0 to E = 15.
fn check_closer(means: &[(u32, f64)]) {
    let (_, m0) = means[0];
    let (_, m15) = means[means.len() - 1];
    assert!(
        m15 < m0,
        "E = 15's mean error {m15} is not below E = 0's {m0}"
    );
}

negative_control!(
    e15_is_closer_to_the_reference_than_e0,
    "E = 0's and E = 15's means swapped",
    expected = "is not below E = 0's",
    check_closer(&[(0, 0.01), (15, 0.02)])
);
