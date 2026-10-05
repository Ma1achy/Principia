//! The image embedding's hybrid variant (`principia_dd_image_embedding.md` §5, §7): per-bit `k`-fold redundancy plus
//! tiling, recovered after arbitrary rotation by the angle search (REQ-TOOL-064), and the evidence for its default
//! `k` (REQ-TOOL-111, R-71). Each test registers its negative control (R-176).
//!
//! The rotation is written here as a pipeline applies it, forward from the image it is given, independently of the
//! reader's angle search, which undoes it: nearest neighbour, counter-clockwise, about the image's centre, onto a
//! canvas that holds the whole rotated image, the corners outside it transparent black.

use render::embed::hybrid::{self, Redundancy, Rotation, DEFAULT};
use render::embed::reader::{self, Corrupt, CorruptReason, Dihedral, Outcome};
use render::embed::record::{bit_slot, encode, record_bit, Flags, Record, Variant};
use render::embed::writer::{self, EmbedError, Placed};
use render::embed::{Image, Plane};
use validation::negative_control;

/// §7's measured config-only payload, deflated.
const CONFIG_PAYLOAD: usize = 382;

/// §7's config payload with the full shader, 382 + 2,969 B.
const SHADER_PAYLOAD: usize = 382 + 2969;

/// §7's arbitrary rotations, in degrees counter-clockwise.
const ANGLES: [f64; 5] = [7.0, 34.0, 45.0, 7.3, 34.7];

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

/// An opaque `width` × `height` RGBA image whose R, G and B are pseudo-random.
fn canvas(width: u32, height: u32, seed: u32) -> Image {
    let mut pixels = noise((width * height * 4) as usize, seed);
    for alpha in pixels.iter_mut().skip(3).step_by(4) {
        *alpha = 255;
    }
    Image::new(width, height, true, pixels)
}

/// A `width` × `height` canvas with a `len`-byte payload embedded under the hybrid variant at `redundancy`, and the
/// payload.
fn embedded(
    width: u32,
    height: u32,
    len: usize,
    seed: u32,
    redundancy: Redundancy,
) -> (Image, Vec<u8>) {
    let payload = noise(len, seed ^ 0x5EED);
    let mut image = canvas(width, height, seed);
    hybrid::embed(&mut image, &payload, false, redundancy).expect("the payload embeds");
    (image, payload)
}

/// `image` rotated `degrees` counter-clockwise about its centre, nearest neighbour. With `expand` the output is
/// `⌈w |cos θ| + h |sin θ|⌉` × `⌈w |sin θ| + h |cos θ|⌉`, holding the whole rotated image; without, it keeps `image`'s
/// size and the corners are cut. Each output pixel takes the input pixel under its centre rotated back; pixels
/// outside the input are transparent black.
fn rotate(image: &Image, degrees: f64, expand: bool) -> Image {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (w, h) = (f64::from(image.width()), f64::from(image.height()));
    let (out_w, out_h) = if expand {
        (
            (w * cos.abs() + h * sin.abs() - 1e-9).ceil(),
            (w * sin.abs() + h * cos.abs() - 1e-9).ceil(),
        )
    } else {
        (w, h)
    };
    let channels = if image.has_alpha() { 4 } else { 3 };
    let mut pixels = vec![0u8; (out_w * out_h) as usize * channels];
    for y in 0..out_h as usize {
        for x in 0..out_w as usize {
            let (dx, dy) = (x as f64 + 0.5 - out_w / 2.0, y as f64 + 0.5 - out_h / 2.0);
            let sx = (dx * cos - dy * sin + w / 2.0).floor();
            let sy = (dx * sin + dy * cos + h / 2.0).floor();
            if sx >= 0.0 && sy >= 0.0 && sx < w && sy < h {
                let from = (sy as usize * image.width() as usize + sx as usize) * channels;
                let to = (y * out_w as usize + x) * channels;
                pixels[to..to + channels].copy_from_slice(&image.pixels()[from..from + channels]);
            }
        }
    }
    Image::new(out_w as u32, out_h as u32, image.has_alpha(), pixels)
}

/// Flips each low bit of `image`, alpha's included, with probability `p`, the flips drawn from `seed`: §5's uniform
/// noise.
fn flip_low_bits(image: &mut Image, p: f64, seed: u32) {
    let draws = noise(image.pixels().len() * 2, seed);
    for (byte, draw) in image.pixels_mut().iter_mut().zip(draws.chunks_exact(2)) {
        if f64::from(u16::from_le_bytes([draw[0], draw[1]])) < p * 65536.0 {
            *byte ^= 1;
        }
    }
}

// --- The checks -----------------------------------------------------------------------------------------------------

/// `outcome` recovered `payload` from a hybrid record, through `dihedral` and `rotation`, with `tiles` RGB records
/// found against `of`.
fn check_rotated(
    outcome: &Outcome,
    payload: &[u8],
    dihedral: Dihedral,
    rotation: Option<Rotation>,
    tiles: (u32, u32),
) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("the payload was not recovered: {outcome:?}");
    };
    assert_eq!(
        recovered.record.payload, payload,
        "the payload was not recovered"
    );
    assert_eq!(
        recovered.record.flags.variant,
        Variant::Hybrid,
        "the record is not the hybrid variant's"
    );
    let how = recovered.how;
    assert_eq!(
        (how.transform.dihedral, how.transform.rotation),
        (dihedral, rotation),
        "the report does not say how it was recovered"
    );
    assert_eq!(
        (how.tiles.found, how.tiles.of),
        tiles,
        "the report does not count the records recovered"
    );
}

/// The rotation the angle search reports undoing: `degrees`, the default redundancy.
fn undone(degrees: u32) -> Option<Rotation> {
    Some(Rotation {
        degrees,
        redundancy: DEFAULT,
    })
}

// --- Arbitrary rotation (REQ-TOOL-064) ------------------------------------------------------------------------------

/// The first whole degree whose blocks hold an intact record (§4: the first that verifies is the one read) for each of
/// §7's angles, on a 256² image at the default `k`, which holds one RGB tile: 7.3° is read at 7°, 34.7° at 34°.
const UNDONE_256: [u32; 5] = [7, 34, 45, 7, 34];

#[test]
fn embed_hybrid_rotation_recovers_section_7s_angles() {
    let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 1, DEFAULT);
    for (degrees, undo) in ANGLES.into_iter().zip(UNDONE_256) {
        let rotated = rotate(&image, degrees, true);
        check_rotated(
            &hybrid::read(&rotated),
            &payload,
            Dihedral::Identity,
            undone(undo),
            (1, 1),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_recovers_section_7s_angles,
    "without the angle search, §4's three searches alone recover nothing after 7° of rotation",
    expected = "the payload was not recovered",
    {
        let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 1, DEFAULT);
        let rotated = rotate(&image, ANGLES[0], true);
        check_rotated(
            &reader::read(&rotated),
            &payload,
            Dihedral::Identity,
            undone(7),
            (1, 1),
        )
    }
);

/// Beyond 90°, the dihedral search gives the multiple of 90° and the angle search the rest: 97° is a 90° rotation
/// and 7°, and 214° is 180° and 34°. An image flipped after 7° of rotation is the image flipped, then rotated −7°:
/// the search undoes 83° and is left with the flip followed by a 270° rotation, the anti-diagonal reflection.
#[test]
fn embed_hybrid_rotation_beyond_90_degrees_is_dihedral_and_angle() {
    let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 2, DEFAULT);
    let cases = [
        (rotate(&image, 97.0, true), Dihedral::Rot90, 7),
        (rotate(&image, 214.0, true), Dihedral::Rot180, 34),
        (
            flip_h(&rotate(&image, 7.0, true)),
            Dihedral::AntiTranspose,
            83,
        ),
    ];
    for (rotated, dihedral, degrees) in cases {
        check_rotated(
            &hybrid::read(&rotated),
            &payload,
            dihedral,
            undone(degrees),
            (1, 1),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_beyond_90_degrees_is_dihedral_and_angle,
    "97° reported as 7° alone must fail",
    expected = "the report does not say how it was recovered",
    {
        let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 2, DEFAULT);
        let rotated = rotate(&image, 97.0, true);
        check_rotated(
            &hybrid::read(&rotated),
            &payload,
            Dihedral::Identity,
            undone(7),
            (1, 1),
        )
    }
);

/// `image` flipped left to right.
fn flip_h(image: &Image) -> Image {
    let (w, channels) = (
        image.width() as usize,
        if image.has_alpha() { 4 } else { 3 },
    );
    let mut pixels = Vec::with_capacity(image.pixels().len());
    for row in image.pixels().chunks_exact(w * channels) {
        for x in (0..w).rev() {
            pixels.extend_from_slice(&row[x * channels..(x + 1) * channels]);
        }
    }
    Image::new(image.width(), image.height(), image.has_alpha(), pixels)
}

/// A 512² image rotated within its own frame, the corners cut: §7's angles still recover from the tiles left whole,
/// 3 × 3 of side 165 px at the default `k`. The tiles counted are those whose blocks the first verifying angle reads.
#[test]
fn embed_hybrid_rotation_with_the_corners_cut() {
    let (image, payload) = embedded(512, 512, CONFIG_PAYLOAD, 3, DEFAULT);
    let expected = [(7, 3), (34, 4), (45, 5), (7, 1), (35, 4)];
    for (degrees, (undo, found)) in ANGLES.into_iter().zip(expected) {
        let rotated = rotate(&image, degrees, false);
        check_rotated(
            &hybrid::read(&rotated),
            &payload,
            Dihedral::Identity,
            undone(undo),
            (found, 9),
        );
    }
}

negative_control!(
    embed_hybrid_rotation_with_the_corners_cut,
    "the tiles recovered after 7° with the corners cut are not all nine",
    expected = "the report does not count the records recovered",
    {
        let (image, payload) = embedded(512, 512, CONFIG_PAYLOAD, 3, DEFAULT);
        let rotated = rotate(&image, 7.0, false);
        check_rotated(
            &hybrid::read(&rotated),
            &payload,
            Dihedral::Identity,
            undone(7),
            (9, 9),
        )
    }
);

/// An unrotated hybrid image is read by §4's decimation search (each block is a nearest upscale's pixel), with no
/// rotation reported; under uniform noise the single pixel decimation reads is not enough, and the angle search's
/// block vote at 0° recovers it.
#[test]
fn embed_hybrid_rotation_none_is_decimation_or_the_vote_at_0() {
    let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 4, DEFAULT);
    let read = hybrid::read(&image);
    assert_eq!(
        read,
        reader::read(&image),
        "the upright image is not read by §4's searches"
    );
    let Outcome::Recovered(plain) = read else {
        panic!("the payload was not recovered");
    };
    assert_eq!(
        plain.record.payload, payload,
        "the payload was not recovered"
    );
    assert_eq!(
        (plain.how.transform.decimate, plain.how.transform.rotation),
        (5, None),
        "the report does not say how it was recovered"
    );
    let mut noisy = image.clone();
    flip_low_bits(&mut noisy, 0.05, 40);
    check_rotated(
        &hybrid::read(&noisy),
        &payload,
        Dihedral::Identity,
        undone(0),
        (1, 1),
    );
}

negative_control!(
    embed_hybrid_rotation_none_is_decimation_or_the_vote_at_0,
    "the noisy image is not recovered by §4's three searches alone",
    expected = "the payload was not recovered",
    {
        let (mut image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 4, DEFAULT);
        flip_low_bits(&mut image, 0.05, 40);
        check_rotated(
            &reader::read(&image),
            &payload,
            Dihedral::Identity,
            undone(0),
            (1, 1),
        )
    }
);

/// `image` translated `left` pixels right and `top` down onto a larger canvas, the new strips pseudo-random from `seed`.
fn pad(image: &Image, left: u32, top: u32, seed: u32) -> Image {
    let mut out = canvas(image.width() + left, image.height() + top, seed);
    let (from_row, to_row) = (image.width() as usize * 4, out.width() as usize * 4);
    for y in 0..image.height() as usize {
        let to = (y + top as usize) * to_row + left as usize * 4;
        out.pixels_mut()[to..to + from_row]
            .copy_from_slice(&image.pixels()[y * from_row..(y + 1) * from_row]);
    }
    out
}

/// Sets every low bit of the pixels in column class `column` (`x mod 5`) or row class `row` (`y mod 5`) to 1, each
/// such pixel with probability 0.7, drawn from `seed`: set bits, not changes, concentrated inside the blocks.
fn stripe(image: &mut Image, column: u32, row: u32, seed: u32) {
    let width = image.width();
    let draws = noise(image.pixels().len() / 4, seed);
    for (at, (pixel, draw)) in image
        .pixels_mut()
        .chunks_exact_mut(4)
        .zip(draws)
        .enumerate()
    {
        let (x, y) = (at as u32 % width, at as u32 / width);
        if (x % 5 == column || y % 5 == row) && f64::from(draw) < 0.7 * 256.0 {
            for byte in pixel {
                *byte |= 1;
            }
        }
    }
}

/// A 256² hybrid image at `k = 25`, translated by (2, 1) so its blocks start at column phase 2 and row phase 1, with
/// the low bits of column class 4 and row class 3, two pixels inside each block, set to 1 at random (`stripe`), under
/// 2 % uniform noise so that §4's searches alone read nothing.
fn striped() -> (Image, Vec<u8>) {
    let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 12, Redundancy::K25);
    let mut image = pad(&image, 2, 1, 12);
    stripe(&mut image, 4, 3, 120);
    flip_low_bits(&mut image, 0.02, 121);
    (image, payload)
}

/// The block phase is where the low bits *change* most often, not where they are most often set (§5, "The hybrid's
/// layout"). In `striped`, the striped classes and the class after each hold more set bits from one pixel to the next
/// than the blocks' edges (0.85 a bit against 0.75) but fewer changes (0.35 against 0.5), so a phase found from set
/// bits lands two or three pixels inside the blocks on both axes, and a vote over blocks that straddle four of the
/// writer's loses the record. The changes find the blocks at their non-zero phase, and the vote at 0° recovers it.
#[test]
fn embed_hybrid_rotation_block_phase_counts_changes() {
    let (image, payload) = striped();
    check_rotated(
        &hybrid::read(&image),
        &payload,
        Dihedral::Identity,
        undone(0),
        (1, 1),
    );
}

negative_control!(
    embed_hybrid_rotation_block_phase_counts_changes,
    "the striped, noisy image is not recovered by §4's three searches alone",
    expected = "the payload was not recovered",
    {
        let (image, payload) = striped();
        check_rotated(
            &reader::read(&image),
            &payload,
            Dihedral::Identity,
            undone(0),
            (1, 1),
        )
    }
);

// --- The tiled variant stays the default (REQ-TOOL-064) -------------------------------------------------------------

/// The variant `outcome`'s record carries.
fn variant(outcome: &Outcome) -> Variant {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("the payload was not recovered: {outcome:?}");
    };
    recovered.record.flags.variant
}

/// The writer that takes no variant is the tiled one (§5, "Default: tiled"); the hybrid variant is written only when
/// asked for, and its records say so.
#[test]
fn embed_hybrid_rotation_tiled_stays_the_default() {
    let payload = noise(CONFIG_PAYLOAD, 5);
    let mut tiled = canvas(256, 256, 5);
    writer::embed(&mut tiled, &payload, false).expect("the payload embeds");
    assert_eq!(
        variant(&hybrid::read(&tiled)),
        Variant::Tiled,
        "the default writer is not the tiled variant"
    );
    let (image, _) = embedded(256, 256, CONFIG_PAYLOAD, 5, DEFAULT);
    assert_eq!(
        variant(&hybrid::read(&image)),
        Variant::Hybrid,
        "the hybrid writer's records do not say so"
    );
}

negative_control!(
    embed_hybrid_rotation_tiled_stays_the_default,
    "a hybrid image's records are not the tiled variant's",
    expected = "the default writer is not the tiled variant",
    {
        let (image, _) = embedded(256, 256, CONFIG_PAYLOAD, 5, DEFAULT);
        assert_eq!(
            variant(&hybrid::read(&image)),
            Variant::Tiled,
            "the default writer is not the tiled variant"
        );
    }
);

// --- The layout -----------------------------------------------------------------------------------------------------

/// Every pixel of `image` against `before`: each block of the hybrid grid at `redundancy` that a record's bit lands in
/// holds that bit in all `k` of its pixels' slot, every other low bit is `before`'s, and no channel moved by more than
/// 1. The record is the one `payload` gives with `n_records` RGB tiles.
fn check_layout(
    before: &Image,
    image: &Image,
    payload: &[u8],
    redundancy: Redundancy,
    n_records: u16,
) {
    let bytes = encode(&Record {
        flags: Flags {
            variant: Variant::Hybrid,
            source: false,
        },
        n_records,
        payload: payload.to_vec(),
    })
    .expect("the record encodes");
    let b = redundancy.block();
    let mut expected = before.clone();
    for (plane, side) in [(Plane::Rgb, 33), (Plane::Alpha, 57)] {
        let bpp = plane.bits_per_pixel();
        let (cols, rows) = (image.width() / b / side, image.height() / b / side);
        let limit = if plane == Plane::Rgb {
            usize::from(n_records)
        } else {
            usize::MAX
        };
        for (row, col) in (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .take(limit)
        {
            for i in 0..8 * bytes.len() as u64 {
                let slot = bit_slot(i, side, bpp);
                let bit = u8::from(record_bit(&bytes, i));
                for (dx, dy) in (0..b).flat_map(|dy| (0..b).map(move |dx| (dx, dy))) {
                    let x = (col * side + slot.x) * b + dx;
                    let y = (row * side + slot.y) * b + dy;
                    expected.set_low(plane, x, y, slot.channel, bit);
                }
            }
        }
    }
    for (i, (want, got)) in expected.pixels().iter().zip(image.pixels()).enumerate() {
        assert_eq!(want, got, "byte {i} is not the hybrid layout's");
    }
    let delta = before
        .pixels()
        .iter()
        .zip(image.pixels())
        .map(|(a, b)| a.abs_diff(*b))
        .max();
    assert!(delta <= Some(1), "a channel moved by more than 1");
}

/// The hybrid writer's layout at both redundancies: §2's tiles, sides 33 (RGB) and 57 (alpha) for §7's config
/// payload, laid in `b` × `b` blocks, `k = b²` copies of every bit. At 512² the RGB grid holds 3 × 3 tiles of
/// 5-pixel blocks and 5 × 5 of 3-pixel ones; the alpha grid 1 and 2 × 2.
#[test]
fn embed_hybrid_rotation_layout_is_k_copies_of_each_bit() {
    for (redundancy, rgb, alpha) in [(Redundancy::K25, 9, 1), (Redundancy::K9, 25, 4)] {
        let before = canvas(512, 512, 6);
        let payload = noise(CONFIG_PAYLOAD, 6);
        let mut image = before.clone();
        let placed =
            hybrid::embed(&mut image, &payload, false, redundancy).expect("the payload embeds");
        assert_eq!(
            placed,
            Placed { rgb, alpha },
            "the writer did not place the grid's tiles"
        );
        check_layout(&before, &image, &payload, redundancy, rgb);
    }
}

negative_control!(
    embed_hybrid_rotation_layout_is_k_copies_of_each_bit,
    "the tiled writer's layout is not the hybrid's",
    expected = "is not the hybrid layout's",
    {
        let before = canvas(512, 512, 6);
        let payload = noise(CONFIG_PAYLOAD, 6);
        let mut image = before.clone();
        writer::embed(&mut image, &payload, false).expect("the payload embeds");
        check_layout(&before, &image, &payload, Redundancy::K25, 9);
    }
);

// --- The default k's evidence (REQ-TOOL-111, R-71) ------------------------------------------------------------------

/// The RGB tiles the hybrid writer places for a `len`-byte payload at each of §7's resolutions, 64² to 1024²: 0 where
/// the image is too small for one.
fn tiles_placed(len: usize, redundancy: Redundancy) -> [u16; 5] {
    [64, 128, 256, 512, 1024].map(|size| {
        let mut image = Image::new(size, size, false, vec![0; (size * size * 3) as usize]);
        match hybrid::embed(&mut image, &vec![0; len], false, redundancy) {
            Ok(placed) => placed.rgb,
            Err(EmbedError::TooSmall { .. }) => 0,
            Err(other) => panic!("the payload did not embed: {other:?}"),
        }
    })
}

/// `placed` gives §7's capacity table under the hybrid variant at `k = 9` and `k = 25`: the RGB tiles at 64² to 1024²
/// for §7's config payload and for the config with the full shader.
fn check_capacity(placed: impl Fn(usize, Redundancy) -> [u16; 5]) {
    assert_eq!(
        (
            placed(CONFIG_PAYLOAD, Redundancy::K9),
            placed(SHADER_PAYLOAD, Redundancy::K9)
        ),
        ([0, 1, 4, 25, 100], [0, 0, 0, 1, 9]),
        "the capacity at k = 9 is not the one measured"
    );
    assert_eq!(
        (
            placed(CONFIG_PAYLOAD, Redundancy::K25),
            placed(SHADER_PAYLOAD, Redundancy::K25)
        ),
        ([0, 0, 1, 9, 36], [0, 0, 0, 1, 4]),
        "the capacity at k = 25 is not the one measured"
    );
}

/// The capacity cost of `k` (§7's table, under the hybrid variant): each bit takes `k` pixels' slots, so a tile's
/// side grows `b`-fold and the image holds about `k` times fewer. §7's tiled table is 1, 9, 49, 225, 961 tiles
/// (config only) and 0, 1, 4, 25, 100 (config + full shader).
#[test]
fn embed_hybrid_rotation_capacity_cost_of_k() {
    check_capacity(tiles_placed);
}

negative_control!(
    embed_hybrid_rotation_capacity_cost_of_k,
    "the tiled writer's capacity, §7's own table, is not the hybrid's",
    expected = "the capacity at k = 9 is not the one measured",
    {
        check_capacity(|len, _| {
            [64, 128, 256, 512, 1024].map(|size| {
                let mut image = Image::new(size, size, false, vec![0; (size * size * 3) as usize]);
                writer::embed(&mut image, &vec![0; len], false).map_or(0, |placed| placed.rgb)
            })
        })
    }
);

/// The image too small for one tile at the default `k` is refused and left untouched: 128² holds 25 blocks of 5 px a
/// side, fewer than the 33 a tile needs, so the tile is 165 px.
#[test]
fn embed_hybrid_rotation_too_small_is_refused() {
    let before = canvas(128, 128, 7);
    let mut image = before.clone();
    let result = hybrid::embed(&mut image, &noise(CONFIG_PAYLOAD, 7), false, DEFAULT);
    assert_eq!(
        result,
        Err(EmbedError::TooSmall {
            side: 165,
            width: 128,
            height: 128
        }),
        "the image was not refused"
    );
    assert_eq!(image, before, "the refused image was changed");
}

negative_control!(
    embed_hybrid_rotation_too_small_is_refused,
    "at k = 9 the 128² image holds a tile",
    expected = "the image was not refused",
    {
        let mut image = canvas(128, 128, 7);
        let result = hybrid::embed(&mut image, &noise(CONFIG_PAYLOAD, 7), false, Redundancy::K9);
        assert_eq!(
            result,
            Err(EmbedError::TooSmall {
                side: 99,
                width: 128,
                height: 128
            }),
            "the image was not refused"
        );
    }
);

/// The RGB records found when `image`, holding a hybrid record at `redundancy`, is read after `damage`; `None` when
/// the payload is lost.
fn recovered_tiles(image: &Image, payload: &[u8]) -> Option<u32> {
    match hybrid::read(image) {
        Outcome::Recovered(recovered) => {
            assert_eq!(
                recovered.record.payload, payload,
                "a wrong payload was recovered"
            );
            Some(recovered.how.tiles.found)
        }
        _ => None,
    }
}

/// §5's measurement under uniform noise, at 512²: `k = 9` holds at 5 % and is lost at 15 %; `k = 25` holds at 15 %.
/// The counts are the RGB records still intact of the grid's 25 and 9.
#[test]
fn embed_hybrid_rotation_uniform_noise_by_k() {
    let noisy = |redundancy, p| {
        let (mut image, payload) = embedded(512, 512, CONFIG_PAYLOAD, 8, redundancy);
        flip_low_bits(&mut image, p, 80);
        recovered_tiles(&image, &payload)
    };
    assert_eq!(
        [
            noisy(Redundancy::K9, 0.05),
            noisy(Redundancy::K9, 0.15),
            noisy(Redundancy::K25, 0.15)
        ],
        [Some(24), None, Some(8)],
        "the records left intact under uniform noise are not the ones measured"
    );
}

negative_control!(
    embed_hybrid_rotation_uniform_noise_by_k,
    "k = 25 at 25 % noise is lost",
    expected = "the records left intact under uniform noise are not the ones measured",
    {
        let (mut image, payload) = embedded(512, 512, CONFIG_PAYLOAD, 8, Redundancy::K25);
        flip_low_bits(&mut image, 0.25, 80);
        assert_eq!(
            [recovered_tiles(&image, &payload)],
            [Some(9)],
            "the records left intact under uniform noise are not the ones measured"
        );
    }
);

/// The RGB records recovered from a 256² image holding §7's config payload at `redundancy`, rotated `degrees`.
fn rotated_tiles(redundancy: Redundancy, degrees: f64) -> Option<u32> {
    let (image, payload) = embedded(256, 256, CONFIG_PAYLOAD, 9, redundancy);
    recovered_tiles(&rotate(&image, degrees, true), &payload)
}

/// `k = 9` loses the payload after `degrees` of rotation.
fn check_k9_lost(degrees: f64) {
    assert_eq!(
        rotated_tiles(Redundancy::K9, degrees),
        None,
        "k = 9 recovered after {degrees}° of rotation"
    );
}

/// §5's reason for the hybrid: nearest rotation leaves bit errors whole-record repetition cannot absorb. At 256²,
/// `k = 25` recovers its one record after each of §7's angles; `k = 9` loses all four of its records after 7°.
#[test]
fn embed_hybrid_rotation_by_k() {
    assert_eq!(
        ANGLES.map(|degrees| rotated_tiles(Redundancy::K25, degrees)),
        [Some(1); 5],
        "the records recovered after rotation at k = 25 are not the ones measured"
    );
    check_k9_lost(7.0);
}

negative_control!(
    embed_hybrid_rotation_by_k,
    "k = 9 recovers after 34° of rotation",
    expected = "k = 9 recovered after 34° of rotation",
    { check_k9_lost(34.0) }
);

// --- The three outcomes under the angle search (§9) -----------------------------------------------------------------

/// A plain canvas, upright and rotated, holds no state under either reader: the angle search's views count only an
/// intact header as state present, so its 90 angles and two redundancies do not raise the false-present rate. Images
/// too small for any block's tile, the empty one included, read as no state too.
#[test]
fn embed_hybrid_rotation_plain_image_is_none() {
    let plain = canvas(256, 256, 10);
    let tiny = [canvas(2, 2, 10), canvas(1, 7, 10), canvas(0, 0, 10)];
    for image in [plain.clone(), rotate(&plain, 34.0, true)]
        .iter()
        .chain(&tiny)
    {
        assert_eq!(
            hybrid::read(image),
            Outcome::None,
            "the canvas reads as state present"
        );
    }
}

negative_control!(
    embed_hybrid_rotation_plain_image_is_none,
    "a rotated hybrid image is not a plain canvas",
    expected = "the canvas reads as state present",
    {
        let (image, _) = embedded(256, 256, CONFIG_PAYLOAD, 10, DEFAULT);
        assert_eq!(
            hybrid::read(&rotate(&image, 34.0, true)),
            Outcome::None,
            "the canvas reads as state present"
        );
    }
);

/// A rotated hybrid image whose every record has its payload damaged, its header intact, is state present but
/// corrupt: the angle search finds the intact header and no intact record.
#[test]
fn embed_hybrid_rotation_damaged_records_are_corrupt() {
    let (mut image, _) = embedded(256, 256, CONFIG_PAYLOAD, 11, DEFAULT);
    // Byte 100 of the record, the payload's, lies in block row ⌊800 / 3 / 33⌋ = 8 of the tile: its pixel rows 40 to
    // 44. Every low bit of those rows set flips some of the payload's bits in every plane.
    let row = 4 * 256;
    for byte in &mut image.pixels_mut()[40 * row..45 * row] {
        *byte ^= 1;
    }
    assert_eq!(
        hybrid::read(&rotate(&image, 34.0, true)),
        Outcome::Corrupt(Corrupt {
            intact: 0,
            reason: CorruptReason::NoIntactRecord
        }),
        "the damaged records do not read as state present, corrupt"
    );
}

negative_control!(
    embed_hybrid_rotation_damaged_records_are_corrupt,
    "an undamaged rotated hybrid image is recovered",
    expected = "the damaged records do not read as state present, corrupt",
    {
        let (image, _) = embedded(256, 256, CONFIG_PAYLOAD, 11, DEFAULT);
        assert_eq!(
            hybrid::read(&rotate(&image, 34.0, true)),
            Outcome::Corrupt(Corrupt {
                intact: 0,
                reason: CorruptReason::NoIntactRecord
            }),
            "the damaged records do not read as state present, corrupt"
        );
    }
);

// --- The default k's evidence, measured (REQ-TOOL-111, R-71) --------------------------------------------------------

/// Half a degree from the nearest whole degree, the worst residual the whole-degree angle search leaves.
const HALF_DEGREES: [f64; 6] = [7.5, 12.5, 22.5, 34.5, 44.5, 60.5];

/// The RGB records recovered from a `size`² canvas `seed` holding §7's config payload at `redundancy`, rotated
/// `degrees` with the canvas expanded or, without `expand`, the corners cut; `None` when the payload is lost.
fn recovered_after(
    redundancy: Redundancy,
    size: u32,
    seed: u32,
    degrees: f64,
    expand: bool,
) -> Option<u32> {
    let (image, payload) = embedded(size, size, CONFIG_PAYLOAD, seed, redundancy);
    recovered_tiles(&rotate(&image, degrees, expand), &payload)
}

/// The payload on a `size`² canvas `seed` at `redundancy` is recovered after `degrees` of rotation.
fn check_recovered(redundancy: Redundancy, size: u32, seed: u32, degrees: f64) {
    assert!(
        recovered_after(redundancy, size, seed, degrees, true).is_some(),
        "the payload was lost after {degrees}° at {size}², k = {}, canvas {seed}",
        redundancy.k()
    );
}

/// `outcomes` as a table cell: the records recovered on each canvas, or "lost".
fn cell(outcomes: impl IntoIterator<Item = Option<u32>>) -> String {
    outcomes
        .into_iter()
        .map(|found| found.map_or("lost".to_owned(), |n| n.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `image` rotated back clockwise by `degrees`, as the reader's angle search does: onto a canvas
/// `⌈w cos θ + h sin θ⌉` × `⌈w sin θ + h cos θ⌉` centred on the image's centre, each pixel taking the image's pixel
/// under its centre rotated counter-clockwise, the pixels outside the image 0.
fn rotate_back(image: &Image, degrees: f64) -> Image {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (w, h) = (f64::from(image.width()), f64::from(image.height()));
    let (out_w, out_h) = ((w * cos + h * sin).ceil(), (w * sin + h * cos).ceil());
    let channels = if image.has_alpha() { 4 } else { 3 };
    let mut pixels = vec![0u8; (out_w * out_h) as usize * channels];
    for y in 0..out_h as usize {
        for x in 0..out_w as usize {
            let (dx, dy) = (x as f64 + 0.5 - out_w / 2.0, y as f64 + 0.5 - out_h / 2.0);
            let sx = (dx * cos + dy * sin + w / 2.0).floor();
            let sy = (-dx * sin + dy * cos + h / 2.0).floor();
            if sx >= 0.0 && sy >= 0.0 && sx < w && sy < h {
                let from = (sy as usize * image.width() as usize + sx as usize) * channels;
                let to = (y * out_w as usize + x) * channels;
                pixels[to..to + channels].copy_from_slice(&image.pixels()[from..from + channels]);
            }
        }
    }
    Image::new(out_w as u32, out_h as u32, image.has_alpha(), pixels)
}

/// The fraction of `original`'s R, G and B low bits that `back`, `original` rotated and rotated back, holds wrong at
/// the same place relative to the two images' centres, over the pixels of `original` that `back` covers.
fn bit_error(original: &Image, back: &Image) -> f64 {
    let (size_w, size_h) = (f64::from(original.width()), f64::from(original.height()));
    let (back_w, back_h) = (f64::from(back.width()), f64::from(back.height()));
    let (mut wrong, mut bits) = (0u64, 0u64);
    for v in 0..back.height() {
        for u in 0..back.width() {
            let x = (f64::from(u) + 0.5 - back_w / 2.0 + size_w / 2.0).floor();
            let y = (f64::from(v) + 0.5 - back_h / 2.0 + size_h / 2.0).floor();
            if x < 0.0 || y < 0.0 || x >= size_w || y >= size_h {
                continue;
            }
            let at = (y as usize * original.width() as usize + x as usize) * 4;
            let to = (v as usize * back.width() as usize + u as usize) * 4;
            for c in 0..3 {
                wrong += u64::from((original.pixels()[at + c] ^ back.pixels()[to + c]) & 1);
                bits += 1;
            }
        }
    }
    wrong as f64 / bits as f64
}

/// The measurements behind the proposed default `k = 25` (REQ-TOOL-111, R-71), printed as the tables the proposal
/// gives, from fixed canvases: uniform noise by `k` at 512²; the raw low-bit error left by nearest rotation and
/// rotation back by the nearest whole degree, at 512²; rotation by `k` at 512² and 256², the canvas expanded and the
/// corners cut; the half-degree residuals at 512² and 1024²; and the angle search's time. It asserts what the
/// proposal rests on: `k = 25` recovers after every half-degree residual. Release only, ignored by default:
/// `cargo test -p render --release --test embed_hybrid -- --ignored --nocapture embed_hybrid_default_k_evidence`.
#[test]
#[ignore = "the R-71 evidence harness, minutes in release: run it with --ignored"]
fn embed_hybrid_default_k_evidence() {
    println!("uniform low-bit noise, 512², canvases 0, 1, 2 (noise seeds 100, 101, 102): RGB records intact");
    for redundancy in [Redundancy::K9, Redundancy::K25] {
        for p in [0.02, 0.05, 0.08, 0.10, 0.12, 0.15, 0.18, 0.20, 0.25] {
            let found = (0..3).map(|seed| {
                let (mut image, payload) = embedded(512, 512, CONFIG_PAYLOAD, seed, redundancy);
                flip_low_bits(&mut image, p, seed + 100);
                recovered_tiles(&image, &payload)
            });
            println!("  k = {}, p = {p}: {}", redundancy.k(), cell(found));
        }
    }

    println!("raw low-bit error after nearest rotation and back by the nearest whole degree, 512², canvas 77");
    let plain = canvas(512, 512, 77);
    for degrees in [7.0, 34.0, 45.0, 7.3, 34.7, 12.0, 20.0, 60.0, 83.0] {
        let back = rotate_back(&rotate(&plain, degrees, true), f64::round(degrees));
        println!(
            "  {degrees}°, {}° undone: {:.2} %",
            f64::round(degrees),
            100.0 * bit_error(&plain, &back)
        );
    }

    for (label, size, seeds, expand) in [
        ("512², expanded", 512, 0..4, true),
        ("256², expanded", 256, 0..3, true),
        ("512², corners cut", 512, 0..4, false),
    ] {
        println!("rotation by k, {label}, canvases {seeds:?}: RGB records recovered");
        for redundancy in [Redundancy::K25, Redundancy::K9] {
            for degrees in ANGLES {
                let found = seeds
                    .clone()
                    .map(|seed| recovered_after(redundancy, size, seed, degrees, expand));
                println!("  k = {}, {degrees}°: {}", redundancy.k(), cell(found));
            }
        }
    }

    for (size, seeds) in [(512, 0..4), (1024, 0..2)] {
        println!(
            "half-degree residuals, {size}², k = 25, canvases {seeds:?}: RGB records recovered"
        );
        for degrees in HALF_DEGREES {
            let found = seeds
                .clone()
                .map(|seed| recovered_after(DEFAULT, size, seed, degrees, true));
            println!("  {degrees}°: {}", cell(found));
            for seed in seeds.clone() {
                check_recovered(DEFAULT, size, seed, degrees);
            }
        }
    }

    println!("the angle search's time");
    for (size, degrees) in [(256, 34.0), (512, 34.0)] {
        let (image, _) = embedded(size, size, CONFIG_PAYLOAD, 0, DEFAULT);
        let rotated = rotate(&image, degrees, true);
        let start = std::time::Instant::now();
        let outcome = hybrid::read(&rotated);
        println!(
            "  {size}², {degrees}°: {:.2} s, {}",
            start.elapsed().as_secs_f64(),
            matches!(outcome, Outcome::Recovered(_))
        );
    }
    for size in [128, 512] {
        let plain = canvas(size, size, 9);
        let start = std::time::Instant::now();
        let outcome = hybrid::read(&plain);
        println!(
            "  {size}², no state: {:.2} s, {outcome:?}",
            start.elapsed().as_secs_f64()
        );
    }
}

negative_control!(
    embed_hybrid_default_k_evidence,
    "k = 9 loses the payload after 7° of rotation at 256²",
    expected = "the payload was lost after",
    { check_recovered(Redundancy::K9, 256, 0, 7.0) }
);
