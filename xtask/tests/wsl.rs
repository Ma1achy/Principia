//! The WSL2 distro `principia-wsl` (RQ-751 option (c), built in parallel to replace (b) and never blocking; the owner's
//! words of 9 Nov 2026, with the ruling given the same day on `scripts/contain.sh`'s unprivileged mode): the image
//! script `scripts/wsl/build-image.sh`, the Windows side `.github/scripts/wsl.ps1`, the inside `scripts/wsl/inside.sh`,
//! and `win-gpu.yml`'s jobs that run them. Off Windows there is no PowerShell and no WSL, so, as `win_contain.rs` does,
//! these read the scripts and the workflow; the proofs themselves are `wsl-proofs`, run on the PC. `parity.py`, which
//! compares proof 6's two sides, runs here. Each test has its negative controls (R-176).
//!
//! What they hold: every job of the distro runs only when dispatched naming it, never on a push, the schedule or `all`;
//! no job or script of it names a secret or a token, and every checkout leaves no credential; `wsl-cpu` refuses while
//! `.github/wsl-cpu-enabled` is absent, as it is until a reviewed PR adds it after all six proofs have passed;
//! `/etc/wsl.conf` is exactly the owner's, with no `[boot]` and so no systemd, and the image has no sudo and no sudoers
//! entry, the purge forced past sudo's own guard for that command alone and the build failing unless sudo is then gone;
//! the base and rustup are pinned by hash and checked before use, the packages come from one dated snapshot,
//! the toolchain is `rust-toolchain.toml`'s; the image installs the Vulkan packages `ci.yml`'s lavapipe steps install,
//! and `inside.sh` selects the backend those jobs select; registration refuses without a pinned manifest hash, and
//! unregisters an import it could not verify; proof 3 tries the host over WSL's virtual switch, at the distro's gateway
//! and on SMB's 445, as well as over the LAN; and proof 4 runs the unprivileged check and fails unless the processes
//! outside survive it, inside and on Windows.

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
/// default user the Windows session asked for.
const OWNERS_CONF: &str =
    "[automount]\nenabled=false\n\n[interop]\nenabled=false\nappendWindowsPath=false\n\n[user]\ndefault=ci\n";

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
                "          path: ${{ runner.temp }}/wsl-image\n      - name: Register principia-wsl\n",
                "          path: ${{ runner.temp }}/wsl-image\n          run-id: 1\n      - name: Register principia-wsl\n"
            ),
            &scripts()
        )
    );
}

// ---- wsl-cpu stays off --------------------------------------------------------------------------------------------

/// `.github/wsl-cpu-enabled` is absent, and `wsl.ps1 cpu` refuses first thing without it. Adding it is a reviewed PR,
/// after all six proofs pass, which changes this test with it.
fn check_flag(flag_exists: bool, script: &str) {
    assert!(
        !flag_exists,
        "{FLAG} exists: wsl-cpu would run before all six proofs have passed"
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
fn wsl_cpu_is_off_until_its_flag_exists() {
    check_flag(root().join(FLAG).exists(), &read(PS));
}

negative_control!(
    wsl_cpu_is_off_until_its_flag_exists,
    "the flag added",
    expected = "wsl-cpu would run before all six proofs have passed",
    check_flag(true, &read(PS))
);

mod flag_unchecked {
    use super::*;
    negative_control!(
        wsl_cpu_is_off_until_its_flag_exists,
        "a cpu arm that doesn't look for the flag",
        expected = "wsl.ps1 cpu doesn't check the flag first",
        check_flag(
            false,
            &edited(
                &read(PS),
                "        if (-not (Test-Path -LiteralPath '.github\\wsl-cpu-enabled')) {",
                "        if ($false) {"
            )
        )
    );
}

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

fn check_image(build: &str, conf: &str, mutants: &str) {
    assert_eq!(
        conf, OWNERS_CONF,
        "{CONF} isn't exactly the owner's wsl.conf"
    );
    assert!(
        build.contains("sudo install -m 0644 -o 0 -g 0 \"$here/wsl.conf\" \"$root/etc/wsl.conf\""),
        "the image's /etc/wsl.conf isn't {CONF}"
    );
    for word in [
        "[boot]",
        "systemd=true",
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
    let tools = format!(
        "tool: cargo-mutants@{},cargo-nextest@{}",
        setting(build, "MUTANTS_VERSION"),
        setting(build, "NEXTEST_VERSION")
    );
    assert!(
        mutants.contains(&tools),
        "the image's cargo tools aren't mutants.yml's ({tools})"
    );
}

#[test]
fn wsl_image_is_pinned_and_has_the_owners_config() {
    check_image(
        &read(BUILD),
        &read(CONF),
        &read(".github/workflows/mutants.yml"),
    );
}

negative_control!(
    wsl_image_is_pinned_and_has_the_owners_config,
    "systemd switched on",
    expected = "scripts/wsl/wsl.conf isn't exactly the owner's wsl.conf",
    check_image(
        &read(BUILD),
        &format!("{}\n[boot]\nsystemd=true\n", read(CONF)),
        &read(".github/workflows/mutants.yml")
    )
);

mod keeps_sudo {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "sudo kept",
        expected = "the image keeps sudo",
        check_image(
            &edited(&read(BUILD), PURGE, ""),
            &read(CONF),
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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
            check_image(&moved, &read(CONF), &read(".github/workflows/mutants.yml"));
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
            check_image(&moved, &read(CONF), &read(".github/workflows/mutants.yml"));
        }
    );
}

mod other_nextest {
    use super::*;
    negative_control!(
        wsl_image_is_pinned_and_has_the_owners_config,
        "a nextest other than mutants.yml's",
        expected = "the image's cargo tools aren't mutants.yml's",
        check_image(
            &edited(
                &read(BUILD),
                "NEXTEST_VERSION=0.9.146",
                "NEXTEST_VERSION=0.9.999"
            ),
            &read(CONF),
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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
            &read(".github/workflows/mutants.yml")
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

/// `wsl.ps1 register` reads the pin first, refuses one that isn't a SHA-256, checks the copy and the manifest against
/// it before `wsl --import`, imports under E:\wsl\, and derives the manifest again inside the distro after.
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
                .find("if ($copied -ne $built)")
                .is_some_and(|c| c < import)
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
            "[Console]::Out.WriteLine(\"::error::wsl.ps1: $message\")",
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
        "    [Console]::Out.WriteLine(\"::error::wsl.ps1: $message\")\n",
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
}
