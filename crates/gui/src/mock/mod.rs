//! The mock engine (R-390): a test double of the engine's GUI-facing interface, always compiled (RQ-252); the `mock`
//! feature only chooses it as the binary's engine. It serves plausible GUI-sized snapshots, applies `SetField` with undo and redo (R-69), carries its
//! log entries in its snapshots (RQ-245), supplies the fake clock's deterministic tick (RQ-246) and draws the figure's
//! stand-in through the canvas trait (RQ-247), redrawn for its chart `(z₀, q₁, q₂)` as navigation edits it (R-390;
//! REQ-GUI-171). It raises the precision events from its zoom (R-54: the GUI reads the events, never a depth). It
//! earns no privilege: the app reaches it only through the interface, as it reaches the real engine
//! (gui_state_contract §1). Its values are placeholder content, not corpus values.

pub mod canvas;
pub mod clock;

use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::render_state::{Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice,
};
use engine::contract::snapshot::{FrameSummary, History, LiveMemory, Precision, Snapshot, Tier};
use engine::contract::store::{set_field_message, write};

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

/// The mock's initial `z₀`: 01_main.png's eight values (placeholder content).
pub const MOCK_Z0: Latent = [0.180, 0.410, 0.0, -0.227, 0.0, 0.312, 0.333, 0.333];

/// The mock's initial basis: `q₁ = 1·z_α`, `q₂ = 1·z_β`, the chart "z_α × z_β" at zoom 0, as 01_main.png's Chart
/// rows read (placeholder content).
pub const MOCK_Q1: Latent = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// See [`MOCK_Q1`].
pub const MOCK_Q2: Latent = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

/// The zoom, in octaves below the mock's initial scale, from which the mock reports `DECODE_SWITCHOVER` on the visible
/// quads (placeholder content: the real engine raises it from its decode, deep_zoom §2).
pub const MOCK_SWITCHOVER_OCTAVES: f64 = 20.0;
/// The zoom from which the mock reports `AT_F32_FLOOR` (placeholder content).
pub const MOCK_F32_FLOOR_OCTAVES: f64 = 23.0;

/// One undoable change: the edit restoring the value before, and the edit itself.
#[derive(Clone, Debug)]
struct Change {
    before: Edit,
    after: Edit,
}

/// The mock engine.
#[derive(Debug)]
pub struct MockEngine {
    sim: SimConfig,
    render: RenderState,
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
        let (sim, render) = state(0.0);
        Self {
            sim,
            render,
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

    /// The chart the stand-in is drawn for: the plane `(z₀, q₁, q₂)` as the mock holds it now.
    pub fn plane(&self) -> &Plane {
        &self.sim.plane
    }

    /// The zoom, in octaves below the initial scale: `−log₂` of the longer basis vector's length.
    fn octaves(&self) -> f64 {
        let norm = |q: &Latent| q.iter().map(|v| v * v).sum::<f64>().sqrt();
        -norm(&self.sim.plane.q1)
            .max(norm(&self.sim.plane.q2))
            .log2()
    }
}

impl Default for MockEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// The mock's initial state: the chart "z_α × z_β" at 01_main.png's `z₀`, unlocked, the other groups empty, the physics
/// kernel and `t = 0.0` (placeholder content).
fn state(t: f64) -> (SimConfig, RenderState) {
    (
        SimConfig {
            chart: Chart {},
            plane: Plane {
                z0: MOCK_Z0,
                q1: MOCK_Q1,
                q2: MOCK_Q2,
            },
            slice: Slice {},
            lock: Lock {
                locked: false,
                z_locked: [0.0; 8],
            },
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
        let before = write(&mut self.sim, &mut self.render, edit.edit.clone());
        self.push(
            Severity::Info,
            Source::Contract,
            set_field_message(&before, &edit),
        );
        if !edit.no_history {
            self.undo.push(Change {
                before,
                after: edit.edit,
            });
            self.redo.clear();
        }
    }

    fn snapshot(&mut self) -> Snapshot {
        let octaves = self.octaves();
        Snapshot {
            sim: self.sim.clone(),
            render: self.render.clone(),
            tier: Tier {},
            history: History {
                undo_depth: self.undo.len() as u32,
                redo_depth: self.redo.len() as u32,
            },
            precision: Precision {
                decode_switchover: octaves >= MOCK_SWITCHOVER_OCTAVES,
                at_f32_floor: octaves >= MOCK_F32_FLOOR_OCTAVES,
            },
            frame: MOCK_FRAME,
            log: std::mem::take(&mut self.log),
        }
    }

    fn undo(&mut self) {
        if let Some(change) = self.undo.pop() {
            write(&mut self.sim, &mut self.render, change.before.clone());
            self.redo.push(change);
        }
    }

    fn redo(&mut self) {
        if let Some(change) = self.redo.pop() {
            write(&mut self.sim, &mut self.render, change.after.clone());
            self.undo.push(change);
        }
    }
}
