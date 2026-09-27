//! QA tests for TASK-M0-28, written from REQ-VAL-158 and REQ-VAL-159 and the rulings they cite (R-215, R-218):
//! - REQ-VAL-158: "a check copied into a controls target (`crates/validation/tests/controls.rs`'s copies of the `gpu`
//!   and `prop` unit-test checks) must be replaced by the test's own check", verified as "no check body is copied into
//!   a controls target; every registered control still trips its test's assertion";
//! - REQ-VAL-159: "an input a control must share with its test (shader text, a fixture, a constant) must live once, in
//!   the shared test-support module that both use ...; the duplicated GPU shader text in
//!   `crates/validation/tests/qa_TASK-M0-04_controls.rs` moves there", verified as "no input the test and its control
//!   both need is copied between qa's test files and the controls targets".
//!
//! The tests read the source files the requirements name. Whole-line comments are dropped before scanning, so a doc
//! comment that mentions a call or a macro is not read as one. Each test registers a negative control (R-176, R-199,
//! R-212) that runs the same check on the source with the forbidden shape (a copied check, a copied shader) added,
//! and trips that check's assertion.

use std::path::Path;
use validation::negative_control;

/// The file at `rel` (from the validation crate's root), read to a string.
fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// `text` without its whole-line comments (`//`, `///`, `//!`).
fn code(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each `negative_control!` in `text`: the test it names and its body (the text up to the macro call's closing `);`
/// at the start of a line).
fn controls_in(text: &str) -> Vec<(String, String)> {
    code(text)
        .split("negative_control!(")
        .skip(1)
        .map(|chunk| {
            let name: String = chunk
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let body = chunk.split("\n);").next().unwrap_or(chunk).to_owned();
            (name, body)
        })
        .collect()
}

/// The names of the `check_*` functions called in `text`.
fn check_calls(text: &str) -> Vec<String> {
    let mut calls: Vec<String> = text
        .match_indices("check_")
        .filter(|(i, _)| {
            // A whole identifier, not the tail of one.
            !text[..*i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .filter_map(|(i, _)| {
            let ident: String = text[i..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            text[i + ident.len()..].starts_with('(').then_some(ident)
        })
        .collect();
    calls.sort();
    calls.dedup();
    calls
}

/// The body of the unit test `fn name()` in `src` (up to the end of the function at the `mod tests` indent).
fn unit_test_body<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let start = src.find(&format!("fn {name}() {{"))?;
    let rest = &src[start..];
    Some(&rest[..rest.find("\n    }\n").unwrap_or(rest.len())])
}

/// The bodies of every `pub mod checks` in `src` (each up to the `mod tests` that follows it: a closing brace at
/// column 0 also ends a line of the WGSL the module holds), joined.
fn checks_modules(src: &str) -> String {
    let bodies: Vec<&str> = src
        .match_indices("pub mod checks {")
        .map(|(start, _)| {
            let rest = &src[start..];
            &rest[..rest.find("\nmod tests {").unwrap_or(rest.len())]
        })
        .collect();
    assert!(
        !bodies.is_empty(),
        "no `pub mod checks` in the unit tests' source"
    );
    bodies.join("\n")
}

/// REQ-VAL-158's first half: the controls target holds no assertion of its own, so no check body is copied into it;
/// every check its controls run is called, not written there.
fn check_no_check_copied_into(controls: &str) {
    let copied: Vec<String> = code(controls)
        .lines()
        .filter(|l| {
            ["assert!", "assert_eq!", "assert_ne!", "panic!("]
                .iter()
                .any(|m| l.contains(m))
        })
        .map(str::to_owned)
        .collect();
    assert!(
        copied.is_empty(),
        "the controls target copies a check:\n{}",
        copied.join("\n")
    );
}

/// REQ-VAL-158's second half: each control in the controls target names a unit test in `src`, and calls a check of the
/// shared `checks` module that its test calls too, so it trips the test's own assertion.
fn check_controls_call_their_tests_checks(src: &str, controls: &str) {
    let found = controls_in(controls);
    assert!(
        !found.is_empty(),
        "no negative_control! in the controls target"
    );
    let checks = checks_modules(src);
    let defined: Vec<String> = checks
        .match_indices("pub fn check_")
        .map(|(i, _)| {
            let s = &checks[i + "pub fn ".len()..];
            s[..s.find('(').unwrap_or(s.len())].to_owned()
        })
        .collect();
    for (name, body) in found {
        let test = unit_test_body(src, &name)
            .unwrap_or_else(|| panic!("control `{name}` names no unit test in gpu.rs or prop.rs"));
        let test_calls = check_calls(test);
        let common: Vec<String> = check_calls(&body)
            .into_iter()
            .filter(|c| test_calls.contains(c) && defined.contains(c))
            .collect();
        assert!(
            !common.is_empty(),
            "control `{name}` calls none of its test's shared checks (test calls {test_calls:?})"
        );
    }
}

/// The unit tests' source: `src/gpu.rs` and `src/prop.rs`, whose `checks` modules hold the shared checks.
fn unit_tests_src() -> String {
    format!("{}\n{}", read("src/gpu.rs"), read("src/prop.rs"))
}

/// REQ-VAL-158: `tests/controls.rs` holds no copy of a `gpu` or `prop` unit-test check.
#[test]
fn qa_m0_28_controls_target_copies_no_check() {
    check_no_check_copied_into(&read("tests/controls.rs"));
}

negative_control!(
    qa_m0_28_controls_target_copies_no_check,
    "the controls target with a control that copies its test's assertion inline, as before TASK-M0-28",
    expected = "the controls target copies a check",
    check_no_check_copied_into(&format!(
        "{}\nnegative_control!(\n    gpu_backend_env_selects_backend,\n    \"d\",\n    expected = \"e\",\n    \
         assert_eq!(backend_from(Some(\"metal\")), Ok(wgpu::Backends::VULKAN), \"control: copied\")\n);\n",
        read("tests/controls.rs")
    ))
);

/// REQ-VAL-158: every control in `tests/controls.rs` calls a check from `gpu::checks` or `prop::checks` that the unit
/// test it names calls too.
#[test]
fn qa_m0_28_each_control_calls_its_tests_shared_check() {
    check_controls_call_their_tests_checks(&unit_tests_src(), &read("tests/controls.rs"));
}

negative_control!(
    qa_m0_28_each_control_calls_its_tests_shared_check,
    "a control of gpu_backend_env_selects_backend that calls a check its test does not",
    expected = "control `gpu_backend_env_selects_backend` calls none of its test's shared checks",
    check_controls_call_their_tests_checks(
        &unit_tests_src(),
        "negative_control!(\n    gpu_backend_env_selects_backend,\n    \"d\",\n    expected = \"e\",\n    \
         check_no_persistence(&Config::default())\n);\n"
    )
);

/// qa's M0-04 test file and its controls target, and the implementer's controls target: none may hold shader text
/// of its own (REQ-VAL-159).
const SHADER_USERS: &[&str] = &[
    "tests/qa_TASK-M0-04.rs",
    "tests/qa_TASK-M0-04_controls.rs",
    "tests/controls.rs",
];

/// The shared test-support module of qa's M0-04 files, which holds their shader text (REQ-VAL-159).
const SHADER_MODULE: &str = "tests/support/qa_m0_04.rs";

/// REQ-VAL-159: no file in `files` (name, text) holds WGSL; it takes its shader text from the shared module.
fn check_no_shader_text_in(files: &[(&str, String)]) {
    for (name, text) in files {
        let wgsl: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("@compute") || l.contains("@group(") || l.contains("@builtin("))
            .collect();
        assert!(
            wgsl.is_empty(),
            "{name} holds shader text outside the shared module:\n{}",
            wgsl.join("\n")
        );
    }
}

/// REQ-VAL-159: every `run_wgsl(MODULE, "entry", …)` in `files` dispatches an entry point of a `pub const MODULE`
/// in `shared`, so the test and its controls run one text; each file dispatches at least once.
fn check_dispatches_resolve_in(files: &[(&str, String)], shared: &str) {
    for (name, text) in files {
        let text = code(text);
        let dispatches: Vec<(String, String)> = text
            .split("run_wgsl(")
            .skip(1)
            .map(|chunk| {
                let module = chunk.split(',').next().unwrap_or("").trim().to_owned();
                let entry = chunk.split('"').nth(1).unwrap_or("").to_owned();
                (module, entry)
            })
            .collect();
        assert!(!dispatches.is_empty(), "{name} dispatches no shader");
        for (module, entry) in dispatches {
            let decl = format!("pub const {module}: &str = r\"");
            let at = shared.find(&decl).unwrap_or_else(|| {
                panic!("{name}: `{module}` is not a shader of the shared module")
            });
            let body = &shared[at + decl.len()..];
            let body = &body[..body.find("\";").unwrap_or(body.len())];
            assert!(
                body.contains(&format!("fn {entry}(")),
                "{name}: entry `{entry}` of `{module}` is not in the shared module's text"
            );
        }
    }
}

fn shader_users() -> Vec<(&'static str, String)> {
    SHADER_USERS.iter().map(|f| (*f, read(f))).collect()
}

/// REQ-VAL-159: the shader text lives in the shared module only.
#[test]
fn qa_m0_28_no_shader_text_outside_the_shared_module() {
    check_no_shader_text_in(&shader_users());
}

negative_control!(
    qa_m0_28_no_shader_text_outside_the_shared_module,
    "qa's M0-04 controls target with its pre-TASK-M0-28 copy of the faulty kernels",
    expected = "tests/qa_TASK-M0-04_controls.rs holds shader text outside the shared module",
    {
        let mut files = shader_users();
        files[1].1.push_str(
            "\nconst FAULTY: &str = r\"\n@group(0) @binding(0) var<storage, read> input: array<u32>;\n\
             @compute @workgroup_size(64)\nfn skip_last(@builtin(global_invocation_id) id: vec3<u32>) {}\n\";\n",
        );
        check_no_shader_text_in(&files);
    }
);

/// REQ-VAL-159: qa's M0-04 test and its controls dispatch the shared module's shaders, each entry point found there.
#[test]
fn qa_m0_28_dispatched_shaders_are_the_shared_modules() {
    check_dispatches_resolve_in(&shader_users()[..2], &read(SHADER_MODULE));
}

negative_control!(
    qa_m0_28_dispatched_shaders_are_the_shared_modules,
    "the controls target dispatching a local copy of the faulty kernels, as before TASK-M0-28",
    expected = "`FAULTY` is not a shader of the shared module",
    {
        let mut files = shader_users();
        files[1].1.push_str(
            "\nfn f(h: &GpuHarness) { h.run_wgsl(FAULTY, \"skip_last\", &[&[1u32]]); }\n",
        );
        check_dispatches_resolve_in(&files[..2], &read(SHADER_MODULE));
    }
);
