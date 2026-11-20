//! QA tests for TASK-M1-13, after the merge of TASK-M1-12's `debug-views` suite: written from REQ-TOOL-010 ("every
//! debug view the registry lists has its `debug-views` case and fixture") and the task's deliverable "the structural
//! debug views' cases also go in `fixtures/golden/debug-views/`" (RQ-237).
//!
//! The showcase's cases (`golden_scene::debug_cases`) leave the structural views out, their cases being the structural
//! scenes; so every debug view the registry lists that is no showcase case must have a `debug-views` fixture of its
//! own name rendering the structural scene of that name. The fixture check here does not use `is_structural`, the
//! rule under test, only the registry, the showcase's cases and the fixtures on disk.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use render::registry::{self, Category};
use validation::golden_scene::debug_cases;
use validation::negative_control;
use validation::structural_scene;

/// The structural views' shaders this task adds under `frag/debug/` (debug_tooling_plan §F).
const STRUCTURAL: [&str; 9] = [
    "s_depth",
    "s_state",
    "s_coherence",
    "s_impurity",
    "s_spread",
    "s_suspect",
    "s_priority",
    "s_cache_age",
    "s_ancestor_gap",
];

fn suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/debug-views")
}

/// The suite's fixtures: each directory's name and the harness scene its `case.json` renders (`None` without one).
fn fixtures() -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = std::fs::read_dir(suite())
        .expect("the debug-views suite's directory")
        .map(|d| {
            let name = d
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            let harness = std::fs::read_to_string(suite().join(&name).join("case.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|j| j["render"]["harness"].as_str().map(str::to_owned));
            (name, harness)
        })
        .collect();
    out.sort();
    out
}

/// The registry's debug views that are no case of the showcase.
fn uncased() -> Vec<String> {
    let cases = debug_cases().unwrap_or_else(|e| panic!("{e}"));
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|e| e.category == Category::Debug)
        .map(|e| e.id)
        .filter(|id| cases.iter().all(|c| &c.id != id))
        .collect()
}

/// Checks that each of `ids`, a debug view `debug/<name>` that is no showcase case, has a `debug-views` fixture named
/// `<name>` whose harness renders the structural scene `<name>`.
fn check_fixtured(ids: &[String], fixtures: &[(String, Option<String>)]) {
    for id in ids {
        let name = id
            .strip_prefix("debug/")
            .unwrap_or_else(|| panic!("`{id}` is no debug view's id"));
        assert!(
            structural_scene::NAMES.contains(&name),
            "the debug view `{id}` is no showcase case and no structural scene"
        );
        let fixture = fixtures.iter().find(|(n, _)| n == name);
        assert!(
            fixture.is_some(),
            "the debug view `{id}` has no debug-views fixture"
        );
        assert_eq!(
            fixture.and_then(|(_, h)| h.as_deref()),
            Some(name),
            "the debug view `{id}`'s fixture renders another scene"
        );
    }
}

#[test]
fn qa_every_uncased_debug_view_has_its_structural_fixture() {
    let ids = uncased();
    // The views left out of the showcase are exactly the structural views.
    let mut got: Vec<&str> = ids.iter().map(|id| id.as_str()).collect();
    got.sort_unstable();
    let mut want: Vec<String> = STRUCTURAL.iter().map(|v| format!("debug/{v}")).collect();
    want.sort();
    assert_eq!(got, want, "the debug views that are no showcase case");
    check_fixtured(&ids, &fixtures());
    // The ensemble spread scene has its fixture too (the spread view with `has_ensemble()` true).
    assert!(
        fixtures()
            .iter()
            .any(|(n, h)| n == "s_spread_ensemble" && h.as_deref() == Some("s_spread_ensemble")),
        "the s_spread_ensemble fixture"
    );
}

negative_control!(
    qa_every_uncased_debug_view_has_its_structural_fixture,
    "a structural view whose debug-views fixture is missing",
    expected = "the debug view `debug/s_depth` has no debug-views fixture",
    {
        let without: Vec<(String, Option<String>)> = fixtures()
            .into_iter()
            .filter(|(n, _)| n != "s_depth")
            .collect();
        check_fixtured(&uncased(), &without)
    }
);
