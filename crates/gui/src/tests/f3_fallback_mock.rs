//! `f3_fallback_mock` (REQ-GUI-179; R-407's fallback; chart_decoder_contract § "Past the unit square — each axis's
//! extension type (R-407)"): with F3 off, the stand-in continues past the shown rect as an affine chart, and a pixel
//! there the mock flags invalid (its test hook, `MockCanvas::flag_invalid`: the stand-in is defined everywhere) is
//! drawn with the render contract's hatch, `debug_invalid`, as forbidden: never the stand-in, never the clear colour.
//! Inside the shown rect, `[0, 1]²`, the fallback does not apply. Each check also runs on an input it must reject
//! (R-176, through `rejects`). These run on the GPU, on `PRIN_GPU_BACKEND`'s backend (R-206).

use engine::contract::canvas::hatch::{hatch, hatch_stripe, uniforms_binding};

use super::f3_fills_window_mock::{canvas, check_same_over, pixel, shown_px};
use super::support::rejects;
use crate::capture::{shoot, Step};
use crate::mock::canvas::{CHART_BINDING, LOOK_BINDING, NO_INVALID};

/// The field point `(s, t)` of the capture's pixel `(x, y)`, at its centre, the shown rect `r` in pixels being
/// `[0, 1]²`, `t` Y-up.
fn field_at(r: [f64; 4], x: usize, y: usize) -> (f64, f64) {
    let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
    (
        (cx - r[0]) / (r[2] - r[0]),
        1.0 - (cy - r[1]) / (r[3] - r[1]),
    )
}

/// Whether `(s, t)` lies in `region`, `[s₀, t₀, s₁, t₁)`, more than `margin` inside it (`margin` > 0) or more than
/// `−margin` outside it (`margin` < 0), so a pixel on an edge, where f32 and f64 may round apart, is neither.
fn within(region: [f32; 4], (s, t): (f64, f64), margin: f64) -> bool {
    let r = region.map(f64::from);
    s >= r[0] + margin && s < r[2] - margin && t >= r[1] + margin && t < r[3] - margin
}

/// The hatch's colour at the capture's pixel `(x, y)`, as 8-bit sRGB: `debug_invalid`'s stripe at its centre.
fn hatch_at(x: usize, y: usize) -> [u8; 4] {
    let [r, g, b] = hatch().colours[hatch_stripe([x as f64 + 0.5, y as f64 + 0.5])];
    [r, g, b, 255]
}

/// Every pixel of `rgba` more than a pixel's width inside `region` is the hatch.
fn check_hatched(rgba: &[u8], region: [f32; 4]) -> usize {
    let r = shown();
    let mut hatched = 0;
    for y in 0..1350 {
        for x in 0..2160 {
            if within(region, field_at(r, x, y), 1e-3) {
                assert_eq!(
                    pixel(rgba, x, y),
                    hatch_at(x, y),
                    "not hatched at ({x}, {y})"
                );
                hatched += 1;
            }
        }
    }
    hatched
}

/// The shown rect in the capture's pixels, as `f64`s.
fn shown() -> [f64; 4] {
    shown_px().map(|v| v as f64)
}

/// Left of the shown rect, inside the window: `s` in `[−0.4, −0.1)`, `t` in `[0.2, 0.8)`.
const LEFT: [f32; 4] = [-0.4, 0.2, -0.1, 0.8];

#[test]
fn f3_fallback_mock_flagged_pixels_are_hatched() {
    let canvas = canvas();
    let plain = shoot(canvas.clone(), &[Step::F3]);
    canvas.flag_invalid(LEFT);
    let flagged = shoot(canvas.clone(), &[Step::F3]);
    let hatched = check_hatched(&flagged.rgba, LEFT);
    assert!(hatched > 10_000, "only {hatched} pixels in the region");
    // Both of the hatch's colours are drawn, so it is the hatch and not one flat colour.
    let colours = hatch().colours;
    let r = shown();
    let mut seen = [false; 2];
    for y in 0..1350 {
        for x in 0..2160 {
            if within(LEFT, field_at(r, x, y), 1e-3) {
                let p = pixel(&flagged.rgba, x, y);
                seen[0] |= p[..3] == colours[0];
                seen[1] |= p[..3] == colours[1];
            }
        }
    }
    assert_eq!(seen, [true, true], "both stripes");
    // Outside the region, past its edges by more than a pixel, the stand-in is drawn as before.
    for y in 0..1350 {
        for x in 0..2160 {
            if !within(LEFT, field_at(r, x, y), -1e-3) {
                assert_eq!(
                    pixel(&flagged.rgba, x, y),
                    pixel(&plain.rgba, x, y),
                    "({x}, {y})"
                );
            }
        }
    }
    // A flagged pixel drawn as the stand-in fails the check.
    rejects("the region drawn as the stand-in", || {
        check_hatched(&plain.rgba, LEFT);
    });
    // Flagging nothing draws nothing hatched.
    canvas.flag_invalid(NO_INVALID);
    let cleared = shoot(canvas, &[Step::F3]);
    assert_eq!(cleared.rgba, plain.rgba);
}

/// Inside the shown rect the fallback does not apply: a region across its left edge is hatched only past it, and the
/// shown rect is drawn as before, F3 off and on.
#[test]
fn f3_fallback_mock_never_inside_the_shown_rect() {
    let canvas = canvas();
    let plain_off = shoot(canvas.clone(), &[Step::F3]);
    let plain_on = shoot(canvas.clone(), &[]);
    let across = [-0.3, 0.3, 0.3, 0.7];
    canvas.flag_invalid(across);
    let off = shoot(canvas.clone(), &[Step::F3]);
    let on = shoot(canvas.clone(), &[]);
    let rect = shown_px();
    check_same_over(
        &off.rgba,
        &plain_off.rgba,
        rect,
        "the shown rect, flagged, F3 off",
    );
    assert_eq!(on.rgba, plain_on.rgba, "the shown layout, flagged");
    let past = [-0.3, 0.3, 0.0, 0.7];
    let hatched = check_hatched(&off.rgba, past);
    assert!(
        hatched > 1000,
        "only {hatched} pixels hatched past the edge"
    );
    // On each axis, past either side: below, above and right of the shown rect, each flagged on its own.
    for region in [
        [0.3, -0.3, 0.7, -0.05],
        [0.3, 1.02, 0.7, 1.04],
        [1.05, 0.3, 1.3, 0.7],
    ] {
        canvas.flag_invalid(region);
        let shot = shoot(canvas.clone(), &[Step::F3]);
        assert!(check_hatched(&shot.rgba, region) > 100, "{region:?}");
    }
    rejects("the shown rect hatched", || {
        check_hatched(&plain_off.rgba, [0.3, 0.3, 0.7, 0.7]);
    });
}

/// The stand-in's uniform leaves the prelude's binding to it, and its own two are apart.
#[test]
fn f3_fallback_mock_bindings_apart() {
    let prelude = uniforms_binding();
    assert_eq!(prelude.group, 0);
    assert_ne!(CHART_BINDING, prelude.binding);
    assert_ne!(LOOK_BINDING, prelude.binding);
    assert_ne!(CHART_BINDING, LOOK_BINDING);
}
