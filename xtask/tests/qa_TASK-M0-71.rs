//! qa for TASK-M0-71, written from REQ-TOOL-244: "The Windows runner must run the GPU suites only, with no job of the
//! nightly Windows build and CPU tests (`win-tests`); and no test a job on the runner can reach may start a bare `bash`
//! or `wsl`, or System32's copy of either, which on the runner's account is WSL's launcher and runs in its default
//! distro, unless the test's file is excluded on Windows with a stated reason."
//!
//! These checks are qa's own reading of the requirement, not the guard's (`ci_win_no_wsl.rs`): they parse the
//! workflow with a YAML crate, take the packages from the workspace manifest, read a program name from any Rust string
//! literal form (plain, raw, raw with hashes), and accept a Windows exclusion only as a file-level inner attribute, in
//! the run of comments and inner attributes that opens the file. They start no process. Each has its negative controls
//! (R-176).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const NEXTEST: &str = ".config/nextest.toml";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("qa: the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("qa: {rel}: {e}"))
}

#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa control: {from:?} is not in the text"
    );
    text.replacen(from, to, 1)
}

/// The text `Command::new` with a string literal, built so that this file holds no such call itself.
#[cfg(feature = "controls")]
fn spawn(literal: &str) -> String {
    format!(
        "fn _qa_planted() {{ let _ = std::process::Command::{}({literal}); }}\n",
        "new"
    )
}

// ---- The workspace's sources ---------------------------------------------------------------------------------------

#[derive(Clone)]
struct Sources {
    /// Package name to its directory, from the workspace manifest's `members`.
    packages: BTreeMap<String, String>,
    /// Every `.rs` file under a member's `src/` or `tests/`, by `/`-separated path from the root.
    files: BTreeMap<String, String>,
}

fn collect(dir: &Path, rel: &str, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.map(|e| e.expect("qa: a directory entry")) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = format!("{rel}/{name}");
        if entry.path().is_dir() {
            collect(&entry.path(), &rel, out);
        } else if name.ends_with(".rs") {
            out.insert(
                rel,
                std::fs::read_to_string(entry.path()).expect("qa: a source"),
            );
        }
    }
}

fn sources() -> Sources {
    let manifest = read("Cargo.toml");
    let members = manifest
        .split("members = [")
        .nth(1)
        .and_then(|m| m.split(']').next())
        .expect("qa: the workspace manifest lists its members");
    let mut packages = BTreeMap::new();
    let mut files = BTreeMap::new();
    for member in members
        .split(',')
        .map(|m| m.trim().trim_matches('"'))
        .filter(|m| !m.is_empty())
    {
        let toml = read(&format!("{member}/Cargo.toml"));
        let name = toml
            .lines()
            .find_map(|l| l.trim().strip_prefix("name = \"")?.strip_suffix('"'))
            .unwrap_or_else(|| panic!("qa: {member} names no package"));
        packages.insert(name.to_owned(), member.to_owned());
        for sub in ["src", "tests"] {
            collect(
                &root().join(member).join(sub),
                &format!("{member}/{sub}"),
                &mut files,
            );
        }
    }
    assert!(
        packages.len() >= 5,
        "qa: the workspace has too few members: {packages:?}"
    );
    Sources { packages, files }
}

// ---- The runner's jobs and their cargo commands --------------------------------------------------------------------

fn workflow(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("qa: win-gpu.yml parses")
        .remove(0)
}

/// A job runs on the runner unless `runs-on` is plainly one of GitHub's hosted images.
fn on_runner(job: &Yaml) -> bool {
    match &job["runs-on"] {
        Yaml::String(label) => {
            !["ubuntu-", "macos-", "windows-"]
                .iter()
                .any(|h| label.starts_with(h))
                || label.contains("${{")
        }
        _ => true,
    }
}

struct CargoCommand {
    words: Vec<String>,
    profile: String,
    at: String,
}

fn runner_cargo(text: &str) -> Vec<CargoCommand> {
    let doc = workflow(text);
    let mut out = Vec::new();
    for (id, job) in doc["jobs"].as_hash().expect("qa: win-gpu.yml has jobs") {
        if !on_runner(job) {
            continue;
        }
        let id = id.as_str().expect("qa: a job id");
        for step in job["steps"].as_vec().into_iter().flatten() {
            let Some(run) = step["run"].as_str() else {
                continue;
            };
            let profile = [&step["env"], &job["env"], &doc["env"]]
                .iter()
                .find_map(|env| env["NEXTEST_PROFILE"].as_str())
                .unwrap_or("default")
                .to_owned();
            for line in run.lines() {
                let words: Vec<String> = line
                    .split_whitespace()
                    .map(|w| w.trim_matches(|c| c == '\'' || c == '"').to_owned())
                    .collect();
                if let Some(at) = words.iter().position(|w| w == "cargo") {
                    out.push(CargoCommand {
                        words: words[at..].to_vec(),
                        profile: profile.clone(),
                        at: format!("{id} / {}", step["name"].as_str().unwrap_or("?")),
                    });
                }
            }
        }
    }
    out
}

/// What a test command selects, as read from its words.
#[derive(Default, Debug)]
struct Selection {
    packages: Vec<String>,
    whole_workspace: bool,
    targets: Vec<String>,
    names: Vec<String>,
    profile: Option<String>,
    doc: bool,
}

/// `None` for a cargo command that runs no test (`cargo xtask …`, `cargo build …`); panics on a word it can't read.
fn selection(command: &CargoCommand) -> Option<Selection> {
    let w = &command.words;
    let start = match (w.get(1).map(String::as_str), w.get(2).map(String::as_str)) {
        (Some("nextest"), Some("run")) => 3,
        (Some("test"), _) => 2,
        (Some(_), _) => return None,
        (None, _) => panic!("qa: {}: a bare cargo", command.at),
    };
    let mut s = Selection::default();
    let mut it = w[start..].iter();
    while let Some(word) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| panic!("qa: {}: {word} without a value", command.at))
                .clone()
        };
        match word.as_str() {
            "-p" | "--package" => s.packages.push(value()),
            "--workspace" | "--all" => s.whole_workspace = true,
            "--test" => s.targets.push(value()),
            "--profile" | "-P" => s.profile = Some(value()),
            "-E" | "--filter-expr" | "--filterset" => {
                let expr = value();
                let path = expr
                    .strip_prefix("test(=")
                    .and_then(|e| e.strip_suffix(')'))
                    .unwrap_or_else(|| {
                        panic!("qa: {}: a filter qa can't read: {expr}", command.at)
                    });
                s.names
                    .push(path.rsplit("::").next().unwrap_or(path).to_owned());
            }
            "--features" | "-F" | "--retries" | "--partition" => {
                value();
            }
            "--doc" => s.doc = true,
            "--no-capture" | "--nocapture" | "--no-fail-fast" | "--release" | "--locked" => {}
            flag if flag.starts_with('-') => {
                panic!("qa: {}: an argument qa can't read: {flag}", command.at)
            }
            name => s.names.push(name.to_owned()),
        }
    }
    if s.profile.is_none() {
        s.profile = Some(command.profile.clone());
    }
    Some(s)
}

/// The `binary_id(=package::target)` list that is a profile's whole default filter, if it has one.
fn profile_list(nextest: &str, profile: &str) -> Option<Vec<(String, String)>> {
    let section = nextest.split(&format!("\n[profile.{profile}]\n")).nth(1)?;
    let section = section.split("\n[").next()?;
    let filter = section
        .split("default-filter = \"\"\"")
        .nth(1)?
        .split("\"\"\"")
        .next()?;
    Some(
        filter
            .split('|')
            .map(|item| {
                let item = item.trim();
                let id = item
                    .strip_prefix("binary_id(=")
                    .and_then(|i| i.strip_suffix(')'))
                    .unwrap_or_else(|| {
                        panic!("qa: profile {profile}'s filter holds {item}, which qa can't read")
                    });
                let (p, t) = id.split_once("::").expect("qa: a binary id with a package");
                (p.to_owned(), t.to_owned())
            })
            .collect(),
    )
}

fn target_file(src: &Sources, dir: &str, name: &str) -> Option<String> {
    [
        format!("{dir}/tests/{name}.rs"),
        format!("{dir}/tests/{name}/main.rs"),
    ]
    .into_iter()
    .find(|f| src.files.contains_key(f))
}

/// The files a reached file pulls in by `mod name;`, with or without `#[path = "…"]`.
fn included(path: &str, text: &str, src: &Sources) -> Vec<String> {
    let (dir, file) = path.rsplit_once('/').expect("qa: a path with a directory");
    let owns_dir =
        dir.ends_with("/tests") || file == "main.rs" || file == "mod.rs" || file == "lib.rs";
    let base = if owns_dir {
        dir.to_owned()
    } else {
        format!("{dir}/{}", file.trim_end_matches(".rs"))
    };
    let mut out = Vec::new();
    let mut path_attr: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if let Some(p) = line
            .strip_prefix("#[path = \"")
            .and_then(|p| p.strip_suffix("\"]"))
        {
            path_attr = Some(p.to_owned());
            continue;
        }
        let decl = line
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub ");
        if let Some(name) = decl.strip_prefix("mod ").and_then(|d| d.strip_suffix(';')) {
            let candidates = match path_attr.take() {
                Some(p) => vec![format!("{dir}/{p}")],
                None => vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")],
            };
            out.extend(
                candidates
                    .into_iter()
                    .filter(|c| src.files.contains_key(c))
                    .take(1),
            );
        } else if !line.starts_with("#[") {
            path_attr = None;
        }
    }
    out
}

/// Every test file the runner's jobs can reach, with the step that reaches it.
fn reach(workflow_text: &str, nextest: &str, src: &Sources) -> BTreeMap<String, String> {
    let mut queue = Vec::new();
    let mut tests_read = 0;
    for command in runner_cargo(workflow_text) {
        let Some(s) = selection(&command) else {
            continue;
        };
        tests_read += 1;
        if s.doc {
            continue; // doctests are the libraries' own text, read whole below
        }
        let packages: Vec<String> = if s.whole_workspace {
            src.packages.keys().cloned().collect()
        } else {
            s.packages.clone()
        };
        assert!(!packages.is_empty(), "qa: {}: no package named", command.at);
        for package in &packages {
            let dir = src
                .packages
                .get(package)
                .unwrap_or_else(|| panic!("qa: {}: no package {package}", command.at));
            let tests = format!("{dir}/tests/");
            if !s.targets.is_empty() {
                for t in &s.targets {
                    let f = target_file(src, dir, t).unwrap_or_else(|| {
                        panic!("qa: {}: no test target {package}::{t}", command.at)
                    });
                    queue.push((f, command.at.clone()));
                }
            } else if !s.names.is_empty() {
                for (f, text) in &src.files {
                    if f.starts_with(&tests) && s.names.iter().any(|n| text.contains(n.as_str())) {
                        queue.push((f.clone(), command.at.clone()));
                    }
                }
            } else if let Some(list) =
                profile_list(nextest, s.profile.as_deref().unwrap_or("default"))
            {
                for (p, t) in list.iter().filter(|(p, _)| p == package) {
                    let f = target_file(src, dir, t).unwrap_or_else(|| {
                        panic!("qa: the profile lists {p}::{t}, which isn't there")
                    });
                    queue.push((f, command.at.clone()));
                }
            } else {
                for f in src.files.keys().filter(|f| f.starts_with(&tests)) {
                    queue.push((f.clone(), command.at.clone()));
                }
            }
        }
    }
    assert!(
        tests_read > 0,
        "qa: no test command read from the runner's jobs"
    );
    let mut out = BTreeMap::new();
    while let Some((f, at)) = queue.pop() {
        if out.contains_key(&f) {
            continue;
        }
        queue.extend(
            included(&f, &src.files[&f], src)
                .into_iter()
                .map(|m| (m, at.clone())),
        );
        out.insert(f, at);
    }
    out
}

// ---- What a file starts --------------------------------------------------------------------------------------------

/// The string literal at the start of `rest`, decoded: `"…"` with its escapes, `r"…"` and `r#"…"#`.
fn literal(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    if let Some(raw) = rest.strip_prefix('r') {
        let hashes = raw.chars().take_while(|&c| c == '#').count();
        let body = raw[hashes..].strip_prefix('"')?;
        let close = format!("\"{}", "#".repeat(hashes));
        return body.find(&close).map(|e| body[..e].to_owned());
    }
    let body = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// Whether a program, as written, reaches WSL on the runner: `bash` or `wsl` by bare name, or System32's copy.
fn reaches_wsl(program: &str) -> bool {
    let lower = program.to_lowercase();
    let base = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    let base = base.strip_suffix(".exe").unwrap_or(base);
    let bare = !lower.contains(['/', '\\']);
    (base == "bash" || base == "wsl") && (bare || lower.contains("system32"))
}

/// The programs a file starts through `Command::new` that reach WSL; whole-line `//` comments don't compile, doc
/// comments (doctests) do.
fn starts_wsl(text: &str) -> Vec<String> {
    let code: String = text
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            !l.starts_with("//") || l.starts_with("///") || l.starts_with("//!")
        })
        .map(|l| {
            let t = l.trim_start();
            t.strip_prefix("///")
                .or_else(|| t.strip_prefix("//!"))
                .unwrap_or(l)
        })
        .collect::<Vec<_>>()
        .join("\n");
    code.match_indices("Command::new(")
        .filter_map(|(i, m)| literal(&code[i + m.len()..]))
        .filter(|p| reaches_wsl(p))
        .collect()
}

/// Whether the file is left out of a Windows build by a file-level inner attribute, with a reason on the comment line
/// right above it. Only the opening run of comments, blank lines and inner attributes is read: an inner `cfg` deeper
/// in, inside a module, leaves out that module, not the file.
fn excluded_on_windows(text: &str) -> bool {
    let mut previous_comment = false;
    for line in text.lines().map(str::trim) {
        let squeezed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if [
            "#![cfg(unix)]",
            "#![cfg(not(windows))]",
            "#![cfg(not(target_os=\"windows\"))]",
            "#![cfg(target_family=\"unix\")]",
        ]
        .contains(&squeezed.as_str())
        {
            return previous_comment;
        }
        if line.starts_with("//") {
            previous_comment = line.trim_start_matches(['/', '!']).trim().len() > 1;
        } else if line.is_empty() || line.starts_with("#![") {
            previous_comment = false;
        } else {
            return false;
        }
    }
    false
}

fn wsl_findings(workflow_text: &str, nextest: &str, src: &Sources) -> Vec<String> {
    let mut files = reach(workflow_text, nextest, src);
    for f in src.files.keys().filter(|f| f.contains("/src/")) {
        files
            .entry(f.clone())
            .or_insert_with(|| "library code".to_owned());
    }
    let mut out = Vec::new();
    for (f, at) in &files {
        let text = &src.files[f];
        if excluded_on_windows(text) {
            continue;
        }
        for p in starts_wsl(text) {
            out.push(format!("{f} starts {p:?} (reached by {at})"));
        }
    }
    out
}

fn check_no_wsl(workflow_text: &str, nextest: &str, src: &Sources) {
    let found = wsl_findings(workflow_text, nextest, src);
    assert!(
        found.is_empty(),
        "qa: a test on the Windows runner starts WSL's launcher:\n{}",
        found.join("\n")
    );
}

// ---- REQ-TOOL-244: no `win-tests` --------------------------------------------------------------------------------

fn check_no_win_tests(text: &str) {
    let doc = workflow(text);
    let jobs = doc["jobs"].as_hash().expect("qa: win-gpu.yml has jobs");
    assert!(
        jobs.len() >= 3,
        "qa: win-gpu.yml has too few jobs to be read"
    );
    for (id, _) in jobs {
        assert_ne!(
            id.as_str(),
            Some("win-tests"),
            "qa: the win-tests job is on the runner"
        );
    }
    let options = doc["on"]["workflow_dispatch"]["inputs"]["only"]["options"]
        .as_vec()
        .expect("qa: the dispatch choice `only` has options");
    assert!(
        options.iter().all(|o| o.as_str() != Some("win-tests")),
        "qa: the dispatch choice still offers win-tests"
    );
    assert!(
        options.iter().any(|o| o.as_str() == Some("win-vulkan")),
        "qa: the dispatch choice offers no win-vulkan"
    );
}

#[test]
fn qa_m071_no_win_tests_job() {
    check_no_win_tests(&read(WORKFLOW));
}

negative_control!(
    qa_m071_no_win_tests_job,
    "win-tests put back as a job on the runner",
    expected = "qa: the win-tests job is on the runner",
    check_no_win_tests(&edited(
        &read(WORKFLOW),
        "  win-self-check:\n",
        "  win-tests:\n    runs-on: [self-hosted, windows, nvidia]\n    steps: []\n\n  win-self-check:\n"
    ))
);

mod qa_m071_option_back {
    use super::*;
    negative_control!(
        qa_m071_no_win_tests_job,
        "win-tests put back in the dispatch choice",
        expected = "qa: the dispatch choice still offers win-tests",
        check_no_win_tests(&edited(
            &read(WORKFLOW),
            "          - win-self-check\n",
            "          - win-tests\n          - win-self-check\n"
        ))
    );
}

// ---- REQ-TOOL-244: the GPU suites only: no runner test command runs a package or the workspace whole ----------------

fn check_gpu_only(text: &str, nextest: &str) {
    let mut tests = 0;
    for command in runner_cargo(text) {
        if command.words.get(1).map(String::as_str) == Some("build")
            && command
                .words
                .iter()
                .any(|w| w == "--workspace" || w == "--all-targets")
        {
            panic!("qa: {} builds the workspace on the runner", command.at);
        }
        let Some(s) = selection(&command) else {
            continue;
        };
        tests += 1;
        assert!(
            !s.whole_workspace,
            "qa: {} runs the workspace's tests on the runner",
            command.at
        );
        assert!(
            !s.packages.is_empty(),
            "qa: {} names no package",
            command.at
        );
        assert!(
            s.doc
                || !s.targets.is_empty()
                || !s.names.is_empty()
                || profile_list(nextest, s.profile.as_deref().unwrap_or("default")).is_some(),
            "qa: {} runs a package's tests whole on the runner",
            command.at
        );
    }
    assert!(
        tests >= 5,
        "qa: only {tests} test commands read from the runner's jobs"
    );
}

#[test]
fn qa_m071_runner_runs_gpu_suites_only() {
    check_gpu_only(&read(WORKFLOW), &read(NEXTEST));
}

#[cfg(feature = "controls")]
const GUARD_STEP: &str = "      - name: The guard on this workflow\n";

#[cfg(feature = "controls")]
fn with_step(command: &str) -> String {
    let text = read(WORKFLOW);
    assert!(text.contains(GUARD_STEP), "qa: the guard step is gone");
    text.replacen(
        GUARD_STEP,
        &format!("      - name: planted\n        run: |\n          {command}\n{GUARD_STEP}"),
        1,
    )
}

negative_control!(
    qa_m071_runner_runs_gpu_suites_only,
    "the whole-workspace test step back on the runner",
    expected = "runs the workspace's tests on the runner",
    check_gpu_only(
        &with_step("cargo nextest run --workspace --no-fail-fast --retries 0"),
        &read(NEXTEST)
    )
);

mod qa_m071_whole_package {
    use super::*;
    negative_control!(
        qa_m071_runner_runs_gpu_suites_only,
        "xtask's tests run whole on the runner",
        expected = "runs a package's tests whole on the runner",
        check_gpu_only(
            &with_step("cargo nextest run -p xtask --no-fail-fast"),
            &read(NEXTEST)
        )
    );
}

mod qa_m071_workspace_build {
    use super::*;
    negative_control!(
        qa_m071_runner_runs_gpu_suites_only,
        "the workspace's all-targets build back on the runner",
        expected = "builds the workspace on the runner",
        check_gpu_only(
            &with_step("cargo build --workspace --all-targets --keep-going"),
            &read(NEXTEST)
        )
    );
}

// ---- REQ-TOOL-244: what the runner reaches -------------------------------------------------------------------------

fn check_reach(text: &str, nextest: &str, src: &Sources) {
    let files = reach(text, nextest, src);
    for want in [
        "xtask/tests/ci_self_hosted.rs",
        "crates/engine/tests/canon_gpu.rs",
        "crates/engine/tests/decode_stage_gpu.rs",
        "crates/validation/tests/codegen_selftest.rs",
        "xtask/tests/build_kernel.rs",
    ] {
        assert!(
            files.contains_key(want),
            "qa: the runner's jobs do not reach {want}"
        );
    }
    for not in [
        "xtask/tests/jobq.rs",
        "xtask/tests/qa_R-658.rs",
        "xtask/tests/wsl.rs",
    ] {
        assert!(
            !files.contains_key(not),
            "qa: the runner's jobs reach {not}"
        );
    }
}

#[test]
fn qa_m071_reach_is_the_gpu_suites_and_the_guard() {
    check_reach(&read(WORKFLOW), &read(NEXTEST), &sources());
}

negative_control!(
    qa_m071_reach_is_the_gpu_suites_and_the_guard,
    "the gpu-kernel profile dropped from the GPU suites' step, which then runs three packages whole",
    expected = "qa: the runner's jobs reach xtask/tests/",
    check_reach(
        &edited(&read(WORKFLOW), "          NEXTEST_PROFILE: gpu-kernel\n", "          NEXTEST_PROFILE: none\n"),
        &read(NEXTEST),
        &sources()
    )
);

mod qa_m071_guard_gone {
    use super::*;
    negative_control!(
        qa_m071_reach_is_the_gpu_suites_and_the_guard,
        "win-self-check's guard step runs another target",
        expected = "qa: the runner's jobs do not reach xtask/tests/ci_self_hosted.rs",
        check_reach(
            &edited(
                &read(WORKFLOW),
                "--test ci_self_hosted",
                "--test build_kernel"
            ),
            &read(NEXTEST),
            &sources()
        )
    );
}

// ---- REQ-TOOL-244: no reached test, and no library, starts bash or wsl ---------------------------------------------

#[test]
fn qa_m071_nothing_reached_starts_wsl() {
    check_no_wsl(&read(WORKFLOW), &read(NEXTEST), &sources());
}

/// The tree with `addition` put at the top of `path` (a new file if there is none).
#[cfg(feature = "controls")]
fn with_text(path: &str, addition: &str) -> Sources {
    let mut src = sources();
    let text = src.files.entry(path.to_owned()).or_default();
    text.insert_str(0, addition);
    src
}

#[cfg(feature = "controls")]
fn planted_check(path: &str, addition: &str) {
    check_no_wsl(&read(WORKFLOW), &read(NEXTEST), &with_text(path, addition));
}

negative_control!(
    qa_m071_nothing_reached_starts_wsl,
    "a bare bash in the guard test win-self-check runs",
    expected = "xtask/tests/ci_self_hosted.rs starts \"bash\"",
    planted_check("xtask/tests/ci_self_hosted.rs", &spawn("\"bash\""))
);

mod qa_m071_raw_system32 {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "System32's wsl.exe as a raw string, the usual spelling of a Windows path, in a GPU target",
        expected =
            "crates/engine/tests/canon_gpu.rs starts \"C:\\\\Windows\\\\System32\\\\wsl.exe\"",
        planted_check(
            "crates/engine/tests/canon_gpu.rs",
            &spawn(r#"r"C:\Windows\System32\wsl.exe""#)
        )
    );
}

mod qa_m071_raw_hash_system32 {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "System32's bash.exe as a raw string with hashes, upper case, in a GPU target",
        expected = "starts \"C:\\\\WINDOWS\\\\SYSTEM32\\\\BASH.EXE\"",
        planted_check(
            "crates/engine/tests/encode_gpu.rs",
            &spawn(r###"r#"C:\WINDOWS\SYSTEM32\BASH.EXE"#"###)
        )
    );
}

mod qa_m071_slash_system32 {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "System32's bash by a forward-slash path with no extension, in the profile's xtask target",
        expected = "xtask/tests/build_kernel.rs starts \"C:/Windows/System32/bash\"",
        planted_check(
            "xtask/tests/build_kernel.rs",
            &spawn("\"C:/Windows/System32/bash\"")
        )
    );
}

mod qa_m071_bare_wsl_exe {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "wsl.exe by bare name in a validation GPU target",
        expected = "crates/validation/tests/codegen_selftest.rs starts \"wsl.exe\"",
        planted_check(
            "crates/validation/tests/codegen_selftest.rs",
            &spawn("\"wsl.exe\"")
        )
    );
}

mod qa_m071_split_call {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a bare bash with the literal on the line after `Command::new(`",
        expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
        planted_check(
            "crates/engine/tests/canon_gpu.rs",
            &format!(
                "fn _qa() {{ let _ = std::process::Command::{}(\n    \"bash\",\n); }}\n",
                "new"
            )
        )
    );
}

mod qa_m071_filter_selects_new_file {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a new xtask test file holding the name win-vulkan's golden filter selects, starting wsl",
        expected = "xtask/tests/qa_m071_planted.rs starts \"wsl\"",
        planted_check(
            "xtask/tests/qa_m071_planted.rs",
            &format!(
                "#[test]\nfn {}_planted() {{}}\n{}",
                ["golden", "halfway"].join("_"),
                spawn("\"wsl\"")
            )
        )
    );
}

mod qa_m071_in_module {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a bare bash in a folder module (tests/qa_helper/mod.rs) a reached target includes by `mod qa_helper;`",
        expected = "crates/engine/tests/qa_helper/mod.rs starts \"bash\"",
        check_no_wsl(&read(WORKFLOW), &read(NEXTEST), &{
            let mut src = with_text("crates/engine/tests/canon_gpu.rs", "mod qa_helper;\n");
            src.files.insert(
                "crates/engine/tests/qa_helper/mod.rs".to_owned(),
                spawn("\"bash\""),
            );
            src
        })
    );
}

mod qa_m071_in_library {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a bare bash in a library source of the kernel crate",
        expected = "crates/kernel/src/lib.rs starts \"bash\"",
        planted_check("crates/kernel/src/lib.rs", &spawn("\"bash\""))
    );
}

mod qa_m071_in_doctest {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a bare wsl in a doctest, which the runner's --doc step runs",
        expected = "crates/engine/src/lib.rs starts \"wsl\"",
        planted_check(
            "crates/engine/src/lib.rs",
            &format!("/// ```\n/// let _ = std::process::Command::{}(\"wsl\");\n/// ```\npub fn _qa() {{}}\n", "new")
        )
    );
}

mod qa_m071_no_reason {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a Windows exclusion with no reason above it",
        expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
        planted_check(
            "crates/engine/tests/canon_gpu.rs",
            &format!("#![cfg(unix)]\n{}", spawn("\"bash\""))
        )
    );
}

mod qa_m071_module_cfg_is_not_the_file {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "an inner cfg(unix) with its reason inside a nested module, while the file itself starts bash",
        expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
        planted_check(
            "crates/engine/tests/canon_gpu.rs",
            &format!(
                "{}mod qa_unix_only {{\n    // Needs a Unix shell; hosted Linux runs it.\n    #![cfg(unix)]\n}}\n",
                spawn("\"bash\"")
            )
        )
    );
}

mod qa_m071_unreadable_argument {
    use super::*;
    negative_control!(
        qa_m071_nothing_reached_starts_wsl,
        "a test argument on the runner qa can't read",
        expected = "an argument qa can't read: --ignored",
        check_no_wsl(
            &edited(
                &read(WORKFLOW),
                "--test ci_self_hosted --no-fail-fast",
                "--test ci_self_hosted --ignored --no-fail-fast"
            ),
            &read(NEXTEST),
            &sources()
        )
    );
}

/// What the requirement allows stays allowed: a file left out on Windows with its reason, `sh` (not WSL's), and Git's
/// bash by its path, each in a reached GPU target.
#[test]
fn qa_m071_allowed_forms_pass() {
    check_no_wsl(&read(WORKFLOW), &read(NEXTEST), &allowed());
}

fn allowed() -> Sources {
    let mut src = sources();
    let new = |s: &str| {
        format!(
            "fn _qa_{}() {{ let _ = std::process::Command::{}({s}); }}\n",
            s.len(),
            "new"
        )
    };
    src.files.get_mut("crates/engine/tests/canon_gpu.rs").expect("qa: canon_gpu.rs").insert_str(
        0,
        &format!(
            "//! A GPU target.\n\n// Starts bash, which the Windows runner must not; hosted Linux runs it.\n#![cfg(not(windows))]\n{}",
            new("\"bash\"")
        ),
    );
    src.files
        .get_mut("crates/engine/tests/encode_gpu.rs")
        .expect("qa: encode_gpu.rs")
        .insert_str(
            0,
            &format!(
                "{}{}",
                new("\"sh\""),
                new(r#"r"C:\Program Files\Git\bin\bash.exe""#)
            ),
        );
    src
}

negative_control!(
    qa_m071_allowed_forms_pass,
    "the same tree with canon_gpu.rs's exclusion lifted",
    expected = "crates/engine/tests/canon_gpu.rs starts \"bash\"",
    check_no_wsl(&read(WORKFLOW), &read(NEXTEST), &{
        let mut src = allowed();
        let text = src
            .files
            .get_mut("crates/engine/tests/canon_gpu.rs")
            .expect("qa: canon_gpu.rs");
        *text = text.replacen("#![cfg(not(windows))]\n", "", 1);
        src
    })
);
