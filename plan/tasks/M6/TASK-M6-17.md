# TASK-M6-17 — The arbiter: one writer, the throughput model, the ladder and the lever order

- **Milestone:** M6
- **Closes:** REQ-PERF-056, REQ-PERF-057, REQ-PERF-058, REQ-PERF-065, REQ-PERF-046, REQ-PERF-047, REQ-PERF-049, REQ-PERF-036, REQ-GUI-013
- **Depends on:** TASK-M6-02, TASK-M6-16
- **Needs (earlier milestones):** REQ-SCHED-046, REQ-PERF-016, REQ-PERF-021, REQ-RENDER-042, REQ-RENDER-052, REQ-SCHED-020
- **Reviewers:** code, qa, perf, gui
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
The live quality controller exists for Auto: a single arbiter owns every write to `QualitySettings`; sensors only update one throughput model; knobs are a pure function of the model and frame budget; the output is one integer rung on an offline ladder of ~8–12 complete presets with the six named tiers pinned on it. Under pressure levers move in memory_tiers §5's order (render_scale, E, refinement floor, then FTLE, word), memory is the hard clamp and compute soft; FTLE and word change only at natural invalidation moments while E and the refinement floor move live; `e_motion_gating` drops E to ≤ 1 during a march when unaffordable. Lock-to-native (default on) pins `render_scale = 1.0` and removes it from the levers. Named tier or Custom switches the arbiter off entirely.

## References
- `docs/design/principia_quality_device_note.md` § "1. One arbiter, many sensors"
- `docs/design/principia_quality_device_note.md` § "2. Model-based, not reactive"
- `docs/design/principia_quality_device_note.md` § "3. The quality ladder"
- `docs/design/principia_quality_device_note.md` § "Open sub-questions (settle at implementation)"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_quality_device_note.md` § "10. Two sanctities: the user, and observability"
- `docs/design/principia_quality_device_note.md` § "The reframe: quality is a preset selector populating one settings struct"
- `docs/design/principia_quality_device_note.md` § "6. Change under cover (the perceptual magic)"
- `docs/design/principia_memory_tiers.md` § "6. Auto-mode tier selection"
- `docs/design/principia_quality_device_note.md` § "5. Timescale cascade (each layer absorbs what's too fast for the one above)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 9 — Ensemble / SSAA sampling (dispatch rules)"
- `docs/design/principia_quality_device_note.md` § "What this subsystem resolves (two previously-open questions)"
- `docs/design/principia_temporal_architecture_note.md` § "Still open (settle at implementation, or next edit pass)"
- `docs/design/principia_memory_tiers.md` § ""Lock to native" toggle"

## Deliverables
- `crates/engine/src/quality/arbiter.rs`: `Arbiter` (sole writer, via a capability token other modules cannot construct), `ThroughputModel`, `rung_for(model, budget)`.
- `crates/engine/src/quality/ladder.rs`: the ladder as a data table (monotone cost asserted at load), named tiers as pinned rungs.
- Lever sequencing with lock-to-native; `e_motion_gating`; the refinement-floor lever driving TASK-M6-02's motion floor; deferral queue for sim-key rung components.

## Acceptance tests
- Review checklist (code) — only the arbiter module has write access to QualitySettings in auto mode (REQ-PERF-056).
- `cargo test -p engine model_pure_function` — same model value → same rung; sensor updates change the model, never knobs directly (REQ-PERF-057).
- Review checklist (perf) + `cargo test -p engine ladder_monotone` — the ladder is a data table of ~8–12 rungs containing the six named tiers; cost is monotone across rungs; the policy returns an integer (REQ-PERF-058).
- `cargo test -p engine named_tier_arbiter_off` — under load with High selected, no knob changes (REQ-PERF-065).
- `cargo test -p engine lever_order` — simulated pressure sequence shows the lever order; memory estimate never exceeds the ceiling while compute overrun only lowers rungs (REQ-PERF-046).
- `cargo test -p engine sim_key_levers_deferred` — a controller-driven FTLE/word change is deferred until the next invalidation event; E drops under motion leave nominal payloads intact (REQ-PERF-047).
- `cargo test -p engine live_levers_within_ceiling` — scripted motion: levers drop and restore; memory estimate stays ≤ ceiling throughout (REQ-PERF-049).
- `cargo test -p engine e_motion_gating` — with a low-throughput model, E drops to ≤ 1 during a march and returns to full E at rest and in export (REQ-PERF-036).
- `cargo test -p engine lock_to_native` + `cargo test -p gui render_scale_slider_disabled` — with lock on, the controller under load never changes render_scale; slider is disabled (REQ-GUI-013).

## Notes
- The ladder's rung presets are set by REQ-PERF-038's measurement (TASK-M6-16); this task ships the table with the named-tier rows and labelled placeholder rungs between them.
