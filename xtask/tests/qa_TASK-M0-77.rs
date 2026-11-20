//! qa's tests for TASK-M0-77 (REQ-TOOL-253), written from the requirement, not the code: the GPU jobs take the kernel
//! from a cache keyed on its source hash, and the build step stays `cargo xtask build-kernel`, so the cached build must
//! do on a hit what the plain build does after compiling, the fragment snippets checked against the kernel's SPIR-V
//! (R-116). Otherwise a hand edit of a fragment file, which is not a kernel input and so doesn't change the hash, would
//! pass every warm CI run.
//!
//! The test drives `xtask::build_kernel::run_cached` on a scratch workspace, never the binary, which would write into
//! this checkout's `target/spirv/`. The scratch workspace's `rust-toolchain.toml` names no channel, so a miss fails at
//! once in the plain build's first step, with an error of its own, and starts no rust-gpu build.
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};

use validation::negative_control;
use xtask::build_kernel::run_cached;
use xtask::kernel_cache::{self, INPUTS, OUTPUTS};

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa_m0_77_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("an old scratch folder removed");
    }
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
    fs::write(path, bytes).expect("a file");
}

/// A workspace with a file for each kernel input; its `rust-toolchain.toml` names no channel.
fn workspace() -> PathBuf {
    let root = scratch("ws");
    for input in INPUTS {
        let path = if input.contains('.') {
            root.join(input)
        } else {
            root.join(input).join("file.rs")
        };
        write(&path, format!("# {input}\n").as_bytes());
    }
    root
}

/// An entry, stored under `key` in a fresh cache, whose SPIR-V is bytes naga can't read; then the cached build of the
/// workspace. On a hit the outputs are restored and the fragment check runs on the restored SPIR-V, so the build fails
/// with naga's error, not the plain build's.
fn check_hit_checks_fragments(key: impl Fn(&Path) -> String) {
    let root = workspace();
    let cache = scratch("cache");
    let built = scratch("built");
    for output in OUTPUTS {
        write(
            &built.join(output),
            format!("not a kernel: {output}\n").as_bytes(),
        );
    }
    kernel_cache::store(&cache, &key(&root), &built).expect("stored");
    let err = run_cached(&root.join("Cargo.toml"), &cache)
        .expect_err("a cached SPIR-V naga can't read passed the build");
    assert!(
        err.contains("naga could not read the SPIR-V"),
        "the cached build's fragment check didn't run on the cached SPIR-V: {err}"
    );
    for output in OUTPUTS {
        assert_eq!(
            fs::read(root.join(output)).expect("restored"),
            fs::read(built.join(output)).expect("built"),
            "{output} not restored from the cache"
        );
    }
}

#[test]
fn cached_build_checks_the_fragments_on_a_hit() {
    check_hit_checks_fragments(|root| kernel_cache::source_hash(root).expect("a hash"));
}

negative_control!(
    cached_build_checks_the_fragments_on_a_hit,
    "the entry stored under another hash, so the build misses and fails at the plain build's first step",
    expected = "the cached build's fragment check didn't run on the cached SPIR-V",
    check_hit_checks_fragments(|_| "0".repeat(64))
);
