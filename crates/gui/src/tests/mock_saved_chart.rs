//! `mock_saved_chart`: the capture of a saved chart, the `01_main` case of that name.
//!
//! **The criterion**, written before the first run: the case's steps, read from `fixtures/screenshot/01_main/cases.json`,
//! show the picker reading "Saved 1" with its rename and delete, and the header its provenance, each inside the
//! Manifold view's margin; on the least window, the picker's row and the Save view as chart button inside it too. Controls:
//! the steps without the last Enter show none of them; a margin of twice the panel's.

use super::f3_fills_window_mock::canvas;
use super::save_view_as_chart::{frames, save};
use super::support::{mock_app, rejects};
use crate::capture::{shoot, Shot, Step, PIXELS_PER_POINT, SIZE};
use crate::explore::manifold_view::{saved, MARGIN, PRESET};
use crate::headless::Headless;
use crate::layout::{Layout, LEAST_WINDOW};

/// The case's steps.
fn steps() -> Vec<Step> {
    let text = include_str!("../../../../fixtures/screenshot/01_main/cases.json");
    let all: serde_json::Value = serde_json::from_str(text).expect("JSON");
    let case = all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"] == "mock_saved_chart")
        .expect("the case");
    case["surface"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| Step::parse(s.as_str().expect("a name")).expect("a step"))
        .collect()
}

#[test]
fn mock_saved_chart_shows_the_chart() {
    let canvas = canvas();
    let window = eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::vec2(SIZE[0] as f32, SIZE[1] as f32) / PIXELS_PER_POINT,
    );
    let panel = Layout::new(window, PIXELS_PER_POINT).manifold_view;
    let check = |shot: &Shot| {
        for text in ["Saved 1", "rename", "delete", "from z_α × z_β"] {
            let name = shot
                .names
                .iter()
                .find(|n| n.name == text || (text.len() > 8 && n.name.starts_with(text)))
                .unwrap_or_else(|| panic!("no {text:?}"));
            let r = name.rect.expect("drawn");
            let inside = r[0] >= f64::from(panel.min.x * PIXELS_PER_POINT)
                && r[2] <= f64::from((panel.max.x - MARGIN) * PIXELS_PER_POINT);
            assert!(inside, "{text:?} at {r:?}");
        }
    };
    let steps = steps();
    check(&shoot(canvas.clone(), &steps));
    let short = shoot(canvas, &steps[..steps.len() - 1]);
    rejects("the steps without the last Enter", || check(&short));
}

#[test]
fn mock_saved_chart_fits_the_least_window() {
    let size = LEAST_WINDOW.map(|v| (v * PIXELS_PER_POINT) as u32);
    let mut h = Headless::new(size, PIXELS_PER_POINT);
    let mut app = mock_app();
    frames(&mut h, &mut app, 3);
    save(&mut h, &mut app);
    let panel = Layout::new(h.screen(), PIXELS_PER_POINT).manifold_view;
    let fits = |margin: f32| {
        for id in [PRESET, saved::RENAME, saved::DELETE, saved::SAVE] {
            let r = app.keyboard.place_of(id).expect("drawn").rect;
            assert!(
                r.min.x >= panel.min.x + margin - 0.5 && r.max.x <= panel.max.x - margin + 0.5,
                "{id} at {r:?} in {panel:?}"
            );
        }
    };
    fits(MARGIN);
    rejects("a margin of twice the panel's", || fits(2.0 * MARGIN));
}
