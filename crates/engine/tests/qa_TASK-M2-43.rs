//! QA tests for TASK-M2-43 on the authoring side, written from REQ-CHART-014's verify as R-544 amends it and from
//! chart_decoder_contract Part 5's `system_image`, not from the implementation.
//!
//! - `qa_m243_chart_descriptors_spherical_bijective` (REQ-CHART-014, R-544): the shape sphere chart under the spherical
//!   map is the whole sphere, each pixel its own system, so its `system_image` is `Bijective`, whatever its held masses,
//!   momenta, `I` and `φ_f`; the descriptor validates. Its control reads the map as R-141's `NToOne(2)`, which must
//!   fail. The exponential map's value is TASK-M2-28's (REQ-CHART-075) and is not asserted here.
//!
//! Each test has its negative control (R-176).

use engine::chart_authoring::{validate, Authored};
use kernel::chart::{ShapeMap, ShapeSphere, SystemImage};
use validation::decode_f32_eps::SplitMix;
use validation::gauge::uniform;
use validation::negative_control;

/// A spherical shape-sphere chart with random held values and gauge.
fn spherical(rng: &mut SplitMix) -> ShapeSphere<f64> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.0,
        inertia: uniform(rng, 0.25, 4.0),
        phase: uniform(rng, -3.0, 3.0),
        z_mass: [uniform(rng, -3.0, 3.0), uniform(rng, -3.0, 3.0)],
        z_mom: [0, 1, 2, 3].map(|_| uniform(rng, -2.0, 2.0)),
    }
}

/// Every drawn spherical chart reports `want` and validates.
fn check_spherical(want: SystemImage) {
    let mut rng = SplitMix::new(0x4387);
    for _ in 0..32 {
        let c = spherical(&mut rng);
        let image = c.descriptor().system_image;
        assert_eq!(
            image, want,
            "the spherical map's system_image is {image:?}, not {want:?}"
        );
        if let Err(r) = validate(c, None) {
            panic!("the spherical shape-sphere chart is refused: {r}");
        }
    }
}

#[test]
fn qa_m243_chart_descriptors_spherical_bijective() {
    check_spherical(SystemImage::Bijective);
}

negative_control!(
    qa_m243_chart_descriptors_spherical_bijective,
    "the spherical map read as R-141's n-to-1 with n = 2 must fail",
    expected = "system_image",
    check_spherical(SystemImage::NToOne(2))
);
