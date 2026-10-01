//! R-346 (the human's (B) and (C)): `scripts/cloud-setup.sh` installs exactly what CI's Linux jobs install, and reads
//! every pin from the files CI reads, never from a copy of its own, so that it cannot drift from CI.
//!
//! The check reads CI's Linux jobs (`runs-on: ubuntu-*`) and the root toolchain file itself, here, into a plan of
//! `(kind, name, version)` items, and compares it with the plan the script prints under `--dry-run`:
//! - the two name the same items (no tool CI installs that the script doesn't, nor the other way);
//! - each item at the same version;
//! - the script's text holds no version literal, and none of CI's pinned versions;
//! - the script's dry run follows a changed pin in the files it reads;
//! - the script refuses a CI step that installs by a means it doesn't know, as this check does.

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
        for job in linux_jobs(file, text) {
            // The ref of the `dtolnay/rust-toolchain` step being read, and the indent of the `run:` block being read.
            let mut action_ref: Option<String> = None;
            let mut run_block: Option<usize> = None;
            for line in &job {
                let mut key = line.trim();
                if let Some(at) = run_block {
                    if indent(line) > at {
                        scan_command(file, key, &mut plan);
                        continue;
                    }
                    run_block = None;
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

/// The plan the script prints under `--dry-run` in the tree at `root`, which must exit well to give one.
fn script_plan(root: &Path) -> BTreeSet<Item> {
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
                3,
                "a plan line is `<kind> <name> <version>`: `{line}`"
            );
            (
                words[0].to_owned(),
                words[1].to_owned(),
                words[2].to_owned(),
            )
        })
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
