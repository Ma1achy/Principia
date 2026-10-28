//! `mock_invalid_hook` (REQ-GUI-226; R-467 D7, D8; render_gui_spec §G12): the mock's invalid-pixel test hook,
//! `MockCanvas::flag_invalid`, stays and is turned on at launch by `PRIN_MOCK_INVALID="s₀,t₀,s₁,t₁"`, read by the
//! window and by the capture mode, flagging its region as `flag_invalid` does; a malformed value is refused, naming it.
//! A region reaching inside `[0,1]²` draws there as past it, the canvas background in the presentation view and the
//! hatch in the debug view, and the console logs one warning per view, severity `warn`, source `app`, naming the mock's
//! chart, the first such pixel in row order from the bottom-left and their count; a pan logs it again once, a repaint
//! without a view change does not, and a region wholly past the square logs none. Each check also runs on an input it
//! must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{Key, Modifiers};
use engine::contract::log::{LogEntry, Severity, Source};
use engine::contract::set_field::{Edit, SetField, SimField};

use super::f3_fallback_mock::{check_background, check_hatched, debug_then, DEBUG_VIEW};
use super::f3_fills_window_mock::{canvas, layout};
use super::support::{headless, press, rejects};
use crate::app::App;
use crate::capture::{shoot, Step, FORMAT, PIXELS_PER_POINT};
use crate::cli::mock::{canvas_flagging, dispatch, invalid_region, INVALID_VAR};
use crate::headless::Headless;
use crate::mock::canvas::{MockCanvas, NO_INVALID};
use crate::mock::invalid::{warning, Inside};
use crate::mock::MockEngine;
use crate::side::{EngineSide, FigureView, MockSide};

#[test]
fn mock_invalid_hook_parses_the_switch() {
    assert_eq!(invalid_region(None), Ok(None), "unset flags none");
    assert_eq!(
        invalid_region(Some("-0.5,-0.5,0,1.5")),
        Ok(Some([-0.5, -0.5, 0.0, 1.5]))
    );
    assert_eq!(
        invalid_region(Some(" 0.4, 0.4 ,0.6,0.6 ")),
        Ok(Some([0.4, 0.4, 0.6, 0.6])),
        "spaces around the parts"
    );
    // At the edge of empty: a region one step wide on either axis is a region.
    let tiny = f32::from_bits(1);
    assert_eq!(
        invalid_region(Some(&format!("0,0,{tiny:e},{tiny:e}"))),
        Ok(Some([0.0, 0.0, tiny, tiny]))
    );
    for bad in [
        "",
        "abc",
        "0,0,1",
        "0,0,1,1,1",
        "0,0,x,1",
        "NaN,0,1,1",
        "0,0,inf,1",
        "0,-inf,1,1",
        "0,0,0,1",
        "0,0,1,0",
        "1,0,0,1",
        "0,1,1,0",
    ] {
        let err = invalid_region(Some(bad)).expect_err(bad);
        assert!(
            err.starts_with(&format!("{INVALID_VAR}={bad:?} is not a region")),
            "{err}"
        );
    }
    rejects("a malformed value taken", || {
        invalid_region(Some("0,0,1")).expect("taken");
    });
    // The launch path refuses it before opening a device.
    let err = canvas_flagging(Some("0,0,1"))
        .err()
        .expect("a malformed value refused at launch");
    assert!(
        err.contains(INVALID_VAR) && err.contains("\"0,0,1\""),
        "{err}"
    );
}

/// Reads the capture the capture mode wrote into `dir`.
fn read_capture(dir: &std::path::Path) -> Vec<u8> {
    let file = std::fs::File::open(dir.join(crate::capture::CAPTURE)).expect("the capture");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("a PNG");
    let mut rgba = vec![0; reader.output_buffer_size().expect("a size")];
    reader.next_frame(&mut rgba).expect("its pixels");
    rgba
}

#[test]
fn mock_invalid_hook_launch_flags_as_flag_invalid_does() {
    let region = [-0.5, -0.5, 0.0, 1.5];
    let launched = canvas_flagging(Some("-0.5,-0.5,0,1.5")).expect("a canvas");
    assert_eq!(launched.invalid(), region);
    assert_eq!(
        canvas_flagging(None).expect("a canvas").invalid(),
        NO_INVALID
    );
    let hooked = canvas();
    hooked.flag_invalid(region);
    let want = shoot(hooked.clone(), &[Step::F3]);
    assert_eq!(shoot(launched, &[Step::F3]).rgba, want.rgba);
    // The capture mode reads the variable: its capture is the hooked one. Only a well-formed value is ever set, so a
    // test running beside this one that launches the mock still launches.
    let out = std::env::temp_dir().join(format!("gui-mock-invalid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let args: Vec<String> = ["capture", "--screen", "01_main", "--steps", "f3", "--out"]
        .iter()
        .map(|a| a.to_string())
        .chain([out.to_string_lossy().into_owned()])
        .collect();
    std::env::set_var(INVALID_VAR, "-0.5,-0.5,0,1.5");
    let outcome = dispatch(&args, |_, _, _| Ok(()));
    std::env::remove_var(INVALID_VAR);
    assert_eq!(outcome, Ok(()));
    let captured = read_capture(&out);
    std::fs::remove_dir_all(&out).expect("scratch removed");
    assert!(
        captured == want.rgba,
        "the capture mode ignored the variable"
    );
    let plain = shoot(canvas(), &[Step::F3]);
    rejects("a launch that flags nothing", || {
        assert!(plain.rgba == want.rgba)
    });
}

/// Inside `[0,1]²`, as D8's second example.
const INSIDE: [f32; 4] = [0.4, 0.4, 0.6, 0.6];

#[test]
fn mock_invalid_hook_inside_the_square_is_drawn_as_past_it() {
    let canvas = canvas();
    canvas.flag_invalid(INSIDE);
    let presented = shoot(canvas.clone(), &[]);
    let debugged = shoot(canvas.clone(), &DEBUG_VIEW);
    let hidden = shoot(canvas.clone(), &debug_then(&[Step::F3]));
    canvas.flag_invalid(NO_INVALID);
    assert!(check_background(&presented.rgba, INSIDE) > 10_000);
    assert!(check_hatched(&debugged.rgba, INSIDE) > 10_000);
    assert!(check_hatched(&hidden.rgba, INSIDE) > 10_000, "F3 off");
    rejects("the presentation view hatching inside the square", || {
        check_hatched(&presented.rgba, INSIDE);
    });
}

/// An app on a frozen mock drawing through a canvas flagging `region`, after a few frames.
fn app_flagging(region: [f32; 4]) -> (App<MockSide>, Headless, Arc<MockCanvas>) {
    let canvas = canvas();
    canvas.flag_invalid(region);
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    let mut h = headless();
    repaint(&mut h, &mut app);
    (app, h, canvas)
}

/// Four frames with no input.
fn repaint(h: &mut Headless, app: &mut App<MockSide>) {
    for _ in 0..4 {
        let _ = h.frame(app, Vec::new());
    }
}

/// The console's decoder-bug warnings.
fn warnings(app: &App<MockSide>) -> Vec<LogEntry> {
    app.console()
        .iter()
        .filter(|e| e.message.starts_with("decoder bug"))
        .cloned()
        .collect()
}

/// The decoder-bug warning for `region` in the figure `view`, found pixel by pixel: of the window's pixels the figure
/// is drawn on, those whose centre's field point, in `f32` as the shader takes it, is in `region` and in `[0, 1]²`,
/// the first in row order from the bottom-left, and their count.
fn expected(view: &FigureView, region: [f32; 4]) -> Option<Inside> {
    let [w, h] = view.size;
    let [x0, y0, x1, y1] = view.clip;
    let p = view.placement;
    let mut first = None;
    let mut count = 0;
    for up in 0..h {
        let row = h - 1 - up;
        if row < y0 || row >= y1 {
            continue;
        }
        let v = 1.0 - (row as f32 + 0.5) / h as f32 * 2.0;
        let yt = (v - p.centre[1]) / p.half[1];
        let t = (yt + 1.0) * 0.5;
        for x in x0..x1 {
            let u = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let xs = (u - p.centre[0]) / p.half[0];
            let s = (xs + 1.0) * 0.5;
            let flagged = s >= region[0] && s < region[2] && t >= region[1] && t < region[3];
            if flagged && xs.abs() <= 1.0 && yt.abs() <= 1.0 {
                count += 1;
                first.get_or_insert(([x, up], [s, t]));
            }
        }
    }
    first.map(|(pixel, st)| Inside { count, pixel, st })
}

/// Checks that `entries` hold exactly one decoder-bug warning, `warn` from `app`, reading `want`.
fn check_one(entries: &[LogEntry], want: &str) {
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0].severity, Severity::Warn);
    assert_eq!(entries[0].source, Source::App);
    assert_eq!(entries[0].message, want);
}

#[test]
fn mock_invalid_hook_warns_once_per_view() {
    let (mut app, mut h, _canvas) = app_flagging(INSIDE);
    let shown = FigureView::of(
        &layout(),
        true,
        PIXELS_PER_POINT,
        crate::mock::canvas::default_background(),
    );
    let inside = expected(&shown, INSIDE).expect("pixels inside the square");
    assert!(inside.count > 10_000, "{inside:?}");
    let want = warning("z_α × z_β", &inside);
    assert!(
        want.starts_with("decoder bug: chart z_α × z_β fails validity inside [0,1]² at pixel ("),
        "{want}"
    );
    assert!(want.ends_with(&format!("; {} pixels in this view", inside.count)));
    check_one(&warnings(&app), &want);
    // A repaint, F3 off and on: no view change, no new warning.
    repaint(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    repaint(&mut h, &mut app);
    press(&mut h, &mut app, Key::F3, Modifiers::NONE);
    repaint(&mut h, &mut app);
    check_one(&warnings(&app), &want);
    assert_eq!(app.counts().warnings, 1, "the footer counts it");
    // Controls: a warning per frame, and one naming another pixel than the first, fail.
    let twice: Vec<LogEntry> = [warnings(&app), warnings(&app)].concat();
    rejects("a warning per frame", || check_one(&twice, &want));
    let other = warning(
        "z_α × z_β",
        &Inside {
            pixel: [inside.pixel[0] + 1, inside.pixel[1]],
            ..inside
        },
    );
    rejects("a warning naming another pixel than the first", || {
        check_one(&warnings(&app), &other)
    });
    // A pan: logged again, once.
    let mut z0 = app.snapshot().sim.plane.z0;
    z0[0] += 0.01;
    app.set_field(SetField {
        edit: Edit::Sim(SimField::Z0(z0)),
        no_history: false,
    });
    repaint(&mut h, &mut app);
    repaint(&mut h, &mut app);
    let after = warnings(&app);
    assert_eq!(after.len(), 2, "{after:?}");
    check_one(&after[1..], &want);
}

#[test]
fn mock_invalid_hook_warns_again_for_another_region() {
    let (mut app, mut h, canvas) = app_flagging(INSIDE);
    assert_eq!(warnings(&app).len(), 1);
    let narrower = [0.45, 0.4, 0.6, 0.6];
    canvas.flag_invalid(narrower);
    repaint(&mut h, &mut app);
    let all = warnings(&app);
    assert_eq!(all.len(), 2, "{all:?}");
    let shown = FigureView::of(
        &layout(),
        true,
        PIXELS_PER_POINT,
        crate::mock::canvas::default_background(),
    );
    let inside = expected(&shown, narrower).unwrap();
    check_one(&all[1..], &warning("z_α × z_β", &inside));
}

#[test]
fn mock_invalid_hook_past_the_square_warns_nothing() {
    for region in [
        [-0.4, 0.2, -0.1, 0.8],
        [1.0, 0.2, 1.3, 0.8],
        [0.2, 1.0, 0.8, 1.3],
    ] {
        let (mut app, mut h, _canvas) = app_flagging(region);
        press(&mut h, &mut app, Key::F3, Modifiers::NONE);
        repaint(&mut h, &mut app);
        assert!(
            warnings(&app).is_empty(),
            "{region:?}: {:?}",
            warnings(&app)
        );
        assert!(
            app.side().engine().flagged().flagged > 0,
            "{region:?} drawn"
        );
    }
    // Control: reaching one pixel's width into the square warns.
    let (app, _, _) = app_flagging([-0.4, 0.2, 0.01, 0.8]);
    rejects("a region reaching into the square", || {
        assert!(warnings(&app).is_empty())
    });
}

/// The figure's clip in its viewport's own pixels: `of` measures it from the window's corner, so a window whose
/// corner is not the screen's origin clips the same pixels as one at the origin.
fn check_clip_from_the_window(view: &FigureView, at_origin: &FigureView) {
    assert_eq!(view.size, at_origin.size, "the viewport's size");
    assert_eq!(
        view.clip, at_origin.clip,
        "the clip, measured from the window's corner"
    );
}

#[test]
fn mock_invalid_hook_clip_measured_from_the_window() {
    use eframe::egui::{pos2, vec2, Rect};

    use crate::layout::Layout;

    let size = vec2(1080.0, 675.0);
    let ppp = PIXELS_PER_POINT;
    let background = [20, 20, 24];
    let origin = Layout::new(Rect::from_min_size(pos2(0.0, 0.0), size), ppp);
    let offset = Layout::new(Rect::from_min_size(pos2(40.0, 30.0), size), ppp);
    for shown in [true, false] {
        let at_origin = FigureView::of(&origin, shown, ppp, background);
        let view = FigureView::of(&offset, shown, ppp, background);
        check_clip_from_the_window(&view, &at_origin);
        rejects("a clip measured from the screen's origin", || {
            let mut from_screen = view;
            from_screen.clip[0] += (40.0 * ppp) as u32;
            from_screen.clip[1] += (30.0 * ppp) as u32;
            check_clip_from_the_window(&from_screen, &at_origin);
        });
    }
}
