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
//! entry; the base and rustup are pinned by hash and checked before use, the packages come from one dated snapshot,
//! the toolchain is `rust-toolchain.toml`'s; registration refuses without a pinned manifest hash; proof 3 tries the
//! host over WSL's virtual switch, at the distro's gateway and on SMB's 445, as well as over the LAN; and proof 4 runs
//! the unprivileged check and fails unless the processes outside survive it, inside and on Windows.

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
        build.contains("in_root apt-get -q -y purge sudo")
            && build.contains("sudo rm -rf \"$root/etc/sudoers.d\""),
        "the image keeps sudo"
    );
    let url = setting(build, "BASE_URL");
    assert!(
        url.starts_with("https://") && url.contains("24.04"),
        "the base isn't Ubuntu 24.04 over https: {url}"
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
            &edited(&read(BUILD), "in_root apt-get -q -y purge sudo\n", ""),
            &read(CONF),
            &read(".github/workflows/mutants.yml")
        )
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
            && register.contains("if ($liveHash -ne $pinned) {"),
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
