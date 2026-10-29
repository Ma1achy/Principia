//! QA's tests for TASK-M2-32 on the GUI side, written from R-444, R-446, R-449, render_gui_spec §G6 and R-136, not
//! from the implementation:
//!
//! - REQ-COL-101 / R-444 / R-446 / R-467 (the mock's outcome legend, `qa_m2_32_legend_outcome_key`): the default
//!   stain's outcome key is the nine classes, in colour_composition §1.4's order and named as shown (R-449); it has no
//!   "degenerate", "collision @ t=0", "running", `sim_failed` or "not yet" key, and, since R-467 (TASK-M6-46), no
//!   "forbidden region" key, which left the presentation legends.
//! - The same key drawn (`qa_m2_32_legend_swatches_drawn`): each of the nine swatches, rendered through the mock's
//!   canvas, is one flat colour, its 8-bit sRGB R-444's, typed in here; the hatch swatch, since R-467 the debug view's
//!   "forbidden (chart)" entry (View › Debug view on, D5), is the hatch, drawn as the style (render_gui_spec §G6,
//!   "compared as the style"; R-136): every interior pixel `debug_invalid`'s colour at its physical position, both of
//!   the hatch's colours present.
//!
//! gui takes no dependency on validation (R-187), so each check runs its own control (R-176) through [`rejects`].

// The file name `qa_TASK-M2-32` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::sync::Arc;

use eframe::egui::{self, Event, Rect};
use engine::contract::interface::EngineInterface;
use engine::contract::legend::debug_invalid;
use engine::contract::set_field::{Edit, RenderField, SetField};
use gui::app::App;
use gui::capture::{render_frame, Shot, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::canvas::MockCanvas;
use gui::mock::MockEngine;
use gui::side::MockSide;

/// R-444's nine classes as shown (R-449), in §1.4's order, with their sRGB swatches.
const CLASSES: [(&str, u32); 9] = [
    ("bounded / nothing yet", 0x141418),
    ("collision 1–2", 0xDE2D2D),
    ("collision 1–3", 0x2EBC4E),
    ("collision 2–3", 0x3462E0),
    ("triple collision", 0xF29620),
    ("escape body 1", 0xF0DE32),
    ("escape body 2", 0xE034C6),
    ("escape body 3", 0x30C8DC),
    ("triple ejection", 0x611597),
];

/// The forbidden-region key's label, which R-467 removes from the presentation legend (render_gui_spec §G6).
const FORBIDDEN: &str = "forbidden region";

/// The debug view's "Debug" key's entry for the pixels the chart classifies as not a system, in the hatch style
/// (R-467 D5; render_gui_spec §G6).
const DEBUG_FORBIDDEN: &str = "forbidden (chart)";

/// The keys R-444 and R-446 remove from the outcome legend.
const REMOVED: [&str; 6] = [
    "degenerate",
    "collision @ t=0",
    "running",
    "sim_failed",
    "not yet",
    "body 0",
];

/// The hatch's two colours, 8-bit sRGB (REQ-COL-055).
const HATCH: [[u8; 3]; 2] = [[0x9B, 0x00, 0xFF], [0x48, 0xFF, 0xFF]];

fn rgb8(h: u32) -> [u8; 3] {
    [(h >> 16) as u8, (h >> 8) as u8, h as u8]
}

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

impl Rig {
    /// The app on the frozen mock, View › Debug view `debug` (R-467 D5).
    fn new(canvas: Option<Arc<MockCanvas>>, debug: bool) -> Self {
        let mut engine = MockEngine::frozen();
        if debug {
            engine.set_field(SetField {
                edit: Edit::Render(RenderField::DebugView(true)),
                no_history: false,
            });
        }
        let mut rig = Rig {
            app: App::new(MockSide::new(engine, canvas), FORMAT),
            h: Headless::new(SIZE, PIXELS_PER_POINT),
        };
        for _ in 0..2 {
            let _ = rig.h.frame(&mut rig.app, Vec::new());
        }
        let _ = rig.h.frame(&mut rig.app, vec![Event::PointerGone]);
        for _ in 0..2 {
            let _ = rig.h.frame(&mut rig.app, Vec::new());
        }
        rig
    }

    /// The texts drawn inside the legend, top to bottom then left to right, with their rects in points.
    fn legend_texts(&mut self) -> Vec<(String, Rect)> {
        let legend = layout().legend;
        let out = self.h.frame(&mut self.app, Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        let mut t: Vec<(String, Rect)> = flat
            .into_iter()
            .filter_map(|s| match &s {
                egui::Shape::Text(t) => {
                    Some((t.galley.text().to_owned(), s.visual_bounding_rect()))
                }
                _ => None,
            })
            .filter(|(_, r)| legend.expand(1.0).contains_rect(*r))
            .collect();
        t.sort_by(|a, b| {
            (a.1.min.y.round() as i64, a.1.min.x as i64)
                .cmp(&(b.1.min.y.round() as i64, b.1.min.x as i64))
        });
        t
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

fn layout() -> Layout {
    Layout::new(
        Headless::new(SIZE, PIXELS_PER_POINT).screen(),
        PIXELS_PER_POINT,
    )
}

/// The legend's texts hold the nine classes in order, and none of the removed keys, "forbidden region" among them
/// (R-467).
fn check_outcome_key(texts: &[String]) {
    let mut at = 0;
    for name in CLASSES.iter().map(|c| c.0) {
        at += texts[at..]
            .iter()
            .position(|t| t == name)
            .unwrap_or_else(|| panic!("the legend has no `{name}` in order: {texts:?}"))
            + 1;
    }
    for gone in REMOVED {
        assert!(
            !texts.iter().any(|t| t.to_lowercase().contains(gone)),
            "the legend draws `{gone}`, which R-444 / R-446 removed: {texts:?}"
        );
    }
    assert!(
        !texts.iter().any(|t| t.to_lowercase().contains(FORBIDDEN)),
        "the legend draws `{FORBIDDEN}`, which R-467 removed: {texts:?}"
    );
}

#[test]
fn qa_m2_32_legend_outcome_key() {
    let mut rig = Rig::new(None, false);
    let texts: Vec<String> = rig.legend_texts().into_iter().map(|(t, _)| t).collect();
    check_outcome_key(&texts);
    rejects("a legend with the degenerate key", || {
        let mut t = texts.clone();
        t.push("degenerate".to_owned());
        check_outcome_key(&t)
    });
    rejects("a legend that keeps the forbidden region (R-467)", || {
        let mut t = texts.clone();
        t.push(FORBIDDEN.to_owned());
        check_outcome_key(&t)
    });
    rejects("a legend without a class", || {
        let t: Vec<String> = texts
            .iter()
            .filter(|t| *t != "triple ejection")
            .cloned()
            .collect();
        check_outcome_key(&t)
    });
    rejects("a legend with two classes swapped", || {
        let mut t = texts.clone();
        let a = t.iter().position(|x| x == "escape body 1").unwrap();
        let b = t.iter().position(|x| x == "escape body 3").unwrap();
        t.swap(a, b);
        check_outcome_key(&t)
    });
}

fn px(shot: &Shot, x: usize, y: usize) -> [u8; 3] {
    let i = (y * shot.size[0] as usize + x) * 4;
    [shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2]]
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
}

/// IEC 61966-2-1's sRGB encode of a linear channel, to 8 bits.
fn encode8(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0).round() as u8
}

/// The hatch's colour at physical pixel `(x, y)`: `debug_invalid` at its centre, encoded.
fn hatch(x: usize, y: usize) -> [u8; 3] {
    debug_invalid([x as f64 + 0.5, y as f64 + 0.5]).map(encode8)
}

/// The bounding box, in physical pixels, of the pixels `is` picks left of `label` on its row: the swatch beside it,
/// searched within four label heights, inset two pixels from its outline.
fn swatch_box(shot: &Shot, label: Rect, is: impl Fn([u8; 3]) -> bool) -> [usize; 4] {
    let p = PIXELS_PER_POINT;
    let half = label.height() / 2.0;
    let (ya, yb) = (
        ((label.min.y - half) * p) as usize,
        ((label.max.y + half) * p).ceil() as usize,
    );
    let xb = (label.min.x * p) as usize;
    let xa = xb.saturating_sub((4.0 * label.height() * p) as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in ya..yb {
        for x in xa..xb {
            if is(px(shot, x, y)) {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    assert!(
        x1 >= x0 + 8 && y1 >= y0 + 8,
        "no swatch beside the label at {label:?}"
    );
    [x0 + 2, y0 + 2, x1 - 2, y1 - 2]
}

/// Every pixel of `[x0, x1) × [y0, y1)` is `want(x, y)`, within 8-bit rounding.
fn check_box(
    shot: &Shot,
    [x0, y0, x1, y1]: [usize; 4],
    what: &str,
    want: impl Fn(usize, usize) -> [u8; 3],
) {
    for y in y0..y1 {
        for x in x0..x1 {
            let (got, w) = (px(shot, x, y), want(x, y));
            assert!(
                near(got, w),
                "{what}: pixel ({x}, {y}) is {got:02X?}, not {w:02X?}"
            );
        }
    }
}

#[test]
fn qa_m2_32_legend_swatches_drawn() {
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    // The presentation legend, the debug view off: the nine classes.
    let mut rig = Rig::new(Some(canvas.clone()), false);
    let texts = rig.legend_texts();
    let shot = render_frame(&canvas, &mut rig.app, &mut rig.h);
    let label = |name: &str| {
        texts
            .iter()
            .find(|(t, _)| t == name)
            .unwrap_or_else(|| panic!("no `{name}` in the legend"))
            .1
    };
    // The nine classes: each swatch one flat colour, R-444's.
    for (name, hex) in CLASSES {
        let c = rgb8(hex);
        let b = swatch_box(&shot, label(name), |p| near(p, c));
        check_box(&shot, b, name, |_, _| c);
    }
    let (classes, class_texts) = (shot, texts);
    let label_of = |name: &str| {
        class_texts
            .iter()
            .find(|(t, _)| t == name)
            .unwrap_or_else(|| panic!("no `{name}` in the legend"))
            .1
    };
    // The debug view on, so the legend draws the "Debug" key, whose "forbidden (chart)" entry is the hatch (R-467 D5).
    let mut rig = Rig::new(Some(canvas.clone()), true);
    let texts = rig.legend_texts();
    let shot = render_frame(&canvas, &mut rig.app, &mut rig.h);
    let label = |name: &str| {
        texts
            .iter()
            .find(|(t, _)| t == name)
            .unwrap_or_else(|| panic!("no `{name}` in the legend"))
            .1
    };
    // The debug view's forbidden (chart) entry: the hatch at each physical pixel, both colours present.
    let is_hatch = |p: [u8; 3]| HATCH.iter().any(|h| near(p, *h));
    let b = swatch_box(&shot, label(DEBUG_FORBIDDEN), is_hatch);
    check_box(&shot, b, DEBUG_FORBIDDEN, hatch);
    let [x0, y0, x1, y1] = b;
    let seen: Vec<[u8; 3]> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .map(|(x, y)| px(&shot, x, y))
        .collect();
    assert!(
        HATCH.iter().all(|h| seen.iter().any(|p| near(*p, *h))),
        "the forbidden (chart) swatch is not both of the hatch's colours"
    );
    // Controls: the hatch read one pixel off its position; a hatch key painted as one flat colour; a class's swatch
    // read as its neighbour's.
    rejects("the hatch one pixel off its position", || {
        check_box(&shot, b, DEBUG_FORBIDDEN, |x, y| hatch(x + 1, y))
    });
    let flat = Shot {
        size: shot.size,
        rgba: shot
            .rgba
            .chunks(4)
            .flat_map(|_| [0x9B, 0x00, 0xFF, 255])
            .collect(),
        names: Vec::new(),
    };
    rejects("a flat violet hatch key", || {
        check_box(&flat, b, DEBUG_FORBIDDEN, hatch)
    });
    rejects("escape body 1's swatch read as escape body 2's", || {
        let c = rgb8(CLASSES[5].1);
        let b5 = swatch_box(&classes, label_of(CLASSES[5].0), |p| near(p, c));
        check_box(&classes, b5, "escape body 1", |_, _| rgb8(CLASSES[6].1))
    });
}
