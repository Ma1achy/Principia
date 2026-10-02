//! The mutants caps' fixture (TASK-M0-49, REQ-VAL-179, R-348). `change.diff` adds [`settled`] and [`provisioned`],
//! and each test waits until its function says yes. cargo-mutants' mutant returning `false` from [`settled`] makes
//! `work_settles` hang, which the per-mutant timeout records as a timeout; the one returning `false` from
//! [`provisioned`] makes `work_is_provisioned` allocate without bound, until the per-process memory cap kills its test
//! process. Run it only under both caps (CI's Linux runners): a Mac enforces the timeout alone (R-352).
//!
//! The negative controls (R-176) bound each: `CAPS_FIXTURE_HANG_SECS` ends the hang after that many seconds, under the
//! timeout, and `CAPS_FIXTURE_ALLOC_MIB` ends the allocation after that many MiB, under the cap; either test then
//! fails by its own assertion, not by a cap.

/// Whether the work has settled: at once.
pub fn settled() -> bool {
    true
}

/// Whether the work has the memory it needs: at once.
pub fn provisioned() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{provisioned, settled};
    use std::time::{Duration, Instant};

    /// The negative control's bound in `var`, if it is set.
    fn bound(var: &str) -> Option<u64> {
        let value = std::env::var(var).ok()?;
        Some(
            value
                .parse()
                .unwrap_or_else(|_| panic!("{var}={value:?} is not a whole number")),
        )
    }

    #[test]
    fn work_settles() {
        let limit = bound("CAPS_FIXTURE_HANG_SECS").map(Duration::from_secs);
        let start = Instant::now();
        while !settled() {
            assert!(
                limit.is_none_or(|l| start.elapsed() < l),
                "not settled after {limit:?}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn work_is_provisioned() {
        let limit = bound("CAPS_FIXTURE_ALLOC_MIB");
        let mut held: Vec<Vec<u8>> = Vec::new();
        while !provisioned() {
            assert!(
                limit.is_none_or(|l| (held.len() as u64) < l),
                "not provisioned after {limit:?} MiB"
            );
            // Each MiB written, so that it is resident as well as reserved.
            held.push(std::hint::black_box(vec![1u8; 1 << 20]));
        }
    }
}
