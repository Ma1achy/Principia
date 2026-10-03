//! The shared proptest configuration: one case count, and a seed that is always fixed, so a failure names the seed
//! that reproduces it. Re-run a failure with `PROPTEST_RNG_SEED=<seed>` (proptest's own variable).

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, RngSeed, TestCaseError, TestError, TestRunner};
use std::fmt;

/// Cases per property: the calibration value of REQ-VAL-151 (R-203, R-71). 256, proptest's own default, confirmed by
/// the human at the M0 gate (R-376).
pub const CASES: u32 = 256;

/// True while [`CASES`] is the proposed, unconfirmed value (R-182, R-203); false since the M0 gate confirmed it
/// (R-376).
pub const CASES_PROVISIONAL: bool = false;

/// [`CASES`] with its status, as the tests print it.
pub fn cases_status() -> String {
    let status = if CASES_PROVISIONAL {
        "provisional until confirmed at the M0 gate"
    } else {
        "confirmed"
    };
    format!("prop::CASES = {CASES} ({status}; REQ-VAL-151, R-203, R-376)")
}

/// The config every property test uses: [`CASES`] cases, the given seed, no persistence file (the seed replaces it).
pub fn config(seed: u64) -> Config {
    Config {
        cases: CASES,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        ..Config::default()
    }
}

/// The seed to run with: `PROPTEST_RNG_SEED` if set, otherwise a fresh random one.
pub fn seed() -> u64 {
    match Config::default().rng_seed {
        RngSeed::Fixed(seed) => seed,
        RngSeed::Random => {
            use std::hash::{BuildHasher, RandomState};
            RandomState::new().hash_one(std::time::SystemTime::now())
        }
    }
}

/// A failed property: the seed it ran with, the minimal failing case and the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub seed: u64,
    pub case: String,
    pub reason: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "property failed: {}; minimal case: {}; re-run with PROPTEST_RNG_SEED={}",
            self.reason, self.case, self.seed
        )
    }
}

/// Runs `test` over `strategy` with `seed`, returning the failure instead of panicking.
pub fn check_with_seed<S: Strategy>(
    seed: u64,
    strategy: &S,
    test: impl Fn(S::Value) -> Result<(), TestCaseError>,
) -> Result<(), Failure> {
    TestRunner::new(config(seed))
        .run(strategy, test)
        .map_err(|e| match e {
            TestError::Fail(reason, case) => Failure {
                seed,
                case: format!("{case:?}"),
                reason: reason.to_string(),
            },
            TestError::Abort(reason) => Failure {
                seed,
                case: "(aborted)".into(),
                reason: reason.to_string(),
            },
        })
}

/// Runs a property with [`seed`]; on failure, prints the seed and panics with it.
pub fn run<S: Strategy>(strategy: &S, test: impl Fn(S::Value) -> Result<(), TestCaseError>) {
    if let Err(failure) = check_with_seed(seed(), strategy, test) {
        eprintln!("{failure}");
        panic!("{failure}");
    }
}

/// The checks of `prop::tests`, which `tests/controls.rs` calls so that each control runs its test's own check, not a
/// copy of it (REQ-VAL-158; R-215).
#[cfg(any(test, feature = "controls"))]
pub mod checks {
    use super::*;
    use proptest::prelude::*;
    use std::cell::Cell;

    /// Fails on any word at or above 2^20, and records the first failing word drawn (before shrinking), which depends
    /// on the seed; the minimal case after shrinking does not.
    pub fn first_failing_draw(seed: u64) -> (Failure, u32) {
        let first = Cell::new(None);
        let failure = check_with_seed(seed, &any::<u32>(), |x| {
            if x >= 1 << 20 && first.get().is_none() {
                first.set(Some(x));
            }
            prop_assert!(x < 1 << 20);
            Ok(())
        })
        .expect_err("the property was made to fail");
        (failure, first.get().expect("a failing draw was recorded"))
    }

    /// `prop_seed_is_printed_and_reproduces`'s check: `seed` and `again` fail on the same minimal case, for the same
    /// reason, after the same first failing draw.
    pub fn check_reproduces(seed: u64, again: u64) {
        let key = |seed| {
            let (failure, draw) = first_failing_draw(seed);
            (failure.case, failure.reason, draw)
        };
        assert_eq!(
            key(seed),
            key(again),
            "the printed seed did not reproduce the failing case"
        );
    }

    /// `prop_seed_config_has_no_persistence_file`'s check: `config` loads and saves no failure-persistence file (the
    /// fixed seed replaces it).
    pub fn check_no_persistence(config: &Config) {
        assert!(
            config.failure_persistence.is_none(),
            "the shared config sets a failure-persistence file"
        );
    }

    /// `prop_seed_runs_the_confirmed_case_count`'s check: a property that always holds runs [`CASES`] cases under
    /// `config`.
    pub fn check_runs_cases(config: Config) {
        let runs = Cell::new(0u32);
        TestRunner::new(config)
            .run(&any::<u32>(), |_| {
                runs.set(runs.get() + 1);
                Ok(())
            })
            .expect("the property holds");
        assert_eq!(
            runs.get(),
            CASES,
            "the shared config ran a different case count"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::checks::*;
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn prop_seed_is_printed_and_reproduces() {
        let panic = std::panic::catch_unwind(|| {
            run(&any::<u32>(), |x| {
                prop_assert!(x < 1 << 20);
                Ok(())
            })
        })
        .expect_err("the property was made to fail");
        let message = panic
            .downcast_ref::<String>()
            .expect("a formatted panic message");
        let printed: u64 = message
            .rsplit("PROPTEST_RNG_SEED=")
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("no seed in the failure message: {message}"));
        assert_eq!(first_failing_draw(printed).0.seed, printed);
        check_reproduces(printed, printed);
    }

    #[test]
    fn prop_seed_config_has_no_persistence_file() {
        check_no_persistence(&config(seed()));
    }

    #[test]
    fn prop_seed_runs_the_confirmed_case_count() {
        let status = cases_status();
        println!("{status}");
        assert!(
            status.contains("confirmed"),
            "CASES was confirmed at the M0 gate (R-376): {status}"
        );
        check_runs_cases(config(seed()));
    }
}
