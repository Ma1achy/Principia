//! The first slice's record comparisons (TASK-M2-86; REQ-DEC-057), which `first_slice` (today's CPU f32 decode) and
//! `first_slice_gpu` (the GPU decode stage) both measure the narrowed f64 records against: each component's difference
//! in f32 ulps at `max(1, |x|)`, `x` the narrowed record's value, and the share of neighbouring sample pairs whose
//! records are bitwise identical. Measured and reported, never asserted.

use std::fmt;

use kernel::chart::{Chart, ValidationResult};
use kernel::stage::decode::ic_words;

/// A record's components, in `write_ic`'s word order.
pub const COMPONENTS: [&str; 15] = [
    "m0", "m1", "m2", "r0x", "r0y", "r1x", "r1y", "r2x", "r2y", "p0x", "p0y", "p1x", "p1y", "p2x",
    "p2y",
];

/// The upper bounds, in ulps, of [`Diff::bins`]' finite bins after the first (bitwise equal): `(0, 1]`, `(1, 4]`,
/// `(4, 16]`, `(16, 256]`, then `> 256` and a non-finite difference.
pub const BOUNDS: [f64; 4] = [1.0, 4.0, 16.0, 256.0];

/// The f32 spacing at `max(1, |x|)`: `2^(e − 23)`, `e` that value's binade, read on the bits; `x` finite.
pub fn ulp_at(x: f32) -> f64 {
    let y = f32::max(x.abs(), 1.0);
    let e = ((y.to_bits() >> 23) & 0xFF) as i32 - 127;
    f64::powi(2.0, e - 23)
}

/// `a`'s difference from the record's `x` in f32 ulps at `max(1, |x|)`: 0 where the bits are equal, infinite where
/// they differ and either is not finite.
pub fn ulps(x: f32, a: f32) -> f64 {
    if x.to_bits() == a.to_bits() {
        0.0
    } else if !(x.is_finite() && a.is_finite()) {
        f64::INFINITY
    } else {
        (f64::from(x) - f64::from(a)).abs() / ulp_at(x)
    }
}

/// The bin of a difference of `u` ulps between words whose bits are `equal` or not: 0 bitwise equal, then
/// [`BOUNDS`]' four, then `> 256`, then non-finite.
pub fn bin(equal: bool, u: f64) -> usize {
    if equal {
        0
    } else if !u.is_finite() {
        6
    } else {
        1 + BOUNDS.iter().take_while(|&&b| u > b).count()
    }
}

/// One record that differs: its sample, its largest difference and the component it is in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Differing {
    /// The sample's index.
    pub sample: usize,
    /// The largest component difference, in ulps.
    pub ulps: f64,
    /// The component of it, an index of [`COMPONENTS`].
    pub component: usize,
}

/// The comparison of two quads' records over the samples it was given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diff {
    /// The samples compared.
    pub samples: usize,
    /// Each record that differs in any word, in sample order.
    pub differing: Vec<Differing>,
    /// Per component, the largest difference in ulps.
    pub max: [f64; 15],
    /// Per component, the samples whose word differs.
    pub words: [usize; 15],
    /// Over every component of every sample, the count in each bin of [`bin`].
    pub bins: [usize; 7],
}

/// `records` against `other`, both `ic_words` words a sample, over the samples `which` admits.
pub fn compare(records: &[u32], other: &[u32], which: impl Fn(usize) -> bool) -> Diff {
    let n = ic_words();
    let mut d = Diff::default();
    for (i, (a, b)) in records.chunks(n).zip(other.chunks(n)).enumerate() {
        if !which(i) {
            continue;
        }
        d.samples += 1;
        let mut worst: Option<Differing> = None;
        for (k, (&x, &y)) in a.iter().zip(b).enumerate() {
            let u = ulps(f32::from_bits(x), f32::from_bits(y));
            d.bins[bin(x == y, u)] += 1;
            if x != y {
                d.words[k] += 1;
                d.max[k] = f64::max(d.max[k], u);
                if worst.is_none_or(|w| u > w.ulps) {
                    worst = Some(Differing {
                        sample: i,
                        ulps: u,
                        component: k,
                    });
                }
            }
        }
        d.differing.extend(worst);
    }
    d
}

impl fmt::Display for Diff {
    /// The counts, the bins and each component's largest difference.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words = self.samples * 15;
        writeln!(
            f,
            "    records differing: {} of {} ({:.4})",
            self.differing.len(),
            self.samples,
            self.differing.len() as f64 / self.samples.max(1) as f64
        )?;
        writeln!(
            f,
            "    words by ulps: equal {} | (0,1] {} | (1,4] {} | (4,16] {} | (16,256] {} | >256 {} | non-finite {} \
             (of {}; equal share {:.4})",
            self.bins[0],
            self.bins[1],
            self.bins[2],
            self.bins[3],
            self.bins[4],
            self.bins[5],
            self.bins[6],
            words,
            self.bins[0] as f64 / words.max(1) as f64
        )?;
        let worst: Vec<String> = COMPONENTS
            .iter()
            .zip(self.max.iter().zip(self.words))
            .map(|(c, (m, n))| format!("{c} {m:.3e}/{n}"))
            .collect();
        writeln!(f, "    largest ulps/words differing: {}", worst.join(", "))
    }
}

/// The neighbouring sample pairs of a `size[0] × size[1]` quad, across and up, both of whose samples `which` admits,
/// and how many of them have bitwise identical records.
pub fn identical_pairs(
    records: &[u32],
    size: [usize; 2],
    which: impl Fn(usize) -> bool,
) -> (usize, usize) {
    let n = ic_words();
    let record = |x: usize, y: usize| &records[(y * size[0] + x) * n..][..n];
    let (mut same, mut pairs) = (0, 0);
    for y in 0..size[1] {
        for x in 0..size[0] {
            for (dx, dy) in [(1, 0), (0, 1)] {
                let (x1, y1) = (x + dx, y + dy);
                if x1 < size[0]
                    && y1 < size[1]
                    && which(y * size[0] + x)
                    && which(y1 * size[0] + x1)
                {
                    pairs += 1;
                    same += usize::from(record(x, y) == record(x1, y1));
                }
            }
        }
    }
    (same, pairs)
}

/// Whether `chart` finds `(u, v)` `Valid`, a state the system integrates; a point validation projects or clamps is not.
pub fn in_domain<C: Chart>(chart: &C, [u, v]: [f64; 2]) -> bool {
    matches!(chart.validate(u, v), ValidationResult::Valid)
}
