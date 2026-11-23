//! Physics defined once, compiled twice (canonical_spec §1; core_design §1; parity_contract Tier L, part 1): the GPU
//! module and the CPU reference are built from `crates/kernel` only, and nothing of the march exists a second time.
//!
//! - `single_source_one_gpu_crate`: `crates/kernel` is the one crate rust-gpu builds: the one manifest that takes
//!   `spirv-std`, and the crate `cargo xtask build-kernel` compiles; the CPU side reaches the same crate as a
//!   dependency. Its control adds a second manifest taking `spirv-std`.
//! - `single_source_no_wgsl_compute_source`: no compute WGSL is checked in or written in a production Rust source: no
//!   `@compute` entry in any `.wgsl` file or `.rs` file outside test code. Test harness entries over the generated read
//!   side (`crates/ledger/tests/*.wgsl`, the engine's `compute/tests.rs`) are test code. Its control seeds a hand-written
//!   march in `crates/render/wgsl/`.
//! - `single_source_one_implementation`: outside `crates/kernel`, no type implements the occupant seam (`Advance`), a
//!   stepper (`Stepper`, `AzStepper`, `HgStepper`, `LhStepper`) or a regularisation (`Regularisation`), so step,
//!   wrapper, detector and decode logic have one implementation, instantiated at f32 and f64 by the kernel's generics.
//!   A `Cadence` outside the kernel is an observer the march calls after each step (an inspector's recorder, a
//!   validation probe), not a step; a test double in test code is not a second implementation either. Its control
//!   seeds an `impl Advance` in the engine.
//!
//! What a scan cannot see, a hand-written copy of the physics under another name, is the code reviewer's inspection
//! (Tier L part 1: by inspection, never by agreement of results). The engine's `precision` path (RK45, DOP853) is the
//! independent high-accuracy reference, a different integrator by design, not a copy of an occupant.

use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use validation::negative_control;

/// The kernel's seams and steppers: a type implementing one outside `crates/kernel` is a second implementation.
const SEAMS: [&str; 6] = [
    "Advance",
    "Stepper",
    "AzStepper",
    "HgStepper",
    "LhStepper",
    "Regularisation",
];

/// The production files that hold compute WGSL that is not physics, each with its exact count of entries and why.
const COMPUTE_ALLOWED: &[(&str, usize, &str)] = &[(
    "crates/validation/src/gpu.rs",
    5,
    "the GPU harness's own probes: the identity kernel, the fast-math probe's two kernels and the two extractBits \
     overloads; no physics",
)];

/// The workspace root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file below the root with one of `exts`, by its path from the root, skipping build output, git's own, and the
/// local, uncommitted folders.
fn files(exts: &[&str]) -> Vec<(String, String)> {
    let root = root();
    let mut stack = vec![root.clone()];
    let mut out = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("a directory") {
            let path = entry.expect("a directory entry").path();
            let rel = path
                .strip_prefix(&root)
                .expect("below the root")
                .to_string_lossy()
                .replace('\\', "/");
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                let skip = name.starts_with('.')
                    || name.starts_with("target")
                    || name == "node_modules"
                    || rel == "docs/incoming";
                if !skip {
                    stack.push(path);
                }
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| exts.contains(&e))
            {
                if let Ok(text) = fs::read_to_string(&path) {
                    out.push((rel, text));
                }
            }
        }
    }
    out.sort();
    out
}

/// Whether the file at `rel` is test code: under a `tests`, `fixtures`, `benches` or `examples` directory, or a
/// `tests.rs` module.
fn test_code(rel: &str) -> bool {
    let mut parts = rel.split('/');
    let name = rel.rsplit('/').next().unwrap_or_default();
    name == "tests.rs" || parts.any(|p| matches!(p, "tests" | "fixtures" | "benches" | "examples"))
}

/// Each `@compute` entry in `text`: the attribute followed, past whitespace, by another attribute or `fn`.
fn compute_entries(text: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    let mut from = 0;
    while let Some(n) = text[from..].find("@compute") {
        let at = from + n;
        let rest = text[at + "@compute".len()..].trim_start();
        if rest.starts_with('@') || rest.starts_with("fn ") {
            lines.push(1 + text[..at].matches('\n').count());
        }
        from = at + 1;
    }
    lines
}

/// Panics naming each production file of `files` that holds a compute entry.
fn check_no_compute(files: &[(String, String)]) {
    let found: Vec<String> = files
        .iter()
        .filter(|(rel, _)| !test_code(rel))
        .filter_map(|(rel, text)| {
            let lines = compute_entries(text);
            let allowed = COMPUTE_ALLOWED
                .iter()
                .find(|a| a.0 == rel)
                .map_or(0, |a| a.1);
            (lines.len() != allowed).then(|| format!("{rel} ({allowed} allowed): lines {lines:?}"))
        })
        .collect();
    assert!(
        files
            .iter()
            .any(|(rel, _)| rel == "crates/render/wgsl/decode.wgsl"),
        "the scan reads the checked-in WGSL"
    );
    assert!(
        found.is_empty(),
        "compute WGSL outside the kernel's translation: {}",
        found.join(", ")
    );
}

#[test]
fn single_source_no_wgsl_compute_source() {
    check_no_compute(&files(&["wgsl", "rs"]));
}

negative_control!(
    single_source_no_wgsl_compute_source,
    "a hand-written march in crates/render/wgsl/",
    expected = "compute WGSL outside the kernel's translation",
    {
        let mut all = files(&["wgsl", "rs"]);
        all.push((
            "crates/render/wgsl/march.wgsl".to_owned(),
            "@compute @workgroup_size(64)\nfn march(@builtin(global_invocation_id) id: vec3<u32>) {}\n"
                .to_owned(),
        ));
        check_no_compute(&all)
    }
);

/// Collects the seams a file implements.
struct Impls(Vec<(usize, String)>);

impl<'ast> Visit<'ast> for Impls {
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if let Some((_, path, _)) = &i.trait_ {
            if let Some(last) = path.segments.last() {
                let name = last.ident.to_string();
                if SEAMS.contains(&name.as_str()) {
                    use syn::spanned::Spanned;
                    self.0.push((i.span().start().line, name));
                }
            }
        }
        visit::visit_item_impl(self, i);
    }
}

/// Panics naming each `.rs` file of `files` outside `crates/kernel` that implements a seam.
fn check_one_implementation(files: &[(String, String)]) {
    let mut found = Vec::new();
    let mut read = 0;
    for (rel, text) in files {
        if rel.starts_with("crates/kernel/") || test_code(rel) {
            continue;
        }
        let file =
            syn::parse_file(text).unwrap_or_else(|e| panic!("{rel}: syn could not parse it: {e}"));
        read += 1;
        let mut impls = Impls(Vec::new());
        impls.visit_file(&file);
        found.extend(
            impls
                .0
                .into_iter()
                .map(|(l, n)| format!("{rel}:{l}: impl {n}")),
        );
    }
    assert!(
        read > 100,
        "the scan read only {read} files outside the kernel"
    );
    assert!(
        found.is_empty(),
        "a second implementation of the kernel's physics: {}",
        found.join(", ")
    );
}

#[test]
fn single_source_one_implementation() {
    check_one_implementation(&files(&["rs"]));
}

negative_control!(
    single_source_one_implementation,
    "an occupant implemented in the engine",
    expected = "a second implementation of the kernel's physics",
    {
        let mut all = files(&["rs"]);
        all.push((
            "crates/engine/src/second_rk4.rs".to_owned(),
            "struct MyRk4;\nimpl kernel::driver::advance::Advance for MyRk4 {}\n".to_owned(),
        ));
        check_one_implementation(&all)
    }
);

/// Panics unless `manifests` (each `Cargo.toml` by its path from the root) name `crates/kernel` as the one crate that
/// takes `spirv-std`, and `build_kernel`, xtask's build step, compiles `crates/kernel` and no other crate.
fn check_one_gpu_crate(manifests: &[(String, String)], build_kernel: &str) {
    let gpu: Vec<&str> = manifests
        .iter()
        .filter(|(_, text)| {
            text.lines()
                .any(|l| l.trim_start().starts_with("spirv-std"))
        })
        .map(|(rel, _)| rel.as_str())
        .collect();
    assert_eq!(
        gpu,
        ["crates/kernel/Cargo.toml"],
        "the crates rust-gpu can build are not crates/kernel alone"
    );
    assert_eq!(
        build_kernel.matches("root.join(\"crates/kernel\")").count(),
        1,
        "build-kernel does not compile crates/kernel"
    );
    assert_eq!(
        build_kernel.matches("to_spirv_builder(").count(),
        1,
        "build-kernel builds more than one crate to SPIR-V"
    );
    let engine = manifests
        .iter()
        .find(|(rel, _)| rel == "crates/engine/Cargo.toml")
        .expect("the engine's manifest");
    assert!(
        engine.1.contains("kernel = { path = \"../kernel\""),
        "the engine's CPU reference does not take crates/kernel"
    );
}

#[test]
fn single_source_one_gpu_crate() {
    let manifests: Vec<(String, String)> = files(&["toml"])
        .into_iter()
        .filter(|(rel, _)| rel.ends_with("Cargo.toml"))
        .collect();
    let build_kernel =
        fs::read_to_string(root().join("xtask/src/build_kernel.rs")).expect("build_kernel.rs");
    check_one_gpu_crate(&manifests, &build_kernel);
}

negative_control!(
    single_source_one_gpu_crate,
    "a second crate taking spirv-std",
    expected = "the crates rust-gpu can build are not crates/kernel alone",
    {
        let mut manifests: Vec<(String, String)> = files(&["toml"])
            .into_iter()
            .filter(|(rel, _)| rel.ends_with("Cargo.toml"))
            .collect();
        manifests.push((
            "crates/shadow_kernel/Cargo.toml".to_owned(),
            "[dependencies]\nspirv-std = \"=0.10.0-alpha.1\"\n".to_owned(),
        ));
        let build_kernel =
            fs::read_to_string(root().join("xtask/src/build_kernel.rs")).expect("build_kernel.rs");
        check_one_gpu_crate(&manifests, &build_kernel)
    }
);
