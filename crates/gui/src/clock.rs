//! The GUI's clock (R-101; RQ-246): it reads `ViewUI`'s transport and, while playing, advances `RenderState`'s
//! playhead through a `SetField` marked "no history", by as much as its time source moved since the last frame. The
//! engine never reads the transport; the engine side may supply the time source the clock reads (the mock's
//! deterministic tick), and with none the clock holds.

use engine::contract::render_state::Playhead;
use engine::contract::set_field::{Edit, RenderField, SetField};

/// A time source the clock reads, in the playhead's units of `t`.
pub trait TimeSource {
    /// Moves the source on by one frame's tick.
    fn tick(&mut self);
    /// The source's time now.
    fn now(&self) -> f64;
    /// How many ticks a second the source expects while playing: the app asks for a frame that often.
    fn rate_hz(&self) -> f64;
}

/// The GUI's clock.
#[derive(Debug, Default)]
pub struct Clock {
    /// The source's time at the previous frame.
    last: Option<f64>,
    /// The playhead the clock last wrote, while playing; `None` until it next reads the snapshot's.
    t: Option<f64>,
}

impl Clock {
    /// A clock that has read no tick yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// One frame: ticks `source` and, while `playing`, returns the no-history `SetField` that advances the playhead by
    /// the time the source moved, from the clock's own playhead or else `snapshot_t`, the latest snapshot's.
    pub fn frame(
        &mut self,
        source: &mut dyn TimeSource,
        playing: bool,
        snapshot_t: f64,
    ) -> Option<SetField> {
        source.tick();
        let now = source.now();
        let last = self.last.replace(now);
        if !playing {
            self.t = None;
            return None;
        }
        let dt = now - last?;
        if dt == 0.0 {
            return None;
        }
        let t = self.t.unwrap_or(snapshot_t) + dt;
        self.t = Some(t);
        Some(SetField {
            edit: Edit::Render(RenderField::Playhead(Playhead { t })),
            no_history: true,
        })
    }

    /// Forgets the playhead the clock last wrote, so its next advance starts from the snapshot's: after an undo, a
    /// redo or an undoable edit has moved it.
    pub fn resync(&mut self) {
        self.t = None;
    }
}
