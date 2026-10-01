//! `cargo xtask build-kernel` — compiles the one kernel source to the GPU (canonical_spec §1 item 2; lowering Part 2):
//! rust-gpu compiles `crates/kernel` to f32 SPIR-V, written to [`SPV`], and naga translates that to WGSL, written to
//! [`WGSL`], which `validation`'s `toolchain_trivial_kernel` dispatches through the harness. The native build is
//! ordinary `rustc`.
//!
//! In `cargo xtask ci` it runs as a process of its own, through the cargo that runs xtask ([`run_in_ci`]), as the gate
//! runner does.
//!
//! The `rustc_codegen_spirv` backend is built by cargo-gpu's installer on the nightly its release pins, into cargo-gpu's
//! cache, once; the build is refused unless that nightly is the one `rust-toolchain.toml` pins, so the workspace, the
//! kernel's native build and its SPIR-V build share one compiler (R-169).
//!
//! The backend's version is pinned exactly, [`BACKEND_VERSION`]: cargo-gpu's installer resolves `rustc_codegen_spirv`
//! at run time, in a crate of its own in its cache, under a caret requirement on the kernel's `spirv-std` version, which
//! a later rust-gpu release (0.10.0, on another nightly) also meets, and it deletes that crate's lockfile before it
//! builds. So build-kernel builds the backend itself first ([`build_backend`]), in that crate, with an exact requirement
//! ([`BACKEND_TOML`]) and [`BACKEND_LOCK`], the lockfile of a backend build on the pinned nightly, under `--locked`; the
//! installer then finds it built and uses it. The backend and its dependencies are the same on every machine, whatever
//! rust-gpu has released since.

use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_source::{CrateMetadata, SpirvSource};

use crate::deps::cargo;

/// The SPIR-V module, relative to the workspace root.
pub const SPV: &str = "target/spirv/kernel.spv";

/// The WGSL naga translates it to, relative to the workspace root.
pub const WGSL: &str = "target/spirv/kernel.wgsl";

/// rust-gpu's target: Vulkan 1.1 SPIR-V, the version naga's SPIR-V front end reads.
pub const TARGET: &str = "spirv-unknown-vulkan1.1";

/// The rust-gpu release the backend is built from: the one whose backend builds on the pinned nightly (R-169). The
/// kernel's `spirv-std` and xtask's `cargo-gpu-install` require exactly this version.
pub const BACKEND_VERSION: &str = "0.10.0-alpha.1";

/// The lockfile of cargo-gpu's backend crate (`rustc_codegen_spirv_dummy`), from a build of [`BACKEND_VERSION`]'s
/// backend on the pinned nightly: `rustc_codegen_spirv` and every dependency at the version that build used.
pub const BACKEND_LOCK: &str = include_str!("../rust-gpu-backend.lock");

/// The version of `package` a lockfile's text pins, if it holds that package once.
pub fn locked_version(lock: &str, package: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = lock.parse().ok()?;
    let mut found = doc
        .get("package")?
        .as_array_of_tables()?
        .iter()
        .filter(|p| p.get("name").and_then(|n| n.as_str()) == Some(package))
        .filter_map(|p| p.get("version").and_then(|v| v.as_str()).map(str::to_owned));
    let version = found.next()?;
    found.next().is_none().then_some(version)
}

/// The version an exact requirement `=<version>` names; `Err` for any other requirement.
pub fn exact_requirement(requirement: &str) -> Result<&str, String> {
    requirement
        .trim()
        .strip_prefix('=')
        .map(str::trim)
        .filter(|v| !v.is_empty() && !v.contains([',', '*', '<', '>', '^', '~', ' ']))
        .ok_or_else(|| {
            format!("{requirement:?} is not an exact requirement (`={BACKEND_VERSION}`)")
        })
}

/// cargo-gpu's backend crate, `rustc_codegen_spirv_dummy`, as its installer writes it, but with an exact requirement:
/// the installer writes `version = "<spirv-std's version>"`, which rust-gpu 0.10.0 also meets.
pub const BACKEND_TOML: &str = "[package]
name = \"rustc_codegen_spirv_dummy\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies.spirv-builder]
package = \"rustc_codegen_spirv\"
version = \"=0.10.0-alpha.1\"
";

/// The backend library's file name on this host.
fn backend_library() -> String {
    format!(
        "{}rustc_codegen_spirv{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

/// The backend in cargo-gpu's crate at `install_dir` was built from [`BACKEND_VERSION`]: its library is there, and the
/// lockfile it was built under pins `rustc_codegen_spirv` at that version. Such a backend is used as it is; any other is
/// rebuilt by [`build_backend`].
pub fn backend_built(install_dir: &Path) -> bool {
    install_dir.join(backend_library()).is_file()
        && std::fs::read_to_string(install_dir.join("Cargo.lock")).is_ok_and(|lock| {
            locked_version(&lock, "rustc_codegen_spirv").as_deref() == Some(BACKEND_VERSION)
        })
}

/// Writes cargo-gpu's backend crate at `install_dir` with [`BACKEND_TOML`] and [`BACKEND_LOCK`], and removes any backend
/// library built there before.
pub fn prepare_backend(install_dir: &Path) -> Result<(), String> {
    let at = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let src = install_dir.join("src");
    std::fs::create_dir_all(&src).map_err(|e| at(&src, e))?;
    for (path, text) in [
        (src.join("lib.rs"), ""),
        (install_dir.join("Cargo.toml"), BACKEND_TOML),
        (install_dir.join("Cargo.lock"), BACKEND_LOCK),
    ] {
        std::fs::write(&path, text).map_err(|e| at(&path, e))?;
    }
    let library = install_dir.join(backend_library());
    match std::fs::remove_file(&library) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(at(&library, e)),
        _ => Ok(()),
    }
}

/// Builds the backend in cargo-gpu's crate at `install_dir` on `channel`, as cargo-gpu's installer does, but from
/// [`BACKEND_LOCK`] under `--locked`: the installer deletes the crate's lockfile and resolves afresh, so a later rust-gpu
/// release would change what it builds. The library goes where the installer looks for it, so the installer then finds
/// it built and uses it.
pub fn build_backend(install_dir: &Path, channel: &str) -> Result<(), String> {
    prepare_backend(install_dir)?;
    println!("build-kernel: building rust-gpu's backend, rustc_codegen_spirv {BACKEND_VERSION}, on {channel}");
    let mut cargo = cargo_gpu_install::spirv_builder::cargo_cmd::CargoCmd::new();
    cargo.env_remove("CLIPPY_ARGS");
    let status = cargo
        .current_dir(install_dir)
        .arg(format!("+{channel}"))
        .args(["build", "--release", "--locked"])
        .status()
        .map_err(|e| format!("build-kernel: cannot run cargo to build rust-gpu's backend: {e}"))?;
    if !status.success() {
        return Err(format!(
            "build-kernel: building rust-gpu's backend failed ({status})"
        ));
    }
    let target = install_dir.join("target");
    let built = target.join("release").join(backend_library());
    std::fs::rename(&built, install_dir.join(backend_library()))
        .map_err(|e| format!("{}: {e}", built.display()))?;
    std::fs::remove_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))
}

/// The channel `rust-toolchain.toml` pins, from its text.
pub fn pinned_channel(toolchain_toml: &str) -> Result<String, String> {
    let doc: toml_edit::DocumentMut = toolchain_toml
        .parse()
        .map_err(|e| format!("rust-toolchain.toml: {e}"))?;
    doc.get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(|c| c.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "rust-toolchain.toml: no [toolchain] channel".to_owned())
}

/// Refuses a backend built on another nightly than the pinned one, naming both.
pub fn check_channel(pinned: &str, backend: &str) -> Result<(), String> {
    if pinned == backend {
        Ok(())
    } else {
        Err(format!(
            "rust-gpu's backend needs toolchain {backend}, but rust-toolchain.toml pins {pinned}: pin {backend}"
        ))
    }
}

/// `spv`, a SPIR-V module, translated to WGSL by naga and validated on the way.
pub fn to_wgsl(spv: &[u8]) -> Result<String, String> {
    let options = naga::front::spv::Options::default();
    let module = naga::front::spv::parse_u8_slice(spv, &options)
        .map_err(|e| format!("naga could not read the SPIR-V: {e}"))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|e| format!("naga rejected the module: {e:?}"))?;
    naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
        .map_err(|e| format!("naga could not write WGSL: {e}"))
}

/// Builds `crates/kernel` of the workspace whose `Cargo.toml` is `manifest` to [`SPV`] and [`WGSL`] under its root.
pub fn run(manifest: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let toolchain = root.join("rust-toolchain.toml");
    let pinned = pinned_channel(
        &std::fs::read_to_string(&toolchain)
            .map_err(|e| format!("{}: {e}", toolchain.display()))?,
    )?;
    let kernel = root.join("crates/kernel");
    let metadata = CrateMetadata::query(kernel.clone())
        .map_err(|e| format!("build-kernel: reading the kernel's metadata: {e:#}"))?;
    let source = SpirvSource::new(&metadata, None, None)
        .map_err(|e| format!("build-kernel: the kernel's rust-gpu: {e:#}"))?;
    match &source {
        SpirvSource::CratesIO(v) if v.to_string() == BACKEND_VERSION => {}
        other => {
            return Err(format!(
                "build-kernel: the kernel's spirv-std is {other}, not crates.io {BACKEND_VERSION}, the rust-gpu \
                 release whose backend builds on the pinned nightly"
            ))
        }
    }
    let install_dir = source
        .install_dir()
        .map_err(|e| format!("build-kernel: rust-gpu's cache: {e:#}"))?;
    if !backend_built(&install_dir) {
        build_backend(&install_dir, &pinned)?;
    }
    let backend = Install::from_shader_crate(kernel.clone())
        .run()
        .map_err(|e| format!("build-kernel: installing rust-gpu: {e:#}"))?;
    let lock = std::fs::read_to_string(install_dir.join("Cargo.lock")).unwrap_or_default();
    let built = locked_version(&lock, "rustc_codegen_spirv");
    if built.as_deref() != Some(BACKEND_VERSION) {
        return Err(format!(
            "build-kernel: rust-gpu's backend resolved to rustc_codegen_spirv {built:?}, not {BACKEND_VERSION}"
        ));
    }
    check_channel(&pinned, &backend.toolchain_channel)?;
    println!(
        "build-kernel: rust-gpu backend rustc_codegen_spirv {BACKEND_VERSION} {} on {}",
        backend.rustc_codegen_spirv_location.display(),
        backend.toolchain_channel
    );
    let built = backend
        .to_spirv_builder(&kernel, TARGET)
        .build()
        .map_err(|e| format!("build-kernel: rust-gpu: {e}"))?;
    let module: PathBuf = built.module.unwrap_single().to_path_buf();
    let spv = std::fs::read(&module).map_err(|e| format!("{}: {e}", module.display()))?;
    let wgsl = to_wgsl(&spv)?;
    for (path, bytes) in [(SPV, spv.as_slice()), (WGSL, wgsl.as_bytes())] {
        let out = root.join(path);
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("build-kernel: wrote {path} ({} B)", bytes.len());
    }
    Ok(())
}

/// `cargo xtask build-kernel` on the workspace of `manifest`, as `cargo xtask ci`'s runner: `cargo run -p xtask --
/// build-kernel`, through the cargo that runs xtask; `Err` when it fails.
pub fn run_in_ci(manifest: &Path) -> Result<(), String> {
    let status = Command::new(cargo())
        .args(["run", "--quiet", "--manifest-path"])
        .arg(manifest)
        .args(["-p", "xtask", "--", "build-kernel"])
        .status()
        .map_err(|e| format!("cannot run cargo run -p xtask -- build-kernel: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("build-kernel failed ({status})"))
    }
}
