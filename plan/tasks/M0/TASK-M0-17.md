# TASK-M0-17 — Profiler schema v1: the frame record, the five stages and the nested sections

- **Milestone:** M0
- **Closes:** REQ-TOOL-005, REQ-TOOL-008, REQ-TOOL-120
- **Depends on:** TASK-M0-16
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
Profiler schema v1 exists as typed Rust (`engine::contract::profile`, serde) and as a checked-in JSON Schema: a superset of telemetry §2 in JSON (R-56). At the top level, the frame record — `frame_ms`, `quads_computed`, `quads_reused`, `samples`, `substeps_total`, `playhead_dt`, `camera_delta`, `tree_depth_max` and leaf count, `stage_ms` — and its five stages integrate / reduce / colour / upload / present; beneath them, nested scopes, GPU passes, allocations and events, with the finer categories (quadtree, stain + style, IC decode, readback, egui) as scopes under one of the five stages. The frame record type is the measurement struct, always compiled — reporting is toggleable, measurement is not (telemetry §5.5).

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/engine/src/contract/profile.rs` — `SessionHeader`, `FrameRecord`, `Stage` (the five), `Scope`, `GpuPass`, `Allocation`, `Event`; writer and reader.
- `crates/engine/src/contract/schema/profile_v1.json` — the JSON Schema; `cargo test -p engine` validates written traces against it.
- `crates/engine/src/contract/tests/profile_v1.rs`.

## Acceptance tests
- `cargo test -p engine profile_v1_stages` — an exported trace validates against the v1 JSON Schema; the top level has exactly the five stages; every other scope has one of them as an ancestor; a trace with a top-level `quadtree` scope fails (REQ-TOOL-005).
- `cargo test -p engine profile_v1_superset` — a profiler dump parses as telemetry §2's frame record (the five stages) and contains the nested sections: scopes, GPU passes, allocations, events (REQ-TOOL-008).
- Definition: profiler schema v1's header and record keys and nesting written into dd_telemetry_and_tiers §5 and approved by the physics reviewer (REQ-TOOL-120).

## Notes
- The schema lives in `crates/engine` because the engine (writer, from the first frame loop), `prin` (reader/writer) and the dev GUI (reader, M8) all consume it; reviewers may place it elsewhere.
- Leak flags and hot-path summaries (REQ-TOOL-100, M8) and `prin profile query` (REQ-TOOL-101, M8) are not here.
- See Gaps: v1's nested key names and shape (REQ-TOOL-120).
- RQ-93 ruled: R-113 — R-56's "the measurement struct lands with the first frame loop" is a separate M1 requirement (REQ-TOOL-131, TASK-M1-05); this task defines the struct and its schema.
- Closes, for gaps the corpus leaves open: REQ-TOOL-120 (R-72 definition) (classification accepted by R-132).
