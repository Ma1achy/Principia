//! R-346 (the human's (B) and (C)): `scripts/cloud-setup.sh` installs exactly what CI's Linux jobs install, and reads
//! every pin from the files CI reads, never from a copy of its own, so that it cannot drift from CI.
//!
//! The check reads CI's Linux jobs (`runs-on: ubuntu-*`) and the root toolchain file itself, here, into a plan of
//! `(kind, name, version)` items, and compares it with the plan the script prints under `--dry-run`:
//! - the two name the same items (no tool CI installs that the script doesn't, nor the other way);
//! - each item at the same version;
//! - the script's text holds no version literal, and none of CI's pinned versions;
//! - the script's dry run follows a changed pin in the files it reads;
//! - the script refuses a CI step that installs by a means it doesn't know, as this check does;
//! - R-347: cargo-nextest comes from its official prebuilt installer (get.nexte.st) and cargo-mutants through
//!   cargo-binstall, each at CI's pin, and each falls back to `cargo install --locked` only when its download fails. The
//!   dry run names that route, and the script's install function, run with `curl`, `tar`, `cargo` and `uname` stubbed,
//!   takes it;
//! - under `pipefail`, the script pipes into no reader that can quit before its input ends (`head`, `grep -q`, `-m`,
//!   `-l`): the writer would die of SIGPIPE and fail the pipeline at random.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use validation::spawn::Spawn;

/// One install item: its kind, its name, and its version (`-` where it has none).
type Item = (String, String, String);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The workflows under `<root>/.github/workflows/`, as (file name, text), sorted by name.
fn workflows(root: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = fs::read_dir(root.join(".github/workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(&path).unwrap())
        })
        .collect();
    found.sort();
    found
}

/// The root toolchain file's text, as CI's bare `rustup toolchain install` reads it, if there is one.
fn toolchain_file(root: &Path) -> Option<String> {
    ["rust-toolchain.toml", "rust-toolchain"]
        .iter()
        .find_map(|name| fs::read_to_string(root.join(name)).ok())
}

fn script(root: &Path) -> String {
    fs::read_to_string(root.join("scripts/cloud-setup.sh")).unwrap()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches(|c| c == '"' || c == '\'')
}

/// The jobs of `workflow` that run on Linux, each as its lines. A job on a runner that is neither Linux, macOS nor
/// Windows fails the check, since it can't tell what that job is.
fn linux_jobs(file: &str, workflow: &str) -> Vec<Vec<String>> {
    let mut jobs: Vec<Vec<String>> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match indent(line) {
            0 => in_jobs = trimmed == "jobs:",
            2 if in_jobs => jobs.push(Vec::new()),
            _ if in_jobs => jobs.last_mut().unwrap().push(line.to_owned()),
            _ => {}
        }
    }
    jobs.into_iter()
        .filter(|job| {
            let runner = job
                .iter()
                .find_map(|l| l.trim().strip_prefix("runs-on:"))
                .map(unquote)
                .unwrap_or_default();
            assert!(
                ["ubuntu-", "macos-", "windows-"]
                    .iter()
                    .any(|os| runner.starts_with(os)),
                "{file}: a job runs on `{runner}`, which this check can't place as Linux or not"
            );
            runner.starts_with("ubuntu-")
        })
        .collect()
}

/// The workflow's top-level `env:` value of `PRIN_GPU_BACKEND`, which each of its jobs inherits. A top-level `env:`
/// written inline that names it fails the check, since it is not read.
fn workflow_backend(file: &str, workflow: &str) -> Option<String> {
    let mut in_env = false;
    let mut found = None;
    for line in workflow.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if indent(line) == 0 {
            in_env = trimmed == "env:";
            assert!(
                in_env || !trimmed.starts_with("env:") || !trimmed.contains("PRIN_GPU_BACKEND"),
                "{file}: a Linux job installs through a step this check doesn't know: `{trimmed}`"
            );
        } else if in_env {
            if let Some(value) = trimmed.strip_prefix("PRIN_GPU_BACKEND:") {
                found = Some(unquote(value).to_owned());
            }
        }
    }
    found
}

/// The packages a shell command installs after `key` (`apt-get install`, `pip install`): its words up to the end of
/// the command, less options.
fn installed_after(command: &str, key: &str) -> Vec<String> {
    let rest = &command[command.find(key).unwrap() + key.len()..];
    rest.split_whitespace()
        .take_while(|w| !matches!(*w, "&&" | "||" | ";" | "|" | "\\"))
        .filter(|w| !w.starts_with('-'))
        .map(str::to_owned)
        .collect()
}

/// The actions CI uses that install nothing a cloud machine needs: checkout, caches and artifacts.
const NO_INSTALL: &[&str] = &[
    "actions/checkout@",
    "actions/cache@",
    "actions/cache/restore@",
    "actions/cache/save@",
    "actions/upload-artifact@",
    "actions/download-artifact@",
    "Swatinem/rust-cache@",
];

/// What a shell command in a Linux job installs, into `plan`. A command that installs by a means the check doesn't
/// know fails it.
fn scan_command(file: &str, command: &str, plan: &mut BTreeSet<Item>) {
    let command = command.trim();
    if command.is_empty() || command.starts_with('#') {
        return;
    }
    let item = |kind: &str, name: String| (kind.to_owned(), name, "-".to_owned());
    if command.contains("apt-get install") {
        plan.extend(
            installed_after(command, "apt-get install")
                .into_iter()
                .map(|p| item("apt", p)),
        );
    } else if command.contains("pip install") {
        plan.extend(
            installed_after(command, "pip install")
                .into_iter()
                .map(|p| item("pip", p)),
        );
    } else if command
        .split(['&', ';', '|'])
        .any(|part| part.trim() == "rustup toolchain install")
    {
        // The root toolchain file's toolchain, which `ci_plan` adds from the file itself.
    } else {
        let unknown = [
            "cargo install",
            "cargo binstall",
            "rustup component add",
            "rustup target add",
            "rustup toolchain install",
            "apt install",
            "snap install",
            "brew install",
            "npm install",
            "npm ci",
            "curl ",
            "wget ",
        ];
        assert!(
            !unknown.iter().any(|u| command.contains(u)),
            "{file}: a Linux job installs through a step this check doesn't know: `{command}`"
        );
    }
}

/// What CI's Linux jobs in `workflows` install, with the toolchain `toolchain` (the root toolchain file) pins.
fn ci_plan(workflows: &[(String, String)], toolchain: Option<&str>) -> BTreeSet<Item> {
    let mut plan: BTreeSet<Item> = BTreeSet::new();
    let item = |kind: &str, name: &str, version: &str| {
        (kind.to_owned(), name.to_owned(), version.to_owned())
    };
    for (file, text) in workflows {
        let jobs = linux_jobs(file, text);
        if let Some(backend) = workflow_backend(file, text).filter(|_| !jobs.is_empty()) {
            plan.insert(item("env", "PRIN_GPU_BACKEND", &backend));
        }
        for job in jobs {
            // The ref of the `dtolnay/rust-toolchain` step being read, the indent of the `run:` block being read, and
            // the command a `\`-ended line of that block continues.
            let mut action_ref: Option<String> = None;
            let mut run_block: Option<usize> = None;
            let mut continued = String::new();
            for line in job.iter().map(String::as_str).chain([""]) {
                let mut key = line.trim();
                if let Some(at) = run_block {
                    if !line.is_empty() && indent(line) > at {
                        match key.strip_suffix('\\') {
                            Some(head) => {
                                continued.push_str(head);
                                continued.push(' ');
                            }
                            None => {
                                scan_command(file, &format!("{continued}{key}"), &mut plan);
                                continued.clear();
                            }
                        }
                        continue;
                    }
                    if !continued.is_empty() {
                        scan_command(file, &continued, &mut plan);
                        continued.clear();
                    }
                    run_block = None;
                }
                if line.is_empty() {
                    break;
                }
                if key.starts_with('#') {
                    continue;
                }
                if let Some(rest) = key.strip_prefix("- ") {
                    action_ref = None;
                    key = rest.trim();
                }
                if let Some(action) = key.strip_prefix("uses:").map(unquote) {
                    if let Some(r) = action.strip_prefix("dtolnay/rust-toolchain@") {
                        plan.insert(item("toolchain", "action", r));
                        plan.insert(item("profile", r, "minimal"));
                        action_ref = Some(r.to_owned());
                    } else {
                        assert!(
                            NO_INSTALL.iter().any(|a| action.starts_with(a))
                                || action.starts_with("taiki-e/install-action@")
                                || action.starts_with("actions/setup-python@"),
                            "{file}: a Linux job installs through a step this check doesn't know: `{key}`"
                        );
                    }
                } else if let Some(list) = key.strip_prefix("components:") {
                    let r = action_ref.as_deref().unwrap_or_else(|| {
                        panic!("{file}: `{key}` outside a `dtolnay/rust-toolchain` step")
                    });
                    for c in unquote(list).split([',', ' ']).filter(|c| !c.is_empty()) {
                        plan.insert(item("component", &format!("{r}/{c}"), "-"));
                    }
                } else if let Some(tools) = key.strip_prefix("tool:") {
                    for tool in unquote(tools).split([',', ' ']).filter(|t| !t.is_empty()) {
                        let (name, version) = tool.split_once('@').unwrap_or((tool, "-"));
                        plan.insert(item("cargo-tool", name, version));
                    }
                } else if let Some(version) = key.strip_prefix("python-version:") {
                    plan.insert(item("python", "python", unquote(version)));
                } else if let Some(backend) = key.strip_prefix("PRIN_GPU_BACKEND:") {
                    plan.insert(item("env", "PRIN_GPU_BACKEND", unquote(backend)));
                } else if let Some(command) = key.strip_prefix("run:") {
                    let command = command.trim();
                    if ["|", ">", "|-", ">-", "|+", ">+"].contains(&command) {
                        run_block = Some(indent(line));
                    } else {
                        assert!(
                            !command.ends_with('\\'),
                            "{file}: a Linux job installs through a step this check doesn't know: `{key}`"
                        );
                        scan_command(file, command, &mut plan);
                    }
                }
            }
        }
    }
    if let Some(text) = toolchain {
        plan.extend(toolchain_items(text));
    }
    plan
}

/// The items a root toolchain file pins: its channel, profile, components and targets. A key the check doesn't know
/// fails it.
fn toolchain_items(text: &str) -> Vec<Item> {
    let item =
        |kind: &str, name: String, version: &str| (kind.to_owned(), name, version.to_owned());
    if !text.contains('=') {
        let channel = text.trim().to_owned();
        return vec![
            item("toolchain", "file".to_owned(), &channel),
            item("profile", channel, "-"),
        ];
    }
    let doc: toml_edit::DocumentMut = text.parse().expect("the root toolchain file is TOML");
    let table = doc["toolchain"]
        .as_table()
        .expect("the root toolchain file has a [toolchain] table");
    let mut items = Vec::new();
    let channel = table["channel"]
        .as_str()
        .expect("the root toolchain file names a channel")
        .to_owned();
    items.push(item("toolchain", "file".to_owned(), &channel));
    let profile = table.get("profile").and_then(|p| p.as_str()).unwrap_or("-");
    items.push(item("profile", channel.clone(), profile));
    for (key, value) in table.iter() {
        let kind = match key {
            "channel" | "profile" => continue,
            "components" => "component",
            "targets" => "target",
            other => panic!("the root toolchain file has a key this check doesn't know: `{other}`"),
        };
        for entry in value.as_array().expect("a list").iter() {
            let name = format!("{channel}/{}", entry.as_str().unwrap());
            items.push(item(kind, name, "-"));
        }
    }
    items
}

/// Runs `bash scripts/cloud-setup.sh --dry-run` in the tree at `root`.
fn run_dry(root: &Path) -> Output {
    Command::new("bash")
        .arg(root.join("scripts/cloud-setup.sh"))
        .arg("--dry-run")
        .timed_output()
        .expect("run bash")
}

/// The lines the script prints under `--dry-run` in the tree at `root`, which must exit well to give them, each as its
/// item and its install method.
fn script_lines(root: &Path) -> Vec<(Item, String)> {
    let output = run_dry(root);
    assert!(
        output.status.success(),
        "cloud-setup.sh --dry-run failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.starts_with("smoke "))
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                words.len(),
                4,
                "a plan line is `<kind> <name> <version> <method>`: `{line}`"
            );
            (
                (
                    words[0].to_owned(),
                    words[1].to_owned(),
                    words[2].to_owned(),
                ),
                words[3].to_owned(),
            )
        })
        .collect()
}

/// The plan the script prints under `--dry-run` in the tree at `root`.
fn script_plan(root: &Path) -> BTreeSet<Item> {
    script_lines(root)
        .into_iter()
        .map(|(item, _)| item)
        .collect()
}

/// `plan`'s items by (kind, name), each with its versions.
fn by_name(plan: &BTreeSet<Item>) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut found: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (kind, name, version) in plan {
        found
            .entry((kind.clone(), name.clone()))
            .or_default()
            .insert(version.clone());
    }
    found
}

/// The script names exactly the items CI does.
fn check_same_items(ci: &BTreeSet<Item>, script: &BTreeSet<Item>) {
    let (ci, script) = (by_name(ci), by_name(script));
    let ci_only: Vec<_> = ci.keys().filter(|k| !script.contains_key(*k)).collect();
    let script_only: Vec<_> = script.keys().filter(|k| !ci.contains_key(*k)).collect();
    assert!(
        !ci.is_empty() && ci_only.is_empty() && script_only.is_empty(),
        "cloud-setup.sh does not install exactly what CI's Linux jobs install: CI only {ci_only:?}; the script only \
         {script_only:?}"
    );
}

/// Each item both name is at the same version in each.
fn check_same_versions(ci: &BTreeSet<Item>, script: &BTreeSet<Item>) {
    let script = by_name(script);
    let differ: Vec<_> = by_name(ci)
        .into_iter()
        .filter_map(|(key, versions)| {
            let theirs = script.get(&key)?;
            (theirs != &versions)
                .then(|| format!("{key:?}: CI {versions:?}, the script {theirs:?}"))
        })
        .collect();
    assert!(
        differ.is_empty(),
        "cloud-setup.sh and CI disagree on a version: {differ:?}"
    );
}

/// The script's text holds no version literal: no `<digit>.<digit>`, no `@<digit>`, no `nightly-<digit>`, no date,
/// and none of the versions CI pins (toolchains, cargo tools, Python).
fn check_no_version(text: &str, ci: &BTreeSet<Item>) {
    let bytes = text.as_bytes();
    let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let mut found: Vec<String> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        let literal = (b == b'.' && i > 0 && digit(i - 1) && digit(i + 1))
            || (b == b'@' && digit(i + 1))
            || (bytes[i..].starts_with(b"nightly-") && digit(i + 8))
            || (b == b'-'
                && (1..=4).all(|k| i >= k && digit(i - k))
                && digit(i + 1)
                && digit(i + 2));
        if literal {
            let line = text[..i].lines().count().max(1);
            found.push(format!(
                "line {line}: `{}`",
                text.lines().nth(line - 1).unwrap_or("").trim()
            ));
        }
    }
    for (kind, _, version) in ci {
        if matches!(kind.as_str(), "toolchain" | "cargo-tool" | "python")
            && version != "-"
            && text.contains(version.as_str())
        {
            found.push(format!("CI's {kind} version `{version}`"));
        }
    }
    assert!(
        found.is_empty(),
        "cloud-setup.sh hard-codes a version: {found:?}"
    );
}

/// A scratch folder of its own for `case`, emptied first. The test and its control name different cases (R-333).
fn scratch(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("cloud_setup_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::create_dir_all(dir.join("scripts")).unwrap();
    dir
}

/// A copy, at `dir`, of what the script reads: itself, CI's workflows (each through `edit`) and the root toolchain
/// file (or `toolchain` in its place, when given).
fn copy_tree(dir: &Path, edit: impl Fn(&str) -> String, toolchain: Option<&str>) {
    let root = root();
    fs::write(dir.join("scripts/cloud-setup.sh"), script(&root)).unwrap();
    for (name, text) in workflows(&root) {
        fs::write(dir.join(".github/workflows").join(name), edit(&text)).unwrap();
    }
    if let Some(text) = toolchain
        .map(str::to_owned)
        .or_else(|| toolchain_file(&root))
    {
        fs::write(dir.join("rust-toolchain.toml"), text).unwrap();
    }
}

fn the_ci_plan() -> BTreeSet<Item> {
    let root = root();
    ci_plan(&workflows(&root), toolchain_file(&root).as_deref())
}

#[test]
fn cloud_setup_installs_each_item_ci_s_linux_jobs_install() {
    check_same_items(&the_ci_plan(), &script_plan(&root()));
}

validation::negative_control!(
    cloud_setup_installs_each_item_ci_s_linux_jobs_install,
    "a CI whose lavapipe step installs one more apt package, required to match the script's plan",
    expected = "cloud-setup.sh does not install exactly what CI's Linux jobs install",
    {
        let root = root();
        let more: Vec<(String, String)> = workflows(&root)
            .into_iter()
            .map(|(name, text)| {
                (
                    name,
                    text.replace("mesa-vulkan-drivers", "mesa-vulkan-drivers xvfb"),
                )
            })
            .collect();
        check_same_items(
            &ci_plan(&more, toolchain_file(&root).as_deref()),
            &script_plan(&root),
        )
    }
);

#[test]
fn cloud_setup_installs_each_item_at_ci_s_version() {
    check_same_versions(&the_ci_plan(), &script_plan(&root()));
}

validation::negative_control!(
    cloud_setup_installs_each_item_at_ci_s_version,
    "a CI pinning another cargo-nextest version, required to match the script's plan",
    expected = "cloud-setup.sh and CI disagree on a version",
    {
        let root = root();
        let bumped: Vec<(String, String)> = workflows(&root)
            .into_iter()
            .map(|(name, text)| (name, text.replace("cargo-nextest@", "cargo-nextest@1")))
            .collect();
        check_same_versions(
            &ci_plan(&bumped, toolchain_file(&root).as_deref()),
            &script_plan(&root),
        )
    }
);

#[test]
fn cloud_setup_hard_codes_no_version() {
    check_no_version(&script(&root()), &the_ci_plan());
}

validation::negative_control!(
    cloud_setup_hard_codes_no_version,
    "a script naming CI's cargo-nextest pin, required to hold no version",
    expected = "cloud-setup.sh hard-codes a version",
    {
        let ci = the_ci_plan();
        let pin = ci
            .iter()
            .find(|(kind, name, _)| kind == "cargo-tool" && name == "cargo-nextest")
            .map(|(_, _, version)| version.clone())
            .unwrap();
        let text = script(&root()).replace(
            "set -euo pipefail",
            &format!("set -euo pipefail\nNEXTEST={pin}"),
        );
        check_no_version(&text, &ci)
    }
);

/// The pins the copy at `dir` was given: cargo-nextest's raised, and a toolchain file of its own.
const COPY_TOOLCHAIN: &str =
    "[toolchain]\nchannel = \"nightly-2001-02-03\"\ncomponents = [\"rust-src\"]\n";

fn bump(text: &str) -> String {
    text.replace("cargo-nextest@", "cargo-nextest@1")
}

/// The dry run of the tree at `run_in` reports the pins of the copy at `copy`, which differ from the repository's.
fn check_follows(copy: &Path, run_in: &Path) {
    let wanted = ci_plan(&workflows(copy), toolchain_file(copy).as_deref());
    assert!(
        wanted.contains(&(
            "toolchain".into(),
            "file".into(),
            "nightly-2001-02-03".into()
        )) && wanted != the_ci_plan(),
        "the copy's pins are not the changed ones"
    );
    assert!(
        script_plan(run_in) == wanted,
        "cloud-setup.sh's dry run does not follow the pins in the files it reads"
    );
    fs::remove_dir_all(copy).unwrap();
}

#[test]
fn cloud_setup_reads_its_pins_from_ci_s_files() {
    let copy = scratch("follows");
    copy_tree(&copy, bump, Some(COPY_TOOLCHAIN));
    check_follows(&copy, &copy);
}

validation::negative_control!(
    cloud_setup_reads_its_pins_from_ci_s_files,
    "the repository's own dry run, required to report the copy's changed pins",
    expected = "cloud-setup.sh's dry run does not follow the pins in the files it reads",
    {
        let copy = scratch("follows_control");
        copy_tree(&copy, bump, Some(COPY_TOOLCHAIN));
        check_follows(&copy, &root())
    }
);

/// The dry run of the copy at `dir` refuses it, naming a step it doesn't know.
fn check_refused(dir: &Path) {
    let output = run_dry(dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && stderr.contains("by a step this script doesn't know"),
        "cloud-setup.sh accepted a CI install step it doesn't know:\n{stderr}"
    );
    fs::remove_dir_all(dir).unwrap();
}

/// Adds a step installing through an action the script doesn't know to the first Linux job's steps.
fn add_unknown_step(text: &str) -> String {
    match text.find("    runs-on: ubuntu-") {
        Some(at) => {
            let steps = at + text[at..].find("    steps:\n").unwrap() + "    steps:\n".len();
            format!(
                "{}      - uses: some-owner/setup-something@v1\n{}",
                &text[..steps],
                &text[steps..]
            )
        }
        None => text.to_owned(),
    }
}

#[test]
fn cloud_setup_refuses_an_install_step_it_does_not_know() {
    let copy = scratch("refuses");
    copy_tree(&copy, add_unknown_step, None);
    check_refused(&copy);
}

validation::negative_control!(
    cloud_setup_refuses_an_install_step_it_does_not_know,
    "the workflows unchanged, required to be refused",
    expected = "cloud-setup.sh accepted a CI install step it doesn't know",
    {
        let copy = scratch("refuses_control");
        copy_tree(&copy, str::to_owned, None);
        check_refused(&copy)
    }
);

/// The one-line lavapipe step CI's Linux jobs use, and the same step as a `run: |` block whose `apt-get install`
/// continues, after a `\`, onto a line naming one more package.
const LAVAPIPE_LINE: &str =
    "        run: sudo apt-get update && sudo apt-get install -y mesa-vulkan-drivers\n";
const LAVAPIPE_CONTINUED: &str = "        run: |\n          sudo apt-get update\n          sudo apt-get install -y \\\n            mesa-vulkan-drivers xvfb\n";

/// CI's value of `PRIN_GPU_BACKEND` for its Linux jobs.
fn ci_backend() -> String {
    the_ci_plan()
        .into_iter()
        .find(|(kind, _, _)| kind == "env")
        .map(|(_, _, value)| value)
        .expect("CI's Linux jobs set PRIN_GPU_BACKEND")
}

/// `text` with its lavapipe step continued onto a second line (`LAVAPIPE_CONTINUED`), and its jobs' and steps'
/// `PRIN_GPU_BACKEND: <CI's value>` moved to the workflow's top-level `env:`.
fn continue_and_lift(text: &str) -> String {
    let backend = format!("PRIN_GPU_BACKEND: {}", ci_backend());
    let text = text.replace(LAVAPIPE_LINE, LAVAPIPE_CONTINUED);
    if !text.lines().any(|l| indent(l) > 0 && l.trim() == backend) {
        return text;
    }
    let kept: String = text
        .lines()
        .filter(|l| l.trim() != backend)
        .map(|l| format!("{l}\n"))
        .collect();
    match kept.find("\nenv:\n") {
        Some(at) => format!("{}  {backend}\n{}", &kept[..at + 6], &kept[at + 6..]),
        None => kept.replacen("\njobs:\n", &format!("\nenv:\n  {backend}\njobs:\n"), 1),
    }
}

/// The dry run of the tree at `run_in` installs what the copy at `copy` installs, whose lavapipe step continues onto a
/// second line and whose PRIN_GPU_BACKEND is set only at its workflows' top level.
fn check_reads_continued_and_lifted(copy: &Path, run_in: &Path) {
    let wanted = ci_plan(&workflows(copy), toolchain_file(copy).as_deref());
    let backend = format!("PRIN_GPU_BACKEND: {}", ci_backend());
    let job_level = workflows(copy)
        .iter()
        .any(|(_, text)| text.lines().any(|l| indent(l) > 2 && l.trim() == backend));
    assert!(
        wanted.contains(&("apt".into(), "xvfb".into(), "-".into()))
            && wanted.contains(&("env".into(), "PRIN_GPU_BACKEND".into(), ci_backend()))
            && !job_level,
        "the copy's continued line and top-level PRIN_GPU_BACKEND are not the changed ones"
    );
    assert!(
        script_plan(run_in) == wanted,
        "cloud-setup.sh's dry run misses a continued install line or the workflow-level PRIN_GPU_BACKEND"
    );
    fs::remove_dir_all(copy).unwrap();
}

#[test]
fn cloud_setup_reads_continued_lines_and_the_workflow_env() {
    let copy = scratch("continued");
    copy_tree(&copy, continue_and_lift, None);
    check_reads_continued_and_lifted(&copy, &copy);
}

validation::negative_control!(
    cloud_setup_reads_continued_lines_and_the_workflow_env,
    "the repository's own dry run, required to report the copy's continued package",
    expected = "cloud-setup.sh's dry run misses a continued install line or the workflow-level PRIN_GPU_BACKEND",
    {
        let copy = scratch("continued_control");
        copy_tree(&copy, continue_and_lift, None);
        check_reads_continued_and_lifted(&copy, &root())
    }
);

/// The dry run of a copy of the tree under a folder whose name has a space, `dir`, exits well and gives CI's plan.
fn check_runs_under_a_space(dir: &Path) {
    let output = run_dry(dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        dir.to_string_lossy().contains(' ')
            && output.status.success()
            && script_plan(dir) == the_ci_plan(),
        "cloud-setup.sh's dry run fails from a checkout whose path has a space:\n{stderr}"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cloud_setup_runs_from_a_path_with_a_space() {
    let copy = scratch("sp ace");
    copy_tree(&copy, str::to_owned, None);
    check_runs_under_a_space(&copy);
}

validation::negative_control!(
    cloud_setup_runs_from_a_path_with_a_space,
    "a script passing the workflow files to awk word-split, required to run from a path with a space",
    expected = "cloud-setup.sh's dry run fails from a checkout whose path has a space",
    {
        let copy = scratch("sp ace_control");
        copy_tree(&copy, str::to_owned, None);
        let quoted = "' \"${files[@]}\" | sort -u";
        let text = script(&root());
        assert!(text.contains(quoted), "the script passes the files as `{quoted}`");
        fs::write(
            copy.join("scripts/cloud-setup.sh"),
            text.replace(quoted, "' ${files[*]} | sort -u"),
        )
        .unwrap();
        check_runs_under_a_space(&copy)
    }
);

/// The route R-347 gives cargo-nextest: its official prebuilt installer, then `cargo install --locked`.
const NEXTEST_ROUTE: &str = "prebuilt:get.nexte.st,fallback:cargo-install";
/// The route R-347 gives cargo-mutants: cargo-binstall (prebuilt), then `cargo install --locked`.
const MUTANTS_ROUTE: &str = "prebuilt:cargo-binstall,fallback:cargo-install";

/// How one stubbed run of the script's `install_cargo_tool` goes: whether `curl` succeeds, whether `cargo binstall`
/// succeeds, and whether `cargo-binstall` is on the machine already.
struct Stubs {
    curl_ok: bool,
    binstall_ok: bool,
    have_binstall: bool,
}

/// Sources the script at `script` and runs `install_cargo_tool <tool> <version>` with `curl`, `tar`, `cargo`, `uname`
/// (Linux x86_64) and, if `stubs.have_binstall`, `cargo-binstall` stubbed as shell functions that log their arguments.
/// PATH is the system's alone, so no real cargo-binstall is found. Returns whether it exited well, and the log.
fn stubbed_install(
    script: &Path,
    dir: &Path,
    tool: &str,
    version: &str,
    stubs: &Stubs,
) -> (bool, Vec<String>) {
    let log = dir.join("log");
    let _ = fs::remove_file(&log);
    // Sourcing must leave the caller's shell options alone (no errexit, no pipefail), whatever its arguments.
    let body = r#"s=$1 t=$2 v=$3
. "$s"
case $- in *e*) echo "sourcing set errexit" >>"$LOG"; exit 1 ;; esac
if shopt -oq pipefail; then echo "sourcing set pipefail" >>"$LOG"; exit 1; fi
uname() { case "$1" in -m) echo x86_64 ;; *) echo Linux ;; esac; }
curl() { echo "curl $*" >>"$LOG"; [ "$CURL_OK" = 1 ]; }
tar() { echo "tar $*" >>"$LOG"; cat >/dev/null; }
cargo() { echo "cargo $*" >>"$LOG"; if [ "$1" = binstall ]; then [ "$BINSTALL_OK" = 1 ]; fi; }
if [ "$HAVE_BINSTALL" = 1 ]; then cargo-binstall() { :; }; fi
install_cargo_tool "$t" "$v""#;
    let flag = |b: bool| if b { "1" } else { "0" };
    let output = Command::new("bash")
        .arg("-c")
        .arg(body)
        .arg("stubbed")
        .arg(script)
        .arg(tool)
        .arg(version)
        .env("PATH", "/usr/bin:/bin")
        .env("CARGO_HOME", dir.join("cargo"))
        .env("LOG", &log)
        .env("CURL_OK", flag(stubs.curl_ok))
        .env("BINSTALL_OK", flag(stubs.binstall_ok))
        .env("HAVE_BINSTALL", flag(stubs.have_binstall))
        .timed_output()
        .expect("run bash");
    let lines = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    (output.status.success(), lines)
}

/// CI's pinned version of cargo tool `tool`.
fn ci_pin(ci: &BTreeSet<Item>, tool: &str) -> String {
    ci.iter()
        .find(|(kind, name, _)| kind == "cargo-tool" && name == tool)
        .map(|(_, _, version)| version.clone())
        .unwrap_or_else(|| panic!("CI's Linux jobs install no {tool}"))
}

/// The tree at `tree` installs cargo-nextest and cargo-mutants by R-347's routes: its dry run names them, and its
/// `install_cargo_tool` downloads each prebuilt at CI's pin, falling back to `cargo install --locked` only when the
/// download fails.
fn check_prebuilt_routes(tree: &Path, scratch_dir: &Path, ci: &BTreeSet<Item>) {
    let mut wrong: Vec<String> = Vec::new();
    let methods: BTreeMap<String, String> = script_lines(tree)
        .into_iter()
        .filter(|((kind, _, _), _)| kind == "cargo-tool")
        .map(|((_, name, _), method)| (name, method))
        .collect();
    for (tool, route) in [
        ("cargo-nextest", NEXTEST_ROUTE),
        ("cargo-mutants", MUTANTS_ROUTE),
    ] {
        if methods.get(tool).map(String::as_str) != Some(route) {
            wrong.push(format!(
                "the dry run gives {tool} {:?}, not `{route}`",
                methods.get(tool)
            ));
        }
    }
    let script = tree.join("scripts/cloud-setup.sh");
    let mut case = |what: &str, tool: &str, stubs: Stubs, want: &dyn Fn(&[String]) -> bool| {
        let version = ci_pin(ci, tool);
        let (ok, log) = stubbed_install(&script, scratch_dir, tool, &version, &stubs);
        if !ok || !want(&log) {
            wrong.push(format!("{tool}, {what}: exited well {ok}, ran {log:?}"));
        }
    };
    let nextest = ci_pin(ci, "cargo-nextest");
    let mutants = ci_pin(ci, "cargo-mutants");
    let nextest_url = format!("https://get.nexte.st/{nextest}/linux");
    // The fallback, the last command run, and only after the download failed.
    let nextest_built = format!("cargo install --locked cargo-nextest@{nextest}");
    let mutants_built = format!("cargo install --locked cargo-mutants@{mutants}");
    let binstall =
        format!("cargo binstall --no-confirm --disable-strategies compile cargo-mutants@{mutants}");
    case(
        "the download succeeds",
        "cargo-nextest",
        Stubs {
            curl_ok: true,
            binstall_ok: true,
            have_binstall: true,
        },
        &|log| {
            log.iter()
                .any(|l| l.starts_with("curl ") && l.ends_with(&nextest_url))
                && log.iter().any(|l| l.starts_with("tar zxf - -C "))
                && !log.iter().any(|l| l.starts_with("cargo install"))
        },
    );
    case(
        "the download fails",
        "cargo-nextest",
        Stubs {
            curl_ok: false,
            binstall_ok: true,
            have_binstall: true,
        },
        &|log| {
            log.iter()
                .any(|l| l.starts_with("curl ") && l.ends_with(&nextest_url))
                && log.last() == Some(&nextest_built)
        },
    );
    case(
        "cargo-binstall present, the download succeeds",
        "cargo-mutants",
        Stubs {
            curl_ok: true,
            binstall_ok: true,
            have_binstall: true,
        },
        &|log| log == [binstall.clone()],
    );
    case(
        "cargo-binstall present, the download fails",
        "cargo-mutants",
        Stubs {
            curl_ok: true,
            binstall_ok: false,
            have_binstall: true,
        },
        &|log| log == [binstall.clone(), mutants_built.clone()],
    );
    case(
        "cargo-binstall missing",
        "cargo-mutants",
        Stubs {
            curl_ok: true,
            binstall_ok: true,
            have_binstall: false,
        },
        &|log| {
            log.len() == 2
                && log[0].starts_with("curl ")
                && log[0].contains("https://raw.githubusercontent.com/cargo-bins/cargo-binstall/")
                && log[1] == binstall
        },
    );
    assert!(
        wrong.is_empty(),
        "cloud-setup.sh does not install cargo-nextest and cargo-mutants prebuilt, with `cargo install --locked` only \
         when the download fails: {wrong:#?}"
    );
}

#[test]
fn cloud_setup_downloads_nextest_and_mutants_prebuilt_with_a_cargo_install_fallback() {
    let dir = scratch("prebuilt");
    check_prebuilt_routes(&root(), &dir, &the_ci_plan());
    fs::remove_dir_all(&dir).unwrap();
}

validation::negative_control!(
    cloud_setup_downloads_nextest_and_mutants_prebuilt_with_a_cargo_install_fallback,
    "a script that builds cargo-nextest with `cargo install --locked` straight away, required to download it first",
    expected = "cloud-setup.sh does not install cargo-nextest and cargo-mutants prebuilt",
    {
        let copy = scratch("prebuilt_control");
        copy_tree(&copy, str::to_owned, None);
        let route = format!("cargo-nextest) echo \"{NEXTEST_ROUTE}\" ;;");
        let text = script(&root());
        assert!(text.contains(&route), "the script names cargo-nextest's route as `{route}`");
        fs::write(
            copy.join("scripts/cloud-setup.sh"),
            text.replace(&route, "cargo-nextest) echo \"cargo-install\" ;;"),
        )
        .unwrap();
        check_prebuilt_routes(&copy, &copy, &the_ci_plan())
    }
);

/// The pipes in `text` whose reader can quit before reading all its input: `head`, or `grep` with `-q`, `-m`, `-l` or
/// their long forms. Under `pipefail` the writer then dies of SIGPIPE whenever it writes after the reader has gone, and
/// the pipeline fails at random (seen on Linux bash 5, where `printf` writes line by line).
fn early_exit_readers(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found: Vec<String> = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'|' || (i > 0 && bytes[i - 1] == b'|') || bytes.get(i + 1) == Some(&b'|') {
            continue;
        }
        let rest = text[i + 1..].replace("\\\n", " ");
        let mut words = rest.split_whitespace();
        let early = match words.next() {
            Some("head") => true,
            Some("grep") => words
                .take_while(|word| word.starts_with('-'))
                .any(|option| {
                    matches!(
                        option.split('=').next().unwrap(),
                        "--quiet" | "--silent" | "--max-count" | "--files-with-matches"
                    ) || (!option.starts_with("--") && option.contains(['q', 'm', 'l']))
                }),
            _ => false,
        };
        if early {
            let line = text[..i].lines().count().max(1);
            found.push(format!(
                "line {line}: `{}`",
                text.lines().nth(line - 1).unwrap_or("").trim()
            ));
        }
    }
    found
}

fn check_no_early_exit_reader(text: &str) {
    let found = if text.contains("pipefail") {
        early_exit_readers(text)
    } else {
        Vec::new()
    };
    assert!(
        found.is_empty(),
        "cloud-setup.sh pipes into a reader that can quit before its input ends, under pipefail: {found:?}"
    );
}

#[test]
fn cloud_setup_pipes_into_no_early_exit_reader_under_pipefail() {
    check_no_early_exit_reader(&script(&root()));
}

validation::negative_control!(
    cloud_setup_pipes_into_no_early_exit_reader_under_pipefail,
    "a script whose channel check pipes printf into `grep -q`, as it did before, required to use no such pipe",
    expected = "cloud-setup.sh pipes into a reader that can quit before its input ends",
    {
        let here = r#"grep -q '^channel ' <<<"$tc""#;
        let text = script(&root());
        assert!(text.contains(here), "the script checks the channel with `{here}`");
        check_no_early_exit_reader(&text.replace(here, r#"printf '%s\n' "$tc" | grep -q '^channel '"#))
    }
);
