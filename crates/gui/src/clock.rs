//! The GUI's clock (R-101; RQ-246): it reads `ViewUI`'s transport and, while playing, advances `RenderState`'s
//! playhead through a `SetField` marked "no history", by as much as its time source moved since the last frame. The
//! engine never reads the transport; the engine side may supply the time source the clock reads (the mock's
//! deterministic tick), and with none the clock holds. It moves the playhead at the transport's speed and stops at
//! the horizon `T`: the app then pauses the transport.

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
    /// The seconds what turns at a rate in degrees per second (the shape sphere, REQ-GUI-157) is held at, for a source
    /// frozen as capture mode runs it; `None`, by default, for a running source, and what turns then reads the wall
    /// clock, so it turns at its rate in real time whatever the frame rate.
    fn held_seconds(&self) -> Option<f64> {
        None
    }
}

/// What the clock reads of `ViewUI`'s transport and the snapshot for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    /// Whether the transport plays.
    pub playing: bool,
    /// The transport's speed: units of `t` per unit of the source's time.
    pub speed: f64,
    /// The latest snapshot's playhead.
    pub t: f64,
    /// The horizon `T`, where playback stops.
    pub t_max: f64,
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

    /// One frame: ticks `source` and, while `playback` plays, returns the no-history `SetField` that advances the
    /// playhead by the time the source moved times the speed, from the clock's own playhead or else the latest
    /// snapshot's, stopping at the horizon `T`.
    pub fn frame(&mut self, source: &mut dyn TimeSource, playback: Playback) -> Option<SetField> {
        source.tick();
        let now = source.now();
        let last = self.last.replace(now);
        if !playback.playing {
            self.t = None;
            return None;
        }
        let dt = (now - last?) * playback.speed;
        let from = self.t.unwrap_or(playback.t);
        if dt == 0.0 || from >= playback.t_max {
            return None;
        }
        let t = (from + dt).min(playback.t_max);
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
