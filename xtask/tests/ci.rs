//! `cargo xtask ci` runs its registered runners in order and fails when any runner fails (R-177).

use std::sync::Mutex;

use xtask::ci::{run, Runner, RUNNERS};

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
        },
        Runner {
            name: "second",
            run: second,
        },
        Runner {
            name: "third",
            run: third,
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

#[test]
fn ci_registry_is_empty_at_task_m0_01() {
    assert!(RUNNERS.is_empty());
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
        }]),
        Ok(()),
        "control: the run with a failing runner did not pass"
    )
);

validation::negative_control!(
    ci_registry_is_empty_at_task_m0_01,
    "a registry holding one runner, checked for emptiness",
    expected = "control: a registry holding one runner is not empty",
    assert!(
        [Runner {
            name: "failing",
            run: failing,
        }]
        .is_empty(),
        "control: a registry holding one runner is not empty"
    )
);
