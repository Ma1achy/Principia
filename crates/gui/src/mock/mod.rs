//! The mock engine (R-390): a test double of the engine's GUI-facing interface, always compiled (RQ-252); the `mock`
//! feature only chooses it as the binary's engine. It serves plausible GUI-sized snapshots, applies `SetField` with undo and redo (R-69), carries its
//! log entries in its snapshots (RQ-245), supplies the fake clock's deterministic tick (RQ-246) and draws the figure's
//! stand-in through the canvas trait (RQ-247), redrawn for its chart `(z₀, q₁, q₂)` as navigation edits it (R-390;
//! REQ-GUI-171). It raises the precision events from its zoom (R-54: the GUI reads the events, never a depth). It
//! earns no privilege: the app reaches it only through the interface, as it reaches the real engine
//! (gui_state_contract §1). It serves the legend's keys from its fake stain (`legend`), the sample under the cursor
//! the hover names (`sample`), and draws into its stand-in the hover path and, with tile bounds on, its fake quads'
//! bounds (`quads`), and gives the Profiler window a canned trace in profiler schema v1's shape (`profile`). It holds
//! 02_stain.png's stain graph, serves the Stain editor's library, code and Problems, and draws its preview in a look
//! taken from the graph, so an edit of the graph shows there (`stain`). It counts the pixels its invalid-pixel test
//! hook flags for the debug view's "Debug" key, and warns of those inside `[0,1]²` as a decoder bug, once per view
//! (`invalid`; R-467 D5, D7, D8). Its values are placeholder content, not corpus values.

pub mod canvas;
pub mod clock;
pub mod invalid;
pub mod legend;
pub mod profile;
pub mod quads;
pub mod sample;
pub mod stain;

use engine::contract::interface::EngineInterface;
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, Window,
};

use crate::side::FigureView;
use engine::contract::snapshot::{FrameSummary, History, LiveMemory, Precision, Snapshot, Tier};
use engine::contract::store::{set_field_message, write};

use self::canvas::Line;
use self::clock::MockClock;
use self::legend::FakeStain;

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

/// The mock's horizon `T`, 01_main.png's scrubber's end, "of 50" (placeholder content).
pub const MOCK_T_MAX: f64 = 50.0;

/// The tile bounds' colour over the stand-in: white at half opacity (a look choice, R-390).
pub const TILE_BOUNDS_RGBA: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
/// The hover path's colour, white as 01_main.png's, its opacity by its distance off the slice (a look choice, R-390).
pub const HOVER_RGB: [f32; 3] = [1.0, 1.0, 1.0];

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
    stain: FakeStain,
    /// What the test hook flags in the figure as last drawn.
    flagged: invalid::Flagged,
    /// The view the decoder-bug warning was last logged for.
    warned: Option<WarnedView>,
}

/// A view as the decoder-bug warning tells views apart (R-467 D7): `z₀` and the basis, the chart's window, and the
/// region the test hook flags, the mock's stand-in for the chart's validity.
#[derive(Clone, Debug, PartialEq)]
struct WarnedView {
    plane: Plane,
    window: Window,
    invalid: [f32; 4],
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
            stain: legend::default_stain(),
            flagged: invalid::Flagged::default(),
            warned: None,
        }
    }

    /// Whether View › Debug view is on (`RenderState`'s `debug_view`).
    pub fn debug_view(&self) -> bool {
        self.render.debug_view
    }

    /// What the test hook flags in the figure as last drawn.
    pub fn flagged(&self) -> invalid::Flagged {
        self.flagged
    }

    /// The figure as the app draws it, `view`, with the region `invalid` the test hook flags: the mock counts the
    /// flagged pixels for the debug view's "Debug" key, and, when some lie inside `[0,1]²` and the view is not the one
    /// last warned of, logs the decoder-bug warning, severity `warn`, source `app`, naming the mock's chart, the first
    /// such pixel and their count (R-467 D7, D8; render_gui_spec §G12). A repaint of the same view logs nothing; a view
    /// with none inside forgets the view last warned of.
    pub fn observe(&mut self, view: &FigureView, invalid: [f32; 4]) {
        self.flagged = invalid::flagged(view, invalid);
        let Some(inside) = self.flagged.inside else {
            // A view with no flagged pixel inside the square is a change of view too: the last warned view is
            // forgotten, so a return to it warns again (R-467 D7; render_gui_spec §G12).
            self.warned = None;
            return;
        };
        let seen = WarnedView {
            plane: self.sim.plane.clone(),
            window: self.sim.chart.window,
            invalid,
        };
        if self.warned.as_ref() != Some(&seen) {
            let chart = crate::explore::manifold_view::View::of(&self.sim.plane).name();
            self.push(
                Severity::Warn,
                Source::App,
                invalid::warning(&chart, &inside),
            );
            self.warned = Some(seen);
        }
    }

    /// The fake stain the legend is evaluated from.
    pub fn stain(&self) -> &FakeStain {
        &self.stain
    }

    /// Sets the fake stain, as an edit of the stain graph would (its editor is TASK-M6-29's).
    pub fn set_stain(&mut self, stain: FakeStain) {
        self.stain = stain;
    }

    /// The marks the mock draws into its stand-in: the hover path, while the hover is on the figure, and the fake
    /// quads' bounds, while the overlay set has tile bounds on.
    pub fn marks(&self) -> Vec<Line> {
        self.marks_over(quads::UNIT)
    }

    /// [`MockEngine::marks`] over the figure's span `span` (`[s₀, t₀, s₁, t₁]`): with the egui layer hidden, the
    /// fake quads' bounds over the whole window, the extra quads at the shown quads' size (R-406).
    pub fn marks_over(&self, span: [f64; 4]) -> Vec<Line> {
        let plane = &self.sim.plane;
        let mut lines = Vec::new();
        if self.render.overlays.tile_bounds {
            lines.extend(
                quads::bounds_over(plane, span)
                    .into_iter()
                    .map(|([a, b], outside)| Line {
                        ends: [a.map(|v| v as f32), b.map(|v| v as f32)],
                        rgba: TILE_BOUNDS_RGBA,
                        outside,
                    }),
            );
        }
        if let Some(at) = self.render.hover.at {
            let path = sample::hover_path(plane, at);
            lines.extend(path.windows(2).map(|w| {
                let [r, g, b] = HOVER_RGB;
                Line {
                    ends: [w[0].at.map(|v| v as f32), w[1].at.map(|v| v as f32)],
                    rgba: [r, g, b, w[0].alpha.min(w[1].alpha)],
                    outside: false,
                }
            }));
        }
        lines
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

    /// The stain graph as the mock holds it now, which its preview is drawn from.
    pub fn graph(&self) -> &StainGraph {
        &self.render.stain_graph
    }

    /// Recompiles the stain, as the Problems pane's Recompile asks: logs [`stain::RECOMPILED`] from the stain.
    pub fn recompile(&mut self) {
        self.push(Severity::Info, Source::Stain, stain::RECOMPILED.to_owned());
    }

    /// The mock's `computeIC` (trajectory_viewing §1, §6): the sample of the IC `ic`, given at the figure point `at`,
    /// integrated under the horizon as it stands and read at the playhead, as the hover's sample is (R-453 D5).
    pub fn compute_ic(&self, ic: &Latent, at: [f64; 2]) -> engine::contract::sample::Sample {
        sample::integrate(*ic, at, self.render.playhead.t, self.sim.horizon.t)
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

/// The mock's initial state: the chart "z_α × z_β" at 01_main.png's `z₀`, unlocked, the horizon [`MOCK_T_MAX`], every
/// overlay off, no hover, the stain graph 02_stain.png's (`stain`), the other groups empty, the physics kernel and
/// `t = 0.0` (placeholder content).
fn state(t: f64) -> (SimConfig, RenderState) {
    (
        SimConfig {
            chart: Chart {
                window: Window::SQUARE,
            },
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
            links: Links::defaults(),
            integrator: Integrator {},
            kernel_variant: KernelVariant::Physics,
            horizon: Horizon { t: MOCK_T_MAX },
            collision: Collision {},
            quality: Quality {},
        },
        RenderState {
            stain_graph: stain::default_graph(),
            overlays: Overlays::default(),
            palette: Palette {},
            playhead: Playhead { t },
            hover: Hover::default(),
            debug_view: false,
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
            legend: {
                let mut legend = legend::evaluate(&self.stain);
                if self.render.debug_view {
                    legend.keys.push(legend::debug_key(&self.flagged));
                }
                legend
            },
            sample: self.render.hover.at.map(|at| {
                sample::sample(
                    &self.sim.plane,
                    at,
                    self.render.playhead.t,
                    self.sim.horizon.t,
                )
            }),
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
