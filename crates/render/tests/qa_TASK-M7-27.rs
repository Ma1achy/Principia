//! QA tests for TASK-M7-27, written from REQ-TOOL-060, REQ-TOOL-062, REQ-TOOL-109, REQ-TOOL-110 and REQ-TOOL-118
//! against their source, `principia_dd_image_embedding.md` §2, §3, §7 and §9 (and R-380 for the prototype's values),
//! not from the implementation. Every expected byte is built here by hand from §2's table: an own CRC-32 (PNG's,
//! checked on `123456789`), big-endian integers, the magic and version §2 states. Each test has a registered negative
//! control (R-176). Test names carry the acceptance filters (`embed_record_version`, `embed_record_crc`,
//! `embed_tile_side`) so the task's acceptance commands run them too.

use render::embed::record::{
    decode, encode, read_tile, record_bits, tile_grid, tile_origin, tile_side, write_tile, Discard,
    Flags, Record, Variant,
};
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// §2's values, transcribed from the doc.

/// §2, "Magic and version": the proposed magic.
const DOC_MAGIC: [u8; 4] = [0x8F, 0x50, 0x72, 0x6E];
/// §2: the proposed version byte, 3.
const DOC_VERSION: u8 = 3;
/// §2 (R-380): the prototype's magic `PRPX` and its highest version byte, 2.
const PROTO_MAGIC: [u8; 4] = [0x50, 0x52, 0x50, 0x58];
const PROTO_VERSION: u8 = 2;
/// §2: the PNG signature.
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// §2, "The CRC is PNG's": reflected polynomial 0xEDB88320, init and final XOR 0xFFFFFFFF, bit by bit.
fn own_crc(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    c ^ 0xFFFF_FFFF
}

/// §2's record by hand: `magic ‖ version ‖ flags ‖ payload_len(u32 BE) ‖ n_records(u16 BE) ‖ crc32(bytes 0–11) ‖
/// payload ‖ crc32(payload)`.
fn by_hand(magic: [u8; 4], version: u8, flags: u8, n_records: u16, payload: &[u8]) -> Vec<u8> {
    let mut v = magic.to_vec();
    v.push(version);
    v.push(flags);
    v.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    v.extend_from_slice(&n_records.to_be_bytes());
    let hc = own_crc(&v);
    v.extend_from_slice(&hc.to_be_bytes());
    v.extend_from_slice(payload);
    v.extend_from_slice(&own_crc(payload).to_be_bytes());
    v
}

/// A payload of `len` bytes with no structure a lucky CRC could hide behind.
fn payload(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761).wrapping_add(12_345);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 11) as u8
        })
        .collect()
}

fn rec(variant: Variant, source: bool, n_records: u16, payload: Vec<u8>) -> Record {
    Record {
        flags: Flags { variant, source },
        n_records,
        payload,
    }
}

/// §2's flag byte for a variant and `source`: variant in bits 0–1 (tiled 0, redundant 1, hybrid 2), source bit 2.
fn doc_flags(variant: Variant, source: bool) -> u8 {
    let v = match variant {
        Variant::Tiled => 0,
        Variant::Redundant => 1,
        Variant::Hybrid => 2,
    };
    v | (u8::from(source) << 2)
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-118: a newly embedded record's header version differs from the prototype layout's.

fn check_new_version(bytes: &[u8]) {
    assert!(bytes.len() > 4, "record shorter than its version byte");
    assert_ne!(
        bytes[4], PROTO_VERSION,
        "new record's version byte reuses the prototype's"
    );
    assert_eq!(
        bytes[4], DOC_VERSION,
        "new record's version byte is not §2's proposed value"
    );
    let d = decode(bytes).expect("a fresh record decodes");
    assert_eq!(
        d.version, bytes[4],
        "decode reports a version other than byte 4"
    );
}

#[test]
fn embed_record_version_qa_new_record_is_not_prototype() {
    for (i, len) in [0usize, 1, 382, 2969].into_iter().enumerate() {
        let bytes = encode(&rec(Variant::Tiled, false, 9, payload(len, i as u32))).unwrap();
        check_new_version(&bytes);
    }
}

negative_control!(
    embed_record_version_qa_new_record_is_not_prototype,
    "a record carrying the prototype's version must fail the check",
    expected = "reuses the prototype's",
    check_new_version(&by_hand(DOC_MAGIC, PROTO_VERSION, 0, 1, b"x"))
);

/// §2: the version is read, not checked; an intact record of another layout is reported with its own version.
fn check_version_reported(bytes: &[u8], version: u8) {
    let d = decode(bytes)
        .unwrap_or_else(|e| panic!("intact version-{version} record discarded: {e:?}"));
    assert_eq!(
        d.version, version,
        "intact record's version not reported as read"
    );
}

#[test]
fn embed_record_version_qa_other_layout_is_read_not_checked() {
    for v in [0u8, 1, PROTO_VERSION, DOC_VERSION, 4, 0x7F, 0xFF] {
        check_version_reported(&by_hand(DOC_MAGIC, v, 0, 1, &payload(40, 7)), v);
    }
}

negative_control!(
    embed_record_version_qa_other_layout_is_read_not_checked,
    "a version byte rewritten after the header CRC is corruption, not another layout: the check must fail",
    expected = "discarded",
    {
        let mut b = by_hand(DOC_MAGIC, DOC_VERSION, 0, 1, &payload(40, 7));
        b[4] = PROTO_VERSION;
        check_version_reported(&b, PROTO_VERSION)
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-060 and REQ-TOOL-110: the record is §2's bytes exactly.

fn check_wire(record: &Record, expected: &[u8]) {
    let got = encode(record).expect("encodes");
    assert_eq!(
        got, expected,
        "encoded record differs from §2's bytes by hand"
    );
}

#[test]
fn embed_record_crc_qa_bytes_are_section_2_by_hand() {
    assert_eq!(own_crc(b"123456789"), 0xCBF4_3926, "own CRC is not PNG's");
    let cases = [
        (Variant::Tiled, false, 1u16, 0usize),
        (Variant::Tiled, false, 961, 382),
        (Variant::Redundant, true, 0x0102, 0x0304),
        (Variant::Hybrid, false, u16::MAX, 2969),
        (Variant::Hybrid, true, 0xA55A, 70_000),
    ];
    for (i, (variant, source, n, len)) in cases.into_iter().enumerate() {
        let p = payload(len, i as u32 + 1);
        let expected = by_hand(DOC_MAGIC, DOC_VERSION, doc_flags(variant, source), n, &p);
        assert_eq!(expected.len(), 16 + len + 4);
        check_wire(&rec(variant, source, n, p), &expected);
    }
}

negative_control!(
    embed_record_crc_qa_bytes_are_section_2_by_hand,
    "little-endian payload_len must fail the check",
    expected = "differs from §2's bytes",
    {
        let p = payload(0x0304, 3);
        let mut expected = by_hand(DOC_MAGIC, DOC_VERSION, 0, 0x0102, &p);
        expected[6..10].reverse();
        let hc = own_crc(&expected[..12]);
        expected[12..16].copy_from_slice(&hc.to_be_bytes());
        check_wire(&rec(Variant::Tiled, false, 0x0102, p), &expected);
    }
);

/// Decodes `bytes`, built by hand, to the record by hand; bytes after the payload's CRC are not part of it.
fn check_decodes(bytes: &[u8], expected: &Record) {
    let d = decode(bytes).unwrap_or_else(|e| panic!("hand-built record discarded: {e:?}"));
    assert_eq!(d.version, DOC_VERSION, "version misread");
    assert_eq!(&d.record, expected, "hand-built record decoded wrongly");
}

#[test]
fn embed_record_crc_qa_hand_built_records_decode() {
    for (i, variant) in [Variant::Tiled, Variant::Redundant, Variant::Hybrid]
        .into_iter()
        .enumerate()
    {
        for source in [false, true] {
            let p = payload(100 + i, i as u32);
            let mut b = by_hand(
                DOC_MAGIC,
                DOC_VERSION,
                doc_flags(variant, source),
                0x1234,
                &p,
            );
            let expected = rec(variant, source, 0x1234, p);
            check_decodes(&b, &expected);
            b.extend_from_slice(&[0xFF; 37]);
            check_decodes(&b, &expected);
        }
    }
}

negative_control!(
    embed_record_crc_qa_hand_built_records_decode,
    "a record whose n_records is read wrongly must fail the check",
    expected = "decoded wrongly",
    {
        let p = payload(100, 0);
        let b = by_hand(DOC_MAGIC, DOC_VERSION, 0, 0x1234, &p);
        check_decodes(&b, &rec(Variant::Tiled, false, 0x3412, p));
    }
);

/// REQ-TOOL-060's verify: `records` each decode on their own; corrupting byte `at` of record `k` by `mask` discards
/// record `k` whole and leaves every other record decoding to itself.
fn check_one_corrupt(records: &[Record], k: usize, at: usize, mask: u8) {
    let mut images: Vec<Vec<u8>> = records.iter().map(|r| encode(r).unwrap()).collect();
    images[k][at] ^= mask;
    for (j, (bytes, original)) in images.iter().zip(records).enumerate() {
        let got = decode(bytes);
        if j == k {
            assert!(
                got.is_err(),
                "record {k} with byte {at} ^ {mask:#04x} was not discarded"
            );
        } else {
            assert_eq!(
                got.map(|d| d.record).as_ref(),
                Ok(original),
                "untouched record {j} did not decode"
            );
        }
    }
}

#[test]
fn embed_record_crc_qa_every_single_byte_corruption_discards_only_its_record() {
    let records: Vec<Record> = (0..4)
        .map(|i| {
            rec(
                [Variant::Tiled, Variant::Redundant, Variant::Hybrid][i % 3],
                i % 2 == 1,
                4,
                payload(382, 100 + i as u32),
            )
        })
        .collect();
    let len = 16 + 382 + 4;
    for k in 0..records.len() {
        for at in 0..len {
            for mask in [0x01u8, 0x80, 0xFF, 0x5A] {
                check_one_corrupt(&records, k, at, mask);
            }
        }
    }
}

negative_control!(
    embed_record_crc_qa_every_single_byte_corruption_discards_only_its_record,
    "an untouched record (mask 0) is not discarded, failing the check",
    expected = "was not discarded",
    check_one_corrupt(
        &[
            rec(Variant::Tiled, false, 2, payload(382, 1)),
            rec(Variant::Tiled, false, 2, payload(382, 2))
        ],
        0,
        200,
        0
    )
);

/// §2, "A record is trusted or discarded whole": each listed reason discards the record.
fn check_discarded(bytes: &[u8], what: &str) {
    assert!(
        decode(bytes).is_err(),
        "record that is {what} was not discarded"
    );
}

#[test]
fn embed_record_crc_qa_section_2_discard_reasons() {
    let p = payload(64, 9);
    let good = by_hand(DOC_MAGIC, DOC_VERSION, 0, 1, &p);
    // Shorter than its header, at every length.
    for n in 0..16 {
        check_discarded(&good[..n], "shorter than its header");
    }
    // Shorter than its payload_len says, at every length.
    for n in 16..good.len() {
        check_discarded(&good[..n], "shorter than its payload_len");
    }
    // A different magic with a valid header CRC: the prototype's, and the PNG signature's first four bytes.
    check_discarded(
        &by_hand(PROTO_MAGIC, DOC_VERSION, 0, 1, &p),
        "carrying PRPX",
    );
    check_discarded(
        &by_hand(
            [
                PNG_SIGNATURE[0],
                PNG_SIGNATURE[1],
                PNG_SIGNATURE[2],
                PNG_SIGNATURE[3],
            ],
            DOC_VERSION,
            0,
            1,
            &p,
        ),
        "carrying the PNG signature",
    );
    // A payload_len that overstates the payload, header CRC valid.
    let mut long = by_hand(DOC_MAGIC, DOC_VERSION, 0, 1, &p);
    long[6..10].copy_from_slice(&u32::MAX.to_be_bytes());
    let hc = own_crc(&long[..12]);
    long[12..16].copy_from_slice(&hc.to_be_bytes());
    check_discarded(&long, "claiming a 4 GiB payload");
    // Every undefined flags byte with valid CRCs: variant 3, or any reserved bit 3–7 set.
    for f in 0..=255u8 {
        if f & 0b1111_1000 != 0 || f & 0b11 == 3 {
            check_discarded(
                &by_hand(DOC_MAGIC, DOC_VERSION, f, 1, &p),
                "flagged with an undefined combination",
            );
        }
    }
}

negative_control!(
    embed_record_crc_qa_section_2_discard_reasons,
    "an intact record must fail the discard check",
    expected = "was not discarded",
    check_discarded(
        &by_hand(DOC_MAGIC, DOC_VERSION, 0, 1, &payload(64, 9)),
        "intact"
    )
);

/// §2, "Flag bits": a flags byte is defined iff its variant (bits 0–1) is not 3 and bits 3–7 are 0.
fn doc_defined(f: u8) -> bool {
    f & 0b1111_1000 == 0 && f & 0b11 != 3
}

/// Decodes a record with flags byte `f` and valid CRCs: it decodes, to §2's variant and `source`, iff `defined`.
fn check_flags_byte(f: u8, defined: bool) {
    let got = decode(&by_hand(DOC_MAGIC, DOC_VERSION, f, 1, b"p"));
    match (defined, got) {
        (true, Ok(d)) => {
            let variant = [Variant::Tiled, Variant::Redundant, Variant::Hybrid][usize::from(f & 3)];
            assert_eq!(
                d.record.flags,
                Flags {
                    variant,
                    source: f & 4 != 0
                },
                "flags byte {f:#04x} decoded to the wrong flags"
            );
        }
        (true, Err(e)) => panic!("defined flags byte {f:#04x} discarded: {e:?}"),
        (false, Ok(_)) => panic!("undefined flags byte {f:#04x} decoded"),
        (false, Err(e)) => assert_eq!(e, Discard::Flags, "flags byte {f:#04x} discarded for {e:?}"),
    }
}

#[test]
fn embed_record_crc_qa_flags_bits_are_section_2() {
    for f in 0..=255u8 {
        check_flags_byte(f, doc_defined(f));
    }
}

negative_control!(
    embed_record_crc_qa_flags_bits_are_section_2,
    "a definition that takes source (bit 2) for a reserved bit must fail on 0x04",
    expected = "undefined flags byte 0x04 decoded",
    check_flags_byte(0x04, false)
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-109: the proposed magic is §2's, collides with no PNG signature window nor the prototype's, and a newly
// written record carries it.

fn check_magic(magic: [u8; 4]) {
    for w in PNG_SIGNATURE.windows(4) {
        assert_ne!(&magic[..], w, "magic collides with a PNG signature window");
    }
    assert!(
        magic.iter().any(|&b| b > 0x7F),
        "magic is ASCII: could be a PNG chunk type or text"
    );
    for (m, p) in magic.iter().zip(PROTO_MAGIC) {
        assert_ne!(*m, p, "magic shares a byte with the prototype's PRPX");
    }
    let ones: u32 = magic.iter().map(|b| b.count_ones()).sum();
    assert_eq!(ones, 16, "magic does not have 16 of its 32 bits set");
    let word = u32::from_be_bytes(magic);
    assert_ne!(word, word.reverse_bits(), "magic equals its bit reversal");
}

#[test]
fn embed_record_version_qa_magic_proposal_is_section_2() {
    check_magic(DOC_MAGIC);
    let bytes = encode(&rec(Variant::Tiled, false, 1, payload(10, 1))).unwrap();
    assert_eq!(bytes[..4], DOC_MAGIC, "new record's magic is not §2's");
}

negative_control!(
    embed_record_version_qa_magic_proposal_is_section_2,
    "the PNG signature's first four bytes must fail the check",
    expected = "collides with a PNG signature window",
    check_magic([0x89, 0x50, 0x4E, 0x47])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-062: side = ceil(sqrt(ceil(record_bits / 3))), and §7's tile counts.

/// The side by §3's formula in exact integers: the least s with s² ≥ ceil(bits / b).
fn doc_side(bits: u64, b: u64) -> u64 {
    let pixels = bits.div_ceil(b);
    let mut s = (pixels as f64).sqrt() as u64;
    while s * s < pixels {
        s += 1;
    }
    while s > 0 && (s - 1) * (s - 1) >= pixels {
        s -= 1;
    }
    s
}

fn check_side(payload_len: usize, expected: u32) {
    let bits = 8 * (16 + payload_len as u64 + 4);
    assert_eq!(
        record_bits(payload_len),
        bits,
        "record_bits is not 8 × (16 + len + 4)"
    );
    assert_eq!(
        tile_side(bits, 3),
        expected,
        "RGB tile side for a {payload_len} B payload is not {expected}"
    );
}

#[test]
fn embed_tile_side_qa_measured_382_b_record() {
    // §7: 382 B config payload → record 402 B = 3216 bits → ceil(3216/3) = 1072 → ceil(sqrt(1072)) = 33.
    assert_eq!(doc_side(3216, 3), 33);
    check_side(382, 33);
    // §2: both of §7's payloads, 382 + 2969 B, give a side of 95.
    assert_eq!(doc_side(8 * (16 + 382 + 2969 + 4), 3), 95);
    check_side(382 + 2969, 95);
}

negative_control!(
    embed_tile_side_qa_measured_382_b_record,
    "a side of 32 (floor, not ceil, of the root) must fail",
    expected = "is not 32",
    check_side(382, 32)
);

/// `tile_side(bits, bpp)` against §3's formula at `formula_bpp` bits per pixel, over a range and its edges.
fn check_side_sweep(bpp: u32, formula_bpp: u64) {
    for bits in (0..20_000u64).chain([3 * 1089, 3 * 1089 + 1, 3 * 1088 + 1, 1u64 << 40]) {
        assert_eq!(
            u64::from(tile_side(bits, bpp)),
            doc_side(bits, formula_bpp),
            "tile side for {bits} bits at {bpp} bits/pixel is off §3's formula"
        );
    }
}

#[test]
fn embed_tile_side_qa_formula_over_range_rgb_and_alpha() {
    check_side_sweep(3, 3);
    check_side_sweep(1, 1);
}

negative_control!(
    embed_tile_side_qa_formula_over_range_rgb_and_alpha,
    "the RGB side judged by the alpha plane's formula must fail",
    expected = "is off §3's formula",
    check_side_sweep(3, 1)
);

/// §7's table: tiles per square resolution, `None` for "too small".
fn check_counts(payload_len: usize, table: &[(u32, Option<u32>)]) {
    let side = tile_side(record_bits(payload_len), 3);
    for &(res, tiles) in table {
        let (c, r) = tile_grid(res, res, side);
        let got = c * r;
        assert_eq!(
            got,
            tiles.unwrap_or(0),
            "{res}² with a {payload_len} B payload holds {got} tiles, not §7's"
        );
    }
}

#[test]
fn embed_tile_side_qa_reproduces_section_7_table() {
    check_counts(
        382,
        &[
            (64, Some(1)),
            (128, Some(9)),
            (256, Some(49)),
            (512, Some(225)),
            (1024, Some(961)),
        ],
    );
    check_counts(
        382 + 2969,
        &[
            (64, None),
            (128, Some(1)),
            (256, Some(4)),
            (512, Some(25)),
            (1024, Some(100)),
        ],
    );
    // §2: tile (c, r) at pixel (c × side, r × side); the strip beyond the last whole tile is not used.
    assert_eq!(tile_origin(2, 5, 33), (66, 165));
    assert_eq!(tile_grid(100, 65, 33), (3, 1));
}

negative_control!(
    embed_tile_side_qa_reproduces_section_7_table,
    "a wrong tile count at 1024² must fail",
    expected = "not §7's",
    check_counts(382, &[(1024, Some(1000))])
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-110: bit order (MSB first) and placement within a tile (row by row from the top-left; R, G, B in a pixel).

/// §2's slot of bit `i` in a `side` tile at `b` bits/pixel, as an index into `lows[(y × side + x) × b + channel]`.
fn doc_slot(i: usize, side: usize, b: usize) -> usize {
    let p = i / b;
    let (x, y) = (p % side, p / side);
    (y * side + x) * b + i % b
}

fn check_placement(record: &[u8], side: u32, b: u32, lows: &[u8], before: u8) {
    let (s, bu) = (side as usize, b as usize);
    let n = record.len() * 8;
    for i in 0..n {
        let bit = (record[i / 8] >> (7 - i % 8)) & 1;
        assert_eq!(
            lows[doc_slot(i, s, bu)],
            bit,
            "record bit {i} is not at §2's slot"
        );
    }
    let used: std::collections::HashSet<usize> = (0..n).map(|i| doc_slot(i, s, bu)).collect();
    for (k, &v) in lows.iter().enumerate() {
        if !used.contains(&k) {
            assert_eq!(
                v, before,
                "spare slot {k} after the record's last bit was written"
            );
        }
    }
}

#[test]
fn embed_tile_side_qa_bit_order_and_placement() {
    for (b, before) in [(3u32, 0u8), (3, 1), (1, 0), (1, 1)] {
        let bytes = encode(&rec(Variant::Tiled, true, 9, payload(382, 5))).unwrap();
        let side = tile_side(8 * bytes.len() as u64, b);
        let mut lows = vec![before; (side * side * b) as usize];
        write_tile(&bytes, side, b, &mut lows);
        check_placement(&bytes, side, b, &lows, before);
        let back = read_tile(&lows, side, b);
        assert_eq!(&back[..bytes.len()], &bytes[..], "tile does not read back");
        assert!(decode(&back).is_ok(), "record read from tile discarded");
    }
    // A hand-checked corner: record byte 0x8F starts with bits 1,0,0,0,1,1,1,1; in RGB, pixel (0,0) gets R=1,G=0,B=0,
    // pixel (1,0) R=0,G=1,B=1, pixel (2,0) R=1,G=1.
    let rec_bytes = [0x8Fu8, 0x00];
    let mut lows = vec![0u8; 3 * 3 * 3];
    write_tile(&rec_bytes, 3, 3, &mut lows);
    assert_eq!(&lows[..8], &[1, 0, 0, 0, 1, 1, 1, 1]);
    // Alpha, side 4: row 0 is bits 0–3, row 1 bits 4–7.
    let mut lows = vec![0u8; 16];
    write_tile(&[0x8F, 0x01], 4, 1, &mut lows);
    assert_eq!(lows, [1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
}

negative_control!(
    embed_tile_side_qa_bit_order_and_placement,
    "least-significant-bit-first placement must fail",
    expected = "is not at §2's slot",
    {
        let bytes = [0x8Fu8, 0x50, 0x72, 0x6E];
        let mut lows = vec![0u8; 16 * 3];
        for i in 0..32 {
            lows[doc_slot(i, 4, 3)] = (bytes[i / 8] >> (i % 8)) & 1;
        }
        check_placement(&bytes, 4, 3, &lows, 0);
    }
);
