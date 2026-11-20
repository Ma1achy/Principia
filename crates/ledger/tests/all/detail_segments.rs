//! The `detail` view's segments and legend, in the ledger (`ledger::gen::catalogue::detail_segments`; REQ-COL-004,
//! REQ-TOOL-022; dd_colouring §3.7): one segment per state in which payload §2 gives `detail` a meaning, at that
//! state's code, its classes `4k + 1 … 4k + 4`; none for bounded, running and the reserved codes 6–7. The render
//! crate's `detail_legend_per_state` checks the legend's labels and the view's pixels against it; this checks the
//! segments' keys in the ledger's own tests. Each test has a registered negative control (R-176).

use ledger::gen::catalogue::{detail_classes, detail_segment, detail_segments, no_detail};
use validation::negative_control;

/// Payload §2's states with a `detail` meaning, by code (escape 0, collision 2, sim_failed 4, decode_failed 5), in
/// the ledger's order, and each one's first class.
const SEGMENTS: [(&str, u32, u32); 4] = [
    ("escape", 0, 1),
    ("collision", 2, 5),
    ("sim_failed", 4, 9),
    ("decode_failed", 5, 13),
];

/// A segment as the checks compare it: its state's name, its code, and each class's `(detail, class)`.
type Keyed = (&'static str, u32, Vec<(u32, u32)>);

/// Checks that `segment` keys each state code 0–7 as `want` does: its state's name, code and four classes from its
/// first, the detail codes 0–3 in order, or none.
fn check_segments(segment: impl Fn(u32) -> Option<Keyed>) {
    for code in 0..8 {
        let want = SEGMENTS
            .iter()
            .find(|(_, c, _)| *c == code)
            .map(|&(state, c, first)| (state, c, (0..4).map(|d| (d, first + d)).collect()));
        assert_eq!(segment(code), want, "state code {code}'s detail segment");
    }
}

/// The ledger's segment of state `code`, as (state, code, [(detail, class)]).
fn ledger_segment(code: u32) -> Option<Keyed> {
    detail_segment(code).map(|s| {
        (
            s.state,
            s.code,
            s.classes.iter().map(|c| (c.detail, c.class)).collect(),
        )
    })
}

#[test]
fn detail_segments_are_keyed_by_state_code() {
    check_segments(ledger_segment);
    let all: Vec<&str> = detail_segments().iter().map(|s| s.state).collect();
    assert_eq!(all, SEGMENTS.map(|(s, _, _)| s), "the segments' order");
    assert_eq!(no_detail(), 0, "the blank class");
    assert_eq!(
        detail_classes(),
        17,
        "the blank class and four segments of four"
    );
}

negative_control!(
    detail_segments_are_keyed_by_state_code,
    "the segments looked up by their index, not their state's code",
    expected = "state code 1's detail segment",
    check_segments(|code| {
        detail_segments().get(code as usize).map(|s| {
            (
                s.state,
                s.code,
                s.classes.iter().map(|c| (c.detail, c.class)).collect(),
            )
        })
    })
);
