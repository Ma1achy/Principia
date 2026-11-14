//! `cargo xtask measure libm-pin` (R-991, REQ-CHART-212): whether the pinned libm makes the CPU evaluator's records
//! the same on every machine. The harness (`xtask/libm_pin/`) writes one run, an arm on a machine, as a file of record
//! digests over the first slice's three charts and grids; this module reads runs, counts the records that differ
//! between two of them at f64 and after narrowing, per chart and zoom, and gives the outcome. Spawning the builds and
//! the runs is `main.rs`'s.
//!
//! **The criterion, R-991's, written before any run.** PASS: under the pinned path, the count of records differing from
//! the Mac's is 0 on Linux, Windows and wasm, at both levels. The std path against the pinned on the Mac, and the std
//! path's cross-machine counts, are reported as the size of the change. The control, the pinned path with `sin` moved
//! one ulp (`measure-perturb-sin`), must give a non-zero count against the pinned at f64, or the harness is void. PASS
//! needs all three pinned arms, Linux, Windows and wasm, compared; with any not run and none differing the outcome is
//! INCOMPLETE, naming the arms missing, the arms that ran still reported (R-649: a measurement, never a gate).

use std::fmt::Write as _;

/// A run's header: its arm, the machine and the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// `pinned`, `std`, or a control arm.
    pub arm: String,
    /// The machine, `<os>-<arch>` or `wasm32-node-<version>`.
    pub host: String,
    /// The grid's width.
    pub width: usize,
    /// The grid's height.
    pub height: usize,
}

/// One run: its header and every record's digest, chart by chart, zoom by zoom, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The header.
    pub header: Header,
    /// The digests: high half the f64 level's hash, low half the narrowed record's.
    pub digests: Vec<u64>,
}

/// The charts and the zooms a run holds, in its order (the harness's `CHARTS` and `ZOOMS`).
pub const CHARTS: [&str; 3] = ["latent", "shape-sphere-exponential", "invariant-lz-e"];

/// The zooms, `k` in `2ᵏ`.
pub const ZOOMS: [u32; 3] = [0, 10, 20];

/// Reads a run from its file's bytes: the header line, then `CHARTS × ZOOMS × width × height` little-endian u64s.
pub fn parse(bytes: &[u8]) -> Result<Run, String> {
    let end = bytes
        .iter()
        .position(|&b| b == b'\n')
        .ok_or("libm-pin: no header line")?;
    let line =
        std::str::from_utf8(&bytes[..end]).map_err(|_| "libm-pin: the header is not UTF-8")?;
    let words: Vec<&str> = line.split(' ').collect();
    let [magic, version, arm, host, width, height] = words.as_slice() else {
        return Err(format!(
            "libm-pin: header `{line}` is not `libm-pin 1 <arm> <host> <width> <height>`"
        ));
    };
    if *magic != "libm-pin" || *version != "1" {
        return Err(format!("libm-pin: header `{line}` is not a version 1 run"));
    }
    let size = |s: &str| {
        s.parse::<usize>()
            .map_err(|_| format!("libm-pin: header `{line}`: `{s}` is not a size"))
    };
    let header = Header {
        arm: arm.to_string(),
        host: host.to_string(),
        width: size(width)?,
        height: size(height)?,
    };
    let body = &bytes[end + 1..];
    let want = CHARTS.len() * ZOOMS.len() * header.width * header.height * 8;
    if body.len() != want {
        return Err(format!(
            "libm-pin: {} holds {} bytes of digests, not {want}",
            header.host,
            body.len()
        ));
    }
    let digests = body
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().expect("8 bytes")))
        .collect();
    Ok(Run { header, digests })
}

/// The records that differ between two runs on one chart at one zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    /// The chart, an index of [`CHARTS`].
    pub chart: usize,
    /// The zoom, an index of [`ZOOMS`].
    pub zoom: usize,
    /// Records whose f64 values differ.
    pub wide: usize,
    /// Records whose narrowed f32 words differ.
    pub narrow: usize,
    /// Records compared.
    pub of: usize,
}

/// The counts of `b` against `a`, one per chart and zoom; `Err` where their grids differ.
pub fn compare(a: &Run, b: &Run) -> Result<Vec<Count>, String> {
    if (a.header.width, a.header.height) != (b.header.width, b.header.height)
        || a.digests.len() != b.digests.len()
    {
        return Err(format!(
            "libm-pin: {} and {} hold different grids",
            a.header.host, b.header.host
        ));
    }
    let n = a.header.width * a.header.height;
    let mut counts = Vec::new();
    for (k, (x, y)) in a.digests.chunks(n).zip(b.digests.chunks(n)).enumerate() {
        let (mut wide, mut narrow) = (0, 0);
        for (p, q) in x.iter().zip(y) {
            wide += usize::from(p >> 32 != q >> 32);
            narrow += usize::from(p & 0xFFFF_FFFF != q & 0xFFFF_FFFF);
        }
        counts.push(Count {
            chart: k / ZOOMS.len(),
            zoom: k % ZOOMS.len(),
            wide,
            narrow,
            of: n,
        });
    }
    Ok(counts)
}

/// The totals of `counts` at f64 and after narrowing.
pub fn totals(counts: &[Count]) -> (usize, usize) {
    counts
        .iter()
        .fold((0, 0), |(w, n), c| (w + c.wide, n + c.narrow))
}

/// The pinned arms PASS needs against the Mac's, by the prefix of their host: Linux, Windows and wasm.
pub const PINNED_ARMS: [&str; 3] = ["linux", "windows", "wasm32"];

/// The measurement's outcome (the module's note).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every one of [`PINNED_ARMS`] compared, each count 0 at both levels, the control non-zero at f64.
    Pass,
    /// A pinned cross-machine count non-zero, naming the machines.
    Unexplained(Vec<String>),
    /// No pinned count non-zero, but some of [`PINNED_ARMS`] not compared, naming them.
    Incomplete(Vec<String>),
    /// The control gave 0 at f64: the harness cannot see a one-ulp change.
    Void,
}

/// The outcome of the control's counts against the pinned run, `control`, and each other machine's pinned run against
/// the Mac's, `pinned`, named by its host.
pub fn outcome(control: &[Count], pinned: &[(String, Vec<Count>)]) -> Outcome {
    if totals(control).0 == 0 {
        return Outcome::Void;
    }
    let differing: Vec<String> = pinned
        .iter()
        .filter(|(_, counts)| totals(counts) != (0, 0))
        .map(|(host, _)| host.clone())
        .collect();
    if !differing.is_empty() {
        return Outcome::Unexplained(differing);
    }
    let missing: Vec<String> = PINNED_ARMS
        .iter()
        .filter(|arm| !pinned.iter().any(|(host, _)| host.starts_with(*arm)))
        .map(|arm| (*arm).to_owned())
        .collect();
    if missing.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Incomplete(missing)
    }
}

/// One comparison's lines: its title, then per chart and zoom the counts at f64 and after narrowing.
pub fn table(title: &str, counts: &[Count]) -> String {
    let mut s = format!("{title}\n");
    for c in counts {
        let _ = writeln!(
            s,
            "  {:<26} 2^{:<3} f64 {:>9}  narrowed {:>9}  of {}",
            CHARTS[c.chart], ZOOMS[c.zoom], c.wide, c.narrow, c.of
        );
    }
    let (w, n) = totals(counts);
    let _ = writeln!(s, "  {:<32} f64 {w:>9}  narrowed {n:>9}", "total");
    s
}
