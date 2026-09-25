# TASK-M5-29 — Tier 3 tile debug shaders and the quad-state transition table

- **Milestone:** M5
- **Closes:** REQ-RENDER-055, REQ-RENDER-056, REQ-TOOL-116
- **Depends on:** TASK-M5-15, TASK-M1-15
- **Needs (earlier milestones):** REQ-TOOL-026, REQ-TOOL-028, REQ-RENDER-024, REQ-RENDER-020
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~320 lines

## Goal
The Tier 3 tile debug shaders are fed by a small CPU-packed per-visible-quad buffer read in the postprocess
stage, share the compile pipeline and prelude with the sim shaders and cost nothing when off; overlays never move the
per-sample `SimState` array across the membrane. The quad-state view (loaded / pending / refinable / terminal / stale)
checks transitions against a legal-transition table written into debug_tooling_plan §F (REQ-TOOL-116).

## References
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `docs/design/principia_debug_tooling_plan.md` §F: the legal state-transition table (REQ-TOOL-116), with the "Removed
  lines" note.
- `crates/engine/src/debug/quad_meta_buffer.rs` (packed only while an overlay is on); `crates/render` Tier 3 views.
- `crates/engine/tests/quad_state.rs`: only listed transitions occur over scripted sessions.
- `cargo xtask bench tile-debug-off` (no upload or pass when off).

## Acceptance tests
- `cargo xtask bench tile-debug-off` — with every Tier 3 overlay off, no buffer upload or extra pass occurs (frame time equals the no-overlay baseline); on, the buffer is uploaded only while active (REQ-RENDER-055).
- Review checklist (code) — no overlay code path reads back SimState buffers (REQ-RENDER-056).
- Review checklist (physics) — debug_tooling_plan §F (or the scheduler contract it cites) gives the transition table the view checks against, listing every legal transition; the quad-state view's test asserts only listed transitions occur; physics reviewer approved; the doc change is in this PR and the physics reviewer approves it before merge (REQ-TOOL-116).
- `cargo test -p engine quad_state_transitions` — the test that follows from the written definition: debug_tooling_plan §F (or the scheduler contract it cites) gives the transition table the view checks against, listing every legal transition; the quad-state view's test asserts only listed transitions occur; physics reviewer approved (REQ-TOOL-116).

## Notes
- Definitions (R-72) this task writes: REQ-TOOL-116.
- The terminal state is tied to the true floors (scheduler Part 4), which M6 builds; the table lists them now and
  the M5 test exercises the transitions that exist in M5.
