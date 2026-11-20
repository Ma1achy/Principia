//! The tiled writer, the majority-vote reader and its three outcomes (`principia_dd_image_embedding.md` §2, §3, §4,
//! §9): REQ-TOOL-059, REQ-TOOL-061, REQ-TOOL-071. Each test registers its negative control (R-176).

use render::embed::reader::{read, Corrupt, CorruptReason, Count, How, Outcome, Transform};
use render::embed::record::{
    bit_slot, crc32, encode, read_tile, record_bits, tile_grid, tile_origin, tile_side, write_tile,
    Flags, Record, Variant, HEADER_LEN, VERSION,
};
use render::embed::writer::{embed, EmbedError, Placed};
use render::embed::{Image, Plane};
use validation::negative_control;

/// §7's measured config-only payload, deflated.
const CONFIG_PAYLOAD: usize = 382;

/// `len` payload bytes that are not all alike, varied by `seed`.
fn payload(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i * 37 + 11) as u8 ^ seed.wrapping_mul(101))
        .collect()
}

/// An opaque `width` × `height` image whose R, G and B are pseudo-random, as a rendered slice's pixels are not
/// regular; `alpha` adds an alpha channel at 255.
fn canvas(width: u32, height: u32, alpha: bool) -> Image {
    let mut state = 0x2545_F491_u32 ^ width.wrapping_mul(31) ^ height;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 24) as u8
    };
    let channels = if alpha { 4 } else { 3 };
    let pixels = (0..width * height * channels)
        .map(|i| if alpha && i % 4 == 3 { 255 } else { next() })
        .collect();
    Image::new(width, height, alpha, pixels)
}

/// A `width` × `height` RGBA canvas with `payload` embedded.
fn embedded(width: u32, height: u32, payload: &[u8]) -> Image {
    let mut image = canvas(width, height, true);
    embed(&mut image, payload, false).expect("the payload embeds");
    image
}

/// `plane`'s tile side for a payload of `len` bytes.
fn side(plane: Plane, len: usize) -> u32 {
    tile_side(record_bits(len), plane.bits_per_pixel())
}

/// `plane`'s grid tiles for a payload of `len` bytes, row by row from the top-left.
fn tiles(image: &Image, plane: Plane, len: usize) -> Vec<(u32, u32)> {
    let (cols, rows) = tile_grid(image.width(), image.height(), side(plane, len));
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (col, row)))
        .collect()
}

/// The record bytes the writer places for `payload` with `n_records`.
fn record_bytes(payload: &[u8], n_records: u16) -> Vec<u8> {
    encode(&Record {
        flags: Flags {
            variant: Variant::Tiled,
            source: false,
        },
        n_records,
        payload: payload.to_vec(),
    })
    .expect("the record encodes")
}

/// Writes `bytes` into tile `(col, row)` of `plane` at side `side`.
fn overwrite(image: &mut Image, plane: Plane, (col, row): (u32, u32), side: u32, bytes: &[u8]) {
    let mut lows = image.tile_lows(plane, col, row, side);
    write_tile(bytes, side, plane.bits_per_pixel(), &mut lows);
    image.set_tile_lows(plane, col, row, side, &lows);
}

/// Flips the low bit that holds bit `bit` of the record in tile `(col, row)` of `plane` at side `side`.
fn flip(image: &mut Image, plane: Plane, (col, row): (u32, u32), side: u32, bit: u64) {
    let mut lows = image.tile_lows(plane, col, row, side);
    let slot = bit_slot(bit, side, plane.bits_per_pixel());
    let at = ((slot.y * side + slot.x) * plane.bits_per_pixel() + slot.channel) as usize;
    lows[at] ^= 1;
    image.set_tile_lows(plane, col, row, side, &lows);
}

/// Sets every low bit of `plane` to 0, as a pipeline that clears the plane leaves it.
fn clear_plane(image: &mut Image, plane: Plane) {
    let alpha = image.has_alpha();
    for (i, byte) in image.pixels_mut().iter_mut().enumerate() {
        let is_alpha = alpha && i % 4 == 3;
        if is_alpha == (plane == Plane::Alpha) {
            *byte &= !1;
        }
    }
}

/// `outcome` recovered `payload`, with `tiles` and `alpha` its counts.
fn check_recovered(outcome: Outcome, payload: &[u8], tiles: Count, alpha: Option<Count>) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("the payload was not recovered: {outcome:?}");
    };
    assert_eq!(
        recovered.record.payload, payload,
        "the payload was not recovered"
    );
    assert_eq!(
        (recovered.how.tiles, recovered.how.alpha),
        (tiles, alpha),
        "the counts are not the records placed"
    );
}

// --- The tiled writer: RGB records, alpha a second systematic copy (REQ-TOOL-059) ----------------------------------

#[test]
fn embed_rgb_alpha_stripped_rgb_alone_recovers() {
    // §2's 128² (9 tiles) and §7's 512² (225/225 with alpha stripped).
    for (size, n) in [(128, 9), (512, 225)] {
        let data = payload(CONFIG_PAYLOAD, 1);
        let stripped = embedded(size, size, &data).without_alpha();
        check_recovered(read(&stripped), &data, Count { found: n, of: n }, None);
    }
}

negative_control!(
    embed_rgb_alpha_stripped_rgb_alone_recovers,
    "with alpha stripped and the RGB plane cleared, nothing is left to recover",
    expected = "the payload was not recovered",
    {
        let data = payload(CONFIG_PAYLOAD, 1);
        let mut stripped = embedded(128, 128, &data).without_alpha();
        clear_plane(&mut stripped, Plane::Rgb);
        check_recovered(read(&stripped), &data, Count { found: 9, of: 9 }, None)
    }
);

#[test]
fn embed_rgb_alpha_alpha_alone_recovers() {
    let data = payload(CONFIG_PAYLOAD, 2);
    let mut image = embedded(128, 128, &data);
    clear_plane(&mut image, Plane::Rgb);
    // §2: 4 alpha tiles at 128² for a 402 B record; the RGB plane's 9 are gone.
    check_recovered(
        read(&image),
        &data,
        Count { found: 0, of: 9 },
        Some(Count { found: 4, of: 4 }),
    );
}

negative_control!(
    embed_rgb_alpha_alpha_alone_recovers,
    "with the alpha plane cleared as well, nothing is left to recover",
    expected = "the payload was not recovered",
    {
        let data = payload(CONFIG_PAYLOAD, 2);
        let mut image = embedded(128, 128, &data);
        clear_plane(&mut image, Plane::Rgb);
        clear_plane(&mut image, Plane::Alpha);
        check_recovered(
            read(&image),
            &data,
            Count { found: 0, of: 9 },
            Some(Count { found: 4, of: 4 }),
        )
    }
);

/// Every tile of `plane`'s grid at side `side` holds `bytes` whole, from its first slot.
fn check_tiles_hold(image: &Image, plane: Plane, side: u32, bytes: &[u8], count: usize) {
    let (cols, rows) = tile_grid(image.width(), image.height(), side);
    for row in 0..rows {
        for col in 0..cols {
            let read = read_tile(
                &image.tile_lows(plane, col, row, side),
                side,
                plane.bits_per_pixel(),
            );
            assert_eq!(
                read.get(..bytes.len()),
                Some(bytes),
                "tile ({col}, {row}) of the {plane:?} plane does not hold the record"
            );
        }
    }
    assert_eq!(
        (cols * rows) as usize,
        count,
        "the {plane:?} grid does not hold the count placed"
    );
}

#[test]
fn embed_rgb_alpha_tiles_hold_whole_records() {
    let data = payload(CONFIG_PAYLOAD, 3);
    let mut image = canvas(128, 128, true);
    let placed = embed(&mut image, &data, false).expect("the payload embeds");
    assert_eq!(placed, Placed { rgb: 9, alpha: 4 });
    let bytes = record_bytes(&data, 9);
    check_tiles_hold(&image, Plane::Rgb, 33, &bytes, 9);
    check_tiles_hold(&image, Plane::Alpha, 57, &bytes, 4);
}

negative_control!(
    embed_rgb_alpha_tiles_hold_whole_records,
    "reading the RGB grid at a side one short of §3's must fail",
    expected = "of the Rgb plane does not hold the record",
    {
        let data = payload(CONFIG_PAYLOAD, 3);
        let image = embedded(128, 128, &data);
        check_tiles_hold(&image, Plane::Rgb, 32, &record_bytes(&data, 9), 9)
    }
);

/// `after` differs from `before` in low bits only, and only in the slots of `plane`'s grid tiles that hold a record
/// bit: the strip beyond the last whole tile and each tile's slots after the record's last bit are untouched (§2).
fn check_low_bits_only(before: &Image, after: &Image, plane: Plane, len: usize) {
    for (b, a) in before.pixels().iter().zip(after.pixels()) {
        assert_eq!(b & !1, a & !1, "the writer changed more than the low bit");
    }
    let side = side(plane, len);
    let bits = record_bits(len) as usize;
    let in_record = |x: u32, y: u32, channel: u32| {
        let (col, row) = (x / side, y / side);
        let (cols, rows) = tile_grid(before.width(), before.height(), side);
        let (ox, oy) = tile_origin(col, row, side);
        let slot = ((y - oy) * side + (x - ox)) * plane.bits_per_pixel() + channel;
        col < cols && row < rows && (slot as usize) < bits
    };
    for y in 0..before.height() {
        for x in 0..before.width() {
            for channel in 0..plane.bits_per_pixel() {
                if !in_record(x, y, channel) {
                    assert_eq!(
                        before.low(plane, x, y, channel),
                        after.low(plane, x, y, channel),
                        "the writer wrote outside the records' slots"
                    );
                }
            }
        }
    }
}

#[test]
fn embed_rgb_alpha_changes_only_the_records_low_bits() {
    let data = payload(CONFIG_PAYLOAD, 4);
    let before = canvas(128, 128, true);
    let mut after = before.clone();
    embed(&mut after, &data, false).expect("the payload embeds");
    check_low_bits_only(&before, &after, Plane::Rgb, data.len());
    check_low_bits_only(&before, &after, Plane::Alpha, data.len());
}

negative_control!(
    embed_rgb_alpha_changes_only_the_records_low_bits,
    "a channel moved by 2 must fail the low-bit check",
    expected = "the writer changed more than the low bit",
    {
        let data = payload(CONFIG_PAYLOAD, 4);
        let before = canvas(128, 128, true);
        let mut after = before.clone();
        embed(&mut after, &data, false).expect("the payload embeds");
        after.pixels_mut()[5] ^= 2;
        check_low_bits_only(&before, &after, Plane::Rgb, data.len())
    }
);

/// `result` refuses a `width`² image too small for one RGB tile of side `side`, and leaves it unchanged.
fn check_refused(result: Result<Placed, EmbedError>, before: &Image, after: &Image, side: u32) {
    let width = before.width();
    assert_eq!(
        result,
        Err(EmbedError::TooSmall {
            side,
            width,
            height: width
        }),
        "an image smaller than one tile was not refused"
    );
    assert_eq!(before, after, "a refused image was changed");
}

#[test]
fn embed_rgb_alpha_image_smaller_than_one_tile_is_refused() {
    let data = payload(CONFIG_PAYLOAD, 5);
    let before = canvas(32, 32, true);
    let mut after = before.clone();
    let result = embed(&mut after, &data, false);
    check_refused(result, &before, &after, 33);
}

negative_control!(
    embed_rgb_alpha_image_smaller_than_one_tile_is_refused,
    "a 33² image holds one tile and must not be refused",
    expected = "an image smaller than one tile was not refused",
    {
        let data = payload(CONFIG_PAYLOAD, 5);
        let before = canvas(33, 33, true);
        let mut after = before.clone();
        let result = embed(&mut after, &data, false);
        check_refused(result, &before, &after, 33)
    }
);

// --- Majority vote per byte (REQ-TOOL-061) -------------------------------------------------------------------------

/// A 128² image holding `data` in its 9 RGB and 4 alpha tiles, with its first `rgb` RGB tiles and first `alpha` alpha
/// tiles (each plane row by row) overwritten by an intact record of the same length whose payload differs in
/// consistent bytes.
fn forged_image(data: &[u8], rgb: usize, alpha: usize) -> Image {
    let mut image = embedded(128, 128, data);
    let mut wrong = data.to_vec();
    for i in [0, 17, 200, data.len() - 1] {
        wrong[i] ^= 0x5A;
    }
    let wrong = record_bytes(&wrong, 9);
    for (plane, forged) in [(Plane::Rgb, rgb), (Plane::Alpha, alpha)] {
        for tile in tiles(&image, plane, data.len()).into_iter().take(forged) {
            overwrite(&mut image, plane, tile, side(plane, data.len()), &wrong);
        }
    }
    image
}

/// `outcome` recovered `data`, the majority's payload, with `outvoted` intact records outvoted.
fn check_majority_wins(outcome: Outcome, data: &[u8], outvoted: u32) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("the majority did not win: {outcome:?}");
    };
    assert_eq!(recovered.record.payload, data, "the majority did not win");
    assert_eq!(
        recovered.how.outvoted, outvoted,
        "the outvoted records are not counted"
    );
}

#[test]
fn embed_majority_vote_majority_wins() {
    let data = payload(CONFIG_PAYLOAD, 6);
    // 6 of 13 intact records carry the forged bytes: 5 RGB tiles and 1 alpha tile.
    let outcome = read(&forged_image(&data, 5, 1));
    let Outcome::Recovered(recovered) = &outcome else {
        panic!("the majority did not win: {outcome:?}");
    };
    assert_eq!(recovered.how.tiles, Count { found: 4, of: 9 });
    assert_eq!(recovered.how.alpha, Some(Count { found: 3, of: 4 }));
    check_majority_wins(outcome, &data, 6);
}

negative_control!(
    embed_majority_vote_majority_wins,
    "7 forged records of 13, 5 RGB and 2 alpha, are the majority, and the original must lose",
    expected = "the majority did not win",
    {
        let data = payload(CONFIG_PAYLOAD, 6);
        check_majority_wins(read(&forged_image(&data, 5, 2)), &data, 6)
    }
);

/// A 66 × 33 RGB image, two RGB tiles of a 402 B record, the second overwritten by a forged intact record when
/// `forge` is set.
fn two_record_image(data: &[u8], forge: bool) -> Image {
    let mut image = canvas(66, 33, false);
    embed(&mut image, data, false).expect("the payload embeds");
    if forge {
        let mut wrong = data.to_vec();
        wrong[3] ^= 0xFF;
        overwrite(&mut image, Plane::Rgb, (1, 0), 33, &record_bytes(&wrong, 2));
    }
    image
}

/// `outcome` is corrupt for want of a majority among `intact` intact records.
fn check_tie_is_corrupt(outcome: Outcome, intact: u32) {
    assert_eq!(
        outcome,
        Outcome::Corrupt(Corrupt {
            intact,
            reason: CorruptReason::NoMajority
        }),
        "a tie was not reported corrupt"
    );
}

#[test]
fn embed_majority_vote_tie_is_corrupt() {
    let data = payload(CONFIG_PAYLOAD, 7);
    check_tie_is_corrupt(read(&two_record_image(&data, true)), 2);
}

negative_control!(
    embed_majority_vote_tie_is_corrupt,
    "two agreeing records are a majority, not a tie",
    expected = "a tie was not reported corrupt",
    {
        let data = payload(CONFIG_PAYLOAD, 7);
        check_tie_is_corrupt(read(&two_record_image(&data, false)), 2)
    }
);

// --- The three outcomes (REQ-TOOL-071) -----------------------------------------------------------------------------

/// `outcome` is "no embedded state".
fn check_none(outcome: Outcome) {
    assert_eq!(
        outcome,
        Outcome::None,
        "a plain image did not read as no embedded state"
    );
}

#[test]
fn embed_outcomes_plain_image_is_none() {
    check_none(read(&canvas(128, 128, true)));
    check_none(read(&canvas(64, 96, false)));
    for value in [0, 255] {
        check_none(read(&Image::new(64, 64, true, vec![value; 64 * 64 * 4])));
    }
}

negative_control!(
    embed_outcomes_plain_image_is_none,
    "an image with a payload embedded is not plain",
    expected = "a plain image did not read as no embedded state",
    check_none(read(&embedded(128, 128, &payload(CONFIG_PAYLOAD, 8))))
);

/// A 128² image holding `data`, with one bit flipped in every record of both planes but the last `spared` alpha
/// tiles: a payload bit in even tiles and an `n_records` bit, which fails `crc32(header)`, in odd ones.
fn all_corrupted(data: &[u8], spared: usize) -> Image {
    let mut image = embedded(128, 128, data);
    let mut k = 0;
    for plane in [Plane::Rgb, Plane::Alpha] {
        let mut plane_tiles = tiles(&image, plane, data.len());
        if plane == Plane::Alpha {
            plane_tiles.truncate(plane_tiles.len() - spared);
        }
        for tile in plane_tiles {
            let bit = if k % 2 == 0 {
                8 * (HEADER_LEN as u64 + 10)
            } else {
                8 * 11
            };
            flip(&mut image, plane, tile, side(plane, data.len()), bit);
            k += 1;
        }
    }
    image
}

/// `outcome` is "state present, corrupt" with no record intact.
fn check_corrupt(outcome: Outcome) {
    assert_eq!(
        outcome,
        Outcome::Corrupt(Corrupt {
            intact: 0,
            reason: CorruptReason::NoIntactRecord
        }),
        "corrupted state did not read as present but corrupt"
    );
}

#[test]
fn embed_outcomes_all_records_corrupted_is_corrupt() {
    check_corrupt(read(&all_corrupted(&payload(CONFIG_PAYLOAD, 9), 0)));
}

negative_control!(
    embed_outcomes_all_records_corrupted_is_corrupt,
    "one alpha record spared is recovered, not corrupt",
    expected = "corrupted state did not read as present but corrupt",
    check_corrupt(read(&all_corrupted(&payload(CONFIG_PAYLOAD, 9), 1)))
);

/// `outcome` recovered `data` at the current version, tiled with the `source` bit set, and reports `how`.
fn check_report(outcome: Outcome, data: &[u8], how: How) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("a valid image was not recovered: {outcome:?}");
    };
    assert_eq!(recovered.version, VERSION);
    assert_eq!(
        recovered.record,
        Record {
            flags: Flags {
                variant: Variant::Tiled,
                source: true
            },
            n_records: 9,
            payload: data.to_vec(),
        }
    );
    assert_eq!(
        recovered.how, how,
        "the report does not say how it was recovered"
    );
}

/// §4's report for a pristine 128² image: no transform, 9/9 RGB tiles, 4/4 alpha tiles, nothing outvoted.
const PRISTINE_128: How = How {
    transform: Transform::IDENTITY,
    tiles: Count { found: 9, of: 9 },
    alpha: Some(Count { found: 4, of: 4 }),
    outvoted: 0,
};

#[test]
fn embed_outcomes_valid_is_recovered_with_report() {
    let data = payload(CONFIG_PAYLOAD, 10);
    let mut image = canvas(128, 128, true);
    embed(&mut image, &data, true).expect("the payload embeds");
    check_report(read(&image), &data, PRISTINE_128);
}

negative_control!(
    embed_outcomes_valid_is_recovered_with_report,
    "an image with one RGB record lost reports 8/9, not a pristine 9/9",
    expected = "the report does not say how it was recovered",
    {
        let data = payload(CONFIG_PAYLOAD, 10);
        let mut image = canvas(128, 128, true);
        embed(&mut image, &data, true).expect("the payload embeds");
        flip(&mut image, Plane::Rgb, (2, 1), 33, 8 * 40);
        check_report(read(&image), &data, PRISTINE_128)
    }
);

/// A 128² image whose every record is intact at version 4, another layout's; with `stale_crc`, its header CRC is
/// left as the version-3 header's, so no record is intact.
fn other_version_image(data: &[u8], stale_crc: bool) -> Image {
    let mut image = canvas(128, 128, true);
    let mut bytes = record_bytes(data, 9);
    bytes[4] = VERSION + 1;
    if !stale_crc {
        let crc = crc32(&bytes[..12]);
        bytes[12..16].copy_from_slice(&crc.to_be_bytes());
    }
    for plane in [Plane::Rgb, Plane::Alpha] {
        for tile in tiles(&image, plane, data.len()) {
            overwrite(&mut image, plane, tile, side(plane, data.len()), &bytes);
        }
    }
    image
}

/// `outcome` recovered `data` and reports `version`, never corrupt (§2: "an intact record of another layout is
/// reported as such").
fn check_other_version(outcome: Outcome, data: &[u8], version: u8) {
    let Outcome::Recovered(recovered) = outcome else {
        panic!("an intact record of another layout was not recovered: {outcome:?}");
    };
    assert_eq!(
        (recovered.version, recovered.record.payload.as_slice()),
        (version, data)
    );
}

#[test]
fn embed_outcomes_other_version_is_recovered_and_reported() {
    let data = payload(CONFIG_PAYLOAD, 11);
    check_other_version(read(&other_version_image(&data, false)), &data, VERSION + 1);
}

negative_control!(
    embed_outcomes_other_version_is_recovered_and_reported,
    "a version byte changed without its header CRC is corruption, not another layout",
    expected = "an intact record of another layout was not recovered",
    {
        let data = payload(CONFIG_PAYLOAD, 11);
        check_other_version(read(&other_version_image(&data, true)), &data, VERSION + 1)
    }
);
