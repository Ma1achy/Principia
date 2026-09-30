//! QA tests for TASK-M0-06, third round: reading a reference image. The case format holds a reference image
//! (TASK-M0-06 Deliverables), compared with the render to REQ-VAL-138's tolerance in 8-bit steps; the runner's reader
//! states its domain as "an 8-bit RGB or RGBA PNG; alpha is dropped". The merge moved the reader from png 0.17 to
//! 0.18, so these pin the reader's contract from outside: an RGBA reference reads as its RGB with alpha dropped (not
//! premultiplied), a reference in any other colour type or depth is refused naming the file rather than misread as
//! 8-bit RGB, a damaged file is refused rather than panicking, and an image written by the runner reads back as
//! itself at sizes whose rows are not a multiple of four bytes. Known values are hand-filled. Each test has a
//! registered negative control (R-176).

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::golden::Image;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_m006_png_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `data` as a PNG of the given colour type and depth, with png itself (not the runner's writer).
fn write_raw(
    path: &Path,
    w: u32,
    h: u32,
    colour: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(BufWriter::new(file), w, h);
    encoder.set_color(colour);
    encoder.set_depth(depth);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(data)
        .unwrap();
}

// --- An RGBA reference reads as its RGB, alpha dropped ------------------------------------------------------------

/// A 3x2 RGBA image whose pixels carry alphas 0, 1, 128, 254, 255 and 7.
const RGBA: [u8; 24] = [
    10, 20, 30, 0, //
    40, 50, 60, 1, //
    70, 80, 90, 128, //
    100, 110, 120, 254, //
    130, 140, 150, 255, //
    255, 0, 200, 7, //
];

/// The RGB the reader must give for `RGBA`: each pixel's first three bytes.
fn rgb_dropping_alpha() -> Vec<u8> {
    RGBA.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect()
}

/// The RGB a reader that premultiplied by alpha would give: a wrong reading.
#[cfg(feature = "controls")]
fn rgb_premultiplied() -> Vec<u8> {
    RGBA.chunks(4)
        .flat_map(|p| {
            let m = |c: u8| ((u32::from(c) * u32::from(p[3]) + 127) / 255) as u8;
            [m(p[0]), m(p[1]), m(p[2])]
        })
        .collect()
}

fn check_rgba_reads_as(name: &str, expected: Vec<u8>) {
    let path = scratch(name).join("rgba.png");
    write_raw(
        &path,
        3,
        2,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &RGBA,
    );
    let image = Image::read_png(&path)
        .unwrap_or_else(|e| panic!("an 8-bit RGBA reference was refused: {e}"));
    assert!(
        (image.width, image.height) == (3, 2) && image.rgb == expected,
        "the RGBA reference did not read as its RGB with alpha dropped: {image:?}"
    );
}

#[test]
fn qa_m006_png_rgba_reference_drops_alpha() {
    check_rgba_reads_as("rgba", rgb_dropping_alpha());
}

negative_control!(
    qa_m006_png_rgba_reference_drops_alpha,
    "the RGB a premultiplying reader would give, taken as the expected reading",
    expected = "did not read as its RGB with alpha dropped",
    check_rgba_reads_as("ctl_rgba", rgb_premultiplied())
);

// --- A reference that is not 8-bit RGB or RGBA is refused, naming the file -----------------------------------------

/// Each (colour type, depth, bytes for a 2x1 image) given is written and must be refused, naming the file and the
/// 8-bit RGB/RGBA domain; none may be read as some other image.
fn check_formats_refused(name: &str, formats: &[(png::ColorType, png::BitDepth, Vec<u8>)]) {
    let dir = scratch(name);
    for (i, (colour, depth, data)) in formats.iter().enumerate() {
        let path = dir.join(format!("f{i}.png"));
        write_raw(&path, 2, 1, *colour, *depth, data);
        match Image::read_png(&path) {
            Ok(image) => panic!("a {colour:?} {depth:?} reference was read as an image: {image:?}"),
            Err(e) => assert!(
                e.contains(&path.display().to_string()) && e.contains("8-bit RGB or RGBA"),
                "a {colour:?} {depth:?} reference was refused without naming the file and the domain: {e}"
            ),
        }
    }
}

#[test]
fn qa_m006_png_other_formats_refused() {
    use png::{BitDepth::*, ColorType::*};
    check_formats_refused(
        "formats",
        &[
            (Rgb, Sixteen, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            (Rgba, Sixteen, vec![9; 16]),
            (Grayscale, Eight, vec![17, 200]),
            (Grayscale, Sixteen, vec![1, 2, 3, 4]),
            (GrayscaleAlpha, Eight, vec![17, 255, 200, 255]),
        ],
    );
}

negative_control!(
    qa_m006_png_other_formats_refused,
    "an 8-bit RGB reference given to the refusal check",
    expected = "reference was read as an image",
    check_formats_refused(
        "ctl_formats",
        &[(
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            vec![1, 2, 3, 4, 5, 6]
        )]
    )
);

// --- A damaged reference is refused, not a panic ------------------------------------------------------------------

/// A valid 7x5 RGB reference, cut to `keep` of its bytes (all of them when `keep` is `None`), must be refused naming
/// the file.
fn check_damaged_refused(name: &str, keep: Option<usize>) {
    let path = scratch(name).join("damaged.png");
    let data: Vec<u8> = (0..7 * 5 * 3).map(|i| (i * 37 % 256) as u8).collect();
    write_raw(
        &path,
        7,
        5,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &data,
    );
    if let Some(keep) = keep {
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, &bytes[..keep.min(bytes.len())]).unwrap();
    }
    match Image::read_png(&path) {
        Ok(_) => panic!("a damaged reference was read as an image"),
        Err(e) => assert!(
            e.contains(&path.display().to_string()),
            "a damaged reference was refused without naming the file: {e}"
        ),
    }
}

#[test]
fn qa_m006_png_damaged_reference_refused() {
    // Empty; the signature only; the header but no image data; the image data cut short.
    for (i, keep) in [0, 8, 33, 50].into_iter().enumerate() {
        check_damaged_refused(&format!("damaged_{i}"), Some(keep));
    }
    // Not a PNG at all.
    let path = scratch("not_png").join("reference.png");
    fs::write(&path, b"P6\n2 1\n255\n\x01\x02\x03\x04\x05\x06").unwrap();
    assert!(
        Image::read_png(&path).is_err_and(|e| e.contains(&path.display().to_string())),
        "a file that is not a PNG was not refused naming the file"
    );
}

negative_control!(
    qa_m006_png_damaged_reference_refused,
    "the undamaged file given to the refusal check",
    expected = "a damaged reference was read as an image",
    check_damaged_refused("ctl_damaged", None)
);

// --- The runner's writer and reader round-trip ---------------------------------------------------------------------

/// A w x h image whose every byte differs from its neighbours'.
fn pattern(w: u32, h: u32) -> Image {
    Image {
        width: w,
        height: h,
        rgb: (0..w * h * 3).map(|i| (i * 53 % 256) as u8).collect(),
    }
}

/// Writes each image with the runner's writer and reads it back with its reader; the read must be `expected(image)`.
fn check_round_trip(name: &str, expected: impl Fn(&Image) -> Image) {
    let dir = scratch(name);
    for (w, h) in [(1, 1), (3, 2), (7, 5), (1, 9), (257, 1)] {
        let image = pattern(w, h);
        let path = dir.join(format!("{w}x{h}.png"));
        image.write_png(&path).unwrap();
        let read = Image::read_png(&path).unwrap();
        assert!(
            read == expected(&image),
            "{w}x{h}: the image did not read back as written"
        );
    }
}

#[test]
fn qa_m006_png_round_trip() {
    check_round_trip("round_trip", Image::clone);
}

negative_control!(
    qa_m006_png_round_trip,
    "the expected reading with its first byte changed by one step",
    expected = "the image did not read back as written",
    check_round_trip("ctl_round_trip", |i| {
        let mut j = i.clone();
        j.rgb[0] = j.rgb[0].wrapping_add(1);
        j
    })
);
