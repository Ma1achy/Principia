# TASK-M4-02 — The native in-process wgpu parity harness: GPU stepOnce and decodeOnly on shared states

- **Milestone:** M4
- **Closes:** REQ-VAL-063, REQ-INT-071
- **Depends on:** TASK-M4-01, TASK-M3-21
- **Needs (earlier milestones):** REQ-SYS-017, REQ-GEN-004, REQ-VAL-026, REQ-INT-036, REQ-EVT-003, REQ-INT-028, REQ-DEC-022
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10, PIT-3
- **Size:** ~400 lines

## Goal
The mechanics every parity tier runs on: a native `#[test]` process that instantiates the kernel at CPU-f64 and CPU-f32, dispatches the GPU build of the same source through native `wgpu`, reads the outputs back and compares them in the same process. The harness takes **shared states, not shared ICs**: GPU entry points for `decodeOnly` and one `stepOnce` accept a state buffer, so both pipelines are fed the identical input and compared immediately. Synthetic state generators cover the three regimes a real trajectory rarely reaches — deep near-collision, high substep count, and the invariant charts' feasibility edge. The three-body-pair miscompile (the GPU summing one pair of three) is the first regression the harness carries.

## References
- `docs/contracts/principia_parity_contract.md` § "3. The load-bearing discipline: never accumulate before comparing"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `docs/contracts/principia_parity_contract.md` § "2. The three tiers"
- `docs/contracts/principia_parity_contract.md` § "8. Test index (drill-down suite → tier)"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/read_first/principia_01_pitfalls.md` § "10. GENERALISING A STATELESS RESULT TO A TRAJECTORY"

## Deliverables
- `crates/validation/src/harness/`: in-process native `wgpu` device setup, buffer upload/readback, and the `gpu_decode_only(states)` / `gpu_step_once(states, dt, params)` dispatchers over harness entry points compiled from `crates/kernel` (the same source; no parity-only physics).
- `crates/validation/src/synth.rs`: generators for shared states — near-collision (pair `d²` straddling `r_coll²`), high-substep (`d²` across the frozen `N_sub` thresholds up to `N_sub == N_max`), and near-feasibility-edge (invariant-chart targets straddling `K* = L_z²/2I`).
- Every harness test names its parity tier (L / B / N / S) in its name or doc comment.

## Acceptance tests
- `cargo test -p validation harness_shared_state` — the `stepOnce` / `decodeOnly` harness takes shared states, not shared ICs; synthetic generators exist for the three named regimes and each generator's output is shown to lie in its regime (and a generator deliberately pointed outside it is detected) (REQ-VAL-063).
- `cargo test -p validation three_pairs_hamiltonian` — GPU vs CPU Hamiltonian on the fixed spike state sums all three body pairs: H = −1.732, not −0.577 (REQ-INT-071).

## Notes
- Domain statement (pitfalls §10): these are fixed-input comparisons. They say nothing about labels along a trajectory; TASK-M4-18 owns that regime.
