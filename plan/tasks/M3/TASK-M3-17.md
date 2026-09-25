# TASK-M3-17 — Terminal states, precedence and the terminal latch

- **Milestone:** M3
- **Closes:** REQ-EVT-001, REQ-EVT-007, REQ-EVT-008, REQ-EVT-010, REQ-EVT-020, REQ-INT-029, REQ-INT-033, REQ-INT-034, REQ-PAY-046, REQ-PAY-049, REQ-PAY-051, REQ-PAY-058
- **Depends on:** TASK-M3-09, TASK-M3-10, TASK-M3-12, TASK-M3-13, TASK-M3-14, TASK-M3-16
- **Needs (earlier milestones):** REQ-PAY-004, REQ-PAY-013, REQ-PAY-014, REQ-PAY-031
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1, PIT-1.4, PIT-1.6, PIT-2.4
- **Size:** ~500 lines

## Goal
The label writer: `state` holds escape / bounded / collision / running / sim_failed / decode_failed, mutually exclusive, with no timeout state (reaching T is `bounded`) and no MAX_SUBSTEPS terminal — the `N_max` cap advances under-resolved with `saturated` set, identically on both pipelines. Precedence is R-30's: sim_failed first, then the earlier event in time with a same-time tie to collision, triple ejection as a detail of escape. `detail` is written with `state` as the union keyed by it. On termination the sample latches: `done` is set and the production loop ends (R-103), the state stops advancing, accumulators freeze (FTLE's S/T at t_esc), `t_end_step` is latched at the firing step. The post-escape march never touches the payload.

## References
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `decisions.md` § "R-31 — Escape doesn't terminate until pitfalls §2.4's three checks pass *(IE-3)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/read_first/principia_01_pitfalls.md` § "2.4 Should it terminate?"
- `decisions.md` § "R-95 — After escape fires *(closes RQ-48, in part)*"
- `decisions.md` § "R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `decisions.md` § "R-30 — Event precedence is by time *(IE-2)*"
- `decisions.md` § "R-6 — The event priority order goes to the decision sheet *(closes RQ-4)*"
- `docs/design/principia_dd_integrator.md` § "6. Deferred / flagged"
- `docs/contracts/principia_parity_contract.md` § "Tier L — exact, no tolerance (the bulk of the suite)"
- `docs/contracts/principia_integrator_contract.md` § "Part 6 — Three boundaries stated honestly (deferred work lands here)"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_simstate_payload.md` § "`times` (u32)"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/kernel/src/detect/label.rs` — the precedence function and the single write of `state` + `detail`.
- `crates/kernel/src/driver/terminal.rs` — the latch: `done` set on collision or escape, loop exit through the loop conditions only, every SimState word frozen after termination.
- Doc follow-through: dd_integrator §3.6's priority block and test 6 updated to R-30 (the R-6 pin retired as the label writer lands), with the ruling cited.
- Fixtures for each precedence case and a forced near-singular encounter.

## Acceptance tests
- `cargo test -p kernel totality_labels` — property test: fuzzed ICs, every sample's state is one of the six labels; saturated samples still reach a dynamical outcome (REQ-EVT-001).
- `cargo test -p kernel state_enum` — horizon reached without event → bounded; NaN injected mid-march → sim_failed; substep cap → not terminal (REQ-EVT-007).
- Review (code): detector code lives in the wrapper module and takes only state + params (REQ-EVT-008).
- `cargo test -p kernel escape_latch` — escape-fired fixture: state reads escape, t_end_step is latched at the firing step and the production loop ends there (done set); a collision stops the march (the harness's §2.4 march leaving the payload byte-identical is asserted in TASK-M3-33) (REQ-EVT-010).
- `cargo test -p kernel terminal_precedence` — escape before collision → escape; collision before escape → collision; same-step tie → collision; sim_failed with any other event → sim_failed; triple ejection reported as escape detail; identical at f32 and f64 (REQ-EVT-020).
- `cargo test -p kernel nmax_cap_continues` — force a near-singular encounter: N_sub hits N_max, the march continues to a dynamical outcome, saturated set (REQ-INT-029).
- Review (physics): no KS state representation in v1; saturated regions continue to their outcome (REQ-INT-033).
- `cargo test -p kernel terminal_latch` — synthetic samples run to collision and to escape: the loop ends with done set at the terminating step; every SimState word is bit-identical on all subsequent dispatches; state == its terminal value (REQ-INT-034).
- `cargo test -p kernel detail_union` — for each state, pack/unpack each detail code and assert the decoded meaning (REQ-PAY-046).
- Review (code): no `timeout` or `survived_horizon` state exists; the horizon T is part of the sim key (REQ-PAY-049).
- `cargo test -p kernel t_end_step_counter` — property test: running and terminal states, t_end_step equals the step counter (latched at the escape firing step); t_dmin_step equals the argmin step (REQ-PAY-051).
- `cargo test -p kernel escape_freezes_accumulators` — on a fixture escaper the production loop ends at the firing step; state, t_end_step, S and the other accumulators and latches are unchanged by later dispatches; the derived FTLE equals S/T at t_esc (the harness byte-identity arm is TASK-M3-33's) (REQ-PAY-058).

## Notes
- R-103 governs: in production `done` is set when escape fires; the §2.4 checks run only in `crates/validation` on the harness's own state (TASK-M3-33).
- RQ-97 ruled: R-113 — REQ-INT-029's "CPU and GPU take the same capped step" is dropped from M3; M4 covers it by REQ-VAL-059 and REQ-VAL-061.
