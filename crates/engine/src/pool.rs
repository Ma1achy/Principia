//! The engine's CPU worker pool (dd_telemetry_and_tiers §6, "CPU": "Leave cores"): a rayon pool sized from the cores
//! this process may use minus a reserve, so the compositor, the browser and the OS keep a core while the engine works,
//! with the size overridable in [`PoolConfig`].
//!
//! **The pool changes no result bit.** [`Pool::map`] runs `f` at every index once, on whichever worker, and returns
//! the results in index order, the error of the lowest failing index where any fails, exactly as a serial loop over the
//! indices would: each sample is independent, so where it runs cannot change what it computes.
//!
//! **The reserve.** The plan is unclear here: it asks for "at least one, more on small-core devices, per the calibrated
//! reserve", and no calibration exists yet (the measurement it needs is UI and OS responsiveness under a full pool on
//! the smallest-core target devices, which this code cannot take). Chose one core on every device, the floor the plan
//! states, because a larger reserve on small devices would be a number with no measurement behind it; the calibration
//! proposal goes with the PR. On a one-core device nothing can be reserved: the pool still has its one worker.
//!
//! **The override lives outside the sim key.** The thread count changes no result, so it is not part of `SimConfig`
//! (it would change the canonical serialisation and every digest for nothing): it is the process's own setting, given
//! once through [`configure`] before the shared pool is first used.

use std::num::NonZeroUsize;
use std::sync::OnceLock;

use rayon::prelude::*;

use crate::compute::ComputeError;

/// The pool's configuration: an explicit worker count, or `None` for the default, the available cores less the
/// reserve ([`default_size`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolConfig {
    /// The worker count, overriding the default; any count, the machine's whole core count or more included.
    pub threads: Option<NonZeroUsize>,
}

/// The cores held back from the pool when `available` cores may be used: one, except on a one-core device, where
/// there is none to spare (the module's note).
pub fn reserve(available: NonZeroUsize) -> usize {
    usize::from(available.get() > 1)
}

/// The default pool size on `available` cores: `available` less its [`reserve`], at least one worker.
pub fn default_size(available: NonZeroUsize) -> NonZeroUsize {
    NonZeroUsize::new(available.get() - reserve(available)).unwrap_or(NonZeroUsize::MIN)
}

/// The pool size `config` gives on `available` cores: its override where it sets one, else [`default_size`].
pub fn size(available: NonZeroUsize, config: &PoolConfig) -> NonZeroUsize {
    config.threads.unwrap_or_else(|| default_size(available))
}

/// The cores this process may use (`std::thread::available_parallelism`, which honours affinity and cgroup quotas),
/// one where the count is not reported.
pub fn available() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

/// A pool of CPU workers.
pub struct Pool {
    workers: rayon::ThreadPool,
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pool")
            .field("threads", &self.threads())
            .finish()
    }
}

impl Pool {
    /// A pool sized by `config` on this process's [`available`] cores ([`size`]).
    pub fn new(config: &PoolConfig) -> Result<Pool, ComputeError> {
        Pool::with_threads(size(available(), config))
    }

    /// A pool of exactly `threads` workers, named `principia-cpu-<i>`. Its own pool, never rayon's global one.
    pub fn with_threads(threads: NonZeroUsize) -> Result<Pool, ComputeError> {
        let workers = rayon::ThreadPoolBuilder::new()
            .num_threads(threads.get())
            .thread_name(|i| format!("principia-cpu-{i}"))
            .build()
            .map_err(|e| ComputeError(format!("the CPU worker pool cannot start: {e}")))?;
        Ok(Pool { workers })
    }

    /// The pool's worker count.
    pub fn threads(&self) -> usize {
        self.workers.current_num_threads()
    }

    /// `f` at every index below `n`, on the pool's workers, in index order: what `(0..n).map(f).collect()` returns,
    /// bit for bit, the error of the lowest failing index where any fails. Every index runs, a failing one included.
    pub fn map<T: Send, E: Send>(
        &self,
        n: usize,
        f: impl Fn(usize) -> Result<T, E> + Sync,
    ) -> Result<Vec<T>, E> {
        // An indexed parallel iterator collects in index order, whatever order its items finish in. Each index is its
        // own job (`with_max_len(1)`): rayon otherwise splits a range only into a few leaf runs, which a worker then
        // runs whole, and a sample's cost varies by orders of magnitude (an early escape against a full march), so two
        // workers reached 1.66× one on the start view's field where three reached 3.0×. A job costs microseconds, a
        // sample milliseconds.
        let all: Vec<Result<T, E>> = self
            .workers
            .install(|| (0..n).into_par_iter().with_max_len(1).map(&f).collect());
        all.into_iter().collect()
    }
}

/// The process's shared pool, set by [`configure`] or built from the default config on first use.
static SHARED: OnceLock<Pool> = OnceLock::new();

/// Sets the shared pool from `config`, before its first use; an error once it is built (by an earlier call or a
/// first use), when the pool already running keeps its size.
pub fn configure(config: &PoolConfig) -> Result<(), ComputeError> {
    let pool = Pool::new(config)?;
    SHARED.set(pool).map_err(|_| {
        ComputeError(
            "the CPU worker pool is already running; configure it before its first use".to_owned(),
        )
    })
}

/// The process's shared pool: the one [`configure`] set, else one from the default config, built on first use.
pub fn shared() -> Result<&'static Pool, ComputeError> {
    if let Some(pool) = SHARED.get() {
        return Ok(pool);
    }
    let pool = Pool::new(&PoolConfig::default())?;
    // Another thread may have set it meanwhile: keep the one set first.
    Ok(SHARED.get_or_init(|| pool))
}
