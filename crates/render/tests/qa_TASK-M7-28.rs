//! QA tests for TASK-M7-28, written from REQ-TOOL-059, REQ-TOOL-061 and REQ-TOOL-071 against their source,
//! `principia_dd_image_embedding.md` §2, §3, §4 and §9, not from the implementation. Every record and every bit
//! position is built here by hand from §2's table and "Within a tile" / "Tile side and the grid": an own CRC-32
//! (PNG's), big-endian integers, the doc's magic and version, `side = ceil(sqrt(ceil(record_bits / b)))`, the grid
//! from the top-left with `floor(width / side)` × `floor(height / side)` tiles. The only crate items used are the
//! public writer (`embed`), reader (`read`) and the `Image` they work on. Test names carry the acceptance filters
//! (`embed_rgb_alpha`, `embed_majority_vote`, `embed_outcomes`). Each test has a registered negative control (R-176).

use render::embed::reader::{read, Dihedral, Outcome};
use render::embed::writer::embed;
use render::embed::Image;
use validation::negative_control;

// ---------------------------------------------------------------------------------------------------------------
// §2's values and layout, by hand.

/// §2, "Magic and version": the proposed magic and version (R-71, R-380).
const DOC_MAGIC: [u8; 4] = [0x8F, 0x50, 0x72, 0x6E];
const DOC_VERSION: u8 = 3;
/// §7: the measured config-only payload, deflated.
const CONFIG: usize = 382;

/// §2, "The CRC is PNG's": reflected polynomial 0xEDB88320, init and final XOR 0xFFFFFFFF.
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

fn check_png_crc(crc: fn(&[u8]) -> u32) {
    assert_eq!(crc(b"123456789"), 0xCBF4_3926, "not PNG's CRC-32");
}

#[test]
fn embed_outcomes_qa_own_crc_is_pngs() {
    check_png_crc(own_crc);
}

negative_control!(
    embed_outcomes_qa_own_crc_is_pngs,
    "a CRC without the final XOR is not PNG's: the check must fail",
    expected = "not PNG's CRC-32",
    check_png_crc(|b| own_crc(b) ^ 0xFFFF_FFFF)
);

/// §2's record: `magic ‖ version ‖ flags ‖ payload_len(u32 BE) ‖ n_records(u16 BE) ‖ crc32(0..12) ‖ payload ‖
/// crc32(payload)`.
fn by_hand(version: u8, flags: u8, n_records: u16, payload: &[u8]) -> Vec<u8> {
    let mut v = DOC_MAGIC.to_vec();
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

/// A noisy image: every channel, alpha included, pseudo-random, so that a write to a slot the doc says is not
/// written shows up.
fn noisy(width: u32, height: u32, alpha: bool, seed: u32) -> Image {
    let ch = if alpha { 4 } else { 3 };
    Image::new(
        width,
        height,
        alpha,
        noise((width * height * ch) as usize, seed),
    )
}

/// `ceil(sqrt(n))` in integers.
fn ceil_sqrt(n: u64) -> u64 {
    let mut s = (n as f64).sqrt() as u64;
    while s * s < n {
        s += 1;
    }
    while s > 0 && (s - 1) * (s - 1) >= n {
        s -= 1;
    }
    s
}

/// §2, "Tile side and the grid": `side = ceil(sqrt(ceil(record_bits / b)))`, `record_bits = 8 × (20 + payload_len)`.
fn doc_side(payload_len: usize, b: u64) -> u32 {
    let bits = 8 * (16 + payload_len as u64 + 4);
    ceil_sqrt(bits.div_ceil(b)) as u32
}

/// The bits a pixel holds in a plane: 3 in RGB, 1 in alpha.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum P {
    Rgb,
    Alpha,
}

/// §2, "Within a tile": the byte index in the pixel buffer of record bit `i` of tile `(col, row)` of side `side`.
fn slot(img: &Image, plane: P, col: u32, row: u32, side: u32, i: u64) -> usize {
    let ch = if img.has_alpha() { 4 } else { 3 };
    let (p, channel) = match plane {
        P::Rgb => (i / 3, (i % 3) as usize),
        P::Alpha => (i, 3),
    };
    let x = u64::from(col * side) + p % u64::from(side);
    let y = u64::from(row * side) + p / u64::from(side);
    ((y * u64::from(img.width()) + x) as usize) * ch + channel
}

/// Writes `bytes` (MSB first) into tile `(col, row)` of `plane` by the doc's layout.
fn put(img: &mut Image, plane: P, col: u32, row: u32, side: u32, bytes: &[u8]) {
    for i in 0..8 * bytes.len() as u64 {
        let bit = (bytes[(i / 8) as usize] >> (7 - i % 8)) & 1;
        let at = slot(img, plane, col, row, side, i);
        let px = img.pixels_mut();
        px[at] = (px[at] & !1) | bit;
    }
}

/// Reads `n` bytes (MSB first) from tile `(col, row)` of `plane` by the doc's layout.
fn get(img: &Image, plane: P, col: u32, row: u32, side: u32, n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n];
    for i in 0..8 * n as u64 {
        let at = slot(img, plane, col, row, side, i);
        out[(i / 8) as usize] |= (img.pixels()[at] & 1) << (7 - i % 8);
    }
    out
}

/// The grid's tiles, row by row from the top-left.
fn grid(img: &Image, side: u32) -> Vec<(u32, u32)> {
    let (cols, rows) = (img.width() / side, img.height() / side);
    (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .collect()
}

/// The image with its alpha channel dropped, by hand.
fn strip_alpha(img: &Image) -> Image {
    assert!(img.has_alpha());
    let px = img
        .pixels()
        .chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    Image::new(img.width(), img.height(), false, px)
}

/// The recovered payload, `n_records` and version, or a panic naming what was reported instead.
fn recovered(outcome: &Outcome) -> &render::embed::reader::Recovered {
    match outcome {
        Outcome::Recovered(r) => r,
        other => panic!("not recovered: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-059: payload in the low bit of every RGB channel as whole records in a 2-D square tile grid; alpha a
// second systematic copy. Verify: embed then read with alpha stripped, RGB alone recovers the payload.

/// The embedded image holds, in every tile of each plane's grid, §2's record with `n_records` = the RGB tile count,
/// variant tiled; every other byte of the image is unchanged, and no channel moved by more than 1.
fn check_layout(
    before: &Image,
    after: &Image,
    payload: &[u8],
    rgb_tiles: usize,
    alpha_tiles: usize,
) {
    let rgb_side = doc_side(payload.len(), 3);
    let rgb_grid = grid(before, rgb_side);
    assert_eq!(rgb_grid.len(), rgb_tiles, "RGB grid size differs from §3");
    let expected = by_hand(DOC_VERSION, 0, rgb_tiles as u16, payload);
    let mut written = vec![false; before.pixels().len()];
    for &(c, r) in &rgb_grid {
        assert_eq!(
            get(after, P::Rgb, c, r, rgb_side, expected.len()),
            expected,
            "RGB tile ({c}, {r}) does not hold the doc's record"
        );
        for i in 0..8 * expected.len() as u64 {
            written[slot(after, P::Rgb, c, r, rgb_side, i)] = true;
        }
    }
    if before.has_alpha() {
        let a_side = doc_side(payload.len(), 1);
        let a_grid = grid(before, a_side);
        assert_eq!(a_grid.len(), alpha_tiles, "alpha grid size differs from §2");
        for &(c, r) in &a_grid {
            assert_eq!(
                get(after, P::Alpha, c, r, a_side, expected.len()),
                expected,
                "alpha tile ({c}, {r}) does not hold the doc's record"
            );
            for i in 0..8 * expected.len() as u64 {
                written[slot(after, P::Alpha, c, r, a_side, i)] = true;
            }
        }
    }
    for (k, (&b, &a)) in before.pixels().iter().zip(after.pixels()).enumerate() {
        assert_eq!(b & !1, a & !1, "byte {k}: more than the low bit changed");
        if !written[k] {
            assert_eq!(b, a, "byte {k} is no record slot but was written");
        }
    }
}

#[test]
fn embed_rgb_alpha_qa_tiles_hold_doc_records() {
    // §7: a 402 B record gives side 33 and 1, 9, 49 tiles at 64², 128², 256²; alpha 4 at 128² (§2). Non-square too.
    for (w, h, rgb, alpha, seed) in [
        (64u32, 64u32, 1usize, 1usize, 1u32),
        (128, 128, 9, 4, 2),
        (256, 256, 49, 16, 3),
        (100, 70, 6, 1, 4),
    ] {
        assert_eq!(doc_side(CONFIG, 3), 33);
        let payload = noise(CONFIG, seed + 100);
        let before = noisy(w, h, true, seed);
        let mut after = before.clone();
        embed(&mut after, &payload, false).expect("embeds");
        check_layout(&before, &after, &payload, rgb, alpha);
    }
    // A short payload: smaller tiles, more of them.
    let payload = noise(10, 9);
    let before = noisy(64, 64, true, 9);
    let mut after = before.clone();
    embed(&mut after, &payload, false).expect("embeds");
    let s3 = doc_side(10, 3);
    let s1 = doc_side(10, 1);
    check_layout(
        &before,
        &after,
        &payload,
        ((64 / s3) * (64 / s3)) as usize,
        ((64 / s1) * (64 / s1)) as usize,
    );
}

negative_control!(
    embed_rgb_alpha_qa_tiles_hold_doc_records,
    "an image whose last RGB tile lost one record bit must fail the layout check",
    expected = "does not hold the doc's record",
    {
        let payload = noise(CONFIG, 102);
        let before = noisy(128, 128, true, 2);
        let mut after = before.clone();
        embed(&mut after, &payload, false).unwrap();
        let at = slot(&after, P::Rgb, 2, 2, 33, 8 * 100 + 3);
        after.pixels_mut()[at] ^= 1;
        check_layout(&before, &after, &payload, 9, 4);
    }
);

/// RGB alone recovers the payload with every RGB tile counted, and reports no alpha plane.
fn check_rgb_alone(outcome: &Outcome, payload: &[u8], tiles: u32) {
    let r = recovered(outcome);
    assert_eq!(
        r.record.payload, payload,
        "RGB alone recovered the wrong payload"
    );
    assert_eq!(
        r.record.n_records as u32, tiles,
        "n_records differs from the RGB tile count"
    );
    assert_eq!(r.version, DOC_VERSION);
    assert_eq!(
        (r.how.tiles.found, r.how.tiles.of),
        (tiles, tiles),
        "tiles count is not {tiles}/{tiles}"
    );
    assert!(
        r.how.alpha.is_none(),
        "an alpha count reported on an image with no alpha"
    );
}

#[test]
fn embed_rgb_alpha_qa_stripped_rgb_alone_recovers() {
    // §2 / §7: a pristine 128² image reports 9/9; a 512² image with its alpha stripped reports 225/225.
    for (n, tiles, seed) in [(128u32, 9u32, 11u32), (512, 225, 12)] {
        let payload = noise(CONFIG, seed);
        let mut img = noisy(n, n, true, seed);
        embed(&mut img, &payload, false).expect("embeds");
        check_rgb_alone(&read(&strip_alpha(&img)), &payload, tiles);
    }
    // Stripped and then also its whole alpha plane's content gone: an RGB-only image embedded directly.
    let payload = noise(CONFIG, 13);
    let mut img = noisy(128, 128, false, 13);
    embed(&mut img, &payload, false).expect("embeds an RGB image");
    check_rgb_alone(&read(&img), &payload, 9);
}

negative_control!(
    embed_rgb_alpha_qa_stripped_rgb_alone_recovers,
    "with alpha stripped and the RGB low bits re-randomised nothing can be recovered: the check must fail",
    expected = "not recovered",
    {
        let payload = noise(CONFIG, 11);
        let mut img = noisy(128, 128, true, 11);
        embed(&mut img, &payload, false).unwrap();
        let mut rgb = strip_alpha(&img);
        let fresh = noise(rgb.pixels().len(), 999);
        for (p, f) in rgb.pixels_mut().iter_mut().zip(fresh) {
            *p = (*p & !1) | (f & 1);
        }
        check_rgb_alone(&read(&rgb), &payload, 9);
    }
);

/// The alpha plane is a systematic copy: it alone carries the whole payload.
fn check_alpha_alone(outcome: &Outcome, payload: &[u8], alpha_tiles: u32) {
    let r = recovered(outcome);
    assert_eq!(
        r.record.payload, payload,
        "alpha alone recovered the wrong payload"
    );
    let a = r.how.alpha.expect("an alpha count on an RGBA image");
    assert_eq!((a.found, a.of), (alpha_tiles, alpha_tiles), "alpha count");
    assert_eq!(
        r.how.tiles.found, 0,
        "RGB records counted after the RGB plane was wiped"
    );
}

#[test]
fn embed_rgb_alpha_qa_alpha_is_systematic_copy() {
    let payload = noise(CONFIG, 21);
    let mut img = noisy(128, 128, true, 21);
    embed(&mut img, &payload, false).expect("embeds");
    // Wipe every RGB low bit to 0 (all-zero low plane never reads as the magic, §2).
    for (k, p) in img.pixels_mut().iter_mut().enumerate() {
        if k % 4 != 3 {
            *p &= !1;
        }
    }
    check_alpha_alone(&read(&img), &payload, 4);
}

negative_control!(
    embed_rgb_alpha_qa_alpha_is_systematic_copy,
    "with both planes wiped the alpha-alone check must fail",
    expected = "not recovered",
    {
        let payload = noise(CONFIG, 21);
        let mut img = noisy(128, 128, true, 21);
        embed(&mut img, &payload, false).unwrap();
        for p in img.pixels_mut() {
            *p &= !1;
        }
        check_alpha_alone(&read(&img), &payload, 4);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-061: collect every intact record and majority-vote per byte. Verify: with a minority of records carrying
// consistent corrupted-but-CRC-passing bytes, the majority wins.

/// An RGB-only 128² image (9 tiles of side 33 for a 382 B payload) with `records[k]` placed by hand in tile `k`,
/// row by row; the remaining tiles keep the noise.
fn forged(records: &[Vec<u8>], alpha: &[Vec<u8>], seed: u32) -> Image {
    let mut img = noisy(128, 128, !alpha.is_empty(), seed);
    if !alpha.is_empty() {
        // Zero the alpha low plane so only the placed alpha records sit there.
        for (k, p) in img.pixels_mut().iter_mut().enumerate() {
            if k % 4 == 3 {
                *p &= !1;
            }
        }
    }
    let g = grid(&img, 33);
    for (bytes, &(c, r)) in records.iter().zip(&g) {
        put(&mut img, P::Rgb, c, r, 33, bytes);
    }
    let ga = grid(&img, doc_side(CONFIG, 1));
    for (bytes, &(c, r)) in alpha.iter().zip(&ga) {
        put(&mut img, P::Alpha, c, r, doc_side(CONFIG, 1), bytes);
    }
    img
}

/// `payload` with byte `at` changed, CRCs recomputed: corrupted but CRC-passing.
fn forge(payload: &[u8], at: usize, xor: u8) -> Vec<u8> {
    let mut p = payload.to_vec();
    p[at] ^= xor;
    by_hand(DOC_VERSION, 0, 9, &p)
}

fn check_majority_wins(outcome: &Outcome, genuine: &[u8]) {
    let r = recovered(outcome);
    assert_eq!(r.record.payload, genuine, "the majority did not win");
}

#[test]
fn embed_majority_vote_qa_consistent_minority_loses() {
    let genuine = noise(CONFIG, 31);
    let good = by_hand(DOC_VERSION, 0, 9, &genuine);
    let bad = forge(&genuine, 200, 0x5A);
    // 5 genuine against 4 consistent forgeries, the forgeries in the first tiles the reader meets.
    let mut recs = vec![bad.clone(); 4];
    recs.extend(std::iter::repeat_n(good.clone(), 5));
    check_majority_wins(&read(&forged(&recs, &[], 31)), &genuine);
    // Forgeries last.
    let mut recs = vec![good.clone(); 5];
    recs.extend(std::iter::repeat_n(bad.clone(), 4));
    check_majority_wins(&read(&forged(&recs, &[], 32)), &genuine);
    // 2 genuine, 1 forged, the rest of the tiles noise: every intact record collected, not a fixed count.
    check_majority_wins(
        &read(&forged(&[bad.clone(), good.clone(), good.clone()], &[], 33)),
        &genuine,
    );
}

negative_control!(
    embed_majority_vote_qa_consistent_minority_loses,
    "when the consistent forgeries are the majority, the genuine payload must not be reported",
    expected = "the majority did not win",
    {
        let genuine = noise(CONFIG, 31);
        let good = by_hand(DOC_VERSION, 0, 9, &genuine);
        let bad = forge(&genuine, 200, 0x5A);
        let mut recs = vec![bad; 5];
        recs.extend(std::iter::repeat_n(good, 4));
        check_majority_wins(&read(&forged(&recs, &[], 31)), &genuine);
    }
);

/// `payload` with byte `at` XORed by `xor` and the four bytes at `fix..fix + 4` adjusted so the payload's CRC-32 is
/// unchanged: a forgery whose every byte outside those five matches the genuine record, its CRC bytes included.
/// CRC-32 is affine over GF(2), so the change a difference makes to it is linear in the difference, and any 32
/// consecutive bits can absorb it (a 32 × 32 solve).
fn crc_preserving(payload: &[u8], at: usize, xor: u8, fix: usize) -> Vec<u8> {
    let delta = |d: &[u8]| {
        let mut q = payload.to_vec();
        for (a, b) in q.iter_mut().zip(d) {
            *a ^= b;
        }
        own_crc(&q) ^ own_crc(payload)
    };
    let mut e = vec![0u8; payload.len()];
    e[at] = xor;
    let target = delta(&e);
    // Columns: the CRC change of each single bit of the window. Solve sum(x_j col_j) = target by elimination.
    let mut rows: Vec<(u32, u32)> = (0..32)
        .map(|j| {
            let mut d = vec![0u8; payload.len()];
            d[fix + j / 8] = 0x80 >> (j % 8);
            (delta(&d), 1u32 << j)
        })
        .collect();
    let mut want = (target, 0u32);
    for bit in (0..32).rev() {
        let Some(k) = (0..rows.len()).find(|&k| rows[k].0 >> bit & 1 == 1) else {
            continue;
        };
        let pivot = rows.remove(k);
        for r in rows.iter_mut() {
            if r.0 >> bit & 1 == 1 {
                *r = (r.0 ^ pivot.0, r.1 ^ pivot.1);
            }
        }
        if want.0 >> bit & 1 == 1 {
            want = (want.0 ^ pivot.0, want.1 ^ pivot.1);
        }
    }
    assert_eq!(want.0, 0, "the window cannot absorb the CRC change");
    let mut q = payload.to_vec();
    q[at] ^= xor;
    for j in 0..32 {
        if want.1 >> j & 1 == 1 {
            q[fix + j / 8] ^= 0x80 >> (j % 8);
        }
    }
    assert_eq!(own_crc(&q), own_crc(payload), "forgery changed the CRC");
    assert_ne!(q, payload);
    q
}

#[test]
fn embed_majority_vote_qa_vote_is_per_byte() {
    // 2 genuine records and 3 forgeries with the genuine payload CRC, each wrong in its own five bytes: no whole
    // record holds a majority (2 of 5), but every byte's genuine value, CRC bytes included, is held by at least 4 of
    // 5. Only a per-byte vote recovers the genuine payload.
    let genuine = noise(CONFIG, 41);
    let good = by_hand(DOC_VERSION, 0, 9, &genuine);
    let f = |at, xor, fix| by_hand(DOC_VERSION, 0, 9, &crc_preserving(&genuine, at, xor, fix));
    let recs = vec![
        f(5, 0x01, 10),
        good.clone(),
        f(150, 0x80, 160),
        good,
        f(300, 0xFF, 370),
    ];
    check_majority_wins(&read(&forged(&recs, &[], 41)), &genuine);
}

negative_control!(
    embed_majority_vote_qa_vote_is_per_byte,
    "three identical forgeries against two genuine records outvote it byte by byte: the check must fail",
    expected = "the majority did not win",
    {
        let genuine = noise(CONFIG, 41);
        let good = by_hand(DOC_VERSION, 0, 9, &genuine);
        let bad = by_hand(DOC_VERSION, 0, 9, &crc_preserving(&genuine, 150, 0x80, 160));
        let recs = vec![bad.clone(), good.clone(), bad.clone(), good, bad];
        check_majority_wins(&read(&forged(&recs, &[], 41)), &genuine);
    }
);

#[test]
fn embed_majority_vote_qa_alpha_records_join_the_vote() {
    // RGB: 4 genuine, 5 forged (an RGB-only vote would pick the forgery). Alpha: 4 genuine. Every intact record of
    // both planes votes: 8 genuine against 5.
    let genuine = noise(CONFIG, 51);
    let good = by_hand(DOC_VERSION, 0, 9, &genuine);
    let bad = forge(&genuine, 17, 0x33);
    let mut recs = vec![bad; 5];
    recs.extend(std::iter::repeat_n(good.clone(), 4));
    let alpha = vec![good; 4];
    check_majority_wins(&read(&forged(&recs, &alpha, 51)), &genuine);
}

negative_control!(
    embed_majority_vote_qa_alpha_records_join_the_vote,
    "with alpha stripped the RGB forgeries are the majority: the check must fail",
    expected = "the majority did not win",
    {
        let genuine = noise(CONFIG, 51);
        let good = by_hand(DOC_VERSION, 0, 9, &genuine);
        let bad = forge(&genuine, 17, 0x33);
        let mut recs = vec![bad; 5];
        recs.extend(std::iter::repeat_n(good.clone(), 4));
        let alpha = vec![good; 4];
        check_majority_wins(&read(&strip_alpha(&forged(&recs, &alpha, 51))), &genuine);
    }
);

#[test]
fn embed_majority_vote_qa_damaged_records_are_discarded_whole() {
    // 8 of 9 RGB tiles damaged so their CRC fails (each a different bit), one intact: the intact one is the vote.
    let payload = noise(CONFIG, 61);
    let mut img = noisy(128, 128, false, 61);
    embed(&mut img, &payload, false).expect("embeds");
    for (k, (c, r)) in grid(&img, 33).into_iter().enumerate().take(8) {
        let at = slot(&img, P::Rgb, c, r, 33, 8 * (16 + 7 * k as u64) + 2);
        img.pixels_mut()[at] ^= 1;
    }
    check_majority_wins(&read(&img), &payload);
}

negative_control!(
    embed_majority_vote_qa_damaged_records_are_discarded_whole,
    "with the last tile damaged too nothing intact remains: the check must fail",
    expected = "not recovered",
    {
        let payload = noise(CONFIG, 61);
        let mut img = noisy(128, 128, false, 61);
        embed(&mut img, &payload, false).unwrap();
        for (k, (c, r)) in grid(&img, 33).into_iter().enumerate() {
            let at = slot(&img, P::Rgb, c, r, 33, 8 * (16 + 7 * k as u64) + 2);
            img.pixels_mut()[at] ^= 1;
        }
        check_majority_wins(&read(&img), &payload);
    }
);

// ---------------------------------------------------------------------------------------------------------------
// REQ-TOOL-071: exactly one of three distinguishable outcomes. Verify: plain PNG → none; all records corrupted →
// corrupt; valid → recovered + transform report.

fn check_none(outcome: &Outcome) {
    assert!(
        matches!(outcome, Outcome::None),
        "not 'no embedded state': {outcome:?}"
    );
}

#[test]
fn embed_outcomes_qa_plain_image_is_none() {
    for (w, h, alpha, seed) in [
        (128u32, 128u32, true, 71u32),
        (128, 128, false, 72),
        (256, 96, true, 73),
        (20, 20, true, 74),
        (1, 1, false, 75),
    ] {
        check_none(&read(&noisy(w, h, alpha, seed)));
    }
    // Blank and saturated planes (§2: never read as the magic).
    for v in [0u8, 255] {
        check_none(&read(&Image::new(64, 64, true, vec![v; 64 * 64 * 4])));
    }
}

negative_control!(
    embed_outcomes_qa_plain_image_is_none,
    "an embedded image is not 'no embedded state': the check must fail",
    expected = "not 'no embedded state'",
    {
        let mut img = noisy(128, 128, true, 71);
        embed(&mut img, &noise(CONFIG, 1), false).unwrap();
        check_none(&read(&img));
    }
);

fn check_corrupt(outcome: &Outcome) {
    assert!(
        matches!(outcome, Outcome::Corrupt(_)),
        "not 'present, corrupt': {outcome:?}"
    );
}

/// Every record of both planes damaged, each by one flipped bit at `bit` (record bit index).
fn all_damaged(bit: u64, seed: u32) -> Image {
    let mut img = noisy(128, 128, true, seed);
    embed(&mut img, &noise(CONFIG, seed), false).expect("embeds");
    for (c, r) in grid(&img, 33) {
        let at = slot(&img, P::Rgb, c, r, 33, bit);
        img.pixels_mut()[at] ^= 1;
    }
    let a = doc_side(CONFIG, 1);
    for (c, r) in grid(&img, a) {
        let at = slot(&img, P::Alpha, c, r, a, bit);
        img.pixels_mut()[at] ^= 1;
    }
    img
}

#[test]
fn embed_outcomes_qa_all_records_corrupted_is_corrupt() {
    // A payload bit (payload CRC fails), a payload_len bit and a header-CRC bit (header CRC fails), the payload CRC.
    for (bit, seed) in [
        (8 * 16 + 5, 81u32),
        (8 * 200, 82),
        (8 * 7 + 1, 83),
        (8 * 13, 84),
        (8 * (16 + CONFIG as u64) + 30, 85),
    ] {
        check_corrupt(&read(&all_damaged(bit, seed)));
    }
    // Also an RGB-only image, every record damaged.
    check_corrupt(&read(&strip_alpha(&all_damaged(8 * 20, 86))));
}

negative_control!(
    embed_outcomes_qa_all_records_corrupted_is_corrupt,
    "an undamaged embedded image is not corrupt: the check must fail",
    expected = "not 'present, corrupt'",
    {
        let mut img = noisy(128, 128, true, 81);
        embed(&mut img, &noise(CONFIG, 81), false).unwrap();
        check_corrupt(&read(&img));
    }
);

/// Recovered, the payload exact, and the report says how: the identity transform, decimation 1, tiles n/n and the
/// alpha count.
fn check_report(outcome: &Outcome, payload: &[u8], tiles: u32, alpha: Option<u32>) {
    let r = recovered(outcome);
    assert_eq!(r.record.payload, payload, "wrong payload");
    let t = r.how.transform;
    assert_eq!(
        (t.dx, t.dy, t.dihedral, t.decimate),
        (0, 0, Dihedral::Identity, 1),
        "transform report is not the identity on an untransformed image"
    );
    assert_eq!(
        (r.how.tiles.found, r.how.tiles.of),
        (tiles, tiles),
        "tiles report"
    );
    assert_eq!(
        r.how.alpha.map(|a| (a.found, a.of)),
        alpha.map(|a| (a, a)),
        "alpha report"
    );
}

#[test]
fn embed_outcomes_qa_valid_is_recovered_with_report() {
    for (n, tiles, alpha, seed) in [
        (64u32, 1u32, 1u32, 91u32),
        (128, 9, 4, 92),
        (256, 49, 16, 93),
    ] {
        let payload = noise(CONFIG, seed);
        let mut img = noisy(n, n, true, seed);
        embed(&mut img, &payload, false).expect("embeds");
        check_report(&read(&img), &payload, tiles, Some(alpha));
    }
}

negative_control!(
    embed_outcomes_qa_valid_is_recovered_with_report,
    "an image with one RGB tile damaged must not report 9/9: the check must fail",
    expected = "tiles report",
    {
        let payload = noise(CONFIG, 92);
        let mut img = noisy(128, 128, true, 92);
        embed(&mut img, &payload, false).unwrap();
        let at = slot(&img, P::Rgb, 1, 1, 33, 8 * 40);
        img.pixels_mut()[at] ^= 1;
        check_report(&read(&img), &payload, 9, Some(4));
    }
);

/// The three outcomes are distinguishable on one image as it degrades: recovered → corrupt → none.
fn check_three(outcomes: [&Outcome; 3]) {
    assert!(
        matches!(outcomes[0], Outcome::Recovered(_)),
        "first not recovered"
    );
    assert!(
        matches!(outcomes[1], Outcome::Corrupt(_)),
        "second not corrupt"
    );
    assert!(matches!(outcomes[2], Outcome::None), "third not none");
}

#[test]
fn embed_outcomes_qa_three_on_one_image() {
    let payload = noise(CONFIG, 101);
    let mut img = noisy(128, 128, true, 101);
    embed(&mut img, &payload, false).expect("embeds");
    let valid = read(&img);
    let damaged = all_damaged(8 * 30, 101);
    let mut wiped = img.clone();
    for p in wiped.pixels_mut() {
        *p &= !1;
    }
    check_three([&valid, &read(&damaged), &read(&wiped)]);
}

negative_control!(
    embed_outcomes_qa_three_on_one_image,
    "a damaged image read in place of the valid one must fail",
    expected = "first not recovered",
    {
        let damaged = read(&all_damaged(8 * 30, 101));
        let none = read(&noisy(64, 64, true, 1));
        check_three([&damaged, &damaged, &none]);
    }
);
