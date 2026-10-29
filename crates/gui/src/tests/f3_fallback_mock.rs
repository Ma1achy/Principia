//! `f3_fallback_mock` (REQ-GUI-179, REQ-GUI-225; R-407's fallback as R-467 draws it; chart_decoder_contract § "Past
//! the unit square — each axis's extension type (R-407)"): with F3 off, the stand-in continues past the shown rect as an
//! affine chart, and a pixel the mock flags invalid (its test hook, `MockCanvas::flag_invalid`: the stand-in is defined
//! everywhere), past the shown rect or inside it, is not part of the chart. The presentation view draws it as the canvas
//! background, the colour the window is cleared to, with no hatch; the debug view (View › Debug view) with the render
//! contract's hatch, `debug_invalid`; every other pixel is the stand-in in both. Each check also runs on an input it
//! must reject (R-176, through `rejects`). These run on the GPU, on `PRIN_GPU_BACKEND`'s backend (R-206).

use engine::contract::canvas::hatch::{hatch, hatch_stripe, uniforms_binding};

use super::f3_fills_window_mock::{canvas, clear, pixel, shown_px};
use super::support::rejects;
use crate::capture::{shoot, Step};
use crate::mock::canvas::{CHART_BINDING, LOOK_BINDING, NO_INVALID};

/// The steps that turn View › Debug view on from the keyboard: Tab to the top bar, Enter into it, → to View, Enter to
/// open it on F3 hide, ↓ to Debug view, Enter.
pub(super) const DEBUG_VIEW: [Step; 6] = [
    Step::Tab,
    Step::Enter,
    Step::ArrowRight,
    Step::Enter,
    Step::ArrowDown,
    Step::Enter,
];

/// [`DEBUG_VIEW`], then `then`.
pub(super) fn debug_then(then: &[Step]) -> Vec<Step> {
    DEBUG_VIEW.iter().chain(then).copied().collect()
}

/// The field point `(s, t)` of the capture's pixel `(x, y)`, at its centre, the shown rect `r` in pixels being
/// `[0, 1]²`, `t` Y-up.
pub(super) fn field_at(r: [f64; 4], x: usize, y: usize) -> (f64, f64) {
    let (cx, cy) = (x as f64 + 0.5, y as f64 + 0.5);
    (
        (cx - r[0]) / (r[2] - r[0]),
        1.0 - (cy - r[1]) / (r[3] - r[1]),
    )
}

/// Whether `(s, t)` lies in `region`, `[s₀, t₀, s₁, t₁)`, more than `margin` inside it (`margin` > 0) or more than
/// `−margin` outside it (`margin` < 0), so a pixel on an edge, where f32 and f64 may round apart, is neither.
pub(super) fn within(region: [f32; 4], (s, t): (f64, f64), margin: f64) -> bool {
    let r = region.map(f64::from);
    s >= r[0] + margin && s < r[2] - margin && t >= r[1] + margin && t < r[3] - margin
}

/// The hatch's colour at the capture's pixel `(x, y)`, as 8-bit sRGB: `debug_invalid`'s stripe at its centre.
pub(super) fn hatch_at(x: usize, y: usize) -> [u8; 4] {
    let [r, g, b] = hatch().colours[hatch_stripe([x as f64 + 0.5, y as f64 + 0.5])];
    [r, g, b, 255]
}

/// The shown rect in the capture's pixels, as `f64`s.
pub(super) fn shown() -> [f64; 4] {
    shown_px().map(|v| v as f64)
}

/// Every pixel of `rgba` more than a pixel's width inside `region` is what `want` gives at it; returns how many.
fn check_region(rgba: &[u8], region: [f32; 4], want: impl Fn(usize, usize) -> [u8; 4]) -> usize {
    let r = shown();
    let mut n = 0;
    for y in 0..1350 {
        for x in 0..2160 {
            if within(region, field_at(r, x, y), 1e-3) {
                assert_eq!(pixel(rgba, x, y), want(x, y), "at ({x}, {y})");
                n += 1;
            }
        }
    }
    n
}

/// Every pixel more than a pixel's width inside `region` is the hatch; returns how many.
pub(super) fn check_hatched(rgba: &[u8], region: [f32; 4]) -> usize {
    check_region(rgba, region, hatch_at)
}

/// Every pixel more than a pixel's width inside `region` is the canvas background, the clear colour; returns how many.
pub(super) fn check_background(rgba: &[u8], region: [f32; 4]) -> usize {
    check_region(rgba, region, |_, _| clear())
}

/// Every pixel more than a pixel's width outside `region` is the same in `a` and `b`.
pub(super) fn check_same_outside(a: &[u8], b: &[u8], region: [f32; 4], what: &str) {
    let r = shown();
    for y in 0..1350 {
        for x in 0..2160 {
            if !within(region, field_at(r, x, y), -1e-3) {
                assert_eq!(pixel(a, x, y), pixel(b, x, y), "{what} at ({x}, {y})");
            }
        }
    }
}

/// Left of the shown rect, inside the window: `s` in `[−0.4, −0.1)`, `t` in `[0.2, 0.8)`.
const LEFT: [f32; 4] = [-0.4, 0.2, -0.1, 0.8];

#[test]
fn f3_fallback_mock_flagged_pixels_background_and_hatched() {
    let canvas = canvas();
    let plain = shoot(canvas.clone(), &[Step::F3]);
    let plain_debug = shoot(canvas.clone(), &debug_then(&[Step::F3]));
    // With nothing flagged, the debug view draws the figure as the presentation view does.
    assert_eq!(
        plain_debug.rgba, plain.rgba,
        "the debug view, nothing flagged"
    );
    canvas.flag_invalid(LEFT);
    let presented = shoot(canvas.clone(), &[Step::F3]);
    let debugged = shoot(canvas.clone(), &debug_then(&[Step::F3]));
    // The presentation view: the region is the canvas background, never the hatch.
    let n = check_background(&presented.rgba, LEFT);
    assert!(n > 10_000, "only {n} pixels in the region");
    // The debug view: the same pixels hatched, both of the hatch's colours, so it is the hatch and not one colour.
    assert_eq!(check_hatched(&debugged.rgba, LEFT), n);
    let colours = hatch().colours;
    let r = shown();
    let mut seen = [false; 2];
    for y in 0..1350 {
        for x in 0..2160 {
            if within(LEFT, field_at(r, x, y), 1e-3) {
                let p = pixel(&debugged.rgba, x, y);
                seen[0] |= p[..3] == colours[0];
                seen[1] |= p[..3] == colours[1];
            }
        }
    }
    assert_eq!(seen, [true, true], "both stripes");
    // Outside the region, past its edges by more than a pixel, the stand-in is drawn as before in both.
    check_same_outside(&presented.rgba, &plain.rgba, LEFT, "presented");
    check_same_outside(&debugged.rgba, &plain.rgba, LEFT, "debugged");
    // Controls: the region drawn as the stand-in, a presentation view that hatches it and a debug view that draws it
    // as the background each fail.
    rejects("the region drawn as the stand-in", || {
        check_background(&plain.rgba, LEFT);
    });
    rejects("a presentation view that hatches the region", || {
        check_background(&debugged.rgba, LEFT);
    });
    rejects(
        "a debug view that draws the region as the background",
        || {
            check_hatched(&presented.rgba, LEFT);
        },
    );
    // Flagging nothing draws nothing flagged.
    canvas.flag_invalid(NO_INVALID);
    let cleared = shoot(canvas, &[Step::F3]);
    assert_eq!(cleared.rgba, plain.rgba);
}

/// A flagged region inside the shown rect is drawn as past it (R-467 D8): the background in the presentation view and
/// the hatch in the debug view, with F3 off and on; and each side of the rect, flagged on its own, likewise.
#[test]
fn f3_fallback_mock_inside_the_shown_rect_too() {
    let canvas = canvas();
    let plain_off = shoot(canvas.clone(), &[Step::F3]);
    let plain_on = shoot(canvas.clone(), &[]);
    let across = [-0.3, 0.3, 0.3, 0.7];
    canvas.flag_invalid(across);
    let off = shoot(canvas.clone(), &[Step::F3]);
    let on = shoot(canvas.clone(), &[]);
    let on_debug = shoot(canvas.clone(), &DEBUG_VIEW);
    let inside = [0.0, 0.3, 0.3, 0.7];
    let past = [-0.3, 0.3, 0.0, 0.7];
    assert!(check_background(&off.rgba, inside) > 1000, "F3 off, inside");
    assert!(check_background(&off.rgba, past) > 1000, "F3 off, past");
    assert!(check_background(&on.rgba, inside) > 1000, "F3 on, inside");
    assert!(check_hatched(&on_debug.rgba, inside) > 1000, "F3 on, debug");
    check_same_outside(&off.rgba, &plain_off.rgba, across, "F3 off");
    rejects("the region inside the rect drawn as the stand-in", || {
        check_background(&plain_on.rgba, inside);
    });
    // On each axis, past either side: below, above and right of the shown rect, each flagged on its own.
    for region in [
        [0.3, -0.3, 0.7, -0.05],
        [0.3, 1.02, 0.7, 1.04],
        [1.05, 0.3, 1.3, 0.7],
    ] {
        canvas.flag_invalid(region);
        let shot = shoot(canvas.clone(), &[Step::F3]);
        assert!(check_background(&shot.rgba, region) > 100, "{region:?}");
        let shot = shoot(canvas.clone(), &debug_then(&[Step::F3]));
        assert!(check_hatched(&shot.rgba, region) > 100, "{region:?}, debug");
    }
    canvas.flag_invalid(NO_INVALID);
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
