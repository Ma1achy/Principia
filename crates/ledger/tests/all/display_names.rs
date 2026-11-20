//! The display names (`ledger::display`; R-449; REQ-GUI-188): the one conversion function of `state` and `detail` and
//! the body and pair names it is built from. For every `state` code 0–7 and `detail` code 0–3 the function returns
//! R-449's name; the pair names agree with the ledger's pair map (`pair_bodies`); the failure categories with the
//! ledger's `detail_meanings`; the outcome classes' names are the black class's "bounded / nothing yet" and the
//! function's for the other eight. The negative arm: every outcome-legend key, every name the function gives and the
//! `detail` and `dmin_pair` legends' keys show no internal id, neither "body 0" nor "pair " and a digit. The swatch
//! labels' half is the render crate's (`display_names_swatch_labels_*`, which `ledger`, the build DAG's root, cannot
//! reach), the GUI's the gui crate's. Each test has a registered negative control (R-176).

use ledger::display::{
    body_name, body_names, body_number, class_name, class_name_of, name, name_of, pair_name,
    pair_names, shows_an_internal_id, Detail, State, BLACK_CLASS,
};
use ledger::gen::catalogue::{body_channels, detail_segments, dmin_pair_legend};
use ledger::payload::{detail_meanings, pair_bodies, states};
use validation::negative_control;

/// R-449's names (payload §2, "What a person reads"), by `state` code, each state's four `detail` codes in order.
const NAMES: [[&str; 4]; 8] = [
    [
        "escape body 1",
        "escape body 2",
        "escape body 3",
        "triple ejection",
    ],
    ["bounded"; 4],
    [
        "collision 2–3",
        "collision 1–3",
        "collision 1–2",
        "triple collision",
    ],
    ["running"; 4],
    [
        "sim_failed: NaN in state",
        "sim_failed: Inf/overflow in state",
        "sim_failed: non-finite derived quantity",
        "sim_failed: reserved",
    ],
    [
        "decode_failed: non-finite decode output",
        "decode_failed: degenerate configuration",
        "decode_failed: invalid mass construction",
        "decode_failed: other/reserved",
    ],
    ["reserved state 6"; 4],
    ["reserved state 7"; 4],
];

/// Checks that `f` names every `(state, detail)` code as [`NAMES`] does.
fn check_names(f: impl Fn(u32, u32) -> Option<&'static str>) {
    for (state, row) in NAMES.iter().enumerate() {
        for (detail, want) in row.iter().enumerate() {
            assert_eq!(
                f(state as u32, detail as u32),
                Some(*want),
                "state {state}, detail {detail}"
            );
        }
    }
}

#[test]
fn display_names_every_state_and_detail() {
    check_names(name_of);
    check_names(|s, d| {
        Some(name(
            State::from_code(s).unwrap(),
            Detail::from_code(d).unwrap(),
        ))
    });
    // Past each field's codes there is no name: 8 is no state, 4 no detail.
    assert_eq!(name_of(8, 0), None);
    assert_eq!(name_of(0, 4), None);
    assert_eq!(name_of(7, 3), Some("reserved state 7"));
}

negative_control!(
    display_names_every_state_and_detail,
    "a function naming escapes by their 0-based body fails",
    expected = "state 0, detail 0",
    check_names(|s, d| if s == 0 && d < 3 {
        Some(["escape body 0", "escape body 1", "escape body 2"][d as usize])
    } else {
        name_of(s, d)
    })
);

/// Checks the codes' round trip: each state and detail at its code, none past the field.
fn check_codes(state: impl Fn(u32) -> Option<State>, detail: impl Fn(u32) -> Option<Detail>) {
    for (code, s) in State::ALL.iter().enumerate() {
        assert_eq!(state(code as u32), Some(*s), "state code {code}");
        assert_eq!(s.code(), code as u32, "{s:?}'s code");
    }
    assert_eq!(state(8), None, "state code 8");
    assert_eq!(state(u32::MAX), None, "state code u32::MAX");
    for (code, d) in Detail::ALL.iter().enumerate() {
        assert_eq!(detail(code as u32), Some(*d), "detail code {code}");
        assert_eq!(d.code(), code as u32, "{d:?}'s code");
    }
    assert_eq!(detail(4), None, "detail code 4");
}

#[test]
fn display_names_codes_round_trip() {
    check_codes(State::from_code, Detail::from_code);
    // The ledger names the first six states, payload §2's: the names here read the same.
    assert_eq!(State::Reserved6.code() as usize, states().len());
    assert_eq!(states()[1], name(State::Bounded, Detail::D0));
    assert_eq!(states()[3], name(State::Running, Detail::D0));
}

negative_control!(
    display_names_codes_round_trip,
    "a state read past the field's eight codes fails",
    expected = "state code 8",
    check_codes(|c| State::from_code(c % 8), Detail::from_code)
);

/// The bodies and pairs as R-449 shows them: body `k` as "body `k + 1`"; pair `k` by its two bodies, the side opposite
/// body `k` (R-22), shown 1-based, smaller first.
fn check_bodies_and_pairs(
    body: impl Fn(u32) -> Option<&'static str>,
    pair: impl Fn(u32) -> Option<&'static str>,
) {
    for k in 0..3u32 {
        assert_eq!(
            body(k),
            Some(format!("body {}", k + 1)).as_deref(),
            "body {k}"
        );
        let [a, b] = pair_bodies()[k as usize];
        assert!(a != k && b != k, "pair {k} is the side opposite body {k}");
        let shown = format!("{}–{}", a.min(b) + 1, a.max(b) + 1);
        assert_eq!(pair(k), Some(shown.as_str()), "pair {k}'s name");
    }
    assert_eq!(body(3), None, "body 3");
    assert_eq!(pair(3), None, "pair 3");
}

#[test]
fn display_names_pairs_agree_with_pair_bodies() {
    check_bodies_and_pairs(body_name, pair_name);
    // R-449's table: pair 2 = bodies (0, 1), pair 1 = (2, 0), pair 0 = (1, 2).
    assert_eq!(name(State::Collision, Detail::D2), "collision 1–2");
    assert_eq!(name(State::Collision, Detail::D1), "collision 1–3");
    assert_eq!(name(State::Collision, Detail::D0), "collision 2–3");
    assert_eq!(name(State::Collision, Detail::D3), "triple collision");
    assert_eq!(name(State::Escape, Detail::D3), "triple ejection");
    // The `(state, detail)` names are built from the body and pair names.
    for (k, d) in Detail::ALL[..3].iter().enumerate() {
        assert_eq!(
            name(State::Escape, *d),
            format!("escape {}", body_names()[k])
        );
        assert_eq!(
            name(State::Collision, *d),
            format!("collision {}", pair_names()[k])
        );
        assert_eq!(body_number(k as u32), Some(["1", "2", "3"][k]));
    }
    assert_eq!(body_number(3), None);
}

negative_control!(
    display_names_pairs_agree_with_pair_bodies,
    "a pair named by its pair id fails",
    expected = "pair 0's name",
    check_bodies_and_pairs(body_name, |k| {
        (k < 3).then_some(["pair 0", "pair 1", "pair 2"][k as usize])
    })
);

/// Checks that the failure states' names are the state's ledger name and its category, payload §2's.
fn check_failures(f: impl Fn(State, Detail) -> String) {
    for (state, code) in [(State::SimFailed, 2), (State::DecodeFailed, 3)] {
        let (ledger_state, meanings) = detail_meanings()[code];
        assert_eq!(ledger_state, states()[state.code() as usize]);
        for (d, meaning) in Detail::ALL.iter().zip(meanings) {
            assert_eq!(
                f(state, *d),
                format!("{ledger_state}: {meaning}"),
                "{state:?} {d:?}"
            );
        }
    }
}

#[test]
fn display_names_failures_read_their_category() {
    check_failures(|s, d| name(s, d).to_owned());
}

negative_control!(
    display_names_failures_read_their_category,
    "a failure named without its category fails",
    expected = "SimFailed D0",
    check_failures(|s, _| states()[s.code() as usize].to_owned())
);

/// Checks the outcome classes' names: the black class's for `bounded`, `running`, the failures and the reserved codes,
/// the function's for an escape or a collision, nine in all.
fn check_class_names(f: impl Fn(u32, u32) -> Option<&'static str>) {
    let mut names = Vec::new();
    for state in 0..8u32 {
        for detail in 0..4u32 {
            let want = match state {
                0 | 2 => name_of(state, detail).unwrap(),
                _ => "bounded / nothing yet",
            };
            let got = f(state, detail).unwrap();
            assert_eq!(
                got, want,
                "the class name of state {state}, detail {detail}"
            );
            if !names.contains(&got) {
                names.push(got);
            }
        }
    }
    assert_eq!(names.len(), 9, "the outcome palette's nine classes");
    assert_eq!(f(8, 0), None);
    assert_eq!(f(0, 4), None);
}

#[test]
fn display_names_class_names() {
    check_class_names(class_name_of);
    assert_eq!(BLACK_CLASS, "bounded / nothing yet");
    assert_eq!(class_name(State::Running, Detail::D2), BLACK_CLASS);
    assert_eq!(class_name(State::Escape, Detail::D0), "escape body 1");
}

negative_control!(
    display_names_class_names,
    "a black class named \"bounded\" fails",
    expected = "the class name of state 1, detail 0",
    check_class_names(|s, d| match s {
        1 => Some("bounded"),
        _ => class_name_of(s, d),
    })
);

/// Every key a person reads that the ledger gives: the outcome legend's (the class names), the function's names, the
/// `detail` view's legend keys, the `dmin_pair` view's and the body channels' legend.
fn keys() -> Vec<String> {
    let mut out = Vec::new();
    for state in 0..8 {
        for detail in 0..4 {
            out.push(class_name_of(state, detail).unwrap().to_owned());
            out.push(name_of(state, detail).unwrap().to_owned());
        }
    }
    for s in detail_segments() {
        out.extend(s.classes.into_iter().map(|c| c.label));
    }
    out.extend(dmin_pair_legend().into_iter().map(|(_, k)| k.to_owned()));
    out.push(body_channels());
    out
}

/// Checks that no key shows an internal id.
fn check_no_internal_id(keys: &[String]) {
    for k in keys {
        assert!(
            !shows_an_internal_id(k),
            "the key `{k}` shows an internal id"
        );
    }
}

#[test]
fn display_names_no_key_shows_an_internal_id() {
    let keys = keys();
    check_no_internal_id(&keys);
    // The `dmin_pair` legend: each pair id with its pair's name.
    assert_eq!(
        dmin_pair_legend(),
        [(0, "2–3"), (1, "1–3"), (2, "1–2")],
        "the dmin_pair legend"
    );
    assert_eq!(body_channels(), "red body 1, green body 2, blue body 3");
    // The ledger's machine data keeps its 0-based ids; no legend shows it.
    assert!(shows_an_internal_id(detail_meanings()[0].1[0]));
    assert!(shows_an_internal_id(detail_meanings()[1].1[2]));
}

negative_control!(
    display_names_no_key_shows_an_internal_id,
    "a legend keyed with the ledger's own `detail` meanings, \"body 0\" and \"pair 0\", fails",
    expected = "the key `body 0` shows an internal id",
    {
        let mut keys = keys();
        keys.extend(detail_meanings()[0].1.iter().map(|m| (*m).to_owned()));
        check_no_internal_id(&keys);
    }
);

/// Checks the predicate on its edges: each `(text, shows)`.
fn check_predicate(f: impl Fn(&str) -> bool) {
    for (text, shows) in [
        ("body 0", true),
        ("escape body 0", true),
        ("body 1", false),
        ("pair 0", true),
        ("pair 9", true),
        ("a pair 2 here", true),
        ("pair x", false),
        ("pair", false),
        ("pair ", false),
        ("pair  1", false),
        ("pairs 1", false),
        ("the pair, then pair 1", true),
        ("collision 1–2", false),
    ] {
        assert_eq!(f(text), shows, "`{text}`");
    }
}

#[test]
fn display_names_internal_id_predicate() {
    check_predicate(shows_an_internal_id);
}

negative_control!(
    display_names_internal_id_predicate,
    "a predicate that misses a pair id fails",
    expected = "`pair 0`",
    check_predicate(|t| t.contains("body 0"))
);

/// The generated debug views' WGSL, by file stem: each field's view and each reduction, from the payload ledger.
fn view_sources() -> Vec<(String, String)> {
    let ledger = ledger::layout();
    let entries = ledger::gen::validate(&ledger).unwrap_or_else(|e| panic!("{e}"));
    let mut out: Vec<(String, String)> = ledger::gen::catalogue::views(&ledger.words, &entries)
        .into_iter()
        .map(|v| (v.field.to_owned(), v.wgsl))
        .collect();
    out.extend(
        ledger::gen::catalogue::reductions(&ledger.words, &entries)
            .into_iter()
            .map(|r| (r.name, r.wgsl)),
    );
    out
}

/// Checks that each generated view's header gives its legend in the names a person reads: `detail`'s keys the
/// function's, `dmin_pair`'s the pairs', the ternary masses' and each `k = 6` vector's direction cosines' the bodies
/// 1, 2, 3, and `n`'s, whose channels are not bodies, none; and that no view's header shows an internal id.
fn check_view_legends(views: &[(String, String)]) {
    let source = |stem: &str| {
        views
            .iter()
            .find(|(s, _)| s == stem)
            .map(|(_, w)| w.replace("\n// ", " "))
            .unwrap_or_else(|| panic!("no view `{stem}`"))
    };
    let detail = source("detail");
    for key in [
        "0 = escape body 1",
        "3 = triple ejection",
        "0 = collision 2–3",
        "2 = collision 1–2",
        "0 = sim_failed: NaN in state",
    ] {
        assert!(
            detail.contains(key),
            "the detail view's legend has no `{key}`"
        );
    }
    assert!(
        source("dmin_pair").contains("0 = 2–3, 1 = 1–3, 2 = 1–2."),
        "the dmin_pair view's legend"
    );
    let bodies = "red body 1, green body 2, blue body 3";
    for stem in [
        "masses_ternary",
        "r_dircos",
        "p_dircos",
        "r_sh_dircos",
        "p_sh_dircos",
    ] {
        assert!(source(stem).contains(bodies), "the `{stem}` view's legend");
    }
    assert!(
        !source("n_dircos").contains(bodies),
        "the `n_dircos` view names bodies"
    );
    for (stem, wgsl) in views {
        for line in wgsl.lines().filter(|l| l.starts_with("//")) {
            assert!(
                !shows_an_internal_id(line),
                "the `{stem}` view's header shows an internal id: `{line}`"
            );
        }
    }
}

#[test]
fn display_names_debug_view_legends() {
    check_view_legends(&view_sources());
}

negative_control!(
    display_names_debug_view_legends,
    "a detail view keyed by the ledger's 0-based meanings fails",
    expected = "the detail view's legend has no `0 = escape body 1`",
    {
        let views: Vec<(String, String)> = view_sources()
            .into_iter()
            .map(|(s, w)| (s, w.replace("escape body 1", "body 0")))
            .collect();
        check_view_legends(&views);
    }
);
