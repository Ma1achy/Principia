//! QA tests for TASK-M2-28's equal-area golden, written from chart_reference §3.3 ("A display point outside the
//! ellipse shows no chart point: it is no pixel of the chart, drawn blank (R-470), and never counted"), REQ-RENDER-026
//! and REQ-CHART-047, not from the occupant's code.
//!
//! Blank is the canvas background the harness renders with (`validation::golden_scene::BACKGROUND`), not the debug
//! views' hatch and not any octant's colour. So:
//!
//! - `qa_m228_margin_blank_reference`: in the checked-in `shape-sphere-projection/equal_area` reference, every pixel
//!   whose display tile's centre lies outside Hammer's ellipse, `(2x − 1)² + (2y − 1)² > 1`, is exactly the canvas
//!   background, and no pixel inside it is; the other shape-sphere references, which show a chart point at every
//!   pixel, draw no blank pixel. Control: the reference with its margin redrawn in the debug views' hatch (the
//!   withdrawn behaviour) fails, as not blank.
//! - `qa_m228_margin_blank_scene`: the equal-area scene's CPU render passes the same check. Control: the scene's render
//!   with one pixel inside the ellipse drawn blank fails, as a chart pixel drawn blank.

use std::path::{Path, PathBuf};

use render::present;
use validation::golden_scene::BACKGROUND;
use validation::negative_control;
use validation::shape_scene;

/// The equal-area golden's display: 64 × 32 tiles of 2 px, 128 × 64 px.
const TILES: (u32, u32) = (64, 32);
const TILE: u32 = 2;

/// Whether pixel `(x, y)`, `y` from the top, lies in a display tile whose centre is outside Hammer's ellipse.
fn outside(x: u32, y: u32) -> bool {
    let (col, row) = (x / TILE, y / TILE);
    let cx = (f64::from(col) + 0.5) / f64::from(TILES.0);
    let cy = 1.0 - (f64::from(row) + 0.5) / f64::from(TILES.1);
    let (h, k) = (2.0 * cx - 1.0, 2.0 * cy - 1.0);
    h * h + k * k > 1.0
}

/// An image's size and pixels, rows from the top, each pixel compared by equality.
struct Image<P> {
    size: (u32, u32),
    pixels: Vec<P>,
}

/// The margin check: every margin pixel is `blank`, every pixel inside the ellipse is not, and there is a margin.
fn check_margin<P: PartialEq + std::fmt::Debug>(name: &str, image: &Image<P>, blank: &P) {
    assert_eq!(image.size, (TILES.0 * TILE, TILES.1 * TILE), "{name}: size");
    let (w, h) = image.size;
    let mut margin = 0;
    for y in 0..h {
        for x in 0..w {
            let p = &image.pixels[(y * w + x) as usize];
            if outside(x, y) {
                margin += 1;
                assert_eq!(
                    p, blank,
                    "{name}: margin pixel ({x}, {y}) is not drawn blank"
                );
            } else {
                assert_ne!(
                    p, blank,
                    "{name}: pixel ({x}, {y}) inside the ellipse, a chart pixel, is drawn blank"
                );
            }
        }
    }
    assert!(margin > 0, "{name}: no pixel lies outside Hammer's ellipse");
}

fn golden(suite: &str, case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(suite)
        .join(case)
        .join("reference.png")
}

fn read_png(path: &Path) -> Image<[u8; 3]> {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut buf = vec![0; reader.output_buffer_size().expect("a small image")];
    let info = reader.next_frame(&mut buf).expect("one frame");
    assert_eq!(
        (info.color_type, info.bit_depth),
        (png::ColorType::Rgb, png::BitDepth::Eight),
        "{}: not 8-bit RGB",
        path.display()
    );
    buf.truncate(info.buffer_size());
    Image {
        size: (info.width, info.height),
        pixels: buf.chunks(3).map(|p| [p[0], p[1], p[2]]).collect(),
    }
}

/// A linear colour as a reference PNG holds it: each linear channel times 255, rounded (`BASELINES.md`'s "every channel
/// at least … of an 8-bit step from a rounding tie").
fn quantised(c: [f64; 3]) -> [u8; 3] {
    c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Blank as a reference PNG holds it: the canvas background, decoded to linear as the harness's f32 uniform holds it,
/// quantised.
fn blank_reference() -> [u8; 3] {
    quantised(present::srgb8(BACKGROUND).map(|c| f64::from(c as f32)))
}

#[test]
fn qa_m228_margin_blank_reference() {
    let image = read_png(&golden("shape-sphere-projection", "equal_area"));
    check_margin(
        "shape-sphere-projection/equal_area",
        &image,
        &blank_reference(),
    );
    // The references with no margin: every pixel shows a chart point, so none is blank.
    for (suite, case) in [
        ("shape-sphere", "spherical"),
        ("shape-sphere", "exponential_straddling"),
        ("shape-sphere-projection", "equirectangular"),
    ] {
        let image = read_png(&golden(suite, case));
        assert!(
            image.pixels.iter().all(|p| *p != blank_reference()),
            "{suite}/{case}: a pixel showing a chart point is drawn blank"
        );
    }
}

/// The equal-area reference with its margin redrawn in the debug views' hatch, quantised as the reference is.
#[cfg(feature = "controls")]
fn hatched_reference() -> Image<[u8; 3]> {
    let mut image = read_png(&golden("shape-sphere-projection", "equal_area"));
    let w = image.size.0;
    for (i, p) in image.pixels.iter_mut().enumerate() {
        let (x, y) = (i as u32 % w, i as u32 / w);
        if outside(x, y) {
            *p = quantised(present::debug_invalid([
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
            ]));
        }
    }
    image
}

negative_control!(
    qa_m228_margin_blank_reference,
    "the margin hatched as the debug views hatch a non-system sample is not blank, so it must fail",
    expected = "is not drawn blank",
    {
        check_margin("hatched", &hatched_reference(), &blank_reference());
    }
);

/// The equal-area scene's CPU render, linear, each pixel compared bit for bit.
fn cpu_render() -> Image<[u64; 3]> {
    let s = shape_scene::scene("shape_equal_area").unwrap_or_else(|e| panic!("{e}"));
    Image {
        size: s.scene.size(),
        pixels: s.expected().iter().map(|c| c.map(f64::to_bits)).collect(),
    }
}

/// The canvas background as the CPU render's linear value, bit for bit: the 8-bit code decoded, held as the f32 the
/// harness's uniform holds.
fn blank_linear() -> [u64; 3] {
    present::srgb8(BACKGROUND).map(|c| f64::from(c as f32).to_bits())
}

#[test]
fn qa_m228_margin_blank_scene() {
    check_margin("shape_equal_area", &cpu_render(), &blank_linear());
}

negative_control!(
    qa_m228_margin_blank_scene,
    "a chart pixel inside the ellipse drawn blank is a system hidden as margin, so it must fail",
    expected = "a chart pixel, is drawn blank",
    {
        let mut image = cpu_render();
        // The display's centre pixel, inside the ellipse.
        let (w, h) = image.size;
        image.pixels[((h / 2) * w + w / 2) as usize] = blank_linear();
        check_margin("one chart pixel blank", &image, &blank_linear());
    }
);
