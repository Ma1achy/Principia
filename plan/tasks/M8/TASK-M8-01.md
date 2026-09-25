# TASK-M8-01 — The state contract: SimConfig / RenderState / ViewUI, provenance serialisation and the GUI-sized snapshot

- **Milestone:** M8
- **Closes:** REQ-GUI-036, REQ-GUI-037, REQ-GUI-039, REQ-PERF-071, REQ-PERF-091
- **Depends on:** TASK-M6-21, TASK-M6-22, TASK-M7-11
- **Needs (earlier milestones):** REQ-SYS-011, REQ-GUI-007, REQ-GUI-011, REQ-GUI-016, REQ-SCHED-048, REQ-PERF-016, REQ-GUI-010, REQ-CHART-032, REQ-INT-026
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The GUI-facing surface exists, defined once in Rust: the three typed structs of gui_state_contract §2 with every field in the struct the contract names (SimConfig on the sim key, RenderState on the render key, ViewUI pure UI), the `SetField` path type, and the state `Snapshot`. SimConfig + RenderState serialise as the provenance object, so two clients can edit one schema. ViewUI is defined but the engine never reads it, and the snapshot carries only GUI-sized state (view state, tier, a few scalars, the reported events). This is the surface every later M8 task writes through.

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*"
- `docs/contracts/principia_gui_state_contract.md` § "7. What a replacement GUI must honour (the teardown contract)"
- `docs/design/principia_trajectory_viewing.md` § "6. Placement"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `decisions.md` § "R-94 — The GUI snapshot is ~10 Hz *(closes RQ-45)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/engine/src/contract/state/{sim_config,render_state,view_ui}.rs` — the three structs with the §2 field lists as amended by R-96, R-101 and R-106: the link ids are the chart registry's link-function ids; playback transport (play / pause / speed / loop), the linked views, the kept orbits and `t_cursor` are ViewUI fields.
- `crates/engine/src/contract/state/set_field.rs` — `SetField { path, value, history: History }` over plain serialised data (the history mark is used by TASK-M8-03).
- `crates/engine/src/contract/state/snapshot.rs` — `Snapshot`: view state, current tier, frame scalars, the DECODE_SWITCHOVER / AT_F32_FLOOR events (REQ-GUI-010). No payload, quadtree or reduction type is reachable from it.
- `crates/engine/src/contract/state/provenance.rs` — serde of SimConfig + RenderState as the provenance object.
- `crates/engine/src/state_surface.rs` — the engine's `pub` re-export of this surface; the engine applies SetFields to SimConfig / RenderState and has no ViewUI parameter anywhere.
- Tests: `schema_fields`, `provenance_roundtrip`, `two_clients`, `snapshot_bound`.

## Acceptance tests
- `cargo test -p engine schema_fields` — schema test: each listed field lives in the named struct; playback transport and the linked views are ViewUI fields and are absent from SimConfig; SimConfig's link ids are the chart registry's link-function ids (REQ-GUI-036).
- Review checklist (code reviewer), with `cargo test -p engine no_viewui_reads` (a grep test over `crates/engine/src`) — no engine-crate code references ViewUI fields (REQ-GUI-037).
- `cargo test -p engine provenance_roundtrip` and `cargo test -p engine two_clients` — serialise, deserialise, compare; a second client's edit is visible in the first's snapshot (REQ-GUI-039).
- `cargo test -p engine snapshot_bound` — the snapshot type contains no payload / quadtree / reduction types, and its serialised size stays below a fixed small bound at any zoom depth (REQ-PERF-071).
- Proposal: the snapshot's serialised-size bound, with measured sizes across zoom depths and tiers; the human confirms it at the M8 gate (REQ-PERF-091).

## Notes
- The corpus puts the surface "in the engine crate" (gui_state_contract §1); here it lives in `crates/engine` and the engine re-exports it as its only `pub` surface (RQ-76 layout). TASK-M8-04 adds the compile-fail tests.
- The fixed small bound on the serialised snapshot size (REQ-PERF-071's verify) is not given by the corpus.
- RQ-93 ruled: R-113 — REQ-TOOL-002 (M0) records the M0 contract skeleton's config; the profiler file's header config being this task's REQ-GUI-039 provenance object is verified by REQ-TOOL-098 (TASK-M8-28).
- Closes, for gaps the corpus leaves open: REQ-PERF-091 (R-71 calibration) (classification accepted by R-132).
