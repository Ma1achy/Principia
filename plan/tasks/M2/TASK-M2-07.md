# TASK-M2-07 — Canonicaliser C: the ordered procedure, the mirror tie and the scale gauge

- **Milestone:** M2
- **Closes:** REQ-DEC-002, REQ-DEC-017, REQ-DEC-018, REQ-DEC-023, REQ-ENC-015, REQ-ENC-006, REQ-ENC-016
- **Depends on:** TASK-M2-06
- **Needs (earlier milestones):** REQ-PAY-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~420 lines

## Goal
One canonicalise function in the shared source quotients any physical state onto the decode's section, in the normative order (R-23): 1a subtract CoM → 0 rescale to I = 1 (λ = I_in^{−1/2} recorded, `lookup_rescaled`) → 1b remove the boost → 2 rotate ρ̃ to +x (ρ̃ = 0 → `lookup_clamped`, R-13) → 3 mirror iff λ̃_y < −δ_λ (`lookup_mirrored`; |λ̃_y| ≤ δ_λ → no mirror, R-82), every rigid operation applied to every rᵢ and every pᵢ. The scale gauge of §3.5 is the same similarity map. On canonical-frame decodes C is a no-op away from the λ̃_y = 0 seam; both precisions run the identical sequence.

## References
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/design/principia_core_design.md` § "5. Canonicalisation is the one seam to (m, r, p)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise"
- `docs/design/principia_chart_reference.md` § "0.4 Canonicalisation `C`"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/design/principia_dd_decoder.md` § "3.5 Scale gauge"
- `docs/design/principia_chart_reference.md` § "0.5 Scale gauge"
- `docs/design/principia_dd_encode.md` § "6. Deferred / flagged"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 2 — The scale gauge gap (missing from the original inverse policy)"
- `decisions.md` § "R-23 — Encode order: CoM subtraction precedes `I` *(CD-3)*"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_dd_encode.md` § "5. Unit tests"
- `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"

## Deliverables
- `crates/kernel/src/canon.rs`: `canonicalise` (the ordered steps, the three notices, λ recorded) and the scale gauge.
- A physical Burrau (3,4,5) state fixture built from chart_reference §4.2 at 2× scale, `fixtures/states/burrau_2x.json`.
- Tests in `crates/kernel/tests/canon.rs`; the GPU-f32 branch checks through the TASK-M2-06 test dispatch (`crates/engine/tests/canon_gpu.rs`).

## Acceptance tests
- `cargo test -p kernel canonicalise_section` — property: on random physical states the output has CoM at the origin, ρ̃ on +x, λ̃_y ≥ −δ_λ and I = 1, and canonicalise is idempotent (REQ-DEC-002).
- `cargo test -p engine mirror_tie` — states with λ̃_y at exactly −δ_λ, at +δ_λ and straddling both: no mirror for |λ̃_y| ≤ δ_λ, mirror just below −δ_λ; the same output twice and on the CPU-f64 and GPU-f32 builds; the test reads λ̃_y, not λ_y; the T1 sweep in TASK-M2-15 includes deadband-straddling rotations (REQ-DEC-017).
- `cargo test -p kernel canonical_noop` — decoder test 5: C applied to canonical-frame decodes of fuzzed z leaves them unchanged; at the seam the R-82 tie gives the same result twice (REQ-DEC-018).
- `cargo test -p kernel scale_gauge` — decoder test 10: applied to canonical output it is the identity; applied to a scaled input it yields I = 1 and transforms E, L_z per the encode contract's table (REQ-DEC-023).
- `cargo test -p engine canonicalise_order` — an off-CoM, boosted, scaled, rotated state: I is computed after CoM subtraction and the result equals the canonical representative on both backends (REQ-ENC-015).
- `cargo test -p kernel scale_step` — encode test 4: the Burrau state at 2× scale gives I_in = 4·I_canon, λ = 1/2, E_canon = 2·E_in, timing factor I_in^{3/4}; the whole table row is asserted, and `lookup_rescaled` carries λ (REQ-ENC-006).
- `cargo test -p engine coincident_inner_pair` — encode test 9: ρ → 0 inputs hit the step-2 guard with the `lookup_clamped` reason on CPU-f64 and GPU-f32, same branch (REQ-ENC-016).

## Notes
- The mirror test reads λ̃_y = √μ_λ·λ_y (encode's frame), the one test R-82 kept; a λ_y test is the defect.
- Branch inputs (the mirror comparison, the ρ = 0 guard) are comparisons against constants on single-rounded values (R-34, R-84); PIT-10 — the cross-backend claim is for identical inputs only.
