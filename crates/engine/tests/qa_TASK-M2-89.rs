//! QA's tests for TASK-M2-89, the per-quad forward-table builder, written from REQ-DEC-059's statement and verify
//! detail ("a quad's forward table must be built on the CPU from `decode_only_with` on threads and narrowed once,
//! bitwise equal to the per-sample evaluation (R-926 item 6), one exact node per sample (R-935, under RQ-766 (a))"):
//!
//! - `qa_m2_89_table_is_per_sample_evaluation`: on each chart of the first slice and each of REQ-DEC-057's grids, at
//!   quad shapes the implementer's test does not use (one sample, one column, one row, a nearly whole chunk, an odd
//!   non-square quad) and thread counts it does not use (5, 7, every core, 64), `build`'s records equal word for word
//!   the first slice's own per-sample evaluation (`engine::first_slice::evaluate`, TASK-M2-86's), and each record equals
//!   the sample decoded alone at its f64 point, narrowed by `as`, written alone into its own buffer;
//! - `qa_m2_89_every_component_narrowed_once`: every one of a node's components, including values that round, overflow
//!   to infinity and fall in the f32 subnormals, lands in its own word as the single f64-to-f32 rounding of its value;
//! - `qa_m2_89_one_node_per_sample`: each sample's node is evaluated exactly once, and no other index is asked for;
//! - `qa_m2_89_evaluation_on_threads`: the evaluation runs on more than one thread at once (the first and the last
//!   sample meet at a rendezvous that a builder on one thread can never reach);
//! - `qa_m2_89_first_failing_sample`: over pseudo-random failing sets, sample counts and thread counts, the builder
//!   returns the error of the lowest failing index, the one an evaluation sample by sample meets first.
//!
//! Each test has its negative control (R-176), run under `--features controls`.
//!
//! **Domain** (pitfalls §10): one quad's evaluation, its chart, window and sim key held fixed; the synthetic tests hold
//! the node function fixed.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use engine::compute::ComputeError;
use engine::decode_dispatch::decode_only_with;
use engine::first_slice::{evaluate, Quad, SliceChart};
use engine::forward_table::{build, build_with};
use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::stage::decode::{ic_words, read_ic, write_ic};
use validation::first_slice::{Preset, GRIDS};
use validation::negative_control;

fn threads(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("a thread count above 0")
}

/// Every core this process may use.
fn all_cores() -> usize {
    std::thread::available_parallelism().map_or(4, NonZeroUsize::get)
}

/// A small deterministic generator (xorshift64*), so the sampled domain is the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

// ── The table is the per-sample evaluation, bitwise ──────────────────────────────────────────────────────────────────

/// Quad shapes `[w, h]`: one sample; one column of 1025; one row of 1025; 31 × 33 = 1023; 97 × 23 = 2231.
const SHAPES: [[usize; 2]; 5] = [[1, 1], [1, 1025], [1025, 1], [31, 33], [97, 23]];

/// A builder of a quad's table on a thread count.
type Builder<C> = fn(&Quad<C>, NonZeroUsize) -> Result<Vec<u32>, ComputeError>;

/// Sample `i` of `quad` decoded alone at its f64 point under the quad's sim key, each component narrowed by `as`, and
/// written alone into its own buffer of one record.
fn alone<C: SliceChart>(quad: &Quad<C>, i: usize) -> Vec<u32> {
    let ic = decode_only_with::<C, Defaults>(&quad.chart, quad.uv(i), &quad.sim_key(), None)
        .expect("the physics kernel decodes")
        .ic;
    let narrowed = Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    };
    let mut out = vec![0u32; ic_words()];
    write_ic(&mut out, 0, &narrowed);
    out
}

fn check_per_sample<C: Preset + Sync>(build: Builder<C>) {
    let n = ic_words();
    let counts = [1, 5, 7, all_cores(), 64];
    for grid in GRIDS {
        for shape in SHAPES {
            let quad = C::quad(grid, shape);
            let want = evaluate(&quad).expect("the first slice evaluates");
            assert_eq!(
                want.len(),
                quad.len() * n,
                "the per-sample evaluation's words"
            );
            for (i, rec) in want.chunks(n).enumerate() {
                assert_eq!(
                    rec,
                    alone(&quad, i).as_slice(),
                    "{} {} {shape:?}: the oracles disagree at sample {i}",
                    C::NAME,
                    grid.name()
                );
            }
            for t in counts {
                let got = build(&quad, threads(t)).expect("the physics kernel decodes");
                assert_eq!(
                    got.len(),
                    want.len(),
                    "{} {} {shape:?} on {t} threads: the table has another number of words",
                    C::NAME,
                    grid.name()
                );
                if let Some(i) = (0..quad.len()).find(|&i| got[i * n..][..n] != want[i * n..][..n])
                {
                    panic!(
                        "{} {} {shape:?} on {t} threads: the table differs from the per-sample evaluation at sample {i}",
                        C::NAME,
                        grid.name()
                    );
                }
            }
        }
    }
}

fn check_all_charts(
    latent: Builder<Latent<f32>>,
    sphere: Builder<ShapeSphere<f32>>,
    invariant: Builder<InvariantLE<f32>>,
) {
    check_per_sample(latent);
    check_per_sample(sphere);
    check_per_sample(invariant);
}

#[test]
fn qa_m2_89_table_is_per_sample_evaluation() {
    check_all_charts(build, build, build);
}

/// A threaded builder that rounds the sample's point to f32 before evaluating it: a second narrowing, on the input,
/// which the zoomed grids must expose.
#[cfg(feature = "controls")]
fn point_rounded<C: SliceChart + Sync>(
    quad: &Quad<C>,
    t: NonZeroUsize,
) -> Result<Vec<u32>, ComputeError> {
    let key = quad.sim_key();
    build_with(quad.len(), t, |i| {
        let uv = quad.uv(i).map(|x| f64::from(x as f32));
        decode_only_with::<C, Defaults>(&quad.chart, uv, &key, None).map(|out| out.ic)
    })
}

negative_control!(
    qa_m2_89_table_is_per_sample_evaluation,
    "a builder that narrows the sample's point to f32 before its f64 evaluation must fail",
    expected = "the table differs from the per-sample evaluation",
    check_all_charts(point_rounded, point_rounded, point_rounded)
);

// ── Every component narrowed once ────────────────────────────────────────────────────────────────────────────────────

/// A builder of `samples` synthetic nodes on a thread count.
type BuildWith<E> =
    fn(usize, NonZeroUsize, &(dyn Fn(usize) -> Result<Ic<f64>, E> + Sync)) -> Result<Vec<u32>, E>;

/// Component `k` (0 to 14, in `m`, `r`, `p` order) of synthetic node `i`: no two components alike, most not f32
/// values, some negative, and on a cycle of samples a value past f32's range, one in the f32 subnormals and one exactly
/// halfway between two f32s (a tie, to even).
fn component(i: usize, k: usize) -> f64 {
    let base = (i as f64 + 1.0) * (1.0 + 0.1 * k as f64) + 1.0 / 3.0;
    let x = match (i + k) % 7 {
        0 => base * 1e39,
        1 => base * 1e-41,
        2 => 1.0 + f64::powi(2.0, -24) * (1.0 + 2.0 * (k as f64)),
        _ => base,
    };
    if k % 2 == 1 {
        -x
    } else {
        x
    }
}

fn node(i: usize) -> Ic<f64> {
    let c = |k| component(i, k);
    Ic {
        m: [c(0), c(1), c(2)],
        r: [[c(3), c(4)], [c(5), c(6)], [c(7), c(8)]],
        p: [[c(9), c(10)], [c(11), c(12)], [c(13), c(14)]],
    }
}

/// The 15 components of `ic` in `m`, `r`, `p` order.
fn flat<T: Copy>(ic: &Ic<T>) -> Vec<T> {
    let mut v = ic.m.to_vec();
    v.extend(ic.r.iter().flatten());
    v.extend(ic.p.iter().flatten());
    v
}

fn check_narrowed(build: BuildWith<()>) {
    let samples = 3 * 1024 + 11;
    for t in [1, 3, 16] {
        let out = build(samples, threads(t), &|i| Ok(node(i))).expect("no node fails");
        assert_eq!(
            out.len(),
            samples * ic_words(),
            "on {t} threads: the table's words"
        );
        for i in 0..samples {
            let got = flat(&read_ic(&out, i));
            for (k, (y, x)) in got.iter().zip(flat(&node(i))).enumerate() {
                assert_eq!(
                    y.to_bits(),
                    (x as f32).to_bits(),
                    "on {t} threads: sample {i} component {k}: {y:e} is not the one rounding of {x:e}"
                );
            }
        }
    }
}

#[test]
fn qa_m2_89_every_component_narrowed_once() {
    check_narrowed(|n, t, f| build_with(n, t, f));
}

negative_control!(
    qa_m2_89_every_component_narrowed_once,
    "a builder that rounds each component through f32 twice, the second time toward zero, must fail",
    expected = "is not the one rounding of",
    check_narrowed(|n, t, f| build_with(n, t, |i| {
        f(i).map(|ic| {
            // Round to f32 by `as`, then truncate the f64 to the f32 below in magnitude where it rounded up: a
            // narrowing that is not IEEE's single round to nearest.
            let toward_zero = |x: f64| {
                let y = x as f32;
                if f64::from(y).abs() > x.abs() && y.is_finite() {
                    f64::from(if y > 0.0 { y.next_down() } else { y.next_up() })
                } else {
                    f64::from(y)
                }
            };
            Ic {
                m: ic.m.map(toward_zero),
                r: ic.r.map(|v| v.map(toward_zero)),
                p: ic.p.map(|v| v.map(toward_zero)),
            }
        })
    }))
);

// ── One node per sample ──────────────────────────────────────────────────────────────────────────────────────────────

fn check_once(build: BuildWith<()>) {
    for (samples, t) in [
        (1, 8),
        (1023, 2),
        (4 * 1024 + 1, 5),
        (7 * 1024, all_cores()),
    ] {
        let calls: Vec<AtomicUsize> = (0..samples).map(|_| AtomicUsize::new(0)).collect();
        let stray = AtomicUsize::new(0);
        let out = build(samples, threads(t), &|i| {
            match calls.get(i) {
                Some(c) => {
                    c.fetch_add(1, Ordering::SeqCst);
                }
                None => {
                    stray.fetch_add(1, Ordering::SeqCst);
                }
            }
            Ok(node(i))
        })
        .expect("no node fails");
        assert_eq!(
            out.len(),
            samples * ic_words(),
            "{samples} samples: the table's words"
        );
        assert_eq!(
            stray.load(Ordering::SeqCst),
            0,
            "{samples} samples: an index past the quad was evaluated"
        );
        for (i, c) in calls.iter().enumerate() {
            let c = c.load(Ordering::SeqCst);
            assert_eq!(
                c, 1,
                "{samples} samples on {t} threads: sample {i} evaluated {c} times, not once"
            );
        }
    }
}

#[test]
fn qa_m2_89_one_node_per_sample() {
    check_once(|n, t, f| build_with(n, t, f));
}

negative_control!(
    qa_m2_89_one_node_per_sample,
    "a builder that evaluates each sample twice must fail",
    expected = "times, not once",
    check_once(|n, t, f| build_with(n, t, |i| {
        let _ = f(i);
        f(i)
    }))
);

// ── The evaluation is on threads ─────────────────────────────────────────────────────────────────────────────────────

/// How long the first and the last sample wait for each other: far beyond any scheduling delay, short enough that a
/// builder on one thread fails the check quickly.
const RENDEZVOUS: Duration = Duration::from_secs(10);

fn check_threads(build: BuildWith<()>) {
    let samples = 8 * 1024;
    let met = (Mutex::new(0usize), Condvar::new());
    let missed = AtomicUsize::new(0);
    let out = build(samples, threads(2), &|i| {
        if i == 0 || i == samples - 1 {
            let (count, cv) = &met;
            let mut n = count.lock().expect("the rendezvous");
            *n += 1;
            cv.notify_all();
            let (n, timeout) = cv
                .wait_timeout_while(n, RENDEZVOUS, |n| *n < 2)
                .expect("the rendezvous");
            if timeout.timed_out() && *n < 2 {
                missed.fetch_add(1, Ordering::SeqCst);
            }
        }
        Ok(node(i))
    })
    .expect("no node fails");
    assert_eq!(out.len(), samples * ic_words(), "the table's words");
    assert_eq!(
        missed.load(Ordering::SeqCst),
        0,
        "the first and the last sample never ran at the same time: the table is not built on threads"
    );
}

#[test]
fn qa_m2_89_evaluation_on_threads() {
    check_threads(|n, t, f| build_with(n, t, f));
}

negative_control!(
    qa_m2_89_evaluation_on_threads,
    "a builder on one thread must fail",
    expected = "never ran at the same time",
    check_threads(|n, _, f| build_with(n, NonZeroUsize::MIN, f))
);

// ── The first failing sample's error ─────────────────────────────────────────────────────────────────────────────────

fn check_first_error(build: BuildWith<usize>) {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..200 {
        let samples = 1 + rng.below(9 * 1024);
        let t = 1 + rng.below(12);
        let failing: Vec<usize> = (0..1 + rng.below(6)).map(|_| rng.below(samples)).collect();
        let lowest = *failing.iter().min().expect("a failing sample");
        let got = build(samples, threads(t), &|i| {
            if failing.contains(&i) {
                Err(i)
            } else {
                Ok(node(i))
            }
        });
        assert_eq!(
            got,
            Err(lowest),
            "case {case}: {samples} samples on {t} threads failing at {failing:?}: not the lowest failing sample's error"
        );
    }
}

#[test]
fn qa_m2_89_first_failing_sample() {
    check_first_error(|n, t, f| build_with(n, t, f));
}

negative_control!(
    qa_m2_89_first_failing_sample,
    "a builder that reports the highest failing sample's error must fail",
    expected = "not the lowest failing sample's error",
    check_first_error(|n, t, f| {
        let highest = (0..n).rev().find_map(|i| f(i).err());
        match highest {
            Some(e) => Err(e),
            None => build_with(n, t, f),
        }
    })
);
