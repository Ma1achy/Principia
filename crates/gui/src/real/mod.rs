//! The real engine's figure: the field `engine::field` computes, its samples' stored records uploaded once and drawn
//! through the stain graph (`engine::contract::stain::view::StainView`) into the figure's rect at the placement the app
//! gives, as the mock's canvas draws its stand-in. The field is computed once, blocking, before the window opens;
//! nothing re-computes it when the view moves yet, and nothing does when the stain changes: an edit of the graph draws
//! the same records another way. `field.wgsl` draws the outcome palette's image of a field with no stain, what the
//! tests hold the outcome stain's picture to.

pub mod stain;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use engine::contract::canvas::{Canvas, CanvasContext};
use engine::contract::render_state::{NodeId, StainGraph};
use engine::contract::sim_config::SimConfig;
use engine::contract::stain::view::{self, FrameInputs, Records, Stain, StainView};
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

/// One stain view and what it was last told.
struct View {
    view: StainView,
    /// The graph last shown.
    graph: Option<StainGraph>,
    /// The stain last asked for, with its nodes' ids.
    asked: Option<(Vec<NodeId>, Stain)>,
    /// The params it holds, by node and name.
    params: BTreeMap<(NodeId, String), Vec<f64>>,
    /// The params it refused, to ask again: a node whose new occupant is still compiling declares none of its params.
    waiting: BTreeMap<(NodeId, String), Vec<f64>>,
}

impl View {
    /// Shows `g`: asks for its stain where its nodes, occupants or wires changed, and sets each param that did. Nothing
    /// where `g` is the graph shown.
    fn show(&mut self, g: &StainGraph) -> Result<(), String> {
        if self.graph.as_ref() == Some(g) {
            return Ok(());
        }
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

    /// Sets each param waiting that differs from the one held; one refused waits on.
    fn settle(&mut self) {
        for ((id, name), value) in std::mem::take(&mut self.waiting) {
            let held = self.params.get(&(id, name.clone()));
            if held == Some(&value) {
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

/// The views of one target format: the placed figure's and the plain draws', apart, as each has one placement and the
/// two may be drawn in one frame (the figure, and the Chart builder's quick render).
struct Views {
    format: wgpu::TextureFormat,
    views: [Option<View>; 2],
}

/// The real engine's canvas: its device, and the field's records drawn on it through the stain.
pub struct RealCanvas {
    context: CanvasContext,
    payload: Payload,
    /// The views, built on first use for the first target format they are drawn on.
    views: Mutex<Option<Views>>,
    /// The graph the next draws show.
    stain: Mutex<StainGraph>,
    /// The placement of the placed figure's next draw.
    placement: Mutex<Placement>,
    /// The canvas background, 8-bit sRGB: the colour the window is cleared to.
    background: Mutex<[u8; 3]>,
    /// The last draw's errors.
    problems: Mutex<Vec<String>>,
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
            views: Mutex::new(None),
            stain: Mutex::new(start().1.stain_graph),
            placement: Mutex::new(Placement::IDENTITY),
            background: Mutex::new(crate::mock::canvas::default_background()),
            problems: Mutex::new(Vec::new()),
        }
    }

    /// The next draws show the graph `g`: the same records, drawn another way.
    pub fn set_stain(&self, g: StainGraph) {
        *self.stain.lock().expect("the stain lock") = g;
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

    /// What drawing the stain reported: the last draw's errors, and the note on the records.
    pub fn reported(&self) -> stain::Drawn {
        stain::Drawn {
            problems: self.problems.lock().expect("the problems lock").clone(),
            notes: self.payload.note.iter().cloned().collect(),
        }
    }

    /// How many pipelines the views have compiled or tried to.
    pub fn compiles(&self) -> u64 {
        let mut views = self.views.lock().expect("the views lock");
        let built = views.iter_mut().flat_map(|v| v.views.iter_mut().flatten());
        built.map(|v| v.view.compiles()).sum()
    }

    /// Waits until every stain asked for has compiled: the tests'. A frame never waits for an edit's compile.
    pub fn wait(&self) {
        let mut views = self.views.lock().expect("the views lock");
        for v in views.iter_mut().flat_map(|v| v.views.iter_mut().flatten()) {
            v.view.wait();
        }
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
        })
    }

    /// Draws the field into `pass` on a target of `format`: placed, at [`RealCanvas::set_placement`]'s placement;
    /// plain, over the whole viewport.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat, placed: bool) {
        let mut problems = Vec::new();
        if let Err(why) = self.try_draw(pass, format, placed, &mut problems) {
            problems.push(why);
        }
        *self.problems.lock().expect("the problems lock") = problems;
    }

    /// [`RealCanvas::draw`], the pipelines' errors to `problems`, or why it draws nothing.
    fn try_draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        format: wgpu::TextureFormat,
        placed: bool,
        problems: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut views = self.views.lock().expect("the views lock");
        let views = match views.take() {
            Some(v) if v.format == format => views.insert(v),
            _ => views.insert(Views {
                format,
                views: [None, None],
            }),
        };
        let graph = self.stain.lock().expect("the stain lock").clone();
        let slot = &mut views.views[usize::from(placed)];
        let v = match slot {
            Some(v) => v,
            None => {
                let mut v = self.build(format)?;
                // A view's first draw has no last picture to keep, so it waits for its first compile.
                v.show(&graph)?;
                v.view.wait();
                slot.insert(v)
            }
        };
        v.show(&graph)?;
        v.view.cache().poll();
        v.settle();
        let placement = if placed {
            self.placement()
        } else {
            Placement::IDENTITY
        };
        v.view.set_placement(view::Placement {
            centre: placement.centre,
            half: placement.half,
        });
        v.view
            .set_background(*self.background.lock().expect("the background lock"));
        // The layer is drawn in a submit of its own, before the pass this is recorded into is submitted.
        let drawn = v.view.prepare();
        v.view.paint(pass);
        let errors = v.view.errors();
        problems.extend(errors.iter().map(ToString::to_string));
        if problems.is_empty() {
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
        self.draw(pass, format, false);
    }
}

/// The figure through the canvas trait: the placed draw, the field at [`RealCanvas::set_placement`]'s placement.
pub struct FigureCanvas(pub Arc<RealCanvas>);

impl Canvas for FigureCanvas {
    fn context(&self) -> &CanvasContext {
        self.0.context()
    }

    fn draw_figure(&self, pass: &mut wgpu::RenderPass<'_>, format: wgpu::TextureFormat) {
        self.0.draw(pass, format, true);
    }
}
