//! The per-quad forward-table builder: a quad's table built from `decode_only_with` on threads, narrowed once, is
//! bitwise equal to the per-sample evaluation.
//!
//! - `forward_table_builder_bitwise_per_sample`: on each chart of the first slice (the latent chart, the shape sphere's
//!   exponential map, the `(L_z, E)` chart) and each of its grids (`GRIDS`), at a size whose last chunk is short and a
//!   size of whole chunks, the builder's records on 1, 2, 3 and 8 threads equal, word for word, the records formed here
//!   sample by sample: `decode_only_with` at the sample, each component rounded to f32 by `as`, written with `write_ic`.
//! - `forward_table_builder_threads_by_chunk`: the work is on threads, chunk `c` of `CHUNK` samples on worker `c mod n`,
//!   none on the caller's thread, and each sample's node is written at its own index; no sample, no thread.
//! - `forward_table_builder_first_error`: where nodes fail, the builder returns the lowest failing index's error, as an
//!   evaluation sample by sample would, on any thread count, and a worker evaluates nothing after its first failure.
//!
//! **Domain** (pitfalls §10): one quad's evaluation, its chart, window and sim key held fixed.

use std::collections::{BTreeMap, HashSet};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::thread::ThreadId;

use engine::compute::ComputeError;
use engine::decode_dispatch::decode_only_with;
use engine::first_slice::{Quad, SliceChart};
use engine::forward_table::{build, build_with, CHUNK};
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{ic_words, write_ic};
use validation::first_slice::{Preset, GRIDS};
use validation::negative_control;

/// `n` threads.
fn threads(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("a thread count above 0")
}

// ── The table is the per-sample evaluation, bitwise ──────────────────────────────────────────────────────────────────

/// The sizes tested: 67 × 45 = 3015 samples, two whole chunks and a short one; 64 × 64 = 4096, four whole chunks.
const SIZES: [[usize; 2]; 2] = [[67, 45], [64, 64]];

/// The thread counts tested: one, two, one that divides no chunk count of [`SIZES`], and more than either has chunks.
const THREADS: [usize; 4] = [1, 2, 3, 8];

/// `quad`'s records formed here, sample by sample: `decode_only_with` at each sample, each component rounded to f32 by
/// `as`, written with `write_ic`.
fn per_sample<C: SliceChart>(quad: &Quad<C>) -> Vec<u32> {
    let key = quad.sim_key();
    let mut out = vec![0u32; quad.len() * ic_words()];
    for i in 0..quad.len() {
        let ic = decode_only_with::<C, Defaults>(&quad.chart, quad.uv(i), &key, None)
            .expect("the physics kernel decodes")
            .ic;
        let narrowed = Ic {
            m: ic.m.map(|x| x as f32),
            r: ic.r.map(|v| v.map(|x| x as f32)),
            p: ic.p.map(|v| v.map(|x| x as f32)),
        };
        write_ic(&mut out, i, &narrowed);
    }
    out
}

/// A builder of a quad's table on a thread count.
type Builder<C> = fn(&Quad<C>, NonZeroUsize) -> Result<Vec<u32>, ComputeError>;

/// `build`'s records equal [`per_sample`]'s on every grid, size and thread count, sample by sample.
fn check_bitwise<C: Preset + Sync>(build: Builder<C>) {
    let n = ic_words();
    for grid in GRIDS {
        for size in SIZES {
            let quad = C::quad(grid, size);
            let want = per_sample(&quad);
            for t in THREADS {
                let got = build(&quad, threads(t)).expect("the physics kernel decodes");
                assert_eq!(
                    got.len(),
                    want.len(),
                    "{} {} {size:?} on {t} threads: another number of words",
                    C::NAME,
                    grid.name()
                );
                for (i, (a, b)) in got.chunks(n).zip(want.chunks(n)).enumerate() {
                    assert_eq!(
                        a,
                        b,
                        "{} {} {size:?} on {t} threads: sample {i}'s record is not the per-sample evaluation's",
                        C::NAME,
                        grid.name()
                    );
                }
            }
        }
    }
}

/// Each chart of the first slice under `build`.
fn check_charts(
    latent: Builder<Latent<f32>>,
    sphere: Builder<ShapeSphere<f32>>,
    invariant: Builder<InvariantLE<f32>>,
) {
    check_bitwise(latent);
    check_bitwise(sphere);
    check_bitwise(invariant);
}

#[test]
fn forward_table_builder_bitwise_per_sample() {
    check_charts(build, build, build);
}

/// Today's decode, the chart evaluated at f32 and not narrowed from f64, written with `write_ic`: a table that is not
/// the f64 evaluation narrowed once.
#[cfg(feature = "controls")]
fn today_records<C: SliceChart>(quad: &Quad<C>, _: NonZeroUsize) -> Result<Vec<u32>, ComputeError> {
    use engine::first_slice::today;
    let mut out = vec![0u32; quad.len() * ic_words()];
    for (i, d) in today(quad).iter().enumerate() {
        write_ic(&mut out, i, &d.ic);
    }
    Ok(out)
}

negative_control!(
    forward_table_builder_bitwise_per_sample,
    "a table evaluated at f32, not narrowed once from f64, must fail",
    expected = "is not the per-sample evaluation's",
    check_charts(today_records, today_records, today_records)
);

// ── The work is on threads, by chunk ─────────────────────────────────────────────────────────────────────────────────

/// A builder of `samples` nodes on a thread count, each node from the closure.
type BuildWith<E> =
    fn(usize, NonZeroUsize, &(dyn Fn(usize) -> Result<Ic<f64>, E> + Sync)) -> Result<Vec<u32>, E>;

/// The node of sample `i` in the synthetic tables: its first mass `i`, exact at f32 below 2²⁴, and every other
/// component 0.
fn node_of(i: usize) -> Ic<f64> {
    Ic {
        m: [i as f64, 0.0, 0.0],
        r: [[0.0; 2]; 3],
        p: [[0.0; 2]; 3],
    }
}

/// On `t` threads over `samples` samples: each sample's node is written at its own index, the samples of a chunk are
/// evaluated on one thread, chunks `c` and `c'` on the same thread exactly when `c ≡ c' (mod t)`, as many threads as
/// lanes with a chunk, none of them the caller's.
fn check_lanes(build: BuildWith<()>, samples: usize, t: usize) {
    let seen: Mutex<Vec<Option<ThreadId>>> = Mutex::new(vec![None; samples]);
    let node = |i: usize| {
        seen.lock().expect("the record of threads")[i] = Some(std::thread::current().id());
        Ok(node_of(i))
    };
    let out = build(samples, threads(t), &node).expect("no node fails");
    let n = ic_words();
    assert_eq!(out.len(), samples * n, "the words of {samples} samples");
    for i in 0..samples {
        assert_eq!(
            out[i * n],
            (i as f32).to_bits(),
            "sample {i}'s node is not written at its index"
        );
    }
    let seen = seen.into_inner().expect("the record of threads");
    let mut by_chunk: BTreeMap<usize, ThreadId> = BTreeMap::new();
    for (i, id) in seen.iter().enumerate() {
        let id = id.unwrap_or_else(|| panic!("sample {i} was not evaluated"));
        let first = *by_chunk.entry(i / CHUNK).or_insert(id);
        assert_eq!(first, id, "sample {i}'s chunk is split across threads");
    }
    let chunks = samples.div_ceil(CHUNK);
    for (&c, a) in &by_chunk {
        for (&d, b) in &by_chunk {
            assert_eq!(
                a == b,
                c % t == d % t,
                "chunks {c} and {d} are not on the threads of their lanes mod {t}"
            );
        }
    }
    let distinct: HashSet<_> = by_chunk.values().collect();
    assert_eq!(
        distinct.len(),
        chunks.min(t),
        "{samples} samples on {t} threads: the threads that ran"
    );
    assert!(
        !distinct.contains(&std::thread::current().id()),
        "a chunk ran on the caller's thread, not on the builder's"
    );
}

/// [`check_lanes`] on a short last chunk, more threads than chunks, one thread, and no sample.
fn check_threads(build: BuildWith<()>) {
    check_lanes(build, 5 * CHUNK + 7, 4);
    check_lanes(build, 3 * CHUNK, 8);
    check_lanes(build, 2 * CHUNK + 1, 1);
    let empty = build(0, threads(4), &|i| panic!("node {i} of no sample"));
    assert_eq!(empty, Ok(Vec::new()), "no sample, no record");
}

#[test]
fn forward_table_builder_threads_by_chunk() {
    check_threads(|samples, t, node| build_with(samples, t, node));
}

negative_control!(
    forward_table_builder_threads_by_chunk,
    "a builder that evaluates every chunk on one thread must fail",
    expected = "are not on the threads of their lanes",
    check_threads(|samples, _, node| build_with(samples, NonZeroUsize::MIN, node))
);

// ── The first failing sample's error ─────────────────────────────────────────────────────────────────────────────────

/// Nodes failing at `bad`, each failure its index: the builder returns the lowest of them on 1, 4 and 8 threads, and
/// no sample after a worker's first failure in its lane is evaluated.
fn check_first_error(build: BuildWith<usize>) {
    let samples = 6 * CHUNK + 3;
    // Seven chunks, the last short. On 4 threads: chunk 1 (lane 1) at `CHUNK + 9`, chunk 2 (lane 2) at its first
    // sample, chunk 3 (lane 3), and chunk 5 (lane 1 again, after its first failure).
    let bad = [3 * CHUNK + 5, CHUNK + 9, 2 * CHUNK, 5 * CHUNK + 1];
    for t in [1, 4, 8] {
        let called = Mutex::new(vec![false; samples]);
        let node = |i: usize| {
            called.lock().expect("the record of calls")[i] = true;
            if bad.contains(&i) {
                Err(i)
            } else {
                Ok(node_of(i))
            }
        };
        assert_eq!(
            build(samples, threads(t), &node),
            Err(CHUNK + 9),
            "on {t} threads: not the lowest failing index's error"
        );
        let called = called.into_inner().expect("the record of calls");
        for (i, &was) in called.iter().enumerate().skip(CHUNK + 10) {
            let same_lane = (i / CHUNK) % t == 1 % t;
            assert!(
                !(same_lane && was),
                "on {t} threads: sample {i} was evaluated after its lane's first failure"
            );
        }
    }
}

#[test]
fn forward_table_builder_first_error() {
    check_first_error(|samples, t, node| build_with(samples, t, node));
}

negative_control!(
    forward_table_builder_first_error,
    "a builder that returns the highest failing index's error must fail",
    expected = "not the lowest failing index's error",
    check_first_error(|samples, _, node| {
        let mut last = None;
        for i in 0..samples {
            if let Err(e) = node(i) {
                last = Some(e);
            }
        }
        last.map_or_else(|| build_with(samples, NonZeroUsize::MIN, node), Err)
    })
);
