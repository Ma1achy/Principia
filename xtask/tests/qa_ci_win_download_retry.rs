//! QA's tests for the Windows workflow's artifact downloads (PR #500, "win-gpu survives a download that ends short;
//! job-only re-runs find their artifacts"), written from what the change must guarantee, not from how it is built:
//!
//! - a download that hangs, fails or ends short never reaches registration unretried;
//! - the retried image is checked, its length and SHA-256 against those built, before registration;
//! - the job fails, naming the artifact, when both tries fail;
//! - a re-run of the failed jobs alone finds the artifacts by name, and a re-run of the uploading job replaces them;
//! - the per-try time limits fit inside the job's limit, and a try's limit doesn't cut off a whole download. Since
//!   R-988 the job's limit is 1.5 times its measured time, not a budget of its steps, so the job no longer has to hold
//!   the old budget's estimates for the image's SHA-256 checks and the registration; it still has to let each try, after
//!   the PC reaches the download and the cache lookup runs to its limit, run to its own limit.
//!
//! Off Windows there is no PowerShell, so the workflow is read and its steps' `if:` conditions are evaluated by a small
//! model of GitHub Actions' step semantics (`continue-on-error`, `timeout-minutes`, `outcome` against `conclusion`,
//! the implicit `success()`), driven through every pair of download results. Each test has its negative control
//! (R-176): an edited copy of the workflow or script that breaks the guarantee, which the same check must reject.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const PS: &str = ".github/scripts/wsl.ps1";
const BUILD: &str = "scripts/wsl/build-image.sh";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(
        text.contains(from),
        "qa control: the edit's target is gone: {from}"
    );
    text.replacen(from, to, 1)
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses")
        .remove(0)
}

fn job<'a>(doc: &'a Yaml, name: &str) -> &'a Yaml {
    let job = &doc["jobs"][name];
    assert!(!job.is_badvalue(), "the workflow has no job {name}");
    job
}

fn steps(job: &Yaml) -> &[Yaml] {
    job["steps"].as_vec().expect("the job has steps")
}

fn text<'a>(step: &'a Yaml, key: &str) -> Option<&'a str> {
    step[key].as_str()
}

fn is_download(step: &Yaml) -> bool {
    text(step, "uses").is_some_and(|u| u.starts_with("actions/download-artifact@"))
}

fn is_upload(step: &Yaml) -> bool {
    text(step, "uses").is_some_and(|u| u.starts_with("actions/upload-artifact@"))
}

/// The artifact a download or upload step names, without its `-${{ … }}` suffix: `wsl-image`, `wsl-manifest`, ….
fn artifact(step: &Yaml) -> String {
    let name = step["with"]["name"]
        .as_str()
        .expect("an artifact step names its artifact");
    name.split("-${{").next().unwrap().to_string()
}

/// One try of a download, as download-artifact v4.3.0 has been seen to end on the PC.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Try {
    /// Finished, the whole file written, `download-path` set.
    Whole,
    /// Stalled mid-stream and still exited 0: the step green, `download-path` set, part of the file written.
    Short,
    /// No output until its time limit ended it: outcome failure, no `download-path`, part of the file written.
    Hung,
    /// Failed outright: outcome failure, no `download-path`.
    Failed,
}

const TRIES: [Try; 4] = [Try::Whole, Try::Short, Try::Hung, Try::Failed];

#[derive(Debug, Default)]
struct Run {
    /// The target step ran, with each artifact's state then (`true` = the whole file is there).
    reached: Option<HashMap<String, bool>>,
    /// The job failed at a step: its `::error::` line, or its name.
    failed_with: Option<String>,
    /// A hang ran to the job's own limit: the job was cancelled.
    cancelled: bool,
    /// Downloads made, per artifact.
    downloads: HashMap<String, usize>,
    /// Minutes spent in hung downloads, each ended by its step's limit.
    hung_minutes: i64,
}

#[derive(Clone)]
struct StepState {
    outcome: &'static str,
    conclusion: &'static str,
    download_path: bool,
}

/// Evaluates an `if:` condition, as GitHub Actions does for the terms the workflow uses: `&&` of `steps.<id>.outcome`,
/// `.conclusion` or `.outputs.<name>` compared with `==` / `!=` to a quoted literal, and the status functions. A
/// condition with no status function has an implicit `success()`.
fn holds(cond: Option<&str>, states: &HashMap<String, StepState>, job_failed: bool) -> bool {
    let cond = cond.unwrap_or("success()").trim();
    let cond = cond
        .strip_prefix("${{")
        .and_then(|c| c.strip_suffix("}}"))
        .unwrap_or(cond)
        .trim();
    assert!(
        !cond.contains("||"),
        "the model reads only && conditions: {cond}"
    );
    let status = ["success()", "failure()", "always()", "cancelled()"];
    let mut value = if status.iter().any(|s| cond.contains(s)) {
        true
    } else {
        !job_failed
    };
    for term in cond.split("&&").map(str::trim) {
        let term_value = match term {
            "success()" => !job_failed,
            "failure()" => job_failed,
            "always()" => true,
            "cancelled()" => false,
            _ => {
                let (lhs, rhs, equal) = if let Some((l, r)) = term.split_once("!=") {
                    (l.trim(), r.trim(), false)
                } else if let Some((l, r)) = term.split_once("==") {
                    (l.trim(), r.trim(), true)
                } else {
                    panic!("the model can't read the term {term}")
                };
                let literal = rhs
                    .strip_prefix('\'')
                    .and_then(|r| r.strip_suffix('\''))
                    .unwrap_or_else(|| panic!("the model reads only quoted literals: {rhs}"));
                let path: Vec<&str> = lhs.split('.').collect();
                assert_eq!(path[0], "steps", "the model reads only steps.*: {lhs}");
                let state = states.get(path[1]).cloned().unwrap_or(StepState {
                    outcome: "",
                    conclusion: "",
                    download_path: false,
                });
                let actual: String = match &path[2..] {
                    ["outcome"] => state.outcome.into(),
                    ["conclusion"] => state.conclusion.into(),
                    ["outputs", "download-path"] => {
                        if state.download_path {
                            "C:\\runner\\_temp\\somewhere".into()
                        } else {
                            String::new()
                        }
                    }
                    other => panic!("the model can't read steps.{}.{other:?}", path[1]),
                };
                (actual == literal) == equal
            }
        };
        value &= term_value;
    }
    value
}

/// Runs `job`'s steps from its first download through the step whose script contains `target`, each artifact's
/// downloads ending as `tries` gives, in order.
fn simulate(doc: &Yaml, job_name: &str, target: &str, tries: &HashMap<&str, Vec<Try>>) -> Run {
    let job = job(doc, job_name);
    let job_limit = job["timeout-minutes"]
        .as_i64()
        .expect("the job has a time limit");
    let all = steps(job);
    let first = all
        .iter()
        .position(is_download)
        .expect("the job downloads an artifact");
    let mut run = Run::default();
    let mut states: HashMap<String, StepState> = HashMap::new();
    let mut whole: HashMap<String, bool> = HashMap::new();
    let mut job_failed = false;
    for step in &all[first..] {
        let id = text(step, "id").map(str::to_string);
        let script = text(step, "run").unwrap_or("");
        let continues = step["continue-on-error"].as_bool().unwrap_or(false);
        let record = |states: &mut HashMap<String, StepState>, s: StepState| {
            if let Some(id) = &id {
                states.insert(id.clone(), s);
            }
        };
        if script.contains(target) {
            if holds(text(step, "if"), &states, job_failed) {
                run.reached = Some(whole.clone());
            }
            return run;
        }
        if !holds(text(step, "if"), &states, job_failed) {
            record(
                &mut states,
                StepState {
                    outcome: "skipped",
                    conclusion: "skipped",
                    download_path: false,
                },
            );
            continue;
        }
        let (succeeded, download_path) = if is_download(step) {
            let name = artifact(step);
            let n = run.downloads.entry(name.clone()).or_default();
            let t = *tries
                .get(name.as_str())
                .and_then(|ts| ts.get(*n))
                .unwrap_or_else(|| panic!("the model has no try {} of {name}", *n + 1));
            *n += 1;
            match t {
                Try::Whole => {
                    whole.insert(name, true);
                    (true, true)
                }
                Try::Short => {
                    whole.insert(name, false);
                    (true, true)
                }
                Try::Hung => {
                    match step["timeout-minutes"].as_i64() {
                        Some(limit) if limit < job_limit => run.hung_minutes += limit,
                        _ => {
                            run.cancelled = true;
                            return run;
                        }
                    }
                    whole.insert(name, false);
                    (false, false)
                }
                Try::Failed => {
                    whole.entry(name).or_insert(false);
                    (false, false)
                }
            }
        } else if script.contains("check-image") {
            (whole.get("wsl-image").copied().unwrap_or(false), false)
        } else if script.contains("exit 1") && !script.contains("powershell") {
            (false, false)
        } else {
            (true, false)
        };
        let outcome = if succeeded { "success" } else { "failure" };
        let conclusion = if succeeded || continues {
            "success"
        } else {
            "failure"
        };
        record(
            &mut states,
            StepState {
                outcome,
                conclusion,
                download_path,
            },
        );
        if !succeeded && !continues && !job_failed {
            job_failed = true;
            let message = script
                .lines()
                .find(|l| l.contains("::error::"))
                .map(str::to_string)
                .unwrap_or_else(|| text(step, "name").unwrap_or("").to_string());
            run.failed_with = Some(message);
        }
    }
    panic!("{job_name} has no step running {target}")
}

/// Every pair of tries of the image's download, through `wsl-register`.
fn check_register(workflow: &str) {
    let doc = parse(workflow);
    for t1 in TRIES {
        for t2 in TRIES {
            let tries = HashMap::from([("wsl-image", vec![t1, t2])]);
            let run = simulate(&doc, "wsl-register", "wsl.ps1 register", &tries);
            let case = format!("tries {t1:?} then {t2:?}");
            assert!(
                !run.cancelled,
                "qa: a hung image download ran to the job's limit ({case})"
            );
            if let Some(state) = &run.reached {
                assert!(
                    state.get("wsl-image").copied().unwrap_or(false),
                    "qa: registration reached an image that is not whole ({case})"
                );
            }
            let any_whole = t1 == Try::Whole || t2 == Try::Whole;
            if any_whole {
                assert!(
                    run.reached.is_some(),
                    "qa: a whole try never reached registration ({case}): failed with {:?}",
                    run.failed_with
                );
            } else {
                assert!(
                    run.reached.is_none(),
                    "qa: registration reached with no whole try ({case})"
                );
                let message = run.failed_with.clone().unwrap_or_default();
                assert!(
                    message.contains("::error::") && message.contains("the image"),
                    "qa: both tries failed and the job didn't fail naming the image ({case}): {message:?}"
                );
            }
            if t1 == Try::Whole {
                assert_eq!(
                    run.downloads.get("wsl-image").copied(),
                    Some(1),
                    "qa: a whole first try was downloaded again ({case})"
                );
            }
        }
    }
}

#[test]
fn qa_image_download_retries_and_registers_only_a_whole_image() {
    check_register(&read(WORKFLOW));
}

// The retry runs only when the first download failed outright, not when it ended short with exit 0.
negative_control!(
    qa_image_download_retries_and_registers_only_a_whole_image,
    "a retry that skips a short first try must fail",
    expected = "qa: a whole try never reached registration (tries Short then Whole)",
    check_register(&edited(
        &read(WORKFLOW),
        "if: steps.image-check.outcome != 'success'\n",
        "if: steps.image.outcome != 'success'\n"
    ))
);

#[cfg(feature = "controls")]
mod qa_image_download_hang_control {
    use super::*;
    // The first try has no limit of its own: a hang runs to the job's.
    negative_control!(
        qa_image_download_retries_and_registers_only_a_whole_image,
        "a first try with no time limit of its own must fail",
        expected = "qa: a hung image download ran to the job's limit",
        check_register(&edited(
            &read(WORKFLOW),
            "        id: image\n        timeout-minutes: 25\n",
            "        id: image\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_image_retry_unchecked_control {
    use super::*;
    // The stop step looks at the second download's own outcome, so a short second try, exit 0, goes unchecked.
    negative_control!(
        qa_image_download_retries_and_registers_only_a_whole_image,
        "an unchecked retried image must fail",
        expected = "qa: registration reached an image that is not whole",
        check_register(&edited(
            &read(WORKFLOW),
            "&& steps.image-check-again.outcome != 'success'",
            "&& steps.image-again.outcome != 'success'"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_image_failure_unnamed_control {
    use super::*;
    // The stop step fails without naming the image.
    negative_control!(
        qa_image_download_retries_and_registers_only_a_whole_image,
        "a failure that doesn't name the image must fail",
        expected = "qa: both tries failed and the job didn't fail naming the image",
        check_register(&edited(&read(WORKFLOW), "twice (the image)", "twice"))
    );
}

#[cfg(feature = "controls")]
mod qa_image_failure_missing_control {
    use super::*;
    // No stop step: both tries bad and the job goes on to register.
    negative_control!(
        qa_image_download_retries_and_registers_only_a_whole_image,
        "a job that goes on after two bad tries must fail",
        expected = "qa: registration reached",
        check_register(&edited(
            &read(WORKFLOW),
            "the length and SHA-256 built'\n          exit 1\n",
            "the length and SHA-256 built'\n          exit 0\n"
        ))
    );
}

/// `wsl-proofs`' two small downloads: each retried after a hang or a failure, and the job failing naming it when
/// both tries fail.
fn check_proofs(workflow: &str) {
    let doc = parse(workflow);
    for (name, word) in [("wsl-manifest", "manifest"), ("wsl-parity", "parity")] {
        for t1 in [Try::Whole, Try::Hung, Try::Failed] {
            for t2 in [Try::Whole, Try::Hung, Try::Failed] {
                let mut tries = HashMap::from([
                    ("wsl-manifest", vec![Try::Whole, Try::Whole]),
                    ("wsl-parity", vec![Try::Whole, Try::Whole]),
                ]);
                tries.insert(name, vec![t1, t2]);
                let run = simulate(&doc, "wsl-proofs", "wsl.ps1 proofs", &tries);
                let case = format!("{name}: tries {t1:?} then {t2:?}");
                assert!(
                    !run.cancelled,
                    "qa: a hung download ran to the job's limit ({case})"
                );
                if t1 == Try::Whole || t2 == Try::Whole {
                    assert!(
                        run.reached.is_some(),
                        "qa: a whole try never reached the proofs ({case}): failed with {:?}",
                        run.failed_with
                    );
                } else {
                    assert!(
                        run.reached.is_none(),
                        "qa: the proofs ran with no download ({case})"
                    );
                    let message = run.failed_with.clone().unwrap_or_default();
                    assert!(
                        message.contains("::error::") && message.contains(word),
                        "qa: both tries failed and the job didn't fail naming the {word} ({case}): {message:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn qa_proofs_downloads_retry_and_fail_naming_the_artifact() {
    check_proofs(&read(WORKFLOW));
}

// The parity results' retry is gone: a failed first try and the job stops though a second would have been whole.
negative_control!(
    qa_proofs_downloads_retry_and_fail_naming_the_artifact,
    "a download with no retry must fail",
    expected = "qa: a whole try never reached the proofs (wsl-parity",
    check_proofs(&edited(
        &read(WORKFLOW),
        "if: steps.parity.outputs.download-path == ''\n",
        "if: steps.parity.outputs.download-path != ''\n"
    ))
);

#[cfg(feature = "controls")]
mod qa_proofs_hang_control {
    use super::*;
    // The manifest's first try has no limit of its own.
    negative_control!(
        qa_proofs_downloads_retry_and_fail_naming_the_artifact,
        "a manifest try with no time limit must fail",
        expected = "qa: a hung download ran to the job's limit (wsl-manifest",
        check_proofs(&edited(
            &read(WORKFLOW),
            "        id: manifest\n        timeout-minutes: 5\n",
            "        id: manifest\n"
        ))
    );
}

/// Every artifact a job downloads is uploaded under the same name, a name without the attempt, and the upload
/// overwrites: a re-run of the failed jobs alone (a new attempt) finds the first attempt's, and a re-run of the
/// uploading job replaces it rather than failing on a name already taken.
fn check_names(workflow: &str) {
    let doc = parse(workflow);
    let jobs = doc["jobs"].as_hash().expect("the workflow has jobs");
    let mut uploads: Vec<(&str, bool)> = Vec::new();
    let mut downloads: Vec<&str> = Vec::new();
    for (_, job) in jobs {
        let Some(steps) = job["steps"].as_vec() else {
            continue;
        };
        for step in steps {
            let name = step["with"]["name"].as_str();
            if is_upload(step) {
                let overwrite = step["with"]["overwrite"].as_bool().unwrap_or(false);
                uploads.push((name.expect("an upload names its artifact"), overwrite));
            } else if is_download(step) {
                downloads.push(name.expect("a download names its artifact"));
            }
        }
    }
    assert!(!downloads.is_empty(), "qa: the workflow downloads nothing");
    for name in downloads {
        assert!(
            !name.contains("run_attempt"),
            "qa: a download names the run's attempt, which a re-run of the failed jobs changes: {name}"
        );
        let matching: Vec<bool> = uploads
            .iter()
            .filter(|(u, _)| *u == name)
            .map(|(_, o)| *o)
            .collect();
        assert!(!matching.is_empty(), "qa: no upload is named {name}");
        assert!(
            matching.iter().all(|o| *o),
            "qa: the upload of {name} doesn't overwrite, so a re-run of its job can't replace it"
        );
    }
}

#[test]
fn qa_reruns_find_their_artifacts_by_name() {
    check_names(&read(WORKFLOW));
}

// The image's upload names the attempt again; its download doesn't.
negative_control!(
    qa_reruns_find_their_artifacts_by_name,
    "an upload that names the attempt must fail",
    expected = "qa: no upload is named wsl-image-",
    check_names(&edited(
        &read(WORKFLOW),
        "name: wsl-image-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-image\n          retention-days",
        "name: wsl-image-${{ github.run_id }}-${{ github.run_attempt }}\n          path: ${{ runner.temp }}/wsl-image\n          retention-days"
    ))
);

#[cfg(feature = "controls")]
mod qa_names_overwrite_control {
    use super::*;
    // The parity results' upload doesn't overwrite.
    negative_control!(
        qa_reruns_find_their_artifacts_by_name,
        "an upload that doesn't overwrite must fail",
        expected = "qa: the upload of wsl-parity-",
        check_names(&edited(
            &read(WORKFLOW),
            "path: ${{ runner.temp }}/wsl-parity\n          retention-days: 90\n          overwrite: true\n",
            "path: ${{ runner.temp }}/wsl-parity\n          retention-days: 90\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod qa_names_attempt_control {
    use super::*;
    // Both sides name the attempt again: they match, but a re-run of the failed jobs looks for an attempt never made.
    negative_control!(
        qa_reruns_find_their_artifacts_by_name,
        "a download that names the attempt must fail",
        expected = "qa: a download names the run's attempt",
        check_names(&read(WORKFLOW).replace(
            "wsl-manifest-${{ github.run_id }}\n",
            "wsl-manifest-${{ github.run_id }}-${{ github.run_attempt }}\n"
        ))
    );
}

/// The slower of the two whole downloads of the image on the PC, in minutes: 16 min 27.8 s (run 42954587933, the
/// evidence the workflow cites).
const SLOWER_WHOLE_IMAGE_MINUTES: f64 = 16.0 + 27.8 / 60.0;
/// The time the PC has taken to reach the download, in minutes: 3 min 59 s at most, as the workflow stated it before
/// R-988 (its budget's "4 minutes the PC has taken to reach the download").
const REACH_DOWNLOAD: i64 = 4;

/// A try's limit lets a whole download finish; reaching the download, the cache lookup to its limit and two tries that
/// each run to their limit fit in the job's limit (R-988 replaced the job's budget with its measured time, so the
/// budget's estimates for the checks and the registration leave the sum); and every download of `wsl-proofs` is limited
/// well inside its job's.
fn check_limits(workflow: &str) {
    let doc = parse(workflow);
    let register = job(&doc, "wsl-register");
    let job_limit = register["timeout-minutes"]
        .as_i64()
        .expect("wsl-register has a limit");
    let image: Vec<i64> = steps(register)
        .iter()
        .filter(|s| is_download(s) && artifact(s) == "wsl-image")
        .map(|s| {
            s["timeout-minutes"]
                .as_i64()
                .expect("qa: an image download has no time limit of its own")
        })
        .collect();
    assert_eq!(
        image.len(),
        2,
        "qa: the image is not downloaded at most twice"
    );
    for limit in &image {
        assert!(
            *limit as f64 > SLOWER_WHOLE_IMAGE_MINUTES,
            "qa: a try's limit of {limit} min cuts off a whole download ({SLOWER_WHOLE_IMAGE_MINUTES:.2} min)"
        );
    }
    let lookup = steps(register)
        .iter()
        .find(|s| text(s, "id") == Some("cache"))
        .map(|s| {
            s["timeout-minutes"]
                .as_i64()
                .expect("qa: the cache lookup has no time limit of its own")
        })
        .unwrap_or(0);
    let worst = REACH_DOWNLOAD + lookup + image.iter().sum::<i64>();
    assert!(
        worst <= job_limit,
        "qa: reaching the download, the cache lookup and two tries to their limits take {worst} min, over \
         wsl-register's {job_limit}"
    );
    let proofs = job(&doc, "wsl-proofs");
    let proofs_limit = proofs["timeout-minutes"]
        .as_i64()
        .expect("wsl-proofs has a limit");
    let downloads: i64 = steps(proofs)
        .iter()
        .filter(|s| is_download(s))
        .map(|s| {
            s["timeout-minutes"]
                .as_i64()
                .expect("qa: a proofs download has no time limit of its own")
        })
        .sum();
    assert!(
        downloads < proofs_limit,
        "qa: wsl-proofs' downloads can take {downloads} min, its whole limit {proofs_limit}"
    );
}

#[test]
fn qa_download_limits_fit_inside_the_job() {
    check_limits(&read(WORKFLOW));
}

// Each try gets 40 minutes: two of them no longer fit in the job's limit.
negative_control!(
    qa_download_limits_fit_inside_the_job,
    "tries too long for the job must fail",
    expected = "qa: reaching the download, the cache lookup and two tries to their limits take",
    check_limits(&read(WORKFLOW).replace(
        "        timeout-minutes: 25\n        continue-on-error: true\n        uses: actions/download-artifact",
        "        timeout-minutes: 40\n        continue-on-error: true\n        uses: actions/download-artifact"
    ))
);

#[cfg(feature = "controls")]
mod qa_limits_short_try_control {
    use super::*;
    // A try's limit below the slower whole download.
    negative_control!(
        qa_download_limits_fit_inside_the_job,
        "a try that cuts off a whole download must fail",
        expected = "qa: a try's limit of 15 min cuts off a whole download",
        check_limits(&edited(
            &read(WORKFLOW),
            "        id: image\n        timeout-minutes: 25\n",
            "        id: image\n        timeout-minutes: 15\n"
        ))
    );
}

/// The body of `name`'s definition in `script`, through its closing brace at the start of a line.
fn ps_function<'a>(script: &'a str, name: &str) -> Option<&'a str> {
    let start = script.find(&format!("function {name}("))?;
    let rest = &script[start..];
    let end = rest.find("\n}").map(|e| e + 2).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// The text of `verb`'s arm of wsl.ps1's switch, through to the next arm.
fn ps_arm_only<'a>(script: &'a str, verb: &str) -> &'a str {
    let open = format!("    '{verb}' {{");
    let start = script
        .find(&open)
        .unwrap_or_else(|| panic!("qa: wsl.ps1 has no {verb} verb"));
    let rest = &script[start + open.len()..];
    let end = rest.find("\n    '").unwrap_or(rest.len());
    &rest[..end]
}

/// The words of `arm` that name a function `script` defines, each with its byte offset in `arm` and its body.
fn ps_calls<'a>(script: &'a str, arm: &str) -> Vec<(usize, &'a str)> {
    let mut calls = Vec::new();
    let mut offset = 0;
    for word in arm.split(|c: char| !(c.is_alphanumeric() || c == '-')) {
        let at = offset;
        offset += word.len() + 1;
        if word.contains('-') {
            if let Some(body) = ps_function(script, word) {
                calls.push((at, body));
            }
        }
    }
    calls
}

/// `check-image`, which the workflow runs after each try, and `register`, before its import, each check the image's
/// length (from `rootfs.bytes`) and its SHA-256 (from `rootfs.sha256`), failing on a short copy by name.
fn check_ps(script: &str) {
    for verb in ["check-image", "register"] {
        let arm = ps_arm_only(script, verb);
        let mut seen = arm.to_string();
        for (_, body) in ps_calls(script, arm) {
            seen.push('\n');
            seen.push_str(body);
        }
        for needle in ["rootfs.bytes", "rootfs.sha256", ".Length", "Get-Sha256"] {
            assert!(
                seen.contains(needle),
                "qa: wsl.ps1 {verb} doesn't check the image by {needle}"
            );
        }
        assert!(
            seen.contains("short"),
            "qa: wsl.ps1 {verb} doesn't name a short copy"
        );
    }
    let register = ps_arm_only(script, "register");
    let import = register
        .find("--import")
        .expect("qa: register imports nothing");
    let check = register.find("rootfs.bytes").into_iter().chain(
        ps_calls(script, register)
            .into_iter()
            .filter(|(_, body)| body.contains("rootfs.bytes"))
            .map(|(at, _)| at),
    );
    assert!(
        check.min().is_some_and(|at| at < import),
        "qa: wsl.ps1 register imports before it checks the image's length"
    );
}

#[test]
fn qa_each_try_and_registration_check_the_image() {
    check_ps(&read(PS));
}

// check-image looks only at the hash: a short copy is not named as one.
negative_control!(
    qa_each_try_and_registration_check_the_image,
    "a check without the length must fail",
    expected = "qa: wsl.ps1 check-image doesn't check the image by rootfs.bytes",
    check_ps(&edited(
        &read(PS),
        "    'check-image' {\n        Assert-Image $env:WSL_IMAGE\n",
        "    'check-image' {\n        $null = Get-Sha256 (Join-Path $env:WSL_IMAGE 'rootfs.tar.gz')\n"
    ))
);

/// The line of build-image.sh that writes `rootfs.bytes`, run in bash on a file of known length, writes that length
/// as bare digits, which is what the check reads (`^[0-9]+$`), and only that.
#[cfg(unix)]
fn check_length_file(build: &str) {
    let line = build
        .lines()
        .find(|l| l.contains("rootfs.bytes") && !l.trim_start().starts_with('#'))
        .expect("qa: build-image.sh writes no rootfs.bytes");
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "qa-win-download-retry-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let length = 98_765usize;
    std::fs::write(dir.join("rootfs.tar.gz"), vec![7u8; length]).unwrap();
    let status = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("set -euo pipefail; out=\"$1\"; {line}"))
        .arg("bash")
        .arg(&dir)
        .status()
        .expect("bash runs");
    let written = std::fs::read_to_string(dir.join("rootfs.bytes")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        status.success(),
        "qa: build-image.sh's rootfs.bytes line failed: {line}"
    );
    assert_eq!(
        written,
        length.to_string(),
        "qa: rootfs.bytes is not the image's length as bare digits"
    );
}

#[cfg(unix)]
#[test]
fn qa_build_writes_the_images_length() {
    check_length_file(&read(BUILD));
}

// `wc -c` given the file by name writes its name after the length.
#[cfg(unix)]
negative_control!(
    qa_build_writes_the_images_length,
    "a length file holding more than the length must fail",
    expected = "qa: rootfs.bytes is not the image's length as bare digits",
    check_length_file(&edited(
        &read(BUILD),
        "wc -c < \"$out/rootfs.tar.gz\"",
        "wc -c \"$out/rootfs.tar.gz\""
    ))
);
