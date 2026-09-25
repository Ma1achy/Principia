# TASK-M3-06 — SimUniforms: the wrapper parameters, their defaults and the dispatch refusals

- **Milestone:** M3
- **Closes:** REQ-INT-026, REQ-PAY-037, REQ-PAY-040, REQ-SYS-018, REQ-INT-081
- **Depends on:** TASK-M3-04
- **Needs (earlier milestones):** REQ-PAY-012, REQ-GEN-006, REQ-INT-002, REQ-RENDER-019
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
The wrapper's loop parameters are one `SimUniforms` struct in `crates/engine`, bound once per frame/tier, with integrator contract Part 3's defaults and owners (T_horizon default 50, dt_macro = max(10⁻³, T/65535), R-132); `r_coll` nonzero by default and `epsilon` 0 by default and optional, both canonical and fixed at t = 0, with ε > 0 runs tagged. Dispatch refuses any `(T, dt_macro)` with `horizon_steps = ⌈T/dt_macro⌉ > 65535` and asserts `horizon_steps × N_max ≤ 2³²−1`; `t_end_step`/`t_dmin_step` pack as exact u16 macro-step indices in `times`.

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

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*"
## Deliverables
- `crates/engine/src/contract/sim_uniforms.rs` — `SimUniforms { t_horizon, dt_macro, n_max, r_sub, gamma_sub, r_coll, tau, escape_window, r_close, eps_e, eps_l, g, m_total, epsilon, n_renorm, delta0 }` with `Default`, and its sim-key byte contribution.
- `crates/engine/src/dispatch/validate.rs` — the horizon refusal and the joint `total_substeps` bound, returning a typed error.
- Provenance tag for ε > 0 runs on the sim key; no co-moving option exists.

## Acceptance tests
- `cargo test -p engine sim_uniforms_defaults` — default SimUniforms match the Part 3 table (T_horizon = 50, dt_macro = 10⁻³); dt_macro = T/65535 for T above 65.535 (REQ-INT-026; the sim-key half is REQ-SCHED-007's, M4).
- `cargo test -p engine horizon_refusal` — T/dt_macro giving 65536 steps is refused at dispatch; t_end_step of a terminal sample equals the latched step count (REQ-PAY-037).
- `cargo test -p ledger times_roundtrip` — round-trip t_end_step/t_dmin_step exactly; a config with ⌈T/dt_macro⌉ > 65535 is refused at dispatch (REQ-PAY-040).
- `cargo test -p engine collision_softening_defaults` — r_coll nonzero and ε = 0 by default; no co-moving option; ε > 0 results carry a tag in provenance and the sim key (REQ-SYS-018).
- `cargo test -p engine horizon_default` — default T_horizon = 50 with dt_macro = 10⁻³; at T = 200, dt_macro = 200/65535 and ⌈T/dt_macro⌉ = 65535; no T in [50, 200] is refused at dispatch (REQ-INT-081).

## Notes
- R-132 settles the T_horizon default (50) and dt_macro (max(10⁻³, T/65535)). Gap: Part 3 gives no value for `tau`; `SimUniforms::default()` needs it. The sync schedule of the regularised occupants (`n_sync`, `eta`) is not in the Part 3 table either.
- The full sim-key composition is M4/M5's (REQ-SCHED-007, REQ-SCHED-048); this task provides the wrapper-parameter part of it.
- RQ-98 ruled: R-113 — the sim-key clause and its verify half are dropped from REQ-INT-026; REQ-SCHED-007 (M4, TASK-M4-08) carries them, its fixture including each SimUniforms field.
- REQ-INT-081 was an R-71 calibration; R-132 rules its values, so nothing here stays calibrated.
