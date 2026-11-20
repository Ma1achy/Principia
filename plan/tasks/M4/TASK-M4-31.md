# TASK-M4-31 — Near-tie golden cases held only on CI's backends, elsewhere reported (R-695)

- **Milestone:** M4
- **Closes:** REQ-VAL-349
- **Depends on:** TASK-M4-30
- **Needs (earlier milestones):** none
- **Reviewers:** code
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
R-695 takes RQ-686's option (b). The runner keys a near-tie reference by API, so the `vulkan` references, made on lavapipe, are compared on every Vulkan driver. It stops doing that on a driver CI does not render on, and reports instead.

## References
- `decisions.md` § "R-695 — Near-tie golden cases are held only on the backends CI renders on; on any other driver their result is reported, not compared to pass *(closes RQ-686; amends R-296)*"
- `decisions.md` § "R-296 — R-269's half-way fixture keeps one reference per backend; explicit quantisation makes exact ties identical, not values near one *(closes RQ-175; amends R-287)*"
- `decisions.md` § "R-376 — The six M0 calibrations are confirmed: REQ-VAL-138, REQ-VAL-149 (8 shards × 300 min, a ceiling, not a target), REQ-VAL-151, REQ-VAL-156 (600 s), REQ-VAL-180 and REQ-VAL-181 *(closes RQ-202; amends R-71, R-203, R-214, R-217, R-233, R-269, R-287, R-296, R-302, R-305, R-348 and R-352 as they apply to these values)*"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"

## Deliverables
- `xtask/src/golden.rs`: the runner knows the render's driver; a case with one reference per backend is compared on Metal and on lavapipe, and elsewhere its max step and pixel count are reported with the driver's name, with no pass or fail; its module doc says so.
- No reference, baseline row or tolerance changes.

## Acceptance tests
- `cargo test -p xtask golden_near_tie` — REQ-VAL-349's verify, with its control.
- `cargo xtask golden --all` in `gpu-metal` and `gpu-lavapipe` — every case as before, 0 steps (REQ-VAL-349).

## Notes
- Added by R-695 (4 Nov 2026, RQ-686 (b)).
- It depends on TASK-M4-30 for the adapter's identity (applied per R-369).
- Today's per-backend cases are `quantise/halfway` and `quantise/halfway_automatic` (`fixtures/golden/BASELINES.md`).
- *17 Nov 2026 (R-1034; logged in `plan/DECISIONS-LOG.md`):* the reviewers follow the diff's area: physics no longer reviews this task, whose deliverables are outside that area. *Was (Reviewers):* "code, qa, physics".
- 19 Nov 2026 (TASK-M0-88): the Windows runner's `win-vulkan` reports, and does not assert, the `golden_halfway` reference tests and the golden suite `quantise` (R-695), through the `win-reported` and `win-asserted` profiles of `.config/nextest.toml` and the reporting steps of `.github/workflows/win-gpu.yml`. When this task lands, each test its scoping makes pass on that card goes back to an ordinary asserting step: it leaves both profiles' lists, and `xtask/tests/ci_self_hosted.rs`'s `REPORTED` list, in this task's pull request.
- *19 Nov 2026 (R-1034, applied by TASK-M0-76; logged in `plan/DECISIONS-LOG.md`):* qa's independent tests are asked only of physics-critical tasks, those naming physics among their reviewers, and this task's list names no physics, so qa no longer reviews it. *Was (Reviewers):* "code, qa".
