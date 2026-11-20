//! The per-quad forward-table builder on the CPU (R-926 item 6; R-935, RQ-766 (a); REQ-DEC-059; TASK-M2-89): a
//! quad's forward table, one exact node per sample, each node the chart's f64 evaluation at the sample
//! (`decode_only_with`, the CPU's evaluation, R-926 item 5), narrowed once to `Ic<f32>` and written with `write_ic` as
//! the record the integrator reads, the samples evaluated on threads.
//!
//! **The work on threads** ([`build_with`]): the records are cut into chunks of [`CHUNK`] samples, the last one
//! shorter, and chunk `c` goes to worker `c mod n` of `n` threads, each worker filling its own chunks in order, so no
//! two threads write the same words and which thread evaluates a sample changes nothing it writes: the table is
//! bitwise the per-sample evaluation's, whatever the thread count. A worker stops at its first failing sample, and the
//! builder returns the failure at the lowest sample index, the one an evaluation sample by sample meets first.
//!
//! **Applied per R-369** (implementation choices that change no result, written before any run): the chunk is 1024
//! samples, the register's `forward_table_chunk`; the assignment is interleaved and fixed, not taken from a shared
//! queue; a worker is spawned only for a lane that has a chunk; the records' buffer is allocated once, zeroed, at the
//! table's size.
//!
//! **Domain** (pitfalls §10): one quad's evaluation, the chart, its window and its sim key held fixed.

use std::num::NonZeroUsize;

use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{ic_words, write_ic};
use ledger::constants::FORWARD_TABLE_CHUNK;

use crate::compute::ComputeError;
use crate::decode_dispatch::decode_only_with;
use crate::first_slice::{narrow, Quad, SliceChart};

/// The samples a chunk of the table holds, the unit one thread fills at a time (applied per R-369, the module's note):
/// the register's `forward_table_chunk`.
pub const CHUNK: usize = FORWARD_TABLE_CHUNK.number() as usize;

/// `quad`'s forward table on `threads` threads: at each sample, `decode_only_with` at the sample's `(u, v)` under the
/// quad's sim key, `η_E` off, narrowed once ([`narrow`]) and written with `write_ic` at the sample, `ic_words` words a
/// sample, the integrator's input buffer. The first failing sample's error, by index, where a sample fails.
pub fn build<C: SliceChart + Sync>(
    quad: &Quad<C>,
    threads: NonZeroUsize,
) -> Result<Vec<u32>, ComputeError> {
    let key = quad.sim_key();
    build_with(quad.len(), threads, |i| {
        decode_only_with::<C, Defaults>(&quad.chart, quad.uv(i), &key, None).map(|out| out.ic)
    })
}

/// The table of `samples` nodes on `threads` threads, node `i` being `node(i)`, narrowed once and written with
/// `write_ic` at `i` (the module's note: chunks of [`CHUNK`] samples, chunk `c` on worker `c mod threads`). Where a
/// node fails, the error of the lowest failing index.
pub fn build_with<E, F>(samples: usize, threads: NonZeroUsize, node: F) -> Result<Vec<u32>, E>
where
    E: Send,
    F: Fn(usize) -> Result<Ic<f64>, E> + Sync,
{
    let words = ic_words();
    let mut out = vec![0u32; samples * words];
    let mut lanes: Vec<Vec<(usize, &mut [u32])>> = (0..threads.get()).map(|_| Vec::new()).collect();
    for (c, chunk) in out.chunks_mut(CHUNK * words).enumerate() {
        lanes[c % threads.get()].push((c * CHUNK, chunk));
    }
    let node = &node;
    let failed = std::thread::scope(|s| {
        let workers: Vec<_> = lanes
            .into_iter()
            .filter(|lane| !lane.is_empty())
            .map(|lane| s.spawn(move || fill(lane, node)))
            .collect();
        workers
            .into_iter()
            .filter_map(|w| w.join().unwrap_or_else(|p| std::panic::resume_unwind(p)))
            .min_by_key(|&(i, _)| i)
    });
    match failed {
        Some((_, e)) => Err(e),
        None => Ok(out),
    }
}

/// One worker's chunks, each `(first sample, its words)`, filled in order: the first failing sample and its error, if
/// one fails, the rest of the lane left unwritten.
fn fill<E>(
    lane: Vec<(usize, &mut [u32])>,
    node: &impl Fn(usize) -> Result<Ic<f64>, E>,
) -> Option<(usize, E)> {
    for (first, chunk) in lane {
        for k in 0..chunk.len() / ic_words() {
            match node(first + k) {
                Ok(ic) => write_ic(chunk, k, &narrow(&ic)),
                Err(e) => return Some((first + k, e)),
            }
        }
    }
    None
}
