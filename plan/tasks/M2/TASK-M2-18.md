# TASK-M2-18 — Burrau encode: on-curve closed form, off-curve projection and the fold

- **Milestone:** M2
- **Closes:** REQ-ENC-011, REQ-ENC-017, REQ-ENC-020, REQ-ENC-026, REQ-ENC-022
- **Depends on:** TASK-M2-11, TASK-M2-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
Kind-4 encode exists for the Burrau curve: on-curve, recover the acute angle from the canonical configuration, set ν(θ) = sec θ − tan θ and verify the mass coupling μ₁ = log(b/c), μ₂ = log(a/c); off-curve, project by ν* = argmin_ν ‖n(α,β) − n(α_γ(ν), β_γ(ν))‖² + ‖m − m_γ(ν)‖² (shape-sphere chordal ⊕ mass-simplex Euclidean, unit weights, R-24) with a Brent/golden-section-grade 1-D minimiser, report d(ν*), and fall back to latent z beyond the tolerance. What encode does at θ > π/4 on the folded chart is written into dd_encode §3.1, and the projection-distance tolerance is proposed with evidence.

## References
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 5 — Inverse by axis kind (completing the four kinds)"
- `docs/design/principia_dd_encode.md` § "3.1 Forward ↔ inverse pairing table — closures verified"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_encode.md` § "3.4 Curve projection (off-curve input) — metric pinned"
- `docs/design/principia_dd_encode.md` § "6. Deferred / flagged"
- `decisions.md` § "R-24 — The encode projection metric is confirmed *(CD-4)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"

## Deliverables
- `crates/kernel/src/encode/curve.rs` (on-curve closed form, the 1-D minimiser, the latent fallback).
- The dd_encode §3.1 doc change (the θ > π/4 behaviour and its flag), with a "Removed lines" note.
- Tests `crates/kernel/tests/encode_burrau.rs`; the d(ν*) distributions in `fixtures/gates/curve_projection/`.

## Acceptance tests
- `cargo test -p kernel encode_burrau_on_curve` — encode test 5: physical (3, 4, 5) → masses (5, 4, 3)/12, right angle recovered, ν matches ν(θ) = sec θ − tan θ; decode back → the same triangle to ε_phys (REQ-ENC-024) (REQ-ENC-011).
- `cargo test -p kernel encode_curve_projection` — encode test 7: on-curve input — argmin and closed form agree on ν; off-curve — d(ν*) reported, latent fallback taken beyond the tolerance (REQ-ENC-026, calibrated) and the fallback still satisfies T2 (REQ-ENC-017).
- `cargo test -p kernel encode_projection_metric_bruteforce` — off-curve fixture: the projected point minimises the chordal ⊕ Euclidean unit-weight metric, compared against a brute-force search (REQ-ENC-020).
- Calibration: d(ν*) distributions for on-curve and off-curve fixtures under the R-24 metric, with the proposed fallback tolerance; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-ENC-026).
- Doc review (physics): dd_encode §3.1 states the θ > π/4 behaviour (relabel, latent fallback or refusal) and the flag it raises; physics reviewer approved (REQ-ENC-022).

## Notes
- The brute-force comparison is the control that shows the projection test can fail (PIT-3).
