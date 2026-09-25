# TASK-M5-18 — Resolve stage: ensemble spread, error_ratio and the co-computations

- **Milestone:** M5
- **Closes:** REQ-REF-002, REQ-REF-003, REQ-REF-004, REQ-REF-005, REQ-REF-010, REQ-PAY-061, REQ-RENDER-051, REQ-RENDER-053, REQ-INT-072, REQ-TOOL-133
- **Depends on:** TASK-M5-17
- **Needs (earlier milestones):** REQ-PAY-019, REQ-PAY-021, REQ-INT-039, REQ-SYS-019
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~460 lines

## Goal
The resolve pass derives the footprint spread from its E+1 samples and aggregates it into `QuadReduction`:
`spread_shape = spread(n̂)/2.0`, `spread_event = disagreement(class⊕detail)/(1 − 1/(E+1))`, `spread_winner`,
`ensemble_spread = max(spread_shape, spread_event)`, and `error_ratio = sigma_E(t)/sigma_E(0)` by max-deviation, NaN-safe,
`max`-aggregated, a boolean flag only. Every footprint keeps all E+1 copies (a failed copy raises spread); no conserved
quantity or other excluded field contributes; an undetermined shape spread is reported undetermined, never replaced by the
event value through a NaN-ignoring max. Outcome entropy H and σ²_T are computed at resolve (dd_integrator §3.8). Spread
is never a `SimState` field; the data side reads classified outcomes, never colours, and iterates the same sample indices
as the colour resolve.

## References
- `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors"
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `docs/design/principia_dd_simstate_payload.md` § "VERTICAL-SLICE ADDITIONS"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/notes/principia_sampling_msaa_note.md` § "Stability metrics: FTLE and diffusion are SimState fields; spread is a resolve-stage reduction"
- `docs/notes/principia_sampling_msaa_note.md` § "Amendments this makes"
- `docs/contracts/principia_scheduler_contract.md` § "Part 9 — Ensemble / SSAA sampling (dispatch rules)"
- `docs/notes/principia_sampling_msaa_note.md` § "Ensemble copies ARE the SSAA samples"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `docs/design/principia_dd_integrator.md` § "3.8 Co-computations (tier-gated, ride the forward pass)"
- `decisions.md` § "R-80 — Samples per footprint *(closes RQ-31)*"
- `decisions.md` § "R-18 — The agreement value is `spread_event` *(closes RQ-16)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/kernel/src/reduce/spread.rs` (shared source) and the resolve-stage entropy / σ²_T co-computation.
- Tests `crates/kernel/tests/reduce_spread.rs`: hand-computed footprints; the 2.9%-drift-among-8 case (≈ 204.8) and exact
  dynamics (1.0); injected non-finite copy; shape-NaN footprint; dd test 11 (basin interior H = 0, boundary H > 0).

## Acceptance tests
- `cargo test -p kernel spread_hand_computed` — hand-computed footprints (REQ-REF-002).
- `cargo test -p kernel error_ratio_max_deviation` — a copy at 2.9% energy drift among 8 yields a ratio ≫ 1 (≈204.8 in the measured case); exact dynamics → 1.0 (REQ-REF-003).
- `cargo test -p kernel spread_keeps_all_copies` — inject a non-finite copy: spread rises; copy count stays E+1 (REQ-REF-004).
- Review checklist (physics) — the spread reduction reads only shape and event (REQ-REF-005).
- `cargo test -p kernel spread_shape_nan_undetermined` — shape spread NaN → reduction reports undetermined, not the event value (REQ-REF-010).
- Review checklist (code) — no spread field in SimState; the resolve pass computes spread from the E+1 grouped samples and writes it into QuadReduction (REQ-PAY-061).
- Review checklist (code) — SimState has no spread field; the data-side spread path reads classified outcomes, not colours; QuadReduction carries ensemble_spread (REQ-RENDER-051).
- `cargo test -p kernel resolve_same_indices` — resolve and spread reductions iterate the same sample indices; QuadReduction holds spread_event f16 and no agreement field (REQ-RENDER-053).
- `cargo test -p kernel ensemble_entropy_sigma` — dd test 11: deep basin-interior pixel gives H = 0 and tiny σ²; boundary-straddling pixel gives H > 0 (REQ-INT-072; the copy offsets are REQ-INT-083, TASK-M4-06).
- Review checklist (qa) — the ensemble rows of render contract Part 6's field-view table are ticked against the generated catalogue on a real E ≥ 1 resolve (REQ-TOOL-133).

## Notes
- How `error_ratio` enters `ensemble_spread` is open in §3.7 ("how error_ratio enters is OPEN"); this task stores it
  as a flag only and does not join it to the max (see Gaps).
- REQ-REF-010 is a pitfalls §9 regression: `f64::max` returning the event value when the shape spread is NaN is a check
  whose output cannot show the failure.
- RQ-98 ruled: R-113 — REQ-INT-072 keeps H and σ²_T at resolve here; the copy offsets moved to M4 (REQ-INT-083, TASK-M4-06).
- RQ-94 ruled: R-113 — REQ-TOOL-011's tier-gated ensemble views are REQ-TOOL-133, closed here on a real resolve.
