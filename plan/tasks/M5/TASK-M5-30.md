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
GPU → CPU crossing is the `QuadReduction` readback, feeding only the scheduler; `SimState` and `ICDescriptor` never leave
the GPU in normal operation; the only other pulls are the user-initiated async ones (single-IC trace, columnar export
decode); no code path reads the screen texture into data.

## References
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_systems_architecture.md` § "The return paths"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"

## Deliverables
- `xtask/src/lint/membrane.rs` (`cargo xtask lint membrane`): enumerates every buffer map / readback / texture read
  in the engine and render crates against an allow-list of the five crossings; runs in CI.
- The allow-list with one entry per sanctioned crossing and the section it rests on.

## Acceptance tests
- Review checklist (code) — audit every buffer map/readback: only QuadReduction is read back per frame; other readbacks are behind a user action and async (REQ-SYS-033).
- Review checklist (code) — an audit of all buffer map/read calls finds only the reduction readback and the two sanctioned pulls (REQ-SYS-036).
- Review checklist (code) — no code path reads the screen texture into data structures; scheduler inputs come only from QuadReduction (REQ-SCHED-047).

## Notes
- The lint must be shown to fail on an injected `SimState` readback (pitfalls §9 applies to the lint itself).
