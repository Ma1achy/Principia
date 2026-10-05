//! The majority-vote reader and its three outcomes (`principia_dd_image_embedding.md` §2, §4, §9; REQ-TOOL-061,
//! REQ-TOOL-063, REQ-TOOL-071).
//!
//! The reader runs §4's three searches ([`search`](super::search)) and collects every intact record the first
//! candidate that verifies holds, from each plane the image has, RGB and alpha, then majority-votes per byte (§2,
//! "Read"). It reports exactly one of three outcomes (§9): no embedded state, state present but corrupt, or recovered
//! and how, the how being the transform the search found.

use super::record::{decode, record_bits, tile_grid, tile_side, Discard, Record};
use super::search::{search, Hit};
use super::{Image, Plane};

/// What the reader found: exactly one of §9's three outcomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No embedded state: no tile of either plane, at any side a record can have, starts with the magic in the grid
    /// the writer lays from the image's top-left pixel, and in no view the search tried does a tile, at any pixel and
    /// any side, start with an intact header ([`Search::present`](super::search::Search::present)).
    None,
    /// State present, corrupt: state is present ([`Search::present`](super::search::Search::present)), but no record
    /// can be trusted.
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
    /// State is present, but every record is discarded whole (§2).
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
///
/// `dx` and `dy` are in the pixels of the view the records were read in, the image decimated and its dihedral
/// transform undone. They are the RGB plane's grid origin when an RGB record agrees with the vote, and otherwise the
/// alpha plane's, whose tiles have their own side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    /// The grid's origin, in pixels from the view's left edge.
    pub dx: u32,
    /// The grid's origin, in pixels from the view's top edge.
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

/// The eight dihedral transforms of the square (§4: rot 90/180/270, flip, transpose), each the transform the image
/// underwent; rotations are counter-clockwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dihedral {
    /// No transform.
    Identity,
    /// Rotated 90° counter-clockwise.
    Rot90,
    /// Rotated 180°.
    Rot180,
    /// Rotated 270° counter-clockwise, 90° clockwise.
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

/// Reads `image`'s embedded state and reports exactly one of the three outcomes (§9).
pub fn read(image: &Image) -> Outcome {
    let searched = search(image);
    let Some(view) = searched.found else {
        return if searched.present {
            Outcome::Corrupt(Corrupt {
                intact: 0,
                reason: CorruptReason::NoIntactRecord,
            })
        } else {
            Outcome::None
        };
    };
    let found = view.hits;
    let intact = u32::try_from(found.len()).unwrap_or(u32::MAX);
    let corrupt = |reason| Outcome::Corrupt(Corrupt { intact, reason });
    let Some(voted) = vote(&found) else {
        return corrupt(CorruptReason::NoMajority);
    };
    let decoded = match decode(&voted) {
        Ok(decoded) => decoded,
        Err(discard) => return corrupt(CorruptReason::VotedRecordFails(discard)),
    };
    let agree = |plane| u32::try_from(agreeing(&found, plane, &voted).count()).unwrap_or(u32::MAX);
    let origin = |plane| agreeing(&found, plane, &voted).next().map(Hit::grid_origin);
    let tiles = Count {
        found: agree(Plane::Rgb),
        of: u32::from(decoded.record.n_records),
    };
    let alpha = image.has_alpha().then(|| {
        let side = tile_side(
            record_bits(decoded.record.payload.len()),
            Plane::Alpha.bits_per_pixel(),
        );
        let (ax, ay) = origin(Plane::Alpha).unwrap_or((0, 0));
        let (cols, rows) = tile_grid(view.width - ax, view.height - ay, side);
        Count {
            found: agree(Plane::Alpha),
            of: cols * rows,
        }
    });
    let agreed = tiles.found + alpha.map_or(0, |a| a.found);
    let (dx, dy) = origin(Plane::Rgb)
        .or_else(|| origin(Plane::Alpha))
        .unwrap_or((0, 0));
    Outcome::Recovered(Recovered {
        version: decoded.version,
        record: decoded.record,
        how: How {
            transform: Transform {
                dx,
                dy,
                dihedral: view.dihedral,
                decimate: view.decimate,
            },
            tiles,
            alpha,
            outvoted: intact - agreed,
        },
    })
}

/// The records of `found` from `plane` whose bytes are the vote's.
fn agreeing<'a>(found: &'a [Hit], plane: Plane, voted: &'a [u8]) -> impl Iterator<Item = &'a Hit> {
    found
        .iter()
        .filter(move |hit| hit.plane == plane && hit.bytes == voted)
}

/// The majority vote over the intact records (§2, "Read"): the length more than half of them have, then each byte's
/// value more than half of the records of that length hold. `None` when a length or a byte has no such majority.
fn vote(found: &[Hit]) -> Option<Vec<u8>> {
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
