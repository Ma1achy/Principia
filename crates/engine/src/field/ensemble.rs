//! Supersampling: each nominal sample of the field marched as E+1 copies, each a full simulation with its own record.
//!
//! **The offsets** ([`offset`]): copy 0 is the un-jittered sample centre; copy `k` in 1..E sits at point `k` of the
//! Halton (2,3) sequence, centred (minus ½), in units of the sample's footprint: the base-2 radical inverse along the
//! window's first axis, the base-3 along its second. The set is a fixed prefix, so copy `k` is the same for every
//! `E ≥ k`, and it depends on `k` alone: never on time, the frame or the sample. Copies jitter the initial condition
//! only; every copy marches to the same horizon over the same sync grid.
//!
//! **The footprint** is the sample spacing of the window and size asked for: `(2·h_u/w, 2·h_v/h)` in the chart's
//! coordinates, so copy `k` of sample `(x, y)` is the window's point `((x + ½ + o_x)/w, (y + ½ + o_y)/h)`.
//!
//! **E by tier** ([`copies`]): 0 for Potato and Low, then 1, 3, 7 and 15 for Medium, High, Ultra and Extreme.

use kernel::chart::Latent;
use kernel::occupant::table::Tier;
use kernel::payload::{ReadParams, SimState};

use super::record::{Record, RecordBytes, Stored};
use super::{Field, Outcome};
use crate::first_slice::Quad;

/// The copies a quality tier marches beside each nominal sample, `E`.
pub fn copies(tier: Tier) -> u32 {
    match tier {
        Tier::Potato | Tier::Low => 0,
        Tier::Medium => 1,
        Tier::High => 3,
        Tier::Ultra => 7,
        Tier::Extreme => 15,
    }
}

/// The radical inverse of `k` in `base` as an exact fraction `(numerator, denominator)`: the digits of `k` mirrored
/// about the radix point.
fn radical_inverse(base: u64, mut k: u64) -> (u64, u64) {
    let (mut num, mut den) = (0, 1);
    while k > 0 {
        num = num * base + k % base;
        den *= base;
        k /= base;
    }
    (num, den)
}

/// Point `k` of the Halton (2,3) sequence, each coordinate one division of exact integers. Don't accumulate the digits
/// in floating point: base 3's fractions would then differ from `n/3ʲ` by an ulp.
pub fn halton(k: u32) -> [f64; 2] {
    let at = |base| {
        let (n, d) = radical_inverse(base, u64::from(k));
        n as f64 / d as f64
    };
    [at(2), at(3)]
}

/// Copy `k`'s offset from its sample's centre, in footprints along the window's two axes: `(0, 0)` for copy 0, and
/// Halton (2,3) point `k` minus ½ for every other copy.
pub fn offset(k: u32) -> [f64; 2] {
    if k == 0 {
        return [0.0, 0.0];
    }
    halton(k).map(|c| c - 0.5)
}

/// The chart point of copy `k` of `quad`'s sample `i` (the module's note): copy 0 is the sample's own point,
/// `Quad::uv`'s, taken as it is; copy `k` the window's point at the sample's centre moved by [`offset`]`(k)` footprints.
pub fn point(quad: &Quad<Latent<f32>>, i: usize, k: u32) -> [f64; 2] {
    if k == 0 {
        return quad.uv(i);
    }
    let [w, h] = quad.size;
    let [ox, oy] = offset(k);
    let s = ((i % w) as f64 + 0.5 + ox) / w as f64;
    let t = ((i / w) as f64 + 0.5 + oy) / h as f64;
    quad.window.at(s, t)
}

/// A `w × h` field marched as `E + 1` copies per nominal sample, each copy a full simulation with its own outcome and
/// record. **Layout**: copy-major, `E + 1` planes of `w·h`, each plane in the field's order (row by row from the
/// bottom left): copy `k` of sample `(x, y)` is element `k·w·h + y·w + x` of [`Supersampled::outcomes`] and
/// [`Supersampled::records`]. Plane 0 is the un-jittered field, [`super::Cpu::stored_on`]'s bit for bit, and each plane
/// is a whole field's records, as a stain view binds them ([`Supersampled::copy`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Supersampled {
    /// `E`, the copies beside each nominal sample.
    pub copies: u32,
    /// `[w, h]`, the nominal samples.
    pub size: [usize; 2],
    /// Every copy's outcome, `(E + 1)·w·h`, in the layout above.
    pub outcomes: Vec<Outcome>,
    /// Every copy's record, in the same layout.
    pub records: Vec<Record>,
    /// The read side's values for every record.
    pub read: ReadParams,
}

impl Supersampled {
    /// The nominal samples per plane, `w·h`.
    fn plane(&self) -> usize {
        self.size[0] * self.size[1]
    }

    /// Copy `k` of sample `(x, y)`'s element index in the layout.
    pub fn index(&self, k: u32, x: usize, y: usize) -> usize {
        k as usize * self.plane() + y * self.size[0] + x
    }

    /// Copy `k` of sample `(x, y)` unpacked by the read side.
    pub fn sample(&self, k: u32, x: usize, y: usize) -> SimState {
        self.records[self.index(k, x, y)].read(&self.read)
    }

    /// Copy `k`'s plane as a field with its records: for `k = 0`, the un-jittered field.
    pub fn copy(&self, k: u32) -> Stored {
        let range = k as usize * self.plane()..(k as usize + 1) * self.plane();
        Stored {
            field: Field {
                size: self.size,
                outcomes: self.outcomes[range.clone()].to_vec(),
            },
            records: self.records[range].to_vec(),
            read: self.read,
        }
    }

    /// Every copy's records as the three per-sample buffers' bytes, plane by plane: copy `k`'s at `[k]`.
    pub fn bytes(&self) -> Vec<RecordBytes> {
        (0..=self.copies).map(|k| self.copy(k).bytes()).collect()
    }
}
