//! The Print graph's registry check (TASK-M7-54; principia_print_graph.md §4.3, §11 test 8; REQ-RENDER-104): the
//! shipped registry and presets pass; a fixture preset naming `linocutt` fails naming it; a fixture setting
//! `linocut.depth` outside its range fails naming the preset, the node and the parameter; and a length parameter
//! registered without a unit fails. That the build runs this check is `xtask/tests/print_registry_build.rs`'s. Each
//! test registers its negative control (R-176).

use render::print::registry::{check, Checked, Registry, RegistryError};
use render::print::{shipped, NODES, PRESETS, WGSL};
use validation::negative_control;

// This file reads the fixture files, not the fixture registry or presets.
#[allow(dead_code)]
#[path = "support/print_fixtures.rs"]
mod support;

use support::{fixture, FIXTURE_NODES, FIXTURE_WGSL};

/// The fixture files `names` under `fixtures/print_graph/<dir>/`: each name and text.
fn files(dir: &str, names: &[&str]) -> Vec<(String, String)> {
    names
        .iter()
        .map(|f| ((*f).to_owned(), fixture(&format!("{dir}/{f}"))))
        .collect()
}

/// `shipped` with `more` after it.
fn with(shipped: &[(&str, &str)], more: &[(String, String)]) -> Vec<(String, String)> {
    shipped
        .iter()
        .map(|(n, t)| ((*n).to_owned(), (*t).to_owned()))
        .chain(more.iter().cloned())
        .collect()
}

fn refs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect()
}

/// The check over the shipped files with the fixture entries and the presets `presets` (name, text) added.
fn checked(presets: &[(String, String)]) -> Result<Checked, RegistryError> {
    let nodes = with(NODES, &files("nodes", &FIXTURE_NODES));
    let wgsl = with(WGSL, &files("nodes", &FIXTURE_WGSL));
    let presets = with(PRESETS, presets);
    check(&refs(&nodes), &refs(&wgsl), &refs(&presets))
}

/// The check over `presets` (added to the shipped files and the fixture entries) fails, its error holding each of
/// `named`.
fn check_fails(presets: &[(String, String)], named: &[&str]) {
    match checked(presets) {
        Ok(_) => panic!("the check passed the presets"),
        Err(e) => {
            for n in named {
                assert!(e.0.contains(n), "the check's error does not name {n}: {e}");
            }
        }
    }
}

// ----- the shipped files pass -----

/// The check over the shipped files and `presets` passes, with Plain among the presets.
fn check_passes(presets: &[(String, String)]) {
    let c = checked(presets)
        .unwrap_or_else(|e| panic!("the shipped registry and presets do not pass: {e}"));
    assert!(c.presets.contains_key("Plain"));
}

#[test]
fn print_registry_check_shipped_passes() {
    let c = check(NODES, WGSL, PRESETS).expect("the shipped files pass");
    assert_eq!(&c, shipped());
    let names: Vec<&str> = c.presets.keys().map(String::as_str).collect();
    assert_eq!(names, ["Plain"], "the shipped presets are not Plain alone");
    check_passes(&files("presets", &["Polaroid.json"]));
}

negative_control!(
    print_registry_check_shipped_passes,
    "the shipped files with the preset naming `linocutt` added",
    expected = "the shipped registry and presets do not pass",
    check_passes(&files("presets", &["Typo.json"]))
);

// ----- a bad preset fails, named -----

#[test]
fn print_registry_check_unknown_type_named() {
    check_fails(
        &files("presets", &["Typo.json"]),
        &["preset `Typo`", "node `cut`", "`linocutt`"],
    );
}

negative_control!(
    print_registry_check_unknown_type_named,
    "a preset whose every node is registered",
    expected = "the check passed the presets",
    check_fails(&files("presets", &["Polaroid.json"]), &["`linocutt`"])
);

#[test]
fn print_registry_check_out_of_range_named() {
    check_fails(
        &files("presets", &["Too deep.json"]),
        &[
            "preset `Too deep`",
            "node `cut` (linocut)",
            "parameter `depth` = 7 is outside its range [0, 1]",
        ],
    );
}

negative_control!(
    print_registry_check_out_of_range_named,
    "the same preset with its depth in range",
    expected = "the check passed the presets",
    {
        let text = fixture("presets/Too deep.json").replace("\"depth\":7", "\"depth\":0.5");
        check_fails(&[("Too deep.json".to_owned(), text)], &["`depth`"])
    }
);

// ----- a length without a unit -----

/// The fixture linocut, `edit`ed, fails to register, the refusal holding `why`.
fn check_unregistered(edit: impl Fn(&str) -> String, why: &str) {
    let decl = edit(&fixture("nodes/linocut.json"));
    match Registry::new().register(&decl, &support::wgsl) {
        Ok(_) => panic!("the edited linocut registered"),
        Err(e) => assert!(e.0.contains(why), "the refusal {e} does not say {why:?}"),
    }
}

const GOUGE_UNIT: &str = r#""step": 0.05, "unit": "ref_px""#;

#[test]
fn print_registry_check_length_without_unit() {
    check_unregistered(
        |d| d.replace(GOUGE_UNIT, r#""step": 0.05"#),
        "entry `linocut`: parameter `gouge`: it has no unit",
    );
    check_unregistered(
        |d| d.replace(GOUGE_UNIT, r#""step": 0.05, "unit": "px""#),
        "parameter `gouge`: it has no unit",
    );
}

negative_control!(
    print_registry_check_length_without_unit,
    "the linocut with gouge's unit",
    expected = "the edited linocut registered",
    check_unregistered(|d| d.to_owned(), "no unit")
);

// ----- every other declaration the registry refuses -----

/// Each edit of the fixture linocut's declaration fails to register, the refusal holding the text given with it.
fn check_declarations(cases: &[(&str, &str, &str)]) {
    for (from, to, why) in cases {
        let decl = fixture("nodes/linocut.json");
        assert!(decl.contains(from), "the fixture does not hold {from:?}");
        match Registry::new().register(&decl.replace(from, to), &support::wgsl) {
            Ok(_) => panic!("a declaration that must be refused registered: {from:?} → {to:?}"),
            Err(e) => assert!(
                e.0.contains(why),
                "{from:?} → {to:?}: {e} does not say {why:?}"
            ),
        }
    }
}

const DEPTH: &str = r#""default": 0.7, "min": 0, "max": 1, "step": 0.01"#;

#[test]
fn print_registry_check_declaration_refusals() {
    check_declarations(&[
        (
            r#""id": "linocut""#,
            r#""id": "Linocut""#,
            "not an identifier",
        ),
        (r#""id": "linocut""#, r#""id": "9cut""#, "not an identifier"),
        (r#""id": "linocut""#, r#""id": """#, "not an identifier"),
        (
            r#""category": "print""#,
            r#""category": "source""#,
            "the category is none of",
        ),
        (
            r#""category": "print""#,
            r#""category": 3"#,
            "the category is none of",
        ),
        (
            r#""label": "Linocut""#,
            r#""label": " ""#,
            "`label` is not a non-empty string",
        ),
        (
            r#""label": "Linocut""#,
            r#""label": "Linocut", "x": 1"#,
            "`x` is no member of an entry",
        ),
        (
            "defaults.\"",
            "defaults.\\n\\nA second paragraph.\"",
            "more than one paragraph",
        ),
        (
            r#""doc": "A test"#,
            r#""doc": 1, "d": "A test"#,
            "`d` is no member",
        ),
        (
            r#""cost": "medium""#,
            r#""cost": "cheap""#,
            "the cost is not light, medium or heavy",
        ),
        (
            r#""motion": "reduced""#,
            r#""motion": "half""#,
            "motion policy not full, reduced or bypass",
        ),
        (
            r#""temporally_stable": true"#,
            r#""temporally_stable": "yes""#,
            "each true or false",
        ),
        (
            r#""screen_medium": false"#,
            r#""screen_medium": 0"#,
            "each true or false",
        ),
        (
            r#""deviations": ["#,
            r#""deviations": [1, "#,
            "`deviations` is not a list of strings",
        ),
        (
            r#""passes": ["linocut.wgsl"]"#,
            r#""passes": ["gone.wgsl"]"#,
            "the pass `gone.wgsl` is not there",
        ),
        (
            r#""passes": ["linocut.wgsl"]"#,
            r#""passes": "linocut.wgsl""#,
            "`passes` is not a list",
        ),
        (
            r#""passes": ["linocut.wgsl"]"#,
            r#""passes": ["output.wgsl", "blur_h.wgsl", "Polaroid"]"#,
            "the pass `Polaroid` is not there",
        ),
        (
            r#""name": "flow", "type": "direction""#,
            r#""name": "flow""#,
            "the port `flow` of `inputs` has no type",
        ),
        (
            r#""name": "flow", "type": "direction""#,
            r#""name": "flow", "type": "vector""#,
            "has no type",
        ),
        (
            r#""type": "direction", "default": "black""#,
            r#""type": "direction""#,
            "the input `flow` has no default",
        ),
        (
            r#""type": "direction", "default": "black""#,
            r#""type": "direction", "default": "grey""#,
            "the input `flow` has no default",
        ),
        (r#""name": "flow""#, r#""name": "in""#, "is used twice"),
        (
            r#""name": "flow""#,
            r#""name": "Flow""#,
            "is not an identifier",
        ),
        (
            r#""default": "black" }"#,
            r#""default": "black", "pos": 1 }"#,
            "carries `pos`, which no port has",
        ),
        (
            r#"[{ "name": "out", "type": "image" }]"#,
            r#"[{ "name": "out", "type": "image", "default": "render" }]"#,
            "carries `default`",
        ),
        (
            r#"[{ "name": "out", "type": "image" }]"#,
            r#"[{ "name": "out" }]"#,
            "the port `out` of `outputs` has no type",
        ),
        (
            r#"[{ "name": "out", "type": "image" }]"#,
            r#"{}"#,
            "`outputs` is not a list",
        ),
        (
            r#"[{ "name": "out", "type": "image" }]"#,
            r#"[3]"#,
            "holds 3, not a port",
        ),
        (r#""inputs": ["#, r#""inputs": [3, "#, "holds 3, not a port"),
        (
            DEPTH,
            r#""default": 0.7, "min": 1, "max": 1, "step": 0.01"#,
            "its range [1, 1] is empty",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 1, "max": 0, "step": 0.01"#,
            "its range [1, 0] is empty",
        ),
        (
            DEPTH,
            r#""default": 0.7, "max": 1, "step": 0.01"#,
            "it has no range",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 0, "step": 0.01"#,
            "it has no range",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 0, "max": 1"#,
            "it has no step",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 0, "max": 1, "step": 0"#,
            "it has no step",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 0, "max": 1, "step": -1"#,
            "it has no step",
        ),
        (
            DEPTH,
            r#""default": 1.5, "min": 0, "max": 1, "step": 0.01"#,
            "its default 1.5 is outside its range [0, 1]",
        ),
        (
            DEPTH,
            r#""default": -0.5, "min": 0, "max": 1, "step": 0.01"#,
            "its default -0.5 is outside its range [0, 1]",
        ),
        (
            DEPTH,
            r#""default": "0.7", "min": 0, "max": 1, "step": 0.01"#,
            "it has no default of its kind",
        ),
        (
            DEPTH,
            r#""min": 0, "max": 1, "step": 0.01"#,
            "it has no default of its kind",
        ),
        (
            DEPTH,
            r#""default": 0.7, "min": 0, "max": 1, "step": 0.01, "options": []"#,
            "`options` is no member of a number parameter",
        ),
        (
            r#""kind": "number", "default": 0.7"#,
            r#""kind": "slider", "default": 0.7"#,
            "its kind is not number, colour or choice",
        ),
        (
            r#""name": "depth""#,
            r#""name": "gouge""#,
            "the parameter name \"gouge\" is not an identifier, or is used twice",
        ),
        (
            r#""name": "depth""#,
            r#""name": "Depth""#,
            "is not an identifier",
        ),
        (
            r#""params": ["#,
            r#""params": [3, "#,
            "`params` holds 3, not a parameter",
        ),
        (
            r##""default": "#171414""##,
            r##""default": "#17141""##,
            "is no value of its kind, a colour",
        ),
        (
            r##""default": "#171414""##,
            r#""default": "171414""#,
            "is no value of its kind, a colour",
        ),
        (
            r##""default": "#171414""##,
            r##""default": "#17141g""##,
            "is no value of its kind, a colour",
        ),
        (
            r##""default": "#171414""##,
            r#""default": 3"#,
            "is no value of its kind, a colour",
        ),
        (
            r#""default": 0, "options""#,
            r#""default": 2, "options""#,
            "is no value of its kind, an index of",
        ),
        (
            r#""default": 0, "options""#,
            r#""default": -1, "options""#,
            "is no value of its kind, an index of",
        ),
        (
            r#""default": 0, "options""#,
            r#""default": 0.5, "options""#,
            "is no value of its kind, an index of",
        ),
        (
            r#""options": ["v", "u"]"#,
            r#""options": []"#,
            "it has no options",
        ),
        (
            r#""options": ["v", "u"]"#,
            r#""options": [1]"#,
            "it has no options",
        ),
    ]);
    // A declaration that is not JSON, or not an object; a pass that defines no `run`; and an id taken twice.
    let wgsl = |f: &str| {
        (f == "x.wgsl")
            .then(|| "fn runner(uv: vec2<f32>) -> vec4<f32> { return vec4<f32>(); }".to_owned())
    };
    let mut r = Registry::new();
    assert!(r.register("{", &wgsl).unwrap_err().0.contains("not JSON"));
    assert!(r
        .register("[]", &wgsl)
        .unwrap_err()
        .0
        .contains("not a JSON object"));
    let lino = fixture("nodes/linocut.json");
    let no_run = lino.replace(r#""passes": ["linocut.wgsl"]"#, r#""passes": ["x.wgsl"]"#);
    assert!(r
        .register(&no_run, &wgsl)
        .unwrap_err()
        .0
        .contains("defines no `fn run`"));
    r.register(&lino, &support::wgsl)
        .expect("the linocut registers");
    assert!(r
        .register(&lino, &support::wgsl)
        .unwrap_err()
        .0
        .contains("registered twice"));
    assert!(r.get("linocut").is_some() && r.get("linocutt").is_none());
    // Its ranges' ends and a choice's last index are admitted.
    for (from, to) in [
        (DEPTH, r#""default": 0, "min": 0, "max": 1, "step": 0.01"#),
        (DEPTH, r#""default": 1, "min": 0, "max": 1, "step": 0.01"#),
        (r#""default": 0, "options""#, r#""default": 1, "options""#),
        (r##""default": "#171414""##, r##""default": "#ABCDEF""##),
        ("fn run(", "fn  run ("),
    ] {
        let decl = lino.replace(from, to);
        let read = |f: &str| support::wgsl(f).map(|t| t.replace(from, to));
        Registry::new()
            .register(&decl, &read)
            .unwrap_or_else(|e| panic!("{to}: {e}"));
    }
}

negative_control!(
    print_registry_check_declaration_refusals,
    "the fixture as it is, which registers",
    expected = "a declaration that must be refused registered",
    check_declarations(&[(r#""id": "linocut""#, r#""id": "linocut""#, "")])
);

// ----- preset files -----

#[test]
fn print_registry_check_preset_files() {
    check_preset_refusals(&[
        ("{", "preset file Bad.json: not JSON"),
        (
            &PRESETS[0].1.replace("\"Plain\"", "\"Plain\",\"x\":1"),
            "preset file Bad.json",
        ),
        (PRESETS[0].1, "a preset named `Plain` is already there"),
        (
            &PRESETS[0]
                .1
                .replace("\"Plain\"", "\"P2\"")
                .replace("\"type\":\"render\"", "\"type\":\"preset:Gone\""),
            "preset `P2`: node `render`: there is no preset `Gone`",
        ),
        (
            &PRESETS[0].1.replace("\"Plain\"", "\"P3\"").replace(
                "\"type\":\"render\"",
                "\"type\":\"render\",\"params\":{\"k\":1}",
            ),
            "preset `P3`: node `render` (render): `k` is no parameter of `render`",
        ),
        (
            &PRESETS[0]
                .1
                .replace("\"Plain\"", "\"P4\"")
                .replace("\"type\":\"output\"", "\"type\":\"render\""),
            "preset `P4`: node `out` (render) has no input `in`",
        ),
    ]);
    // A node entry that does not register fails the check, naming its file.
    let err = check(&[("bad.json", "{}")], WGSL, PRESETS).expect_err("an empty entry passed");
    assert!(err.0.starts_with("bad.json: entry `?`: no `id`"), "{err}");
}

/// Each preset text, added to the shipped files as `Bad.json`, fails the check, the error holding the text given.
fn check_preset_refusals(cases: &[(&str, &str)]) {
    for (text, why) in cases {
        let mut presets: Vec<(&str, &str)> = PRESETS.to_vec();
        presets.push(("Bad.json", text));
        match check(NODES, WGSL, &presets) {
            Ok(_) => panic!("a bad preset file passed: {text}"),
            Err(e) => assert!(e.0.contains(why), "{e} does not say {why:?}"),
        }
    }
}

negative_control!(
    print_registry_check_preset_files,
    "a second preset that is well formed",
    expected = "a bad preset file passed",
    check_preset_refusals(&[(&PRESETS[0].1.replace("\"Plain\"", "\"Plain 2\""), "")])
);
