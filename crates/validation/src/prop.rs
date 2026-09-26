//! The shared proptest configuration: one case count, and a seed that is always fixed, so a failure names the seed
//! that reproduces it. Re-run a failure with `PROPTEST_RNG_SEED=<seed>` (proptest's own variable).

use proptest::strategy::Strategy;
use proptest::test_runner::{Config, RngSeed, TestCaseError, TestError, TestRunner};
use std::fmt;

/// Cases per property: the calibration value of REQ-VAL-151 (R-203, R-71). 256 is the proposed value, proptest's own
/// default; it is provisional until the human confirms or changes it at the M0 gate, and is marked so (R-182).
pub const CASES: u32 = 256;

/// True while [`CASES`] is the proposed, unconfirmed value (R-182, R-203). Set false when the M0 gate confirms it.
pub const CASES_PROVISIONAL: bool = true;

/// [`CASES`] with its status, as the tests print it.
pub fn cases_status() -> String {
    let status = if CASES_PROVISIONAL {
        "provisional until confirmed at the M0 gate"
    } else {
        "confirmed"
    };
    format!("prop::CASES = {CASES} ({status}; REQ-VAL-151, R-203)")
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::cell::Cell;

    /// Fails on any word at or above 2^20, and records the first failing word drawn (before shrinking), which depends
    /// on the seed; the minimal case after shrinking does not.
    fn first_failing_draw(seed: u64) -> (Failure, u32) {
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
        let (failure, draw) = first_failing_draw(printed);
        let (again, draw_again) = first_failing_draw(printed);
        assert_eq!(failure.seed, printed);
        assert_eq!(
            (failure, draw),
            (again, draw_again),
            "the printed seed did not reproduce the failing case"
        );
        // Control: another seed fails on a different first draw, so the equality above is the seed's doing.
        let (_, other) = first_failing_draw(printed.wrapping_add(1));
        assert_ne!(draw, other, "control: two seeds drew the same failing case");
    }

    /// True if `config` loads and saves no failure-persistence file (the fixed seed replaces it).
    fn persists_nothing(config: &Config) -> bool {
        config.failure_persistence.is_none()
    }

    #[test]
    fn prop_seed_config_has_no_persistence_file() {
        assert!(
            persists_nothing(&config(seed())),
            "the shared config sets a failure-persistence file"
        );
        // Control: proptest's default config sets one, and the check rejects it.
        assert!(
            !persists_nothing(&Config::default()),
            "control: the persistence check does not read the field"
        );
    }

    /// Runs a property that always holds under `config` and counts the cases it ran.
    fn cases_run(config: Config) -> u32 {
        let runs = Cell::new(0u32);
        TestRunner::new(config)
            .run(&any::<u32>(), |_| {
                runs.set(runs.get() + 1);
                Ok(())
            })
            .expect("the property holds");
        runs.get()
    }

    #[test]
    fn prop_seed_runs_the_provisional_case_count() {
        let status = cases_status();
        println!("{status}");
        assert!(
            status.contains("provisional"),
            "CASES is provisional until the M0 gate (R-182, R-203): {status}"
        );
        assert_eq!(
            cases_run(config(seed())),
            CASES,
            "the shared config ran a different case count"
        );
        // Control: a config with half the cases runs half, so the count above is the config's doing.
        let half = Config {
            cases: CASES / 2,
            ..config(seed())
        };
        assert_ne!(
            cases_run(half),
            CASES,
            "control: the case count is not read"
        );
    }
}
