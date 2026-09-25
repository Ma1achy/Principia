# TASK-M8-02 — Headless drive: the engine from a JSON state file, and the native headless survey

- **Milestone:** M8
- **Closes:** REQ-SYS-054, REQ-SYS-048
- **Depends on:** TASK-M8-01
- **Needs (earlier milestones):** REQ-SYS-021, REQ-SYS-017, REQ-TOOL-006, REQ-TOOL-054, REQ-SCHED-004
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
The engine can be driven with no GUI attached: a SimConfig + RenderState JSON file is loaded, frames run, and an image is written. The same path runs the survey headless natively, from the same kernel as the browser build, writing the outputs Paper 2's numbers come from. This is the "brutal and concrete" test of gui_state_contract §1.

## References
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"

## Deliverables
- `crates/prin/src/cmd/run.rs` — `prin run --state state.json --frames N --out out/` loading the provenance object from TASK-M8-01 and driving the engine's frame loop headless.
- `crates/prin/src/cmd/survey.rs` — `prin survey --state … --out …`: the native headless survey writing its outputs (payload summary, image, frame records).
- `crates/prin/Cargo.toml` — no dependency on `crates/gui` or egui.
- Tests: `headless_json_drive`, `headless_survey`.

## Acceptance tests
- `cargo test -p prin headless_json_drive` — headless test loads a SimConfig + RenderState JSON, runs frames and produces an image with no GUI crate linked (REQ-SYS-054).
- `cargo test -p prin headless_survey` — a native headless survey run completes without a window and writes its outputs (REQ-SYS-048).

## Notes
- none
