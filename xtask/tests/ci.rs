//! `cargo xtask ci` runs its registered runners in order and fails when any runner fails (R-177); its registry holds
//! `build-kernel` (TASK-M0-14), `controls` (R-198), `lint constants` (TASK-M0-08), `lint vocab` (TASK-M0-16), `gate`
//! (TASK-M0-05) and `golden` (TASK-M0-06). `cargo xtask ci --list` runs each runner's listing-only form alone, which
//! for `controls` runs no control (R-235).

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use serde_json::json;
use validation::spawn::Spawn;
use xtask::ci::{list, run, Runner, PROCESSES, RUNNERS};

#[path = "support/stand_in.rs"]
mod stand_in;

static ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn first() -> Result<(), String> {
    ORDER.lock().unwrap().push("first");
    Ok(())
}

fn second() -> Result<(), String> {
    ORDER.lock().unwrap().push("second");
    Err("boom".to_owned())
}

fn third() -> Result<(), String> {
    ORDER.lock().unwrap().push("third");
    Ok(())
}

#[test]
fn ci_runs_runners_in_order_and_reports_failures() {
    let runners = [
        Runner {
            name: "first",
            run: first,
            list: first,
        },
        Runner {
            name: "second",
            run: second,
            list: second,
        },
        Runner {
            name: "third",
            run: third,
            list: third,
        },
    ];
    let result = run(&runners);
    assert_eq!(*ORDER.lock().unwrap(), ["first", "second", "third"]);
    let message = result.unwrap_err();
    assert!(message.contains("second") && !message.contains("first") && !message.contains("third"));
}

#[test]
fn ci_with_no_runners_passes() {
    assert_eq!(run(&[]), Ok(()));
}

/// The registry's names: `build-kernel` (TASK-M0-14), `controls` (TASK-M0-22, R-198),
/// `lint constants` (TASK-M0-08), `lint vocab` (TASK-M0-16), `lint wgsl` (TASK-M0-13), `lint compute-pipelines`
/// (TASK-M0-44), `lint no-alloc` (TASK-M3-01), then `gate` (TASK-M0-05), then `golden` (TASK-M0-06, R-110).
fn check_the_registry(runners: &[Runner]) {
    let names: Vec<&str> = runners.iter().map(|runner| runner.name).collect();
    assert_eq!(
        names,
        [
            "build-kernel",
            "controls",
            "lint constants",
            "lint vocab",
            "lint wgsl",
            "lint compute-pipelines",
            "lint no-alloc",
            "gate",
            "golden"
        ],
        "the ci registry is not `build-kernel`, `controls`, `lint constants`, `lint vocab`, `lint wgsl`, \
         `lint compute-pipelines`, `lint no-alloc`, `gate`, then `golden`"
    );
}

#[test]
fn ci_registry_runs_controls() {
    check_the_registry(RUNNERS);
}

/// A runner for the controls that fails without touching `ORDER`, which the tests read.
#[cfg(feature = "controls")]
fn failing() -> Result<(), String> {
    Err("boom".to_owned())
}

validation::negative_control!(
    ci_runs_runners_in_order_and_reports_failures,
    "a run whose one runner passes, required to report it failed",
    expected = "control: the passing runner was not reported failed",
    {
        let passing = Runner {
            name: "second",
            run: || Ok(()),
            list: || Ok(()),
        };
        assert!(run(&[passing])
            .expect_err("control: the passing runner was not reported failed")
            .contains("second"));
    }
);

validation::negative_control!(
    ci_with_no_runners_passes,
    "a run with a failing runner, required to pass",
    expected = "control: the run with a failing runner did not pass",
    assert_eq!(
        run(&[Runner {
            name: "failing",
            run: failing,
            list: failing,
        }]),
        Ok(()),
        "control: the run with a failing runner did not pass"
    )
);

validation::negative_control!(
    ci_registry_runs_controls,
    "a registry holding a runner other than `build-kernel`, `controls`, `lint constants`, `lint vocab`, \
     `lint wgsl`, `lint compute-pipelines`, `lint no-alloc`, `gate` and `golden`",
    expected = "the ci registry is not `build-kernel`, `controls`, `lint constants`, `lint vocab`, \
                `lint wgsl`, `lint compute-pipelines`, `lint no-alloc`, `gate`, then `golden`",
    check_the_registry(&[Runner {
        name: "failing",
        run: failing,
        list: failing,
    }])
);

thread_local! {
    /// The forms of `alpha` and `beta` run on this test's thread, in order.
    static FORMS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn form(name: &'static str) {
    FORMS.with(|forms| forms.borrow_mut().push(name));
}

/// Two runners, `alpha` and `beta`, each recording which of its forms ran; `beta`'s listing form fails.
fn alpha_and_beta() -> [Runner; 2] {
    [
        Runner {
            name: "alpha",
            run: || {
                form("alpha run");
                Ok(())
            },
            list: || {
                form("alpha list");
                Ok(())
            },
        },
        Runner {
            name: "beta",
            run: || {
                form("beta run");
                Ok(())
            },
            list: || {
                form("beta list");
                Err("boom".to_owned())
            },
        },
    ]
}

/// `result`, from a run of [`alpha_and_beta`] on this thread, ran each runner's listing form alone, in order, and
/// failed naming `beta` alone.
fn check_the_listing(result: Result<(), String>) {
    let forms = FORMS.with(|forms| forms.take());
    assert_eq!(
        forms,
        ["alpha list", "beta list"],
        "xtask ci --list did not run each runner's listing form alone, in order"
    );
    let message = result.expect_err("xtask ci --list passed with a failing listing");
    assert!(message.contains("beta") && !message.contains("alpha"));
}

#[test]
fn ci_list_runs_each_runners_listing_form() {
    check_the_listing(list(&alpha_and_beta()));
}

validation::negative_control!(
    ci_list_runs_each_runners_listing_form,
    "the runners' full forms run in place of their listing forms",
    expected = "xtask ci --list did not run each runner's listing form alone",
    check_the_listing(run(&alpha_and_beta()))
);

/// Runs `xtask <args>` with `CARGO` set to a stand-in, in a directory of its own named `case`, and returns whether it
/// passed with each argument list the stand-in was called with. The stand-in reports one crate declaring `controls`,
/// lists one test with its control, and answers any other call, each of bare `ci`'s steps among them, as a run of that
/// control which made its test fail, exiting 0.
fn run_with_stand_in_cargo(case: &str, args: &[&str]) -> (bool, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cargo = stand_in::stand_in_cargo(
        &dir,
        json!({
            "log": dir.join("calls.log"),
            "metadata": stand_in::metadata("stand_in"),
            "listed": "pairs: test\npairs::negative_control: test\n",
            "output": "test pairs::negative_control - should panic ... ok\n",
        }),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .env("CARGO", &cargo)
        .timed_output()
        .expect("run xtask");
    let calls = fs::read_to_string(dir.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    (output.status.success(), calls)
}

/// `run`, from [`run_with_stand_in_cargo`], passed, and called cargo only for `metadata` and listings: no control ran.
fn check_no_control_ran((passed, calls): (bool, Vec<String>)) {
    assert!(passed, "xtask ci --list failed: {calls:?}");
    assert!(
        calls
            .iter()
            .any(|call| call.split(' ').any(|a| a == "--list")),
        "xtask ci --list listed no test: {calls:?}"
    );
    let runs: Vec<&String> = calls
        .iter()
        .filter(|call| !call.starts_with("metadata") && !call.split(' ').any(|a| a == "--list"))
        .collect();
    assert!(
        runs.is_empty(),
        "xtask ci --list ran cargo beyond metadata and the listings: {runs:?}"
    );
}

#[test]
fn ci_list_runs_no_control() {
    check_no_control_ran(run_with_stand_in_cargo("list", &["ci", "--list"]));
}

validation::negative_control!(
    ci_list_runs_no_control,
    "bare `xtask ci`, which runs the controls, given to the no-run check",
    expected = "xtask ci --list ran cargo beyond metadata and the listings",
    check_no_control_ran(run_with_stand_in_cargo("ctl_list", &["ci"]))
);

/// `processes`, the table bare `xtask ci` runs, holds the registry's runners by name, in its order.
fn check_processes_are_the_registry(processes: &[Runner]) {
    let names = |runners: &[Runner]| runners.iter().map(|r| r.name).collect::<Vec<_>>();
    assert_eq!(
        names(processes),
        names(RUNNERS),
        "xtask ci's processes are not the registry's runners, in its order"
    );
}

#[test]
fn ci_processes_are_the_registry() {
    check_processes_are_the_registry(PROCESSES);
}

validation::negative_control!(
    ci_processes_are_the_registry,
    "the processes without their last runner",
    expected = "xtask ci's processes are not the registry's runners, in its order",
    check_processes_are_the_registry(&PROCESSES[..PROCESSES.len() - 1])
);

/// Each step of bare `xtask ci`, by its runner's name: words its one call to cargo holds, and words it ends with.
const STEPS: [(&str, &str, &str); 9] = [
    ("build-kernel", "-p xtask --", "-- build-kernel"),
    ("controls", "--features controls", "negative_control"),
    ("lint constants", "-p xtask --", "lint constants"),
    ("lint vocab", "-p xtask --", "lint vocab"),
    ("lint wgsl", "-p xtask --", "lint wgsl"),
    (
        "lint compute-pipelines",
        "-p xtask --",
        "lint compute-pipelines",
    ),
    ("lint no-alloc", "-p xtask --", "lint no-alloc"),
    ("gate", "-p validation --bin gate --", "--all"),
    ("golden", "-p xtask --", "golden --all"),
];

/// `run`, from [`run_with_stand_in_cargo`], passed, and every step reached the stand-in cargo, so none did its work in
/// xtask's own process: no golden harness was built, as a golden run in that process builds it.
fn check_every_step_through_cargo((passed, calls): (bool, Vec<String>)) {
    for (name, holds, ends) in STEPS {
        assert!(
            calls
                .iter()
                .filter(|c| c.contains(holds) && c.ends_with(ends))
                .count()
                == 1,
            "xtask ci did not run {name} through its cargo once: {calls:?}"
        );
    }
    assert!(
        !calls.iter().any(|c| c.contains("golden_harness")),
        "xtask ci built the golden harness in its own process: {calls:?}"
    );
    assert!(passed, "xtask ci failed with every step passing: {calls:?}");
}

#[test]
fn ci_runs_every_step_through_its_cargo() {
    check_every_step_through_cargo(run_with_stand_in_cargo("steps", &["ci"]));
}

validation::negative_control!(
    ci_runs_every_step_through_its_cargo,
    "bare `xtask controls`, which runs one step of ci's, given to the check",
    expected = "xtask ci did not run build-kernel through its cargo once",
    check_every_step_through_cargo(run_with_stand_in_cargo("ctl_steps", &["controls"]))
);
