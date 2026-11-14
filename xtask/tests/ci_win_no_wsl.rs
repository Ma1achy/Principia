//! No test the Windows runner runs may start a bare `bash` or `wsl` (the owner, 14 Nov 2026: "The PC runs the GPU suites
//! only. Tests that fail there for lack of Unix tools ... are excluded on Windows with a stated reason (they run on
//! hosted Linux already), not fixed"; TASK-M0-71).
//!
//! On the PC, `Command::new("bash")` finds C:\Windows\System32\bash.exe, WSL's launcher, before any bash on PATH, and so
//! runs in the account's default WSL distro, whose files it can change: the nightly `win-tests` job's xtask tests did,
//! appending to `ci`'s `~/.cargo/config.toml` in `principia-wsl` (run 43593938536). `wsl` goes there by name.
//!
//! - `win_runner_tests_start_no_bare_bash_or_wsl`: every cargo command of every job of `win-gpu.yml` on the runner is
//!   read, with the nextest profile its step names, and each test file it can reach is found: a `--test` target, a
//!   profile's `binary_id(=…)` list, every test file holding a name filter's text (a superset of the tests the filter
//!   selects), or every test of a package that it runs whole; and the modules each of those files includes. Those
//!   files, and every library source of the workspace, which any test binary may link, must hold no
//!   `Command::new` of `bash`, `wsl` or System32's copy of either, unless the file is left out on Windows by an inner
//!   `#![cfg(unix)]` or `#![cfg(not(windows))]` with its reason in the comment above it. A command the check can't read
//!   fails it, so a new kind of step can't pass by being skipped.
//!
//! It starts no process: it reads files only, so that it holds wherever it runs. Its negative controls (R-176) put
//! back a whole-workspace run on the runner, plant a bare `bash` in a reached test and in a library source, lift a
//! file's Windows exclusion or its reason, and give the check a command it can't read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const NEXTEST: &str = ".config/nextest.toml";
/// The programs that reach WSL on the PC, by bare name or as System32 holds them.
const WSL_PROGRAMS: [&str; 4] = ["bash", "bash.exe", "wsl", "wsl.exe"];
/// The inner attributes that leave a file out of a Windows build.
const NOT_ON_WINDOWS: [&str; 4] = [
    "#![cfg(unix)]",
    "#![cfg(not(windows))]",
    "#![cfg(not(target_os=\"windows\"))]",
    "#![cfg(target_family=\"unix\")]",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("ci_win_no_wsl: {rel}: {e}"))
}

/// The workspace's sources: each package by name, its directory, and every `.rs` file under its `src/` and `tests/`,
/// by path from the workspace root with `/` separators.
#[derive(Clone)]
struct Tree {
    packages: BTreeMap<String, String>,
    files: BTreeMap<String, String>,
}

fn walk(dir: &Path, rel: &str, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("a directory entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        let rel = format!("{rel}/{name}");
        if path.is_dir() {
            walk(&path, &rel, out);
        } else if name.ends_with(".rs") {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("ci_win_no_wsl: {rel}: {e}"));
            out.insert(rel, text);
        }
    }
}

fn tree() -> Tree {
    let root = root();
    let mut dirs = vec!["xtask".to_owned()];
    for entry in std::fs::read_dir(root.join("crates")).expect("crates/") {
        let name = entry
            .expect("an entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        dirs.push(format!("crates/{name}"));
    }
    let mut packages = BTreeMap::new();
    let mut files = BTreeMap::new();
    for dir in dirs {
        let Ok(manifest) = std::fs::read_to_string(root.join(&dir).join("Cargo.toml")) else {
            continue;
        };
        let name = manifest
            .lines()
            .find_map(|l| l.strip_prefix("name = \""))
            .and_then(|l| l.strip_suffix('"'))
            .unwrap_or_else(|| panic!("ci_win_no_wsl: {dir}/Cargo.toml names no package"));
        packages.insert(name.to_owned(), dir.clone());
        for sub in ["src", "tests"] {
            walk(
                &root.join(&dir).join(sub),
                &format!("{dir}/{sub}"),
                &mut files,
            );
        }
    }
    Tree { packages, files }
}

/// A command line's words, as a shell splits them: quotes group and are dropped.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    assert!(
        quote.is_none(),
        "ci_win_no_wsl: an unclosed quote in {line:?}"
    );
    if started {
        out.push(word);
    }
    out
}

/// The `binary_id(=<package>::<target>)` list a profile's `default-filter` is, and nothing else; `None` when the profile
/// has no `default-filter`.
fn profile_binaries(nextest: &str, profile: &str) -> Option<Vec<(String, String)>> {
    let head = format!("[profile.{profile}]\n");
    let body = nextest.split(head.as_str()).nth(1)?;
    let body = body.split("\n[").next().unwrap_or(body);
    let filter = body
        .split("default-filter = \"\"\"")
        .nth(1)?
        .split("\"\"\"")
        .next()?;
    let mut out = Vec::new();
    for item in filter.split('|') {
        let item = item.trim();
        let target = item
            .strip_prefix("binary_id(=")
            .and_then(|i| i.strip_suffix(')'))
            .unwrap_or_else(|| {
                panic!("ci_win_no_wsl: the profile {profile}'s filter holds {item:?}, which the check can't read")
            });
        let (package, binary) = target
            .split_once("::")
            .unwrap_or_else(|| panic!("ci_win_no_wsl: {target:?} names no package"));
        out.push((package.to_owned(), binary.to_owned()));
    }
    Some(out)
}

/// The modules a test file includes, by `mod <name>;`, with a `#[path = "…"]` on the line above it if there is one. A
/// test target's modules sit beside it, as a `main.rs`'s or `mod.rs`'s do; another module's, in a folder of its name.
/// One found nowhere is left out: it wouldn't compile, so it runs nothing.
fn modules(path: &str, text: &str, tree: &Tree) -> Vec<String> {
    let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
    let target = dir.ends_with("/tests") || matches!(file, "mod.rs" | "main.rs");
    let base = if target {
        dir.to_owned()
    } else {
        format!("{dir}/{}", file.trim_end_matches(".rs"))
    };
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let line = line.strip_prefix("pub ").unwrap_or(line);
        let Some(name) = line.strip_prefix("mod ").and_then(|l| l.strip_suffix(';')) else {
            continue;
        };
        let explicit = i
            .checked_sub(1)
            .and_then(|p| lines[p].strip_prefix("#[path = \""))
            .and_then(|p| p.strip_suffix("\"]"));
        let candidates = match explicit {
            Some(p) => vec![format!("{dir}/{p}")],
            None => vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")],
        };
        out.extend(
            candidates
                .into_iter()
                .find(|c| tree.files.contains_key(c.as_str())),
        );
    }
    out
}

/// Each test target file of `dir`: `tests/<name>.rs` and `tests/<name>/main.rs`.
fn targets(tree: &Tree, dir: &str) -> Vec<String> {
    let tests = format!("{dir}/tests/");
    tree.files
        .keys()
        .filter(|f| {
            f.strip_prefix(tests.as_str()).is_some_and(|rest| {
                !rest.contains('/') || rest.matches('/').count() == 1 && rest.ends_with("/main.rs")
            })
        })
        .cloned()
        .collect()
}

/// The test files one cargo command on the runner can reach, before their modules; `what` names it in a failure.
fn reached(
    command: &[String],
    profile: &str,
    nextest: &str,
    tree: &Tree,
    what: &str,
) -> Vec<String> {
    let tests = match command.get(1).map(String::as_str) {
        Some("nextest") if command.get(2).map(String::as_str) == Some("run") => 3,
        Some("test") => 2,
        // `cargo xtask …`, `cargo build …`: what they run is the workspace's library code, which is read whole.
        Some(_) => return Vec::new(),
        None => panic!("ci_win_no_wsl: {what}: a bare `cargo`"),
    };
    let (mut packages, mut targets_named, mut names) = (Vec::new(), Vec::new(), Vec::new());
    let (mut doc, mut profile) = (false, profile.to_owned());
    let mut args = command[tests..].iter();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("ci_win_no_wsl: {what}: {arg} has no value"))
                .clone()
        };
        match arg.as_str() {
            "-p" | "--package" => packages.push(value()),
            "--test" => targets_named.push(value()),
            "--profile" => profile = value(),
            "-E" | "--filter-expr" => {
                let expr = value();
                let name = expr
                    .strip_prefix("test(=")
                    .and_then(|e| e.strip_suffix(')'))
                    .unwrap_or_else(|| {
                        panic!("ci_win_no_wsl: {what}: the filter {expr:?}, which the check can't read")
                    });
                names.push(name.rsplit("::").next().unwrap_or(name).to_owned());
            }
            "--features" | "--retries" | "--partition" => {
                value();
            }
            "--workspace" => packages.extend(tree.packages.keys().cloned()),
            "--doc" => doc = true,
            "--no-capture" | "--no-fail-fast" | "--release" => {}
            other if other.starts_with('-') => {
                panic!("ci_win_no_wsl: {what}: the argument {other}, which the check can't read")
            }
            name => names.push(name.to_owned()),
        }
    }
    assert!(
        !packages.is_empty(),
        "ci_win_no_wsl: {what}: no package named"
    );
    if doc {
        // Doctests are the libraries' own, read whole.
        return Vec::new();
    }
    let dir = |package: &str| {
        tree.packages
            .get(package)
            .unwrap_or_else(|| panic!("ci_win_no_wsl: {what}: no package {package}"))
            .clone()
    };
    let mut out = Vec::new();
    for package in &packages {
        let dir = dir(package);
        let all = targets(tree, &dir);
        if !targets_named.is_empty() {
            for name in &targets_named {
                let target = all
                    .iter()
                    .find(|t| {
                        **t == format!("{dir}/tests/{name}.rs")
                            || **t == format!("{dir}/tests/{name}/main.rs")
                    })
                    .unwrap_or_else(|| {
                        panic!("ci_win_no_wsl: {what}: no test target {package}::{name}")
                    });
                out.push(target.clone());
            }
        } else if !names.is_empty() {
            // A name filter selects by a test's whole path, its modules' names among it: a file holding the text
            // anywhere stands for every test it might select.
            let prefix = format!("{dir}/tests/");
            out.extend(
                tree.files
                    .iter()
                    .filter(|(f, text)| {
                        f.starts_with(prefix.as_str())
                            && names.iter().any(|n| text.contains(n.as_str()))
                    })
                    .map(|(f, _)| f.clone()),
            );
        } else if let Some(binaries) = profile_binaries(nextest, &profile) {
            for (p, binary) in binaries.iter().filter(|(p, _)| p == package) {
                let target = all
                    .iter()
                    .find(|t| **t == format!("{dir}/tests/{binary}.rs") || **t == format!("{dir}/tests/{binary}/main.rs"))
                    .unwrap_or_else(|| panic!("ci_win_no_wsl: the profile {profile} names {p}::{binary}, which isn't there"));
                out.push(target.clone());
            }
        } else {
            out.extend(all);
        }
    }
    out
}

/// The text a file compiles from, its plain `//` comments left out; doc comments stay, as doctests run them.
fn code(text: &str) -> String {
    text.lines()
        .filter(|l| {
            let l = l.trim_start();
            !l.starts_with("//") || l.starts_with("///") || l.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each program a file starts with `Command::new("…")` that reaches WSL on the PC.
fn wsl_spawns(text: &str) -> Vec<String> {
    let code = code(text);
    let mut out = Vec::new();
    for (i, _) in code.match_indices("Command::new(") {
        let rest = code[i + "Command::new(".len()..].trim_start();
        let Some(literal) = rest.strip_prefix('"').and_then(|r| r.split('"').next()) else {
            continue;
        };
        let lower = literal.to_lowercase();
        let base = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
        let bare = !lower.contains(['/', '\\']);
        if WSL_PROGRAMS.contains(&base) && (bare || lower.contains("system32")) {
            out.push(literal.to_owned());
        }
    }
    out
}

/// Whether a file is left out on Windows by an inner `cfg`, its reason in the comment on the line above.
fn excluded_on_windows(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(i, line)| {
        let squeezed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        NOT_ON_WINDOWS.contains(&squeezed.as_str())
            && i.checked_sub(1).is_some_and(|p| {
                lines[p].starts_with("//") && lines[p].trim_start_matches('/').trim().len() > 1
            })
    })
}

/// The runner's jobs' cargo commands, each with its nextest profile and a name for failures.
fn runner_commands(workflow: &str) -> Vec<(Vec<String>, String, String)> {
    let doc = YamlLoader::load_from_str(workflow)
        .expect("ci_win_no_wsl: win-gpu.yml parses")
        .remove(0);
    let jobs = doc["jobs"]
        .as_hash()
        .expect("ci_win_no_wsl: win-gpu.yml has jobs");
    let profile_of = |env: &Yaml| env["NEXTEST_PROFILE"].as_str().map(str::to_owned);
    let mut out = Vec::new();
    for (id, job) in jobs {
        let id = id.as_str().expect("a job id");
        let hosted = job["runs-on"].as_str().is_some_and(|l| {
            !l.contains("${{")
                && ["ubuntu-", "macos-", "windows-"]
                    .iter()
                    .any(|i| l.starts_with(i))
        });
        if hosted {
            continue;
        }
        for step in job["steps"].as_vec().into_iter().flatten() {
            let Some(run) = step["run"].as_str() else {
                continue;
            };
            let profile = profile_of(&step["env"])
                .or_else(|| profile_of(&job["env"]))
                .or_else(|| profile_of(&doc["env"]))
                .unwrap_or_else(|| "default".to_owned());
            let name = step["name"].as_str().unwrap_or("?");
            for line in run.lines() {
                let all = words(line);
                if let Some(at) = all.iter().position(|w| w == "cargo") {
                    out.push((all[at..].to_vec(), profile.clone(), format!("{id}/{name}")));
                }
            }
        }
    }
    out
}

fn check(workflow: &str, nextest: &str, tree: &Tree) {
    let commands = runner_commands(workflow);
    assert!(
        commands
            .iter()
            .any(|(c, _, _)| c.get(1).is_some_and(|s| s == "nextest" || s == "test")),
        "ci_win_no_wsl: no test command read from the runner's jobs"
    );
    let mut reach: BTreeMap<String, String> = BTreeMap::new();
    let mut queue: Vec<(String, String)> = Vec::new();
    for (command, profile, what) in &commands {
        for file in reached(command, profile, nextest, tree, what) {
            queue.push((file, what.clone()));
        }
    }
    while let Some((file, what)) = queue.pop() {
        if reach.contains_key(&file) {
            continue;
        }
        for module in modules(&file, &tree.files[&file], tree) {
            queue.push((module, what.clone()));
        }
        reach.insert(file, what);
    }
    assert!(
        !reach.is_empty(),
        "ci_win_no_wsl: no test file reached from the runner's jobs"
    );
    for file in tree.files.keys().filter(|f| f.contains("/src/")) {
        reach
            .entry(file.clone())
            .or_insert_with(|| "any test binary, as library code".to_owned());
    }
    let mut found = BTreeSet::new();
    for (file, what) in &reach {
        let text = &tree.files[file];
        if excluded_on_windows(text) {
            continue;
        }
        for program in wsl_spawns(text) {
            found.insert(format!("{file} starts \"{program}\" (reached by {what})"));
        }
    }
    assert!(
        found.is_empty(),
        "ci_win_no_wsl: a test the Windows runner runs starts bash or wsl, which on the PC is WSL's launcher:\n{}",
        found.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn win_runner_tests_start_no_bare_bash_or_wsl() {
    check(&read(WORKFLOW), &read(NEXTEST), &tree());
}

/// The tree with one file's text edited, or a file added; the edit must find its text.
#[cfg(feature = "controls")]
fn planted(path: &str, from: &str, to: &str) -> Tree {
    let mut tree = tree();
    let text = tree.files.entry(path.to_owned()).or_default();
    assert!(
        text.contains(from),
        "ci_win_no_wsl: a control's edit is gone from {path}"
    );
    *text = text.replacen(from, to, 1);
    tree
}

#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "ci_win_no_wsl: a control's edit is gone"
    );
    text.replacen(from, to, 1)
}

negative_control!(
    win_runner_tests_start_no_bare_bash_or_wsl,
    "The nightly whole-workspace run put back on the runner, which reaches xtask's tests that start bash.",
    expected = "a test the Windows runner runs starts bash or wsl",
    check(
        &edited(
            &read(WORKFLOW),
            "      - name: The guard on this workflow\n",
            "      - name: The workspace\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\win_measure.ps1 run workspace cargo nextest run --workspace --no-fail-fast --retries 0\n          exit $LASTEXITCODE\n      - name: The guard on this workflow\n"
        ),
        &read(NEXTEST),
        &tree()
    )
);

mod in_a_reached_test {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "The guard test win-self-check runs, starting a bare bash.",
        expected = "xtask/tests/ci_self_hosted.rs starts \"bash\"",
        check(
            &read(WORKFLOW),
            &read(NEXTEST),
            &planted(
                "xtask/tests/ci_self_hosted.rs",
                "use std::path::{Path, PathBuf};\n",
                "use std::path::{Path, PathBuf};\nfn _bash() { let _ = std::process::Command::new(\"bash\"); }\n"
            )
        )
    );
}

mod system32 {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A GPU test target of the gpu-kernel profile starting System32's wsl.exe by its path.",
        expected = "starts \"C:\\\\Windows\\\\System32\\\\wsl.exe\"",
        check(
            &read(WORKFLOW),
            &read(NEXTEST),
            &planted(
                "crates/engine/tests/canon_gpu.rs",
                "",
                "fn _wsl() { let _ = std::process::Command::new(\"C:\\\\Windows\\\\System32\\\\wsl.exe\"); }\n"
            )
        )
    );
}

mod by_name_filter {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A new test file in engine whose tests a name filter of win-vulkan selects, starting wsl.",
        expected = "crates/engine/tests/planted.rs starts \"wsl\"",
        check(
            &read(WORKFLOW),
            &read(NEXTEST),
            &planted(
                "crates/engine/tests/planted.rs",
                "",
                "#[test]\nfn canonicaliser_planted() { let _ = std::process::Command::new(\"wsl\"); }\n"
            )
        )
    );
}

mod in_a_module {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A module a reached test target includes, starting bash.",
        expected = "crates/engine/tests/planted_helper.rs starts \"bash\"",
        check(&read(WORKFLOW), &read(NEXTEST), &{
            let mut tree = planted(
                "crates/engine/tests/canon_gpu.rs",
                "",
                "#[path = \"planted_helper.rs\"]\nmod helper;\n",
            );
            tree.files.insert(
                "crates/engine/tests/planted_helper.rs".to_owned(),
                "pub fn run() { let _ = std::process::Command::new(\"bash\"); }\n".to_owned(),
            );
            tree
        })
    );
}

mod in_library_code {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A library source of the workspace, which any test binary may link, starting bash.",
        expected = "crates/validation/src/spawn.rs starts \"bash\"",
        check(
            &read(WORKFLOW),
            &read(NEXTEST),
            &planted(
                "crates/validation/src/spawn.rs",
                "",
                "fn _bash() { let _ = std::process::Command::new(\"bash\"); }\n"
            )
        )
    );
}

mod exclusion_without_reason {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A reached file left out on Windows with no reason given above its cfg.",
        expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
        check(
            &read(WORKFLOW),
            &read(NEXTEST),
            &planted(
                "crates/engine/tests/canon_gpu.rs",
                "",
                "#![cfg(unix)]\nfn _bash() { let _ = std::process::Command::new(\"bash\"); }\n"
            )
        )
    );
}

mod unread_command {
    use super::*;
    negative_control!(
        win_runner_tests_start_no_bare_bash_or_wsl,
        "A test command on the runner with an argument the check can't read.",
        expected = "the argument --ignored, which the check can't read",
        check(
            &edited(
                &read(WORKFLOW),
                "--test ci_self_hosted --no-fail-fast",
                "--test ci_self_hosted --ignored --no-fail-fast"
            ),
            &read(NEXTEST),
            &tree()
        )
    );
}

/// A file left out on Windows with its reason is not read: the exclusion the owner's words allow.
#[test]
fn a_file_excluded_on_windows_with_its_reason_is_allowed() {
    let mut tree = tree();
    tree.files
        .get_mut("crates/engine/tests/canon_gpu.rs")
        .expect("canon_gpu.rs")
        .insert_str(
            0,
            "// Needs bash, which Windows' runner lacks; hosted Linux runs it.\n#![cfg(not(windows))]\nfn _bash() { let _ = std::process::Command::new(\"bash\"); }\n",
        );
    check(&read(WORKFLOW), &read(NEXTEST), &tree);
}

negative_control!(
    a_file_excluded_on_windows_with_its_reason_is_allowed,
    "The same file with no exclusion is refused.",
    expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
    check(
        &read(WORKFLOW),
        &read(NEXTEST),
        &planted(
            "crates/engine/tests/canon_gpu.rs",
            "",
            "fn _bash() { let _ = std::process::Command::new(\"bash\"); }\n"
        )
    )
);

/// The test files the runner's jobs reach include the GPU suites' and the guard's, so that the check can't pass by
/// reaching nothing it should.
#[test]
fn the_runner_reaches_the_gpu_suites_and_the_guard() {
    check_reach(&read(WORKFLOW), &read(NEXTEST), &tree());
}

fn check_reach(workflow: &str, nextest: &str, tree: &Tree) {
    let files: BTreeSet<String> = runner_commands(workflow)
        .iter()
        .flat_map(|(c, p, w)| reached(c, p, nextest, tree, w))
        .collect();
    for want in [
        "xtask/tests/ci_self_hosted.rs",
        "crates/engine/tests/canon_gpu.rs",
        "crates/validation/tests/codegen_selftest.rs",
        "xtask/tests/build_kernel.rs",
    ] {
        assert!(
            files.contains(want),
            "ci_win_no_wsl: the runner's jobs don't reach {want}: {files:#?}"
        );
    }
    for unwanted in ["xtask/tests/jobq.rs", "xtask/tests/ci_debug_info.rs"] {
        assert!(
            !files.contains(unwanted),
            "ci_win_no_wsl: the runner's jobs reach {unwanted}"
        );
    }
}

negative_control!(
    the_runner_reaches_the_gpu_suites_and_the_guard,
    "win-self-check's guard step gone: the guard test is no longer reached.",
    expected = "the runner's jobs don't reach xtask/tests/ci_self_hosted.rs",
    check_reach(
        &edited(
            &read(WORKFLOW),
            "--test ci_self_hosted",
            "--test build_kernel"
        ),
        &read(NEXTEST),
        &tree()
    )
);

mod reaches_too_much {
    use super::*;
    negative_control!(
        the_runner_reaches_the_gpu_suites_and_the_guard,
        "A whole-package run of xtask on the runner reaches the job queue's tests.",
        expected = "the runner's jobs reach xtask/tests/jobq.rs",
        check_reach(
            &edited(&read(WORKFLOW), "--test ci_self_hosted", ""),
            &read(NEXTEST),
            &tree()
        )
    );
}
