//! `cargo xtask build-kernel` (canonical_spec §1 item 2; R-169): the pinned channel it reads from
//! `rust-toolchain.toml`, its refusal of a rust-gpu backend built on another nightly, its exact pin of the backend and
//! the backend build it makes in a scratch crate through a stand-in cargo (never rust-gpu's real cache), naga's SPIR-V
//! → WGSL translation of the kernel it built, and its listing form in `cargo xtask ci --list`, which builds nothing
//! (R-235).
//!
//! `build_kernel_translates_the_kernel` reads `target/spirv/kernel.spv`, so `cargo xtask build-kernel` runs first, as
//! CI runs it before the tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use cargo_gpu_install::spirv_source::SpirvSource;
use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::build_kernel::{
    backend_built, build_backend, check_channel, check_locked, check_source, ensure_backend,
    exact_requirement, locked_version, pinned_channel, prepare_backend, run, run_cached_with,
    spirv_fingerprints, to_parity_wgsl, to_read_wgsl, to_wgsl, BACKEND_LOCK, BACKEND_TOML,
    BACKEND_VERSION, PARITY_ENTRIES, PARITY_WGSL, READ_ENTRIES, READ_WGSL, SPV, TARGET, WGSL,
};
use xtask::fragment_wgsl::{DECODE_FILE, ENCODE_FILE, SHAPE_FILE};
use xtask::kernel_cache::{self, FINGERPRINT, OUTPUTS};

#[path = "support/stand_in.rs"]
mod stand_in;

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

/// The read-back's entries, 13 of them, are in `read` alone (TASK-M4-27): the production module holds none.
fn check_parted(production: &str, read: &str) {
    let entries = |wgsl: &str| wgsl.matches("@compute").count();
    let read_back = format!("fn {READ_ENTRIES}");
    assert_eq!(
        production.matches(&read_back).count(),
        0,
        "the production module holds a read-back entry"
    );
    assert_eq!(read.matches(&read_back).count(), 13);
    assert_eq!(
        entries(read),
        13,
        "the read-back's module holds another entry"
    );
    assert!(entries(production) > 0);
}

#[test]
fn build_kernel_parts_the_read_back_from_the_production_module() {
    let spv = fs::read(root().join(SPV))
        .expect("target/spirv/kernel.spv: run `cargo xtask build-kernel` first");
    let (production, read) = (to_wgsl(&spv).unwrap(), to_read_wgsl(&spv).unwrap());
    check_parted(&production, &read);
    let on_disk = fs::read_to_string(root().join(READ_WGSL)).expect("the read-back's module");
    assert!(on_disk == read, "{READ_WGSL} is not the translation");
    let err = to_read_wgsl(&spv[..spv.len() / 2]).expect_err("half a module translated");
    assert!(err.contains("naga could not read the SPIR-V"), "{err}");
}

negative_control!(
    build_kernel_parts_the_read_back_from_the_production_module,
    "the two modules taken the wrong way round",
    expected = "the production module holds a read-back entry",
    {
        let spv = fs::read(root().join(SPV)).expect("target/spirv/kernel.spv");
        check_parted(&to_read_wgsl(&spv).unwrap(), &to_wgsl(&spv).unwrap())
    }
);

/// The parity harness's entries, 7 of them, are in `parity` alone: the production module holds none.
fn check_parity_parted(production: &str, parity: &str) {
    let entry = format!("fn {PARITY_ENTRIES}");
    assert_eq!(
        production.matches(&entry).count(),
        0,
        "the production module holds a parity entry"
    );
    assert_eq!(parity.matches(&entry).count(), 7);
    assert_eq!(
        parity.matches("@compute").count(),
        7,
        "the parity module holds another entry"
    );
}

#[test]
fn build_kernel_parts_the_parity_entries_from_the_other_modules() {
    let spv = fs::read(root().join(SPV))
        .expect("target/spirv/kernel.spv: run `cargo xtask build-kernel` first");
    let parity = to_parity_wgsl(&spv).unwrap();
    check_parity_parted(&to_wgsl(&spv).unwrap(), &parity);
    check_parity_parted(&to_read_wgsl(&spv).unwrap(), &parity);
    let on_disk = fs::read_to_string(root().join(PARITY_WGSL)).expect("the parity module");
    assert!(on_disk == parity, "{PARITY_WGSL} is not the translation");
}

negative_control!(
    build_kernel_parts_the_parity_entries_from_the_other_modules,
    "the two modules taken the wrong way round",
    expected = "the production module holds a parity entry",
    {
        let spv = fs::read(root().join(SPV)).expect("target/spirv/kernel.spv");
        check_parity_parted(&to_parity_wgsl(&spv).unwrap(), &to_wgsl(&spv).unwrap())
    }
);

/// `xtask <args>` against a stand-in `cargo` (`CARGO`) that lists one test with its control, answers `run … --
/// build-kernel` with exit status `build`, and answers any other run, each of bare `ci`'s other steps among them, as that
/// control tripping: whether xtask passed, its output, and each argument list the stand-in was called with.
fn with_stand_in(case: &str, args: &[&str], build: u8) -> (bool, String, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cargo = stand_in::stand_in_cargo(
        &dir,
        json!({
            "log": dir.join("calls.log"),
            "metadata": stand_in::metadata("build_kernel_fake"),
            "listed": "build_kernel_t::t: test\nbuild_kernel_t::t::negative_control: test\n",
            "build": build,
            "output": "test build_kernel_t::t::negative_control - should panic ... ok\n",
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env("CARGO", &cargo)
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

/// `requirement` is refused as not exact.
fn check_not_exact(requirement: &str) {
    assert!(
        exact_requirement(requirement).is_err(),
        "{requirement:?} was read as an exact requirement"
    );
}

#[test]
fn build_kernel_reads_only_an_exact_requirement() {
    assert_eq!(
        exact_requirement(&format!(" = {BACKEND_VERSION} ")),
        Ok(BACKEND_VERSION)
    );
    for requirement in [
        "=",
        "= ",
        "=0.10.0-alpha.1, <0.11",
        "=0.10.*",
        "=>0.10",
        "^0.10.0-alpha.1",
        "~0.10",
        "0.10.0-alpha.1",
        "=0.10.0 alpha",
    ] {
        check_not_exact(requirement);
    }
}

negative_control!(
    build_kernel_reads_only_an_exact_requirement,
    "the exact requirement itself, given to the refusal check",
    expected = "was read as an exact requirement",
    check_not_exact("=0.10.0-alpha.1")
);

/// A scratch directory standing for cargo-gpu's backend crate, holding nothing.
fn empty_backend_dir(case: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_backend_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// [`prepare_backend`] on `dir` writes the pinned crate and leaves no library.
fn check_prepared(dir: &Path) {
    prepare_backend(dir).unwrap_or_else(|e| panic!("prepare_backend failed: {e}"));
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
        BACKEND_TOML
    );
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.lock")).unwrap(),
        BACKEND_LOCK
    );
    assert_eq!(fs::read_to_string(dir.join("src/lib.rs")).unwrap(), "");
    assert!(!dir.join(backend_library()).exists());
}

/// [`prepare_backend`] on `dir`, whose library cannot be removed, fails naming the library.
fn check_prepare_refused(dir: &Path) {
    let err = prepare_backend(dir)
        .expect_err("prepare_backend passed with a library it could not remove");
    assert!(err.contains(&backend_library()), "{err}");
}

#[test]
fn build_kernel_prepares_a_crate_with_no_library() {
    // No library built there before: nothing to remove, and no error.
    check_prepared(&empty_backend_dir("prepare_absent"));
    // A library that cannot be removed (a directory in its place): an error naming it, not one taken as absent.
    let blocked = empty_backend_dir("prepare_blocked");
    fs::create_dir_all(blocked.join(backend_library()).join("inside")).unwrap();
    check_prepare_refused(&blocked);
}

negative_control!(
    build_kernel_prepares_a_crate_with_no_library,
    "a crate with no library at all, whose preparation passes, given to the refusal check",
    expected = "prepare_backend passed with a library it could not remove",
    check_prepare_refused(&empty_backend_dir("ctl_prepare_absent"))
);

/// A stand-in `cargo` in its own scratch directory: it logs its working directory and arguments to `calls.log` there,
/// writes a library at `target/release/<backend library>` in its working directory, and exits with `status`. Its path
/// and the log's.
fn stand_in_cargo(case: &str, status: u8) -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_kernel_cargo_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let log = dir.join("calls.log");
    let cargo = stand_in::stand_in_cargo(
        &dir,
        json!({
            "log": log,
            "log_cwd": true,
            "library": backend_library(),
            "status": status,
        }),
    );
    (cargo, log)
}

/// Runs `command` through the shared spawn helper (R-214, REQ-SYS-070): what [`build_backend`] and [`ensure_backend`] are
/// given to run cargo with here, in place of `Command::status`.
fn spawned(command: &mut Command) -> std::io::Result<ExitStatus> {
    command.timed_output().map(|output| output.status)
}

/// The lines of the stand-in cargo's log `log`; none if it never ran.
fn calls(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// [`build_backend`] with a stand-in cargo exiting `status`, in an empty crate: it ran cargo once, in the crate, as
/// `+<channel> build --release --locked` on the pinned crate; on success the library is moved into the crate and
/// `target` removed, and on failure the build fails naming the backend.
fn check_backend_build(case: &str, status: u8) {
    let dir = empty_backend_dir(case);
    let (cargo, log) = stand_in_cargo(case, status);
    let result = build_backend(&dir, "nightly-2026-04-11", Command::new(cargo), spawned);
    // Without Windows' verbatim prefix, which `canonicalize` adds there and no working directory prints (REQ-TOOL-237).
    let dir_real = validation::paths::plain(&dir.canonicalize().unwrap());
    assert_eq!(
        calls(&log),
        [format!(
            "{} +nightly-2026-04-11 build --release --locked",
            dir_real.display()
        )],
        "build_backend did not run cargo once, in the crate, on the pinned channel under --locked"
    );
    assert_eq!(
        fs::read_to_string(dir.join("Cargo.lock")).unwrap(),
        BACKEND_LOCK
    );
    if status == 0 {
        result.unwrap_or_else(|e| panic!("build_backend failed with cargo passing: {e}"));
        assert!(
            dir.join(backend_library()).is_file() && !dir.join("target").exists(),
            "build_backend left the library in target"
        );
        assert!(backend_built(&dir));
    } else {
        let err = result.expect_err("build_backend passed with cargo failing");
        assert!(err.contains("building rust-gpu's backend failed"), "{err}");
    }
}

#[test]
fn build_kernel_builds_the_backend_locked() {
    check_backend_build("build_ok", 0);
    check_backend_build("build_failed", 1);
}

negative_control!(
    build_kernel_builds_the_backend_locked,
    "a failing cargo, given to the check as passing",
    expected = "build_backend failed with cargo passing",
    {
        let dir = empty_backend_dir("ctl_build");
        let (cargo, _) = stand_in_cargo("ctl_build", 1);
        let result = build_backend(&dir, "nightly-2026-04-11", Command::new(cargo), spawned);
        result.unwrap_or_else(|e| panic!("build_backend failed with cargo passing: {e}"));
    }
);

/// [`ensure_backend`] on `dir` ran the stand-in cargo `builds` times (0 or 1), and leaves the backend built.
fn check_ensured(case: &str, dir: &Path, builds: usize) {
    let (cargo, log) = stand_in_cargo(case, 0);
    ensure_backend(dir, "nightly-2026-04-11", Command::new(cargo), spawned)
        .unwrap_or_else(|e| panic!("ensure_backend failed: {e}"));
    assert_eq!(
        calls(&log).len(),
        builds,
        "ensure_backend built the backend a number of times other than {builds}"
    );
    assert!(backend_built(dir), "ensure_backend left no backend built");
}

#[test]
fn build_kernel_builds_only_a_backend_not_built() {
    // An empty crate: built.
    check_ensured("ensure_empty", &empty_backend_dir("ensure_empty"), 1);
    // Built under the pinned lockfile: used as it is.
    check_ensured(
        "ensure_built",
        &backend_dir("ensure_built", BACKEND_LOCK),
        0,
    );
}

negative_control!(
    build_kernel_builds_only_a_backend_not_built,
    "an empty crate, which is built, given to the check as one already built",
    expected = "ensure_backend built the backend a number of times other than 0",
    check_ensured("ctl_ensure", &empty_backend_dir("ctl_ensure"), 0)
);

/// [`check_locked`] refuses the crate at `dir`, naming rust-gpu-backend.lock.
fn check_lock_refused(dir: &Path) {
    let err =
        check_locked(dir).expect_err("a lockfile other than rust-gpu-backend.lock was accepted");
    assert!(err.contains("rust-gpu-backend.lock"), "{err}");
}

#[test]
fn build_kernel_refuses_a_backend_resolved_afresh() {
    assert_eq!(
        check_locked(&backend_dir("locked_same", BACKEND_LOCK)),
        Ok(())
    );
    check_lock_refused(&backend_dir(
        "locked_dependency",
        &locked_dependency_changed(),
    ));
    check_lock_refused(&empty_backend_dir("locked_none"));
}

negative_control!(
    build_kernel_refuses_a_backend_resolved_afresh,
    "a crate locked by rust-gpu-backend.lock itself, given to the refusal check",
    expected = "a lockfile other than rust-gpu-backend.lock was accepted",
    check_lock_refused(&backend_dir("ctl_locked", BACKEND_LOCK))
);

/// A crates.io rust-gpu source at `version`, as cargo-gpu's installer names one given a version: built directly, so
/// no `cargo metadata` is run.
fn crates_io(version: &str) -> SpirvSource {
    SpirvSource::CratesIO(version.parse().expect("a semver version"))
}

/// `source` is refused, naming crates.io's [`BACKEND_VERSION`].
fn check_source_refused(source: &SpirvSource) {
    let err = check_source(source)
        .expect_err("a rust-gpu other than crates.io's pinned release was accepted");
    assert!(
        err.contains(&format!("not crates.io {BACKEND_VERSION}")),
        "{err}"
    );
}

#[test]
fn build_kernel_takes_only_the_pinned_rust_gpu() {
    assert_eq!(check_source(&crates_io(BACKEND_VERSION)), Ok(()));
    check_source_refused(&crates_io("0.10.0"));
    check_source_refused(&SpirvSource::Git {
        url: "https://github.com/Rust-GPU/rust-gpu".to_owned(),
        rev: BACKEND_VERSION.to_owned(),
    });
}

negative_control!(
    build_kernel_takes_only_the_pinned_rust_gpu,
    "crates.io's pinned release itself, given to the refusal check",
    expected = "a rust-gpu other than crates.io's pinned release was accepted",
    check_source_refused(&crates_io(BACKEND_VERSION))
);

// ── the workspace's host flags never reach the SPIR-V build (R-547, REQ-TOOL-188) ──────────────────────────────

/// spirv-builder 0.10.0-alpha.1's own rustflags for the SPIR-V build, after its `-Zcodegen-backend=<backend>`
/// (`spirv-builder/src/lib.rs`, `invoke_rustc`, the `rustflags` list), as build-kernel's builder configures it: no
/// `-Cllvm-args`, target features, `-Dwarnings` or RUSTGPU_RUSTFLAGS. Its own `-Zmir-enable-passes=-GVN` is among
/// them; spirv-builder passes the list as CARGO_ENCODED_RUSTFLAGS, which overrides every rustflags table of the
/// workspace's `.cargo/config.toml`.
const SPIRV_BUILDER_FLAGS: &[&str] = &[
    "-Zbinary-dep-depinfo",
    "-Csymbol-mangling-version=v0",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(rust_gpu)",
    "-Coverflow-checks=off",
    "-Cdebug-assertions=off",
    "-Zinline-mir=off",
    "-Zmir-enable-passes=-GVN",
    "-Zshare-generics=off",
];

/// `flags`, a SPIR-V build's rustflags, are exactly spirv-builder's own: its codegen backend, then
/// [`SPIRV_BUILDER_FLAGS`], nothing of the workspace's added.
fn check_spirv_flags(flags: &[String]) {
    assert!(
        flags
            .first()
            .is_some_and(|f| f.starts_with("-Zcodegen-backend=")),
        "the SPIR-V build's rustflags do not start with spirv-builder's codegen backend: {flags:?}"
    );
    assert!(
        flags[1..] == *SPIRV_BUILDER_FLAGS,
        "the SPIR-V build's rustflags are not spirv-builder's own: {flags:?}"
    );
}

/// cargo's fingerprint directory of the kernel's SPIR-V builds, in spirv-builder's target directory
/// (`<target>/spirv-builder/<target triple>/release/.fingerprint`).
fn spirv_fingerprint_dir() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .expect("the target directory")
        .join("spirv-builder")
        .join(TARGET)
        .join("release/.fingerprint")
}

/// The rustflags each build of the kernel's library for the SPIR-V target was compiled with, as cargo's fingerprints
/// in spirv-builder's target directory record them ([`spirv_fingerprint_dir`]), written by `cargo xtask build-kernel`;
/// at least one. On a kernel cache hit they are the fingerprints of the build that made the cached kernel, stored in its
/// entry with it and put back here (`xtask::build_kernel::run_cached`), so the check below reads a real build's flags
/// either way.
fn spirv_kernel_rustflags() -> Vec<Vec<String>> {
    let target = spirv_fingerprint_dir();
    let mut found = Vec::new();
    for entry in fs::read_dir(&target).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; run `cargo xtask build-kernel` first",
            target.display()
        )
    }) {
        let path = entry.unwrap().path().join("lib-kernel.json");
        if let Ok(text) = fs::read_to_string(&path) {
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            let flags = json["rustflags"]
                .as_array()
                .unwrap_or_else(|| panic!("{}: no rustflags", path.display()))
                .iter()
                .map(|f| f.as_str().unwrap().to_owned())
                .collect();
            found.push(flags);
        }
    }
    assert!(
        !found.is_empty(),
        "{}: no fingerprint of the kernel's SPIR-V build",
        target.display()
    );
    found
}

#[test]
fn build_kernel_spirv_flags_are_spirv_builders_own() {
    for flags in spirv_kernel_rustflags() {
        check_spirv_flags(&flags);
    }
}

negative_control!(
    build_kernel_spirv_flags_are_spirv_builders_own,
    "the kernel's SPIR-V rustflags with the workspace's host flag added to spirv-builder's own",
    expected = "the SPIR-V build's rustflags are not spirv-builder's own",
    {
        let mut flags = spirv_kernel_rustflags().remove(0);
        flags.push("-Zmir-enable-passes=-GVN".to_owned());
        check_spirv_flags(&flags)
    }
);

/// `found`, the fingerprint directory `build-kernel` puts a cached build's fingerprints in, is the one the flags test
/// above reads.
fn check_fingerprint_dir(found: &Path) {
    assert_eq!(
        found,
        spirv_fingerprint_dir(),
        "build-kernel's fingerprint directory is not the one the flags test reads"
    );
}

#[test]
fn build_kernel_cache_restores_fingerprints_where_the_flags_test_reads() {
    check_fingerprint_dir(&spirv_fingerprints(&root()).expect("the fingerprint directory"));
}

negative_control!(
    build_kernel_cache_restores_fingerprints_where_the_flags_test_reads,
    "the host build's fingerprint directory in spirv-builder's target directory",
    expected = "build-kernel's fingerprint directory is not the one the flags test reads",
    check_fingerprint_dir(
        &spirv_fingerprints(&root())
            .expect("the fingerprint directory")
            .join("../../../release/.fingerprint")
    )
);

/// A scratch folder under this test binary's own.
fn cache_scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "build_kernel_cache_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("an old scratch folder removed");
    }
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// A workspace holding this one's fragment snippet files, and a cache whose entry for its source hash holds this
/// workspace's built kernel (`cargo xtask build-kernel`'s outputs) and, with `fingerprints`, a kernel fingerprint
/// (`store_built`), or none (`store`); then the cached build of that workspace into a fresh fingerprint directory. The
/// hit passes the fragment check on the real kernel, and the fingerprint is put back where the build wrote it.
fn check_hit_restores_fingerprints(fingerprints: bool) {
    let ws = cache_scratch("ws");
    for file in [DECODE_FILE, ENCODE_FILE, SHAPE_FILE] {
        let to = ws.join(file);
        fs::create_dir_all(to.parent().expect("a parent")).expect("a folder");
        fs::copy(root().join(file), &to).expect("a fragment file copied");
    }
    let built = cache_scratch("built");
    for output in OUTPUTS {
        let to = built.join(output);
        fs::create_dir_all(to.parent().expect("a parent")).expect("a folder");
        fs::copy(root().join(output), &to)
            .unwrap_or_else(|e| panic!("{output}: {e}; run `cargo xtask build-kernel` first"));
    }
    let from = cache_scratch("from");
    let file = Path::new("kernel-4d5226f7ba38a476").join(FINGERPRINT);
    fs::create_dir_all(from.join("kernel-4d5226f7ba38a476")).expect("a folder");
    fs::write(from.join(&file), "{\"rustflags\":[]}").expect("a fingerprint");
    let cache = cache_scratch("cache");
    let hash = kernel_cache::source_hash(&ws).expect("a hash");
    if fingerprints {
        kernel_cache::store_built(&cache, &hash, &built, &from).expect("stored");
    } else {
        kernel_cache::store(&cache, &hash, &built).expect("stored");
    }
    let into = cache_scratch("into");
    run_cached_with(&ws.join("Cargo.toml"), &cache, Ok(into.clone()))
        .expect("the cached build of a whole entry");
    assert_eq!(
        fs::read(into.join(&file)).expect("the fingerprint restored"),
        fs::read(from.join(&file)).expect("the fingerprint stored"),
        "the restored fingerprint differs"
    );
}

#[test]
fn build_kernel_cache_hit_restores_the_builds_fingerprints() {
    check_hit_restores_fingerprints(true);
}

negative_control!(
    build_kernel_cache_hit_restores_the_builds_fingerprints,
    "the entry stored without its build's fingerprints",
    expected = "holds no fingerprint of its SPIR-V build",
    check_hit_restores_fingerprints(false)
);

/// The table of `.cargo/config.toml` that carries the native build's `-Zmir-enable-passes=-GVN` (R-547).
const HOST_TABLE: &str = r#"cfg(not(target_arch = "spirv"))"#;

/// In `config`, the text of a `.cargo/config.toml`: `[target.'cfg(not(target_arch = "spirv"))']` carries
/// `-Zmir-enable-passes=-GVN`, and no other rustflags, `[build]`'s or another target table's, carries any
/// `-Zmir-enable-passes`, so the flag reaches every target but SPIR-V.
fn check_config_scope(config: &str) {
    let doc: toml_edit::DocumentMut = config.parse().expect(".cargo/config.toml parses");
    let flags = |item: Option<&toml_edit::Item>| -> Vec<String> {
        item.and_then(|t| t.get("rustflags"))
            .and_then(|f| f.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let gvn = |flags: &[String]| flags.iter().any(|f| f.starts_with("-Zmir-enable-passes"));
    let targets = doc.get("target").and_then(|t| t.as_table_like());
    assert!(
        flags(targets.and_then(|t| t.get(HOST_TABLE)))
            .contains(&"-Zmir-enable-passes=-GVN".to_owned()),
        "[target.'{HOST_TABLE}'] does not carry -Zmir-enable-passes=-GVN"
    );
    assert!(
        !gvn(&flags(doc.get("build"))),
        "[build] rustflags carry -Zmir-enable-passes, which a build for the SPIR-V target would take"
    );
    for (name, table) in targets.into_iter().flat_map(|t| t.iter()) {
        assert!(
            name == HOST_TABLE || !gvn(&flags(Some(table))),
            "[target.'{name}'] carries -Zmir-enable-passes"
        );
    }
}

#[test]
fn build_kernel_gvn_flag_is_scoped_off_spirv() {
    check_config_scope(&read(".cargo/config.toml"));
}

negative_control!(
    build_kernel_gvn_flag_is_scoped_off_spirv,
    "the workspace's config with the GVN flag in [build] rustflags too, which reach every target",
    expected = "[build] rustflags carry -Zmir-enable-passes",
    check_config_scope(&format!(
        "{}\n[build]\nrustflags = [\"-Zmir-enable-passes=-GVN\"]\n",
        read(".cargo/config.toml")
    ))
);

/// The same check, here for its control on the host's table: the native build takes the flag at all.
#[test]
fn build_kernel_gvn_flag_reaches_the_host() {
    check_config_scope(&read(".cargo/config.toml"));
}

negative_control!(
    build_kernel_gvn_flag_reaches_the_host,
    "the workspace's config with the host table's rustflags emptied",
    expected = "does not carry -Zmir-enable-passes=-GVN",
    check_config_scope(&read(".cargo/config.toml").replace(
        "rustflags = [\"-Zmir-enable-passes=-GVN\"]",
        "rustflags = []"
    ))
);
