//! CI's debug info (the owner's CI list, 4 Nov 2026, item 4: "`debug = \"line-tables-only\"` (or 0) for the workspace
//! and 0 for deps, `CARGO_INCREMENTAL=0`, `strip = \"debuginfo\"` for test binaries ... confirm panic backtraces stay
//! usable"), read against R-374 (every CI job keeps line tables, so panics name file and line), R-337 (rust-cache hashes
//! the CARGO_* environment into its key, so every workflow sets the same values) and R-233 (local builds are unaffected).
//! Read from `.github/workflows/` and `.github/actions/cargo-ci-config/`:
//!
//! - every workflow sets the same CARGO_* values at its top level, `CARGO_INCREMENTAL=0` among them, and nothing below
//!   that level sets them again or strips anything;
//! - every job that runs cargo writes the dependencies' config before its first cargo command;
//! - that config covers dependencies only, in the `dev` and `test` profiles, in the runner's CARGO_HOME;
//! - the fixture pool's key follows these settings, in both jobs that use the pool.
//!
//! And from behaviour: a scratch workspace outside every cargo config but its own, built with what the script writes,
//! panics through a non-member dependency; the panic and the workspace's own frame name file and line, and the
//! dependency's frame carries no line table. Its control is why no test binary is stripped: `strip = "debuginfo"` takes
//! the workspace's line tables too.
//!
//! Each test has its negative control (R-176).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use validation::negative_control;
use validation::spawn::Spawn;

const SCRIPT: &str = ".github/actions/cargo-ci-config/config.sh";
const SCRIPT_STEP: &str = "uses: ./.github/actions/cargo-ci-config";
/// How the one line that may name the script does: `ci.yml`'s `cachekey` (twice, in `ci` and `xtask-ci`), hashing it.
const CACHEKEY: &str = "echo \"cachekey=";
const HASHES_SCRIPT: &str = "cat .github/actions/cargo-ci-config/config.sh;";
/// What `ci.yml`'s `cachekey` hashes of the environment, naming every setting in [`SETTINGS`].
const KEYED_ENV: &str = "grep -E '^CARGO_(INCREMENTAL|PROFILE)'";
/// The jobs that write the config themselves, in steps `qa_ci_archive_shrink` and `ci_workspace_disk` read, with what
/// profiles each builds in: `ci-archive` builds in `test` alone.
const OWN_STEP: [(&str, &str); 2] = [
    ("ci-archive", "- name: Dependencies without debug info"),
    (
        "ci-workspace",
        "- name: Dependencies without debug info, dev and test",
    ),
];
const RUST_CACHE: &str = "uses: Swatinem/rust-cache@";
const SELF_HOSTED: &str = "runs-on: [self-hosted,";
const INCREMENTAL: &str = "CARGO_INCREMENTAL";
const SETTINGS: [(&str, &str); 3] = [
    ("CARGO_INCREMENTAL", "0"),
    ("CARGO_PROFILE_DEV_DEBUG", "line-tables-only"),
    ("CARGO_PROFILE_TEST_DEBUG", "line-tables-only"),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every workflow file, as (name, text).
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(root().join(".github/workflows"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

fn script() -> String {
    fs::read_to_string(root().join(SCRIPT)).expect("read the config script")
}

fn ci_yml() -> String {
    fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("read ci.yml")
}

/// A line with its YAML comment removed (a `#` at the line's start or after whitespace).
fn uncommented(l: &str) -> &str {
    let cut = l
        .char_indices()
        .find(|&(i, c)| c == '#' && (i == 0 || l[..i].ends_with(char::is_whitespace)))
        .map_or(l.len(), |(i, _)| i);
    l[..cut].trim_end()
}

/// The `CARGO_*` pairs of the workflow's top-level `env:` block (its lines indented by two spaces), sorted.
fn top_cargo_env(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        if !l.starts_with(' ') {
            inside = l == "env:";
            continue;
        }
        if inside && !l.starts_with("   ") {
            if let Some((k, v)) = l.trim().split_once(':') {
                if k.starts_with("CARGO_") {
                    let value = v.trim().trim_matches(['"', '\'']);
                    out.push((k.to_owned(), value.to_owned()));
                }
            }
        }
    }
    out.sort();
    out
}

/// The lines of the workflow outside its top-level `env:` block, uncommented.
fn below_top_env(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        if !l.starts_with(' ') {
            inside = l == "env:";
            continue;
        }
        if !inside || l.starts_with("   ") {
            out.push(l);
        }
    }
    out
}

/// Each job of the workflow, as (id, its uncommented lines).
fn jobs(text: &str) -> Vec<(String, Vec<&str>)> {
    let mut out: Vec<(String, Vec<&str>)> = Vec::new();
    let mut in_jobs = false;
    for l in text.lines().map(uncommented) {
        if l.is_empty() {
            continue;
        }
        if !l.starts_with(' ') {
            in_jobs = l == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let id = l
            .strip_prefix("  ")
            .filter(|h| !h.starts_with(' '))
            .and_then(|h| h.strip_suffix(':'));
        match (id, out.last_mut()) {
            (Some(id), _) => out.push((id.to_owned(), Vec::new())),
            (None, Some((_, lines))) => lines.push(l),
            (None, None) => {}
        }
    }
    out
}

/// Whether the job's line runs cargo: the word `cargo`, on anything but a step's name or an action's input.
fn runs_cargo(l: &str) -> bool {
    let t = l.trim();
    let named = ["- name:", "name:", "tool:", "uses:", "key:", "path:"]
        .iter()
        .any(|k| t.starts_with(k));
    !named && t.split_whitespace().any(|w| w == "cargo")
}

/// Every workflow sets [`SETTINGS`], every one that restores a rust-cache sets the same CARGO_* values as ci.yml, and
/// none is set again below the top level.
fn check_env(files: &[(String, String)]) {
    assert!(
        files.len() >= 8,
        "expected every workflow of .github/workflows, found {}",
        files.len()
    );
    let ci = &files
        .iter()
        .find(|(n, _)| n == "ci.yml")
        .expect("no ci.yml")
        .1;
    let want = top_cargo_env(ci);
    for (key, value) in SETTINGS {
        assert!(
            want.iter().any(|(k, v)| k == key && v == value),
            "ci.yml's top-level env does not set {key} to {value}: {want:?}"
        );
    }
    for (name, text) in files {
        let env = top_cargo_env(text);
        for (key, value) in SETTINGS {
            assert!(
                env.iter().any(|(k, v)| k == key && v == value),
                "{name} sets other CARGO_* values than ci.yml: {key} is not {value}"
            );
        }
        assert!(
            !text.contains(RUST_CACHE) || env == want,
            "{name} sets other CARGO_* values than ci.yml, so rust-cache computes another key (R-337)"
        );
        for l in below_top_env(text) {
            let squeezed = l.replace(' ', "");
            assert!(
                !squeezed.contains(INCREMENTAL) && !squeezed.contains("_STRIP"),
                "{name} sets a build setting below its top-level env: `{}`",
                l.trim()
            );
            let strips = squeezed.contains("strip=") || squeezed.contains("-Cstrip");
            assert!(
                !strips,
                "{name} strips a binary, which takes R-374's line tables: `{}`",
                l.trim()
            );
        }
    }
}

#[test]
fn every_workflow_sets_the_same_build_settings() {
    check_env(&workflows());
}

/// The workflows with `from` replaced by `to`, once, in `file`.
#[cfg(feature = "controls")]
fn edited(file: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut files = workflows();
    let (_, text) = files
        .iter_mut()
        .find(|(f, _)| f == file)
        .unwrap_or_else(|| panic!("there is no .github/workflows/{file}"));
    assert!(text.contains(from), "{file} has no `{from}` to edit");
    *text = text.replacen(from, to, 1);
    files
}

negative_control!(
    every_workflow_sets_the_same_build_settings,
    "a workflow with another CARGO_INCREMENTAL than ci.yml's",
    expected = "gate.yml sets other CARGO_* values than ci.yml",
    check_env(&edited(
        "gate.yml",
        "CARGO_INCREMENTAL: \"0\"",
        "CARGO_INCREMENTAL: \"1\""
    ))
);

#[cfg(feature = "controls")]
mod another_key {
    use super::*;

    negative_control!(
        every_workflow_sets_the_same_build_settings,
        "a workflow that restores a rust-cache with a CARGO_* value ci.yml does not set",
        expected = "pr-check.yml sets other CARGO_* values than ci.yml, so rust-cache computes another key",
        check_env(&edited(
            "pr-check.yml",
            "CARGO_INCREMENTAL: \"0\"\n",
            "CARGO_INCREMENTAL: \"0\"\n  CARGO_NET_RETRY: \"3\"\n",
        ))
    );
}

#[cfg(feature = "controls")]
mod ci_without_it {
    use super::*;

    negative_control!(
        every_workflow_sets_the_same_build_settings,
        "every workflow without CARGO_INCREMENTAL, the same in each",
        expected = "ci.yml's top-level env does not set CARGO_INCREMENTAL to 0",
        {
            let files: Vec<(String, String)> = workflows()
                .into_iter()
                .map(|(n, t)| (n, t.replace("CARGO_INCREMENTAL: \"0\"", "")))
                .collect();
            check_env(&files)
        }
    );
}

#[cfg(feature = "controls")]
mod job_override {
    use super::*;

    negative_control!(
        every_workflow_sets_the_same_build_settings,
        "a job that turns incremental builds back on for itself",
        expected = "ci.yml sets a build setting below its top-level env",
        check_env(&edited(
            "ci.yml",
            "      PRIN_GPU_BACKEND: vulkan\n",
            "      PRIN_GPU_BACKEND: vulkan\n      CARGO_INCREMENTAL: \"1\"\n",
        ))
    );
}

#[cfg(feature = "controls")]
mod strips_by_env {
    use super::*;

    negative_control!(
        every_workflow_sets_the_same_build_settings,
        "a job that strips its test binaries through the environment",
        expected = "ci.yml sets a build setting below its top-level env",
        check_env(&edited(
            "ci.yml",
            "      PRIN_GPU_BACKEND: vulkan\n",
            "      PRIN_GPU_BACKEND: vulkan\n      CARGO_PROFILE_TEST_STRIP: debuginfo\n",
        ))
    );
}

#[cfg(feature = "controls")]
mod strips_by_config {
    use super::*;

    negative_control!(
        every_workflow_sets_the_same_build_settings,
        "a step that strips test binaries through a cargo config",
        expected = "strips a binary",
        check_env(&edited(
            "mutants.yml",
            "        uses: ./.github/actions/cargo-ci-config\n",
            "        run: echo 'strip = \"debuginfo\"' >> config.toml\n",
        ))
    );
}

/// Every job that runs cargo writes the config, in the script's step or, for [`OWN_STEP`]'s two, in their own, before
/// its first cargo command and after its checkout, which brings the script.
fn check_jobs(files: &[(String, String)]) {
    let mut checked = 0;
    for (name, text) in files {
        for (job, lines) in jobs(text) {
            // The self-hosted runner's CARGO_HOME is read-only to its jobs and outlives them, so they can't write this
            // config (R-711, which lands first in its own rulings PR; `win-gpu.yml`, whose header says so).
            if lines.iter().any(|l| l.trim().starts_with(SELF_HOSTED)) {
                continue;
            }
            let Some(first) = lines.iter().position(|l| runs_cargo(l)) else {
                continue;
            };
            checked += 1;
            let own = OWN_STEP
                .iter()
                .find(|(j, _)| name == "ci.yml" && *j == job)
                .map(|(_, step)| *step);
            let step = lines
                .iter()
                .position(|l| match own {
                    Some(step) => l.trim() == step,
                    None => l.trim() == SCRIPT_STEP,
                })
                .unwrap_or_else(|| {
                    panic!("{name}'s job `{job}` runs cargo and writes no dependencies' config")
                });
            assert!(
                step < first,
                "{name}'s job `{job}` writes the dependencies' config after its first cargo command"
            );
            let checkout = lines
                .iter()
                .position(|l| l.contains("uses: actions/checkout@"))
                .unwrap_or_else(|| panic!("{name}'s job `{job}` has no checkout"));
            assert!(
                checkout < step,
                "{name}'s job `{job}` runs the config script before its checkout brings it"
            );
        }
    }
    assert!(
        checked >= 20,
        "the parser found {checked} jobs that run cargo, far too few"
    );
}

#[test]
fn every_job_that_runs_cargo_writes_the_dependencies_config() {
    check_jobs(&workflows());
}

/// `xtask-ci`'s header in ci.yml: the job that needs the step most (92 GB of `target/debug/deps`, run 41756812074).
#[cfg(feature = "controls")]
const XTASK_CI: &str = "\n  xtask-ci:\n";
/// The step as the workflows write it.
#[cfg(feature = "controls")]
const STEP_TEXT: &str =
    "      - name: Dependencies without debug info, every profile\n        uses: ./.github/actions/cargo-ci-config\n";

/// ci.yml with the step cut from `xtask-ci`, and `then` applied to the text.
#[cfg(feature = "controls")]
fn without_step(then: impl Fn(String) -> String) -> Vec<(String, String)> {
    workflows()
        .into_iter()
        .map(|(n, t)| {
            if n != "ci.yml" {
                return (n, t);
            }
            let job = t.find(XTASK_CI).expect("ci.yml has no job xtask-ci");
            let at = job + t[job..].find(STEP_TEXT).expect("xtask-ci has no step");
            let cut = format!("{}{}", &t[..at], &t[at + STEP_TEXT.len()..]);
            (n, then(cut))
        })
        .collect()
}

negative_control!(
    every_job_that_runs_cargo_writes_the_dependencies_config,
    "`xtask-ci` without the step",
    expected = "ci.yml's job `xtask-ci` runs cargo and writes no dependencies' config",
    check_jobs(&without_step(|t| t))
);

#[cfg(feature = "controls")]
mod after_cargo {
    use super::*;

    negative_control!(
        every_job_that_runs_cargo_writes_the_dependencies_config,
        "`xtask-ci` writing the config after its build",
        expected =
            "ci.yml's job `xtask-ci` writes the dependencies' config after its first cargo command",
        {
            let run = "        run: cargo xtask ci --partition ${{ matrix.shard }}/4\n";
            check_jobs(&without_step(|t| {
                assert!(t.contains(run));
                t.replacen(run, &format!("{run}{STEP_TEXT}"), 1)
            }))
        }
    );
}

#[cfg(feature = "controls")]
mod before_checkout {
    use super::*;

    negative_control!(
        every_job_that_runs_cargo_writes_the_dependencies_config,
        "`xtask-ci` running the script before the checkout that brings it",
        expected = "ci.yml's job `xtask-ci` runs the config script before its checkout brings it",
        {
            check_jobs(&without_step(|t| {
                let job = t.find(XTASK_CI).unwrap();
                let checkout = "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n";
                let at = job + t[job..].find(checkout).unwrap();
                format!("{}{STEP_TEXT}{}", &t[..at], &t[at..])
            }))
        }
    );
}

#[cfg(feature = "controls")]
mod own_step_gone {
    use super::*;

    negative_control!(
        every_job_that_runs_cargo_writes_the_dependencies_config,
        "`ci-workspace` with its own step gone",
        expected = "ci.yml's job `ci-workspace` runs cargo and writes no dependencies' config",
        check_jobs(&edited(
            "ci.yml",
            "- name: Dependencies without debug info, dev and test",
            "- name: something else",
        ))
    );
}

/// What running `script` leaves in a CARGO_HOME whose config already holds a line of its own, without that line.
fn written_by(script: &str, case: &str) -> String {
    let home =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_debug_info_home_{case}"));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
    let config = home.join("config.toml");
    let kept = "# already here\n";
    fs::write(&config, kept).unwrap();
    let path = home.join("config.sh");
    fs::write(&path, script).unwrap();
    let out = Command::new("bash")
        .arg(&path)
        .env("CARGO_HOME", &home)
        .current_dir(&home)
        .timed_output()
        .expect("run the config script");
    assert!(
        out.status.success(),
        "the config script failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(&config).expect("read the written config");
    text.strip_prefix(kept)
        .unwrap_or_else(|| {
            panic!("the script lost what the CARGO_HOME config already held:\n{text}")
        })
        .to_owned()
}

/// The written config sets `debug = false` on the dependencies of the `dev` and `test` profiles, and nothing else.
fn check_config(written: &str) {
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(
        lines,
        [
            "[profile.dev.package.\"*\"]",
            "debug = false",
            "[profile.test.package.\"*\"]",
            "debug = false",
        ],
        "the written config is not `debug = false` on the dependencies of the dev and test profiles alone"
    );
    assert!(
        toml_edit::DocumentMut::from_str(written).is_ok(),
        "the written config is not TOML:\n{written}"
    );
}

#[test]
fn the_script_writes_the_dependencies_config_and_nothing_else() {
    check_config(&written_by(&script(), "script"));
}

negative_control!(
    the_script_writes_the_dependencies_config_and_nothing_else,
    "a script that takes the workspace's own line tables as well (R-374)",
    expected = "is not `debug = false` on the dependencies",
    check_config(&written_by(
        &script().replace("[profile.test.package.\"*\"]", "[profile.test]"),
        "whole"
    ))
);

#[cfg(feature = "controls")]
mod strips {
    use super::*;

    negative_control!(
        the_script_writes_the_dependencies_config_and_nothing_else,
        "a script that also strips the test binaries",
        expected = "is not `debug = false` on the dependencies",
        check_config(&written_by(
            &script().replace(
                "\\ndebug = false\\n' >>",
                "\\ndebug = false\\nstrip = \"debuginfo\"\\n' >>"
            ),
            "strip"
        ))
    );
}

#[cfg(feature = "controls")]
mod clobbers {
    use super::*;

    negative_control!(
        the_script_writes_the_dependencies_config_and_nothing_else,
        "a script that overwrites the runner's CARGO_HOME config instead of appending",
        expected = "lost what the CARGO_HOME config already held",
        check_config(&written_by(
            &script().replace("' >> \"", "' > \""),
            "clobber"
        ))
    );
}

/// The workflows' step is an action, whose one step runs the script: a `run:` step would run in the tests that run a
/// job's `run:` steps outside CI, and write to that machine's CARGO_HOME (R-233). So no line of a workflow names the
/// script, on a `run:` line or inside a `run: |` block, but `cachekey`'s, which only hashes it.
fn check_action(action: &str, files: &[(String, String)]) {
    assert!(
        action.contains("using: composite")
            && action.contains("run: bash \"$GITHUB_ACTION_PATH/config.sh\""),
        "the action does not run its config.sh:\n{action}"
    );
    for (name, text) in files {
        for l in text.lines().map(uncommented) {
            let rest = if l.trim_start().starts_with(CACHEKEY) {
                l.replacen(HASHES_SCRIPT, "", 1)
            } else {
                l.to_owned()
            };
            assert!(
                !rest.contains("config.sh"),
                "{name} runs the config script in a `run:` step: `{}`",
                l.trim()
            );
        }
    }
}

fn action() -> String {
    fs::read_to_string(root().join(".github/actions/cargo-ci-config/action.yml"))
        .expect("read the action")
}

#[test]
fn the_config_is_written_by_an_action() {
    check_action(&action(), &workflows());
}

negative_control!(
    the_config_is_written_by_an_action,
    "an action that runs something else",
    expected = "the action does not run its config.sh",
    check_action(&action().replace("config.sh\"", "other.sh\""), &workflows())
);

#[cfg(feature = "controls")]
mod run_step {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "a workflow that runs the script in a `run:` step",
        expected = "gate.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "gate.yml",
                "        uses: ./.github/actions/cargo-ci-config\n",
                "        run: bash .github/actions/cargo-ci-config/config.sh\n",
            )
        )
    );
}

#[cfg(feature = "controls")]
mod run_block {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "a workflow that runs the script from the body of a `run: |` block",
        expected = "gate.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "gate.yml",
                "        uses: ./.github/actions/cargo-ci-config\n",
                "        run: |\n          set -eu\n          bash .github/actions/cargo-ci-config/config.sh\n",
            )
        )
    );
}

#[cfg(feature = "controls")]
mod run_beside {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "a job that keeps the action and also sources the script in a step of its own",
        expected = "gate.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "gate.yml",
                "        uses: ./.github/actions/cargo-ci-config\n",
                "        uses: ./.github/actions/cargo-ci-config\n      - run: |\n          . .github/actions/cargo-ci-config/config.sh\n",
            )
        )
    );
}

#[cfg(feature = "controls")]
mod run_in_cachekey {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "`cachekey`'s line running the script as well as hashing it",
        expected = "ci.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "ci.yml",
                HASHES_SCRIPT,
                "cat .github/actions/cargo-ci-config/config.sh; bash .github/actions/cargo-ci-config/config.sh;",
            )
        )
    );
}

#[cfg(feature = "controls")]
mod run_piped_in_cachekey {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "`cachekey`'s line piping the script to a shell, which is not the `cat` that only hashes it",
        expected = "ci.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "ci.yml",
                HASHES_SCRIPT,
                "cat .github/actions/cargo-ci-config/config.sh | bash;",
            )
        )
    );
}

#[cfg(feature = "controls")]
mod cat_beyond_cachekey {
    use super::*;

    negative_control!(
        the_config_is_written_by_an_action,
        "the hashing `cat` on a line that is not `cachekey`'s: only that line may name the script",
        expected = "gate.yml runs the config script in a `run:` step",
        check_action(
            &action(),
            &edited(
                "gate.yml",
                "        uses: ./.github/actions/cargo-ci-config\n",
                "        run: |\n          cat .github/actions/cargo-ci-config/config.sh; true\n",
            )
        )
    );
}

/// The repository's own files, which local builds read, set no profile: the settings are CI's alone (R-233).
fn check_local(manifest: &str, config: &str) {
    for (file, text) in [("Cargo.toml", manifest), (".cargo/config.toml", config)] {
        assert!(
            !text.lines().any(|l| l.trim_start().starts_with("[profile")),
            "{file} sets a profile, which local builds read (R-233)"
        );
        assert!(
            !text
                .lines()
                .any(|l| l.trim_start().starts_with("incremental")),
            "{file} sets incremental builds, which local builds read (R-233)"
        );
    }
}

fn local_files() -> (String, String) {
    (
        fs::read_to_string(root().join("Cargo.toml")).unwrap(),
        fs::read_to_string(root().join(".cargo/config.toml")).unwrap(),
    )
}

#[test]
fn local_builds_keep_their_profiles() {
    let (manifest, config) = local_files();
    check_local(&manifest, &config);
}

negative_control!(
    local_builds_keep_their_profiles,
    "the settings written into the repository's .cargo/config.toml",
    expected = ".cargo/config.toml sets a profile",
    {
        let (manifest, config) = local_files();
        check_local(
            &manifest,
            &format!("{config}\n[profile.test.package.\"*\"]\ndebug = false\n"),
        )
    }
);

/// The `cachekey` lines of `ci`, which saves the fixture pool, and `xtask-ci`, which restores it: the same line, naming
/// each build setting the pool's builds carry. A pool built under other settings would restore and be rebuilt whole
/// beside itself.
fn check_pool_key(ci: &str) {
    let keyed: Vec<&str> = ci
        .lines()
        .map(uncommented)
        .filter(|l| l.contains("echo \"cachekey="))
        .collect();
    assert_eq!(
        keyed.len(),
        2,
        "`cachekey` is computed {} times, not twice",
        keyed.len()
    );
    assert_eq!(
        keyed[0], keyed[1],
        "`ci` and `xtask-ci` compute different `cachekey`s, so `xtask-ci` never restores `ci`'s pool"
    );
    assert!(
        keyed[0].contains(KEYED_ENV),
        "`cachekey` does not follow the env's build settings: {}",
        keyed[0]
    );
    assert!(
        keyed[0].contains(SCRIPT),
        "`cachekey` does not follow the dependencies' config: {}",
        keyed[0]
    );
}

#[test]
fn the_fixture_pool_key_follows_the_build_settings() {
    check_pool_key(&ci_yml());
}

negative_control!(
    the_fixture_pool_key_follows_the_build_settings,
    "`cachekey` back on the toolchain alone, in both jobs",
    expected = "does not follow the env's build settings",
    check_pool_key(&ci_yml().replace(KEYED_ENV, "grep -E '^CARGO_TERM'"))
);

#[cfg(feature = "controls")]
mod no_script {
    use super::*;

    negative_control!(
        the_fixture_pool_key_follows_the_build_settings,
        "`cachekey` without the dependencies' config, in both jobs",
        expected = "does not follow the dependencies' config",
        check_pool_key(&ci_yml().replace(" cat .github/actions/cargo-ci-config/config.sh;", ""))
    );
}

#[cfg(feature = "controls")]
mod apart {
    use super::*;

    negative_control!(
        the_fixture_pool_key_follows_the_build_settings,
        "`xtask-ci` computing another `cachekey`, so it never restores `ci`'s pool",
        expected = "compute different `cachekey`s",
        {
            let ci = ci_yml();
            let at = ci.rfind("echo \"cachekey=").unwrap();
            check_pool_key(&format!("{}echo \"cachekey=x{}", &ci[..at], &ci[at + 14..]))
        }
    );
}

// ---- Behaviour: what the script writes, built.

/// Fails if a cargo config sits in `dir` or above it. cargo reads every `.cargo/config.toml` from its working directory
/// up to the root, and those outrank `$CARGO_HOME`, so a scratch `CARGO_HOME` does not hide one.
fn check_no_config_above(dir: &Path) {
    for parent in dir.ancestors() {
        for file in ["config.toml", "config"] {
            let config = parent.join(".cargo").join(file);
            assert!(
                !config.exists(),
                "{} would reach the scratch workspace",
                config.display()
            );
        }
    }
}

/// A fresh directory for `case`, this process's alone, that no cargo config reaches but the one the test writes. Not
/// `CARGO_TARGET_TMPDIR`: on a runner that sits under `/home/runner/work`, below the `/home/runner/.cargo/config.toml`
/// the workflows' action writes, so that config would set what the scratch's own is here to set.
fn scratch_dir(case: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("ci_debug_info_{}_{case}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    check_no_config_above(&base);
    base
}

/// Builds and runs a scratch workspace's failing test under the workflows' env, with `config` as the CARGO_HOME config,
/// and returns everything it printed.
fn run_scratch(case: &str, config: &str) -> String {
    let base = scratch_dir(case);
    let write = |p: PathBuf, s: &str| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    };
    // The dependency sits outside the workspace's directory, so it is not a member: `package."*"` matches it.
    write(
        base.join("dep/Cargo.toml"),
        "[package]\nname = \"cidbg_dep\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    write(
        base.join("dep/src/lib.rs"),
        "#[inline(never)]\npub fn cidbg_call_through(f: fn()) {\n    f();\n    std::hint::black_box(());\n}\n",
    );
    write(
        base.join("ws/Cargo.toml"),
        "[package]\nname = \"cidbg_app\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n\
         cidbg_dep = { path = \"../dep\" }\n\n[workspace]\n",
    );
    write(
        base.join("ws/src/lib.rs"),
        "#[inline(never)]\npub fn cidbg_boom() {\n    panic!(\"cidbg boom\");\n}\n\n#[test]\nfn panics() {\n    \
         cidbg_dep::cidbg_call_through(cidbg_boom);\n}\n",
    );
    let home = base.join("cargo_home");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("config.toml"), config).unwrap();
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()));
    command
        .current_dir(base.join("ws"))
        .args(["test", "--offline", "--lib"])
        .env("CARGO_HOME", &home)
        .env("CARGO_TARGET_DIR", base.join("target"))
        .env("RUST_BACKTRACE", "1")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("NEXTEST_PROFILE");
    for (key, value) in SETTINGS {
        command.env(key, value);
    }
    let out = command
        .timed_output()
        .expect("run cargo test on the scratch workspace");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = fs::remove_dir_all(&base);
    assert!(
        !out.status.success() && text.contains("cidbg boom"),
        "the scratch test did not build and panic:\n{text}"
    );
    text
}

/// The `at <file>:<line>:<col>` line under the backtrace frame of function `symbol`, if it has one.
fn frame_location(text: &str, symbol: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| {
            l.trim_start().split_once(": ").is_some_and(|(n, s)| {
                let s = s.trim();
                n.chars().all(|c| c.is_ascii_digit())
                    && (s == symbol || s.ends_with(&format!("::{symbol}")))
            })
        })
        .unwrap_or_else(|| panic!("the backtrace has no frame `{symbol}`:\n{text}"));
    lines
        .get(at + 1)
        .and_then(|l| l.trim().strip_prefix("at "))
        .map(str::to_owned)
}

/// R-374 on what the build printed: the panic and the workspace's own frame name file and line; the dependency's frame
/// is named without them, so the dependency's line tables went.
fn check_backtrace(text: &str) {
    assert!(
        text.contains("panicked at src/lib.rs:3:"),
        "the panic message does not name its file and line:\n{text}"
    );
    let own = frame_location(text, "cidbg_boom");
    assert!(
        own.as_deref()
            .is_some_and(|l| l.ends_with("src/lib.rs:3:5")),
        "the workspace's frame `cidbg_boom` has no file:line (R-374): {own:?}\n{text}"
    );
    let dep = frame_location(text, "cidbg_call_through");
    assert!(
        dep.is_none(),
        "the dependency's frame `cidbg_call_through` still carries file:line: {dep:?}"
    );
}

#[test]
fn a_panic_under_the_ci_settings_names_file_and_line() {
    check_backtrace(&run_scratch("settings", &written_by(&script(), "panic")));
}

negative_control!(
    a_panic_under_the_ci_settings_names_file_and_line,
    "the owner's `strip = \"debuginfo\"` on test binaries: it takes the workspace's line tables as well (R-374)",
    expected = "has no file:line (R-374)",
    {
        let config = format!(
            "{}[profile.test]\nstrip = \"debuginfo\"\n",
            written_by(&script(), "panic_strip")
        );
        check_backtrace(&run_scratch("strip", &config))
    }
);

#[cfg(feature = "controls")]
mod dependency_keeps_its_tables {
    use super::*;

    negative_control!(
        a_panic_under_the_ci_settings_names_file_and_line,
        "no dependencies' config: the dependency keeps its line tables",
        expected = "still carries file:line",
        check_backtrace(&run_scratch("none", ""))
    );
}

#[cfg(feature = "controls")]
mod config_above_the_scratch {
    use super::*;

    negative_control!(
        a_panic_under_the_ci_settings_names_file_and_line,
        "a cargo config above the scratch workspace, as on a runner under `/home/runner`, which CARGO_HOME does not hide",
        expected = "would reach the scratch workspace",
        {
            let above = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("ci_debug_info_above_{}", std::process::id()));
            fs::create_dir_all(above.join(".cargo")).unwrap();
            fs::write(above.join(".cargo/config.toml"), "").unwrap();
            check_no_config_above(&above.join("ws"))
        }
    );
}
