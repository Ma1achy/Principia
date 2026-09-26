//! `cargo xtask deps` against the metadata fixtures (REQ-SYS-004): the workspace graph passes, and each
//! fixture that adds one forbidden edge fails, naming the edge and its dependency kind. And R-187's condition,
//! checked by compiling synthetic workspaces (R-191).

use std::path::PathBuf;
use std::process::Command;

use xtask::deps::{check, DepKind, Edge, Metadata};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Runs `xtask deps --metadata <fixture>`; returns (success, stderr).
fn run_deps(name: &str) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(fixture(name))
        .output()
        .expect("run xtask");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn assert_fails_naming(name: &str, expected: &str) {
    let (ok, stderr) = run_deps(name);
    assert!(
        !ok,
        "{name}: xtask deps passed, but the fixture has a forbidden edge"
    );
    assert!(
        stderr.contains(expected),
        "{name}: stderr does not name {expected:?}:\n{stderr}"
    );
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
    assert_fails_naming(
        "metadata_kernel_engine.json",
        "forbidden edge kernel → engine (normal dependency)",
    );
}

#[test]
fn deps_forbidden_engine_to_gui_fails() {
    assert_fails_naming(
        "metadata_engine_gui.json",
        "forbidden edge engine → gui (normal dependency)",
    );
}

#[test]
fn deps_forbidden_ledger_to_engine_fails() {
    assert_fails_naming(
        "metadata_ledger_engine.json",
        "forbidden edge ledger → engine (normal dependency)",
    );
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
    assert!(
        !edges.iter().any(|e| e.from == "ledger"),
        "ledger must have no workspace dependency"
    );
    // Non-workspace dependencies (xtask's serde, serde_json) are not edges.
    assert!(!edges.iter().any(|e| e.to.starts_with("serde")));
}

#[test]
fn deps_live_workspace_passes() {
    let edges = Metadata::from_cargo().unwrap().edges().unwrap();
    assert_eq!(
        check(&edges),
        vec![],
        "the workspace crate graph has a forbidden edge"
    );
}

fn edge(from: &str, to: &str, kind: DepKind) -> Edge {
    Edge {
        from: from.to_owned(),
        to: to.to_owned(),
        kind,
    }
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
        assert!(
            !forbidden(from, to, kind),
            "{from} → {to} ({kind}) should be allowed"
        );
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
        assert!(
            forbidden(from, to, kind),
            "{from} → {to} ({kind}) should be forbidden"
        );
    }
}

/// R-187, which closes RQ-129: kernel and ledger may take validation as a dev-dependency, never as a normal or
/// build dependency; gui never depends on validation; validation never depends on prin.
#[test]
fn deps_applies_r_187_to_the_validation_edges() {
    use DepKind::*;
    for from in ["kernel", "ledger"] {
        // Allowed: the dev-dependency. It is also the control for the two failing kinds below.
        assert_eq!(
            check(&[edge(from, "validation", Dev)]),
            vec![],
            "{from} → validation (dev) is allowed (R-187)"
        );
        for kind in [Normal, Build] {
            let violations = check(&[edge(from, "validation", kind)]);
            assert_eq!(
                violations.len(),
                1,
                "{from} → validation ({kind}) should be forbidden"
            );
            assert!(
                violations[0].rule.contains("dev-dependency"),
                "{}",
                violations[0].rule
            );
        }
    }
    // gui → validation fails in every kind. Control: the same dev edge from engine passes.
    assert!(!forbidden("engine", "validation", Dev));
    for kind in [Normal, Dev, Build] {
        let violations = check(&[edge("gui", "validation", kind)]);
        assert_eq!(
            violations.len(),
            1,
            "gui → validation ({kind}) should be forbidden"
        );
        assert!(
            violations[0].rule.contains("R-187"),
            "{}",
            violations[0].rule
        );
    }
    // validation → prin fails in every kind. Control: validation → engine passes.
    assert!(!forbidden("validation", "engine", Normal));
    for kind in [Normal, Dev, Build] {
        let violations = check(&[edge("validation", "prin", kind)]);
        assert_eq!(
            violations.len(),
            1,
            "validation → prin ({kind}) should be forbidden"
        );
        assert!(
            violations[0].rule.contains("R-187"),
            "{}",
            violations[0].rule
        );
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
    let registry = r#"{"name": "gui", "kind": null, "source": "registry+https://github.com/rust-lang/crates.io-index"}"#;
    assert_eq!(metadata_json(registry).edges().unwrap(), vec![]);
    let elsewhere = r#"{"name": "gui", "kind": null, "source": null, "path": "/elsewhere/gui"}"#;
    assert_eq!(metadata_json(elsewhere).edges().unwrap(), vec![]);
    // Control: the same dependency as a path dependency on the member is an edge, and a forbidden one.
    let member = r#"{"name": "gui", "kind": null, "source": null, "path": "/ws/crates/gui"}"#;
    let edges = metadata_json(member).edges().unwrap();
    assert_eq!(edges, vec![edge("engine", "gui", DepKind::Normal)]);
    assert_eq!(check(&edges).len(), 1);
}

#[test]
fn deps_a_fixture_skips_the_compile_check_and_says_so() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--metadata"])
        .arg(fixture("metadata_workspace.json"))
        .output()
        .expect("run xtask");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("compile check skipped"), "{stdout}");
    // Control: the live workspace, which has sources, is not skipped.
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("deps")
        .output()
        .expect("run xtask");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!stdout.contains("compile check skipped"), "{stdout}");
}

/// Metadata for kernel, ledger and validation under `root`, each with its library target at `lib` (relative to the
/// crate; none if `None`). For the target check, which reads the metadata only.
fn target_metadata(case: &str, lib: Option<&str>) -> Metadata {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("deps_targets")
        .join(case);
    let ws = root.display();
    let package = |name: &str| {
        let targets = lib.map_or(String::new(), |lib| {
            format!(r#", "targets": [{{"kind": ["lib"], "src_path": "{ws}/crates/{name}/{lib}"}}]"#)
        });
        format!(
            r#"{{"name": "{name}", "id": "path+file://{ws}/crates/{name}#0.1.0",
                "manifest_path": "{ws}/crates/{name}/Cargo.toml", "dependencies": []{targets}}}"#
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
    Metadata::from_json(doc.as_bytes()).unwrap()
}

/// A kernel or ledger library target outside src/ (`[lib] path = "lib/lib.rs"`) fails, naming the crate (R-187,
/// R-191). Control: the library at src/lib.rs passes.
#[test]
fn deps_a_kernel_or_ledger_lib_outside_src_fails() {
    let err = target_metadata("outside", Some("lib/lib.rs"))
        .check_targets(true)
        .unwrap_err();
    assert!(
        err.contains("ledger: its lib target is at ") && err.contains("R-191"),
        "{err}"
    );
    target_metadata("inside", Some("src/lib.rs"))
        .check_targets(true)
        .unwrap();
}

/// The workspace must have its targets to check: metadata without kernel's and ledger's targets is an error, not a
/// silent pass. Control: the same metadata read as a fixture (`require_sources` off) passes.
#[test]
fn deps_missing_targets_are_an_error_for_a_workspace() {
    let metadata = target_metadata("no_targets", None);
    let err = metadata.check_targets(true).unwrap_err();
    assert!(err.contains("no library or binary target"), "{err}");
    metadata.check_targets(false).unwrap();
}

/// R-191's compile check runs on real cargo workspaces: a synthetic one with ledger, kernel (with a build script that
/// writes `$OUT_DIR/generated.rs`, and ledger as its build-dependency, R-185) and validation, each tiny. `dev` lists
/// the crates that take validation as a dev-dependency; `files` (paths relative to the workspace root) are written
/// last, over the defaults.
fn cargo_workspace(case: &str, dev: &[&str], files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("deps_r191")
        .join(case);
    let _ = std::fs::remove_dir_all(&root);
    let package = |name: &str, extra: &str| {
        let dev_dep = if dev.contains(&name) {
            "\n[dev-dependencies]\nvalidation = { path = \"../validation\" }\n"
        } else {
            ""
        };
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}{dev_dep}")
    };
    let mut all: Vec<(String, String)> = vec![
        (
            "Cargo.toml".into(),
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/ledger\", \"crates/kernel\", \"crates/validation\"]\n"
                .into(),
        ),
        ("crates/ledger/Cargo.toml".into(), package("ledger", "")),
        ("crates/ledger/src/lib.rs".into(), "pub fn ledger() {}\n".into()),
        ("crates/kernel/Cargo.toml".into(), package("kernel", "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n")),
        ("crates/kernel/build.rs".into(), build_rs("pub const GENERATED: u32 = 1;")),
        ("crates/kernel/src/lib.rs".into(), "pub fn kernel() {}\n".into()),
        (
            "crates/validation/Cargo.toml".into(),
            package("validation", "\n[dependencies]\nledger = { path = \"../ledger\" }\nkernel = { path = \"../kernel\" }\n"),
        ),
        ("crates/validation/src/lib.rs".into(), "pub struct Harness;\n".into()),
    ];
    all.extend(files.iter().map(|(p, t)| (p.to_string(), t.to_string())));
    for (rel, text) in all {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

/// A kernel build script that writes `generated` to `$OUT_DIR/generated.rs`.
fn build_rs(generated: &str) -> String {
    format!(
        "use ledger as _;\nfn main() {{\n    let out = std::env::var(\"OUT_DIR\").unwrap();\n    \
         std::fs::write(format!(\"{{out}}/generated.rs\"), {generated:?}).unwrap();\n}}\n"
    )
}

/// Runs `xtask deps --manifest-path <root>/Cargo.toml`; returns (success, stdout, stderr).
fn run_workspace(root: &std::path::Path) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["deps", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("run xtask");
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (
        output.status.success(),
        text(&output.stdout),
        text(&output.stderr),
    )
}

/// The control for each failing case: with the dev-dependency, the same workspace compiles its tests, so the use is
/// real and it is removing the dev-dependency, not a broken fixture, that fails the check.
fn assert_compiles_with_the_dependency(root: &std::path::Path, krate: &str) {
    let output = Command::new(env!("CARGO"))
        .args([
            "check",
            "--offline",
            "--tests",
            "-p",
            krate,
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("run cargo check");
    assert!(
        output.status.success(),
        "control: {krate} fails to compile with validation:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts that `xtask deps` fails on `root`, citing R-187 and R-191 and showing the compiler's error at `file`
/// (relative to the workspace root, not the copy), and that its control compiles.
fn assert_fails_to_compile(case: &str, root: &std::path::Path, krate: &str, file: &str) {
    let (ok, _, stderr) = run_workspace(root);
    assert!(
        !ok,
        "{case}: a unit test in {krate} src/ that uses validation passes xtask deps"
    );
    assert!(
        stderr.contains("R-187, R-191"),
        "{case}: stderr does not cite R-187 and R-191:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E04"),
        "{case}: stderr does not show the compiler's error:\n{stderr}"
    );
    assert!(
        stderr.contains(file),
        "{case}: stderr does not name {file}:\n{stderr}"
    );
    assert_compiles_with_the_dependency(root, krate);
}

const UNIT_TEST: &str =
    "pub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    use validation::Harness;\n\n    \
                         #[test]\n    fn t() {\n        let _ = Harness;\n    }\n}\n";
const INTEGRATION_TEST: &str =
    "use validation::Harness;\n\n#[test]\nfn t() {\n    let _ = Harness;\n}\n";

/// R-187, R-191: in kernel and ledger, a unit test in src/ that uses validation fails `xtask deps`, showing the
/// compiler's error. Controls: the same workspace compiles with the dev-dependency, and the same use as an
/// integration test in tests/ passes.
#[test]
fn deps_a_unit_test_using_validation_fails_and_an_integration_test_passes() {
    for krate in ["kernel", "ledger"] {
        let lib = format!("crates/{krate}/src/lib.rs");
        let tests = format!("crates/{krate}/tests/uses.rs");
        let root = cargo_workspace(&format!("{krate}_unit"), &[krate], &[(&lib, UNIT_TEST)]);
        assert_fails_to_compile(krate, &root, krate, &lib);

        let root = cargo_workspace(
            &format!("{krate}_integration"),
            &[krate],
            &[(&tests, INTEGRATION_TEST)],
        );
        let (ok, stdout, stderr) = run_workspace(&root);
        assert!(
            ok,
            "{krate}: an integration test that uses validation fails xtask deps:\n{stderr}"
        );
        assert!(
            stdout.contains(&format!("compile check passed: {krate} compiles")),
            "{krate}: {stdout}"
        );
        assert_compiles_with_the_dependency(&root, krate);
    }
}

/// R-191: every route by which a kernel unit test can reach validation fails to compile without it: an alias, a
/// macro, and a file outside src/ brought in by `#[path]` or `include!`. The token scan these replace was bypassed by
/// each (R-189, R-190). Control: each workspace compiles with the dev-dependency (`assert_fails_to_compile`).
#[test]
fn deps_every_route_to_validation_from_a_unit_test_fails() {
    /// One route: the kernel lib.rs that takes it, an optional extra file (path, contents) it brings in, and the
    /// file the compiler's error must name.
    struct Route {
        name: &'static str,
        lib_rs: &'static str,
        extra_file: Option<(&'static str, &'static str)>,
        error_file: &'static str,
    }
    let outside = ("crates/kernel/outside/t.rs", INTEGRATION_TEST);
    let routes = [
        Route {
            name: "alias",
            lib_rs: "#[cfg(test)]\nuse validation as v;\n\n#[cfg(test)]\n#[test]\nfn t() {\n    let _ = v::Harness;\n}\n",
            extra_file: None,
            error_file: "crates/kernel/src/lib.rs",
        },
        Route {
            name: "macro",
            lib_rs: "macro_rules! from {\n    ($k:ident) => {\n        #[cfg(test)]\n        #[test]\n        fn t() {\n            \
             let _ = $k::Harness;\n        }\n    };\n}\n\nfrom!(validation);\n",
            extra_file: None,
            error_file: "crates/kernel/src/lib.rs",
        },
        Route {
            name: "path",
            lib_rs: "#[cfg(test)]\n#[path = \"../outside/t.rs\"]\nmod t;\n",
            extra_file: Some(outside),
            error_file: "outside/t.rs",
        },
        Route {
            name: "include",
            lib_rs: "#[cfg(test)]\nmod t {\n    include!(\"../outside/t.rs\");\n}\n",
            extra_file: Some(outside),
            error_file: "outside/t.rs",
        },
    ];
    for route in routes {
        let mut files = vec![("crates/kernel/src/lib.rs", route.lib_rs)];
        files.extend(route.extra_file);
        let root = cargo_workspace(&format!("route_{}", route.name), &["kernel"], &files);
        assert_fails_to_compile(route.name, &root, "kernel", route.error_file);
    }
}

/// R-192, R-194: the compile check builds with `--all-features`, so a kernel unit test behind a feature (`x`, off by
/// default) that uses validation fails, and only that feature set sees it. Control: the same workspace with the use removed passes, so it is the use, not the
/// feature, that fails the check.
#[test]
fn deps_a_unit_test_behind_a_feature_using_validation_fails() {
    let manifest = "[package]\nname = \"kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\nx = []\n\n\
                    [build-dependencies]\nledger = { path = \"../ledger\" }\n\n\
                    [dev-dependencies]\nvalidation = { path = \"../validation\" }\n";
    let with_use = "pub fn f() {}\n\n#[cfg(all(test, feature = \"x\"))]\nmod tests {\n    #[test]\n    fn t() {\n        \
                    let _ = validation::Harness;\n    }\n}\n";
    let without_use = "pub fn f() {}\n\n#[cfg(all(test, feature = \"x\"))]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    let lib = "crates/kernel/src/lib.rs";
    let toml = "crates/kernel/Cargo.toml";

    let root = cargo_workspace("feature_unit", &[], &[(toml, manifest), (lib, with_use)]);
    let (ok, _, stderr) = run_workspace(&root);
    assert!(
        !ok,
        "a kernel unit test behind feature x that uses validation passes xtask deps"
    );
    assert!(
        stderr.contains("R-194") && stderr.contains("with --all-features in the dev profile"),
        "stderr does not name --all-features in the dev profile and cite R-194:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E04") && stderr.contains(lib),
        "stderr does not show the compiler's error:\n{stderr}"
    );

    let root = cargo_workspace(
        "feature_unit_control",
        &[],
        &[(toml, manifest), (lib, without_use)],
    );
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(
        ok,
        "control: the same unit test behind feature x without the use fails xtask deps:\n{stderr}"
    );
    assert!(
        stdout.contains("compile check passed: kernel compiles"),
        "control: {stdout}"
    );
}

/// A kernel manifest with the validation dev-dependency and the given `[features]` table.
fn kernel_manifest(features: &str) -> String {
    format!(
        "[package]\nname = \"kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\n{features}\n\
         [build-dependencies]\nledger = {{ path = \"../ledger\" }}\n\n\
         [dev-dependencies]\nvalidation = {{ path = \"../validation\" }}\n"
    )
}

/// validation's manifest, taking kernel without its default features.
const VALIDATION_WITHOUT_KERNEL_DEFAULTS: &str = "[package]\nname = \"validation\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
     [dependencies]\nledger = { path = \"../ledger\" }\nkernel = { path = \"../kernel\", default-features = false }\n";

/// A kernel lib whose unit test sits under `#[cfg(all(test, <gate>))]`, with or without a use of validation.
fn gated_unit_test(gate: &str, uses: bool) -> String {
    let body = if uses {
        "let _ = validation::Harness;"
    } else {
        ""
    };
    format!("pub fn f() {{}}\n\n#[cfg(all(test, {gate}))]\nmod tests {{\n    #[test]\n    fn t() {{\n        {body}\n    }}\n}}\n")
}

/// R-194: a kernel unit test under `gate` that uses validation fails the check at the first run that compiles it,
/// and the failure names that run (`expected`: its feature set and profile) and cites R-194. Premise: plain
/// `cargo test` with `premise_args` builds and runs the test with validation linked. Control: the same workspace
/// without the use passes, with the compile check run, so it is the use, not the gate, that fails the check.
fn assert_gated_unit_test_fails(
    case: &str,
    features: &str,
    gate: &str,
    premise_args: &[&str],
    expected: &str,
    extra: &[(&str, &str)],
) {
    let toml = "crates/kernel/Cargo.toml";
    let lib = "crates/kernel/src/lib.rs";
    let manifest = kernel_manifest(features);

    let mut files = vec![(toml, manifest.as_str())];
    files.extend_from_slice(extra);
    let with_use = gated_unit_test(gate, true);
    let root = cargo_workspace(
        case,
        &[],
        &[files.as_slice(), &[(lib, with_use.as_str())]].concat(),
    );
    let premise = Command::new(env!("CARGO"))
        .args([
            "test",
            "--offline",
            "--lib",
            "-p",
            "kernel",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .args(premise_args)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("run cargo test");
    let stdout = String::from_utf8_lossy(&premise.stdout);
    assert!(
        premise.status.success() && stdout.contains("test tests::t ... ok"),
        "{case}: premise: cargo test {premise_args:?} does not run the unit test:\n{stdout}{}",
        String::from_utf8_lossy(&premise.stderr)
    );
    let (ok, _, stderr) = run_workspace(&root);
    assert!(
        !ok,
        "{case}: a kernel unit test under {gate} that uses validation passes xtask deps"
    );
    assert!(
        stderr.contains(expected),
        "{case}: stderr does not name {expected:?}:\n{stderr}"
    );
    assert!(
        stderr.contains("R-194"),
        "{case}: stderr does not cite R-194:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E04") && stderr.contains(lib),
        "{case}: stderr does not show the compiler's error:\n{stderr}"
    );

    let control = format!("{case}_control");
    let without_use = gated_unit_test(gate, false);
    let root = cargo_workspace(
        &control,
        &[],
        &[files.as_slice(), &[(lib, without_use.as_str())]].concat(),
    );
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(
        ok,
        "{control}: the same unit test without the use fails xtask deps:\n{stderr}"
    );
    assert!(
        stdout.contains("compile check passed: kernel compiles"),
        "{control}: {stdout}"
    );
}

/// R-194: a unit test behind the absence of a non-default feature (`not(feature = "x")`), which plain `cargo test`
/// builds, fails; `--all-features` alone would not see it. Control in `assert_gated_unit_test_fails`.
#[test]
fn deps_a_unit_test_behind_a_missing_feature_using_validation_fails() {
    assert_gated_unit_test_fails(
        "missing_feature",
        "x = []\n",
        "not(feature = \"x\")",
        &[],
        "with --no-default-features in the dev profile",
        &[],
    );
}

/// R-194: a unit test behind `not(debug_assertions)`, which `cargo test --release` builds, fails in the release
/// profile; every dev-profile run passes it. Control in `assert_gated_unit_test_fails`.
#[test]
fn deps_a_unit_test_behind_the_release_profile_using_validation_fails() {
    assert_gated_unit_test_fails(
        "release_profile",
        "",
        "not(debug_assertions)",
        &["--release"],
        "with --no-default-features in the release profile",
        &[],
    );
}

/// R-194: a unit test behind the absence of a default feature (`not(feature = "d")`, `default = ["d"]`), which only
/// `cargo test --no-default-features` builds, fails; neither default features nor `--all-features` see it. Control
/// in `assert_gated_unit_test_fails`. validation takes kernel with `default-features = false` here: with kernel's
/// defaults, cargo would unify `d` back on for `cargo test --no-default-features`, and the test would never be built.
#[test]
fn deps_a_unit_test_present_only_without_default_features_using_validation_fails() {
    assert_gated_unit_test_fails(
        "no_default_features",
        "default = [\"d\"]\nd = []\n",
        "not(feature = \"d\")",
        &["--no-default-features"],
        "with --no-default-features in the dev profile",
        &[(
            "crates/validation/Cargo.toml",
            VALIDATION_WITHOUT_KERNEL_DEFAULTS,
        )],
    );
}

/// R-194: a kernel doctest in src/ may use validation; rustdoc builds it as a separate crate, and the check does not
/// compile doctests. Premise: `cargo test --doc` compiles and runs the doctest with validation linked. Control: the
/// same use in a unit test in the same file fails the check.
#[test]
fn deps_a_doctest_using_validation_passes() {
    let lib = "crates/kernel/src/lib.rs";
    let doc =
        "/// ```\n/// let _ = validation::Harness;\n/// kernel::f();\n/// ```\npub fn f() {}\n";
    let root = cargo_workspace("doctest", &["kernel"], &[(lib, doc)]);
    let premise = Command::new(env!("CARGO"))
        .args([
            "test",
            "--offline",
            "--doc",
            "-p",
            "kernel",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
        .expect("run cargo test --doc");
    let stdout = String::from_utf8_lossy(&premise.stdout);
    assert!(
        premise.status.success() && stdout.contains("1 passed"),
        "premise: cargo test --doc does not run the doctest:\n{stdout}{}",
        String::from_utf8_lossy(&premise.stderr)
    );
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(
        ok,
        "a kernel doctest that uses validation fails xtask deps:\n{stderr}"
    );
    assert!(
        stdout.contains("compile check passed: kernel compiles"),
        "{stdout}"
    );

    let with_unit = format!("{doc}\n#[cfg(test)]\nmod tests {{\n    #[test]\n    fn t() {{\n        let _ = validation::Harness;\n    }}\n}}\n");
    let root = cargo_workspace("doctest_control", &["kernel"], &[(lib, &with_unit)]);
    assert_fails_to_compile("doctest_control", &root, "kernel", lib);
}

/// R-191 allows a local item named validation again: a unit test that uses kernel's own `mod validation` passes.
/// Control: the unit test that uses the crate fails (`deps_a_unit_test_using_validation_fails_and_…`), and here the
/// same test with the local module removed fails.
#[test]
fn deps_a_local_mod_validation_passes() {
    let test = "\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        crate::validation::run();\n    }\n}\n";
    let lib = format!("mod validation {{\n    pub fn run() {{}}\n}}\n{test}");
    let root = cargo_workspace(
        "local_mod",
        &["kernel"],
        &[("crates/kernel/src/lib.rs", &lib)],
    );
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(ok, "a local mod validation fails xtask deps:\n{stderr}");
    assert!(stdout.contains("compile check passed"), "{stdout}");
    let root = cargo_workspace(
        "local_mod_control",
        &["kernel"],
        &[("crates/kernel/src/lib.rs", test)],
    );
    let (ok, _, stderr) = run_workspace(&root);
    assert!(
        !ok && stderr.contains("R-191"),
        "control: without the local module the test passes:\n{stderr}"
    );
}

/// R-185, R-191: kernel includes the ledger's generated code from `OUT_DIR`, as its build script writes it, and
/// passes. Control: the same include of generated code that uses validation in a unit test fails.
#[test]
fn deps_the_kernel_out_dir_include_passes() {
    let lib = "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\n\npub fn g() -> u32 {\n    GENERATED\n}\n";
    let root = cargo_workspace("out_dir", &["kernel"], &[("crates/kernel/src/lib.rs", lib)]);
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(ok, "kernel's OUT_DIR include fails xtask deps:\n{stderr}");
    assert!(stdout.contains("compile check passed"), "{stdout}");
    let generated = "pub const GENERATED: u32 = 1;\n#[cfg(test)]\n#[test]\nfn t() { let _ = validation::Harness; }\n";
    let root = cargo_workspace(
        "out_dir_control",
        &["kernel"],
        &[
            ("crates/kernel/src/lib.rs", lib),
            ("crates/kernel/build.rs", &build_rs(generated)),
        ],
    );
    assert_fails_to_compile("out_dir_control", &root, "kernel", "generated.rs");
}

/// Without a validation dev-dependency in kernel or ledger there is nothing to check, and `xtask deps` says so.
/// Control: the same workspace with the dev-dependency runs the check.
#[test]
fn deps_the_compile_check_runs_only_with_the_dev_dependency() {
    let root = cargo_workspace("no_dev", &[], &[]);
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(
        ok && stdout.contains("compile check not needed"),
        "{stdout}\n{stderr}"
    );
    let root = cargo_workspace("with_dev", &["ledger"], &[]);
    let (ok, stdout, stderr) = run_workspace(&root);
    assert!(
        ok && stdout.contains("compile check passed: ledger compiles"),
        "{stdout}\n{stderr}"
    );
}

/// R-187, R-191: `test = false` on a `[[bin]]` keeps its unit tests out of `cargo check --tests`, but
/// `cargo test --bin` still builds them with the dev-dependency, so the check sets `test = true` on every binary. A
/// unit test that uses validation fails, in a `[[bin]]` with a path, in one that names an auto-discovered
/// `src/bin/` file, and in the inline-array form `bin = [{ … }]`. Controls: each workspace compiles with the
/// dev-dependency, and the same manifest without the use passes the check.
#[test]
fn deps_a_unit_test_of_a_binary_with_test_false_fails() {
    let head = "[package]\nname = \"kernel\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    let tail = "\n[build-dependencies]\nledger = { path = \"../ledger\" }\n\n\
                [dev-dependencies]\nvalidation = { path = \"../validation\" }\n";
    let cases: [(&str, String, &str); 3] = [
        (
            "bin_path",
            format!("{head}\n[[bin]]\nname = \"k\"\npath = \"src/main.rs\"\ntest = false\n{tail}"),
            "src/main.rs",
        ),
        (
            "bin_auto",
            format!("{head}\n[[bin]]\nname = \"tool\"\ntest = false\n{tail}"),
            "src/bin/tool.rs",
        ),
        (
            "bin_inline",
            format!(
                "bin = [{{ name = \"k\", path = \"src/main.rs\", test = false }}]\n\n{head}{tail}"
            ),
            "src/main.rs",
        ),
    ];
    let with_use = format!("fn main() {{}}\n\n{UNIT_TEST}");
    let without_use = "fn main() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    for (case, manifest, main) in cases {
        let main = format!("crates/kernel/{main}");
        let toml = "crates/kernel/Cargo.toml";
        let root = cargo_workspace(case, &[], &[(toml, &manifest), (&main, &with_use)]);
        assert_fails_to_compile(case, &root, "kernel", &main);

        let control = format!("{case}_control");
        let root = cargo_workspace(&control, &[], &[(toml, &manifest), (&main, without_use)]);
        let (ok, stdout, stderr) = run_workspace(&root);
        assert!(
            ok,
            "{control}: the same binary without the use fails xtask deps:\n{stderr}"
        );
        assert!(
            stdout.contains("compile check passed: kernel compiles"),
            "{control}: {stdout}"
        );
    }
}
