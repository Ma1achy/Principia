//! QA tests for TASK-M7-30 against the definition `principia_dd_image_embedding.md` now gives the hybrid variant:
//! §2 "The hybrid layout" (blocks of `m` × `m` pixels, `k = m²`, tiles of blocks, the record field for field with
//! variant `2` its only marker) and §5's report (the angle in whole degrees counter-clockwise, 0 to 89, undone before
//! the dihedral transform; the `k` reported the one that verified). Written from those sections, not from the
//! implementation: the layout is checked through the tiled reader (§4) on the image subsampled at each of a block's
//! `k` pixels, and through which pixels the writer may touch, never through the crate's own slot arithmetic.
//!
//! Expected counts are §2's by hand. §7's config payload is 382 B, the record 402 B = 3,216 bits; the RGB tile side
//! is ⌈√⌈3216 / 3⌉⌉ = 33 and the alpha side ⌈√3216⌉ = 57, counted in blocks. Each test has a registered negative
//! control (R-176).

use render::embed::hybrid::{self, Redundancy};
use render::embed::reader::{self, Dihedral, Outcome, Recovered};
use render::embed::record::Variant;
use render::embed::Image;
use validation::negative_control;

/// §7: the measured config-only payload, deflated.
const CONFIG: usize = 382;
/// §2 by hand: the RGB and alpha tile sides of §7's config record.
const RGB_SIDE: u32 = 33;
const ALPHA_SIDE: u32 = 57;
/// A non-square size that is a multiple of neither block side, so rows and columns, and the strip beyond the last
/// whole block, are told apart.
const WIDTH: u32 = 523;
const HEIGHT: u32 = 347;

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

/// A `width` × `height` opaque RGBA canvas whose colour channels are pseudo-random.
fn canvas(width: u32, height: u32, seed: u64) -> Image {
    let mut px = bytes((width * height) as usize * 4, seed);
    for a in px.iter_mut().skip(3).step_by(4) {
        *a = 255;
    }
    Image::new(width, height, true, px)
}

/// `image`'s pixel `(x0 + i × m, y0 + j × m)` for every whole block `(i, j)`: one of each block's `k` pixels.
fn subsample(image: &Image, m: u32, x0: u32, y0: u32) -> Image {
    let (w, h) = (image.width() / m, image.height() / m);
    let ch = if image.has_alpha() { 4 } else { 3 };
    let mut out = Vec::with_capacity((w * h) as usize * ch);
    for j in 0..h {
        for i in 0..w {
            let at = ((y0 + j * m) * image.width() + x0 + i * m) as usize * ch;
            out.extend_from_slice(&image.pixels()[at..at + ch]);
        }
    }
    Image::new(w, h, image.has_alpha(), out)
}

/// The record `outcome` recovered, panicking with "not recovered" otherwise.
fn recovered<'a>(outcome: &'a Outcome, what: &str) -> &'a Recovered {
    match outcome {
        Outcome::Recovered(r) => r,
        other => panic!("{what}: not recovered: {other:?}"),
    }
}

/// §2's tile count of a plane whose side is `side` blocks, on a `width` × `height` image of `m` × `m` blocks.
fn tiles(width: u32, height: u32, m: u32, side: u32) -> u32 {
    (width / m / side) * (height / m / side)
}

// ---------------------------------------------------------------------------------------------------------------
// §2 "The hybrid layout": every one of a block's `k` pixels holds the bit, and the blocks hold the tiled layout of a
// `⌊width / m⌋` × `⌊height / m⌋` image: so the image subsampled at any one pixel of each block is a tiled image the
// tiled reader reads at the identity, with every tile of both grids intact, and the record says variant 2.

fn check_every_block_pixel_is_a_tiled_image(image: &Image, payload: &[u8], m: u32) {
    let rgb = tiles(WIDTH, HEIGHT, m, RGB_SIDE);
    let alpha = tiles(WIDTH, HEIGHT, m, ALPHA_SIDE);
    for dy in 0..m {
        for dx in 0..m {
            let what = format!("m = {m}, block pixel ({dx}, {dy})");
            let outcome = reader::read(&subsample(image, m, dx, dy));
            let r = recovered(&outcome, &what);
            assert_eq!(r.record.payload, payload, "{what}: wrong payload");
            assert_eq!(r.record.flags.variant, Variant::Hybrid, "{what}: variant");
            assert_eq!(u32::from(r.record.n_records), rgb, "{what}: n_records");
            assert_eq!(
                (r.how.tiles.found, r.how.tiles.of),
                (rgb, rgb),
                "{what}: RGB tiles"
            );
            let a = r.how.alpha.expect("the image has alpha");
            assert_eq!((a.found, a.of), (alpha, alpha), "{what}: alpha tiles");
            let t = r.how.transform;
            assert_eq!(
                (t.dx, t.dy, t.dihedral, t.decimate, t.rotation),
                (0, 0, Dihedral::Identity, 1, None),
                "{what}: not the writer's own grid"
            );
        }
    }
}

#[test]
fn embed_hybrid_rotation_qa_layout_every_block_pixel_is_the_tiled_layout() {
    for (redundancy, m, rgb, alpha) in [(Redundancy::K25, 5, 6, 1), (Redundancy::K9, 3, 15, 6)] {
        assert_eq!(redundancy.block(), m, "§2: m = √k");
        assert_eq!(redundancy.k(), m * m, "§2: k = m²");
        let payload = bytes(CONFIG, u64::from(m));
        let mut image = canvas(WIDTH, HEIGHT, u64::from(m));
        let placed = hybrid::embed(&mut image, &payload, false, redundancy).expect("embeds");
        // §2 by hand: 523 × 347 is 104 × 69 blocks of 5 (3 × 2 RGB tiles, 1 alpha) and 174 × 115 of 3 (5 × 3, 3 × 2).
        assert_eq!(
            (u32::from(placed.rgb), placed.alpha),
            (rgb, alpha),
            "m = {m}"
        );
        check_every_block_pixel_is_a_tiled_image(&image, &payload, m);
    }
}

negative_control!(
    embed_hybrid_rotation_qa_layout_every_block_pixel_is_the_tiled_layout,
    "one bit of one block's last pixel flipped in every RGB tile breaks that subsample's records",
    expected = "RGB tiles",
    {
        let payload = bytes(CONFIG, 5);
        let mut image = canvas(WIDTH, HEIGHT, 5);
        hybrid::embed(&mut image, &payload, false, Redundancy::K25).expect("embeds");
        // Slot 0 of every RGB tile, at the block's pixel (4, 4): the record's first bit, R.
        for r in 0..2 {
            for c in 0..3 {
                let (x, y) = ((c * RGB_SIDE) * 5 + 4, (r * RGB_SIDE) * 5 + 4);
                let low = image.low(render::embed::Plane::Rgb, x, y, 0);
                image.set_low(render::embed::Plane::Rgb, x, y, 0, low ^ 1);
            }
        }
        check_every_block_pixel_is_a_tiled_image(&image, &payload, 5);
    }
);

/// §2: the pixels beyond the last whole block, and the blocks beyond the last whole tile of either plane, are not
/// written, and no channel moves by more than 1 (§7: max pixel delta 1).
fn check_untouched_outside_the_grids(before: &Image, after: &Image, m: u32) {
    let rgb_w = (WIDTH / m / RGB_SIDE) * RGB_SIDE * m;
    let rgb_h = (HEIGHT / m / RGB_SIDE) * RGB_SIDE * m;
    let a_w = (WIDTH / m / ALPHA_SIDE) * ALPHA_SIDE * m;
    let a_h = (HEIGHT / m / ALPHA_SIDE) * ALPHA_SIDE * m;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let at = ((y * WIDTH + x) * 4) as usize;
            for c in 0..4 {
                let (was, now) = (before.pixels()[at + c], after.pixels()[at + c]);
                assert!(
                    was.abs_diff(now) <= 1,
                    "({x}, {y}) channel {c} moved by more than 1"
                );
                let inside = if c < 3 {
                    x < rgb_w && y < rgb_h
                } else {
                    x < a_w && y < a_h
                };
                assert!(
                    inside || was == now,
                    "m = {m}: ({x}, {y}) channel {c} outside the grid was written"
                );
            }
        }
    }
}

#[test]
fn embed_hybrid_rotation_qa_layout_writes_nothing_outside_the_grids() {
    for redundancy in [Redundancy::K25, Redundancy::K9] {
        let m = redundancy.block();
        let before = canvas(WIDTH, HEIGHT, 50 + u64::from(m));
        let mut after = before.clone();
        hybrid::embed(&mut after, &bytes(CONFIG, 7), false, redundancy).expect("embeds");
        check_untouched_outside_the_grids(&before, &after, m);
    }
}

negative_control!(
    embed_hybrid_rotation_qa_layout_writes_nothing_outside_the_grids,
    "the tiled writer, whose grid is m times smaller in pixels, writes outside the hybrid's",
    expected = "outside the grid was written",
    {
        let before = canvas(WIDTH, HEIGHT, 55);
        let mut after = before.clone();
        render::embed::writer::embed(&mut after, &bytes(CONFIG, 7), false).expect("embeds");
        check_untouched_outside_the_grids(&before, &after, 5);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// §5's report: the angle in whole degrees counter-clockwise, 0 to 89, undone before the dihedral transform reported
// beside it; the `k` reported the one that verified.

/// `image` rotated `degrees` counter-clockwise on screen (y down) about its centre, nearest neighbour, onto a canvas
/// holding the whole rotated image; every pixel outside the source is transparent black.
fn rotate(image: &Image, degrees: f64) -> Image {
    let (s, c) = degrees.to_radians().sin_cos();
    let (w, h) = (f64::from(image.width()), f64::from(image.height()));
    let ow = (w * c.abs() + h * s.abs() - 1e-9).ceil() as u32;
    let oh = (w * s.abs() + h * c.abs() - 1e-9).ceil() as u32;
    let ch = if image.has_alpha() { 4 } else { 3 };
    let src = image.pixels();
    let mut out = vec![0u8; (ow * oh) as usize * ch];
    for y in 0..oh {
        for x in 0..ow {
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

/// `outcome` recovered `payload` reporting `dihedral` and, within one degree (the first whole degree that verifies
/// may be the one before or after the nearest, §5), `degrees` counter-clockwise in 0 to 89, at `redundancy`.
fn check_report(
    outcome: &Outcome,
    payload: &[u8],
    dihedral: Dihedral,
    degrees: u32,
    redundancy: Redundancy,
    what: &str,
) {
    let r = recovered(outcome, what);
    assert_eq!(r.record.payload, payload, "{what}: wrong payload");
    let t = r.how.transform;
    let rot = t
        .rotation
        .unwrap_or_else(|| panic!("{what}: no rotation reported"));
    assert_eq!(t.dihedral, dihedral, "{what}: wrong dihedral");
    assert!(
        rot.degrees < 90,
        "{what}: angle {} outside 0 to 89",
        rot.degrees
    );
    assert!(
        rot.degrees.abs_diff(degrees) <= 1,
        "{what}: wrong angle, {} reported for {degrees}",
        rot.degrees
    );
    assert_eq!(rot.redundancy, redundancy, "{what}: wrong k reported");
}

/// §5's own examples, at 256² and `k = 25`: 7° counter-clockwise is the identity and 7°; 97° counter-clockwise a 90°
/// rotation and 7°; 34° clockwise a 270° rotation and 56°.
#[test]
fn embed_hybrid_rotation_qa_report_angle_direction() {
    let payload = bytes(CONFIG, 8);
    let mut image = canvas(256, 256, 8);
    hybrid::embed(&mut image, &payload, false, Redundancy::K25).expect("embeds");
    for (turn, dihedral, degrees) in [
        (7.0, Dihedral::Identity, 7),
        (97.0, Dihedral::Rot90, 7),
        (-34.0, Dihedral::Rot270, 56),
    ] {
        check_report(
            &hybrid::read(&rotate(&image, turn)),
            &payload,
            dihedral,
            degrees,
            Redundancy::K25,
            &format!("rotated {turn}° counter-clockwise"),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_qa_report_angle_direction,
    "34° clockwise read as 34° counter-clockwise must fail",
    expected = "wrong",
    {
        let payload = bytes(CONFIG, 8);
        let mut image = canvas(256, 256, 8);
        hybrid::embed(&mut image, &payload, false, Redundancy::K25).expect("embeds");
        check_report(
            &hybrid::read(&rotate(&image, -34.0)),
            &payload,
            Dihedral::Identity,
            34,
            Redundancy::K25,
            "34° clockwise",
        );
    }
);

/// Flips every low bit of `image`'s colour channels with probability `p`: §5's uniform noise.
fn noise(image: &mut Image, p: f64, seed: u64) {
    let draws = bytes(image.pixels().len() * 2, seed);
    for (i, (byte, d)) in image
        .pixels_mut()
        .iter_mut()
        .zip(draws.chunks_exact(2))
        .enumerate()
    {
        if i % 4 != 3 && f64::from(u16::from_be_bytes([d[0], d[1]])) < p * 65_536.0 {
            *byte ^= 1;
        }
    }
}

/// §5: the `k` reported is the one that verified, which is the one written. Upright, with uniform noise inside each
/// `k`'s §5 figure (3 % < 5 % for `k = 9`, 10 % < 15 % for `k = 25`) and in every channel (alpha included) so that no
/// record survives §4's searches, the angle search reads at 0° with the written `k`.
fn noisy_report(redundancy: Redundancy, p: f64, written: Redundancy) {
    let payload = bytes(CONFIG, 9);
    let mut image = canvas(512, 512, 9);
    hybrid::embed(&mut image, &payload, false, written).expect("embeds");
    noise(&mut image, p, 90);
    // Alpha too, so §4's searches find no intact alpha record either.
    let draws = bytes(image.pixels().len(), 91);
    for (i, byte) in image.pixels_mut().iter_mut().enumerate() {
        if i % 4 == 3 && f64::from(draws[i]) < p * 256.0 {
            *byte ^= 1;
        }
    }
    assert!(
        !matches!(reader::read(&image), Outcome::Recovered(_)),
        "§4's searches alone read the noisy image: the test does not reach the angle search"
    );
    check_report(
        &hybrid::read(&image),
        &payload,
        Dihedral::Identity,
        0,
        redundancy,
        &format!("k = {} written, {} % noise", written.k(), p * 100.0),
    );
}

#[test]
fn embed_hybrid_rotation_qa_report_k_is_the_one_written() {
    noisy_report(Redundancy::K9, 0.03, Redundancy::K9);
    noisy_report(Redundancy::K25, 0.10, Redundancy::K25);
}

negative_control!(
    embed_hybrid_rotation_qa_report_k_is_the_one_written,
    "a k = 9 image reported as k = 25 must fail",
    expected = "wrong k reported",
    { noisy_report(Redundancy::K25, 0.03, Redundancy::K9) }
);
