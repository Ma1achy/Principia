//! The WSL2 distro `principia-wsl` (RQ-751 option (c), built in parallel to replace (b) and never blocking; the owner's
//! words of 9 Nov 2026, with the ruling given the same day on `scripts/contain.sh`'s unprivileged mode): the image
//! script `scripts/wsl/build-image.sh`, the Windows side `.github/scripts/wsl.ps1`, the inside `scripts/wsl/inside.sh`,
//! and `win-gpu.yml`'s jobs that run them. Off Windows there is no PowerShell and no WSL, so, as `win_contain.rs` does,
//! these read the scripts and the workflow; the proofs themselves are `wsl-proofs`, run on the PC. `parity.py`, which
//! compares proof 6's two sides, runs here. Each test has its negative controls (R-176).
//!
//! What they hold: every job of the distro runs only when dispatched naming it, never on a push, the schedule or `all`;
//! no job or script of it names a secret or a token, and every checkout leaves no credential; `wsl-cpu` refuses while
//! `.github/wsl-cpu-enabled` is absent, and runs one named mutation workload and nothing else (the owner's answer to
//! RQ-825 (closed by R-1029)), its five inputs reaching the run only as its step's environment and `inside.sh mutants` refusing each
//! malformed one before cargo runs, then running the criterion's `cargo mutants` through `contain.sh`;
//! `/etc/wsl.conf` is exactly the owner's with R-1010's two additions, WSL's GPU share off and a `[boot]` holding the
//! one command that removes the `WSLInterop` registration, and so no systemd, `systemd=` refused anywhere in it and in
//! the build; ci's cargo config is root's and read-only to ci; and the image has no sudo and no sudoers
//! entry, the purge forced past sudo's own guard for that command alone and the build failing unless sudo is then gone;
//! the base and rustup are pinned by hash and checked before use, the packages come from one dated snapshot,
//! the toolchain is `rust-toolchain.toml`'s; the image installs the Vulkan packages `ci.yml`'s lavapipe steps install,
//! and `inside.sh` selects the backend those jobs select; registration refuses without a pinned manifest hash, and
//! unregisters an import it could not verify; proof 3 tries the host over WSL's virtual switch, at the distro's gateway
//! and on SMB's 445, as well as over the LAN; and proof 4 runs the unprivileged check and fails unless the processes
//! outside survive it, inside and on Windows; and `inside.sh parity` fails, printing the end of cargo's log, when no
//! test ran.

use std::path::{Path, PathBuf};

use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

#[cfg(unix)]
#[path = "../../crates/validation/tests/support/scratch.rs"]
mod scratch;

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const PS: &str = ".github/scripts/wsl.ps1";
const BUILD: &str = "scripts/wsl/build-image.sh";
const INSIDE: &str = "scripts/wsl/inside.sh";
const CONF: &str = "scripts/wsl/wsl.conf";
const FLAG: &str = ".github/wsl-cpu-enabled";
const SIGNERS: &str = ".github/allowed_signers";
const GUARD: &str = "github.repository == 'Ma1achy/Principia' && github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch'";
/// Each job of the distro and the `only` values it runs for.
const JOBS: [(&str, &str); 5] = [
    (
        "wsl-image",
        "(inputs.only == 'wsl-register' || inputs.only == 'wsl-proofs')",
    ),
    ("wsl-parity", "inputs.only == 'wsl-proofs'"),
    ("wsl-register", "inputs.only == 'wsl-register'"),
    ("wsl-proofs", "inputs.only == 'wsl-proofs'"),
    ("wsl-cpu", "inputs.only == 'wsl-cpu'"),
];
/// The owner's wsl.conf, `[automount] enabled=false`, `[interop] enabled=false, appendWindowsPath=false`, and the
/// default user the Windows session asked for; then R-1010's two additions and nothing else: `[gpu] enabled=false`,
/// which closes WSL's GPU share, the driver store on C: 9p-mounted at `/usr/lib/wsl/drivers`; and a `[boot]` holding
/// one `command=`, which WSL runs as root at the distro's start, without systemd, removing the `WSLInterop`
/// registration WSL2 makes at every start whatever `[interop] enabled` says (run 43418528499's proof 2 found both).
const OWNERS_CONF: &str = "[automount]\nenabled=false\n\n[interop]\nenabled=false\nappendWindowsPath=false\n\n[user]\ndefault=ci\n\n[gpu]\nenabled=false\n\n[boot]\ncommand=echo -1 > /proc/sys/fs/binfmt_misc/WSLInterop\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` with `from` replaced by `to`, once; the edit's target must be there.
#[cfg(feature = "controls")]
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the edit's target is gone: {from}");
    text.replacen(from, to, 1)
}

fn parse(text: &str) -> Yaml {
    YamlLoader::load_from_str(text)
        .expect("the workflow parses")
        .remove(0)
}

/// The text of `verb`'s arm of wsl.ps1's switch, through to the next arm.
fn arm<'a>(script: &'a str, verb: &str) -> &'a str {
    let start = script
        .find(&format!("\n    '{verb}' {{\n"))
        .unwrap_or_else(|| panic!("{PS} has no `{verb}` arm"));
    let rest = &script[start + 1..];
    let end = rest[1..]
        .find("\n    '")
        .or_else(|| rest.find("\n    default {"))
        .map_or(rest.len(), |e| e + 1);
    &rest[..end]
}

// ---- Dispatch only ----------------------------------------------------------------------------------------------

fn check_dispatch_only(text: &str) {
    let doc = parse(text);
    let only = doc["on"]["workflow_dispatch"]["inputs"]["only"]["options"]
        .as_vec()
        .expect("the `only` choices");
    for (job, condition) in JOBS {
        let found = doc["jobs"][job]["if"].as_str().unwrap_or("");
        assert_eq!(
            found,
            format!("{GUARD} && {condition}"),
            "{job} runs other than when dispatched naming it"
        );
    }
    for job in ["wsl-register", "wsl-proofs", "wsl-cpu"] {
        assert!(
            only.iter().any(|o| o.as_str() == Some(job)),
            "{job} can't be dispatched: no `only` choice for it"
        );
    }
    assert_eq!(
        doc["on"]["workflow_dispatch"]["inputs"]["replace"]["default"].as_bool(),
        Some(false),
        "registration replaces a distro unless asked not to"
    );
}

#[test]
fn wsl_jobs_run_only_when_dispatched_naming_them() {
    check_dispatch_only(&read(WORKFLOW));
}

negative_control!(
    wsl_jobs_run_only_when_dispatched_naming_them,
    "wsl-cpu also on `all`",
    expected = "wsl-cpu runs other than when dispatched naming it",
    check_dispatch_only(&edited(
        &read(WORKFLOW),
        "&& inputs.only == 'wsl-cpu'",
        "&& (inputs.only == 'wsl-cpu' || inputs.only == 'all')"
    ))
);

mod on_push {
    use super::*;
    negative_control!(
        wsl_jobs_run_only_when_dispatched_naming_them,
        "the image built on any event",
        expected = "wsl-image runs other than when dispatched naming it",
        check_dispatch_only(&edited(
            &read(WORKFLOW),
            "github.actor == 'Ma1achy' && github.event_name == 'workflow_dispatch' && (inputs.only == 'wsl-register'",
            "github.actor == 'Ma1achy' && (inputs.only == 'wsl-register'"
        ))
    );
}

mod replace_default {
    use super::*;
    negative_control!(
        wsl_jobs_run_only_when_dispatched_naming_them,
        "replace on by default",
        expected = "registration replaces a distro unless asked not to",
        check_dispatch_only(&edited(
            &read(WORKFLOW),
            "        type: boolean\n        default: false\n",
            "        type: boolean\n        default: true\n"
        ))
    );
}

// ---- No credential ------------------------------------------------------------------------------------------------

fn check_no_credential(workflow: &str, scripts: &[(&str, String)]) {
    let doc = parse(workflow);
    for (job, _) in JOBS {
        let steps = doc["jobs"][job]["steps"].as_vec().expect("steps");
        for step in steps {
            if step["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout@"))
            {
                assert_eq!(
                    step["with"]["persist-credentials"].as_bool(),
                    Some(false),
                    "{job} leaves the token in its checkout"
                );
            }
            assert!(
                step["with"]["github-token"].is_badvalue() && step["with"]["run-id"].is_badvalue(),
                "{job} downloads with a token or from another run"
            );
        }
    }
    for (name, text) in scripts {
        let lower = text.to_lowercase();
        for word in [
            "secrets.",
            "github_token",
            "github.token",
            "gh_token",
            "authorization:",
        ] {
            assert!(!lower.contains(word), "{name} names {word}");
        }
    }
}

fn scripts() -> Vec<(&'static str, String)> {
    [
        PS,
        BUILD,
        INSIDE,
        "scripts/wsl/manifest.sh",
        "scripts/wsl/parity.py",
    ]
    .into_iter()
    .map(|f| (f, read(f)))
    .collect()
}

#[test]
fn wsl_jobs_and_scripts_hold_no_credential() {
    check_no_credential(&read(WORKFLOW), &scripts());
}

negative_control!(
    wsl_jobs_and_scripts_hold_no_credential,
    "a script that sends a token",
    expected = "scripts/wsl/inside.sh names github_token",
    {
        let mut s = scripts();
        s[2].1
            .push_str("\ncurl -H \"Authorization: Bearer $GITHUB_TOKEN\" x\n");
        check_no_credential(&read(WORKFLOW), &s);
    }
);

mod another_run {
    use super::*;
    negative_control!(
        wsl_jobs_and_scripts_hold_no_credential,
        "the image fetched from another run, with a token",
        expected = "wsl-register downloads with a token or from another run",
        check_no_credential(
            &edited(
                &read(WORKFLOW),
                "          path: ${{ runner.temp }}/wsl-image\n      - name: Check this run's image\n",
                "          path: ${{ runner.temp }}/wsl-image\n          run-id: 1\n      - name: Check this run's image\n"
            ),
            &scripts()
        )
    );
}

// ---- Downloads that fail, hang or end short, and job-only re-runs ------------------------------------------------

/// The download action, pinned in the workflow.
const DOWNLOAD: &str = "actions/download-artifact@";
/// `wsl-register`'s lookup of this recipe's image in the PC's cache didn't find it whole.
const CACHE_MISS: &str = "steps.cache.outcome != 'success'";

fn is_download(step: &Yaml) -> bool {
    step["uses"]
        .as_str()
        .is_some_and(|u| u.starts_with(DOWNLOAD))
}

fn is_image_check(step: &Yaml) -> bool {
    step["run"]
        .as_str()
        .is_some_and(|r| r.contains("\\wsl.ps1 check-image\n"))
}

/// Every download in the distro's jobs has a time limit of its own and `continue-on-error`, so a failure, or a hang its
/// limit ends, leads on to the next step instead of ending the job; download-artifact v4.3.0 can also stall, exit 0
/// and pass over part of a file, setting `download-path` only once a download has finished. A small artifact's
/// download is followed by the same download again, run only if the first set no `download-path`, and then by a step
/// that fails the job naming it if neither set one. The image's each try is followed by `wsl.ps1 check-image` on the
/// directory it wrote, run only if that try set `download-path`; the second try runs unless the first check passed,
/// and the stop unless one of the checks did. The image's first try, its second and its stop also run only on a miss of
/// the PC's image cache (`ci_wsl_cache.rs`). The three downloads are the image, the manifest and the parity results.
fn check_downloads(text: &str) {
    let doc = parse(text);
    let mut seen = 0;
    for (job, _) in JOBS {
        let steps = doc["jobs"][job]["steps"].as_vec().expect("steps");
        let mut at = 0;
        while at < steps.len() {
            let first = &steps[at];
            if !is_download(first) {
                at += 1;
                continue;
            }
            seen += 1;
            let what = first["with"]["name"].as_str().unwrap_or("");
            let tolerant = |s: &Yaml, which: &str| {
                assert_eq!(
                    s["continue-on-error"].as_bool(),
                    Some(true),
                    "{job}'s {which} of {what} ends the job when it fails or hangs"
                );
            };
            let limited = |s: &Yaml, which: &str| {
                assert!(
                    s["timeout-minutes"].as_i64().is_some_and(|m| m > 0),
                    "{job}'s {which} of {what} has no time limit of its own, so a hang takes the job"
                );
            };
            let id = |s: &Yaml, which: &str| -> String {
                s["id"]
                    .as_str()
                    .unwrap_or_else(|| {
                        panic!("{job}'s {which} of {what} has no id, so nothing can see it didn't finish")
                    })
                    .to_string()
            };
            let finished = |id: &str| format!("steps.{id}.outputs.download-path != ''");
            let unfinished = |id: &str| format!("steps.{id}.outputs.download-path == ''");
            let first_id = id(first, "download");
            tolerant(first, "download");
            limited(first, "download");
            let image = what.starts_with("wsl-image-");
            if image {
                assert_eq!(
                    first["if"].as_str(),
                    Some(CACHE_MISS),
                    "{job}'s download of {what} runs on a hit of the image cache"
                );
            }
            let check = |s: Option<&Yaml>, which: &str, after: &str| -> String {
                let s = s.filter(|s| is_image_check(s)).unwrap_or_else(|| {
                    panic!("{job}'s {which} of {what} isn't checked for its length and hash")
                });
                assert_eq!(
                    s["if"].as_str(),
                    Some(finished(after).as_str()),
                    "{job}'s check of {what} doesn't follow its finished download"
                );
                assert_eq!(
                    s["env"]["WSL_IMAGE"], first["with"]["path"],
                    "{job}'s check of {what} checks another directory than the download wrote"
                );
                tolerant(s, "check");
                id(s, "check")
            };
            let mut next = at + 1;
            let again_if = if image {
                let checked = check(steps.get(next), "download", &first_id);
                next += 1;
                format!("steps.{checked}.outcome != 'success' && {CACHE_MISS}")
            } else {
                unfinished(&first_id)
            };
            let again = steps
                .get(next)
                .filter(|s| {
                    is_download(s)
                        && s["uses"] == first["uses"]
                        && s["with"] == first["with"]
                        && s["if"].as_str() == Some(again_if.as_str())
                })
                .unwrap_or_else(|| {
                    panic!("{job}'s download of {what} isn't run again when it didn't finish whole")
                });
            tolerant(again, "second download");
            limited(again, "second download");
            assert_eq!(
                again["timeout-minutes"], first["timeout-minutes"],
                "{job}'s two downloads of {what} have different limits"
            );
            let again_id = id(again, "second download");
            next += 1;
            let neither = if image {
                let first_check = id(&steps[at + 1], "check");
                let second_check = check(steps.get(next), "second download", &again_id);
                next += 1;
                format!(
                    "steps.{first_check}.outcome != 'success' && steps.{second_check}.outcome != 'success' && {CACHE_MISS}"
                )
            } else {
                format!("{} && {}", unfinished(&first_id), unfinished(&again_id))
            };
            assert!(
                steps.get(next).is_some_and(|s| {
                    s["if"].as_str() == Some(neither.as_str())
                        && s["continue-on-error"].is_badvalue()
                        && s["run"].as_str().is_some_and(|r| {
                            r.contains("::error::") && r.trim_end().lines().last() == Some("exit 1")
                        })
                }),
                "{job} goes on when neither download of {what} finished"
            );
            at = next + 1;
        }
    }
    assert_eq!(
        seen, 3,
        "the distro's jobs download {seen} artifacts, not the image, the manifest and the parity results"
    );
}

#[test]
fn wsl_downloads_run_again_and_stop_if_neither_finished() {
    check_downloads(&read(WORKFLOW));
}

negative_control!(
    wsl_downloads_run_again_and_stop_if_neither_finished,
    "the parity results downloaded once",
    expected = "wsl-proofs's download of wsl-parity-${{ github.run_id }} isn't run again when it didn't finish",
    check_downloads(&edited(
        &read(WORKFLOW),
        "        if: steps.parity.outputs.download-path == ''\n",
        "        if: steps.parity.outputs.download-path != ''\n"
    ))
);

mod download_unnamed {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the image's download with no id",
        expected = "wsl-register's download of wsl-image-${{ github.run_id }} has no id",
        check_downloads(&edited(&read(WORKFLOW), "        id: image\n", ""))
    );
}

mod download_again_other {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a second download of another artifact",
        expected = "wsl-proofs's download of wsl-manifest-${{ github.run_id }} isn't run again",
        check_downloads(&edited(
            &read(WORKFLOW),
            "          name: wsl-manifest-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-image\n      - name: This run's rebuilt manifest, never",
            "          name: wsl-other-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-image\n      - name: This run's rebuilt manifest, never"
        ))
    );
}

mod download_again_unnamed {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a second download with no id",
        expected = "wsl-register's second download of wsl-image-${{ github.run_id }} has no id",
        check_downloads(&edited(&read(WORKFLOW), "        id: image-again\n", ""))
    );
}

mod neither_passes {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a job that goes on with neither image whole",
        expected =
            "wsl-register goes on when neither download of wsl-image-${{ github.run_id }} finished",
        check_downloads(&edited(
            &read(WORKFLOW),
            "the length and SHA-256 built'\n          exit 1\n",
            "the length and SHA-256 built'\n          exit 0\n"
        ))
    );
}

mod neither_unchecked {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a stop that looks only at the first download",
        expected = "wsl-proofs goes on when neither download of wsl-parity-${{ github.run_id }} finished",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        if: steps.parity.outputs.download-path == '' && steps.parity-again.outputs.download-path == ''\n",
            "        if: steps.parity.outputs.download-path == ''\n"
        ))
    );
}

mod download_dropped {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the parity results never downloaded",
        expected = "the distro's jobs download 2 artifacts",
        {
            let text = read(WORKFLOW);
            let from = text
                .find("      - name: The hosted runner's parity results\n")
                .expect("the parity download");
            let to = text
                .find("      - name: The six proofs\n")
                .expect("the proofs step");
            check_downloads(&format!("{}{}", &text[..from], &text[to..]));
        }
    );
}

mod image_on_a_hit {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the image downloaded on a cache hit too",
        expected = "wsl-register's download of wsl-image-${{ github.run_id }} runs on a hit of the image cache",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        timeout-minutes: 25\n        if: steps.cache.outcome != 'success'\n",
            "        timeout-minutes: 25\n"
        ))
    );
}

mod hang_ends_job {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the image's download that ends the job when it hangs",
        expected = "wsl-register's download of wsl-image-${{ github.run_id }} ends the job when it fails or hangs",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        id: image\n        timeout-minutes: 25\n        if: steps.cache.outcome != 'success'\n        continue-on-error: true\n",
            "        id: image\n        timeout-minutes: 25\n        if: steps.cache.outcome != 'success'\n"
        ))
    );
}

mod hang_unlimited {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the manifest's download with only the job's limit",
        expected = "wsl-proofs's download of wsl-manifest-${{ github.run_id }} has no time limit of its own",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        id: manifest\n        timeout-minutes: 5\n",
            "        id: manifest\n"
        ))
    );
}

mod again_longer {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a second try of the image with a longer limit",
        expected =
            "wsl-register's two downloads of wsl-image-${{ github.run_id }} have different limits",
        check_downloads(&edited(
            &read(WORKFLOW),
            "          && steps.cache.outcome != 'success'\n        timeout-minutes: 25\n",
            "          && steps.cache.outcome != 'success'\n        timeout-minutes: 30\n"
        ))
    );
}

mod image_unchecked {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the image's download not checked",
        expected = "wsl-register's download of wsl-image-${{ github.run_id }} isn't checked for its length and hash",
        check_downloads(&edited(
            &read(WORKFLOW),
            "\\wsl.ps1 check-image\n",
            "\\wsl.ps1 cpu\n"
        ))
    );
}

mod again_unchecked {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "the second image never checked",
        expected = "wsl-register's check of wsl-image-${{ github.run_id }} doesn't follow its finished download",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        if: steps.image-again.outputs.download-path != ''\n",
            "        if: false\n"
        ))
    );
}

mod check_elsewhere {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a check of another directory",
        expected =
            "wsl-register's check of wsl-image-${{ github.run_id }} checks another directory",
        check_downloads(&edited(
            &read(WORKFLOW),
            "          WSL_IMAGE: ${{ runner.temp }}/wsl-image\n        run: |\n",
            "          WSL_IMAGE: ${{ runner.temp }}/elsewhere\n        run: |\n"
        ))
    );
}

mod bad_image_kept {
    use super::*;
    negative_control!(
        wsl_downloads_run_again_and_stop_if_neither_finished,
        "a finished but bad image not tried again",
        expected = "wsl-register's download of wsl-image-${{ github.run_id }} isn't run again when it didn't finish whole",
        check_downloads(&edited(
            &read(WORKFLOW),
            "        if: steps.image-check.outcome != 'success'\n          && steps.cache.outcome",
            "        if: steps.image.outputs.download-path == ''\n          && steps.cache.outcome"
        ))
    );
}

/// The slower of the two whole downloads of the image on the PC, in seconds: 16 min 27.8 s (run 42954587933; the
/// other, run 42944409475, took 15 min 51.5 s). Each try of the image gets 1.5 times it, rounded up to the minute.
const SLOWER_WHOLE_IMAGE: f64 = 987.8;
/// The longest the PC took from the job's start to the image's download, in minutes, rounded up: 3 min 59 s.
const REACH_DOWNLOAD: i64 = 4;
/// The two checks' SHA-256 of the image, in minutes.
const CHECKS: i64 = 5;
/// What the job's old limit of 60 minutes left the registration after a 25-minute download, in minutes.
const REGISTRATION: i64 = 31;
/// The cache lookup's limit, in minutes: it reads the cached image once, for its SHA-256, as a check does.
const LOOKUP: i64 = 5;

/// The image's tries are each 1.5 times its slower whole download, rounded up to the minute; the cache lookup has a
/// limit of its own, a check's; `wsl-register`'s limit holds reaching the download, the lookup, both tries, both checks
/// and the registration; and in each job the downloads' limits together end before the job's does, so the stop step
/// runs.
fn check_limits(text: &str) {
    let doc = parse(text);
    let minutes = |s: &Yaml| s["timeout-minutes"].as_i64().unwrap_or(0);
    let register = &doc["jobs"]["wsl-register"];
    let image = register["steps"]
        .as_vec()
        .expect("steps")
        .iter()
        .find(|s| is_download(s))
        .expect("the image's download");
    let per_try = (1.5 * SLOWER_WHOLE_IMAGE / 60.0).ceil() as i64;
    assert_eq!(
        minutes(image),
        per_try,
        "the image's download limit isn't 1.5 times its slower whole download, rounded up to the minute"
    );
    let lookup = register["steps"]
        .as_vec()
        .expect("steps")
        .iter()
        .find(|s| s["id"].as_str() == Some("cache"))
        .expect("the cache lookup");
    assert_eq!(
        minutes(lookup),
        LOOKUP,
        "the cache lookup's limit isn't a check's"
    );
    assert!(
        minutes(register) >= REACH_DOWNLOAD + LOOKUP + 2 * per_try + CHECKS + REGISTRATION,
        "wsl-register's limit doesn't hold two tries of the image and the registration"
    );
    for job in ["wsl-register", "wsl-proofs"] {
        let job_minutes = minutes(&doc["jobs"][job]);
        let downloads: i64 = doc["jobs"][job]["steps"]
            .as_vec()
            .expect("steps")
            .iter()
            .filter(|s| is_download(s))
            .map(minutes)
            .sum();
        assert!(
            downloads < job_minutes,
            "the downloads' limits outlast {job}'s: {downloads} of {job_minutes} minutes"
        );
    }
}

#[test]
fn wsl_download_limits_fit_the_evidence() {
    check_limits(&read(WORKFLOW));
}

negative_control!(
    wsl_download_limits_fit_the_evidence,
    "a try shorter than the evidence allows",
    expected = "the image's download limit isn't 1.5 times its slower whole download",
    check_limits(&edited(
        &read(WORKFLOW),
        "        id: image\n        timeout-minutes: 25\n",
        "        id: image\n        timeout-minutes: 20\n"
    ))
);

mod job_too_short {
    use super::*;
    negative_control!(
        wsl_download_limits_fit_the_evidence,
        "wsl-register a minute short",
        expected = "wsl-register's limit doesn't hold two tries of the image and the registration",
        check_limits(&edited(
            &read(WORKFLOW),
            "    runs-on: [self-hosted, windows, nvidia]\n    timeout-minutes: 95\n",
            "    runs-on: [self-hosted, windows, nvidia]\n    timeout-minutes: 94\n"
        ))
    );
}

mod lookup_unlimited {
    use super::*;
    negative_control!(
        wsl_download_limits_fit_the_evidence,
        "a cache lookup with only the job's limit",
        expected = "the cache lookup's limit isn't a check's",
        check_limits(&edited(
            &read(WORKFLOW),
            "        id: cache\n        timeout-minutes: 5\n",
            "        id: cache\n"
        ))
    );
}

mod downloads_outlast {
    use super::*;
    negative_control!(
        wsl_download_limits_fit_the_evidence,
        "wsl-proofs as short as its downloads",
        expected = "the downloads' limits outlast wsl-proofs's: 20 of 20 minutes",
        check_limits(&edited(
            &read(WORKFLOW),
            "    timeout-minutes: 180\n",
            "    timeout-minutes: 20\n"
        ))
    );
}

/// What is downloaded is found again by a re-run of the failed jobs alone: each name a download asks for is uploaded in
/// this workflow, names the run and not the attempt (attempt 2 asks for attempt 1's artifact, as the hosted job isn't
/// re-run), and is uploaded with `overwrite: true`, so a re-run of the hosted job replaces it rather than failing.
fn check_names(text: &str) {
    let doc = parse(text);
    let jobs = doc["jobs"].as_hash().expect("the jobs");
    let steps = |job: &Yaml| {
        job["steps"]
            .as_vec()
            .map_or(&[][..], Vec::as_slice)
            .to_vec()
    };
    let mut uploads = Vec::new();
    let mut downloads = Vec::new();
    for (id, job) in jobs {
        let id = id.as_str().unwrap_or("");
        for step in steps(job) {
            let name = step["with"]["name"].as_str().unwrap_or("").to_string();
            if step["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/upload-artifact@"))
            {
                uploads.push((id.to_string(), name, step["with"]["overwrite"].as_bool()));
            } else if is_download(&step) {
                downloads.push((id.to_string(), name));
            }
        }
    }
    let mut names: Vec<&str> = downloads.iter().map(|(_, n)| n.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        3,
        "the workflow downloads {names:?}, not three artifacts"
    );
    for (job, name) in &downloads {
        assert!(
            name.contains("${{ github.run_id }}") && !name.contains("github.run_attempt"),
            "{job} downloads {name}, which names the attempt or not the run: a re-run of the failed jobs alone can't find the first attempt's"
        );
        let (from, _, overwrite) = uploads
            .iter()
            .find(|(_, n, _)| n == name)
            .unwrap_or_else(|| panic!("{job} downloads {name}, which no job uploads"));
        assert_eq!(
            *overwrite,
            Some(true),
            "{name}'s upload doesn't overwrite: a re-run of {from} would fail on the first attempt's"
        );
    }
}

#[test]
fn wsl_artifacts_are_found_by_a_rerun_of_the_failed_jobs() {
    check_names(&read(WORKFLOW));
}

negative_control!(
    wsl_artifacts_are_found_by_a_rerun_of_the_failed_jobs,
    "the image named by its attempt, on both sides",
    expected = "wsl-register downloads wsl-image-${{ github.run_id }}-${{ github.run_attempt }}, which names the attempt",
    check_names(&read(WORKFLOW).replace(
        "name: wsl-image-${{ github.run_id }}\n",
        "name: wsl-image-${{ github.run_id }}-${{ github.run_attempt }}\n"
    ))
);

mod upload_kept {
    use super::*;
    negative_control!(
        wsl_artifacts_are_found_by_a_rerun_of_the_failed_jobs,
        "the manifest's upload without overwrite",
        expected = "wsl-manifest-${{ github.run_id }}'s upload doesn't overwrite: a re-run of wsl-image would fail",
        check_names(&edited(
            &read(WORKFLOW),
            "          path: ${{ runner.temp }}/wsl-image/manifest.txt\n          retention-days: 90\n          overwrite: true\n",
            "          path: ${{ runner.temp }}/wsl-image/manifest.txt\n          retention-days: 90\n"
        ))
    );
}

mod upload_other {
    use super::*;
    negative_control!(
        wsl_artifacts_are_found_by_a_rerun_of_the_failed_jobs,
        "the parity results uploaded under another name",
        expected = "wsl-proofs downloads wsl-parity-${{ github.run_id }}, which no job uploads",
        check_names(&edited(
            &read(WORKFLOW),
            "          name: wsl-parity-${{ github.run_id }}\n          path: ${{ runner.temp }}/wsl-parity\n          retention-days: 90\n",
            "          name: wsl-parity-${{ github.run_id }}-hosted\n          path: ${{ runner.temp }}/wsl-parity\n          retention-days: 90\n"
        ))
    );
}

mod download_unrun {
    use super::*;
    negative_control!(
        wsl_artifacts_are_found_by_a_rerun_of_the_failed_jobs,
        "a download named without the run",
        expected = "wsl-register downloads wsl-image, which names the attempt or not the run",
        check_names(&read(WORKFLOW).replace(
            "name: wsl-image-${{ github.run_id }}\n",
            "name: wsl-image\n"
        ))
    );
}

/// `build-image.sh` records the tarball's length beside its hash. `Assert-Image` reads it and fails naming a short copy,
/// N of M bytes, before it hashes the copy; `wsl.ps1 check-image`, which the workflow runs after each download, is
/// that check and nothing else, and `register` runs it again before the import.
fn check_short_copy(build: &str, script: &str) {
    let tar = build
        .find("sudo tar --numeric-owner -czf \"$out/rootfs.tar.gz\"")
        .expect("no tarball");
    assert!(
        build
            .find("wc -c < \"$out/rootfs.tar.gz\" | tr -d '[:space:]' > \"$out/rootfs.bytes\"\n")
            .is_some_and(|w| w > tar),
        "{BUILD} doesn't record the tarball's length"
    );
    let assert_image = ps_function(script, "Assert-Image");
    let hashed = assert_image
        .find("$copied = Get-Sha256 $rootfs")
        .expect("Assert-Image hashes no copy");
    let short = assert_image.find(
        "if ($copiedLength -lt [int64]$length) { Fail \"rootfs.tar.gz is a short copy, $copiedLength of $length bytes:",
    );
    assert!(
        assert_image.contains("$copiedLength = (Get-Item -LiteralPath $rootfs).Length")
            && assert_image.contains("if ($length -notmatch '^[0-9]+$') { Fail")
            && short.is_some_and(|s| s < hashed),
        "Assert-Image doesn't name a short copy before hashing it"
    );
    assert_eq!(
        arm(script, "check-image"),
        "    'check-image' {\n        Assert-Image $env:WSL_IMAGE\n        exit 0\n    }",
        "wsl.ps1 check-image does other than check the image"
    );
}

#[test]
fn wsl_register_names_a_short_copy() {
    check_short_copy(&read(BUILD), &read(PS));
}

negative_control!(
    wsl_register_names_a_short_copy,
    "no length recorded",
    expected = "doesn't record the tarball's length",
    check_short_copy(
        &edited(
            &read(BUILD),
            "wc -c < \"$out/rootfs.tar.gz\" | tr -d '[:space:]' > \"$out/rootfs.bytes\"\n",
            ""
        ),
        &read(PS)
    )
);

mod length_unchecked {
    use super::*;
    negative_control!(
        wsl_register_names_a_short_copy,
        "the length never compared",
        expected = "Assert-Image doesn't name a short copy before hashing it",
        check_short_copy(
            &read(BUILD),
            &edited(
                &read(PS),
                "if ($copiedLength -lt [int64]$length)",
                "if ($false)"
            )
        )
    );
}

mod length_after_hash {
    use super::*;
    negative_control!(
        wsl_register_names_a_short_copy,
        "the length compared after the hash",
        expected = "Assert-Image doesn't name a short copy before hashing it",
        {
            let ps = read(PS);
            let line = ps
                .lines()
                .find(|l| l.contains("if ($copiedLength -lt [int64]$length)"))
                .expect("the length check")
                .to_string();
            let moved = edited(&ps, &format!("{line}\n"), "");
            let moved = edited(
                &moved,
                "    $copied = Get-Sha256 $rootfs\n",
                &format!("    $copied = Get-Sha256 $rootfs\n{line}\n"),
            );
            check_short_copy(&read(BUILD), &moved);
        }
    );
}

mod check_image_passes {
    use super::*;
    negative_control!(
        wsl_register_names_a_short_copy,
        "check-image that checks nothing",
        expected = "wsl.ps1 check-image does other than check the image",
        check_short_copy(
            &read(BUILD),
            &edited(&read(PS), "        Assert-Image $env:WSL_IMAGE\n", "")
        )
    );
}

// ---- wsl-cpu: one named mutation workload --------------------------------------------------------------------------

/// `.github/wsl-cpu-enabled` exists, added once all six proofs had passed (run 43756345312) and the owner answered
/// RQ-825 (closed by R-1029), and `wsl.ps1 cpu` still refuses first thing without it, so removing it turns `wsl-cpu` off.
fn check_flag(flag_exists: bool, script: &str) {
    assert!(
        flag_exists,
        "{FLAG} is absent: wsl-cpu can't run its mutation workload"
    );
    let cpu = arm(script, "cpu");
    let first = cpu.lines().nth(1).unwrap_or("").trim();
    assert_eq!(
        first, "if (-not (Test-Path -LiteralPath '.github\\wsl-cpu-enabled')) {",
        "wsl.ps1 cpu doesn't check the flag first"
    );
    assert!(
        cpu.lines()
            .nth(2)
            .unwrap_or("")
            .trim()
            .starts_with("Fail '"),
        "wsl.ps1 cpu goes on without the flag"
    );
}

#[test]
fn wsl_cpu_is_off_unless_its_flag_exists() {
    check_flag(root().join(FLAG).exists(), &read(PS));
}

negative_control!(
    wsl_cpu_is_off_unless_its_flag_exists,
    "the flag removed",
    expected = "wsl-cpu can't run its mutation workload",
    check_flag(false, &read(PS))
);

mod flag_unchecked {
    use super::*;
    negative_control!(
        wsl_cpu_is_off_unless_its_flag_exists,
        "a cpu arm that doesn't look for the flag",
        expected = "wsl.ps1 cpu doesn't check the flag first",
        check_flag(
            true,
            &edited(
                &read(PS),
                "        if (-not (Test-Path -LiteralPath '.github\\wsl-cpu-enabled')) {",
                "        if ($false) {"
            )
        )
    );
}

/// The five inputs that name `wsl-cpu`'s workload, and the environment variable of its step each reaches.
const WORKLOAD: [(&str, &str); 5] = [
    ("mutants_commit", "WSL_MUTANTS_COMMIT"),
    ("mutants_base", "WSL_MUTANTS_BASE"),
    ("mutants_packages", "WSL_MUTANTS_PACKAGES"),
    ("mutants_timeout", "WSL_MUTANTS_TIMEOUT"),
    ("mutants_filter", "WSL_MUTANTS_FILTER"),
];

/// The two optional inputs that tune it (R-1017: made faster on the PC), cargo mutants' jobs and shard, and the
/// environment variable of its step each reaches.
const TUNING: [(&str, &str); 2] = [
    ("mutants_jobs", "WSL_MUTANTS_JOBS"),
    ("mutants_shard", "WSL_MUTANTS_SHARD"),
];

/// `wsl-cpu` runs the one named mutation workload and nothing else (the owner's answer to RQ-825 (closed by R-1029); R-1014 and R-912
/// keep the CPU tests and the job queue's mutants off the PC): each of the five inputs is a string, empty unless the
/// dispatch sets it, and reaches the run only as an environment variable of the step that starts `wsl.ps1 cpu`, never
/// as text in a script, as do the two that tune it, the jobs a choice of 1 to 4, 1 by default, and the shard a
/// string, empty by default; that arm refuses unless each of the five is set, and starts no verb of `inside.sh` but
/// `mutants`, its workload on stdin, and `results`.
fn check_workload(workflow: &str, script: &str) {
    let doc = parse(workflow);
    let inputs = &doc["on"]["workflow_dispatch"]["inputs"];
    for (input, _) in WORKLOAD {
        assert!(
            inputs[input]["type"].as_str() == Some("string")
                && inputs[input]["default"].as_str() == Some(""),
            "the dispatch input {input} is not a string, empty by default"
        );
    }
    let options: Vec<&str> = inputs["mutants_jobs"]["options"]
        .as_vec()
        .map(|o| o.iter().filter_map(Yaml::as_str).collect())
        .unwrap_or_default();
    assert!(
        inputs["mutants_jobs"]["type"].as_str() == Some("choice")
            && inputs["mutants_jobs"]["default"].as_str() == Some("1")
            && options == ["1", "2", "3", "4"],
        "the dispatch input mutants_jobs is not a choice of 1 to 4, 1 by default"
    );
    assert!(
        inputs["mutants_shard"]["type"].as_str() == Some("string")
            && inputs["mutants_shard"]["default"].as_str() == Some(""),
        "the dispatch input mutants_shard is not a string, empty by default"
    );
    let steps = doc["jobs"]["wsl-cpu"]["steps"].as_vec().expect("steps");
    let runs: Vec<&Yaml> = steps
        .iter()
        .filter(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("\\wsl.ps1 cpu"))
        })
        .collect();
    assert_eq!(runs.len(), 1, "wsl-cpu doesn't start wsl.ps1 cpu once");
    let env = runs[0]["env"].as_hash().cloned().unwrap_or_default();
    let want: Vec<(String, String)> = WORKLOAD
        .iter()
        .chain(TUNING.iter())
        .map(|(input, var)| ((*var).to_owned(), format!("${{{{ inputs.{input} }}}}")))
        .collect();
    let got: Vec<(String, String)> = env
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().unwrap_or("").to_owned(),
                v.as_str().unwrap_or("").to_owned(),
            )
        })
        .collect();
    assert_eq!(got, want, "wsl-cpu's step doesn't hand the workload over");
    for line in workflow.lines() {
        assert!(
            !line.contains("inputs.mutants_") || line.trim_start().starts_with("WSL_MUTANTS_"),
            "the workflow reads a workload input other than as its step's environment: {line}"
        );
    }
    let cpu = arm(script, "cpu");
    let lines: Vec<&str> = cpu.lines().map(str::trim).collect();
    let at = |what: &str| {
        lines
            .iter()
            .position(|l| l.starts_with(what))
            .unwrap_or_else(|| panic!("wsl.ps1 cpu has no line `{what}`"))
    };
    let workload = at("$workload = @($env:WSL_MUTANTS_COMMIT, $env:WSL_MUTANTS_BASE, $env:WSL_MUTANTS_PACKAGES, $env:WSL_MUTANTS_TIMEOUT, $env:WSL_MUTANTS_FILTER)");
    assert!(
        lines.get(workload + 1)
            == Some(&"if (@($workload | Where-Object { -not \"$_\".Trim() }).Count) {")
            && lines
                .get(workload + 2)
                .is_some_and(|l| l.starts_with("Fail 'cpu runs one named mutation workload")),
        "wsl.ps1 cpu goes on without the whole workload"
    );
    assert!(
        workload < at("$cloned = Initialize-Clone"),
        "wsl.ps1 cpu clones before checking the workload"
    );
    let verbs: Vec<&str> = cpu
        .match_indices("$Inside, '")
        .map(|(i, m)| {
            let rest = &cpu[i + m.len()..];
            &rest[..rest.find('\'').unwrap_or(0)]
        })
        .collect();
    assert_eq!(
        verbs,
        ["trust", "mutants", "results"],
        "wsl.ps1 cpu starts more inside than the mutation workload"
    );
    assert!(
        cpu.contains("Invoke-Ci @('bash', $Inside, 'mutants') $stdin\n"),
        "wsl.ps1 cpu doesn't send the workload on stdin"
    );
    let trust = at("$trust = Invoke-Ci @('bash', $Inside, 'trust') $stdin");
    assert!(
        lines
            .get(trust + 1)
            .is_some_and(|l| l.starts_with("if ($trust.Code -ne 0) { Fail "))
            && trust < at("$run = Invoke-Ci @('bash', $Inside, 'mutants') $stdin"),
        "wsl.ps1 cpu runs the workload though inside.sh trust refused it"
    );
}

#[test]
fn wsl_cpu_runs_only_the_named_mutation_workload() {
    check_workload(&read(WORKFLOW), &read(PS));
}

negative_control!(
    wsl_cpu_runs_only_the_named_mutation_workload,
    "the CPU tests and the job queue's mutants started again",
    expected = "wsl.ps1 cpu starts more inside than the mutation workload",
    check_workload(
        &read(WORKFLOW),
        &edited(
            &read(PS),
            "            $run = Invoke-Ci @('bash', $Inside, 'mutants') $stdin\n",
            "            $run = Invoke-Ci @('bash', $Inside, 'cpu')\n            $run = Invoke-Ci @('bash', $Inside, 'mutants') $stdin\n"
        )
    )
);

mod workload_in_script {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "an input written into the step's script",
        expected = "the workflow reads a workload input other than as its step's environment",
        check_workload(
            &edited(
                &read(WORKFLOW),
                "\\wsl.ps1 cpu\n",
                "\\wsl.ps1 cpu ${{ inputs.mutants_filter }}\n"
            ),
            &read(PS)
        )
    );
}

mod workload_dropped {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "the filterset not handed over",
        expected = "wsl-cpu's step doesn't hand the workload over",
        check_workload(
            &edited(
                &read(WORKFLOW),
                "          WSL_MUTANTS_FILTER: ${{ inputs.mutants_filter }}\n",
                ""
            ),
            &read(PS)
        )
    );
}

mod workload_unchecked {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "a cpu arm that runs with part of the workload",
        expected = "wsl.ps1 cpu goes on without the whole workload",
        check_workload(
            &read(WORKFLOW),
            &edited(
                &read(PS),
                "        if (@($workload | Where-Object { -not \"$_\".Trim() }).Count) {",
                "        if ($false) {"
            )
        )
    );
}

mod trust_ignored {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "inside.sh trust's refusal not stopping the run",
        expected = "wsl.ps1 cpu runs the workload though inside.sh trust refused it",
        check_workload(
            &read(WORKFLOW),
            &edited(
                &read(PS),
                "            if ($trust.Code -ne 0) { Fail ",
                "            if ($false) { Fail "
            )
        )
    );
}

mod input_default {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "an input with a default workload",
        expected = "the dispatch input mutants_packages is not a string, empty by default",
        check_workload(
            &edited(
                &read(WORKFLOW),
                "        description: wsl-cpu, the packages, space-separated, each a crate under crates/\n        type: string\n        default: \"\"\n",
                "        description: wsl-cpu, the packages, space-separated, each a crate under crates/\n        type: string\n        default: xtask\n"
            ),
            &read(PS)
        )
    );
}

mod jobs_default {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "more than one job by default",
        expected = "the dispatch input mutants_jobs is not a choice of 1 to 4, 1 by default",
        check_workload(
            &edited(
                &read(WORKFLOW),
                "        type: choice\n        default: \"1\"\n",
                "        type: choice\n        default: \"3\"\n"
            ),
            &read(PS)
        )
    );
}

mod shard_dropped {
    use super::*;
    negative_control!(
        wsl_cpu_runs_only_the_named_mutation_workload,
        "the shard not handed over",
        expected = "wsl-cpu's step doesn't hand the workload over",
        check_workload(
            &edited(
                &read(WORKFLOW),
                "          WSL_MUTANTS_SHARD: ${{ inputs.mutants_shard }}\n",
                ""
            ),
            &read(PS)
        )
    );
}

/// The `cpu` verb, dormant (`wsl.ps1` never starts it), builds in a target of its own, never in `mutants`' warm
/// build cache, ~/target: the cache is that verb's alone (R-1017's speed-up, the internal task's item 2).
fn check_cpu_target(inside: &str) {
    let start = inside.find("\ncpu() {\n").expect("inside.sh has cpu()");
    let body = &inside[start..start + inside[start..].find("\n}\n").expect("cpu() ends")];
    let target = body
        .lines()
        .find_map(|l| l.trim().strip_prefix("export CARGO_TARGET_DIR="))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("cpu() sets no CARGO_TARGET_DIR"));
    let cache = inside
        .lines()
        .find_map(|l| l.trim().strip_prefix("local cache="))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("mutants() names its cache");
    assert_ne!(
        target, cache,
        "the cpu verb builds in mutants' warm build cache"
    );
}

#[test]
fn wsl_inside_cpu_builds_outside_the_mutants_cache() {
    check_cpu_target(&read(INSIDE));
}

negative_control!(
    wsl_inside_cpu_builds_outside_the_mutants_cache,
    "the cpu verb back on ~/target",
    expected = "the cpu verb builds in mutants' warm build cache",
    check_cpu_target(&edited(
        &read(INSIDE),
        "export CARGO_TARGET_DIR=\"$HOME/target-cpu\"",
        "export CARGO_TARGET_DIR=\"$HOME/target\""
    ))
);

/// `wsl.ps1 cpu`'s two optional inputs (R-1017): refused with a line break in either, the jobs unless 1 to 4 (1 when
/// empty), and the run unless the drive holding the distro's disk has 3 GiB free at one job, or 12 GiB a job and 3 GiB
/// over at more, all before the distro is looked up; and the jobs and the shard go on stdin as lines six and seven.
fn check_tuning_ps(script: &str) {
    let cpu = arm(script, "cpu");
    let at = |what: &str| {
        cpu.find(what)
            .unwrap_or_else(|| panic!("wsl.ps1 cpu has no `{what}`"))
    };
    for global in ["\n$JobGiB = 12\n", "\n$FloorGiB = 3\n"] {
        assert!(
            script.contains(global),
            "wsl.ps1's room for a job is another: no `{}`",
            global.trim()
        );
    }
    let distro = at("Get-Distros");
    let checks = [
        "if (\"$($env:WSL_MUTANTS_JOBS)|$($env:WSL_MUTANTS_SHARD)\" -match \"[`r`n]\") { Fail ",
        "if (-not $jobs) { $jobs = '1' }\n        if ($jobs -notmatch '^[1-4]$') { Fail ",
        "$needGiB = $FloorGiB + $(if ([int]$jobs -gt 1) { [int]$jobs * $JobGiB } else { 0 })",
        "if ($freeGiB -lt $needGiB) { Fail ",
    ];
    for check in checks {
        assert!(
            at(check) < distro,
            "wsl.ps1 cpu checks its tuning after the distro's lookup: {check}"
        );
    }
    assert!(
        cpu.contains("@($workload) + @($jobs, \"$($env:WSL_MUTANTS_SHARD)\")"),
        "wsl.ps1 cpu doesn't send the jobs and the shard on stdin"
    );
}

#[test]
fn wsl_cpu_checks_its_jobs_shard_and_room_before_the_distro() {
    check_tuning_ps(&read(PS));
}

negative_control!(
    wsl_cpu_checks_its_jobs_shard_and_room_before_the_distro,
    "the drive's room not checked",
    expected = "wsl.ps1 cpu has no `if ($freeGiB -lt $needGiB) { Fail `",
    check_tuning_ps(&edited(
        &read(PS),
        "        if ($freeGiB -lt $needGiB) { Fail ",
        "        if ($false) { Fail "
    ))
);

/// `wsl.ps1 cpu` fails the run when the drive holding the distro's disk ends it below the 3 GiB floor, whatever the
/// run's own exit: the disk never shrinks, so a drive that filled under the run is still below the floor once the
/// distro stops, and its builds' failures would otherwise read as unviable mutants. The check reads the drive after
/// `Stop-Distro` and comes before the exit.
fn check_room_after_ps(script: &str) {
    let cpu = arm(script, "cpu");
    let at = |what: &str| {
        cpu.find(what)
            .unwrap_or_else(|| panic!("wsl.ps1 cpu has no `{what}`"))
    };
    let free = at("$freeAfterGiB = [math]::Floor($drive.AvailableFreeSpace / 1GB)\n");
    let check = at("if ($freeAfterGiB -lt $FloorGiB) { Fail ");
    let stop = cpu
        .rfind("Stop-Distro")
        .expect("wsl.ps1 cpu never stops the distro");
    let exit = at("exit $(if ($run.Code -ne 0) { $run.Code } else { $tar.Code })");
    assert!(
        stop < free && free < check && check < exit,
        "wsl.ps1 cpu checks the drive's room other than after the distro stops and before the exit"
    );
}

#[test]
fn wsl_cpu_fails_a_run_that_ends_below_the_floor() {
    check_room_after_ps(&read(PS));
}

negative_control!(
    wsl_cpu_fails_a_run_that_ends_below_the_floor,
    "the room after the run not checked",
    expected = "wsl.ps1 cpu has no `if ($freeAfterGiB -lt $FloorGiB) { Fail `",
    check_room_after_ps(&edited(
        &read(PS),
        "        if ($freeAfterGiB -lt $FloorGiB) { Fail ",
        "        if ($false) { Fail "
    ))
);

mod room_after_at_the_floor {
    use super::*;
    negative_control!(
        wsl_cpu_fails_a_run_that_ends_below_the_floor,
        "a run ending exactly at the floor failed",
        expected = "wsl.ps1 cpu has no `if ($freeAfterGiB -lt $FloorGiB) { Fail `",
        check_room_after_ps(&edited(
            &read(PS),
            "if ($freeAfterGiB -lt $FloorGiB) { Fail ",
            "if ($freeAfterGiB -le $FloorGiB) { Fail "
        ))
    );
}

mod room_after_before_the_stop {
    use super::*;
    negative_control!(
        wsl_cpu_fails_a_run_that_ends_below_the_floor,
        "the room read before the distro stops",
        expected = "wsl.ps1 cpu checks the drive's room other than after the distro stops and before the exit",
        check_room_after_ps(&edited(
            &edited(
                &read(PS),
                "        } finally {\n            Stop-Distro\n        }\n        $freeAfterGiB",
                "        } finally {\n        }\n        $freeAfterGiB"
            ),
            "        exit $(if ($run.Code -ne 0) { $run.Code } else { $tar.Code })\n",
            "        Stop-Distro\n        exit $(if ($run.Code -ne 0) { $run.Code } else { $tar.Code })\n"
        ))
    );
}

mod tuning_after_distro {
    use super::*;
    negative_control!(
        wsl_cpu_checks_its_jobs_shard_and_room_before_the_distro,
        "the jobs checked after the distro's lookup",
        expected = "wsl.ps1 cpu checks its tuning after the distro's lookup",
        check_tuning_ps(&edited(
            &edited(
                &read(PS),
                "        if (-not $jobs) { $jobs = '1' }\n        if ($jobs -notmatch '^[1-4]$') { Fail \"mutants_jobs '$jobs' is not 1 to 4\" }\n",
                ""
            ),
            "        if ((Get-Distros) -notcontains $Distro) { Fail \"$Distro isn't registered\" }\n",
            "        if ((Get-Distros) -notcontains $Distro) { Fail \"$Distro isn't registered\" }\n        if (-not $jobs) { $jobs = '1' }\n        if ($jobs -notmatch '^[1-4]$') { Fail \"mutants_jobs '$jobs' is not 1 to 4\" }\n"
        ))
    );
}

mod tuning_no_room {
    use super::*;
    negative_control!(
        wsl_cpu_checks_its_jobs_shard_and_room_before_the_distro,
        "no room asked for a job",
        expected = "wsl.ps1's room for a job is another",
        check_tuning_ps(&edited(&read(PS), "\n$JobGiB = 12\n", "\n$JobGiB = 0\n"))
    );
}

/// `inside.sh mutants`, run in a scratch repository whose `scripts/contain.sh` and `cargo` are stand-ins that record
/// how they were called.
#[cfg(unix)]
mod mutants {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use validation::spawn::{write_executable, Spawn};

    use crate::scratch::Scratch;

    /// The criterion's filterset (PR #552, TASK-M2-89).
    const FILTER: &str = "binary_id(=engine::forward_table_builder) | binary_id(=engine::qa_TASK-M2-89) | binary_id(=validation::qa_TASK-M2-89) | binary_id(=ledger::schema_version) | (binary_id(=validation) & test(/bench::tests::bench_forward_table/))";

    /// Records `check` and `run` with the mode they were given, and runs `run`'s command.
    const CONTAIN: &str = "#!/usr/bin/env bash\ncase $1 in\n  check) echo \"check ${PRIN_CONTAIN:-}\" >> \"$HOME/contain-calls\" ;;\n  run) shift; echo \"run ${PRIN_CONTAIN:-}\" >> \"$HOME/contain-calls\"; exec \"$@\" ;;\nesac\n";

    /// Records its arguments, one a line, and where and how it ran, its target among it; writes `mutants.out/` under
    /// its `-o`; exits with the code in `$HOME/cargo-exit`.
    const CARGO: &str = "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$HOME/cargo-args\"\n{ echo \"pwd $(pwd)\"; echo \"profile $NEXTEST_PROFILE\"; echo \"head $(git rev-parse HEAD)\"; echo \"target ${CARGO_TARGET_DIR:-unset}\"; echo \"tmpdir ${TMPDIR:-unset}\"; } > \"$HOME/cargo-env\"\nout=; prev=\nfor a; do [ \"$prev\" = -o ] && out=$a; prev=$a; done\nmkdir -p \"$out/mutants.out\"\necho '{\"outcomes\": []}' > \"$out/mutants.out/outcomes.json\"\nfor f in caught missed timeout unviable; do : > \"$out/mutants.out/$f.txt\"; done\nexit \"$(cat \"$HOME/cargo-exit\")\"\n";

    /// The first program named `name` on this test's PATH.
    fn which(name: &str) -> PathBuf {
        let path = std::env::var_os("PATH").expect("PATH is set");
        std::env::split_paths(&path)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no {name} on PATH"))
    }

    /// A scratch repository holding `inside`, the stand-ins, and four crates, with two commits: the base, and one
    /// changing engine. Returns the scratch, the base and the commit.
    fn repository(tag: &str, inside: &str) -> (Scratch, String, String) {
        let dir = Scratch::new(tag);
        let repo = dir.join("repo");
        let home = dir.join("home");
        let bin = home.join(".cargo/bin");
        std::fs::create_dir_all(repo.join("scripts/wsl")).expect("scratch");
        std::fs::create_dir_all(&bin).expect("scratch");
        std::fs::write(repo.join(INSIDE), inside).expect("scratch");
        write_executable(&repo.join("scripts/contain.sh"), CONTAIN).expect("scratch");
        for krate in ["engine", "validation", "ledger", "gui"] {
            std::fs::create_dir_all(repo.join(format!("crates/{krate}/src"))).expect("scratch");
            std::fs::write(
                repo.join(format!("crates/{krate}/Cargo.toml")),
                format!("[package]\nname = \"{krate}\"\n"),
            )
            .expect("scratch");
            std::fs::write(repo.join(format!("crates/{krate}/src/lib.rs")), "").expect("scratch");
        }
        std::fs::write(
            repo.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"t\"\n",
        )
        .expect("scratch");
        std::fs::write(repo.join("Cargo.lock"), "version = 4\n").expect("scratch");
        std::os::unix::fs::symlink(which("git"), bin.join("git")).expect("scratch");
        write_executable(&bin.join("cargo"), CARGO).expect("scratch");
        let git = |args: &[&str]| {
            let out = Command::new(bin.join("git"))
                .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
                .args(["-c", "commit.gpgsign=false", "-C"])
                .arg(&repo)
                .args(args)
                .env("HOME", &home)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output_within(Duration::from_secs(60))
                .expect("git ran");
            assert!(out.status.success(), "scratch git {args:?}: {out:?}");
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        let base = git(&["rev-parse", "HEAD"]);
        std::fs::write(
            repo.join("crates/engine/src/lib.rs"),
            "pub fn f() -> u8 { 1 }\n",
        )
        .expect("scratch");
        git(&["commit", "-q", "-a", "-m", "change"]);
        let commit = git(&["rev-parse", "HEAD"]);
        (dir, base, commit)
    }

    /// `inside.sh mutants` with `workload` on stdin, cargo exiting `exit`: its exit code and output. Each run starts
    /// with no record of an earlier one.
    fn run(dir: &Scratch, workload: &str, exit: i32) -> (Option<i32>, String) {
        let home = dir.join("home");
        for f in ["cargo-args", "cargo-env", "contain-calls"] {
            let _ = std::fs::remove_file(home.join(f));
        }
        std::fs::write(home.join("cargo-exit"), exit.to_string()).expect("scratch");
        std::fs::write(dir.join("workload"), workload).expect("scratch");
        let out = Command::new("bash")
            .args(["-c", "exec bash \"$1\" mutants < \"$2\"", "inside"])
            .arg(dir.join("repo").join(INSIDE))
            .arg(dir.join("workload"))
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output_within(Duration::from_secs(120))
            .expect("bash ran");
        (
            out.status.code(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    fn spec(commit: &str, base: &str, packages: &str, timeout: &str, filter: &str) -> String {
        format!("{commit}\n{base}\n{packages}\n{timeout}\n{filter}\n")
    }

    /// The criterion's workload runs, through `contain.sh run` in its unprivileged mode after `contain.sh check`, as
    /// exactly `cargo mutants --in-diff <base..commit> --package engine --package validation --package ledger -j 1
    /// --timeout 300 -o ~/results -- -E <filterset>`, at the commit, under nextest's `mutants` profile; the diff, the
    /// wall time and the host are kept. cargo mutants' 2 (missed) and 3 (timed out) are results, the verb passing;
    /// any other failure fails it. Each malformed input is refused before cargo runs, and the bounds of each check
    /// hold: a timeout of 1 and of 9999 seconds runs, 0 and 10000 don't.
    fn check_mutants(tag: &str, inside: &str) {
        let (dir, base, commit) = repository(tag, inside);
        let home = dir.join("home");
        let results = home.join("results");
        let good = spec(&commit, &base, "engine validation ledger", "300", FILTER);
        let (code, out) = run(&dir, &good, 0);
        assert_eq!(
            code,
            Some(0),
            "inside.sh mutants fails the criterion's workload:\n{out}"
        );
        let args = std::fs::read_to_string(home.join("cargo-args"))
            .unwrap_or_else(|_| panic!("inside.sh mutants never ran cargo:\n{out}"));
        let diff = results.join("diff.patch");
        let want = [
            "mutants",
            "--in-diff",
            &diff.display().to_string(),
            "--package",
            "engine",
            "--package",
            "validation",
            "--package",
            "ledger",
            "-j",
            "1",
            "--timeout",
            "300",
            "-o",
            &results.display().to_string(),
            "--",
            "-E",
            FILTER,
        ]
        .join("\n");
        assert_eq!(
            args.trim_end(),
            want,
            "cargo mutants ran other than as the criterion has it"
        );
        let env = std::fs::read_to_string(home.join("cargo-env")).expect("cargo's record");
        assert!(
            env.contains("/work/mutants\n")
                && env.contains("profile mutants\n")
                && env.contains(&format!("head {commit}\n")),
            "cargo mutants ran other than at the commit under the `mutants` profile:\n{env}"
        );
        let calls = std::fs::read_to_string(home.join("contain-calls")).unwrap_or_default();
        assert_eq!(
            calls, "check unprivileged\nrun unprivileged\n",
            "cargo mutants ran other than through contain.sh run, unprivileged, after its check"
        );
        let patch = std::fs::read_to_string(&diff).unwrap_or_default();
        let host = std::fs::read_to_string(results.join("host.txt")).unwrap_or_default();
        let wall = std::fs::read_to_string(results.join("wall.txt")).unwrap_or_default();
        assert!(
            patch.contains("crates/engine/src/lib.rs")
                && host.contains(&format!("commit {commit}\nbase {base}\n"))
                && host.contains("\ncores ")
                && host.contains("\nload at end ")
                && wall.starts_with("wall_seconds ")
                && results.join("mutants.out/outcomes.json").is_file(),
            "inside.sh mutants doesn't keep the diff, the host, the wall time and mutants.out:\n{patch}\n{host}\n{wall}"
        );
        for (exit, want) in [(2, Some(0)), (3, Some(0)), (1, Some(1)), (4, Some(4))] {
            let (code, out) = run(&dir, &good, exit);
            assert_eq!(
                code, want,
                "inside.sh mutants misreads cargo mutants' exit {exit}:\n{out}"
            );
        }
        for timeout in ["1", "9999"] {
            let (code, out) = run(&dir, &spec(&commit, &base, "engine", timeout, FILTER), 0);
            assert_eq!(
                code,
                Some(0),
                "inside.sh mutants refuses a timeout of {timeout} s:\n{out}"
            );
        }
        let short = &commit[..39];
        let refused = [
            (
                "a short commit",
                spec(short, &base, "engine", "300", FILTER),
            ),
            (
                "a short base",
                spec(&commit, &base[..39], "engine", "300", FILTER),
            ),
            (
                "a base that isn't an ancestor",
                spec(&base, &commit, "engine", "300", FILTER),
            ),
            (
                "an empty diff",
                spec(&commit, &commit, "engine", "300", FILTER),
            ),
            ("no package", spec(&commit, &base, "", "300", FILTER)),
            (
                "xtask, the job queue's",
                spec(&commit, &base, "xtask", "300", FILTER),
            ),
            ("gui", spec(&commit, &base, "engine gui", "300", FILTER)),
            (
                "a package that isn't a name",
                spec(&commit, &base, "engine;true", "300", FILTER),
            ),
            (
                "a timeout of 0",
                spec(&commit, &base, "engine", "0", FILTER),
            ),
            (
                "a timeout of 10000",
                spec(&commit, &base, "engine", "10000", FILTER),
            ),
            (
                "a timeout with a unit",
                spec(&commit, &base, "engine", "300s", FILTER),
            ),
            (
                "a filterset with a quote",
                spec(&commit, &base, "engine", "300", "test(=a')"),
            ),
            (
                "a filterset with a substitution",
                spec(&commit, &base, "engine", "300", "$(true)"),
            ),
            (
                "an empty filterset",
                spec(&commit, &base, "engine", "300", ""),
            ),
            ("four lines", format!("{commit}\n{base}\nengine\n300\n")),
        ];
        for (what, workload) in refused {
            std::fs::create_dir_all(&results).expect("scratch");
            std::fs::write(results.join("earlier"), "kept").expect("scratch");
            let (code, out) = run(&dir, &workload, 0);
            assert!(
                code == Some(2)
                    && out.contains("mutants: refused: ")
                    && !home.join("cargo-args").exists(),
                "inside.sh mutants ran a workload it should refuse ({what}): {code:?}\n{out}"
            );
            assert!(
                results.join("earlier").is_file(),
                "inside.sh mutants removed an earlier run's results before refusing ({what}):\n{out}"
            );
        }
    }

    #[test]
    fn wsl_inside_mutants_runs_the_named_workload_alone() {
        check_mutants("wsl_mutants", &read(INSIDE));
    }

    negative_control!(
        wsl_inside_mutants_runs_the_named_workload_alone,
        "a missed mutant failing the run",
        expected = "inside.sh mutants misreads cargo mutants' exit 2",
        check_mutants(
            "wsl_mutants_missed",
            &edited(
                &read(INSIDE),
                "    0 | 2 | 3) return 0 ;;\n",
                "    0) return 0 ;;\n"
            )
        )
    );

    mod timeout_boundary {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "a timeout of 9999 s refused",
            expected = "inside.sh mutants refuses a timeout of 9999 s",
            check_mutants(
                "wsl_mutants_9999",
                &edited(
                    &read(INSIDE),
                    "secs='^[1-9][0-9]{0,3}$'",
                    "secs='^[1-9][0-9]{0,2}$'"
                )
            )
        );
    }

    mod not_a_crate {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "a package outside crates/ let through",
            expected = "inside.sh mutants ran a workload it should refuse (xtask, the job queue's)",
            check_mutants(
                "wsl_mutants_xtask",
                &edited(
                    &read(INSIDE),
                    "    [ -f \"$dir/crates/$p/Cargo.toml\" ] ||",
                    "    true ||"
                )
            )
        );
    }

    mod any_filterset {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "any filterset let through",
            expected =
                "inside.sh mutants ran a workload it should refuse (a filterset with a quote)",
            check_mutants(
                "wsl_mutants_filter",
                &edited(
                    &read(INSIDE),
                    "set='^[A-Za-z0-9_:=()|&!/.~ -]+$'",
                    "set='^.+$'"
                )
            )
        );
    }

    mod uncontained {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "cargo mutants run outside contain.sh",
            expected = "cargo mutants ran other than through contain.sh run",
            check_mutants(
                "wsl_mutants_uncontained",
                &edited(
                    &read(INSIDE),
                    "NEXTEST_PROFILE=mutants bash \"$repo/scripts/contain.sh\" run cargo mutants",
                    "NEXTEST_PROFILE=mutants cargo mutants"
                )
            )
        );
    }

    mod no_timeout {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "the timeout not passed",
            expected = "cargo mutants ran other than as the criterion has it",
            check_mutants(
                "wsl_mutants_no_timeout",
                &edited(&read(INSIDE), " --timeout \"$timeout\"", "")
            )
        );
    }

    mod results_cleared_early {
        use super::*;
        negative_control!(
            wsl_inside_mutants_runs_the_named_workload_alone,
            "an earlier run's results removed before the checks that read the clone",
            expected = "inside.sh mutants removed an earlier run's results before refusing (a base that isn't an ancestor)",
            check_mutants(
                "wsl_mutants_cleared_early",
                &edited(
                    &read(INSIDE),
                    "  # Nothing is fetched:",
                    "  rm -rf \"$results\"\n  # Nothing is fetched:"
                )
            )
        );
    }

    // ---- R-1017: the jobs, the shard and the warm build cache -------------------------------------------------------

    /// The per-process cap a job's memory is counted in, 8 GiB in kB (`.cargo/mutants.toml`'s `prlimit --as`).
    const JOB_KIB: u64 = 8_388_608;

    /// `inside` reading the distro's memory from `$HOME/meminfo`, which the test writes, not /proc/meminfo.
    fn seamed(inside: &str) -> String {
        assert!(
            inside.contains("\nmeminfo=/proc/meminfo\n"),
            "inside.sh reads the memory from somewhere else"
        );
        inside.replacen(
            "\nmeminfo=/proc/meminfo\n",
            "\nmeminfo=\"$HOME/meminfo\"\n",
            1,
        )
    }

    /// The distro's memory, `kib` kB, or none to read.
    fn memory(dir: &Scratch, kib: Option<u64>) {
        let file = dir.join("home/meminfo");
        match kib {
            Some(k) => std::fs::write(&file, format!("MemFree:  1 kB\nMemTotal:       {k} kB\n"))
                .expect("scratch"),
            None => {
                let _ = std::fs::remove_file(&file);
            }
        }
    }

    /// `cargo mutants`' arguments for the commit's diff of engine at the timeout of 300 s, with `jobs` and `shard`.
    fn tuned(dir: &Scratch, jobs: &str, shard: Option<&str>) -> String {
        let results = dir.join("home/results");
        let mut want = vec![
            "mutants".to_owned(),
            "--in-diff".to_owned(),
            results.join("diff.patch").display().to_string(),
            "--package".to_owned(),
            "engine".to_owned(),
            "-j".to_owned(),
            jobs.to_owned(),
        ];
        if let Some(s) = shard {
            want.extend(["--shard".to_owned(), s.to_owned()]);
        }
        want.extend(
            ["--timeout", "300", "-o"]
                .iter()
                .map(|s| (*s).to_owned())
                .chain([results.display().to_string()])
                .chain(["--", "-E", FILTER].iter().map(|s| (*s).to_owned())),
        );
        want.join("\n")
    }

    /// Two lines may follow the five, each empty or absent for its default: cargo mutants' jobs, 1 to 4 (1), and its
    /// shard, k/n with k from 0 to n-1 and n from 1 to 99 (none), reaching cargo as `-j <jobs>` and `--shard <k/n>`
    /// and recorded in host.txt. More than one job runs only where the distro's memory is at least the allowance of 8 GiB a job, exactly that
    /// much included; its memory unreadable, or 1 kB short, it is refused. Each malformed value, and each bound's
    /// first value past it, is refused with nothing run.
    fn check_tuning(tag: &str, inside: &str) {
        let (dir, base, commit) = repository(tag, &seamed(inside));
        let home = dir.join("home");
        let five = spec(&commit, &base, "engine", "300", FILTER);
        let ran = [
            (
                "two jobs and a shard, the memory exactly two jobs'",
                "2\n1/3\n",
                Some(2 * JOB_KIB),
                "2",
                Some("1/3"),
            ),
            (
                "four jobs, the memory exactly four jobs'",
                "4\n\n",
                Some(4 * JOB_KIB),
                "4",
                None,
            ),
            ("one job named, the memory unread", "1\n\n", None, "1", None),
            ("an empty jobs line", "\n", None, "1", None),
            (
                "three jobs, the shard's line absent",
                "3\n",
                Some(3 * JOB_KIB + 1),
                "3",
                None,
            ),
            ("the first of one shard", "\n0/1\n", None, "1", Some("0/1")),
            (
                "the last of 99 shards",
                "\n98/99\n",
                None,
                "1",
                Some("98/99"),
            ),
        ];
        for (what, extra, kib, jobs, shard) in ran {
            memory(&dir, kib);
            let (code, out) = run(&dir, &format!("{five}{extra}"), 0);
            assert_eq!(code, Some(0), "inside.sh mutants refused {what}:\n{out}");
            let args = std::fs::read_to_string(home.join("cargo-args")).unwrap_or_default();
            assert_eq!(
                args.trim_end(),
                tuned(&dir, jobs, shard),
                "cargo mutants ran other than with the jobs and shard asked ({what})"
            );
            let host = std::fs::read_to_string(home.join("results/host.txt")).unwrap_or_default();
            assert!(
                host.contains(&format!(
                    "\njobs {jobs}\nshard {}\n",
                    shard.unwrap_or("none")
                )),
                "host.txt doesn't record the jobs and the shard ({what}):\n{host}"
            );
        }
        let refused = [
            ("no job", "0\n", Some(64 * JOB_KIB)),
            ("five jobs", "5\n", Some(64 * JOB_KIB)),
            ("a job count with a letter", "2a\n", Some(64 * JOB_KIB)),
            (
                "two jobs, the memory 1 kB short",
                "2\n",
                Some(2 * JOB_KIB - 1),
            ),
            ("two jobs, the memory unreadable", "2\n", None),
            ("the shard n of n", "\n3/3\n", None),
            ("a shard of none", "\n0/0\n", None),
            ("100 shards", "\n1/100\n", None),
            ("a negative shard", "\n-1/3\n", None),
            ("a shard of three parts", "\n1/3/4\n", None),
            ("a shard with a leading zero", "\n01/3\n", None),
            ("a shard with a letter", "\na/3\n", None),
            ("a shard with a space", "\n1 /3\n", None),
        ];
        for (what, extra, kib) in refused {
            memory(&dir, kib);
            let (code, out) = run(&dir, &format!("{five}{extra}"), 0);
            assert!(
                code == Some(2)
                    && out.contains("mutants: refused: ")
                    && !home.join("cargo-args").exists(),
                "inside.sh mutants ran a workload it should refuse ({what}): {code:?}\n{out}"
            );
        }
    }

    #[test]
    fn wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds() {
        check_tuning("wsl_tuning", &read(INSIDE));
    }

    negative_control!(
        wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
        "the jobs asked not passed",
        expected =
            "cargo mutants ran other than with the jobs and shard asked (two jobs and a shard",
        check_tuning(
            "wsl_tuning_jobs",
            &edited(&read(INSIDE), " -j \"$jobs\" ", " -j 1 ")
        )
    );

    mod tuning_shard_n_of_n {
        use super::*;
        negative_control!(
            wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
            "a shard k of n let through at k = n",
            expected = "inside.sh mutants ran a workload it should refuse (the shard n of n)",
            check_tuning(
                "wsl_tuning_n_of_n",
                &edited(
                    &read(INSIDE),
                    "[ \"${BASH_REMATCH[1]}\" -lt \"${BASH_REMATCH[2]}\" ]",
                    "[ \"${BASH_REMATCH[1]}\" -le \"${BASH_REMATCH[2]}\" ]"
                )
            )
        );
    }

    mod tuning_hundred_shards {
        use super::*;
        negative_control!(
            wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
            "any number of shards let through",
            expected = "inside.sh mutants ran a workload it should refuse (100 shards)",
            check_tuning(
                "wsl_tuning_hundred",
                &edited(
                    &read(INSIDE),
                    "part='^(0|[1-9][0-9]?)/([1-9][0-9]?)$'",
                    "part='^(0|[1-9][0-9]*)/([1-9][0-9]*)$'"
                )
            )
        );
    }

    mod tuning_five_jobs {
        use super::*;
        negative_control!(
            wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
            "five jobs let through",
            expected = "inside.sh mutants ran a workload it should refuse (five jobs)",
            check_tuning(
                "wsl_tuning_five",
                &edited(&read(INSIDE), "ways='^[1-4]$'", "ways='^[1-5]$'")
            )
        );
    }

    mod tuning_memory_unchecked {
        use super::*;
        negative_control!(
            wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
            "the memory not checked for more than one job",
            expected = "inside.sh mutants ran a workload it should refuse (two jobs, the memory 1 kB short)",
            check_tuning(
                "wsl_tuning_memory",
                &edited(
                    &read(INSIDE),
                    "  if [ \"$jobs\" -gt 1 ]; then\n    mem=",
                    "  if false; then\n    mem="
                )
            )
        );
    }

    mod tuning_memory_boundary {
        use super::*;
        negative_control!(
            wsl_inside_mutants_takes_jobs_and_a_shard_within_bounds,
            "the memory of exactly the jobs' caps refused",
            expected =
                "inside.sh mutants refused two jobs and a shard, the memory exactly two jobs'",
            check_tuning(
                "wsl_tuning_memory_boundary",
                &edited(
                    &read(INSIDE),
                    "[ \"$mem\" -ge $((jobs * job_kib)) ]",
                    "[ \"$mem\" -gt $((jobs * job_kib)) ]"
                )
            )
        );
    }

    /// The warm build cache, ~/target: keyed on the commit's toolchain and lockfile as git holds them (their blobs),
    /// the key kept in it; a cache missing is made; a cache of that key and within its bound is kept, at its bound
    /// too; one of another key, of none, or past its bound by 1 kB is cleared before the run. At -j 1 cargo mutants
    /// builds in it; at -j 2 its copies of the tree don't share it, nor any target (CARGO_TARGET_DIR unset), and it is
    /// left as it is, key and all, even one of another key, for the next -j 1 run. host.txt records its state. And
    /// cargo mutants' copies go in the verb's own temporary folder, emptied before and after each run.
    fn check_cache(tag: &str, inside: &str) {
        let inside = seamed(inside);
        let (dir, base, commit) = repository(tag, &inside);
        let home = dir.join("home");
        let repo = dir.join("repo");
        let cache = home.join("target");
        let key_file = cache.join(".prin-cache-key");
        let marker = cache.join("debug-marker");
        let blob = |path: &str| {
            let out = Command::new(home.join(".cargo/bin/git"))
                .arg("-C")
                .arg(&repo)
                .args(["rev-parse", &format!("{commit}:{path}")])
                .env("HOME", &home)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output_within(Duration::from_secs(60))
                .expect("git ran");
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        };
        let key = format!(
            "toolchain {} lockfile {}",
            blob("rust-toolchain.toml"),
            blob("Cargo.lock")
        );
        assert_eq!(
            key.len(),
            "toolchain  lockfile ".len() + 80,
            "the scratch repository's blobs: {key}"
        );
        memory(&dir, Some(64 * JOB_KIB));
        let one = spec(&commit, &base, "engine", "300", FILTER);
        let two = format!("{one}2\n");
        let state = || {
            let host = std::fs::read_to_string(home.join("results/host.txt")).unwrap_or_default();
            host.lines()
                .find_map(|l| l.strip_prefix("cache ").filter(|r| !r.starts_with("key ")))
                .unwrap_or("")
                .to_owned()
        };
        let target = || {
            std::fs::read_to_string(home.join("cargo-env"))
                .unwrap_or_default()
                .lines()
                .find_map(|l| l.strip_prefix("target ").map(str::to_owned))
                .unwrap_or_default()
        };
        let plant = |key: Option<&str>| {
            std::fs::create_dir_all(&cache).expect("scratch");
            std::fs::write(&marker, "built").expect("scratch");
            match key {
                Some(k) => std::fs::write(&key_file, format!("{k}\n")).expect("scratch"),
                None => {
                    let _ = std::fs::remove_file(&key_file);
                }
            }
        };
        let _ = std::fs::remove_dir_all(&cache);
        // A copy an earlier run, stopped by a signal, left in the verb's temporary folder.
        let stale = home.join("work/tmp/cargo-mutants-principia-left.tmp");
        std::fs::create_dir_all(stale.join("target")).expect("scratch");
        let (code, out) = run(&dir, &one, 0);
        let env = std::fs::read_to_string(home.join("cargo-env")).unwrap_or_default();
        assert!(
            !stale.exists()
                && !home.join("work/tmp").exists()
                && env.contains(&format!("\ntmpdir {}\n", home.join("work/tmp").display())),
            "cargo mutants' copies went elsewhere than the verb's own temporary folder, emptied before and after: {code:?}\n{env}\n{out}"
        );
        assert!(
            code == Some(0) && state().starts_with("miss"),
            "inside.sh mutants didn't make a cache where there was none: {code:?} {}\n{out}",
            state()
        );
        assert_eq!(
            std::fs::read_to_string(&key_file)
                .unwrap_or_default()
                .trim_end(),
            key,
            "the cache's key isn't the commit's toolchain and lockfile"
        );
        assert_eq!(
            target(),
            cache.display().to_string(),
            "at -j 1 cargo mutants didn't build in the cache"
        );
        plant(Some(&key));
        let (_, out) = run(&dir, &one, 0);
        assert!(
            marker.is_file() && state().starts_with("hit"),
            "inside.sh mutants cleared a cache of the commit's key: {}\n{out}",
            state()
        );
        let (code, out) = run(&dir, &two, 0);
        assert!(
            code == Some(0) && target() == "unset",
            "at -j 2 cargo mutants' copies shared the cache, or a target: {code:?} {}\n{out}",
            target()
        );
        plant(Some("toolchain 0 lockfile 0"));
        let (_, out) = run(&dir, &two, 0);
        assert!(
            marker.is_file()
                && std::fs::read_to_string(&key_file).unwrap_or_default()
                    == "toolchain 0 lockfile 0\n"
                && state().starts_with("unused at -j 2"),
            "a -j 2 run, which doesn't use the cache, cleared it or rekeyed it: {}\n{out}",
            state()
        );
        for (what, planted) in [
            ("another key", Some("toolchain 0 lockfile 0")),
            ("no key", None),
        ] {
            plant(planted);
            let (_, out) = run(&dir, &one, 0);
            assert!(
                !marker.exists() && state().starts_with("cleared: its key was"),
                "inside.sh mutants kept a cache of {what}: {}\n{out}",
                state()
            );
            assert_eq!(
                std::fs::read_to_string(&key_file)
                    .unwrap_or_default()
                    .trim_end(),
                key,
                "the cache's key isn't the commit's toolchain and lockfile"
            );
        }
        // Its bound: at exactly its size the cache stays; 1 kB under, it goes.
        for (slack, kept) in [(0_i64, true), (-1, false)] {
            plant(Some(&key));
            let du = Command::new("du")
                .arg("-sk")
                .arg(&cache)
                .output_within(Duration::from_secs(60))
                .expect("du ran");
            let size: i64 = String::from_utf8_lossy(&du.stdout)
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .expect("du's size");
            let bounded = inside.replacen(
                "\ncache_cap_kib=25165824\n",
                &format!("\ncache_cap_kib={}\n", size + slack),
                1,
            );
            assert_ne!(bounded, inside, "inside.sh's bound is another");
            std::fs::write(repo.join(INSIDE), &bounded).expect("scratch");
            let (_, out) = run(&dir, &one, 0);
            assert_eq!(
                marker.is_file(),
                kept,
                "inside.sh mutants misjudged a cache of {size} kB against a bound of {} kB: {}\n{out}",
                size + slack,
                state()
            );
        }
        std::fs::write(repo.join(INSIDE), &inside).expect("scratch");
    }

    #[test]
    fn wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job() {
        check_cache("wsl_cache", &read(INSIDE));
    }

    negative_control!(
        wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
        "the copies of -j 2 sharing the cache",
        expected = "at -j 2 cargo mutants' copies shared the cache, or a target",
        check_cache(
            "wsl_cache_shared",
            &edited(
                &read(INSIDE),
                "    unset CARGO_TARGET_DIR\n",
                "    export CARGO_TARGET_DIR=\"$cache\"\n"
            )
        )
    );

    mod cache_cleared_at_two_jobs {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "a -j 2 run clearing the cache it doesn't use",
            expected = "a -j 2 run, which doesn't use the cache, cleared it or rekeyed it",
            check_cache(
                "wsl_cache_two_jobs",
                &edited(
                    &read(INSIDE),
                    "    state=\"unused at -j $jobs",
                    "    rm -rf \"$cache\"\n    state=\"unused at -j $jobs"
                )
            )
        );
    }

    mod cache_tmp_kept {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "an earlier run's copies left in the temporary folder",
            expected = "cargo mutants' copies went elsewhere than the verb's own temporary folder",
            check_cache(
                "wsl_cache_tmp",
                &edited(
                    &edited(
                        &read(INSIDE),
                        "  rm -rf \"$work/tmp\"\n  mkdir -p \"$work/tmp\"\n",
                        "  mkdir -p \"$work/tmp\"\n"
                    ),
                    "  } >> \"$results/host.txt\"\n  rm -rf \"$work/tmp\"\n",
                    "  } >> \"$results/host.txt\"\n"
                )
            )
        );
    }

    mod cache_tmp_elsewhere {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "the copies in the system's temporary folder",
            expected = "cargo mutants' copies went elsewhere than the verb's own temporary folder",
            check_cache(
                "wsl_cache_tmp_elsewhere",
                &edited(&read(INSIDE), "  export TMPDIR=\"$work/tmp\"\n", "")
            )
        );
    }

    mod cache_key_ignored {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "a cache of another key kept",
            expected = "inside.sh mutants kept a cache of another key",
            check_cache(
                "wsl_cache_key",
                &edited(
                    &read(INSIDE),
                    "  elif [ \"$have\" != \"$key\" ]; then",
                    "  elif false; then"
                )
            )
        );
    }

    mod cache_bound {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "a cache at its bound cleared",
            expected = "inside.sh mutants misjudged a cache of",
            check_cache(
                "wsl_cache_bound",
                &edited(
                    &read(INSIDE),
                    "[ \"${size:-0}\" -gt \"$cache_cap_kib\" ]",
                    "[ \"${size:-0}\" -ge \"$cache_cap_kib\" ]"
                )
            )
        );
    }

    mod cache_key_unwritten {
        use super::*;
        negative_control!(
            wsl_inside_mutants_keeps_a_keyed_bounded_cache_for_one_job,
            "the key never written",
            expected = "the cache's key isn't the commit's toolchain and lockfile",
            check_cache(
                "wsl_cache_unwritten",
                &edited(
                    &read(INSIDE),
                    "  echo \"$key\" > \"$cache/.prin-cache-key\"\n",
                    "  true\n"
                )
            )
        );
    }
}

/// `inside.sh trust`, run in a clone of a scratch origin, as `wsl.ps1`'s Initialize-Clone makes one, with commits signed
/// by throwaway keys: `owner`, pinned in the clone's `.github/allowed_signers`, and `other`, not.
#[cfg(unix)]
mod trust {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use validation::spawn::Spawn;

    use crate::scratch::Scratch;

    /// The first program named `name` on this test's PATH.
    fn which(name: &str) -> PathBuf {
        let path = std::env::var_os("PATH").expect("PATH is set");
        std::env::split_paths(&path)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no {name} on PATH"))
    }

    /// The commits of the scratch origin each case names.
    struct Origin {
        dir: Scratch,
        clone: PathBuf,
        /// main's first commit, every case's base but one.
        base: String,
        /// Two commits signed with `owner`'s key on a branch, its head.
        good: String,
        /// That branch with main merged in, the merge signed with `owner`'s key, main's own commit unsigned.
        merged: String,
        /// A commit signed with `owner`'s key, then an unsigned one on top, on a branch.
        unsigned: String,
        /// A commit signed with `other`'s key, on a branch.
        other: String,
        /// A branch off a commit main doesn't hold: that commit, and the head, both signed with `owner`'s key.
        off_main_base: String,
        off_main: String,
        /// A commit signed with `owner`'s key under `refs/pull/1/head` alone, fetched into the clone by id.
        pull: String,
    }

    fn git(dir: &Path, home: &Path, args: &[&str]) -> String {
        let out = Command::new(home.join(".cargo/bin/git"))
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(["-c", "commit.gpgsign=false", "-c", "gpg.format=ssh"])
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("HOME", home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("XDG_CONFIG_HOME")
            .output_within(Duration::from_secs(60))
            .expect("git ran");
        assert!(out.status.success(), "scratch git {args:?}: {out:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    fn origin(tag: &str, inside: &str) -> Origin {
        let dir = Scratch::new(tag);
        let home = dir.join("home");
        let bin = home.join(".cargo/bin");
        let keys = dir.join("keys");
        let origin = dir.join("origin");
        for d in [&bin, &keys, &origin] {
            std::fs::create_dir_all(d).expect("scratch");
        }
        // inside.sh's PATH starts at ~/.cargo/bin, so the git and ssh-keygen this test found are the ones it runs.
        for tool in ["git", "ssh-keygen"] {
            std::os::unix::fs::symlink(which(tool), bin.join(tool)).expect("scratch");
        }
        for key in ["owner", "other"] {
            let out = Command::new(bin.join("ssh-keygen"))
                .args(["-q", "-t", "ed25519", "-N", "", "-C", key, "-f"])
                .arg(keys.join(key))
                .output_within(Duration::from_secs(60))
                .expect("ssh-keygen ran");
            assert!(out.status.success(), "ssh-keygen: {out:?}");
        }
        let key = |name: &str| keys.join(name).display().to_string();
        let g = |args: &[&str]| git(&origin, &home, args);
        // A commit changing `file`, signed with `key`'s key, or unsigned with no key.
        let commit = |key_name: Option<&str>, text: &str| {
            std::fs::write(origin.join("file"), text).expect("scratch");
            g(&["add", "file"]);
            match key_name {
                Some(k) => {
                    let signing = format!("user.signingkey={}", key(k));
                    g(&["-c", &signing, "commit", "-q", "-S", "-m", text])
                }
                None => g(&["commit", "-q", "-m", text]),
            };
            g(&["rev-parse", "HEAD"])
        };
        g(&["init", "-q", "-b", "main"]);
        let base = commit(None, "base");
        g(&["checkout", "-q", "-b", "task/good"]);
        commit(Some("owner"), "good 1");
        let good = commit(Some("owner"), "good 2");
        g(&["checkout", "-q", "main"]);
        let main_after = commit(None, "main after base, unsigned");
        g(&["checkout", "-q", "-b", "task/merged", &good]);
        let signing = format!("user.signingkey={}", key("owner"));
        g(&[
            "-c",
            &signing,
            "merge",
            "-q",
            "-S",
            "--no-edit",
            "-X",
            "theirs",
            &main_after,
        ]);
        let merged = g(&["rev-parse", "HEAD"]);
        g(&["checkout", "-q", "-b", "task/unsigned", &base]);
        commit(Some("owner"), "signed, then");
        let unsigned = commit(None, "unsigned");
        g(&["checkout", "-q", "-b", "task/other", &base]);
        let other = commit(Some("other"), "another key");
        g(&["checkout", "-q", "-b", "side", &base]);
        let off_main_base = commit(Some("owner"), "off main");
        let off_main = commit(Some("owner"), "on top of off main");
        g(&["checkout", "-q", "--detach", &base]);
        let pull = commit(Some("owner"), "a pull request's head");
        g(&["update-ref", "refs/pull/1/head", &pull]);
        g(&["checkout", "-q", "main"]);
        // The clone, as Initialize-Clone makes it: origin's branches alone, under refs/remotes/origin. The old
        // `fetch origin <id>` then brings the pull request's head into its objects, under no branch.
        let clone = home.join("work/principia");
        std::fs::create_dir_all(home.join("work")).expect("scratch");
        g(&[
            "clone",
            "-q",
            &origin.display().to_string(),
            &clone.display().to_string(),
        ]);
        git(
            &clone,
            &home,
            &["fetch", "-q", "origin", "refs/pull/1/head"],
        );
        std::fs::create_dir_all(clone.join("scripts/wsl")).expect("scratch");
        std::fs::create_dir_all(clone.join(".github")).expect("scratch");
        std::fs::write(clone.join(INSIDE), inside).expect("scratch");
        let public = std::fs::read_to_string(keys.join("owner.pub")).expect("owner.pub");
        let pinned: Vec<&str> = public.split_whitespace().take(2).collect();
        std::fs::write(
            clone.join(SIGNERS),
            format!(
                "# the test's owner\nowner namespaces=\"git\" {}\n",
                pinned.join(" ")
            ),
        )
        .expect("scratch");
        Origin {
            dir,
            clone,
            base,
            good,
            merged,
            unsigned,
            other,
            off_main_base,
            off_main,
            pull,
        }
    }

    /// `inside.sh trust` with the commit and base as the first two of the workload's five lines.
    fn run(o: &Origin, commit: &str, base: &str) -> (Option<i32>, String) {
        let home = o.dir.join("home");
        let stdin = o.dir.join("stdin");
        std::fs::write(
            &stdin,
            format!("{commit}\n{base}\nengine\n300\nbinary_id(=engine::x)\n"),
        )
        .expect("scratch");
        let out = Command::new("bash")
            .args(["-c", "exec bash \"$1\" trust < \"$2\"", "inside"])
            .arg(o.clone.join(INSIDE))
            .arg(&stdin)
            .env("HOME", &home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("XDG_CONFIG_HOME")
            .output_within(Duration::from_secs(120))
            .expect("bash ran");
        (
            out.status.code(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    /// The owner's signed commits pass, those of a branch that merged main too (main's own commits being the chain
    /// the PC's gate already trusts); an unsigned commit, one signed with another key, a base off main and a commit
    /// under `refs/pull` alone are each refused, exit 2 and named.
    fn check_trust(tag: &str, inside: &str) {
        let o = origin(tag, inside);
        for (what, commit) in [
            ("the owner's signed commits", &o.good),
            ("the owner's merge of main", &o.merged),
        ] {
            let (code, out) = run(&o, commit, &o.base);
            assert!(
                code == Some(0) && out.contains("trust: "),
                "inside.sh trust refused {what}: {code:?}\n{out}"
            );
        }
        let refused = [
            ("an unsigned commit", &o.unsigned, &o.base),
            ("a commit signed with another key", &o.other, &o.base),
            ("a base off main", &o.off_main, &o.off_main_base),
            ("a commit under refs/pull alone", &o.pull, &o.base),
        ];
        for (what, commit, base) in refused {
            let (code, out) = run(&o, commit, base);
            assert!(
                code == Some(2) && out.contains("mutants: refused: "),
                "inside.sh trust accepted {what}: {code:?}\n{out}"
            );
        }
    }

    #[test]
    fn wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches() {
        check_trust("wsl_trust", &read(INSIDE));
    }

    negative_control!(
        wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches,
        "the signatures not checked",
        expected = "inside.sh trust accepted an unsigned commit",
        check_trust(
            "wsl_trust_unchecked",
            &edited(
                &read(INSIDE),
                "    \"${verify[@]}\" verify-commit \"$c\" ||",
                "    true ||"
            )
        )
    );

    mod any_key {
        use super::*;
        negative_control!(
            wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches,
            "any signature taken, whatever its key",
            expected = "inside.sh trust accepted a commit signed with another key",
            check_trust(
                "wsl_trust_any_key",
                &edited(
                    &read(INSIDE),
                    "    \"${verify[@]}\" verify-commit \"$c\" ||",
                    "    git -C \"$repo\" cat-file commit \"$c\" | grep '^gpgsig ' > /dev/null ||"
                )
            )
        );
    }

    mod base_anywhere {
        use super::*;
        negative_control!(
            wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches,
            "a base off main taken",
            expected = "inside.sh trust accepted a base off main",
            check_trust(
                "wsl_trust_base_anywhere",
                &edited(&read(INSIDE), "  on_main \"$base\" ||", "  true ||")
            )
        );
    }

    mod any_object {
        use super::*;
        negative_control!(
            wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches,
            "a commit taken that no branch holds",
            expected = "inside.sh trust accepted a commit under refs/pull alone",
            check_trust(
                "wsl_trust_any_object",
                &edited(&read(INSIDE), "  on_branch \"$commit\" ||", "  true ||")
            )
        );
    }

    mod main_not_trusted {
        use super::*;
        negative_control!(
            wsl_inside_trust_accepts_only_the_owners_signed_commits_on_origins_branches,
            "main's own commits checked as the branch's",
            expected = "inside.sh trust refused the owner's merge of main",
            check_trust(
                "wsl_trust_main_checked",
                &edited(
                    &read(INSIDE),
                    "rev-list \"$commit\" --not \"$base\" refs/remotes/origin/main)",
                    "rev-list \"$commit\" --not \"$base\")"
                )
            )
        );
    }
}

/// The key `.github/allowed_signers` pins: the owner's, the one GitHub lists as Ma1achy's signing key ("Principia Mac
/// signing", id 1305828), principal and namespace as `inside.sh trust` reads them, and that one line alone.
fn check_signers(text: &str) {
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    assert_eq!(
        lines,
        [OWNER_KEY],
        "{SIGNERS} doesn't pin the owner's signing key alone"
    );
}

/// The owner's public signing key, as `.github/allowed_signers` holds it.
const OWNER_KEY: &str = "principia-commit-signing namespaces=\"git\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMj+yb+Tf/QlClWSAZSMmHJu53+1E9Q+IU+Rvr9L46Er";

#[test]
fn wsl_trust_pins_the_owners_signing_key() {
    check_signers(&read(SIGNERS));
}

negative_control!(
    wsl_trust_pins_the_owners_signing_key,
    "a second key added",
    expected = "doesn't pin the owner's signing key alone",
    check_signers(&format!("{}other ssh-ed25519 AAAA\n", read(SIGNERS)))
);

// ---- The image ----------------------------------------------------------------------------------------------------

/// The purge, forced past sudo's pre-removal guard, which refuses while root's password is locked as the base's is.
const PURGE: &str = "in_root env SUDO_FORCE_REMOVE=yes apt-get -q -y purge sudo\n";
/// What the build runs in the chroot straight after the purge, and fails on: dpkg holds no sudo, not even its
/// conffiles, and no sudo is on PATH, at /usr/bin/sudo or as /etc/sudoers.
const SUDO_GONE: &str = r#"in_root sh -eu -s << 'EOF' || {
dpkg-query --version > /dev/null
state=$(dpkg-query -W -f '${db:Status-Status}' sudo 2> /dev/null || true)
case "$state" in
  "" | not-installed) ;;
  *) echo "dpkg still holds sudo: $state" && exit 1 ;;
esac
if command -v sudo; then echo "a sudo is on PATH" && exit 1; fi
for f in /usr/bin/sudo /etc/sudoers; do
  if [ -e "$f" ] || [ -L "$f" ]; then echo "$f is left" && exit 1; fi
done
echo "sudo: purged; no package, no binary, no /etc/sudoers"
EOF
  echo "::error::build-image.sh: sudo isn't proved gone from the image after the purge"
  exit 1
}
"#;

/// The value of `NAME=...` in the build script.
fn setting<'a>(build: &'a str, name: &str) -> &'a str {
    build
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{name}=")))
        .unwrap_or_else(|| panic!("{BUILD} sets no {name}"))
}

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn check_image(build: &str, conf: &str, ci: &str) {
    assert!(
        !conf.contains("systemd="),
        "{CONF} sets `systemd=`: systemd switched on"
    );
    assert_eq!(
        conf, OWNERS_CONF,
        "{CONF} isn't exactly the owner's wsl.conf with R-1010's two additions"
    );
    assert!(
        build.contains("sudo install -m 0644 -o 0 -g 0 \"$here/wsl.conf\" \"$root/etc/wsl.conf\""),
        "the image's /etc/wsl.conf isn't {CONF}"
    );
    for word in [
        "[boot]",
        "systemd=",
        "sudoers.d/",
        "usermod",
        "--groups",
        "-G sudo",
        "NOPASSWD",
    ] {
        assert!(
            !build.contains(word),
            "{BUILD} writes `{word}`: systemd or a sudo entry"
        );
    }
    assert!(
        build.contains(PURGE) && build.contains("sudo rm -rf \"$root/etc/sudoers.d\""),
        "the image keeps sudo"
    );
    let forced: Vec<&str> = build
        .lines()
        .filter(|l| l.contains("SUDO_FORCE_REMOVE") && !l.starts_with('#'))
        .collect();
    assert_eq!(
        forced,
        [PURGE.trim_end()],
        "SUDO_FORCE_REMOVE is set beyond the purge's own command"
    );
    let purge = build.find(PURGE).expect("checked above");
    let gone = build
        .find(SUDO_GONE)
        .expect("the build doesn't prove sudo gone, or doesn't fail on it");
    assert!(
        purge < gone
            && build[purge + PURGE.len()..gone]
                .lines()
                .all(|l| l.starts_with('#')),
        "the proof that sudo is gone doesn't follow the purge"
    );
    let url = setting(build, "BASE_URL");
    assert!(
        url.starts_with("https://") && url.contains("24.04"),
        "the base isn't Ubuntu 24.04 over https: {url}"
    );
    let old = setting(build, "BASE_OLD_URL");
    assert!(
        old.starts_with("https://old-releases.ubuntu.com/releases/noble/")
            && url.rsplit('/').next() == old.rsplit('/').next(),
        "the base's fallback isn't the same image on old-releases.ubuntu.com: {old}"
    );
    for name in ["BASE_SHA256", "RUSTUP_SHA256"] {
        assert!(hex64(setting(build, name)), "{name} isn't a SHA-256");
    }
    let verify = build
        .find("echo \"$BASE_SHA256  $work/base.tar.gz\" | sha256sum -c -")
        .expect("the base isn't verified");
    let unpack = build
        .find("sudo tar -xzpf \"$work/base.tar.gz\"")
        .expect("no unpack");
    assert!(verify < unpack, "the base is unpacked before it's verified");
    let snapshot = setting(build, "SNAPSHOT");
    assert!(
        snapshot.len() == 16
            && snapshot.ends_with('Z')
            && build.contains("URIs: https://snapshot.ubuntu.com/ubuntu/$SNAPSHOT"),
        "packages don't come from one dated snapshot: {snapshot}"
    );
    assert!(
        build.contains("tomllib.load(open(sys.argv[1], \"rb\"))[\"toolchain\"]")
            && build.contains("\"$repo/rust-toolchain.toml\""),
        "the toolchain isn't rust-toolchain.toml's"
    );
    let nextest = format!("tool: cargo-nextest@{}", setting(build, "NEXTEST_VERSION"));
    assert!(
        ci.contains(&nextest),
        "the image's nextest isn't ci.yml's ({nextest})"
    );
}

#[test]
fn wsl_image_is_pinned_and_has_the_owners_config() {
    check_image(&read(BUILD), &read(CONF), &read(".github/workflows/ci.yml"));
}

negative_control!(
    wsl_image_is_pinned_and_has_the_owners_config,
    "systemd switched on",
    expected = "scripts/wsl/wsl.conf sets `systemd=`",
    check_image(
        &read(BUILD),
        &format!("{}\n[boot]\nsystemd=true\n", read(CONF)),
        &read(".github/workflows/ci.yml")
    )
);

/// `check_image` on the build and ci.yml as they are, and `conf` for the distro's wsl.conf.
#[cfg(feature = "controls")]
fn check_conf(conf: &str) {
    check_image(&read(BUILD), conf, &read(".github/workflows/ci.yml"));
}

/// `check_image` on wsl.conf and ci.yml as they are, and the build with `line` added after its wsl.conf install.
#[cfg(feature = "controls")]
fn check_build_writing(line: &str) {
    let install = "sudo install -m 0644 -o 0 -g 0 \"$here/wsl.conf\" \"$root/etc/wsl.conf\"\n";
    check_image(
        &edited(&read(BUILD), install, &format!("{install}{line}\n")),
        &read(CONF),
        &read(".github/workflows/ci.yml"),
    );
}

/// The `[boot]` line R-1010 allows, the one command in its section.
#[cfg(feature = "controls")]
const BOOT_COMMAND: &str = "command=echo -1 > /proc/sys/fs/binfmt_misc/WSLInterop\n";

mod systemd_in_boot {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "systemd= a second line of the [boot] section",
        expected = "scripts/wsl/wsl.conf sets `systemd=`",
        check_conf(&edited(
            &read(CONF),
            BOOT_COMMAND,
            &format!("{BOOT_COMMAND}systemd=true\n")
        ))
    );
}

mod systemd_elsewhere {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "systemd= off, in another section",
        expected = "scripts/wsl/wsl.conf sets `systemd=`",
        check_conf(&edited(
            &read(CONF),
            "default=ci\n",
            "default=ci\nsystemd=false\n"
        ))
    );
}

mod boot_other_command {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "another command at boot",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(
            &read(CONF),
            BOOT_COMMAND,
            "command=mount -t drvfs C: /mnt/c\n"
        ))
    );
}

mod boot_second_line {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a second line in the [boot] section",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(
            &read(CONF),
            BOOT_COMMAND,
            &format!("{BOOT_COMMAND}protectBinfmt=false\n")
        ))
    );
}

mod gpu_missing {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the GPU share left on",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(&read(CONF), "\n[gpu]\nenabled=false\n", ""))
    );
}

mod boot_missing {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the WSLInterop registration left in place",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(
            &read(CONF),
            &format!("\n[boot]\n{BOOT_COMMAND}"),
            ""
        ))
    );
}

mod owners_line_changed {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the owner's Windows PATH line changed",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(
            &read(CONF),
            "appendWindowsPath=false\n",
            "appendWindowsPath=true\n"
        ))
    );
}

mod owners_automount_changed {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the owner's automount line changed",
        expected =
            "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf with R-1010's two additions",
        check_conf(&edited(
            &read(CONF),
            "[automount]\nenabled=false\n",
            "[automount]\nenabled=true\n"
        ))
    );
}

mod build_writes_systemd {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the build appending systemd= to the installed file",
        expected = "scripts/wsl/build-image.sh writes `systemd=`",
        check_build_writing("echo 'systemd=true' | sudo tee -a \"$root/etc/wsl.conf\" > /dev/null")
    );
}

mod build_writes_boot {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the build writing a boot section of its own",
        expected = "scripts/wsl/build-image.sh writes `[boot]`",
        check_build_writing(
            "printf '[boot]\\ncommand=true\\n' | sudo tee -a \"$root/etc/wsl.conf\" > /dev/null"
        )
    );
}

mod build_writes_sudoers {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the build writing a sudoers entry",
        expected = "scripts/wsl/build-image.sh writes `sudoers.d/`",
        check_build_writing(
            "echo 'ci ALL=(ALL) NOPASSWD: ALL' | sudo tee \"$root/etc/sudoers.d/ci\" > /dev/null"
        )
    );
}

mod keeps_sudo {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "sudo kept",
        expected = "the image keeps sudo",
        check_image(
            &edited(&read(BUILD), PURGE, ""),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod unforced_purge {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the purge left to sudo's guard, which refuses it",
        expected = "the image keeps sudo",
        check_image(
            &edited(&read(BUILD), PURGE, "in_root apt-get -q -y purge sudo\n"),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod forced_everywhere {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the bypass exported to every command",
        expected = "SUDO_FORCE_REMOVE is set beyond the purge's own command",
        check_image(
            &edited(
                &read(BUILD),
                PURGE,
                &format!("export SUDO_FORCE_REMOVE=yes\n{PURGE}")
            ),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod unproved {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the purge taken on apt's word",
        expected = "the build doesn't prove sudo gone, or doesn't fail on it",
        check_image(
            &edited(&read(BUILD), SUDO_GONE, ""),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod proof_not_fatal {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a failed proof that lets the build go on",
        expected = "the build doesn't prove sudo gone, or doesn't fail on it",
        check_image(
            &edited(
                &read(BUILD),
                "after the purge\"\n  exit 1\n}\n",
                "after the purge\"\n}\n"
            ),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod proof_before_purge {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "sudo proved gone before it's purged",
        expected = "the proof that sudo is gone doesn't follow the purge",
        {
            let build = read(BUILD);
            let moved =
                edited(&build, SUDO_GONE, "").replace(PURGE, &format!("{SUDO_GONE}{PURGE}"));
            check_image(&moved, &read(CONF), &read(".github/workflows/ci.yml"));
        }
    );
}

mod unverified {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "the base unpacked first",
        expected = "the base is unpacked before it's verified",
        {
            let build = read(BUILD);
            let check = "echo \"$BASE_SHA256  $work/base.tar.gz\" | sha256sum -c -\n";
            let moved = edited(&build, check, "").replace(
                "sudo mkdir \"$root\"\n",
                &format!(
                    "sudo mkdir \"$root\"\nsudo tar -xzpf \"$work/base.tar.gz\" -C x\n{check}"
                ),
            );
            check_image(&moved, &read(CONF), &read(".github/workflows/ci.yml"));
        }
    );
}

mod other_nextest {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a nextest other than ci.yml's",
        expected = "the image's nextest isn't ci.yml's",
        check_image(
            &edited(
                &read(BUILD),
                "NEXTEST_VERSION=0.9.146",
                "NEXTEST_VERSION=0.9.999"
            ),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod other_fallback {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a fallback to another point release",
        expected = "the base's fallback isn't the same image on old-releases.ubuntu.com",
        check_image(
            &edited(
                &read(BUILD),
                "BASE_OLD_URL=https://old-releases.ubuntu.com/releases/noble/ubuntu-24.04.5-wsl-amd64.wsl",
                "BASE_OLD_URL=https://old-releases.ubuntu.com/releases/noble/ubuntu-24.04.2-wsl-amd64.wsl"
            ),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

mod other_host {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a fallback on another host",
        expected = "the base's fallback isn't the same image on old-releases.ubuntu.com",
        check_image(
            &edited(
                &read(BUILD),
                "BASE_OLD_URL=https://old-releases.ubuntu.com/",
                "BASE_OLD_URL=https://example.com/"
            ),
            &read(CONF),
            &read(".github/workflows/ci.yml")
        )
    );
}

// ---- ci's cargo config --------------------------------------------------------------------------------------------

/// What `.github/actions/cargo-ci-config` runs to give a hosted job its cargo config.
const CARGO_CI_CONFIG: &str = ".github/actions/cargo-ci-config/config.sh";
/// How the build puts ci's cargo config in the image: root's, 0644, from the build's own copy.
const CARGO_CONFIG_INSTALL: &str =
    "sudo install -m 0644 -o 0 -g 0 \"$work/cargo-config.toml\" \"$root/home/ci/.cargo/config.toml\"";

/// ci's cargo config in the image holds the bytes `config.sh` appends to a hosted job's CARGO_HOME, written by the
/// build to its own copy and installed root's, 0644, so ci reads it and can't write it (on 14 Nov 2026 a bare `bash`
/// reached the distro as ci and appended them twice more, run 43593938536); nothing else names the file; the install
/// follows every command run as ci, and no command after it touches ci's home.
fn check_cargo_config(build: &str, config_sh: &str) {
    let format = config_sh
        .lines()
        .find_map(|l| l.strip_prefix("printf '"))
        .and_then(|rest| rest.split_once('\''))
        .map(|(format, _)| format)
        .expect("config.sh prints no cargo config");
    let write = format!("printf '{format}' > \"$work/cargo-config.toml\"");
    let commands: Vec<(usize, &str)> = build
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with('#'))
        .collect();
    assert!(
        commands.iter().any(|&(_, l)| l == write),
        "ci's cargo config isn't the bytes {CARGO_CI_CONFIG} gives CI's jobs"
    );
    let others: Vec<&str> = commands
        .iter()
        .map(|&(_, l)| l)
        .filter(|l| l.contains("config.toml") && *l != write)
        .collect();
    assert_eq!(
        others,
        [CARGO_CONFIG_INSTALL],
        "ci's cargo config isn't installed by root alone, root's and 0644"
    );
    let install = commands
        .iter()
        .find(|&&(_, l)| l == CARGO_CONFIG_INSTALL)
        .map(|&(n, _)| n)
        .expect("checked above");
    assert!(
        commands
            .iter()
            .filter(|(_, l)| l.starts_with("as_ci "))
            .all(|&(n, _)| n < install),
        "a command runs as ci after ci's cargo config is installed"
    );
    let after: Vec<&str> = commands
        .iter()
        .filter(|&&(n, l)| n > install && l.contains("/home"))
        .map(|&(_, l)| l)
        .collect();
    assert!(
        after.is_empty(),
        "a command after ci's cargo config is installed touches ci's home: {after:?}"
    );
}

#[test]
fn wsl_image_cargo_config_is_roots_and_read_only_to_ci() {
    check_cargo_config(&read(BUILD), &read(CARGO_CI_CONFIG));
}

/// The build as it was before: ci writes its own cargo config, which it can then append to.
#[cfg(feature = "controls")]
const AS_CI_WRITE: &str = r#"as_ci sh -c 'printf '\''[profile.dev.package."*"]\ndebug = false\n[profile.test.package."*"]\ndebug = false\n'\'' > /home/ci/.cargo/config.toml'"#;

negative_control!(
    wsl_image_cargo_config_is_roots_and_read_only_to_ci,
    "ci writing its own cargo config, as the image did",
    expected = "ci's cargo config isn't installed by root alone, root's and 0644",
    check_cargo_config(
        &edited(&read(BUILD), CARGO_CONFIG_INSTALL, AS_CI_WRITE),
        &read(CARGO_CI_CONFIG)
    )
);

mod cargo_config_appended {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "ci appending to the installed config",
        expected = "ci's cargo config isn't installed by root alone, root's and 0644",
        check_cargo_config(
            &edited(
                &read(BUILD),
                CARGO_CONFIG_INSTALL,
                &format!(
                    "{CARGO_CONFIG_INSTALL}\nas_ci sh -c 'cat /tmp/x >> /home/ci/.cargo/config.toml'"
                )
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

mod cargo_config_writable {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "the config installed group-writable",
        expected = "ci's cargo config isn't installed by root alone, root's and 0644",
        check_cargo_config(
            &edited(
                &read(BUILD),
                "sudo install -m 0644 -o 0 -g 0 \"$work/cargo-config.toml\"",
                "sudo install -m 0664 -o 0 -g 0 \"$work/cargo-config.toml\""
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

mod cargo_config_cis {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "the config installed ci's",
        expected = "ci's cargo config isn't installed by root alone, root's and 0644",
        check_cargo_config(
            &edited(
                &read(BUILD),
                "sudo install -m 0644 -o 0 -g 0 \"$work/cargo-config.toml\"",
                "sudo install -m 0644 -o 1000 -g 1000 \"$work/cargo-config.toml\""
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

mod cargo_config_other_bytes {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "debug info kept for the dependencies' tests",
        expected = "ci's cargo config isn't the bytes .github/actions/cargo-ci-config/config.sh gives CI's jobs",
        check_cargo_config(
            &edited(
                &read(BUILD),
                "[profile.test.package.\"*\"]\\ndebug = false\\n' > \"$work/cargo-config.toml\"",
                "[profile.test.package.\"*\"]\\ndebug = true\\n' > \"$work/cargo-config.toml\""
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

mod cargo_config_ci_changed {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "CI's config changed and the image's not",
        expected = "ci's cargo config isn't the bytes .github/actions/cargo-ci-config/config.sh gives CI's jobs",
        check_cargo_config(
            &read(BUILD),
            &edited(&read(CARGO_CI_CONFIG), "printf '[profile.dev.package.\"*\"]\\ndebug = false", "printf '[profile.dev.package.\"*\"]\\ndebug = 1")
        )
    );
}

mod cargo_config_before_ci {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "the config installed before the cargo tools are, as ci",
        expected = "a command runs as ci after ci's cargo config is installed",
        {
            let build = read(BUILD);
            let tools = "as_ci cargo install --locked \"cargo-nextest@$NEXTEST_VERSION\" \"cargo-mutants@$MUTANTS_VERSION\"\n";
            let moved = edited(
                &edited(&build, tools, ""),
                &format!("{CARGO_CONFIG_INSTALL}\n"),
                &format!("{CARGO_CONFIG_INSTALL}\n{tools}"),
            );
            check_cargo_config(&moved, &read(CARGO_CI_CONFIG));
        }
    );
}

mod cargo_config_handed_back {
    use super::*;
    negative_control!(
        wsl_image_cargo_config_is_roots_and_read_only_to_ci,
        "ci's home handed back to ci after the install",
        expected = "a command after ci's cargo config is installed touches ci's home",
        check_cargo_config(
            &edited(
                &read(BUILD),
                &format!("{CARGO_CONFIG_INSTALL}\n"),
                &format!("{CARGO_CONFIG_INSTALL}\nsudo chown -R 1000:1000 \"$root/home/ci\"\n")
            ),
            &read(CARGO_CI_CONFIG)
        )
    );
}

// ---- Vulkan, as on the hosted runner ------------------------------------------------------------------------------

const CI: &str = ".github/workflows/ci.yml";

/// The packages of `NAME=(...)` in the build script.
fn array<'a>(build: &'a str, name: &str) -> Vec<&'a str> {
    setting(build, name)
        .strip_prefix('(')
        .and_then(|a| a.strip_suffix(')'))
        .unwrap_or_else(|| panic!("{BUILD}'s {name} isn't an array"))
        .split_whitespace()
        .collect()
}

/// `ci.yml`'s jobs that install lavapipe: the packages each one's step installs, sorted, and every `PRIN_GPU_BACKEND`
/// or `VK_*` setting of the job's env or a step's.
fn hosted_vulkan(ci: &str) -> (Vec<Vec<String>>, Vec<(String, String)>) {
    let doc = parse(ci);
    let (mut installs, mut env) = (Vec::new(), Vec::new());
    let jobs = doc["jobs"].as_hash().expect("ci.yml's jobs");
    for job in jobs.values() {
        let steps = job["steps"].as_vec().map_or(&[][..], Vec::as_slice);
        let mut lavapipe = false;
        for step in steps {
            let named = step["name"].as_str().unwrap_or("").contains("lavapipe");
            let run = step["run"].as_str().unwrap_or("");
            if let Some((_, rest)) = run.split_once("apt-get install").filter(|_| named) {
                lavapipe = true;
                let mut packages: Vec<String> = rest
                    .split_whitespace()
                    .filter(|w| !w.starts_with('-'))
                    .map(str::to_owned)
                    .collect();
                packages.sort();
                installs.push(packages);
            }
        }
        if !lavapipe {
            continue;
        }
        let envs = std::iter::once(&job["env"]).chain(steps.iter().map(|s| &s["env"]));
        for map in envs.filter_map(Yaml::as_hash) {
            for (k, v) in map {
                let key = k.as_str().unwrap_or("");
                if key == "PRIN_GPU_BACKEND" || key.starts_with("VK_") {
                    env.push((key.to_owned(), v.as_str().unwrap_or("").to_owned()));
                }
            }
        }
    }
    (installs, env)
}

fn check_vulkan(ci: &str, build: &str, inside: &str) {
    let (installs, env) = hosted_vulkan(ci);
    let first = installs
        .first()
        .unwrap_or_else(|| panic!("{CI} installs no lavapipe"));
    assert!(
        installs.iter().all(|i| i == first),
        "{CI}'s lavapipe steps install different packages: {installs:?}"
    );
    let packages = array(build, "PACKAGES");
    for package in first {
        assert!(
            packages.contains(&package.as_str()),
            "the image doesn't install the hosted job's Vulkan package {package}"
        );
    }
    assert!(
        env.iter().any(|(k, _)| k == "PRIN_GPU_BACKEND"),
        "{CI}'s lavapipe jobs select no GPU backend"
    );
    // The settings `inside.sh` makes for every verb, before its first function.
    let top = &inside[..inside.find("\nok() {").expect("inside.sh's ok()")];
    for (key, value) in &env {
        let set = format!("{key}={value}");
        assert!(
            top.split_whitespace().any(|w| w == set),
            "inside.sh doesn't select the hosted job's Vulkan: no {set}"
        );
    }
}

#[test]
fn wsl_image_provisions_the_hosted_jobs_vulkan() {
    check_vulkan(&read(CI), &read(BUILD), &read(INSIDE));
}

negative_control!(
    wsl_image_provisions_the_hosted_jobs_vulkan,
    "no lavapipe in the image",
    expected = "the image doesn't install the hosted job's Vulkan package mesa-vulkan-drivers",
    check_vulkan(
        &read(CI),
        &edited(&read(BUILD), " mesa-vulkan-drivers)", ")"),
        &read(INSIDE)
    )
);

mod hosted_loader {
    use super::*;
    negative_control!(
        wsl_image_provisions_the_hosted_jobs_vulkan,
        "the hosted job installs a package the image lacks",
        expected = "the image doesn't install the hosted job's Vulkan package vulkan-tools",
        {
            let ci = read(CI).replace(
                "apt-get install -y mesa-vulkan-drivers",
                "apt-get install -y mesa-vulkan-drivers vulkan-tools",
            );
            check_vulkan(&ci, &read(BUILD), &read(INSIDE));
        }
    );
}

mod hosted_differ {
    use super::*;
    negative_control!(
        wsl_image_provisions_the_hosted_jobs_vulkan,
        "two hosted jobs install different packages",
        expected = "lavapipe steps install different packages",
        check_vulkan(
            &edited(
                &read(CI),
                "apt-get install -y mesa-vulkan-drivers",
                "apt-get install -y mesa-vulkan-drivers vulkan-tools"
            ),
            &read(BUILD),
            &read(INSIDE)
        )
    );
}

mod hosted_none {
    use super::*;
    negative_control!(
        wsl_image_provisions_the_hosted_jobs_vulkan,
        "no hosted lavapipe step",
        expected = "installs no lavapipe",
        check_vulkan(
            &read(CI).replace("Install Mesa (lavapipe)", "Install Mesa"),
            &read(BUILD),
            &read(INSIDE)
        )
    );
}

mod no_backend {
    use super::*;
    negative_control!(
        wsl_image_provisions_the_hosted_jobs_vulkan,
        "the distro selects no backend",
        expected = "inside.sh doesn't select the hosted job's Vulkan: no PRIN_GPU_BACKEND=vulkan",
        check_vulkan(
            &read(CI),
            &read(BUILD),
            &edited(&read(INSIDE), " PRIN_GPU_BACKEND=vulkan", "")
        )
    );
}

mod hosted_icd {
    use super::*;
    negative_control!(
        wsl_image_provisions_the_hosted_jobs_vulkan,
        "the hosted job names an ICD the distro doesn't",
        expected = "no VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json",
        check_vulkan(
            &read(CI).replace(
                "\n      PRIN_GPU_BACKEND: vulkan\n",
                "\n      PRIN_GPU_BACKEND: vulkan\n      VK_ICD_FILENAMES: /usr/share/vulkan/icd.d/lvp_icd.x86_64.json\n",
            ),
            &read(BUILD),
            &read(INSIDE)
        )
    );
}

// ---- Registration -------------------------------------------------------------------------------------------------

/// `wsl.ps1 register` reads the pin first, refuses one that isn't a SHA-256, checks the copy (`Assert-Image`) and the
/// manifest against it before `wsl --import`, imports under E:\wsl\, and derives the manifest again inside the
/// distro after.
fn check_register(script: &str) {
    let register = arm(script, "register");
    let pinned = register
        .find("$pinned = Get-Pinned")
        .expect("register reads no pin");
    let import = register
        .find("& wsl.exe --import $Distro $Disk $rootfs --version 2")
        .expect("no import");
    assert!(
        pinned < import
            && register
                .find("        Assert-Image $image\n")
                .is_some_and(|c| c < import)
            && ps_function(script, "Assert-Image").contains("if ($copied -ne $built)")
            && register
                .find("if ($hash -ne $pinned)")
                .is_some_and(|c| c < import),
        "register imports before checking the image against the pin"
    );
    assert!(
        script.contains("if ($pin -notmatch '^[0-9a-f]{64}$') {"),
        "an unpinned manifest doesn't refuse registration"
    );
    assert!(
        script.contains("$Disk = 'E:\\wsl\\principia-wsl'"),
        "the distro's disk isn't under E:\\wsl\\"
    );
    assert!(
        register
            .find("$live = Get-LiveManifest")
            .is_some_and(|l| l > import)
            && register.contains("if ($liveHash -eq $pinned) {"),
        "register doesn't derive the manifest again inside the distro"
    );
    assert!(
        register.contains("if ($env:WSL_REPLACE -ne 'true') { Fail"),
        "register replaces a registered distro unasked"
    );
}

#[test]
fn wsl_register_refuses_without_the_pinned_manifest() {
    check_register(&read(PS));
}

negative_control!(
    wsl_register_refuses_without_the_pinned_manifest,
    "any manifest accepted",
    expected = "an unpinned manifest doesn't refuse registration",
    check_register(&edited(
        &read(PS),
        "if ($pin -notmatch '^[0-9a-f]{64}$') {",
        "if ($false) {"
    ))
);

mod unchecked_copy {
    use super::*;
    negative_control!(
        wsl_register_refuses_without_the_pinned_manifest,
        "the copy's hash never compared",
        expected = "register imports before checking the image against the pin",
        check_register(&edited(&read(PS), "if ($copied -ne $built)", "if ($false)"))
    );
}

/// After a successful `wsl --import`, everything up to the success line runs in a `try` whose `finally` stops and
/// unregisters the distro unless the manifest derived inside it was the pin: a throw, a mismatch or an interrupted
/// check leaves no unverified distro registered. No `Fail` or `exit` sits inside the `try`; the arm fails after the
/// `finally` has run, and the success line is reached only verified.
fn check_register_guard(script: &str) {
    let register = arm(script, "register");
    let import = register
        .find("if ($LASTEXITCODE -ne 0) { Fail \"wsl --import: exit $LASTEXITCODE\" }")
        .expect("no import check");
    let after = &register[import..];
    let at = |needle: &str| after.find(needle);
    let (Some(unset), Some(try_at), Some(finally_at), Some(success)) = (
        at("$verified = $false"),
        at("\n        try {\n"),
        at("\n        } finally {\n"),
        at("\"registered ${Distro}: its manifest, derived inside it, is the pinned $pinned\""),
    ) else {
        panic!("the import isn't followed by a guard that unregisters on failure");
    };
    assert!(
        unset < try_at && try_at < finally_at && finally_at < success,
        "the import isn't followed by a guard that unregisters on failure"
    );
    let guarded = &after[try_at..finally_at];
    assert!(
        guarded.find("$live = Get-LiveManifest").is_some() && guarded.find("Stop-Distro").is_some(),
        "the check of the imported distro runs outside the guard"
    );
    assert!(
        guarded.contains("if ($liveHash -eq $pinned) {\n                $verified = $true\n")
            && after.matches("$verified = $true").count() == 1,
        "the guard counts the distro verified without the pin"
    );
    assert!(
        !guarded.contains("Fail ") && !guarded.contains("exit"),
        "the guarded check exits from inside the guard"
    );
    let cleanup = &after[finally_at..success];
    let cleanup = &cleanup[..cleanup
        .find("\n        }\n")
        .expect("the finally block ends")];
    assert!(
        cleanup.contains("if (-not $verified) {")
            && cleanup.contains("try { Stop-Distro } catch { }")
            && cleanup.contains("& wsl.exe --unregister $Distro"),
        "the guard doesn't stop and unregister an unverified distro"
    );
    let verdict = &after[finally_at..success];
    assert!(
        verdict.contains("\n        if (-not $verified) {\n            if ($unregistered) { Fail ")
            && verdict.contains("\n            Fail \"${why}; wsl --unregister"),
        "an unverified distro reaches the success line"
    );
}

#[test]
fn wsl_register_unregisters_a_half_done_import() {
    check_register_guard(&read(PS));
}

negative_control!(
    wsl_register_unregisters_a_half_done_import,
    "the finally keeps the distro",
    expected = "the guard doesn't stop and unregister an unverified distro",
    check_register_guard(&edited(
        &read(PS),
        "$unregistered = try { & wsl.exe --unregister $Distro | Out-Null; $LASTEXITCODE -eq 0 } catch { $false }",
        "$unregistered = $true"
    ))
);

mod register_unguarded {
    use super::*;
    negative_control!(
        wsl_register_unregisters_a_half_done_import,
        "no guard after the import",
        expected = "the import isn't followed by a guard that unregisters on failure",
        check_register_guard(&edited(
            &read(PS),
            "\n        } finally {\n",
            "\n        }\n        {\n"
        ))
    );
}

mod register_check_outside {
    use super::*;
    negative_control!(
        wsl_register_unregisters_a_half_done_import,
        "the live manifest derived before the guard",
        expected = "the check of the imported distro runs outside the guard",
        check_register_guard(&edited(
            &read(PS),
            "        try {\n            $live = Get-LiveManifest\n",
            "        $live = Get-LiveManifest\n        try {\n"
        ))
    );
}

mod register_verified_unconditionally {
    use super::*;
    negative_control!(
        wsl_register_unregisters_a_half_done_import,
        "verified whatever the hash",
        expected = "the guard counts the distro verified without the pin",
        check_register_guard(&edited(
            &read(PS),
            "if ($liveHash -eq $pinned) {\n                $verified = $true\n",
            "if ($true) {\n                $verified = $true\n"
        ))
    );
}

mod register_fails_inside {
    use super::*;
    negative_control!(
        wsl_register_unregisters_a_half_done_import,
        "a Fail inside the try",
        expected = "the guarded check exits from inside the guard",
        check_register_guard(&edited(
            &read(PS),
            "$why = \"inside the imported distro the manifest derives to $liveHash, not the pinned $pinned\"",
            "Fail \"inside the imported distro the manifest derives to $liveHash, not the pinned $pinned\""
        ))
    );
}

mod register_success_unverified {
    use super::*;
    negative_control!(
        wsl_register_unregisters_a_half_done_import,
        "the failure verdict dropped",
        expected = "an unverified distro reaches the success line",
        check_register_guard(&edited(
            &read(PS),
            "            if ($unregistered) { Fail ",
            "            if ($unregistered) { \"\" + "
        ))
    );
}

// ---- The scripts parse under Windows PowerShell ---------------------------------------------------------------------

/// Where the repository's PowerShell scripts live.
const PS_DIRS: [&str; 2] = [".github/scripts", "scripts"];
/// The namespaces a `$name:` may really name: PowerShell's scopes and the drives the scripts read variables from.
const PS_SCOPES: [&str; 8] = [
    "env", "script", "global", "using", "local", "private", "function", "variable",
];

/// Every `.ps1` under `PS_DIRS`, at any depth, by its path from the root, with its text.
fn ps_scripts() -> Vec<(String, String)> {
    fn walk(root: &Path, rel: &Path, found: &mut Vec<(String, String)>) {
        let entries =
            std::fs::read_dir(root.join(rel)).unwrap_or_else(|e| panic!("{}: {e}", rel.display()));
        for entry in entries {
            let entry = entry.expect("a directory entry");
            let path = rel.join(entry.file_name());
            if entry.file_type().expect("a file type").is_dir() {
                walk(root, &path, found);
            } else if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ps1"))
            {
                let name = path.to_string_lossy().replace('\\', "/");
                found.push((name.clone(), read(&name)));
            }
        }
    }
    let mut found = Vec::new();
    for dir in PS_DIRS {
        walk(&root(), Path::new(dir), &mut found);
    }
    found.sort();
    found
}

/// A character of a PowerShell variable's name: a letter or a digit, ASCII or not, `_`, or `?`.
fn ps_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '?'
}

/// Each `$name:` in `text`, with its line number, but for a scope's or a drive's with a name after its colon.
/// PowerShell reads `$name:` as a variable in the drive or scope `name`, in a double-quoted string as much as in code,
/// and what follows the colon must then be a variable's name: "$pinned: refusing it" doesn't parse
/// (`InvalidVariableReferenceWithDrive`), no more does "$script: failed", and "${pinned}: refusing it" does. This reads
/// the text, not PowerShell's tokens, so it is stricter than the parser: it names one in a comment or a single-quoted
/// string too, where `${name}:` reads the same to a person; one whose name is no scope's with a name after the colon
/// (`$pinned:x`), which parses and reads a drive that isn't there; and `$type::Member`, which no script here uses and
/// `($type)::Member` says as well.
fn drive_references(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        for (at, _) in line.match_indices('$') {
            let rest = &line[at + 1..];
            let len = rest.find(|c: char| !ps_name_char(c)).unwrap_or(rest.len());
            let name = &rest[..len];
            let Some(after) = rest[len..].strip_prefix(':') else {
                continue;
            };
            let scoped = PS_SCOPES.iter().any(|s| s.eq_ignore_ascii_case(name))
                && after.starts_with(ps_name_char);
            if !name.is_empty() && !scoped {
                found.push((n + 1, format!("${name}:")));
            }
        }
    }
    found
}

/// No script holds a `$name:` that isn't a scope's or a drive's; a script that doesn't parse never runs at all.
fn check_ps_references(scripts: &[(String, String)]) {
    assert!(
        scripts.iter().any(|(name, _)| name == PS),
        "the scan found no {PS}: it reads nothing"
    );
    for (name, text) in scripts {
        let found = drive_references(text);
        assert!(
            found.is_empty(),
            "{name} doesn't parse under Windows PowerShell: `$name:` reads as a drive-qualified variable; write \
             `${{name}}:`: {found:?}"
        );
    }
}

/// `scripts`, with `from` replaced by `to` in wsl.ps1.
#[cfg(feature = "controls")]
fn ps_scripts_edited(from: &str, to: &str) -> Vec<(String, String)> {
    let mut scripts = ps_scripts();
    for (name, text) in &mut scripts {
        if name == PS {
            *text = edited(text, from, to);
        }
    }
    scripts
}

#[test]
fn ps_scripts_hold_no_drive_qualified_reference() {
    check_ps_references(&ps_scripts());
}

negative_control!(
    ps_scripts_hold_no_drive_qualified_reference,
    "the register script's first message, as run 42944409475 refused it",
    expected = "doesn't parse under Windows PowerShell",
    check_ps_references(&ps_scripts_edited(
        "the pin is ${pinned}: refusing it",
        "the pin is $pinned: refusing it"
    ))
);

mod unregister_message {
    use super::*;
    negative_control!(
        ps_scripts_hold_no_drive_qualified_reference,
        "the unregister message, as that run refused it",
        expected = "doesn't parse under Windows PowerShell",
        check_ps_references(&ps_scripts_edited(
            "wsl --unregister ${Distro}: exit",
            "wsl --unregister $Distro: exit"
        ))
    );
}

mod unread {
    use super::*;
    negative_control!(
        ps_scripts_hold_no_drive_qualified_reference,
        "a scan that reads no script",
        expected = "it reads nothing",
        check_ps_references(&[])
    );
}

/// The scan's edges: what it names and what it leaves.
fn check_drive_references(text: &str, expected: &[(usize, &str)]) {
    let found = drive_references(text);
    let found: Vec<(usize, &str)> = found.iter().map(|(n, r)| (*n, r.as_str())).collect();
    assert_eq!(
        found, expected,
        "the scan of {text:?} names the wrong references"
    );
}

/// A scope's or a drive's reference, in any case, and one to a name starting with `_`, `?`, a digit or a letter that
/// isn't ASCII; a braced name; a colon apart from the name; a subexpression's; a bare `$`; a name with no colon.
const PS_SOUND: &str =
    "\"$env:A $script:b $GLOBAL:c $using:d $local:e $private:f $function:g $variable:h\"\n\
     \"$env:_a $env:?b $env:1 $env:\u{e9}t\u{e9}\"\n\
                        \"${pinned}: $pinned : $($r.Code): $ : $: $pinned\"\n";

#[test]
fn ps_scan_names_only_what_does_not_parse() {
    check_drive_references(PS_SOUND, &[]);
    check_drive_references(
        "\"$a: b\"\n\n\"x $_: $envx: $en: ${ok}: $B2:$c:\"\n$last:",
        &[
            (1, "$a:"),
            (3, "$_:"),
            (3, "$envx:"),
            (3, "$en:"),
            (3, "$B2:"),
            (3, "$c:"),
            (4, "$last:"),
        ],
    );
    // A scope's name is a reference only with a name after its colon: before a space, at a string's end, at the
    // line's end and before a second colon it is the same parse error.
    check_drive_references(
        "\"$script: failed\"\n\"$env: x\"\n\"$local:\"\n$Function:\n[int]$global::MaxValue $x::y",
        &[
            (1, "$script:"),
            (2, "$env:"),
            (3, "$local:"),
            (4, "$Function:"),
            (5, "$global:"),
            (5, "$x:"),
        ],
    );
    // `?` and letters outside ASCII are a name's characters, and don't end it early.
    check_drive_references(
        "\"$ok?: $na\u{ef}ve: $?: $\u{e9}:\"",
        &[
            (1, "$ok?:"),
            (1, "$na\u{ef}ve:"),
            (1, "$?:"),
            (1, "$\u{e9}:"),
        ],
    );
}

negative_control!(
    ps_scan_names_only_what_does_not_parse,
    "a scan that takes every name for a scope",
    expected = "names the wrong references",
    check_drive_references("\"the pin is $pinned: refusing it\"", &[])
);

mod scope_alone {
    use super::*;
    negative_control!(
        ps_scan_names_only_what_does_not_parse,
        "a scope's name exempt whatever follows its colon",
        expected = "names the wrong references",
        check_drive_references("\"$script: failed\"", &[])
    );
}

// ---- A refusal's line reaches the log ------------------------------------------------------------------------------

/// The text of wsl.ps1's function `name`, from its `function` line to its closing brace.
fn ps_function<'a>(script: &'a str, name: &str) -> &'a str {
    let start = script
        .find(&format!("\nfunction {name}"))
        .unwrap_or_else(|| panic!("{PS} has no function {name}"));
    let rest = &script[start + 1..];
    &rest[..rest.find("\n}\n").expect("the function ends") + 2]
}

/// `Fail` writes its `::error::` line to the console and exits 125. Left on the output stream, the line is lost with
/// the exit whenever the caller is collecting that output: `$pinned = Get-Pinned`, the refusal of an unpinned
/// manifest, exited 125 and said nothing. So `Fail` emits nothing: its body is the write and the exit.
fn check_fail(script: &str) {
    let fail = ps_function(script, "Fail");
    let body: Vec<&str> = fail.lines().map(str::trim).collect();
    assert_eq!(
        body,
        [
            "function Fail($message) {",
            "[Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1: $message\"))",
            "exit 125",
            "}"
        ],
        "Fail's line can be lost to a caller collecting output, or its exit isn't 125"
    );
    // The refusal that showed it is still Fail's, from a function whose output register assigns.
    assert!(
        ps_function(script, "Get-Pinned").contains("        Fail \"no manifest hash is pinned")
            && arm(script, "register").contains("$pinned = Get-Pinned"),
        "the unpinned refusal isn't Fail's"
    );
}

#[test]
fn wsl_fail_writes_where_no_assignment_collects_it() {
    check_fail(&read(PS));
}

negative_control!(
    wsl_fail_writes_where_no_assignment_collects_it,
    "the line left on the output stream, as it was",
    expected = "Fail's line can be lost to a caller collecting output",
    check_fail(&edited(
        &read(PS),
        "    [Console]::Out.WriteLine((Hide-Tailnet \"::error::wsl.ps1: $message\"))\n",
        "    \"::error::wsl.ps1: $message\"\n"
    ))
);

mod other_status {
    use super::*;
    negative_control!(
        wsl_fail_writes_where_no_assignment_collects_it,
        "Fail exits 1",
        expected = "or its exit isn't 125",
        check_fail(&edited(&read(PS), "    exit 125\n}", "    exit 1\n}"))
    );
}

// ---- What wsl.exe is sent, and what a failed proof 6 shows --------------------------------------------------------

/// Nothing before what Invoke-Wsl sends: Windows PowerShell's .NET writes a started process's stdin in
/// Console.InputEncoding, with its preamble first (run 43418528499: every stdin began with a byte-order mark). So the
/// script sets that encoding to UTF-8 with no preamble before any process starts, a newer .NET's StandardInputEncoding
/// too, and Invoke-Wsl refuses to send anything while a preamble remains.
fn check_stdin_encoding(script: &str) {
    let first_start = script
        .find("[System.Diagnostics.Process]::Start(")
        .expect("wsl.ps1 starts no process");
    let set = script
        .find("try { [Console]::InputEncoding = $NoBom }")
        .filter(|&at| at < first_start);
    assert!(
        script.contains("$NoBom = New-Object System.Text.UTF8Encoding $false\n") && set.is_some(),
        "wsl.ps1 doesn't set stdin's encoding to UTF-8 with no byte-order mark before it starts a process"
    );
    assert!(
        script.contains(
            "if ($psi.PSObject.Properties['StandardInputEncoding']) { $psi.StandardInputEncoding = $NoBom }"
        ),
        "Start-Wsl leaves a newer .NET's stdin encoding as it was"
    );
    let invoke = &script[script.find("function Invoke-Wsl(").expect("no Invoke-Wsl")..];
    let guard = invoke
        .find(
            "if ($Stdin -and [Console]::InputEncoding.GetPreamble().Length -ne 0) {\n        Fail ",
        )
        .filter(|&at| {
            at < invoke
                .find("$p = Start-Wsl")
                .expect("Invoke-Wsl starts nothing")
        });
    assert!(
        guard.is_some(),
        "Invoke-Wsl sends stdin in an encoding with a byte-order mark"
    );
}

#[test]
fn wsl_stdin_carries_no_byte_order_mark() {
    check_stdin_encoding(&read(PS));
}

negative_control!(
    wsl_stdin_carries_no_byte_order_mark,
    "UTF-8 with its byte-order mark, as Windows PowerShell gave it",
    expected = "doesn't set stdin's encoding to UTF-8 with no byte-order mark",
    check_stdin_encoding(&edited(
        &read(PS),
        "System.Text.UTF8Encoding $false",
        "System.Text.UTF8Encoding $true"
    ))
);

mod encoding_unset {
    use super::*;
    negative_control!(
        wsl_stdin_carries_no_byte_order_mark,
        "the encoding never set",
        expected = "doesn't set stdin's encoding to UTF-8 with no byte-order mark",
        check_stdin_encoding(&edited(
            &read(PS),
            "try { [Console]::InputEncoding = $NoBom }",
            "try { }"
        ))
    );
}

mod newer_dotnet {
    use super::*;
    negative_control!(
        wsl_stdin_carries_no_byte_order_mark,
        "a newer .NET's own stdin encoding left as it was",
        expected = "Start-Wsl leaves a newer .NET's stdin encoding as it was",
        check_stdin_encoding(&edited(
            &read(PS),
            "{ $psi.StandardInputEncoding = $NoBom }",
            "{ }"
        ))
    );
}

mod unguarded {
    use super::*;
    negative_control!(
        wsl_stdin_carries_no_byte_order_mark,
        "a mark left in place sent anyway",
        expected = "Invoke-Wsl sends stdin in an encoding with a byte-order mark",
        check_stdin_encoding(&edited(
            &read(PS),
            "if ($Stdin -and [Console]::InputEncoding.GetPreamble().Length -ne 0) {",
            "if ($false) {"
        ))
    );
}

/// A failed proof 6 says why: Invoke-Wsl keeps the stderr it shows, and proof 6's line carries the last lines each
/// failed side wrote, stderr's included, where a traceback is (run 43418528499: an empty line from `parity:*` alone).
fn check_proof_six_tail(script: &str) {
    assert!(
        script.contains(
            "[pscustomobject]@{ Code = $p.ExitCode; Lines = $lines.ToArray(); Err = \"$e\" }"
        ),
        "Invoke-Wsl drops the stderr it showed"
    );
    let tail = &script[script.find("function Get-Tail(").expect("no Get-Tail")..];
    assert!(
        tail.contains("@($Result.Lines) + @(\"$($Result.Err)\" -split")
            && tail.contains("Select-Object -Last $Count"),
        "Get-Tail isn't the last lines of stdout and stderr"
    );
    let six = &script[script.find("# Proof 6:").expect("no proof 6")..];
    for side in ["parity", "compare"] {
        assert!(
            six.contains(&format!(
                "if (${side}.Code -ne 0) {{ $evidence += \"; inside.sh {side} exited $(${side}.Code), ending: \
                 $(Get-Tail ${side})\" }}"
            )),
            "proof 6's line leaves out how inside.sh {side} ended"
        );
    }
    assert!(
        six.contains("Report 6 ($parity.Code -eq 0 -and $compare.Code -eq 0) $evidence"),
        "proof 6 reports something other than its evidence"
    );
}

#[test]
fn wsl_proof_six_says_how_a_side_failed() {
    check_proof_six_tail(&read(PS));
}

negative_control!(
    wsl_proof_six_says_how_a_side_failed,
    "the comparison's traceback left out",
    expected = "proof 6's line leaves out how inside.sh compare ended",
    check_proof_six_tail(&edited(
        &read(PS),
        "if ($compare.Code -ne 0) {",
        "if ($false) {"
    ))
);

mod parity_tail {
    use super::*;
    negative_control!(
        wsl_proof_six_says_how_a_side_failed,
        "the run's tail left out",
        expected = "proof 6's line leaves out how inside.sh parity ended",
        check_proof_six_tail(&edited(&read(PS), ", ending: $(Get-Tail $parity)", ""))
    );
}

mod stderr_dropped {
    use super::*;
    negative_control!(
        wsl_proof_six_says_how_a_side_failed,
        "stderr shown and dropped, as before",
        expected = "Invoke-Wsl drops the stderr it showed",
        check_proof_six_tail(&edited(&read(PS), "; Err = \"$e\" }", " }"))
    );
}

mod stdout_alone {
    use super::*;
    negative_control!(
        wsl_proof_six_says_how_a_side_failed,
        "the tail of stdout alone",
        expected = "Get-Tail isn't the last lines of stdout and stderr",
        check_proof_six_tail(&edited(
            &read(PS),
            " + @(\"$($Result.Err)\" -split \"`r?`n\")",
            ""
        ))
    );
}

// ---- Proof 3, the host over WSL's virtual switch -------------------------------------------------------------------

/// Proof 3 tries the host over WSL's virtual switch as well as over the LAN: the distro's default gateway (in NAT mode,
/// the host's side of the switch) and the host's vEthernet and WSL addresses, on a port the host listens on and on
/// SMB's 445. It passes only if the host listens at the gateway, so that the refusal shows something, and nothing there
/// is reached from inside.
fn check_proof_three(script: &str) {
    let three = &script[script.find("# Proof 3:").expect("no proof 3")
        ..script.find("# Proof 4:").expect("no proof 4")];
    assert!(
        three.contains(
            "$gateway = ((Invoke-Ci @('bash', $Inside, 'gateway')).Lines | Select-Object -Last 1)"
        ),
        "proof 3 doesn't ask inside for the gateway"
    );
    assert!(
        three.contains("$_.InterfaceAlias -like 'vEthernet*' -or $_.InterfaceAlias -like '*WSL*'"),
        "proof 3 leaves out the host's addresses on the virtual switch"
    );
    assert!(
        three.contains("foreach ($p in @($outPort, 445))"),
        "proof 3 doesn't try both the host's listener and SMB over the switch"
    );
    assert!(
        three.contains("$gatewayListens = $gateway -and (Test-Tcp $gateway $outPort)"),
        "proof 3 doesn't check the host listens at the gateway"
    );
    let ok = three
        .lines()
        .find(|l| l.trim_start().starts_with("$ok = "))
        .expect("proof 3 has no verdict");
    assert!(
        ok.contains(" -and $gatewayListens"),
        "proof 3 passes without the host listening at the gateway"
    );
    assert!(
        ok.contains(" -and -not $viaSwitch"),
        "proof 3 passes with the host reached over the virtual switch"
    );
    // WSL's localhost forwarding relays a connection from Windows to the listener, which then sees it come from
    // 127.0.0.1, as the one from inside does (run 43418528499): each beyond that one got in.
    assert!(
        three.contains(
            "$relayed = [Math]::Max(0, @($accepts | Where-Object { $_ -eq 'accepted 127.0.0.1' }).Count - \
             $(if ($selfIn) { 1 } else { 0 }))"
        ),
        "proof 3 doesn't count the connections relayed through localhost beyond the one from inside"
    );
    assert!(
        ok.contains(" -and -not $accepted -and $relayed -eq 0 -and "),
        "proof 3 passes with a connection relayed through localhost"
    );
    assert!(
        three.contains("relayed through localhost: $relayed;"),
        "proof 3's evidence doesn't show the connections relayed through localhost"
    );
}

#[test]
fn wsl_proof_three_tries_the_host_over_the_switch() {
    check_proof_three(&read(PS));
}

negative_control!(
    wsl_proof_three_tries_the_host_over_the_switch,
    "the switch's results ignored",
    expected = "proof 3 passes with the host reached over the virtual switch",
    check_proof_three(&edited(
        &read(PS),
        " -and $gatewayListens -and -not $viaSwitch\n",
        " -and $gatewayListens\n"
    ))
);

mod no_smb {
    use super::*;
    negative_control!(
        wsl_proof_three_tries_the_host_over_the_switch,
        "SMB left out",
        expected = "proof 3 doesn't try both the host's listener and SMB over the switch",
        check_proof_three(&edited(&read(PS), "@($outPort, 445)", "@($outPort)"))
    );
}

mod relayed_passes {
    use super::*;
    negative_control!(
        wsl_proof_three_tries_the_host_over_the_switch,
        "a connection relayed through localhost passes",
        expected = "proof 3 passes with a connection relayed through localhost",
        check_proof_three(&edited(&read(PS), " -and $relayed -eq 0", ""))
    );
}

mod relayed_unshown {
    use super::*;
    negative_control!(
        wsl_proof_three_tries_the_host_over_the_switch,
        "the relayed count left out of the evidence",
        expected = "proof 3's evidence doesn't show the connections relayed through localhost",
        check_proof_three(&edited(
            &read(PS),
            ", relayed through localhost: $relayed",
            ""
        ))
    );
}

mod relayed_uncounted {
    use super::*;
    negative_control!(
        wsl_proof_three_tries_the_host_over_the_switch,
        "every connection from 127.0.0.1 taken as the one from inside",
        expected = "proof 3 doesn't count the connections relayed through localhost beyond the one from inside",
        check_proof_three(&edited(
            &read(PS),
            ".Count - $(if ($selfIn) { 1 } else { 0 })",
            ".Count - $(if ($selfIn) { 9 } else { 9 })"
        ))
    );
}

mod gateway_unchecked {
    use super::*;
    negative_control!(
        wsl_proof_three_tries_the_host_over_the_switch,
        "a gateway the host may not listen at",
        expected = "proof 3 passes without the host listening at the gateway",
        check_proof_three(&edited(
            &read(PS),
            " -and $gatewayListens -and -not $viaSwitch\n",
            " -and -not $viaSwitch\n"
        ))
    );
}

/// `inside.sh gateway`, run on route tables written as the kernel writes /proc/net/route.
#[cfg(unix)]
mod gateway {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use validation::spawn::Spawn;

    use crate::scratch::Scratch;

    const HEAD: &str = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n";

    /// `inside` saved and run as `inside.sh gateway <table>`: its exit code and output.
    fn gateway(tag: &str, inside: &str, table: &str) -> (Option<i32>, String) {
        let dir = Scratch::new(tag);
        std::fs::create_dir_all(&*dir).expect("scratch");
        let script = dir.join("inside.sh");
        let route = dir.join("route");
        std::fs::write(&script, inside).expect("scratch");
        std::fs::write(&route, format!("{HEAD}{table}")).expect("scratch");
        let out = Command::new("bash")
            .arg(&script)
            .arg("gateway")
            .arg(&route)
            .output_within(Duration::from_secs(60))
            .expect("bash ran");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).trim().to_owned(),
        )
    }

    /// The first default route with RTF_GATEWAY (2) set gives its gateway, read little-endian; a default route without
    /// the flag, and a route that isn't the default, are passed over; with none, it exits 1 naming why.
    fn check_gateway(tag: &str, inside: &str) {
        let tables = [
            // eth0's default, through 172.29.112.1 (WSL's NAT switch), after a route to 10.0.0.0/8 through 10.0.0.1.
            (
                "eth0\t0000000A\t0100000A\t0003\t0\t0\t0\t000000FF\neth0\t00000000\t01701DAC\t0003\t0\t0\t0\t00000000\n",
                (Some(0), "172.29.112.1"),
            ),
            // A default route without the flag (on-link), then one through 192.168.0.1 with it (flags 0x0002 alone).
            (
                "eth1\t00000000\t00000000\t0001\t0\t0\t0\t00000000\neth0\t00000000\t0100A8C0\t0002\t0\t0\t0\t00000000\n",
                (Some(0), "192.168.0.1"),
            ),
        ];
        for (n, (table, (code, out))) in tables.iter().enumerate() {
            let got = gateway(&format!("{tag}_{n}"), inside, table);
            assert_eq!(
                got,
                (*code, (*out).to_owned()),
                "inside.sh gateway misreads the table"
            );
        }
        let none = gateway(
            &format!("{tag}_none"),
            inside,
            "eth0\t00000000\t0100A8C0\t0001\t0\t0\t0\t00000000\n",
        );
        assert!(
            none.0 == Some(1)
                && none
                    .1
                    .starts_with("gateway: no default route through a gateway"),
            "inside.sh gateway names a gateway with no default route through one: {none:?}"
        );
    }

    #[test]
    fn wsl_inside_reads_the_default_gateway() {
        check_gateway("wsl_gateway", &read(INSIDE));
    }

    negative_control!(
        wsl_inside_reads_the_default_gateway,
        "the gateway read big-endian",
        expected = "inside.sh gateway misreads the table",
        check_gateway(
            "wsl_gateway_endian",
            &edited(&read(INSIDE), "struct.pack(\"<L\"", "struct.pack(\">L\"")
        )
    );

    mod flag_ignored {
        use super::*;
        negative_control!(
            wsl_inside_reads_the_default_gateway,
            "the gateway flag ignored",
            expected = "inside.sh gateway misreads the table",
            check_gateway(
                "wsl_gateway_flag",
                &edited(&read(INSIDE), " and int(row[3], 16) & 2", "")
            )
        );
    }
}

// ---- Proof 4 ------------------------------------------------------------------------------------------------------

/// Proof 4 runs `contain.sh check` in its unprivileged mode inside, and passes only if the check passed, the ci process
/// outside its namespace was seen before and is alive after, the runner agent was seen, and no gh-runner process on
/// Windows was lost.
fn check_proof_four(script: &str, inside: &str) {
    assert!(
        inside.contains(
            "  check) PRIN_CONTAIN=unprivileged bash \"$repo/scripts/contain.sh\" check ;;"
        ),
        "inside.sh check isn't contain.sh's unprivileged check"
    );
    assert!(
        script.contains(
            "$ok = $check.Code -eq 0 -and $seen -and $ciAlive -and $agent.Count -ge 2 -and -not $lost -and $winAlive"
        ),
        "proof 4 passes without every survivor"
    );
    assert!(
        script.contains("$_.Name -in 'Runner.Listener.exe', 'Runner.Worker.exe'"),
        "proof 4 doesn't look for the runner agent"
    );
    let before = script
        .find("$before = @(Get-RunnerProcesses)")
        .expect("no snapshot before");
    let check = script
        .find("$check = Invoke-Ci @('bash', $Inside, 'check')")
        .expect("no check");
    let after = script
        .find("$after = @(Get-RunnerProcesses)")
        .expect("no snapshot after");
    assert!(
        before < check && check < after,
        "proof 4's snapshots don't bracket the check"
    );
}

#[test]
fn wsl_proof_four_needs_every_survivor() {
    check_proof_four(&read(PS), &read(INSIDE));
}

negative_control!(
    wsl_proof_four_needs_every_survivor,
    "Windows' losses ignored",
    expected = "proof 4 passes without every survivor",
    check_proof_four(
        &edited(
            &read(PS),
            " -and -not $lost -and $winAlive",
            " -and $winAlive"
        ),
        &read(INSIDE)
    )
);

mod strict_inside {
    use super::*;
    negative_control!(
        wsl_proof_four_needs_every_survivor,
        "the strict mode inside, which needs sudo",
        expected = "inside.sh check isn't contain.sh's unprivileged check",
        check_proof_four(
            &read(PS),
            &edited(
                &read(INSIDE),
                "PRIN_CONTAIN=unprivileged bash \"$repo/scripts/contain.sh\" check ;;",
                "PRIN_CONTAIN=required bash \"$repo/scripts/contain.sh\" check ;;"
            )
        )
    );
}

// ---- Proof 6's hosted side, on lavapipe ----------------------------------------------------------------------------

/// The hosted `wsl-parity` job installs the lavapipe `ci.yml`'s test jobs install, the same packages, before proof 6's
/// command: kernel's GPU harness tests run on the Vulkan `inside.sh` selects, which the image has (run 43418528499:
/// without it, 13 GPU tests failed there and passed in the distro).
fn check_parity_lavapipe(workflow: &str, ci: &str) {
    let (installs, _) = hosted_vulkan(ci);
    let hosted = installs
        .first()
        .unwrap_or_else(|| panic!("{CI} installs no lavapipe"));
    let doc = parse(workflow);
    let steps = doc["jobs"]["wsl-parity"]["steps"]
        .as_vec()
        .expect("wsl-parity has steps");
    let proof = steps
        .iter()
        .position(|s| s["name"].as_str() == Some("Proof 6, the hosted side"))
        .expect("wsl-parity runs no proof 6");
    let install = steps.iter().position(|s| {
        s["name"].as_str().unwrap_or("").contains("lavapipe")
            && s["run"].as_str().unwrap_or("").contains("apt-get install")
    });
    let Some(install) = install else {
        panic!("wsl-parity installs no lavapipe");
    };
    assert!(
        install < proof,
        "wsl-parity installs lavapipe after proof 6 has run"
    );
    let run = steps[install]["run"].as_str().unwrap_or("");
    let mut packages: Vec<String> = run
        .split_once("apt-get install")
        .map_or("", |(_, rest)| rest)
        .split_whitespace()
        .filter(|w| !w.starts_with('-'))
        .map(str::to_owned)
        .collect();
    packages.sort();
    assert_eq!(
        &packages, hosted,
        "wsl-parity installs other packages than {CI}'s lavapipe steps"
    );
}

#[test]
fn wsl_parity_runs_on_the_hosted_jobs_lavapipe() {
    check_parity_lavapipe(&read(WORKFLOW), &read(CI));
}

/// The workflow with `from`, in the `wsl-parity` job alone, replaced by `to`.
#[cfg(feature = "controls")]
fn parity_job_edited(from: &str, to: &str) -> String {
    let workflow = read(WORKFLOW);
    let start = workflow
        .find("\n  wsl-parity:\n")
        .expect("no wsl-parity job");
    let end = start
        + 1
        + workflow[start + 1..]
            .find("\n  wsl-register:")
            .expect("no wsl-register job");
    format!(
        "{}{}{}",
        &workflow[..start],
        edited(&workflow[start..end], from, to),
        &workflow[end..]
    )
}

negative_control!(
    wsl_parity_runs_on_the_hosted_jobs_lavapipe,
    "the job as run 43418528499 ran it, with no lavapipe",
    expected = "wsl-parity installs no lavapipe",
    check_parity_lavapipe(
        &parity_job_edited(
            "      - name: Install Mesa (lavapipe)\n",
            "      - name: Install Mesa\n"
        ),
        &read(CI)
    )
);

mod parity_lavapipe_late {
    use super::*;
    negative_control!(
        wsl_parity_runs_on_the_hosted_jobs_lavapipe,
        "lavapipe installed once the tests have run",
        expected = "wsl-parity installs lavapipe after proof 6 has run",
        {
            // Proof 6's step moved to before the cache's restore.
            const PROOF: &str = "      - name: Proof 6, the hosted side\n        run: |\n          mkdir -p \"$RUNNER_TEMP/wsl-parity\"\n          bash scripts/wsl/inside.sh parity \"$WSL_PARITY_COMMIT\" > \"$RUNNER_TEMP/wsl-parity/hosted.tsv\"\n";
            let moved = parity_job_edited(PROOF, "");
            let moved = moved.replacen(
                "          tool: cargo-nextest@0.9.146\n      - name: Restore the Mesa packages\n",
                &format!("          tool: cargo-nextest@0.9.146\n{PROOF}      - name: Restore the Mesa packages\n"),
                1,
            );
            check_parity_lavapipe(&moved, &read(CI));
        }
    );
}

mod parity_lavapipe_other {
    use super::*;
    negative_control!(
        wsl_parity_runs_on_the_hosted_jobs_lavapipe,
        "a package list other than the hosted jobs'",
        expected = "wsl-parity installs other packages than",
        check_parity_lavapipe(
            &parity_job_edited(
                "apt-get install -y mesa-vulkan-drivers -o",
                "apt-get install -y mesa-vulkan-drivers vulkan-tools -o"
            ),
            &read(CI)
        )
    );
}

// ---- Proof 6's comparison -----------------------------------------------------------------------------------------

#[cfg(unix)]
mod parity {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use validation::spawn::Spawn;

    use crate::scratch::Scratch;

    /// `parity.py <args>`, with `stdin`'s file on its stdin: through `sh`, as `output_within` closes the child's own.
    fn parity(args: &[&str], stdin: Option<&Path>) -> (Option<i32>, String) {
        let mut command = Command::new("sh");
        command
            .args(["-c", "f=$1; shift; if [ -n \"$f\" ]; then exec python3 \"$@\" < \"$f\"; else exec python3 \"$@\"; fi", "parity"])
            .arg(stdin.map_or_else(String::new, |p| p.display().to_string()))
            .arg(root().join("scripts/wsl/parity.py"))
            .args(args);
        let out = command
            .output_within(Duration::from_secs(60))
            .expect("python3 ran");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }

    const EVENTS: &str = concat!(
        "{\"type\":\"suite\",\"event\":\"started\",\"test_count\":3}\n",
        "{\"type\":\"test\",\"event\":\"started\",\"name\":\"kernel::a$one\"}\n",
        "{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::a$one\",\"exec_time\":0.5}\n",
        "{\"type\":\"test\",\"event\":\"failed\",\"name\":\"kernel::a$two\",\"exec_time\":0.25}\n",
        "not json\n",
        "{\"type\":\"test\",\"event\":\"ignored\",\"name\":\"kernel::b$three\"}\n",
    );

    /// A side's summary of `events`, with `wall` and `exit`, written to `dir/name`.
    fn summary(dir: &Path, name: &str, events: &str, wall: &str, exit: &str) -> PathBuf {
        let input = dir.join(format!("{name}.json"));
        std::fs::write(&input, events).expect("scratch");
        let (code, out) = parity(&["summarize", wall, exit], Some(&input));
        assert_eq!(code, Some(0), "summarize failed");
        let path = dir.join(name);
        std::fs::write(&path, out).expect("scratch");
        path
    }

    /// Two sides compared: identical outcomes and exits pass, whatever the wall times; a test that differs, one side's
    /// missing test, an empty run or a different exit fails.
    fn check_compare(other_events: &str, other_exit: &str, expect_same: bool, tag: &str) {
        let dir = Scratch::new(tag);
        std::fs::create_dir_all(&*dir).expect("scratch");
        let one = summary(&dir, "one", EVENTS, "100", "100");
        assert_eq!(
            std::fs::read_to_string(&one).expect("summary"),
            "# wall 100 exit 100 tests 3\nkernel::a$one\tok\t0.5\nkernel::a$two\tfailed\t0.25\nkernel::b$three\tignored\t0\n",
            "the summary isn't each test's last outcome, sorted"
        );
        let other = summary(&dir, "other", other_events, "300", other_exit);
        let (code, out) = parity(
            &[
                "compare",
                one.to_str().expect("utf-8"),
                other.to_str().expect("utf-8"),
            ],
            None,
        );
        assert_eq!(
            code == Some(0),
            expect_same,
            "the comparison's verdict is wrong: {out}"
        );
        assert!(
            out.contains("wall 100 s") && out.contains("wall 300 s"),
            "a wall time isn't reported: {out}"
        );
    }

    #[test]
    fn wsl_parity_compares_test_by_test() {
        check_compare(EVENTS, "100", true, "wsl_parity_same");
        check_compare(
            &EVENTS.replace("\"failed\"", "\"ok\""),
            "100",
            false,
            "wsl_parity_outcome",
        );
        check_compare(
            &EVENTS.replace("kernel::b$three", "kernel::b$four"),
            "100",
            false,
            "wsl_parity_missing",
        );
        check_compare(EVENTS, "0", false, "wsl_parity_exit");
    }

    negative_control!(
        wsl_parity_compares_test_by_test,
        "a test that passes on one side and fails on the other, taken as the same",
        expected = "the comparison's verdict is wrong",
        check_compare(
            &EVENTS.replace("\"failed\"", "\"ok\""),
            "100",
            true,
            "wsl_parity_control"
        )
    );

    /// Nothing ran on either side: no evidence, so not identical.
    fn check_empty(tag: &str, events: &str) {
        let dir = Scratch::new(tag);
        std::fs::create_dir_all(&*dir).expect("scratch");
        let one = summary(&dir, "one", events, "1", "0");
        let other = summary(&dir, "other", events, "1", "0");
        let (code, out) = parity(
            &[
                "compare",
                one.to_str().expect("utf-8"),
                other.to_str().expect("utf-8"),
            ],
            None,
        );
        assert!(
            code == Some(1) && out.contains("no test ran"),
            "an empty run passed: {out}"
        );
    }

    #[test]
    fn wsl_parity_refuses_an_empty_run() {
        check_empty("wsl_parity_empty", "");
    }

    mod empty {
        use super::*;
        negative_control!(
            wsl_parity_refuses_an_empty_run,
            "a run with tests",
            expected = "an empty run passed",
            check_empty("wsl_parity_empty_control", EVENTS)
        );
    }

    /// `inside` saved as `<scratch>/scripts/wsl/inside.sh`, outside any git repository and with HOME the scratch, and
    /// run as `inside.sh parity <commit>`: git fails, and its message must reach stderr, never stdout, which the hosted
    /// job writes to the results `parity.py compare` reads.
    fn check_parity_streams(tag: &str, inside: &str) {
        let dir = Scratch::new(tag);
        let wsl = dir.join("scripts/wsl");
        std::fs::create_dir_all(&wsl).expect("scratch");
        std::fs::write(wsl.join("inside.sh"), inside).expect("scratch");
        let out = Command::new("bash")
            .arg(wsl.join("inside.sh"))
            .args(["parity", "2a69136e"])
            .env("HOME", &*dir)
            .env("GIT_CEILING_DIRECTORIES", &*dir)
            .output_within(Duration::from_secs(60))
            .expect("bash ran");
        let (stdout, stderr) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        assert_ne!(
            out.status.code(),
            Some(0),
            "parity passed with no repository"
        );
        assert!(
            stdout.is_empty() && !stderr.is_empty(),
            "inside.sh parity's stderr reaches its stdout: stdout '{stdout}', stderr '{stderr}'"
        );
    }

    #[test]
    fn wsl_parity_keeps_stderr_off_its_results() {
        check_parity_streams("wsl_parity_streams", &read(INSIDE));
    }

    /// `inside` and `parity` saved as `<scratch>/scripts/wsl/inside.sh` and `parity.py`, with HOME the scratch and this
    /// side's results in `~/work/parity.tsv`; `inside.sh compare` run with the other side's results, a byte-order mark
    /// first, on its stdin, as wsl.ps1 sent them in run 43418528499. The arm hands parity.py the stdin it already has,
    /// `-`, never `/dev/stdin`, which `wsl.exe -u ci` refuses to open again (EACCES there); parity.py drops the mark,
    /// in a summary and in nextest's JSON alike.
    fn check_marked_stdin(tag: &str, inside: &str, parity_py: &str) {
        const BOM: &str = "\u{feff}";
        let arm = inside
            .lines()
            .find(|l| l.trim_start().starts_with("compare)"))
            .expect("inside.sh has no compare arm");
        assert!(
            arm.contains("parity.py\" compare - ") && !inside.contains("/dev/stdin"),
            "inside.sh compare opens /dev/stdin again: {arm}"
        );
        let dir = Scratch::new(tag);
        let wsl = dir.join("scripts/wsl");
        std::fs::create_dir_all(&wsl).expect("scratch");
        std::fs::create_dir_all(dir.join("work")).expect("scratch");
        std::fs::write(wsl.join("inside.sh"), inside).expect("scratch");
        std::fs::write(wsl.join("parity.py"), parity_py).expect("scratch");
        let summary = "# wall 100 exit 0 tests 2\nkernel::a\tok\t0.5\nkernel::b\tignored\t0\n";
        std::fs::write(dir.join("work/parity.tsv"), summary).expect("scratch");
        let other = dir.join("other.tsv");
        std::fs::write(&other, format!("{BOM}{summary}")).expect("scratch");
        let out = Command::new("sh")
            .args(["-c", "exec bash \"$1\" compare < \"$2\"", "compare"])
            .arg(wsl.join("inside.sh"))
            .arg(&other)
            .env("HOME", &*dir)
            .output_within(Duration::from_secs(60))
            .expect("bash ran");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.code() == Some(0) && stdout.contains("parity: identical, test by test"),
            "parity.py doesn't read a summary sent with a byte-order mark: {stdout}"
        );
        let events = dir.join("events.json");
        std::fs::write(
            &events,
            format!("{BOM}{{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::a\"}}\n"),
        )
        .expect("scratch");
        let out = Command::new("sh")
            .args([
                "-c",
                "exec python3 \"$1\" summarize 1 0 < \"$2\"",
                "summarize",
            ])
            .arg(wsl.join("parity.py"))
            .arg(&events)
            .output_within(Duration::from_secs(60))
            .expect("python3 ran");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "# wall 1 exit 0 tests 1\nkernel::a\tok\t0\n",
            "parity.py drops nextest's first event after a byte-order mark"
        );
    }

    #[test]
    fn wsl_parity_reads_its_stdin_past_a_byte_order_mark() {
        check_marked_stdin(
            "wsl_parity_marked",
            &read(INSIDE),
            &read("scripts/wsl/parity.py"),
        );
    }

    mod marked {
        use super::*;
        negative_control!(
            wsl_parity_reads_its_stdin_past_a_byte_order_mark,
            "the mark read as text",
            expected = "parity.py doesn't read a summary sent with a byte-order mark",
            check_marked_stdin(
                "wsl_parity_marked_control",
                &read(INSIDE),
                &read("scripts/wsl/parity.py").replace("\"utf-8-sig\"", "\"utf-8\"")
            )
        );
    }

    mod marked_events {
        use super::*;
        negative_control!(
            wsl_parity_reads_its_stdin_past_a_byte_order_mark,
            "nextest's events read from the raw stdin",
            expected = "parity.py drops nextest's first event after a byte-order mark",
            check_marked_stdin(
                "wsl_parity_marked_events_control",
                &read(INSIDE),
                &edited(
                    &read("scripts/wsl/parity.py"),
                    "summarize(argv[2], argv[3], stdin())",
                    "summarize(argv[2], argv[3], sys.stdin)"
                )
            )
        );
    }

    mod reopened {
        use super::*;
        negative_control!(
            wsl_parity_reads_its_stdin_past_a_byte_order_mark,
            "the arm as run 43418528499 ran it",
            expected = "inside.sh compare opens /dev/stdin again",
            check_marked_stdin(
                "wsl_parity_reopened_control",
                &edited(
                    &read(INSIDE),
                    "parity.py\" compare - ",
                    "parity.py\" compare /dev/stdin "
                ),
                &read("scripts/wsl/parity.py")
            )
        );
    }

    mod merged {
        use super::*;
        negative_control!(
            wsl_parity_keeps_stderr_off_its_results,
            "parity's stderr merged into stdout",
            expected = "inside.sh parity's stderr reaches its stdout",
            check_parity_streams(
                "wsl_parity_streams_control",
                &edited(&read(INSIDE), "  results | parity) ;;", "  results) ;;")
            )
        );
    }

    /// The first program named `name` on this test's PATH.
    fn on_path(name: &str) -> PathBuf {
        let path = std::env::var_os("PATH").expect("PATH is set");
        std::env::split_paths(&path)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no {name} on PATH"))
    }

    /// `inside` and parity.py saved in a scratch git repository at `scripts/wsl/`, committed, and run as
    /// `inside.sh parity <that commit>` with HOME a scratch home whose `.cargo/bin` holds this test's git and python3
    /// and a `cargo` that prints `events` (nextest's libtest JSON) on stdout, `error: <log>` on stderr, and exits
    /// `exit`. Returns the verb's exit, its stdout (the results) and its stderr.
    fn run_parity(
        tag: &str,
        inside: &str,
        events: &str,
        exit: i32,
    ) -> (Option<i32>, String, String) {
        use std::os::unix::fs::PermissionsExt;
        let dir = Scratch::new(tag);
        let repo = dir.join("repo");
        let wsl = repo.join("scripts/wsl");
        let bin = dir.join("home/.cargo/bin");
        std::fs::create_dir_all(&wsl).expect("scratch");
        std::fs::create_dir_all(&bin).expect("scratch");
        std::fs::write(wsl.join("inside.sh"), inside).expect("scratch");
        std::fs::write(wsl.join("parity.py"), read("scripts/wsl/parity.py")).expect("scratch");
        for tool in ["git", "python3"] {
            std::os::unix::fs::symlink(on_path(tool), bin.join(tool)).expect("scratch");
        }
        std::fs::write(dir.join("events.json"), events).expect("scratch");
        let cargo = bin.join("cargo");
        std::fs::write(
            &cargo,
            format!(
                "#!/bin/sh\ncat '{}'\necho 'error: the end of cargo'\"'\"'s log' >&2\nexit {exit}\n",
                dir.join("events.json").display()
            ),
        )
        .expect("scratch");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("scratch");
        let git = |args: &[&str]| {
            let out = Command::new(bin.join("git"))
                .args([
                    "-c",
                    "user.name=wsl",
                    "-c",
                    "user.email=wsl@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(["-C"])
                .arg(&repo)
                .args(args)
                .env("HOME", dir.join("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output_within(Duration::from_secs(60))
                .expect("git ran");
            assert!(out.status.success(), "scratch git {args:?}: {out:?}");
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "scratch"]);
        let commit = git(&["rev-parse", "HEAD"]);
        let out = Command::new("bash")
            .arg(wsl.join("inside.sh"))
            .args(["parity", &commit])
            .env("HOME", dir.join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output_within(Duration::from_secs(60))
            .expect("bash ran");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    const TWO_PASS: &str = concat!(
        "{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::a\"}\n",
        "{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::b\"}\n",
    );
    const ONE_FAILS: &str = concat!(
        "{\"type\":\"test\",\"event\":\"ok\",\"name\":\"kernel::a\"}\n",
        "{\"type\":\"test\",\"event\":\"failed\",\"name\":\"kernel::b\"}\n",
    );

    /// `inside.sh parity` fails when no test ran, whatever cargo's exit (run 43603140749: exit 101 on an invalid cargo
    /// config, 0 tests, and the verb passed), and prints the end of cargo's log on stderr then and whenever cargo exits
    /// non-zero; with tests run, a non-zero exit is left to `compare`, which compares the exits and each test's outcome.
    fn check_parity_fails_empty(tag: &str, inside: &str) {
        let log = "error: the end of cargo's log";
        let (code, out, err) = run_parity(&format!("{tag}_ok"), inside, TWO_PASS, 0);
        assert!(
            code == Some(0) && out.starts_with("# wall ") && out.contains(" exit 0 tests 2\n"),
            "inside.sh parity fails a run whose two tests passed: {code:?}\n{out}\n{err}"
        );
        assert!(
            !err.contains(log),
            "inside.sh parity prints cargo's log when cargo passed: {err}"
        );
        for (case, events, exit) in [("broken", "", 101), ("none", "", 0)] {
            let (code, out, err) = run_parity(&format!("{tag}_{case}"), inside, events, exit);
            assert!(
                code.is_some_and(|c| c != 0) && err.contains("no test ran"),
                "inside.sh parity passes with no test run (cargo exit {exit}): {code:?}\n{out}\n{err}"
            );
            assert!(
                err.contains(log) && !out.contains(log),
                "inside.sh parity doesn't print the end of cargo's log on stderr when no test ran (cargo exit {exit}):\n{out}\n{err}"
            );
        }
        let (code, out, err) = run_parity(&format!("{tag}_failed"), inside, ONE_FAILS, 100);
        assert!(
            code == Some(0) && out.contains(" exit 100 tests 2\n") && out.contains("kernel::b\tfailed"),
            "inside.sh parity doesn't hand a run with a failed test to compare: {code:?}\n{out}\n{err}"
        );
        assert!(
            err.contains(log) && !out.contains(log),
            "inside.sh parity doesn't print the end of cargo's log on stderr when cargo exits non-zero:\n{out}\n{err}"
        );
    }

    #[test]
    fn wsl_parity_fails_when_no_test_ran() {
        check_parity_fails_empty("wsl_parity_empty_verb", &read(INSIDE));
    }

    mod empty_passes {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "an empty run handed to compare, as run 43603140749's was",
            expected = "inside.sh parity passes with no test run (cargo exit 101)",
            check_parity_fails_empty(
                "wsl_parity_empty_verb_control",
                &edited(
                    &read(INSIDE),
                    "    echo \"parity: FAIL: no test ran, so there is nothing to compare\" >&2\n    return 1\n",
                    "    echo \"parity: no test ran\" >&2\n"
                )
            )
        );
    }

    mod empty_on_exit_only {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "the failure keyed on cargo's exit, not on the tests run",
            expected = "inside.sh parity passes with no test run (cargo exit 0)",
            check_parity_fails_empty(
                "wsl_parity_exit_only_control",
                &edited(
                    &read(INSIDE),
                    "  if [ \"${tests:-0}\" -eq 0 ]; then\n    echo \"parity: FAIL",
                    "  if [ \"$code\" -ne 0 ] && [ \"${tests:-0}\" -eq 0 ]; then\n    echo \"parity: FAIL"
                )
            )
        );
    }

    mod log_unprinted {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "cargo's log left unprinted",
            expected = "inside.sh parity doesn't print the end of cargo's log on stderr when no test ran (cargo exit 101)",
            check_parity_fails_empty(
                "wsl_parity_log_control",
                &edited(&read(INSIDE), "    tail -n 40 \"$work/parity.log\" >&2\n", "")
            )
        );
    }

    mod log_into_results {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "cargo's log written into the results",
            expected = "inside.sh parity doesn't print the end of cargo's log on stderr when no test ran (cargo exit 101)",
            check_parity_fails_empty(
                "wsl_parity_log_stdout_control",
                &edited(&read(INSIDE), "    tail -n 40 \"$work/parity.log\" >&2\n", "    tail -n 40 \"$work/parity.log\"\n")
            )
        );
    }

    mod log_only_when_empty {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "cargo's log printed only when no test ran",
            expected = "inside.sh parity doesn't print the end of cargo's log on stderr when cargo exits non-zero",
            check_parity_fails_empty(
                "wsl_parity_log_empty_only_control",
                &edited(
                    &read(INSIDE),
                    "  if [ \"$code\" -ne 0 ] || [ \"${tests:-0}\" -eq 0 ]; then\n",
                    "  if [ \"${tests:-0}\" -eq 0 ]; then\n"
                )
            )
        );
    }

    mod failed_run_refused {
        use super::*;
        negative_control!(
            wsl_parity_fails_when_no_test_ran,
            "a run with a failed test refused before compare",
            expected = "inside.sh parity doesn't hand a run with a failed test to compare",
            check_parity_fails_empty(
                "wsl_parity_failed_refused_control",
                &edited(
                    &read(INSIDE),
                    "  if [ \"${tests:-0}\" -eq 0 ]; then\n    echo \"parity: FAIL",
                    "  if [ \"$code\" -ne 0 ] || [ \"${tests:-0}\" -eq 0 ]; then\n    echo \"parity: FAIL"
                )
            )
        );
    }
}
