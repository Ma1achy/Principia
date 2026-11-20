//! QA tests for TASK-M0-01 after R-193, written from REQ-SYS-004's statement and verify detail — "in kernel and
//! ledger no unit test uses validation (R-187) ... runs cargo check -p kernel -p ledger --lib --tests --all-features
//! (R-192, so a unit test behind any feature is compiled), and fails ... whatever the route of the use ...; the
//! compile check builds for the host only, so a unit test gated on another platform is not seen, its known limit
//! (R-193)" — not from the implementation.
//!
//! R-193 names one known limit: a unit test gated on another platform. Every other unit test that `cargo test` builds
//! on the host, with the validation dev-dependency, is one R-187 forbids, and the check must see it:
//! - a unit test gated on the host's own platform (the scope R-193 keeps);
//! - a unit test gated on the *absence* of a feature, which plain `cargo test` (default features) builds, and which
//!   `--all-features` alone switches off;
//! - a unit test gated on `not(debug_assertions)`, which `cargo test --release` builds;
//! - a `[lib]` with `test = false` written as an inline or dotted table, and ledger's `[lib] test = false`.
//!
//! Each failing case first shows its premise: `cargo test` (with the flags named) builds and runs that unit test with
//! the dev-dependency, so it is a unit test that uses validation. Control for each (R-176): the same workspace with
//! the use removed passes `xtask deps` with the compile check run.
#![allow(non_snake_case)]

#[path = "support/qa_m0_01_r193.rs"]
mod qa_m0_01_r193;

use qa_m0_01_r193::*;

/// R-193 keeps the host in scope: a unit test gated on the host's own `target_os`, and on its family, is seen.
#[test]
fn qa_a_unit_test_gated_on_the_host_platform_fails() {
    let os = std::env::consts::OS;
    let family = std::env::consts::FAMILY;
    fails_and_its_control_passes(
        "host_os",
        "kernel",
        "",
        "",
        &format!("all(test, target_os = \"{os}\")"),
        &[],
    );
    fails_and_its_control_passes(
        "host_family",
        "ledger",
        "",
        "",
        &format!("all(test, {family})"),
        &[],
    );
}

/// R-192 compiles "a unit test behind any feature"; a unit test behind the absence of a feature is built by plain
/// `cargo test` (default features) with the dev-dependency, and is not gated on another platform (R-193), so the
/// check must see it too.
#[test]
fn qa_a_unit_test_gated_on_a_missing_feature_fails() {
    let features = "\n[features]\ngpu = []\n";
    let cfg = "all(test, not(feature = \"gpu\"))";
    fails_and_its_control_passes("not_feature_kernel", "kernel", "", features, cfg, &[]);
    fails_and_its_control_passes("not_feature_ledger", "ledger", "", features, cfg, &[]);
}

/// `cargo test --release` builds a unit test gated on `not(debug_assertions)` with the dev-dependency, on the host, so
/// it is a unit test that uses validation (R-187) outside R-193's known limit, and the check must see it.
#[test]
fn qa_a_unit_test_gated_on_the_release_profile_fails() {
    fails_and_its_control_passes(
        "release_kernel",
        "kernel",
        "",
        "",
        "all(test, not(debug_assertions))",
        &["--release"],
    );
}

/// `test = false` on the library, written as a `[lib]` table for ledger, and as an inline (`lib = { … }`) or dotted
/// (`lib.test = …`) key for kernel: `cargo test --lib` still builds the unit test with the dev-dependency, so the check
/// must fail.
#[test]
fn qa_a_lib_with_test_false_in_every_toml_form_fails() {
    fails_and_its_control_passes(
        "lib_table_ledger",
        "ledger",
        "",
        "\n[lib]\ntest = false\n",
        "test",
        &[],
    );
    fails_and_its_control_passes(
        "lib_inline_kernel",
        "kernel",
        "lib = { test = false }\n\n",
        "",
        "test",
        &[],
    );
    fails_and_its_control_passes(
        "lib_dotted_kernel",
        "kernel",
        "lib.test = false\n\n",
        "",
        "test",
        &[],
    );
}
