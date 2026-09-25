# TASK-M3-13 — Welford streaming diffusion and the definition of y

- **Milestone:** M3
- **Closes:** REQ-INT-041, REQ-PAY-054, REQ-PAY-057, REQ-PAY-073, REQ-RENDER-028
- **Depends on:** TASK-M3-04
- **Needs (earlier milestones):** REQ-PAY-030, REQ-GEN-012, REQ-PAY-010
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The diffusion slope is a Welford streaming slope per macro-step: the time-only moments derived in closed form (`mean_t = (n+1)h/2`, `C_tt(n) = h²n(n²−1)/12`) with no mutable shared global, and the per-sample `mean_y`, `C_ty` updated in two f32 accumulators with the old-mean time deviation `δ_t = 0.5·n·h`; the slope `C_ty/C_tt(n)` reads the −1.0 sentinel for n < 2. The per-sample `y` the regression fits is defined in payload §4 (R-72). Every stability metric is a fixed-size O(1) accumulator.

## References
- `docs/design/principia_dd_integrator.md` § "3.5 Invariant monitoring (per `STEP`, post-projection)"
- `decisions.md` § "R-17 — The diffusion sentinel uses the streaming slope *(closes RQ-15)*"
- `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_temporal_architecture_note.md` § "Stability metrics are just running accumulators (they get simpler)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- Doc change: `docs/design/principia_dd_simstate_payload.md` §4 — `y` as a formula over SimState fields (definition, REQ-PAY-073).
- `crates/kernel/src/driver/welford.rs` — the per-macro-step update; closed-form time moments.
- Read side: the diffusion sentinel reads the ledger's streaming-slope field (R-17).

## Acceptance tests
- `cargo test -p kernel welford_slope` — the streaming slope equals the least-squares slope of y on t for random series within f32 tolerance; n = 0, 1 return the sentinel; no shared mutable global in the dispatch (REQ-INT-041).
- `cargo test -p kernel time_moments_closed_form` — the closed form equals brute-force Σ(t_i − mean_t)² for n = 0..1000 (REQ-PAY-054).
- Review (code): every accumulator field has a fixed size; no per-sample growable buffer exists (REQ-PAY-057).
- Payload §4 states what y is per sample as a formula over SimState fields; the Welford update and the diffusion views use it; physics reviewer approved (REQ-PAY-073).
- `cargo test -p render diffusion_sentinel_streaming` — the sentinel reads the ledger's streaming-slope field; a fixture with known slope trips the sentinel at the ledger-defined value (REQ-RENDER-028).

## Notes
- Definitions written here (R-72; physics reviewer approves before merge): REQ-PAY-073.
