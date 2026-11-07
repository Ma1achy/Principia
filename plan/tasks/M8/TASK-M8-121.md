# TASK-M8-121 — A run from the measurement panel's button marked in its report as not a measurement (R-826)

- **Milestone:** M8
- **Closes:** REQ-PERF-119
- **Depends on:** TASK-M8-86
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~60 lines

## Goal
The owner's ruling on RQ-702 (R-826, option (c)): a browser run started from the measurement panel's button reads the refresh rate measured once at page load, so its report is marked as not a measurement and is never read as one. Runs started by `?run=`, which every measurement TASK-M8-96 takes uses, are unchanged.

## References
- `decisions.md` § "R-826 — A browser run started from the measurement panel's button is marked in its report as not a measurement *(closes RQ-702)*"

## Deliverables
- The measurement report's mark on a panel-button run, in the browser and natively, and its absence on a `?run=` run.

## Acceptance tests
- `cargo test -p gui measure_button_run_marked` — REQ-PERF-119's arm and control.

## Notes
- TASK-M8-86, which built the panel, has merged, so this is its own task. If TASK-M8-37 removes the panel first, this task closes with it, as review decides.
