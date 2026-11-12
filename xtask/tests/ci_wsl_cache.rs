//! The PC's cache of the WSL2 image (the owner's words of 12 Nov 2026: "Cache the verified image on E:, filed by
//! SHA-256. Register from the cache when the expected hash is present; download only otherwise. Always check length +
//! SHA-256 against the current run's expected values before registering; never trust a cached file by name. Keep at
//! most the current and previous image."), within the runner's rules: no secrets, no new trigger, E: only.
//!
//! The image's bytes differ from build to build of one recipe (run 43312851701), so `wsl-register` files it by its
//! recipe key, with the length and SHA-256 it had when built, and checks both every time before it imports it. Off
//! Windows there is no PowerShell, so, as `wsl.rs` does, these read `.github/scripts/wsl.ps1` and `win-gpu.yml`; the
//! workflow's step conditions are evaluated by a small model of GitHub Actions' (`continue-on-error`, `outcome`, the
//! implicit `success()`), through a hit, a miss and a missing folder. No `pull_request` trigger reaches the job:
//! `ci_self_hosted.rs`'s `self_hosted_jobs_have_only_the_three_triggers` holds that for every self-hosted job. Each
//! test has its negative controls (R-176).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use validation::negative_control;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOW: &str = ".github/workflows/win-gpu.yml";
const PS: &str = ".github/scripts/wsl.ps1";
const BUILD: &str = "scripts/wsl/build-image.sh";
/// The folder the owner makes on the PC, writable by gh-runner.
const CACHE_ROOT: &str = "$CacheRoot = 'E:\\runner-tools\\wsl-cache'";
/// The lookup's step id, and what its outcome is on a hit.
const LOOKUP_ID: &str = "cache";

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

/// The text of the function `name`, from `function name` through its closing brace at the start of a line.
fn function<'a>(script: &'a str, name: &str) -> &'a str {
    let start = script
        .find(&format!("\nfunction {name}"))
        .unwrap_or_else(|| panic!("{PS} has no function {name}"));
    let rest = &script[start + 1..];
    &rest[..rest.find("\n}\n").expect("the function ends") + 2]
}

/// The statements of `body`, trimmed, without blank lines and comments.
fn statements(body: &str) -> Vec<&str> {
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

// ---- The job: a hit downloads nothing, a miss downloads, a missing folder stops it --------------------------------

/// A step's end in the model.
#[derive(Clone, Copy)]
struct State {
    outcome: &'static str,
    download_path: bool,
}

/// Evaluates an `if:` as GitHub Actions does for the terms `wsl-register` uses: `&&` of `steps.<id>.outcome` or
/// `.outputs.download-path` against a quoted literal, with the implicit `success()`. Anything else stops the test, so a
/// condition the model can't read is never taken as true.
fn holds(cond: Option<&str>, states: &HashMap<String, State>, failed: bool) -> bool {
    let Some(cond) = cond else {
        return !failed;
    };
    assert!(!cond.contains("||"), "the model reads only &&: {cond}");
    assert!(
        !cond.contains("()"),
        "the model reads no status function: {cond}"
    );
    let mut value = !failed;
    for term in cond.split("&&").map(str::trim) {
        let (lhs, rhs, equal) = if let Some((l, r)) = term.split_once("!=") {
            (l.trim(), r.trim(), false)
        } else if let Some((l, r)) = term.split_once("==") {
            (l.trim(), r.trim(), true)
        } else {
            panic!("the model can't read {term}")
        };
        let literal = rhs
            .strip_prefix('\'')
            .and_then(|r| r.strip_suffix('\''))
            .unwrap_or_else(|| panic!("the model reads only quoted literals: {rhs}"));
        let path: Vec<&str> = lhs.split('.').collect();
        assert_eq!(path[0], "steps", "the model reads only steps: {lhs}");
        let state = states.get(path[1]).copied().unwrap_or(State {
            outcome: "",
            download_path: false,
        });
        let actual = match &path[2..] {
            ["outcome"] => state.outcome.to_string(),
            ["outputs", "download-path"] if state.download_path => "D:\\x".to_string(),
            ["outputs", "download-path"] => String::new(),
            other => panic!("the model can't read steps.{}.{other:?}", path[1]),
        };
        value &= (actual == literal) == equal;
    }
    value
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Case {
    /// The folder is missing or can't be written.
    NoFolder,
    /// The lookup found this recipe's image, whole.
    Hit,
    /// The lookup didn't; the first download is whole.
    Miss,
}

#[derive(Debug, Default)]
struct Run {
    lookups: usize,
    downloads: usize,
    /// The registration ran, with WSL_CACHE_HIT as the step gives it.
    registered: Option<String>,
    /// The step that failed the job, by name.
    failed_at: Option<String>,
}

/// `wsl-register`'s steps, from the first after the checkout's record through the registration, in `case`.
fn simulate(workflow: &str, case: Case) -> Run {
    let doc = parse(workflow);
    let steps = doc["jobs"]["wsl-register"]["steps"]
        .as_vec()
        .expect("wsl-register's steps");
    let start = steps
        .iter()
        .position(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("win_measure.ps1 start"))
        })
        .expect("the record's start")
        + 1;
    let mut run = Run::default();
    let mut states: HashMap<String, State> = HashMap::new();
    let mut failed = false;
    for step in &steps[start..] {
        let script = step["run"].as_str().unwrap_or("");
        let name = step["name"].as_str().unwrap_or("").to_string();
        let runs = holds(step["if"].as_str(), &states, failed);
        if script.contains("wsl.ps1 register") {
            if runs {
                run.registered = Some(
                    step["env"]["WSL_CACHE_HIT"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                );
            }
            return run;
        }
        if !runs {
            if let Some(id) = step["id"].as_str() {
                states.insert(
                    id.into(),
                    State {
                        outcome: "skipped",
                        download_path: false,
                    },
                );
            }
            continue;
        }
        let downloads = step["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"));
        let (ok, path) = if script.contains("wsl.ps1 cache-ready") {
            (case != Case::NoFolder, false)
        } else if script.contains("wsl.ps1 cache-lookup") {
            run.lookups += 1;
            (case == Case::Hit, false)
        } else if downloads {
            run.downloads += 1;
            (true, true)
        } else if script.contains("::error::") && script.trim_end().ends_with("exit 1") {
            (false, false)
        } else {
            (true, false)
        };
        if let Some(id) = step["id"].as_str() {
            states.insert(
                id.into(),
                State {
                    outcome: if ok { "success" } else { "failure" },
                    download_path: path,
                },
            );
        }
        if !ok && !step["continue-on-error"].as_bool().unwrap_or(false) && !failed {
            failed = true;
            run.failed_at = Some(name);
        }
    }
    panic!("wsl-register has no step running wsl.ps1 register")
}

/// The lookup comes first, the folder's check before it; a hit downloads nothing and registers, told it was a hit; a
/// miss downloads and registers; a missing or unwritable folder stops the job at its check, before any lookup or
/// download.
fn check_paths(workflow: &str) {
    let hit = simulate(workflow, Case::Hit);
    assert_eq!(hit.downloads, 0, "a cache hit downloads the image: {hit:?}");
    assert!(
        hit.registered.is_some(),
        "a cache hit never reaches registration: {hit:?}"
    );
    let miss = simulate(workflow, Case::Miss);
    assert!(
        miss.lookups == 1 && miss.downloads >= 1 && miss.registered.is_some(),
        "a cache miss doesn't download and register: {miss:?}"
    );
    for run in [&hit, &miss] {
        assert_eq!(
            run.registered.as_deref(),
            Some("${{ steps.cache.outcome }}"),
            "register isn't told the lookup's outcome: {run:?}"
        );
    }
    let none = simulate(workflow, Case::NoFolder);
    assert!(
        none.registered.is_none() && none.downloads == 0 && none.lookups == 0,
        "a missing cache folder doesn't stop the job before the lookup: {none:?}"
    );
    assert!(
        none.failed_at
            .as_deref()
            .is_some_and(|n| n.contains("image cache")),
        "the job doesn't fail at the folder's check: {none:?}"
    );
    let doc = parse(workflow);
    let lookup = doc["jobs"]["wsl-register"]["steps"]
        .as_vec()
        .expect("steps")
        .iter()
        .find(|s| s["id"].as_str() == Some(LOOKUP_ID))
        .expect("the lookup's step");
    assert!(
        lookup["run"]
            .as_str()
            .is_some_and(|r| r.contains("wsl.ps1 cache-lookup")),
        "the step with id {LOOKUP_ID} isn't the cache lookup"
    );
}

#[test]
fn wsl_cache_hit_downloads_nothing_and_a_miss_downloads() {
    check_paths(&read(WORKFLOW));
}

negative_control!(
    wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
    "the first try of the image runs on a hit",
    expected = "a cache hit downloads the image",
    check_paths(&edited(
        &read(WORKFLOW),
        "        timeout-minutes: 25\n        if: steps.cache.outcome != 'success'\n",
        "        timeout-minutes: 25\n"
    ))
);

#[cfg(feature = "controls")]
mod retry_on_a_hit {
    use super::*;
    // The second try's condition loses its cache term: on a hit the first check is skipped, so the retry runs.
    negative_control!(
        wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
        "the second try of the image runs on a hit",
        expected = "a cache hit downloads the image",
        check_paths(&edited(
            &read(WORKFLOW),
            "        if: steps.image-check.outcome != 'success'\n          && steps.cache.outcome != 'success'\n",
            "        if: steps.image-check.outcome != 'success'\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod stop_on_a_hit {
    use super::*;
    // The stop step loses its cache term: on a hit neither check ran, so it fails the job.
    negative_control!(
        wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
        "the stop step fires on a hit",
        expected = "a cache hit never reaches registration",
        check_paths(&edited(
            &read(WORKFLOW),
            " && steps.image-check-again.outcome != 'success' && steps.cache.outcome != 'success'\n",
            " && steps.image-check-again.outcome != 'success'\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod miss_ends_job {
    use super::*;
    // The lookup without continue-on-error: a miss is a failed step, and the job ends there.
    negative_control!(
        wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
        "a miss that ends the job",
        expected = "a cache miss doesn't download and register",
        check_paths(&edited(
            &read(WORKFLOW),
            "        id: cache\n        timeout-minutes: 5\n        continue-on-error: true\n",
            "        id: cache\n        timeout-minutes: 5\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod hit_untold {
    use super::*;
    // Register isn't given the lookup's outcome, so it would take the download's folder on a hit.
    negative_control!(
        wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
        "register not told of the hit",
        expected = "register isn't told the lookup's outcome",
        check_paths(&edited(
            &read(WORKFLOW),
            "          WSL_CACHE_HIT: ${{ steps.cache.outcome }}\n",
            ""
        ))
    );
}

#[cfg(feature = "controls")]
mod folder_tolerated {
    use super::*;
    // The folder's check with continue-on-error: a missing folder goes on to the lookup.
    negative_control!(
        wsl_cache_hit_downloads_nothing_and_a_miss_downloads,
        "a missing folder tolerated",
        expected = "a missing cache folder doesn't stop the job before the lookup",
        check_paths(&edited(
            &read(WORKFLOW),
            "      - name: The image cache on E:, there and writable\n        run: |\n",
            "      - name: The image cache on E:, there and writable\n        continue-on-error: true\n        run: |\n"
        ))
    );
}

// ---- Never registered unchecked, never trusted by name --------------------------------------------------------------

/// `register` imports the cache's copy, whichever way it got there, and only after `Assert-Image` has checked its
/// length and SHA-256 against those recorded when it was built, and its manifest against the pin: `$image` is set once,
/// to the cache's folder for this recipe, and neither it nor `$rootfs` changes between the check and the import. A
/// miss's download is itself checked whole, and its manifest the pin, before `Save-Image` moves any of it; the lookup
/// reports a hit only after checking the cached copy. `Assert-Image`'s own comparisons are `wsl.rs`'s and
/// `qa_ci_win_download_retry.rs`'s to hold.
fn check_checked(script: &str) {
    let register = arm(script, "register");
    let import = register
        .find("& wsl.exe --import $Distro $Disk $rootfs --version 2")
        .expect("register imports nothing");
    let assigns = |var: &str| -> Vec<usize> {
        register
            .match_indices(&format!("{var} = "))
            .map(|(at, _)| at)
            .collect()
    };
    let image = assigns("$image");
    assert!(
        image.len() == 1
            && register[image[0]..].starts_with("$image = Join-Path $CacheRoot $key\n"),
        "register imports other than the cache's copy of this recipe"
    );
    let rootfs = assigns("$rootfs");
    assert!(
        rootfs.len() == 1
            && register[rootfs[0]..].starts_with("$rootfs = Join-Path $image 'rootfs.tar.gz'\n"),
        "register points the import elsewhere than the image it checks"
    );
    let save = register
        .find("Save-Image $env:WSL_IMAGE $image $pinned")
        .expect("register caches no download");
    let checked = register
        .rfind("\n        Assert-Image $image\n")
        .filter(|&c| c > save && c > rootfs[0] && c < import);
    let pinned = register
        .find("if ($hash -ne $pinned) { Fail")
        .filter(|&p| checked.is_some_and(|c| p > c) && p < import);
    assert!(
        checked.is_some() && pinned.is_some(),
        "register imports an image it hasn't checked after caching it"
    );
    let saving = function(script, "Save-Image");
    let first_write = ["Remove-Item", "New-Item", "Move-Item", "Rename-Item"]
        .iter()
        .filter_map(|w| saving.find(w))
        .min()
        .expect("Save-Image writes nothing");
    assert!(
        saving
            .find("    Assert-Image $From\n")
            .is_some_and(|a| a < first_write)
            && saving
                .find("if ($hash -ne $Pinned) { Fail")
                .is_some_and(|p| p < first_write),
        "Save-Image caches a download it hasn't checked"
    );
    let lookup = arm(script, "cache-lookup");
    let hit = lookup.rfind("exit 0").expect("the lookup reports no hit");
    assert!(
        lookup
            .find("        Assert-Image $entry\n")
            .is_some_and(|a| a < hit)
            && lookup.matches("exit 0").count() == 1,
        "cache-lookup reports a hit it hasn't checked"
    );
}

#[test]
fn wsl_cache_never_registers_an_unchecked_image() {
    check_checked(&read(PS));
}

negative_control!(
    wsl_cache_never_registers_an_unchecked_image,
    "the cache's copy imported unchecked",
    expected = "register imports an image it hasn't checked after caching it",
    check_checked(&edited(
        &read(PS),
        "        $manifest = Join-Path $image 'manifest.txt'\n        Assert-Image $image\n",
        "        $manifest = Join-Path $image 'manifest.txt'\n"
    ))
);

#[cfg(feature = "controls")]
mod import_elsewhere {
    use super::*;
    // After the check, the import is pointed at the download's folder.
    negative_control!(
        wsl_cache_never_registers_an_unchecked_image,
        "the import pointed at the download after the check",
        expected = "register points the import elsewhere than the image it checks",
        check_checked(&edited(
            &read(PS),
            "        Limit-Cache $key\n",
            "        Limit-Cache $key\n        $rootfs = Join-Path $env:WSL_IMAGE 'rootfs.tar.gz'\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod download_imported {
    use super::*;
    // The image is the download's on a miss, so the cache's copy is never what is imported.
    negative_control!(
        wsl_cache_never_registers_an_unchecked_image,
        "the download imported in place of the cache's copy",
        expected = "register imports other than the cache's copy of this recipe",
        check_checked(&edited(
            &read(PS),
            "            Save-Image $env:WSL_IMAGE $image $pinned\n",
            "            Save-Image $env:WSL_IMAGE $image $pinned\n            $image = $env:WSL_IMAGE\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod trusted_by_name {
    use super::*;
    // The lookup reports a hit because a folder of that name exists.
    negative_control!(
        wsl_cache_never_registers_an_unchecked_image,
        "a hit trusted by the folder's name",
        expected = "cache-lookup reports a hit it hasn't checked",
        check_checked(&edited(&read(PS), "        Assert-Image $entry\n", ""))
    );
}

#[cfg(feature = "controls")]
mod cached_unchecked {
    use super::*;
    // A download moved into the cache before it is checked.
    negative_control!(
        wsl_cache_never_registers_an_unchecked_image,
        "a download cached unchecked",
        expected = "Save-Image caches a download it hasn't checked",
        check_checked(&edited(&read(PS), "    Assert-Image $From\n", ""))
    );
}

// ---- The folder: the owner's, never made here, a clean failure without it -----------------------------------------

/// `Assert-Cache` fails, naming the folder, unless it is there and a folder; every verb and function of the cache runs
/// it before anything else; nothing in the script makes the folder; and `cache-ready` proves it writable with a probe
/// inside it. In the workflow, `cache-ready` is the first step after the record's start, with no condition and no
/// `continue-on-error`, so the job fails there and writes nothing anywhere else.
fn check_folder(script: &str, workflow: &str) {
    let assert_cache = statements(function(script, "Assert-Cache"));
    assert!(
        assert_cache.len() == 5
            && assert_cache[1]
                == "if (-not (Test-Path -LiteralPath $CacheRoot -PathType Container)) {"
            && assert_cache[2].starts_with("Fail \"the image cache $CacheRoot\\ doesn't exist:"),
        "Assert-Cache doesn't fail, naming the folder, when it is missing"
    );
    for line in script.lines() {
        let makes = ["New-Item", "CreateDirectory", "mkdir", " md "]
            .iter()
            .any(|m| line.contains(m));
        assert!(
            !(makes && line.contains("$CacheRoot")),
            "wsl.ps1 makes the cache's folder: {line}"
        );
    }
    let first = |body: &str| statements(body).get(1).copied().unwrap_or("").to_string();
    for name in ["Save-Image", "Limit-Cache"] {
        assert_eq!(
            first(function(script, name)),
            "Assert-Cache",
            "{name} doesn't check the folder first"
        );
    }
    for verb in ["cache-ready", "cache-lookup"] {
        assert_eq!(
            first(arm(script, verb)),
            "Assert-Cache",
            "wsl.ps1 {verb} doesn't check the folder first"
        );
    }
    let ready = statements(arm(script, "cache-ready"));
    assert!(
        ready.contains(&"$probe = Join-Path $CacheRoot \".write-probe-$PID\"")
            && ready.contains(&"[IO.File]::WriteAllText($probe, '')"),
        "cache-ready doesn't prove the folder writable"
    );
    let doc = parse(workflow);
    let steps = doc["jobs"]["wsl-register"]["steps"]
        .as_vec()
        .expect("steps");
    let start = steps
        .iter()
        .position(|s| {
            s["run"]
                .as_str()
                .is_some_and(|r| r.contains("win_measure.ps1 start"))
        })
        .expect("the record's start");
    let ready = &steps[start + 1];
    assert!(
        ready["run"]
            .as_str()
            .is_some_and(|r| r.contains("\\wsl.ps1 cache-ready\n"))
            && ready["if"].is_badvalue()
            && ready["continue-on-error"].is_badvalue(),
        "wsl-register doesn't stop at the folder's check before anything else"
    );
}

#[test]
fn wsl_cache_fails_cleanly_without_its_folder() {
    check_folder(&read(PS), &read(WORKFLOW));
}

negative_control!(
    wsl_cache_fails_cleanly_without_its_folder,
    "the folder made when missing",
    expected = "wsl.ps1 makes the cache's folder",
    check_folder(
        &edited(
            &read(PS),
            "    Assert-Cache\n    Assert-Image $From\n",
            "    New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null\n    Assert-Cache\n    Assert-Image $From\n"
        ),
        &read(WORKFLOW)
    )
);

#[cfg(feature = "controls")]
mod save_unchecked {
    use super::*;
    // Save-Image writes without the folder's check; New-Item would make the missing folder's path.
    negative_control!(
        wsl_cache_fails_cleanly_without_its_folder,
        "a save that doesn't check the folder",
        expected = "Save-Image doesn't check the folder first",
        check_folder(
            &edited(
                &read(PS),
                "    Assert-Cache\n    Assert-Image $From\n",
                "    Assert-Image $From\n"
            ),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod silent_when_missing {
    use super::*;
    // A missing folder passes the check.
    negative_control!(
        wsl_cache_fails_cleanly_without_its_folder,
        "a check that passes a missing folder",
        expected = "Assert-Cache doesn't fail, naming the folder, when it is missing",
        check_folder(
            &edited(
                &read(PS),
                "    if (-not (Test-Path -LiteralPath $CacheRoot -PathType Container)) {\n",
                "    if ($false) {\n"
            ),
            &read(WORKFLOW)
        )
    );
}

#[cfg(feature = "controls")]
mod checked_late {
    use super::*;
    // The folder's check moved after the lookup.
    negative_control!(
        wsl_cache_fails_cleanly_without_its_folder,
        "the folder checked after the lookup",
        expected = "wsl-register doesn't stop at the folder's check before anything else",
        check_folder(&read(PS), &{
            let text = read(WORKFLOW);
            let step = "      - name: The image cache on E:, there and writable\n        run: |\n          powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .github\\scripts\\wsl.ps1 cache-ready\n          exit $LASTEXITCODE\n";
            let moved = edited(&text, step, "");
            edited(
                &moved,
                "      - name: This run's image\n",
                &format!("{step}      - name: This run's image\n"),
            )
        })
    );
}

// ---- E: only ----------------------------------------------------------------------------------------------------

/// Every write `wsl.ps1` makes, each with where it writes: the cache's under `$CacheRoot` on E:, the distro's disk
/// under E:\wsl\, and the record under the runner's temporary folder, which the runner's `.env` keeps on E:.
const WRITES: [&str; 13] = [
    // Invoke-Wsl -OutFile: the CPU results' tar stream, into the record.
    "$file = [IO.File]::Create($OutFile)",
    // Save-Image: `$partial` and `$Entry` are `<$CacheRoot>\<key>` and its `.partial`.
    "if (Test-Path -LiteralPath $old) { Remove-Item -LiteralPath $old -Recurse -Force }",
    "New-Item -ItemType Directory -Path $partial | Out-Null",
    "foreach ($f in $ImageFiles) { Move-Item -LiteralPath (Join-Path $From $f) -Destination (Join-Path $partial $f) }",
    "Rename-Item -LiteralPath $partial -NewName (Split-Path -Leaf $Entry)",
    // Limit-Cache: the current entry's time, and the entries listed in $CacheRoot.
    "(Get-Item -LiteralPath (Join-Path $CacheRoot $Current)).LastWriteTimeUtc = [DateTime]::UtcNow",
    "Remove-Item -LiteralPath $e.FullName -Recurse -Force",
    // cache-ready: the probe, in $CacheRoot.
    "[IO.File]::WriteAllText($probe, '')",
    "Remove-Item -LiteralPath $probe -Force",
    // register: the distro's disk, E:\wsl\principia-wsl.
    "New-Item -ItemType Directory -Force -Path $Disk | Out-Null",
    // proofs and cpu: the record.
    "New-Item -ItemType Directory -Force -Path $Record | Out-Null",
    "$results | Set-Content -LiteralPath (Join-Path $Record 'wsl-proofs.txt')",
    "New-Item -ItemType Directory -Force -Path $Record | Out-Null",
];

/// The cache is E:\runner-tools\wsl-cache, a path on E: is the only kind the script names, and every write it makes is
/// one of [`WRITES`]: a new one, anywhere, fails here until it is listed with where it writes. Save-Image's entry is the
/// cache's folder for this recipe, and Limit-Cache removes only what it lists in the cache's folder, by the names the
/// cache gives.
fn check_e_only(script: &str) {
    assert!(
        script.contains(&format!("\n{CACHE_ROOT}\n")),
        "the image cache isn't E:\\runner-tools\\wsl-cache"
    );
    let bytes = script.as_bytes();
    for at in 1..bytes.len().saturating_sub(1) {
        if bytes[at] == b':'
            && bytes[at + 1] == b'\\'
            && bytes[at - 1].is_ascii_alphabetic()
            && !bytes
                .get(at.wrapping_sub(2))
                .is_some_and(u8::is_ascii_alphanumeric)
        {
            assert!(
                bytes[at - 1].eq_ignore_ascii_case(&b'E'),
                "wsl.ps1 names a path off E: at byte {at}: {}",
                &script[at - 1..(at + 20).min(script.len())]
            );
        }
    }
    let verbs = [
        "Move-Item",
        "Copy-Item",
        "Remove-Item",
        "Rename-Item",
        "New-Item",
        "Set-Content",
        "Add-Content",
        "Out-File",
        "WriteAll",
        "LastWriteTimeUtc =",
        "IO.File]::Create",
        "Expand-Archive",
        "Invoke-WebRequest",
    ];
    let mut found: Vec<&str> = statements(script)
        .into_iter()
        .filter(|l| verbs.iter().any(|v| l.contains(v)))
        .collect();
    let mut listed = WRITES.to_vec();
    found.sort_unstable();
    listed.sort_unstable();
    assert_eq!(
        found, listed,
        "wsl.ps1 writes somewhere this test doesn't list"
    );
    assert!(
        arm(script, "register").contains("$image = Join-Path $CacheRoot $key\n")
            && function(script, "Save-Image").contains("    $partial = \"$Entry.partial\"\n"),
        "Save-Image's entry isn't the cache's folder for this recipe"
    );
    let limit = function(script, "Limit-Cache");
    assert!(
        limit.contains("Get-ChildItem -LiteralPath $CacheRoot -Directory | Where-Object { $_.Name -match '^[0-9a-f]{64}(\\.partial)?$' }"),
        "Limit-Cache removes what the cache didn't name"
    );
}

#[test]
fn wsl_cache_writes_only_under_e() {
    check_e_only(&read(PS));
}

negative_control!(
    wsl_cache_writes_only_under_e,
    "a copy of the image to C:",
    expected = "wsl.ps1 names a path off E:",
    check_e_only(&edited(
        &read(PS),
        "    \"cached this run's image as $Entry\"\n",
        "    Copy-Item -LiteralPath $Entry -Destination 'C:\\wsl-cache' -Recurse\n    \"cached this run's image as $Entry\"\n"
    ))
);

#[cfg(feature = "controls")]
mod unlisted_write {
    use super::*;
    // A write with no drive in it, to the user's profile.
    negative_control!(
        wsl_cache_writes_only_under_e,
        "a write this test doesn't list",
        expected = "wsl.ps1 writes somewhere this test doesn't list",
        check_e_only(&edited(
            &read(PS),
            "    \"cached this run's image as $Entry\"\n",
            "    Copy-Item -LiteralPath $Entry -Destination $env:USERPROFILE -Recurse\n    \"cached this run's image as $Entry\"\n"
        ))
    );
}

#[cfg(feature = "controls")]
mod cache_on_c {
    use super::*;
    negative_control!(
        wsl_cache_writes_only_under_e,
        "the cache on C:",
        expected = "the image cache isn't E:\\runner-tools\\wsl-cache",
        check_e_only(&edited(
            &read(PS),
            "$CacheRoot = 'E:\\runner-tools\\wsl-cache'",
            "$CacheRoot = 'C:\\runner-tools\\wsl-cache'"
        ))
    );
}

#[cfg(feature = "controls")]
mod removes_anything {
    use super::*;
    // Limit-Cache removes every folder in the cache's, whatever its name.
    negative_control!(
        wsl_cache_writes_only_under_e,
        "a limit that removes what the cache didn't name",
        expected = "Limit-Cache removes what the cache didn't name",
        check_e_only(&edited(
            &read(PS),
            "Where-Object { $_.Name -match '^[0-9a-f]{64}(\\.partial)?$' }",
            "Where-Object { $_.Name -match '.' }"
        ))
    );
}

// ---- At most two images -----------------------------------------------------------------------------------------

/// `Limit-Cache` marks this recipe's image as just used, keeps it and the one other complete image used last, and
/// removes every other image and every half-filled one; `register` runs it on every registration, hit or miss, once the
/// image is checked and before the import.
fn check_keep_two(script: &str) {
    let limit = statements(function(script, "Limit-Cache"));
    let has = |s: &str| limit.contains(&s);
    assert!(
        has("(Get-Item -LiteralPath (Join-Path $CacheRoot $Current)).LastWriteTimeUtc = [DateTime]::UtcNow")
            && has("$previous = @($entries | Where-Object { $_.Name -match '^[0-9a-f]{64}$' -and $_.Name -ne $Current } |")
            && has("Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1 | ForEach-Object { $_.Name })")
            && has("$keep = @($Current) + $previous")
            && has("if ($keep -notcontains $e.Name) {")
            && has("Remove-Item -LiteralPath $e.FullName -Recurse -Force"),
        "Limit-Cache doesn't keep this image and the one before it, and no more"
    );
    let register = arm(script, "register");
    let limited = register.find("        Limit-Cache $key\n");
    let checked = register.rfind("        Assert-Image $image\n");
    let import = register.find("& wsl.exe --import");
    assert!(
        limited.is_some_and(|l| checked.is_some_and(|c| c < l) && import.is_some_and(|i| l < i)),
        "register doesn't limit the cache on every registration"
    );
}

#[test]
fn wsl_cache_keeps_this_image_and_the_one_before() {
    check_keep_two(&read(PS));
}

negative_control!(
    wsl_cache_keeps_this_image_and_the_one_before,
    "three images kept",
    expected = "Limit-Cache doesn't keep this image and the one before it",
    check_keep_two(&edited(
        &read(PS),
        "Select-Object -First 1 | ForEach-Object",
        "Select-Object -First 2 | ForEach-Object"
    ))
);

#[cfg(feature = "controls")]
mod never_limited {
    use super::*;
    negative_control!(
        wsl_cache_keeps_this_image_and_the_one_before,
        "a registration that doesn't limit the cache",
        expected = "register doesn't limit the cache on every registration",
        check_keep_two(&edited(&read(PS), "        Limit-Cache $key\n", ""))
    );
}

#[cfg(feature = "controls")]
mod oldest_kept {
    use super::*;
    // The oldest other image kept, in place of the one used last.
    negative_control!(
        wsl_cache_keeps_this_image_and_the_one_before,
        "the oldest kept",
        expected = "Limit-Cache doesn't keep this image and the one before it",
        check_keep_two(&edited(
            &read(PS),
            "Sort-Object LastWriteTimeUtc -Descending",
            "Sort-Object LastWriteTimeUtc"
        ))
    );
}

// ---- The recipe key ---------------------------------------------------------------------------------------------

/// The files `$RecipeFiles` lists, in order.
fn recipe_files(script: &str) -> Vec<String> {
    let line = script
        .lines()
        .find(|l| l.starts_with("$RecipeFiles = @("))
        .expect("wsl.ps1 lists no recipe files");
    line.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The repository's files `build-image.sh` reads, by its `"$here/<f>"` (scripts/wsl/) and `"$repo/<f>"` references.
fn read_by_build(build: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (var, dir) in [("\"$here/", "scripts/wsl/"), ("\"$repo/", "")] {
        for (at, _) in build.match_indices(var) {
            let rest = &build[at + var.len()..];
            let name = &rest[..rest.find('"').expect("a closing quote")];
            if !name.starts_with("..") {
                out.push(format!("{dir}{name}"));
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The key as `Get-RecipeKey` makes it: the SHA-256 of `principia-wsl recipe 1` and one line per recipe file, its
/// path and its bytes' SHA-256, each line ending in a newline.
fn recipe_key(files: &[String]) -> String {
    let hex = |b: &[u8]| -> String {
        Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect()
    };
    let mut text = String::from("principia-wsl recipe 1\n");
    for f in files {
        let bytes = std::fs::read(root().join(f)).unwrap_or_else(|e| panic!("{f}: {e}"));
        text.push_str(&format!("{f} {}\n", hex(&bytes)));
    }
    hex(text.as_bytes())
}

/// The recipe key covers the build script, which holds the snapshot date, the base and the tools' versions; every file
/// of the repository the script reads; and the manifest's pin. `Get-RecipeKey` hashes those files' bytes, line by line,
/// as [`recipe_key`] does, which prints this head's key, for the register run's log to show.
fn check_recipe(script: &str, build: &str) {
    let files = recipe_files(script);
    let mut needed = read_by_build(build);
    needed.push(BUILD.to_string());
    needed.push("scripts/wsl/manifest.sha256".to_string());
    for f in &needed {
        assert!(
            files.contains(f),
            "the recipe key misses {f}, which the image is built from"
        );
    }
    for f in &files {
        assert!(
            root().join(f).is_file(),
            "the recipe lists {f}, which isn't a file"
        );
    }
    assert!(
        build.contains("\nSNAPSHOT=") && build.contains("snapshot.ubuntu.com/ubuntu/$SNAPSHOT"),
        "the snapshot date isn't in the build script the key hashes"
    );
    assert_eq!(
        statements(function(script, "Get-RecipeKey")),
        [
            "function Get-RecipeKey {",
            "$lines = @('principia-wsl recipe 1') + @(foreach ($f in $RecipeFiles) { \"$f $(Get-Sha256 $f)\" })",
            "Get-TextSha256 $lines",
            "}"
        ],
        "Get-RecipeKey isn't the hash of the recipe's files"
    );
    println!("recipe key at this head: {}", recipe_key(&files));
}

#[test]
fn wsl_recipe_key_covers_what_the_build_reads() {
    check_recipe(&read(PS), &read(BUILD));
}

negative_control!(
    wsl_recipe_key_covers_what_the_build_reads,
    "a file the build reads, left out of the key",
    expected = "the recipe key misses scripts/wsl/extra.conf",
    check_recipe(
        &read(PS),
        &edited(
            &read(BUILD),
            "sudo install -m 0644 -o 0 -g 0 \"$here/wsl.conf\"",
            "sudo install -m 0644 -o 0 -g 0 \"$here/extra.conf\" \"$root/etc/extra.conf\"\nsudo install -m 0644 -o 0 -g 0 \"$here/wsl.conf\""
        )
    )
);

#[cfg(feature = "controls")]
mod toolchain_dropped {
    use super::*;
    // rust-toolchain.toml dropped from the key: a new toolchain would register the old image.
    negative_control!(
        wsl_recipe_key_covers_what_the_build_reads,
        "the toolchain left out of the key",
        expected = "the recipe key misses rust-toolchain.toml",
        check_recipe(
            &edited(&read(PS), ", 'rust-toolchain.toml')", ")"),
            &read(BUILD)
        )
    );
}

#[cfg(feature = "controls")]
mod pin_dropped {
    use super::*;
    negative_control!(
        wsl_recipe_key_covers_what_the_build_reads,
        "the manifest's pin left out of the key",
        expected = "the recipe key misses scripts/wsl/manifest.sha256",
        check_recipe(
            &edited(&read(PS), ", 'scripts/wsl/manifest.sha256'", ""),
            &read(BUILD)
        )
    );
}
