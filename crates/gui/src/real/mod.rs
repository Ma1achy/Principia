//! The real engine's figure: the field `engine::field` computes, its samples' stored records uploaded once and drawn
//! through the stain graph (`engine::contract::stain::view::StainView`) into the figure's rect at the placement the app
//! gives, as the mock's canvas draws its stand-in. The field is computed once, blocking, before the window opens;
//! nothing re-computes it when the view moves yet, and nothing does when the stain changes: an edit of the graph draws
//! the same records another way. `field.wgsl` draws the outcome palette's image of a field with no stain, what the
//! tests hold the outcome stain's picture to.

pub mod stain;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use engine::contract::canvas::{Canvas, CanvasContext};
use engine::contract::render_state::{NodeId, StainGraph};
use engine::contract::sim_config::SimConfig;
use engine::contract::stain::view::{self, CompileError, FrameInputs, Records, Stain, StainView};
use engine::field::{Field, FieldBackend, RecordBytes, Stored};

use crate::layout::Placement;

/// The state the real app starts on: the mock's (`crate::mock::state`), the chart "z_α × z_β" at 01_main.png's `z₀`,
/// the horizon 50, the kernel's default, the playhead at `t = 0`, so both engines open on the same slice. The plan is
/// unclear here (it names no start view); chose the mock's because the app's layout and captures are built around it.
/// Its stain is the outcome stain, the picture the app opens on: the mock's is placeholder content.
pub fn start() -> (SimConfig, engine::contract::render_state::RenderState) {
    let (sim, mut render) = crate::mock::state(0.0);
    render.stain_graph = engine::presets::outcome()
        .expect("the outcome stain is a graph, as the engine's tests check");
    (sim, render)
}

/// The state every image taken from the app is taken on: [`start`]'s, the chart "z_α × z_β" untilted over its default
/// window, with the slice centre `z₀` at the origin, every slice offset 0. The capture's view is the owner's choice of
/// 20 Nov 2026 for every image taken from the app; the app's window still opens on [`start`].
pub fn capture_view() -> (SimConfig, engine::contract::render_state::RenderState) {
    let (mut sim, render) = start();
    sim.plane.z0 = [0.0; 8];
    (sim, render)
}

/// The samples across and up of the field the app shows.
pub const FIELD_SIZE: [usize; 2] = [64, 64];

/// The field of `sim`'s chart over its window, `size` samples, through `backend`: the one call the GUI makes for it.
pub fn compute(
    backend: &dyn FieldBackend,
    sim: &SimConfig,
    size: [usize; 2],
) -> Result<Field, String> {
    backend
        .field(sim, sim.chart.window, size)
        .map_err(|e| format!("gui: the field: {e}"))
}

/// A field as it is drawn: its samples' records, as the GPU reads them, and the read side's values to read them at.
#[derive(Clone, Debug)]
pub struct Payload {
    /// The field.
    pub field: Field,
    /// Its records.
    pub bytes: RecordBytes,
    /// The read side's values.
    pub inputs: FrameInputs,
    /// What the records do not hold, for the Problems pane; none where they are the field's own.
    pub note: Option<String>,
}

/// The note on records that hold an outcome alone.
pub const OUTCOMES_ONLY: &str =
    "note: these records hold each sample's outcome alone; every other field reads 0";

/// The note on stored records' FTLE: the CPU march carries no shadow.
pub const NO_FTLE: &str = "note: no FTLE in these records: the CPU march has no shadow yet";

/// The read side's values at `read`, the masses those of the first record in `ic`: wrong past the first sample on a chart
/// that moves the masses.
// The plan is unclear here; chose the first sample's masses because the read side takes one set a draw.
fn inputs(read: &[f32; 2], counts: [u32; 2], ic: &[u8]) -> FrameInputs {
    let masses = [0, 1, 2].map(|k| {
        ic.get(4 * k..4 * k + 4)
            .and_then(|b| b.try_into().ok())
            .map_or(0.0, f32::from_le_bytes)
    });
    FrameInputs {
        ensemble: 0,
        // Absent: the canonical quiet NaN, as the prelude tests for it.
        ensemble_spread: f32::from_bits(0x7fc0_0000),
        masses,
        dt_macro: read[0],
        delta_0: read[1],
        n_renorm: counts[0],
        horizon_steps: counts[1],
    }
}

impl Payload {
    /// The field with its own records, read at the values they were stored at.
    pub fn stored(stored: Stored) -> Self {
        let bytes = stored.bytes();
        let r = stored.read;
        Self {
            inputs: inputs(
                &[r.dt_macro, r.delta_0],
                [r.n_renorm, r.horizon_steps],
                &bytes.ic,
            ),
            field: stored.field,
            bytes,
            note: Some(NO_FTLE.to_owned()),
        }
    }

    /// `field` with records that hold its outcomes and nothing else, for a backend that keeps none: the synthetic
    /// payload harness's, fresh but for `state` and `detail`.
    pub fn outcomes(field: Field) -> Result<Self, String> {
        let n = u32::try_from(field.outcomes.len()).map_err(|e| e.to_string())?;
        let grid = engine::synthetic::row(n)?;
        let mut set = engine::synthetic::Synthetic::flat(grid, 0);
        for (i, o) in (0..n).zip(&field.outcomes) {
            set.sample(i).state(o.state).detail(o.detail);
        }
        let b = set.bytes_at(stain::TIER.has_ftle);
        let bytes = RecordBytes {
            simstate: b.simstate,
            word: b.word,
            ic: b.ic,
        };
        Ok(Self {
            inputs: inputs(&[0.0, 0.0], [0, 0], &bytes.ic),
            field,
            bytes,
            note: Some(OUTCOMES_ONLY.to_owned()),
        })
    }
}

/// The field of `sim`'s chart over its window with its records, `size` samples, through `backend`: computed once. A
/// backend that keeps no records has its outcomes alone drawn, and says so.
pub fn payload(
    backend: &dyn FieldBackend,
    sim: &SimConfig,
    size: [usize; 2],
) -> Result<Payload, String> {
    match backend.stored(sim, sim.chart.window, size) {
        Ok(stored) => Ok(Payload::stored(stored)),
        Err(why) => {
            let mut p = Payload::outcomes(compute(backend, sim, size)?)?;
            p.note = Some(format!("{OUTCOMES_ONLY} ({why})"));
            Ok(p)
        }
    }
}

/// The stain view, which every draw of a frame is painted by, and what it was last told.
struct View {
    view: StainView,
    /// The graph last shown.
    graph: Option<StainGraph>,
    /// The stain last asked for, with its nodes' ids.
    asked: Option<(Vec<NodeId>, Stain)>,
    /// The params it holds, by node and name.
    params: BTreeMap<(NodeId, String), Vec<f64>>,
    /// The params not yet the drawn picture's, to ask again.
    waiting: BTreeMap<(NodeId, String), Vec<f64>>,
    /// The fragment key of the graph last shown.
    key: Option<u64>,
}

impl View {
    /// Shows `g`: asks for its stain where its nodes, occupants or wires changed, and sets each param that did. Nothing
    /// where `g` is the graph shown.
    fn show(&mut self, g: &StainGraph) -> Result<(), String> {
        if self.graph.as_ref() == Some(g) {
            return Ok(());
        }
        self.key = crate::stain::catalogue::key(g);
        let lowered = g.keyed().map_err(|e| e.to_string())?;
        if self.asked.as_ref() != Some(&lowered) {
            let keys: Vec<view::NodeKey> = lowered.0.iter().map(|id| u64::from(id.0)).collect();
            self.view
                .set_stain(&lowered.1, &keys)
                .map_err(|e| e.to_string())?;
            self.asked = Some(lowered);
        }
        self.waiting.clear();
        for n in g.nodes() {
            for u in g.uniforms(n.id).map_err(|e| e.to_string())? {
                let value = n.params.get(&u.name).unwrap_or(&u.default).clone();
                self.waiting.insert((n.id, u.name), value);
            }
        }
        self.graph = Some(g.clone());
        self.settle();
        Ok(())
    }

    /// Whether the stain last asked for is still compiling.
    fn compiling(&mut self) -> bool {
        let Some((_, asked)) = &self.asked else {
            return false;
        };
        // A superseded compile may still be with the worker after the stain asked for was found compiled.
        let cache = self.view.cache();
        cache.in_flight() > 0 && cache.rendered() != Some(asked)
    }

    /// Sets each param waiting that differs from the one held; one refused waits on. While the stain compiles all wait,
    /// and after it failed the failing nodes': the picture still drawn is the last valid one's, whose nodes may declare
    /// params of the same names.
    fn settle(&mut self) {
        if self.compiling() {
            return;
        }
        let errors = self.view.errors();
        let failed = |id: NodeId| {
            errors.iter().any(|e| match e {
                CompileError::Node { key, .. } => *key == u64::from(id.0),
                CompileError::Stain(_) => true,
            })
        };
        for ((id, name), value) in std::mem::take(&mut self.waiting) {
            let held = self.params.get(&(id, name.clone()));
            if held == Some(&value) {
                continue;
            }
            if failed(id) {
                self.waiting.insert((id, name), value);
                continue;
            }
            match self.view.set_param(u64::from(id.0), &name, &value) {
                Ok(_) => {
                    self.params.insert((id, name), value);
                }
                Err(_) => {
                    self.waiting.insert((id, name), value);
                }
            }
        }
    }
}

/// The real engine's canvas: its device, and the field's records drawn on it through the stain.
pub struct RealCanvas {
    context: CanvasContext,
    payload: Payload,
    /// The view, built on first use for the first target format it is drawn on, with that format.
    view: Mutex<Option<(wgpu::TextureFormat, View)>>,
    /// The graph the next draws show.
    stain: Mutex<StainGraph>,
    /// The placement of the placed figure's next draw.
    placement: Mutex<Placement>,
    /// The canvas background, 8-bit sRGB: the colour the window is cleared to.
    background: Mutex<[u8; 3]>,
    /// What the last draw reported.
    drawn: Mutex<stain::Drawn>,
    /// Whether, since [`RealCanvas::drew_compiling`] was last asked, a draw kept the last valid picture for a compile or
    /// another graph was set.
    drew_compiling: AtomicBool,
    /// The control's: every compile waited for and drawn, whatever was asked since.
    #[cfg(test)]
    pub(crate) completion_order: AtomicBool,
}

impl RealCanvas {
    /// [`RealCanvas::of`] `field`'s outcomes alone ([`Payload::outcomes`]).
    pub fn open(field: &Field) -> Result<Self, String> {
        Self::of(Payload::outcomes(field.clone())?)
    }

    /// Opens a device (`crate::mock::canvas::open`, its backend `PRIN_GPU_BACKEND`'s) for `payload`.
    pub fn of(payload: Payload) -> Result<Self, String> {
        Ok(Self::on(
            crate::mock::canvas::open("gui real canvas")?,
            payload,
        ))
    }

    /// The canvas on the open device `context`, drawing `payload` in the outcome stain until another is set.
    pub fn on(context: CanvasContext, payload: Payload) -> Self {
        Self {
            context,
            payload,
            view: Mutex::new(None),
            stain: Mutex::new(start().1.stain_graph),
            placement: Mutex::new(Placement::IDENTITY),
            background: Mutex::new(crate::mock::canvas::default_background()),
            drawn: Mutex::default(),
            drew_compiling: AtomicBool::new(false),
            #[cfg(test)]
            completion_order: Default::default(),
        }
    }

    /// The next draws show the graph `g`: the same records, drawn another way. A new one counts as drawn compiling
    /// ([`RealCanvas::drew_compiling`]), since the draw that finds it so comes after the frame that sets it.
    pub fn set_stain(&self, g: StainGraph) {
        let mut stain = self.stain.lock().expect("the stain lock");
        if *stain != g {
            self.drew_compiling.store(true, Ordering::SeqCst);
            *stain = g;
        }
    }

    /// The placed figure's next draw puts the field at `placement` in its viewport.
    pub fn set_placement(&self, placement: Placement) {
        *self.placement.lock().expect("the placement lock") = placement;
    }

    /// The placement the placed figure's next draw puts the field at.
    pub fn placement(&self) -> Placement {
        *self.placement.lock().expect("the placement lock")
    }

    /// The canvas background the next draws use, 8-bit sRGB.
    pub fn set_background(&self, background: [u8; 3]) {
        *self.background.lock().expect("the background lock") = background;
    }

    /// What drawing the stain reported: the last draw's report, and the note on the records.
    pub fn reported(&self) -> stain::Drawn {
        stain::Drawn {
            notes: self.payload.note.iter().cloned().collect(),
            ..self.drawn.lock().expect("the drawn lock").clone()
        }
    }

    /// Whether a draw since this was last asked was of a stain still compiling, or another stain was set. A report no draw renews does not count:
    /// the compile's end shows in a draw only, so with none a frame asked for it would be asked for for ever.
    pub fn drew_compiling(&self) -> bool {
        self.drew_compiling.swap(false, Ordering::SeqCst)
    }

    /// `f` of the view, once there is one.
    fn viewed<T>(&self, f: impl FnOnce(&mut StainView) -> T) -> Option<T> {
        let mut view = self.view.lock().expect("the view lock");
        view.as_mut().map(|(_, v)| f(&mut v.view))
    }

    /// How many pipelines have been compiled or tried to.
    pub fn compiles(&self) -> u64 {
        self.viewed(|v| v.compiles()).unwrap_or(0)
    }

    /// How many times the records have been uploaded.
    pub fn uploads(&self) -> u64 {
        self.viewed(|v| v.uploads()).unwrap_or(0)
    }

    /// Waits until every stain asked for has compiled: the tests'. A frame never waits for an edit's compile.
    pub fn wait(&self) {
        self.viewed(|v| v.wait());
    }

    /// A view on a target of `format`.
    fn build(&self, format: wgpu::TextureFormat) -> Result<View, String> {
        let RecordBytes { simstate, word, ic } = &self.payload.bytes;
        let view = StainView::new(
            &self.context.device,
            &self.context.queue,
            format,
            self.payload.field.size.map(|n| n as u32),
            Records { simstate, word, ic },
            stain::TIER,
            self.payload.inputs,
        )?;
        Ok(View {
            view,
            graph: None,
            asked: None,
            params: BTreeMap::new(),
            waiting: BTreeMap::new(),
            key: None,
        })
    }

    /// Draws the field into `pass` on a target of `format`, at `placement` in its viewport.
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        format: wgpu::TextureFormat,
        placement: Placement,
    ) {
        let mut drawn = stain::Drawn::default();
        if let Err(why) = self.try_draw(pass, format, placement, &mut drawn) {
            drawn.problems.push(why);
        }
        self.drew_compiling
            .fetch_or(drawn.compiling, Ordering::SeqCst);
        *self.drawn.lock().expect("the drawn lock") = drawn;
    }

    /// [`RealCanvas::draw`], what the pipelines report to `report`, or why it draws nothing.
    fn try_draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        format: wgpu::TextureFormat,
        placement: Placement,
        report: &mut stain::Drawn,
    ) -> Result<(), String> {
        let mut slot = self.view.lock().expect("the view lock");
        let graph = self.stain.lock().expect("the stain lock").clone();
        let (_, v) = match slot.take() {
            Some(kept) if kept.0 == format => slot.insert(kept),
            _ => {
                let mut v = self.build(format)?;
                // The first draw has no last picture to keep, so it waits for its first compile.
                v.show(&graph)?;
                v.view.wait();
                slot.insert((format, v))
            }
        };
        #[cfg(test)]
        if self.completion_order.load(Ordering::SeqCst) {
            v.view.wait();
        }
        // Asked for before a finished compile is taken, so one this graph supersedes is never drawn. A graph that cannot
        // be asked for leaves the last picture.
        if let Err(why) = v.show(&graph) {
            report.problems.push(why);
        }
        v.view.cache().poll();
        v.settle();
        v.view
            .set_background(*self.background.lock().expect("the background lock"));
        // The layer is drawn in a submit of its own, before the pass this is recorded into is submitted.
        let generation = v.view.cache().generation();
        let mut drawn = v.view.prepare();
        if v.view.cache().generation() != generation {
            // A compile finished between the poll and the draw, whose params are still to be set.
            v.settle();
            drawn = v.view.prepare();
        }
        let placement = view::Placement {
            centre: placement.centre,
            half: placement.half,
        };
        v.view.paint(pass, placement);
        report.key = v.key;
        report.compiling = v.compiling();
        let errors = v.view.errors();
        report
            .problems
            .extend(errors.iter().map(ToString::to_string));
        if report.problems.is_empty() {
            drawn?;
        }
        Ok(())
    }
}

impl Canvas for RealCanvas {
    fn context(&self) -> &CanvasContext {
        &self.context
    }

    /// The plain draw: the field over the whole viewport, as the Chart builder's quick render and the Stain editor's
    /// preview draw it.
    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.draw(pass, format, Placement::IDENTITY);
    }
}

/// The figure through the canvas trait: the placed draw, the field at [`RealCanvas::set_placement`]'s placement.
pub struct FigureCanvas(pub Arc<RealCanvas>);

impl Canvas for FigureCanvas {
    fn context(&self) -> &CanvasContext {
        self.0.context()
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.0.draw(pass, format, self.0.placement());
    }
}
