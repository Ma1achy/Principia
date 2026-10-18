//! The engine-side adapter (RQ-247): it constructs the engine the app runs on and says whether it is the mock, which
//! the footer's "mock engine" tag and the window title read (REQ-GUI-167; RQ-248). It hands the app the canvas, §1's
//! sanctioned exception, when the engine side has one, the time source the app's clock reads (RQ-246), the profiler
//! trace the Profiler window reads, in profiler schema v1's shape (R-56; render_gui_spec §G5), and the Stain editor's
//! content and preview (render_gui_spec Part II §1; R-390), drawn through the canvas as the figure is. The figure is
//! drawn at the placement the app gives, the shown figure's rect in the viewport, so with the egui layer hidden it fills
//! the window (R-406); the Chart builder's quick render and the preview never take that placement.

use std::sync::Arc;

use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::profile::Trace;
use engine::contract::render_state::RenderState;
use engine::contract::sample::Sample;
use engine::contract::sim_config::{Latent, SimConfig};
use engine::contract::store::StateStore;

use crate::clock::TimeSource;
use crate::layout::Placement;

/// What the app needs from the engine side.
pub trait EngineSide {
    /// The engine, through its GUI-facing interface only.
    type Engine: EngineInterface;

    /// The engine.
    fn engine(&mut self) -> &mut Self::Engine;

    /// Whether the engine is the mock.
    fn is_mock(&self) -> bool;

    /// The canvas the field is drawn through over its whole viewport, as the Chart builder's quick render draws it;
    /// `None` draws none.
    fn canvas(&self) -> Option<Arc<dyn Canvas>>;

    /// The canvas the figure is drawn through, the field at `placement` in its viewport (R-406); `None` draws no
    /// figure.
    fn figure(&self, placement: Placement) -> Option<Arc<dyn Canvas>>;

    /// The time source the app's clock reads; `None` holds the clock.
    fn time_source(&mut self) -> Option<&mut dyn TimeSource>;

    /// The profiler trace, schema v1 (R-56), the Profiler window shows; `None` when the engine side has none.
    fn trace(&self) -> Option<&Trace>;

    /// The Stain editor's content for the stain as it stands: its library, assembled code, per-node code and Problems;
    /// `None` draws the panes empty.
    fn stain_content(&self) -> Option<crate::stain::Content>;

    /// The canvas the Stain editor's preview is drawn through, in the stain's look as it stands; `None` draws no
    /// preview.
    fn preview(&self) -> Option<Arc<dyn Canvas>>;

    /// Asks the engine side to recompile the stain (the Problems pane's Recompile).
    fn recompile(&mut self);

    /// `computeIC` (trajectory_viewing §1, §6): the sample of the IC `ic`, given at the figure point `at`, integrated
    /// under the sim key as it stands and read at the playhead, as the hover's sample is: what the Trajectory panel's
    /// pin re-integrates (R-453 D1, D5). `None` when the engine side has none.
    fn compute_ic(&mut self, ic: &Latent, at: [f64; 2]) -> Option<Sample>;
}

/// The real engine's data contract, with no figure, no time source and no trace: its frame loop and canvas are
/// TASK-M8-05's, its telemetry stream TASK-M8-27's and TASK-M8-28's.
pub struct RealSide {
    store: StateStore,
}

impl RealSide {
    /// The real engine's state store over the explicit initial state.
    pub fn new(sim: SimConfig, render: RenderState) -> Self {
        Self {
            store: StateStore::new(sim, render),
        }
    }
}

impl EngineSide for RealSide {
    type Engine = StateStore;

    fn engine(&mut self) -> &mut StateStore {
        &mut self.store
    }

    fn is_mock(&self) -> bool {
        false
    }

    fn canvas(&self) -> Option<Arc<dyn Canvas>> {
        None
    }

    /// None: the real engine's figure, and its fill past `[0,1]²` (REQ-GUI-180), are TASK-M8-05's.
    fn figure(&self, _placement: Placement) -> Option<Arc<dyn Canvas>> {
        None
    }

    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        None
    }

    fn trace(&self) -> Option<&Trace> {
        None
    }

    /// None: the real engine's Stain content is TASK-M7-24's and TASK-M8-19's.
    fn stain_content(&self) -> Option<crate::stain::Content> {
        None
    }

    /// None: the real engine's canvas is TASK-M8-05's.
    fn preview(&self) -> Option<Arc<dyn Canvas>> {
        None
    }

    /// Nothing: the real engine's compile is TASK-M7-24's.
    fn recompile(&mut self) {}

    /// None: the real engine's `computeIC` and its Trajectory panel are TASK-M8-10's and TASK-M8-11's.
    fn compute_ic(&mut self, _ic: &Latent, _at: [f64; 2]) -> Option<Sample> {
        None
    }
}

/// The mock engine, with its canvas when one is open, and its canned profiler trace (R-390).
pub struct MockSide {
    engine: crate::mock::MockEngine,
    canvas: Option<Arc<crate::mock::canvas::MockCanvas>>,
    trace: Trace,
}

impl MockSide {
    /// The mock engine `engine`, drawing through `canvas` if given.
    pub fn new(
        engine: crate::mock::MockEngine,
        canvas: Option<Arc<crate::mock::canvas::MockCanvas>>,
    ) -> Self {
        Self {
            engine,
            canvas,
            trace: crate::mock::profile::trace(),
        }
    }
}

impl EngineSide for MockSide {
    type Engine = crate::mock::MockEngine;

    fn engine(&mut self) -> &mut crate::mock::MockEngine {
        &mut self.engine
    }

    fn is_mock(&self) -> bool {
        true
    }

    /// The canvas, told the mock's chart and marks first, so it draws the stand-in for the chart as it stands
    /// (REQ-GUI-171), with the hover path and the tile bounds the mock draws into it.
    fn canvas(&self) -> Option<Arc<dyn Canvas>> {
        let canvas = self.canvas.clone()?;
        canvas.set_chart(self.engine.plane());
        canvas.set_marks(self.engine.marks());
        Some(canvas as Arc<dyn Canvas>)
    }

    /// The canvas, told the mock's chart, the placement and the marks over the viewport's span of the field first, so
    /// it draws the stand-in at the shown layout's scale over the whole viewport, the fake quads' bounds over it too.
    fn figure(&self, placement: Placement) -> Option<Arc<dyn Canvas>> {
        let canvas = self.canvas.clone()?;
        canvas.set_chart(self.engine.plane());
        canvas.set_placement(placement, self.engine.marks_over(placement.span()));
        Some(Arc::new(crate::mock::canvas::FigureCanvas(canvas)) as Arc<dyn Canvas>)
    }

    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        Some(self.engine.clock())
    }

    fn trace(&self) -> Option<&Trace> {
        Some(&self.trace)
    }

    fn stain_content(&self) -> Option<crate::stain::Content> {
        Some(crate::mock::stain::content(self.engine.graph()))
    }

    /// The preview through the canvas, told the chart and the look of the mock's graph first.
    fn preview(&self) -> Option<Arc<dyn Canvas>> {
        let canvas = self.canvas.clone()?;
        canvas.set_chart(self.engine.plane());
        canvas.set_look(crate::mock::stain::look(self.engine.graph()));
        Some(Arc::new(crate::mock::canvas::PreviewCanvas(canvas)) as Arc<dyn Canvas>)
    }

    fn recompile(&mut self) {
        self.engine.recompile();
    }

    fn compute_ic(&mut self, ic: &Latent, at: [f64; 2]) -> Option<Sample> {
        Some(self.engine.compute_ic(ic, at))
    }
}
