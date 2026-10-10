//! qa's tests for the ops PR #184 (branch `ops/golden-harness-ci`): a golden run builds validation's `golden_harness`
//! once per suite and renders its harness cases concurrently, so a suite takes cargo's build lock once and its cases
//! compile their shaders side by side. Written from what the change must keep, not from the code:
//!
//! - a suite builds the harness once, a suite with no harness case not at all, and spawns the built binary once per
//!   harness case (`qa_ops_gh_each_suite_builds_once`, `qa_ops_gh_real_cargo_builds_once`);
//! - a suite mixing harness and shader cases reports every case in case order, each harness case's stderr just
//!   before its report, whatever order the harness processes end in, so the output is byte for byte the output of a
//!   run whose cases end in turn (`qa_ops_gh_mixed_suite_is_byte_identical_to_in_turn`);
//! - a failed build is the error of the first harness case: the shader cases before it are reported, nothing after
//!   it is, and no harness process runs (`qa_ops_gh_failed_build_is_reported_at_the_first_harness_case`);
//! - a harness case that fails stops the run there, as running the cases in turn did, even when a later case fails
//!   first (`qa_ops_gh_mixed_suite_stops_at_the_first_failing_case`).
//!
//! The suites are built in a scratch workspace root of their own, from the repository's cases and their
//! `BASELINES.md` rows, so the shader and harness cases can be mixed in one suite (no checked-in suite mixes them).
//! `xtask golden` reads only its own workspace's fixtures, so each run is this test binary re-run as a child that
//! calls `xtask::golden::cli` on the scratch root, with `CARGO` set to a stand-in. Each test has its negative control
//! (R-176), which feeds its check an output differing in the one respect the test turns on.

use std::path::{Path, PathBuf};
use std::process::Command;

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::golden::{harness_width, HARNESS_MAGIC};

/// The environment variable that makes a re-run of this binary the child run, naming its scratch root.
const CHILD_ROOT: &str = "QA_OPS_GH_ROOT";
/// The child run's `xtask golden` arguments, one per line.
const CHILD_ARGS: &str = "QA_OPS_GH_ARGS";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("qa-ops-gh: workspace root")
        .to_path_buf()
}

/// When this process is a child run, runs `xtask golden` on its root and exits with its status, printing its error
/// as the xtask binary does. Every test calls it first.
fn child_entry() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let args = std::env::var(CHILD_ARGS).unwrap_or_default();
    let args: Vec<&str> = args.lines().collect();
    let code = match xtask::golden::cli(Path::new(&root), &args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("xtask: {e}");
            1
        }
    };
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}

/// A case to copy into a scratch suite: its new name and the repository case it copies (`<suite>/<case>`).
struct Copy {
    name: &'static str,
    from: &'static str,
}

/// Copies directory `from` into `to`, files only (the cases have no subdirectories).
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

/// A scratch workspace root at `root` holding `suites` (each a name and its cases), with `decisions.md` and a
/// `BASELINES.md` holding, for each copied case, its original's row under its new name.
fn scratch_root(root: &Path, suites: &[(&str, &[Copy])]) {
    let _ = std::fs::remove_dir_all(root);
    let golden = root.join("fixtures/golden");
    std::fs::create_dir_all(&golden).unwrap();
    std::fs::copy(repo().join("decisions.md"), root.join("decisions.md")).unwrap();
    let baselines = std::fs::read_to_string(repo().join("fixtures/golden/BASELINES.md")).unwrap();
    let mut rows = baselines.clone();
    for (suite, cases) in suites {
        for c in *cases {
            copy_dir(
                &repo().join("fixtures/golden").join(c.from),
                &golden.join(suite).join(c.name),
            );
            let cell = format!("| `{}` |", c.from);
            let row = baselines
                .lines()
                .find(|l| l.starts_with(&cell))
                .unwrap_or_else(|| panic!("qa-ops-gh: BASELINES.md has no row for {}", c.from));
            rows.push_str(&row.replacen(&cell, &format!("| `{suite}/{}` |", c.name), 1));
            rows.push('\n');
        }
    }
    std::fs::write(golden.join("BASELINES.md"), rows).unwrap();
}

/// The scene and size of the harness case `from` (`<suite>/<case>`), or `None` for a shader case.
fn harness_of(from: &str) -> Option<(String, u32, u32)> {
    let text = std::fs::read_to_string(repo().join("fixtures/golden").join(from).join("case.json"))
        .unwrap();
    let case: serde_json::Value = serde_json::from_str(&text).unwrap();
    let r = &case["render"];
    let scene = r["harness"].as_str()?.to_owned();
    Some((
        scene,
        r["width"].as_u64()? as u32,
        r["height"].as_u64()? as u32,
    ))
}

/// A float image of the harness's format, `width` × `height`, every pixel (0, 0, 0, 1).
fn zero_image(width: u32, height: u32) -> Vec<u8> {
    let mut out = HARNESS_MAGIC.to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    for _ in 0..width * height {
        for c in [0.0f32, 0.0, 0.0, 1.0] {
            out.extend(c.to_le_bytes());
        }
    }
    out
}

/// A stand-in harness and cargo in `dir`. The harness logs its scene (`started.log`), waits until the scene named in
/// `wait/<scene>` has ended (if any, for at most 20 s), writes two stderr lines naming its scene, logs its end
/// (`done.log`), and exits 1 for a scene marked in `fail/`, else writes the scene's image. The cargo logs its
/// arguments (`cargo.log`) and answers a build with the harness's artifact message, or, where `build_fails`, with a
/// line on stderr and exit 101.
struct StandIn {
    dir: PathBuf,
}

impl StandIn {
    fn new(dir: &Path, build_fails: bool) -> StandIn {
        let _ = std::fs::remove_dir_all(dir);
        for sub in ["scenes", "wait", "fail", "done"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        let d = dir.display();
        let harness = dir.join("golden_harness");
        validation::spawn::write_executable(
            &harness,
            format!(
                "#!/bin/sh\nD='{d}'\ns=\"$2\"\necho \"$s\" >> \"$D/started.log\"\nw=\"$(cat \"$D/wait/$s\" 2>/dev/null)\"\n\
                 if [ -n \"$w\" ]; then i=0; while [ ! -e \"$D/done/$w\" ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done; fi\n\
                 echo \"stand-in harness: $s: first\" >&2\necho \"stand-in harness: $s: second\" >&2\n\
                 echo \"$s\" >> \"$D/done.log\"\ntouch \"$D/done/$s\"\n\
                 if [ -e \"$D/fail/$s\" ]; then exit 1; fi\ncat \"$D/scenes/$s\"\n"
            ),
        )
        .unwrap();
        let message = serde_json::json!({
            "reason": "compiler-artifact",
            "target": { "name": "golden_harness", "kind": ["bin"] },
            "executable": harness.to_string_lossy(),
        });
        let answer = if build_fails {
            "echo 'stand-in cargo: error: could not compile golden_harness' >&2\nexit 101\n"
                .to_owned()
        } else {
            format!("echo '{message}'\n")
        };
        validation::spawn::write_executable(
            &dir.join("cargo"),
            format!("#!/bin/sh\necho \"$*\" >> '{d}/cargo.log'\n{answer}"),
        )
        .unwrap();
        StandIn {
            dir: dir.to_path_buf(),
        }
    }

    /// Serves `scene` as a `width` × `height` image, waiting for `wait` to end first, and failing where `fails`.
    fn scene(&self, scene: &str, width: u32, height: u32, wait: Option<&str>, fails: bool) {
        std::fs::write(
            self.dir.join("scenes").join(scene),
            zero_image(width, height),
        )
        .unwrap();
        std::fs::write(self.dir.join("wait").join(scene), wait.unwrap_or("")).unwrap();
        if fails {
            std::fs::write(self.dir.join("fail").join(scene), "").unwrap();
        }
    }

    fn read(&self, file: &str) -> String {
        std::fs::read_to_string(self.dir.join(file)).unwrap_or_default()
    }
}

/// A child run's output: its stdout and stderr as one stream, in the order written, and whether it passed.
struct Run {
    out: String,
    ok: bool,
}

/// Re-runs this binary's test `test` as the child run of `xtask golden <args>` on `root`, with `CARGO` set to `cargo`
/// and its outputs under `target`.
fn child_run(test: &str, root: &Path, args: &[&str], cargo: &Path, target: &Path) -> Run {
    let output = Command::new("sh")
        .args([
            "-c",
            "exec \"$0\" --exact \"$1\" --nocapture --test-threads=1 2>&1",
            std::env::current_exe().unwrap().to_str().unwrap(),
            test,
        ])
        .env(CHILD_ROOT, root)
        .env(CHILD_ARGS, args.join("\n"))
        .env("CARGO", cargo)
        .env("CARGO_TARGET_DIR", target)
        .timed_output()
        .expect("qa-ops-gh: run the child");
    Run {
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        ok: output.status.success(),
    }
}

/// The run's lines from `xtask golden` and the stand-ins, in order: each case's report as `report <suite>/<case>`,
/// each stand-in stderr line as itself, and the run's error as `error <text>`.
fn stream(out: &str) -> Vec<String> {
    out.lines()
        .filter_map(|l| {
            if l.starts_with("stand-in ") {
                return Some(l.to_owned());
            }
            if let Some(e) = l.strip_prefix("xtask: ") {
                return Some(format!("error {e}"));
            }
            let rest = l.strip_prefix("xtask golden: ")?;
            let (name, _) = rest.split_once(": ")?;
            name.contains('/').then(|| format!("report {name}"))
        })
        .collect()
}

/// The scratch directory of `name`.
fn scratch(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("qa_ops_gh")
        .join(name)
}

/// The mixed suite: shader cases and harness cases interleaved, in the runner's (sorted) order.
const MIXED: &[Copy] = &[
    Copy {
        name: "a_gradient",
        from: "selftest/gradient",
    },
    Copy {
        name: "b_d_min",
        from: "m1-numeric/d_min",
    },
    Copy {
        name: "c_gradient",
        from: "selftest/gradient",
    },
    Copy {
        name: "d_ftle",
        from: "m1-numeric/ftle",
    },
    Copy {
        name: "e_diffusion",
        from: "m1-numeric/diffusion",
    },
    Copy {
        name: "f_gradient",
        from: "selftest/gradient",
    },
    Copy {
        name: "g_length_view",
        from: "m1-numeric/length_view",
    },
];

/// The harness cases of `cases`: their index, scene, width and height.
fn harness_cases(cases: &[Copy]) -> Vec<(usize, String, u32, u32)> {
    cases
        .iter()
        .enumerate()
        .filter_map(|(k, c)| harness_of(c.from).map(|(s, w, h)| (k, s, w, h)))
        .collect()
}

/// The stream a run of the mixed suite writes when its cases run in turn, up to `end` cases, with `error` after.
fn in_turn(suite: &str, cases: &[Copy], end: usize, error: Option<(usize, &str)>) -> Vec<String> {
    let mut want = Vec::new();
    for (k, c) in cases.iter().enumerate().take(end) {
        if let Some((scene, ..)) = harness_of(c.from) {
            want.push(format!("stand-in harness: {scene}: first"));
            want.push(format!("stand-in harness: {scene}: second"));
        }
        if error.is_none_or(|(e, _)| k < e) {
            want.push(format!("report {suite}/{}", c.name));
        }
    }
    want.extend(error.map(|(_, e)| format!("error {e}")));
    want
}

// ---------------------------------------------------------------------------------------------------------------
// Byte-identical to a run whose cases end in turn.

/// Checks the two runs of the mixed suite: `in_turn_run`, whose harness processes ended in case order (`done_a`),
/// and `reversed_run`, whose first ones ended in reverse (`done_b`). Both report the stream a run in turn writes, and
/// are byte for byte the same.
fn check_byte_identical(
    in_turn_run: &Run,
    done_a: &[String],
    reversed_run: &Run,
    done_b: &[String],
    want: &[String],
) {
    assert_ne!(
        done_a, done_b,
        "qa-ops-gh: both runs' harness processes ended in the same order, so the order kept is not shown"
    );
    // The stand-in's images are black, so the harness cases fail their references and the run ends in an error
    // naming them; the order checked is the cases' and their stderr's.
    let cases = |out: &str| -> Vec<String> {
        stream(out)
            .into_iter()
            .filter(|l| !l.starts_with("error "))
            .collect()
    };
    assert_eq!(
        cases(&in_turn_run.out),
        want,
        "qa-ops-gh: the in-turn run's cases and stderr are not in case order:\n{}",
        in_turn_run.out
    );
    assert_eq!(
        cases(&reversed_run.out),
        want,
        "qa-ops-gh: the reversed run's cases and stderr are not in case order:\n{}",
        reversed_run.out
    );
    assert!(
        in_turn_run.out == reversed_run.out,
        "qa-ops-gh: the output is not byte-identical to the in-turn run's:\n--- in turn\n{}\n--- reversed\n{}",
        in_turn_run.out,
        reversed_run.out
    );
    assert_eq!(
        in_turn_run.ok, reversed_run.ok,
        "qa-ops-gh: the runs' statuses differ"
    );
}

/// Runs the mixed suite with the harness cases ending as `wait` orders them (the scene each waits for).
fn mixed_run(test: &str, tag: &str, wait: impl Fn(usize) -> Option<usize>) -> (Run, Vec<String>) {
    let root = scratch(test).join("root");
    let jobs = harness_cases(MIXED);
    let stand_in = StandIn::new(&scratch(test).join(tag), false);
    for (j, (_, scene, w, h)) in jobs.iter().enumerate() {
        stand_in.scene(scene, *w, *h, wait(j).map(|i| jobs[i].1.as_str()), false);
    }
    let run = child_run(
        test,
        &root,
        &["mixed"],
        &stand_in.dir.join("cargo"),
        &scratch(test).join("target"),
    );
    let done = stand_in
        .read("done.log")
        .lines()
        .map(str::to_owned)
        .collect();
    (run, done)
}

#[test]
fn qa_ops_gh_mixed_suite_is_byte_identical_to_in_turn() {
    child_entry();
    let test = "qa_ops_gh_mixed_suite_is_byte_identical_to_in_turn";
    scratch_root(&scratch(test).join("root"), &[("mixed", MIXED)]);
    let jobs = harness_cases(MIXED).len();
    // In turn: each harness case waits for the one before. Reversed: within the processes running at once, each
    // waits for the one after, so they end last first.
    let wide = harness_width().min(jobs);
    assert!(
        wide > 1,
        "qa-ops-gh: one harness process at a time shows no order"
    );
    let (a, done_a) = mixed_run(test, "in_turn", |j| j.checked_sub(1));
    let (b, done_b) = mixed_run(test, "reversed", |j| (j + 1 < wide).then_some(j + 1));
    check_byte_identical(
        &a,
        &done_a,
        &b,
        &done_b,
        &in_turn("mixed", MIXED, MIXED.len(), None),
    );
}

negative_control!(
    qa_ops_gh_mixed_suite_is_byte_identical_to_in_turn,
    "a reversed run writing its harness stderr where each process ended, not before its case's report",
    expected = "qa-ops-gh: the reversed run's cases and stderr are not in case order",
    {
        let want = in_turn("mixed", MIXED, MIXED.len(), None);
        let text = |lines: &[String]| -> String {
            lines
                .iter()
                .map(|l| match l.strip_prefix("report ") {
                    Some(n) => format!("xtask golden: {n}: pass\n"),
                    None => format!("{l}\n"),
                })
                .collect()
        };
        // The same lines, the stderr of the second harness case moved before the first case's report.
        let mut moved = want.clone();
        let second = moved
            .iter()
            .position(|l| l.contains("ftle: first"))
            .unwrap();
        let lines: Vec<String> = moved.drain(second..second + 2).collect();
        for (k, l) in lines.into_iter().enumerate() {
            moved.insert(k, l);
        }
        check_byte_identical(
            &Run { out: text(&want), ok: true },
            &["a".to_owned()],
            &Run { out: text(&moved), ok: true },
            &["b".to_owned()],
            &want,
        )
    }
);

// ---------------------------------------------------------------------------------------------------------------
// A failed build is the first harness case's error.

/// Checks the run of the mixed suite whose harness build failed: the shader cases before the first harness case
/// reported, then the build's error, nothing after; cargo called once to build, and no harness process run.
fn check_failed_build(run: &Run, cargo_log: &str, started: &str) {
    assert!(
        !run.ok,
        "qa-ops-gh: the run passed with a failed build:\n{}",
        run.out
    );
    let first = harness_cases(MIXED)[0].0;
    let error = "cargo build -p validation --bin golden_harness failed (exit status: 101)";
    let got: Vec<String> = stream(&run.out)
        .into_iter()
        .filter(|l| !l.starts_with("stand-in cargo"))
        .collect();
    let mut want: Vec<String> = MIXED[..first]
        .iter()
        .map(|c| format!("report mixed/{}", c.name))
        .collect();
    want.push(format!("error {error}"));
    assert_eq!(
        got, want,
        "qa-ops-gh: a failed build is not reported at the first harness case:\n{}",
        run.out
    );
    let builds: Vec<&str> = cargo_log
        .lines()
        .filter(|l| l.contains("golden_harness"))
        .collect();
    assert_eq!(
        builds.len(),
        1,
        "qa-ops-gh: the failed build was tried {} times, not once: {builds:?}",
        builds.len()
    );
    assert!(
        started.is_empty(),
        "qa-ops-gh: a harness process ran with no harness built:\n{started}"
    );
}

#[test]
fn qa_ops_gh_failed_build_is_reported_at_the_first_harness_case() {
    child_entry();
    let test = "qa_ops_gh_failed_build_is_reported_at_the_first_harness_case";
    let root = scratch(test).join("root");
    scratch_root(&root, &[("mixed", MIXED)]);
    let stand_in = StandIn::new(&scratch(test).join("stand-in"), true);
    for (_, scene, w, h) in harness_cases(MIXED) {
        stand_in.scene(&scene, w, h, None, false);
    }
    let run = child_run(
        test,
        &root,
        &["mixed"],
        &stand_in.dir.join("cargo"),
        &scratch(test).join("target"),
    );
    check_failed_build(
        &run,
        &stand_in.read("cargo.log"),
        &stand_in.read("started.log"),
    );
}

negative_control!(
    qa_ops_gh_failed_build_is_reported_at_the_first_harness_case,
    "a failed build reported before the shader case that precedes the first harness case",
    expected = "qa-ops-gh: a failed build is not reported at the first harness case",
    check_failed_build(
        &Run {
            out: "xtask: golden suite(s) failed: cargo build -p validation --bin golden_harness failed (exit status: 101)\n"
                .to_owned(),
            ok: false,
        },
        "build -p validation --bin golden_harness\n",
        ""
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The first failing case, in case order, stops the run.

/// Checks the run of the mixed suite whose harness cases `first` and `later` (indices among its harness cases)
/// failed, `later` ending first: it stops at `first`'s case, as a run in turn did, reporting nothing after it.
fn check_stops_at_first(run: &Run, first: usize, later: usize) {
    assert!(
        !run.ok,
        "qa-ops-gh: the run passed with failing cases:\n{}",
        run.out
    );
    let jobs = harness_cases(MIXED);
    let (k, scene, ..) = &jobs[first];
    let error = format!("golden_harness --scene {scene} failed (exit status: 1)");
    let want = in_turn("mixed", MIXED, k + 1, Some((*k, &error)));
    let got = stream(&run.out);
    assert_eq!(
        got, want,
        "qa-ops-gh: the run did not stop at the first failing case in case order:\n{}",
        run.out
    );
    let later_scene = &jobs[later].1;
    assert!(
        !run.out.contains(&format!("--scene {later_scene} failed")),
        "qa-ops-gh: the later failing case is reported:\n{}",
        run.out
    );
}

#[test]
fn qa_ops_gh_mixed_suite_stops_at_the_first_failing_case() {
    child_entry();
    let test = "qa_ops_gh_mixed_suite_stops_at_the_first_failing_case";
    let root = scratch(test).join("root");
    scratch_root(&root, &[("mixed", MIXED)]);
    let jobs = harness_cases(MIXED);
    let wide = harness_width().min(jobs.len());
    assert!(
        wide > 1,
        "qa-ops-gh: one harness process at a time ends nothing out of order"
    );
    // Harness cases 0 and 1 fail; case 0 waits for case 1, so the later failure ends first.
    let (first, later) = (0, 1);
    let stand_in = StandIn::new(&scratch(test).join("stand-in"), false);
    for (j, (_, scene, w, h)) in jobs.iter().enumerate() {
        let wait = (j == first).then(|| jobs[later].1.as_str());
        stand_in.scene(scene, *w, *h, wait, j == first || j == later);
    }
    let run = child_run(
        test,
        &root,
        &["mixed"],
        &stand_in.dir.join("cargo"),
        &scratch(test).join("target"),
    );
    let done: Vec<String> = stand_in
        .read("done.log")
        .lines()
        .map(str::to_owned)
        .collect();
    let pos = |s: &str| done.iter().position(|d| d == s);
    assert!(
        pos(&jobs[later].1) < pos(&jobs[first].1),
        "qa-ops-gh: the later failing case did not end first, so the order kept is not shown: {done:?}"
    );
    check_stops_at_first(&run, first, later);
}

negative_control!(
    qa_ops_gh_mixed_suite_stops_at_the_first_failing_case,
    "a run that stops at the failing case that ended first, not the first in case order",
    expected = "qa-ops-gh: the run did not stop at the first failing case in case order",
    {
        let jobs = harness_cases(MIXED);
        let (k, ..) = jobs[1];
        let error = format!(
            "golden_harness --scene {} failed (exit status: 1)",
            jobs[1].1
        );
        let lines = in_turn("mixed", MIXED, k + 1, Some((k, &error)));
        let out: String = lines
            .iter()
            .map(|l| {
                if let Some(n) = l.strip_prefix("report ") {
                    format!("xtask golden: {n}: pass\n")
                } else if let Some(e) = l.strip_prefix("error ") {
                    format!("xtask: {e}\n")
                } else {
                    format!("{l}\n")
                }
            })
            .collect();
        check_stops_at_first(&Run { out, ok: false }, 0, 1)
    }
);

// ---------------------------------------------------------------------------------------------------------------
// One build per suite, none for a suite without harness cases.

/// Checks an `--all` run of suites with `harness_suites` harness suites and `harness_cases` harness cases in all:
/// cargo built the harness once per harness suite, and the harness ran once per harness case.
fn check_builds_per_suite(
    cargo_log: &str,
    started: &str,
    harness_suites: usize,
    harness_cases: usize,
) {
    let builds: Vec<&str> = cargo_log
        .lines()
        .filter(|l| l.contains("golden_harness"))
        .collect();
    assert_eq!(
        builds.len(),
        harness_suites,
        "qa-ops-gh: the harness was built {} times for {harness_suites} suites with harness cases: {builds:?}",
        builds.len()
    );
    assert!(
        builds.iter().all(|b| b.starts_with("build ")),
        "qa-ops-gh: the harness was not built with `cargo build`: {builds:?}"
    );
    assert_eq!(
        started.lines().count(),
        harness_cases,
        "qa-ops-gh: the harness ran {} times for {harness_cases} harness cases:\n{started}",
        started.lines().count()
    );
}

#[test]
fn qa_ops_gh_each_suite_builds_once() {
    child_entry();
    let test = "qa_ops_gh_each_suite_builds_once";
    let root = scratch(test).join("root");
    let shaders: &[Copy] = &[
        Copy {
            name: "a",
            from: "selftest/gradient",
        },
        Copy {
            name: "b",
            from: "selftest/gradient",
        },
    ];
    let one: &[Copy] = &[Copy {
        name: "only",
        from: "m1-numeric/ftle",
    }];
    scratch_root(
        &root,
        &[("mixed", MIXED), ("shaders", shaders), ("one", one)],
    );
    let stand_in = StandIn::new(&scratch(test).join("stand-in"), false);
    for (_, scene, w, h) in harness_cases(MIXED).into_iter().chain(harness_cases(one)) {
        stand_in.scene(&scene, w, h, None, false);
    }
    let run = child_run(
        test,
        &root,
        &["--all"],
        &stand_in.dir.join("cargo"),
        &scratch(test).join("target"),
    );
    let reported = stream(&run.out)
        .iter()
        .filter(|l| l.starts_with("report "))
        .count();
    assert_eq!(
        reported,
        MIXED.len() + shaders.len() + one.len(),
        "qa-ops-gh: --all did not report every case:\n{}",
        run.out
    );
    check_builds_per_suite(
        &stand_in.read("cargo.log"),
        &stand_in.read("started.log"),
        2,
        harness_cases(MIXED).len() + 1,
    );
}

negative_control!(
    qa_ops_gh_each_suite_builds_once,
    "a run that builds the harness for the suite with no harness case too",
    expected = "qa-ops-gh: the harness was built 3 times for 2 suites",
    check_builds_per_suite(
        "build -p validation --bin golden_harness\nbuild -p validation --bin golden_harness\nbuild -p validation --bin golden_harness\n",
        "a\nb\n",
        2,
        2
    )
);

// ---------------------------------------------------------------------------------------------------------------
// The real cargo, behind a logging stand-in, as qa_TASK-M0-22's golden step sees it.

/// Checks a real `xtask golden m1-outcome` run through a stand-in that logs each cargo call and passes it on: it
/// passed, reporting its cases in order, and cargo was called once for the harness, with `build`, never `run`.
fn check_real_build_once(run: &Run, cargo_log: &str, cases: &[String]) {
    assert!(run.ok, "qa-ops-gh: the real run failed:\n{}", run.out);
    let reported: Vec<String> = stream(&run.out)
        .into_iter()
        .filter_map(|l| l.strip_prefix("report m1-outcome/").map(str::to_owned))
        .collect();
    assert_eq!(
        reported, cases,
        "qa-ops-gh: the real run's reports:\n{}",
        run.out
    );
    let calls: Vec<&str> = cargo_log
        .lines()
        .filter(|l| l.contains("golden_harness"))
        .collect();
    assert!(
        calls.len() == 1 && calls[0].starts_with("build "),
        "qa-ops-gh: cargo was called {} times for the harness, not once to build it: {calls:?}",
        calls.len()
    );
}

#[test]
fn qa_ops_gh_real_cargo_builds_once() {
    child_entry();
    let dir = scratch("qa_ops_gh_real_cargo_builds_once");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let real = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            "#!/bin/sh\necho \"$*\" >> '{}/cargo.log'\nexec '{real}' \"$@\"\n",
            dir.display()
        ),
    )
    .unwrap();
    let suite = repo().join("fixtures/golden/m1-outcome");
    let mut cases: Vec<String> = std::fs::read_dir(&suite)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.path()
                .join("case.json")
                .exists()
                .then(|| e.file_name().to_string_lossy().into_owned())
        })
        .collect();
    cases.sort();
    let mut command = Command::new("sh");
    command
        .args([
            "-c",
            "exec \"$0\" golden m1-outcome 2>&1",
            env!("CARGO_BIN_EXE_xtask"),
        ])
        .env("CARGO", &cargo);
    let output = command.timed_output().expect("qa-ops-gh: run xtask golden");
    let run = Run {
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        ok: output.status.success(),
    };
    check_real_build_once(
        &run,
        &std::fs::read_to_string(dir.join("cargo.log")).unwrap_or_default(),
        &cases,
    );
}

negative_control!(
    qa_ops_gh_real_cargo_builds_once,
    "a run that calls `cargo run` for the harness once per case",
    expected = "qa-ops-gh: cargo was called 2 times for the harness",
    check_real_build_once(
        &Run {
            out: "xtask golden: m1-outcome/a: pass\nxtask golden: m1-outcome/b: pass\n".to_owned(),
            ok: true,
        },
        "run --quiet -p validation --bin golden_harness -- --scene a\nrun --quiet -p validation --bin golden_harness -- --scene b\n",
        &["a".to_owned(), "b".to_owned()]
    )
);
