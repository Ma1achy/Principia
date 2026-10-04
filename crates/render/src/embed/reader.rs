//! The majority-vote reader and its three outcomes (`principia_dd_image_embedding.md` §2, §4, §9; REQ-TOOL-061,
//! REQ-TOOL-071).
//!
//! The reader collects every intact record from each plane the image has, RGB and alpha, and majority-votes per byte
//! (§2, "Read"). It reports exactly one of three outcomes (§9): no embedded state, state present but corrupt, or
//! recovered and how. It reads the grid as the writer laid it, from the image's top-left pixel; the offset, dihedral
//! and decimation searches (§4, REQ-TOOL-063) are not part of it, so the transform it reports is the identity.
//!
//! The record's length, and so each plane's tile side, is not known before a record is read: the reader tries every
//! side a record can have, reads the header at the start of each tile, and reads the whole tile only when the header
//! is intact and its `payload_len` gives that side.

use super::record::{
    bit_slot, decode, decode_header, read_tile, record_bits, record_len, tile_grid, tile_origin,
    tile_side, Discard, Record, CRC_LEN, HEADER_LEN, MAGIC,
};
use super::{Image, Plane};

/// What the reader found: exactly one of §9's three outcomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No embedded state: no tile of either plane, at any side a record can have, starts with the magic.
    None,
    /// State present, corrupt: some tile starts with the magic, but no record can be trusted.
    Corrupt(Corrupt),
    /// Recovered, and how.
    Recovered(Recovered),
}

/// State present but corrupt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corrupt {
    /// The intact records found, across both planes.
    pub intact: u32,
    /// Why no record can be trusted.
    pub reason: CorruptReason,
}

/// Why the state present is corrupt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorruptReason {
    /// Tiles start with the magic, but every record is discarded whole (§2).
    NoIntactRecord,
    /// Intact records were found, but no length, or no value of some byte, is held by more than half of them.
    NoMajority,
    /// The voted bytes are not themselves an intact record, so why it is discarded.
    VotedRecordFails(Discard),
}

/// A recovered record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovered {
    /// The record's `version`, as read (§2: an intact record of another layout is reported as such).
    pub version: u8,
    /// The record the vote settled on.
    pub record: Record,
    /// How it was recovered.
    pub how: How,
}

/// How a record was recovered (§4, "the reader reports what it found").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct How {
    /// The transform the grid was read through.
    pub transform: Transform,
    /// The RGB plane's records that agree with the vote, against the `n_records` the record carries (§4's
    /// `tiles: 9/9`).
    pub tiles: Count,
    /// The alpha plane's records that agree with the vote, against the tiles its grid holds at the alpha side for the
    /// voted record (§2); `None` when the image has no alpha channel.
    pub alpha: Option<Count>,
    /// The intact records, across both planes, whose bytes differ from the vote.
    pub outvoted: u32,
}

/// A count of records found against the count expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    /// The records found.
    pub found: u32,
    /// The records expected.
    pub of: u32,
}

/// The transform through which the reader read the grid (§4's three searches).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    /// The grid's origin, in pixels from the image's left edge.
    pub dx: u32,
    /// The grid's origin, in pixels from the image's top edge.
    pub dy: u32,
    /// The dihedral transform undone.
    pub dihedral: Dihedral,
    /// The decimation factor `s`: every `s`-th pixel of every `s`-th row.
    pub decimate: u32,
}

impl Transform {
    /// The grid as the writer laid it: no offset, no dihedral transform, no decimation.
    pub const IDENTITY: Self = Self {
        dx: 0,
        dy: 0,
        dihedral: Dihedral::Identity,
        decimate: 1,
    };
}

/// The eight dihedral transforms of the square (§4: rot 90/180/270, flip, transpose).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dihedral {
    /// No transform.
    Identity,
    /// Rotated 90°.
    Rot90,
    /// Rotated 180°.
    Rot180,
    /// Rotated 270°.
    Rot270,
    /// Flipped left to right.
    FlipH,
    /// Flipped top to bottom.
    FlipV,
    /// Reflected in the main diagonal.
    Transpose,
    /// Reflected in the anti-diagonal.
    AntiTranspose,
}

/// An intact record's bytes, `header ‖ payload ‖ crc32(payload)`, and the plane it came from.
struct Found {
    plane: Plane,
    bytes: Vec<u8>,
}

/// Reads `image`'s embedded state and reports exactly one of the three outcomes (§9).
pub fn read(image: &Image) -> Outcome {
    let mut present = false;
    let mut found = Vec::new();
    for &plane in image.planes() {
        present |= scan(image, plane, &mut found);
    }
    if found.is_empty() {
        return if present {
            Outcome::Corrupt(Corrupt {
                intact: 0,
                reason: CorruptReason::NoIntactRecord,
            })
        } else {
            Outcome::None
        };
    }
    let intact = u32::try_from(found.len()).unwrap_or(u32::MAX);
    let corrupt = |reason| Outcome::Corrupt(Corrupt { intact, reason });
    let Some(voted) = vote(&found) else {
        return corrupt(CorruptReason::NoMajority);
    };
    let decoded = match decode(&voted) {
        Ok(decoded) => decoded,
        Err(discard) => return corrupt(CorruptReason::VotedRecordFails(discard)),
    };
    let agree = |plane| {
        let n = found
            .iter()
            .filter(|f| f.plane == plane && f.bytes == voted)
            .count();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let tiles = Count {
        found: agree(Plane::Rgb),
        of: u32::from(decoded.record.n_records),
    };
    let alpha = image.has_alpha().then(|| {
        let side = tile_side(
            record_bits(decoded.record.payload.len()),
            Plane::Alpha.bits_per_pixel(),
        );
        let (cols, rows) = tile_grid(image.width(), image.height(), side);
        Count {
            found: agree(Plane::Alpha),
            of: cols * rows,
        }
    });
    let agreed = tiles.found + alpha.map_or(0, |a| a.found);
    Outcome::Recovered(Recovered {
        version: decoded.version,
        record: decoded.record,
        how: How {
            transform: Transform::IDENTITY,
            tiles,
            alpha,
            outvoted: intact - agreed,
        },
    })
}

/// Collects `plane`'s intact records into `found`, trying every tile side a record can have; returns whether any
/// tile starts with the magic.
fn scan(image: &Image, plane: Plane, found: &mut Vec<Found>) -> bool {
    let bpp = plane.bits_per_pixel();
    let smallest = tile_side(record_bits(0), bpp);
    let largest = image.width().min(image.height());
    let mut present = false;
    for side in smallest..=largest {
        let (cols, rows) = tile_grid(image.width(), image.height(), side);
        for row in 0..rows {
            for col in 0..cols {
                if read_prefix(image, plane, col, row, side, MAGIC.len()) != MAGIC {
                    continue;
                }
                present = true;
                let Ok(header) =
                    decode_header(&read_prefix(image, plane, col, row, side, HEADER_LEN))
                else {
                    continue;
                };
                let payload_len =
                    usize::try_from(header.payload_len).expect("a payload_len beyond usize");
                if tile_side(record_bits(payload_len), bpp) != side {
                    continue;
                }
                let bytes = read_tile(&image.tile_lows(plane, col, row, side), side, bpp);
                if decode(&bytes).is_ok() {
                    let len = record_len(payload_len);
                    debug_assert!(len <= bytes.len() && len >= HEADER_LEN + CRC_LEN);
                    found.push(Found {
                        plane,
                        bytes: bytes[..len].to_vec(),
                    });
                }
            }
        }
    }
    present
}

/// The first `n` bytes tile `(col, row)` of side `side` in `plane` spells, each byte most significant bit first, its
/// bits in the slots [`bit_slot`] gives (§2, "Bit order", "Within a tile").
fn read_prefix(image: &Image, plane: Plane, col: u32, row: u32, side: u32, n: usize) -> Vec<u8> {
    let bpp = plane.bits_per_pixel();
    let (ox, oy) = tile_origin(col, row, side);
    let mut bytes = vec![0u8; n];
    for i in 0..8 * n {
        let slot = bit_slot(i as u64, side, bpp);
        bytes[i / 8] |= image.low(plane, ox + slot.x, oy + slot.y, slot.channel) << (7 - i % 8);
    }
    bytes
}

/// The majority vote over the intact records (§2, "Read"): the length more than half of them have, then each byte's
/// value more than half of the records of that length hold. `None` when a length or a byte has no such majority.
fn vote(found: &[Found]) -> Option<Vec<u8>> {
    let len = majority(found.iter().map(|f| f.bytes.len()))?;
    let voters: Vec<&[u8]> = found
        .iter()
        .map(|f| f.bytes.as_slice())
        .filter(|bytes| bytes.len() == len)
        .collect();
    (0..len)
        .map(|i| majority(voters.iter().map(|bytes| bytes[i])))
        .collect()
}

/// The value more than half of `items` hold, if one does (Boyer–Moore's vote, then a count to confirm it).
fn majority<T: Copy + Eq>(items: impl Iterator<Item = T> + Clone) -> Option<T> {
    let mut candidate = None;
    let mut lead = 0usize;
    for item in items.clone() {
        if lead == 0 {
            candidate = Some(item);
            lead = 1;
        } else if candidate == Some(item) {
            lead += 1;
        } else {
            lead -= 1;
        }
    }
    let candidate = candidate?;
    let (held, total) = items.fold((0usize, 0usize), |(held, total), item| {
        (held + usize::from(item == candidate), total + 1)
    });
    (2 * held > total).then_some(candidate)
}
