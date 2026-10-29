//! What gui's tests share: the app on the mock or on the real engine's data contract, run headless, and the
//! accessible names of a frame. gui's tests have no route to `negative_control!` (R-187: gui takes no dependency on
//! validation, and `cargo xtask controls` skips it), so each check is also run, in its test, on an input it must
//! reject, through [`rejects`].

use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};

use crate::app::App;
use crate::capture::{PIXELS_PER_POINT, SIZE};
use crate::headless::{Headless, Name};
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide, RealSide};

/// The mock's slice frame, the defaults on its chart `z_α × z_β`: the normal and both targets `z_q3` (gui_state_contract
/// §2).
pub const SLICE_FRAME: SliceFrame = SliceFrame::along(Z_Q3);

/// The target format the tests' apps draw on.
pub const FORMAT: wgpu::TextureFormat = crate::capture::FORMAT;

/// The app on a mock with a running clock and no canvas.
pub fn mock_app() -> App<MockSide> {
    App::new(MockSide::new(MockEngine::new(), None), FORMAT)
}

/// The skeleton state with the playhead at `t`.
pub fn state(t: f64) -> (SimConfig, RenderState) {
    let sim = SimConfig {
        chart: Chart {
            window: Window::SQUARE,
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice { frame: SLICE_FRAME },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 0.0 },
        collision: Collision {},
        quality: Quality {},
    };
    let render = RenderState {
        stain_graph: StainGraph::new(),
        overlays: Overlays::default(),
        palette: Palette {},
        playhead: Playhead { t },
        hover: Hover::default(),
        debug_view: false,
    };
    (sim, render)
}

/// The app on the real engine's data contract, over the skeleton state with `t = 0.0`, and no figure.
pub fn real_app() -> App<RealSide> {
    let (sim, render) = state(0.0);
    App::new(RealSide::new(sim, render), FORMAT)
}

/// A headless context at the capture's size and scale.
pub fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

/// The names of one more frame of `app` on `headless`.
pub fn names<S: EngineSide>(headless: &mut Headless, app: &mut App<S>) -> Vec<Name> {
    let output = headless.frame(app, Vec::new());
    headless.names(&output)
}

/// Just the names' text.
pub fn texts(names: &[Name]) -> Vec<String> {
    names.iter().map(|n| n.name.clone()).collect()
}

/// The text of the names of one more frame.
pub fn frame_texts<S: EngineSide>(headless: &mut Headless, app: &mut App<S>) -> Vec<String> {
    texts(&names(headless, app))
}

/// A playhead edit.
pub fn playhead(t: f64, no_history: bool) -> SetField {
    SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t })),
        no_history,
    }
}

/// Clicks the control named `name`: three frames, at its rect's centre.
pub fn click<S: EngineSide>(headless: &mut Headless, app: &mut App<S>, name: &str) {
    let names = names(headless, app);
    let rect = names
        .iter()
        .find(|n| n.name == name)
        .and_then(|n| n.rect)
        .unwrap_or_else(|| panic!("no `{name}` to click in {:?}", texts(&names)));
    let scale = f64::from(headless.pixels_per_point());
    let centre = eframe::egui::pos2(
        ((rect[0] + rect[2]) / 2.0 / scale) as f32,
        ((rect[1] + rect[3]) / 2.0 / scale) as f32,
    );
    for events in crate::capture::click(centre) {
        let _ = headless.frame(app, events);
    }
}

/// Clicks scope `id` where it was last placed: three frames, at its rect's centre.
pub fn click_scope<S: EngineSide>(headless: &mut Headless, app: &mut App<S>, id: &str) {
    let rect = app
        .keyboard
        .place_of(id)
        .unwrap_or_else(|| panic!("no `{id}` placed to click"))
        .rect;
    for events in crate::capture::click(rect.center()) {
        let _ = headless.frame(app, events);
    }
}

/// Asserts that `check` panics: the check can fail on the input it was given (R-176's control, run in the test).
pub fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// Presses and releases `key` with `modifiers`: two frames.
pub fn press<S: EngineSide>(
    headless: &mut Headless,
    app: &mut App<S>,
    key: eframe::egui::Key,
    modifiers: eframe::egui::Modifiers,
) {
    for events in crate::capture::key(key, modifiers) {
        let _ = headless.frame(app, events);
    }
}

/// `ViewUI`'s focus path, as `&str`s.
pub fn focus<S: EngineSide>(app: &App<S>) -> Vec<&str> {
    app.view.focus.path.iter().map(String::as_str).collect()
}

/// One AccessKit node of a frame: its role, its bounds in points, its numeric value and its text.
#[derive(Clone, Debug)]
pub struct Node {
    /// Its role.
    pub role: eframe::egui::accesskit::Role,
    /// Its bounds, in points.
    pub rect: eframe::egui::Rect,
    /// Its numeric value: a slider's or a drag value's.
    pub value: Option<f64>,
    /// Its label or value text.
    pub text: String,
}

/// The nodes with bounds of one more frame of `app`.
pub fn nodes<S: EngineSide>(headless: &mut Headless, app: &mut App<S>) -> Vec<Node> {
    let output = headless.frame(app, Vec::new());
    let Some(update) = &output.platform_output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let b = node.bounds()?;
            Some(Node {
                role: node.role(),
                rect: eframe::egui::Rect::from_min_max(
                    eframe::egui::pos2(b.x0 as f32, b.y0 as f32),
                    eframe::egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
                value: node.numeric_value(),
                text: node.label().or(node.value()).unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

/// The shapes of one more frame of `app`, flattened, in paint order.
pub fn shapes<S: EngineSide>(
    headless: &mut Headless,
    app: &mut App<S>,
) -> Vec<eframe::egui::Shape> {
    let output = headless.frame(app, Vec::new());
    let mut flat = Vec::new();
    fn flatten(shape: &eframe::egui::Shape, out: &mut Vec<eframe::egui::Shape>) {
        match shape {
            eframe::egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
            other => out.push(other.clone()),
        }
    }
    for clipped in &output.shapes {
        flatten(&clipped.shape, &mut flat);
    }
    flat
}

/// A key no command binds: the tests press it to turn keyboard navigation on, as any key press does (R-460, R-464 M2),
/// with nothing else happening.
pub const UNBOUND: eframe::egui::Key = eframe::egui::Key::F12;

/// Turns keyboard navigation on, the ring shown where the focus is, by a key press that does nothing else
/// ([`UNBOUND`]): for a test that put the focus somewhere itself, as no user can, so the next Enter acts as from the
/// keyboard rather than only showing the ring (R-460 H1, R-464 M2).
pub fn keys_on<S: EngineSide>(headless: &mut Headless, app: &mut App<S>) {
    press(headless, app, UNBOUND, eframe::egui::Modifiers::NONE);
}
