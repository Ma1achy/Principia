//! QA tests for TASK-M0-01, R-187's integration-test condition as R-191 and R-192 check it, written from
//! REQ-SYS-004's verify detail — "in kernel and ledger no unit test uses validation (R-187): when either takes
//! validation as a dev-dependency, cargo xtask deps copies the workspace to a temporary directory, removes validation
//! from their dev-dependencies there and runs cargo check -p kernel -p ledger --lib --tests --all-features (R-192),
//! and fails, showing the compiler's error, when that does not compile, whatever the route of the use (alias, macro,
//! #[path], include!) (R-191, which lifts R-189's and R-190's token rules)" — and the task's Deliverables ("with a
//! stable `CARGO_TARGET_DIR` under `target/`"), not from the implementation.
//!
//! These replace qa's token-level tests of R-187 to R-190 (R-191's one-round exception). Each case is a real cargo
//! workspace on disk: `ledger`; `kernel`, with `ledger` as its build-dependency (R-185); `validation`, depending on
//! both. `xtask deps --manifest-path` runs on it.
//!
//! Controls (R-176): every failing case is paired with (1) a plain `cargo check --tests` of the same workspace, with
//! its dev-dependency, which must pass, so the use is real and compiles where R-187 allows it, and (2) where the case
//! is about the place of the use, the same workspace with the use moved or removed, which must pass `xtask deps`.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r191.rs"]
mod qa_m0_01_r191;

use qa_m0_01_r191::*;

/// R-187 for ledger as for kernel, in the crate root and in a nested module file; the failure shows the compiler's
/// error at that file. Control: the same use in `tests/` passes the check.
#[test]
fn qa_unit_test_use_in_kernel_or_ledger_fails_and_its_integration_test_passes() {
    for krate in ["kernel", "ledger"] {
        let cargo = format!("crates/{krate}/Cargo.toml");
        let text = if krate == "kernel" {
            kernel_manifest(VALIDATION_DEV)
        } else {
            manifest(krate, VALIDATION_DEV)
        };
        for (at, lib, file) in [
            (
                "root",
                UNIT_USE.to_owned(),
                format!("crates/{krate}/src/lib.rs"),
            ),
            (
                "nested",
                "mod a;\n".to_owned(),
                format!("crates/{krate}/src/a/b.rs"),
            ),
        ] {
            let case = format!("{krate}_{at}");
            let lib_path = format!("crates/{krate}/src/lib.rs");
            let root = workspace(
                &case,
                &[
                    (&cargo, &text),
                    (&lib_path, &lib),
                    (&format!("crates/{krate}/src/a/mod.rs"), "mod b;\n"),
                    (&format!("crates/{krate}/src/a/b.rs"), UNIT_USE),
                ],
            );
            fails_with_compiler_error(&case, &root, krate, &file);
        }
        let case = format!("{krate}_integration");
        let root = workspace(
            &case,
            &[
                (&cargo, &text),
                (&format!("crates/{krate}/tests/uses.rs"), EXTERNAL_USE),
            ],
        );
        passes_the_compile_check(&case, &root);
        compiles_with_the_dependency(&case, &root, krate);
    }
}

/// "removes validation from their dev-dependencies" covers every way a manifest declares that dev-dependency: renamed
/// (`harness = { package = "validation" }`), inherited from the workspace (`validation.workspace = true`), inherited
/// and renamed, and in a platform table (`[target.'cfg(all())'.dev-dependencies]`, which applies on every host).
/// Control for each: `compiles_with_the_dependency`; and the same manifest with a unit test that does not use the
/// crate passes the check.
#[test]
fn qa_every_form_of_the_dev_dependency_is_removed() {
    let ws_members = "[workspace]\nresolver = \"2\"\nmembers = [\"crates/ledger\", \"crates/kernel\", \"crates/validation\"]\n";
    let ws_dep = format!(
        "{ws_members}\n[workspace.dependencies]\nvalidation = {{ path = \"crates/validation\" }}\n"
    );
    let ws_renamed =
        format!("{ws_members}\n[workspace.dependencies]\nharness = {{ package = \"validation\", path = \"crates/validation\" }}\n");
    let harness_use = UNIT_USE.replace("validation::", "harness::");
    let cases: [(&str, &str, String, String); 4] = [
        (
            "renamed",
            ws_members,
            kernel_manifest("\n[dev-dependencies]\nharness = { package = \"validation\", path = \"../validation\" }\n"),
            harness_use.clone(),
        ),
        ("inherited", &ws_dep, kernel_manifest("\n[dev-dependencies]\nvalidation.workspace = true\n"), UNIT_USE.into()),
        (
            "inherited_renamed",
            &ws_renamed,
            kernel_manifest("\n[dev-dependencies]\nharness = { workspace = true }\n"),
            harness_use,
        ),
        (
            "platform_table",
            ws_members,
            kernel_manifest("\n[target.'cfg(all())'.dev-dependencies]\nvalidation = { path = \"../validation\" }\n"),
            UNIT_USE.into(),
        ),
    ];
    for (case, ws, kernel, lib) in cases {
        let files = |lib: &str| {
            vec![
                ("Cargo.toml".to_owned(), ws.to_owned()),
                ("crates/kernel/Cargo.toml".to_owned(), kernel.clone()),
                ("crates/kernel/src/lib.rs".to_owned(), lib.to_owned()),
            ]
        };
        let as_refs = |v: &[(String, String)]| {
            v.iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect::<Vec<_>>()
        };
        let with = as_refs(&files(&lib));
        let with: Vec<(&str, &str)> = with.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let root = workspace(case, &with);
        fails_with_compiler_error(case, &root, "kernel", "crates/kernel/src/lib.rs");

        let control = format!("{case}_control");
        let without = as_refs(&files(
            "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
        ));
        let without: Vec<(&str, &str)> = without
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        passes_the_compile_check(&control, &workspace(&control, &without));
    }
}

/// `--tests` compiles the unit tests of binary targets too: a unit test in kernel's `src/main.rs` or in
/// `src/bin/tool.rs` that uses validation fails. Control: the same binaries without the use pass.
#[test]
fn qa_unit_test_in_a_kernel_binary_fails() {
    let cargo = kernel_manifest(VALIDATION_DEV);
    let main = format!("fn main() {{}}\n\n{UNIT_USE}");
    let plain = "fn main() {}\n";
    for (case, file) in [
        ("bin_main", "crates/kernel/src/main.rs"),
        ("bin_tool", "crates/kernel/src/bin/tool.rs"),
    ] {
        let root = workspace(case, &[("crates/kernel/Cargo.toml", &cargo), (file, &main)]);
        fails_with_compiler_error(case, &root, "kernel", file);
        let control = format!("{case}_control");
        passes_the_compile_check(
            &control,
            &workspace(
                &control,
                &[("crates/kernel/Cargo.toml", &cargo), (file, plain)],
            ),
        );
    }
}

/// `[lib] test = false` stops `cargo test` from building the unit tests by default, but `cargo test -p kernel --lib`
/// still builds and runs them (premise, checked below), with the validation dev-dependency, so they are unit tests
/// that use validation (R-187), and the check must fail. Control: the same manifest without the use passes.
#[test]
fn qa_unit_test_of_a_lib_with_test_false_fails() {
    let cargo = kernel_manifest(&format!("\n[lib]\ntest = false\n{VALIDATION_DEV}"));
    let root = workspace(
        "lib_test_false",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    );
    // Premise: the unit test is built and run by `cargo test --lib`.
    premise_cargo_test_lib_runs_the_unit_test(&root);
    check_lib_test_false_fails(&root);
    let control = workspace(
        "lib_test_false_control",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", "pub fn f() {}\n"),
        ],
    );
    passes_the_compile_check("lib_test_false_control", &control);
}

/// R-192 for ledger: a ledger unit test behind a ledger feature (off by default) that uses validation fails.
/// Control: the same feature-gated test without the use passes.
#[test]
fn qa_ledger_unit_test_behind_a_feature_fails() {
    let cargo = manifest(
        "ledger",
        &format!("\n[features]\nslow = []\n{VALIDATION_DEV}"),
    );
    let gated = UNIT_USE.replace("#[cfg(test)]", "#[cfg(all(test, feature = \"slow\"))]");
    let root = workspace(
        "ledger_feature",
        &[
            ("crates/ledger/Cargo.toml", &cargo),
            ("crates/ledger/src/lib.rs", &gated),
        ],
    );
    fails_with_compiler_error(
        "ledger_feature",
        &root,
        "ledger",
        "crates/ledger/src/lib.rs",
    );
    let plain = "pub fn f() {}\n\n#[cfg(all(test, feature = \"slow\"))]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    let control = workspace(
        "ledger_feature_control",
        &[
            ("crates/ledger/Cargo.toml", &cargo),
            ("crates/ledger/src/lib.rs", plain),
        ],
    );
    passes_the_compile_check("ledger_feature_control", &control);
}

/// The routes qa's token-level tests used (R-188 to R-190), now checked by compiling: in ledger, a `#[path]` under
/// `cfg_attr`, and an `include!` imported under another name; in kernel, `include` passed to a macro as an ident, a
/// `path = …` passed as a `meta` fragment, and an `include!` in a macro defined in one file and invoked in another.
/// Each loads a file outside `src/` whose unit test uses validation, and must fail. Control: R-191 lifts the token
/// rules, so the same route loading the same file without the use passes.
#[test]
fn qa_path_and_include_routes_fail_only_when_the_loaded_file_uses_validation() {
    let used = "#[test]\nfn t() {\n    let _ = validation::Harness;\n}\n";
    let unused = "#[test]\nfn t() {}\n";
    let routes: [(&str, &str, &str, Option<&str>); 5] = [
        ("ledger", "cfg_attr_path", "#[cfg(test)]\n#[cfg_attr(all(), path = \"../gen/t.rs\")]\nmod t;\n", None),
        (
            "ledger",
            "include_alias",
            "use std::include as inc;\n\n#[cfg(test)]\nmod t {\n    super::inc!(\"../gen/t.rs\");\n}\n",
            None,
        ),
        (
            "kernel",
            "macro_ident",
            "macro_rules! load {\n    ($i:ident) => {\n        #[cfg(test)]\n        mod t {\n            \
             $i!(\"../gen/t.rs\");\n        }\n    };\n}\n\nload!(include);\n",
            None,
        ),
        (
            "kernel",
            "meta_fragment",
            "macro_rules! load {\n    ($a:meta) => {\n        #[cfg(test)]\n        #[$a]\n        mod t;\n    };\n}\n\n\
             load!(path = \"../gen/t.rs\");\n",
            None,
        ),
        (
            "kernel",
            "macro_elsewhere",
            "#[macro_use]\nmod m;\n\n#[cfg(test)]\nmod t {\n    load!();\n}\n",
            Some("macro_rules! load {\n    () => {\n        include!(\"../gen/t.rs\");\n    };\n}\n"),
        ),
    ];
    for (krate, route, lib, macros) in routes {
        let cargo_path = format!("crates/{krate}/Cargo.toml");
        let cargo = if krate == "kernel" {
            kernel_manifest(VALIDATION_DEV)
        } else {
            manifest(krate, VALIDATION_DEV)
        };
        let lib_path = format!("crates/{krate}/src/lib.rs");
        let mod_path = format!("crates/{krate}/src/m.rs");
        let gen_path = format!("crates/{krate}/gen/t.rs");
        let files = |body: &'static str| -> Vec<(String, String)> {
            let mut v = vec![
                (cargo_path.clone(), cargo.clone()),
                (lib_path.clone(), lib.to_owned()),
                (gen_path.clone(), body.to_owned()),
            ];
            if let Some(m) = macros {
                v.push((mod_path.clone(), m.to_owned()));
            }
            v
        };
        let with = files(used);
        let with: Vec<(&str, &str)> = with.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let case = format!("route_{route}");
        let root = workspace(&case, &with);
        fails_with_compiler_error(&case, &root, krate, "gen/t.rs");

        let without = files(unused);
        let without: Vec<(&str, &str)> = without
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let control = format!("route_{route}_control");
        passes_the_compile_check(&control, &workspace(&control, &without));
    }
}

/// Integration tests, examples and benches are not unit tests: R-187 lets them use validation, however they are
/// declared (auto-discovered, `tests/<dir>/main.rs`, or an explicit `[[test]]`, `[[example]]` or `[[bench]]`).
/// Control: `compiles_with_the_dependency` shows each does use it; the unit-test cases above show the check can fail.
#[test]
fn qa_integration_tests_examples_and_benches_may_use_validation() {
    let example = "fn main() {\n    let _ = validation::Harness;\n}\n";
    let explicit = kernel_manifest(&format!(
        "{VALIDATION_DEV}\n[[test]]\nname = \"explicit\"\npath = \"checks/explicit.rs\"\n\n\
         [[example]]\nname = \"demo\"\npath = \"demos/demo.rs\"\n\n[[bench]]\nname = \"b\"\npath = \"perf/b.rs\"\nharness = false\n"
    ));
    let auto = kernel_manifest(VALIDATION_DEV);
    let cases: [(&str, Vec<(&str, &str)>); 2] = [
        (
            "auto",
            vec![
                ("crates/kernel/Cargo.toml", &auto),
                ("crates/kernel/tests/dir/main.rs", EXTERNAL_USE),
                ("crates/kernel/examples/e.rs", example),
                ("crates/kernel/benches/b.rs", example),
            ],
        ),
        (
            "explicit",
            vec![
                ("crates/kernel/Cargo.toml", &explicit),
                ("crates/kernel/checks/explicit.rs", EXTERNAL_USE),
                ("crates/kernel/demos/demo.rs", example),
                ("crates/kernel/perf/b.rs", example),
            ],
        ),
    ];
    for (case, files) in cases {
        let case = format!("external_{case}");
        let root = workspace(&case, &files);
        compiles_with_the_dependency(&case, &root, "kernel");
        passes_the_compile_check(&case, &root);
    }
}

/// The check edits a copy, never the workspace: after a failing and a passing run, kernel's manifest and sources are
/// byte-identical. And it compiles with a stable `CARGO_TARGET_DIR` under the workspace's target directory (task
/// Deliverables): a directory created there by the run, the same one on a second run. Control on the premise: before
/// the first run, the target directory has no such new directory.
#[test]
fn qa_the_check_leaves_the_workspace_unchanged_and_uses_a_stable_target_dir_under_target() {
    let cargo = kernel_manifest(VALIDATION_DEV);
    let root = workspace(
        "unchanged",
        &[
            ("crates/kernel/Cargo.toml", &cargo),
            ("crates/kernel/src/lib.rs", UNIT_USE),
        ],
    );
    let target = target_directory(&root);
    // This workspace's own target directory is inside it (no CARGO_TARGET_DIR in force) only if cargo says so; the
    // check's directory must be under whatever cargo reports.
    let before = dirs(&target);

    let (manifest_before, lib_before) = (
        read(&root, "crates/kernel/Cargo.toml"),
        read(&root, "crates/kernel/src/lib.rs"),
    );
    check_the_use_fails(&root);
    check_unchanged(&root, "crates/kernel/Cargo.toml", &manifest_before);
    check_unchanged(&root, "crates/kernel/src/lib.rs", &lib_before);
    check_no_orig(&root);

    let after_first = dirs(&target);
    check_a_new_target_dir(&target, &before, &after_first);
    std::fs::write(root.join("crates/kernel/src/lib.rs"), "pub fn f() {}\n").unwrap();
    passes_the_compile_check("unchanged_second", &root);
    check_the_same_target_dir(&target, &after_first);
    check_unchanged(&root, "crates/kernel/Cargo.toml", &manifest_before);
}
