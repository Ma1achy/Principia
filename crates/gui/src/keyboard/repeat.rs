//! Held keys: delay, then repeat (render_gui_spec §G3, "the DAS / ARR model"). A press acts at once; held for the
//! delay (DAS), it acts again, then once every interval (ARR) until released. The system's own key repeats are
//! ignored, so the rate is the GUI's on every platform.
//!
//! The delay and the interval (REQ-GUI-146) are calibrations proposed here (R-71), used provisionally until the human
//! confirms them at the M8 gate (R-182): the delay is Windows' default keyboard delay, between X11's 660 ms and
//! Windows' shortest 250 ms; the interval is X11's default 25 repeats a second, within Windows' 2.5 to 30.

use eframe::egui::{Key, Modifiers};

/// The delay before a held key first repeats, in milliseconds: proposed, R-71 (REQ-GUI-146).
pub const DELAY_MS: u64 = 500;
/// The interval between repeats, in milliseconds (25 a second): proposed, R-71 (REQ-GUI-146).
pub const INTERVAL_MS: u64 = 40;

/// `seconds` of egui's input time in whole milliseconds, rounded.
pub fn millis(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    key: Key,
    modifiers: Modifiers,
    since_ms: u64,
    fired: u64,
}

/// The held key, if a repeating one is held: the last one pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Repeat {
    held: Option<Held>,
}

impl Repeat {
    /// `key` went down at `now_ms` with `modifiers`, which its repeats keep.
    pub fn press(&mut self, key: Key, modifiers: Modifiers, now_ms: u64) {
        self.held = Some(Held {
            key,
            modifiers,
            since_ms: now_ms,
            fired: 0,
        });
    }

    /// `key` went up; it stops repeating if it was the held one.
    pub fn release(&mut self, key: Key) {
        if self.held.is_some_and(|h| h.key == key) {
            self.held = None;
        }
    }

    /// Whether `key` is the held one.
    pub fn holds(&self, key: Key) -> bool {
        self.held.is_some_and(|h| h.key == key)
    }

    /// The repeats due by `now_ms` and not yet given, with the held key and its modifiers.
    pub fn due(&mut self, now_ms: u64) -> Option<(Key, Modifiers, u64)> {
        let held = self.held.as_mut()?;
        let elapsed = now_ms.saturating_sub(held.since_ms);
        let total = match elapsed.checked_sub(DELAY_MS) {
            Some(after) => after / INTERVAL_MS + 1,
            None => 0,
        };
        let new = total.saturating_sub(held.fired);
        held.fired = held.fired.max(total);
        (new > 0).then_some((held.key, held.modifiers, new))
    }

    /// When the next repeat falls due, in milliseconds of input time, while a key is held.
    pub fn next_due_ms(&self) -> Option<u64> {
        self.held
            .map(|h| h.since_ms + DELAY_MS + h.fired * INTERVAL_MS)
    }
}
