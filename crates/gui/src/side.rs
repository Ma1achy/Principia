//! The engine-side adapter (RQ-247): it constructs the engine the app runs on and says whether it is the mock, which
//! the footer's "mock engine" tag and the window title read (REQ-GUI-167; RQ-248). It hands the app the canvas, §1's
//! sanctioned exception, when the engine side has one, and the time source the app's clock reads (RQ-246).

use std::sync::Arc;

use engine::contract::canvas::Canvas;
use engine::contract::interface::EngineInterface;
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
}

/// The real engine's data contract, with no figure and no time source: its frame loop and canvas are TASK-M8-05's.
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
}

/// The mock engine, with its canvas when one is open (R-390).
#[cfg(any(test, feature = "mock"))]
pub struct MockSide {
    engine: crate::mock::MockEngine,
    canvas: Option<Arc<crate::mock::canvas::MockCanvas>>,
}

#[cfg(any(test, feature = "mock"))]
impl MockSide {
    /// The mock engine `engine`, drawing through `canvas` if given.
    pub fn new(
        engine: crate::mock::MockEngine,
        canvas: Option<Arc<crate::mock::canvas::MockCanvas>>,
    ) -> Self {
        Self { engine, canvas }
    }
}

#[cfg(any(test, feature = "mock"))]
impl EngineSide for MockSide {
    type Engine = crate::mock::MockEngine;

    fn engine(&mut self) -> &mut crate::mock::MockEngine {
        &mut self.engine
    }

    fn is_mock(&self) -> bool {
        true
    }

    fn canvas(&self) -> Option<Arc<dyn Canvas>> {
        self.canvas.clone().map(|c| c as Arc<dyn Canvas>)
    }

    fn time_source(&mut self) -> Option<&mut dyn TimeSource> {
        Some(self.engine.clock())
    }
}
