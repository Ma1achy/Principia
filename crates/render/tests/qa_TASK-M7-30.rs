//! QA tests for TASK-M7-30, written from REQ-TOOL-064 and REQ-TOOL-111 against their source,
//! `principia_dd_image_embedding.md` §5 ("Two variants, one choice") and §7 ("Measured capacity and behaviour"), with
//! §2's tile geometry and §4's "the CRC is the oracle", not from the implementation.
//!
//! The rotation is applied here forward, as an image editor applies it: nearest neighbour, counter-clockwise (y down,
//! so a counter-clockwise turn on screen), about the image's centre, onto a canvas that holds the whole rotated image
//! plus `pad` transparent-black pixels on every side, a pipeline's own border. The reader is given no hint of the
//! angle, the canvas size or the padding.
//!
//! Expected counts come from §2 by hand: §7's config payload is 382 B, the record 402 B = 3,216 bits, the RGB tile
//! side ⌈√1072⌉ = 33 slots. Under the hybrid variant a slot is a `b` × `b` block, `k = b²` (§5, "each bit written
//! `k`×"), so at `k = 25` a tile is 33 × 5 = 165 px a side.
//!
//! The only crate items used are the public writers, readers, `Image` and the reported types. Test names carry the
//! acceptance filter (`embed_hybrid_rotation`). Each test has a registered negative control (R-176).

use render::embed::hybrid::{self, Redundancy};
use render::embed::reader::Outcome;
use render::embed::record::Variant;
use render::embed::writer;
use render::embed::Image;
use validation::negative_control;

/// §7: the measured config-only payload, deflated.
const CONFIG: usize = 382;
/// §7's arbitrary rotations, REQ-TOOL-064's verify detail, degrees counter-clockwise.
const ANGLES: [f64; 5] = [7.0, 34.0, 45.0, 7.3, 34.7];
/// §2 by hand: the RGB tile side of §7's config record, in slots.
const RGB_SIDE: u32 = 33;

/// SplitMix64 bytes, `seed` picking the stream.
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

/// A `width` × `height` canvas whose colour channels are pseudo-random; opaque RGBA when `alpha`, else RGB.
fn canvas(width: u32, height: u32, alpha: bool, seed: u64) -> Image {
    let channels = if alpha { 4 } else { 3 };
    let mut px = bytes((width * height) as usize * channels, seed);
    if alpha {
        for a in px.iter_mut().skip(3).step_by(4) {
            *a = 255;
        }
    }
    Image::new(width, height, alpha, px)
}

/// A canvas with §7's config payload embedded under the hybrid variant at `redundancy`, and the payload.
fn hybrid_image(
    width: u32,
    height: u32,
    alpha: bool,
    redundancy: Redundancy,
    seed: u64,
) -> (Image, Vec<u8>) {
    let payload = bytes(CONFIG, seed ^ 0x0BAD_CAFE);
    let mut image = canvas(width, height, alpha, seed);
    hybrid::embed(&mut image, &payload, false, redundancy).expect("the payload embeds");
    (image, payload)
}

/// `image` rotated `degrees` counter-clockwise about its centre, nearest neighbour, onto a canvas that holds the whole
/// rotated image with `pad` pixels to spare on every side; every pixel outside the source is transparent black.
fn rotate(image: &Image, degrees: f64, pad: u32) -> Image {
    let (s, c) = degrees.to_radians().sin_cos();
    let (w, h) = (f64::from(image.width()), f64::from(image.height()));
    // The rotated image's bounding box, rounded up, and the padding.
    let ow = (w * c.abs() + h * s.abs() - 1e-9).ceil() as u32 + 2 * pad;
    let oh = (w * s.abs() + h * c.abs() - 1e-9).ceil() as u32 + 2 * pad;
    let ch = if image.has_alpha() { 4 } else { 3 };
    let src = image.pixels();
    let mut out = vec![0u8; (ow * oh) as usize * ch];
    for y in 0..oh {
        for x in 0..ow {
            // The output pixel's centre relative to the output centre, turned clockwise back into the source.
            let px = f64::from(x) + 0.5 - f64::from(ow) / 2.0;
            let py = f64::from(y) + 0.5 - f64::from(oh) / 2.0;
            let sx = c * px - s * py + w / 2.0;
            let sy = s * px + c * py + h / 2.0;
            if (0.0..w).contains(&sx) && (0.0..h).contains(&sy) {
                let from = (sy as usize * image.width() as usize + sx as usize) * ch;
                let to = (y * ow + x) as usize * ch;
                out[to..to + ch].copy_from_slice(&src[from..from + ch]);
            }
        }
    }
    Image::new(ow, oh, image.has_alpha(), out)
}

/// Flips every low bit of `image` (alpha's included) with probability `p`: §5's uniform noise.
fn noise(image: &mut Image, p: f64, seed: u64) {
    let draws = bytes(image.pixels().len() * 2, seed);
    for (byte, d) in image.pixels_mut().iter_mut().zip(draws.chunks_exact(2)) {
        if f64::from(u16::from_be_bytes([d[0], d[1]])) < p * 65_536.0 {
            *byte ^= 1;
        }
    }
}

/// The payload `outcome` recovered from a hybrid record, panicking with "not recovered" otherwise.
fn recovered_hybrid(outcome: &Outcome, payload: &[u8], what: &str) {
    let Outcome::Recovered(r) = outcome else {
        panic!("{what}: not recovered: {outcome:?}");
    };
    assert_eq!(
        r.record.payload, payload,
        "{what}: a wrong payload was recovered"
    );
    assert_eq!(
        r.record.flags.variant,
        Variant::Hybrid,
        "{what}: not the hybrid variant's record"
    );
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-064: the hybrid recovers after §7's arbitrary rotations.

/// §7's angles at §7's resolutions that hold a `k = 25` tile (256², 512², 1024²: 64² and 128² are smaller than
/// 165 px), on a canvas padded by a pipeline's border, with and without alpha (§7: alpha stripped is survived).
fn check_angles(size: u32, alpha: bool, pad: u32, angles: &[f64], seed: u64) {
    let (image, payload) = hybrid_image(size, size, alpha, Redundancy::K25, seed);
    for &deg in angles {
        let rotated = rotate(&image, deg, pad);
        recovered_hybrid(
            &hybrid::read(&rotated),
            &payload,
            &format!("{size}² rotated {deg}°"),
        );
    }
}

#[test]
fn embed_hybrid_rotation_qa_angles_at_256_rgb_only_padded() {
    check_angles(256, false, 7, &ANGLES, 1);
}

negative_control!(
    embed_hybrid_rotation_qa_angles_at_256_rgb_only_padded,
    "the same rotations of the tiled variant are not recovered",
    expected = "not recovered",
    {
        let payload = bytes(CONFIG, 1);
        let mut image = canvas(256, 256, false, 1);
        writer::embed(&mut image, &payload, false).expect("the payload embeds");
        let rotated = rotate(&image, ANGLES[3], 7);
        recovered_hybrid(&hybrid::read(&rotated), &payload, "tiled rotated 7.3°");
    }
);

#[test]
fn embed_hybrid_rotation_qa_angles_at_1024() {
    // The largest of §7's resolutions, and the angle furthest from a whole degree (0.3° from 35°, 0.7° from 34°): the
    // fraction of a degree left over drifts furthest across it.
    check_angles(1024, true, 0, &[34.7], 2);
}

negative_control!(
    embed_hybrid_rotation_qa_angles_at_1024,
    "the plain reader (§4's searches alone) does not recover 34.7° at 1024²",
    expected = "not recovered",
    {
        let (image, payload) = hybrid_image(1024, 1024, true, Redundancy::K25, 2);
        recovered_hybrid(
            &render::embed::reader::read(&rotate(&image, 34.7, 0)),
            &payload,
            "plain reader",
        );
    }
);

/// A non-square image: the rotated canvas's two sides differ, so a reader that confuses them misplaces every block.
#[test]
fn embed_hybrid_rotation_qa_angles_non_square() {
    let (image, payload) = hybrid_image(420, 250, true, Redundancy::K25, 3);
    for &deg in &ANGLES {
        recovered_hybrid(
            &hybrid::read(&rotate(&image, deg, 3)),
            &payload,
            &format!("420 × 250 rotated {deg}°"),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_qa_angles_non_square,
    "a non-square image with its records wiped is not recovered",
    expected = "not recovered",
    {
        let (mut image, payload) = hybrid_image(420, 250, true, Redundancy::K25, 3);
        for b in image.pixels_mut() {
            *b &= !1;
        }
        recovered_hybrid(&hybrid::read(&rotate(&image, 34.0, 3)), &payload, "wiped");
    }
);

/// An image exactly one `k = 25` tile in size, 165², holds one record and no slack: every block of the record must be
/// read where the writer put it. Upright with uniform noise inside §5's `k = 25` figure (10 % < 15 %), the single pixel
/// §4's decimation reads is not enough and only the vote over each block recovers it; rotated by §7's angles it
/// recovers too.
#[test]
fn embed_hybrid_rotation_qa_one_tile_no_slack() {
    let size = RGB_SIDE * Redundancy::K25.block();
    assert_eq!(size, 165, "§2 by hand: 33 slots of 5 px");
    let (image, payload) = hybrid_image(size, size, false, Redundancy::K25, 4);
    let mut noisy = image.clone();
    noise(&mut noisy, 0.10, 40);
    recovered_hybrid(&hybrid::read(&noisy), &payload, "165² upright, 10 % noise");
    for &deg in &ANGLES {
        recovered_hybrid(
            &hybrid::read(&rotate(&image, deg, 0)),
            &payload,
            &format!("165² rotated {deg}°"),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_qa_one_tile_no_slack,
    "a 164² image holds no k = 25 tile, so nothing is embedded or recovered",
    expected = "not recovered",
    {
        let payload = bytes(CONFIG, 4);
        let mut image = canvas(164, 164, false, 4);
        assert!(hybrid::embed(&mut image, &payload, false, Redundancy::K25).is_err());
        recovered_hybrid(&hybrid::read(&image), &payload, "164²");
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-064: the tiled variant stays the default, and it is the hybrid that rotation needs (§5).

/// §5: nearest rotation leaves bit errors that whole-record repetition cannot absorb. A tiled image, rotated, is not
/// recovered as a payload by any reader, and is never reported recovered with a wrong payload.
#[test]
fn embed_hybrid_rotation_qa_tiled_does_not_survive_rotation() {
    let payload = bytes(CONFIG, 5);
    let mut image = canvas(256, 256, true, 5);
    let placed = writer::embed(&mut image, &payload, false).expect("the payload embeds");
    assert_eq!(placed.rgb, 49, "§7: 49 tiles at 256²");
    let outcome = hybrid::read(&rotate(&image, 7.0, 0));
    assert!(
        !matches!(outcome, Outcome::Recovered(_)),
        "a tiled image rotated 7° was recovered: {outcome:?}"
    );
}

negative_control!(
    embed_hybrid_rotation_qa_tiled_does_not_survive_rotation,
    "an unrotated tiled image is recovered",
    expected = "was recovered",
    {
        let payload = bytes(CONFIG, 5);
        let mut image = canvas(256, 256, true, 5);
        writer::embed(&mut image, &payload, false).expect("the payload embeds");
        let outcome = hybrid::read(&rotate(&image, 0.0, 0));
        assert!(
            !matches!(outcome, Outcome::Recovered(_)),
            "a tiled image rotated 0° was recovered: {outcome:?}"
        );
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-064: the hybrid is redundancy *plus tiling* (§5's table: ✓ on localised damage as well as noise).

/// `image` cropped to the `width` × `height` window at `(x0, y0)`.
fn crop(image: &Image, x0: u32, y0: u32, width: u32, height: u32) -> Image {
    let ch = if image.has_alpha() { 4 } else { 3 };
    let mut out = Vec::with_capacity((width * height) as usize * ch);
    for y in y0..y0 + height {
        let at = (y * image.width() + x0) as usize * ch;
        out.extend_from_slice(&image.pixels()[at..at + width as usize * ch]);
    }
    Image::new(width, height, image.has_alpha(), out)
}

/// §5: tiling keeps the hybrid alive under localised damage. At 512² and `k = 25` the grid holds ⌊512 / 165⌋² = 9
/// tiles; with the top half overwritten (§7's "top 50% overwritten") and the image cropped at §7's arbitrary 37 px
/// offset, then rotated by §7's angles, the surviving tiles still give the payload.
#[test]
fn embed_hybrid_rotation_qa_survives_localised_damage_then_rotation() {
    let (mut image, payload) = hybrid_image(512, 512, true, Redundancy::K25, 6);
    let half = image.pixels().len() / 2;
    let fresh = bytes(half, 66);
    image.pixels_mut()[..half].copy_from_slice(&fresh);
    let damaged = crop(&image, 37, 37, 512 - 37, 512 - 37);
    for deg in [7.3, 34.7] {
        recovered_hybrid(
            &hybrid::read(&rotate(&damaged, deg, 2)),
            &payload,
            &format!("damaged, rotated {deg}°"),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_qa_survives_localised_damage_then_rotation,
    "overwriting the whole image leaves nothing to recover",
    expected = "not recovered",
    {
        let (mut image, payload) = hybrid_image(512, 512, true, Redundancy::K25, 6);
        let all = image.pixels().len();
        image.pixels_mut().copy_from_slice(&bytes(all, 66));
        recovered_hybrid(
            &hybrid::read(&rotate(&image, 7.3, 2)),
            &payload,
            "overwritten",
        );
    }
);
