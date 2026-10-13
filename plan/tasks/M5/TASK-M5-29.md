# TASK-M5-29 — Tier 3 tile debug shaders and the quad-state transition table

- **Milestone:** M5
- **Closes:** REQ-RENDER-055, REQ-RENDER-056, REQ-TOOL-116, REQ-TOOL-166
- **Depends on:** TASK-M5-15, TASK-M1-15
- **Needs (earlier milestones):** REQ-TOOL-026, REQ-TOOL-028, REQ-RENDER-024, REQ-RENDER-020, REQ-TOOL-011
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~320 lines

## Goal
The Tier 3 tile debug shaders are fed by a small CPU-packed per-visible-quad buffer read in the postprocess
stage, share the compile pipeline and prelude with the sim shaders and cost nothing when off; overlays never move the
per-sample `SimState` array across the membrane. The quad-state view (loaded / pending / refinable / terminal / stale)
checks transitions against a legal-transition table written into debug_tooling_plan §F (REQ-TOOL-116). The
visible-set and locked/stale overlays, which R-425 moved here from the M1 checklist, are defined in debug_tooling_plan
§F from the corpus's existing meanings where they exist (`quad_state` 4 for "stale") and drawn on the Tier 3 path
(REQ-TOOL-166, R-72).

## References
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-425 — The visible-set and locked/stale overlays move to TASK-M5-29 under a new defining requirement, which reuses the corpus's existing meanings (`quad_state` 4 for "stale") rather than defining new ones *(closes RQ-276)*"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record"

## Deliverables
- `docs/design/principia_debug_tooling_plan.md` §F: the legal state-transition table (REQ-TOOL-116), with the "Removed
  lines" note.
- `docs/design/principia_debug_tooling_plan.md` §F: what the visible-set and locked/stale overlays mark per visible
  quad, each reading cited from the corpus (REQ-TOOL-166, R-425), with the "Removed lines" note; both overlays on the
  Tier 3 path.
- `crates/engine/src/debug/quad_meta_buffer.rs` (packed only while an overlay is on); `crates/render` Tier 3 views.
- `crates/engine/tests/quad_state.rs`: only listed transitions occur over scripted sessions.
- `cargo xtask bench tile-debug-off` (no upload or pass when off).

## Acceptance tests
- `cargo xtask bench tile-debug-off` — with every Tier 3 overlay off, no buffer upload or extra pass occurs (frame time equals the no-overlay baseline); on, the buffer is uploaded only while active (REQ-RENDER-055).
- Review checklist (code) — no overlay code path reads back SimState buffers (REQ-RENDER-056).
- Review checklist (physics) — debug_tooling_plan §F (or the scheduler contract it cites) gives the transition table the view checks against, listing every legal transition; the quad-state view's test asserts only listed transitions occur; physics reviewer approved; the doc change is in this PR and the physics reviewer approves it before merge (REQ-TOOL-116).
- `cargo test -p engine quad_state_transitions` — the test that follows from the written definition: debug_tooling_plan §F (or the scheduler contract it cites) gives the transition table the view checks against, listing every legal transition; the quad-state view's test asserts only listed transitions occur; physics reviewer approved (REQ-TOOL-116).
- Review checklist (physics) — debug_tooling_plan §F defines what the visible-set and the locked/stale overlays mark per visible quad, each from an existing corpus meaning where one exists ("stale" as `quad_state` 4, dd_generation_root §3.7a), cited, or saying none exists, with no new meaning where one exists; the doc change is in this PR and the physics reviewer approves it before merge (REQ-TOOL-166).
- Review checklist (qa) — render_contract Part 6's structural overlays row's visible-set and locked/stale cells are ticked against the Tier 3 overlays, each drawing exactly the quads its definition marks on a scripted session (REQ-TOOL-166).

## Notes
- Definitions (R-72) this task writes: REQ-TOOL-116, REQ-TOOL-166.
- The terminal state is tied to the true floors (scheduler Part 4), which M6 builds; the table lists them now and
  the M5 test exercises the transitions that exist in M5.
- RQ-239, decided per R-369 (7 Oct 2026): this task, not TASK-M1-13, builds the Tier-3 tile debug path; at M1 the fallback tint and the pending hatch are post occupants reading `ctx.quad`. Its per-visible-quad record is REQ-PERF-079's (TASK-M8-08).
- **R-425 (13 Oct 2026) rules RQ-276** (TASK-M1-19's pre-flight), option (b): "Both overlays move to TASK-M5-29 under
  a new defining requirement, which reuses existing corpus meanings where they exist (for example, quad state 4 for
  "stale") rather than defining new ones." The candidate readings RQ-276 listed (archived) are the task's to choose
  from: for "locked", the chart (tilt) lock (chart decoder contract, "The lock (projective microscope)") or caching
  contract Part 4's pinned class; for "visible-set", the live set (scheduler contract Part 7). The PR states which it
  takes, citing it. Each overlay's slot in the per-visible-quad record counts toward REQ-PERF-079's (TASK-M8-08).
