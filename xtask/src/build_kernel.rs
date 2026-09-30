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

use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_gpu_install::install::Install;

use crate::deps::cargo;

/// The SPIR-V module, relative to the workspace root.
pub const SPV: &str = "target/spirv/kernel.spv";

/// The WGSL naga translates it to, relative to the workspace root.
pub const WGSL: &str = "target/spirv/kernel.wgsl";

/// rust-gpu's target: Vulkan 1.1 SPIR-V, the version naga's SPIR-V front end reads.
pub const TARGET: &str = "spirv-unknown-vulkan1.1";

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
    let backend = Install::from_shader_crate(kernel.clone())
        .run()
        .map_err(|e| format!("build-kernel: installing rust-gpu: {e:#}"))?;
    check_channel(&pinned, &backend.toolchain_channel)?;
    println!(
        "build-kernel: rust-gpu backend {} on {}",
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
