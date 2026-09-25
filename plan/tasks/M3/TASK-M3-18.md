# TASK-M3-18 — The failure path: sim_failed, NaN-free storage and the failed-state values

- **Milestone:** M3
- **Closes:** REQ-PAY-039, REQ-PAY-086, REQ-SYS-019, REQ-VAL-039
- **Depends on:** TASK-M3-17, TASK-M1-14
- **Needs (earlier milestones):** REQ-PAY-024, REQ-DEC-038, REQ-PAY-014
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~250 lines

## Goal
A blown-up sample stores the defined failed-state values and storage never holds NaN: payload §1's storage rules list the stored value of each f32 field (phase state, shadow, accumulators, drift refs) for `sim_failed` and `decode_failed` (R-72, R-79); diffusion reads −1.0 when n < 2; the state enum and the sticky `saturated` bit are the primary invalid signals. An integration failure is recorded as an outcome and counted, never dropped or filtered by drift. The sim_failed/decode_failed detail codes are cross-checked against the integrator contract's failure enumeration.

## References
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `decisions.md` § "R-17 — The diffusion sentinel uses the streaming slope *(closes RQ-15)*"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/read_first/principia_00_philosophy.md` § "4.3 Failure is a measurement, not missing data"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"

## Deliverables
- Doc change: `docs/design/principia_dd_simstate_payload.md` (the storage rules beside §1) — the failed-state value of each f32 field (definition, REQ-PAY-086).
- `crates/kernel/src/driver/failure.rs` — non-finite detection per step, the `sim_failed` write with its detail code, the failed-state value writes.
- A NaN scanner over every stored float of a payload buffer, used by the tests.

## Acceptance tests
- `cargo test -p kernel no_nan_in_storage` — scan every stored float in synthetic, forced-failure and real payloads: no NaN bit patterns; a sample with n < 2 reads diffusion == −1.0 (REQ-PAY-039).
- The payload doc lists the stored value of each f32 field for sim_failed and decode_failed samples, none NaN; the failure path writes exactly those; physics reviewer approved (REQ-PAY-086).
- `cargo test -p kernel failure_is_measurement` — force an integration failure on a fixture pixel: it is stored with a failure outcome and counted, not dropped or masked; review: no code path filters samples by drift (REQ-SYS-019).
- Review (physics): payload §2's failure enum cross-checked against the integrator contract's list; any mismatch raised as an RQ (REQ-VAL-039).

## Notes
- Definitions written here (R-72; physics reviewer approves before merge): REQ-PAY-086.
