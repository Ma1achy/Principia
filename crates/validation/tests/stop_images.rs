//! The stop images' checks (`validation::stop_images`), each with a control: the seam check fires on a synthetic
//! patchwork (a smooth field frozen at the `t_end` strata of a smooth termination time) and not on the smooth field,
//! and resolves a moderate rate either side of its hard edge; the strata check refuses ~32 `t_end` values; the
//! one-variable check holds on a small render of the app's zero view, and each of its three legs names the pixel that
//! breaks it; and (ignored, for its time) the energy-floor control on the zero view fires the seam check and fails the
//! tolerance.

use validation::negative_control;
use validation::stop_images::{
    check_one_variable, control_seams, histogram, measure, render, seams, strata, theta_colour,
    tolerance_failures, Arm, Image, Pixel, Rule, View, TOLERANCE,
};

/// The side of the synthetic fields.
const N: usize = 64;
/// The synthetic patchwork's strata: rings a few pixels wide at this side (the patchwork's ~32 strata were seen on far
/// larger images).
const LEVELS: u32 = 8;
/// The rate the synthetic live field turns at, radians per time unit: a stratum of 50/8 time units freezes `θ̃` about
/// 3 radians apart from its neighbour.
const OMEGA: f64 = 0.5;

/// A synthetic view: `θ̃` smooth over the square (the live image); `t_end` a smooth radial termination time on
/// `levels` strata over the horizon 50; the stop image `θ̃` frozen at `t_end`, `θ̃(t) = θ̃ + ω·t` with `ω` the rate
/// the live field turns at. `levels` = 0 is the smooth control: every pixel stops at the horizon, so the stop image is
/// the live one.
fn synthetic(levels: u32, omega: f64) -> (Image, Image, Vec<bool>, Vec<u32>) {
    let mut live = Vec::new();
    let mut stopped = Vec::new();
    let mut t_end = Vec::new();
    for y in 0..N {
        for x in 0..N {
            let (u, v) = (x as f64 / N as f64, y as f64 / N as f64);
            let base = 1.5 * u + 0.75 * v;
            let r = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() / 0.71;
            let t = if levels == 0 {
                50.0
            } else {
                (r * f64::from(levels)).floor() / f64::from(levels) * 50.0
            };
            let colour = |theta: f64| {
                theta_colour(&engine::precision::SimState64 {
                    theta,
                    packed_a: kernel::payload::set_state(0, kernel::payload::STATE_ESCAPE),
                    ..Default::default()
                })
            };
            live.push(colour(base + omega * 50.0));
            stopped.push(colour(base + omega * t));
            t_end.push((t * 1000.0) as u32);
        }
    }
    (stopped, live, vec![true; N * N], t_end)
}

/// The seam check on the synthetic view with `levels` strata must fire.
fn check_fires(levels: u32) {
    let (stopped, live, escaped, t_end) = synthetic(levels, OMEGA);
    let s = seams(N, &stopped, &live, &escaped, &t_end);
    println!("{levels} strata: {s:?}");
    assert!(
        s.fires(),
        "the seam check did not fire on the synthetic patchwork: {s:?}"
    );
}

#[test]
fn stop_images_seams_fire_on_a_patchwork() {
    check_fires(LEVELS);
}

negative_control!(
    stop_images_seams_fire_on_a_patchwork,
    "the smooth field, with no strata, gives the seam check nothing to fire on",
    expected = "the seam check did not fire",
    check_fires(0)
);

/// The seam check on the synthetic view with `levels` strata must not fire.
fn check_quiet(levels: u32) {
    let (stopped, live, escaped, t_end) = synthetic(levels, OMEGA);
    let s = seams(N, &stopped, &live, &escaped, &t_end);
    assert!(
        !s.fires(),
        "the seam check fired on a field with no seams: {s:?}"
    );
}

#[test]
fn stop_images_seams_quiet_on_a_smooth_field() {
    check_quiet(0);
    // Strata in `t_end` with a field that has stopped moving (ω = 0) are no seam: freezing a converged quantity is
    // nearly a no-op (pitfalls §1.6).
    let (stopped, live, escaped, t_end) = synthetic(LEVELS, 0.0);
    let s = seams(N, &stopped, &live, &escaped, &t_end);
    assert!(
        !s.fires(),
        "the seam check fired on a converged field: {s:?}"
    );
}

negative_control!(
    stop_images_seams_quiet_on_a_smooth_field,
    "the synthetic patchwork is not a field with no seams",
    expected = "fired on a field with no seams",
    check_quiet(LEVELS)
);

/// A rate the synthetic field turns at just past the hard edge: a stratum of 50/8 time units steps `θ̃` by
/// 6.25 × 0.064 = 0.4 rad, which the cyclic ramp draws as a step above [`validation::stop_images::HARD_EDGE`] (32) at
/// 82% of phases (measured on the ramp alone, every phase at 2000 points: 7 to 55 steps).
const OMEGA_FIRES: f64 = 0.064;
/// A rate just short of it: 6.25 × 0.025 ≈ 0.16 rad per stratum, plus at most 1.5/64 rad of the base field between
/// neighbours, under 0.2 rad, which the ramp draws as at most 28 steps at every phase: no pair clears the hard edge.
const OMEGA_QUIET: f64 = 0.025;

/// Criterion (written before the run): the seam check fires on the synthetic patchwork turning at `fires` and gives no
/// seam pair at all at `quiet`, so it resolves a factor of 2.5 in the rate about its hard edge, not only the extremes.
fn check_resolution(fires: f64, quiet: f64) {
    let (stopped, live, escaped, t_end) = synthetic(LEVELS, fires);
    let s = seams(N, &stopped, &live, &escaped, &t_end);
    println!("omega {fires}: {s:?}");
    assert!(
        s.fires(),
        "the seam check did not fire at omega {fires}: {s:?}"
    );
    let (stopped, live, escaped, t_end) = synthetic(LEVELS, quiet);
    let s = seams(N, &stopped, &live, &escaped, &t_end);
    println!("omega {quiet}: {s:?}");
    assert!(
        s.seams == 0,
        "the seam check found seam pairs at omega {quiet}: {s:?}"
    );
}

#[test]
fn stop_images_seams_resolution() {
    check_resolution(OMEGA_FIRES, OMEGA_QUIET);
}

negative_control!(
    stop_images_seams_resolution,
    "the two rates swapped: the quiet rate cannot fire the seam check",
    expected = "the seam check did not fire at omega",
    check_resolution(OMEGA_QUIET, OMEGA_FIRES)
);

/// The strata check on `values` distinct `t_end` values over 640 pixels.
fn check_strata(values: u32) {
    let t_end: Vec<u32> = (0..640).map(|i| i % values).collect();
    let h = histogram(&t_end, &vec![true; 640]);
    let got = strata(&h);
    assert!(
        got.is_ok(),
        "the strata check refused {values} values: {got:?}"
    );
}

#[test]
fn stop_images_strata() {
    // The live label's marks: a window every 0.4 time units over T = 50, 125 values.
    check_strata(125);
}

negative_control!(
    stop_images_strata,
    "32 t_end values, the patchwork's ~32 strata",
    expected = "the strata check refused 32 values",
    check_strata(32)
);

/// The side of the one-variable checks' render of the zero view.
const SMALL: usize = 8;

/// The one-variable check on a small render of the zero view, with `perturb` applied to the pixels first. The render
/// must hold an escape pixel of the stop arm, so the escape path is exercised.
fn check_one_variable_on(perturb: fn(&mut [Pixel])) {
    let view = View::Zero.spec().expect("the zero view");
    let mut pixels = render(&view, SMALL, Rule::Default).expect("the view renders");
    let escapes = pixels
        .iter()
        .filter(|p| kernel::payload::sd_state(p.stop.packed_a) == kernel::payload::STATE_ESCAPE)
        .count();
    println!("{SMALL} x {SMALL}: {escapes} escape pixels of the stop arm");
    assert!(escapes > 0, "the render holds no escape pixel");
    perturb(&mut pixels);
    let failures = check_one_variable(&pixels);
    assert!(
        failures.is_empty(),
        "the one-variable check failed: {failures:?}"
    );
}

/// The first pixel the stop arm labels escape (the controls').
#[cfg(feature = "controls")]
fn escape_pixel(pixels: &mut [Pixel]) -> &mut Pixel {
    pixels
        .iter_mut()
        .find(|p| kernel::payload::sd_state(p.stop.packed_a) == kernel::payload::STATE_ESCAPE)
        .expect("an escape pixel")
}

#[test]
fn stop_images_one_variable() {
    check_one_variable_on(|_| {});
}

negative_control!(
    stop_images_one_variable,
    "a stop-arm pixel off escape whose theta is not the live arm's",
    expected = "not labelled escape by the stop arm, yet its words differ",
    check_one_variable_on(|pixels| {
        let p = pixels
            .iter_mut()
            .find(|p| kernel::payload::sd_state(p.stop.packed_a) != kernel::payload::STATE_ESCAPE)
            .expect("a pixel off escape");
        p.stop.theta += 1.0;
    })
);

/// The one-variable check's second leg: the harness's state, `θ̃` and `t_end` are the live arm's.
#[test]
fn stop_images_one_variable_harness() {
    check_one_variable_on(|_| {});
}

negative_control!(
    stop_images_one_variable_harness,
    "a harness pixel whose theta is not the live arm's (an escape pixel, so the first leg does not read it)",
    expected = "the harness's state, theta or t_end differs",
    check_one_variable_on(|pixels| {
        escape_pixel(pixels).harness.theta += 1.0;
    })
);

/// The one-variable check's third leg: the stop and live classes differ only where a stop-arm escape is a live
/// collision.
#[test]
fn stop_images_one_variable_class() {
    check_one_variable_on(|_| {});
}

negative_control!(
    stop_images_one_variable_class,
    "a stop-arm escape the live arm labels bounded, not a collision (an escape pixel, and only its live state changed, \
     so neither other leg reads it)",
    expected = "not an escape taken by a collision",
    check_one_variable_on(|pixels| {
        let p = escape_pixel(pixels);
        p.live.packed_a = kernel::payload::set_state(p.live.packed_a, kernel::payload::STATE_BOUNDED);
    })
);

/// Criterion (#70's physics review, note 1; the tolerance's basis, #74): the stop arm under the energy floor, on the
/// zero view at `n × n` (`stop_images control --n 64`), fires the seam check, and fails the tolerance in at least one
/// leg.
fn check_control(n: usize, rule: Rule) {
    let view = View::Zero.spec().expect("the zero view");
    let pixels = render(&view, n, rule).expect("the view renders");
    let (s, hist) = control_seams(n, &pixels);
    let legs = tolerance_failures(&measure(&pixels, Arm::Live), TOLERANCE);
    println!(
        "control {rule:?} at {n} x {n}: {s:?}; t_end takes {} values; the tolerance fails {legs:?}",
        hist.len()
    );
    assert!(
        s.fires(),
        "the seam check did not fire on the control: {s:?}"
    );
    assert!(!legs.is_empty(), "the tolerance holds on the control");
}

#[test]
#[ignore = "64 x 64 to T = 50 three ways: about 45 s in release, far longer in debug: cargo test --release -p validation --test stop_images -- --ignored stop_images_control_fires --nocapture"]
fn stop_images_control_fires() {
    check_control(64, Rule::EnergyFloor);
}

negative_control!(
    stop_images_control_fires,
    "the default rule, at 8 x 8 to keep the control cheap: no patchwork to fire on",
    expected = "the seam check did not fire on the control",
    check_control(8, Rule::Default)
);

/// Four synthetic pixels, every arm labelled `class`, the stop arm's `θ̃` moved by `dtheta` from the live arm's on
/// the first.
fn tolerance_pixels(class: u32, dtheta: f64) -> Vec<Pixel> {
    let s = engine::precision::SimState64 {
        theta: 1.0,
        packed_a: kernel::payload::set_state(0, class),
        ..Default::default()
    };
    let mut pixels = vec![
        Pixel {
            live: s,
            stop: s,
            harness: s,
        };
        4
    ];
    pixels[0].stop.theta += dtheta;
    pixels
}

/// Criterion: on synthetic pixels the tolerance holds at `|Δθ̃|` = `dtheta`, and fails, naming the leg, where `θ̃` is
/// a NaN, where no pixel is an escape, and where the escape counts differ.
fn check_tolerance(dtheta: f64) {
    let escape = kernel::payload::STATE_ESCAPE;
    let legs = |p: &[Pixel]| tolerance_failures(&measure(p, Arm::Live), TOLERANCE);
    let held = legs(&tolerance_pixels(escape, dtheta));
    assert!(
        held.is_empty(),
        "the tolerance failed at |dtheta| {dtheta}: {held:?}"
    );
    let nan = legs(&tolerance_pixels(escape, f64::NAN));
    assert!(
        nan.iter().any(|f| f.contains("NaN")),
        "a NaN theta passes the tolerance: {nan:?}"
    );
    let none = legs(&tolerance_pixels(kernel::payload::STATE_COLLISION, 0.0));
    assert!(
        none.iter().any(|f| f.contains("nothing to measure")),
        "no escape pixel passes the tolerance: {none:?}"
    );
    let mut counts = tolerance_pixels(escape, 0.0);
    counts[1].live.packed_a = kernel::payload::set_state(0, kernel::payload::STATE_COLLISION);
    let counts = legs(&counts);
    assert!(
        counts
            .iter()
            .any(|f| f.contains("pixels escape, the live arm")),
        "differing escape counts pass the tolerance: {counts:?}"
    );
}

#[test]
fn stop_images_tolerance_legs() {
    check_tolerance(0.05);
}

negative_control!(
    stop_images_tolerance_legs,
    "a stop arm 0.2 rad off the live arm's theta, twice the tolerance",
    expected = "the tolerance failed at |dtheta| 0.2",
    check_tolerance(0.2)
);
