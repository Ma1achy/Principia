//! The stop images' checks (`validation::stop_images`), each with a control: the seam check fires on a synthetic
//! patchwork (a smooth field frozen at the `t_end` strata of a smooth termination time) and not on the smooth field;
//! the strata check refuses ~32 `t_end` values; the one-variable check holds on a small render of the app's zero view
//! and names a stop-arm pixel off escape whose words differ from the live arm's.

use validation::negative_control;
use validation::stop_images::{
    check_one_variable, histogram, render, seams, strata, theta_colour, Image, Pixel, Rule,
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

/// The one-variable check on a small render of the zero view, with `perturb` applied to the pixels first.
fn check_one_variable_on(perturb: fn(&mut [Pixel])) {
    let mut pixels = render(8, Rule::Default).expect("the view renders");
    perturb(&mut pixels);
    let failures = check_one_variable(&pixels);
    assert!(
        failures.is_empty(),
        "the one-variable check failed: {failures:?}"
    );
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
