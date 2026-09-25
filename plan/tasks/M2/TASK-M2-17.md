# TASK-M2-17 — Encode by axis kind: the invariant fibre reuses decode, feasibility first

- **Milestone:** M2
- **Closes:** REQ-ENC-008, REQ-ENC-009, REQ-ENC-010, REQ-CHART-005, REQ-ENC-030
- **Depends on:** TASK-M2-12, TASK-M2-14, TASK-M2-15
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
Encode into the active chart dispatches by axis kind: kind 1 reads the component; kind 2 applies the block inverse plus the axis's own residual convention (one convention, declared once, used both ways); kind 3 evaluates L_z(x), E(x), K(x) directly for chart coordinates and, for the latent point, runs the chart's own forward construction at the frozen configuration and free-momentum-inverts the constructed p (encode reuses decode; smallest-latent-norm only as the documented fallback); annotation axes have nothing to invert; otherwise latent z. Feasibility (|L_z| ≤ √(2I(E−U)), K ≥ L_z²/2I) is checked before construction, infeasible pairs go through project / clamp / reject with the flag surfaced, and a no-qualifying-seed configuration returns the pixel path's DEGENERATE.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 5 — Inverse by axis kind (completing the four kinds)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — The canonical inverse policy, completed"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/design/principia_dd_encode.md` § "3.1 Forward ↔ inverse pairing table — closures verified"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `docs/design/principia_dd_encode.md` § "5. Unit tests"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"

## Deliverables
- `crates/kernel/src/encode/by_kind.rs` (the axis-kind dispatch, the invariant fibre via the TASK-M2-09 construction, the residual conventions shared with the forward axes).
- Tests `crates/kernel/tests/encode_by_kind.rs`.

## Acceptance tests
- `cargo test -p kernel encode_axis_kinds` — a state round-trips through each axis kind; a state not aligned with any chart lands in latent z (REQ-ENC-008).
- `cargo test -p kernel encode_invariant_reuses_decode` — entering (L_z, E) numerically and clicking the corresponding pixel give identical z to precision (REQ-ENC-009).
- `cargo test -p kernel encode_infeasible_invariant` — encode test 6: (L_z, E) with K* < L_z²/2 is projected, clamped or rejected and flagged, never silent, never NaN; a degenerate-seed configuration returns the pixel path's DEGENERATE (REQ-ENC-010).
- `cargo test -p kernel encode_residual_convention` — encoding 'm₀ = 0.4' under hold-m₁:m₂ and under hold-m₁=m₂ gives different z, each round-tripping only under its own convention (REQ-CHART-005).
- Definition: the nearest-valid-point metric and the qualitatively-different-IC criterion written into inverse_encode_contract's chart-aware validation and approved by the physics reviewer (REQ-ENC-030).

## Notes
- Gap G16: "project … to the nearest valid point" (onto the parabola for (L_z, E)) has no metric, and "only when projecting or clamping would give a qualitatively different IC" has no criterion (inverse_encode chart-aware validation).
- Closes, for gaps the corpus leaves open: REQ-ENC-030 (R-72 definition) (classification accepted by R-132).
