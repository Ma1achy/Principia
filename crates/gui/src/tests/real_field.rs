//! The real field in the window: the app on the real engine shows the field `engine::field` computes. A fixed 16 × 16
//! frame of the start chart has the stored outcome histogram, and the mock's field swapped in for it does not; the
//! real side's canvas draws each sample's swatch where its field point lies in the figure, the bottom row at the
//! bottom. Each check also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use engine::contract::legend::SWATCHES;
use engine::field::{Cpu, Field, Outcome};

use super::support::rejects;
use crate::app::App;
use crate::capture::{render_frame_on, FORMAT, PIXELS_PER_POINT, SIZE};
use crate::headless::Headless;
use crate::mock::field::MockField;
use crate::real::{compute, start, RealCanvas};
use crate::side::RealSide;

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

/// The start state's horizon is the kernel's default.
#[test]
fn the_start_horizon_is_the_default() {
    let (sim, _) = start();
    let default = engine::contract::sim_uniforms::SimUniforms::default().t_horizon;
    assert_eq!(sim.horizon.t, default);
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
