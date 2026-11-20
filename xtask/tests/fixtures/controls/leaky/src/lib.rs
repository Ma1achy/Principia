//! Fixture: a test whose control leaves it passing, a parity check that masks the bits the fork lands in
//! (pitfalls §9).

/// Packs `bucket` into bits 2–4.
pub fn pack(bucket: u32) -> u32 {
    (bucket & 7) << 2
}
