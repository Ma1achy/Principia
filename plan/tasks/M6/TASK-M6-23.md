# TASK-M6-23 — The first refine milestone's measurements: depth past level 6, tau, merging under motion, zoom regression

- **Milestone:** M6
- **Closes:** REQ-VAL-092, REQ-REF-032, REQ-VAL-090, REQ-VAL-085, REQ-VAL-142, REQ-REF-050
- **Depends on:** TASK-M6-06, TASK-M6-12, TASK-M6-17, TASK-M6-20, TASK-M5-23
- **Needs (earlier milestones):** REQ-VAL-081, REQ-TOOL-050, REQ-RENDER-046
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-6, PIT-10, PIT-3
- **Size:** ~400 lines

## Goal
The refinement policy's open measurements are taken and recorded at this, the first refine milestone (R-49): depth beyond level 6, a calibrated grid with `tau` set as its own threshold (never `eps × k`), and merging under real camera motion. The zoom regression is benchmarked (in-view texel size flat, `max_depth` tracking the camera, quad count far below the `alpha_lo = 0` degeneration), and checkerboard, coarser refinement during motion and E reduction under motion are confirmed to compose.

## References
- `docs/design/principia_dd_refinement_policy.md` § "7. Open"
- `docs/design/principia_dd_refinement_policy.md` § "5. WHERE THE SAVING GENERALISES — and where it does not"
- `decisions.md` § "R-49 — The refinement policy's open measurements are taken at the first refine milestone *(RS-9)*"
- `docs/design/principia_dd_refinement_policy.md` § "5.2 The tolerance is inert exactly where the policy works"
- `docs/design/principia_dd_refinement_policy.md` § "0.1 In view, the camera decides depth and the criterion decides ORDER"
- `docs/contracts/principia_checkerboard_contract.md` § "8. Build-time settles (measure on the real system)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `cargo xtask bench refine-open-measurements` (depth > 6, the charts × horizons × eps grid with `tau`, merging on scripted camera paths), results recorded under `fixtures/bench/results/`.
- `cargo xtask bench zoom-regression` on `config_stability` and `preset_shape_h1`.
- `cargo xtask bench motion-composition` (checkerboard + motion floor + E gating, each alone and together).
- Config check that `tau` has no `eps`-derived default.

## Acceptance tests
- `cargo xtask bench refine-open-measurements` — a recorded results entry covers each of the three measurements (REQ-VAL-092).
- `cargo xtask bench refine-open-measurements --tau-grid` — calibrate tau per the tolerance grid (charts × horizons × eps) at the first refine milestone and record it; config has no tau = k·eps derivation (REQ-REF-032).
- `cargo xtask bench zoom-regression` — scripted zoom over two octaves on config_stability and preset_shape_h1: converged in-view texel size stays flat (~1.5 px) and max_depth tracks camera target depth; quad count stays far below the alpha_lo = 0 degeneration (+49% / +222%) (REQ-VAL-090).
- `cargo xtask bench motion-composition` — motion trace with all three active: reconstruction error and visual quality recorded against each alone (REQ-VAL-085).
- Proposal: the composed-lever degradation bound with the motion-trace evidence; the human confirms it at the M6 gate (REQ-VAL-142).
- Proposal: tau with the calibrated-grid evidence; the human confirms it at the M6 gate (REQ-REF-050).

## Notes
- REQ-REF-032's `tau` value is a measured setting; it goes to the human at the M6 gate with the other recorded values.
- REQ-VAL-085 gives no threshold for "without over-degrading" — see Gaps.
- Closes, for gaps the corpus leaves open: REQ-VAL-142 (R-71 calibration), REQ-REF-050 (R-71 calibration) (classification accepted by R-132).
