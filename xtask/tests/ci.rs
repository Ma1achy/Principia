//! `cargo xtask ci` runs its registered runners in order and fails when any runner fails (R-177); its registry holds
//! `plan-check` (TASK-M0-02), `controls` (R-198), `lint constants` (TASK-M0-08), `lint vocab` (TASK-M0-16), `gate`
//! (TASK-M0-05) and `golden` (TASK-M0-06). `cargo xtask ci --list` runs each runner's listing-only form alone, which
//! for `controls` runs no control (R-235).

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use validation::spawn::Spawn;
use xtask::ci::{list, run, Runner, RUNNERS};

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

/// The registry's names: `plan-check` (TASK-M0-02), `build-kernel` (TASK-M0-14), `controls` (TASK-M0-22, R-198),
/// `lint constants` (TASK-M0-08), `lint vocab` (TASK-M0-16), then `gate` (TASK-M0-05), then `golden` (TASK-M0-06,
/// R-110).
fn check_the_registry(runners: &[Runner]) {
    let names: Vec<&str> = runners.iter().map(|runner| runner.name).collect();
    assert_eq!(
        names,
        [
            "plan-check",
            "build-kernel",
            "controls",
            "lint constants",
            "lint vocab",
            "gate",
            "golden"
        ],
        "the ci registry is not `plan-check`, `build-kernel`, `controls`, `lint constants`, `lint vocab`, `gate`, then \
         `golden`"
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
    "a registry holding a runner other than `plan-check`, `build-kernel`, `controls`, `lint constants`, `lint vocab`, \
     `gate` and `golden`",
    expected = "the ci registry is not `plan-check`, `build-kernel`, `controls`, `lint constants`, `lint vocab`, `gate`, \
                then `golden`",
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
/// lists one test with its control, and answers any other call as a run of that control which made its test fail.
fn run_with_stand_in_cargo(case: &str, args: &[&str]) -> (bool, Vec<String>) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("ci_{case}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cargo = dir.join("cargo");
    validation::spawn::write_executable(
        &cargo,
        format!(
            r#"#!/bin/sh
echo "$*" >> '{log}'
if [ "$1" = metadata ]; then
  echo '{{"packages":[{{"name":"stand_in","features":{{"controls":[]}},"targets":[{{"doctest":false}}]}}]}}'
  exit 0
fi
for a in "$@"; do
  if [ "$a" = --list ]; then printf 'pairs: test\npairs::negative_control: test\n'; exit 0; fi
done
echo 'test pairs::negative_control - should panic ... ok'
"#,
            log = dir.join("calls.log").display()
        ),
    )
    .unwrap();
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
