# TASK-M2-19 — Lookup: the exact and partial forms and the three-layer validation ladder

- **Milestone:** M2
- **Closes:** REQ-ENC-012, REQ-ENC-013, REQ-ENC-019, REQ-ENC-023, REQ-ENC-027, REQ-ENC-031
- **Depends on:** TASK-M2-17, TASK-M2-18
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~460 lines

## Goal
Lookup exists CPU-side: the exact-IC form ((m, n), a triple, a raw z, or physical (mᵢ, rᵢ, pᵢ) → encode into the latent chart → lock) and the partial-specification form (masses equal unless the chart implies Burrau, Burrau geometry = the locked ν or ν = 1/2, momentum = rest, latent = 0, then the slice basis spans the unspecified DOF). Lookup and lock validate in order — (1) hypercube bounds in the space written into the contract, (2) chart feasibility per chart, (3) decode sanity — preferring project, then clamp (`lookup_clamped`, showing the moved coordinates), then reject naming the failed constraint. There is no coincident-bodies rejection (R-13).

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*"
- `decisions.md` § "R-12 — The shape-sphere chart map stays as the markdown has it *(closes RQ-10)*"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/engine/src/lookup.rs` (the forms, the ladder, the outcome with its notices and moved coordinates).
- The inverse_encode_contract chart-aware-validation doc change naming layer 1's space and bounds, with a "Removed lines" note.
- Tests `crates/engine/tests/lookup.rs`; gate `cargo xtask gate decode-sanity` (fixture `fixtures/gates/decode_sanity/`).

## Acceptance tests
- `cargo test -p engine lookup_forms` — each exact form and a partial specification entered; the resulting lock and slice basis asserted (REQ-ENC-012).
- `cargo test -p engine lookup_validation_ladder` — one fixture per layer and per failure mode, in order; project preferred to clamp to reject; bodies within r_coll are not rejected (REQ-ENC-013).
- `cargo test -p engine lookup_coincident` — bodies within r_coll pass lookup unrejected; exactly coincident bodies → `lookup_clamped`; no separate rejection branch exists (REQ-ENC-019; the t = 0 collision label at dispatch is REQ-ENC-033, TASK-M3-09).
- Doc review (physics): layer 1 names its space and bounds consistently with z ∈ ℝ⁸ passing through σ / tanh; physics reviewer approved (REQ-ENC-023).
- `cargo xtask gate decode-sanity` — measures CoM and Σp residuals of decode at f64 and f32 over fuzzed z and proposes the layer-3 tolerance above them; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-ENC-027).
- Definition: the slice-basis rule for more than two unspecified DOF written into inverse_encode_contract Part 6 and approved by the physics reviewer (REQ-ENC-031).

## Notes
- Gap G16: the projection metric and the "qualitatively different IC" criterion.
- Gap G17: a partial specification that leaves more than two DOF unspecified — which two the slice basis spans.
- RQ-95 ruled: R-113 — REQ-ENC-019 is split: this task asserts only that lookup has no rejection branch; the t = 0 collision label at dispatch is REQ-ENC-033, closed at M3 with REQ-EVT-002 (TASK-M3-09). This settles Gap G24.
- Closes, for gaps the corpus leaves open: REQ-ENC-031 (R-72 definition) (classification accepted by R-132).
