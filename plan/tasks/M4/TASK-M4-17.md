# TASK-M4-17 — The uniform-grid survey: CPU↔GPU aggregate agreement, and the prebake path

- **Milestone:** M4
- **Closes:** REQ-VAL-065, REQ-VAL-125, REQ-SYS-027
- **Depends on:** TASK-M4-09, TASK-M4-06, TASK-M3-34
- **Needs (earlier milestones):** REQ-VAL-004, REQ-VAL-135, REQ-SCHED-001, REQ-SYS-017, REQ-VAL-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-10
- **Size:** ~400 lines

## Goal
The test that certifies "the picture is the same": over a uniform grid (not quadtree leaves), the same region is computed on both instantiations and compared on outcome-class fractions, class-boundary location within a pixel or two, and (where computed) island prevalence and decay-time distribution shape, within sampling noise. The statistic and its threshold (parity §5's Q3) are calibrated from measured sampling noise (R-71). The same uniform path is the prebake: uniform tiles, refinement off, no scheduler, camera or frame budget — advance every tile to t and stitch.

## References
- `docs/contracts/principia_parity_contract.md` § "5. The aggregate-survey agreement test (what certifies "the picture is the same")"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/read_first/principia_00_philosophy.md` § "The prebake — a measuring instrument"
- `docs/read_first/principia_00_philosophy.md` § "3. The consequence people get wrong"

## Deliverables
- `crates/engine/src/prebake.rs`: the prebake entry point over uniform tiles (chunked dispatch, TASK-M4-09), asserting refinement disabled and no camera/frame budget consulted.
- `xtask` gate `aggregate-survey` (nightly and before release, headless, R-134) and `survey-statistic` (the calibration measurement: sampling noise vs grid resolution).

## Acceptance tests
- `cargo xtask gate aggregate-survey` — nightly and before-release headless run (R-134) on a uniform grid on both instantiations: outcome-class fractions within grid sampling noise; boundary sets match within 1–2 pixels; statistic and threshold per REQ-VAL-125 (calibrated) (REQ-VAL-065).
- `cargo xtask gate survey-statistic` — the proposal names the statistic and sets the threshold from the measured sampling noise on uniform grids; recorded in decisions.md after the human confirms it at the M4 gate (REQ-VAL-125, calibrated).
- `cargo test -p engine prebake_uniform` plus code review — the prebake entry point advances every tile to t and stitches; it asserts Policy/quadtree refinement is disabled and no camera/frame budget is consulted (REQ-SYS-027).

## Notes
- REQ-VAL-125 is a calibration (R-71): the PR carries the proposed value, its evidence and the reviewer's check, marked pending; the human confirms it at the M4 gate and it is then recorded in decisions.md.
- Any fraction quoted from the survey is shown to converge under grid refinement first (REQ-VAL-004, threshold REQ-VAL-135).
