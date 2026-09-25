# TASK-M3-09 — Collision detection, t = 0 terminals and the d_min pair

- **Milestone:** M3
- **Closes:** REQ-EVT-002, REQ-EVT-003, REQ-EVT-004, REQ-EVT-009, REQ-PAY-047, REQ-ENC-033
- **Depends on:** TASK-M3-04, TASK-M3-05
- **Needs (earlier milestones):** REQ-PAY-004, REQ-PAY-013, REQ-SYS-016, REQ-DEC-008, REQ-ENC-019
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~300 lines

## Goal
Collision is detected every STEP by counting pairs with `‖rᵢ − rⱼ‖² < r_coll²` against the sim-key constant: one pair gives `collision` with that pair id (pair k opposite body k, R-22), two or more give `detail = 3`. The squared minimum separation is computed once per step and shared by the substep bucket, the collision test and `d_min`, whose pair `dmin_pair` is latched. At dispatch, before the first step, the detectors run on the decoded IC: inside `r_coll` is `collision` with `t_end_step = 0` (COLLISION_T0, looked-up ICs included), there is no t = 0 escape, and `decode_failed` is only the decoder's failure.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `decisions.md` § "R-60 — No t = 0 escape outcome *(closes RQ-19)*"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `docs/design/principia_chart_reference.md` § "0.7 Degeneracy — every pixel gets a label"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "What is already settled (in the payload spec, not here)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/kernel/src/detect/collision.rs` — pair count and classification, reading the step's one `min d²`.
- `crates/kernel/src/detect/t0.rs` — the dispatch-time detector pass.
- `dmin_pair` latch (3 = unset initially) written through the generated descriptor setters; `r_coll` carried in every output's provenance.

## Acceptance tests
- `cargo test -p kernel t0_detectors` — IC inside r_coll → collision, t_end_step 0; IC already escaping → not escape at t = 0; non-finite IC → decode_failed (REQ-EVT-002).
- `cargo test -p kernel collision_detector` — dd test 6: head-on pair → collision with correct 0-based pair id; two pairs below r_coll in one step → detail = 3, never a binary label (REQ-EVT-003).
- Review (code, physics): one min-separation computation per step; every output/provenance record carries r_coll (REQ-EVT-004).
- `cargo test -p kernel collision_t0_lookup` — an IC with a pair inside r_coll gives COLLISION_T0 with the opposite-side pair id and t_event = 0; the same IC entered via lookup gives the same label (REQ-EVT-009).
- `cargo test -p kernel lookup_t0_collision_dispatch` — a lookup fixture with bodies within r_coll reaches dispatch and is labelled `collision`, `t_end_step = 0` (REQ-ENC-033).
- `cargo test -p kernel dmin_pair_latch` — on a fixture close encounter dmin_pair equals the pair of minimum separation; initial value is 3 (REQ-PAY-047).

## Notes
- RQ-95 ruled: R-113 — REQ-ENC-019's t = 0 collision label at dispatch is split off as REQ-ENC-033 and closed here with REQ-EVT-002; lookup's no-rejection half stays in TASK-M2-19.
