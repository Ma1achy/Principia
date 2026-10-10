//! `cargo xtask golden`'s harness case kind (RQ-229, decided per R-369; TASK-M1-09): a case's `render` names a
//! synthetic scene, `{ "harness": "<scene>", "width": <w>, "height": <h> }`; the runner spawns validation's
//! `golden_harness` binary, which writes the scene's `Rgba32Float` image, and quantises it in its own pass:
//! - the config's fields: a harness case has `harness`, `width` and `height` alone (`golden_harness_config_*`);
//! - the float image's format, refused unless whole and of the case's size (`golden_harness_decode_*`);
//! - a harness case renders through the binary, its stored levels the floats rounded half to even, and has no
//!   automatic output (`golden_harness_renders_*`);
//! - a golden run builds the binary once, with `cargo build`, finds it in cargo's JSON messages, and spawns it per
//!   case, so it takes cargo's build lock once, not once per case (`golden_harness_executable_*`,
//!   `golden_harness_builds_once_per_run`);
//! - the cases' harness processes run up to `harness_width` at once, and the run reports the cases, and the first
//!   that fails, in case order whatever order they finish in (`golden_harness_in_parallel_*`,
//!   `golden_harness_cases_render_concurrently`, `golden_harness_failure_*`).
//!
//! Each test registers its negative control (R-176).

use std::path::Path;

use serde_json::json;
use validation::negative_control;
use validation::spawn::Spawn;
use xtask::golden::{
    decode_harness, harness_executable, harness_width, in_parallel, Config, Output, Renderer,
    HARNESS_MAGIC,
};

fn config(render: serde_json::Value) -> Result<Config, String> {
    Config::from_render(&render)
}

/// Checks that `render` is refused with a message containing `why`.
fn check_refused(render: serde_json::Value, why: &str) {
    match config(render.clone()) {
        Ok(_) => panic!("{render} is accepted"),
        Err(e) => assert!(e.contains(why), "{render} is refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_config_names_a_scene() {
    let c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("ftle"));
    assert_eq!(c.size(), Ok((64, 8)));
    let shader = json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 });
    assert_eq!(config(shader).map(|c| c.harness().is_none()), Ok(true));
    check_refused(
        json!({ "harness": "ftle", "width": 64 }),
        "`render.height` is missing",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "`render.shader` is not a render field",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "constants": { "k": 1 } }),
        "`render.constants` is not a render field",
    );
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "prepend": ["lib/coords.wgsl"] }),
        "`render.prepend` is not a render field",
    );
    let prepended = json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1, "prepend": ["lib/coords.wgsl"] });
    assert_eq!(config(prepended).map(|c| c.harness().is_none()), Ok(true));
    check_refused(
        json!({ "harness": 3, "width": 64, "height": 8 }),
        "`render.harness` is not a string",
    );
    check_refused(
        json!({ "shader": "a.wgsl", "width": 1, "height": 1 }),
        "`render.fragment` is missing",
    );
}

negative_control!(
    golden_harness_config_names_a_scene,
    "a harness case with a shader is refused, so the check of its acceptance fails",
    expected = "is refused as",
    check_refused(
        json!({ "harness": "ftle", "width": 64, "height": 8, "shader": "a.wgsl" }),
        "accepted"
    )
);

/// Checks that setting `field` on `c` is refused, naming the fields.
fn check_set_refused(c: &mut Config, field: &str) {
    match c.set(field, json!(1)) {
        Ok(()) => panic!("`{field}` is set"),
        Err(e) => assert!(e.contains("is not a field"), "`{field}`: {e}"),
    }
}

#[test]
fn golden_harness_config_sets_its_own_fields() {
    let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    c.set("harness", json!("d_min"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(c.harness(), Some("d_min"));
    c.set("width", json!(32)).unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut c, "constants.k");
    check_set_refused(&mut c, "shader");
    let mut s = config(json!({ "shader": "a.wgsl", "fragment": "f", "width": 1, "height": 1 }))
        .unwrap_or_else(|e| panic!("{e}"));
    s.set("constants.k", json!(2))
        .unwrap_or_else(|e| panic!("{e}"));
    check_set_refused(&mut s, "harness");
}

negative_control!(
    golden_harness_config_sets_its_own_fields,
    "a harness case's own field is settable, so expecting it refused fails",
    expected = "is set",
    {
        let mut c = config(json!({ "harness": "ftle", "width": 64, "height": 8 }))
            .unwrap_or_else(|e| panic!("{e}"));
        check_set_refused(&mut c, "harness");
    }
);

/// The float image of `width` × `height` pixels, each `(k, 0, 0, 1)` for its index `k`, with `extra` bytes more or
/// fewer.
fn image(width: u32, height: u32, extra: isize) -> Vec<u8> {
    let mut out = HARNESS_MAGIC.to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    for k in 0..width * height {
        for c in [k as f32, 0.0, 0.0, 1.0] {
            out.extend(c.to_le_bytes());
        }
    }
    match extra {
        0 => {}
        e if e > 0 => out.extend(std::iter::repeat_n(0u8, e as usize)),
        e => out.truncate(out.len() - e.unsigned_abs()),
    }
    out
}

/// Checks that `bytes` is refused as a `width` × `height` image with a message containing `why`.
fn check_decode_refused(bytes: &[u8], width: u32, height: u32, why: &str) {
    match decode_harness(bytes, width, height) {
        Ok(_) => panic!("the image is accepted"),
        Err(e) => assert!(e.contains(why), "refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_decode_is_whole_and_sized() {
    let floats = decode_harness(&image(3, 2, 0), 3, 2).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 3 * 2 * 16);
    assert_eq!(&floats[16..20], &1.0f32.to_le_bytes(), "pixel 1's red");
    assert_eq!(decode_harness(&image(0, 0, 0), 0, 0), Ok(Vec::new()));
    check_decode_refused(&image(3, 2, 1), 3, 2, "not 96");
    check_decode_refused(&image(3, 2, -1), 3, 2, "not 96");
    check_decode_refused(
        &image(3, 2, 0),
        2,
        3,
        "the scene is 3 × 2, but the case says 2 × 3",
    );
    check_decode_refused(&image(3, 2, 0), 3, 3, "the scene is 3 × 2");
    check_decode_refused(&image(3, 2, 0)[1..], 3, 2, "magic");
    check_decode_refused(&HARNESS_MAGIC[..], 3, 2, "no size");
    check_decode_refused(&image(3, 2, 0)[..15], 3, 2, "no size");
}

negative_control!(
    golden_harness_decode_is_whole_and_sized,
    "a whole image of the right size is accepted",
    expected = "the image is accepted",
    check_decode_refused(&image(3, 2, 0), 3, 2, "not 96")
);

/// Checks that `levels` (RGB bytes) are `floats` rounded half to even, per channel.
fn check_levels(floats: &[[f32; 4]], levels: &[u8]) {
    for (k, p) in floats.iter().enumerate() {
        for c in 0..3 {
            let want = (p[c].clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
            assert_eq!(
                levels[3 * k + c],
                want,
                "pixel {k} channel {c}: {} is not stored as {want}",
                p[c]
            );
        }
    }
}

#[test]
fn golden_harness_renders_through_the_binary() {
    let renderer = Renderer::new().unwrap_or_else(|e| panic!("{e}"));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let c = config(json!({ "harness": "length_ramp", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    let floats = renderer
        .render_float(dir, &c)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(floats.len(), 64 * 8);
    let image = renderer.render(dir, &c).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((image.width, image.height), (64, 8));
    check_levels(&floats, &image.rgb);
    let distinct: std::collections::BTreeSet<[u8; 3]> =
        (0..64).map(|x| image.pixel(x, 0)).collect();
    assert!(
        distinct.len() > 4,
        "the scene's samples differ: {distinct:?}"
    );
    match renderer.render_output(dir, &c, Output::Automatic) {
        Ok(_) => panic!("a harness case rendered with the automatic output"),
        Err(e) => assert!(e.contains("only the quantised output"), "{e}"),
    }
    let wrong = config(json!({ "harness": "length_ramp", "width": 32, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &wrong) {
        Ok(_) => panic!("a case of the wrong size rendered"),
        Err(e) => assert!(
            e.contains("the scene is 64 × 8, but the case says 32 × 8"),
            "{e}"
        ),
    }
    let unknown = config(json!({ "harness": "no_such_scene", "width": 64, "height": 8 }))
        .unwrap_or_else(|e| panic!("{e}"));
    match renderer.render(dir, &unknown) {
        Ok(_) => panic!("an unknown scene rendered"),
        Err(e) => assert!(
            e.contains("golden_harness --scene no_such_scene failed"),
            "{e}"
        ),
    }
}

negative_control!(
    golden_harness_renders_through_the_binary,
    "levels rounded half away from zero, not to even, at a tie, are not the quantise pass's",
    expected = "is not stored as",
    check_levels(&[[0.5 / 255.0, 0.0, 0.0, 1.0]], &[1, 0, 0])
);

/// cargo's `compiler-artifact` message for the target `name`, with `executable` (JSON `null` when `None`).
fn artifact(name: &str, executable: Option<&str>) -> String {
    json!({
        "reason": "compiler-artifact",
        "target": { "name": name, "kind": ["bin"] },
        "executable": executable,
    })
    .to_string()
}

/// Checks that `messages` are refused with a message containing `why`.
fn check_executable_refused(messages: &str, why: &str) {
    match harness_executable(messages) {
        Ok(path) => panic!("{} is found in {messages:?}", path.display()),
        Err(e) => assert!(e.contains(why), "refused as `{e}`, not `{why}`"),
    }
}

#[test]
fn golden_harness_executable_is_the_one_artifact() {
    let messages = [
        "plain text, not a message".to_owned(),
        artifact("validation", None),
        artifact("gate", Some("/t/debug/gate")),
        json!({ "reason": "build-script-executed", "target": { "name": "golden_harness" }, "executable": "/x" })
            .to_string(),
        artifact("golden_harness", Some("/t/debug/golden_harness")),
        json!({ "reason": "build-finished", "success": true }).to_string(),
    ]
    .join("\n");
    assert_eq!(
        harness_executable(&messages),
        Ok(std::path::PathBuf::from("/t/debug/golden_harness"))
    );
    check_executable_refused("", "named no golden_harness executable");
    check_executable_refused(
        &artifact("golden_harness", None),
        "named no golden_harness executable",
    );
    check_executable_refused(
        &artifact("gate", Some("/t/debug/gate")),
        "named no golden_harness executable",
    );
    check_executable_refused(
        &json!({ "reason": "compiler-message", "target": { "name": "golden_harness" }, "executable": "/x" })
            .to_string(),
        "named no golden_harness executable",
    );
    let twice = [
        artifact("golden_harness", Some("/a/golden_harness")),
        artifact("golden_harness", Some("/b/golden_harness")),
    ]
    .join("\n");
    check_executable_refused(&twice, "named 2 golden_harness executables");
    check_executable_refused("{not json", "cargo's message `{not json`");
}

negative_control!(
    golden_harness_executable_is_the_one_artifact,
    "the harness's one artifact is found, so expecting it refused fails",
    expected = "is found in",
    check_executable_refused(
        &artifact("golden_harness", Some("/t/debug/golden_harness")),
        "named no golden_harness executable"
    )
);

/// A harness case of a suite: its directory's name, scene, width and height.
struct HarnessCase {
    name: String,
    scene: String,
    width: u32,
    height: u32,
}

/// The harness cases of `suite`, in the order the runner takes them (its directories' names, sorted).
fn harness_cases(suite: &str) -> Vec<HarnessCase> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/golden")
        .join(suite);
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let Ok(text) = std::fs::read_to_string(entry.path().join("case.json")) else {
            continue;
        };
        let case: serde_json::Value = serde_json::from_str(&text).unwrap();
        let render = &case["render"];
        if let Some(scene) = render["harness"].as_str() {
            let dim = |k: &str| render[k].as_u64().unwrap() as u32;
            cases.push(HarnessCase {
                name: entry.file_name().to_string_lossy().into_owned(),
                scene: scene.to_owned(),
                width: dim("width"),
                height: dim("height"),
            });
        }
    }
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    cases
}

/// The suite every stand-in run renders: all its cases are harness cases.
const SUITE: &str = "m1-numeric";

/// A run of `xtask golden` on [`SUITE`] through a stand-in cargo, which answers the build with a stand-in harness,
/// and that harness, which serves each scene's float image. The harness logs its arguments (`harness.log`), sleeps
/// for the scene's delay (`delays/<scene>`, seconds), logs how many harness processes are running as it ends
/// (`peaks.log`) and the scene it ended (`done.log`), writes `stand-in harness: <scene>` to its stderr, and exits 1
/// for a scene marked in `fail/`. `out` is xtask's stdout and stderr, as one stream, in the order written.
struct StandIn {
    dir: std::path::PathBuf,
    cases: Vec<HarnessCase>,
    out: String,
    ok: bool,
}

impl StandIn {
    /// Runs it in a directory of its own named `name`, each case `k` of the suite's harness cases sleeping `delay(k)`
    /// seconds and failing where `fails(k)`.
    fn run(name: &str, delay: impl Fn(usize) -> f64, fails: impl Fn(usize) -> bool) -> StandIn {
        let cases = harness_cases(SUITE);
        assert!(
            cases.len() > 2,
            "{SUITE} has {} harness cases, too few to show anything",
            cases.len()
        );
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["scenes", "delays", "fail", "running"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        for (k, c) in cases.iter().enumerate() {
            std::fs::write(
                dir.join("scenes").join(&c.scene),
                image(c.width, c.height, 0),
            )
            .unwrap();
            std::fs::write(dir.join("delays").join(&c.scene), format!("{}", delay(k))).unwrap();
            if fails(k) {
                std::fs::write(dir.join("fail").join(&c.scene), "").unwrap();
            }
        }
        let d = dir.display();
        let harness = dir.join("golden_harness");
        validation::spawn::write_executable(
            &harness,
            format!(
                "#!/bin/sh\nD='{d}'\necho \"$*\" >> \"$D/harness.log\"\ntouch \"$D/running/$2\"\n\
                 sleep \"$(cat \"$D/delays/$2\")\"\nls \"$D/running\" | wc -l | tr -d ' ' >> \"$D/peaks.log\"\n\
                 rm \"$D/running/$2\"\necho \"$2\" >> \"$D/done.log\"\necho \"stand-in harness: $2\" >&2\n\
                 if [ -e \"$D/fail/$2\" ]; then exit 1; fi\ncat \"$D/scenes/$2\"\n"
            ),
        )
        .unwrap();
        let cargo = dir.join("cargo");
        validation::spawn::write_executable(
            &cargo,
            format!(
                "#!/bin/sh\necho \"$*\" >> '{d}/cargo.log'\necho '{}'\n",
                artifact("golden_harness", Some(&harness.to_string_lossy()))
            ),
        )
        .unwrap();
        let output = std::process::Command::new("sh")
            .args([
                "-c",
                "exec \"$0\" golden \"$1\" 2>&1",
                env!("CARGO_BIN_EXE_xtask"),
                SUITE,
            ])
            .env("CARGO", &cargo)
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .timed_output()
            .expect("run xtask golden");
        StandIn {
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            ok: output.status.success(),
            dir,
            cases,
        }
    }

    /// The stand-in's log `file`.
    fn read(&self, file: &str) -> String {
        std::fs::read_to_string(self.dir.join(file)).unwrap_or_default()
    }

    /// The cases the run reported, in its order.
    fn reported(&self) -> Vec<String> {
        let prefix = format!("xtask golden: {SUITE}/");
        self.out
            .lines()
            .filter_map(|l| l.strip_prefix(&prefix))
            .filter_map(|l| l.split_once(':'))
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    /// The suite's harness cases' names, from the first to `end`.
    fn names(&self, end: usize) -> Vec<String> {
        self.cases[..end].iter().map(|c| c.name.clone()).collect()
    }

    /// The stand-in harness's stderr lines and the run's case reports, in the order the run wrote them, each as
    /// `stderr <scene>` or `report <case>`.
    fn stream(&self) -> Vec<String> {
        let prefix = format!("xtask golden: {SUITE}/");
        self.out
            .lines()
            .filter_map(|l| {
                if let Some(scene) = l.strip_prefix("stand-in harness: ") {
                    Some(format!("stderr {scene}"))
                } else {
                    let (name, _) = l.strip_prefix(&prefix)?.split_once(':')?;
                    Some(format!("report {name}"))
                }
            })
            .collect()
    }

    /// The stream a run reporting cases `..end` in turn writes: each case's harness stderr, then its report, and
    /// `last`'s stderr alone after them where given.
    fn in_turn(&self, end: usize, last: Option<usize>) -> Vec<String> {
        let mut want: Vec<String> = self.cases[..end]
            .iter()
            .flat_map(|c| [format!("stderr {}", c.scene), format!("report {}", c.name)])
            .collect();
        want.extend(last.map(|k| format!("stderr {}", self.cases[k].scene)));
        want
    }
}

/// Checks that the stand-in cargo was called once, to build the harness, and the harness once per case: `cargo` and
/// `harness` are their calls, one per line, and `cases` the suite's harness cases.
fn check_built_once(cargo: &str, harness: &str, cases: usize) {
    let builds: Vec<&str> = cargo
        .lines()
        .filter(|l| l.contains("golden_harness"))
        .collect();
    assert_eq!(
        builds.len(),
        1,
        "the harness was built {} times, not once: {builds:?}",
        builds.len()
    );
    assert!(
        builds[0].starts_with("build ") && builds[0].contains("--message-format=json"),
        "the harness was not built with `cargo build`'s JSON messages: {}",
        builds[0]
    );
    assert_eq!(
        harness.lines().count(),
        cases,
        "the harness ran {} times for {cases} cases:\n{harness}",
        harness.lines().count()
    );
}

#[test]
fn golden_harness_builds_once_per_run() {
    let run = StandIn::run("golden_harness_builds_once", |_| 0.0, |_| false);
    check_built_once(
        &run.read("cargo.log"),
        &run.read("harness.log"),
        run.cases.len(),
    );
    for c in &run.cases {
        assert!(
            run.read("harness.log")
                .contains(&format!("--scene {}\n", c.scene)),
            "the harness never rendered {}:\n{}",
            c.scene,
            run.out
        );
    }
}

negative_control!(
    golden_harness_builds_once_per_run,
    "a run that builds the harness per case, through `cargo run`, is not building it once",
    expected = "the harness was built 2 times",
    check_built_once(
        "run --quiet -p validation --bin golden_harness -- --scene a\nrun --quiet -p validation --bin golden_harness -- --scene b\n",
        "",
        2
    )
);

/// Checks that `results` are `0..n` doubled, in order, and that `peak`, the most jobs running at once, is more than
/// one and at most `width`.
fn check_in_parallel(results: &[usize], n: usize, peak: usize, width: usize) {
    assert_eq!(
        results,
        (0..n).map(|k| 2 * k).collect::<Vec<_>>(),
        "the results are not in index order"
    );
    assert!(
        peak > 1 && peak <= width,
        "{peak} jobs ran at once, on {width} threads"
    );
}

/// Runs `n` jobs on `width` threads, job `k` sleeping longer the smaller `k` is, so they finish in reverse: its
/// results, the most running at once, and the order they finished in.
fn run_jobs(n: usize, width: usize) -> (Vec<usize>, usize, Vec<usize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let running = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let done = std::sync::Mutex::new(Vec::new());
    let results = in_parallel(n, width, |k| {
        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(20 * (n - k) as u64));
        running.fetch_sub(1, Ordering::SeqCst);
        done.lock().unwrap().push(k);
        2 * k
    });
    (results, peak.into_inner(), done.into_inner().unwrap())
}

#[test]
fn golden_harness_in_parallel_keeps_index_order() {
    let (results, peak, done) = run_jobs(8, 4);
    check_in_parallel(&results, 8, peak, 4);
    assert_ne!(
        done,
        (0..8).collect::<Vec<_>>(),
        "the jobs finished in index order, so the order kept is not shown"
    );
    let (results, peak, done) = run_jobs(5, 1);
    assert_eq!(results, vec![0, 2, 4, 6, 8]);
    assert_eq!(
        (peak, done),
        (1, vec![0, 1, 2, 3, 4]),
        "one thread runs them in turn"
    );
    let (results, peak, _) = run_jobs(3, 16);
    check_in_parallel(&results, 3, peak, 3);
    let (results, peak, _) = run_jobs(1, 0);
    assert_eq!(
        (results, peak),
        (vec![0], 1),
        "a width of 0 still runs the job"
    );
    assert_eq!(in_parallel(0, 4, |k| k), Vec::<usize>::new());
    assert_eq!(
        harness_width(),
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        "the harness runs as wide as the machine offers"
    );
}

negative_control!(
    golden_harness_in_parallel_keeps_index_order,
    "results in the order the jobs finished, not index order, are refused",
    expected = "the results are not in index order",
    check_in_parallel(&[2, 0], 2, 2, 2)
);

/// Checks the stand-in run `run` rendered its cases at once, up to `width`, and reported them in case order: `peaks`
/// is the harness's count of running processes as each ended, `done` the scenes in the order they ended.
fn check_concurrent(run: &StandIn, peaks: &str, done: &str, width: usize) {
    let peak = peaks
        .lines()
        .map(|l| l.trim().parse::<usize>().unwrap())
        .max()
        .unwrap_or(0);
    let want = width.min(run.cases.len());
    assert!(
        peak > 1 || want == 1,
        "at most {peak} harness processes ran at once, with {want} allowed:\n{}",
        run.out
    );
    assert!(
        peak <= want,
        "{peak} harness processes ran at once, past {want}"
    );
    let scenes: Vec<&str> = run.cases.iter().map(|c| c.scene.as_str()).collect();
    if want > 1 {
        assert_ne!(
            done.lines().collect::<Vec<_>>(),
            scenes,
            "the cases finished in case order, so the order kept is not shown"
        );
    }
    assert_eq!(
        run.reported(),
        run.names(run.cases.len()),
        "the run did not report its cases in case order:\n{}",
        run.out
    );
    assert_eq!(
        run.stream(),
        run.in_turn(run.cases.len(), None),
        "the harness's stderr is not where running the cases in turn wrote it:\n{}",
        run.out
    );
}

#[test]
fn golden_harness_cases_render_concurrently() {
    // Each case sleeps longer than the next, so they end in reverse order where they run at once.
    let run = StandIn::run(
        "golden_harness_concurrently",
        |k| 0.1 * (10 - k.min(9)) as f64,
        |_| false,
    );
    check_concurrent(
        &run,
        &run.read("peaks.log"),
        &run.read("done.log"),
        harness_width(),
    );
}

negative_control!(
    golden_harness_cases_render_concurrently,
    "harness processes that ran one at a time are not concurrent",
    expected = "at most 1 harness processes ran at once",
    {
        let run = StandIn {
            dir: std::path::PathBuf::new(),
            cases: harness_cases(SUITE),
            out: String::new(),
            ok: true,
        };
        check_concurrent(&run, "1\n1\n1\n", "", 4)
    }
);

/// Checks that the stand-in run `run`, whose cases `first` and `later` failed, failed as running the cases in turn
/// would: the cases before `first` reported in order, then `first`'s error, and nothing of `later`.
fn check_first_failure(run: &StandIn, first: usize, later: usize) {
    assert!(!run.ok, "the run passed with failing cases:\n{}", run.out);
    assert_eq!(
        run.reported(),
        run.names(first),
        "the run did not report the cases before the first failing one, and only those:\n{}",
        run.out
    );
    let error = |k: usize| {
        format!(
            "golden_harness --scene {} failed (exit status: 1)",
            run.cases[k].scene
        )
    };
    assert!(
        run.out.contains(&error(first)),
        "the run does not report `{}`:\n{}",
        error(first),
        run.out
    );
    assert!(
        !run.out.contains(&error(later)),
        "the run reports the later failing case `{}`:\n{}",
        error(later),
        run.out
    );
    assert_eq!(
        run.stream(),
        run.in_turn(first, Some(first)),
        "the harness's stderr is not what running the cases in turn wrote:\n{}",
        run.out
    );
}

#[test]
fn golden_harness_failure_is_reported_in_case_order() {
    // Cases 3 and 6 fail; case 6 ends first, case 3 last of all.
    let (first, later) = (3, 6);
    let run = StandIn::run(
        "golden_harness_failure_order",
        |k| match k {
            3 => 1.0,
            6 => 0.0,
            _ => 0.2,
        },
        |k| k == first || k == later,
    );
    check_first_failure(&run, first, later);
}

negative_control!(
    golden_harness_failure_is_reported_in_case_order,
    "a run reporting the failing case that ended first, not the first in case order, is refused",
    expected = "the run does not report",
    {
        let cases = harness_cases(SUITE);
        let names: String = cases[..3]
            .iter()
            .map(|c| format!("xtask golden: {SUITE}/{}: FAIL\n", c.name))
            .collect();
        let run = StandIn {
            dir: std::path::PathBuf::new(),
            out: format!(
                "{names}xtask: golden_harness --scene {} failed (exit status: 1)",
                cases[6].scene
            ),
            ok: false,
            cases,
        };
        check_first_failure(&run, 3, 6)
    }
);
