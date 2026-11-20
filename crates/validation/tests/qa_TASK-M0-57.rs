//! QA tests for TASK-M0-57, written from REQ-SYS-097's statement and R-862's words: "DX12 must be an opt-in backend,
//! off by default: behind a cargo feature, `dx12`, and selected only by an explicit `PRIN_GPU_BACKEND=dx12` in a build
//! with that feature; an unset value never selects it (R-206); without the feature `dx12` is refused as an error naming
//! the variable and the feature; it is documented as "local testing only, untested in CI"".
//!
//! Which build this is comes from `cfg!(feature = "dx12")` here, in the test crate, never from the implementation's own
//! flag, so a library that misreports its build is caught. The tests hold in both builds, `cargo test -p validation`
//! and `cargo test -p validation --features dx12`; each registers a negative control (R-176). The harness is opened in a
//! `qa_child` child process (R-210), so this process's `PRIN_GPU_BACKEND` is never changed.

use std::path::{Path, PathBuf};
use std::process::Command;
use validation::gpu::{backend_choice, backend_from, BACKEND_VAR};
use validation::spawn::Spawn;

/// Whether this test build has validation's `dx12` feature, read by the test crate itself.
const FEATURE: bool = cfg!(feature = "dx12");

/// The words R-862 has the opt-in documented with.
const DOC_WORDS: &str = "local testing only, untested in CI";

/// A selection as `validation::gpu::backend_choice` gives it: the backends and the line logged for them, or the error.
type Choice = Result<(wgpu::Backends, String), String>;

fn this_build(value: Option<&str>) -> Choice {
    backend_choice(value).map_err(|e| e.to_string())
}

/// R-206's platform default, from the ruling: metal on macOS, vulkan elsewhere.
fn platform() -> wgpu::Backends {
    if cfg!(target_os = "macos") {
        wgpu::Backends::METAL
    } else {
        wgpu::Backends::VULKAN
    }
}

/// What a refusal of `dx12` lacks, if anything: it must name the variable, and, once the variable's own setting
/// (`PRIN_GPU_BACKEND="dx12"` or `=dx12`) is taken out, still name the feature: the word "feature" and `dx12`. Without
/// taking it out, the value would name `dx12` for any refusal at all.
fn refusal_lacks(err: &str) -> Option<String> {
    if !err.contains(BACKEND_VAR) {
        return Some(format!("does not name {BACKEND_VAR}: {err}"));
    }
    let mut rest = err.to_owned();
    for setting in ["=\"dx12\"", "='dx12'", "=dx12"] {
        rest = rest.replace(&format!("{BACKEND_VAR}{setting}"), BACKEND_VAR);
    }
    let rest = rest.to_lowercase();
    (!(rest.contains("feature") && rest.contains("dx12")))
        .then(|| format!("does not name the feature `dx12`: {err}"))
}

fn check_names_the_feature(err: &str) {
    if let Some(lack) = refusal_lacks(err) {
        panic!("the refusal of dx12 {lack}");
    }
}

/// `PRIN_GPU_BACKEND=dx12` as `choice` takes it in a build with the feature (`feature`) or without: with it, DX12 and
/// only DX12, its log line naming dx12 (R-206: every run says what it ran on); without it, refused naming the variable
/// and the feature, never another backend.
fn check_dx12(choice: &dyn Fn(Option<&str>) -> Choice, feature: bool) {
    match (feature, choice(Some("dx12"))) {
        (true, Ok((backends, log))) => {
            assert_eq!(
                backends,
                wgpu::Backends::DX12,
                "dx12 with the feature selected {backends:?}"
            );
            assert!(
                log.to_lowercase().contains("dx12"),
                "dx12 with the feature: the log does not name it: {log}"
            );
        }
        (true, Err(e)) => panic!("dx12 with the feature was refused: {e}"),
        (false, Ok((backends, _))) => {
            panic!("dx12 without the feature was accepted, as {backends:?}")
        }
        (false, Err(e)) => check_names_the_feature(&e),
    }
}

/// REQ-SYS-097: this build's own reading of `PRIN_GPU_BACKEND=dx12`, through both of the harness's parsers.
#[test]
fn qa_m0_57_dx12_as_this_build_reads_it() {
    check_dx12(&this_build, FEATURE);
    let from = backend_from(Some("dx12")).map_err(|e| e.to_string());
    assert_eq!(
        from.is_ok(),
        FEATURE,
        "backend_from and the build disagree on dx12: {from:?}"
    );
    if FEATURE {
        assert_eq!(from, Ok(wgpu::Backends::DX12));
    }
}

validation::negative_control!(
    qa_m0_57_dx12_as_this_build_reads_it,
    "this build read as the other one",
    expected = "dx12 with",
    check_dx12(&this_build, !FEATURE)
);

/// A build without the feature refuses `dx12` naming the variable and the feature, read through the library's
/// both-arms entry so that it is tested in the feature build too.
#[test]
fn qa_m0_57_dx12_refusal_names_the_feature() {
    check_names_the_feature(&default_build_refusal("dx12"));
}

/// The refusal of `value` in a build without the feature.
fn default_build_refusal(value: &str) -> String {
    validation::gpu::backend_choice_in(Some(value), false)
        .map(|(b, _)| panic!("{value} was accepted without the feature, as {b:?}"))
        .unwrap_or_else(|e| e.to_string())
}

validation::negative_control!(
    qa_m0_57_dx12_refusal_names_the_feature,
    "an unknown value's refusal, which names the variable and no feature",
    expected = "does not name the feature",
    check_names_the_feature(&default_build_refusal("gl"))
);

/// "An unset value never selects it (R-206)": unset is the platform's backend, never DX12, in this build and in either
/// arm of the selection.
fn check_unset(choice: &dyn Fn(Option<&str>) -> Choice) {
    let (backends, _) = choice(None).unwrap_or_else(|e| panic!("unset was refused: {e}"));
    assert!(
        !backends.contains(wgpu::Backends::DX12),
        "unset {BACKEND_VAR} selected DX12: {backends:?}"
    );
    assert_eq!(backends, platform(), "unset is not the platform's backend");
}

#[test]
fn qa_m0_57_unset_never_selects_dx12() {
    check_unset(&this_build);
    for feature in [false, true] {
        check_unset(&|v| validation::gpu::backend_choice_in(v, feature).map_err(|e| e.to_string()));
    }
}

validation::negative_control!(
    qa_m0_57_unset_never_selects_dx12,
    "unset read as dx12 in a build with the feature",
    expected = "selected DX12",
    check_unset(
        &|v| validation::gpu::backend_choice_in(Some(v.unwrap_or("dx12")), true)
            .map_err(|e| e.to_string())
    )
);

/// The `qa_child` body `open_harness` (`GpuHarness::new()`) with `PRIN_GPU_BACKEND` set to `value` or removed: its
/// marker line and its whole output.
fn open_harness(value: Option<&str>) -> (String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_qa_child"));
    cmd.arg("open_harness");
    match value {
        Some(v) => cmd.env(BACKEND_VAR, v),
        None => cmd.env_remove(BACKEND_VAR),
    };
    let o = cmd.timed_output().expect("qa_child ran");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(o.status.success(), "the child crashed: {text}");
    let line = text
        .lines()
        .find(|l| l.starts_with("QA_OPEN_"))
        .unwrap_or_else(|| panic!("the child printed no QA_OPEN_ line: {text}"))
        .to_owned();
    (line, text)
}

/// `GpuHarness::new()` given `PRIN_GPU_BACKEND=dx12`, as its child printed it: with the feature, it opens DX12, logging
/// dx12, or nothing (no DX12 adapter off Windows), never Metal or Vulkan; without it, it opens nothing and fails naming
/// the variable and the feature.
fn check_harness(feature: bool, line: &str, text: &str) {
    if feature {
        assert!(
            line == "QA_OPEN_OK backend=Dx12" || line.starts_with("QA_OPEN_ERR"),
            "dx12 at the harness, with the feature, opened another backend: {line}"
        );
        // R-206: a run says what it ran on. With no DX12 adapter (off Windows) nothing ran, so only an opened harness
        // is held to the log line.
        if line.starts_with("QA_OPEN_OK") {
            assert!(
                text.lines()
                    .any(|l| l.starts_with("gpu backend:") && l.contains("dx12")),
                "dx12 at the harness, with the feature, logged no dx12 line: {text}"
            );
        }
    } else {
        let err = line
            .strip_prefix("QA_OPEN_ERR ")
            .unwrap_or_else(|| panic!("dx12 at the harness, without the feature, opened: {line}"));
        if let Some(lack) = refusal_lacks(err) {
            panic!("dx12 at the harness, without the feature: the refusal {lack}");
        }
    }
}

/// REQ-SYS-097 at `GpuHarness::new()` itself, not only its parser.
#[test]
fn qa_m0_57_harness_reads_dx12_as_this_build() {
    let (line, text) = open_harness(Some("dx12"));
    check_harness(FEATURE, &line, &text);
}

validation::negative_control!(
    qa_m0_57_harness_reads_dx12_as_this_build,
    "the harness's output for an unset variable, which opens the platform's backend",
    expected = "dx12 at the harness",
    {
        let (line, text) = open_harness(None);
        check_harness(FEATURE, &line, &text)
    }
);

/// The workspace's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `[features]` table's lines of `manifest`, up to the next table.
fn features(manifest: &str) -> Vec<&str> {
    manifest
        .lines()
        .skip_while(|l| l.trim() != "[features]")
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .collect()
}

/// The `wgpu = …` dependency line of `manifest`, if it has one.
fn wgpu_line(manifest: &str) -> Option<&str> {
    manifest
        .lines()
        .find(|l| l.trim_start().starts_with("wgpu ") || l.trim_start().starts_with("wgpu="))
}

/// The crate's manifest (`name` for messages) declares `dx12`, enabling wgpu's DX12 backend, off by default: not in
/// `default`, not among wgpu's own features, and documented, in the comment just above it, "local testing only,
/// untested in CI".
fn check_manifest(name: &str, manifest: &str) {
    let table = features(manifest);
    let at = table
        .iter()
        .position(|l| l.trim_start().starts_with("dx12 ") || l.trim_start().starts_with("dx12="))
        .unwrap_or_else(|| panic!("{name}: no `dx12` feature"));
    assert!(
        table[at].contains("\"wgpu/dx12\""),
        "{name}: the `dx12` feature does not enable wgpu's DX12 backend: {}",
        table[at]
    );
    if let Some(default) = table.iter().find(|l| l.trim_start().starts_with("default")) {
        assert!(
            !default.contains("dx12"),
            "{name}: `dx12` is on by default: {default}"
        );
    }
    let wgpu = wgpu_line(manifest).unwrap_or_else(|| panic!("{name}: no wgpu dependency"));
    assert!(
        !wgpu.contains("dx12"),
        "{name}: wgpu's DX12 backend is in the default build: {wgpu}"
    );
    let comment: String = table[..at]
        .iter()
        .rev()
        .take_while(|l| l.trim_start().starts_with('#'))
        .map(|l| l.trim_start().trim_start_matches('#').trim())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        comment.contains(DOC_WORDS),
        "{name}: the `dx12` feature is not documented \"{DOC_WORDS}\": {comment:?}"
    );
}

/// The deliverable's crates: each that selects a backend from `PRIN_GPU_BACKEND` (validation, gui) and each that builds
/// wgpu (engine, gui, validation).
const CRATES: [&str; 3] = ["engine", "gui", "validation"];

#[test]
fn qa_m0_57_manifests_declare_dx12_off_by_default() {
    for name in CRATES {
        let path = root().join("crates").join(name).join("Cargo.toml");
        let manifest =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        check_manifest(name, &manifest);
    }
}

/// A manifest as the deliverable has it, with `edit` applied.
fn manifest_with(edit: (&str, &str)) -> String {
    let good = "[dependencies]\n\
                wgpu = { version = \"30\", default-features = false, features = [\"metal\", \"vulkan\"] }\n\
                \n\
                [features]\n\
                controls = []\n\
                # DX12, opt-in and off by default (R-862): local testing only, untested in CI.\n\
                dx12 = [\"wgpu/dx12\"]\n";
    good.replacen(edit.0, edit.1, 1)
}

validation::negative_control!(
    qa_m0_57_manifests_declare_dx12_off_by_default,
    "a manifest whose dx12 feature is on by default",
    expected = "is on by default",
    {
        check_manifest("good", &manifest_with(("", "")));
        check_manifest(
            "on by default",
            &manifest_with(("controls = []", "default = [\"dx12\"]")),
        )
    }
);

/// The other ways a manifest can miss the opt-in each fail the check: DX12 in wgpu's base features, the feature
/// enabling nothing of wgpu's, no feature, and the feature undocumented.
#[test]
fn qa_m0_57_manifest_check_sees_each_miss() {
    for (edit, message) in [
        (
            ("\"vulkan\"] }", "\"vulkan\", \"dx12\"] }"),
            "in the default build",
        ),
        (("[\"wgpu/dx12\"]", "[]"), "does not enable wgpu's DX12"),
        (
            ("dx12 = [\"wgpu/dx12\"]", "nothing = []"),
            "no `dx12` feature",
        ),
        (
            ("local testing only, untested in CI", "opt-in"),
            "not documented",
        ),
    ] {
        let manifest = manifest_with(edit);
        let outcome =
            std::panic::catch_unwind(|| check_manifest("edited", &manifest)).expect_err(message);
        let said = outcome
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(said.contains(message), "{message:?}: {said}");
    }
}

validation::negative_control!(
    qa_m0_57_manifest_check_sees_each_miss,
    "the manifest as the deliverable has it, which the check passes",
    expected = "the check refused nothing",
    {
        let manifest = manifest_with(("", ""));
        std::panic::catch_unwind(|| check_manifest("good", &manifest))
            .expect_err("the check refused nothing");
    }
);

/// `line`, a wgpu dependency, keeps DX12 out of its base features.
fn check_wgpu_base(name: &str, line: &str) {
    assert!(
        !line.contains("dx12"),
        "{name}: wgpu's DX12 backend is in the default build: {line}"
    );
}

/// Every wgpu dependency in the workspace keeps DX12 out of its base features, so a default build has no DX12.
#[test]
fn qa_m0_57_no_crate_builds_dx12_by_default() {
    let mut manifests = vec![root().join("xtask/Cargo.toml")];
    for entry in std::fs::read_dir(root().join("crates")).expect("crates/") {
        let path = entry.expect("an entry").path().join("Cargo.toml");
        if path.exists() {
            manifests.push(path);
        }
    }
    let mut read = 0;
    for path in manifests {
        let manifest = std::fs::read_to_string(&path).expect("a manifest");
        if let Some(line) = wgpu_line(&manifest) {
            read += 1;
            check_wgpu_base(&path.display().to_string(), line);
        }
    }
    assert!(read >= CRATES.len(), "read {read} wgpu dependencies");
}

validation::negative_control!(
    qa_m0_57_no_crate_builds_dx12_by_default,
    "a wgpu line with DX12 in its base features",
    expected = "wgpu's DX12 backend is in the default build",
    {
        let manifest = manifest_with(("\"vulkan\"] }", "\"vulkan\", \"dx12\"] }"));
        check_wgpu_base("edited", wgpu_line(&manifest).expect("a wgpu line"))
    }
);

/// Where the harness documents `PRIN_GPU_BACKEND`'s values (`validation/src/gpu.rs`'s module docs) and where the mock
/// canvas does (`gui/src/mock/canvas.rs`'s), `dx12` is named with "local testing only, untested in CI".
fn check_documented(name: &str, source: &str) {
    let docs: String = source
        .lines()
        .take_while(|l| l.starts_with("//!") || l.trim().is_empty())
        .map(|l| l.trim_start_matches("//!").trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        docs.contains(BACKEND_VAR) && docs.contains("dx12") && docs.contains(DOC_WORDS),
        "{name}: the module docs on {BACKEND_VAR} do not document dx12 as \"{DOC_WORDS}\""
    );
}

#[test]
fn qa_m0_57_values_documented_local_testing_only() {
    for file in [
        "crates/validation/src/gpu.rs",
        "crates/gui/src/mock/canvas.rs",
    ] {
        let source = std::fs::read_to_string(root().join(file)).expect("the source");
        check_documented(file, &source);
    }
}

validation::negative_control!(
    qa_m0_57_values_documented_local_testing_only,
    "module docs naming the variable's values without R-862's words",
    expected = "do not document dx12",
    check_documented(
        "synthetic",
        "//! `PRIN_GPU_BACKEND`'s values: `metal`, `vulkan`, and `dx12` with the feature.\n\nuse std::fmt;\n"
    )
);
