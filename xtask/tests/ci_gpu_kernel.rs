//! `ci.yml`'s gpu-kernel shards and what decides whether a run needs them (`.github/scripts/gpu_touch.sh`): a change to
//! any file the kernel's build reads runs them, they together run every suite slice, every package's controls and the
//! golden renders once, and `gpu-kernel` passes only when all of them did. And `cargo xtask controls --package`, which
//! the control shards call: it runs the packages it names, and refuses a name it can't run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::kernel_cache::INPUTS;
use yaml_rust2::{Yaml, YamlLoader};

#[allow(dead_code)] // its one-package `metadata`
#[path = "support/stand_in.rs"]
mod stand_in;

const CI: &str = include_str!("../../.github/workflows/ci.yml");
const TOUCH: &str = include_str!("../../.github/scripts/gpu_touch.sh");

/// An empty directory of this test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_gpu_kernel_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// `text` with `from` replaced by `to`; the edit must apply.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the control's edit doesn't apply");
    text.replacen(from, to, 1)
}

// ---- What runs the shards -----------------------------------------------------------------------------------------

/// What `script` prints in a repository whose HEAD changes `path` alone against `base`.
#[cfg(unix)]
struct Repo {
    dir: PathBuf,
    script: PathBuf,
}

#[cfg(unix)]
impl Repo {
    fn new(name: &str, script: &str) -> Repo {
        let dir = scratch(name);
        let repo = Repo {
            script: dir.join("gpu_touch.sh"),
            dir: dir.join("repo"),
        };
        fs::write(&repo.script, script).expect("the script");
        fs::create_dir_all(&repo.dir).expect("the repository's directory");
        repo.git(&["init", "-q"]);
        repo.commit(&["README.md"]);
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&self.dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .timed_output()
            .expect("run git");
        assert!(output.status.success(), "git {args:?}: {output:?}");
    }

    /// Commits a file of each of `paths`.
    fn commit(&self, paths: &[&str]) {
        for path in paths {
            let file = self.dir.join(path);
            fs::create_dir_all(file.parent().expect("a parent")).expect("a directory");
            fs::write(&file, "changed\n").expect("a file");
            self.git(&["add", "-f", path]);
        }
        self.git(&["commit", "-q", "--no-verify", "-m", "t"]);
    }

    /// The script's output, with `base` as the pull request's base (none on a push).
    fn touch(&self, base: Option<&str>) -> Result<String, String> {
        let mut command = Command::new("bash");
        command
            .arg(&self.script)
            .current_dir(&self.dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("BASE_REF", base.unwrap_or(""));
        let output = command.timed_output().expect("run bash");
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    /// The script's output on a pull request that changes `path` alone.
    fn touched(&self, path: &str) -> String {
        self.git(&["checkout", "-q", "--detach", "refs/remotes/origin/main"]);
        self.commit(&[path]);
        self.touch(Some("main")).expect("the script passes")
    }
}

/// A pull request changing a file of any entry of [`INPUTS`], any kernel file, or the checked-in WGSL runs the shards.
#[cfg(unix)]
fn check_touch_covers(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    let inputs = INPUTS.map(|input| {
        if input.contains('.') {
            input.to_owned()
        } else {
            format!("{input}/sub/file.rs")
        }
    });
    let others = [
        "crates/kernel/tests/t.rs",
        "crates/render/wgsl/decode.wgsl",
        "crates/render/wgsl/encode.wgsl",
        ".github/workflows/ci.yml",
        ".github/scripts/gpu_touch.sh",
    ];
    for path in inputs.iter().map(String::as_str).chain(others) {
        assert_eq!(
            repo.touched(path),
            "run=true",
            "a change to {path} doesn't run the GPU suites"
        );
    }
}

#[cfg(unix)]
#[test]
fn gpu_touch_covers_every_kernel_input() {
    check_touch_covers("covers", TOUCH);
}

#[cfg(unix)]
negative_control!(
    gpu_touch_covers_every_kernel_input,
    "the list without one input, a ledger directory",
    expected = "a change to crates/ledger/data/sub/file.rs doesn't run the GPU suites",
    check_touch_covers("ctl_covers", &edited(TOUCH, "  crates/ledger/data\n", ""))
);

/// A pull request that changes nothing the kernel's build reads doesn't run the shards.
#[cfg(unix)]
fn check_touch_skips(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    for path in [
        "docs/a.md",
        "crates/gui/src/lib.rs",
        "crates/kernelled/a.rs",
    ] {
        assert_eq!(
            repo.touched(path),
            "run=false",
            "a change to {path} runs the GPU suites"
        );
    }
}

#[cfg(unix)]
#[test]
fn gpu_touch_skips_other_changes() {
    check_touch_skips("skips", TOUCH);
}

#[cfg(unix)]
negative_control!(
    gpu_touch_skips_other_changes,
    "a list that takes every crate",
    expected = "a change to crates/gui/src/lib.rs runs the GPU suites",
    check_touch_skips(
        "ctl_skips",
        &edited(TOUCH, "  crates/kernel\n", "  crates\n")
    )
);

/// A run with no base (push, schedule, dispatch) runs the shards, and a diff that can't be taken fails, saying neither.
#[cfg(unix)]
fn check_touch_without_a_diff(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    assert_eq!(
        repo.touch(None).as_deref(),
        Ok("run=true"),
        "a push doesn't run the GPU suites"
    );
    let missing = repo.touch(Some("missing"));
    assert!(
        missing.is_err(),
        "a base that isn't there passed: {missing:?}"
    );
}

#[cfg(unix)]
#[test]
fn gpu_touch_runs_without_a_base_and_fails_without_a_diff() {
    check_touch_without_a_diff("base", TOUCH);
}

#[cfg(unix)]
negative_control!(
    gpu_touch_runs_without_a_base_and_fails_without_a_diff,
    "the diff taken in a pipe's condition, whose failure reads as no change",
    expected = "a base that isn't there passed",
    check_touch_without_a_diff(
        "ctl_base",
        &edited(TOUCH, "-- \"${paths[@]}\")", "|| true)")
    )
);

// ---- The shards ---------------------------------------------------------------------------------------------------

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("ci.yml parses")
        .remove(0)
}

/// The shards: gpu-shard's matrix entries.
fn shards(ci: &Yaml) -> &[Yaml] {
    ci["jobs"]["gpu-shard"]["strategy"]["matrix"]["include"]
        .as_vec()
        .expect("gpu-shard's matrix")
}

/// gpu-shard's step whose command starts with `command`.
fn step<'a>(ci: &'a Yaml, command: &str) -> &'a Yaml {
    let found: Vec<&Yaml> = ci["jobs"]["gpu-shard"]["steps"]
        .as_vec()
        .expect("gpu-shard's steps")
        .iter()
        .filter(|s| s["run"].as_str().is_some_and(|r| r.starts_with(command)))
        .collect();
    assert_eq!(found.len(), 1, "gpu-shard's steps that run `{command}`");
    found[0]
}

/// The control shards together name every package of the workspace that declares `controls`, once, and run what they
/// name.
fn check_controls_matrix(ci: &str) {
    let ci = parse(ci);
    let run = step(&ci, "cargo xtask controls");
    assert_eq!(
        run["run"].as_str(),
        Some("cargo xtask controls --package ${{ matrix.controls }}")
    );
    assert_eq!(
        run["if"].as_str(),
        Some("${{ !cancelled() && matrix.controls }}")
    );
    let mut named: Vec<&str> = shards(&ci)
        .iter()
        .filter_map(|shard| shard["controls"].as_str())
        .flat_map(|names| names.split(','))
        .collect();
    named.sort_unstable();
    let mut want = xtask::controls::packages(&xtask::workspace_manifest()).expect("the packages");
    want.sort_unstable();
    assert_eq!(named, want, "the control shards' packages");
}

#[test]
fn gpu_shards_cover_every_controls_package_once() {
    check_controls_matrix(CI);
}

#[cfg(feature = "controls")]
mod controls_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_controls_package_once,
        "a shard that drops a package",
        expected = "the control shards' packages",
        check_controls_matrix(&edited(CI, "controls: ledger,prin\n", "controls: ledger\n"))
    );
}

#[cfg(feature = "controls")]
mod controls_twice {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_controls_package_once,
        "a package in two shards",
        expected = "the control shards' packages",
        check_controls_matrix(&edited(CI, "controls: engine\n", "controls: engine,prin\n"))
    );
}

/// The suite shards are every slice 1 to n of the n the step splits into, once each, and one shard runs the golden
/// renders.
fn check_suites_matrix(ci: &str) {
    let ci = parse(ci);
    let suites = step(&ci, "cargo nextest run");
    let n: i64 = suites["run"]
        .as_str()
        .and_then(|run| {
            run.strip_prefix(
                "cargo nextest run -p validation -p xtask -p engine --partition hash:${{ matrix.slice }}/",
            )
        })
        .and_then(|n| n.parse().ok())
        .expect("the suites' partition");
    assert_eq!(suites["if"].as_str(), Some("matrix.slice"));
    assert_eq!(
        suites["env"]["NEXTEST_PROFILE"].as_str(),
        Some("gpu-kernel")
    );
    let mut slices: Vec<i64> = shards(&ci)
        .iter()
        .filter_map(|shard| shard["slice"].as_i64())
        .collect();
    slices.sort_unstable();
    assert_eq!(
        slices,
        (1..=n).collect::<Vec<_>>(),
        "the suite shards' slices"
    );
    let golden = step(&ci, "cargo xtask golden --all");
    assert!(golden["if"]
        .as_str()
        .is_some_and(|when| when.contains("matrix.golden == true")));
    let goldens = shards(&ci)
        .iter()
        .filter(|shard| shard["golden"].as_bool() == Some(true));
    assert_eq!(goldens.count(), 1, "the shards that run the golden renders");
}

#[test]
fn gpu_shards_cover_every_suite_slice_and_the_golden_renders() {
    check_suites_matrix(CI);
}

#[cfg(feature = "controls")]
mod slice_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_suite_slice_and_the_golden_renders,
        "a split into three, with two shards",
        expected = "the suite shards' slices",
        check_suites_matrix(&edited(
            CI,
            "hash:${{ matrix.slice }}/2",
            "hash:${{ matrix.slice }}/3"
        ))
    );
}

#[cfg(feature = "controls")]
mod golden_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_suite_slice_and_the_golden_renders,
        "no shard runs the golden renders",
        expected = "the shards that run the golden renders",
        check_suites_matrix(&edited(CI, "            golden: true\n", ""))
    );
}

/// One shard saves the caches, on main alone.
fn check_one_shard_saves(ci: &str) {
    let ci = parse(ci);
    let savers = shards(&ci)
        .iter()
        .filter(|shard| shard["save"].as_bool() == Some(true));
    assert_eq!(savers.count(), 1, "the shards that save the caches");
    let when = "github.ref == 'refs/heads/main' && matrix.save == true";
    let expression = format!("${{{{ {when} }}}}");
    for step in ci["jobs"]["gpu-shard"]["steps"]
        .as_vec()
        .expect("gpu-shard's steps")
    {
        let uses = step["uses"].as_str().unwrap_or("");
        if uses.starts_with("actions/cache/save@") {
            assert_eq!(
                step["if"].as_str(),
                Some(when),
                "a cache is saved by {step:?}"
            );
        } else if uses.starts_with("Swatinem/rust-cache@") {
            let save = step["with"]["save-if"].as_str();
            assert_eq!(save, Some(expression.as_str()), "rust-cache saves");
        } else {
            assert!(
                !uses.starts_with("actions/cache@"),
                "a cache is restored and saved by {step:?}"
            );
        }
    }
}

#[test]
fn gpu_shards_save_each_cache_once_on_main() {
    check_one_shard_saves(CI);
}

negative_control!(
    gpu_shards_save_each_cache_once_on_main,
    "every shard saves the compiled kernel",
    expected = "a cache is saved by",
    check_one_shard_saves(&edited(
        CI,
        "        if: github.ref == 'refs/heads/main' && matrix.save == true\n",
        "        if: github.ref == 'refs/heads/main'\n"
    ))
);

/// `gpu-kernel` needs the touch job and the shards, runs unless the touch job said no, and its step passes only when
/// each of them succeeded; a shard that fails doesn't stop the others.
#[cfg(unix)]
fn check_gate(ci: &str) {
    let ci = parse(ci);
    let gate = &ci["jobs"]["gpu-kernel"];
    let needs: Vec<&str> = gate["needs"]
        .as_vec()
        .map(|needs| needs.iter().filter_map(Yaml::as_str).collect())
        .unwrap_or_default();
    assert_eq!(needs, ["gpu-touch", "gpu-shard"], "what gpu-kernel needs");
    assert!(
        gate["name"].is_badvalue(),
        "gpu-kernel reports under another name"
    );
    assert_eq!(
        gate["if"].as_str(),
        Some("always() && needs.gpu-touch.outputs.run != 'false'"),
        "when gpu-kernel runs"
    );
    let shard = &ci["jobs"]["gpu-shard"];
    assert_eq!(shard["needs"].as_str(), Some("gpu-touch"));
    assert_eq!(
        shard["if"].as_str(),
        Some("needs.gpu-touch.outputs.run == 'true'")
    );
    assert_eq!(shard["strategy"]["fail-fast"].as_bool(), Some(false));
    let steps = gate["steps"].as_vec().expect("gpu-kernel's steps");
    assert_eq!(steps.len(), 1);
    assert_eq!(
        steps[0]["env"]["RESULTS"].as_str(),
        Some("${{ join(needs.*.result, ' ') }}")
    );
    let run = steps[0]["run"].as_str().expect("gpu-kernel's command");
    for (results, passes) in [
        ("success success", true),
        ("success failure", false),
        ("success cancelled", false),
        ("success skipped", false),
        ("failure skipped", false),
        ("failure success", false),
        ("", false),
    ] {
        let output = Command::new("bash")
            .args(["-e", "-c", run])
            .env("RESULTS", results)
            .timed_output()
            .expect("run bash");
        assert_eq!(
            output.status.success(),
            passes,
            "gpu-kernel on the results `{results}`"
        );
    }
}

#[cfg(unix)]
#[test]
fn gpu_kernel_passes_only_when_every_shard_did() {
    check_gate(CI);
}

#[cfg(all(unix, feature = "controls"))]
mod gate_skipped {
    use super::*;
    negative_control!(
        gpu_kernel_passes_only_when_every_shard_did,
        "a skipped shard taken as passed",
        expected = "gpu-kernel on the results `success skipped`",
        check_gate(&edited(
            CI,
            "do [ \"$result\" = success ] ||",
            "do [ \"$result\" = success ] || [ \"$result\" = skipped ] ||"
        ))
    );
}

#[cfg(all(unix, feature = "controls"))]
mod gate_blind {
    use super::*;
    negative_control!(
        gpu_kernel_passes_only_when_every_shard_did,
        "a gate that doesn't need the shards",
        expected = "what gpu-kernel needs",
        check_gate(&edited(
            CI,
            "needs: [gpu-touch, gpu-shard]",
            "needs: [gpu-touch]"
        ))
    );
}

#[cfg(all(unix, feature = "controls"))]
mod gate_not_always {
    use super::*;
    negative_control!(
        gpu_kernel_passes_only_when_every_shard_did,
        "a gate skipped when a shard fails",
        expected = "when gpu-kernel runs",
        check_gate(&edited(CI, "if: always() && needs", "if: needs"))
    );
}

// ---- cargo xtask controls --package -------------------------------------------------------------------------------

/// Runs `xtask controls --package <names>` on a stand-in workspace of `alpha`, `beta` and `gamma`, and `bare`, which
/// declares no `controls`: whether it passed, its output, and the packages whose controls it ran.
fn run_package(case: &str, names: &str) -> (bool, String, Vec<String>) {
    let dir = scratch(case);
    let package = |name: &str, features: serde_json::Value| json!({"name": name, "features": features, "targets": [{"doctest": false}]});
    let controls = || json!({"controls": []});
    let cargo = stand_in::stand_in_cargo(
        &dir,
        json!({
            "log": dir.join("log"),
            "metadata": json!({"packages": [
                package("alpha", controls()),
                package("beta", controls()),
                package("bare", json!({})),
                package("gamma", controls()),
            ]}).to_string(),
            "listed": "m::t: test\nm::t::negative_control: test\n",
            "filtered": true,
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["controls", "--package", names])
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let log = fs::read_to_string(dir.join("log")).unwrap_or_default();
    let ran = log
        .lines()
        .filter(|line| line.ends_with("-- negative_control"))
        .map(|line| {
            let (_, rest) = line.split_once("-p ").expect("a package");
            rest.split(' ').next().unwrap_or("").to_owned()
        })
        .collect();
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), out, ran)
}

/// `shards` together run the controls of each package that declares them, once, and no other's.
fn check_packages_run_once(case: &str, shards: &[&str]) {
    let mut ran = Vec::new();
    for (i, names) in shards.iter().enumerate() {
        let (ok, out, shard) = run_package(&format!("{case}_{i}"), names);
        assert!(ok, "controls --package {names} failed:\n{out}");
        ran.extend(shard);
    }
    ran.sort_unstable();
    assert_eq!(ran, ["alpha", "beta", "gamma"], "the packages run");
}

#[test]
fn controls_package_runs_each_named_package_once() {
    check_packages_run_once("once", &["beta", "gamma,alpha"]);
}

negative_control!(
    controls_package_runs_each_named_package_once,
    "shards that leave a package out",
    expected = "the packages run",
    check_packages_run_once("ctl_once", &["beta", "alpha"])
);

/// Each of `bad` is refused, naming why, and no package's controls run.
fn check_package_refused(case: &str, bad: &[(&str, &str)]) {
    for (i, (names, why)) in bad.iter().enumerate() {
        let (ok, out, ran) = run_package(&format!("{case}_{i}"), names);
        assert!(
            !ok && out.contains(why) && ran.is_empty(),
            "controls --package `{names}` wasn't refused with `{why}`: ran {ran:?}\n{out}"
        );
    }
}

#[test]
fn controls_package_refuses_a_name_it_cannot_run() {
    check_package_refused(
        "refused",
        &[
            ("alpha,delta", "no package `delta`"),
            ("alpha,bare", "`bare` declares no `controls` feature"),
            ("alpha,beta,alpha", "names `alpha` twice"),
            ("alpha,", "no package ``"),
            ("", "no package ``"),
        ],
    );
}

negative_control!(
    controls_package_refuses_a_name_it_cannot_run,
    "a package it can run",
    expected = "controls --package `alpha` wasn't refused",
    check_package_refused("ctl_refused", &[("alpha", "no package")])
);
