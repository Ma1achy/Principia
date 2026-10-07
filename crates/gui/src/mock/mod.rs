//! The mock engine (R-390): a test double of the engine's GUI-facing interface, always compiled (RQ-252); the `mock`
//! feature only chooses it as the binary's engine. It serves plausible GUI-sized snapshots, applies `SetField` with undo and redo (R-69), carries its
//! log entries in its snapshots (RQ-245), supplies the fake clock's deterministic tick (RQ-246) and draws the figure's
//! stand-in through the canvas trait (RQ-247). It earns no privilege: the app reaches it only through the interface,
//! as it reaches the real engine (gui_state_contract §1). Its values are placeholder content, not corpus values.

pub mod canvas;
pub mod clock;

use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice,
};
use engine::contract::snapshot::{FrameSummary, History, LiveMemory, Precision, Snapshot, Tier};

use self::clock::MockClock;

/// The mock's wall-clock frame time, in ms: a steady 60 Hz frame (placeholder content, R-390).
pub const MOCK_FRAME_MS: f64 = 16.7;

/// The mock's frame summary: plausible values (placeholder content, R-390). Every snapshot carries the same latest
/// frame, a steady stream, so its fps is 1000 / that `frame_ms`, as gui_state_contract §2 defines it (REQ-GUI-176);
/// 01_main.png's "60 fps · 4.1 ms" is illustrative (R-68).
pub const MOCK_FRAME: FrameSummary = FrameSummary {
    frame_ms: Some(MOCK_FRAME_MS),
    fps: Some(1000.0 / MOCK_FRAME_MS),
    quad_count: Some(1842),
    live_memory: Some(LiveMemory {
        heap_bytes: 148_000_000,
        gpu_bytes: 464_000_000,
    }),
};

/// The wall-clock time the mock's session starts at, for its entries' `at`: 12:03:59.410 UTC on 7 Oct 2026, the
/// console artboard's first time (placeholder content).
pub const MOCK_START_MS: u64 = 1_791_374_639_410;

/// The warning [`MockEngine::raise_warning`] logs.
pub const MOCK_WARNING: &str = "mock warning (placeholder)";
/// The error [`MockEngine::raise_error`] logs.
pub const MOCK_ERROR: &str = "mock error (placeholder)";

/// One undoable playhead change: the value before and after.
#[derive(Clone, Copy, Debug)]
struct Change {
    before: f64,
    after: f64,
}

/// The mock engine.
#[derive(Debug)]
pub struct MockEngine {
    playhead: f64,
    undo: Vec<Change>,
    redo: Vec<Change>,
    log: Vec<LogEntry>,
    clock: MockClock,
}

impl MockEngine {
    /// A mock at `t = 0.0`, its clock running.
    pub fn new() -> Self {
        Self::with_clock(MockClock::running())
    }

    /// A mock at `t = 0.0`, its clock frozen, as capture mode runs it (RQ-246).
    pub fn frozen() -> Self {
        Self::with_clock(MockClock::frozen())
    }

    fn with_clock(clock: MockClock) -> Self {
        Self {
            playhead: 0.0,
            undo: Vec::new(),
            redo: Vec::new(),
            log: Vec::new(),
            clock,
        }
    }

    /// The fake clock's time source, which the app's clock reads (RQ-246).
    pub fn clock(&mut self) -> &mut MockClock {
        &mut self.clock
    }

    /// Logs a warning from the stain, as the screenshot step `raise_warning` does.
    pub fn raise_warning(&mut self) {
        self.push(Severity::Warn, Source::Stain, MOCK_WARNING.to_owned());
    }

    /// Logs an error from the integrator, as the screenshot step `raise_error` does.
    pub fn raise_error(&mut self) {
        self.push(Severity::Error, Source::Integrator, MOCK_ERROR.to_owned());
    }

    fn push(&mut self, severity: Severity, source: Source, message: String) {
        self.log.push(LogEntry {
            severity,
            at: MOCK_START_MS + self.clock.elapsed_ms(),
            source,
            message,
        });
    }

    /// Sets the playhead, logging the applied edit (render_gui_spec §G12).
    fn write(&mut self, t: f64, no_history: bool) -> f64 {
        let before = std::mem::replace(&mut self.playhead, t);
        let marker = if no_history { " (no history)" } else { "" };
        self.push(
            Severity::Info,
            Source::Contract,
            format!("SetField Playhead.t {before} → {t}{marker}"),
        );
        before
    }
}

impl Default for MockEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// The mock's initial state: the surfaces' empty groups, the physics kernel and `t = 0.0` (placeholder content).
fn state(t: f64) -> (SimConfig, RenderState) {
    (
        SimConfig {
            chart: Chart {},
            plane: Plane {},
            slice: Slice {},
            lock: Lock {},
            links: Links {},
            integrator: Integrator {},
            kernel_variant: KernelVariant::Physics,
            horizon: Horizon {},
            collision: Collision {},
            quality: Quality {},
        },
        RenderState {
            stain_graph: StainGraph {},
            overlays: Overlays {},
            palette: Palette {},
            playhead: Playhead { t },
        },
    )
}

impl EngineInterface for MockEngine {
    fn set_field(&mut self, edit: SetField) {
        let t = match edit.edit {
            Edit::Sim(field) => match field {},
            Edit::Render(RenderField::Playhead(Playhead { t })) => t,
        };
        let before = self.write(t, edit.no_history);
        if !edit.no_history {
            self.undo.push(Change { before, after: t });
            self.redo.clear();
        }
    }

    fn snapshot(&mut self) -> Snapshot {
        let (sim, render) = state(self.playhead);
        Snapshot {
            sim,
            render,
            tier: Tier {},
            history: History {
                undo_depth: self.undo.len() as u32,
                redo_depth: self.redo.len() as u32,
            },
            precision: Precision {
                decode_switchover: false,
                at_f32_floor: false,
            },
            frame: MOCK_FRAME,
            log: std::mem::take(&mut self.log),
        }
    }

    fn undo(&mut self) {
        if let Some(change) = self.undo.pop() {
            self.playhead = change.before;
            self.redo.push(change);
        }
    }

    fn redo(&mut self) {
        if let Some(change) = self.redo.pop() {
            self.playhead = change.after;
            self.undo.push(change);
        }
    }
}
