//! QA tests for TASK-M2-33 on the render side (REQ-GUI-188; R-449; colour_composition §1.4), written from the
//! requirement, not from the implementation: every swatch label of the built-in outcome occupant, both swatch sets,
//! is its class's name as a person reads it, keyed by the class's machine id (the param, which stays 0-based:
//! `escape_body_0` is "escape body 1", `collision_pair_2`, bodies 0 and 1 by R-22, is "collision 1–2"); a sample's
//! class, through the occupant's own `class(state, detail)`, carries the label R-449 names for that `state` and
//! `detail`; and no label shows "body 0" or "pair " and a digit. The names are typed in from R-449's table. Each test
//! has its negative control (R-176).
// The file name `qa_TASK-M2-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use render::colour::outcome::{class, Swatch, OKLAB_UNIFORM, SWATCHES};
use validation::negative_control;

/// R-449's display name of each class, by the param's machine id (colour_composition §1.4; R-22: pair `k` is the side
/// opposite body `k`).
const BY_PARAM: [(&str, &str); 9] = [
    ("bounded", "bounded / nothing yet"),
    ("collision_pair_2", "collision 1–2"),
    ("collision_pair_1", "collision 1–3"),
    ("collision_pair_0", "collision 2–3"),
    ("triple_collision", "triple collision"),
    ("escape_body_0", "escape body 1"),
    ("escape_body_1", "escape body 2"),
    ("escape_body_2", "escape body 3"),
    ("triple_ejection", "triple ejection"),
];

/// R-449's name of the class of a sample with `state` and `detail` (payload §2: 0 escape, 1 bounded, 2 collision,
/// 3 running, 4 sim_failed, 5 decode_failed, 6–7 reserved), the black class for every state without a body or pair.
fn by_state(state: u32, detail: u32) -> &'static str {
    match (state, detail) {
        (0, 0) => "escape body 1",
        (0, 1) => "escape body 2",
        (0, 2) => "escape body 3",
        (0, _) => "triple ejection",
        (2, 0) => "collision 2–3",
        (2, 1) => "collision 1–3",
        (2, 2) => "collision 1–2",
        (2, _) => "triple collision",
        _ => "bounded / nothing yet",
    }
}

fn shows_internal_id(text: &str) -> bool {
    let l = text.to_lowercase();
    l.contains("body 0")
        || l.match_indices("pair ")
            .any(|(i, _)| l[i + 5..].starts_with(|c: char| c.is_ascii_digit()))
}

fn check_swatches(sets: &[(&str, [Swatch; 9])]) {
    for (set, swatches) in sets {
        for (want_param, want_label) in BY_PARAM {
            let s = swatches
                .iter()
                .find(|s| s.param.trim_start_matches("oklab_") == want_param)
                .unwrap_or_else(|| panic!("{set}: no swatch `{want_param}`"));
            assert_eq!(
                s.label, want_label,
                "{set}: the swatch `{}`'s label",
                s.param
            );
            assert!(
                !shows_internal_id(s.label),
                "{set}: the swatch `{}`'s label `{}` shows an internal id",
                s.param,
                s.label
            );
        }
        for state in 0..8 {
            for detail in 0..4 {
                let got = swatches[class(state, detail)].label;
                assert_eq!(
                    got,
                    by_state(state, detail),
                    "{set}: the class of state {state}, detail {detail} is labelled `{got}`"
                );
            }
        }
    }
}

#[test]
fn qa_m2_33_swatch_labels_are_the_shown_names() {
    check_swatches(&[("sRGB", SWATCHES), ("OKLab-uniform", OKLAB_UNIFORM)]);
}

negative_control!(
    qa_m2_33_swatch_labels_are_the_shown_names,
    "a variant set whose collision swatches are labelled by pair id fails",
    expected = "OKLab-uniform: the swatch `oklab_collision_pair_2`'s label",
    {
        let mut oklab = OKLAB_UNIFORM;
        for s in oklab.iter_mut() {
            if s.param == "oklab_collision_pair_2" {
                s.label = "collision pair 2";
            }
        }
        check_swatches(&[("sRGB", SWATCHES), ("OKLab-uniform", oklab)]);
    }
);
