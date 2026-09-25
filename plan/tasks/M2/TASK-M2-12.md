# TASK-M2-12 — The invariant charts (L_z, E) and (L_z, K): the warp, the η_E refusal and the gradient gate

- **Milestone:** M2
- **Closes:** REQ-CHART-017, REQ-CHART-044, REQ-CHART-031, REQ-VAL-015
- **Depends on:** TASK-M2-04, TASK-M2-06, TASK-M2-09
- **Needs (earlier milestones):** REQ-PAY-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~400 lines

## Goal
`InvariantLE` and `InvariantLK` fix geometry and mass and map the unit square onto the feasible interior by the warp K(t) = K_max·t^γ_K, E(t) = U + K(t), L_max(t) = √(2·I·K(t)), L_z(s, t) = (2s − 1)·L_max(t) (K* = K(t) directly on (L_z, K)), feeding the TASK-M2-09 construction. Both declare `forbids_energy_normalisation`, and the validation pass refuses, in code, any configuration combining them with an energy-normalisation override. The invariant-chart gradient view — E_0 on (L_z, E) — is a perfect axis-aligned gradient, checked as a gate on the decode stage's output. K_max and γ_K are proposed with evidence.

## References
- `docs/design/principia_chart_reference.md` § "2. Invariant-momentum charts `(Lz, E)` and `(Lz, K)`"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-25 — `η_E` is kept, with an explicit off switch *(CD-5)*"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"

- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"
## Deliverables
- `crates/kernel/src/chart/invariant.rs` (both charts; frozen configuration and mass as chart params).
- The refusal in the CPU validation pass (`crates/engine/src/chart_validate.rs`), tested.
- Gate `cargo xtask gate invariant-gradient` reading E₀ = K_0 + V_0 from the GPU decode stage (TASK-M2-06); the K_max / γ_K proposal in `fixtures/gates/invariant_warp/`.

## Acceptance tests
- `cargo test -p kernel invariant_warp_feasible` — property: no (u, v) in [0,1]² produces K* < K_min under the warp, on both charts (chart_reference §5.2) (REQ-CHART-017).
- Calibration: K_max (> 0) and γ_K (≥ 1) proposed, showing the (L_z, E) and (L_z, K) charts at the defaults covering the feasible interior with usable resolution near K_min; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-CHART-044).
- `cargo test -p engine forbids_energy_normalisation` — a config combining (L_z, E) with an E* override is refused and the test asserts the refusal, for `Some(0)` as for any other `Some(E*)` (R-25, applied by R-124); the same for (L_z, K) (REQ-CHART-031).
- `cargo xtask gate invariant-gradient` — on an (L_z, E) chart, E_0 is constant along the L_z axis and monotone along E to within f32 rounding of the target; a control with a perturbed construction fails the gate (REQ-VAL-015).

## Notes
- Gap G8: "fix geometry and mass" — where the frozen configuration and masses come from (z₀'s blocks, the lock, chart params) isn't stated; U is the potential at that configuration.
- R-133: the evidence renders show decode-time fields (feasibility, K, L_z coverage); no integrated field is needed at M2.
- RQ-92 ruled: R-124 — R-25 is applied in the docs: the refusal covers every `Some(E*)`, including `Some(0)`.
- RQ-100 ruled: R-113 — TASK-M2-10 and TASK-M2-11 depend on this task for K_max and γ_K.
