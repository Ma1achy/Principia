//! Negative controls for qa's tests in `qa_TASK-M0-01_r191.rs` (REQ-VAL-007; R-199, R-209, R-212): each runs its
//! test's check, called from the shared module its test calls too (REQ-VAL-161; R-215, R-218), on a workspace the
//! check must reject, so the test can fail (philosophy §4.4). A case's use of validation is taken out by rewriting the
//! shared `UNIT_USE`; where a test runs several checks, its control passes the earlier ones and trips the last, as the
//! test would. Workspaces are named `m022_*`, apart from qa's cases, which write to the same directory.
#![cfg(feature = "controls")]

#[path = "support/qa_m0_01_r191.rs"]
mod qa_m0_01_r191;

use qa_m0_01_r191::*;
use validation::negative_control;

/// `UNIT_USE` with its use of validation taken out: the same unit test, compiling without the crate.
fn unused() -> String {
    UNIT_USE.replace("validation::Harness", "()")
}

negative_control!(
    qa_unit_test_use_in_kernel_or_ledger_fails_and_its_integration_test_passes,
    "the use in an integration test of kernel, which passes, given to the unit-test check",
    expected = "m022_integration: a unit test of kernel that uses validation passes xtask deps",
    {
        let cargo = kernel_manifest(VALIDATION_DEV);
        let root = workspace(
            "m022_integration",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/tests/m022.rs", EXTERNAL_USE),
            ],
        );
        passes_the_compile_check("m022_integration", &root);
        compiles_with_the_dependency("m022_integration", &root, "kernel");
        fails_with_compiler_error("m022_integration", &root, "kernel", "tests/m022.rs");
    }
);

negative_control!(
    qa_every_form_of_the_dev_dependency_is_removed,
    "a renamed dev-dependency on validation that kernel's unit test does not use",
    expected = "m022_renamed: a unit test of kernel that uses validation passes xtask deps",
    {
        let cargo = kernel_manifest(
            "\n[dev-dependencies]\nm022 = { package = \"validation\", path = \"../validation\" }\n",
        );
        let root = workspace(
            "m022_renamed",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/src/lib.rs", &unused()),
            ],
        );
        fails_with_compiler_error("m022_renamed", &root, "kernel", "crates/kernel/src/lib.rs");
    }
);

negative_control!(
    qa_unit_test_in_a_kernel_binary_fails,
    "a kernel binary whose unit test does not use validation",
    expected = "m022_bin: a unit test of kernel that uses validation passes xtask deps",
    {
        let cargo = kernel_manifest(VALIDATION_DEV);
        let main = format!("fn main() {{}}\n\n{}", unused());
        let root = workspace(
            "m022_bin",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/src/bin/m022.rs", &main),
            ],
        );
        fails_with_compiler_error("m022_bin", &root, "kernel", "crates/kernel/src/bin/m022.rs");
    }
);

negative_control!(
    qa_unit_test_of_a_lib_with_test_false_fails,
    "a lib with `test = false` whose unit test, which `cargo test --lib` runs, does not use validation",
    expected = "in a lib with `test = false`, passes xtask deps",
    {
        let cargo = kernel_manifest(&format!(
            "{VALIDATION_DEV}\n[lib]\ntest = false\ndoctest = false\n"
        ));
        let root = workspace(
            "m022_lib_test_false",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/src/lib.rs", &unused()),
            ],
        );
        premise_cargo_test_lib_runs_the_unit_test(&root);
        check_lib_test_false_fails(&root);
    }
);

negative_control!(
    qa_ledger_unit_test_behind_a_feature_fails,
    "a ledger unit test behind a feature that does not use validation",
    expected = "m022_ledger_feature: a unit test of ledger that uses validation passes xtask deps",
    {
        let cargo = manifest(
            "ledger",
            &format!("\n[features]\nm022 = []\n{VALIDATION_DEV}"),
        );
        let gated = unused().replace("#[cfg(test)]", "#[cfg(all(test, feature = \"m022\"))]");
        let root = workspace(
            "m022_ledger_feature",
            &[
                ("crates/ledger/Cargo.toml", &cargo),
                ("crates/ledger/src/lib.rs", &gated),
            ],
        );
        passes_the_compile_check("m022_ledger_feature", &root);
        fails_with_compiler_error(
            "m022_ledger_feature",
            &root,
            "ledger",
            "crates/ledger/src/lib.rs",
        );
    }
);

negative_control!(
    qa_path_and_include_routes_fail_only_when_the_loaded_file_uses_validation,
    "a `#[path]` route in ledger loading a unit test that does not use validation",
    expected = "m022_route: a unit test of ledger that uses validation passes xtask deps",
    {
        let cargo = manifest("ledger", VALIDATION_DEV);
        let root = workspace(
            "m022_route",
            &[
                ("crates/ledger/Cargo.toml", &cargo),
                (
                    "crates/ledger/src/lib.rs",
                    "#[cfg(test)]\n#[path = \"../m022/t.rs\"]\nmod t;\n",
                ),
                ("crates/ledger/m022/t.rs", "#[test]\nfn t() {}\n"),
            ],
        );
        fails_with_compiler_error("m022_route", &root, "ledger", "m022/t.rs");
    }
);

negative_control!(
    qa_integration_tests_examples_and_benches_may_use_validation,
    "an integration test that uses validation beside a unit test that uses it too",
    expected = "m022_external_and_unit: xtask deps fails",
    {
        let cargo = kernel_manifest(VALIDATION_DEV);
        let root = workspace(
            "m022_external_and_unit",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/tests/m022/main.rs", EXTERNAL_USE),
                ("crates/kernel/src/lib.rs", UNIT_USE),
            ],
        );
        compiles_with_the_dependency("m022_external_and_unit", &root, "kernel");
        passes_the_compile_check("m022_external_and_unit", &root);
    }
);

negative_control!(
    qa_the_check_leaves_the_workspace_unchanged_and_uses_a_stable_target_dir_under_target,
    "the directories under the target after the first run, compared with those from before it",
    expected = "the second run used a different target directory: not stable",
    {
        let cargo = kernel_manifest(VALIDATION_DEV);
        let root = workspace(
            "m022_unchanged",
            &[
                ("crates/kernel/Cargo.toml", &cargo),
                ("crates/kernel/src/lib.rs", UNIT_USE),
            ],
        );
        let target = target_directory(&root);
        let before = dirs(&target);
        let manifest_before = read(&root, "crates/kernel/Cargo.toml");
        check_the_use_fails(&root);
        check_unchanged(&root, "crates/kernel/Cargo.toml", &manifest_before);
        check_no_orig(&root);
        check_a_new_target_dir(&target, &before, &dirs(&target));
        check_the_same_target_dir(&target, &before);
    }
);
