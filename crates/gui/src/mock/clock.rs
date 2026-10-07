//! The fake clock's time source (RQ-246): a deterministic tick, which the app's clock reads when it runs on the mock,
//! frozen at a fixed time in capture mode so captures are stable. It reads no `ViewUI`: whether Time plays is the
//! app's clock's to decide. Its rate and `dt` are placeholder content, named here, not values of the corpus.

use crate::clock::TimeSource;

/// How many ticks a second the mock's clock runs at while playing (placeholder content, RQ-246).
pub const MOCK_TICK_HZ: f64 = 60.0;

/// How far the playhead's `t` moves per tick (placeholder content, RQ-246): one unit of `t` a second at
/// [`MOCK_TICK_HZ`].
pub const MOCK_DT: f64 = 1.0 / 60.0;

/// The mock's deterministic tick.
#[derive(Debug)]
pub struct MockClock {
    ticks: u64,
    frozen: bool,
}

impl MockClock {
    /// A clock that moves one tick per frame.
    pub fn running() -> Self {
        Self {
            ticks: 0,
            frozen: false,
        }
    }

    /// A clock frozen at its start, as capture mode runs it.
    pub fn frozen() -> Self {
        Self {
            ticks: 0,
            frozen: true,
        }
    }

    /// The wall-clock milliseconds its ticks stand for at [`MOCK_TICK_HZ`], for the mock's log entries.
    pub fn elapsed_ms(&self) -> u64 {
        self.ticks * 1000 / MOCK_TICK_HZ as u64
    }
}

impl TimeSource for MockClock {
    fn tick(&mut self) {
        if !self.frozen {
            self.ticks += 1;
        }
    }

    fn now(&self) -> f64 {
        self.ticks as f64 * MOCK_DT
    }

    fn rate_hz(&self) -> f64 {
        MOCK_TICK_HZ
    }
}
