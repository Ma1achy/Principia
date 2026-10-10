# TASK-M2-11 — Acute-angle Burrau charts, the δm strip and the two Burrau quotients

- **Milestone:** M2
- **Closes:** REQ-CHART-024, REQ-CHART-045, REQ-CHART-046, REQ-CHART-027, REQ-CHART-025, REQ-CHART-055
- **Depends on:** TASK-M2-10, TASK-M2-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~380 lines

## Goal
The folded acute-angle Burrau charts exist: θ ∈ (0, π/4], ν(θ) = sec θ − tan θ, Φ_θ,K(u, v) = (ν(θ(u)), m_Burrau(ν(θ(u))), (L_z = 0, K(v))) with θ(u) linear on [θ_min, θ_max] and K(v) = K_max·v^γ_K, the variant Φ_θ,L_z (fixed K, swept L_z), and the δm bifurcation strip m(u, v) = (1 − v)·m_Burrau + v·m_target (default equal masses, rest). Both Burrau shape charts are kept and labelled via `system_image` with the quotient each covers — the full-range Euclid chart `DoubleCover` — and shape fractions refuse the full-range chart (R-27, R-104). θ_min, θ_max, and Φ_θ,L_z's K and L_z range are proposed with evidence. Each Burrau-family axis's extension type past `[0,1]²` (R-407) is written into chart_reference §5.4 as a definition (R-72), physics-reviewed.

## References
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-27 — Both Burrau charts are kept, each labelled with its quotient *(CD-7)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `decisions.md` § "R-147 — The R-97 to R-109 follow-ups are applied *(closes RQ-77)*"
- `decisions.md` § "R-157 — `DoubleCover` stays, for the full-range Burrau chart *(closes RQ-127, amends R-141)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"

## Deliverables
- `crates/kernel/src/chart/burrau_acute.rs` (Φ_θ,K, Φ_θ,L_z, Φ_θ,δm).
- `system_image` values on both Burrau shape charts, shown in the chart label; the fraction-statistics guard.
- The Burrau family's axis extension types (R-407; REQ-CHART-055): chart_reference §5.4's Burrau row filled in, one type
  per axis of every Burrau-family chart (TASK-M2-10's Euclid plane and `(ν, K)` chart, and this task's acute-angle charts
  and strips), with the reasoning; an axis given nothing stays bounded, the default.
- Tests `crates/kernel/tests/burrau_acute.rs`; the calibration evidence (decode-time renders via the TASK-M2-06 dispatch) in `fixtures/gates/burrau_acute/`.

## Acceptance tests
- `cargo test -p kernel burrau_theta` — ν(θ) inverts θ(ν) on (0, π/4]; L_z = 0 on Φ_θ,K; Φ_θ,L_z holds K fixed (REQ-CHART-024).
- Calibration: θ_min and θ_max stated with renders of the θ × K strip at the proposed defaults; reviewer-checked, confirmed by the human at the M2 gate, recorded in `decisions.md` (REQ-CHART-045).
- Calibration: Φ_θ,L_z's K and L_z range stated, feasible under |L_z| ≤ √(2IK), with a render at the defaults; reviewer-checked, confirmed at the M2 gate, recorded in `decisions.md` (REQ-CHART-046).
- `cargo test -p kernel burrau_delta_m_strip` — the v = 0 row equals the Burrau masses; the v = 1 row equals m_target (REQ-CHART-027).
- Definition (physics): chart_reference §5.4's Burrau row gives each Burrau-family axis's extension type with its reasoning (`ν` past `(0,1)` and θ past `(0, π/4]` give a zero or negative mass); no axis is given more than bounded without the physics reviewer's approval (REQ-CHART-055, R-407).
- Review (physics): both charts registered; their `system_image` values differ and are shown in the chart label; fraction statistics refuse the full-range chart; `cargo test -p kernel burrau_quotients` asserts the refusal (REQ-CHART-025).

## Notes
- RQ-71 ruled: R-141 — the shape sphere is n-to-1 (n = 2). RQ-127 ruled: R-157 — `DoubleCover` stays as the full-range Euclid chart's `system_image` (R-27: the leg swap relabels the bodies); REQ-CHART-025 is unblocked.
- R-133: the calibrations' evidence renders ask for coverage of the chart domain (feasibility, K, L_z), which decode-time renders show; no integrated field is needed at M2.
- RQ-77 ruled: R-147 — chart_reference §4.5 now records R-27 (both charts kept, applied before any Burrau statistic).
- RQ-100 ruled: R-113 — Φ_θ,K's K_max and γ_K are REQ-CHART-044's, calibrated in TASK-M2-12, which this task now depends on.
- R-407 (10 Oct 2026): clause 3 assigns the Burrau family's axis types "per axis, with physics review". They are a
  definition requirement (R-72) on this task, which completes the Burrau charts (applied per R-369, R-407, A6); the
  types are declared in code by TASK-M8-05 with the extension mechanism (REQ-CHART-054). No types are given now.
