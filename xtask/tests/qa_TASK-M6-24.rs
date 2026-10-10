//! QA tests for TASK-M6-24 on the screenshot runner's side, written from the requirements, not from the
//! implementation:
//!
//! - REQ-GUI-162: "`gui` must ship a headless capture mode that `cargo xtask screenshot` spawns as a separate process:
//!   it renders a named window offscreen and writes the PNG and the AccessKit names, with no crate depending on `gui`
//!   (R-274)"; verify: "a screenshot case with surface kind `gui` spawns the capture mode for a named window and gets
//!   its PNG and names back; `cargo xtask deps` shows no edge into gui".
//! - RQ-253, as the task gives it: a case's `surface` stays a path for a `data` case, and may instead be an object
//!   `{ "kind": "gui", "screen": "01_main", "steps": [...] }`, whose steps are a closed list (`f3`, `raise_warning`,
//!   `raise_error`, `click_footer`); the runner spawns `cargo run --quiet -p gui --features mock -- capture --screen …
//!   --steps … --out <case dir>`.
//! - R-275: a name counts as present only when its rect intersects the visible surface.
//! - The task's screenshot cases `01_main/mock_shell`, `mock_f3_off`, `mock_footer`, `mock_warning`, against
//!   01_main.png: F3 on and off, a raised warning and error, a footer click opening the console.
//!
//! The runs that spawn cargo stay the implementer's `screenshot_gui_surface` tests, in the `ci-workspace` profile;
//! these read the runner's parts without spawning a build. Each test's control (R-176) feeds the same check an input
//! differing in the one respect the requirement turns on.
// The file name `qa_TASK-M6-24` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use validation::negative_control;
use xtask::deps::{self, Edge, Metadata};
use xtask::screenshot::{self, Case, GuiKind, GuiStep, GuiSurface, SurfaceRef};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn case(surface: Value) -> Result<Case, String> {
    serde_json::from_value(json!({ "name": "c", "surface": surface, "artboard": "a.png" }))
        .map_err(|e| e.to_string())
}

// --- RQ-253: the two surface forms -----------------------------------------------------------------------------------

/// A path reads as a `data` surface; the object as a `gui` surface with its steps in order; anything outside the
/// closed forms is refused.
fn check_forms(accepted_step: &str) {
    let data = case(json!("surface.json")).expect("a path surface");
    assert_eq!(
        data.surface,
        SurfaceRef::Data("surface.json".into()),
        "a path is not a data surface"
    );
    let gui = case(json!({ "kind": "gui", "screen": "01_main", "steps": ["click_footer", "f3", "raise_error", "raise_warning"] }))
        .expect("a gui surface");
    assert_eq!(
        gui.surface,
        SurfaceRef::Gui(GuiSurface {
            kind: GuiKind::Gui,
            screen: "01_main".into(),
            steps: vec![
                GuiStep::ClickFooter,
                GuiStep::F3,
                GuiStep::RaiseError,
                GuiStep::RaiseWarning
            ],
        }),
        "the gui surface's steps"
    );
    let refused = [
        json!({ "kind": "gui", "screen": "01_main", "steps": [accepted_step] }),
        json!({ "kind": "web", "screen": "01_main", "steps": [] }),
        json!({ "kind": "gui", "steps": [] }),
        json!({ "kind": "gui", "screen": "01_main", "steps": [], "extra": 1 }),
        json!(7),
    ];
    for surface in refused {
        assert!(
            case(surface.clone()).is_err(),
            "the surface {surface} is outside the closed forms but was accepted"
        );
    }
}

#[test]
fn qa_screenshot_surface_forms() {
    check_forms("scroll");
}

negative_control!(
    qa_screenshot_surface_forms,
    "a step of the closed list must be accepted, so the check must fail on it",
    expected = "outside the closed forms but was accepted",
    check_forms("f3")
);

// --- The task's 01_main cases ----------------------------------------------------------------------------------------

fn cases_at(path: &Path) -> Vec<Case> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let file: Value = serde_json::from_str(&text).expect("cases.json is JSON");
    file["cases"]
        .as_array()
        .expect("a cases list")
        .iter()
        .map(|c| {
            serde_json::from_value(c.clone())
                .unwrap_or_else(|e| panic!("a case of {}: {e}", path.display()))
        })
        .collect()
}

fn steps(c: &Case) -> Vec<GuiStep> {
    match &c.surface {
        SurfaceRef::Gui(g) => {
            assert_eq!(g.screen, "01_main", "case {} is not on 01_main", c.name);
            g.steps.clone()
        }
        SurfaceRef::Data(p) => panic!("case {} is a data surface {p}, not the dev GUI", c.name),
    }
}

/// The suite holds the four named cases, each a gui surface on 01_main beside 01_main.png: `mock_shell` with no step,
/// `mock_f3_off` pressing F3, and among them a raised warning and error and a footer click; and, since TASK-M6-26,
/// the four cases that task names, `mock_manifold_view`, `mock_compass_slice`, `mock_compass_tilt` and
/// `mock_axis_labels`, and no other.
fn check_01_main(cases: &[Case]) {
    let mut names: Vec<&str> = cases.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "mock_axis_labels",
            "mock_compass_slice",
            "mock_compass_tilt",
            "mock_f3_off",
            "mock_footer",
            "mock_manifold_view",
            "mock_shell",
            "mock_warning"
        ],
        "the 01_main cases"
    );
    for c in cases {
        let art = c
            .artboard
            .as_deref()
            .unwrap_or_else(|| panic!("case {} has no artboard", c.name));
        assert_eq!(
            art, "docs/gui/design/01_main.png",
            "case {}'s artboard",
            c.name
        );
        assert!(
            repo().join(art).is_file(),
            "the artboard {art} is not checked in"
        );
    }
    let by = |n: &str| steps(cases.iter().find(|c| c.name == n).expect("named above"));
    assert_eq!(by("mock_shell"), [], "mock_shell plays a step");
    assert!(
        by("mock_f3_off").contains(&GuiStep::F3),
        "mock_f3_off does not press F3"
    );
    let all: Vec<GuiStep> = cases.iter().flat_map(steps).collect();
    for want in [
        GuiStep::RaiseWarning,
        GuiStep::RaiseError,
        GuiStep::ClickFooter,
    ] {
        assert!(all.contains(&want), "no 01_main case plays {want:?}");
    }
    let warned = by("mock_warning");
    assert!(
        warned.contains(&GuiStep::RaiseWarning) && warned.contains(&GuiStep::RaiseError),
        "mock_warning does not raise a warning and an error: {warned:?}"
    );
}

#[test]
fn qa_screenshot_01_main_cases() {
    check_01_main(&cases_at(
        &repo().join("fixtures/screenshot/01_main/cases.json"),
    ));
    // A data case is unchanged: the selftest suite still reads, its surfaces paths.
    for c in cases_at(&repo().join("fixtures/screenshot/selftest/cases.json")) {
        assert!(
            matches!(c.surface, SurfaceRef::Data(_)),
            "selftest's case {} is no longer a data case",
            c.name
        );
    }
}

negative_control!(
    qa_screenshot_01_main_cases,
    "the suite with mock_f3_off's F3 removed must fail",
    expected = "mock_f3_off does not press F3",
    {
        let mut cases = cases_at(&repo().join("fixtures/screenshot/01_main/cases.json"));
        for c in &mut cases {
            if let SurfaceRef::Gui(g) = &mut c.surface {
                if c.name == "mock_f3_off" {
                    g.steps.retain(|s| *s != GuiStep::F3);
                }
            }
        }
        check_01_main(&cases)
    }
);

// --- RQ-253: the spawned command -------------------------------------------------------------------------------------

/// The runner spawns cargo running gui's capture mode on the mock: `run --quiet -p gui --features mock -- capture
/// --screen <screen> --steps <comma-separated> --out <dir>`.
fn check_command(steps: Vec<GuiStep>, want_steps: &str) {
    let out = Path::new("/tmp/qa-case-dir");
    let gui = GuiSurface {
        kind: GuiKind::Gui,
        screen: "01_main".into(),
        steps,
    };
    let command = screenshot::capture_command(&gui, out);
    let program = PathBuf::from(command.get_program());
    assert!(
        program.file_stem().is_some_and(|s| s == "cargo"),
        "the runner does not spawn cargo: {}",
        program.display()
    );
    let args: Vec<String> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args.first().map(String::as_str),
        Some("run"),
        "not `cargo run`: {args:?}"
    );
    assert!(args.iter().any(|a| a == "--quiet"), "not quiet: {args:?}");
    let tail = [
        "-p",
        "gui",
        "--features",
        "mock",
        "--",
        "capture",
        "--screen",
        "01_main",
        "--steps",
        want_steps,
        "--out",
        "/tmp/qa-case-dir",
    ];
    assert!(
        args.windows(tail.len()).any(|w| w == tail),
        "the command's arguments {args:?} lack `{}`",
        tail.join(" ")
    );
}

#[test]
fn qa_screenshot_capture_command() {
    check_command(
        vec![
            GuiStep::RaiseWarning,
            GuiStep::RaiseError,
            GuiStep::ClickFooter,
        ],
        "raise_warning,raise_error,click_footer",
    );
    check_command(vec![], "");
}

negative_control!(
    qa_screenshot_capture_command,
    "the steps in another order must not match",
    expected = "the command's arguments",
    check_command(
        vec![GuiStep::RaiseError, GuiStep::RaiseWarning],
        "raise_warning,raise_error"
    )
);

// --- R-275: the names file's rects -----------------------------------------------------------------------------------

/// A name whose rect meets the capture, even in part, counts; one wholly outside, or with no rect, is clipped.
fn check_names(size: [u32; 2]) {
    let text = r#"{"size":[200,100],"names":[
        {"name":"whole","rect":[10,10,50,30]},
        {"name":"straddles","rect":[190,90,230,130]},
        {"name":"beyond","rect":[201,0,260,20]},
        {"name":"below","rect":[0,101,10,120]},
        {"name":"unplaced","rect":null}
    ]}"#;
    let got = screenshot::read_names(text, size)
        .unwrap_or_else(|e| panic!("the names file was refused: {e}"));
    assert_eq!(got.names, ["whole", "straddles"], "the visible names");
    assert_eq!(
        got.clipped,
        ["beyond", "below", "unplaced"],
        "the clipped names"
    );
}

#[test]
fn qa_screenshot_names_by_visibility() {
    check_names([200, 100]);
}

negative_control!(
    qa_screenshot_names_by_visibility,
    "names written for another capture size must be refused",
    expected = "the names file was refused",
    check_names([2160, 1350])
);

// --- R-274: no crate depends on gui ----------------------------------------------------------------------------------

fn check_no_edge_into_gui(edges: &[Edge]) {
    let into: Vec<&Edge> = edges.iter().filter(|e| e.to == "gui").collect();
    assert!(into.is_empty(), "edges into gui: {into:?}");
    assert!(
        deps::check(edges).is_empty(),
        "the workspace's edges break the crate map: {:?}",
        deps::check(edges)
    );
}

#[test]
fn qa_no_crate_depends_on_gui() {
    let edges = Metadata::from_cargo()
        .expect("cargo metadata")
        .edges()
        .expect("the edges");
    assert!(
        edges.iter().any(|e| e.from == "gui" && e.to == "engine"),
        "gui does not depend on engine: {edges:?}"
    );
    check_no_edge_into_gui(&edges);
}

negative_control!(
    qa_no_crate_depends_on_gui,
    "xtask depending on gui, as a runner linking the app would, must fail",
    expected = "edges into gui",
    check_no_edge_into_gui(&[Edge {
        from: "xtask".into(),
        to: "gui".into(),
        kind: xtask::deps::DepKind::Normal
    }])
);
