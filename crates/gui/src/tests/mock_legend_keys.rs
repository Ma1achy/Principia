//! `mock_legend_keys` (REQ-GUI-157's Legend frame; render_gui_spec §G2, §G6; R-399; R-444, R-446): the mock evaluates its
//! fake stain into the legend's keys, walking back from OUT, one key per dimension in order; the outcome key is the nine
//! classes, each a flat swatch, the black one `#141418`, then the "forbidden region" key in the hatch style, with no
//! "degenerate", "collision @ t=0", "running" or `sim_failed` key and no "not yet" swatch; the mock's fake samples take
//! only classes of the nine; the GUI draws exactly the keys it receives, in order, and an empty list as an empty frame;
//! adding a dimension or removing brightness changes the keys drawn; a `running` class, which another field's key may
//! show, is drawn as the "not yet" style, its grey with the dot stipple of `debug_not_yet`'s CPU mirror, never one flat
//! colour. Each check also runs on an input it must reject (R-176, through `rejects`).

use std::sync::Arc;

use eframe::egui::{self, pos2, vec2, Rect};
use engine::contract::display;
use engine::contract::legend::{
    not_yet_dot_at, Legend, LegendEntry, LegendKey, SwatchStyle, NOT_YET_DOT_SRGB8, NOT_YET_SRGB8,
    SWATCHES,
};

use super::support::{frame_texts, headless, mock_app, rejects};
use crate::app::App;
use crate::capture::{render_frame, FORMAT, PIXELS_PER_POINT};
use crate::explore::legend::{
    entry_rect, key_height, key_rects, not_yet_mesh, ramp_mesh, share, COLUMNS, MARGIN, NOTE,
    ROW_H, SWATCH, TITLE,
};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::mock::canvas::MockCanvas;
use crate::mock::legend::{
    brightness, default_stain, evaluate, Class, Dimension, FakeStain, DARKEST, FORBIDDEN,
    RAMP_SAMPLES,
};
use crate::mock::sample::FATES;
use crate::mock::MockEngine;
use crate::side::{EngineSide, MockSide};

/// The label of the outcome swatch `param`.
fn label(param: &str) -> String {
    SWATCHES
        .iter()
        .find(|s| s.param == param)
        .unwrap()
        .label
        .to_owned()
}

/// The keys' titles and, for a categorical key, its labels, in order.
fn outline(legend: &Legend) -> Vec<(String, Vec<String>)> {
    legend
        .keys
        .iter()
        .map(|k| match k {
            LegendKey::Categorical { title, entries } => (
                title.clone(),
                entries.iter().map(|e| e.label.clone()).collect(),
            ),
            LegendKey::Ramp { title, .. } => (title.clone(), Vec::new()),
        })
        .collect()
}

#[test]
fn mock_legend_keys_exact_keys_in_order() {
    let legend = evaluate(&default_stain());
    let classes = [
        "bounded",
        "collision_pair_2",
        "collision_pair_1",
        "collision_pair_0",
        "triple_collision",
        "escape_body_0",
        "escape_body_1",
        "escape_body_2",
        "triple_ejection",
    ];
    assert_eq!(
        classes,
        SWATCHES.map(|s| s.param),
        "the nine, in §1.4's order"
    );
    let mut labels: Vec<String> = classes.iter().map(|c| label(c)).collect();
    labels.push(FORBIDDEN.to_owned());
    let want = vec![
        ("colour · outcome class".to_owned(), labels),
        ("brightness · t_end (late fade)".to_owned(), Vec::new()),
    ];
    assert_eq!(outline(&legend), want);
    rejects("the keys in another order", || {
        let mut other = want.clone();
        other.reverse();
        assert_eq!(outline(&legend), other)
    });
    let LegendKey::Categorical { entries, .. } = &legend.keys[0] else {
        panic!("the first key is the outcome class")
    };
    // The shares are the weights over their sum, in percent; every swatch is its palette colour, flat; the forbidden
    // region is the hatch.
    let total: f64 = entries.iter().map(|e| e.share).sum();
    assert!((total - 100.0).abs() < 1e-9, "{total}");
    let weights: f64 = [31.2, 6.4, 5.9, 6.3, 0.9, 11.0, 24.8, 12.1, 1.4, 0.6]
        .iter()
        .sum();
    assert!(
        (entries[0].share - 100.0 * 31.2 / weights).abs() < 1e-9,
        "{}",
        entries[0].share
    );
    for (entry, class) in entries.iter().zip(classes) {
        let srgb = SWATCHES.iter().find(|s| s.param == class).unwrap().srgb8;
        assert_eq!(entry.swatch, SwatchStyle::Flat(srgb), "{class}");
    }
    assert_eq!(
        (entries[0].label.as_str(), entries[0].swatch),
        (
            "bounded / nothing yet",
            SwatchStyle::Flat([0x14, 0x14, 0x18])
        ),
        "the black one"
    );
    assert_eq!(entries.last().unwrap().swatch, SwatchStyle::Hatch);
    for gone in ["degenerate", "collision @ t=0", "running", "sim_failed"] {
        assert!(!entries.iter().any(|e| e.label == gone), "a \"{gone}\" key");
    }
    assert!(
        !entries.iter().any(|e| e.swatch == SwatchStyle::NotYet),
        "a \"not yet\" swatch"
    );
    rejects("a running key beside the nine", || {
        let mut stain = default_stain();
        let Dimension::Categorical { classes, .. } = &mut stain.out[0] else {
            panic!()
        };
        classes.push((Class::Running, 0.6));
        let LegendKey::Categorical { entries, .. } = &evaluate(&stain).keys[0] else {
            panic!()
        };
        assert!(!entries.iter().any(|e| e.swatch == SwatchStyle::NotYet));
    });
    // The bar: full at 0, darkest at 50, its samples falling between.
    let LegendKey::Ramp {
        lo,
        hi,
        lo_note,
        hi_note,
        samples,
        ..
    } = &legend.keys[1]
    else {
        panic!("the second key is the bar")
    };
    assert_eq!(
        (*lo, *hi, lo_note.as_str(), hi_note.as_str()),
        (0.0, 50.0, "full", "darkest")
    );
    assert_eq!(samples.len(), RAMP_SAMPLES);
    assert_eq!(samples[0], [255; 3]);
    let dark = (DARKEST * 255.0).round() as u8;
    assert_eq!(*samples.last().unwrap(), [dark; 3]);
    assert!(
        samples.windows(2).all(|w| w[1][0] <= w[0][0]),
        "darker to the right"
    );
}

/// Checks that every fate in `fates` is a class of the nine (colour_composition §1.4; R-444).
fn check_fates(fates: &[&str]) {
    for fate in fates {
        assert!(
            SWATCHES.iter().any(|s| s.param == *fate),
            "the fake samples' fate `{fate}`"
        );
    }
}

#[test]
fn mock_legend_keys_fake_samples_take_the_nine() {
    check_fates(&FATES);
    rejects("a fate removed by R-445 and R-446", || {
        check_fates(&["degenerate", "collision_at_start"])
    });
}

#[test]
fn mock_legend_keys_brightness_and_shares_at_their_edges() {
    assert_eq!(brightness(0.0, 0.0, 50.0), 1.0);
    assert!((brightness(50.0, 0.0, 50.0) - DARKEST).abs() < 1e-12);
    assert!((brightness(25.0, 0.0, 50.0) - (1.0 - 0.5 * (1.0 - DARKEST))).abs() < 1e-12);
    assert_eq!(brightness(-1.0, 0.0, 50.0), 1.0, "clamped below");
    assert!(
        (brightness(51.0, 0.0, 50.0) - DARKEST).abs() < 1e-12,
        "clamped above"
    );
    rejects("a channel brightest at the end", || {
        assert_eq!(brightness(50.0, 0.0, 50.0), 1.0)
    });
    // No weight at all: every share 0.
    let empty = FakeStain {
        out: vec![Dimension::Categorical {
            title: "nothing",
            classes: vec![(Class::Swatch("bounded"), 0.0), (Class::Running, 0.0)],
        }],
    };
    let LegendKey::Categorical { entries, .. } = &evaluate(&empty).keys[0] else {
        panic!()
    };
    assert!(entries.iter().all(|e| e.share == 0.0));
    // A one-sample bar and an empty one draw.
    let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 10.0));
    assert!(ramp_mesh(bar, &[]).is_empty());
    assert_eq!(ramp_mesh(bar, &[[9; 3]]).vertices.len(), 2);
    let mesh = ramp_mesh(bar, &[[0; 3], [128; 3], [255; 3]]);
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(mesh.indices.len(), 12);
    assert_eq!(
        mesh.vertices[4].pos.x, 100.0,
        "the last sample at the bar's right end"
    );
    assert_eq!(share(31.24), "31.2");
}

/// The app with the mock's stain `stain`, after a frame that reads it.
fn with_stain(stain: FakeStain) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    app.side().engine().set_stain(stain);
    let mut h = headless();
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    (app, h)
}

#[test]
fn mock_legend_keys_the_gui_draws_what_it_receives() {
    let (mut app, mut h) = with_stain(default_stain());
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.iter().any(|t| t == TITLE) && texts.iter().any(|t| t == NOTE));
    assert!(
        texts.iter().any(|t| t == "colour · outcome class"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == FORBIDDEN));
    assert!(
        texts.iter().any(|t| t == "31.0"),
        "bounded's share: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("Fate")),
        "called Legend, never Fate"
    );
    // Another dimension, added: drawn after the others.
    let mut more = default_stain();
    more.out.push(Dimension::Categorical {
        title: "colour · stability",
        classes: vec![(Class::Swatch("bounded"), 1.0)],
    });
    assert_eq!(evaluate(&more).keys.len(), 3);
    let (mut app, mut h) = with_stain(more);
    let texts = frame_texts(&mut h, &mut app);
    let at = |t: &str| texts.iter().position(|x| x == t);
    assert!(at("colour · stability").is_some(), "the new key: {texts:?}");
    assert!(
        at("colour · outcome class") < at("colour · stability"),
        "in OUT's order"
    );
    // Brightness removed: no bar.
    let mut fewer = default_stain();
    fewer.out.pop();
    let (mut app, mut h) = with_stain(fewer.clone());
    let texts = frame_texts(&mut h, &mut app);
    assert_eq!(app.snapshot().legend, evaluate(&fewer));
    assert_eq!(app.snapshot().legend.keys.len(), 1);
    assert!(!texts.iter().any(|t| t.contains("brightness")), "{texts:?}");
    rejects("a legend that keeps its own bar", || {
        assert!(texts.iter().any(|t| t.contains("brightness")))
    });
    // None: an empty frame, its title and note only.
    let (mut app, mut h) = with_stain(FakeStain { out: Vec::new() });
    let texts = frame_texts(&mut h, &mut app);
    assert!(app.snapshot().legend.keys.is_empty());
    assert!(texts.iter().any(|t| t == TITLE));
    assert!(!texts
        .iter()
        .any(|t| t == FORBIDDEN || t.contains("outcome class")));
}

#[test]
fn mock_legend_keys_layout() {
    let body = Rect::from_min_size(pos2(10.0, 20.0), vec2(400.0, 300.0));
    let legend = evaluate(&default_stain());
    let rects = key_rects(&legend, body);
    assert_eq!(rects.len(), 2);
    assert!(rects[0].max.y < rects[1].min.y, "top to bottom");
    assert_eq!(rects[0].height(), key_height(&legend.keys[0]));
    // Ten entries in two columns: five rows and the title's.
    assert_eq!(key_height(&legend.keys[0]), ROW_H * 6.0);
    let odd = LegendKey::Categorical {
        title: String::new(),
        entries: vec![
            LegendEntry {
                label: String::new(),
                swatch: SwatchStyle::NotYet,
                share: 0.0,
            };
            3
        ],
    };
    assert_eq!(key_height(&odd), ROW_H * 3.0, "three entries take two rows");
    assert!(key_rects(&Legend::default(), body).is_empty());
    let origin = pos2(0.0, 0.0);
    assert_eq!(entry_rect(400.0, origin, 0).min, origin);
    assert_eq!(entry_rect(400.0, origin, 1).min, pos2(200.0, 0.0));
    assert_eq!(entry_rect(400.0, origin, COLUMNS).min, pos2(0.0, ROW_H));
    rejects("entries in one column", || {
        assert_eq!(entry_rect(400.0, origin, 1).min, pos2(0.0, ROW_H))
    });
}

// --- The "not yet" style: running, never one flat colour (R-399), in a key of another field (R-446) ----------------------

#[test]
fn mock_legend_keys_running_is_the_not_yet_stipple() {
    // The mesh: the grey, and a dot on each pixel the mirror dots.
    let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(SWATCH, SWATCH));
    let mesh = not_yet_mesh(rect, PIXELS_PER_POINT);
    let px = |v: f32| (v * PIXELS_PER_POINT).round() as i64;
    let mut dots = 0;
    for y in px(rect.min.y)..px(rect.max.y) {
        for x in px(rect.min.x)..px(rect.max.x) {
            dots += usize::from(not_yet_dot_at([x as f64 + 0.5, y as f64 + 0.5]));
        }
    }
    assert!(dots > 0);
    assert_eq!(
        mesh.vertices.len(),
        4 * (1 + dots),
        "the grey and one quad per dot"
    );
    let grey = egui::Color32::from_rgb(NOT_YET_SRGB8[0], NOT_YET_SRGB8[1], NOT_YET_SRGB8[2]);
    let dot = egui::Color32::from_rgb(
        NOT_YET_DOT_SRGB8[0],
        NOT_YET_DOT_SRGB8[1],
        NOT_YET_DOT_SRGB8[2],
    );
    assert!(mesh.vertices[..4].iter().all(|v| v.color == grey));
    assert!(mesh.vertices[4..].iter().all(|v| v.color == dot));
    rejects("a flat swatch", || assert_eq!(mesh.vertices.len(), 4));
    // On the screen: the running swatch's pixels are the grey and the dot, both, as the mirror places them.
    let canvas = Arc::new(MockCanvas::new().expect("a GPU adapter for the mock's canvas"));
    let mut app = App::new(
        MockSide::new(MockEngine::frozen(), Some(canvas.clone())),
        FORMAT,
    );
    // The outcome key has no running class since R-446; a key of another field, set first, shows it.
    let mut stain = default_stain();
    stain.out.insert(
        0,
        Dimension::Categorical {
            title: "colour · d_min",
            classes: vec![(Class::Swatch("bounded"), 1.0), (Class::Running, 1.0)],
        },
    );
    app.side().engine().set_stain(stain);
    let mut h = headless();
    for _ in 0..2 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let shot = render_frame(&canvas, &mut app, &mut h);
    let legend = app.snapshot().legend.clone();
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let body = layout.legend.shrink(MARGIN);
    let area = key_rects(&legend, body)[0];
    let LegendKey::Categorical { entries, .. } = &legend.keys[0] else {
        panic!()
    };
    let i = entries
        .iter()
        .position(|e| e.swatch == SwatchStyle::NotYet)
        .unwrap();
    let cell = entry_rect(area.width(), area.min + vec2(0.0, ROW_H), i);
    let square = Rect::from_center_size(
        pos2(cell.min.x + SWATCH / 2.0, cell.center().y),
        vec2(SWATCH, SWATCH),
    )
    .shrink(2.0);
    let (mut seen_grey, mut seen_dot, mut wrong) = (0, 0, 0);
    for y in px(square.min.y)..px(square.max.y) {
        for x in px(square.min.x)..px(square.max.x) {
            let i = (y as usize * shot.size[0] as usize + x as usize) * 4;
            let p = [shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2]];
            let want = if not_yet_dot_at([x as f64 + 0.5, y as f64 + 0.5]) {
                NOT_YET_DOT_SRGB8
            } else {
                NOT_YET_SRGB8
            };
            if p == NOT_YET_SRGB8 {
                seen_grey += 1;
            } else if p == NOT_YET_DOT_SRGB8 {
                seen_dot += 1;
            }
            if p != want {
                wrong += 1;
            }
        }
    }
    assert!(
        seen_grey > 0 && seen_dot > 0,
        "grey {seen_grey}, dots {seen_dot}"
    );
    assert_eq!(wrong, 0, "every pixel as the mirror places it");
    rejects("one flat colour", || assert_eq!(seen_dot, 0));
}

/// Checks that every key of `legend` reads the display module's names (R-449; REQ-GUI-188): an outcome swatch its
/// class name, the black one "bounded / nothing yet", and that no label shows an internal id.
fn check_keys_read_the_module(legend: &Legend) {
    for key in &legend.keys {
        let LegendKey::Categorical { entries, .. } = key else {
            continue;
        };
        for e in entries {
            assert!(
                !display::shows_an_internal_id(&e.label),
                "the key `{}` shows an internal id",
                e.label
            );
            if let SwatchStyle::Flat(srgb) = e.swatch {
                let s = SWATCHES
                    .iter()
                    .find(|s| s.srgb8 == srgb)
                    .unwrap_or_else(|| panic!("no swatch of `{}`", e.label));
                assert_eq!(
                    Some(e.label.as_str()),
                    display::class_name_of(s.state, s.detail),
                    "the key of `{}`",
                    s.param
                );
            }
        }
    }
}

#[test]
fn mock_legend_keys_read_the_display_names() {
    let legend = evaluate(&default_stain());
    check_keys_read_the_module(&legend);
    let LegendKey::Categorical { entries, .. } = &legend.keys[0] else {
        panic!("the first key is the outcome class")
    };
    let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "bounded / nothing yet",
            "collision 1–2",
            "collision 1–3",
            "collision 2–3",
            "triple collision",
            "escape body 1",
            "escape body 2",
            "escape body 3",
            "triple ejection",
            "forbidden region",
        ]
    );
    // A running key, which another field's legend may show, reads the function's name too.
    let running = evaluate(&FakeStain {
        out: vec![Dimension::Categorical {
            title: "colour · d_min",
            classes: vec![(Class::Running, 1.0)],
        }],
    });
    assert_eq!(outline(&running)[0].1, ["running"]);
    // The legend the GUI draws: none of its texts shows an internal id.
    let mut h = headless();
    let mut app = mock_app();
    for t in frame_texts(&mut h, &mut app) {
        assert!(!display::shows_an_internal_id(&t), "the frame draws `{t}`");
    }
    rejects("a key written 0-based", || {
        let mut l = legend.clone();
        if let LegendKey::Categorical { entries, .. } = &mut l.keys[0] {
            entries[5].label = "escape body 0".to_owned();
        }
        check_keys_read_the_module(&l)
    });
    rejects("a key named by its pair id", || {
        let mut l = legend.clone();
        if let LegendKey::Categorical { entries, .. } = &mut l.keys[0] {
            entries[1].label = "collision pair 2".to_owned();
        }
        check_keys_read_the_module(&l)
    });
}
