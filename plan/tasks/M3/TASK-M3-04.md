# TASK-M3-04 — The wrapper: the ADVANCE seam, the done-flag loop and the per-substep callback

- **Milestone:** M3
- **Closes:** REQ-INT-007, REQ-INT-008, REQ-INT-009, REQ-INT-020, REQ-INT-022, REQ-INT-023, REQ-INT-025, REQ-INT-038, REQ-INT-053, REQ-SCHED-003
- **Depends on:** TASK-M3-02, TASK-M3-03, TASK-M2-23
- **Needs (earlier milestones):** REQ-DEC-005, REQ-SYS-012, REQ-CHART-015, REQ-CHART-028
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~500 lines

## Goal
The fixed shared wrapper exists: it owns the loop (two `while` loops exiting only on the `done` flag, zero `break`), the integer macro-step horizon counter against `horizon_steps = ⌈T/dt_macro⌉`, adaptive substepping, and the per-substep cadence, which it passes into the occupant as a callback through the seam `ADVANCE(state, t_now, t_target, params) -> state'` (R-19). KDK/Yoshida implement `ADVANCE` as the fixed `N_sub` `STEP` loop; the occupant reads only the per-sample `(m, r, p)`, `dt` and `G` and never learns the chart; every IC integrates from its own decoded state.

## References
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/design/principia_dd_integrator.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "The pixel's life = one ladder traversal"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/contracts/principia_integrator_contract.md` § "What this preserves, and what it costs"
- `decisions.md` § "R-19 — Change 8 landed in full *(closes RQ-17)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature"
- `docs/design/principia_dd_integrator.md` § "3.6 Detectors (per `STEP`, on the projected state)"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_integrator.md` § "3.3 Substep law — and the determinism pin, made concrete"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 1 — The firewall: the scheduler is arbitrary about *what* it looks at, never about *what* it sees"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/kernel/src/driver/advance.rs` — the `Advance` trait with the cadence callback parameter; the blanket impl for steppers (`N_sub` STEP calls at `dt_macro/N_sub`, invoking the callback after each).
- `crates/kernel/src/driver/wrapper.rs` — `integrate(ic, uniforms) -> SimState` in integrator contract Part 1's loop shape (done flag in both loop conditions; bounded loops; integer horizon counter; no float time accumulation).
- `crates/kernel/src/driver/cadence.rs` — the per-substep callback (projection, invariant accumulation and detection hooks, filled by TASK-M3-05/09/12) and its instrumentation counter for tests.
- Occupant-reported `total_substeps` for every occupant, and a property test harness that fuzzes ICs over all occupants.
- Seam-3 test: every chart in the M2 lowering appendix drives the same kernel unmodified.

## Acceptance tests
- `cargo test -p kernel same_spec_twice` — property test: the same spec rendered twice gives bit-identical CPU output (fixed dt, count-bound, never wall-clock) (REQ-INT-007).
- `cargo test -p kernel seam4_occupant_swap` — property test: swapping Euler, KDK, Yoshida-4, Yoshida-6 and RK4 on the same IC set changes only the §3.2 arithmetic; the wrapper code path and branch trace are shared (no occupant-specific branch in wrapper code) (REQ-INT-008).
- `cargo test -p kernel cadence_callback_count` — property test: for every occupant, callback invocations == internal steps taken; an occupant that omits the call fails (REQ-INT-009).
- `cargo test -p kernel advance_equals_nsub_steps` — for KDK/Yoshida, ADVANCE(t, t+dt_macro) equals N_sub STEP calls bit-for-bit (the AZ arm is TASK-M3-07's) (REQ-INT-020).
- `cargo test -p kernel total_substeps_honest` — property test over fuzzed ICs: reported `total_substeps` equals the per-substep callback invocations for every occupant (REQ-INT-022).
- Review (code): no wall-clock or timer read on any path that affects the number or size of steps (REQ-INT-023).
- `cargo test -p kernel seam3_every_chart` — property test: every chart in the lowering appendix drives the same kernel unmodified; the occupant signature takes no chart identifier (REQ-INT-025).
- Review (code): no `t += dt` feeds the `t >= T` branch; the horizon compare is on an integer counter (REQ-INT-038).
- `cargo test -p kernel advance_seam_r19` — for each stepper composition the cadence callback is invoked once per substep (the owns_time_mapping arm over regularisations is in TASK-M3-07) (REQ-INT-053).
- Review (code): the integration entry point takes only the decoded IC and the sim-key uniforms; no path passes neighbour or parent state into the march (REQ-SCHED-003).

## Notes
- REQ-INT-053's owns_time_mapping arm needs the regularisation axis; TASK-M3-07's acceptance runs it over every stepper × regularisation pair.
- RQ-97 ruled: R-113 — REQ-INT-007's branch-exact GPU arm is dropped from M3; M4 covers it by REQ-VAL-059 (Tier L, TASK-M4-03).
