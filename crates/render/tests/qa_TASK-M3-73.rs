//! QA tests for TASK-M3-73, written from REQ-GUI-438 and R-976 (RQ-794 (c)), not from the implementation.
//!
//! REQ-GUI-438: "There must be one source for the statement that the FTLE shadow's calibration (‖δ₀‖ 3e-3, n_renorm
//! 300, dd test 10's tolerance 0.2) is provisional and unsupported on close-encounter chaotic orbits, which every
//! surface showing the FTLE reads and which falls silent once REQ-INT-076's values are confirmed [...]; until then every
//! field view of `ftle` already built (TASK-M1-12's finalised `ftle` and running `S/t` views, and every catalogue view
//! that reads `ftle`) must state it."
//!
//! Which views read the FTLE is decided here independently of the implementation's field-set rule: by the view's own
//! WGSL, which reads the finalised value as `ctx.sample.ftle` or the Benettin sum the running `S/t` view divides as
//! `ctx.sample.S` (render contract Part 5, "FTLE-running `S/t`"; Part 6, "Field views (one per field, every struct)").
//!
//! - `qa_every_view_reading_the_ftle_states_it`: every debug field view whose WGSL reads `ftle` or `S` carries the
//!   statement while unconfirmed, the four built views among them; no other view does. Control: the generated `S`
//!   view without it.
//! - `qa_every_baked_catalogue_view_states_it`: through the baker, every catalogue field's legend line is the recorded
//!   switch's statement exactly where its view reads the FTLE. Control: the baked `ftle` view without it.
//! - `qa_statement_says_what_r976_requires`: the text names the calibration's three values, provisional, and
//!   unsupported on close-encounter chaotic orbits. Control: a text dropping "unsupported" fails.
//! - `qa_silent_once_any_confirming_ruling_is_named`: whatever ruling heading records the confirmation, every view falls
//!   silent. Control: a line that ignores the switch.
//! - `qa_one_copy_of_the_statement_in_the_workspace`: no source file of any crate but the one source holds the
//!   statement's text. Control: a second copy in another file is found.

use std::path::{Path, PathBuf};

use render::assemble::Kind;
use render::debug_bake::DebugViews;
use render::ftle_provisional::{entry_legend_line_with, statement, CONFIRMED_BY, STATEMENT};
use render::registry::{self, Catalogue, Category, Entry};
use validation::negative_control;

/// The FTLE views TASK-M1-12 built, which REQ-GUI-438 names: the generated `ftle` and `S` views, the derived finalised
/// `ftle` view and the running `S/t` accumulator view.
const BUILT: [&str; 4] = [
    "debug/generated/ftle",
    "debug/generated/S",
    "debug/derived/ftle",
    "debug/accumulators/ftle_running",
];

/// Whether `wgsl`, its `//` comments dropped, reads `ctx.sample.<field>` as a whole identifier.
fn reads(wgsl: &str, field: &str) -> bool {
    let needle = format!("ctx.sample.{field}");
    wgsl.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .any(|code| {
            code.match_indices(&needle).any(|(at, _)| {
                code[at + needle.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
            })
        })
}

/// Whether a view's WGSL shows the FTLE: it reads the finalised `ftle` or the Benettin sum `S`.
fn shows_the_ftle(wgsl: &str) -> bool {
    reads(wgsl, "ftle") || reads(wgsl, "S")
}

/// The registry's debug field views.
fn field_views() -> Vec<Entry> {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| e.category == Category::Debug && e.slot == Kind::Colour)
        .collect()
}

/// Unconfirmed, `line` gives every field view whose WGSL shows the FTLE exactly the statement, and every other none.
fn check_every_view(line: &dyn Fn(&Entry) -> Option<&'static str>) {
    let views = field_views();
    for id in BUILT {
        let e = views
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not a debug field view"));
        assert!(
            shows_the_ftle(&e.source),
            "`{id}`'s WGSL reads neither `ctx.sample.ftle` nor `ctx.sample.S`"
        );
    }
    for e in &views {
        let got = line(e);
        if shows_the_ftle(&e.source) {
            assert_eq!(
                got,
                Some(STATEMENT),
                "`{}` shows the FTLE and carries no statement while unconfirmed",
                e.id
            );
        } else {
            assert_eq!(
                got, None,
                "`{}` does not show the FTLE and carries a statement",
                e.id
            );
        }
    }
}

fn unconfirmed(e: &Entry) -> Option<&'static str> {
    entry_legend_line_with(e, None).unwrap_or_else(|err| panic!("`{}`: {err}", e.id))
}

#[test]
fn qa_every_view_reading_the_ftle_states_it() {
    // The oracle itself: it sees a read, and not a near name or a comment.
    assert!(reads("let v = ctx.sample.S / 2.0;", "S"));
    assert!(!reads("let v = ctx.sample.S_final;", "S"));
    assert!(!reads("let v = ctx.sample.ftle_valid;", "ftle"));
    assert!(!reads("// ctx.sample.ftle", "ftle"));
    check_every_view(&unconfirmed);
}

negative_control!(
    qa_every_view_reading_the_ftle_states_it,
    "the generated `S` view without the statement fails",
    expected = "shows the FTLE and carries no statement while unconfirmed",
    check_every_view(&|e| if e.id == "debug/generated/S" {
        None
    } else {
        unconfirmed(e)
    })
);

/// Every catalogue field's baked legend line, given by `line`, is the recorded switch's statement where its view
/// shows the FTLE, and none elsewhere.
fn check_baked(line: &mut dyn FnMut(&str) -> Option<&'static str>) {
    let reg = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    let cat = Catalogue::new(&reg);
    let fields: Vec<String> = cat.fields().into_iter().map(str::to_owned).collect();
    assert!(
        fields.iter().any(|f| f == "ftle") && fields.iter().any(|f| f == "S"),
        "the catalogue has no `ftle` or `S` view: {fields:?}"
    );
    for f in &fields {
        let id = format!("debug/generated/{f}");
        let e = reg
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry `{id}`"));
        let want = if shows_the_ftle(&e.source) {
            statement(CONFIRMED_BY)
        } else {
            None
        };
        assert_eq!(line(f), want, "the baked `{f}` view's legend line");
    }
}

#[test]
fn qa_every_baked_catalogue_view_states_it() {
    let reg = registry::registry().unwrap_or_else(|e| panic!("{e}"));
    let mut baked = DebugViews::new(Catalogue::new(&reg));
    check_baked(&mut |f| {
        baked
            .legend_line(f)
            .unwrap_or_else(|e| panic!("`{f}`: {e}"))
    });
    // Today REQ-INT-076's values are unconfirmed (the M3 gate has not confirmed them, R-976), so the recorded switch
    // shows the statement on the baked `ftle` view.
    if CONFIRMED_BY.is_none() {
        assert_eq!(
            baked.legend_line("ftle").unwrap_or_else(|e| panic!("{e}")),
            Some(STATEMENT)
        );
    }
}

negative_control!(
    qa_every_baked_catalogue_view_states_it,
    "the baked `ftle` view without the statement fails",
    expected = "the baked `ftle` view's legend line",
    {
        let reg = registry::registry().unwrap_or_else(|e| panic!("{e}"));
        let mut baked = DebugViews::new(Catalogue::new(&reg));
        check_baked(&mut |f| {
            if f == "ftle" {
                None
            } else {
                baked
                    .legend_line(f)
                    .unwrap_or_else(|e| panic!("`{f}`: {e}"))
            }
        })
    }
);

/// `text` says what R-976 and REQ-GUI-438 require the statement to say.
fn check_statement(text: &str) {
    for part in [
        "‖δ₀‖",
        "n_renorm",
        "3e-3",
        "300",
        "0.2",
        "provisional",
        "unsupported",
        "close-encounter chaotic orbits",
    ] {
        assert!(
            text.contains(part),
            "the statement lacks {part:?}: {text:?}"
        );
    }
}

#[test]
fn qa_statement_says_what_r976_requires() {
    check_statement(STATEMENT);
    assert_eq!(statement(None), Some(STATEMENT));
}

negative_control!(
    qa_statement_says_what_r976_requires,
    "a statement that drops the unsupported region fails",
    expected = "the statement lacks \"unsupported\"",
    check_statement(&STATEMENT.replace("unsupported", "untested"))
);

/// Rulings that might record REQ-INT-076's confirmation, in any wording.
const CONFIRMING: [&str; 3] = [
    "R-1000 — REQ-INT-076 confirmed at the M3 gate",
    "R-2 — the FTLE shadow's calibration",
    "x",
];

/// Under each confirming ruling, `line` gives no field view that shows the FTLE a statement.
fn check_silent(line: &dyn Fn(&Entry, Option<&str>) -> Option<&'static str>) {
    // The views that show the FTLE, the ones the switch governs (every other view is silent either way, which
    // `qa_every_view_reading_the_ftle_states_it` checks).
    let views: Vec<Entry> = field_views()
        .into_iter()
        .filter(|e| shows_the_ftle(&e.source))
        .collect();
    assert!(
        views.len() >= BUILT.len(),
        "too few FTLE views: {}",
        views.len()
    );
    for h in CONFIRMING {
        assert_eq!(statement(Some(h)), None, "the switch under {h:?}");
        for e in &views {
            assert_eq!(
                line(e, Some(h)),
                None,
                "`{}` still carries the statement once {h:?} records the confirmation",
                e.id
            );
        }
    }
}

fn under(e: &Entry, c: Option<&str>) -> Option<&'static str> {
    entry_legend_line_with(e, c).unwrap_or_else(|err| panic!("`{}`: {err}", e.id))
}

#[test]
fn qa_silent_once_any_confirming_ruling_is_named() {
    check_silent(&under);
}

negative_control!(
    qa_silent_once_any_confirming_ruling_is_named,
    "a line that ignores the switch fails",
    expected = "still carries the statement once",
    check_silent(&|e, _| under(e, None))
);

/// The statement's own words, beyond R-976's quotation, which other code may cite.
const OWN_WORDS: &str = "unconfirmed and unsupported on close-encounter chaotic orbits";

/// Source files (`.rs`, `.wgsl`, `.js`, `.ts`, `.html`) under `dir`, recursively.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for d in rd.flatten() {
        let p = d.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| ["rs", "wgsl", "js", "ts", "html"].contains(&x))
        {
            out.push(p);
        }
    }
}

/// Of `files`, only the one source holds the statement's own words, and it does.
fn check_one_copy(files: &[(PathBuf, String)]) {
    assert!(OWN_WORDS.len() > 20 && STATEMENT.contains(OWN_WORDS));
    let holders: Vec<&PathBuf> = files
        .iter()
        .filter(|(_, text)| text.contains(OWN_WORDS))
        .map(|(p, _)| p)
        .collect();
    assert!(
        holders.len() == 1 && holders[0].ends_with("crates/render/src/ftle_provisional.rs"),
        "the statement's text is held by {holders:?}, not by the one source alone"
    );
}

/// Every non-test source file of the workspace's crates and xtask.
fn workspace_sources() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root.join("xtask/src")];
    for c in std::fs::read_dir(root.join("crates"))
        .unwrap_or_else(|e| panic!("{e}"))
        .flatten()
    {
        for sub in ["src", "shaders", "frag", "web", "www"] {
            dirs.push(c.path().join(sub));
        }
    }
    let mut paths = Vec::new();
    for d in dirs {
        sources(&d, &mut paths);
    }
    assert!(
        paths.len() > 50,
        "too few source files found: {}",
        paths.len()
    );
    paths
        .into_iter()
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|t| (p, t)))
        .collect()
}

#[test]
fn qa_one_copy_of_the_statement_in_the_workspace() {
    check_one_copy(&workspace_sources());
}

negative_control!(
    qa_one_copy_of_the_statement_in_the_workspace,
    "a second copy of the statement in another file is found",
    expected = "not by the one source alone",
    {
        let mut files = workspace_sources();
        files.push((
            PathBuf::from("crates/gui/src/legend.rs"),
            format!("const LINE: &str = {STATEMENT:?};"),
        ));
        check_one_copy(&files)
    }
);
