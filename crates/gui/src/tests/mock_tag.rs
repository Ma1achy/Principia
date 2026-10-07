//! `mock_tag` (REQ-GUI-167; R-390, RQ-247, RQ-248): the app runs headless once on the mock and once on the real
//! engine's data contract with no figure, and the "mock engine" tag is in the AccessKit names exactly when the engine
//! is the mock, which the engine-side adapter says, not a snapshot field. The window title carries it too.

use super::support::{frame_texts, headless, mock_app, real_app, rejects};
use crate::app::window_title;
use crate::explore::footer::MOCK_TAG;
use crate::side::EngineSide;

/// The tag is among `texts` exactly when `is_mock`.
fn check_tag(texts: &[String], is_mock: bool) {
    let shown = texts.iter().any(|t| t == MOCK_TAG);
    assert_eq!(
        shown, is_mock,
        "the \"{MOCK_TAG}\" tag: shown {shown}, but the engine is the mock: {is_mock}"
    );
}

#[test]
fn mock_tag_shown_exactly_on_the_mock() {
    let mut mock = mock_app();
    let mock_texts = frame_texts(&mut headless(), &mut mock);
    assert!(mock.side().is_mock());
    check_tag(&mock_texts, true);
    let mut real = real_app();
    let real_texts = frame_texts(&mut headless(), &mut real);
    assert!(!real.side().is_mock());
    check_tag(&real_texts, false);
    // The rest of the footer is there on both.
    for texts in [&mock_texts, &real_texts] {
        assert!(texts.iter().any(|t| t == "? keys"), "{texts:?}");
    }
    rejects("the real engine's names read as the mock's", || {
        check_tag(&real_texts, true)
    });
    rejects("the mock's names read as the real engine's", || {
        check_tag(&mock_texts, false)
    });
}

#[test]
fn mock_tag_in_the_window_title() {
    assert_eq!(window_title(true), "principia · dev — mock engine");
    assert_eq!(window_title(false), "principia · dev");
}
