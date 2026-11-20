//! The compiled kernel's cache and the compile cache's trim (TASK-M0-77; `xtask/src/kernel_cache.rs`):
//!
//! - the kernel's source hash is one definition, the same for the same files wherever the workspace sits, and changes
//!   when any file the kernel's build reads changes, an absent input added among them, and not when a file it doesn't
//!   read does;
//! - an entry stored is restored byte for byte; an entry not whole is no hit, and is removed;
//! - an entry `build-kernel` stores holds the kernel's SPIR-V build fingerprints, which a hit puts back where the build
//!   wrote them, and it is never stored without one; a fingerprint changed in the entry makes it not whole;
//! - the trim removes the oldest target folders until their total is under the cap, never the running job's, keeps the
//!   newest whole kernel entries and removes every partial one, and never follows a link out of the cache.
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use validation::negative_control;
use xtask::kernel_cache::{self, FINGERPRINT, FINGERPRINTS, INPUTS, OUTPUTS, PARTIAL};

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "kernel_cache_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("an old scratch folder removed");
    }
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("a folder");
    fs::write(path, text).expect("a file");
}

/// A workspace holding one file for every input (a directory input gets a file in a subfolder), but `crates/ledger/build.rs`,
/// which is absent as in this repository, and a file the build doesn't read.
fn workspace(name: &str) -> PathBuf {
    let root = scratch(name);
    for input in INPUTS {
        if input == "crates/ledger/build.rs" {
            continue;
        }
        let path = if input.contains('.') {
            root.join(input)
        } else {
            root.join(input).join("sub").join("file.rs")
        };
        write(&path, &format!("// {input}\n"));
    }
    write(&root.join("crates/kernel/tests/t.rs"), "// a test\n");
    root
}

fn hash(root: &Path) -> String {
    kernel_cache::source_hash(root).expect("a hash")
}

/// For each of `edits`, a path relative to the workspace: editing it (or creating it) changes the hash.
fn check_hash_covers(edits: &[&str]) {
    let root = workspace("covers");
    let before = hash(&root);
    assert_eq!(before.len(), 64, "the hash isn't 64 hex digits: {before}");
    assert!(before.bytes().all(|b| b.is_ascii_hexdigit()));
    for edit in edits {
        let path = root.join(edit);
        let old = fs::read_to_string(&path).unwrap_or_default();
        write(&path, &format!("{old}// edited\n"));
        let after = hash(&root);
        assert_ne!(before, after, "hash unchanged after editing {edit}");
        if old.is_empty() {
            fs::remove_file(&path).expect("the added file removed");
        } else {
            write(&path, &old);
        }
        assert_eq!(
            hash(&root),
            before,
            "hash not restored after undoing {edit}"
        );
    }
}

/// Every input, one file of each: a file input itself, a file deep in a directory input, and the absent build script.
fn every_input() -> Vec<String> {
    INPUTS
        .iter()
        .map(|input| {
            if input.contains('.') {
                (*input).to_owned()
            } else {
                format!("{input}/sub/file.rs")
            }
        })
        .collect()
}

#[test]
fn kernel_source_hash_covers_every_input() {
    let edits = every_input();
    let edits: Vec<&str> = edits.iter().map(String::as_str).collect();
    check_hash_covers(&edits);
}

negative_control!(
    kernel_source_hash_covers_every_input,
    "a file the kernel's build doesn't read, its test, taken as an input",
    expected = "hash unchanged after editing crates/kernel/tests/t.rs",
    check_hash_covers(&["crates/kernel/tests/t.rs"])
);

/// The hash of the same files in two places is the same, and a file outside the inputs doesn't change it.
fn check_hash_is_one_definition(outside: &str) {
    let a = workspace("one_a");
    let b = workspace("one_b");
    assert_eq!(
        hash(&a),
        hash(&b),
        "two copies of one workspace hash differently"
    );
    write(&b.join(outside), "// not read by the kernel's build\n");
    assert_eq!(
        hash(&a),
        hash(&b),
        "a file outside the inputs changed the hash: {outside}"
    );
}

#[test]
fn kernel_source_hash_is_one_definition() {
    check_hash_is_one_definition("crates/kernel/tests/t.rs");
    check_hash_is_one_definition("crates/engine/src/lib.rs");
}

negative_control!(
    kernel_source_hash_is_one_definition,
    "an input taken as outside them",
    expected = "a file outside the inputs changed the hash",
    check_hash_is_one_definition("crates/kernel/src/lib.rs")
);

/// The hash of this repository: what `cargo xtask kernel-src-hash` prints, every input but the ledger's build script
/// there.
#[test]
fn kernel_source_hash_reads_this_workspace() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace");
    check_reads(root, "crates/kernel/src/lib.rs");
}

fn check_reads(root: &Path, wanted: &str) {
    let manifest = kernel_cache::manifest(root).expect("the manifest");
    assert!(
        manifest
            .lines()
            .any(|l| l.starts_with(&format!("{wanted} "))),
        "the hash reads no {wanted}"
    );
    assert!(manifest.contains("crates/ledger/build.rs absent"));
}

negative_control!(
    kernel_source_hash_reads_this_workspace,
    "a file outside the inputs wanted",
    expected = "the hash reads no crates/engine/src/lib.rs",
    check_reads(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the workspace"),
        "crates/engine/src/lib.rs"
    )
);

fn outputs(root: &Path, tag: &str) {
    for output in OUTPUTS {
        write(&root.join(output), &format!("{output} {tag}\n"));
    }
}

/// An entry stored from one workspace restores into another byte for byte; after `spoil`, given the entry's folder, no
/// hit, and the entry gone.
fn check_round_trip(spoil: impl Fn(&Path)) {
    let cache = scratch("cache");
    let from = scratch("from");
    let to = scratch("to");
    outputs(&from, "built");
    assert!(
        !kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "a hit in an empty cache"
    );
    kernel_cache::store(&cache, "h1", &from).expect("stored");
    assert!(
        kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "no hit after a store"
    );
    for output in OUTPUTS {
        assert_eq!(
            fs::read(from.join(output)).expect("built"),
            fs::read(to.join(output)).expect("restored"),
            "{output} restored differs"
        );
    }
    assert!(
        fs::read_dir(&cache).expect("the cache").all(|e| {
            !e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .starts_with(PARTIAL)
        }),
        "a partial entry left behind"
    );
    spoil(&cache.join("h1"));
    assert!(
        !kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "an entry not whole restored"
    );
    assert!(
        !cache.join("h1").exists(),
        "an entry not whole left in place"
    );
    kernel_cache::store(&cache, "h1", &from).expect("stored again");
    assert!(
        kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "no hit after storing again"
    );
}

#[test]
fn kernel_cache_restores_only_whole_entries() {
    check_round_trip(|entry| fs::write(entry.join("kernel.wgsl"), "changed").expect("spoiled"));
    check_round_trip(|entry| fs::remove_file(entry.join("complete")).expect("spoiled"));
    check_round_trip(|entry| fs::remove_file(entry.join("kernel.spv")).expect("spoiled"));
}

negative_control!(
    kernel_cache_restores_only_whole_entries,
    "an entry left whole taken as spoiled",
    expected = "an entry not whole restored",
    check_round_trip(|_| ())
);

/// A fingerprint directory of cargo's as a SPIR-V build leaves it: the kernel library's fingerprint, its build script's
/// folder (no [`FINGERPRINT`]) and a dependency's (a [`FINGERPRINT`] of its own name); with `kernel` false, the kernel's library folder is left out.
fn fingerprint_dir(kernel: bool) -> PathBuf {
    let dir = scratch("fingerprints");
    if kernel {
        write(
            &dir.join("kernel-4d5226f7ba38a476").join(FINGERPRINT),
            "{\"rustflags\":[\"-Zcodegen-backend=x\"]}",
        );
    }
    write(
        &dir.join("kernel-cfb0e32c5a9f377a/run-build-script-build-script-build.json"),
        "{}",
    );
    write(&dir.join("glam-0b2c64b9138baa53").join(FINGERPRINT), "{}");
    dir
}

/// An entry stored with the build's fingerprints (`store_built`, or `store` with `built` false) and then hit: the
/// kernel's fingerprint is put back, byte for byte, at its own folder of another fingerprint directory, and nothing else
/// of the build's is; after it is changed in the entry, the entry is no hit.
fn check_fingerprints_round_trip(built: bool) {
    let cache = scratch("cache");
    let from = scratch("from");
    let to = scratch("to");
    let fingerprints = fingerprint_dir(true);
    outputs(&from, "built");
    if built {
        kernel_cache::store_built(&cache, "h1", &from, &fingerprints).expect("stored");
    } else {
        kernel_cache::store(&cache, "h1", &from).expect("stored");
    }
    assert!(
        kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "no hit after a store"
    );
    let into = scratch("into");
    assert_eq!(
        kernel_cache::restore_fingerprints(&cache, "h1", &into).expect("restored"),
        1,
        "the build's fingerprint not restored"
    );
    let file = Path::new("kernel-4d5226f7ba38a476").join(FINGERPRINT);
    assert_eq!(
        fs::read(into.join(&file)).expect("restored"),
        fs::read(fingerprints.join(&file)).expect("built"),
        "the restored fingerprint differs"
    );
    let mut names: Vec<_> = fs::read_dir(&into)
        .expect("the directory")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .into_string()
                .expect("a name")
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["kernel-4d5226f7ba38a476"],
        "more than the kernel's fingerprint restored"
    );
    fs::write(
        cache.join("h1").join(FINGERPRINTS).join(&file),
        "{\"rustflags\":[\"-Zcodegen-backend=x\",\"-Cdebuginfo=2\"]}",
    )
    .expect("changed");
    assert!(
        !kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "an entry with a changed fingerprint restored"
    );
}

#[test]
fn kernel_cache_entry_carries_the_builds_fingerprints() {
    check_fingerprints_round_trip(true);
}

negative_control!(
    kernel_cache_entry_carries_the_builds_fingerprints,
    "the entry stored without the build's fingerprints",
    expected = "the build's fingerprint not restored",
    check_fingerprints_round_trip(false)
);

/// `store_built` from a fingerprint directory with (`kernel`) or without the kernel's library fingerprint: refused
/// without, leaving no entry.
fn check_store_needs_a_fingerprint(kernel: bool) {
    let cache = scratch("cache");
    let from = scratch("from");
    outputs(&from, "built");
    let result = kernel_cache::store_built(&cache, "h1", &from, &fingerprint_dir(kernel));
    assert!(
        result
            .as_ref()
            .is_err_and(|e| e.contains("no fingerprint of the kernel's SPIR-V build")),
        "an entry stored without the build's fingerprint: {result:?}"
    );
    assert!(
        !cache.join("h1").exists(),
        "an entry left without a fingerprint"
    );
}

#[test]
fn kernel_cache_stores_no_built_entry_without_a_fingerprint() {
    check_store_needs_a_fingerprint(false);
}

negative_control!(
    kernel_cache_stores_no_built_entry_without_a_fingerprint,
    "the kernel's fingerprint there",
    expected = "an entry stored without the build's fingerprint",
    check_store_needs_a_fingerprint(true)
);

/// The kernel's fingerprint folders are `kernel-` then letters and digits, at least one: never a path out of the folder.
fn check_kernel_folders(cases: &[(&str, bool)]) {
    for (name, want) in cases {
        assert_eq!(
            kernel_cache::is_kernel_folder(name),
            *want,
            "the kernel folder check on {name:?}"
        );
    }
}

const KERNEL_FOLDERS: [(&str, bool); 9] = [
    ("kernel-4d5226f7ba38a476", true),
    ("kernel-a", true),
    ("kernel-", false),
    ("kernel", false),
    ("Kernel-a", false),
    ("glam-0b2c64b9138baa53", false),
    ("kernel-..", false),
    ("kernel-a/b", false),
    ("kernel-a_b", false),
];

#[test]
fn kernel_cache_fingerprint_folders_are_the_kernels_alone() {
    check_kernel_folders(&KERNEL_FOLDERS);
}

negative_control!(
    kernel_cache_fingerprint_folders_are_the_kernels_alone,
    "a folder of `kernel-` alone taken as the kernel's",
    expected = "the kernel folder check on \"kernel-\"",
    check_kernel_folders(&[("kernel-", true)])
);

/// A mark listing a fingerprint at a path outside the entry's fingerprints makes the entry not whole.
fn check_mark_names(line: &str) {
    let cache = scratch("cache");
    let from = scratch("from");
    let to = scratch("to");
    outputs(&from, "built");
    kernel_cache::store_built(&cache, "h1", &from, &fingerprint_dir(true)).expect("stored");
    let mark = cache.join("h1").join("complete");
    let text = fs::read_to_string(&mark).expect("the mark");
    let (_, sha) = text
        .lines()
        .last()
        .and_then(|l| l.split_once(' '))
        .expect("a fingerprint line");
    let body: Vec<&str> = text.lines().take(OUTPUTS.len()).collect();
    fs::write(&mark, format!("{}\n{line} {sha}\n", body.join("\n"))).expect("rewritten");
    assert!(
        !kernel_cache::restore(&cache, "h1", &to).expect("a lookup"),
        "an entry whose mark names {line} restored"
    );
}

#[test]
fn kernel_cache_mark_names_only_kernel_fingerprints() {
    check_mark_names("fingerprints/glam-0b2c64b9138baa53/lib-kernel.json");
    check_mark_names("fingerprints/kernel-4d5226f7ba38a476/lib-glam.json");
    check_mark_names("kernel-4d5226f7ba38a476/lib-kernel.json");
}

negative_control!(
    kernel_cache_mark_names_only_kernel_fingerprints,
    "the mark's own fingerprint line, left as stored",
    expected = "an entry whose mark names",
    check_mark_names("fingerprints/kernel-4d5226f7ba38a476/lib-kernel.json")
);

const GIB: u64 = 1 << 30;

/// A folder of `bytes` bytes (sparse), its files and itself last written `age` hours ago.
fn folder(path: &Path, bytes: u64, age: u64) {
    fs::create_dir_all(path.join("deep")).expect("a folder");
    let file = fs::File::create(path.join("deep").join("blob")).expect("a file");
    file.set_len(bytes).expect("a size");
    let when = SystemTime::now() - Duration::from_secs(age * 3600);
    file.set_modified(when).expect("a time");
    for dir in [path.join("deep"), path.to_path_buf()] {
        fs::File::open(&dir)
            .and_then(|d| d.set_modified(when))
            .expect("a folder's time");
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .map(|r| {
            r.map(|e| {
                e.expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// The trim of a cache holding four target folders of 2 GiB (oldest `a`, then `b`, `c`, `d`) and seven kernel entries
/// with a partial one, at a 5 GiB cap, keeping `keep` and `kernels` entries.
fn check_trim(keep: &str, kernels: usize, want_targets: &[&str], want_kernels: usize) {
    let root = scratch("trim");
    for (name, age) in [("a", 40), ("b", 30), ("c", 20), ("d", 10)] {
        folder(&root.join("target").join(name), 2 * GIB, age);
    }
    for k in 0..7u64 {
        let from = scratch("entry");
        outputs(&from, &k.to_string());
        kernel_cache::store(&root.join("kernel"), &format!("k{k}"), &from).expect("stored");
        let p = root.join("kernel").join(format!("k{k}"));
        let when = SystemTime::now() - Duration::from_secs((10 - k) * 3600);
        for f in names(&p) {
            fs::File::options()
                .write(true)
                .open(p.join(f))
                .and_then(|d| d.set_modified(when))
                .expect("a file's time");
        }
        fs::File::open(&p)
            .and_then(|d| d.set_modified(when))
            .expect("a folder's time");
    }
    fs::create_dir_all(root.join("kernel").join(format!("{PARTIAL}k9-1")))
        .expect("a partial entry");
    let keep_path = root.join("target").join(keep);
    kernel_cache::trim(&root, 5 * GIB, kernels, Some(&keep_path)).expect("trimmed");
    assert_eq!(
        names(&root.join("target")),
        want_targets,
        "the target folders left"
    );
    let left = names(&root.join("kernel"));
    assert_eq!(
        left.len(),
        want_kernels,
        "the kernel entries left: {left:?}"
    );
    let newest: Vec<String> = (0..7u64)
        .rev()
        .take(want_kernels)
        .map(|k| format!("k{k}"))
        .collect();
    for k in &newest {
        assert!(left.contains(k), "the newest entry {k} removed: {left:?}");
    }
}

#[test]
fn trim_removes_the_oldest_first_but_the_running_jobs() {
    // 8 GiB over a 5 GiB cap: the two oldest go.
    check_trim("d", 5, &["c", "d"], 5);
    // The running job's folder, the oldest, stays; the next two oldest go.
    check_trim("a", 5, &["a", "d"], 5);
}

negative_control!(
    trim_removes_the_oldest_first_but_the_running_jobs,
    "the trim expected to keep the oldest folder that isn't the running job's",
    expected = "the target folders left",
    check_trim("d", 5, &["a", "d"], 5)
);

#[test]
fn trim_keeps_the_newest_kernel_entries() {
    check_trim("d", 5, &["c", "d"], 5);
    check_trim("d", 2, &["c", "d"], 2);
}

negative_control!(
    trim_keeps_the_newest_kernel_entries,
    "every kernel entry expected kept",
    expected = "the kernel entries left",
    check_trim("d", 5, &["c", "d"], 7)
);

/// A link in the target folder to a folder outside the cache: the trim neither follows nor removes what it points at.
#[cfg(unix)]
fn check_link(remove_outside: bool) {
    let root = scratch("link");
    let outside = scratch("outside");
    folder(&outside, 8 * GIB, 100);
    fs::create_dir_all(root.join("target")).expect("a folder");
    std::os::unix::fs::symlink(&outside, root.join("target").join("link")).expect("a link");
    folder(&root.join("target").join("x"), GIB, 1);
    if remove_outside {
        fs::remove_dir_all(outside.join("deep")).expect("removed");
    }
    kernel_cache::trim(&root, 0, 5, None).expect("trimmed");
    assert!(
        outside.join("deep").join("blob").exists(),
        "the trim removed a file outside the cache through a link"
    );
    assert!(
        root.join("target").join("link").exists(),
        "the trim removed a link it should leave"
    );
}

#[cfg(unix)]
#[test]
fn trim_never_follows_a_link() {
    check_link(false);
}

#[cfg(unix)]
negative_control!(
    trim_never_follows_a_link,
    "the file outside gone before the trim",
    expected = "the trim removed a file outside the cache through a link",
    check_link(true)
);

/// The cap's boundary: target folders holding exactly the cap stay whole; one byte over it, the oldest goes.
fn check_cap_boundary(over: u64, want: &[&str]) {
    let root = scratch("cap");
    for (name, age) in [("a", 20), ("b", 10)] {
        folder(&root.join("target").join(name), GIB, age);
    }
    let total: u64 = ["a", "b"]
        .iter()
        .map(|n| {
            kernel_cache::size_and_newest(&root.join("target").join(n))
                .expect("a size")
                .0
        })
        .sum();
    kernel_cache::trim(&root, total - over, 5, None).expect("trimmed");
    assert_eq!(
        names(&root.join("target")),
        want,
        "the target folders left at the cap's boundary"
    );
}

#[test]
fn trim_stops_at_the_cap() {
    check_cap_boundary(0, &["a", "b"]);
    check_cap_boundary(1, &["b"]);
}

negative_control!(
    trim_stops_at_the_cap,
    "folders holding exactly the cap expected trimmed",
    expected = "the target folders left at the cap's boundary",
    check_cap_boundary(0, &["b"])
);
