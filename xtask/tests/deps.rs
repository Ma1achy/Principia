//! `cargo xtask deps` against the metadata fixtures (REQ-SYS-004): the workspace graph passes, and each
//! fixture that adds one forbidden edge fails, naming the edge and its dependency kind.

use std::path::PathBuf;
use std::process::Command;

use xtask::deps::{check, DepKind, Edge, Metadata};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Runs `xtask deps --metadata <fixture>`; returns (success, stderr).
fn run_deps(name: &str) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(fixture(name))
        .output()
        .expect("run xtask");
    (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
}

fn assert_fails_naming(name: &str, expected: &str) {
    let (ok, stderr) = run_deps(name);
    assert!(!ok, "{name}: xtask deps passed, but the fixture has a forbidden edge");
    assert!(stderr.contains(expected), "{name}: stderr does not name {expected:?}:\n{stderr}");
    assert_eq!(
        stderr.matches("forbidden edge").count(),
        1,
        "{name}: expected exactly one forbidden edge:\n{stderr}"
    );
}

#[test]
fn deps_workspace_fixture_passes() {
    let (ok, stderr) = run_deps("metadata_workspace.json");
    assert!(ok, "the workspace graph fixture fails:\n{stderr}");
}

#[test]
fn deps_forbidden_kernel_to_engine_fails() {
    assert_fails_naming("metadata_kernel_engine.json", "forbidden edge kernel → engine (normal dependency)");
}

#[test]
fn deps_forbidden_engine_to_gui_fails() {
    assert_fails_naming("metadata_engine_gui.json", "forbidden edge engine → gui (normal dependency)");
}

#[test]
fn deps_forbidden_ledger_to_engine_fails() {
    assert_fails_naming("metadata_ledger_engine.json", "forbidden edge ledger → engine (normal dependency)");
}

#[test]
fn deps_forbidden_kernel_to_ledger_normal_fails_naming_kind() {
    assert_fails_naming(
        "metadata_kernel_ledger_normal.json",
        "forbidden edge kernel → ledger (normal dependency): kernel → ledger is a build-dependency only (R-185)",
    );
}

#[test]
fn deps_workspace_fixture_has_kernel_ledger_as_build_dependency() {
    let metadata = Metadata::from_file(&fixture("metadata_workspace.json")).unwrap();
    let edges = metadata.edges().unwrap();
    assert!(edges.contains(&edge("kernel", "ledger", DepKind::Build)));
    assert!(!edges.iter().any(|e| e.from == "ledger"), "ledger must have no workspace dependency");
    // Non-workspace dependencies (xtask's serde, serde_json) are not edges.
    assert!(!edges.iter().any(|e| e.to.starts_with("serde")));
}

#[test]
fn deps_live_workspace_passes() {
    let edges = Metadata::from_cargo().unwrap().edges().unwrap();
    assert_eq!(check(&edges), vec![], "the workspace crate graph has a forbidden edge");
}

fn edge(from: &str, to: &str, kind: DepKind) -> Edge {
    Edge { from: from.to_owned(), to: to.to_owned(), kind }
}

fn forbidden(from: &str, to: &str, kind: DepKind) -> bool {
    !check(&[edge(from, to, kind)]).is_empty()
}

#[test]
fn deps_table_allows_the_crate_map_edges() {
    use DepKind::*;
    for (from, to, kind) in [
        ("kernel", "ledger", Build),
        ("render", "ledger", Normal),
        ("engine", "ledger", Normal),
        ("engine", "kernel", Normal),
        ("engine", "render", Normal),
        ("gui", "engine", Normal),
        ("prin", "engine", Normal),
        ("validation", "ledger", Normal),
        ("validation", "kernel", Normal),
        ("validation", "engine", Normal),
        ("validation", "render", Normal),
        ("engine", "validation", Dev),
        ("xtask", "validation", Dev),
        ("prin", "validation", Dev),
        ("render", "validation", Dev),
        ("kernel", "validation", Dev),
        ("ledger", "validation", Dev),
    ] {
        assert!(!forbidden(from, to, kind), "{from} → {to} ({kind}) should be allowed");
    }
}

#[test]
fn deps_table_forbids_edges_outside_the_crate_map() {
    use DepKind::*;
    for (from, to, kind) in [
        ("kernel", "ledger", Dev),
        ("kernel", "render", Build),
        ("ledger", "kernel", Build),
        ("render", "engine", Normal),
        ("validation", "gui", Normal),
        ("prin", "gui", Normal),
        ("gui", "kernel", Normal),
        ("prin", "kernel", Normal),
        ("engine", "validation", Normal),
        ("engine", "validation", Build),
        ("engine", "xtask", Normal),
        ("xtask", "engine", Normal),
        ("engine", "prin", Normal),
        ("gui", "validation", Dev),
        ("validation", "prin", Normal),
        ("kernel", "validation", Normal),
        ("ledger", "validation", Build),
    ] {
        assert!(forbidden(from, to, kind), "{from} → {to} ({kind}) should be forbidden");
    }
}

/// R-187, which closes RQ-129: kernel and ledger may take validation as a dev-dependency, never as a normal or
/// build dependency; gui never depends on validation; validation never depends on prin.
#[test]
fn deps_applies_r_187_to_the_validation_edges() {
    use DepKind::*;
    for from in ["kernel", "ledger"] {
        // Allowed: the dev-dependency. It is also the control for the two failing kinds below.
        assert_eq!(check(&[edge(from, "validation", Dev)]), vec![], "{from} → validation (dev) is allowed (R-187)");
        for kind in [Normal, Build] {
            let violations = check(&[edge(from, "validation", kind)]);
            assert_eq!(violations.len(), 1, "{from} → validation ({kind}) should be forbidden");
            assert!(violations[0].rule.contains("dev-dependency"), "{}", violations[0].rule);
        }
    }
    // gui → validation fails in every kind. Control: the same dev edge from engine passes.
    assert!(!forbidden("engine", "validation", Dev));
    for kind in [Normal, Dev, Build] {
        let violations = check(&[edge("gui", "validation", kind)]);
        assert_eq!(violations.len(), 1, "gui → validation ({kind}) should be forbidden");
        assert!(violations[0].rule.contains("R-187"), "{}", violations[0].rule);
    }
    // validation → prin fails in every kind. Control: validation → engine passes.
    assert!(!forbidden("validation", "engine", Normal));
    for kind in [Normal, Dev, Build] {
        let violations = check(&[edge("validation", "prin", kind)]);
        assert_eq!(violations.len(), 1, "validation → prin ({kind}) should be forbidden");
        assert!(violations[0].rule.contains("R-187"), "{}", violations[0].rule);
    }
}

fn metadata_json(dep: &str) -> Metadata {
    let doc = format!(
        r#"{{"workspace_members": ["path+file:///ws/crates/engine#0.1.0", "path+file:///ws/crates/gui#0.1.0"],
            "packages": [
              {{"name": "engine", "id": "path+file:///ws/crates/engine#0.1.0",
                "manifest_path": "/ws/crates/engine/Cargo.toml", "dependencies": [{dep}]}},
              {{"name": "gui", "id": "path+file:///ws/crates/gui#0.1.0",
                "manifest_path": "/ws/crates/gui/Cargo.toml", "dependencies": []}}]}}"#
    );
    Metadata::from_json(doc.as_bytes()).unwrap()
}

/// REQ-SYS-004 constrains workspace edges. A registry package, or a path package outside the workspace, that
/// shares a member's name is not one.
#[test]
fn deps_a_dependency_that_only_shares_a_member_name_is_not_an_edge() {
    let registry =
        r#"{"name": "gui", "kind": null, "source": "registry+https://github.com/rust-lang/crates.io-index"}"#;
    assert_eq!(metadata_json(registry).edges().unwrap(), vec![]);
    let elsewhere = r#"{"name": "gui", "kind": null, "source": null, "path": "/elsewhere/gui"}"#;
    assert_eq!(metadata_json(elsewhere).edges().unwrap(), vec![]);
    // Control: the same dependency as a path dependency on the member is an edge, and a forbidden one.
    let member = r#"{"name": "gui", "kind": null, "source": null, "path": "/ws/crates/gui"}"#;
    let edges = metadata_json(member).edges().unwrap();
    assert_eq!(edges, vec![edge("engine", "gui", DepKind::Normal)]);
    assert_eq!(check(&edges).len(), 1);
}

/// A synthetic workspace on disk for R-187's source condition: the §7.1 graph among ledger, kernel and
/// validation, plus `extra_dev` from `crate_name` (a raw dependency entry, e.g. a dev-dependency on
/// validation), with `files` (paths relative to that crate) written under it. Returns the metadata file.
fn source_workspace(case: &str, crate_name: &str, extra_dev: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("deps_r187").join(case);
    let _ = std::fs::remove_dir_all(&root);
    for name in ["ledger", "kernel", "validation"] {
        let src = root.join("crates").join(name).join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "").unwrap();
    }
    for (rel, text) in files {
        let path = root.join("crates").join(crate_name).join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let ws = root.display().to_string();
    let dep = |to: &str, kind: &str| {
        format!(r#"{{"name": "{to}", "kind": {kind}, "source": null, "path": "{ws}/crates/{to}"}}"#)
    };
    let deps_of = |name: &str| {
        let mut deps: Vec<String> = match name {
            "kernel" => vec![dep("ledger", r#""build""#)],
            "validation" => vec![dep("ledger", "null"), dep("kernel", "null")],
            _ => vec![],
        };
        if name == crate_name {
            deps.push(extra_dev.replace("{ws}", &ws));
        }
        deps.join(", ")
    };
    let package = |name: &str| {
        format!(
            r#"{{"name": "{name}", "id": "path+file://{ws}/crates/{name}#0.1.0",
                "manifest_path": "{ws}/crates/{name}/Cargo.toml", "dependencies": [{}]}}"#,
            deps_of(name)
        )
    };
    let doc = format!(
        r#"{{"workspace_members": ["path+file://{ws}/crates/ledger#0.1.0", "path+file://{ws}/crates/kernel#0.1.0",
             "path+file://{ws}/crates/validation#0.1.0"],
            "packages": [{}, {}, {}]}}"#,
        package("ledger"),
        package("kernel"),
        package("validation")
    );
    let path = root.join("metadata.json");
    std::fs::write(&path, doc).unwrap();
    path
}

fn run_deps_path(path: &std::path::Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(path)
        .output()
        .expect("run xtask");
    (output.status.success(), String::from_utf8_lossy(&output.stderr).into_owned())
}

const DEV_ON_VALIDATION: &str =
    r#"{"name": "validation", "kind": "dev", "source": null, "path": "{ws}/crates/validation"}"#;
const UNIT_TEST: &str = "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    use validation::Harness;\n}\n";
const INTEGRATION_TEST: &str = "use validation::Harness;\n\n#[test]\nfn t() {}\n";

/// R-187: in kernel and ledger, a test that uses validation is an integration test (tests/), not a unit test
/// in src/. A unit test in src/ fails `xtask deps`, naming the crate, file and line; the control, the same
/// use in tests/, passes.
#[test]
fn deps_validation_use_in_kernel_or_ledger_src_fails_and_in_tests_passes() {
    for name in ["kernel", "ledger"] {
        let unit = source_workspace(
            &format!("{name}_unit"),
            name,
            DEV_ON_VALIDATION,
            &[("src/lib.rs", UNIT_TEST), ("tests/controls.rs", INTEGRATION_TEST)],
        );
        let (ok, stderr) = run_deps_path(&unit);
        assert!(!ok, "{name}: a unit test in src/ that uses validation passes xtask deps");
        let expected = format!("forbidden use of validation in {name} src/ at ");
        assert!(stderr.contains(&expected), "{name}: stderr does not name the use:\n{stderr}");
        assert!(stderr.contains("src/lib.rs:5"), "{name}: stderr does not name the file and line:\n{stderr}");
        assert_eq!(stderr.matches("xtask deps: forbidden").count(), 1, "{name}: expected one violation:\n{stderr}");

        // Control: the same use as an integration test only.
        let integration = source_workspace(
            &format!("{name}_integration"),
            name,
            DEV_ON_VALIDATION,
            &[("src/lib.rs", "pub fn f() {}\n"), ("tests/controls.rs", INTEGRATION_TEST)],
        );
        let (ok, stderr) = run_deps_path(&integration);
        assert!(ok, "{name}: an integration test that uses validation fails xtask deps:\n{stderr}");
    }
}

/// A module file deeper under src/, and a renamed dev-dependency, are scanned too.
#[test]
fn deps_validation_use_is_found_in_submodules_and_under_a_rename() {
    let renamed = r#"{"name": "validation", "rename": "harness", "kind": "dev", "source": null,
                      "path": "{ws}/crates/validation"}"#;
    let metadata = source_workspace(
        "kernel_renamed",
        "kernel",
        renamed,
        &[("src/chart/mod.rs", "#[cfg(test)]\nfn t() { harness::run(); }\n")],
    );
    let (ok, stderr) = run_deps_path(&metadata);
    assert!(!ok, "a use of the renamed validation in src/chart/mod.rs passes xtask deps");
    assert!(stderr.contains("chart/mod.rs:2"), "{stderr}");
    // Control: under the rename, the name `validation` alone is not the crate.
    let metadata = source_workspace(
        "kernel_renamed_control",
        "kernel",
        renamed,
        &[("src/chart/mod.rs", "mod validation { pub fn run() {} }\nfn t() { validation::run(); }\n")],
    );
    let (ok, stderr) = run_deps_path(&metadata);
    assert!(ok, "{stderr}");
}

/// The live workspace must have its sources to scan: a kernel with a validation dependency whose src/ is
/// missing is an error, not a silent pass. Control: the same metadata read as a fixture skips the scan.
#[test]
fn deps_missing_sources_are_an_error_for_the_live_workspace() {
    let path = source_workspace("kernel_no_src", "kernel", DEV_ON_VALIDATION, &[]);
    std::fs::remove_dir_all(path.parent().unwrap().join("crates/kernel/src")).unwrap();
    let metadata = Metadata::from_file(&path).unwrap();
    let err = metadata.source_violations(true).unwrap_err();
    assert!(err.contains("kernel"), "{err}");
    assert_eq!(metadata.source_violations(false).unwrap(), vec![]);
}

/// A `>` that is part of an operator, or a `>` that closes no `<`, does not make the `::validation::…` after it
/// an associated item: after `>=`, `>>=`, a shift `>>` or a comparison `>`, it is a use of the crate in src/ and
/// fails, naming the line. Control: after the `>` or `>>` that closes a qualified path or a generic
/// (`<T as Tr>::validation::X`, `Vec::<Vec<u8>>::validation::X`), `validation` is an associated item, not the
/// crate, and passes.
#[test]
fn deps_validation_path_after_an_operator_with_a_closing_angle_fails() {
    let uses = [
        ("ge", "#[cfg(test)]\nfn t(a: u8) -> bool {\n    a >= ::validation::LIMIT\n}\n"),
        ("shr_assign", "#[cfg(test)]\nfn t(mut a: u8) {\n    a >>= ::validation::SHIFT;\n}\n"),
        ("shr", "#[cfg(test)]\nfn t(a: u8) -> u8 {\n    a >> ::validation::SHIFT\n}\n"),
        ("gt", "#[cfg(test)]\nfn t(a: u8) -> bool {\n    a > ::validation::LIMIT\n}\n"),
        ("lt_shr", "#[cfg(test)]\nfn t(a: u8, b: u8) -> bool {\n    a < b >> ::validation::SHIFT\n}\n"),
    ];
    for (case, src) in uses {
        let metadata = source_workspace(&format!("op_{case}"), "kernel", DEV_ON_VALIDATION, &[("src/lib.rs", src)]);
        let (ok, stderr) = run_deps_path(&metadata);
        assert!(!ok, "{case}: a use of validation after the operator passes xtask deps");
        assert!(stderr.contains("src/lib.rs:3"), "{case}: stderr does not name the line:\n{stderr}");
    }

    // Control: the same position after a `>` or `>>` that closes a `<` is an associated item.
    let items = [
        ("qualified", "fn t() -> u8 {\n    <u8 as Tr>::validation::X\n}\n"),
        ("turbofish", "fn t() -> u8 {\n    Vec::<Vec<u8>>::validation::X\n}\n"),
        ("nested_qualified", "fn t() -> u8 {\n    <Vec<u8> as Tr>::validation::X\n}\n"),
    ];
    for (case, src) in items {
        let metadata =
            source_workspace(&format!("op_control_{case}"), "kernel", DEV_ON_VALIDATION, &[("src/lib.rs", src)]);
        let (ok, stderr) = run_deps_path(&metadata);
        assert!(ok, "{case}: an associated item named validation fails xtask deps:\n{stderr}");
    }
}
