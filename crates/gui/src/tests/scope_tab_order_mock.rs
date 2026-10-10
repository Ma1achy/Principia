//! `scope_tab_order_mock` (REQ-GUI-095; render_gui_spec §G3): on the mock, Tab and Shift+Tab cycle the seven big
//! scopes in the numbered order, 1 top bar · 2 Manifold view · 3 Figure · 4 Trajectory · 5 Compass · 6 Time ·
//! 7 Legend, and Enter on Manifold view enters Chart, its first sub-scope. Each check also runs on an order it must
//! reject (R-176, through `rejects`).

use eframe::egui::{Key, Modifiers};

use super::support::{focus, frame_texts, headless, mock_app, press, rejects};

/// The big scopes' ids in §G3's Tab order.
const ORDER: [&str; 7] = [
    "top_bar",
    "manifold_view",
    "figure",
    "trajectory",
    "compass",
    "time",
    "legend",
];

/// Their breadcrumbs.
const LABELS: [&str; 7] = [
    "Top bar",
    "Manifold view",
    "Figure",
    "Trajectory",
    "Compass",
    "Time",
    "Legend",
];

/// The focus paths and breadcrumbs `presses` of `key` with `modifiers` walk through, from no focus.
fn walk(key: Key, modifiers: Modifiers, presses: usize) -> Vec<(Vec<String>, bool)> {
    let mut app = mock_app();
    let mut headless = headless();
    (0..presses)
        .map(|_| {
            press(&mut headless, &mut app, key, modifiers);
            let path: Vec<String> = focus(&app).iter().map(|s| (*s).to_owned()).collect();
            let label = LABELS[ORDER.iter().position(|o| path == [*o]).unwrap_or(0)];
            let shown = frame_texts(&mut headless, &mut app).contains(&label.to_owned());
            (path, shown)
        })
        .collect()
}

/// `walked` visits `expected`'s scopes in order, one big scope at a time, each with its breadcrumb shown.
fn check_walk(walked: &[(Vec<String>, bool)], expected: &[&str]) {
    assert_eq!(walked.len(), expected.len());
    for ((path, shown), want) in walked.iter().zip(expected) {
        assert_eq!(path, &[want.to_string()], "the focus is not on `{want}`");
        assert!(shown, "no breadcrumb for `{want}`");
    }
}

#[test]
fn scope_tab_order_mock_tab_cycles_the_seven() {
    let walked = walk(Key::Tab, Modifiers::NONE, 8);
    let mut expected = ORDER.to_vec();
    expected.push("top_bar");
    check_walk(&walked, &expected);
    let mut swapped = expected.clone();
    swapped.swap(2, 3);
    rejects("Figure and Trajectory swapped", || {
        check_walk(&walked, &swapped)
    });
}

#[test]
fn scope_tab_order_mock_shift_tab_cycles_back() {
    let walked = walk(Key::Tab, Modifiers::SHIFT, 8);
    let mut expected: Vec<&str> = ORDER.iter().rev().copied().collect();
    expected.push("legend");
    check_walk(&walked, &expected);
    rejects("the forward order", || {
        let mut forward = ORDER.to_vec();
        forward.push("top_bar");
        check_walk(&walked, &forward)
    });
}

/// Enter on Manifold view enters Chart, its breadcrumb "Manifold view › Chart"; Tab from there goes on to Figure, and
/// Shift+Tab back to the top bar.
fn check_enter_chart(path: &[&str], texts: &[String], after_tab: &[&str], after_back: &[&str]) {
    assert_eq!(path, ["manifold_view", "chart"]);
    assert!(
        texts.iter().any(|t| t == "Manifold view › Chart"),
        "no breadcrumb `Manifold view › Chart` in {texts:?}"
    );
    assert_eq!(after_tab, ["figure"]);
    assert_eq!(after_back, ["top_bar"]);
}

#[test]
fn scope_tab_order_mock_enter_manifold_view_enters_chart() {
    let mut app = mock_app();
    let mut headless = headless();
    press(&mut headless, &mut app, Key::Tab, Modifiers::NONE);
    press(&mut headless, &mut app, Key::Tab, Modifiers::NONE);
    press(&mut headless, &mut app, Key::Enter, Modifiers::NONE);
    let path: Vec<String> = focus(&app).iter().map(|s| (*s).to_owned()).collect();
    let texts = frame_texts(&mut headless, &mut app);
    press(&mut headless, &mut app, Key::Tab, Modifiers::NONE);
    let after_tab: Vec<String> = focus(&app).iter().map(|s| (*s).to_owned()).collect();
    press(&mut headless, &mut app, Key::Enter, Modifiers::NONE);
    press(&mut headless, &mut app, Key::Tab, Modifiers::SHIFT);
    press(&mut headless, &mut app, Key::Tab, Modifiers::SHIFT);
    let after_back = focus(&app);
    fn as_str(v: &[String]) -> Vec<&str> {
        v.iter().map(String::as_str).collect()
    }
    check_enter_chart(&as_str(&path), &texts, &as_str(&after_tab), &after_back);
    rejects("Enter landing on Navigate", || {
        check_enter_chart(
            &["manifold_view", "navigate"],
            &texts,
            &as_str(&after_tab),
            &after_back,
        )
    });
}
