# TASK-M2-16 — The round-trip suite: T2 per chart, T3 with the half-mirror catch, ε_phys

- **Milestone:** M2
- **Closes:** REQ-ENC-001, REQ-ENC-004, REQ-ENC-005, REQ-ENC-018, REQ-ENC-024, REQ-DEC-016, REQ-DEC-029, REQ-VAL-013, REQ-ENC-029
- **Depends on:** TASK-M2-14, TASK-M2-15
- **Needs (earlier milestones):** REQ-VAL-007, REQ-VAL-006
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-5, PIT-9, PIT-3
- **Size:** ~450 lines

## Goal
The encode contract's three theorems are asserted per chart, in physical units: T2 (right inverse on the interior) as ‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys with clamp-adjacent samples asserting `lookup_clamped`; T3 (left inverse modulo gauge) with the half-mirror catch — |L_z| sign consistent with `lookup_mirrored`, so a configuration-only mirror fails; decode(encode(x)) = C(x). No per-body momentum cap exists; q_max alone bounds the Jacobi momenta. ε_phys is proposed from the measured residual per chart at f64 and f32.

## References
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in z"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 8 — Round-trip contracts and the new debug view"
- `docs/design/principia_dd_encode.md` § "3.5 Conditioning (numbers, and the tolerance rule)"
- `docs/design/principia_dd_encode.md` § "5. Unit tests"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise"
- `docs/design/principia_dd_decoder.md` § "3.4 Momentum"
- `docs/design/principia_chart_reference.md` § "0.4 Canonicalisation `C`"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/read_first/principia_00_philosophy.md` § "8.2 A per-body momentum cap with CoM re-enforcement"
- `decisions.md` § "R-11 — The per-body momentum cap is rejected *(closes RQ-9)*"
- `docs/read_first/principia_01_pitfalls.md` § "5. MEASURING IN THE WRONG SPACE — `error(B)` in OKLab"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/kernel/tests/roundtrip.rs` (T2 per registered chart, T3, the controls), generated per chart from the chart registry so a new chart gets its T2 test automatically.
- Gate `cargo xtask gate encode-eps-phys` (fixture `fixtures/gates/eps_phys/`) and the calibration proposal.
- Encode's golden pairs (z = 0; Burrau (3,4,5)) asserted as fixtures.

## Acceptance tests
- `cargo test -p kernel encode_reuses_decode` — decode(encode(x)) equals canonicalise(x) within ε_phys (REQ-ENC-024, calibrated) on random states (REQ-ENC-001).
- `cargo test -p kernel t2_per_chart` — encode test 1 / decoder test 7 per chart: interior z → D → E → D, physical residual ≤ ε_phys; clamp-adjacent samples assert `lookup_clamped` instead (REQ-ENC-004).
- `cargo test -p kernel t3_half_mirror` — encode test 3: random scale, orientation, offset and parity; shape angles and masses match, E matches after the recorded rescale, |L_z| matches with sign consistent with `lookup_mirrored`; a configuration-only-mirror control fails the L_z check (REQ-ENC-005).
- `cargo test -p kernel t3_full_state_rigid_ops` — T3 with the half-mirror catch: random-gauge x → E → D equals C(x) with |L_z| sign consistent with `lookup_mirrored`; a config-only mirror (and a config-only rotation) must fail this check (REQ-DEC-016).
- `cargo test -p kernel encode_report` — encode results carry a discarded-quantities report; the T1–T3 tests pass (REQ-ENC-018).
- `cargo xtask gate encode-eps-phys` — measures the physical-unit round-trip residual over interior z per chart at f64 and f32 and proposes ε_phys above it; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-ENC-024).
- `cargo test -p kernel t2_qmax_boundary` — T2 round trip encode(decode(z)) = z over random z including the q_max boundary; decode has no per-body cap; clamped samples carry the flag (REQ-DEC-029).
- `cargo test -p kernel conditioning_physical_space` — encode test 8: a near-clamp input whose z-residual is ~10⁵× the physical residual passes the physical tolerance and would fail a naive z-tolerance (REQ-VAL-013).
- Definition: ‖·‖_phys written into inverse_encode_contract Part 4 and approved by the physics reviewer, before ε_phys (REQ-ENC-024) is proposed (REQ-ENC-029).

## Notes
- Gap G15: ‖·‖_phys (inverse_encode Part 4) isn't defined — which components of (m, r, p) and with what weighting. ε_phys's calibration presupposes it.
- PIT-5: every round-trip tolerance is in physical space; a z-space residual near saturation is not a criterion.
- Closes, for gaps the corpus leaves open: REQ-ENC-029 (R-72 definition) (classification accepted by R-132).
