//! The three searches on read (`principia_dd_image_embedding.md` §4; REQ-TOOL-063): the tile offset `(dx, dy)` over
//! one tile, the 8 dihedral transforms and nearest-neighbour decimation by `s`, with the CRC as the oracle.
//!
//! A candidate is a decimation factor `s` and a dihedral transform. Its view is the image decimated, every `s`-th pixel
//! of every `s`-th row from the top-left, with the transform then undone. The offset search runs inside each view: a
//! record is looked for at every pixel of it, at every tile side a record can have, so a grid shifted by any `(dx, dy)`
//! is found wherever it lies. The candidates are taken in order, the transforms in [`Dihedral::ALL`]'s order and, for
//! each, `s` from 1 up, and the first whose view holds an intact record is the one read (§4: "the first that verifies
//! is correct"); its intact records, from both planes, are what the reader votes on. The untransformed image comes
//! first. Every decimation of a transform is tried before the next transform: an upscaled image's view is then reached
//! at the cost of the smaller decimated views, not of the seven other transforms at full size.
//!
//! The decimation's phase is not searched: a nearest-neighbour upscale by `s` turns each pixel into an `s` × `s` block,
//! and every `s`-th pixel from any start takes exactly one pixel of each block, so phase 0 serves however the upscaled
//! image was later cropped.

use super::reader::Dihedral;
use super::record::{
    bit_slot, decode, decode_header, record_bits, record_len, tile_side, HEADER_LEN, MAGIC,
    RGB_BITS_PER_PIXEL,
};
use super::{Image, Plane};

impl Dihedral {
    /// The eight transforms, in the order the search tries them.
    pub const ALL: [Self; 8] = [
        Self::Identity,
        Self::Rot90,
        Self::Rot180,
        Self::Rot270,
        Self::FlipH,
        Self::FlipV,
        Self::Transpose,
        Self::AntiTranspose,
    ];

    /// Whether the transform swaps an image's width and height.
    pub fn swaps(self) -> bool {
        matches!(
            self,
            Self::Rot90 | Self::Rot270 | Self::Transpose | Self::AntiTranspose
        )
    }

    /// The pixel of a `width` × `height` image the transform produced that holds pixel `(u, v)` of the image before
    /// it.
    ///
    /// # Panics
    /// When `width` or `height` is 0.
    pub fn source(self, u: u32, v: u32, width: u32, height: u32) -> (u32, u32) {
        let (right, bottom) = (width - 1, height - 1);
        match self {
            Self::Identity => (u, v),
            Self::Rot90 => (v, bottom - u),
            Self::Rot180 => (right - u, bottom - v),
            Self::Rot270 => (right - v, u),
            Self::FlipH => (right - u, v),
            Self::FlipV => (u, bottom - v),
            Self::Transpose => (v, u),
            Self::AntiTranspose => (right - v, bottom - u),
        }
    }
}

/// An intact record the search found in a view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// The plane it was found in.
    pub plane: Plane,
    /// Its tile's top-left pixel in the view.
    pub x: u32,
    /// Its tile's top-left pixel in the view.
    pub y: u32,
    /// Its tile's side.
    pub side: u32,
    /// The record's bytes, `header ‖ payload ‖ crc32(payload)`.
    pub bytes: Vec<u8>,
}

impl Hit {
    /// The origin of the grid its tile sits in, `(x mod side, y mod side)`: the offset §4's search found.
    pub fn grid_origin(&self) -> (u32, u32) {
        (self.x % self.side, self.y % self.side)
    }
}

/// The candidate whose view held an intact record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The dihedral transform undone.
    pub dihedral: Dihedral,
    /// The decimation factor `s`.
    pub decimate: u32,
    /// The view's width.
    pub width: u32,
    /// The view's height.
    pub height: u32,
    /// Every intact record in the view, RGB plane first, each plane's in row-major order of their tiles' top-left
    /// pixels.
    pub hits: Vec<Hit>,
}

/// What the search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    /// Whether state is present: in the untransformed, undecimated view, a tile of the grid the writer lays from the
    /// image's top-left pixel starts with the magic, at some side a record can have; or, at any pixel and side of any
    /// view searched, a tile starts with an intact header, its CRC verifying. The magic alone, at a pixel off that
    /// grid or in another view, is not counted: it is 32 bits, which a canvas's own low bits spell by chance somewhere
    /// among the many pixels, sides and views the search tries, so the rate of state reported present in images
    /// that hold none would rise with the search.
    pub present: bool,
    /// The first candidate whose view held an intact record, or `None` when no view held one.
    pub found: Option<Found>,
}

/// Searches `image` for intact records (§4): the eight dihedral transforms and, for each, the decimation factors from 1
/// up while the decimated image can still hold the smallest tile; returns at the first candidate whose view holds one.
pub fn search(image: &Image) -> Search {
    search_in(image, Scope::Image)
}

/// Searches a hybrid record's block view ([`hybrid`](super::hybrid)): `image` is an image's blocks, each pixel of it
/// the majority of one block, read after the angle search undid a rotation. The eight dihedral transforms are tried at
/// decimation 1 alone, and the magic alone never shows state present: no view of the blocks is the image the writer's
/// grid was laid in, so only an intact header does ([`Search::present`]).
pub(super) fn search_blocks(image: &Image) -> Search {
    search_in(image, Scope::Blocks)
}

/// What [`search_in`] searches.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The image as read: every decimation, and the identity undecimated view is the writer's grid's.
    Image,
    /// A hybrid record's block view: decimation 1 alone, and no view is the writer's grid's.
    Blocks,
}

/// The search of [`search`] and [`search_blocks`], over the candidates `scope` allows.
fn search_in(image: &Image, scope: Scope) -> Search {
    let lows = lows(image);
    let (width, height) = (image.width(), image.height());
    let smallest = tile_side(record_bits(0), RGB_BITS_PER_PIXEL);
    let mut present = false;
    let fits = |s: u32| width.div_ceil(s).min(height.div_ceil(s)) >= smallest;
    let decimates = |s: u32| scope == Scope::Image || s == 1;
    for dihedral in Dihedral::ALL {
        for decimate in (1..).take_while(|&s| fits(s) && decimates(s)) {
            let writer = scope == Scope::Image && dihedral == Dihedral::Identity && decimate == 1;
            let view = View::new(&lows, width, height, dihedral, decimate, writer);
            let mut hits = Vec::new();
            for &plane in image.planes() {
                if view.scan(plane, &mut hits) {
                    present = true;
                }
            }
            if !hits.is_empty() {
                return Search {
                    present,
                    found: Some(Found {
                        dihedral,
                        decimate,
                        width: view.width,
                        height: view.height,
                        hits,
                    }),
                };
            }
        }
    }
    Search {
        present,
        found: None,
    }
}

/// The magic's bits that a tile of `plane` at the smallest side holds in its top row: they lie there at every side, so
/// they are checked once for all sides before any side is tried. For each of those pixels, the mask of the plane's
/// bits in its low-bits byte ([`lows`]) and the value the magic gives them.
fn prefix(plane: Plane, smallest: u32) -> Vec<(u8, u8)> {
    let mut prefix: Vec<(u8, u8)> = Vec::new();
    for (i, bit) in MAGIC
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |shift| (byte >> shift) & 1))
        .enumerate()
    {
        let slot = bit_slot(i as u64, smallest, plane.bits_per_pixel());
        if slot.y > 0 {
            break;
        }
        let at = slot.x as usize;
        if prefix.len() == at {
            prefix.push((0, 0));
        }
        let shift = plane.first_channel() as u32 + slot.channel;
        prefix[at].0 |= 1 << shift;
        prefix[at].1 |= bit << shift;
    }
    debug_assert_eq!(
        prefix
            .iter()
            .map(|&(mask, _)| mask.count_ones())
            .sum::<u32>(),
        (smallest * plane.bits_per_pixel()).min(8 * MAGIC.len() as u32),
        "the prefix is not the magic's bits in the smallest tile's top row"
    );
    prefix
}

/// `image`'s low bits, one byte a pixel, `lows[y × width + x]`, whose bit `k` is the low bit of the pixel's channel
/// `k` (R, G, B and, when the image has one, A): so a plane's slot `channel` is bit `first_channel + channel`.
pub(super) fn lows(image: &Image) -> Vec<u8> {
    let channels = if image.has_alpha() { 4 } else { 3 };
    image
        .pixels()
        .chunks_exact(channels)
        .map(|pixel| {
            pixel
                .iter()
                .enumerate()
                .fold(0, |lows, (k, channel)| lows + ((channel & 1) << k))
        })
        .collect()
}

/// A candidate's view of an image's low bits ([`lows`]): every `decimate`-th pixel of every `decimate`-th row from the
/// top-left, with the dihedral transform then undone. The transform is affine, so the view's pixel `(u, v)` is
/// `lows[origin + u × step_u + v × step_v]`, each step a unit of x or y with its sign, scaled by the decimation.
struct View<'a> {
    lows: &'a [u8],
    /// Whether the view is the image itself, the identity undecimated of an image as read (never of a hybrid record's
    /// block view): the only view in which the writer's grid, the tiles at multiples of their side from the top-left
    /// pixel, is where the magic alone shows state present.
    writer: bool,
    width: u32,
    height: u32,
    origin: i64,
    step_u: i64,
    step_v: i64,
}

impl<'a> View<'a> {
    /// The view of candidate `(dihedral, decimate)` of a `width` × `height` image's `lows`; `writer` says whether it is
    /// the image itself, in which the writer's grid lies. The search builds a view only when it is at least the smallest
    /// tile side wide and high, so at least 2 pixels.
    fn new(
        lows: &'a [u8],
        width: u32,
        height: u32,
        dihedral: Dihedral,
        decimate: u32,
        writer: bool,
    ) -> Self {
        let (dw, dh) = (width.div_ceil(decimate), height.div_ceil(decimate));
        let pixel = |(x, y): (u32, u32)| {
            (i64::from(y) * i64::from(width) + i64::from(x)) * i64::from(decimate)
        };
        let origin = pixel(dihedral.source(0, 0, dw, dh));
        let (width, height) = if dihedral.swaps() { (dh, dw) } else { (dw, dh) };
        Self {
            lows,
            writer,
            width,
            height,
            origin,
            step_u: pixel(dihedral.source(1, 0, dw, dh)) - origin,
            step_v: pixel(dihedral.source(0, 1, dw, dh)) - origin,
        }
    }

    /// How far in `lows` the view's pixel `(u + a, v + b)` lies from its pixel `(a, b)`, for any `(a, b)`: so the
    /// view's pixel `(u, v)` is at `origin + offset(u, v)`.
    fn offset(&self, u: u32, v: u32) -> i64 {
        i64::from(u) * self.step_u + i64::from(v) * self.step_v
    }

    /// The low-bits byte at position `at` (never negative: the view's pixels are the image's own).
    fn low(&self, at: i64) -> u8 {
        self.lows[at as usize]
    }

    /// Pushes onto `hits` every intact record of `plane` whose tile lies whole in the view, at any pixel and any side
    /// a record can have; returns whether any of them shows state present ([`Search::present`]).
    fn scan(&self, plane: Plane, hits: &mut Vec<Hit>) -> bool {
        let smallest = tile_side(record_bits(0), plane.bits_per_pixel());
        let prefix = prefix(plane, smallest);
        let mut tiles = Tiles {
            view: self,
            plane,
            offsets: Vec::new(),
        };
        let mut present = false;
        for y in 0..self.height {
            let mut at = self.origin + self.offset(0, y);
            for x in 0..self.width {
                let largest = (self.width - x).min(self.height - y);
                if largest >= smallest
                    && self.starts(at, &prefix)
                    && tiles.probe(x, y, smallest, largest, hits)
                {
                    present = true;
                }
                at += self.step_u;
            }
        }
        present
    }

    /// Whether the run of pixels from position `at` along the view's rows holds `prefix`'s values under its masks.
    fn starts(&self, mut at: i64, prefix: &[(u8, u8)]) -> bool {
        for &(mask, value) in prefix {
            if self.low(at) & mask != value {
                return false;
            }
            at += self.step_u;
        }
        true
    }
}

/// A plane's tiles in a view, read through each side's slots.
struct Tiles<'v, 'a> {
    view: &'v View<'a>,
    plane: Plane,
    /// For each side, where the record's bits `0, 1, …` lie ([`bit_slot`]), as many as have been read at that side:
    /// the pixel's position in the view's `lows` from the tile's top-left pixel, and its bit in that byte.
    offsets: Vec<Vec<(i64, u32)>>,
}

impl Tiles<'_, '_> {
    /// The first `n` bytes the tile of side `side` with top-left pixel `(x, y)` spells, each byte most significant bit
    /// first, bit `i` in the slot [`bit_slot`] gives (§2, "Bit order", "Within a tile").
    fn bytes(&mut self, x: u32, y: u32, side: u32, n: usize) -> Vec<u8> {
        let (view, plane) = (self.view, self.plane);
        let at = side as usize;
        if self.offsets.len() <= at {
            self.offsets.resize(at + 1, Vec::new());
        }
        let offsets = &mut self.offsets[at];
        for i in offsets.len()..8 * n {
            let slot = bit_slot(i as u64, side, plane.bits_per_pixel());
            let shift = plane.first_channel() as u32 + slot.channel;
            offsets.push((view.offset(slot.x, slot.y), shift));
        }
        let origin = view.origin + view.offset(x, y);
        let mut bytes = vec![0u8; n];
        for (i, &(offset, shift)) in offsets[..8 * n].iter().enumerate() {
            bytes[i / 8] |= ((view.low(origin + offset) >> shift) & 1) << (7 - i % 8);
        }
        bytes
    }

    /// Looks for a record whose tile's top-left pixel is `(x, y)`, at every side from `smallest` to `largest`, the
    /// widest that fits the view there, and pushes it onto `hits` when it is intact; returns whether a tile there shows
    /// state present ([`Search::present`]): its header intact, or the magic at the start of a tile of the writer's
    /// grid.
    ///
    /// A side narrower than the header wraps it onto pixels of its own, so each such side is read. From the first side
    /// the header does not wrap at, every wider side reads the same header, and only the side its `payload_len` gives
    /// can hold its record, so that header is read once and that side alone is tried.
    fn probe(&mut self, x: u32, y: u32, smallest: u32, largest: u32, hits: &mut Vec<Hit>) -> bool {
        let unwrapped = (8 * HEADER_LEN as u32).div_ceil(self.plane.bits_per_pixel());
        let mut present = false;
        for side in smallest..unwrapped.min(largest + 1) {
            let (magic, need) = self.try_side(x, y, side, hits);
            if need.is_some() || (magic && self.on_writer_grid(x, y, side..=side)) {
                present = true;
            }
            if need == Some(side) {
                return present;
            }
        }
        if unwrapped <= largest {
            // Every side from `unwrapped` up reads the magic the same, so it starts a tile of the writer's grid when
            // any of those sides that fits divides both coordinates.
            let (magic, need) = self.try_side(x, y, unwrapped, hits);
            if need.is_some() || (magic && self.on_writer_grid(x, y, unwrapped..=largest)) {
                present = true;
            }
            if let Some(need) = need.filter(|&need| need > unwrapped && need <= largest) {
                self.try_side(x, y, need, hits);
            }
        }
        present
    }

    /// Whether `(x, y)` is the top-left pixel of a tile of the writer's grid, the view being the image itself, at
    /// one of `sides`: a multiple of the side in both coordinates.
    fn on_writer_grid(&self, x: u32, y: u32, sides: std::ops::RangeInclusive<u32>) -> bool {
        self.view.writer
            && sides
                .into_iter()
                .any(|side| x.is_multiple_of(side) && y.is_multiple_of(side))
    }

    /// Reads the header of the tile of side `side` with top-left pixel `(x, y)`, and pushes its record onto `hits`
    /// when `side` is the side the header's `payload_len` gives and the record is intact. Returns whether the tile
    /// starts with the magic, and the side the header gives when it is intact.
    fn try_side(&mut self, x: u32, y: u32, side: u32, hits: &mut Vec<Hit>) -> (bool, Option<u32>) {
        if self.bytes(x, y, side, MAGIC.len()) != MAGIC {
            return (false, None);
        }
        let Ok(header) = decode_header(&self.bytes(x, y, side, HEADER_LEN)) else {
            return (true, None);
        };
        let len = usize::try_from(header.payload_len).expect("a payload_len beyond usize");
        let need = tile_side(record_bits(len), self.plane.bits_per_pixel());
        if need == side {
            let bytes = self.bytes(x, y, side, record_len(len));
            if decode(&bytes).is_ok() {
                hits.push(Hit {
                    plane: self.plane,
                    x,
                    y,
                    side,
                    bytes,
                });
            }
        }
        (true, Some(need))
    }
}
