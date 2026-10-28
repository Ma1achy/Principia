# TASK-M4-26 — The raised-cap run: the targets recorded as "not evaluated within cap", evaluated at a calibrated raised horizon (R-636)

- **Milestone:** M4
- **Closes:** REQ-VAL-332
- **Depends on:** TASK-M3-42, TASK-M3-43, TASK-M3-45
- **Needs (earlier milestones):** REQ-VAL-201, REQ-VAL-218, REQ-VAL-222
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
R-636 records every target past R-610's horizon cap as "not evaluated within cap", never as a failure, and queues a separate raised-cap run as a later task: this task. The 667 atlas rows with T > 200 that OC-1, OC-3 and OC-6 record that way are run on the validation harness at a raised cap, a calibration proposed here with its evidence and confirmed by the human at the M4 gate (R-71). R-610's view cap is unchanged.

## References
- `decisions.md` § "R-636 — The 28 Oct update: the Šuvakov-plane targets stay closures and no chart is built yet; targets past the horizon cap are "not evaluated within cap", never failures, and a raised-cap run is a later task; the weekly consistency audit is adopted, report-only, in the cloud; the session's R-369 decisions stand; #326 merges on the Mac's own GPU check; the five-chart build is held; the Mac keeps GPU work, two local agents at most *(closes RQ-583, RQ-592, RQ-602 and RQ-605; supersedes R-391; amends R-262, R-277 and R-357; refines R-633)*"
- `decisions.md` § "R-610 — The horizon's floor is one closure window, 0.4, below which the escape arm cannot fire, and the settings say so; the cap stays at 200 until long horizons are measured for cost: `T_horizon ∈ [0.4, 200]`, default 50 *(closes RQ-555; amends R-575's interim range)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_replication.md` § "5.2 The orbit atlas"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"

## Deliverables
- The raised-cap run on the validation harness: every atlas row recorded as "not evaluated within cap" by OC-1, OC-3 and OC-6, evaluated with their measures at the raised cap; a row still past it stays "not evaluated within cap".
- The calibration proposal for the raised cap, with its evidence: the recorded rows' periods (the longest is T = 886.09) and the run's measured cost.

## Acceptance tests
- `cargo xtask gate replication --target raised-cap --propose` — CALIBRATION (R-71): the cap proposed from the recorded rows' periods and the measured cost of the run; the run's report lists every row it evaluates with OC-1's, OC-3's and OC-6's measures, and every row still past the raised cap as "not evaluated within cap", never as a failure; control: a row past the raised cap must read "not evaluated within cap", not pass or fail; reviewer-checked, confirmed by the human at the M4 gate and recorded in `decisions.md` (REQ-VAL-332).
- `scripts/check_no_atlas_params.sh` — prints `ok`: `atlas-params.json` is not committed, and nothing in this task reads it.

## Notes
- Calibrations proposed here (R-71; human confirmation at the M4 gate, then recorded in decisions.md): REQ-VAL-332.
- Added by R-636 (28 Oct 2026), the owner: "667 orbits past the T cap: record as "not evaluated within cap", never as failures; queue a separate raised-cap run as a later task." Applied per R-369: M4, after M3's runs that record the rows; one requirement, since a task must close one (`plan/check_plan.py`).
- The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com with `docs/incoming/2026-10-26_orbit_catalogues/CITATIONS.md`'s per-family credits, on every fixture, export and figure. `atlas-params.json` never enters the repo (`scripts/check_no_atlas_params.sh`).
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
