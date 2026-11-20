//! QA tests for TASK-M7-29, written from REQ-TOOL-063 and REQ-TOOL-068 against their source,
//! `principia_dd_image_embedding.md` §4 ("The three searches on read") and §7 ("Measured capacity and behaviour"), not
//! from the implementation.
//!
//! Every transform is applied here forward, as a pipeline applies it, from two primitives written from first
//! principles: a 90° counter-clockwise rotation and a left-to-right flip. The other six dihedral transforms are built
//! by composing them (rot180 = rot90², rot270 = rot90³, flip v = rot180 ∘ flip h, transpose = rot90 ∘ flip h,
//! anti-transpose = rot270 ∘ flip h), so a transform reported under the wrong name is caught by the composition, not
//! by a second copy of the same table. Rotations are named counter-clockwise, as `Dihedral`'s documentation states.
//!
//! Expected counts come from §2's geometry by hand: §7's config payload is 382 B, so its record is
//! 16 + 382 + 4 = 402 B = 3,216 bits; the RGB tile side is ⌈√⌈3216 / 3⌉⌉ = ⌈√1072⌉ = 33 and the alpha side
//! ⌈√3216⌉ = 57. At 128², ⌊128 / 33⌋² = 9 tiles (§7's table, §4's `tiles: 9/9`); at 512², ⌊512 / 33⌋² = 225 (§7's
//! 225/225) and ⌊512 / 57⌋² = 64 alpha tiles.
//!
//! The only crate items used are the public writer (`embed`), reader (`read`), `Image` and the reported types. Test
//! names carry the acceptance filters (`embed_searches`, `embed_survives`). Each test has a registered negative
//! control (R-176).

use proptest::prelude::*;
use render::embed::reader::{read, Dihedral, Outcome, Recovered};
use render::embed::writer::embed;
use render::embed::Image;
use validation::{negative_control, prop};

// ---------------------------------------------------------------------------------------------------------------
// Fixtures.

/// §7: the measured config-only payload, deflated.
const CONFIG: usize = 382;
/// §7: the config plus the full shader, deflated (382 + 2,969).
const CONFIG_SHADER: usize = 382 + 2_969;
/// §7: the survives list's resolution, REQ-TOOL-068's verify detail.
const SIDE: u32 = 512;
/// §2 by hand: the RGB tile side of §7's config record (see the module docs).
const RGB_SIDE: u32 = 33;
/// §2 by hand: the alpha tile side of §7's config record.
const ALPHA_SIDE: u32 = 57;

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

/// A canvas with a `len`-byte payload embedded: the image before, after, and the payload.
fn embedded(width: u32, height: u32, len: usize, seed: u64) -> (Image, Image, Vec<u8>) {
    let before = canvas(width, height, seed);
    let payload = bytes(len, seed ^ 0xA5A5_5A5A);
    let mut after = before.clone();
    embed(&mut after, &payload, false).expect("the payload embeds");
    (before, after, payload)
}

// ---------------------------------------------------------------------------------------------------------------
// The transforms, forward.

fn channels(image: &Image) -> usize {
    if image.has_alpha() {
        4
    } else {
        3
    }
}

/// The `w` × `h` image whose pixel `(u, v)` is `image`'s pixel `at(u, v)`.
fn build(image: &Image, w: u32, h: u32, at: impl Fn(u32, u32) -> (u32, u32)) -> Image {
    let c = channels(image);
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

/// Rotated 90° counter-clockwise: the top-right pixel becomes the top-left, the width becomes the height.
fn rot90(image: &Image) -> Image {
    let (w, h) = (image.width(), image.height());
    build(image, h, w, |u, v| (w - 1 - v, u))
}

/// Flipped left to right.
fn flip_h(image: &Image) -> Image {
    let w = image.width();
    build(image, w, image.height(), |u, v| (w - 1 - u, v))
}

/// `image` as `t` leaves it, composed from [`rot90`] and [`flip_h`] only.
fn apply(image: &Image, t: Dihedral) -> Image {
    let r = |i: &Image, n: usize| (0..n).fold(i.clone(), |i, _| rot90(&i));
    match t {
        Dihedral::Identity => image.clone(),
        Dihedral::Rot90 => r(image, 1),
        Dihedral::Rot180 => r(image, 2),
        Dihedral::Rot270 => r(image, 3),
        Dihedral::FlipH => flip_h(image),
        Dihedral::FlipV => r(&flip_h(image), 2),
        Dihedral::Transpose => r(&flip_h(image), 1),
        Dihedral::AntiTranspose => r(&flip_h(image), 3),
    }
}

const DIHEDRAL: [Dihedral; 8] = [
    Dihedral::Identity,
    Dihedral::Rot90,
    Dihedral::Rot180,
    Dihedral::Rot270,
    Dihedral::FlipH,
    Dihedral::FlipV,
    Dihedral::Transpose,
    Dihedral::AntiTranspose,
];

/// Nearest-neighbour upscale by `s`: every pixel an `s` × `s` block.
fn upscale(image: &Image, s: u32) -> Image {
    build(image, image.width() * s, image.height() * s, |u, v| {
        (u / s, v / s)
    })
}

/// The `w` × `h` region from `(x, y)`.
fn crop(image: &Image, x: u32, y: u32, w: u32, h: u32) -> Image {
    build(image, w, h, |u, v| (x + u, y + v))
}

/// Saved as a PNG at deflate level 9 with adaptive filtering (§7: `optimize=True`, `compress_level=9`) and loaded.
fn png_round_trip(image: &Image) -> Image {
    let mut file = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut file, image.width(), image.height());
        enc.set_color(if image.has_alpha() {
            png::ColorType::Rgba
        } else {
            png::ColorType::Rgb
        });
        enc.set_depth(png::BitDepth::Eight);
        enc.set_deflate_compression(png::DeflateCompression::Level(9));
        enc.set_filter(png::Filter::Adaptive);
        let mut w = enc.write_header().expect("PNG header");
        w.write_image_data(image.pixels()).expect("PNG data");
    }
    let mut dec = png::Decoder::new(std::io::Cursor::new(file))
        .read_info()
        .expect("PNG info");
    let mut px = vec![0; dec.output_buffer_size().expect("PNG buffer size")];
    let info = dec.next_frame(&mut px).expect("PNG frame");
    px.truncate(info.buffer_size());
    Image::new(
        info.width,
        info.height,
        info.color_type == png::ColorType::Rgba,
        px,
    )
}

/// The top `rows` rows replaced by other opaque pixels.
fn overwrite_top(image: &Image, rows: u32, seed: u64) -> Image {
    let mut out = image.clone();
    let c = channels(image);
    let n = rows as usize * image.width() as usize * c;
    let fresh = bytes(n, seed);
    for (i, (p, f)) in out.pixels_mut()[..n].iter_mut().zip(fresh).enumerate() {
        *p = if c == 4 && i % 4 == 3 {
            255 ^ (f & 1)
        } else {
            f
        };
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// The checks.

/// The report REQ-TOOL-063 asks for: `{transform, decimate, tiles}`, with the grid offset the search found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    dihedral: Dihedral,
    decimate: u32,
    /// RGB tiles found, of the `n_records` the record carries.
    tiles: (u32, u32),
}

fn recovered<'a>(outcome: &'a Outcome, payload: &[u8]) -> &'a Recovered {
    let Outcome::Recovered(r) = outcome else {
        panic!("the payload was not recovered: {outcome:?}");
    };
    assert_eq!(
        r.record.payload, payload,
        "the payload was not recovered intact"
    );
    r
}

/// `image` reads back `payload`, and the report names `want`.
fn check_report(image: &Image, payload: &[u8], want: Report) -> Recovered {
    let outcome = read(image);
    let r = recovered(&outcome, payload).clone();
    let t = r.how.transform;
    let got = Report {
        dihedral: t.dihedral,
        decimate: t.decimate,
        tiles: (r.how.tiles.found, r.how.tiles.of),
    };
    assert_eq!(
        got, want,
        "the report does not say how the image was recovered"
    );
    r
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-063: the three searches, the CRC as the oracle, and the report.

/// §4's own example: `{transform: rot270, decimate: 2, tiles: 9/9}` — 128² with the config payload (9 tiles, §7),
/// rotated 270° and upscaled ×2.
fn check_section_4_example(t: Dihedral, s: u32, report: Report) {
    let (_, image, payload) = embedded(128, 128, CONFIG, 1);
    check_report(&upscale(&apply(&image, t), s), &payload, report);
}

#[test]
fn embed_searches_qa_section_4_example() {
    check_section_4_example(
        Dihedral::Rot270,
        2,
        Report {
            dihedral: Dihedral::Rot270,
            decimate: 2,
            tiles: (9, 9),
        },
    );
}

negative_control!(
    embed_searches_qa_section_4_example,
    "rot90 instead of rot270 must be reported as rot90, so §4's report fails",
    expected = "the report does not say how",
    check_section_4_example(
        Dihedral::Rot90,
        2,
        Report {
            dihedral: Dihedral::Rot270,
            decimate: 2,
            tiles: (9, 9),
        },
    )
);

/// A 37 px crop of 128² and of 512²: the grid moves to `33 − 37 mod 33 = 29`, and the whole tiles left are
/// ⌊(n − 37 − 29) / 33⌋ a side.
fn check_crop_37(n: u32, at: u32) {
    let (_, image, payload) = embedded(n, n, CONFIG, 2);
    let cropped = crop(&image, at, at, n - at, n - at);
    let origin = (RGB_SIDE - at % RGB_SIDE) % RGB_SIDE;
    let a_side = (n - at - origin) / RGB_SIDE;
    let all = (n / RGB_SIDE).pow(2);
    let r = check_report(
        &cropped,
        &payload,
        Report {
            dihedral: Dihedral::Identity,
            decimate: 1,
            tiles: (a_side * a_side, all),
        },
    );
    assert_eq!(
        (r.how.transform.dx, r.how.transform.dy),
        (origin, origin),
        "the offset found is not the grid's"
    );
}

#[test]
fn embed_searches_qa_crop_37px() {
    check_crop_37(128, 37);
    check_crop_37(SIDE, 37);
}

negative_control!(
    embed_searches_qa_crop_37px,
    "an uncropped image must not report the cropped grid's tile count",
    expected = "the report does not say how",
    {
        let (_, image, payload) = embedded(SIDE, SIDE, CONFIG, 2);
        check_report(
            &image,
            &payload,
            Report {
                dihedral: Dihedral::Identity,
                decimate: 1,
                tiles: (169, 225),
            },
        );
    }
);

/// Every dihedral transform, on a non-square image so a transform that swaps the sides is told from one that does
/// not, alone and with a ×2 and ×3 upscale: each is recovered and reported by its name.
fn check_every_transform(name: impl Fn(Dihedral) -> Dihedral) {
    let (_, image, payload) = embedded(100, 70, CONFIG, 3);
    let all = (100 / RGB_SIDE) * (70 / RGB_SIDE);
    for t in DIHEDRAL {
        for s in [1, 2, 3] {
            check_report(
                &upscale(&apply(&image, t), s),
                &payload,
                Report {
                    dihedral: name(t),
                    decimate: s,
                    tiles: (all, all),
                },
            );
        }
    }
}

#[test]
fn embed_searches_qa_every_transform_and_decimation() {
    check_every_transform(|t| t);
}

negative_control!(
    embed_searches_qa_every_transform_and_decimation,
    "expecting each transform's inverse's name must fail for rot90",
    expected = "the report does not say how",
    check_every_transform(|t| match t {
        Dihedral::Rot90 => Dihedral::Rot270,
        Dihedral::Rot270 => Dihedral::Rot90,
        t => t,
    })
);

/// The three searches at once: upscaled ×3, rotated, then cropped at an offset that is a multiple neither of the
/// scale nor of the tile side, so the decimation's phase and the grid offset both move.
fn check_all_three(t: Dihedral) {
    let (_, image, payload) = embedded(128, 128, CONFIG, 4);
    let big = apply(&upscale(&image, 3), Dihedral::Rot90);
    let cut = crop(&big, 37, 37, big.width() - 37, big.height() - 37);
    // ⌈(384 − 37) / 3⌉ = 116 view pixels, from view pixel 12 or 13 of the original: one full tile row is lost at
    // most, so at least 4 of the 9 tiles remain.
    let outcome = read(&cut);
    let r = recovered(&outcome, &payload);
    assert_eq!(
        (r.how.transform.dihedral, r.how.transform.decimate),
        (t, 3),
        "the report does not say how the image was recovered"
    );
    assert!(r.how.tiles.found >= 4, "too few tiles: {:?}", r.how.tiles);
}

#[test]
fn embed_searches_qa_upscale_rotate_crop_together() {
    check_all_three(Dihedral::Rot90);
}

negative_control!(
    embed_searches_qa_upscale_rotate_crop_together,
    "the report must name rot90, not the identity",
    expected = "the report does not say how",
    check_all_three(Dihedral::Identity)
);

/// The CRC is the oracle: no candidate verifies when every record's payload is damaged, so nothing is recovered, and
/// an image with no state reads as none, through every transform.
fn check_no_false_recovery(damage: bool) {
    let (before, mut image, _) = embedded(128, 128, CONFIG, 5);
    if damage {
        // Flip the low bit of every channel inside the payload of each tile: rows 2..30 of each 33-px tile hold
        // payload bits only (the header and its CRC are the first 128 bits, ⌈128 / 3⌉ = 43 px, row 0 and 1).
        let w = image.width() as usize;
        for (i, p) in image.pixels_mut().iter_mut().enumerate() {
            let y = (i / 4) / w;
            if (2..30).contains(&(y % 33)) {
                *p ^= 1;
            }
        }
    }
    for t in DIHEDRAL {
        let outcome = read(&apply(&image, t));
        assert!(
            !matches!(outcome, Outcome::Recovered(_)),
            "a damaged record was recovered under {t:?}: {outcome:?}"
        );
        assert_ne!(
            outcome,
            Outcome::None,
            "a damaged record read as no state under {t:?}"
        );
    }
    assert_eq!(read(&before), Outcome::None, "a bare canvas reads as state");
}

#[test]
fn embed_searches_qa_crc_is_the_oracle() {
    check_no_false_recovery(true);
}

negative_control!(
    embed_searches_qa_crc_is_the_oracle,
    "an undamaged image must be recovered, so the check must fail",
    expected = "a damaged record was recovered",
    check_no_false_recovery(false)
);

/// §7, "Does not survive": averaging destroys the low bit; the reader must not report a recovery.
fn check_averaged_is_lost(s: u32) {
    let (_, image, payload) = embedded(SIDE / 2, SIDE / 2, CONFIG, 6);
    let big = upscale(&image, s);
    // A 2 × 2 box filter, as a bilinear resample at half a pixel leaves it.
    let c = channels(&big);
    let (w, h) = (big.width() as usize, big.height() as usize);
    let src = big.pixels().to_vec();
    let mut out = big.clone();
    for y in 0..h {
        for x in 0..w {
            for k in 0..c {
                let at = |xx: usize, yy: usize| {
                    u32::from(src[(yy.min(h - 1) * w + xx.min(w - 1)) * c + k])
                };
                out.pixels_mut()[(y * w + x) * c + k] =
                    ((at(x, y) + at(x + 1, y) + at(x, y + 1) + at(x + 1, y + 1) + 2) / 4) as u8;
            }
        }
    }
    if let Outcome::Recovered(r) = read(&out) {
        assert_ne!(r.record.payload, payload, "an averaged image was recovered");
        panic!("an averaged image was recovered with a wrong payload");
    }
}

#[test]
fn embed_searches_qa_averaging_is_not_recovered() {
    check_averaged_is_lost(1);
}

negative_control!(
    embed_searches_qa_averaging_is_not_recovered,
    "a ×2 nearest upscale's 2 × 2 box filter averages equal pixels, so the image is recovered and the check fails",
    expected = "an averaged image was recovered",
    check_averaged_is_lost(2)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-068: §7's "Survives" list at 512², max pixel delta 1.

/// One step a pipeline can apply, from §7's list.
#[derive(Clone, Copy, Debug)]
enum Step {
    Png,
    StripAlpha,
    /// Crop to 50 %, from `(x, y)`.
    CropHalf(u32, u32),
    /// Crop at an arbitrary offset `(x, y)` (§7: 37 px).
    CropAt(u32, u32),
    Dihedral(Dihedral),
    Upscale(u32),
    /// Rot90 and an upscale together.
    Rot90Upscale(u32),
    /// The top 50 % overwritten.
    OverwriteTop,
}

impl Step {
    fn run(self, image: &Image, seed: u64) -> Image {
        let (w, h) = (image.width(), image.height());
        match self {
            Self::Png => png_round_trip(image),
            Self::StripAlpha => image.without_alpha(),
            Self::CropHalf(x, y) => crop(image, x.min(w / 2), y.min(h / 2), w / 2, h / 2),
            Self::CropAt(x, y) => crop(image, x, y, w - x, h - y),
            Self::Dihedral(t) => apply(image, t),
            Self::Upscale(s) => upscale(image, s),
            Self::Rot90Upscale(s) => upscale(&rot90(image), s),
            Self::OverwriteTop => overwrite_top(image, h / 2, seed),
        }
    }
}

/// Each step of §7's list.
fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        Just(Step::Png),
        Just(Step::StripAlpha),
        (0..=SIDE / 2, 0..=SIDE / 2).prop_map(|(x, y)| Step::CropHalf(x, y)),
        Just(Step::CropAt(37, 37)),
        (0..RGB_SIDE * 2, 0..RGB_SIDE * 2).prop_map(|(x, y)| Step::CropAt(x, y)),
        proptest::sample::select(DIHEDRAL.to_vec()).prop_map(Step::Dihedral),
        (2..=3u32).prop_map(Step::Upscale),
        (2..=3u32).prop_map(Step::Rot90Upscale),
        Just(Step::OverwriteTop),
    ]
}

/// The embedding moves no channel by more than 1, and after `step` the payload is recovered intact.
fn check_survives(seed: u64, step: Step) {
    let (before, after, payload) = embedded(SIDE, SIDE, CONFIG, seed);
    check_delta_1(&before, &after);
    recovered(&read(&step.run(&after, seed.wrapping_add(7))), &payload);
}

fn check_delta_1(before: &Image, after: &Image) {
    let worst = before
        .pixels()
        .iter()
        .zip(after.pixels())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .expect("pixels");
    assert!(
        worst <= 1,
        "the embedding moved a channel by {worst}, more than 1"
    );
}

#[test]
fn embed_survives_qa_property() {
    prop::run(&(any::<u64>(), step()), |(seed, step)| {
        check_survives(seed, step);
        Ok(())
    });
}

negative_control!(
    embed_survives_qa_property,
    "LANCZOS/bilinear-style averaging is not on the list (§7): a 2 × 2 box filter on the embedded image is lost",
    expected = "the payload was not recovered",
    {
        let (_, after, payload) = embedded(SIDE, SIDE, CONFIG, 8);
        // Every channel averaged with its right-hand neighbour's.
        let mut out = after.clone();
        let src = after.pixels();
        for (i, p) in out.pixels_mut().iter_mut().enumerate() {
            let j = if (i / 4) % SIDE as usize == SIDE as usize - 1 { i } else { i + 4 };
            *p = (u16::from(src[i]) + u16::from(src[j])).div_ceil(2) as u8;
        }
        recovered(&read(&out), &payload);
    }
);

/// §7's list, entry by entry, with the counts §2's geometry gives at 512² (RGB 33 px, 15 × 15 = 225; alpha 57 px,
/// 8 × 8 = 64), alpha counted where the image keeps it.
fn check_list_counts(expect: impl Fn(&str, u32, Option<u32>) -> (u32, Option<u32>)) {
    let (_, image, payload) = embedded(SIDE, SIDE, CONFIG, 9);
    let alpha = |n: u32| Some(n);
    // From 128: the RGB grid at 33 − 128 mod 33 = 4, ⌊(256 − 4) / 33⌋ = 7; alpha at 57 − 128 mod 57 = 43,
    // ⌊(256 − 43) / 57⌋ = 3.
    // From 37: RGB at 29, ⌊(475 − 29) / 33⌋ = 13; alpha at 20, ⌊(475 − 20) / 57⌋ = 7.
    // Top 256 rows lost: RGB tile rows r with 33 r ≥ 256, r ≥ 8, 7 rows; alpha rows with 57 r ≥ 256, r ≥ 5, 3 rows.
    let cases: [(&str, Image, u32, Option<u32>); 16] = [
        ("png", png_round_trip(&image), 225, alpha(64)),
        ("alpha stripped", image.without_alpha(), 225, None),
        ("crop 50%", crop(&image, 128, 128, 256, 256), 49, alpha(9)),
        ("crop 37", crop(&image, 37, 37, 475, 475), 169, alpha(49)),
        (
            "crop 37 + png",
            png_round_trip(&crop(&image, 37, 37, 475, 475)),
            169,
            alpha(49),
        ),
        ("rot90", apply(&image, Dihedral::Rot90), 225, alpha(64)),
        ("rot180", apply(&image, Dihedral::Rot180), 225, alpha(64)),
        ("rot270", apply(&image, Dihedral::Rot270), 225, alpha(64)),
        ("flip h", apply(&image, Dihedral::FlipH), 225, alpha(64)),
        ("flip v", apply(&image, Dihedral::FlipV), 225, alpha(64)),
        (
            "transpose",
            apply(&image, Dihedral::Transpose),
            225,
            alpha(64),
        ),
        ("upscale 2", upscale(&image, 2), 225, alpha(64)),
        ("upscale 3", upscale(&image, 3), 225, alpha(64)),
        (
            "rot90 + upscale 2",
            upscale(&rot90(&image), 2),
            225,
            alpha(64),
        ),
        (
            "rot90 + upscale 3",
            upscale(&rot90(&image), 3),
            225,
            alpha(64),
        ),
        (
            "top 50% overwritten",
            overwrite_top(&image, 256, 10),
            105,
            alpha(24),
        ),
    ];
    for (name, out, rgb, a) in cases {
        let outcome = read(&out);
        let r = recovered(&outcome, &payload);
        let got = (r.how.tiles.found, r.how.alpha.map(|c| c.found));
        assert_eq!(r.how.tiles.of, 225, "{name}: n_records is not §7's 225");
        assert_eq!(
            got,
            expect(name, rgb, a),
            "{name}: the tiles recovered are not §2's count"
        );
    }
}

#[test]
fn embed_survives_qa_list_with_counts() {
    check_list_counts(|_, rgb, a| (rgb, a));
}

negative_control!(
    embed_survives_qa_list_with_counts,
    "the overwrite's count taken as the untouched image's 225 must fail",
    expected = "the tiles recovered are not",
    check_list_counts(|name, rgb, a| if name.starts_with("top") {
        (225, Some(64))
    } else {
        (rgb, a)
    })
);

/// Max pixel delta 1 at both ends of the channel range, with and without alpha, and with §7's larger payload
/// (config + full shader, 25 tiles at 512²), which still reads back through a rotation.
fn check_extremes(bump: u8) {
    for alpha in [true, false] {
        for len in [CONFIG, CONFIG_SHADER] {
            for fill in [0u8, 255, 254, 1] {
                let c = if alpha { 4 } else { 3 };
                let before = Image::new(SIDE, SIDE, alpha, vec![fill; (SIDE * SIDE) as usize * c]);
                let payload = bytes(len, u64::from(fill) + len as u64);
                let mut after = before.clone();
                embed(&mut after, &payload, false).expect("the payload embeds");
                after.pixels_mut()[0] = after.pixels()[0].wrapping_add(bump);
                check_delta_1(&before, &after);
                assert!(before != after, "the embedding changed nothing");
                if fill == 1 && alpha {
                    check_report(
                        &apply(&after, Dihedral::Rot90),
                        &payload,
                        Report {
                            dihedral: Dihedral::Rot90,
                            decimate: 1,
                            tiles: if len == CONFIG { (225, 225) } else { (25, 25) },
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn embed_survives_qa_max_pixel_delta_1_at_the_extremes() {
    check_extremes(0);
}

negative_control!(
    embed_survives_qa_max_pixel_delta_1_at_the_extremes,
    "one channel moved by 2 more must fail the delta check",
    expected = "more than 1",
    check_extremes(2)
);

#[test]
fn embed_survives_qa_alpha_side_is_57() {
    // Sanity on the by-hand geometry the counts above rest on.
    let bits = 8 * (16 + CONFIG as u32 + 4);
    let side = |b: u32, bpp: u32| (1..).find(|s: &u32| s * s * bpp >= b).expect("a side");
    assert_eq!(
        (side(bits, 3), side(bits, 1)),
        (RGB_SIDE, ALPHA_SIDE),
        "the by-hand sides are wrong"
    );
}

negative_control!(
    embed_survives_qa_alpha_side_is_57,
    "a 2-bit plane's side is not 33 and must fail",
    expected = "the by-hand sides are wrong",
    {
        let bits = 8 * (16 + CONFIG as u32 + 4);
        let side = |b: u32, bpp: u32| (1..).find(|s: &u32| s * s * bpp >= b).expect("a side");
        assert_eq!(
            (side(bits, 2), side(bits, 1)),
            (RGB_SIDE, ALPHA_SIDE),
            "the by-hand sides are wrong"
        );
    }
);
