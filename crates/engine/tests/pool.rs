//! The CPU worker pool (`engine::pool`; TASK-M3-20).
//!
//! - `pool_reserve_default`: by default the pool leaves at least one core free, `1 ≤ size ≤ available − 1` on every
//!   core count from 2 to 512, one worker on one core; and the pool the process builds has that size.
//! - `pool_reserve_override`: a configured worker count is the pool's size, whatever the core count, above it
//!   included, and the built pool runs exactly that many workers.
//! - `pool_matches_serial_bitwise`: computeIC at every sample of a quad, plus ICs that fail at decode, on pools of 2, 3,
//!   5 and 7 workers, is the serial `computeQuad`/`computeICState` output bit for bit, every field of every `SimState`
//!   compared by its bits.
//! - `pool_field_matches_serial`: the CPU field on pools of 2, 3 and 7 workers is the one-worker field, sample for
//!   sample, on a frame that ends in more than one way.
//! - `pool_error_is_the_lowest_index`: where several samples fail, the pool returns the lowest index's error, as a
//!   serial loop does.
//!
//! - `pool_balances_neighbouring_samples`: on 2 and 3 workers, two neighbouring samples that each wait for the other
//!   both start, so a slow sample never holds the next one behind it on the same worker.
//!
//! Each has its control (`--features controls`): a reserve of 0, an ignored override, a pool output out of index order,
//! a later sample's error.

use std::num::NonZeroUsize;

use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::field::{Cpu, Field};
use engine::first_slice::Quad;
use engine::pool::{self, Pool, PoolConfig};
use engine::precision::{compute_ic, compute_ic_state, compute_quad, SimState64};
use validation::negative_control;

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).unwrap()
}

// ── The reserve ──────────────────────────────────────────────────────────────────────────────────────────────────

/// The core counts the reserve is checked at.
const CORES: std::ops::RangeInclusive<usize> = 1..=512;

/// Asserts that `size_of`, the default size on a core count, leaves a core free wherever there is one to leave.
fn check_reserve(size_of: impl Fn(NonZeroUsize) -> NonZeroUsize) {
    for available in CORES {
        let got = size_of(nz(available)).get();
        let most = available.saturating_sub(1).max(1);
        assert!(
            got <= most,
            "the default pool on {available} cores has {got} workers, leaving no core free"
        );
    }
}

#[test]
fn pool_reserve_default() {
    check_reserve(pool::default_size);
    check_reserve(|a| pool::size(a, &PoolConfig::default()));
    for available in CORES {
        assert!(pool::reserve(nz(available)) >= usize::from(available > 1));
    }
    let built = Pool::new(&PoolConfig::default()).unwrap();
    assert_eq!(
        built.threads(),
        pool::default_size(pool::available()).get(),
        "the pool the process builds by default is not the default size on its {} cores",
        pool::available()
    );
    println!(
        "this machine: {} cores available, the default pool {} workers",
        pool::available(),
        built.threads()
    );
}

negative_control!(
    pool_reserve_default,
    "a pool sized to every core leaves none free",
    expected = "leaving no core free",
    check_reserve(|available| available)
);

/// Asserts that `size_of`, the size on a core count under a config, is the config's override.
fn check_override(size_of: impl Fn(NonZeroUsize, &PoolConfig) -> NonZeroUsize) {
    for available in [1, 2, 4, 8, 64] {
        for threads in [1, 2, 3, 8, 65] {
            let config = PoolConfig {
                threads: Some(nz(threads)),
            };
            let got = size_of(nz(available), &config).get();
            assert_eq!(
                got, threads,
                "the pool on {available} cores configured for {threads} workers has {got}"
            );
        }
    }
}

#[test]
fn pool_reserve_override() {
    check_override(pool::size);
    for threads in [1, 2, 5] {
        let built = Pool::new(&PoolConfig {
            threads: Some(nz(threads)),
        })
        .unwrap();
        assert_eq!(
            built.threads(),
            threads,
            "the built pool does not run its configured workers"
        );
    }
}

negative_control!(
    pool_reserve_override,
    "a sizing that ignores the override",
    expected = "configured for",
    check_override(|available, _| pool::default_size(available))
);

// ── The pool changes no bit ──────────────────────────────────────────────────────────────────────────────────────

/// A generic latent chart (the Precision ring tests').
fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

/// The quad the bitwise tests run: 7 × 5, a count no pool size below divides.
fn quad() -> Quad<Latent<f32>> {
    Quad {
        chart: latent(),
        window: Window::SQUARE,
        size: [7, 5],
    }
}

/// The horizon: short, so the debug build stays quick, long enough for many steps.
fn key(t: f64) -> SimConfig {
    SimConfig {
        horizon: Horizon { t },
        ..quad().sim_key()
    }
}

/// The worker counts the pool is checked at besides one.
const WORKERS: [usize; 4] = [2, 3, 5, 7];

/// Every field of `s` as its bits (the declared tail padding aside): equal lists are equal `SimState`s bit for bit,
/// `−0` from `+0` and every NaN payload told apart.
fn words(s: &SimState64) -> Vec<u64> {
    let f = |x: f64| x.to_bits();
    let mut w: Vec<u64> = s.r.iter().chain(&s.p).flatten().map(|&x| f(x)).collect();
    w.extend([s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0].map(f));
    w.extend(
        [
            s.packed_a,
            s.packed_b,
            s.times,
            s.total_substeps,
            s.coll_floor_bits,
        ]
        .map(u64::from),
    );
    w.extend([s.closure_min, s._nhat_lag_0, s._nhat_lag_1, s._nhat_lag_2].map(f));
    w.extend([s.closure_step, s._reserved].map(u64::from));
    w
}

/// Asserts that `pooled` is `serial`, sample for sample, bit for bit.
fn check_bitwise(pooled: &[SimState64], serial: &[SimState64], workers: usize) {
    assert_eq!(
        pooled.len(),
        serial.len(),
        "the pool returned {} samples, not {}",
        pooled.len(),
        serial.len()
    );
    for (i, (a, b)) in pooled.iter().zip(serial).enumerate() {
        assert_eq!(
            words(a),
            words(b),
            "the pool's output on {workers} workers is not the serial output bit for bit at sample {i}"
        );
    }
}

/// The ICs the state check runs: the quad's decoded ICs, then one with a NaN and one with an infinity, which fail at
/// decode, interleaved so the failures sit inside the run.
fn ics() -> Vec<Ic<f64>> {
    let q = quad();
    let k = key(1.0);
    let mut out: Vec<Ic<f64>> = (0..q.len())
        .map(|i| {
            engine::precision::decode_only::<Latent<f32>, Defaults>(&q.chart, q.uv(i), &k)
                .unwrap()
                .ic
        })
        .collect();
    let mut nan = out[0];
    nan.r[1][0] = f64::NAN;
    let mut inf = out[1];
    inf.p[2][1] = f64::INFINITY;
    out.insert(4, nan);
    out.insert(11, inf);
    out
}

/// The quad's `computeIC` on `pool`, and the ICs' `computeICState`, in order.
fn pooled(pool: &Pool, t: f64) -> (Vec<SimState64>, Vec<SimState64>) {
    let q = quad();
    let k = key(t);
    let quad_states = pool
        .map(q.len(), |i| {
            compute_ic::<Latent<f32>, Defaults, Rk4>(&q.chart, q.uv(i), &k)
        })
        .unwrap();
    let ics = ics();
    let ic_states = pool
        .map(ics.len(), |i| compute_ic_state::<Rk4>(&ics[i], &k))
        .unwrap();
    (quad_states, ic_states)
}

/// The same, serially: `computeQuad` and a loop of `computeICState`.
fn serial(t: f64) -> (Vec<SimState64>, Vec<SimState64>) {
    let k = key(t);
    let quad_states = compute_quad::<Latent<f32>, Rk4>(&quad(), &k).unwrap();
    let ic_states = ics()
        .iter()
        .map(|ic| compute_ic_state::<Rk4>(ic, &k).unwrap())
        .collect();
    (quad_states, ic_states)
}

/// The horizon the bitwise check marches to.
const T: f64 = 2.0;

#[test]
fn pool_matches_serial_bitwise() {
    let (quad_serial, ic_serial) = serial(T);
    let states: std::collections::BTreeMap<u32, usize> =
        quad_serial
            .iter()
            .chain(&ic_serial)
            .fold(Default::default(), |mut m, s| {
                *m.entry(kernel::payload::sd_state(s.packed_a)).or_default() += 1;
                m
            });
    println!("the serial run's terminal states (code: samples): {states:?}");
    assert!(
        states.len() >= 2,
        "the run ends in one state only, so it checks little: {states:?}"
    );
    for workers in WORKERS {
        let pool = Pool::with_threads(nz(workers)).unwrap();
        assert_eq!(pool.threads(), workers);
        let (quad_pooled, ic_pooled) = pooled(&pool, T);
        check_bitwise(&quad_pooled, &quad_serial, workers);
        check_bitwise(&ic_pooled, &ic_serial, workers);
    }
}

/// The pool's quad output out of index order, rotated by one sample: what a pool that lost the index order returns.
#[cfg(feature = "controls")]
fn out_of_order() {
    let (quad_serial, _) = serial(T);
    let (mut quad_pooled, _) = pooled(&Pool::with_threads(nz(3)).unwrap(), T);
    quad_pooled.rotate_left(1);
    check_bitwise(&quad_pooled, &quad_serial, 3);
}

negative_control!(
    pool_matches_serial_bitwise,
    "a pool output out of index order",
    expected = "is not the serial output bit for bit",
    out_of_order()
);

/// Asserts that `pooled` is `serial`, sample for sample.
fn check_field(pooled: &Field, serial: &Field, workers: usize) {
    assert_eq!(pooled.size, serial.size);
    for (i, (a, b)) in pooled.outcomes.iter().zip(&serial.outcomes).enumerate() {
        assert_eq!(
            a, b,
            "the field on {workers} workers is not the one-worker field at sample {i}"
        );
    }
}

/// The plane the field check computes: the app's start view before the periodic `β` link (`tests/field.rs`), whose
/// frame ends in more than one way, at its horizon of 50.
fn field_sim() -> SimConfig {
    let e = |k: usize| {
        let mut q = [0.0f32; 8];
        q[k] = 1.0;
        q
    };
    let quad = Quad {
        chart: Latent {
            z0: [0.180, 0.410, 0.0, -0.227, 0.0, 0.312, 0.333, 0.333].map(|x: f64| x as f32),
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

/// The field the check computes: 9 × 7 samples of [`field_sim`]'s plane.
fn field_on(pool: &Pool) -> Field {
    Cpu.field_on(pool, &field_sim(), Window::SQUARE, [9, 7])
        .unwrap()
}

#[test]
fn pool_field_matches_serial() {
    let serial = field_on(&Pool::with_threads(NonZeroUsize::MIN).unwrap());
    let histogram = serial.histogram();
    println!("the one-worker field's histogram: {histogram:?}");
    assert!(
        histogram.len() >= 2,
        "the field ends in one way only, so order is not checked: {histogram:?}"
    );
    for workers in [2, 3, 7] {
        check_field(
            &field_on(&Pool::with_threads(nz(workers)).unwrap()),
            &serial,
            workers,
        );
    }
}

#[cfg(feature = "controls")]
fn field_out_of_order() {
    let serial = field_on(&Pool::with_threads(NonZeroUsize::MIN).unwrap());
    let mut pooled = field_on(&Pool::with_threads(nz(3)).unwrap());
    pooled.outcomes.rotate_left(1);
    check_field(&pooled, &serial, 3);
}

negative_control!(
    pool_field_matches_serial,
    "a field out of index order",
    expected = "is not the one-worker field",
    field_out_of_order()
);

// ── Errors ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// The indices that fail in the error check.
const FAILING: [usize; 3] = [5, 17, 40];

/// Asserts that `got` is the error of the lowest failing index.
fn check_lowest(got: Result<Vec<usize>, usize>, workers: usize) {
    assert_eq!(
        got,
        Err(FAILING[0]),
        "on {workers} workers the pool did not return the lowest failing index's error"
    );
}

#[test]
fn pool_error_is_the_lowest_index() {
    let f = |i: usize| if FAILING.contains(&i) { Err(i) } else { Ok(i) };
    let serial: Result<Vec<usize>, usize> = (0..64).map(f).collect();
    check_lowest(serial, 1);
    for workers in [1, 2, 3, 7] {
        check_lowest(Pool::with_threads(nz(workers)).unwrap().map(64, f), workers);
    }
}

negative_control!(
    pool_error_is_the_lowest_index,
    "a later sample's error",
    expected = "did not return the lowest failing index's error",
    check_lowest(Err(FAILING[2]), 3)
);

// ── Balance ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// How long a sample of the balance check waits for its neighbour.
const PARTNER_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// On `pool`, 64 samples of which the first two each wait for the other to start: whether both saw the other. A pool
/// that runs neighbouring samples as one worker's run, as rayon's default splitting of a range into a few leaf runs
/// does, holds sample 1 behind sample 0 on the same worker, so neither sees the other.
fn neighbours_overlap(pool: &Pool) -> bool {
    let started = std::sync::Mutex::new(0u32);
    let both = std::sync::Condvar::new();
    let saw = pool
        .map(64, |i| -> Result<bool, ()> {
            if i >= 2 {
                return Ok(true);
            }
            let mut n = started.lock().unwrap();
            *n += 1;
            both.notify_all();
            let (n, _) = both
                .wait_timeout_while(n, PARTNER_WAIT, |n| *n < 2)
                .unwrap();
            Ok(*n == 2)
        })
        .unwrap();
    saw[0] && saw[1]
}

/// Asserts that a slow sample on `pool` does not hold its neighbour.
fn check_balance(pool: &Pool) {
    assert!(
        neighbours_overlap(pool),
        "on {} workers a slow sample held its neighbour on the same worker",
        pool.threads()
    );
}

#[test]
fn pool_balances_neighbouring_samples() {
    for workers in [2, 3] {
        check_balance(&Pool::with_threads(nz(workers)).unwrap());
    }
}

negative_control!(
    pool_balances_neighbouring_samples,
    "one worker runs every sample in turn",
    expected = "held its neighbour on the same worker",
    check_balance(&Pool::with_threads(NonZeroUsize::MIN).unwrap())
);
