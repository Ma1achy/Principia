//! `cargo xtask lint wgsl` over the built-in occupants, `crates/render/shaders/wgsl/frag/<slot>/` (gui_state_contract
//! §3; TASK-M7-04): each is linted as the assembler presents it, after the prelude, the library files and its
//! `// @uniform` block declared as the struct `uniforms`, so an occupant that reads `uniforms.<name>` and calls the
//! library parses, validates and is held to the float rules (R-351, R-352), its findings at its own lines. Each scratch
//! workspace holds a clean generated layer, as `lint` requires. Each test has a registered negative control (R-176).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use validation::negative_control;
use xtask::lint_wgsl::{lint, Rule, GENERATED, LIB_FILES, OCCUPANT_DIR, PRELUDE};

/// A prelude: one helper, and nothing a float rule fires on.
const PRELUDE_CLEAN: &str = "// prelude\nfn helper(x: f32) -> f32 { return x * 2.0; }\n";

/// A library file after the prelude, as `colour_space.wgsl` is.
const LIBRARY: &str =
    "// library\nfn to_lab(c: vec3<f32>) -> vec3<f32> { return c * helper(1.0); }\n";

/// An occupant that reads its uniforms and calls the library, clean: the negative control's input.
#[cfg(feature = "controls")]
const OCCUPANT_CLEAN: &str =
    "// combiner\n// @uniform l_min: f32 = 0.0\n// @uniform l_max: f32 = 1.0\n\
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> {\n    \
return vec3<f32>(uniforms.l_min * (1.0 - b) + uniforms.l_max * b, to_lab(rgb).yz);\n}\n";

/// The same occupant, its fifth line comparing a float with itself.
const OCCUPANT_FIRES: &str = "// combiner\n// @uniform l_min: f32 = 0.0\n// @uniform l_max: f32 = 1.0\n\
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> {\n    \
return select(vec3<f32>(uniforms.l_min * (1.0 - b) + uniforms.l_max * b, to_lab(rgb).yz), rgb, b != b);\n}\n";

/// An occupant whose `// @uniform` line has no `:` and no `=`.
const OCCUPANT_MALFORMED: &str = "// combiner\n// @uniform l_min f32\n\
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * uniforms.l_min; }\n";

/// The same occupant, its line well formed: the negative control's input.
#[cfg(feature = "controls")]
const OCCUPANT_WELL_FORMED: &str = "// combiner\n// @uniform l_min: f32 = 0.0\n\
fn combine(rgb: vec3<f32>, b: f32) -> vec3<f32> { return rgb * uniforms.l_min; }\n";

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The workspace this crate is in.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch workspace: a `Cargo.toml`, the clean fixture as the generated layer, and `files` (each a path relative to
/// the root and its source).
fn workspace(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "lint_wgsl_occupant_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        std::fs::write(&path, text).expect("the file");
    };
    write("Cargo.toml", "");
    let clean = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lint_wgsl/clean.wgsl");
    write(GENERATED, &read(&clean));
    for (rel, text) in files {
        write(rel, text);
    }
    root
}

fn occupant_path() -> String {
    format!("{OCCUPANT_DIR}/combiner/test.wgsl")
}

fn library_path() -> String {
    format!("{LIB_FILES}/colour_space.wgsl")
}

/// `occupant`, after a clean prelude and library, lints and fails at its own fifth line with the self-comparison
/// rule.
fn check_fires_at_its_line(occupant: &str) {
    let root = workspace(&[
        (PRELUDE, PRELUDE_CLEAN),
        (&library_path(), LIBRARY),
        (&occupant_path(), occupant),
    ]);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = result.expect("the occupant parses and validates as presented");
    let found: Vec<String> = reports
        .iter()
        .flat_map(|r| r.findings.iter().map(|f| f.at(&r.file)))
        .collect();
    let want = format!("{}:5: [{}] ", occupant_path(), Rule::SelfCompare);
    assert!(
        found.iter().any(|l| l.starts_with(&want)),
        "the occupant's self-comparison did not fire at its line 5: {found:?}"
    );
    assert!(
        found.iter().all(|l| l.starts_with(&occupant_path())),
        "a finding was reported off the occupant: {found:?}"
    );
}

#[test]
fn lint_wgsl_occupant_fires_at_its_line_as_presented() {
    check_fires_at_its_line(OCCUPANT_FIRES);
}

negative_control!(
    lint_wgsl_occupant_fires_at_its_line_as_presented,
    "a clean occupant must not fire",
    expected = "did not fire at its line 5",
    check_fires_at_its_line(OCCUPANT_CLEAN)
);

/// The workspace's own built-in occupants and library, copied into a scratch workspace with each occupant under `dir`:
/// each must lint with every rule holding.
fn check_builtins_lint(dir: &str) {
    let repo = repo();
    let mut files = vec![(PRELUDE.to_owned(), read(&repo.join(PRELUDE)))];
    for entry in std::fs::read_dir(repo.join(LIB_FILES)).expect("the library") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if name.ends_with(".wgsl") && name != "prelude.wgsl" {
            files.push((format!("{LIB_FILES}/{name}"), read(&path)));
        }
    }
    let combiners = repo.join(OCCUPANT_DIR).join("combiner");
    let mut occupants = Vec::new();
    for entry in std::fs::read_dir(&combiners).expect("the combiners") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        if name.ends_with(".wgsl") {
            occupants.push(format!("{dir}/combiner/{name}"));
            files.push((format!("{dir}/combiner/{name}"), read(&path)));
        }
    }
    assert!(
        occupants.iter().any(|o| o.ends_with("/replace_l.wgsl"))
            && occupants.iter().any(|o| o.ends_with("/multiply.wgsl")),
        "the built-in combiners are missing: {occupants:?}"
    );
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let root = workspace(&refs);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let reports = match result {
        Ok(r) => r,
        Err(e) => panic!("the built-in occupants did not lint: {e}"),
    };
    for o in &occupants {
        let report = reports.iter().find(|r| &r.file == o);
        assert!(
            report.is_some_and(|r| r.findings.is_empty()),
            "{o} was not linted clean: {report:?}"
        );
    }
}

#[test]
fn lint_wgsl_occupant_builtins_lint_as_presented() {
    check_builtins_lint(OCCUPANT_DIR);
}

negative_control!(
    lint_wgsl_occupant_builtins_lint_as_presented,
    "an occupant outside the occupant directory is linted after the prelude alone, where `uniforms` is not in scope",
    expected = "the built-in occupants did not lint",
    check_builtins_lint("crates/render/shaders/wgsl/elsewhere")
);

/// A `// @uniform` line the lint cannot present is an error naming the occupant and the line.
fn check_malformed_refused(occupant: &str) {
    let root = workspace(&[(PRELUDE, PRELUDE_CLEAN), (&occupant_path(), occupant)]);
    let result = lint(&root);
    let _ = std::fs::remove_dir_all(&root);
    let error = result.err().unwrap_or_default();
    assert!(
        error.contains(&occupant_path()) && error.contains("line 2"),
        "a malformed `// @uniform` line was not refused naming the occupant and its line: {error:?}"
    );
}

#[test]
fn lint_wgsl_occupant_refuses_a_malformed_uniform() {
    check_malformed_refused(OCCUPANT_MALFORMED);
}

negative_control!(
    lint_wgsl_occupant_refuses_a_malformed_uniform,
    "a well-formed `// @uniform` line lints",
    expected = "was not refused naming the occupant",
    check_malformed_refused(OCCUPANT_WELL_FORMED)
);
