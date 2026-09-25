# TASK-M2-05 — The Chart trait, ValidationResult and the latent affine chart

- **Milestone:** M2
- **Closes:** REQ-CHART-003, REQ-CHART-028, REQ-CHART-043, REQ-CHART-030, REQ-CHART-015, REQ-CHART-029, REQ-CHART-050
- **Depends on:** TASK-M2-03, TASK-M1-07
- **Needs (earlier milestones):** REQ-SYS-009
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~380 lines

## Goal
A chart is a map Φ : [0,1]² → Y followed by the shared D and C, behind one trait: `map(u, v) -> ChartOut`, `forbids_energy_normalisation()`, `name()` (written in every dump header) and `validate(u, v) -> ValidationResult` (R-26), with ValidationResult's variants written into chart_reference §5.1. The first chart, `Latent { z0, q1, q2 }`, is the affine slice z(s,t) = z₀ + (2s−1)q₁ + (2t−1)q₂ with its scale in q (R-83): no per-axis factors. D and C are written once; no chart carries decode formulae. The chart build order of chart_reference §5.1 is the order of this milestone's tasks.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `docs/design/principia_chart_reference.md` § "Chart reference — the maths, for implementation"
- `docs/design/principia_chart_reference.md` § "1.1 Affine slices — the "no tilt" case"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/contracts/principia_canonical_spec.md` § "4. Charts & navigation *(authoritative: `chart_decoder_contract`)*"
- `docs/design/principia_systems_architecture.md` § "0. The ladder — the organising abstraction"
- `decisions.md` § "R-83 — The slice scale lives in q *(closes RQ-34)*"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"

## Deliverables
- `crates/kernel/src/chart/mod.rs` (the `Chart` trait, `ChartOut`, `ValidationResult`) and `crates/kernel/src/chart/latent.rs`.
- The chart_reference §5.1 doc change listing the ValidationResult variants (project / clamp / reject and the failed constraint), with a "Removed lines" note.
- A dump-header writer that stamps `name()`; tests in `crates/kernel/tests/chart_latent.rs`.

## Acceptance tests
- `cargo test -p kernel latent_affine` — z(½, ½) = z₀ and the corners equal z₀ ± q₁ ± q₂; zoom scales q₁ and q₂ by one common factor; the chart has no s_u, s_v parameters (REQ-CHART-003).
- Review (code): the trait has the four methods; every chart implements `validate`; dumps carry the chart name in their header (REQ-CHART-028).
- Doc review (physics): chart_reference §5.1 lists the ValidationResult variants, covering project / clamp / reject and the failed constraint; physics reviewer approved (REQ-CHART-043).
- `cargo test -p kernel latent_axis_aligned_equals_alpha_beta` — the slice with q₁ = ê_α, q₂ = ê_β, compared per pixel against ICs built directly from (α, β) through chart_reference §0.2 (REQ-CHART-030).
- Review (physics): no chart module contains mass/config/momentum decode formulae or integrator code; later charts go through the shared decoder (REQ-CHART-015).
- Review (code): the milestone builds the charts in chart_reference §5.1's order — Latent (this task), ShapeSphere (TASK-M2-08), Burrau (TASK-M2-10, TASK-M2-11), InvariantLE / InvariantLK (TASK-M2-12), MassSimplex (TASK-M2-13) (REQ-CHART-029).
- Definition: the direct (α, β) sweep written into chart_reference §5.2 and approved by the physics reviewer (REQ-CHART-050).

## Notes
- Gap G12: "a direct (α, β) sweep" (chart_reference §5.2) isn't defined further — whether it maps u, v to (α, β) through the links or linearly in angle; the test above compares against the link-mapped construction until ruled.
- Gap G13: chart_reference §5.1 writes `map(&self, u: f64, v: f64)`; the kernel monomorphises Φ into the f32 SPIR-V build (lowering Part 3), so Φ must be generic over `Real`. `validate` stays CPU-side (f64).
- Waits on RQ-86 (`REVIEW_QUEUE.md`): The Chart trait's f64 `map` vs Φ generic over the float type.
- Closes, for gaps the corpus leaves open: REQ-CHART-050 (R-72 definition) (REVIEW_QUEUE RQ-110 lists them for the human).
