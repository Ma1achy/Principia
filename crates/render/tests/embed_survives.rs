//! The image embedding's three searches on read and §7's "Survives" list (`principia_dd_image_embedding.md` §4, §7):
//! REQ-TOOL-063 (`embed_searches_*`) and REQ-TOOL-068 (`embed_survives_*`). Each test registers its negative control
//! (R-176).
//!
//! The transforms are written here as a pipeline applies them, forward from the image it is given, independently of
//! the reader's search, which undoes them. Rotations are counter-clockwise, as the reader's `Dihedral` names them.

use proptest::prelude::*;
use render::embed::reader::{read, Count, Dihedral, How, Outcome, Transform};
use render::embed::record::MAGIC;
use render::embed::writer::embed;
use render::embed::{Image, Plane};
use validation::{negative_control, prop};

/// §7's measured config-only payload, deflated.
const CONFIG_PAYLOAD: usize = 382;

/// §7's config payload with the full shader, 382 + 2,969 B.
const SHADER_PAYLOAD: usize = 382 + 2969;

/// §7's survives-list resolution.
const SIZE: u32 = 512;

/// A pseudo-random byte stream (xorshift), `seed` picking the stream.
fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 11) as u8
        })
        .collect()
}

/// An opaque `width` × `height` RGBA image whose R, G and B are pseudo-random, as a rendered slice's low bits are not
/// regular.
fn canvas(width: u32, height: u32, seed: u32) -> Image {
    let mut pixels = noise((width * height * 4) as usize, seed);
    for alpha in pixels.iter_mut().skip(3).step_by(4) {
        *alpha = 255;
    }
    Image::new(width, height, true, pixels)
}

/// A `width` × `height` canvas with a `len`-byte payload embedded, and the payload.
fn embedded(width: u32, height: u32, len: usize, seed: u32) -> (Image, Vec<u8>) {
    let payload = noise(len, seed ^ 0x5EED);
    let mut image = canvas(width, height, seed);
    embed(&mut image, &payload, false).expect("the payload embeds");
    (image, payload)
}

// --- The transforms, as a pipeline applies them ---------------------------------------------------------------------

/// The `width` × `height` image whose pixel `(a, b)` is `image`'s pixel `from(a, b)`.
fn remap(image: &Image, width: u32, height: u32, from: impl Fn(u32, u32) -> (u32, u32)) -> Image {
    let channels = if image.has_alpha() { 4 } else { 3 };
    let mut pixels = Vec::with_capacity((width * height * channels) as usize);
    for b in 0..height {
        for a in 0..width {
            let (x, y) = from(a, b);
            let at = ((y * image.width() + x) * channels) as usize;
            pixels.extend_from_slice(&image.pixels()[at..at + channels as usize]);
        }
    }
    Image::new(width, height, image.has_alpha(), pixels)
}

/// The `width` × `height` region of `image` whose top-left pixel is `(x, y)`.
fn crop(image: &Image, x: u32, y: u32, width: u32, height: u32) -> Image {
    remap(image, width, height, |a, b| (x + a, y + b))
}

/// `image` under `transform`, as a pipeline's rotate, flip or transpose leaves it.
fn dihedral(image: &Image, transform: Dihedral) -> Image {
    let (w, h) = (image.width(), image.height());
    match transform {
        Dihedral::Identity => image.clone(),
        // Counter-clockwise: the top-right pixel becomes the top-left.
        Dihedral::Rot90 => remap(image, h, w, |a, b| (w - 1 - b, a)),
        Dihedral::Rot180 => remap(image, w, h, |a, b| (w - 1 - a, h - 1 - b)),
        // Counter-clockwise: the bottom-left pixel becomes the top-left.
        Dihedral::Rot270 => remap(image, h, w, |a, b| (b, h - 1 - a)),
        Dihedral::FlipH => remap(image, w, h, |a, b| (w - 1 - a, b)),
        Dihedral::FlipV => remap(image, w, h, |a, b| (a, h - 1 - b)),
        Dihedral::Transpose => remap(image, h, w, |a, b| (b, a)),
        // The bottom-right pixel becomes the top-left.
        Dihedral::AntiTranspose => remap(image, h, w, |a, b| (w - 1 - b, h - 1 - a)),
    }
}

/// `image` upscaled by `s`, nearest neighbour: each pixel an `s` × `s` block.
fn upscale(image: &Image, s: u32) -> Image {
    remap(image, image.width() * s, image.height() * s, |a, b| {
        (a / s, b / s)
    })
}

/// `image` saved as a PNG at the strongest compression with adaptive filtering (§7: `optimize=True`,
/// `compress_level=9`), and loaded back.
fn png_round_trip(image: &Image) -> Image {
    let mut file = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut file, image.width(), image.height());
        encoder.set_color(if image.has_alpha() {
            png::ColorType::Rgba
        } else {
            png::ColorType::Rgb
        });
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        encoder.set_filter(png::Filter::Adaptive);
        let mut writer = encoder.write_header().expect("the PNG header writes");
        writer
            .write_image_data(image.pixels())
            .expect("the PNG data writes");
    }
    let mut reader = png::Decoder::new(std::io::Cursor::new(file))
        .read_info()
        .expect("the PNG reads");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a buffer size")];
    let info = reader.next_frame(&mut pixels).expect("the PNG decodes");
    pixels.truncate(info.buffer_size());
    Image::new(
        info.width,
        info.height,
        info.color_type == png::ColorType::Rgba,
        pixels,
    )
}

/// `image` with its top `rows` rows overwritten by other opaque pixels, as a paste over them leaves it.
fn overwrite_top(image: &Image, rows: u32, seed: u32) -> Image {
    let mut out = image.clone();
    let other = canvas(image.width(), rows, seed);
    out.pixels_mut()[..other.pixels().len()].copy_from_slice(other.pixels());
    out
}

/// Sets every low bit of `plane` to 0.
fn clear_plane(image: &mut Image, plane: Plane) {
    let alpha = image.has_alpha();
    for (i, byte) in image.pixels_mut().iter_mut().enumerate() {
        let is_alpha = alpha && i % 4 == 3;
        if is_alpha == (plane == Plane::Alpha) {
            *byte &= !1;
        }
    }
}

// --- The checks -----------------------------------------------------------------------------------------------------

/// `outcome` recovered `payload`, and its report is `how`.
fn check_recovered(outcome: &Outcome, payload: &[u8], how: How) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("the payload was not recovered: {outcome:?}");
    };
    assert_eq!(
        recovered.record.payload, payload,
        "the payload was not recovered"
    );
    assert_eq!(
        recovered.how, how,
        "the report does not say how it was recovered"
    );
}

/// A report: `dihedral`, decimation `decimate`, grid origin `(d, d)`, `tiles` of `of` RGB records, and `alpha`
/// records of as many alpha tiles (or `None` when the image has no alpha channel), nothing outvoted.
fn how(dihedral: Dihedral, decimate: u32, d: u32, tiles: (u32, u32), alpha: Option<u32>) -> How {
    how_of(dihedral, decimate, d, tiles, alpha.map(|n| (n, n)))
}

/// [`how`], with `alpha` the alpha records found and the alpha tiles expected.
fn how_of(
    dihedral: Dihedral,
    decimate: u32,
    d: u32,
    tiles: (u32, u32),
    alpha: Option<(u32, u32)>,
) -> How {
    How {
        transform: Transform {
            dx: d,
            dy: d,
            dihedral,
            decimate,
        },
        tiles: Count {
            found: tiles.0,
            of: tiles.1,
        },
        alpha: alpha.map(|(found, of)| Count { found, of }),
        outvoted: 0,
    }
}

// --- The three searches (REQ-TOOL-063) ------------------------------------------------------------------------------

/// A 128² image cropped at a 37 px offset to 91²: the RGB grid (side 33) starts at `33 − 37 mod 33 = 29` and holds
/// one tile row and column, `⌊(91 − 29) / 33⌋ = 1`; the alpha grid (side 57) starts at `57 − 37 = 20` and holds one,
/// `⌊(91 − 20) / 57⌋ = 1`.
#[test]
fn embed_searches_crop_37px_is_recovered_and_reported() {
    let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 1);
    let cropped = crop(&image, 37, 37, 91, 91);
    check_recovered(
        &read(&cropped),
        &payload,
        how(Dihedral::Identity, 1, 29, (1, 9), Some(1)),
    );
}

negative_control!(
    embed_searches_crop_37px_is_recovered_and_reported,
    "a grid origin of 0 is the uncropped image's, not the crop's",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 1);
        let cropped = crop(&image, 37, 37, 91, 91);
        check_recovered(
            &read(&cropped),
            &payload,
            how(Dihedral::Identity, 1, 0, (1, 9), Some(1)),
        )
    }
);

#[test]
fn embed_searches_rot270_is_recovered_and_reported() {
    let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 2);
    check_recovered(
        &read(&dihedral(&image, Dihedral::Rot270)),
        &payload,
        how(Dihedral::Rot270, 1, 0, (9, 9), Some(4)),
    );
}

negative_control!(
    embed_searches_rot270_is_recovered_and_reported,
    "rot270 is not rot90: a report of the other rotation must fail",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 2);
        check_recovered(
            &read(&dihedral(&image, Dihedral::Rot270)),
            &payload,
            how(Dihedral::Rot90, 1, 0, (9, 9), Some(4)),
        )
    }
);

#[test]
fn embed_searches_upscale_2_is_recovered_and_reported() {
    let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 3);
    check_recovered(
        &read(&upscale(&image, 2)),
        &payload,
        how(Dihedral::Identity, 2, 0, (9, 9), Some(4)),
    );
}

negative_control!(
    embed_searches_upscale_2_is_recovered_and_reported,
    "an image upscaled by 3 reports decimation 3, not 2",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 3);
        check_recovered(
            &read(&upscale(&image, 3)),
            &payload,
            how(Dihedral::Identity, 2, 0, (9, 9), Some(4)),
        )
    }
);

/// §4's own example: `{transform: rot270, decimate: 2, tiles: 9/9}`, a 128² image rotated 270° and upscaled ×2.
#[test]
fn embed_searches_report_is_sections_example() {
    let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 4);
    let transformed = upscale(&dihedral(&image, Dihedral::Rot270), 2);
    check_recovered(
        &read(&transformed),
        &payload,
        how(Dihedral::Rot270, 2, 0, (9, 9), Some(4)),
    );
}

negative_control!(
    embed_searches_report_is_sections_example,
    "the transformed image read as if untransformed is not §4's report",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(128, 128, CONFIG_PAYLOAD, 4);
        let transformed = upscale(&dihedral(&image, Dihedral::Rot270), 2);
        check_recovered(
            &read(&transformed),
            &payload,
            how(Dihedral::Identity, 1, 0, (9, 9), Some(4)),
        )
    }
);

/// A non-square image, so a transform that swaps width and height is told from one that does not.
#[test]
fn embed_searches_every_dihedral_transform_is_named() {
    let (image, payload) = embedded(132, 99, CONFIG_PAYLOAD, 5);
    // 132 × 99 holds 4 × 3 RGB tiles of side 33 and 2 × 1 alpha tiles of side 57.
    for transform in Dihedral::ALL {
        check_recovered(
            &read(&dihedral(&image, transform)),
            &payload,
            how(transform, 1, 0, (12, 12), Some(2)),
        );
    }
}

negative_control!(
    embed_searches_every_dihedral_transform_is_named,
    "the anti-transpose is not the transpose",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(132, 99, CONFIG_PAYLOAD, 5);
        check_recovered(
            &read(&dihedral(&image, Dihedral::AntiTranspose)),
            &payload,
            how(Dihedral::Transpose, 1, 0, (12, 12), Some(2)),
        )
    }
);

/// An upscaled image cropped by less than the scale is read whatever the decimation's phase: a crop of 1 px from an
/// ×3 upscale leaves blocks of 2 at the edge, and every third pixel still takes one of each block.
#[test]
fn embed_searches_decimation_needs_no_phase() {
    let (image, payload) = embedded(99, 66, CONFIG_PAYLOAD, 6);
    let up = upscale(&image, 3);
    let shifted = crop(&up, 1, 2, up.width() - 1, up.height() - 2);
    // 99 × 66 holds 3 × 2 RGB tiles and 1 × 1 alpha tile; the crop takes no whole source pixel's block away.
    check_recovered(
        &read(&shifted),
        &payload,
        how(Dihedral::Identity, 3, 0, (6, 6), Some(1)),
    );
}

negative_control!(
    embed_searches_decimation_needs_no_phase,
    "an ×3 upscale cropped by a whole block loses the grid's first column, so its origin moves",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(99, 66, CONFIG_PAYLOAD, 6);
        let up = upscale(&image, 3);
        let shifted = crop(&up, 3, 2, up.width() - 3, up.height() - 2);
        check_recovered(
            &read(&shifted),
            &payload,
            how(Dihedral::Identity, 3, 0, (6, 6), Some(1)),
        )
    }
);

/// The smallest record, an empty payload, has RGB side 8 and alpha side 13 (§2's tile rule), so its header wraps at
/// every side up to its own. An 8 × 13 image holds one tile of each plane, flush with its edges; upscaled ×2 and
/// rotated, its decimated view is again exactly the smallest side wide.
#[test]
fn embed_searches_smallest_tiles_flush_with_the_edges() {
    let (image, payload) = embedded(13, 13, 0, 7);
    check_recovered(
        &read(&image),
        &payload,
        how(Dihedral::Identity, 1, 0, (1, 1), Some(1)),
    );
    let (narrow, payload) = embedded(8, 13, 0, 8);
    let transformed = upscale(&dihedral(&narrow, Dihedral::Rot90), 2);
    check_recovered(
        &read(&transformed),
        &payload,
        how(Dihedral::Rot90, 2, 0, (1, 1), Some(0)),
    );
}

negative_control!(
    embed_searches_smallest_tiles_flush_with_the_edges,
    "an 8 × 13 image holds no alpha tile of side 13 × 13, so its alpha count is 0 of 0",
    expected = "the report does not say how it was recovered",
    {
        let (narrow, payload) = embedded(8, 13, 0, 8);
        check_recovered(
            &read(&narrow),
            &payload,
            how(Dihedral::Identity, 1, 0, (1, 1), Some(1)),
        )
    }
);

/// §7's config + full shader record has RGB side 95 and alpha side 165, wider than the header at both (43 and 128
/// pixels), so the header is read once and the side its `payload_len` gives is tried. A 95 × 165 crop of a 512²
/// image at the grids' origin holds one tile of each, flush with its edges; rotated, it is read through the search.
#[test]
fn embed_searches_wide_tiles_read_the_header_once() {
    let (image, payload) = embedded(SIZE, SIZE, SHADER_PAYLOAD, 9);
    let tile = crop(&image, 0, 0, 165, 165);
    check_recovered(
        &read(&dihedral(&tile, Dihedral::Rot180)),
        &payload,
        how(Dihedral::Rot180, 1, 0, (1, 25), Some(1)),
    );
}

negative_control!(
    embed_searches_wide_tiles_read_the_header_once,
    "a 164² crop holds no whole alpha tile of side 165",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(SIZE, SIZE, SHADER_PAYLOAD, 9);
        let tile = crop(&image, 0, 0, 164, 164);
        check_recovered(
            &read(&dihedral(&tile, Dihedral::Rot180)),
            &payload,
            how(Dihedral::Rot180, 1, 0, (1, 25), Some(1)),
        )
    }
);

/// A 660 B payload gives a 680 B record and RGB side 43, the first side the 128-bit header does not wrap at
/// (`⌈128 / 3⌉ = 43`), so the header is read once there and the record must be found once, not twice. A 43² image
/// holds that one tile, flush with its edges, and no alpha tile (the alpha side is 74).
#[test]
fn embed_searches_first_unwrapped_side_finds_the_record_once() {
    let (image, payload) = embedded(43, 43, 660, 16);
    check_recovered(
        &read(&image),
        &payload,
        how_of(Dihedral::Identity, 1, 0, (1, 1), Some((0, 0))),
    );
}

negative_control!(
    embed_searches_first_unwrapped_side_finds_the_record_once,
    "the record counted twice, 2 of 1, is not the report",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(43, 43, 660, 16);
        check_recovered(
            &read(&image),
            &payload,
            how_of(Dihedral::Identity, 1, 0, (2, 1), Some((0, 0))),
        )
    }
);

/// With the RGB plane cleared, the offset reported is the alpha grid's: a 37 px crop of a 512² image moves the alpha
/// grid (side 57) to `57 − 37 = 20`, and the cropped 475² view holds `⌊(475 − 20) / 57⌋² = 49` of its tiles.
#[test]
fn embed_searches_alpha_alone_reports_its_own_grid() {
    let (mut image, payload) = embedded(SIZE, SIZE, CONFIG_PAYLOAD, 10);
    clear_plane(&mut image, Plane::Rgb);
    check_recovered(
        &read(&crop(&image, 37, 37, 475, 475)),
        &payload,
        how(Dihedral::Identity, 1, 20, (0, 225), Some(49)),
    );
}

negative_control!(
    embed_searches_alpha_alone_reports_its_own_grid,
    "with the RGB plane kept, the offset reported is the RGB grid's, 29",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(SIZE, SIZE, CONFIG_PAYLOAD, 10);
        check_recovered(
            &read(&crop(&image, 37, 37, 475, 475)),
            &payload,
            how(Dihedral::Identity, 1, 20, (0, 225), Some(49)),
        )
    }
);

/// A 64² noise patch whose RGB plane spells the magic from its top-left pixel along its top row, as a tile of side 43
/// or more starts with it (`⌈32 / 3⌉ = 11` pixels), followed by noise, so its header's CRC fails.
fn magic_patch() -> Image {
    let mut patch = canvas(64, 64, 17);
    for (i, bit) in MAGIC
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |shift| (byte >> shift) & 1))
        .enumerate()
    {
        let at = (i / 3) * 4 + i % 3;
        patch.pixels_mut()[at] = (patch.pixels()[at] & !1) | bit;
    }
    patch
}

/// `image` with `patch` pasted over it, its top-left pixel at `(x, y)`.
fn paste(mut image: Image, patch: &Image, x: u32, y: u32) -> Image {
    let (width, row) = (image.width(), (patch.width() * 4) as usize);
    for b in 0..patch.height() {
        let from = (b * patch.width() * 4) as usize;
        let to = (((y + b) * width + x) * 4) as usize;
        image.pixels_mut()[to..to + row].copy_from_slice(&patch.pixels()[from..from + row]);
    }
    image
}

/// A 512² noise canvas holding the magic, but no intact header, where only the search finds it: off the writer's
/// grid at `(5, 7)`, rotated 90° at `(101, 203)` and upscaled ×2 at `(300, 300)`. The magic alone there is not state
/// present (§9's three outcomes), so the canvas reads as no state: the search's many views and pixels must not raise
/// the rate of false "state present" above the writer's grid's own.
fn magic_off_the_writer_grid() -> Image {
    let patch = magic_patch();
    let image = paste(canvas(SIZE, SIZE, 18), &patch, 5, 7);
    let image = paste(image, &dihedral(&patch, Dihedral::Rot90), 101, 203);
    paste(image, &upscale(&patch, 2), 300, 300)
}

/// `outcome` is "no embedded state".
fn check_no_state(outcome: &Outcome) {
    assert_eq!(outcome, &Outcome::None, "the canvas reads as state present");
}

#[test]
fn embed_searches_magic_alone_off_the_writer_grid_is_no_state() {
    check_no_state(&read(&magic_off_the_writer_grid()));
}

negative_control!(
    embed_searches_magic_alone_off_the_writer_grid_is_no_state,
    "the magic at the start of a tile of the writer's grid, (0, 0), is state present, corrupt",
    expected = "the canvas reads as state present",
    {
        check_no_state(&read(&paste(
            magic_off_the_writer_grid(),
            &magic_patch(),
            0,
            0,
        )))
    }
);

// --- §7's "Survives" list at 512² (REQ-TOOL-068) --------------------------------------------------------------------

/// One entry of §7's "Survives" list under the tiled variant (the arbitrary rotations need the hybrid variant).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Survive {
    /// PNG save/load, `optimize=True`, `compress_level=9`.
    Png,
    /// Alpha stripped entirely.
    AlphaStripped,
    /// Crop to 50 %: the central 256².
    CropHalf,
    /// Crop at an arbitrary 37 px offset: the 475² from `(37, 37)`.
    Crop37,
    /// The 37 px crop, then a PNG round trip.
    CropPng,
    /// Rotate 90, 180 or 270, flip h or v, transpose.
    Dihedral(Dihedral),
    /// Nearest upscale ×2 or ×3.
    Upscale(u32),
    /// Rot90 and upscale ×2 together.
    Rot90Upscale2,
    /// The top 50 % overwritten.
    OverwriteTop,
}

/// §7's list, every entry once.
const SURVIVES: [Survive; 15] = [
    Survive::Png,
    Survive::AlphaStripped,
    Survive::CropHalf,
    Survive::Crop37,
    Survive::CropPng,
    Survive::Dihedral(Dihedral::Rot90),
    Survive::Dihedral(Dihedral::Rot180),
    Survive::Dihedral(Dihedral::Rot270),
    Survive::Dihedral(Dihedral::FlipH),
    Survive::Dihedral(Dihedral::FlipV),
    Survive::Dihedral(Dihedral::Transpose),
    Survive::Upscale(2),
    Survive::Upscale(3),
    Survive::Rot90Upscale2,
    Survive::OverwriteTop,
];

impl Survive {
    /// `image` as the entry leaves it; `seed` picks what overwrites the top half.
    fn apply(self, image: &Image, seed: u32) -> Image {
        match self {
            Self::Png => png_round_trip(image),
            Self::AlphaStripped => image.without_alpha(),
            Self::CropHalf => crop(image, SIZE / 4, SIZE / 4, SIZE / 2, SIZE / 2),
            Self::Crop37 => crop(image, 37, 37, SIZE - 37, SIZE - 37),
            Self::CropPng => png_round_trip(&Self::Crop37.apply(image, seed)),
            Self::Dihedral(transform) => dihedral(image, transform),
            Self::Upscale(s) => upscale(image, s),
            Self::Rot90Upscale2 => upscale(&dihedral(image, Dihedral::Rot90), 2),
            Self::OverwriteTop => overwrite_top(image, SIZE / 2, seed),
        }
    }

    /// The report a 512² image with §7's config payload gives after the entry. The RGB grid has side 33 and 15 × 15
    /// tiles (§7's 225), the alpha grid side 57 and 8 × 8.
    fn report(self) -> How {
        let all = (225, 225);
        match self {
            // §7: 225/225 tiles.
            Self::Png => how(Dihedral::Identity, 1, 0, all, Some(64)),
            // §7: 225/225, RGB is systematic.
            Self::AlphaStripped => how(Dihedral::Identity, 1, 0, all, None),
            // From 128: the RGB grid at 33 − 128 mod 33 = 4, ⌊(256 − 4) / 33⌋ = 7 a side; the alpha grid at
            // 57 − 128 mod 57 = 43, ⌊(256 − 43) / 57⌋ = 3 a side. The origin reported is the RGB grid's.
            Self::CropHalf => how(Dihedral::Identity, 1, 4, (49, 225), Some(9)),
            // From 37: the RGB grid at 29, ⌊(475 − 29) / 33⌋ = 13 a side; the alpha grid at 20, ⌊455 / 57⌋ = 7.
            Self::Crop37 | Self::CropPng => how(Dihedral::Identity, 1, 29, (169, 225), Some(49)),
            Self::Dihedral(transform) => how(transform, 1, 0, all, Some(64)),
            Self::Upscale(s) => how(Dihedral::Identity, s, 0, all, Some(64)),
            Self::Rot90Upscale2 => how(Dihedral::Rot90, 2, 0, all, Some(64)),
            // The top 256 rows lost: the RGB tile rows from 264 = 8 × 33 survive, 7 of 15; the alpha rows from
            // 285 = 5 × 57, 3 of 8.
            Self::OverwriteTop => how_of(Dihedral::Identity, 1, 0, (105, 225), Some((24, 64))),
        }
    }
}

/// `before` and `after` differ by at most 1 in every channel (§7: "max pixel delta 1 throughout").
fn check_max_delta_1(before: &Image, after: &Image) {
    let delta = before
        .pixels()
        .iter()
        .zip(after.pixels())
        .map(|(b, a)| b.abs_diff(*a))
        .max()
        .unwrap_or(0);
    assert!(
        delta <= 1,
        "the embedding moved a channel by {delta}, not 1"
    );
}

/// Embeds §7's config payload in a 512² canvas picked by `seed`, checks the embedding's delta, applies `entry` and
/// checks the payload is recovered with the entry's report.
fn check_survives(entry: Survive, seed: u32) {
    let before = canvas(SIZE, SIZE, seed);
    let payload = noise(CONFIG_PAYLOAD, seed ^ 0x5EED);
    let mut after = before.clone();
    embed(&mut after, &payload, false).expect("the payload embeds");
    check_max_delta_1(&before, &after);
    check_recovered(
        &read(&entry.apply(&after, seed.wrapping_add(1))),
        &payload,
        entry.report(),
    );
}

#[test]
fn embed_survives_the_list() {
    prop::run(
        &(proptest::sample::select(SURVIVES.to_vec()), any::<u32>()),
        |(entry, seed)| {
            check_survives(entry, seed);
            Ok(())
        },
    );
}

negative_control!(
    embed_survives_the_list,
    "averaging is not on the list (§7: resampling averages, so the low bit is destroyed): each channel averaged with the next pixel's",
    expected = "the payload was not recovered",
    {
        let (image, payload) = embedded(SIZE, SIZE, CONFIG_PAYLOAD, 11);
        let mut averaged = image.clone();
        let source = image.pixels();
        for (i, out) in averaged.pixels_mut().iter_mut().enumerate() {
            let right = source[(i + 4) % source.len()];
            *out = ((u16::from(source[i]) + u16::from(right)) / 2) as u8;
        }
        check_recovered(&read(&averaged), &payload, Survive::Png.report())
    }
);

#[test]
fn embed_survives_every_entry() {
    for (seed, entry) in (100..).zip(SURVIVES) {
        check_survives(entry, seed);
    }
}

negative_control!(
    embed_survives_every_entry,
    "an entry's report checked against another entry's must fail",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(SIZE, SIZE, CONFIG_PAYLOAD, 12);
        check_recovered(
            &read(&Survive::Crop37.apply(&image, 13)),
            &payload,
            Survive::CropHalf.report(),
        )
    }
);

/// The embedding moves no channel by more than 1, on a canvas whose channels sit at both ends of their range.
#[test]
fn embed_survives_max_pixel_delta_is_1() {
    let mut before = canvas(SIZE, SIZE, 14);
    for (i, byte) in before.pixels_mut().iter_mut().enumerate() {
        if i % 4 != 3 {
            *byte = if i % 3 == 0 { 0 } else { 255 };
        }
    }
    let mut after = before.clone();
    embed(&mut after, &noise(CONFIG_PAYLOAD, 14), false).expect("the payload embeds");
    check_max_delta_1(&before, &after);
}

negative_control!(
    embed_survives_max_pixel_delta_is_1,
    "a channel moved by 2 must fail the delta check",
    expected = "not 1",
    {
        let before = canvas(SIZE, SIZE, 14);
        let mut after = before.clone();
        after.pixels_mut()[9] = after.pixels()[9].wrapping_add(2);
        check_max_delta_1(&before, &after)
    }
);
