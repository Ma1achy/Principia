//! The CPU field's wall time on the app's zero view (`gui::real::capture_view`: the latent chart `z_α × z_β` over its
//! default window, `z₀` at the origin, horizon 50), 64 × 64 samples, on a pool of a fixed worker count: five runs after
//! one warm-up, each reported, and the median; and the digests of its outcomes and image, which a change that
//! keeps the field bit for bit leaves as they were. A measurement: it reports and never fails on a number.
//!
//! `cargo run --release -p engine --example field_timing [workers]`.

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
    let pool = Pool::with_threads(workers).expect("the pool");
    let sim = zero_view();
    let mut times = Vec::new();
    let mut digests = Vec::new();
    for run in 0..6 {
        let start = Instant::now();
        let field = Cpu
            .field_on(&pool, &sim, Window::SQUARE, SIZE)
            .expect("the field");
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
        "64 x 64 zero view, {workers} workers: median {:.3} s",
        times[times.len() / 2]
    );
}

/// The 64-bit FNV-1a hash of `bytes`.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}
