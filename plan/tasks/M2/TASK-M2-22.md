# TASK-M2-22 — Navigation as chart construction: pan, slice, zoom, tilt and the direction library

- **Milestone:** M2
- **Closes:** REQ-GUI-002, REQ-GUI-003, REQ-CHART-009, REQ-CHART-016, REQ-SYS-014
- **Depends on:** TASK-M2-05, TASK-M2-10, TASK-M2-12
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-009
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-9
- **Size:** ~420 lines

## Goal
Every navigation gesture is a CPU-side edit of the one chart triple: pan as Δz₀ ∈ span(q₁, q₂), slice as Δz₀ ⊥ span(q₁, q₂) (z₀ ← z₀ + t·d), zoom as a common log-stepped scale of q₁, q₂, tilt as q'(τ) = normalise(cos τ·q + sin τ·d_target), τ ∈ [−90°, +90°], sign significant, offering the six directions orthogonal to the plane plus the named compound directions (mass away from Burrau, energy at fixed L_z, the Burrau-to-unconstrained morph), each evaluated at and recomputed with the chart centre. An oblique slice records its orthonormal (q₁, q₂); raw tilts replace rather than compose. UV addressing stays unsigned [0,1]² and IC space is signed and centred on z₀.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"
- `docs/contracts/principia_chart_decoder_contract.md` § "Slice (centre edit, orthogonal component)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Directions are axis kinds — the unifying rule"
- `docs/design/principia_chart_reference.md` § "4.6 The central hypothesis"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `decisions.md` § "R-83 — The slice scale lives in q *(closes RQ-34)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 4 — Navigation is chart construction (pan, slice, zoom, tilt, lock)"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"

## Deliverables
- `crates/engine/src/nav/` (the operations as typed `SetField` edits of `SimConfig`'s z₀, q₁, q₂; the direction library).
- Tests `crates/engine/tests/nav.rs`, `crates/kernel/tests/uv_ic_space.rs`.

## Acceptance tests
- `cargo test -p engine nav_ops` — after a slice step every pixel's IC changes by the identical hidden-coordinate step; pan keeps the plane; zoom keeps z₀ (REQ-GUI-002).
- `cargo test -p engine tilt` — τ = 0 leaves q unchanged; τ = ±90° gives ±d_target; +30° and −30° give different planes; a curve axis tilts its tangent at the centre (REQ-GUI-003).
- `cargo test -p engine named_directions` — the direction library returns the three vectors; moving the centre changes the energy-at-fixed-L_z vector; the morph supports the chart_reference §4.6 persistence test (REQ-CHART-009).
- `cargo test -p engine oblique_slice_record` — an oblique slice's exported state contains q₁, q₂; applying two raw tilts yields the second, not the composition (REQ-CHART-016).
- `cargo test -p kernel uv_ic_space` — UV (½, ½) → z₀; UV (0, 0) → z₀ − q₁ − q₂; addresses are non-negative at any z₀ (REQ-SYS-014).

## Notes
- Gap G19: the named compound directions aren't given as vectors — the energy-at-fixed-L_z direction as a latent vector (a tangent of which curve, pulled back through which link, normalised how), the mass direction's normalisation, and the morph's target direction.
- The zoom step factor for "log-stepped" is not needed by these tests; the GUI binding lands at M8.
