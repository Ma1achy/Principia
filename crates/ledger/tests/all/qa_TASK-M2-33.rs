//! QA tests for TASK-M2-33 (REQ-GUI-188; R-449), written from the requirement, R-449's table and payload §2's
//! "What a person reads", not from the implementation:
//!
//! - The one conversion function names every `state` code 0–7 and `detail` code 0–3 as R-449 and payload §2 do: body
//!   `k` "body `k + 1`", an escape of body `k` "escape body `k + 1`", a collision of pair `k` by its two bodies 1-based,
//!   smaller first, pair `k` being the side opposite body `k` (R-22), derived here from that rule and not read from the
//!   ledger's pair map; collision and escape `detail` 3 "triple collision" and "triple ejection"; bounded and running
//!   whatever `detail` holds; the failures with their payload §2 category; reserved states "reserved state 6" / "7"
//!   (`qa_m2_33_names_*`).
//! - The body and pair names it is built from: three bodies and three pairs, nothing past them (`qa_m2_33_bodies_*`).
//! - The class names (colour_composition §1.4): the black class "bounded / nothing yet" for bounded, running, the
//!   failures and the reserved codes, every other class its `(state, detail)` name (`qa_m2_33_class_names_*`).
//! - The negative arm: the `detail` and `dmin_pair` legends' keys, the k = 6 and ternary legends and every name the
//!   module gives contain neither "body 0" nor "pair " followed by a digit, checked by QA's own scan, not the
//!   module's predicate (`qa_m2_33_no_internal_id_*`); the module's predicate itself is checked against that scan
//!   (`qa_m2_33_predicate_*`).
//! - Inside stays 0-based: the ledger's hashed `detail` meanings keep "body 0" and "pair 0" (R-449 point 1)
//!   (`qa_m2_33_machine_data_*`).
//!
//! The expected strings are typed in from R-449's table. Each test has its negative control (R-176).
// The file name `qa_TASK-M2-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use ledger::display::{
    body_name, body_names, class_name_of, name_of, pair_name, pair_names, shows_an_internal_id,
};
use ledger::gen::catalogue::{body_channels, detail_segments, dmin_pair_legend};
use validation::negative_control;

// ── R-449's names, typed in ─────────────────────────────────────────────────────────────────────────────────────────

/// Payload §2's `state` codes.
const ESCAPE: u32 = 0;
const BOUNDED: u32 = 1;
const COLLISION: u32 = 2;
const RUNNING: u32 = 3;
const SIM_FAILED: u32 = 4;
const DECODE_FAILED: u32 = 5;

/// What a person reads for `(state, detail)`, from R-449's table and payload §2's `detail` failure enum.
fn r449(state: u32, detail: u32) -> String {
    match state {
        ESCAPE if detail < 3 => format!("escape body {}", detail + 1),
        ESCAPE => "triple ejection".to_owned(),
        COLLISION if detail < 3 => format!("collision {}", pair_shown(detail)),
        COLLISION => "triple collision".to_owned(),
        BOUNDED => "bounded".to_owned(),
        RUNNING => "running".to_owned(),
        _ => unreachable!(),
    }
}

/// Pair `k`'s two bodies, 1-based, smaller first, joined by an en dash: R-22's rule, pair `k` the side opposite body
/// `k`, so its bodies are the other two.
fn pair_shown(k: u32) -> String {
    let others: Vec<u32> = (0..3).filter(|&b| b != k).collect();
    format!("{}–{}", others[0] + 1, others[1] + 1)
}

/// The failure categories of payload §2, in code order: each name must say which failure and which category.
const SIM_FAILED_WORDS: [&str; 4] = ["nan", "inf", "derived", "reserved"];
const DECODE_FAILED_WORDS: [&str; 4] = ["non-finite", "degenerate", "mass", "other"];

/// Checks `f` against R-449 for every `state` 0–7 and `detail` 0–3, and that it names nothing past the fields' codes.
fn check_names(f: impl Fn(u32, u32) -> Option<String>) {
    // R-449's table, literally, before the rule: the derivation above could share a mistake with the code.
    for (state, detail, want) in [
        (COLLISION, 2, "collision 1–2"),
        (COLLISION, 1, "collision 1–3"),
        (COLLISION, 0, "collision 2–3"),
        (COLLISION, 3, "triple collision"),
        (ESCAPE, 0, "escape body 1"),
        (ESCAPE, 1, "escape body 2"),
        (ESCAPE, 2, "escape body 3"),
        (ESCAPE, 3, "triple ejection"),
    ] {
        assert_eq!(
            f(state, detail).as_deref(),
            Some(want),
            "R-449's table: state {state}, detail {detail}"
        );
    }
    for state in 0..8u32 {
        for detail in 0..4u32 {
            let got = f(state, detail).unwrap_or_else(|| panic!("no name for ({state}, {detail})"));
            match state {
                ESCAPE | COLLISION | BOUNDED | RUNNING => {
                    assert_eq!(got, r449(state, detail), "state {state}, detail {detail}")
                }
                SIM_FAILED | DECODE_FAILED => {
                    let (failure, word) = if state == SIM_FAILED {
                        ("sim_failed", SIM_FAILED_WORDS[detail as usize])
                    } else {
                        ("decode_failed", DECODE_FAILED_WORDS[detail as usize])
                    };
                    let l = got.to_lowercase();
                    assert!(
                        l.contains(failure) && l.contains(word),
                        "state {state}, detail {detail}: `{got}` is not {failure} with its category `{word}`"
                    );
                }
                _ => assert_eq!(
                    got,
                    format!("reserved state {state}"),
                    "state {state}, detail {detail}"
                ),
            }
        }
    }
    // The four categories of a failure are four names.
    for state in [SIM_FAILED, DECODE_FAILED] {
        let mut names: Vec<String> = (0..4).filter_map(|d| f(state, d)).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 4, "state {state}'s categories share a name");
    }
    assert_eq!(f(8, 0), None, "state 8 is past the 3-bit field");
    assert_eq!(f(0, 4), None, "detail 4 is past the 2-bit field");
}

#[test]
fn qa_m2_33_names_every_state_and_detail() {
    check_names(|s, d| name_of(s, d).map(str::to_owned));
}

negative_control!(
    qa_m2_33_names_every_state_and_detail,
    "a function that names a collision by the pair's own index's bodies (pair k = bodies k, k + 1) fails",
    expected = "R-449's table: state 2, detail 2",
    check_names(|s, d| {
        if s == COLLISION && d < 3 {
            let (a, b) = (d, (d + 1) % 3);
            Some(format!("collision {}–{}", a.min(b) + 1, a.max(b) + 1))
        } else {
            name_of(s, d).map(str::to_owned)
        }
    })
);

// ── The body and pair names ─────────────────────────────────────────────────────────────────────────────────────────

fn check_bodies_and_pairs(
    body: impl Fn(u32) -> Option<String>,
    pair: impl Fn(u32) -> Option<String>,
) {
    for k in 0..3u32 {
        assert_eq!(body(k), Some(format!("body {}", k + 1)), "body {k}'s name");
        assert_eq!(pair(k), Some(pair_shown(k)), "pair {k}'s name");
    }
    assert_eq!(body(3), None, "there is no fourth body");
    assert_eq!(pair(3), None, "`dmin_pair`'s 3 is unset, not a pair");
}

#[test]
fn qa_m2_33_bodies_and_pairs() {
    check_bodies_and_pairs(
        |k| body_name(k).map(str::to_owned),
        |k| pair_name(k).map(str::to_owned),
    );
    // The arrays agree with the lookups, in index order.
    assert_eq!(body_names(), ["body 1", "body 2", "body 3"]);
    assert_eq!(pair_names(), ["2–3", "1–3", "1–2"]);
}

negative_control!(
    qa_m2_33_bodies_and_pairs,
    "a body named 0-based fails",
    expected = "body 0's name",
    check_bodies_and_pairs(
        |k| (k < 3).then(|| format!("body {k}")),
        |k| pair_name(k).map(str::to_owned),
    )
);

// ── The class names ─────────────────────────────────────────────────────────────────────────────────────────────────

fn check_class_names(f: impl Fn(u32, u32) -> Option<String>) {
    for state in 0..8u32 {
        for detail in 0..4u32 {
            let want = match state {
                ESCAPE | COLLISION => r449(state, detail),
                _ => "bounded / nothing yet".to_owned(),
            };
            assert_eq!(
                f(state, detail),
                Some(want),
                "the class of state {state}, detail {detail}"
            );
        }
    }
    // The nine classes of colour_composition §1.4.
    let mut classes: Vec<String> = (0..8)
        .flat_map(|s| (0..4).map(move |d| (s, d)))
        .filter_map(|(s, d)| f(s, d))
        .collect();
    classes.sort();
    classes.dedup();
    assert_eq!(classes.len(), 9, "the outcome classes: {classes:?}");
}

#[test]
fn qa_m2_33_class_names() {
    check_class_names(|s, d| class_name_of(s, d).map(str::to_owned));
}

negative_control!(
    qa_m2_33_class_names,
    "a running sample's class named \"running\" rather than the black class fails",
    expected = "the class of state 3",
    check_class_names(|s, d| {
        if s == RUNNING {
            Some("running".to_owned())
        } else {
            class_name_of(s, d).map(str::to_owned)
        }
    })
);

// ── The negative arm ────────────────────────────────────────────────────────────────────────────────────────────────

/// QA's own scan for an internal id where a person reads: "body 0", or "pair " followed by a digit, in any case.
fn internal_id(text: &str) -> bool {
    let l = text.to_lowercase();
    let chars: Vec<char> = l.chars().collect();
    let pair: Vec<char> = "pair ".chars().collect();
    l.contains("body 0")
        || (0..chars.len()).any(|i| {
            chars[i..].starts_with(&pair)
                && chars
                    .get(i + pair.len())
                    .is_some_and(|c| c.is_ascii_digit())
        })
}

/// Every key the module's names reach a person through, in this crate: every name and class name, the `detail`
/// legend's keys, the `dmin_pair` legend's keys and the body-channel legend of the ternary and k = 6 views.
fn shown_keys() -> Vec<String> {
    let mut keys = Vec::new();
    for s in 0..8 {
        for d in 0..4 {
            keys.push(name_of(s, d).unwrap().to_owned());
            keys.push(class_name_of(s, d).unwrap().to_owned());
        }
    }
    for segment in detail_segments() {
        for class in segment.classes {
            keys.push(class.label);
        }
    }
    keys.extend(dmin_pair_legend().into_iter().map(|(_, k)| k.to_owned()));
    keys.push(body_channels());
    keys.extend(body_names().map(str::to_owned));
    keys.extend(pair_names().map(str::to_owned));
    keys
}

fn check_no_internal_id(keys: &[String]) {
    for k in keys {
        assert!(!internal_id(k), "the key `{k}` shows an internal id");
    }
}

#[test]
fn qa_m2_33_no_internal_id_in_any_key() {
    let keys = shown_keys();
    assert!(keys.len() > 70, "too few keys gathered: {}", keys.len());
    check_no_internal_id(&keys);
}

negative_control!(
    qa_m2_33_no_internal_id_in_any_key,
    "a legend key naming a pair by its id fails",
    expected = "shows an internal id",
    {
        let mut keys = shown_keys();
        keys.push("Pair 2".to_owned());
        check_no_internal_id(&keys);
    }
);

/// The `detail` legend's keys are the function's names, segment by segment, and the `dmin_pair` legend's keys are the
/// pairs' names by pair id; the body-channel legend reads bodies 1, 2, 3 in red, green, blue.
fn check_legends(detail: &[(u32, u32, String)], dmin: &[(u32, String)], channels: &str) {
    for (state, d, label) in detail {
        if *state == ESCAPE || *state == COLLISION {
            assert_eq!(
                *label,
                r449(*state, *d),
                "the detail legend's ({state}, {d}) key"
            );
        } else {
            assert_eq!(
                Some(label.as_str()),
                name_of(*state, *d),
                "the detail legend's ({state}, {d}) key"
            );
        }
    }
    let states: Vec<u32> = detail.iter().map(|(s, _, _)| *s).collect();
    for s in [ESCAPE, COLLISION, SIM_FAILED, DECODE_FAILED] {
        assert_eq!(
            states.iter().filter(|&&x| x == s).count(),
            4,
            "the detail legend's keys of state {s}"
        );
    }
    let want: Vec<(u32, String)> = (0..3).map(|k| (k, pair_shown(k))).collect();
    assert_eq!(dmin, want.as_slice(), "the dmin_pair legend");
    let (r, g, b) = (
        channels.find("red body 1"),
        channels.find("green body 2"),
        channels.find("blue body 3"),
    );
    assert!(
        r.is_some() && g.is_some() && b.is_some() && r < g && g < b,
        "the body-channel legend `{channels}`"
    );
}

/// The `detail` legend's `(state, detail, key)`s, the `dmin_pair` legend's `(pair id, key)`s and the body-channel
/// legend.
type Legends = (Vec<(u32, u32, String)>, Vec<(u32, String)>, String);

fn legends() -> Legends {
    let detail = detail_segments()
        .into_iter()
        .flat_map(|s| {
            let code = s.code;
            s.classes
                .into_iter()
                .map(move |c| (code, c.detail, c.label))
        })
        .collect();
    let dmin = dmin_pair_legend()
        .into_iter()
        .map(|(k, n)| (k, n.to_owned()))
        .collect();
    (detail, dmin, body_channels())
}

#[test]
fn qa_m2_33_no_internal_id_legends_read_the_names() {
    let (detail, dmin, channels) = legends();
    check_legends(&detail, &dmin, &channels);
}

negative_control!(
    qa_m2_33_no_internal_id_legends_read_the_names,
    "a dmin_pair legend keyed 0-based (pair 0 = bodies 1–2 as \"1–2\") fails",
    expected = "the dmin_pair legend",
    {
        let (detail, mut dmin, channels) = legends();
        dmin[0].1 = "1–2".to_owned();
        check_legends(&detail, &dmin, &channels);
    }
);

/// The module's predicate agrees with QA's scan on the forms the negative arm names, and on near misses.
fn check_predicate(f: impl Fn(&str) -> bool) {
    for text in [
        "escape body 0",
        "body 0",
        "pair 0",
        "pair 2: bodies 0–1",
        "collision (pair 1)",
        "escape body 1 · body 0 · t 3",
    ] {
        assert!(internal_id(text), "QA's scan misses `{text}`");
        assert!(f(text), "the predicate passes `{text}`");
    }
    for text in [
        "escape body 1",
        "collision 1–2",
        "triple collision",
        "bounded / nothing yet",
        "a pair named by its bodies",
    ] {
        assert!(!internal_id(text), "QA's scan flags `{text}`");
        assert!(!f(text), "the predicate flags `{text}`");
    }
}

#[test]
fn qa_m2_33_predicate_agrees() {
    check_predicate(shows_an_internal_id);
}

negative_control!(
    qa_m2_33_predicate_agrees,
    "a predicate that only looks for \"body 0\" fails",
    expected = "the predicate passes `pair 0`",
    check_predicate(|t| t.contains("body 0"))
);

// ── Inside stays 0-based ────────────────────────────────────────────────────────────────────────────────────────────

/// The ledger's `detail` meanings, machine data hashed into the schema version, keep the internal ids (R-449 point 1):
/// the escape meanings name body 0 to 2 and the collision meanings pair 0 to 2.
fn check_machine_data(meanings: &[(&str, [&str; 4])]) {
    let of = |state: &str| {
        meanings
            .iter()
            .find(|(s, _)| *s == state)
            .unwrap_or_else(|| panic!("no `{state}` meanings"))
            .1
    };
    for k in 0..3 {
        assert!(
            of("escape")[k].contains(&format!("body {k}")),
            "escape detail {k}'s meaning is not body {k}, 0-based: {:?}",
            of("escape")[k]
        );
        assert!(
            of("collision")[k].contains(&format!("pair {k}")),
            "collision detail {k}'s meaning is not pair {k}: {:?}",
            of("collision")[k]
        );
    }
}

#[test]
fn qa_m2_33_machine_data_stays_0_based() {
    check_machine_data(&ledger::payload::detail_meanings());
}

negative_control!(
    qa_m2_33_machine_data_stays_0_based,
    "machine data rewritten 1-based fails",
    expected = "escape detail 0's meaning",
    {
        let mut m = ledger::payload::detail_meanings();
        for e in m.iter_mut() {
            if e.0 == "escape" {
                e.1 = ["body 1", "body 2", "body 3", "all three"];
            }
        }
        check_machine_data(&m);
    }
);
