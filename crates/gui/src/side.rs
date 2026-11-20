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
use crate::layout::{Layout, Placement};

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

    /// The figure as the app draws it this frame, told before [`EngineSide::figure`]: its viewport in pixels, the
    /// pixels it is clipped to and the canvas background (R-406; R-467 D2). Nothing by default: the real engine's
    /// frame loop is TASK-M8-05's.
    fn figure_view(&mut self, _view: FigureView) {}

    /// Fit's scale this frame, `None` outside Fit (R-468 E2, E3), told before [`EngineSide::show_span`]: the figure's
    /// screen scale, which asks for finer quads. No `SetField`. Nothing by default: the real engine's Fit is
    /// REQ-GUI-230's.
    fn fit(&mut self, _k: Option<f64>) {}

    /// The span of the field the figure shows this frame, `[s₀, t₀, s₁, t₁]` of the field's `[0, 1]²`, `t` Y-up: the
    /// engine side computes only what of it it does not already hold, keeping what it has, so a canvas resized by a
    /// splitter or by F3 is re-framed, never re-integrated (R-406; R-462 J4). It is no `SetField`: the contract has no
    /// viewport field.
    fn show_span(&mut self, span: [f64; 4]);

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

/// The figure as the app draws it (R-406; R-467 D2): its viewport's size in physical pixels, the shown rect's placement
/// in it, the pixels the draw is clipped to (`[x₀, y₀, x₁, y₁)`, rows from the top, as egui-wgpu's scissor rounds the
/// clip rect), and the canvas background, the colour the window is cleared to, which a pixel that is not part of the
/// chart draws in a presentation view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigureView {
    /// The viewport's size, in pixels.
    pub size: [u32; 2],
    /// The shown rect's placement in the viewport.
    pub placement: Placement,
    /// The pixels the figure is drawn on.
    pub clip: [u32; 4],
    /// The canvas background, 8-bit sRGB.
    pub background: [u8; 3],
}

impl FigureView {
    /// The figure of `layout` at `pixels_per_point`, the egui layer `shown` or hidden, over `background`: the viewport
    /// the window, clipped to the figure's rect as it stands (`Layout::figure_rect`).
    pub fn of(layout: &Layout, shown: bool, pixels_per_point: f32, background: [u8; 3]) -> Self {
        let extend = engine::contract::view_ui::F3Fill::Extend;
        let drawn = layout.drawn(shown, extend, pixels_per_point);
        Self::of_drawn(layout, &drawn, pixels_per_point, background)
    }

    /// The figure of `layout` as `drawn` at `pixels_per_point`, over `background`: the viewport the window, clipped to
    /// `drawn`'s rect.
    pub fn of_drawn(
        layout: &Layout,
        drawn: &crate::layout::Drawn,
        pixels_per_point: f32,
        background: [u8; 3],
    ) -> Self {
        let px = |v: f32| (v * pixels_per_point).round().max(0.0) as u32;
        let window = layout.window;
        let [w, h] = [px(window.width()), px(window.height())];
        let clip = drawn.clip;
        let at = |v: f32, min: f32, max: u32| px(v - min).min(max);
        Self {
            size: [w, h],
            placement: drawn.placement,
            clip: [
                at(clip.min.x, window.min.x, w),
                at(clip.min.y, window.min.y, h),
                at(clip.max.x, window.min.x, w),
                at(clip.max.y, window.min.y, h),
            ],
            background,
        }
    }
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

    /// Nothing: the real engine's figure, and its quads, are TASK-M8-05's, which follows R-462 J4.
    fn show_span(&mut self, _span: [f64; 4]) {}

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
        canvas.set_debug_view(self.engine.debug_view());
        let span = self.engine.figure_span(placement);
        canvas.set_placement(placement, self.engine.marks_over(span));
        Some(Arc::new(crate::mock::canvas::FigureCanvas(canvas)) as Arc<dyn Canvas>)
    }

    /// The mock told Fit's scale, at which its fake quads split (R-468 E3).
    fn fit(&mut self, k: Option<f64>) {
        self.engine.set_fit(k);
    }

    /// The canvas told the background and the debug view, and the mock told the figure's pixels and the region its
    /// test hook flags, which it counts for the legend's "Debug" key and warns of inside `[0,1]²` (R-467 D5, D7, D8).
    fn figure_view(&mut self, view: FigureView) {
        let invalid = self
            .canvas
            .as_ref()
            .map_or(crate::mock::canvas::NO_INVALID, |c| {
                c.set_background(view.background);
                c.invalid()
            });
        self.engine.observe(&view, invalid);
    }

    /// The mock integrates the fake quads over `span` it does not hold yet ([`crate::mock::MockEngine::show_span`]).
    fn show_span(&mut self, span: [f64; 4]) {
        self.engine.show_span(span);
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
