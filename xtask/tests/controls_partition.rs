//! `cargo xtask controls --partition k/n` and `cargo xtask ci --partition k/n` (R-360): the controls split into n
//! slices by a stable hash of the control name, which CI runs as n parallel jobs. The n shards together run every
//! control exactly once, and `cargo xtask ci`'s other runners run in one shard only, but for `build-kernel`, whose
//! kernel controls in any slice read.
//!
//! The controls runs go to a stand-in `cargo` (`CARGO`) that answers `metadata` and the listing from canned text,
//! answers a run as every control it names making its test fail (and an unpartitioned run, by the `negative_control`
//! filter, as every listed control making its test fail), and logs each control named.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use validation::spawn::Spawn;
use xtask::ci::{run_partition, Runner};
use xtask::controls::{shard_of, Partition};

/// The canned listing: 40 tests in two targets' modules, each with its control.
fn listing() -> String {
    (0..40)
        .map(|i| {
            let module = if i % 2 == 0 { "alpha" } else { "beta" };
            format!("{module}::t{i:02}: test\n{module}::t{i:02}::negative_control: test\n")
        })
        .collect()
}

/// The controls in [`listing`].
fn controls() -> Vec<String> {
    listing()
        .lines()
        .filter_map(|l| l.strip_suffix(": test"))
        .filter(|name| name.ends_with("::negative_control"))
        .map(str::to_owned)
        .collect()
}

/// Runs `xtask controls --partition <slice>` with the stand-in `cargo` in a directory of its own named `case`; returns
/// whether xtask passed, its output, and the controls the stand-in was asked to run.
fn run_shard(case: &str, slice: &str) -> (bool, String, Vec<String>) {
    run_shard_on(case, slice, &listing())
}

/// [`run_shard`] with the stand-in listing `listed`.
fn run_shard_on(case: &str, slice: &str, listed: &str) -> (bool, String, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("controls_partition_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("listed.txt"), listed).unwrap();
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            r#"#!/bin/sh
if [ "$1" = metadata ]; then
  echo '{{"packages":[{{"name":"partition_fake","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}]}}'
  exit 0
fi
for a in "$@"; do
  if [ "$a" = --list ]; then cat '{dir}/listed.txt'; exit 0; fi
done
exact=
for a in "$@"; do
  if [ -n "$exact" ]; then
    echo "$a" >> '{dir}/ran.log'
    echo "test $a - should panic ... ok"
  fi
  if [ "$a" = --exact ]; then exact=1; fi
done
if [ -z "$exact" ]; then
  for a in "$@"; do
    if [ "$a" = negative_control ]; then
      sed -n 's/^\(.*::negative_control\): test$/test \1 - should panic ... ok/p' '{dir}/listed.txt'
    fi
  done
fi
"#,
            dir = dir.display()
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["controls", "--partition", slice])
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let ran = fs::read_to_string(dir.join("ran.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), out, ran)
}

/// Runs each shard of `slices` and checks that each passed and that, together, they ran every listed control exactly
/// once: the union is the full set and the shards are disjoint.
fn check_every_control_once(case: &str, slices: &[&str]) {
    let mut runs: BTreeMap<String, Vec<&str>> =
        controls().into_iter().map(|c| (c, Vec::new())).collect();
    for slice in slices {
        let (ok, out, ran) = run_shard(&format!("{case}_{}", slice.replace('/', "of")), slice);
        assert!(ok, "shard {slice} failed:\n{out}");
        println!("shard {slice}: {} control(s)", ran.len());
        for control in ran {
            runs.get_mut(&control)
                .unwrap_or_else(|| panic!("shard {slice} ran `{control}`, which is not listed"))
                .push(slice);
        }
    }
    for (control, shards) in &runs {
        assert!(
            shards.len() == 1,
            "control `{control}` ran in {} shard(s), not exactly one: {shards:?}",
            shards.len()
        );
    }
}

#[test]
fn controls_partition_runs_every_control_once_across_the_shards() {
    check_every_control_once("all", &["1/4", "2/4", "3/4", "4/4"]);
}

validation::negative_control!(
    controls_partition_runs_every_control_once_across_the_shards,
    "shard 3/4 run twice and shard 4/4 not at all",
    expected = "not exactly one",
    check_every_control_once("ctl", &["1/4", "2/4", "3/4", "3/4"])
);

/// How a check runs `xtask controls` as shard `slice` would, in a directory named `case`: its output.
type RunReport = fn(&str, &str) -> String;

/// The real run: `xtask controls --partition <slice>`.
fn sharded(case: &str, slice: &str) -> String {
    run_shard(case, slice).1
}

/// Each shard of 4 names itself and its slice in its report: how many of the 40 controls are its own, and that each
/// failed its test; shard 1 adds that every test has its control. The unpartitioned run, `--partition 1/1`, names no
/// shard, and reports the 40 tests each failed by its control.
fn check_shard_reports(case: &str, run: RunReport) {
    for k in 1..=4u64 {
        let slice = format!("{k}/4");
        let out = run(&format!("{case}_{k}of4"), &slice);
        let mine = controls().iter().filter(|c| shard_of(c, 4) == k).count();
        for line in [
            format!("partition_fake: shard {slice}: {mine} of 40 control(s)"),
            format!("partition_fake: shard {slice}'s {mine} control(s) each failed its test"),
        ] {
            assert!(
                out.contains(&line),
                "shard {slice} did not name its slice: no `{line}` in\n{out}"
            );
        }
        assert_eq!(
            out.contains("; 40 test(s), each with a control"),
            k == 1,
            "shard {slice}: only shard 1 reports the listing\n{out}"
        );
    }
    let out = run(&format!("{case}_1of1"), "1/1");
    assert!(
        !out.contains("shard")
            && out.contains("partition_fake: 40 test(s), each failed by its control"),
        "the unpartitioned run reported as a shard:\n{out}"
    );
}

#[test]
fn controls_partition_names_the_shard_in_its_report() {
    check_shard_reports("report", sharded);
}

validation::negative_control!(
    controls_partition_names_the_shard_in_its_report,
    "a report whose partition prints as nothing, so no shard names its slice",
    expected = "did not name its slice",
    check_shard_reports("ctl_report", |case, slice| {
        let report = sharded(case, slice);
        let shard = format!("shard {slice}");
        report.replace(&shard, "")
    })
);

/// With `listed`, which has a test (`m::alone`) with no control, shard 1 of 4 fails naming it, and shards 2–4 pass:
/// the listing's findings are reported by one shard.
fn check_listing_finding_in_shard_1_only(case: &str, listed: &str) {
    for slice in ["1/4", "2/4", "3/4", "4/4"] {
        let (ok, out, _) = run_shard_on(
            &format!("{case}_{}", slice.replace('/', "of")),
            slice,
            listed,
        );
        if slice == "1/4" {
            assert!(
                !ok && out.contains("m::alone"),
                "shard 1/4 did not report the test with no control:\n{out}"
            );
        } else {
            assert!(
                ok,
                "shard {slice} reported a listing's finding, which shard 1 reports:\n{out}"
            );
        }
    }
}

#[test]
fn controls_partition_reports_a_listing_finding_in_shard_1_only() {
    check_listing_finding_in_shard_1_only("alone", &format!("{}m::alone: test\n", listing()));
}

validation::negative_control!(
    controls_partition_reports_a_listing_finding_in_shard_1_only,
    "a listing in which every test has its control, where shard 1 must find one without",
    expected = "shard 1/4 did not report the test with no control",
    check_listing_finding_in_shard_1_only("ctl_alone", &listing())
);

/// Whether a control, of a crate, is in a slice: the real one reads only the control's name.
type Holds = fn(Partition, &str, &str) -> bool;

/// The real slicing: by the control's libtest name alone (R-360).
fn by_name(slice: Partition, _krate: &str, control: &str) -> bool {
    slice.holds(control)
}

/// 80 controls, 40 in each of two crates, as (crate, libtest name).
fn named_controls() -> Vec<(&'static str, String)> {
    ["validation", "xtask"]
        .into_iter()
        .flat_map(|krate| {
            (0..40).map(move |i| (krate, format!("{krate}_t::t{i:02}::negative_control")))
        })
        .collect()
}

/// The 4 slices by `holds` are disjoint and together hold every name, and a fixed name keeps its slice.
fn check_slices_hold_each_name_once(holds: Holds) {
    let slices: Vec<Partition> = (1..=4).map(|k| Partition { k, n: 4 }).collect();
    for (krate, control) in named_controls() {
        let held: Vec<u64> = slices
            .iter()
            .filter(|p| holds(**p, krate, &control))
            .map(|p| p.k)
            .collect();
        assert!(
            held.len() == 1,
            "control `{control}` is in slices {held:?} of 4, not exactly one"
        );
    }
    // FNV-1a 64 of the name, mod 4, plus one: fixed, whatever the run, machine or toolchain.
    let fixed = "xtask_t::t00::negative_control";
    let slice = (1..=4).find(|k| holds(Partition { k: *k, n: 4 }, "xtask", fixed));
    assert_eq!(slice, Some(shard_of(fixed, 4)), "`{fixed}` moved slice");
}

#[test]
fn controls_partition_slices_hold_each_name_once() {
    check_slices_hold_each_name_once(by_name);
}

validation::negative_control!(
    controls_partition_slices_hold_each_name_once,
    "a partition that drops a name: `t07` in no slice",
    expected = "not exactly one",
    check_slices_hold_each_name_once(|p, _, c| p.holds(c) && !c.contains("::t07::"))
);

/// By `holds`, each crate's controls fall in more than one of the 4 slices, and every slice holds some: the slices are
/// by control name, not by crate (R-360).
fn check_slices_by_name_not_crate(holds: Holds) {
    let controls = named_controls();
    for krate in ["validation", "xtask"] {
        let slices: std::collections::BTreeSet<u64> = controls
            .iter()
            .filter(|(k, _)| *k == krate)
            .flat_map(|(k, c)| (1..=4).filter(move |s| holds(Partition { k: *s, n: 4 }, k, c)))
            .collect();
        assert!(
            slices.len() > 1,
            "crate {krate}'s controls all fall in one slice, {slices:?}: sliced by crate, not by name"
        );
    }
    for k in 1..=4 {
        assert!(
            controls
                .iter()
                .any(|(krate, c)| holds(Partition { k, n: 4 }, krate, c)),
            "slice {k}/4 holds no control"
        );
    }
}

#[test]
fn controls_partition_slices_by_name_not_crate() {
    check_slices_by_name_not_crate(by_name);
}

validation::negative_control!(
    controls_partition_slices_by_name_not_crate,
    "a partition by crate: each crate's controls in its crate name's slice",
    expected = "sliced by crate, not by name",
    check_slices_by_name_not_crate(|p, krate, _| shard_of(krate, p.n) == p.k)
);

static RAN: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Held for each whole check, so the test and its control, in one process under `cargo test`, keep [`RAN`] apart.
static SERIAL: Mutex<()> = Mutex::new(());

/// How a check runs the runners as shard `slice` would.
type RunShard = fn(&[Runner], Partition) -> Result<(), String>;

fn plan_check() -> Result<(), String> {
    RAN.lock().unwrap().push("plan-check");
    Ok(())
}

fn build_kernel() -> Result<(), String> {
    RAN.lock().unwrap().push("build-kernel");
    Ok(())
}

fn lint() -> Result<(), String> {
    RAN.lock().unwrap().push("lint vocab");
    Ok(())
}

/// The runners of `RUNNERS` that `run` runs as shard `slice`, in order.
fn ran_in(run: RunShard, slice: &str) -> Vec<&'static str> {
    RAN.lock().unwrap().clear();
    run(RUNNERS, Partition::parse(slice).unwrap()).unwrap();
    RAN.lock().unwrap().clone()
}

/// `ci --partition`, run by `run`: shard 1 runs every runner; any other shard runs only `build-kernel` (and its
/// controls' slice).
fn check_runners_by_shard(run: RunShard) {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(
        ran_in(run, "1/4"),
        ["plan-check", "build-kernel", "lint vocab"],
        "shard 1/4 did not run every runner"
    );
    for slice in ["2/4", "3/4", "4/4"] {
        assert_eq!(
            ran_in(run, slice),
            ["build-kernel"],
            "shard {slice} ran a runner other than build-kernel, which shard 1 runs"
        );
    }
}

/// Fake runners named as `cargo xtask ci`'s are, none of them `controls`, which would run on this workspace.
const RUNNERS: &[Runner] = &[
    Runner {
        name: "plan-check",
        run: plan_check,
        list: plan_check,
    },
    Runner {
        name: "build-kernel",
        run: build_kernel,
        list: build_kernel,
    },
    Runner {
        name: "lint vocab",
        run: lint,
        list: lint,
    },
];

#[test]
fn controls_partition_ci_runs_the_other_runners_in_shard_1_only() {
    check_runners_by_shard(run_partition);
}

validation::negative_control!(
    controls_partition_ci_runs_the_other_runners_in_shard_1_only,
    "every shard running every runner, as unsharded `cargo xtask ci` does",
    expected = "ran a runner other than build-kernel",
    check_runners_by_shard(|runners, _| xtask::ci::run(runners))
);
