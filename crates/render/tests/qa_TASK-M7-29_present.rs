//! QA tests for TASK-M7-29's "state present" rule, written from REQ-TOOL-071 (§9: three distinguishable outcomes,
//! "plain PNG → none; corrupted all records → corrupt") and REQ-TOOL-063 (§4: the reader searches offset, dihedral
//! and decimation with the CRC as the oracle), not from the implementation.
//!
//! The searches try many pixels, sides and views, so they must not turn a plain image into "state present" by chance,
//! and they must still report "state present, corrupt" when every record is damaged after a transform they fix.
//!
//! Layouts by hand from §2 and §3: the magic is `8F 50 72 6E` (§2, the proposed value); a record's bits go MSB first,
//! three to an RGB pixel (R, G, B), pixels row-major from the tile's top-left. §7's config payload is 382 B, so its
//! record is 16 + 382 + 4 = 402 B = 3,216 bits; at 512² the RGB tiles have side ⌈√1072⌉ = 33 (15 × 15) and the alpha
//! tiles side ⌈√3216⌉ = 57 (8 × 8). The writer lays its grid from the image's top-left pixel, so a tile of side `s`
//! starts at multiples of `s` in both coordinates. Each test has a registered negative control (R-176).

use proptest::prelude::*;
use render::embed::reader::{read, CorruptReason, Outcome};
use render::embed::writer::embed;
use render::embed::{Image, Plane};
use validation::{negative_control, prop};

/// §2's proposed magic.
const MAGIC: [u8; 4] = [0x8F, 0x50, 0x72, 0x6E];
/// §7: the survives list's resolution.
const SIDE: u32 = 512;
/// §7: the measured config-only payload, deflated.
const CONFIG: usize = 382;
/// §2 by hand: the RGB and alpha tile sides of §7's config record, and their counts per row at 512².
const RGB: (u32, u32) = (33, 15);
const ALPHA: (u32, u32) = (57, 8);

/// SplitMix64, `seed` picking the stream.
fn bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut s = seed;
    (0..len)
        .map(|_| {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            (z ^ (z >> 31)) as u8
        })
        .collect()
}

/// An opaque RGBA canvas whose R, G and B are pseudo-random.
fn canvas(width: u32, height: u32, seed: u64) -> Image {
    let mut px = bytes((width * height * 4) as usize, seed);
    for a in px.iter_mut().skip(3).step_by(4) {
        *a = 255;
    }
    Image::new(width, height, true, px)
}

/// The `w` × `h` image whose pixel `(u, v)` is `image`'s pixel `at(u, v)`.
fn build(image: &Image, w: u32, h: u32, at: impl Fn(u32, u32) -> (u32, u32)) -> Image {
    let c = if image.has_alpha() { 4 } else { 3 };
    let src = image.pixels();
    let iw = image.width() as usize;
    let mut out = Vec::with_capacity(w as usize * h as usize * c);
    for v in 0..h {
        for u in 0..w {
            let (x, y) = at(u, v);
            let i = (y as usize * iw + x as usize) * c;
            out.extend_from_slice(&src[i..i + c]);
        }
    }
    Image::new(w, h, image.has_alpha(), out)
}

/// Rotated 90° counter-clockwise.
fn rot90(image: &Image) -> Image {
    let (w, h) = (image.width(), image.height());
    build(image, h, w, |u, v| (w - 1 - v, u))
}

/// The left `n` columns and top `n` rows cut away.
fn crop(image: &Image, n: u32) -> Image {
    build(image, image.width() - n, image.height() - n, |u, v| {
        (u + n, v + n)
    })
}

/// Nearest-neighbour upscale by `s`.
fn upscale(image: &Image, s: u32) -> Image {
    build(image, image.width() * s, image.height() * s, |u, v| {
        (u / s, v / s)
    })
}

/// The three views §4's searches fix that the writer's grid does not: none, rot90 then a 37 px crop, and rot90 ×3
/// then ×2 upscale.
fn views(image: &Image) -> [(&'static str, Image); 3] {
    let rot270 = rot90(&rot90(&rot90(image)));
    [
        ("identity", image.clone()),
        ("rot90 + 37 px crop", crop(&rot90(image), 37)),
        ("rot270 + ×2 upscale", upscale(&rot270, 2)),
    ]
}

// ---------------------------------------------------------------------------------------------------------------
// Corrupted all records, seen through a transform: still "state present, corrupt" (REQ-TOOL-071).

/// §7's config record embedded at 512², then one payload bit (record bit 200, past the 128-bit header) flipped in every
/// tile of both planes: every header intact, every record discarded.
fn all_records_corrupted() -> Image {
    let mut image = canvas(SIDE, SIDE, 31);
    embed(&mut image, &bytes(CONFIG, 32), false).expect("the payload embeds");
    for (plane, (side, n)) in [(Plane::Rgb, RGB), (Plane::Alpha, ALPHA)] {
        for row in 0..n {
            for col in 0..n {
                let mut lows = image.tile_lows(plane, col, row, side);
                lows[200] ^= 1;
                image.set_tile_lows(plane, col, row, side, &lows);
            }
        }
    }
    image
}

fn check_corrupt_in_every_view(image: &Image) {
    for (name, view) in views(image) {
        match read(&view) {
            Outcome::Corrupt(c) => {
                assert_eq!(
                    c.intact, 0,
                    "{name}: an intact record survived the corruption"
                );
                assert_eq!(c.reason, CorruptReason::NoIntactRecord, "{name}");
            }
            other => panic!("{name}: is not state present, corrupt: {other:?}"),
        }
    }
}

#[test]
fn embed_searches_qa_corrupted_records_under_a_transform_are_corrupt() {
    check_corrupt_in_every_view(&all_records_corrupted());
}

negative_control!(
    embed_searches_qa_corrupted_records_under_a_transform_are_corrupt,
    "a plain canvas, with no record, is not state present in any view",
    expected = "is not state present, corrupt",
    { check_corrupt_in_every_view(&canvas(SIDE, SIDE, 31)) }
);

// ---------------------------------------------------------------------------------------------------------------
// The magic alone: state present on the writer's grid at any tile start, not just (0, 0); no state off it.

/// A 64² noise patch whose top row spells the magic from its top-left pixel (11 RGB pixels, so it starts any tile of
/// side 11 or more), followed by noise, so no header's CRC verifies.
fn magic_patch() -> Image {
    let mut patch = canvas(64, 64, 41);
    for i in 0..32 {
        let bit = (MAGIC[i / 8] >> (7 - i % 8)) & 1;
        let at = (i / 3) * 4 + i % 3;
        let px = &mut patch.pixels_mut()[at];
        *px = (*px & !1) | bit;
    }
    patch
}

/// A 512² noise canvas with [`magic_patch`] pasted at `(x, y)`.
fn magic_at(x: u32, y: u32) -> Image {
    let patch = magic_patch();
    let mut image = canvas(SIDE, SIDE, 42);
    let row = (patch.width() * 4) as usize;
    for b in 0..patch.height() {
        let from = (b * patch.width() * 4) as usize;
        let to = (((y + b) * SIDE + x) * 4) as usize;
        image.pixels_mut()[to..to + row].copy_from_slice(&patch.pixels()[from..from + row]);
    }
    image
}

/// Tile starts of the writer's grid away from the origin: (86, 129) = 43 × (2, 3), a side at which the 128-bit header
/// fits in one row (⌈128 / 3⌉ = 43 pixels); (66, 99) = 33 × (2, 3) = 11 × (6, 9), sides at which it wraps.
const ON_GRID: [(u32, u32); 2] = [(86, 129), (66, 99)];
/// The same pixels one step off: (87, 129) has gcd 3 and (67, 99) gcd 1, both below any side a record can have
/// (the smallest record, 20 B = 160 bits, needs side ⌈√54⌉ = 8).
const OFF_GRID: [(u32, u32); 2] = [(87, 129), (67, 99)];

fn check_corrupt_at(points: &[(u32, u32)]) {
    for &(x, y) in points {
        match read(&magic_at(x, y)) {
            Outcome::Corrupt(c) => {
                assert_eq!(c.reason, CorruptReason::NoIntactRecord, "({x}, {y})");
            }
            other => panic!("the magic at ({x}, {y}) is not state present: {other:?}"),
        }
    }
}

fn check_none_at(points: &[(u32, u32)]) {
    for &(x, y) in points {
        assert_eq!(
            read(&magic_at(x, y)),
            Outcome::None,
            "the magic off the writer's grid at ({x}, {y}) reads as state present"
        );
    }
}

#[test]
fn embed_searches_qa_magic_on_the_writer_grid_is_present() {
    check_corrupt_at(&ON_GRID);
}

negative_control!(
    embed_searches_qa_magic_on_the_writer_grid_is_present,
    "the magic one pixel off the writer's grid is not state present",
    expected = "is not state present",
    { check_corrupt_at(&OFF_GRID) }
);

#[test]
fn embed_searches_qa_magic_off_the_writer_grid_is_none() {
    check_none_at(&OFF_GRID);
}

negative_control!(
    embed_searches_qa_magic_off_the_writer_grid_is_none,
    "the magic at a tile start of the writer's grid is state present",
    expected = "reads as state present",
    { check_none_at(&ON_GRID) }
);

// ---------------------------------------------------------------------------------------------------------------
// Plain images: "no embedded state" in every view the searches fix (REQ-TOOL-071, "plain PNG → none").

fn check_plain_is_none(image: &Image) {
    for (name, view) in views(image) {
        assert_eq!(
            read(&view),
            Outcome::None,
            "{name}: a plain image reads as state present"
        );
    }
}

#[test]
fn embed_searches_qa_plain_images_are_none_in_every_view() {
    prop::run(&(any::<u64>(), any::<bool>()), |(seed, alpha)| {
        // Every channel noise, alpha included, so both planes' low bits are random.
        let image = Image::new(128, 128, true, bytes(128 * 128 * 4, seed));
        let image = if alpha { image } else { image.without_alpha() };
        check_plain_is_none(&image);
        Ok(())
    });
}

negative_control!(
    embed_searches_qa_plain_images_are_none_in_every_view,
    "an image holding a record is not plain",
    expected = "a plain image reads as state present",
    {
        let mut image = canvas(128, 128, 51);
        embed(&mut image, &bytes(CONFIG, 52), false).expect("the payload embeds");
        check_plain_is_none(&image)
    }
);
