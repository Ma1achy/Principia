//! The CPU field's wall time on the app's zero view (`gui::real::capture_view`: the latent chart `z_α × z_β` over its
//! default window, `z₀` at the origin, horizon 50), 64 × 64 samples, on a pool of a fixed worker count: five runs after
//! one warm-up, each reported, and the median; and the digests of its outcomes and image, which a change that
//! keeps the field bit for bit leaves as they were. A measurement: it reports and never fails on a number.
//!
//! With `stored`, the field with its records (`Cpu::stored_on`), and the per-sample distribution of what the records
//! store, read back through the read side.
//!
//! `cargo run --release -p engine --example field_timing [workers] [stored]`.

use std::num::NonZeroUsize;
use std::time::Instant;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::Cpu;
use engine::first_slice::Quad;
use engine::pool::Pool;
use kernel::chart::Latent;

const SIZE: [usize; 2] = [64, 64];

/// The app's zero view at its horizon of 50.
fn zero_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let quad = Quad {
        chart: Latent {
            z0: [0.0; 8],
            q1: e(0),
            q2: e(1),
        },
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t: 50.0 },
        ..quad.sim_key()
    }
}

fn main() {
    let workers = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .and_then(NonZeroUsize::new)
        .unwrap_or(NonZeroUsize::new(3).expect("3"));
    let stored = std::env::args().nth(2).as_deref() == Some("stored");
    let pool = Pool::with_threads(workers).expect("the pool");
    let sim = zero_view();
    let mut times = Vec::new();
    let mut digests = Vec::new();
    for run in 0..6 {
        let start = Instant::now();
        let field = if stored {
            let s = Cpu
                .stored_on(&pool, &sim, Window::SQUARE, SIZE)
                .expect("the records");
            if run == 0 {
                summary(&s);
            }
            s.field
        } else {
            Cpu.field_on(&pool, &sim, Window::SQUARE, SIZE)
                .expect("the field")
        };
        let s = start.elapsed().as_secs_f64();
        let outcomes: Vec<u8> = field
            .outcomes
            .iter()
            .flat_map(|o| [o.state.to_le_bytes(), o.detail.to_le_bytes()].concat())
            .collect();
        digests.push((fnv(&outcomes), fnv(&field.rgba8())));
        if run == 0 {
            println!("warm-up: {s:.3} s");
        } else {
            println!("run {run}: {s:.3} s");
            times.push(s);
        }
    }
    times.sort_by(f64::total_cmp);
    digests.dedup();
    println!("outcomes and image digests (FNV-1a 64), every run: {digests:x?}");
    println!(
        "64 x 64 zero view, {} on {workers} workers: median {:.3} s",
        if stored {
            "the field with its records"
        } else {
            "the field"
        },
        times[times.len() / 2]
    );
}

/// The 64-bit FNV-1a hash of `bytes`.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The per-sample distribution of what `s`'s records store, read back: the outcome histogram, and for each value the
/// march computes its least, median and greatest, and how many samples hold 0.
fn summary(s: &engine::field::Stored) {
    println!(
        "outcomes (state, detail, samples): {:?}",
        s.field
            .histogram()
            .iter()
            .map(|(o, n)| (o.state, o.detail, *n))
            .collect::<Vec<_>>()
    );
    let reads: Vec<_> = s.records.iter().map(|r| r.read(&s.read)).collect();
    let labelled = reads
        .iter()
        .zip(&s.field.outcomes)
        .filter(|(r, o)| (r.state, r.detail) == (o.state, o.detail))
        .count();
    println!(
        "records reading their outcome: {labelled} of {}",
        reads.len()
    );
    let show = |name: &str, mut v: Vec<f64>| {
        let zeros = v.iter().filter(|x| **x == 0.0).count();
        v.sort_by(f64::total_cmp);
        println!(
            "{name}: min {:.6e} median {:.6e} max {:.6e}, zero on {zeros} of {}",
            v[0],
            v[v.len() / 2],
            v[v.len() - 1],
            v.len()
        );
    };
    show(
        "t_end_step",
        reads.iter().map(|r| f64::from(r.t_end_step)).collect(),
    );
    show(
        "t_dmin_step",
        reads.iter().map(|r| f64::from(r.t_dmin_step)).collect(),
    );
    show(
        "total_substeps",
        reads.iter().map(|r| f64::from(r.total_substeps)).collect(),
    );
    show("theta", reads.iter().map(|r| f64::from(r.theta)).collect());
    show(
        "closure_min",
        reads.iter().map(|r| f64::from(r.closure_min)).collect(),
    );
    show(
        "mean_y",
        reads.iter().map(|r| f64::from(r.mean_y)).collect(),
    );
    show(
        "diffusion",
        reads.iter().map(|r| f64::from(r.diffusion)).collect(),
    );
    show("d_min", reads.iter().map(|r| f64::from(r.d_min)).collect());
    show(
        "dE_max",
        reads.iter().map(|r| f64::from(r.dE_max)).collect(),
    );
    show(
        "dE_reg_max",
        reads.iter().map(|r| f64::from(r.dE_reg_max)).collect(),
    );
    show(
        "energy_drift",
        reads.iter().map(|r| f64::from(r.energy_drift)).collect(),
    );
    show(
        "word length",
        reads
            .iter()
            .map(|r| f64::from(kernel::payload::fgw_length_raw(r.word)))
            .collect(),
    );
    show(
        "last_symbol (valid only)",
        reads
            .iter()
            .filter(|r| {
                kernel::payload::sd_last_symbol_valid(kernel::payload::fgw_length_raw(r.word))
            })
            .map(|r| f64::from(r.last_symbol))
            .collect(),
    );
    let ftle = reads
        .iter()
        .filter(|r| r.ftle_valid || !r.ftle.is_nan())
        .count();
    println!(
        "samples reading an FTLE: {ftle}; departed: {}; budget exhausted: {}",
        reads.iter().filter(|r| r.departed).count(),
        reads.iter().filter(|r| r.budget_exhausted).count()
    );
}
