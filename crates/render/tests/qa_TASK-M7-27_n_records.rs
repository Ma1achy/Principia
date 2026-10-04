//! QA tests for TASK-M7-27 (REQ-TOOL-110, REQ-TOOL-062), written from `principia_dd_image_embedding.md` §2's
//! "`n_records`" and "The alpha plane's records are an extra redundancy layer" definitions, not from the
//! implementation: `n_records` counts the RGB plane's tiles only, a pristine 128² image reports 9/9, a 512² image with
//! its alpha stripped reports 225/225, and the alpha plane holds 4 records at 128² for a 402 B record, counted
//! separately. The bits are placed into the image here by §2's rule by hand; the crate's reader (`read_tile`,
//! `decode`) recovers them. Each test has a registered negative control (R-176). Test names carry the acceptance
//! filter `embed_tile_side`.

use render::embed::record::{
    decode, encode, placed_records, read_tile, record_bits, tile_grid, tile_side, Flags, Record,
    Variant, ALPHA_BITS_PER_PIXEL, RGB_BITS_PER_PIXEL,
};
use validation::negative_control;

/// §2: a config-only record's payload is §7's 382 B, the record 16 + 382 + 4 = 402 B.
const PAYLOAD_LEN: usize = 382;

/// §2, "Tile side and the grid": side `ceil(sqrt(ceil(record_bits / b)))`, by integers only.
fn doc_side(payload_len: usize, b: u64) -> u32 {
    let bits = 8 * (16 + payload_len as u64 + 4);
    let px = bits.div_ceil(b);
    let mut s = 0u64;
    while s * s < px {
        s += 1;
    }
    s as u32
}

/// §2: a plane's tile count, `⌊width / side⌋ × ⌊height / side⌋`.
fn doc_tiles(width: u32, height: u32, payload_len: usize, b: u64) -> u32 {
    let side = doc_side(payload_len, b);
    (width / side) * (height / side)
}

// ---------------------------------------------------------------------------------------------------------------
// The counts §2 states.

fn check_rgb_count(res: u32, expected: u16) {
    let got = placed_records(res, res, PAYLOAD_LEN);
    assert_eq!(
        got, expected,
        "{res}² with a 402 B record: n_records {got}, not §2's"
    );
    assert_eq!(
        u32::from(got),
        doc_tiles(res, res, PAYLOAD_LEN, 3),
        "{res}²: n_records is not the RGB grid's tile count"
    );
}

#[test]
fn embed_tile_side_qa_n_records_counts_rgb_plane_only() {
    // §2: 9/9 at 128², 225/225 at 512² (§4, §7); the rest of §7's column at the RGB side.
    check_rgb_count(64, 1);
    check_rgb_count(128, 9);
    check_rgb_count(256, 49);
    check_rgb_count(512, 225);
    check_rgb_count(1024, 961);
    // Not over both planes: 128² would be 9 + 4 = 13.
    assert_ne!(
        u32::from(placed_records(128, 128, PAYLOAD_LEN)),
        doc_tiles(128, 128, PAYLOAD_LEN, 3) + doc_tiles(128, 128, PAYLOAD_LEN, 1),
        "n_records counts the alpha plane too"
    );
    // §2: at most 65,535. A 9,900² image holds 300² = 90,000 RGB tiles of side 33.
    assert_eq!(doc_tiles(9_900, 9_900, PAYLOAD_LEN, 3), 90_000);
    assert_eq!(placed_records(9_900, 9_900, PAYLOAD_LEN), u16::MAX);
}

negative_control!(
    embed_tile_side_qa_n_records_counts_rgb_plane_only,
    "n_records over both planes (13 at 128²) must fail",
    expected = "not §2's",
    check_rgb_count(128, 13)
);

fn check_alpha_count(res: u32, expected: u32) {
    let side = tile_side(record_bits(PAYLOAD_LEN), ALPHA_BITS_PER_PIXEL);
    assert_eq!(
        side,
        doc_side(PAYLOAD_LEN, 1),
        "the alpha side is off §2's rule at b = 1"
    );
    let (c, r) = tile_grid(res, res, side);
    assert_eq!(
        c * r,
        expected,
        "{res}²: the alpha plane holds {} records, not §2's",
        c * r
    );
}

#[test]
fn embed_tile_side_qa_alpha_count_at_128() {
    // §2: 4 at 128² for a 402 B record (side 57, ⌊128 / 57⌋² = 4).
    assert_eq!(doc_side(PAYLOAD_LEN, 1), 57);
    check_alpha_count(128, 4);
}

negative_control!(
    embed_tile_side_qa_alpha_count_at_128,
    "the alpha count taken at the RGB side (9) must fail",
    expected = "not §2's",
    check_alpha_count(128, 9)
);

// ---------------------------------------------------------------------------------------------------------------
// End to end: write every tile of both planes by §2's placement, then read back what the reader reports.

/// A `res`² image's low bits: RGB (3 per pixel, `(y × res + x) × 3 + channel`) and alpha (1 per pixel).
struct Image {
    res: u32,
    rgb: Vec<u8>,
    alpha: Vec<u8>,
}

impl Image {
    fn new(res: u32) -> Self {
        let n = (res * res) as usize;
        Image {
            res,
            rgb: vec![0; 3 * n],
            alpha: vec![0; n],
        }
    }

    /// §2, "Within a tile": bit `i` (MSB first in each byte) to pixel `p = ⌊i / b⌋` of the tile, column `p mod side`,
    /// row `⌊p / side⌋`, channel `i mod b`; tile `(c, r)` at `(c × side, r × side)`.
    fn write_plane(&mut self, rec: &[u8], b: u32) {
        let side = doc_side(rec.len() - 20, u64::from(b));
        let n = self.res / side;
        let res = self.res;
        let plane = if b == 3 {
            &mut self.rgb
        } else {
            &mut self.alpha
        };
        for r in 0..n {
            for c in 0..n {
                for i in 0..8 * rec.len() as u32 {
                    let bit = (rec[(i / 8) as usize] >> (7 - i % 8)) & 1;
                    let p = i / b;
                    let x = c * side + p % side;
                    let y = r * side + p / side;
                    plane[((y * res + x) * b + i % b) as usize] = bit;
                }
            }
        }
    }

    /// The intact records the crate's reader recovers from one plane, read tile by tile over its grid.
    fn recover(&self, payload_len: usize, b: u32) -> Vec<Record> {
        let side = tile_side(record_bits(payload_len), b);
        let (cols, rows) = tile_grid(self.res, self.res, side);
        let plane = if b == 3 { &self.rgb } else { &self.alpha };
        let mut out = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let mut lows = Vec::with_capacity((side * side * b) as usize);
                for y in 0..side {
                    for x in 0..side {
                        for ch in 0..b {
                            let (px, py) = (c * side + x, r * side + y);
                            lows.push(plane[((py * self.res + px) * b + ch) as usize]);
                        }
                    }
                }
                if let Ok(d) = decode(&read_tile(&lows, side, b)) {
                    out.push(d.record);
                }
            }
        }
        out
    }

    /// Strips alpha: every pixel opaque, 255, its low bit 1.
    fn strip_alpha(&mut self) {
        self.alpha.iter_mut().for_each(|a| *a = 1);
    }
}

fn payload(len: usize) -> Vec<u8> {
    let mut x = 0x9E37_79B9u32;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 7) as u8
        })
        .collect()
}

/// Embeds a record with `n_records` from `placed_records` in both planes, optionally strips alpha, and checks the
/// RGB report `recovered / n_records` and the alpha report against the alpha grid's count.
fn check_report(res: u32, strip: bool, rgb_report: (usize, u16), alpha_report: (usize, u32)) {
    let n = placed_records(res, res, PAYLOAD_LEN);
    let record = Record {
        flags: Flags {
            variant: Variant::Tiled,
            source: false,
        },
        n_records: n,
        payload: payload(PAYLOAD_LEN),
    };
    let bytes = encode(&record).expect("encodes");
    assert_eq!(bytes.len(), 402, "§2: a config-only record is 402 B");
    let mut img = Image::new(res);
    img.write_plane(&bytes, 3);
    img.write_plane(&bytes, 1);
    if strip {
        img.strip_alpha();
    }
    let rgb = img.recover(PAYLOAD_LEN, RGB_BITS_PER_PIXEL);
    assert!(
        rgb.iter().all(|r| *r == record),
        "{res}²: an RGB record came back changed"
    );
    let reported = rgb.first().map(|r| r.n_records).unwrap_or(0);
    assert_eq!(
        (rgb.len(), reported),
        rgb_report,
        "{res}² (alpha stripped: {strip}): the RGB report is {}/{reported}, not §2's",
        rgb.len()
    );
    let alpha = img.recover(PAYLOAD_LEN, ALPHA_BITS_PER_PIXEL);
    assert!(
        alpha.iter().all(|r| *r == record),
        "{res}²: an alpha record came back changed"
    );
    let (ac, ar) = tile_grid(
        res,
        res,
        tile_side(record_bits(PAYLOAD_LEN), ALPHA_BITS_PER_PIXEL),
    );
    assert_eq!(
        (alpha.len(), ac * ar),
        alpha_report,
        "{res}² (alpha stripped: {strip}): the alpha report is {}/{}, not §2's",
        alpha.len(),
        ac * ar
    );
}

#[test]
fn embed_tile_side_qa_report_pristine_128_is_9_of_9() {
    // §2: a pristine 128² image reports 9/9; its alpha plane, separately, 4 of 4.
    check_report(128, false, (9, 9), (4, 4));
}

negative_control!(
    embed_tile_side_qa_report_pristine_128_is_9_of_9,
    "a 128² report over both planes (13/13) must fail",
    expected = "not §2's",
    check_report(128, false, (13, 13), (4, 4))
);

#[test]
fn embed_tile_side_qa_report_alpha_stripped_512_is_225_of_225() {
    // §2: a 512² image with its alpha stripped reports 225/225; the alpha plane loses only its own copies. At 512²
    // the alpha side is 57 and its grid ⌊512 / 57⌋² = 64.
    assert_eq!(doc_tiles(512, 512, PAYLOAD_LEN, 1), 64);
    check_report(512, true, (225, 225), (0, 64));
}

negative_control!(
    embed_tile_side_qa_report_alpha_stripped_512_is_225_of_225,
    "a stripped 512² image whose RGB report counts alpha's 64 must fail",
    expected = "not §2's",
    check_report(512, true, (225, 289), (0, 64))
);
