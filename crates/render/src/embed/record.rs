//! The embedded record and its tile geometry (`principia_dd_image_embedding.md` §2, §3).
//!
//! `RECORD = header(16 B) ‖ payload ‖ crc32(payload)`, `header = magic(4) ‖ version(1) ‖ flags(1) ‖ payload_len(4) ‖
//! n_records(2) ‖ crc32(header)(4)`, every integer big-endian and every CRC PNG's (§2). The header's fields are
//! listed once, in the `header_format!` invocation below, and that one description generates the writer
//! (`Header::put_fields`), the reader (`Header::get_fields`) and the layout table ([`HEADER_FIELDS`]), so the two
//! sides cannot disagree on a field (REQ-TOOL-110). A record is trusted whole or discarded whole (REQ-TOOL-060).
//!
//! The description also fixes the values a newly written record carries in `magic` and `version` ([`MAGIC`],
//! [`VERSION`]), so [`encode`] writes those and no other; [`decode`] returns the version it reads, unchecked (§2).

/// A header field's wire form: its width in bytes, and its bytes most significant first (§2, "Byte order").
trait Field: Sized {
    /// The field's width in bytes.
    const LEN: usize;
    /// Appends the field's bytes to `out`.
    fn put(&self, out: &mut Vec<u8>);
    /// Reads the field from the first `LEN` bytes of `bytes`, which holds at least that many.
    fn get(bytes: &[u8]) -> Self;
}

impl Field for u8 {
    const LEN: usize = 1;
    fn put(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
    fn get(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl Field for u16 {
    const LEN: usize = 2;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    fn get(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1]])
    }
}

impl Field for u32 {
    const LEN: usize = 4;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    fn get(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl Field for [u8; 4] {
    const LEN: usize = 4;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
    fn get(bytes: &[u8]) -> Self {
        [bytes[0], bytes[1], bytes[2], bytes[3]]
    }
}

/// Generates the header struct, its constructor, its writer, its reader and its layout table from one list of fields
/// in wire order: the `fixed` fields first, each with the value a newly written record carries, then the `free` ones,
/// which the writer's caller gives.
macro_rules! header_format {
    (
        fixed { $($(#[doc = $fdoc:literal])* $fixed:ident: $fty:ty = $value:expr,)+ }
        free { $($(#[doc = $doc:literal])* $field:ident: $ty:ty,)+ }
    ) => {
        /// A record's header fields before `crc32(header)`, as the record carries them (§2).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct Header {
            $($(#[doc = $fdoc])* pub $fixed: $fty,)+
            $($(#[doc = $doc])* pub $field: $ty,)+
        }

        /// The header's fields in wire order, each with its width in bytes; `crc32(header)` follows them.
        pub const HEADER_FIELDS: &[(&str, usize)] = &[
            $((stringify!($fixed), <$fty as Field>::LEN),)+
            $((stringify!($field), <$ty as Field>::LEN),)+
        ];

        /// The width in bytes of the fields `crc32(header)` covers.
        const FIELDS_LEN: usize = 0 $(+ <$fty as Field>::LEN)+ $(+ <$ty as Field>::LEN)+;

        impl Header {
            /// A newly written record's header: the fixed fields at their values, the free ones as given.
            pub fn new($($field: $ty),+) -> Self {
                Self { $($fixed: $value,)+ $($field,)+ }
            }

            /// The writer: appends each field's bytes to `out`, in wire order.
            fn put_fields(&self, out: &mut Vec<u8>) {
                $(Field::put(&self.$fixed, out);)+
                $(Field::put(&self.$field, out);)+
            }

            /// The reader: reads each field in wire order from `bytes`, which holds at least `FIELDS_LEN` bytes.
            fn get_fields(bytes: &[u8]) -> Self {
                let mut at = 0;
                $(
                    let $fixed = <$fty as Field>::get(&bytes[at..]);
                    at += <$fty as Field>::LEN;
                )+
                $(
                    let $field = <$ty as Field>::get(&bytes[at..]);
                    at += <$ty as Field>::LEN;
                )+
                debug_assert_eq!(at, FIELDS_LEN);
                Self { $($fixed,)+ $($field,)+ }
            }
        }
    };
}

header_format! {
    fixed {
        /// The four fixed bytes that mark a record ([`MAGIC`]).
        magic: [u8; 4] = MAGIC,
        /// The record layout's version ([`VERSION`] when written; read, not checked).
        version: u8 = VERSION,
    }
    free {
        /// The flag bits ([`Flags`]).
        flags: u8,
        /// The payload's length in bytes.
        payload_len: u32,
        /// How many records the writer placed in the RGB plane; the alpha plane's are not counted (§2, "`n_records`").
        n_records: u16,
    }
}

/// The width in bytes of a CRC field.
pub const CRC_LEN: usize = 4;

/// The header's width in bytes, `crc32(header)` included.
pub const HEADER_LEN: usize = FIELDS_LEN + CRC_LEN;

const _: () = assert!(HEADER_LEN == 16, "§2 gives a 16-byte header");

/// The record's magic, `8F 50 72 6E`: proposed (R-71, REQ-TOOL-109) and provisional until the human confirms it at
/// the M7 gate (R-182). It differs from the prototype's [`PROTOTYPE_MAGIC`] in every byte (§2, "Magic and version";
/// R-380).
pub const MAGIC: [u8; 4] = [0x8F, 0x50, 0x72, 0x6E];

/// The record layout's version, 3: R-81's contract-name layout, one above the prototype's [`PROTOTYPE_VERSION`]. An
/// R-71 proposal per R-380 (REQ-TOOL-109, REQ-TOOL-118), provisional until the human confirms it at the M7 gate (R-182).
pub const VERSION: u8 = 3;

/// The prototype's magic, `PRPX` (`50 52 50 58`), which a new record must not reuse (§2; R-380).
pub const PROTOTYPE_MAGIC: [u8; 4] = *b"PRPX";

/// The prototype layout's highest version byte, 2, which a new record must not reuse (§2; R-380).
pub const PROTOTYPE_VERSION: u8 = 2;

const _: () = assert!(
    VERSION > PROTOTYPE_VERSION,
    "R-81 bumps the version above the prototype's"
);

/// The variant a record's writer used (§5), flag bits 0–1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Whole-record repetition in a tile grid, the default.
    Tiled,
    /// Each bit written `k` times.
    Redundant,
    /// Per-bit redundancy and tiling together.
    Hybrid,
}

/// A record's flag bits (§2, "Flag bits").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags {
    /// Bits 0–1.
    pub variant: Variant,
    /// Bit 2: the payload carries the source of at least one ejected WGSL node (§6).
    pub source: bool,
}

impl Flags {
    /// Bits 0–1, the variant.
    const VARIANT: u8 = 0b011;
    /// Bit 2, `source`.
    const SOURCE: u8 = 0b100;
    /// Bits 3–7, reserved.
    const RESERVED: u8 = 0b1111_1000;

    /// The flags byte: each defined combination spelled out.
    pub fn to_byte(self) -> u8 {
        match (self.variant, self.source) {
            (Variant::Tiled, false) => 0b000,
            (Variant::Redundant, false) => 0b001,
            (Variant::Hybrid, false) => 0b010,
            (Variant::Tiled, true) => 0b100,
            (Variant::Redundant, true) => 0b101,
            (Variant::Hybrid, true) => 0b110,
        }
    }

    /// The flags a byte holds, or `None` when its variant is the reserved `3` or a reserved bit (3–7) is set.
    pub fn from_byte(byte: u8) -> Option<Self> {
        if byte & Self::RESERVED != 0 {
            return None;
        }
        let variant = match byte & Self::VARIANT {
            0 => Variant::Tiled,
            1 => Variant::Redundant,
            2 => Variant::Hybrid,
            _ => return None,
        };
        Some(Self {
            variant,
            source: byte & Self::SOURCE != 0,
        })
    }
}

/// One record's content: what the writer puts in and the reader gets back from an intact record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The flag bits.
    pub flags: Flags,
    /// How many records the writer placed in the RGB plane; the alpha plane's are not counted (§2, "`n_records`").
    pub n_records: u16,
    /// The payload: the canonical JSON, compressed with raw DEFLATE (§2, "Payload serialisation").
    pub payload: Vec<u8>,
}

/// An intact record as the reader finds it: its layout's version, as read, and its content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    /// The header's `version`, read and not checked: an intact record of another layout is reported as such (§2).
    pub version: u8,
    /// The record's content.
    pub record: Record,
}

/// Why a record could not be encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// The payload is longer than `payload_len`'s u32 can say; its length in bytes.
    PayloadTooLong(usize),
}

/// Why a record was discarded whole (§2, "A record is trusted or discarded whole").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discard {
    /// Shorter than its header, or than its header's `payload_len` says.
    Truncated,
    /// The magic differs from [`MAGIC`].
    Magic,
    /// `crc32(header)` fails.
    HeaderCrc,
    /// The flags are not a defined combination.
    Flags,
    /// `crc32(payload)` fails.
    PayloadCrc,
}

/// PNG's CRC-32 of `bytes` (§2, "The CRC is PNG's").
pub fn crc32(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

/// The record's bytes: `header ‖ payload ‖ crc32(payload)`, with [`MAGIC`], [`VERSION`] and `payload_len` the
/// payload's length.
pub fn encode(record: &Record) -> Result<Vec<u8>, EncodeError> {
    let payload_len = u32::try_from(record.payload.len())
        .map_err(|_| EncodeError::PayloadTooLong(record.payload.len()))?;
    let header = Header::new(record.flags.to_byte(), payload_len, record.n_records);
    let mut out = Vec::with_capacity(record_len(record.payload.len()));
    header.put_fields(&mut out);
    crc32(&out).put(&mut out);
    out.extend_from_slice(&record.payload);
    crc32(&record.payload).put(&mut out);
    Ok(out)
}

/// The record at the start of `bytes`, or why it is discarded whole. Bytes after its `crc32(payload)` are not part of
/// it. The version is returned as read, not checked (§2).
pub fn decode(bytes: &[u8]) -> Result<Decoded, Discard> {
    if bytes.len() < HEADER_LEN {
        return Err(Discard::Truncated);
    }
    let header = Header::get_fields(bytes);
    if header.magic != MAGIC {
        return Err(Discard::Magic);
    }
    if u32::get(&bytes[FIELDS_LEN..]) != crc32(&bytes[..FIELDS_LEN]) {
        return Err(Discard::HeaderCrc);
    }
    let flags = Flags::from_byte(header.flags).ok_or(Discard::Flags)?;
    let payload_len = usize::try_from(header.payload_len).map_err(|_| Discard::Truncated)?;
    let end = HEADER_LEN
        .checked_add(payload_len)
        .and_then(|n| n.checked_add(CRC_LEN))
        .ok_or(Discard::Truncated)?;
    if bytes.len() < end {
        return Err(Discard::Truncated);
    }
    let payload = &bytes[HEADER_LEN..HEADER_LEN + payload_len];
    if u32::get(&bytes[HEADER_LEN + payload_len..]) != crc32(payload) {
        return Err(Discard::PayloadCrc);
    }
    Ok(Decoded {
        version: header.version,
        record: Record {
            flags,
            n_records: header.n_records,
            payload: payload.to_vec(),
        },
    })
}

/// The bits a pixel of the RGB plane holds: the low bits of R, G and B (§2, "Within a tile").
pub const RGB_BITS_PER_PIXEL: u32 = 3;

/// The bits a pixel of the alpha plane holds: the low bit of alpha.
pub const ALPHA_BITS_PER_PIXEL: u32 = 1;

/// The length in bytes of a record whose payload is `payload_len` bytes.
pub fn record_len(payload_len: usize) -> usize {
    HEADER_LEN + payload_len + CRC_LEN
}

/// The length in bits of a record whose payload is `payload_len` bytes.
pub fn record_bits(payload_len: usize) -> u64 {
    8 * record_len(payload_len) as u64
}

/// A tile's side in pixels, `ceil(sqrt(ceil(record_bits / bits_per_pixel)))` (§3; §2, "Tile side and the grid"),
/// computed in integers.
///
/// # Panics
/// When `bits_per_pixel` is 0, or the side does not fit a u32.
pub fn tile_side(record_bits: u64, bits_per_pixel: u32) -> u32 {
    let pixels = record_bits.div_ceil(u64::from(bits_per_pixel));
    let floor = pixels.isqrt();
    let side = if floor * floor < pixels {
        floor + 1
    } else {
        floor
    };
    u32::try_from(side).expect("a tile side wider than u32 pixels")
}

/// The tile grid of a `width` × `height` plane: `(⌊width / side⌋, ⌊height / side⌋)`, its columns and rows.
///
/// # Panics
/// When `side` is 0.
pub fn tile_grid(width: u32, height: u32, side: u32) -> (u32, u32) {
    (width / side, height / side)
}

/// The top-left pixel of tile `(col, row)` in a grid of side `side`, `(col × side, row × side)`.
pub fn tile_origin(col: u32, row: u32, side: u32) -> (u32, u32) {
    (col * side, row * side)
}

/// The `n_records` a writer puts in each record it places in a `width` × `height` image, for a payload of
/// `payload_len` bytes: the RGB plane's tiles, `⌊width / side⌋ × ⌊height / side⌋` at the RGB side, at most 65,535 (§2,
/// "`n_records`"). The alpha plane's records are an extra layer and are not counted.
pub fn placed_records(width: u32, height: u32, payload_len: usize) -> u16 {
    let side = tile_side(record_bits(payload_len), RGB_BITS_PER_PIXEL);
    let (cols, rows) = tile_grid(width, height, side);
    u16::try_from(u64::from(cols) * u64::from(rows)).unwrap_or(u16::MAX)
}

/// Where bit `i` of the record sits in its byte stream: byte `⌊i / 8⌋`, shifted down by `7 − (i mod 8)` (§2, "Bit
/// order": each byte most significant bit first).
fn bit_position(i: u64) -> (usize, u32) {
    let byte = usize::try_from(i / 8).expect("a bit index beyond usize bytes");
    (byte, 7 - (i % 8) as u32)
}

/// Bit `i` of `record`, each byte most significant bit first.
///
/// # Panics
/// When `i` is beyond the record.
pub fn record_bit(record: &[u8], i: u64) -> bool {
    let (byte, shift) = bit_position(i);
    (record[byte] >> shift) & 1 == 1
}

/// A bit's place in a tile: the pixel's column and row from the tile's top-left, and the channel (0, 1, 2 for R, G, B
/// in the RGB plane; 0 for alpha in the alpha plane).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The pixel's column from the tile's left edge.
    pub x: u32,
    /// The pixel's row from the tile's top edge.
    pub y: u32,
    /// The channel within the pixel.
    pub channel: u32,
}

/// Where bit `i` of the record goes in a tile of side `side` whose pixels hold `bits_per_pixel` bits: pixel
/// `p = ⌊i / bits_per_pixel⌋` at column `p mod side`, row `⌊p / side⌋`, channel `i mod bits_per_pixel` (§2, "Within a
/// tile").
pub fn bit_slot(i: u64, side: u32, bits_per_pixel: u32) -> Slot {
    let bpp = u64::from(bits_per_pixel);
    let pixel = i / bpp;
    let side = u64::from(side);
    let narrow = |v: u64| u32::try_from(v).expect("a slot beyond u32");
    Slot {
        x: narrow(pixel % side),
        y: narrow(pixel / side),
        channel: narrow(i % bpp),
    }
}

/// The index of `slot` in a tile's low bits laid out as `lows[(y × side + x) × bits_per_pixel + channel]`.
fn low_index(slot: Slot, side: u32, bits_per_pixel: u32) -> usize {
    let index = (u64::from(slot.y) * u64::from(side) + u64::from(slot.x))
        * u64::from(bits_per_pixel)
        + u64::from(slot.channel);
    usize::try_from(index).expect("a tile beyond usize slots")
}

/// The writer's tile: sets the low bits `lows` (one 0 or 1 per slot, `lows[(y × side + x) × bits_per_pixel +
/// channel]`) of a tile of side `side` to the bits of `record`; the slots after its last bit are left as they are.
///
/// # Panics
/// When `lows` is not `side² × bits_per_pixel` long, or the record does not fit the tile.
pub fn write_tile(record: &[u8], side: u32, bits_per_pixel: u32, lows: &mut [u8]) {
    let slots = u64::from(side) * u64::from(side) * u64::from(bits_per_pixel);
    assert_eq!(lows.len() as u64, slots, "the low bits are not one tile's");
    let bits = 8 * record.len() as u64;
    assert!(
        bits <= slots,
        "a {bits}-bit record does not fit a tile of {slots} slots"
    );
    for i in 0..bits {
        lows[low_index(bit_slot(i, side, bits_per_pixel), side, bits_per_pixel)] =
            u8::from(record_bit(record, i));
    }
}

/// The reader's tile: the bytes the low bits `lows` of a tile of side `side` spell, laid out as [`write_tile`] lays
/// them, every whole byte the tile holds; [`decode`] finds the record at their start.
///
/// # Panics
/// When `lows` is not `side² × bits_per_pixel` long.
pub fn read_tile(lows: &[u8], side: u32, bits_per_pixel: u32) -> Vec<u8> {
    let slots = u64::from(side) * u64::from(side) * u64::from(bits_per_pixel);
    assert_eq!(lows.len() as u64, slots, "the low bits are not one tile's");
    let mut bytes = vec![0u8; usize::try_from(slots / 8).expect("a tile beyond usize bytes")];
    for i in 0..8 * bytes.len() as u64 {
        let (byte, shift) = bit_position(i);
        bytes[byte] |=
            (lows[low_index(bit_slot(i, side, bits_per_pixel), side, bits_per_pixel)] & 1) << shift;
    }
    bytes
}
