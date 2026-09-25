# TASK-M2-11 — Acute-angle Burrau charts, the δm strip and the two Burrau quotients

- **Milestone:** M2
- **Closes:** REQ-CHART-024, REQ-CHART-045, REQ-CHART-046, REQ-CHART-027, REQ-CHART-025
- **Depends on:** TASK-M2-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~380 lines

## Goal
The folded acute-angle Burrau charts exist: θ ∈ (0, π/4], ν(θ) = sec θ − tan θ, Φ_θ,K(u, v) = (ν(θ(u)), m_Burrau(ν(θ(u))), (L_z = 0, K(v))) with θ(u) linear on [θ_min, θ_max] and K(v) = K_max·v^γ_K, the variant Φ_θ,L_z (fixed K, swept L_z), and the δm bifurcation strip m(u, v) = (1 − v)·m_Burrau + v·m_target (default equal masses, rest). Both Burrau shape charts are kept and labelled via `system_image` with the quotient each covers — the full-range Euclid chart `DoubleCover` — and shape fractions refuse the full-range chart (R-27, R-104). θ_min, θ_max, and Φ_θ,L_z's K and L_z range are proposed with evidence.

## References
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"

## Deliverables
- `crates/kernel/src/chart/burrau_acute.rs` (Φ_θ,K, Φ_θ,L_z, Φ_θ,δm).
- `system_image` values on both Burrau shape charts, shown in the chart label; the fraction-statistics guard.
- Tests `crates/kernel/tests/burrau_acute.rs`; the calibration evidence (decode-time renders via the TASK-M2-06 dispatch) in `fixtures/gates/burrau_acute/`.

## Acceptance tests
- `cargo test -p kernel burrau_theta` — ν(θ) inverts θ(ν) on (0, π/4]; L_z = 0 on Φ_θ,K; Φ_θ,L_z holds K fixed (REQ-CHART-024).
- Calibration: θ_min and θ_max stated with renders of the θ × K strip at the proposed defaults; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-CHART-045).
- Calibration: Φ_θ,L_z's K and L_z range stated, feasible under |L_z| ≤ √(2IK), with a render at the defaults; reviewer-checked, confirmed at the M2 gate, recorded in `decisions.md` (REQ-CHART-046).
- `cargo test -p kernel burrau_delta_m_strip` — the v = 0 row equals the Burrau masses; the v = 1 row equals m_target (REQ-CHART-027).
- Review (physics): both charts registered; their `system_image` values differ and are shown in the chart label; fraction statistics refuse the full-range chart; `cargo test -p kernel burrau_quotients` asserts the refusal (REQ-CHART-025).

## Notes
- REQ-CHART-025 carries rq RQ-71 (is the shape sphere DoubleCover or 2-to-1?); the Euclid chart's DoubleCover is R-27/R-104's own case. The gate can't pass until RQ-71 is ruled.
- Gap G21: the calibrations ask for renders at the defaults; before the integrator (M3/M4) only decode-time fields can be rendered.
- Gap G22: whether Φ_θ,K's K_max, γ_K are the invariant-chart defaults (REQ-CHART-044).
- Gap G11: chart_reference §4.5 still says "pending change 2, open" and "Until one quotient is chosen" (RQ-77).
