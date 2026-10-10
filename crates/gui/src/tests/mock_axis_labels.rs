//! `mock_axis_labels` (render_gui_spec §G2, "Axis labels carry the short axis name and the range at each end"; §G1):
//! the figure's axis labels name each axis and its value at each end, in the strips beside the figure, never over it,
//! and their end values update after a pan. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::{self, Event, Modifiers, PointerButton};
use engine::contract::sim_config::Plane;

use super::support::{headless, mock_app, names, rejects};
use crate::app::App;
use crate::capture::PIXELS_PER_POINT;
use crate::explore::axis_labels::{axes, end_text, texts, Axis};
use crate::headless::{Headless, Name};
use crate::layout::Layout;
use crate::side::MockSide;

/// The names inside the strip under the figure, left to right.
fn strip_names(h: &mut Headless, app: &mut App<MockSide>) -> Vec<Name> {
    let strip = Layout::new(h.screen(), PIXELS_PER_POINT).axis_x;
    let px = |v: f32| f64::from(v * PIXELS_PER_POINT);
    let mut inside: Vec<Name> = names(h, app)
        .into_iter()
        .filter(|n| {
            n.rect.is_some_and(|r| {
                r[0] >= px(strip.min.x) - 1.0
                    && r[2] <= px(strip.max.x) + 1.0
                    && r[1] >= px(strip.min.y) - 1.0
                    && r[3] <= px(strip.max.y) + 1.0
            })
        })
        .collect();
    inside.sort_by(|a, b| {
        a.rect
            .map(|r| r[0])
            .partial_cmp(&b.rect.map(|r| r[0]))
            .unwrap()
    });
    inside
}

fn strip_texts(h: &mut Headless, app: &mut App<MockSide>) -> Vec<String> {
    strip_names(h, app).into_iter().map(|n| n.name).collect()
}

/// A primary drag by `delta` from the figure's centre.
fn drag(h: &mut Headless, app: &mut App<MockSide>, delta: egui::Vec2) {
    let from = Layout::new(h.screen(), PIXELS_PER_POINT).figure.center();
    let to = from + delta;
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(to)],
        vec![button(to, false)],
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

fn plane(app: &App<MockSide>) -> Plane {
    app.snapshot().sim.plane.clone()
}

#[test]
fn mock_axis_labels_name_each_axis_and_its_ends_beside_the_figure() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let [horizontal, vertical] = axes(&plane(&app));
    // The mock's chart, z_α × z_β at z₀ = (0.180, 0.410, …), span 2 along each.
    assert_eq!(
        horizontal,
        Axis {
            name: "z_α",
            low: 0.18 - 1.0,
            high: 0.18 + 1.0
        }
    );
    assert_eq!(
        vertical,
        Axis {
            name: "z_β",
            low: 0.41 - 1.0,
            high: 0.41 + 1.0
        }
    );
    let strip = strip_texts(&mut h, &mut app);
    let want: Vec<String> = texts(&horizontal, "→").into();
    assert_eq!(
        strip, want,
        "low end, name, high end, left to right, under the figure"
    );
    assert_eq!(want, ["−0.82", "z_α →", "+1.18"]);
    rejects("the vertical axis's texts", || {
        assert_eq!(strip, Vec::from(texts(&vertical, "→")));
    });
}

#[test]
fn mock_axis_labels_end_values_update_after_a_pan() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let before = strip_texts(&mut h, &mut app);
    let [_, vertical_before] = axes(&plane(&app));
    drag(&mut h, &mut app, egui::vec2(-40.0, 30.0));
    let after = strip_texts(&mut h, &mut app);
    let [horizontal, vertical] = axes(&plane(&app));
    assert_eq!(
        after,
        Vec::from(texts(&horizontal, "→")),
        "the strip reads the panned chart"
    );
    assert_ne!(after[0], before[0], "the low end moved");
    assert_ne!(after[2], before[2], "the high end moved");
    assert_eq!(after[1], before[1], "the name stays");
    // The picture follows the pointer: dragged left and down, the view moves right and up, and the values at the ends
    // rise along both axes.
    assert!(
        horizontal.low > -0.82 && horizontal.high > 1.18,
        "{horizontal:?}"
    );
    assert!(
        vertical.low > vertical_before.low && vertical.high > vertical_before.high,
        "{vertical:?}"
    );
    rejects("the strip before the pan", || {
        assert_eq!(before, Vec::from(texts(&horizontal, "→")));
    });
}

#[test]
fn mock_axis_labels_end_text_signs_and_decimals() {
    assert_eq!(end_text(0.0, 2.0), "+0.00", "zero reads plus");
    assert_eq!(end_text(-0.25, 2.0), "−0.25", "a true minus");
    assert_eq!(end_text(0.25, 1.0), "+0.25", "a span of 1: two decimals");
    assert_eq!(end_text(0.25, 0.1), "+0.250", "a tenth: three");
    assert_eq!(
        end_text(0.25, 0.05),
        "+0.2500",
        "between a hundredth and a tenth: four"
    );
    assert_eq!(
        end_text(0.25, -0.1),
        "+0.250",
        "the span's sign does not count"
    );
    assert_eq!(end_text(0.5, 1e-20), "+0.500000000000", "at most twelve");
    assert_eq!(end_text(0.5, 0.0), "+0.500000000000", "a zero span: twelve");
    assert_eq!(end_text(0.5, 100.0), "+0.50", "at least two");
    rejects("a minus on zero", || {
        assert_eq!(end_text(0.0, 2.0), "−0.00")
    });
}

/// The vertical axis's three texts as drawn: each with its bounds on the screen and its angle.
fn vertical_texts(h: &mut Headless, app: &mut App<MockSide>) -> Vec<(String, egui::Rect, f32)> {
    let output = h.frame(app, Vec::new());
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                egui::Shape::Text(text.clone()).visual_bounding_rect(),
                text.angle,
            )),
            _ => None,
        })
        .collect()
}

/// Asserts that the vertical axis's texts read `texts`, turned to read upwards, in `strip`: the low end at its bottom,
/// the name at its middle, the high end at its top, each 4 points in from its end.
fn check_vertical(drawn: &[(String, egui::Rect, f32)], texts: &[String; 3], strip: egui::Rect) {
    let find = |t: &str| {
        drawn
            .iter()
            .find(|(s, _, _)| s == t)
            .unwrap_or_else(|| panic!("no `{t}` drawn"))
    };
    let [low, name, high] = texts.each_ref().map(|t| find(t));
    for (text, rect, angle) in [low, name, high] {
        assert!(
            (angle + std::f32::consts::FRAC_PI_2).abs() < 1e-6,
            "{text} turned to read upwards: {angle}"
        );
        assert!(
            rect.height() > rect.width(),
            "{text} runs up the strip: {rect:?}"
        );
        assert!(
            strip.expand(1.0).contains_rect(*rect),
            "{text} inside the strip {strip:?}: {rect:?}"
        );
    }
    assert!(
        (low.1.max.y - (strip.max.y - 4.0)).abs() < 1.0,
        "the low end at the bottom: {:?}",
        low.1
    );
    assert!(
        (high.1.min.y - (strip.min.y + 4.0)).abs() < 1.0,
        "the high end at the top: {:?}",
        high.1
    );
    assert!(
        (name.1.center().y - strip.center().y).abs() < 1.0,
        "the name at the middle: {:?}",
        name.1
    );
    for (_, rect, _) in [low, name, high] {
        assert!(
            (rect.center().x - strip.center().x).abs() < 1.0,
            "centred across the strip: {rect:?}"
        );
    }
}

#[test]
fn mock_axis_labels_vertical_axis_reads_upwards_beside_the_figure() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let strip = Layout::new(h.screen(), PIXELS_PER_POINT).axis_y;
    let [_, vertical] = axes(&plane(&app));
    let want = texts(&vertical, "→");
    assert_eq!(want, ["−0.59", "z_β →", "+1.41"]);
    let drawn = vertical_texts(&mut h, &mut app);
    check_vertical(&drawn, &want, strip);
    let swapped = [want[2].clone(), want[1].clone(), want[0].clone()];
    rejects("the ends swapped", || {
        check_vertical(&drawn, &swapped, strip)
    });
}

#[test]
fn mock_axis_labels_decimals_follow_the_span() {
    let narrow = Axis {
        name: "z_α",
        low: 0.5,
        high: 0.501,
    };
    assert_eq!(
        texts(&narrow, "→"),
        ["+0.50000", "z_α →", "+0.50100"],
        "a span of a thousandth: five decimals"
    );
    rejects("the ends' ratio taken for their span", || {
        assert_eq!(texts(&narrow, "→")[0], "+0.50");
    });
}

#[test]
fn mock_axis_labels_start_at_01_mains_values() {
    assert_eq!(
        crate::mock::MOCK_Z0,
        [0.180, 0.410, 0.0, -0.227, 0.0, 0.312, 0.333, 0.333]
    );
    let mut app = mock_app();
    let mut h = headless();
    let texts = super::support::frame_texts(&mut h, &mut app);
    assert!(
        texts.iter().any(|t| t == "-0.227"),
        "z_q1 reads -0.227: {texts:?}"
    );
    rejects("z_q1 unsigned", || {
        assert!(texts.iter().any(|t| t == "0.227"))
    });
}
