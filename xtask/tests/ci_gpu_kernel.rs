//! `ci.yml`'s gpu-kernel shards and what decides which of them a run needs (`.github/scripts/gpu_touch.sh`): a change to
//! any file the kernel's build reads runs all of them, another change to crates/render the render controls' alone, they
//! together run every suite slice, every package's controls and the golden renders once, and `gpu-kernel` passes only
//! when all those asked for did. And `cargo xtask controls --package`, which the control shards call: it runs the
//! packages it names, and refuses a name it can't run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
#[cfg(unix)]
use serde_json::Value;
use validation::negative_control;
use validation::spawn::Spawn;
#[cfg(unix)]
use xtask::kernel_cache::INPUTS;
#[cfg(unix)]
use yaml_rust2::{Yaml, YamlLoader};

#[allow(dead_code)] // its one-package `metadata`
#[path = "support/stand_in.rs"]
mod stand_in;

#[cfg(unix)]
const CI: &str = include_str!("../../.github/workflows/ci.yml");
#[cfg(unix)]
const TOUCH: &str = include_str!("../../.github/scripts/gpu_touch.sh");

/// An empty directory of this test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_gpu_kernel_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// `text` with `from` replaced by `to`; the edit must apply.
#[cfg(all(unix, feature = "controls"))]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the control's edit doesn't apply");
    text.replacen(from, to, 1)
}

// ---- What runs the shards -----------------------------------------------------------------------------------------

/// What `gpu_touch.sh` prints.
#[cfg(unix)]
#[derive(Debug, PartialEq)]
struct Answer {
    run: bool,
    kernel: bool,
    shards: Vec<Value>,
}

#[cfg(unix)]
impl Answer {
    fn parse(out: &str) -> Answer {
        let lines: Vec<&str> = out.lines().collect();
        let [run, kernel, shards] = lines[..] else {
            panic!("not three lines: {out}");
        };
        let value = |line: &str, key: &str| -> Value {
            let text = line
                .strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
                .unwrap_or_else(|| panic!("no `{key}` in `{line}`"));
            serde_json::from_str(text).unwrap_or_else(|e| panic!("{line}: {e}"))
        };
        Answer {
            run: value(run, "run").as_bool().expect("run"),
            kernel: value(kernel, "kernel").as_bool().expect("kernel"),
            shards: value(shards, "shards").as_array().expect("shards").clone(),
        }
    }

    fn none() -> Answer {
        Answer {
            run: false,
            kernel: false,
            shards: Vec::new(),
        }
    }

    fn render_alone() -> Answer {
        Answer {
            run: true,
            kernel: false,
            shards: vec![json!({"name": "controls render", "controls": "render"})],
        }
    }
}

/// A repository whose HEAD is compared with `origin/main`, or `origin/other`, the same commit.
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
        for base in ["main", "other"] {
            repo.git(&["update-ref", &format!("refs/remotes/origin/{base}"), "HEAD"]);
        }
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
    fn touch(&self, base: Option<&str>) -> Result<Answer, String> {
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
        Ok(Answer::parse(&String::from_utf8_lossy(&output.stdout)))
    }

    /// The script's output on a run with no pull request.
    fn push(&self) -> Answer {
        self.touch(None).expect("the script passes")
    }

    /// The script's output on a pull request into `base` that changes `paths` alone.
    fn touched_into(&self, base: &str, paths: &[&str]) -> Answer {
        self.git(&["checkout", "-q", "--detach", "refs/remotes/origin/main"]);
        self.commit(paths);
        self.touch(Some(base)).expect("the script passes")
    }

    /// The script's output on a pull request into main that changes `paths` alone.
    fn touched(&self, paths: &[&str]) -> Answer {
        self.touched_into("main", paths)
    }
}

/// A pull request changing a file of any entry of [`INPUTS`], any kernel file, or the checked-in WGSL runs every shard
/// a push does, alone or with a change to the rest of crates/render.
#[cfg(unix)]
fn check_touch_covers(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    let every = repo.push();
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
            repo.touched(&[path]),
            every,
            "a change to {path} doesn't run the GPU suites"
        );
        assert_eq!(
            repo.touched(&[path, "crates/render/src/lib.rs"]),
            every,
            "a change to {path} and crates/render doesn't run the GPU suites"
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

#[cfg(all(unix, feature = "controls"))]
mod covers_render_only {
    use super::*;
    negative_control!(
        gpu_touch_covers_every_kernel_input,
        "a kernel change that runs the render shard alone",
        expected = "a change to .cargo/config.toml doesn't run the GPU suites",
        check_touch_covers(
            "ctl_covers_render_only",
            &edited(
                TOUCH,
                "if [ -n \"$kernel_changed\" ]; then\n  answer true \"${shards[@]}\"",
                "if [ -n \"$kernel_changed\" ]; then\n  answer true \"$render\""
            )
        )
    );
}

#[cfg(all(unix, feature = "controls"))]
mod covers_render_first {
    use super::*;
    negative_control!(
        gpu_touch_covers_every_kernel_input,
        "a change to the kernel and to crates/render taken as one to crates/render",
        expected = "a change to .cargo/config.toml and crates/render doesn't run the GPU suites",
        check_touch_covers(
            "ctl_covers_render_first",
            &edited(
                TOUCH,
                "if [ -n \"$kernel_changed\" ]; then",
                "if [ -n \"$kernel_changed\" ] && [ -z \"$render_changed\" ]; then"
            )
        )
    );
}

/// A pull request into main that changes crates/render, and nothing the kernel's build reads, runs the render controls'
/// shard alone.
#[cfg(unix)]
fn check_touch_render(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    for path in [
        "crates/render/src/assemble/declare.rs",
        "crates/render/tests/t.rs",
        "crates/render/Cargo.toml",
    ] {
        assert_eq!(
            repo.touched(&[path, "docs/a.md"]),
            Answer::render_alone(),
            "a change to {path} doesn't run the render controls alone"
        );
    }
}

#[cfg(unix)]
#[test]
fn gpu_touch_runs_the_render_controls_alone_on_a_render_change() {
    check_touch_render("render", TOUCH);
}

#[cfg(all(unix, feature = "controls"))]
mod render_dropped {
    use super::*;
    negative_control!(
        gpu_touch_runs_the_render_controls_alone_on_a_render_change,
        "crates/render dropped, its WGSL aside",
        expected = "declare.rs doesn't run the render controls alone",
        check_touch_render(
            "ctl_render",
            &edited(
                TOUCH,
                "changed crates/render)",
                "changed crates/render/wgsl)"
            )
        )
    );
}

#[cfg(all(unix, feature = "controls"))]
mod render_runs_all {
    use super::*;
    negative_control!(
        gpu_touch_runs_the_render_controls_alone_on_a_render_change,
        "a render change that runs every shard",
        expected = "declare.rs doesn't run the render controls alone",
        check_touch_render(
            "ctl_render_all",
            &edited(
                TOUCH,
                "answer false \"$render\"",
                "answer false \"${shards[@]}\""
            )
        )
    );
}

/// Into another branch the diff leaves out that branch's own kernel changes, and changing the base to main starts no new
/// run, so a render change there runs every shard: a passing gpu-kernel it carries to main means all of them.
#[cfg(unix)]
fn check_touch_other_base(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    let other = |path: &str| repo.touched_into("other", &[path]);
    assert_eq!(
        other("crates/render/src/lib.rs").shards,
        repo.push().shards,
        "a render change into another branch doesn't run every shard"
    );
    assert_eq!(other("docs/a.md"), Answer::none());
}

#[cfg(unix)]
#[test]
fn gpu_touch_runs_every_shard_on_a_render_change_into_another_branch() {
    check_touch_other_base("other", TOUCH);
}

#[cfg(unix)]
negative_control!(
    gpu_touch_runs_every_shard_on_a_render_change_into_another_branch,
    "any base taken as main",
    expected = "a render change into another branch doesn't run every shard",
    check_touch_other_base(
        "ctl_other",
        &edited(
            TOUCH,
            "elif [ \"$BASE_REF\" = main ]; then",
            "elif true; then"
        )
    )
);

/// A pull request that changes neither crates/render nor anything the kernel's build reads runs no shard.
#[cfg(unix)]
fn check_touch_skips(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    for path in [
        "docs/a.md",
        "crates/gui/src/lib.rs",
        "crates/kernelled/a.rs",
        "crates/renderer/a.rs",
    ] {
        assert_eq!(
            repo.touched(&[path]),
            Answer::none(),
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

#[cfg(all(unix, feature = "controls"))]
mod render_always {
    use super::*;
    negative_control!(
        gpu_touch_skips_other_changes,
        "the render shard always on",
        expected = "a change to docs/a.md runs the GPU suites",
        check_touch_skips(
            "ctl_render_always",
            &edited(TOUCH, "  answer false\n", "  answer false \"$render\"\n")
        )
    );
}

/// A run with no base (push, schedule, dispatch) runs the shards, and a diff that can't be taken fails, saying neither.
#[cfg(unix)]
fn check_touch_without_a_diff(name: &str, script: &str) {
    let repo = Repo::new(name, script);
    let push = repo.push();
    assert!(
        push.run && push.kernel,
        "a push doesn't run the GPU suites: {push:?}"
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
    check_touch_without_a_diff("ctl_base", &edited(TOUCH, "-- \"$@\"", "-- \"$@\" || true"))
);

// ---- The shards ---------------------------------------------------------------------------------------------------

#[cfg(unix)]
fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("ci.yml parses")
        .remove(0)
}

/// Every shard: the matrix entries `script` prints on a push.
#[cfg(unix)]
fn shards(name: &str, script: &str) -> Vec<Value> {
    Repo::new(name, script).push().shards
}

/// gpu-shard's matrix is what gpu-touch printed and nothing more, and kernel-wgsl's steps run on `kernel`, which a render
/// change doesn't set.
#[cfg(unix)]
fn check_wiring(ci: &str) {
    let ci = parse(ci);
    let jobs = &ci["jobs"];
    let matrix = jobs["gpu-shard"]["strategy"]["matrix"]
        .as_hash()
        .expect("gpu-shard's matrix");
    assert_eq!(matrix.len(), 1, "gpu-shard's matrix: {matrix:?}");
    assert_eq!(
        matrix[&Yaml::String("include".into())].as_str(),
        Some("${{ fromJSON(needs.gpu-touch.outputs.shards) }}"),
        "gpu-shard's matrix"
    );
    let outputs = jobs["gpu-touch"]["outputs"]
        .as_hash()
        .expect("gpu-touch's outputs");
    for key in ["run", "shards"] {
        assert_eq!(
            outputs
                .get(&Yaml::String(key.into()))
                .and_then(Yaml::as_str),
            Some(format!("${{{{ steps.touch.outputs.{key} }}}}").as_str()),
            "gpu-touch's output {key}"
        );
    }
    for job in ["gpu-touch", "kernel-wgsl"] {
        let steps = jobs[job]["steps"].as_vec().expect("the job's steps");
        let at = steps
            .iter()
            .position(|step| step["id"].as_str() == Some("touch"))
            .expect("the touch step");
        assert_eq!(
            steps[at]["run"].as_str(),
            Some("bash .github/scripts/gpu_touch.sh >> \"$GITHUB_OUTPUT\"")
        );
        assert_eq!(
            steps[at]["env"]["BASE_REF"].as_str(),
            Some("${{ github.base_ref }}")
        );
        for step in &steps[at + 1..] {
            assert_eq!(
                step["if"].as_str(),
                Some("steps.touch.outputs.kernel == 'true'"),
                "when a step of {job} runs"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn gpu_shards_are_the_ones_gpu_touch_asks_for() {
    check_wiring(CI);
}

#[cfg(all(unix, feature = "controls"))]
mod wiring_static {
    use super::*;
    negative_control!(
        gpu_shards_are_the_ones_gpu_touch_asks_for,
        "a matrix that isn't gpu-touch's",
        expected = "gpu-shard's matrix",
        check_wiring(&edited(
            CI,
            "include: ${{ fromJSON(needs.gpu-touch.outputs.shards) }}",
            "include: [{name: controls render, controls: render}]"
        ))
    );
}

#[cfg(all(unix, feature = "controls"))]
mod wiring_output {
    use super::*;
    negative_control!(
        gpu_shards_are_the_ones_gpu_touch_asks_for,
        "a gpu-touch that doesn't output the shards",
        expected = "gpu-touch's output shards",
        check_wiring(&edited(
            CI,
            "      shards: ${{ steps.touch.outputs.shards }}\n",
            ""
        ))
    );
}

#[cfg(all(unix, feature = "controls"))]
mod wiring_kernel {
    use super::*;
    negative_control!(
        gpu_shards_are_the_ones_gpu_touch_asks_for,
        "a kernel build on a render change",
        expected = "when a step of kernel-wgsl runs",
        check_wiring(&edited(
            CI,
            "if: steps.touch.outputs.kernel == 'true'",
            "if: steps.touch.outputs.run == 'true'"
        ))
    );
}

/// gpu-shard's step whose command starts with `command`.
#[cfg(unix)]
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
#[cfg(unix)]
fn check_controls_matrix(name: &str, ci: &str, touch: &str) {
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
    let shards = shards(name, touch);
    let mut named: Vec<&str> = shards
        .iter()
        .filter_map(|shard| shard["controls"].as_str())
        .flat_map(|names| names.split(','))
        .collect();
    named.sort_unstable();
    let mut want = xtask::controls::packages(&xtask::workspace_manifest()).expect("the packages");
    want.sort_unstable();
    assert_eq!(named, want, "the control shards' packages");
}

#[cfg(unix)]
#[test]
fn gpu_shards_cover_every_controls_package_once() {
    check_controls_matrix("controls", CI, TOUCH);
}

#[cfg(all(unix, feature = "controls"))]
mod controls_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_controls_package_once,
        "a shard that drops a package",
        expected = "the control shards' packages",
        check_controls_matrix(
            "ctl_controls_dropped",
            CI,
            &edited(
                TOUCH,
                "\"controls\": \"ledger,prin\"",
                "\"controls\": \"ledger\""
            )
        )
    );
}

#[cfg(all(unix, feature = "controls"))]
mod controls_twice {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_controls_package_once,
        "a package in two shards",
        expected = "the control shards' packages",
        check_controls_matrix(
            "ctl_controls_twice",
            CI,
            &edited(
                TOUCH,
                "\"controls\": \"engine\"",
                "\"controls\": \"engine,prin\""
            )
        )
    );
}

/// The suite shards are every slice 1 to n of the n the step splits into, once each, and one shard runs the golden
/// renders.
#[cfg(unix)]
fn check_suites_matrix(name: &str, ci: &str, touch: &str) {
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
    let shards = shards(name, touch);
    let mut slices: Vec<i64> = shards
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
    let goldens = shards
        .iter()
        .filter(|shard| shard["golden"].as_bool() == Some(true));
    assert_eq!(goldens.count(), 1, "the shards that run the golden renders");
}

#[cfg(unix)]
#[test]
fn gpu_shards_cover_every_suite_slice_and_the_golden_renders() {
    check_suites_matrix("suites", CI, TOUCH);
}

#[cfg(all(unix, feature = "controls"))]
mod slice_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_suite_slice_and_the_golden_renders,
        "a split into three, with two shards",
        expected = "the suite shards' slices",
        check_suites_matrix(
            "ctl_slice_dropped",
            &edited(
                CI,
                "hash:${{ matrix.slice }}/2",
                "hash:${{ matrix.slice }}/3"
            ),
            TOUCH
        )
    );
}

#[cfg(all(unix, feature = "controls"))]
mod golden_dropped {
    use super::*;
    negative_control!(
        gpu_shards_cover_every_suite_slice_and_the_golden_renders,
        "no shard runs the golden renders",
        expected = "the shards that run the golden renders",
        check_suites_matrix(
            "ctl_golden_dropped",
            CI,
            &edited(TOUCH, ", \"golden\": true", "")
        )
    );
}

/// One shard saves the caches, on main alone.
#[cfg(unix)]
fn check_one_shard_saves(name: &str, ci: &str, touch: &str) {
    let ci = parse(ci);
    let shards = shards(name, touch);
    let savers = shards
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

#[cfg(unix)]
#[test]
fn gpu_shards_save_each_cache_once_on_main() {
    check_one_shard_saves("saves", CI, TOUCH);
}

#[cfg(unix)]
negative_control!(
    gpu_shards_save_each_cache_once_on_main,
    "every shard saves the compiled kernel",
    expected = "a cache is saved by",
    check_one_shard_saves(
        "ctl_saves",
        &edited(
            CI,
            "        if: github.ref == 'refs/heads/main' && matrix.save == true\n",
            "        if: github.ref == 'refs/heads/main'\n"
        ),
        TOUCH
    )
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
