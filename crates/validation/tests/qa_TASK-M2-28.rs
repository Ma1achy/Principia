//! QA tests for TASK-M2-28's goldens, written from REQ-CHART-002, REQ-RENDER-026 and REQ-CHART-075 (R-544), not from
//! the occupant's code.
//!
//! The render colours each system by its octant, the signs of its shape `n` (`fixtures/golden/BASELINES.md`'s rows),
//! so the whole sphere, pole to pole, both hemispheres told apart and neither flagged, means: every pixel's colour is
//! a function of the octant of the chart point it shows, that octant computed here from R-14's map (or chart_reference
//! §3.3's exponential map, or its Hammer inverse) at the chart cell's centre; distinct octants are distinct colours;
//! both hemispheres' octants are drawn; and a display point showing no chart point (outside Hammer's ellipse) is drawn
//! in no octant's colour.
//!
//! - `qa_m228_references_whole_sphere`: the checked-in references `shape-sphere/spherical`,
//!   `shape-sphere/exponential_straddling`, `shape-sphere-projection/equirectangular` and
//!   `shape-sphere-projection/equal_area` each pass that check (REQ-CHART-002, REQ-RENDER-026, REQ-CHART-075).
//! - `qa_m228_scenes_whole_sphere`: the scenes' CPU renders pass it too. Controls: the one-hemisphere scene (one
//!   hemisphere's octants drawn across the square) and the flagged scene (the lower half striped as redundant) each
//!   fail it, for their reasons (REQ-CHART-002's verify).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use validation::negative_control;
use validation::shape_scene;

const PI: f64 = std::f64::consts::PI;

/// The octant of `n`: `[n.x < 0] + 2·[n.y < 0] + 4·[n.z < 0]`; 4 to 7 below the equator.
fn octant(n: [f64; 3]) -> u8 {
    u8::from(n[0] < 0.0) + 2 * u8::from(n[1] < 0.0) + 4 * u8::from(n[2] < 0.0)
}

/// R-14's spherical point at `(s, t)`: `θ = 2πs`, `φ = π(1 − t)`.
fn sphere_point(s: f64, t: f64) -> [f64; 3] {
    let (theta, phi) = (2.0 * PI * s, PI * (1.0 - t));
    [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
}

/// chart_reference §3.3's exponential map about `n₀ = x̂`, `e₁ = ŷ`, `e₂ = ŵ`, `s = 1`: the straddling golden's
/// frame, as `BASELINES.md` names it.
fn straddling_point(u: f64, v: f64) -> [f64; 3] {
    let (a, b) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    let r = (a * a + b * b).sqrt();
    let k = if r == 0.0 { 1.0 } else { r.sin() / r };
    [r.cos(), k * a, k * b]
}

/// chart_reference §3.3's Hammer inverse, written out: display `(x, y)` → chart `(s, t)`, `None` outside the ellipse.
fn hammer_inverse(x: f64, y: f64) -> Option<(f64, f64)> {
    let (h, k) = (2.0 * x - 1.0, 2.0 * y - 1.0);
    let q = h * h + k * k;
    if q > 1.0 {
        return None;
    }
    let (big_x, big_y) = (2.0 * 2f64.sqrt() * h, 2f64.sqrt() * k);
    let z = (1.0 - q / 2.0).sqrt();
    let lambda = 2.0 * (z * big_x).atan2(2.0 * (1.0 - q));
    let lat = (z * big_y).clamp(-1.0, 1.0).asin();
    Some(((lambda + PI) / (2.0 * PI), lat / PI + 0.5))
}

/// The centre of the cell of `cells` holding `c ∈ [0, 1]`.
fn cell_centre(c: f64, cells: u32) -> f64 {
    let k = ((c * f64::from(cells)).floor() as u32).min(cells - 1);
    (f64::from(k) + 0.5) / f64::from(cells)
}

/// A pixel's colour as a comparable key.
type Key = [u64; 3];

/// An image: its size and each pixel's colour, rows from the top.
struct Image {
    size: (u32, u32),
    pixels: Vec<Key>,
}

/// Which golden layout an image has, and so the octant each of its pixels shows (or none).
#[derive(Clone, Copy)]
enum Layout {
    /// The spherical map, equirectangular: 128 × 64 px over a 32 × 16 chart grid.
    Spherical,
    /// The spherical map through Hammer's projection: 64 × 32 display tiles of 2 px over the same chart grid.
    EqualArea,
    /// The straddling exponential cap: 64 × 64 px over a 32 × 32 grid.
    Straddling,
}

impl Layout {
    fn size(self) -> (u32, u32) {
        match self {
            Layout::Spherical | Layout::EqualArea => (128, 64),
            Layout::Straddling => (64, 64),
        }
    }

    /// The octant pixel `(x, y)` shows, `y` from the top, or `None` where it shows no chart point.
    fn octant(self, x: u32, y: u32) -> Option<u8> {
        let (w, h) = self.size();
        let (px, py) = (
            (f64::from(x) + 0.5) / f64::from(w),
            1.0 - (f64::from(y) + 0.5) / f64::from(h),
        );
        match self {
            Layout::Spherical => Some(octant(sphere_point(
                cell_centre(px, 32),
                cell_centre(py, 16),
            ))),
            Layout::Straddling => Some(octant(straddling_point(
                cell_centre(px, 32),
                cell_centre(py, 32),
            ))),
            Layout::EqualArea => {
                // The display tile's centre: 2 px tiles, 64 × 32 of them.
                let (col, row) = (x / 2, y / 2);
                let (dx, dy) = (
                    (f64::from(col) + 0.5) / 64.0,
                    1.0 - (f64::from(row) + 0.5) / 32.0,
                );
                let (s, t) = hammer_inverse(dx, dy)?;
                Some(octant(sphere_point(cell_centre(s, 32), cell_centre(t, 16))))
            }
        }
    }
}

/// The check: each pixel's colour a function of its octant, distinct octants distinct colours, octants of both
/// hemispheres drawn, and no pixel showing no chart point in an octant's colour.
fn check_whole_sphere(name: &str, layout: Layout, image: &Image) {
    assert_eq!(image.size, layout.size(), "{name}: the image's size");
    let (w, h) = image.size;
    let mut colour_of: BTreeMap<u8, Key> = BTreeMap::new();
    let mut blank = BTreeSet::new();
    for y in 0..h {
        for x in 0..w {
            let key = image.pixels[(y * w + x) as usize];
            match layout.octant(x, y) {
                Some(k) => {
                    let first = *colour_of.entry(k).or_insert(key);
                    assert_eq!(
                        first, key,
                        "{name}: octant {k} is drawn in more than one colour (pixel ({x}, {y}))"
                    );
                }
                None => {
                    blank.insert((key, x, y));
                }
            }
        }
    }
    let colours: BTreeSet<Key> = colour_of.values().copied().collect();
    assert_eq!(
        colours.len(),
        colour_of.len(),
        "{name}: two octants share a colour, so the hemispheres or quadrants are not told apart: {colour_of:?}"
    );
    let upper = colour_of.keys().filter(|&&k| k < 4).count();
    let lower = colour_of.keys().filter(|&&k| k >= 4).count();
    assert!(
        upper > 0 && lower > 0,
        "{name}: only {upper} upper and {lower} lower octants drawn: not the whole sphere"
    );
    if matches!(layout, Layout::EqualArea) {
        assert!(
            !blank.is_empty(),
            "{name}: no display point lies outside Hammer's ellipse"
        );
    }
    for (key, x, y) in blank {
        assert!(
            !colours.contains(&key),
            "{name}: pixel ({x}, {y}) shows no chart point but is drawn in an octant's colour"
        );
    }
}

// ── The checked-in references ────────────────────────────────────────────────────────────────────────────────────

fn golden(suite: &str, case: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/golden")
        .join(suite)
        .join(case)
        .join("reference.png")
}

fn read_png(path: &Path) -> Image {
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
        pixels: buf
            .chunks(3)
            .map(|p| [u64::from(p[0]), u64::from(p[1]), u64::from(p[2])])
            .collect(),
    }
}

#[test]
fn qa_m228_references_whole_sphere() {
    for (suite, case, layout) in [
        ("shape-sphere", "spherical", Layout::Spherical),
        ("shape-sphere", "exponential_straddling", Layout::Straddling),
        (
            "shape-sphere-projection",
            "equirectangular",
            Layout::Spherical,
        ),
        ("shape-sphere-projection", "equal_area", Layout::EqualArea),
    ] {
        let image = read_png(&golden(suite, case));
        check_whole_sphere(&format!("{suite}/{case}"), layout, &image);
    }
}

/// The spherical reference with its lower half replaced by its upper half mirrored: one hemisphere drawn twice.
#[cfg(feature = "controls")]
fn folded_reference() -> Image {
    let mut image = read_png(&golden("shape-sphere", "spherical"));
    let (w, h) = image.size;
    for y in h / 2..h {
        for x in 0..w {
            image.pixels[(y * w + x) as usize] = image.pixels[((h - 1 - y) * w + x) as usize];
        }
    }
    image
}

negative_control!(
    qa_m228_references_whole_sphere,
    "a reference drawing the upper hemisphere's colours on the lower half does not tell the hemispheres apart, so it \
     must fail",
    expected = "two octants share a colour",
    {
        check_whole_sphere("folded", Layout::Spherical, &folded_reference());
    }
);

// ── The scenes' CPU renders and the controls ─────────────────────────────────────────────────────────────────────

fn cpu_render(name: &str) -> Image {
    let s = shape_scene::scene(name).unwrap_or_else(|e| panic!("{e}"));
    Image {
        size: s.scene.size(),
        pixels: s.expected().iter().map(|c| c.map(f64::to_bits)).collect(),
    }
}

#[test]
fn qa_m228_scenes_whole_sphere() {
    for (name, layout) in [
        ("shape_spherical", Layout::Spherical),
        ("shape_equal_area", Layout::EqualArea),
        ("shape_exponential_straddling", Layout::Straddling),
    ] {
        check_whole_sphere(name, layout, &cpu_render(name));
    }
}

negative_control!(
    qa_m228_scenes_whole_sphere,
    "the one-hemisphere control draws the upper hemisphere's octants across the square, so it must fail",
    expected = "two octants share a colour",
    {
        check_whole_sphere(
            "shape_control_one_hemisphere",
            Layout::Spherical,
            &cpu_render("shape_control_one_hemisphere"),
        );
    }
);

mod flagged {
    use super::*;
    negative_control!(
        qa_m228_scenes_whole_sphere,
        "the flagged control stripes the lower half, a second colour within its octants, so it must fail",
        expected = "is drawn in more than one colour",
        {
            check_whole_sphere(
                "shape_control_flagged",
                Layout::Spherical,
                &cpu_render("shape_control_flagged"),
            );
        }
    );
}
