# TASK-M3-22 — Seam 5: the producer writes every ledger field, and the scalar and shape views

- **Milestone:** M3
- **Closes:** REQ-GEN-016, REQ-TOOL-036, REQ-TOOL-037, REQ-VAL-027, REQ-VAL-117, REQ-TOOL-132
- **Depends on:** TASK-M3-19
- **Needs (earlier milestones):** REQ-TOOL-010, REQ-TOOL-011, REQ-TOOL-016, REQ-TOOL-024, REQ-GEN-001, REQ-PAY-034, REQ-TOOL-018
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
The integrator is certified as the payload's producer: every generation-root ledger field is written per its metadata, so each field's debug view (built in M1 on synthetic payloads) is live and sane on real CPU payloads; the SimState scalar views and live shape views assert their invariants; the energy-agreement view compares the kernel's `E_0` with the decode's derived `K₀ + V₀` within the calibrated E₀ agreement tolerance.

## References
- `docs/design/principia_dd_integrator.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-153 — The debug and live-march views have golden images of their own *(closes RQ-123)*"
- `decisions.md` § "R-158 — The six readings are accepted *(closes RQ-128)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `xtask golden producer-fields` — a CPU-computed payload over a fixture slice rendered through every generated field view, with golden images in `fixtures/golden/producer-fields/`.
- `crates/validation/src/checks/scalar_views.rs` — the TOOL-036 assertions over synthetic and golden-IC payloads.
- `xtask golden shape-views` — golden playback frames of u_mode 0/1/2 and the accumulator views.
- `xtask golden live-march-views` — the |n|−1 view and the live effort heatmap on a real march, with their own golden images recorded at the M3 gate (R-153) (REQ-TOOL-132).
- `xtask gate e0-agreement` + calibration proposal for the E₀ agreement tolerance (synthetic and real survey, each precision).

## Acceptance tests
- `cargo xtask golden producer-fields` — dd seam 5: each field's debug view is live and sane; sentinel conventions honoured (REQ-GEN-016).
- `cargo test -p validation scalar_view_assertions` — on synthetic and golden-IC payloads: t_end_step is the completed-step count latched at termination; d_min > 0 and is the trajectory minimum; delta_E_max_abs ≥ |energy_drift| and delta_Lz_max_abs ≥ |Lz_drift|; E_0 = K₀ + V₀; ftle ≈ 0 on Kepler-embedded ICs and large on Burrau; energy_drift oscillates for symplectic steppers; signed drift fields on diverging scales (REQ-TOOL-036).
- `cargo xtask golden shape-views` — golden playback frames: the mode-2 view is flat zero; the θ̃ view has no seams; terminal-latched samples frozen (tolerance: REQ-VAL-122's, calibrated at M1) (REQ-TOOL-037).
- `cargo xtask golden live-march-views` — on a real march the |n|−1 view reads flat zero and the live current-substep effort heatmap shows the march's per-pixel substep counts; both match their own golden images, recorded at the M3 gate (R-153) (REQ-TOOL-132).
- `cargo xtask gate e0-agreement` — on a synthetic and a real survey |E_0 − (K_0 + V_0)| is within the calibrated E₀ agreement tolerance (REQ-VAL-117) everywhere; the ICDescriptor stores no E₀ field (REQ-VAL-027).
- `cargo xtask gate e0-agreement --propose` — the proposal measures |E_0 − (K_0 + V_0)| on a synthetic and a real survey at each precision and sets the tolerance above it; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-117).

## Notes
- R-113 (RQ-94): REQ-TOOL-010's real-march |n|−1 check and the live effort heatmap are split off as REQ-TOOL-132 and closed here; the synthetic-payload half stays in TASK-M1-12.
- The fragment views run over CPU-filled buffers (M1's synthetic path), not a GPU march — the GPU march is M4.
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-117.
- RQ-123 ruled: R-153 — the live-march views are checked against golden images of their own, recorded at the M3 gate, not against artboards.
- RQ-128 ruled: R-158 — the readings taken while applying R-141 to R-156 are accepted.
