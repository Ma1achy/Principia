# TASK-M5-30 — Membrane audit: QuadReduction is the sole automatic return

- **Milestone:** M5
- **Closes:** REQ-SYS-033, REQ-SYS-036, REQ-SCHED-047
- **Depends on:** TASK-M5-14, TASK-M5-26, TASK-M5-29
- **Needs (earlier milestones):** REQ-SYS-021, REQ-TOOL-028
- **Reviewers:** code, qa
- **Pitfalls:** PIT-9
- **Size:** ~200 lines

## Goal
The GPU↔CPU membrane has exactly its five crossings and the rule is enforced by CI: the only automatic
GPU → CPU return of simulation data is the `QuadReduction` readback, feeding only the scheduler (R-288's per-frame
telemetry readback is not simulation data, R-332); `SimState` and `ICDescriptor` never leave
the GPU in normal operation; the only other pulls are the user-initiated async ones (single-IC trace, columnar export
decode); no code path reads the screen texture into data.

## References
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_systems_architecture.md` § "The return paths"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `decisions.md` § "R-332 — `QuadReduction` is the sole automatic return of simulation data; the telemetry readback is not simulation data *(closes RQ-173)*"

## Deliverables
- `xtask/src/lint/membrane.rs` (`cargo xtask lint membrane`): enumerates every buffer map / readback / texture read
  in the engine and render crates against an allow-list of the five crossings; runs in CI.
- The allow-list with one entry per sanctioned crossing and the section it rests on.

## Acceptance tests
- Review checklist (code) — audit every buffer map/readback: only QuadReduction is read back per frame as simulation data, beside R-288's telemetry readback, which is not simulation data; other readbacks are behind a user action and async (REQ-SYS-033, R-332).
- Review checklist (code) — an audit of all buffer map/read calls finds only the reduction readback, R-288's telemetry readback (not simulation data) and the two sanctioned pulls (REQ-SYS-036, R-332).
- Review checklist (code) — no code path reads the screen texture into data structures; scheduler inputs come only from QuadReduction (REQ-SCHED-047).

## Notes
- The lint must be shown to fail on an injected `SimState` readback (pitfalls §9 applies to the lint itself).
- R-332 (1 Oct 2026, closes RQ-173): `QuadReduction` is the sole automatic return of *simulation data*; R-288's
  telemetry readback is not simulation data. REQ-SYS-036, its source systems_architecture §3 and its checklist line read the same way
  (applied per R-332, accepted 1 Oct 2026, R-332's note).
