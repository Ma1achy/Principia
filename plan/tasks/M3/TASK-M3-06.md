# TASK-M3-06 — SimUniforms: the wrapper parameters, their defaults and the dispatch refusals

- **Milestone:** M3
- **Closes:** REQ-INT-026, REQ-PAY-037, REQ-PAY-040, REQ-SYS-018, REQ-INT-081
- **Depends on:** TASK-M3-04
- **Needs (earlier milestones):** REQ-PAY-012, REQ-GEN-006, REQ-INT-002, REQ-RENDER-019
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The wrapper's loop parameters are one `SimUniforms` struct in `crates/engine`, bound once per frame/tier and on the sim key, with integrator contract Part 3's defaults and owners; `r_coll` nonzero by default and `epsilon` 0 by default and optional, both canonical and fixed at t = 0, with ε > 0 runs tagged. Dispatch refuses any `(T, dt_macro)` with `horizon_steps = ⌈T/dt_macro⌉ > 65535` and asserts `horizon_steps × N_max ≤ 2³²−1`; `t_end_step`/`t_dmin_step` pack as exact u16 macro-step indices in `times`.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_simstate_payload.md` § "`times` (u32)"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_dd_generation_root.md` § "Open"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/contract/sim_uniforms.rs` — `SimUniforms { t_horizon, dt_macro, n_max, r_sub, gamma_sub, r_coll, tau, escape_window, r_close, eps_e, eps_l, g, m_total, epsilon, n_renorm, delta0 }` with `Default`, and its sim-key byte contribution.
- `crates/engine/src/dispatch/validate.rs` — the horizon refusal and the joint `total_substeps` bound, returning a typed error.
- Provenance tag for ε > 0 runs on the sim key; no co-moving option exists.

## Acceptance tests
- `cargo test -p engine sim_uniforms_defaults` — default SimUniforms match the Part 3 table; changing any one field changes the sim key (REQ-INT-026).
- `cargo test -p engine horizon_refusal` — T/dt_macro giving 65536 steps is refused at dispatch; t_end_step of a terminal sample equals the latched step count (REQ-PAY-037).
- `cargo test -p ledger times_roundtrip` — round-trip t_end_step/t_dmin_step exactly; a config with ⌈T/dt_macro⌉ > 65535 is refused at dispatch (REQ-PAY-040).
- `cargo test -p engine collision_softening_defaults` — r_coll nonzero and ε = 0 by default; no co-moving option; ε > 0 results carry a tag in provenance and the sim key (REQ-SYS-018).
- Proposal: the T_horizon default, with the 65535-step check at dt_macro = 10⁻³ and the Burrau-resolution evidence; the human confirms it at the M3 gate (REQ-INT-081).

## Notes
- Gap: Part 3 gives `T_horizon ∈ [50, 200]` without a single default, and no value for `tau`; `SimUniforms::default()` needs both. The sync schedule of the regularised occupants (`n_sync`, `eta`) is not in the Part 3 table either.
- The full sim-key composition is M4/M5's (REQ-SCHED-007, REQ-SCHED-048); this task provides the wrapper-parameter part of it.
- Waits on RQ-98 (`REVIEW_QUEUE.md`): M3 and M4 requirements that name later surfaces.
- Closes, for gaps the corpus leaves open: REQ-INT-081 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
