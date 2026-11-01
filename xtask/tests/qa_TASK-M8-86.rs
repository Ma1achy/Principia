//! QA tests for TASK-M8-86, written from REQ-TOOL-220 (R-627: the Pages deploy runs only by hand, only from `main`, and
//! deploys only through the `github-pages` environment, with no other trigger) and from the task's deliverables (the
//! skeleton's site and CI build, the wasm-threads probe, the URL flags and what the runner scripts write), not from the
//! implementation. `qa_TASK-M8-86.mjs` runs the real scripts on Node.
// The file name `qa_TASK-M8-86` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use validation::negative_control;
use validation::spawn::Spawn;
use xtask::pages_workflow::check;
use xtask::web_skeleton::SITE_FILES;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is in the workspace")
        .to_owned()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn workflow() -> String {
    read(".github/workflows/pages.yml")
}

fn edited(from: &str, to: &str) -> String {
    let text = workflow();
    assert!(
        text.contains(from),
        "the control's `{from}` is not in pages.yml"
    );
    text.replace(from, to)
}

/// The workflow must be refused.
fn refused(text: &str, what: &str) {
    assert!(check(text).is_err(), "pages.yml passed with {what}");
}

const DEPLOY_STEP: &str = "actions/deploy-pages@d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e";

/// Every other way the file could start the workflow, or leave `main`, or leave the environment.
fn check_trigger_faults(check_one: impl Fn(&str, &str)) {
    for trigger in [
        "push:",
        "pull_request:",
        "pull_request_target:",
        "schedule:",
        "repository_dispatch:",
        "workflow_call:",
        "workflow_run:",
        "release:",
    ] {
        check_one(
            &edited(
                "  workflow_dispatch:\n",
                &format!("  workflow_dispatch:\n  {trigger}\n"),
            ),
            trigger,
        );
    }
    check_one(
        &edited(
            "on:\n  workflow_dispatch:\n",
            "on: [workflow_dispatch, push]\n",
        ),
        "the flow form",
    );
    check_one(
        &edited("on:\n  workflow_dispatch:\n", "on: push\n"),
        "on: push",
    );
    check_one(
        &edited(
            "if: github.ref == 'refs/heads/main'",
            "if: github.ref != 'refs/heads/main'",
        ),
        "the condition negated",
    );
    check_one(
        &edited(
            "if: github.ref == 'refs/heads/main'",
            "if: github.ref == 'refs/heads/main' || always()",
        ),
        "the condition widened",
    );
    check_one(
        &edited(
            "    if: github.ref == 'refs/heads/main'",
            "    # if: github.ref == 'refs/heads/main'",
        ),
        "the condition commented out",
    );
    check_one(
        &edited(
            "      name: github-pages",
            "      # name: github-pages\n      name: other",
        ),
        "the environment commented out",
    );
}

/// The workflow must be accepted.
fn accepted(text: &str) {
    assert_eq!(check(text), Ok(()));
}

#[test]
fn qa_m8_86_the_real_workflow_passes() {
    accepted(&workflow());
}

negative_control!(
    qa_m8_86_the_real_workflow_passes,
    "the real file with a push trigger added",
    expected = "not workflow_dispatch alone",
    accepted(&edited(
        "  workflow_dispatch:\n",
        "  workflow_dispatch:\n  push:\n"
    ))
);

#[test]
fn qa_m8_86_each_trigger_ref_and_environment_fault_is_refused() {
    check_trigger_faults(refused);
}

negative_control!(
    qa_m8_86_each_trigger_ref_and_environment_fault_is_refused,
    "the real file offered as each fault",
    expected = "pages.yml passed with",
    refused(&workflow(), "the real file")
);

/// REQ-TOOL-220: "deploy only through the `github-pages` environment", "only from `main`". A deploy step in the build job,
/// given what it needs, runs on any ref and through no environment, whatever the deploy job says.
fn deploy_in_the_build_job() -> String {
    edited(
        "    runs-on: ubuntu-24.04\n    steps:\n      - uses: actions/checkout@v4\n",
        &format!(
            "    runs-on: ubuntu-24.04\n    permissions:\n      pages: write\n      id-token: write\n    steps:\n      - uses: {DEPLOY_STEP}\n      - uses: actions/checkout@v4\n"
        ),
    )
}

#[test]
fn qa_m8_86_no_deploy_outside_the_deploy_job() {
    refused(
        &deploy_in_the_build_job(),
        "a deploy step in the build job, outside the ref condition and the environment",
    );
}

negative_control!(
    qa_m8_86_no_deploy_outside_the_deploy_job,
    "the real file offered as the fault",
    expected = "pages.yml passed with",
    refused(&workflow(), "the real file")
);

/// The workflows of `dir` that may deploy to Pages, by the actions and permission that do it.
fn deploying(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .expect("the workflows")
        .map(|e| e.expect("a workflow").path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let text = std::fs::read_to_string(path).expect("a workflow");
            [
                "actions/deploy-pages",
                "actions/upload-pages-artifact",
                "pages: write",
            ]
            .iter()
            .any(|mark| {
                text.lines()
                    .any(|l| !l.trim_start().starts_with('#') && l.contains(mark))
            })
        })
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

fn check_only_pages(dir: &Path) {
    assert_eq!(
        deploying(dir),
        ["pages.yml"],
        "the workflows that deploy to Pages"
    );
}

/// R-627's "No other trigger": a second workflow deploying would be one, and the check of pages.yml would not see it.
#[test]
fn qa_m8_86_pages_yml_is_the_only_deploy() {
    check_only_pages(&root().join(".github/workflows"));
}

negative_control!(
    qa_m8_86_pages_yml_is_the_only_deploy,
    "a directory holding no pages.yml",
    expected = "the workflows that deploy to Pages",
    check_only_pages(&root().join("xtask/src"))
);

/// The job named `name` in a workflow's text, its lines to the next job.
fn job(text: &str, name: &str) -> String {
    let header = format!("  {name}:\n");
    let (_, after) = text
        .split_once(&header)
        .unwrap_or_else(|| panic!("no job {name}"));
    after
        .lines()
        .take_while(|l| l.is_empty() || l.starts_with("   ") || l.starts_with("  #"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn check_ci_builds_wasm(ci: &str) {
    let build = job(ci, "web-skeleton");
    for needed in ["wasm32-unknown-unknown", "--features web", "-p gui"] {
        assert!(build.contains(needed), "the CI job lacks {needed}");
    }
    assert!(
        !build.contains("deploy-pages") && !build.contains("upload-pages"),
        "the CI job is build-only"
    );
}

#[test]
fn qa_m8_86_ci_builds_the_skeleton_and_only_builds() {
    check_ci_builds_wasm(&read(".github/workflows/ci.yml"));
}

negative_control!(
    qa_m8_86_ci_builds_the_skeleton_and_only_builds,
    "a job that also deploys",
    expected = "the CI job is build-only",
    check_ci_builds_wasm(
        "  web-skeleton:\n    steps:\n      - run: cargo rustc -p gui --target wasm32-unknown-unknown --features web\n      - uses: actions/deploy-pages@v4\n"
    )
);

/// The site's page: its scripts are its own (caching_contract Part 6a: self-hosted, same origin, no CDN), the
/// coi-serviceworker before the loader, and the listed files are what the page and the loader name.
fn check_page(html: &str, loader: &str) {
    for attr in html.split("src=\"").skip(1) {
        let src = attr.split('"').next().unwrap_or_default();
        assert!(
            !src.contains("//") && SITE_FILES.contains(&src),
            "the page's script {src} is not one of the site's own"
        );
    }
    let coi = html
        .find("coi-serviceworker.js")
        .expect("the page loads coi-serviceworker");
    let start = html.find("loader.js").expect("the page loads the loader");
    assert!(coi < start, "coi-serviceworker comes first");
    for name in ["worker.js", "probe-worker.js"] {
        assert!(loader.contains(name), "the loader does not start {name}");
        assert!(SITE_FILES.contains(&name), "the site lacks {name}");
    }
    assert!(
        !loader.contains("https://"),
        "the loader reaches off the site"
    );
}

#[test]
fn qa_m8_86_the_site_is_self_hosted() {
    check_page(
        &read("web/skeleton/index.html"),
        &read("web/skeleton/loader.js"),
    );
}

negative_control!(
    qa_m8_86_the_site_is_self_hosted,
    "coi-serviceworker from a CDN",
    expected = "is not one of the site's own",
    check_page(
        &read("web/skeleton/index.html").replace(
            "src=\"coi-serviceworker.js\"",
            "src=\"https://cdn.example/coi-serviceworker.js\""
        ),
        &read("web/skeleton/loader.js")
    )
);

/// The `web` feature is opt-in, and its crates are out of the native build (the task: "the native build unchanged").
fn check_native_unchanged(manifest: &str) {
    let default = manifest
        .lines()
        .any(|l| l.starts_with("default") && l.contains("web"));
    assert!(!default, "the web feature is on by default");
    let (native, _) = manifest
        .split_once("[target.")
        .expect("the wasm dependencies' table");
    for wasm_only in ["wasm-bindgen", "web-sys", "js-sys", "web-time"] {
        let in_deps = native
            .lines()
            .any(|l| l.starts_with(&format!("{wasm_only} ")));
        assert!(!in_deps, "{wasm_only} is in the native build");
    }
    assert!(
        manifest.contains("[target.'cfg(target_arch = \"wasm32\")'.dependencies]"),
        "no wasm-only table"
    );
}

#[test]
fn qa_m8_86_the_native_build_takes_no_web_crate() {
    check_native_unchanged(&read("crates/gui/Cargo.toml"));
}

negative_control!(
    qa_m8_86_the_native_build_takes_no_web_crate,
    "the web feature on by default",
    expected = "the web feature is on by default",
    check_native_unchanged(&format!(
        "default = [\"web\"]\n{}",
        read("crates/gui/Cargo.toml")
    ))
);

/// The pins the task names: exact versions of `@playwright/test` and a WebDriver client, the lock agreeing; the Chrome
/// script on the installed Chrome stable and the Safari script through WebDriver, both writing the fields.
fn check_runner(package: &str, lock: &str, chrome: &str, safari: &str) {
    for name in ["@playwright/test", "selenium-webdriver"] {
        let needle = format!("\"{name}\": \"");
        let (_, after) = package
            .split_once(&needle)
            .unwrap_or_else(|| panic!("no pin of {name}"));
        let version = after.split('"').next().expect("a version");
        assert!(
            version.split('.').count() == 3
                && version.bytes().all(|b| b.is_ascii_digit() || b == b'.'),
            "{name} is at {version}, not pinned"
        );
        let locked = format!("\"node_modules/{name}\": {{\n      \"version\": \"{version}\"");
        assert!(lock.contains(&locked), "the lock has another {name}");
    }
    assert!(
        chrome.contains("channel: \"chrome\""),
        "not the installed Chrome"
    );
    assert!(
        safari.contains("forBrowser(\"safari\")"),
        "not safaridriver"
    );
    for script in [chrome, safari] {
        assert!(
            script.contains("writeFields("),
            "a script that writes nothing"
        );
        assert!(
            script.contains("runUrl("),
            "a script that ignores the flags"
        );
    }
}

fn runner(package: String) {
    check_runner(
        &package,
        &read("web/skeleton/runner/package-lock.json"),
        &read("web/skeleton/runner/chrome.mjs"),
        &read("web/skeleton/runner/safari.mjs"),
    );
}

#[test]
fn qa_m8_86_the_runner_is_pinned() {
    runner(read("web/skeleton/runner/package.json"));
}

negative_control!(
    qa_m8_86_the_runner_is_pinned,
    "a caret range",
    expected = "not pinned",
    runner(read("web/skeleton/runner/package.json").replace("\"1.64.1\"", "\"^1.64.1\""))
);

static RUNS: AtomicU32 = AtomicU32::new(0);

/// Node's flag for a capped address space. By default V8 reserves about 10 GiB of guard regions for any wasm memory, so
/// `new WebAssembly.Memory` throws a RangeError under `.cargo/mutants.toml`'s 8 GiB `RLIMIT_AS` (REQ-VAL-181), however
/// small the memory. With the flag, V8 compiles inline bounds checks and reserves only the memory itself. The probe still
/// makes a real shared memory and runs real atomics across two threads, and the flag is on everywhere, so these tests run
/// one way with or without the cap, with no skip.
const WASM_UNDER_A_CAP: &str = "--disable-wasm-trap-handler";

/// Runs `scenario` of the harness on the skeleton's scripts, each `(file, from, to)` edit applied.
fn node(scenario: &str, edits: &[(&str, &str, &str)]) {
    harness(&["node", WASM_UNDER_A_CAP], scenario, edits);
}

/// [`node`], with `command` as the command line up to the harness's path.
fn harness(command: &[&str], scenario: &str, edits: &[(&str, &str, &str)]) {
    let skeleton = root().join("web/skeleton");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa-m8-86-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    for (from_dir, file) in [
        ("", "loader.js"),
        ("", "probe-worker.js"),
        ("runner/", "common.mjs"),
    ] {
        let mut text =
            std::fs::read_to_string(skeleton.join(from_dir).join(file)).expect("a script");
        for (_, from, to) in edits.iter().filter(|(name, ..)| *name == file) {
            assert!(
                text.contains(from),
                "the control's `{from}` is not in {file}"
            );
            text = text.replace(from, to);
        }
        std::fs::write(dir.join(file), text).expect("a scratch script");
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/qa_TASK-M8-86.mjs");
    let output = std::process::Command::new(command[0])
        .args(&command[1..])
        .arg(script)
        .arg(&dir)
        .arg(scenario)
        .timed_output()
        .expect("node, which the skeleton's runner needs too");
    std::fs::remove_dir_all(&dir).expect("the scratch directory");
    assert!(
        output.status.success(),
        "{scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The probe's scenario under the cap the mutants job sets (the value read from its configuration), through `flags`.
#[cfg(target_os = "linux")]
fn probe_under_the_mutants_cap(flags: &[&str]) {
    let config = read(".cargo/mutants.toml");
    let cap = config
        .split("'--as=")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .expect("the runner's cap in .cargo/mutants.toml");
    let cap = format!("--as={cap}");
    let mut command = vec!["prlimit", &cap, "--core=1", "node"];
    command.extend_from_slice(flags);
    harness(&command, "probe_shared_memory_says_ok", &[]);
}

// Linux alone: macOS has no `prlimit`, and its `setrlimit` refuses this limit.
#[cfg(target_os = "linux")]
#[test]
fn qa_m8_86_the_probe_runs_under_the_mutants_cap() {
    probe_under_the_mutants_cap(&[WASM_UNDER_A_CAP]);
}

#[cfg(target_os = "linux")]
negative_control!(
    qa_m8_86_the_probe_runs_under_the_mutants_cap,
    "node without the flag, which the cap must stop",
    expected = "could not allocate memory",
    probe_under_the_mutants_cap(&[])
);

#[test]
fn qa_m8_86_probe_says_ok_on_shared_memory() {
    node("probe_shared_memory_says_ok", &[]);
}

negative_control!(
    qa_m8_86_probe_says_ok_on_shared_memory,
    "a second worker writing another value",
    expected = "the probe on shared memory said: read",
    node(
        "probe_shared_memory_says_ok",
        &[(
            "probe-worker.js",
            "instance.exports.write(VALUE)",
            "instance.exports.write(VALUE + 1)"
        )]
    )
);

#[test]
fn qa_m8_86_probe_never_says_ok_without_shared_memory() {
    node("probe_unshared_memory_says_why_not", &[]);
}

negative_control!(
    qa_m8_86_probe_never_says_ok_without_shared_memory,
    "a probe that always says ok",
    expected = "the probe said ok with no shared memory",
    node(
        "probe_unshared_memory_says_why_not",
        &[(
            "probe-worker.js",
            "self.onmessage = async ({ data: { role, memory } }) => {",
            "self.onmessage = async ({ data: { role, memory } }) => { if (role === \"first\") self.postMessage(\"ok\"); return;"
        )]
    )
);

#[test]
fn qa_m8_86_layout_main_is_the_main_thread_with_the_panels_fields() {
    node("loader_layout_main", &[]);
}

negative_control!(
    qa_m8_86_layout_main_is_the_main_thread_with_the_panels_fields,
    "a layout flag that is ignored",
    expected = "started a worker",
    node(
        "loader_layout_main",
        &[(
            "loader.js",
            "params.get(\"layout\") ?? \"worker\"",
            "\"worker\""
        )]
    )
);

#[test]
fn qa_m8_86_layout_worker_is_a_worker_on_the_transferred_canvas() {
    node("loader_layout_worker", &[]);
}

negative_control!(
    qa_m8_86_layout_worker_is_a_worker_on_the_transferred_canvas,
    "a layout flag that is ignored",
    expected = "started the main app",
    node(
        "loader_layout_worker",
        &[(
            "loader.js",
            "params.get(\"layout\") ?? \"worker\"",
            "\"main\""
        )]
    )
);

#[test]
fn qa_m8_86_a_bad_layout_flag_is_refused() {
    node("loader_layout_bad_is_refused", &[]);
}

negative_control!(
    qa_m8_86_a_bad_layout_flag_is_refused,
    "a bad flag taken as the main thread",
    expected = "a bad layout flag was not refused",
    node(
        "loader_layout_bad_is_refused",
        &[(
            "loader.js",
            "asked !== \"worker\" && asked !== \"main\"",
            "false"
        )]
    )
);

#[test]
fn qa_m8_86_no_measure_flag_no_panel() {
    node("loader_no_measure_no_environment", &[]);
}

negative_control!(
    qa_m8_86_no_measure_flag_no_panel,
    "a panel always open",
    expected = "no ?measure, and the environment is",
    node(
        "loader_no_measure_no_environment",
        &[("loader.js", "params.has(\"measure\")", "true")]
    )
);

#[test]
fn qa_m8_86_a_missing_api_records_its_reason_and_the_fallback() {
    node("loader_missing_api_is_recorded", &[]);
}

negative_control!(
    qa_m8_86_a_missing_api_records_its_reason_and_the_fallback,
    "the reason dropped",
    expected = "the reason: null",
    node(
        "loader_missing_api_is_recorded",
        &[(
            "loader.js",
            "await startMain(env, String(reason));",
            "await startMain(env, null);"
        )]
    )
);

#[test]
fn qa_m8_86_no_webgpu_fails_with_the_fields() {
    node("loader_no_webgpu_is_failed", &[]);
}

negative_control!(
    qa_m8_86_no_webgpu_fails_with_the_fields,
    "the failure without its fields",
    expected = "the failure lost the panel's fields",
    node(
        "loader_no_webgpu_is_failed",
        &[("loader.js", "{ ...env, layout: asked }", "{}")]
    )
);

#[test]
fn qa_m8_86_the_runner_asks_for_each_flag() {
    node("runner_args_and_url", &[]);
}

negative_control!(
    qa_m8_86_the_runner_asks_for_each_flag,
    "a runner that ignores the layout",
    expected = "the layout flag",
    node(
        "runner_args_and_url",
        &[(
            "common.mjs",
            "url.searchParams.set(\"layout\", options.layout);",
            ""
        )]
    )
);

#[test]
fn qa_m8_86_the_runner_writes_the_panels_fields() {
    node("runner_writes_the_fields", &[]);
}

negative_control!(
    qa_m8_86_the_runner_writes_the_panels_fields,
    "a runner that writes no results",
    expected = "the fields",
    node(
        "runner_writes_the_fields",
        &[(
            "common.mjs",
            "layout: options.layout, results }",
            "layout: options.layout, results: { pan: {}, idle: {} } }"
        )]
    )
);
