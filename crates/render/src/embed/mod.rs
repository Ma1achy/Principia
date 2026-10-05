//! Embedding a slice's provenance in the pixels of its own PNG (`principia_dd_image_embedding.md`).
//!
//! [`record`] is the record format and its tile geometry, [`writer`] the tiled writer, [`search`] §4's three searches
//! on read and [`reader`] the majority-vote reader with its three outcomes. [`hybrid`] is §5's hybrid variant, its
//! writer and the angle search that reads it after arbitrary rotation. [`Image`] is the pixels both sides work on.

pub mod hybrid;
pub mod reader;
pub mod record;
pub mod search;
pub mod writer;

use record::{tile_origin, ALPHA_BITS_PER_PIXEL, RGB_BITS_PER_PIXEL};

/// A low-bit plane (§2, "Layout"): the low bits of R, G and B, or the low bit of alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    /// The low bits of R, G and B, three bits a pixel: the payload's records, counted in `n_records`.
    Rgb,
    /// The low bit of alpha, one bit a pixel: the second systematic copy, not counted in `n_records`.
    Alpha,
}

impl Plane {
    /// The bits a pixel of the plane holds.
    pub fn bits_per_pixel(self) -> u32 {
        match self {
            Self::Rgb => RGB_BITS_PER_PIXEL,
            Self::Alpha => ALPHA_BITS_PER_PIXEL,
        }
    }

    /// The pixel's channel that holds the plane's slot 0: R for the RGB plane, A for alpha.
    fn first_channel(self) -> usize {
        match self {
            Self::Rgb => 0,
            Self::Alpha => 3,
        }
    }
}

/// An 8-bit image as the embedding writes and reads it: rows top first (the first row PNG stores), each row left to
/// right, each pixel R, G, B and, when the image has an alpha channel, A.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    alpha: bool,
    pixels: Vec<u8>,
}

impl Image {
    /// A `width` × `height` image; `alpha` says whether each pixel carries an A after its R, G, B.
    ///
    /// # Panics
    /// When `pixels` is not `width × height` pixels long.
    pub fn new(width: u32, height: u32, alpha: bool, pixels: Vec<u8>) -> Self {
        let channels = if alpha { 4 } else { 3 };
        let len = u64::from(width) * u64::from(height) * channels;
        assert_eq!(
            pixels.len() as u64,
            len,
            "the pixels are not a {width} × {height} image's"
        );
        Self {
            width,
            height,
            alpha,
            pixels,
        }
    }

    /// The width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether the image has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        self.alpha
    }

    /// The pixels' bytes.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The pixels' bytes, to change.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// The image with its alpha channel stripped, as a pipeline that drops alpha leaves it.
    pub fn without_alpha(&self) -> Self {
        if !self.alpha {
            return self.clone();
        }
        let pixels = self
            .pixels
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        Self::new(self.width, self.height, false, pixels)
    }

    /// The planes the image has: RGB, then alpha when it has an alpha channel.
    pub fn planes(&self) -> &'static [Plane] {
        if self.alpha {
            &[Plane::Rgb, Plane::Alpha]
        } else {
            &[Plane::Rgb]
        }
    }

    /// The index in [`Image::pixels`] of slot `channel` of `plane` at pixel `(x, y)`.
    fn index(&self, plane: Plane, x: u32, y: u32, channel: u32) -> usize {
        assert!(
            plane == Plane::Rgb || self.alpha,
            "the image has no alpha plane"
        );
        let channels = if self.alpha { 4 } else { 3 };
        let pixel = usize::try_from(u64::from(y) * u64::from(self.width) + u64::from(x))
            .expect("a pixel beyond usize");
        pixel * channels + plane.first_channel() + channel as usize
    }

    /// The low bit, 0 or 1, of slot `channel` of `plane` at pixel `(x, y)`.
    pub fn low(&self, plane: Plane, x: u32, y: u32, channel: u32) -> u8 {
        self.pixels[self.index(plane, x, y, channel)] & 1
    }

    /// Sets the low bit of slot `channel` of `plane` at pixel `(x, y)` to `bit`'s low bit; every other bit is kept.
    pub fn set_low(&mut self, plane: Plane, x: u32, y: u32, channel: u32, bit: u8) {
        let at = self.index(plane, x, y, channel);
        self.pixels[at] = (self.pixels[at] & !1) | (bit & 1);
    }

    /// The low bits of tile `(col, row)` of side `side` in `plane`, laid out as [`record::write_tile`] takes them:
    /// `lows[(y × side + x) × bits_per_pixel + channel]`.
    pub fn tile_lows(&self, plane: Plane, col: u32, row: u32, side: u32) -> Vec<u8> {
        let (ox, oy) = tile_origin(col, row, side);
        let bpp = plane.bits_per_pixel();
        let mut lows = Vec::with_capacity((side as usize).pow(2) * bpp as usize);
        for y in 0..side {
            for x in 0..side {
                for channel in 0..bpp {
                    lows.push(self.low(plane, ox + x, oy + y, channel));
                }
            }
        }
        lows
    }

    /// Sets the low bits of tile `(col, row)` of side `side` in `plane` to `lows`, laid out as [`Image::tile_lows`]
    /// gives them; every other bit of each pixel is kept.
    ///
    /// # Panics
    /// When `lows` is not the tile's `side² × bits_per_pixel` slots.
    pub fn set_tile_lows(&mut self, plane: Plane, col: u32, row: u32, side: u32, lows: &[u8]) {
        let (ox, oy) = tile_origin(col, row, side);
        let bpp = plane.bits_per_pixel();
        assert_eq!(
            lows.len(),
            (side as usize).pow(2) * bpp as usize,
            "the low bits are not one tile's"
        );
        let mut slots = lows.iter();
        for y in 0..side {
            for x in 0..side {
                for channel in 0..bpp {
                    let at = self.index(plane, ox + x, oy + y, channel);
                    let set = slots.next().expect("one low bit per slot") & 1 == 1;
                    self.pixels[at] = if set {
                        self.pixels[at] | 1
                    } else {
                        self.pixels[at] & !1
                    };
                }
            }
        }
    }
}
