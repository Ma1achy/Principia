//! The engine-side adapter (RQ-247): it constructs the engine the app runs on and says whether it is the mock, which
//! the footer's "mock engine" tag and the window title read (REQ-GUI-167; RQ-248). It hands the app the canvas, §1's
//! sanctioned exception, when the engine side has one, the time source the app's clock reads (RQ-246), and the
//! profiler trace the Profiler window reads, in profiler schema v1's shape (R-56; render_gui_spec §G5).

use std::sync::Arc;

use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
use engine::contract::profile::Trace;
use engine::contract::render_state::RenderState;
use engine::contract::sim_config::SimConfig;
use engine::contract::store::StateStore;

use crate::clock::TimeSource;

/// What the app needs from the engine side.
pub trait EngineSide {
    /// The engine, through its GUI-facing interface only.
    type Engine: EngineInterface;

    /// The engine.
    fn engine(&mut self) -> &mut Self::Engine;

    /// Whether the engine is the mock.
    fn is_mock(&self) -> bool;

    /// The canvas the figure is drawn through; `None` draws no figure.
    fn canvas(&self) -> Option<Arc<dyn Canvas>>;

    /// The time source the app's clock reads; `None` holds the clock.
    fn time_source(&mut self) -> Option<&mut dyn TimeSource>;

    /// The profiler trace, schema v1 (R-56), the Profiler window shows; `None` when the engine side has none.
    fn trace(&self) -> Option<&Trace>;
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

    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        None
    }

    fn trace(&self) -> Option<&Trace> {
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

    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        Some(self.engine.clock())
    }

    fn trace(&self) -> Option<&Trace> {
        Some(&self.trace)
    }
}
