# TASK-M8-08 — Overlays ▾: the grouped toggles, their tiers, the Tier-3 buffer and quad drill-down

- **Milestone:** M8
- **Closes:** REQ-GUI-079, REQ-RENDER-074, REQ-PERF-076, REQ-PERF-079, REQ-PERF-080, REQ-GUI-141, REQ-GUI-153
- **Depends on:** TASK-M8-05, TASK-M4-14, TASK-M5-05
- **Needs (earlier milestones):** REQ-RENDER-024, REQ-RENDER-055, REQ-TOOL-026, REQ-TOOL-045, REQ-CHART-039, REQ-SYS-013, REQ-SCHED-047
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
Overlays ▾ holds §G2's grouped toggles (Quadtree, Integration, Chart, Stain) with Alt+digit shortcuts, all off and save as default, editing RenderState's overlay set. Every overlay is implemented in the tier §12.1's rule assigns it: a post node (Tier 1), a sim shader (Tier 2), or a tile debug shader fed by the per-visible-quad Tier-3 buffer. The Tier-3 record is defined in §12.1 and the buffer fits the ~16 KB budget for the calibrated typical viewport; finer detail goes to the quad drill-down (click a quad → its metadata as text). Where the overlay default set and the saved views are stored is defined in §G2 and §G9.

## References
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"

## Deliverables
- `crates/gui/src/explore/overlays_menu.rs` — the menu, counts, Alt+digit bindings.
- `crates/render/src/overlays/tiers.rs` — the overlay → tier table; Tier-3 tile debug shaders under `crates/render/wgsl/frag/debug/tile/`.
- `crates/engine/src/overlay_buffer.rs` — the per-visible-quad Tier-3 record, packed CPU-side from scheduler verdicts, uploaded only while a Tier-3 overlay is on.
- `crates/gui/src/explore/quad_drilldown.rs` — click a quad: depth, generation, refine decision, residency.
- Doc changes: `docs/gui/principia_render_gui_spec.md` §12.1 (the Tier-3 record's fields and size), §G2 and §G9 (the two stores).
- Calibration proposal (PR description): visible-quad counts over the deep-zoom scenarios and the resulting buffer size.

## Acceptance tests
- `cargo xtask screenshot 01_main` (open menu) and `cargo test -p gui overlay_alt_digits` — screenshot of the open menu against 01_main.png; each Alt+digit toggles its overlay; 'all off' clears the overlay set (REQ-GUI-079).
- Review checklist (gui reviewer) — a table of every Overlays-menu and Display-window overlay with its tier and the §12.1 question that placed it; leaf/internal is not offered (REQ-RENDER-074).
- `cargo test -p engine tier3_buffer_budget` (≤ 16 KB on the typical-viewport fixture of REQ-PERF-080, calibrated) — buffer size for the typical-viewport fixture ≤ 16 KB (REQ-PERF-076).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders" — §12.1 gives the record's fields and size; the reviewer checks it holds the scheduler verdicts the Tier-3 overlays read; the physics reviewer approves the doc change before merge (REQ-PERF-079).
- `cargo xtask bench tier3-visible-quads` — the proposal shows visible-quad counts measured over the deep-zoom scenarios and the resulting buffer size with the defined per-quad record; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-PERF-080).
- `cargo test -p gui quad_drilldown` — click a quad: depth, generation, refine decision and residency are listed (REQ-GUI-141).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)" and § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)" — §G2 and §G9 name each store; the reviewer checks neither is engine state the engine reads; the physics reviewer approves the doc change before merge (REQ-GUI-153).

## Notes
- The Stain group's overlays (class edges, t_end contours) also appear in the Display window (TASK-M8-23); both edit the same RenderState overlay set.
- Calibrations (R-71) proposed here: REQ-PERF-080. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- Definitions (R-72) written here: REQ-PERF-079, REQ-GUI-153. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
