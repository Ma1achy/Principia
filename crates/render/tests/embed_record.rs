//! The embedded record and its tile geometry (`principia_dd_image_embedding.md` §2, §3, §7): REQ-TOOL-060,
//! REQ-TOOL-062, REQ-TOOL-110, REQ-TOOL-118, and the magic's and version's checks for REQ-TOOL-109. Each test
//! registers its negative control (R-176).

use render::embed::record::{
    bit_slot, crc32, decode, encode, read_tile, record_bit, record_bits, record_len, tile_grid,
    tile_origin, tile_side, write_tile, Decoded, Discard, Flags, Record, Slot, Variant,
    ALPHA_BITS_PER_PIXEL, CRC_LEN, HEADER_FIELDS, HEADER_LEN, MAGIC, PROTOTYPE_MAGIC,
    PROTOTYPE_VERSION, RGB_BITS_PER_PIXEL, VERSION,
};
use validation::negative_control;

/// §7's measured config-only payload, deflated.
const CONFIG_PAYLOAD: usize = 382;
/// §7's measured full `frag.glsl`, deflated.
const SHADER_PAYLOAD: usize = 2_969;
/// The PNG file signature.
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// A record with a payload of `len` bytes that are not all alike.
fn record_with(len: usize) -> Record {
    Record {
        flags: Flags {
            variant: Variant::Tiled,
            source: false,
        },
        n_records: 9,
        payload: (0..len).map(|i| (i * 37 + 11) as u8).collect(),
    }
}

fn encoded(record: &Record) -> Vec<u8> {
    encode(record).expect("the record encodes")
}

/// The content `decode` finds in `bytes`, or why it discards them.
fn decoded_record(bytes: &[u8]) -> Result<Record, Discard> {
    decode(bytes).map(|d| d.record)
}

// --- Layout (§2) ---------------------------------------------------------------------------------------------------

/// The header's fields, in wire order and with their widths, are §2's, and the header is 16 bytes.
fn check_layout(fields: &[(&str, usize)], header_len: usize) {
    let section_2 = [
        ("magic", 4),
        ("version", 1),
        ("flags", 1),
        ("payload_len", 4),
        ("n_records", 2),
    ];
    assert_eq!(fields, section_2, "the header's fields are not §2's");
    assert_eq!(header_len, 16, "the header is not §2's 16 bytes");
}

#[test]
fn embed_record_layout_is_section_2() {
    check_layout(HEADER_FIELDS, HEADER_LEN);
}

negative_control!(
    embed_record_layout_is_section_2,
    "fields out of §2's order must fail the layout check",
    expected = "the header's fields are not §2's",
    check_layout(
        &[
            ("magic", 4),
            ("flags", 1),
            ("version", 1),
            ("payload_len", 4),
            ("n_records", 2)
        ],
        16
    )
);

/// `bytes` is the encoding of `record`, laid out byte for byte as §2 defines: big-endian integers, PNG's CRC.
fn check_wire_bytes(record: &Record, bytes: &[u8]) {
    let n = record.payload.len();
    let mut expected = MAGIC.to_vec();
    expected.push(VERSION);
    expected.push(record.flags.to_byte());
    expected.extend_from_slice(&[(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8]);
    expected.extend_from_slice(&[(record.n_records >> 8) as u8, record.n_records as u8]);
    let header_crc = crc32(&expected);
    expected.extend_from_slice(&header_crc.to_be_bytes());
    expected.extend_from_slice(&record.payload);
    expected.extend_from_slice(&crc32(&record.payload).to_be_bytes());
    assert_eq!(bytes, expected, "the record's bytes are not §2's layout");
}

#[test]
fn embed_record_wire_bytes_are_big_endian() {
    let mut record = record_with(0x0102);
    record.n_records = 0x0A0B;
    record.flags = Flags {
        variant: Variant::Hybrid,
        source: true,
    };
    let bytes = encoded(&record);
    check_wire_bytes(&record, &bytes);
    assert_eq!(&bytes[6..12], &[0x00, 0x00, 0x01, 0x02, 0x0A, 0x0B]);
}

negative_control!(
    embed_record_wire_bytes_are_big_endian,
    "a little-endian payload_len must fail the layout check",
    expected = "the record's bytes are not §2's layout",
    {
        let record = record_with(0x0102);
        let mut bytes = encoded(&record);
        bytes.swap(8, 9);
        check_wire_bytes(&record, &bytes)
    }
);

/// `crc32` is PNG's CRC-32: the check value of `123456789` is `0xCBF43926` (§2).
fn check_crc(crc: fn(&[u8]) -> u32) {
    assert_eq!(
        crc(b"123456789"),
        0xCBF4_3926,
        "the CRC is not PNG's CRC-32"
    );
}

#[test]
fn embed_record_crc_is_png_crc32() {
    check_crc(crc32);
}

negative_control!(
    embed_record_crc_is_png_crc32,
    "a CRC without the final XOR must fail the check value",
    expected = "the CRC is not PNG's CRC-32",
    check_crc(|bytes| !crc32(bytes))
);

// --- Trusted or discarded whole (REQ-TOOL-060) ---------------------------------------------------------------------

/// Each of `records` decodes to `expected`, except the one at `corrupt`, which is discarded.
fn check_only_corrupt_discarded(
    records: &[Vec<u8>],
    corrupt: usize,
    expected: &Record,
    what: &str,
) {
    for (k, bytes) in records.iter().enumerate() {
        let decoded = decoded_record(bytes);
        if k == corrupt {
            assert!(
                decoded.is_err(),
                "record {k} was not discarded after {what}"
            );
        } else {
            assert_eq!(
                decoded.as_ref(),
                Ok(expected),
                "intact record {k} did not decode after {what}"
            );
        }
    }
}

/// For every byte of a record and every nonzero XOR of it, the corrupted copy among nine is discarded whole and the
/// other eight decode.
#[test]
fn embed_record_crc_one_corrupt_byte_discards_only_its_record() {
    let record = record_with(CONFIG_PAYLOAD);
    let intact = encoded(&record);
    assert_eq!(intact.len(), record_len(CONFIG_PAYLOAD));
    let copies = 9;
    for at in 0..intact.len() {
        for xor in 1..=255u8 {
            let corrupt = at % copies;
            let mut records = vec![intact.clone(); copies];
            records[corrupt][at] ^= xor;
            check_only_corrupt_discarded(
                &records,
                corrupt,
                &record,
                &format!("byte {at} ^ {xor:#04x}"),
            );
        }
    }
}

negative_control!(
    embed_record_crc_one_corrupt_byte_discards_only_its_record,
    "an uncorrupted record named as the corrupt one must fail the discard check",
    expected = "was not discarded",
    {
        let record = record_with(CONFIG_PAYLOAD);
        let records = vec![encoded(&record); 9];
        check_only_corrupt_discarded(&records, 4, &record, "no corruption")
    }
);

/// `decode` reports `reason` for `bytes`.
fn check_discarded_for(bytes: &[u8], reason: Discard) {
    assert_eq!(
        decode(bytes).err(),
        Some(reason),
        "the record was not discarded for {reason:?}"
    );
}

/// Each way §2 names of failing is reported as itself.
#[test]
fn embed_record_crc_discard_reasons() {
    let intact = encoded(&record_with(CONFIG_PAYLOAD));
    check_discarded_for(&intact[..HEADER_LEN - 1], Discard::Truncated);
    check_discarded_for(&intact[..intact.len() - 1], Discard::Truncated);
    let mut magic = intact.clone();
    magic[0] ^= 1;
    check_discarded_for(&magic, Discard::Magic);
    // A header alone is long enough for the header's own checks, so its magic, not its length, is what fails.
    check_discarded_for(&magic[..HEADER_LEN], Discard::Magic);
    let mut header = intact.clone();
    header[10] ^= 1;
    check_discarded_for(&header, Discard::HeaderCrc);
    let mut payload = intact.clone();
    payload[HEADER_LEN] ^= 1;
    check_discarded_for(&payload, Discard::PayloadCrc);
    let mut tail = intact.clone();
    tail[intact.len() - 1] ^= 1;
    check_discarded_for(&tail, Discard::PayloadCrc);
}

negative_control!(
    embed_record_crc_discard_reasons,
    "an intact record must fail the discard check",
    expected = "the record was not discarded for PayloadCrc",
    check_discarded_for(&encoded(&record_with(CONFIG_PAYLOAD)), Discard::PayloadCrc)
);

/// A record with flags byte `flags`, its header CRC recomputed so only the flags can fail.
fn with_flags_byte(flags: u8) -> Vec<u8> {
    let mut bytes = encoded(&record_with(16));
    bytes[5] = flags;
    let crc = crc32(&bytes[..HEADER_LEN - CRC_LEN]);
    bytes[HEADER_LEN - CRC_LEN..HEADER_LEN].copy_from_slice(&crc.to_be_bytes());
    bytes
}

/// Each of the 256 flags bytes decodes exactly when it is a defined combination (§2, "Flag bits"): variant 0–2 in
/// bits 0–1, `source` in bit 2, bits 3–7 clear.
fn check_flags(defined: fn(u8) -> bool) {
    for byte in 0..=255u8 {
        let decoded = decode(&with_flags_byte(byte));
        if defined(byte) {
            let flags = decoded
                .unwrap_or_else(|d| panic!("defined flags {byte:#04x} were discarded: {d:?}"))
                .record
                .flags;
            assert_eq!(
                flags.to_byte(),
                byte,
                "flags {byte:#04x} did not round-trip"
            );
        } else {
            assert_eq!(
                decoded.err(),
                Some(Discard::Flags),
                "undefined flags {byte:#04x} were not discarded"
            );
        }
    }
}

#[test]
fn embed_record_flags_only_defined_bits_decode() {
    check_flags(|byte| byte & 0b1111_1000 == 0 && byte & 0b11 != 0b11);
    assert_eq!(
        Flags::from_byte(0b110),
        Some(Flags {
            variant: Variant::Hybrid,
            source: true
        })
    );
    assert_eq!(
        Flags::from_byte(0b001),
        Some(Flags {
            variant: Variant::Redundant,
            source: false
        })
    );
}

negative_control!(
    embed_record_flags_only_defined_bits_decode,
    "treating a reserved bit as defined must fail the flags check",
    expected = "were discarded",
    check_flags(|byte| byte & 0b1111_0000 == 0 && byte & 0b11 != 0b11)
);

/// `bytes`, with anything after it, decodes to `expected`.
fn check_decodes_to(bytes: &[u8], expected: &Record) {
    let mut padded = bytes.to_vec();
    padded.extend_from_slice(&[0xA5; 7]);
    assert_eq!(
        decoded_record(&padded).as_ref(),
        Ok(expected),
        "the record did not decode to what was encoded"
    );
}

#[test]
fn embed_record_round_trip_ignores_trailing_bytes() {
    for len in [0, 1, CONFIG_PAYLOAD, CONFIG_PAYLOAD + SHADER_PAYLOAD] {
        let record = record_with(len);
        check_decodes_to(&encoded(&record), &record);
    }
}

negative_control!(
    embed_record_round_trip_ignores_trailing_bytes,
    "another record's bytes must fail the round trip",
    expected = "did not decode to what was encoded",
    check_decodes_to(&encoded(&record_with(5)), &record_with(6))
);

// --- The magic and the version, against the PNG signature and the prototype's (REQ-TOOL-109, REQ-TOOL-118, R-380) ----

/// `magic` meets §2's proposal checks: none of the PNG signature's 4-byte windows, not a chunk type or ASCII (first
/// byte above 0x7F), 16 of 32 bits set, and different from its own bit reversal.
fn check_magic(magic: [u8; 4]) {
    for window in PNG_SIGNATURE.windows(4) {
        assert_ne!(
            magic.as_slice(),
            window,
            "the magic collides with the PNG signature"
        );
    }
    assert!(
        magic[0] > 0x7F,
        "the magic could be a PNG chunk type or ASCII text"
    );
    let word = u32::from_be_bytes(magic);
    assert_eq!(word.count_ones(), 16, "the magic is not balanced");
    assert_ne!(
        word.reverse_bits(),
        word,
        "the magic reads the same back to front"
    );
}

#[test]
fn embed_record_magic_is_not_png() {
    check_magic(MAGIC);
}

negative_control!(
    embed_record_magic_is_not_png,
    "the PNG signature's first four bytes must fail the magic check",
    expected = "collides with the PNG signature",
    check_magic([0x89, 0x50, 0x4E, 0x47])
);

/// `magic` and `version` are the prototype's values as §2 transcribes them (R-380): `PRPX`, `50 52 50 58`, and 2.
fn check_prototype_values(magic: [u8; 4], version: u8) {
    assert_eq!(
        magic,
        [0x50, 0x52, 0x50, 0x58],
        "the prototype's magic is not §2's PRPX"
    );
    assert_eq!(&magic, b"PRPX", "the prototype's magic is not §2's PRPX");
    assert_eq!(version, 2, "the prototype's version is not §2's 2");
}

#[test]
fn embed_record_prototype_values_are_section_2() {
    check_prototype_values(PROTOTYPE_MAGIC, PROTOTYPE_VERSION);
}

negative_control!(
    embed_record_prototype_values_are_section_2,
    "a prototype version of 1, not the transcribed 2, must fail the transcription check",
    expected = "the prototype's version is not §2's 2",
    check_prototype_values(PROTOTYPE_MAGIC, 1)
);

/// `magic` differs from the prototype's `prototype` in every byte (§2, "Magic and version").
fn check_magic_not_prototype(magic: [u8; 4], prototype: [u8; 4]) {
    for (k, (m, p)) in magic.iter().zip(prototype).enumerate() {
        assert_ne!(*m, p, "the magic's byte {k} is the prototype's");
    }
}

#[test]
fn embed_record_magic_is_not_prototype() {
    check_magic_not_prototype(MAGIC, PROTOTYPE_MAGIC);
    let bytes = encoded(&record_with(CONFIG_PAYLOAD));
    check_magic_not_prototype(bytes[..4].try_into().expect("four bytes"), PROTOTYPE_MAGIC);
}

negative_control!(
    embed_record_magic_is_not_prototype,
    "a magic sharing a byte with `PRPX` must fail the prototype check",
    expected = "the magic's byte 1 is the prototype's",
    check_magic_not_prototype([0x8F, 0x52, 0x72, 0x6E], PROTOTYPE_MAGIC)
);

/// A newly embedded record's header carries `version` in byte 4, the reader reports it, and it differs from the
/// prototype layout's `prototype` (REQ-TOOL-118) and is above it (REQ-TOOL-109).
fn check_new_version(bytes: &[u8], prototype: u8) {
    let version = bytes[4];
    assert_ne!(
        version, prototype,
        "a new record's version is the prototype layout's"
    );
    assert!(
        version > prototype,
        "a new record's version is not above the prototype's"
    );
    let decoded = decode(bytes).expect("a new record decodes");
    assert_eq!(
        decoded.version, version,
        "the reader reports another version"
    );
}

#[test]
fn embed_record_version_differs_from_prototype() {
    let bytes = encoded(&record_with(CONFIG_PAYLOAD));
    assert_eq!(bytes[4], VERSION);
    assert_eq!(VERSION, 3, "§2 proposes version 3 (R-380)");
    check_new_version(&bytes, PROTOTYPE_VERSION);
}

negative_control!(
    embed_record_version_differs_from_prototype,
    "a record written at the prototype's version must fail the version check",
    expected = "a new record's version is the prototype layout's",
    {
        let mut bytes = encoded(&record_with(CONFIG_PAYLOAD));
        bytes[4] = PROTOTYPE_VERSION;
        check_new_version(&bytes, PROTOTYPE_VERSION)
    }
);

/// An intact record of `version`, its header CRC recomputed, decodes and reports `version` as read (§2: "A record's
/// `version` is read, not checked").
fn check_other_layout_read(version: u8, reported: fn(&[u8]) -> Option<u8>) {
    let record = record_with(CONFIG_PAYLOAD);
    let mut bytes = encoded(&record);
    bytes[4] = version;
    let crc = crc32(&bytes[..HEADER_LEN - CRC_LEN]);
    bytes[HEADER_LEN - CRC_LEN..HEADER_LEN].copy_from_slice(&crc.to_be_bytes());
    assert_eq!(
        reported(&bytes),
        Some(version),
        "a record of version {version} is not reported as such"
    );
    assert_eq!(decoded_record(&bytes), Ok(record));
}

#[test]
fn embed_record_version_of_another_layout_is_read() {
    for version in [0, 1, PROTOTYPE_VERSION, VERSION + 1, 255] {
        check_other_layout_read(version, |bytes| {
            decode(bytes).ok().map(|d: Decoded| d.version)
        });
    }
}

negative_control!(
    embed_record_version_of_another_layout_is_read,
    "a reader that reports its own version must fail the read check",
    expected = "a record of version 2 is not reported as such",
    check_other_layout_read(PROTOTYPE_VERSION, |bytes| decode(bytes)
        .ok()
        .map(|_| VERSION))
);

// --- Tile side and grid (REQ-TOOL-062, §3, §7) ---------------------------------------------------------------------

/// A record carrying a `payload_len`-byte payload has an RGB tile side of `expected`, and that is
/// `ceil(sqrt(ceil(record_bits / 3)))` computed apart from `tile_side`.
fn check_tile_side(payload_len: usize, expected: u32) {
    let bits = record_bits(payload_len);
    let side = tile_side(bits, RGB_BITS_PER_PIXEL);
    let pixels = bits.div_ceil(3);
    let by_formula = (1u64..).find(|s| s * s >= pixels).expect("a side exists");
    assert_eq!(
        u64::from(side),
        by_formula,
        "the tile side is not ceil(sqrt(ceil(record_bits / 3)))"
    );
    assert_eq!(
        side, expected,
        "the tile side for a {payload_len} B payload is not {expected}"
    );
}

/// §7's measured 382 B payload makes a 402 B, 3216-bit record: 1072 pixels, side 33.
#[test]
fn embed_tile_side_measured_382_b_record() {
    assert_eq!(record_bits(CONFIG_PAYLOAD), 3216);
    check_tile_side(CONFIG_PAYLOAD, 33);
}

negative_control!(
    embed_tile_side_measured_382_b_record,
    "a side of 32, the 382 B record without its 20 B of header and CRC, must fail the check",
    expected = "is not 32",
    check_tile_side(CONFIG_PAYLOAD, 32)
);

/// The RGB grid at each of §7's resolutions holds `counts` tiles of a record carrying `payload_len` bytes.
fn check_tile_counts(payload_len: usize, counts: &[(u32, u32)]) {
    let side = tile_side(record_bits(payload_len), RGB_BITS_PER_PIXEL);
    for &(resolution, count) in counts {
        let (cols, rows) = tile_grid(resolution, resolution, side);
        assert_eq!(
            cols * rows,
            count,
            "{resolution}² holds {} tiles, not §7's {count}",
            cols * rows
        );
    }
}

/// §7's table: config only at 64² to 1024², and config + full shader.
#[test]
fn embed_tile_side_reproduces_section_7_table() {
    let resolutions = [64, 128, 256, 512, 1024];
    let config_only = [1, 9, 49, 225, 961];
    let with_shader = [0, 1, 4, 25, 100];
    check_tile_counts(
        CONFIG_PAYLOAD,
        &resolutions.into_iter().zip(config_only).collect::<Vec<_>>(),
    );
    check_tile_counts(
        CONFIG_PAYLOAD + SHADER_PAYLOAD,
        &resolutions.into_iter().zip(with_shader).collect::<Vec<_>>(),
    );
    assert_eq!(
        tile_side(
            record_bits(CONFIG_PAYLOAD + SHADER_PAYLOAD),
            RGB_BITS_PER_PIXEL
        ),
        95
    );
}

negative_control!(
    embed_tile_side_reproduces_section_7_table,
    "1024 tiles at 1024², a side of 32, must fail the table check",
    expected = "not §7's 1024",
    check_tile_counts(CONFIG_PAYLOAD, &[(1024, 1024)])
);

/// The tile side of `bits` bits at `bpp` bits per pixel is `expected`.
fn check_tile_side_value(bits: u64, bpp: u32, expected: u32) {
    let side = tile_side(bits, bpp);
    assert_eq!(
        side, expected,
        "the side of {bits} bits at {bpp} per pixel is {side}, not {expected}"
    );
}

/// The alpha plane's side is `ceil(sqrt(record_bits))`, a perfect square is its own side, and the grid's tiles sit at
/// multiples of the side from the top-left pixel (§2, "Tile side and the grid").
#[test]
fn embed_tile_side_alpha_and_grid_origin() {
    check_tile_side_value(record_bits(CONFIG_PAYLOAD), ALPHA_BITS_PER_PIXEL, 57);
    check_tile_side_value(9, 1, 3);
    check_tile_side_value(10, 1, 4);
    check_tile_side_value(27, 3, 3);
    check_tile_side_value(28, 3, 4);
    assert_eq!(tile_grid(130, 70, 33), (3, 2));
    assert_eq!(tile_origin(2, 1, 33), (66, 33));
}

negative_control!(
    embed_tile_side_alpha_and_grid_origin,
    "a side rounded down must fail the side check",
    expected = "is 4, not 3",
    check_tile_side_value(10, 1, 3)
);

// --- Bit order within a tile (§2) ----------------------------------------------------------------------------------

/// In `lows` (a tile's low bits, `(y × side + x) × bpp + channel`), bit `i` of `record` sits at pixel
/// `p = ⌊i / bpp⌋`, column `p mod side`, row `⌊p / side⌋`, channel `i mod bpp`, each byte MSB first; and
/// `bit_slot` gives that place.
fn check_bit_order(record: &[u8], lows: &[u8], side: u32, bpp: u32) {
    for i in 0..8 * record.len() as u64 {
        let bit = (record[(i / 8) as usize] >> (7 - i % 8)) & 1;
        let pixel = i / u64::from(bpp);
        let (x, y, channel) = (
            pixel % u64::from(side),
            pixel / u64::from(side),
            i % u64::from(bpp),
        );
        let at = ((y * u64::from(side) + x) * u64::from(bpp) + channel) as usize;
        assert_eq!(
            lows[at], bit,
            "bit {i} is not at column {x}, row {y}, channel {channel}"
        );
        let slot = bit_slot(i, side, bpp);
        assert_eq!(
            slot,
            Slot {
                x: x as u32,
                y: y as u32,
                channel: channel as u32
            },
            "bit_slot places bit {i} elsewhere"
        );
        assert_eq!(
            record_bit(record, i),
            bit == 1,
            "record_bit {i} is not MSB first"
        );
    }
}

#[test]
fn embed_tile_bit_order_rgb_and_alpha() {
    let bytes = encoded(&record_with(CONFIG_PAYLOAD));
    for bpp in [RGB_BITS_PER_PIXEL, ALPHA_BITS_PER_PIXEL] {
        let side = tile_side(record_bits(CONFIG_PAYLOAD), bpp);
        let mut lows = vec![0u8; (side * side * bpp) as usize];
        write_tile(&bytes, side, bpp, &mut lows);
        check_bit_order(&bytes, &lows, side, bpp);
    }
}

negative_control!(
    embed_tile_bit_order_rgb_and_alpha,
    "a tile written least significant bit first must fail the bit-order check",
    expected = "is not at column",
    {
        let bytes = encoded(&record_with(CONFIG_PAYLOAD));
        let side = tile_side(record_bits(CONFIG_PAYLOAD), RGB_BITS_PER_PIXEL);
        let reversed: Vec<u8> = bytes.iter().map(|b| b.reverse_bits()).collect();
        let mut lows = vec![0u8; (side * side * RGB_BITS_PER_PIXEL) as usize];
        write_tile(&reversed, side, RGB_BITS_PER_PIXEL, &mut lows);
        check_bit_order(&bytes, &lows, side, RGB_BITS_PER_PIXEL)
    }
);

/// A tile written by `write` over `fill` and read back decodes to `record`, and its slots after the record keep
/// `fill`.
fn check_tile_round_trip(
    record: &Record,
    bpp: u32,
    fill: u8,
    write: fn(&[u8], u32, u32, &mut [u8]),
) {
    let bytes = encoded(record);
    let side = tile_side(record_bits(record.payload.len()), bpp);
    let mut lows = vec![fill; (side * side * bpp) as usize];
    write(&bytes, side, bpp, &mut lows);
    assert!(
        lows[8 * bytes.len()..].iter().all(|&low| low == fill),
        "the slots after the record were written"
    );
    assert_eq!(
        decoded_record(&read_tile(&lows, side, bpp)).as_ref(),
        Ok(record),
        "the tile did not read back as its record"
    );
}

#[test]
fn embed_tile_round_trip_keeps_spare_slots() {
    for bpp in [RGB_BITS_PER_PIXEL, ALPHA_BITS_PER_PIXEL] {
        for fill in [0, 1] {
            check_tile_round_trip(&record_with(CONFIG_PAYLOAD), bpp, fill, write_tile);
        }
    }
}

negative_control!(
    embed_tile_round_trip_keeps_spare_slots,
    "a writer that clears the spare slots must fail the round-trip check",
    expected = "the slots after the record were written",
    check_tile_round_trip(
        &record_with(CONFIG_PAYLOAD),
        RGB_BITS_PER_PIXEL,
        1,
        |bytes, side, bpp, lows| {
            lows.fill(0);
            write_tile(bytes, side, bpp, lows);
        }
    )
);
