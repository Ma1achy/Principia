//! The compiled kernel's cache and the compile cache's trim (TASK-M0-77, the owner's CI compile-cache order of 17 Nov
//! 2026).
//!
//! [`source_hash`] is the one definition of the kernel's source hash: `cargo xtask kernel-src-hash` prints it, the
//! hosted GPU jobs key their kernel cache on it, the Windows runner names its kernel folder by it
//! (`E:\ci-cache\kernel\<hash>`), and the Mac's queue can call the same command. It is the SHA-256 of
//! [`VERSION`] and, for every file of [`INPUTS`] in sorted order, its path relative to the workspace's root (with `/`)
//! and the SHA-256 of its bytes; an input that isn't there is hashed as absent, so adding it changes the hash. No time,
//! size or path outside the workspace enters it, so a checkout of the same commit gives the same hash on every machine.
//!
//! [`INPUTS`] is everything the build reads: the kernel crate, its one path dependency (`ledger`, a build-dependency,
//! with the files it includes), the workspace's manifest, lockfile, toolchain pin and cargo config, rust-gpu's backend
//! lockfile, and the xtask code that builds and translates the kernel and hashes it. A file outside them can't change
//! [`crate::build_kernel::SPV`], [`crate::build_kernel::WGSL`], [`crate::build_kernel::READ_WGSL`] or
//! [`crate::build_kernel::PARITY_WGSL`]; the fragment snippets `build-kernel` checks are not outputs, and are checked against the cached SPIR-V on every hit
//! ([`crate::build_kernel::run_cached`]).
//!
//! An entry, `<cache>/<hash>/`, holds the four outputs and [`MARK`], which lists each with its SHA-256. It is written
//! whole under a temporary name, [`PARTIAL`], and renamed into place, so a build cancelled part-way leaves no entry that
//! looks whole; an entry whose files don't match its mark is removed and rebuilt.
//!
//! An entry `build-kernel` stores ([`store_built`]) holds too, under [`FINGERPRINTS`], each `kernel-<id>/`[`FINGERPRINT`]
//! of cargo's fingerprints in spirv-builder's target directory: the record of the rustflags the build that made those
//! outputs compiled the kernel with, which `xtask/tests/build_kernel.rs`'s
//! `build_kernel_spirv_flags_are_spirv_builders_own` reads. The mark lists them, so they come from the same build as the
//! SPIR-V beside them (on a persistent target directory every kernel fingerprint there is taken, earlier builds' too, as
//! the test reads them all); on a hit [`restore_fingerprints`] puts them back where that build wrote them, and the test checks
//! them as it checks a fresh build's (Codex's finding on PR #568).
//!
//! [`trim`] keeps the Windows runner's `E:\ci-cache` to its caps (the owner's item 8): the per-job target folders,
//! oldest first by their last write, until their total is under the cap, and the kernel entries, the newest few kept.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::build_kernel::{PARITY_WGSL, READ_WGSL, SPV, WGSL};

/// The hash's first line: a change to how it is computed changes every hash.
pub const VERSION: &str = "principia kernel sources 1";

/// What the kernel's build reads, relative to the workspace's root: a directory means every file under it.
pub const INPUTS: [&str; 15] = [
    ".cargo/config.toml",
    "Cargo.lock",
    "Cargo.toml",
    "crates/kernel/Cargo.toml",
    "crates/kernel/build.rs",
    "crates/kernel/src",
    "crates/ledger/Cargo.toml",
    "crates/ledger/build.rs",
    "crates/ledger/data",
    "crates/ledger/src",
    "crates/ledger/wgsl",
    "rust-toolchain.toml",
    "xtask/rust-gpu-backend.lock",
    "xtask/src/build_kernel.rs",
    "xtask/src/kernel_cache.rs",
];

/// The file in an entry that lists its outputs, each with its SHA-256; written last.
pub const MARK: &str = "complete";

/// The prefix of an entry still being written.
pub const PARTIAL: &str = ".partial-";

/// The outputs an entry holds, as paths relative to the workspace's root.
pub const OUTPUTS: [&str; 4] = [SPV, WGSL, READ_WGSL, PARITY_WGSL];

/// The folder of an entry that holds the kernel's SPIR-V build fingerprints, one `<folder>/`[`FINGERPRINT`] each.
pub const FINGERPRINTS: &str = "fingerprints";

/// The file of a fingerprint folder of cargo's that records the kernel library's build, its rustflags among it.
pub const FINGERPRINT: &str = "lib-kernel.json";

/// The prefix of the kernel's fingerprint folders: cargo names each `kernel-<16 hex digits>`.
const KERNEL_FOLDER: &str = "kernel-";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn at(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Every file under `dir`, relative to `root`, with `/` between its parts.
fn files_under(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| at(dir, e))? {
        let entry = entry.map_err(|e| at(dir, e))?;
        let path = entry.path();
        if path.is_dir() {
            files_under(root, &path, out)?;
        } else {
            out.push(relative(root, &path)?);
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?;
    Ok(rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

/// The text the hash is taken of: [`VERSION`], then one line per input file, sorted, `<path> <SHA-256>`, and one per
/// input that isn't there, `<path> absent`.
pub fn manifest(root: &Path) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();
    for input in INPUTS {
        let path = root.join(input);
        if path.is_dir() {
            let mut files = Vec::new();
            files_under(root, &path, &mut files)?;
            for file in files {
                let bytes = fs::read(root.join(&file)).map_err(|e| at(&root.join(&file), e))?;
                lines.push(format!("{file} {}", sha256_hex(&bytes)));
            }
        } else if path.is_file() {
            let bytes = fs::read(&path).map_err(|e| at(&path, e))?;
            lines.push(format!("{input} {}", sha256_hex(&bytes)));
        } else {
            lines.push(format!("{input} absent"));
        }
    }
    lines.sort();
    Ok(format!("{VERSION}\n{}\n", lines.join("\n")))
}

/// The kernel's source hash of the workspace at `root` (64 hex digits).
pub fn source_hash(root: &Path) -> Result<String, String> {
    Ok(sha256_hex(manifest(root)?.as_bytes()))
}

/// The file name an output has in an entry.
fn entry_name(output: &str) -> &str {
    output.rsplit('/').next().unwrap_or(output)
}

/// The outputs (output, bytes) and the fingerprints (folder, bytes) of an entry.
struct Entry {
    outputs: Vec<(&'static str, Vec<u8>)>,
    fingerprints: Vec<(String, Vec<u8>)>,
}

/// Whether `name` is a kernel fingerprint folder's: `kernel-` then one or more ASCII letters and digits, so it
/// names one folder and nothing outside it.
pub fn is_kernel_folder(name: &str) -> bool {
    name.strip_prefix(KERNEL_FOLDER)
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// An entry's path of the fingerprint of `folder`, relative to the entry, with `/`.
fn fingerprint_name(folder: &str) -> String {
    format!("{FINGERPRINTS}/{folder}/{FINGERPRINT}")
}

/// The mark of an entry: a line for each output, `<name> <SHA-256>`, then one for each fingerprint, in the order given.
fn mark_text(outputs: &[(&str, Vec<u8>)], fingerprints: &[(String, Vec<u8>)]) -> String {
    let outputs = outputs
        .iter()
        .map(|(output, bytes)| format!("{} {}\n", entry_name(output), sha256_hex(bytes)));
    let fingerprints = fingerprints
        .iter()
        .map(|(folder, bytes)| format!("{} {}\n", fingerprint_name(folder), sha256_hex(bytes)));
    outputs.chain(fingerprints).collect()
}

/// `<cache>/<hash>`, read and checked against its mark, or `None` if the entry isn't there or isn't whole: every output
/// and every fingerprint its mark lists there and matching it, the mark listing nothing else.
fn read_entry(entry: &Path) -> Option<Entry> {
    let mark = fs::read_to_string(entry.join(MARK)).ok()?;
    let mut outputs = Vec::new();
    for output in OUTPUTS {
        outputs.push((output, fs::read(entry.join(entry_name(output))).ok()?));
    }
    let mut fingerprints = Vec::new();
    for line in mark.lines().skip(OUTPUTS.len()) {
        let (name, _) = line.split_once(' ')?;
        let folder = name
            .strip_prefix(FINGERPRINTS)?
            .strip_prefix('/')?
            .strip_suffix(FINGERPRINT)?
            .strip_suffix('/')?;
        if !is_kernel_folder(folder) {
            return None;
        }
        let bytes = fs::read(entry.join(FINGERPRINTS).join(folder).join(FINGERPRINT)).ok()?;
        fingerprints.push((folder.to_owned(), bytes));
    }
    (mark == mark_text(&outputs, &fingerprints)).then_some(Entry {
        outputs,
        fingerprints,
    })
}

/// The kernel's fingerprints in `dir`, a fingerprint directory of cargo's: every [`is_kernel_folder`] folder's
/// [`FINGERPRINT`], sorted by folder; a folder without one (a build script's) is passed over.
pub fn fingerprints_in(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| at(dir, e))? {
        let entry = entry.map_err(|e| at(dir, e))?;
        let folder = entry.file_name().to_string_lossy().into_owned();
        let file = entry.path().join(FINGERPRINT);
        if is_kernel_folder(&folder) && file.is_file() {
            out.push((folder, fs::read(&file).map_err(|e| at(&file, e))?));
        }
    }
    out.sort();
    Ok(out)
}

/// Copies the entry for `hash` from `cache` into the workspace at `root`, at [`OUTPUTS`]' paths: `Ok(true)` on a hit,
/// `Ok(false)` if there is no whole entry. An entry there but not whole is removed, so the store after the build can
/// put a whole one in its place.
pub fn restore(cache: &Path, hash: &str, root: &Path) -> Result<bool, String> {
    let entry = cache.join(hash);
    let Some(Entry { outputs, .. }) = read_entry(&entry) else {
        if entry.exists() {
            fs::remove_dir_all(&entry).map_err(|e| at(&entry, e))?;
        }
        return Ok(false);
    };
    for (output, bytes) in outputs {
        let to = root.join(output);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| at(dir, e))?;
        }
        fs::write(&to, bytes).map_err(|e| at(&to, e))?;
    }
    Ok(true)
}

/// Puts the fingerprints of the whole entry for `hash` in `cache` into `dir`, a fingerprint directory of cargo's, each
/// at `dir/<folder>/`[`FINGERPRINT`], where the build that stored them wrote it; the number put back, none if the entry
/// holds none or isn't whole.
pub fn restore_fingerprints(cache: &Path, hash: &str, dir: &Path) -> Result<usize, String> {
    let Some(Entry { fingerprints, .. }) = read_entry(&cache.join(hash)) else {
        return Ok(0);
    };
    for (folder, bytes) in &fingerprints {
        let to = dir.join(folder);
        fs::create_dir_all(&to).map_err(|e| at(&to, e))?;
        let to = to.join(FINGERPRINT);
        fs::write(&to, bytes).map_err(|e| at(&to, e))?;
    }
    Ok(fingerprints.len())
}

/// Stores the outputs at `root` in `cache` as the entry for `hash`, no fingerprint with them ([`store_built`] stores
/// both).
pub fn store(cache: &Path, hash: &str, root: &Path) -> Result<(), String> {
    store_entry(cache, hash, root, Vec::new())
}

/// Stores the outputs at `root` and the kernel's fingerprints in `fingerprints`, the fingerprint directory of the build
/// that wrote them ([`fingerprints_in`]), in `cache` as the entry for `hash`, as [`store`] does; `Err` if there is no
/// fingerprint of the kernel there, so no entry `build-kernel` stores lacks the record of its build's flags.
pub fn store_built(
    cache: &Path,
    hash: &str,
    root: &Path,
    fingerprints: &Path,
) -> Result<(), String> {
    let found = fingerprints_in(fingerprints)?;
    if found.is_empty() {
        return Err(format!(
            "{}: no fingerprint of the kernel's SPIR-V build to store with it",
            fingerprints.display()
        ));
    }
    store_entry(cache, hash, root, found)
}

/// The entry for `hash`, the outputs at `root` with `fingerprints`, written under a [`PARTIAL`] name, its mark last,
/// then renamed into place. If a whole entry for `hash` is there already, it stays and the new copy is dropped.
fn store_entry(
    cache: &Path,
    hash: &str,
    root: &Path,
    fingerprints: Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    let mut files = Vec::new();
    for output in OUTPUTS {
        let from = root.join(output);
        files.push((output, fs::read(&from).map_err(|e| at(&from, e))?));
    }
    let partial = cache.join(format!("{PARTIAL}{hash}-{}", std::process::id()));
    if partial.exists() {
        fs::remove_dir_all(&partial).map_err(|e| at(&partial, e))?;
    }
    fs::create_dir_all(&partial).map_err(|e| at(&partial, e))?;
    for (output, bytes) in &files {
        let to = partial.join(entry_name(output));
        fs::write(&to, bytes).map_err(|e| at(&to, e))?;
    }
    for (folder, bytes) in &fingerprints {
        let to = partial.join(FINGERPRINTS).join(folder);
        fs::create_dir_all(&to).map_err(|e| at(&to, e))?;
        let to = to.join(FINGERPRINT);
        fs::write(&to, bytes).map_err(|e| at(&to, e))?;
    }
    let mark = partial.join(MARK);
    fs::write(&mark, mark_text(&files, &fingerprints)).map_err(|e| at(&mark, e))?;
    let entry = cache.join(hash);
    if read_entry(&entry).is_some() {
        return fs::remove_dir_all(&partial).map_err(|e| at(&partial, e));
    }
    if entry.exists() {
        fs::remove_dir_all(&entry).map_err(|e| at(&entry, e))?;
    }
    fs::rename(&partial, &entry).map_err(|e| at(&entry, e))
}

/// What [`trim`] removed and kept, one line each, for the job's log.
#[derive(Debug, Default)]
pub struct Trimmed {
    pub removed: Vec<PathBuf>,
    pub lines: Vec<String>,
}

/// A folder's size in bytes and its last write, the newest of its own and everything under it. A link (a symlink or a
/// junction) is counted as itself and never followed.
pub fn size_and_newest(path: &Path) -> Result<(u64, SystemTime), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| at(path, e))?;
    let mut newest = meta.modified().map_err(|e| at(path, e))?;
    let mut size = meta.len();
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| at(path, e))? {
            let entry = entry.map_err(|e| at(path, e))?;
            let (s, n) = size_and_newest(&entry.path())?;
            size += s;
            newest = newest.max(n);
        }
    }
    Ok((size, newest))
}

/// The folders directly in `dir`, each with its size and last write; a link, a file or a missing `dir` gives nothing to
/// remove.
fn folders(dir: &Path) -> Result<Vec<(PathBuf, u64, SystemTime)>, String> {
    let Ok(meta) = fs::symlink_metadata(dir) else {
        return Ok(Vec::new());
    };
    if !meta.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| at(dir, e))? {
        let entry = entry.map_err(|e| at(dir, e))?;
        let path = entry.path();
        if fs::symlink_metadata(&path)
            .map_err(|e| at(&path, e))?
            .is_dir()
        {
            let (size, newest) = size_and_newest(&path)?;
            out.push((path, size, newest));
        }
    }
    Ok(out)
}

fn same(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

const GB: u64 = 1 << 30;

/// Trims the compile cache at `root`: the folders of `root/target`, oldest last write first, removed until their total
/// is at most `target_cap` bytes, `keep` (the running job's own) never among them; and the entries of `root/kernel`, all
/// but the newest `keep_kernels` whole ones removed, every partial one too. Only folders directly in those two are
/// removed, never through a link. sccache trims its own folder to its cap.
pub fn trim(
    root: &Path,
    target_cap: u64,
    keep_kernels: usize,
    keep: Option<&Path>,
) -> Result<Trimmed, String> {
    let mut out = Trimmed::default();
    let mut targets = folders(&root.join("target"))?;
    targets.sort_by_key(|(_, _, newest)| *newest);
    let mut total: u64 = targets.iter().map(|(_, size, _)| size).sum();
    out.lines.push(format!(
        "target folders: {} GiB in {}, cap {} GiB",
        total / GB,
        targets.len(),
        target_cap / GB
    ));
    for (path, size, _) in &targets {
        if total <= target_cap {
            break;
        }
        if keep.is_some_and(|k| same(k, path)) {
            out.lines
                .push(format!("kept {} (this job's)", path.display()));
            continue;
        }
        fs::remove_dir_all(path).map_err(|e| at(path, e))?;
        total -= size;
        out.lines
            .push(format!("removed {} ({} GiB)", path.display(), size / GB));
        out.removed.push(path.clone());
    }
    let mut kernels = folders(&root.join("kernel"))?;
    kernels.sort_by_key(|(_, _, newest)| std::cmp::Reverse(*newest));
    let mut whole = 0;
    for (path, _, _) in &kernels {
        let partial = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(PARTIAL));
        if !partial && read_entry(path).is_some() && whole < keep_kernels {
            whole += 1;
            continue;
        }
        fs::remove_dir_all(path).map_err(|e| at(path, e))?;
        out.lines.push(format!("removed {}", path.display()));
        out.removed.push(path.clone());
    }
    out.lines.push(format!(
        "target folders now {} GiB; kernel entries kept: {whole}",
        total / GB
    ));
    Ok(out)
}

/// `cargo xtask ci-cache-trim`'s arguments: `--root <dir> --target-cap-gb <n> --keep-kernels <n> [--keep <dir>]`.
pub fn trim_cli(args: &[&str]) -> Result<(), String> {
    let usage = "usage: cargo xtask ci-cache-trim --root <dir> --target-cap-gb <n> --keep-kernels <n> [--keep <dir>]";
    let (mut root, mut cap, mut kernels, mut keep) = (None, None, None, None);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| usage.to_owned())?;
        match *flag {
            "--root" => root = Some(PathBuf::from(value)),
            "--target-cap-gb" => cap = Some(value.parse::<u64>().map_err(|_| usage.to_owned())?),
            "--keep-kernels" => {
                kernels = Some(value.parse::<usize>().map_err(|_| usage.to_owned())?)
            }
            "--keep" => keep = Some(PathBuf::from(value)),
            _ => return Err(usage.to_owned()),
        }
    }
    let (Some(root), Some(cap), Some(kernels)) = (root, cap, kernels) else {
        return Err(usage.to_owned());
    };
    let trimmed = trim(&root, cap.saturating_mul(GB), kernels, keep.as_deref())?;
    for line in trimmed.lines {
        println!("ci-cache-trim: {line}");
    }
    Ok(())
}
