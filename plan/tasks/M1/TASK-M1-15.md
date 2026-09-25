# TASK-M1-15 — The catalogue's standing checks: WGSL lints and the observation-tap property

- **Milestone:** M1
- **Closes:** REQ-RENDER-015, REQ-RENDER-018, REQ-COL-005, REQ-TOOL-028
- **Depends on:** TASK-M1-14
- **Needs (earlier milestones):** REQ-VAL-007
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~260 lines

## Goal
The rules the fragment side must keep from now on are checked in CI rather than by eye: no `isnan`/`isNan` and no NaN self-comparison (`x != x`) in generated or built-in WGSL — the debug NaN guard is the bitcast test (R-114) (validity via `has_<feature>` and the descriptor predicates; FTLE views guard with `has_ftle && ftle_valid`); colour occupants read payload fields only through the generated accessors and reference no chart, integrator or scheduler identifier; no render path reads the Benettin shadow (`r_sh`, `p_sh`) as a sample. And observation taps are proven passive: payload bytes are identical with every debug view and telemetry on versus off.

## References
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `decisions.md` § "R-114 — The debug NaN guard is the bitcast test *(closes RQ-82)*"

## Deliverables
- `xtask/src/lint_wgsl.rs` + `cargo xtask lint-wgsl` wired into CI: the isnan and self-comparison rule, the accessor-only rule (with the chart/integrator/scheduler identifier deny-list), the shadow rule.
- `crates/render/tests/observation_taps.rs` (proptest): payload-byte hash with all debug views and telemetry on vs off.

## Acceptance tests
- `cargo xtask lint-wgsl` — grep of generated and built-in WGSL for `isnan`/`isNan` and for a NaN self-comparison (`x != x`): zero hits; the debug guard is the bitcast test; FTLE views guard with `has_ftle && ftle_valid` (REQ-RENDER-015).
- `cargo xtask lint-wgsl` + review checklist (code): no render path reads `r_sh`/`p_sh` as a sample (REQ-RENDER-018).
- `cargo xtask lint-wgsl` — seam 5/6: colour WGSL has no non-accessor payload reads and no chart/integrator/scheduler identifiers (REQ-COL-005).
- `cargo test -p render observation_taps` (proptest) — payload bytes are identical with all debug views and telemetry on versus off (REQ-TOOL-028).

## Notes
- Each lint is shown able to fire (PIT-3): a fixture WGSL file with an `isnan`, a `raw != raw`, a raw payload read and an `r_sh` read must fail the lint.
- Open RQ-75 (REQ-RENDER-015): whether `has_ensemble` is a baked const or a uniform changes what the lint accepts as the ensemble guard.
- RQ-82 ruled: R-114 — the generated debug guard is the bitcast test; `raw != raw` is dropped (render_gui_spec §10.1 conformed in step 7), so the lint rejects it.
