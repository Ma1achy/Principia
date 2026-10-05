//! The hybrid variant: per-bit `k`-fold redundancy plus tiling, for arbitrary rotation (`principia_dd_image_embedding.md`
//! §5, §7; REQ-TOOL-064, REQ-TOOL-111).
//!
//! **The layout** is §2's "The hybrid layout": the hybrid writer lays the tiled variant's grid (§2, "Tile side and the
//! grid"; §3) in *blocks* rather than pixels. A block is `m` × `m` pixels, `k = m²` (`m` the block side; §2's `b` is a
//! plane's bits per pixel), and every pixel of a block holds the same low bits, so each bit of the record is written
//! `k` times, side by side. The image's blocks sit in a grid from its top-left pixel, block `(i, j)` at pixel
//! `(i × m, j × m)`; a record's tile is `side` × `side` blocks, `side` §2's tile side, and within it the record's bits
//! fill the blocks as §2 ("Within a tile") fills a tiled record's pixels, three bits a block in the RGB plane and one
//! in alpha. The pixels beyond the last whole block, and the slots after a record's last bit, keep their own low bits.
//! A hybrid image is thus the tiled layout of an image `m` times smaller, upscaled by `m` with nearest neighbour, over
//! the canvas's own high bits. Its records carry the hybrid variant in their flags and, in `n_records`, the RGB
//! plane's tiles; every other field is §2's, and `k` is stored nowhere: the reader finds it by search.
//!
//! **The angle search** (§5). Nearest-neighbour rotation by an arbitrary angle moves each pixel to the nearest pixel of the
//! rotated grid, which is a permutation of most pixels but not of all: rotating back leaves bits wrong near the blocks'
//! edges (§5: ~8 %), which whole-record repetition cannot absorb and a vote over each block can. [`read`]
//! first reads the image as §4's three searches do ([`reader::read`]); when they recover nothing, it undoes a rotation
//! by each whole degree from 0 to 89, the dihedral search covering the rest of the circle, and for each redundancy
//! offered, finds the blocks' phase in the rotated-back image, votes each block's low bits and searches the voted
//! blocks for intact records. The CRC is the oracle (§4): the first angle and redundancy whose blocks hold an intact
//! record are the ones read, and the reader reports them.
//!
//! **The default `k`, [`DEFAULT`], is proposed under R-71 (REQ-TOOL-111)** and provisional until the human confirms it
//! at the M7 gate.

use super::reader::{self, settle, Corrupt, CorruptReason, Outcome};
use super::record::{
    bit_slot, encode, placed_records, record_bit, record_bits, tile_grid, tile_origin, tile_side,
    Flags, Record, Variant,
};
use super::search::{lows, search_blocks, Search};
use super::writer::{EmbedError, Placed};
use super::{Image, Plane};
use std::cmp::Reverse;

/// A per-bit redundancy the hybrid variant offers: `k` copies of each bit, an `m` × `m` block of pixels, `k = m²`
/// (§2, "The hybrid layout"). The two offered are §5's measured `k = 9` and `k = 25`; `m` is odd, so a block's vote has
/// no tie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redundancy {
    /// `k = 9`, 3 × 3 blocks: §5's 5 % uniform noise.
    K9,
    /// `k = 25`, 5 × 5 blocks: §5's 15 % uniform noise.
    K25,
}

/// The default redundancy, `k = 25`: proposed under R-71 (REQ-TOOL-111), provisional until the human confirms it at the
/// M7 gate. §5: nearest rotation leaves ~8 % bit error, which `k = 25` absorbs (15 % uniform noise) and `k = 9` does not
/// (5 %).
pub const DEFAULT: Redundancy = Redundancy::K25;

impl Redundancy {
    /// The redundancies offered, in the order the angle search tries them: the default first.
    pub const ALL: [Self; 2] = [Self::K25, Self::K9];

    /// The copies of each bit, `k`.
    pub fn k(self) -> u32 {
        match self {
            Self::K9 => 9,
            Self::K25 => 25,
        }
    }

    /// A block's side in pixels, `m = √k`.
    pub fn block(self) -> u32 {
        match self {
            Self::K9 => 3,
            Self::K25 => 5,
        }
    }
}

/// The rotation the angle search undid, as [`reader::Transform`] reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rotation {
    /// The angle undone, in whole degrees counter-clockwise, 0 to 89: the dihedral transform reported with it gives
    /// the multiple of 90° beyond it.
    pub degrees: u32,
    /// The redundancy whose blocks were voted.
    pub redundancy: Redundancy,
}

/// Embeds `payload` (the compressed canonical JSON, §2 "Payload serialisation") in `image` under the hybrid variant
/// with `redundancy`, `source` its flag bit 2 (§6). The image is left untouched when the writer returns an error.
///
/// The RGB plane's tiles are written row by row from the top-left, at most the 65,535 `n_records` can count (§2,
/// "`n_records`"); every tile of the alpha plane's grid is written.
pub fn embed(
    image: &mut Image,
    payload: &[u8],
    source: bool,
    redundancy: Redundancy,
) -> Result<Placed, EmbedError> {
    let m = redundancy.block();
    let n_records = placed_records(image.width() / m, image.height() / m, payload.len());
    if n_records == 0 {
        return Err(EmbedError::TooSmall {
            side: tile_side(record_bits(payload.len()), Plane::Rgb.bits_per_pixel()) * m,
            width: image.width(),
            height: image.height(),
        });
    }
    let record = Record {
        flags: Flags {
            variant: Variant::Hybrid,
            source,
        },
        n_records,
        payload: payload.to_vec(),
    };
    let bytes = encode(&record).map_err(EmbedError::Encode)?;
    let rgb = place(image, Plane::Rgb, &bytes, m, usize::from(n_records));
    debug_assert_eq!(rgb, u32::from(n_records));
    let alpha = if image.has_alpha() {
        place(image, Plane::Alpha, &bytes, m, usize::MAX)
    } else {
        0
    };
    Ok(Placed {
        rgb: n_records,
        alpha,
    })
}

/// Writes the record `bytes` into the first `limit` tiles of `plane`'s grid of `m` × `m` blocks, row by row from the
/// top-left, every pixel of a block holding its bits, and returns how many it wrote.
fn place(image: &mut Image, plane: Plane, bytes: &[u8], m: u32, limit: usize) -> u32 {
    let bpp = plane.bits_per_pixel();
    let bits = 8 * bytes.len() as u64;
    let side = tile_side(bits, bpp);
    let (cols, rows) = tile_grid(image.width() / m, image.height() / m, side);
    let mut placed = 0;
    for (row, col) in (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .take(limit)
    {
        let (bx, by) = tile_origin(col, row, side);
        for i in 0..bits {
            let slot = bit_slot(i, side, bpp);
            let bit = u8::from(record_bit(bytes, i));
            let (x, y) = ((bx + slot.x) * m, (by + slot.y) * m);
            for dy in 0..m {
                for dx in 0..m {
                    image.set_low(plane, x + dx, y + dy, slot.channel, bit);
                }
            }
        }
        placed += 1;
    }
    placed
}

/// Reads `image`'s embedded state under either variant and reports exactly one of the three outcomes (§9): §4's three
/// searches first ([`reader::read`]), and when they recover nothing, the angle search. State is present when either
/// finds it so; in the angle search's views only an intact header shows it, never the magic alone, so the search's
/// many views do not raise the rate of state reported present in images that hold none.
pub fn read(image: &Image) -> Outcome {
    let tiled = reader::read(image);
    if matches!(tiled, Outcome::Recovered(_)) {
        return tiled;
    }
    let (present, found) = angle_search(image);
    match found {
        Some((searched, rotation)) => settle(searched, image.has_alpha(), Some(rotation)),
        None if tiled == Outcome::None && present => Outcome::Corrupt(Corrupt {
            intact: 0,
            reason: CorruptReason::NoIntactRecord,
        }),
        None => tiled,
    }
}

/// The angle search: for each whole degree from 0 to 89 and each redundancy offered, the image rotated back, its blocks
/// voted and searched; returns whether any view showed state present, and the first view that held an intact record
/// with the rotation undone to find it.
fn angle_search(image: &Image) -> (bool, Option<(Search, Rotation)>) {
    let lows = lows(image);
    let mut present = false;
    for degrees in 0..90 {
        let canvas = Canvas::rotated_back(&lows, image.width(), image.height(), degrees);
        for redundancy in Redundancy::ALL {
            let blocks = canvas.blocks(redundancy, image.has_alpha());
            let searched = search_blocks(&blocks);
            present |= searched.present;
            if searched.found.is_some() {
                return (
                    present,
                    Some((
                        searched,
                        Rotation {
                            degrees,
                            redundancy,
                        },
                    )),
                );
            }
        }
    }
    (present, None)
}

/// An image's low bits ([`lows`]) rotated back by a whole number of degrees, nearest neighbour, onto a canvas that
/// holds the whole of it; the canvas's pixels outside the image are 0.
struct Canvas {
    width: u32,
    height: u32,
    lows: Vec<u8>,
}

impl Canvas {
    /// `lows`, a `width` × `height` image's, rotated clockwise by `degrees`, undoing a counter-clockwise rotation by as
    /// much. The canvas is `⌈w cos θ + h sin θ⌉` × `⌈w sin θ + h cos θ⌉`, centred on the image's centre; its pixel
    /// `(u, v)` takes the image's pixel under its centre, `(u + ½, v + ½)` rotated counter-clockwise by θ about the two
    /// centres, rounded down to the pixel it falls in.
    fn rotated_back(lows: &[u8], width: u32, height: u32, degrees: u32) -> Self {
        let (sin, cos) = f64::from(degrees).to_radians().sin_cos();
        let (w, h) = (f64::from(width), f64::from(height));
        // At 0° the sides are the image's exactly; at every other whole degree they are not whole numbers.
        let (cw, ch) = (
            (w * cos + h * sin).ceil() as u32,
            (w * sin + h * cos).ceil() as u32,
        );
        let mut out = vec![0u8; cw as usize * ch as usize];
        let (ccx, ccy) = (f64::from(cw) / 2.0, f64::from(ch) / 2.0);
        for v in 0..ch {
            let dy = f64::from(v) + 0.5 - ccy;
            for u in 0..cw {
                let dx = f64::from(u) + 0.5 - ccx;
                let sx = (dx * cos + dy * sin + w / 2.0).floor();
                let sy = (-dx * sin + dy * cos + h / 2.0).floor();
                if sx >= 0.0 && sy >= 0.0 && sx < w && sy < h {
                    out[v as usize * cw as usize + u as usize] =
                        lows[sy as usize * width as usize + sx as usize];
                }
            }
        }
        Self {
            width: cw,
            height: ch,
            lows: out,
        }
    }

    /// The column or row, from 0 to `m` − 1, at which the canvas's blocks of side `m` start along one axis: the one
    /// where the low bits change most often from the pixel before, as they do at a block's edge and seldom inside it.
    /// `across` says whether the axis is the rows' (x) or the columns' (y).
    fn phase(&self, m: u32, across: bool) -> u32 {
        let w = self.width as usize;
        // The step from a pixel to the one before it along the axis.
        let step = if across { 1 } else { w };
        let mut changes = vec![0u64; m as usize];
        for at in 0..self.lows.len() {
            let coordinate = if across { at % w } else { at / w };
            if coordinate > 0 {
                changes[coordinate % m as usize] +=
                    u64::from((self.lows[at] ^ self.lows[at - step]).count_ones());
            }
        }
        // The first of the most frequent, so a tie is settled the same way every time.
        (0..m)
            .max_by_key(|&i| (changes[i as usize], Reverse(i)))
            .unwrap_or(0)
    }

    /// The canvas's blocks of `redundancy`'s side `m` from their phase ([`Canvas::phase`]) as an image, each pixel's R,
    /// G, B and (when `alpha`) A low bits the majority of that bit over the block's `k = m²` pixels, every other bit 0.
    fn blocks(&self, redundancy: Redundancy, alpha: bool) -> Image {
        let (m, k) = (redundancy.block(), redundancy.k());
        let (px, py) = (self.phase(m, true), self.phase(m, false));
        let (bw, bh) = (
            self.width.saturating_sub(px) / m,
            self.height.saturating_sub(py) / m,
        );
        let channels = if alpha { 4 } else { 3 };
        let mut pixels = Vec::with_capacity(bw as usize * bh as usize * channels);
        for j in 0..bh {
            for i in 0..bw {
                let mut ones = [0u32; 4];
                for y in py + j * m..py + (j + 1) * m {
                    let row = y as usize * self.width as usize;
                    for x in px + i * m..px + (i + 1) * m {
                        let low = self.lows[row + x as usize];
                        for (c, n) in ones.iter_mut().enumerate() {
                            *n += u32::from((low >> c) & 1);
                        }
                    }
                }
                pixels.extend(ones[..channels].iter().map(|&n| u8::from(n > k / 2)));
            }
        }
        Image::new(bw, bh, alpha, pixels)
    }
}
