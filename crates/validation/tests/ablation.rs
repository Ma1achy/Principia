//! The step-limit ablation's guarding tests (`validation::gate::ablation`; pitfalls §8; TASK-M3-31), each on an input
//! small enough for the unoptimised test build, with its control. The full five-arm table is the gate's
//! (`cargo xtask gate step-limit-ablation --res <n>`), which asserts nothing.
//!
//! The criteria, written before the first run:
//! - **The limit moves only what it cut.** Between two arms that differ in the limit alone, every pixel whose march the
//!   limit never cut (`predictive_bound = 0` in the limit-on arm) ends bit for bit where the limit-off arm's does, on
//!   both occupants, at 16², for both pairs (none / limit only, and dtau + clamp / all three). At least one such pixel
//!   exists, or the check could not fire. A value that moves where the limit cannot act is a bug.
//! - **A limit-off arm cuts nothing:** `predictive_bound` sums to 0 over every limit-off arm.
//! - **The shipping step control leaves no non-finite pixel** on Heggie or Aarseth–Zare at 16².
//! - **The dτ switch reaches the march:** on one IC, the dtau-only and none arms (which differ in dτ mode alone) end
//!   on different bits.
//! - **The slice is `slices.toml`'s:** the chart's centre is the base `z` plus `centre_derived` along `ê_β` and `ê_α`,
//!   and its corner `half_derived` further along each.
//! - **The dense rule is more than a quarter of the window:** 3 of a 3×3 window is dense, 2 is not.

use std::path::Path;

use kernel::chart::Chart;
use validation::gate::ablation::{
    ablation, config_stability, dense_mask, ic, pixel, Census, Occupant, Raster, Slice, Switches,
    ARMS,
};
use validation::negative_control;

fn slice() -> Slice {
    config_stability(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .expect("config_stability")
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// The arms the tests compare: none, limit only, dtau + clamp (the shipping control with the limit off), all three.
const NO_LIMIT: Switches = Switches {
    limit: false,
    ..Switches::SHIPPING
};
const PAIRS: [(&str, Switches); 4] = [ARMS[0], ARMS[3], ("dtau + clamp", NO_LIMIT), ARMS[4]];

const SMALL: usize = 16;

fn runs(occupant: Occupant) -> Vec<Census> {
    ablation(occupant, &slice(), Raster::full(SMALL), &PAIRS, threads())
}

/// Every pixel the limit-on arm `on` never cut ends bit for bit where `off`'s does; at least one such pixel.
fn check_uncut_unmoved(on: &Census, off: &Census) {
    let mut uncut = 0;
    for (i, (a, b)) in on.pixels.iter().zip(&off.pixels).enumerate() {
        if let (Some(a), Some(b)) = (a, b) {
            if a.predictive_bound == 0 {
                uncut += 1;
                assert_eq!(
                    (a.bits, a.end, a.boundary),
                    (b.bits, b.end, b.boundary),
                    "pixel {i} of {:?}: the limit never cut a step in `{}`, yet it ended elsewhere than in `{}`",
                    on.occupant,
                    on.arm,
                    off.arm
                );
            }
        }
    }
    assert!(
        uncut > 0,
        "{:?} `{}`: no pixel the limit never cut, so the check cannot fire",
        on.occupant,
        on.arm
    );
}

fn check_limit_off_cuts_nothing(c: &Census) {
    assert_eq!(
        c.predictive_bound, 0,
        "a limit-off arm cut steps: {:?} `{}` cut {}",
        c.occupant, c.arm, c.predictive_bound
    );
}

fn check_no_non_finite(c: &Census) {
    assert_eq!(
        c.non_finite, 0,
        "the shipping step control left non-finite pixels: {:?} `{}` left {}",
        c.occupant, c.arm, c.non_finite
    );
}

/// The limit moves only what it cut, on both occupants at 16²; on the same runs, the limit-off arms cut nothing and the
/// shipping arm leaves no non-finite pixel (each of those checks has its own test and control below, on one pixel).
#[test]
fn the_limit_moves_only_what_it_cut() {
    for occupant in [Occupant::Heggie, Occupant::AarsethZare] {
        let r = runs(occupant);
        println!(
            "{occupant:?} at {SMALL}²:\n{}",
            validation::gate::ablation::table(&r)
        );
        check_uncut_unmoved(&r[1], &r[0]);
        check_uncut_unmoved(&r[3], &r[2]);
        check_limit_off_cuts_nothing(&r[0]);
        check_limit_off_cuts_nothing(&r[2]);
        check_no_non_finite(&r[3]);
    }
}

negative_control!(
    the_limit_moves_only_what_it_cut,
    "against an arm whose dτ mode and landing differ too, a pixel the limit never cut ends elsewhere",
    expected = "the limit never cut a step",
    {
        let r = ablation(Occupant::AarsethZare, &slice(), Raster::full(SMALL), &[ARMS[0], ARMS[4]], threads());
        check_uncut_unmoved(&r[1], &r[0]);
    }
);

/// The pixel the 64² census measured non-finite under Aarseth–Zare with every switch off: `(50, 34)` of 64².
const NON_FINITE_PIXEL: Raster = Raster {
    n: 64,
    x0: 50,
    y0: 34,
    side: 1,
};

/// The pixel's census under `arms`, on Heggie and on Aarseth–Zare.
fn on_the_pixel(arms: &[(&'static str, Switches)]) -> Vec<Census> {
    [Occupant::Heggie, Occupant::AarsethZare]
        .into_iter()
        .flat_map(|o| ablation(o, &slice(), NON_FINITE_PIXEL, arms, 1))
        .collect()
}

/// The two limit-off arms cut nothing on the pixel, on either occupant.
#[test]
fn the_limit_off_arms_cut_nothing() {
    for c in on_the_pixel(&[ARMS[0], ("dtau + clamp", NO_LIMIT)]) {
        check_limit_off_cuts_nothing(&c);
    }
}

negative_control!(
    the_limit_off_arms_cut_nothing,
    "the limit-only arm read as a limit-off one",
    expected = "a limit-off arm cut steps",
    check_limit_off_cuts_nothing(&on_the_pixel(&[ARMS[3]])[1])
);

/// The shipping step control leaves the pixel that Aarseth–Zare's unfixed arm leaves non-finite finite, on either
/// occupant. The control is that unfixed arm, so it also holds the input: were the pixel to stop going non-finite
/// unfixed, the control would stop failing the check.
#[test]
fn the_shipping_control_leaves_the_pixel_finite() {
    for c in on_the_pixel(&[ARMS[4]]) {
        check_no_non_finite(&c);
    }
}

negative_control!(
    the_shipping_control_leaves_the_pixel_finite,
    "Aarseth–Zare with every switch off read as the shipping step control",
    expected = "the shipping step control left non-finite pixels",
    check_no_non_finite(&on_the_pixel(&[ARMS[0]])[1])
);

/// `a` and `b`, the arms `none` and `other` on the IC at `(50, 34)` of 64², end on different bits.
fn check_switch_reaches(occupant: Occupant, other: Switches) {
    let s = slice();
    let ic = ic(&s, NON_FINITE_PIXEL.uv(0)).expect("a finite IC");
    let k = validation::gate::ablation::k_target(&s);
    let a = pixel(occupant, ARMS[0].1, &ic, k);
    let b = pixel(occupant, other, &ic, k);
    assert_ne!(
        a.bits, b.bits,
        "{occupant:?}: the switched arm ended on the same bits as `none`"
    );
}

#[test]
fn the_dtau_switch_reaches_the_march() {
    check_switch_reaches(Occupant::Heggie, ARMS[1].1);
    check_switch_reaches(Occupant::AarsethZare, ARMS[1].1);
}

negative_control!(
    the_dtau_switch_reaches_the_march,
    "`none` against itself",
    expected = "ended on the same bits",
    check_switch_reaches(Occupant::AarsethZare, ARMS[0].1)
);

/// `slices.toml`'s `config_stability`: `z_glsl`, and the derived centre and half-width along `ê_β`, `ê_α`.
const Z_GLSL: [f64; 10] = [
    -0.098, 0.11, -0.034, 0.067, -0.093, -0.066, 0.027, -0.114, 0.107, -0.116,
];
const CENTRE: [f64; 2] = [-0.01213, -0.00783];
const HALF: f64 = 0.63763;

fn check_slice(s: &Slice) {
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    let at = |u: f64, v: f64| s.chart.map::<f64>(u, v).z.z;
    let c = at(0.5, 0.5);
    let corner = at(1.0, 1.0);
    let expected = [
        Z_GLSL[1] + CENTRE[1],
        Z_GLSL[0] + CENTRE[0],
        Z_GLSL[4],
        Z_GLSL[5],
        Z_GLSL[6],
        Z_GLSL[7],
        Z_GLSL[8],
        Z_GLSL[9],
    ];
    for i in 0..8 {
        assert!(
            close(c[i], expected[i]),
            "the slice's centre z[{i}] = {} is not {}",
            c[i],
            expected[i]
        );
    }
    assert!(
        close(corner[1], expected[1] + HALF),
        "the slice's u = 1 edge is not half_derived along z_β"
    );
    assert!(
        close(corner[0], expected[0] + HALF),
        "the slice's v = 1 edge is not half_derived along z_α"
    );
}

#[test]
fn the_slice_is_config_stability() {
    check_slice(&slice());
}

negative_control!(
    the_slice_is_config_stability,
    "the slice with its basis transposed, the `shape_pl` mistake",
    expected = "the slice's",
    {
        let mut s = slice();
        std::mem::swap(&mut s.chart.q1, &mut s.chart.q2);
        let mut z0 = s.chart.z0;
        z0.swap(0, 1);
        s.chart.z0 = z0;
        check_slice(&s)
    }
);

/// A 3×3 window, its centre and `k − 1` more of its pixels in the mask, judged at half-width 1.
fn check_dense(k: usize, dense: bool) {
    let mut mask = vec![false; 9];
    mask[4] = true;
    for m in mask.iter_mut().filter(|m| !**m).take(k - 1) {
        *m = true;
    }
    assert_eq!(
        dense_mask(&mask, 3, 1)[4],
        dense,
        "{k} of 9 judged dense = {}",
        !dense
    );
}

#[test]
fn the_dense_rule_is_more_than_a_quarter() {
    check_dense(3, true);
    check_dense(2, false);
}

negative_control!(
    the_dense_rule_is_more_than_a_quarter,
    "2 of 9 expected dense",
    expected = "2 of 9 judged dense",
    check_dense(2, true)
);
