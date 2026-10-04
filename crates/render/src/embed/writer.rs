//! The tiled writer (`principia_dd_image_embedding.md` §2, §3; §5's default variant, REQ-TOOL-059).
//!
//! The payload goes in as one record, written whole into every tile of a 2-D square grid in the low bits of R, G and
//! B, and again into every tile of the alpha plane's own grid, the second systematic copy. Only low bits change, so no
//! channel moves by more than 1, and the strip beyond each plane's last whole tile is left as it was (§2, "Tile side
//! and the grid").

use super::record::{
    encode, placed_records, record_bits, tile_grid, tile_side, write_tile, EncodeError, Flags,
    Record, Variant,
};
use super::{Image, Plane};

/// What the writer placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The records placed in the RGB plane, the `n_records` each of them carries.
    pub rgb: u16,
    /// The records placed in the alpha plane, 0 when the image has none: an extra layer, not counted in `n_records`.
    pub alpha: u32,
}

/// Why the writer embedded nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbedError {
    /// The record could not be encoded.
    Encode(EncodeError),
    /// Not one RGB tile of `side` pixels fits the `width` × `height` image.
    TooSmall {
        /// The RGB plane's tile side the record needs.
        side: u32,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
    },
}

/// Embeds `payload` (the compressed canonical JSON, §2 "Payload serialisation") in `image` under the tiled variant,
/// with `source` its flag bit 2 (§6). The image is left untouched when the writer returns an error.
///
/// The RGB plane's tiles are written row by row from the top-left, at most the 65,535 `n_records` can count (§2,
/// "`n_records`"); every tile of the alpha plane's grid is written.
pub fn embed(image: &mut Image, payload: &[u8], source: bool) -> Result<Placed, EmbedError> {
    let n_records = placed_records(image.width(), image.height(), payload.len());
    let bits = record_bits(payload.len());
    if n_records == 0 {
        return Err(EmbedError::TooSmall {
            side: tile_side(bits, Plane::Rgb.bits_per_pixel()),
            width: image.width(),
            height: image.height(),
        });
    }
    let record = Record {
        flags: Flags {
            variant: Variant::Tiled,
            source,
        },
        n_records,
        payload: payload.to_vec(),
    };
    let bytes = encode(&record).map_err(EmbedError::Encode)?;
    let rgb = place(image, Plane::Rgb, &bytes, usize::from(n_records));
    debug_assert_eq!(rgb, u32::from(n_records));
    let alpha = if image.has_alpha() {
        place(image, Plane::Alpha, &bytes, usize::MAX)
    } else {
        0
    };
    Ok(Placed {
        rgb: n_records,
        alpha,
    })
}

/// Writes the record `bytes` into the first `limit` tiles of `plane`'s grid, row by row from the top-left, and
/// returns how many it wrote.
fn place(image: &mut Image, plane: Plane, bytes: &[u8], limit: usize) -> u32 {
    let bpp = plane.bits_per_pixel();
    let side = tile_side(8 * bytes.len() as u64, bpp);
    let (cols, rows) = tile_grid(image.width(), image.height(), side);
    let mut placed = 0;
    for (row, col) in (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .take(limit)
    {
        let mut lows = image.tile_lows(plane, col, row, side);
        write_tile(bytes, side, bpp, &mut lows);
        image.set_tile_lows(plane, col, row, side, &lows);
        placed += 1;
    }
    placed
}
