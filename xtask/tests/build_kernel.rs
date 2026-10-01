//! `cargo xtask build-kernel` (canonical_spec §1 item 2; R-169): the pinned channel it reads from
//! `rust-toolchain.toml`, its refusal of a rust-gpu backend built on another nightly, naga's SPIR-V → WGSL translation
//! of the kernel it built, and its listing form in `cargo xtask ci --list`, which builds nothing (R-235).
//!
//! `build_kernel_translates_the_kernel` reads `target/spirv/kernel.spv`, so `cargo xtask build-kernel` runs first, as
//! CI runs it before the tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::build_kernel::{
    backend_built, check_channel, exact_requirement, locked_version, pinned_channel,
    prepare_backend, run, to_wgsl, BACKEND_LOCK, BACKEND_TOML, BACKEND_VERSION, SPV, WGSL,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The channel `text` pins is `want`.
fn check_pinned(text: &str, want: &str) {
    assert_eq!(
        pinned_channel(text).as_deref(),
        Ok(want),
        "rust-toolchain.toml does not pin {want}"
    );
}

#[test]
fn build_kernel_reads_the_pin() {
    let text = fs::read_to_string(root().join("rust-toolchain.toml")).expect("rust-toolchain.toml");
    let pinned = pinned_channel(&text).expect("the workspace pins a channel");
    assert!(
        pinned.starts_with("nightly-"),
        "rust-gpu needs a nightly, not {pinned}"
    );
    check_pinned(
        "[toolchain]\nchannel = \"nightly-2026-04-11\"\n",
        "nightly-2026-04-11",
    );
    let err = pinned_channel("[toolchain]\ncomponents = [\"rust-src\"]\n")
        .expect_err("no channel read as a pin");
    assert!(err.contains("no [toolchain] channel"), "{err}");
}

negative_control!(
    build_kernel_reads_the_pin,
    "a pin read from the components line instead of the channel",
    expected = "rust-toolchain.toml does not pin",
    check_pinned("[toolchain]\nchannel = \"stable\"\n", "nightly-2026-04-11")
);

/// A backend on `backend` is refused against the pin `pinned`, naming both.
fn check_refused(pinned: &str, backend: &str) {
    let err =
        check_channel(pinned, backend).expect_err("a backend on another nightly was accepted");
    assert!(err.contains(pinned) && err.contains(backend), "{err}");
}

#[test]
fn build_kernel_refuses_another_nightly() {
    assert_eq!(
        check_channel("nightly-2026-04-11", "nightly-2026-04-11"),
        Ok(())
    );
    check_refused("nightly-2026-04-11", "nightly-2026-05-01");
}

negative_control!(
    build_kernel_refuses_another_nightly,
    "the pinned nightly itself, which must be accepted",
    expected = "a backend on another nightly was accepted",
    check_refused("nightly-2026-04-11", "nightly-2026-04-11")
);

/// The requirement `toml`'s `[dependencies]` gives `package`.
fn requirement(toml: &str, package: &str) -> String {
    let doc: toml_edit::DocumentMut = toml.parse().expect("a manifest");
    let dep = &doc["dependencies"][package];
    dep.as_str()
        .or_else(|| dep.get("version").and_then(|v| v.as_str()))
        .unwrap_or_else(|| panic!("no {package} dependency"))
        .to_owned()
}

/// rust-gpu's backend is pinned exactly at [`BACKEND_VERSION`]: xtask's `cargo-gpu-install`, the kernel's `spirv-std`
/// and the backend crate's `rustc_codegen_spirv` ([`BACKEND_TOML`]) require exactly it, the workspace's `Cargo.lock`
/// holds it, and [`BACKEND_LOCK`] pins `rustc_codegen_spirv` at it.
fn check_pinned_exactly(
    xtask_toml: &str,
    kernel_toml: &str,
    workspace_lock: &str,
    backend_lock: &str,
) {
    for (what, toml, package) in [
        ("xtask", xtask_toml, "cargo-gpu-install"),
        ("kernel", kernel_toml, "spirv-std"),
        ("the rust-gpu backend crate", BACKEND_TOML, "spirv-builder"),
    ] {
        let req = requirement(toml, package);
        let version = exact_requirement(&req).unwrap_or_else(|e| panic!("{what}'s {package}: {e}"));
        assert_eq!(
            version, BACKEND_VERSION,
            "{what}'s {package} is pinned at another version"
        );
    }
    for package in ["cargo-gpu-install", "spirv-std", "spirv-builder"] {
        assert_eq!(
            locked_version(workspace_lock, package).as_deref(),
            Some(BACKEND_VERSION),
            "Cargo.lock does not hold {package} {BACKEND_VERSION}"
        );
    }
    assert_eq!(
        locked_version(backend_lock, "rustc_codegen_spirv").as_deref(),
        Some(BACKEND_VERSION),
        "rust-gpu-backend.lock does not pin rustc_codegen_spirv {BACKEND_VERSION}"
    );
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn build_kernel_pins_the_backend_exactly() {
    check_pinned_exactly(
        &read("xtask/Cargo.toml"),
        &read("crates/kernel/Cargo.toml"),
        &read("Cargo.lock"),
        BACKEND_LOCK,
    );
}

negative_control!(
    build_kernel_pins_the_backend_exactly,
    "the kernel's spirv-std as a caret requirement, which rust-gpu 0.10.0 (another nightly) also meets",
    expected = "is not an exact requirement",
    check_pinned_exactly(
        &read("xtask/Cargo.toml"),
        &read("crates/kernel/Cargo.toml").replace("spirv-std = \"=", "spirv-std = \""),
        &read("Cargo.lock"),
        BACKEND_LOCK,
    )
);

/// The backend library's file name on this host.
fn backend_library() -> String {
    format!(
        "{}rustc_codegen_spirv{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// A scratch directory standing for cargo-gpu's backend crate, with a backend library and `lock` as its `Cargo.lock`.
fn backend_dir(case: &str, lock: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_backend_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(backend_library()), "a stand-in library").unwrap();
    fs::write(dir.join("Cargo.lock"), lock).unwrap();
    dir
}

/// A backend built under `lock` is not taken as built from [`BACKEND_LOCK`]; [`prepare_backend`] then writes the
/// pinned crate and removes the library.
fn check_rebuilt(case: &str, lock: &str) {
    let dir = backend_dir(case, lock);
    assert!(
        !backend_built(&dir),
        "a backend built from rustc_codegen_spirv {:?} under another lockfile was taken as built from rust-gpu-backend.lock",
        locked_version(lock, "rustc_codegen_spirv")
    );
    prepare_backend(&dir).expect("prepare_backend");
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
        BACKEND_TOML
    );
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.lock")).unwrap(),
        BACKEND_LOCK
    );
    assert!(
        !dir.join(backend_library()).exists(),
        "prepare_backend kept the old library"
    );
}

#[test]
fn build_kernel_rebuilds_a_backend_of_another_version() {
    let other = BACKEND_LOCK.replacen(
        &format!("name = \"rustc_codegen_spirv\"\nversion = \"{BACKEND_VERSION}\""),
        "name = \"rustc_codegen_spirv\"\nversion = \"0.10.0\"",
        1,
    );
    assert_ne!(
        other, BACKEND_LOCK,
        "the edit found no rustc_codegen_spirv entry"
    );
    check_rebuilt("other", &other);
    // rustc_codegen_spirv at the pinned version, but one of its dependencies at another: rebuilt too.
    let dependency = locked_dependency_changed();
    assert_eq!(
        locked_version(&dependency, "rustc_codegen_spirv").as_deref(),
        Some(BACKEND_VERSION)
    );
    check_rebuilt("dependency", &dependency);
    // Built under the pinned lockfile: used as it is. With no library: built.
    let same = backend_dir("same", BACKEND_LOCK);
    assert!(backend_built(&same));
    fs::remove_file(same.join(backend_library())).unwrap();
    assert!(!backend_built(&same));
}

/// [`BACKEND_LOCK`] with `spirv-tools`, one of the backend's dependencies, at another version, and
/// `rustc_codegen_spirv` unchanged.
fn locked_dependency_changed() -> String {
    let entry = "name = \"spirv-tools\"\nversion = \"";
    let at = BACKEND_LOCK
        .find(entry)
        .expect("BACKEND_LOCK holds spirv-tools")
        + entry.len();
    let end = at + BACKEND_LOCK[at..].find('"').unwrap();
    format!("{}0.0.1-other{}", &BACKEND_LOCK[..at], &BACKEND_LOCK[end..])
}

negative_control!(
    build_kernel_rebuilds_a_backend_of_another_version,
    "a backend built under the pinned lockfile itself, byte for byte, the one lockfile used as it is: the check \
     that rejects one differing in rustc_codegen_spirv or in any dependency fires on it",
    expected = "was taken as built from rust-gpu-backend.lock",
    check_rebuilt("control", BACKEND_LOCK)
);

/// A workspace under the test's temporary directory holding only `files`, and its `Cargo.toml`'s path.
fn workspace(case: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_ws_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for (name, text) in files {
        fs::write(dir.join(name), text).unwrap();
    }
    dir.join("Cargo.toml")
}

/// build-kernel on the workspace of `manifest` fails, naming `what`, and writes nothing.
fn check_refused_naming(manifest: &Path, what: &str) {
    let err = run(manifest).expect_err("build-kernel passed on a workspace it cannot build");
    assert!(
        err.contains(what),
        "build-kernel's failure does not name {what}: {err}"
    );
    let root = manifest.parent().unwrap();
    assert!(
        !root.join(SPV).exists() && !root.join(WGSL).exists(),
        "build-kernel wrote output on failure"
    );
}

#[test]
fn build_kernel_refuses_a_workspace_without_a_pin() {
    check_refused_naming(
        &workspace("unpinned", &[("Cargo.toml", "[workspace]\n")]),
        "rust-toolchain.toml",
    );
}

negative_control!(
    build_kernel_refuses_a_workspace_without_a_pin,
    "a pinned workspace with no kernel, which fails on the kernel, not the pin",
    expected = "build-kernel's failure does not name rust-toolchain.toml",
    check_refused_naming(
        &workspace(
            "pinned",
            &[
                ("Cargo.toml", "[workspace]\n"),
                (
                    "rust-toolchain.toml",
                    "[toolchain]\nchannel = \"nightly-2026-04-11\"\n"
                ),
            ],
        ),
        "rust-toolchain.toml"
    )
);

/// naga's WGSL for `spv` holds the kernel's entry point at the harness's workgroup size.
fn check_translated(spv: &[u8]) {
    let wgsl = to_wgsl(spv).unwrap_or_else(|e| panic!("the kernel did not translate: {e}"));
    assert!(
        wgsl.contains("@compute @workgroup_size(64, 1, 1)")
            && wgsl.contains("fn toolchain_pack_unpack("),
        "the WGSL has no pack_unpack entry point at workgroup size 64:\n{wgsl}"
    );
}

#[test]
fn build_kernel_translates_the_kernel() {
    let spv = fs::read(root().join(SPV))
        .expect("target/spirv/kernel.spv: run `cargo xtask build-kernel` first");
    check_translated(&spv);
    let err = to_wgsl(&spv[..spv.len() / 2]).expect_err("half a module translated");
    assert!(err.contains("naga could not read the SPIR-V"), "{err}");
}

negative_control!(
    build_kernel_translates_the_kernel,
    "a module cut in half, which naga cannot read",
    expected = "the kernel did not translate",
    {
        let spv = fs::read(root().join(SPV)).expect("target/spirv/kernel.spv");
        check_translated(&spv[..spv.len() / 2])
    }
);

/// `xtask <args>` against a stand-in `cargo` (`CARGO`) that lists one test with its control, answers any run as that
/// control tripping, and answers `run … -- build-kernel` with exit status `build`: whether xtask passed, its output,
/// and each argument list the stand-in was called with.
fn with_stand_in(case: &str, args: &[&str], build: u8) -> (bool, String, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    validation::spawn::write_executable(
        &dir.join("cargo"),
        format!(
            r#"#!/bin/sh
echo "$*" >> '{dir}/calls.log'
if [ "$1" = metadata ]; then
  echo '{{"packages":[{{"name":"build_kernel_fake","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}]}}'
  exit 0
fi
for a in "$@"; do
  if [ "$a" = --list ]; then printf 'build_kernel_t::t: test\nbuild_kernel_t::t::negative_control: test\n'; exit 0; fi
  if [ "$a" = build-kernel ]; then exit {build}; fi
done
echo 'test build_kernel_t::t::negative_control - should panic ... ok'
"#,
            dir = dir.display()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env("CARGO", dir.join("cargo"))
        .timed_output()
        .expect("run xtask");
    let calls = fs::read_to_string(dir.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), out, calls)
}

/// `calls` show no build, and `out` names both files build-kernel writes.
fn check_listed((_, out, calls): (bool, String, Vec<String>)) {
    assert!(
        !calls.iter().any(|c| c.ends_with("build-kernel")),
        "xtask ci --list built the kernel: {calls:?}"
    );
    assert!(
        out.contains(&format!("build-kernel: would write {SPV} and {WGSL}")),
        "xtask ci --list did not list build-kernel's files:\n{out}"
    );
}

#[test]
fn build_kernel_listed_in_ci() {
    check_listed(with_stand_in("listed", &["ci", "--list"], 0));
}

negative_control!(
    build_kernel_listed_in_ci,
    "bare `xtask ci`, which builds the kernel, given to the listing check",
    expected = "xtask ci --list built the kernel",
    check_listed(with_stand_in("ctl_listed", &["ci"], 0))
);

/// Bare `xtask ci` ran build-kernel through the stand-in cargo, `cargo run … -p xtask -- build-kernel`, and passed
/// exactly when that run did (`build` its exit status).
fn check_built_in_ci((ok, out, calls): (bool, String, Vec<String>), build: u8) {
    assert!(
        calls
            .iter()
            .any(|c| c.starts_with("run --quiet --manifest-path")
                && c.ends_with("-p xtask -- build-kernel")),
        "xtask ci did not run build-kernel through its cargo: {calls:?}"
    );
    if build == 0 {
        assert!(ok, "xtask ci failed with build-kernel passing:\n{out}");
    } else {
        assert!(
            !ok && out.contains("build-kernel FAILED"),
            "xtask ci did not fail naming build-kernel when it failed:\n{out}"
        );
    }
}

#[test]
fn build_kernel_runs_in_ci() {
    check_built_in_ci(with_stand_in("built", &["ci"], 0), 0);
    check_built_in_ci(with_stand_in("build_failed", &["ci"], 1), 1);
}

negative_control!(
    build_kernel_runs_in_ci,
    "a failing build-kernel run, given to the check as passing",
    expected = "xtask ci failed with build-kernel passing",
    check_built_in_ci(with_stand_in("ctl_built", &["ci"], 1), 0)
);
