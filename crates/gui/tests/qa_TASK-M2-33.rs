//! QA tests for TASK-M2-33 on the GUI side (REQ-GUI-188; R-449; render_gui_spec §G2, §G6, §G8), written from the
//! requirement, not from the implementation. The names are typed in from R-449's table:
//!
//! - The mock legend's outcome key reads the nine classes as a person reads them, in colour_composition §1.4's order,
//!   the black key "bounded / nothing yet", and, since R-467 (TASK-M6-46), no "forbidden region" key, which left the
//!   presentation legends; a running class's key reads "running" (`qa_m2_33_legend_keys_*`).
//! - Every fate the mock decides, over a grid of the chart at the horizon, is one of the nine outcomes' names (a sample
//!   is "bounded", not the black class's legend name), all nine are reached, and the hover label, the Trajectory
//!   panel's summary and a kept orbit's row each carry it and show no internal id (`qa_m2_33_fates_*`).
//! - The app as drawn, with the playhead at the horizon, the pointer on the figure, an orbit pinned and the Inspector
//!   open on it: no text the frame paints shows "body 0" or "pair " and a digit, the hover label shows its decided
//!   fate, and the Inspector labels the three bodies 1, 2, 3 (§G8 pane 1) (`qa_m2_33_drawn_*`).
//!
//! gui takes no dependency on validation (R-187), so each check runs its own control (R-176) through [`rejects`].
// The file name `qa_TASK-M2-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event};
use engine::contract::legend::LegendKey;
use engine::contract::render_state::Playhead;
use engine::contract::sample::Sample;
use engine::contract::set_field::{Edit, RenderField, SetField};
use engine::contract::view_ui::Window;
use gui::app::App;
use gui::capture::{hover_point, FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::{hover_label, trajectory};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::legend::{default_stain, evaluate, Class, Dimension, FakeStain};
use gui::mock::sample::sample;
use gui::mock::{MockEngine, MOCK_T_MAX};
use gui::side::MockSide;

/// The outcome key's labels, R-449's names in colour_composition §1.4's order, and no forbidden-region key (§G6;
/// R-467: "The legend's 'forbidden region' key and its share leave the presentation legends").
const KEYS: [&str; 9] = [
    "bounded / nothing yet",
    "collision 1–2",
    "collision 1–3",
    "collision 2–3",
    "triple collision",
    "escape body 1",
    "escape body 2",
    "escape body 3",
    "triple ejection",
];

/// A decided sample's fate, R-449's name of its `state` and `detail` (payload §2 "What a person reads").
const FATES: [&str; 9] = [
    "bounded",
    "collision 1–2",
    "collision 1–3",
    "collision 2–3",
    "triple collision",
    "escape body 1",
    "escape body 2",
    "escape body 3",
    "triple ejection",
];

fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

/// QA's scan for an internal id where a person reads: "body 0", or "pair " followed by a digit, in any case.
fn internal_id(text: &str) -> bool {
    let l = text.to_lowercase();
    l.contains("body 0")
        || l.match_indices("pair ")
            .any(|(i, _)| l[i + 5..].starts_with(|c: char| c.is_ascii_digit()))
}

// ── The legend's keys ───────────────────────────────────────────────────────────────────────────────────────────────

fn outcome_labels(stain: &FakeStain) -> Vec<String> {
    match &evaluate(stain).keys[0] {
        LegendKey::Categorical { entries, .. } => entries.iter().map(|e| e.label.clone()).collect(),
        other => panic!("the first key is not categorical: {other:?}"),
    }
}

fn check_keys(labels: &[String]) {
    assert_eq!(
        labels,
        KEYS.map(str::to_owned).as_slice(),
        "the outcome key's labels"
    );
    for l in labels {
        assert!(!internal_id(l), "the key `{l}` shows an internal id");
    }
}

#[test]
fn qa_m2_33_legend_keys_read_the_shown_names() {
    let labels = outcome_labels(&default_stain());
    check_keys(&labels);
    // A running class, where a field holds no value for it, is keyed by its state's name.
    let running = FakeStain {
        out: vec![Dimension::Categorical {
            title: "colour · outcome class",
            classes: vec![(Class::Swatch("escape_body_0"), 1.0), (Class::Running, 1.0)],
        }],
    };
    assert_eq!(
        outcome_labels(&running),
        ["escape body 1", "running"],
        "a running class's key"
    );
    rejects(
        "a legend that keeps the forbidden region key (R-467)",
        || {
            let mut labels = labels.clone();
            labels.push("forbidden region".to_owned());
            check_keys(&labels)
        },
    );
    rejects("a key written 0-based", || {
        let mut labels = labels.clone();
        labels[5] = "escape body 0".to_owned();
        check_keys(&labels)
    });
    rejects("a key naming its pair by id, in the right place", || {
        let mut labels = labels.clone();
        labels[1] = "collision 1–2 (pair 2)".to_owned();
        for l in &labels {
            assert!(!internal_id(l), "the key `{l}` shows an internal id");
        }
    });
}

// ── The fates the mock decides, and the labels that carry them ──────────────────────────────────────────────────────

/// The decided samples of a 24 × 24 grid of the default chart at the horizon.
fn decided() -> Vec<Sample> {
    let app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let plane = app.snapshot().sim.plane.clone();
    (0..24)
        .flat_map(|i| (0..24).map(move |j| [(i as f64 + 0.5) / 24.0, (j as f64 + 0.5) / 24.0]))
        .map(|at| sample(&plane, at, MOCK_T_MAX, MOCK_T_MAX))
        .collect()
}

fn check_fates(samples: &[Sample]) {
    let mut seen = Vec::new();
    for s in samples {
        let fate = s
            .fate
            .as_deref()
            .expect("at the horizon every sample is decided");
        assert!(
            FATES.contains(&fate),
            "the fate `{fate}` is not an outcome's name"
        );
        if !seen.contains(&fate) {
            seen.push(fate);
        }
        for (what, label) in [
            ("hover label", hover_label::text(s, MOCK_T_MAX, MOCK_T_MAX)),
            ("Trajectory summary", trajectory::summary(s)),
            ("kept orbit's row", trajectory::kept_row(1, s)),
        ] {
            assert!(
                label.starts_with(fate) || label.contains(&format!(" {fate} ")),
                "the {what} `{label}` does not carry the fate `{fate}`"
            );
            assert!(
                !internal_id(&label),
                "the {what} `{label}` shows an internal id"
            );
        }
    }
    assert_eq!(seen.len(), FATES.len(), "the fates reached: {seen:?}");
}

#[test]
fn qa_m2_33_fates_are_the_shown_names() {
    let samples = decided();
    check_fates(&samples);
    rejects("a sample whose fate is written 0-based", || {
        let mut samples = samples.clone();
        let i = samples
            .iter()
            .position(|s| s.fate.as_deref() == Some("escape body 1"))
            .unwrap();
        samples[i].fate = Some("escape body 0".to_owned());
        check_fates(&samples)
    });
    rejects("a sample whose fate is its legend class's name", || {
        let mut samples = samples.clone();
        for s in &mut samples {
            if s.fate.as_deref() == Some("bounded") {
                s.fate = Some("bounded / nothing yet".to_owned());
            }
        }
        check_fates(&samples)
    });
}

// ── The app as drawn ────────────────────────────────────────────────────────────────────────────────────────────────

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

/// The texts one frame paints, with their rects.
fn painted(h: &mut Headless, app: &mut App<MockSide>) -> Vec<(String, egui::Rect)> {
    let out = h.frame(app, Vec::new());
    let mut flat = Vec::new();
    for c in &out.shapes {
        flatten(&c.shape, &mut flat);
    }
    flat.into_iter()
        .filter_map(|s| match &s {
            egui::Shape::Text(t) => Some((t.galley.text().to_owned(), s.visual_bounding_rect())),
            _ => None,
        })
        .collect()
}

/// The app at the horizon with the pointer on the figure, the orbit under it pinned and the Inspector open on it: the
/// texts the frame paints, those inside the Inspector, and the sample under the pointer.
fn drawn() -> (Vec<String>, Vec<String>, Sample) {
    let mut app = App::new(MockSide::new(MockEngine::frozen(), None), FORMAT);
    let mut h = Headless::new(SIZE, PIXELS_PER_POINT);
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(SetField {
        edit: Edit::Render(RenderField::Playhead(Playhead { t: MOCK_T_MAX })),
        no_history: false,
    });
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let at = hover_point(layout.figure);
    for _ in 0..4 {
        let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    }
    let sample = app
        .snapshot()
        .sample
        .clone()
        .expect("a sample under the pointer");
    app.pin_at(sample.at);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(at)]);
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    let mut texts = Vec::new();
    for _ in 0..4 {
        texts = painted(&mut h, &mut app);
    }
    assert!(app.is_open(Window::Inspector), "the Inspector did not open");
    let rect = app
        .keyboard
        .place_of(gui::windows::entry(Window::Inspector).scope)
        .expect("the Inspector placed")
        .rect;
    let inside = texts
        .iter()
        .filter(|(_, r)| rect.expand(1.0).contains_rect(*r))
        .map(|(t, _)| t.clone())
        .collect();
    (texts.into_iter().map(|(t, _)| t).collect(), inside, sample)
}

fn check_drawn(all: &[String], inspector: &[String], sample: &Sample) {
    for t in all {
        assert!(!internal_id(t), "the frame paints `{t}`, an internal id");
    }
    let fate = sample
        .fate
        .as_deref()
        .expect("the sample is decided at the horizon");
    assert!(FATES.contains(&fate), "the fate `{fate}`");
    assert!(
        all.iter()
            .any(|t| t.starts_with(&format!("{fate} · t_end"))),
        "no hover label with the fate `{fate}` in {all:?}"
    );
    // §G8 pane 1: the three bodies labelled 1, 2, 3, and no body labelled 0.
    let digits: Vec<&str> = inspector
        .iter()
        .map(String::as_str)
        .filter(|t| t.len() == 1 && t.chars().all(|c| c.is_ascii_digit()))
        .collect();
    for b in ["1", "2", "3"] {
        assert!(
            digits.contains(&b),
            "the Inspector labels no body {b}: its one-digit labels are {digits:?}"
        );
    }
    assert!(
        !digits.contains(&"0"),
        "the Inspector labels a body 0: its one-digit labels are {digits:?}"
    );
}

#[test]
fn qa_m2_33_drawn_frame_shows_no_internal_id() {
    let (all, inspector, sample) = drawn();
    check_drawn(&all, &inspector, &sample);
    rejects("an Inspector labelling its bodies 0, 1, 2", || {
        let inspector: Vec<String> = inspector
            .iter()
            .filter(|t| t.as_str() != "3")
            .cloned()
            .chain(["0".to_owned()])
            .collect();
        check_drawn(&all, &inspector, &sample)
    });
    rejects("a frame painting a pair id", || {
        let mut all = all.clone();
        all.push("dmin_pair: pair 2".to_owned());
        check_drawn(&all, &inspector, &sample)
    });
}
