//! The FTLE views state the shadow's provisional calibration, from one source (R-976, RQ-794 (c); REQ-GUI-438;
//! TASK-M3-73; render contract Part 6, "Field views (one per field, every struct)"):
//! - `ftle_views_state_provisional`: while REQ-INT-076's values are unconfirmed, every catalogue field view that reads
//!   `ftle` (the generated `ftle` view and the derived finalised `ftle` view among them), and the running `S/t` view,
//!   carries the statement in its legend line, the one text of `render::ftle_provisional`; no other view carries it,
//!   no view's WGSL holds a copy of it, its numbers are REQ-INT-076's proposal, and its quotation is R-976's words in
//!   `decisions.md`. Control: a view of `ftle` without it.
//! - `ftle_views_state_provisional_silent_once_confirmed`: once a ruling recording REQ-INT-076's confirmed values is
//!   named, no view carries it. Control: the statement still shown once the confirmed values are recorded.

use std::path::Path;

use render::assemble::Kind;
use render::debug_bake::DebugViews;
use render::ftle_provisional::{
    entry_legend_line, entry_legend_line_with, shows_ftle, statement, CONFIRMED_BY, R976_QUOTE,
    STATEMENT,
};
use render::registry::{self, Catalogue, Category, Entry};
use validation::measure::benettin::{PROPOSED_DELTA_0, PROPOSED_N_RENORM, PROPOSED_TOLERANCE};
use validation::negative_control;

/// The views the task names: the generated `ftle` view, the derived finalised `ftle` view and the running `S/t` view.
const NAMED: [&str; 3] = [
    "debug/generated/ftle",
    "debug/derived/ftle",
    "debug/accumulators/ftle_running",
];

/// A heading standing for the ruling that will record REQ-INT-076's confirmed values.
const CONFIRMING: &str = "R-0000 — REQ-INT-076's values are confirmed at the M3 gate";

/// A legend line as a function of the entry and the switch.
type Line<'a> = &'a dyn Fn(&Entry, Option<&str>) -> Option<&'static str>;

/// The legend line the one source gives.
fn real(e: &Entry, confirmed_by: Option<&str>) -> Option<&'static str> {
    entry_legend_line_with(e, confirmed_by).unwrap_or_else(|err| panic!("`{}`: {err}", e.id))
}

/// The registry's debug field views, each with its field set.
fn views() -> Vec<(Entry, Vec<String>)> {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| e.category == Category::Debug && e.slot == Kind::Colour)
        .map(|e| {
            let stain = render::assemble::Stain::new(registry::view_nodes(&e.source))
                .unwrap_or_else(|err| panic!("`{}`: {err}", e.id));
            let fields = render::assemble::field_set(&stain, render::assemble::Tier::FULL)
                .unwrap_or_else(|err| panic!("`{}`: {err}", e.id));
            (e, fields)
        })
        .collect()
}

/// `decisions.md`'s text.
fn decisions() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../decisions.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The section of `text` under the level-two heading starting `## <heading>`, its white space collapsed.
fn section(text: &str, heading: &str) -> Option<String> {
    let start = text.find(&format!("\n## {heading}"))? + 1;
    let rest = &text[start..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |k| k + 3);
    Some(rest[..end].split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Unconfirmed, `line` gives every view that reads `ftle` or `S`, the named three among them, the one statement, and
/// every other view nothing.
fn check_states(line: Line) {
    let views = views();
    for id in NAMED {
        assert!(
            views.iter().any(|(e, _)| e.id == id),
            "`{id}` is not in the registry"
        );
    }
    for (e, fields) in &views {
        let shows = fields.iter().any(|f| f == "ftle" || f == "S");
        let named = NAMED.contains(&e.id.as_str());
        match line(e, None) {
            Some(l) => {
                assert!(
                    shows,
                    "`{}` reads neither `ftle` nor `S` and carries {l:?}",
                    e.id
                );
                assert!(
                    l == STATEMENT,
                    "`{}` carries a statement that is not the one source's",
                    e.id
                );
            }
            None => assert!(
                !shows && !named,
                "`{}` reads {fields:?} and carries no statement while REQ-INT-076 is unconfirmed",
                e.id
            ),
        }
    }
}

#[test]
fn ftle_views_state_provisional() {
    check_states(&real);

    // The recorded switch is the one the views read; while it is unconfirmed, the statement shows.
    let reg = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    let shown = |e: &&Entry| NAMED.contains(&e.id.as_str()) || e.id == "debug/generated/d_min";
    for e in reg.iter().filter(shown) {
        assert_eq!(
            entry_legend_line(e).unwrap_or_else(|err| panic!("`{}`: {err}", e.id)),
            real(e, CONFIRMED_BY),
            "`{}`'s line is not the recorded switch's",
            e.id
        );
    }
    assert_eq!(
        statement(None),
        Some(STATEMENT),
        "the unconfirmed statement"
    );

    // The baked catalogue view carries it too, and a view of another field does not.
    let mut baked = DebugViews::new(Catalogue::new(&reg));
    assert_eq!(
        baked.legend_line("ftle").unwrap_or_else(|e| panic!("{e}")),
        statement(CONFIRMED_BY),
        "the baked `ftle` view's line"
    );
    assert_eq!(
        baked.legend_line("S").unwrap_or_else(|e| panic!("{e}")),
        statement(CONFIRMED_BY),
        "the baked `S` view's line"
    );
    assert_eq!(
        baked.legend_line("d_min").unwrap_or_else(|e| panic!("{e}")),
        None,
        "the baked `d_min` view's line"
    );
    assert!(
        baked.legend_line("no_such_field").is_err(),
        "a field with no view has no line"
    );

    // Only a debug colour is a field view: a slot colour reading `ftle` and a debug combiner have no line.
    let slot = Entry::new(
        "colour/ftle_custom".to_owned(),
        "colour/ftle_custom.wgsl".into(),
        "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(ctx.sample.ftle); }\n".to_owned(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((slot.category, slot.slot), (Category::Slot, Kind::Colour));
    assert_eq!(real(&slot, None), None, "a slot colour is no field view");
    let combiner = Entry::new(
        "debug/combine_ftle".to_owned(),
        "debug/combine_ftle.wgsl".into(),
        "fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb; }\n".to_owned(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (combiner.category, combiner.slot),
        (Category::Debug, Kind::Combiner)
    );
    assert_eq!(
        real(&combiner, None),
        None,
        "a debug combiner is no field view"
    );

    // The field-set rule: `ftle` or `S`, by name.
    let set = |f: &[&str]| f.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
    assert!(shows_ftle(&set(&["d_min", "ftle"])), "`ftle`");
    assert!(shows_ftle(&set(&["S", "theta"])), "`S`");
    assert!(
        !shows_ftle(&set(&["ftle_valid", "S_final", "d_min"])),
        "neither"
    );

    // One copy of the text: no view's WGSL holds the statement or its quotation.
    for e in &reg {
        assert!(
            !e.source.contains(R976_QUOTE) && !e.source.contains("FTLE provisional"),
            "`{}` holds a copy of the statement",
            e.id
        );
    }

    // Its numbers are REQ-INT-076's proposal, and its quotation R-976's words.
    let values = format!("({PROPOSED_DELTA_0:e}, {PROPOSED_N_RENORM}, {PROPOSED_TOLERANCE})");
    assert_eq!(values, "(3e-3, 300, 0.2)", "REQ-INT-076's proposal");
    assert!(
        STATEMENT.contains(&format!("= {values}")),
        "{STATEMENT:?} does not give {values}"
    );
    assert!(
        STATEMENT.contains(&format!("(R-976: \"{R976_QUOTE}\")")),
        "{STATEMENT:?} does not cite R-976's words"
    );
    let decisions = decisions();
    let r976 = section(&decisions, "R-976 — ").expect("decisions.md has R-976");
    assert!(
        r976.contains(&format!("\"{R976_QUOTE};")),
        "{R976_QUOTE:?} is not R-976's words"
    );

    // A recorded confirmation names a ruling of decisions.md that records REQ-INT-076.
    if let Some(h) = CONFIRMED_BY {
        let ruling = section(&decisions, h).unwrap_or_else(|| panic!("decisions.md has no `{h}`"));
        assert!(
            ruling.contains("REQ-INT-076"),
            "`{h}` does not record REQ-INT-076"
        );
    }
}

negative_control!(
    ftle_views_state_provisional,
    "a view of `ftle` without the statement fails",
    expected = "carries no statement while REQ-INT-076 is unconfirmed",
    check_states(&|e, c| if e.id == "debug/derived/ftle" {
        None
    } else {
        real(e, c)
    })
);

/// Confirmed, `line` gives no view a statement.
fn check_silent(line: Line) {
    for (e, _) in views() {
        assert_eq!(
            line(&e, Some(CONFIRMING)),
            None,
            "`{}` still states the calibration provisional once REQ-INT-076's values are recorded as confirmed",
            e.id
        );
    }
}

#[test]
fn ftle_views_state_provisional_silent_once_confirmed() {
    check_silent(&real);
    assert_eq!(statement(Some(CONFIRMING)), None, "the confirmed switch");
}

negative_control!(
    ftle_views_state_provisional_silent_once_confirmed,
    "the statement still shown once the confirmed values are recorded fails",
    expected = "still states the calibration provisional",
    check_silent(&|e, _| real(e, None))
);
