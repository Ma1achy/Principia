# TASK-M7-15 — The render key and render freedom

- **Milestone:** M7
- **Closes:** REQ-RENDER-060, REQ-COL-010
- **Depends on:** TASK-M7-12, TASK-M5-12
- **Needs (earlier milestones):** REQ-RENDER-075, REQ-RENDER-030, REQ-GEN-017, REQ-RENDER-042
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-5
- **Size:** ~300 lines

## Goal
The frame tier is keyed by a render key hashing the stain graph in its canonical form (nodes, their sources, wires, per-node params), the uniform values and the overlay set: changing any of them re-composites, changing none reuses the frame. Everything in the colour subsystem except Appendix A's bring-up mode is render-key — including the UV, DECODE and ROUNDTRIP fragment presets (R-75) — so no colour edit invalidates the survey cache or re-integrates.

## References
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `docs/design/principia_colour_composition.md` § "0. Scope & membrane position"
- `docs/design/principia_colour_composition.md` § "Appendix A — kernel-side bring-up mode"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"

## Deliverables
- `crates/engine/src/render_key.rs` — the render key over the canonical graph form (REQ-RENDER-075), uniforms and overlay set; the frame-tier reuse check in the frame loop.
- `crates/engine/tests/render_freedom.rs` — the render-freedom property test (dd_colouring unit test 12).

## Acceptance tests
- `cargo test -p engine render_key` — changing any of the graph's nodes, sources, wires, per-node params, the uniform values or the overlay set changes the render key and re-composites; changing none reuses the frame (REQ-RENDER-060).
- `cargo test -p engine render_freedom` (proptest) — at a paused playhead, edits to every colour node and selecting the UV / DECODE / ROUNDTRIP presets leave the sim key and sim-buffer hash unchanged and issue no dispatch (REQ-COL-010).

## Notes
- PIT-5: colour stays downstream of the criterion; no colour-space quantity may reach the sim key or refinement.
- Open RQ-75 (the fragment side's baked `has_ensemble`) decides whether E = 0 ↔ E > 0 is a render-key change; the test must follow its ruling.
