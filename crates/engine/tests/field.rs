//! `engine::field`'s CPU backend: the march's labels on ICs whose ends are known (a head-on collision, the
//! figure-eight bounded) and the image's row order. A frame's histogram is gui's test (`real_field`).

use engine::field::{heggie_params, outcome, Field, Outcome};
use engine::first_slice::uniforms;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::payload::{STATE_BOUNDED, STATE_COLLISION};
use render::colour::outcome::SWATCHES;
use validation::negative_control;

/// `state` at f32, as the field's records hold an IC.
fn ic32(m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> Ic<f32> {
    Ic {
        m: m.map(|x| x as f32),
        r: r.map(|v| v.map(|x| x as f32)),
        p: p.map(|v| v.map(|x| x as f32)),
    }
}

/// The default collision threshold, `r_coll = 1e-3`, as the field reads it from the sim key.
fn radius() -> CollisionRadius {
    uniforms(50.0).expect("the default horizon").radius
}

/// The radial collision passage (validation's `radial`): bodies 0 and 1 fall head-on from rest and collide near
/// `t = 1.36`; marched to `t = 2` under `radius`, it must end `collision` of pair 2, the pair opposite body 2.
fn check_radial_collides(radius: CollisionRadius) {
    let f = validation::az::radial();
    let ic = ic32(f.state.m, f.state.r, f.state.p);
    let got = outcome(&ic, None, radius, f.boundaries, &heggie_params());
    assert_eq!(
        got,
        Outcome {
            state: STATE_COLLISION,
            detail: 2,
        },
        "the radial passage does not end in pair 2's collision"
    );
}

#[test]
fn radial_passage_is_a_collision() {
    check_radial_collides(radius());
}

negative_control!(
    radial_passage_is_a_collision,
    "with no collision threshold the passage runs through its collision to the horizon",
    expected = "does not end in pair 2's collision",
    check_radial_collides(CollisionRadius::none())
);

/// The figure-eight (validation's `figure_eight`), a bounded periodic orbit, marched to the default horizon with
/// `params`: it must end `bounded`.
fn check_figure_eight_bounded(params: engine::field::HgParams32) {
    let f = validation::drift::figure_eight();
    let ic = ic32(f.ic.m, f.ic.r, f.ic.p);
    let k = engine::field::boundaries(50.0, &params);
    let got = outcome(&ic, None, radius(), k, &params);
    assert_eq!(
        got,
        Outcome {
            state: STATE_BOUNDED,
            detail: 0,
        },
        "the figure-eight does not end bounded at the horizon"
    );
}

#[test]
fn figure_eight_is_bounded() {
    check_figure_eight_bounded(heggie_params());
}

negative_control!(
    figure_eight_is_bounded,
    "a step budget of ten steps leaves the march undetermined, `running`",
    expected = "does not end bounded",
    check_figure_eight_bounded(engine::field::HgParams32 {
        step_budget_rate: 0.0,
        n_min: 10,
        ..heggie_params()
    })
);

/// A 1 × 2 field, its bottom sample collision 1–2 (pair 2) and its top bounded: the image's first row must be the top
/// sample's swatch, the texture's rows running from the top.
fn check_image_rows(field: &Field) {
    let rgba = field.rgba8();
    let [r, g, b] = SWATCHES[0].srgb8;
    assert_eq!(
        &rgba[..4],
        &[r, g, b, 255],
        "the image's first row is not the field's top row"
    );
    let [r, g, b] = SWATCHES[1].srgb8;
    assert_eq!(
        &rgba[4..],
        &[r, g, b, 255],
        "the image's last row is not the field's bottom row"
    );
}

fn two_rows(bottom: Outcome, top: Outcome) -> Field {
    Field {
        size: [1, 2],
        outcomes: vec![bottom, top],
    }
}

const PAIR_2: Outcome = Outcome {
    state: STATE_COLLISION,
    detail: 2,
};
const BOUNDED: Outcome = Outcome {
    state: STATE_BOUNDED,
    detail: 0,
};

#[test]
fn image_rows_run_from_the_top() {
    check_image_rows(&two_rows(PAIR_2, BOUNDED));
}

negative_control!(
    image_rows_run_from_the_top,
    "the field's rows swapped, as an image written bottom-up would hold them",
    expected = "is not the field's top row",
    check_image_rows(&two_rows(BOUNDED, PAIR_2))
);

/// A decode failure has no colour: transparent, for the canvas to draw its background.
#[test]
fn a_decode_failure_is_transparent() {
    let failed = Outcome {
        state: kernel::payload::STATE_DECODE_FAILED,
        detail: 1,
    };
    let f = Field {
        size: [1, 1],
        outcomes: vec![failed],
    };
    assert_eq!(f.rgba8(), vec![0; 4]);
    assert_eq!(f.classes().iter().sum::<usize>(), 0);
    assert_eq!(f.histogram(), vec![(failed, 1)]);
}
