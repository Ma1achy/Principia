# TASK-M1-15 — The catalogue's standing checks: WGSL lints and the observation-tap property

- **Milestone:** M1
- **Closes:** REQ-RENDER-015, REQ-RENDER-018, REQ-COL-005, REQ-TOOL-028, REQ-RENDER-086, REQ-COL-069
- **Depends on:** TASK-M1-14
- **Needs (earlier milestones):** REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~1,300 lines

## Goal
The rules the fragment side must keep from now on are checked in CI rather than by eye: no `isnan`/`isNan` and no NaN self-comparison (`x != x`) in generated or built-in WGSL — the debug NaN guard is the bitcast test (R-114) (validity via `has_<feature>` and the descriptor predicates; FTLE views guard with `has_ftle && ftle_valid`, and every aggregate over `ftle` excludes samples by `ftle_valid`, never by NaN propagation, R-255); colour occupants read payload fields only through the generated accessors and reference no chart, integrator or scheduler identifier; no render path reads the Benettin shadow (`r_sh`, `p_sh`) as a sample. And observation taps are proven passive: payload bytes are identical with every debug view and telemetry on versus off. The terms the lint checks are R-72 definitions, written in the render contract's Part 4 and reviewed by physics.

## References
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `decisions.md` § "R-114 — The debug NaN guard is the bitcast test *(closes RQ-82)*"
- `decisions.md` § "R-145 — The fragment side reads `has_ensemble` as a uniform *(closes RQ-75)*"
- `decisions.md` § "R-255 — Every aggregate over `ftle` excludes samples by `ftle_valid`, never by NaN propagation *(condition on R-254)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- The R-72 definitions (REQ-RENDER-086, REQ-COL-069): the render contract's Part 4, "The occupant rules `cargo xtask
  lint wgsl` checks", with pointer notes in dd_colouring §4 and dd_generation_root §3.5.
- `xtask/src/lint_wgsl.rs` and its `occupant` module, in `cargo xtask lint wgsl`, which CI already runs (R-351, R-352:
  the rules join `lint wgsl`, not a new subcommand): the isnan and self-comparison rules it has; the shadow rule; the
  accessor-only rule; the chart/integrator/scheduler identifier deny-list; the FTLE and ensemble guard rule, aggregates
  included (R-255); and the exceptions the definitions name, each to its one rule.
- The WGSL written at runtime (`render::colour::field_ramp`'s field ramps, `engine::presets`' occupants), checked in
  under `crates/render/frag/runtime/` and kept equal to its writers by `crates/render/tests/runtime_wgsl.rs`, so that
  the lint covers it: xtask depends on no workspace crate but ledger, and nothing depends on xtask (systems_architecture
  §7.1). A field ramp of a tier-gated field guards with the tier's `has_<feature>` beside the gate
  (`render::bind::validity_over`).
- `crates/render/tests/observation_taps.rs` (proptest): payload-byte equality with all debug views and telemetry on vs
  off.

## Acceptance tests
- `cargo xtask lint wgsl` — generated and built-in WGSL, the checked-in runtime WGSL among it, has zero `isnan`/`isNan` and zero NaN self-comparisons (`x != x`); the debug guard is the bitcast test; FTLE views guard with `has_ftle && ftle_valid`; every aggregate over `ftle` excludes samples by `ftle_valid`, not by NaN propagation or `min`/`max`/`clamp` on NaN (R-255); fixtures: an unguarded, a gate-only and a `has_ftle`-only read, and an aggregate relying on NaN propagation, each fail (REQ-RENDER-015).
- `cargo xtask lint wgsl` + review checklist (code): no render path reads `r_sh`/`p_sh` as a sample; a fixture reading `ctx.sample.r_sh` fails (REQ-RENDER-018).
- `cargo xtask lint wgsl` — seam 5/6: colour WGSL has no non-accessor payload reads and no chart/integrator/scheduler identifiers; fixtures reading a payload buffer, decoding `ctx.sample.word` by hand, and reading a chart, an integrator and a scheduler identifier each fail (REQ-COL-005).
- `cargo test -p render observation_taps` (proptest) — payload bytes are identical with all debug views and telemetry on versus off (REQ-TOOL-028).
- `cargo test -p render runtime_wgsl` — the checked-in runtime WGSL equals what its writers emit.
- Review checklist (physics): the render contract's Part 4 defines reading the shadow as a sample (REQ-RENDER-086) and the occupant rules' terms, each exception with its one rule (REQ-COL-069); physics reviewer approved.

## Notes
- Each lint is shown able to fire (PIT-3): a fixture WGSL file with an `isnan`, a `raw != raw`, a raw payload read and an `r_sh` read must fail the lint.
- RQ-75 ruled: R-145 — `has_ensemble` is a uniform, so the lint accepts a guard on the uniform as the ensemble guard (REQ-RENDER-015).
- RQ-82 ruled: R-114 — the generated debug guard is the bitcast test; `raw != raw` is dropped (render_gui_spec §10.1 conformed in step 7), so the lint rejects it.
- Applied per R-369 (the orchestrator's dispatch): the subcommand is `cargo xtask lint wgsl` (R-351 item 12, accepted
  by R-352); physics reviews, for the R-72 definitions; the FTLE guard binds the occupants that are not debug views,
  which keep R-114's bitcast guard (R-79's debug exception); R-255's aggregate clause, from REQ-RENDER-015's verify
  line, is in the acceptance, shown on fixtures since no fragment-side aggregate over `ftle` exists yet; the runtime
  WGSL is checked in and linted by xtask, its writers out of xtask's reach (systems_architecture §7.1); the size grows
  from ~260 to match the work.
