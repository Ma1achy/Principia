//! `cargo xtask build-kernel` — compiles the one kernel source to the GPU (canonical_spec §1 item 2; lowering Part 2):
//! rust-gpu compiles `crates/kernel` to f32 SPIR-V, written to [`SPV`], and naga translates that to WGSL, written to
//! [`WGSL`], which `validation`'s `toolchain_trivial_kernel` dispatches through the harness. The native build is
//! ordinary `rustc`.
//!
//! One rust-gpu build gives three WGSL files, parted through naga as `crate::fragment_wgsl` lifts its entries: [`WGSL`]
//! is the production module, [`READ_WGSL`] holds the `stage_branch_read_*` entries alone, and [`PARITY_WGSL`] the parity
//! harness's `stage_parity_*` entries alone, neither of which any runtime path runs. A pipeline's cost follows its
//! module's size, so they stay out of what a shipped build loads.
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
//! builds. So build-kernel builds the backend itself first ([`ensure_backend`], [`build_backend`]), in that crate, with
//! an exact requirement ([`BACKEND_TOML`]) and [`BACKEND_LOCK`], the lockfile of a backend build on the pinned nightly,
//! under `--locked`; the installer then finds it built and uses it. The backend and its dependencies are the same on
//! every machine, whatever rust-gpu has released since.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::cargo_cmd::CargoCmd;
use cargo_gpu_install::spirv_source::{CrateMetadata, SpirvSource};

use crate::deps::cargo;

/// The environment variable that, set and not empty, names the kernel cache `cargo xtask build-kernel` uses
/// ([`run_cached`]): CI sets it, so its build step's command stays `cargo xtask build-kernel` (TASK-M0-77).
pub const CACHE_ENV: &str = "PRIN_KERNEL_CACHE";

/// The SPIR-V module, relative to the workspace root.
pub const SPV: &str = "target/spirv/kernel.spv";

/// The WGSL naga translates it to, relative to the workspace root.
pub const WGSL: &str = "target/spirv/kernel.wgsl";

/// The WGSL of the read-back's entries, relative to the workspace root.
pub const READ_WGSL: &str = "target/spirv/branch_read.wgsl";

/// The entries [`READ_WGSL`] holds and [`WGSL`] doesn't: `kernel::stage::branch_read`'s, as naga names them.
pub const READ_ENTRIES: &str = "stage_branch_read_";

/// The WGSL of the parity harness's entries, relative to the workspace root.
pub const PARITY_WGSL: &str = "target/spirv/parity.wgsl";

/// The entries [`PARITY_WGSL`] holds and [`WGSL`] doesn't: `kernel::stage::parity`'s, as naga names them.
pub const PARITY_ENTRIES: &str = "stage_parity_";

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

/// The backend in cargo-gpu's crate at `install_dir` was built from [`BACKEND_LOCK`]: its library is there, and the
/// lockfile it was built under is [`BACKEND_LOCK`], byte for byte, so `rustc_codegen_spirv` is [`BACKEND_VERSION`] and
/// every dependency is at the version that lockfile pins. Such a backend is used as it is; any other, one whose lockfile
/// differs in any package, is rebuilt by [`build_backend`].
pub fn backend_built(install_dir: &Path) -> bool {
    install_dir.join(backend_library()).is_file() && locked_as_pinned(install_dir)
}

/// The lockfile of cargo-gpu's crate at `install_dir` is [`BACKEND_LOCK`], byte for byte.
fn locked_as_pinned(install_dir: &Path) -> bool {
    std::fs::read_to_string(install_dir.join("Cargo.lock")).is_ok_and(|lock| lock == BACKEND_LOCK)
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
/// it built and uses it. `cargo` is the command it runs cargo as: [`run`] gives it spirv-builder's `CargoCmd`, the cargo
/// the installer runs, with the environment that would leak into a nested build removed. `spawn` runs that command and
/// returns its exit status: [`run`] gives it `Command::status`, its output streamed; a test gives it the shared spawn
/// helper, bounded by its timeout (R-214) and spawning under its lock against stand-in writes (REQ-SYS-070).
pub fn build_backend(
    install_dir: &Path,
    channel: &str,
    mut cargo: Command,
    spawn: impl FnOnce(&mut Command) -> std::io::Result<ExitStatus>,
) -> Result<(), String> {
    prepare_backend(install_dir)?;
    println!("build-kernel: building rust-gpu's backend, rustc_codegen_spirv {BACKEND_VERSION}, on {channel}");
    cargo.env_remove("CLIPPY_ARGS");
    cargo
        .current_dir(install_dir)
        .arg(format!("+{channel}"))
        .args(["build", "--release", "--locked"]);
    let status = spawn(&mut cargo)
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

/// Builds the backend at `install_dir` with [`build_backend`], running `cargo` through `spawn`, unless it is
/// [`backend_built`] already.
pub fn ensure_backend(
    install_dir: &Path,
    channel: &str,
    cargo: Command,
    spawn: impl FnOnce(&mut Command) -> std::io::Result<ExitStatus>,
) -> Result<(), String> {
    if !backend_built(install_dir) {
        build_backend(install_dir, channel, cargo, spawn)?;
    }
    Ok(())
}

/// Refuses a backend crate at `install_dir` whose lockfile is not [`BACKEND_LOCK`] after cargo-gpu's installer has run:
/// the installer resolved it afresh.
pub fn check_locked(install_dir: &Path) -> Result<(), String> {
    if !locked_as_pinned(install_dir) {
        let lock = std::fs::read_to_string(install_dir.join("Cargo.lock")).unwrap_or_default();
        return Err(format!(
            "build-kernel: rust-gpu's backend was resolved other than rust-gpu-backend.lock pins it \
             (rustc_codegen_spirv {:?}; the lockfile differs from BACKEND_LOCK)",
            locked_version(&lock, "rustc_codegen_spirv")
        ));
    }
    Ok(())
}

/// Refuses a kernel whose rust-gpu, `source`, is other than crates.io's [`BACKEND_VERSION`].
pub fn check_source(source: &SpirvSource) -> Result<(), String> {
    match source {
        SpirvSource::CratesIO(v) if v.to_string() == BACKEND_VERSION => Ok(()),
        other => Err(format!(
            "build-kernel: the kernel's spirv-std is {other}, not crates.io {BACKEND_VERSION}, the rust-gpu \
             release whose backend builds on the pinned nightly"
        )),
    }
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

/// The production module of `spv`, a SPIR-V module: every entry but the read-back's and the parity harness's,
/// translated to WGSL by naga and validated on the way.
pub fn to_wgsl(spv: &[u8]) -> Result<String, String> {
    part(spv, |e| {
        !e.starts_with(READ_ENTRIES) && !e.starts_with(PARITY_ENTRIES)
    })
}

/// The read-back's module of `spv`: its [`READ_ENTRIES`] alone, translated as [`to_wgsl`] translates the rest.
pub fn to_read_wgsl(spv: &[u8]) -> Result<String, String> {
    part(spv, |e| e.starts_with(READ_ENTRIES))
}

/// The parity harness's module of `spv`: its [`PARITY_ENTRIES`] alone, translated as [`to_wgsl`] translates the rest.
pub fn to_parity_wgsl(spv: &[u8]) -> Result<String, String> {
    part(spv, |e| e.starts_with(PARITY_ENTRIES))
}

/// The WGSL of `spv`'s entries whose naga name `keep` takes, with what those entries don't reach compacted away.
fn part(spv: &[u8], keep: impl Fn(&str) -> bool) -> Result<String, String> {
    let options = naga::front::spv::Options::default();
    let mut module = naga::front::spv::parse_u8_slice(spv, &options)
        .map_err(|e| format!("naga could not read the SPIR-V: {e}"))?;
    module
        .entry_points
        .retain(|e| keep(&e.name.replace("::", "_")));
    naga::compact::compact(&mut module, naga::compact::KeepUnused::No);
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|e| format!("naga rejected the module: {e:?}"))?;
    naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
        .map_err(|e| format!("naga could not write WGSL: {e}"))
}

/// Builds `crates/kernel` of the workspace whose `Cargo.toml` is `manifest` to [`SPV`], [`WGSL`], [`READ_WGSL`] and
/// [`PARITY_WGSL`] under its root, and checks the fragment snippets against their translation ([`run_with`] without `bless`).
pub fn run(manifest: &Path) -> Result<(), String> {
    run_with(manifest, false)
}

/// [`run`], then the fragment snippets lifted from the same SPIR-V (`crate::fragment_wgsl`; R-116) checked against the
/// checked-in files: `Err` where one differs, so a hand edit or a stale snippet fails the build; with `bless`, each that
/// differs is written instead.
pub fn run_with(manifest: &Path, bless: bool) -> Result<(), String> {
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
    check_source(&source)?;
    let install_dir = source
        .install_dir()
        .map_err(|e| format!("build-kernel: rust-gpu's cache: {e:#}"))?;
    ensure_backend(
        &install_dir,
        &pinned,
        CargoCmd::new().into(),
        Command::status,
    )?;
    let backend = Install::from_shader_crate(kernel.clone())
        .run()
        .map_err(|e| format!("build-kernel: installing rust-gpu: {e:#}"))?;
    check_locked(&install_dir)?;
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
    let read = to_read_wgsl(&spv)?;
    let parity = to_parity_wgsl(&spv)?;
    for (path, bytes) in [
        (SPV, spv.as_slice()),
        (WGSL, wgsl.as_bytes()),
        (READ_WGSL, read.as_bytes()),
        (PARITY_WGSL, parity.as_bytes()),
    ] {
        let out = root.join(path);
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
        println!("build-kernel: wrote {path} ({} B)", bytes.len());
    }
    check_fragments(root, &spv, bless)
}

/// The fragment snippets lifted from `spv` checked against the checked-in files under `root` (`crate::fragment_wgsl`;
/// R-116), each that differs written instead with `bless`.
fn check_fragments(root: &Path, spv: &[u8], bless: bool) -> Result<(), String> {
    let fresh = crate::fragment_wgsl::snippets(spv)?;
    for path in crate::fragment_wgsl::check(root, &fresh, bless)? {
        println!("build-kernel: re-emitted {path}");
    }
    Ok(())
}

/// cargo's fingerprint directory of the kernel's SPIR-V builds in the workspace at `root`:
/// `<target directory>/spirv-builder/<`[`TARGET`]`>/release/.fingerprint`, the target directory as `cargo metadata` gives
/// it for `crates/kernel`, as spirv-builder finds its own (it builds into `spirv-builder` under it). The flags test,
/// `xtask/tests/build_kernel.rs`'s `build_kernel_spirv_flags_are_spirv_builders_own`, reads the same directory.
pub fn spirv_fingerprints(root: &Path) -> Result<PathBuf, String> {
    let manifest = root.join("crates/kernel/Cargo.toml");
    let out = Command::new(cargo())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata --manifest-path {} failed ({}): {}",
            manifest.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata's output: {e}"))?;
    let target = json["target_directory"]
        .as_str()
        .ok_or_else(|| "cargo metadata gave no target_directory".to_owned())?;
    Ok(Path::new(target)
        .join("spirv-builder")
        .join(TARGET)
        .join("release/.fingerprint"))
}

/// `cargo xtask build-kernel --cache-dir <cache>`, or `cargo xtask build-kernel` with [`CACHE_ENV`] naming `<cache>`
/// (TASK-M0-77): [`run_cached_with`], the kernel's fingerprints in [`spirv_fingerprints`]' directory.
pub fn run_cached(manifest: &Path, cache: &Path) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    run_cached_with(manifest, cache, spirv_fingerprints(root))
}

/// The outputs of the workspace of `manifest` taken from `cache`'s entry for its kernel source hash
/// (`crate::kernel_cache`) when that entry is whole, with the kernel's fingerprints of the build that stored it put back
/// in `fingerprints`, cargo's fingerprint directory of the kernel's SPIR-V builds, so the flags test checks that build's
/// rustflags on a hit as it checks a fresh build's; the fragment snippets then checked against the cached SPIR-V as
/// [`run`] checks them; `Err` after that check if the entry holds no fingerprint, or `fingerprints` could not be found.
/// Otherwise [`run`], its outputs then stored as that entry with the build's fingerprints
/// (`crate::kernel_cache::store_built`). Its log says which, with the hash.
pub fn run_cached_with(
    manifest: &Path,
    cache: &Path,
    fingerprints: Result<PathBuf, String>,
) -> Result<(), String> {
    let root = manifest
        .parent()
        .ok_or_else(|| format!("{}: no parent directory", manifest.display()))?;
    let hash = crate::kernel_cache::source_hash(root)?;
    if crate::kernel_cache::restore(cache, &hash, root)? {
        println!(
            "build-kernel: kernel cache hit {hash}, from {}",
            cache.display()
        );
        let restored = match &fingerprints {
            Ok(dir) => crate::kernel_cache::restore_fingerprints(cache, &hash, dir)?,
            Err(_) => 0,
        };
        let spv_path = root.join(SPV);
        let spv = std::fs::read(&spv_path).map_err(|e| format!("{}: {e}", spv_path.display()))?;
        check_fragments(root, &spv, false)?;
        let dir = fingerprints?;
        if restored == 0 {
            return Err(format!(
                "build-kernel: the kernel cache's entry {} holds no fingerprint of its SPIR-V build, so its rustflags \
                 can't be checked; remove it to rebuild",
                cache.join(&hash).display()
            ));
        }
        println!(
            "build-kernel: restored {restored} fingerprint(s) of the cached build into {}",
            dir.display()
        );
        return Ok(());
    }
    println!(
        "build-kernel: kernel cache miss {hash}, building into {}",
        cache.display()
    );
    run(manifest)?;
    crate::kernel_cache::store_built(cache, &hash, root, &fingerprints?)?;
    println!("build-kernel: stored the kernel as {hash}, with its build's fingerprints");
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
