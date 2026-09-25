# TASK-M3-05 — COM projection and invariant monitoring

- **Milestone:** M3
- **Closes:** REQ-INT-010, REQ-INT-011, REQ-INT-012, REQ-INT-030, REQ-INT-040, REQ-VAL-033
- **Depends on:** TASK-M3-04, TASK-M2-24
- **Needs (earlier milestones):** REQ-PAY-010, REQ-PAY-031, REQ-GEN-007, REQ-DEC-025
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~350 lines

## Goal
After every `STEP` the wrapper's callback projects out the CoM position and total linear momentum (with `M` computed once and cached), on the parity surface with identical arithmetic and order on CPU and GPU, in CoM-frame particle coordinates; it never restores E or L_z. Before the loop it stores `E_0`, `L_{z,0}`; after each projected step it accumulates `ΔE_final`, `max|ΔE|`, `ΔL_z` final and `max|ΔL_z|` with the floored relative forms. The drift suspect predicates are read-time predicates over the stored latches, reading the occupant's `symplectic` flag.

## References
- `docs/contracts/principia_integrator_contract.md` § "COM projection and invariant monitoring"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `docs/design/principia_dd_integrator.md` § "3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)"
- `docs/design/principia_dd_integrator.md` § "3.5 Invariant monitoring (per `STEP`, post-projection)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"

## Deliverables
- `crates/kernel/src/driver/project.rs` — `project_com` (position–momentum form; velocity form for a velocity occupant), `M` cached.
- `crates/kernel/src/driver/monitor.rs` — `E_0`/`L_{z,0}` capture, per-step accumulation, `δ_E`, `δ_L` with `eps_E`, `eps_L` floors.
- Read side: the drift suspect predicates (energy-suspect on relative δE, L_z-suspect on absolute ΔL_z) in the generated accessor layer, reading `symplectic` from the profile; no stored suspect bits.
- GPU self-test dispatch of `stepOnce` + `project_com` on shared f32 states.

## Acceptance tests
- `cargo test -p kernel com_projection` — after one STEP from a state with nonzero CoM offset and momentum, R_com and P_com are zero to rounding; M is not recomputed per step (REQ-INT-010).
- Review (physics): no code path modifies state to restore E or L_z (REQ-INT-011).
- `cargo test -p kernel invariant_monitor` — rest start (E_0 ≠ 0, L_z0 = 0): δ_L finite; a synthetic spike-then-recover trace yields max|ΔE| ≫ |ΔE_final| (dd test 7) (REQ-INT-012).
- `cargo xtask gate projection-parity` — property test: stepOnce on shared states, the projected state from CPU-f32 and GPU-f32 matches bit-for-bit on Metal; no Jacobi conversion inside the step (REQ-INT-030).
- `cargo test -p kernel drift_suspect_read_time` — the payload has no suspect bits; the predicate is evaluated at read; Euler + the energy-drift view lights SUSPECT_ENERGY everywhere (REQ-INT-040).
- `cargo test -p kernel close_encounter_drift_shape` — dd test 7: a close-encounter IC shows max|ΔE| ≫ |ΔE_final| in the stored max vs final (REQ-VAL-033).

## Notes
- Per-trajectory projection of the Benettin shadow and ensemble copies is TASK-M3-14's (REQ-INT-039).
- Waits on RQ-97 (`REVIEW_QUEUE.md`): GPU and browser legs before the GPU kernel or the browser exists.
