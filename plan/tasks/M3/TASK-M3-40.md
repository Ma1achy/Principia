# TASK-M3-40 — The fixed sync quantum: one grid of physical time, the march stopping only on it, live, catch-up and re-boot bitwise (R-565)

- **Milestone:** M3
- **Closes:** REQ-INT-100
- **Depends on:** TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4
- **Size:** ~250 lines, the tests included

## Goal
The regularised occupants' sync grid is a fixed quantum of physical time, `Δ_sync`, the same for every run and independent of the horizon and of frame timing (R-565). Steps are sized from `Δ_sync`, never from the target a call asks for; the march stops only on a boundary `t = k·Δ_sync`, `ADVANCE` reaching the latest boundary at or before its target; so live march, catch-up and re-boot reach the same state bitwise. `Δ_sync` is a constant of the build, read from the constants register, its value REQ-INT-080's calibration (TASK-M3-07).

## References
- `docs/contracts/principia_integrator_contract.md` § "The sync grid is a fixed quantum of physical time (R-565)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `docs/contracts/principia_integrator_contract.md` § "What this preserves, and what it costs"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `decisions.md` § "R-565 — The sync grid is a fixed quantum of physical time, `Δ_sync`, the same for every run and independent of the horizon and of frame timing: steps are sized from it, the march stops only on it, the playhead reads boundary states, so live march, catch-up and re-boot reach the same state bitwise; time windows such as the closure window are stated in physical time"
- `decisions.md` § "R-19 — Change 8 landed in full *(closes RQ-17)*"

## Deliverables
- `crates/kernel/src/regularisation/sync.rs` — the grid: boundaries at `k·Δ_sync` from the constants register, `ADVANCE`'s target snapped to the latest boundary at or before it, each interval's step control sized from `Δ_sync` (Aarseth–Zare's `dt_left` is `Δ_sync`).
- `Δ_sync` in the constants register beside `r_pole`, on the sim key's config so a change re-integrates.

## Acceptance tests
- `cargo test -p kernel sync_quantum_bitwise` — REQ-INT-100's verify: for Heggie, Aarseth–Zare and logH on a close-encounter fixture, the state at a grid time reached in one `ADVANCE`, in `ADVANCE` calls each advancing its target by a fixed increment of physical time that is not a multiple of `Δ_sync` (at the proposed `Δ_sync` and at the second value run), so the targets lie off the grid, and by re-boot from `t = 0` after a pause, is bitwise identical, at `T = 50` and `T = 200` with the same boundaries; an off-grid target returns the boundary before it; controls: a march that resizes its step to land on the off-grid frame targets differs and fails, a grid of `t_max / n_sync` differs between the two horizons and fails; a copy that collides between boundaries latches at the firing step, `t_end` that step's time and `ADVANCE` returning there, live, catch-up and re-boot agreeing bitwise; control: a march that carries the collided copy on to the next boundary, or holds `t_end` back to the previous one, fails (REQ-INT-100; qa review 5607548278 of PR #277, Q3 and Q9, applied per R-369).

## Notes
- R-565 (21 Oct 2026). The cost and label-flip evidence for `Δ_sync` is prin-rs's `sync_grid_probe` (PR #34; `docs/incoming/2026-10-21_gap_sweep_integrator/`, a record). The value is proposed by TASK-M3-07 (REQ-INT-080) and confirmed at the M3 gate; until then the tests run at the proposal's value and at a second one.
- The unregularised macro-step grid is unchanged (RQ-524's (a), applied meanwhile).
- The frame loop's side, the live set advancing by whole quanta, is TASK-M5-21's (REQ-SCHED-103).
