//! `engine::field`'s CPU backend: the march's labels on ICs whose ends are known (a head-on collision, the
//! figure-eight bounded) and the image's row order. A frame's histogram is gui's test (`real_field`).
//!
//! `the_start_view_before_the_periodic_link_ends_the_same`: the app's start view, the chart "z_α × z_β", runs along
//! `z_β`, so under the periodic `β` link it is a redefined plane and its frame is a different picture. This holds the
//! picture it showed before: each of the old view's 16 × 16 samples, its `z_β` converted to the periodic link point
//! by point, decodes to the starting state the old link gave, bit for bit, and the frame of those samples has the
//! histogram main stores for that view (253 a triple ejection, 3 a collision of pair 1). So no sample's outcome class moved; what
//! the app's first frame shows differently is the redefined plane alone.

use engine::contract::sim_config::Window;
use engine::field::{boundaries, heggie_params, outcome, Field, Outcome};
use engine::first_slice::{narrow, uniforms, Quad};
use kernel::chart::{Chart, Latent};
use kernel::decode::retired::{converted, decode_retired};
use kernel::decode::{decode, Ic};
use kernel::detect::collision::CollisionRadius;
#[cfg(feature = "controls")]
use kernel::generated::links::TanhBeta;
use kernel::generated::links::{Defaults, Link, RetiredBeta, SigmoidBeta};
use kernel::payload::{STATE_BOUNDED, STATE_COLLISION, STATE_ESCAPE};
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

/// A field of one sample, `o`: it must have no colour, transparent, for the canvas to draw its background, and be in
/// no palette class, while the histogram keeps it under its own `(state, detail)`.
fn check_no_colour(o: Outcome) {
    let f = Field {
        size: [1, 1],
        outcomes: vec![o],
    };
    assert_eq!(f.histogram(), vec![(o, 1)]);
    assert_eq!(f.rgba8(), vec![0; 4], "the sample has a colour");
    assert_eq!(
        f.classes().iter().sum::<usize>(),
        0,
        "the sample is in a palette class"
    );
}

#[test]
fn a_decode_failure_is_transparent() {
    check_no_colour(Outcome {
        state: kernel::payload::STATE_DECODE_FAILED,
        detail: 1,
    });
}

negative_control!(
    a_decode_failure_is_transparent,
    "a bounded sample, which the palette draws in its black class",
    expected = "the sample has a colour",
    check_no_colour(BOUNDED)
);

/// The start view before the periodic link: "z_α × z_β" at the base point whose `z_β` was 0.410 under
/// `sigmoid_beta`, `q₁ = e_α`, `q₂ = e_β`, as the field narrows a plane to f32.
fn old_start_view() -> Latent<f32> {
    let e = |k: usize| {
        let mut q = [0.0f32; 8];
        q[k] = 1.0;
        q
    };
    Latent {
        z0: [0.180, 0.410, 0.0, -0.227, 0.0, 0.312, 0.333, 0.333].map(|x: f64| x as f32),
        q1: e(0),
        q2: e(1),
    }
}

/// The 16 × 16 frame of the old start view through the new decoder, each sample's `z_β` converted from the retired
/// link `L` point by point: asserts each sample decodes bit for bit to the state `sigmoid_beta` gave it, then that the
/// frame's histogram is the one stored before the change. Returns the samples.
fn check_old_start_view<L: Link<1, 1, Codomain = RetiredBeta>>() -> usize {
    const STORED: [(u32, u32, usize); 2] = [(STATE_ESCAPE, 3, 253), (STATE_COLLISION, 1, 3)];
    let quad = Quad {
        chart: old_start_view(),
        window: Window::SQUARE,
        size: [16, 16],
    };
    let params = heggie_params();
    let k_target = boundaries(50.0, &params);
    let outcomes: Vec<Outcome> = (0..quad.len())
        .map(|i| {
            let [u, v] = quad.uv(i);
            let z = quad.chart.map::<f64>(u, v).z;
            let old = decode_retired::<Defaults, SigmoidBeta, f64>(&z);
            let new = decode::<Defaults, f64>(&converted::<L, f64>(&z));
            let bits = |ic: &Ic<f64>| {
                let mut w = ic.m.to_vec();
                w.extend(ic.r.iter().flatten());
                w.extend(ic.p.iter().flatten());
                w.iter().map(|x| x.to_bits()).collect::<Vec<u64>>()
            };
            assert!(
                bits(&old.ic) == bits(&new.ic),
                "sample {i} at ({u}, {v}) does not decode bit for bit to the state the old link gave"
            );
            outcome(&narrow(&new.ic), new.degenerate, radius(), k_target, &params)
        })
        .collect();
    let n = outcomes.len();
    let field = Field {
        size: quad.size,
        outcomes,
    };
    let got: Vec<(u32, u32, usize)> = field
        .histogram()
        .into_iter()
        .map(|(o, n)| (o.state, o.detail, n))
        .collect();
    assert_eq!(
        got, STORED,
        "the old start view's frame is not the histogram stored before the change"
    );
    n
}

#[test]
fn the_start_view_before_the_periodic_link_ends_the_same() {
    let n = check_old_start_view::<SigmoidBeta>();
    eprintln!(
        "the_start_view_before_the_periodic_link_ends_the_same: {n} of {n} samples decode bit for bit; the frame's \
         histogram is the stored 253 a triple ejection, 3 a collision of pair 1"
    );
}

negative_control!(
    the_start_view_before_the_periodic_link_ends_the_same,
    "the samples converted from tanh_beta, not the link the view was made under, must fail",
    expected = "does not decode bit for bit to the state the old link gave",
    {
        check_old_start_view::<TanhBeta>();
    }
);
