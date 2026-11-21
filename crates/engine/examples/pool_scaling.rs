//! The evidence for the CPU worker pool's core reserve (`engine::pool`; TASK-M3-20), on the machine it runs on: the
//! CPU field of the app's start view, 48 × 48 samples to its horizon of 50, on pools of 1 to every available core,
//! and, while each pool works, how late a 1 ms ticker wakes (a stand-in for the compositor and the UI thread, which
//! need a core at a steady cadence). Reports the throughput, each added worker's marginal return, and the ticker's
//! lateness at p50, p99 and its maximum. A measurement: it reports and never fails on a number.
//!
//! `cargo run --release -p engine --example pool_scaling`. The calibration the reserve needs is this run on the
//! smallest-core target devices, with the app's own frame times beside it; this gives the shape on any machine.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::Cpu;
use engine::first_slice::Quad;
use engine::pool::{self, Pool};
use kernel::chart::Latent;

const SIZE: [usize; 2] = [48, 48];
const TICK: Duration = Duration::from_millis(1);

/// The app's start view (`gui::mock::state`) at its horizon of 50.
fn start_view() -> SimConfig {
    let e = |k: usize| core::array::from_fn(|i| if i == k { 1.0f32 } else { 0.0 });
    let z0: [f64; 8] = [
        0.180,
        0.31757693755701344,
        0.0,
        -0.227,
        0.0,
        0.312,
        0.333,
        0.333,
    ];
    let quad = Quad {
        chart: Latent {
            z0: z0.map(|x| x as f32),
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

/// The field on `workers` workers: its wall time in s, and the ticker's lateness past each 1 ms sleep, in ms, sorted.
fn run(workers: NonZeroUsize, sim: &SimConfig) -> (f64, Vec<f64>) {
    let pool = Pool::with_threads(workers).expect("the pool");
    let done = AtomicBool::new(false);
    std::thread::scope(|s| {
        let ticker = s.spawn(|| {
            let mut late = Vec::new();
            while !done.load(Ordering::Relaxed) {
                let start = Instant::now();
                std::thread::sleep(TICK);
                late.push((start.elapsed().saturating_sub(TICK)).as_secs_f64() * 1e3);
            }
            late
        });
        let start = Instant::now();
        let field = Cpu
            .field_on(&pool, sim, Window::SQUARE, SIZE)
            .expect("the field");
        let wall = start.elapsed().as_secs_f64();
        std::hint::black_box(field);
        done.store(true, Ordering::Relaxed);
        let mut late = ticker.join().expect("the ticker");
        late.sort_by(f64::total_cmp);
        (wall, late)
    })
}

fn at(v: &[f64], q: f64) -> f64 {
    let rank = ((q * v.len() as f64).ceil() as usize).clamp(1, v.len());
    v[rank - 1]
}

fn main() {
    let sim = start_view();
    let available = pool::available();
    println!(
        "pool-scaling: the start view's field, {} x {} samples to t = 50, on 1 to {available} workers ({available} cores available; the default pool {} workers)",
        SIZE[0],
        SIZE[1],
        pool::default_size(available)
    );
    // One warm-up run, so the first timed one pays no first-touch costs.
    run(NonZeroUsize::MIN, &sim);
    let mut base = None;
    let mut previous = None;
    for n in 1..=available.get() {
        let (wall, late) = run(NonZeroUsize::new(n).unwrap(), &sim);
        let rate = (SIZE[0] * SIZE[1]) as f64 / wall;
        let base = *base.get_or_insert(rate);
        let marginal = previous.map_or(String::new(), |p: f64| {
            format!(", worker {n} adds {:+.1}%", (rate / p - 1.0) * 100.0)
        });
        previous = Some(rate);
        println!(
            "  {n} workers: {wall:.2} s, {rate:.0} samples/s, {:.2}x one worker{marginal}; the 1 ms ticker late by p50 {:.3} ms, p99 {:.3} ms, max {:.3} ms over {} ticks",
            rate / base,
            at(&late, 0.5),
            at(&late, 0.99),
            late[late.len() - 1],
            late.len()
        );
    }
    println!("pool-scaling: a measurement; it reports and does not gate");
}
