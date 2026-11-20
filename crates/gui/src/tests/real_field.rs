//! The real field in the window: the app on the real engine shows the field `engine::field` computes. A fixed 16 × 16
//! frame of the start chart has the stored outcome histogram, and the mock's field swapped in for it does not; the
//! real side's canvas draws each sample's swatch where its field point lies in the figure, the bottom row at the
//! bottom. The capture shoots the origin slice of z_α × z_β, its field computed from that state. Each check also runs
//! on an input it must reject (through `rejects`).

use std::sync::{Arc, Mutex};

use engine::compute::ComputeError;
use engine::contract::interface::EngineInterface;
use engine::contract::legend::SWATCHES;
use engine::contract::sim_config::{SimConfig, Window};
use engine::field::{Cpu, Field, FieldBackend, Outcome};

use super::support::rejects;
use crate::app::App;
use crate::capture::{render_frame_on, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::cli::real::open_capture;
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{View, AXES};
use crate::headless::Headless;
use crate::mock::field::MockField;
use crate::real::{capture_view, compute, start, RealCanvas};
use crate::side::{EngineSide, RealSide};

/// The fixed frame: the start chart over its window, 16 × 16 samples.
const FRAME: [usize; 2] = [16, 16];

/// The `state` codes (payload §2).
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;

/// The fixed frame's histogram, `(state, detail, samples)`, stored from a run checked by hand (the PR has the run):
/// 253 samples bounded at the horizon and 3 a collision of pair 1 (bodies 1 and 3), no failure of any kind. No escape
/// is labelled until the escape detector is built, so a sample that does not collide ends bounded.
const STORED: &[(u32, u32, usize)] = &[(BOUNDED, 0, 253), (COLLISION, 1, 3)];

/// Asserts that `field`'s histogram is the stored one.
fn check_histogram(field: &Field) {
    let got: Vec<(u32, u32, usize)> = field
        .histogram()
        .into_iter()
        .map(|(o, n)| (o.state, o.detail, n))
        .collect();
    assert_eq!(got, STORED, "the frame's histogram is not the stored one");
}

#[test]
fn the_real_frame_has_the_stored_histogram() {
    let (sim, _) = start();
    let field = compute(&Cpu, &sim, FRAME).expect("the real field");
    let map: String = (0..FRAME[1])
        .rev()
        .map(|y| {
            (0..FRAME[0])
                .map(|x| match field.at(x, y) {
                    Outcome { state: BOUNDED, .. } => '.',
                    Outcome {
                        state: COLLISION,
                        detail,
                    } => char::from(b'0' + detail as u8),
                    _ => '?',
                })
                .collect::<String>()
                + "\n"
        })
        .collect();
    eprintln!("the frame, top row first ('.' bounded, a digit a collision's pair):\n{map}");
    check_histogram(&field);
    let mock = compute(&MockField, &sim, FRAME).expect("the mock's field");
    rejects("the mock's field", || check_histogram(&mock));
}

/// Asserts that the horizon `t` is the kernel's default.
fn check_default_horizon(t: f64) {
    let default = engine::contract::sim_uniforms::SimUniforms::default().t_horizon;
    assert_eq!(t, default, "the horizon is not the default");
}

/// The start state's horizon is the kernel's default.
#[test]
fn the_start_horizon_is_the_default() {
    let (sim, _) = start();
    check_default_horizon(sim.horizon.t);
    rejects("half the start horizon", || {
        check_default_horizon(sim.horizon.t / 2.0)
    });
}

/// A 2 × 2 field, rows from the bottom: collision 1–2 then bounded; bounded then the triple collision.
fn quadrants() -> Field {
    let o = |state, detail| Outcome { state, detail };
    Field {
        size: [2, 2],
        outcomes: vec![
            o(COLLISION, 2),
            o(BOUNDED, 0),
            o(BOUNDED, 0),
            o(COLLISION, 3),
        ],
    }
}

/// The app on the real side drawing `field`, one frame rendered: the pixels, and the canvas, for its placement.
fn drawn(field: &Field) -> (Vec<u8>, Arc<RealCanvas>) {
    let canvas = Arc::new(RealCanvas::open(field).expect("a GPU adapter for the real canvas"));
    let (sim, render) = start();
    let mut app = App::new(RealSide::with_canvas(sim, render, canvas.clone()), FORMAT);
    let mut headless = Headless::new(SIZE, PIXELS_PER_POINT);
    let _ = headless.frame(&mut app, Vec::new());
    let shot = render_frame_on(canvas.as_ref(), &mut app, &mut headless);
    (shot.rgba, canvas)
}

/// Asserts that the pixels `rgba`, drawn at `canvas`'s placement, show [`quadrants`]: at each field point `(s, t)`,
/// `t` up, the swatch of the sample there.
fn check_quadrants(rgba: &[u8], canvas: &RealCanvas) {
    let p = canvas.placement();
    let [w, h] = SIZE.map(|n| n as f32);
    for (s, t, k) in [(0.4, 0.4, 1), (0.6, 0.4, 0), (0.4, 0.6, 0), (0.6, 0.6, 4)] {
        let x = (p.centre[0] + (2.0 * s - 1.0) * p.half[0] + 1.0) / 2.0 * w;
        let y = (1.0 - (p.centre[1] + (2.0 * t - 1.0) * p.half[1])) / 2.0 * h;
        let i = (y as usize * SIZE[0] as usize + x as usize) * 4;
        assert_eq!(
            rgba[i..i + 3],
            SWATCHES[k].srgb8,
            "the pixel at field point ({s}, {t}) is not its sample's swatch"
        );
    }
}

#[test]
fn the_real_canvas_draws_each_sample_where_it_lies() {
    let (rgba, canvas) = drawn(&quadrants());
    check_quadrants(&rgba, &canvas);
    let mut flipped = quadrants();
    flipped.outcomes.rotate_left(2);
    let (rgba, canvas) = drawn(&flipped);
    rejects("the field drawn upside down", || {
        check_quadrants(&rgba, &canvas)
    });
}

/// A backend that integrates nothing: it records each sim key it is asked a field of, and answers all bounded.
#[derive(Default)]
struct Recording(Mutex<Vec<SimConfig>>);

impl FieldBackend for Recording {
    fn field(
        &self,
        sim: &SimConfig,
        _window: Window,
        size: [usize; 2],
    ) -> Result<Field, ComputeError> {
        self.0.lock().expect("the record's lock").push(sim.clone());
        let bounded = Outcome {
            state: BOUNDED,
            detail: 0,
        };
        Ok(Field {
            size,
            outcomes: vec![bounded; size[0] * size[1]],
        })
    }
}

/// Asserts that `sim` is the origin slice of z_α × z_β: the chart on those two axes at scale 1 over its default
/// window, `τ₁ = τ₂ = γ = 0`, and every component of `z₀` 0.
fn check_origin_slice(sim: &SimConfig) {
    let view = View::read(sim);
    assert_eq!(
        (AXES[view.a], AXES[view.b]),
        ("z_α", "z_β"),
        "not the chart z_α × z_β"
    );
    assert_eq!(
        (view.tau1, view.tau2, view.gamma),
        (0.0, 0.0, 0.0),
        "the plane is tilted or turned"
    );
    assert_eq!(view.scale, 1.0, "the plane is zoomed");
    assert_eq!(sim.chart.window, Window::SQUARE, "not the default window");
    assert_eq!(sim.plane.z0, [0.0; 8], "a slice offset is not 0");
}

/// Asserts that the capture's one field was computed from `shown`, the state its app shows, and that `shown` is the
/// origin slice.
fn check_capture(computed: &[SimConfig], shown: &SimConfig) {
    assert_eq!(
        computed,
        std::slice::from_ref(shown),
        "the field was not computed, once, from the state the app shows"
    );
    check_origin_slice(shown);
}

/// The capture's side holds the origin slice of z_α × z_β and its field is computed from that state.
#[test]
fn the_capture_shoots_the_origin_slice() {
    let backend = Recording::default();
    let (mut side, _canvas) = open_capture(&backend, [1, 1]).expect("the capture's side");
    let computed = backend.0.lock().expect("the record's lock").clone();
    let shown = side.engine().snapshot().sim;
    check_capture(&computed, &shown);
    assert_eq!(shown, capture_view().0);
    // The controls: a capture reading the start state, for its field, for its panels, and for both; no field; a tilt.
    let (opening, _) = start();
    rejects("a field computed from the start state", || {
        check_capture(std::slice::from_ref(&opening), &shown)
    });
    rejects("an app showing the start state", || {
        check_capture(&computed, &opening)
    });
    rejects("the start state throughout", || {
        check_capture(std::slice::from_ref(&opening), &opening)
    });
    rejects("no field computed", || check_capture(&[], &shown));
    let mut tilted = shown.clone();
    let angles = Angles {
        tau1: 30.0,
        tau2: 0.0,
        gamma: 0.0,
    };
    (tilted.plane.q1, tilted.plane.q2) = angles.apply(View::read(&shown)).basis();
    rejects("a tilted plane", || check_origin_slice(&tilted));
}
