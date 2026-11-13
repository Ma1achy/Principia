//! QA tests for the chart audit's edit 20 (12 Nov 2026, finding F18), written from the fix it states: the DECODE-preset
//! and ROUNDTRIP-preset goldens' `fixtures/golden/BASELINES.md` rows carry a route column, closed form or table, the two
//! ways R-925 items 1 and 2 give a shader feature to reach a chart; every current row is closed form; a change of route
//! is a recorded conversion under R-931, so the runner reads the column and refuses any other value. Each preset case is
//! found on disk, not from a list, and every check has a registered negative control (R-176).
//!
//! What a route row must still be: its hash checked against the reference (R-110) and its decision a recorded `R-n`
//! (R-110), the same refusals a row without a route has, so the new column opens no way around them.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use validation::negative_control;
use xtask::golden;

/// The golden suites whose reference is the CPU decode at each pixel's chart point (audit F18).
const PRESET_SUITES: [&str; 2] = ["decode-preset", "roundtrip-preset"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("qa_audit20_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch created");
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("dir created");
    for entry in fs::read_dir(from).expect("dir read") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copied");
        }
    }
}

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).expect("read")))
}

/// A markdown table line's cells, trimmed, with the outer pipes dropped.
fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_owned())
        .collect()
}

/// Every preset case under `root`, as `<suite>/<case>`, from the case directories on disk.
fn preset_cases(root: &Path) -> Vec<String> {
    let mut cases = Vec::new();
    for suite in PRESET_SUITES {
        for entry in fs::read_dir(root.join("fixtures/golden").join(suite)).expect("suite read") {
            let entry = entry.expect("entry");
            if entry.path().is_dir() {
                cases.push(format!("{suite}/{}", entry.file_name().to_string_lossy()));
            }
        }
    }
    cases.sort();
    cases
}

/// Each preset case under `root` has exactly one row in `baselines`, of four cells, case, hash, route and decision, its
/// route `closed form` and its hash the case's reference's. Returns how many cases were checked.
fn check_preset_rows(root: &Path, baselines: &str) -> usize {
    let cases = preset_cases(root);
    for case in &cases {
        let rows: Vec<Vec<String>> = baselines
            .lines()
            .map(cells)
            .filter(|c| c[0].trim_matches('`') == case)
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{case}: {} BASELINES.md rows, not one",
            rows.len()
        );
        let row = &rows[0];
        assert_eq!(row.len(), 4, "{case}: its row has no route cell: {row:?}");
        assert_eq!(
            row[2], "closed form",
            "{case}: its route is not closed form: {row:?}"
        );
        let reference = root
            .join("fixtures/golden")
            .join(case)
            .join("reference.png");
        assert_eq!(
            row[1].trim_matches('`'),
            sha(&reference),
            "{case}: its row's hash is not its reference's"
        );
    }
    cases.len()
}

// --- Every current preset row names closed form (edit 20: "every current row is closed form") ---------------------

/// The repo's BASELINES.md against the repo's preset cases; nine cases on disk today (TASK-M2-25's seven, TASK-M2-26's
/// two), so the check reads at least one row of each suite.
fn check_repo_preset_rows(baselines: &str) {
    let n = check_preset_rows(&repo_root(), baselines);
    assert!(n >= 9, "only {n} preset cases found on disk");
}

#[test]
fn qa_every_preset_row_is_closed_form() {
    check_repo_preset_rows(
        &fs::read_to_string(repo_root().join("fixtures/golden/BASELINES.md")).expect("baselines"),
    );
}

negative_control!(
    qa_every_preset_row_is_closed_form,
    "the last preset row's route set to table",
    expected = "its route is not closed form",
    {
        let text = fs::read_to_string(repo_root().join("fixtures/golden/BASELINES.md")).unwrap();
        let at = text.rfind("| closed form |").expect("a route cell");
        let mut edited = text.clone();
        edited.replace_range(at..at + "| closed form |".len(), "| table |");
        check_repo_preset_rows(&edited);
    }
);

/// The same check, as its own test so the dropped-cell control has a name of its own (one control per test).
#[test]
fn qa_every_preset_row_has_a_route_cell() {
    check_repo_preset_rows(
        &fs::read_to_string(repo_root().join("fixtures/golden/BASELINES.md")).expect("baselines"),
    );
}

negative_control!(
    qa_every_preset_row_has_a_route_cell,
    "a preset row with its route cell dropped",
    expected = "its row has no route cell",
    check_repo_preset_rows(
        &fs::read_to_string(repo_root().join("fixtures/golden/BASELINES.md"))
            .unwrap()
            .replacen(" | closed form |", " |", 1),
    )
);

// --- The runner loads the repo's preset suites with their route rows ---------------------------------------------

/// Both preset suites under `root` load, each with at least one case.
fn check_suites_load(root: &Path) {
    for suite in PRESET_SUITES {
        let cases = golden::load_suite(root, suite)
            .unwrap_or_else(|e| panic!("suite {suite} refused: {e}"));
        assert!(!cases.is_empty(), "suite {suite} loaded no case");
    }
}

#[test]
fn qa_runner_loads_preset_suites_with_routes() {
    check_suites_load(&repo_root());
}

negative_control!(
    qa_runner_loads_preset_suites_with_routes,
    "the repo's preset suites with one route misspelt",
    expected = "suite decode-preset refused",
    check_suites_load(&preset_root("ctl_load", |t| with_latent_route(
        t,
        "closed  form"
    )))
);

/// A scratch root holding the repo's preset suites, decisions.md, and the repo's BASELINES.md passed through `edit`.
fn preset_root(name: &str, edit: impl Fn(&str) -> String) -> PathBuf {
    let root = scratch(name);
    for suite in PRESET_SUITES {
        copy_tree(
            &repo_root().join("fixtures/golden").join(suite),
            &root.join("fixtures/golden").join(suite),
        );
    }
    fs::copy(repo_root().join("decisions.md"), root.join("decisions.md")).expect("decisions");
    let text =
        fs::read_to_string(repo_root().join("fixtures/golden/BASELINES.md")).expect("baselines");
    fs::write(root.join("fixtures/golden/BASELINES.md"), edit(&text)).expect("baselines written");
    root
}

/// `decode-preset/latent`'s row, with its route cell replaced by `route`.
fn with_latent_route(text: &str, route: &str) -> String {
    text.lines()
        .map(|line| {
            let c = cells(line);
            if c[0] == "`decode-preset/latent`" && c.len() == 4 {
                format!("| {} | {} | {route} | {} |", c[0], c[1], c[3])
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Loads both preset suites of a scratch root whose `decode-preset/latent` row names `route`.
fn load_latent_route(name: &str, route: &str) -> Result<(), String> {
    let root = preset_root(name, |t| with_latent_route(t, route));
    for suite in PRESET_SUITES {
        golden::load_suite(&root, suite)?;
    }
    Ok(())
}

// --- Both routes R-925 names load; nothing else does (R-925 items 1 and 2, R-931) --------------------------------

/// A row naming `closed form` and a row naming `table` both load: a recorded conversion to the table route is a row
/// the runner accepts.
fn check_both_routes_load(name: &str, routes: [&str; 2]) {
    for route in routes {
        load_latent_route(&format!("{name}_{}", route.replace(' ', "_")), route)
            .unwrap_or_else(|e| panic!("route `{route}` was refused: {e}"));
    }
}

#[test]
fn qa_closed_form_and_table_routes_load() {
    check_both_routes_load("ok", ["closed form", "table"]);
}

negative_control!(
    qa_closed_form_and_table_routes_load,
    "a route spelled as R-922's lookup texture",
    expected = "route `lookup texture` was refused",
    check_both_routes_load("ctl_ok", ["closed form", "lookup texture"])
);

/// Each of `routes` in `decode-preset/latent`'s row is refused, the message naming the route.
fn check_routes_refused(name: &str, routes: &[&str]) {
    for (k, route) in routes.iter().enumerate() {
        match load_latent_route(&format!("{name}_{k}"), route) {
            Ok(()) => panic!("route `{route}` was not refused"),
            Err(e) => assert!(
                e.contains(&format!("`{route}`")) && e.contains("decode-preset/latent"),
                "route `{route}` refused for another reason: {e}"
            ),
        }
    }
}

#[test]
fn qa_other_routes_are_refused() {
    check_routes_refused(
        "bad",
        &[
            "lookup texture",
            "closed-form",
            "CLOSED FORM",
            "tables",
            "closed form or table",
            "R-925",
            "",
        ],
    );
}

negative_control!(
    qa_other_routes_are_refused,
    "the table route, which is one of R-925's two",
    expected = "route `table` was not refused",
    check_routes_refused("ctl_bad", &["table"])
);

// --- A route row keeps R-110's refusals: its hash and its decision are still checked -----------------------------

/// A route row for `decode-preset/latent` whose hash is `roundtrip-preset/latent`'s is refused as a changed reference.
fn check_route_row_hash_checked(name: &str, wrong: bool) {
    let other = sha(&repo_root().join("fixtures/golden/roundtrip-preset/latent/reference.png"));
    let mine = sha(&repo_root().join("fixtures/golden/decode-preset/latent/reference.png"));
    assert_ne!(other, mine, "the two references differ");
    let root = preset_root(name, |t| {
        if wrong {
            t.replacen(
                &format!("`{mine}` | closed form"),
                &format!("`{other}` | closed form"),
                1,
            )
        } else {
            t.to_owned()
        }
    });
    match golden::load_suite(&root, "decode-preset") {
        Ok(_) => panic!("a route row with another reference's hash loaded"),
        Err(e) => assert!(
            e.contains("changed without"),
            "refused for another reason: {e}"
        ),
    }
}

#[test]
fn qa_route_row_hash_is_checked() {
    check_route_row_hash_checked("hash", true);
}

negative_control!(
    qa_route_row_hash_is_checked,
    "the row's hash left right",
    expected = "a route row with another reference's hash loaded",
    check_route_row_hash_checked("ctl_hash", false)
);

/// A route row for `decode-preset/latent` whose decision is `decision` is refused with `refusal`.
fn check_route_row_decision(name: &str, decision: &str, refusal: &str) {
    let root = preset_root(name, |t| {
        t.lines()
            .map(|line| {
                let c = cells(line);
                if c[0] == "`decode-preset/latent`" && c.len() == 4 {
                    format!("| {} | {} | {} | {decision} |", c[0], c[1], c[2])
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    });
    match golden::load_suite(&root, "decode-preset") {
        Ok(_) => panic!("a route row with decision `{decision}` loaded"),
        Err(e) => assert!(e.contains(refusal), "refused for another reason: {e}"),
    }
}

#[test]
fn qa_route_row_decision_is_checked() {
    check_route_row_decision("unnamed", "re-made by hand", "names no decision");
    check_route_row_decision(
        "unrecorded",
        "R-99999",
        "which decisions.md does not record",
    );
}

negative_control!(
    qa_route_row_decision_is_checked,
    "a recorded decision in the route row",
    expected = "a route row with decision `R-931` loaded",
    check_route_row_decision("ctl_decision", "R-931", "names no decision")
);
