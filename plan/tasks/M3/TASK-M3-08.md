# TASK-M3-08 — Heggie and logH occupants, and the visible re-registration count

- **Milestone:** M3
- **Closes:** REQ-INT-006, REQ-INT-018, REQ-INT-073, REQ-VAL-045, REQ-INT-084
- **Depends on:** TASK-M3-07
- **Needs (earlier milestones):** REQ-DEC-005, REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-4, PIT-4.3, PIT-4.4
- **Size:** ~450 lines

## Goal
The regularisation slot is complete: Heggie 1974 global regularisation (three relative vectors, no reference body, never re-registers) is the default, and logH (time transformation only) is the no-chart arm. The re-registration count is visible downstream, and integrator contract Part 2b defines where it lives — a `re_registrations` profile field, a per-trajectory payload count, or both — with any payload field added to the ledger as a schema change. The two-body radial collision gate is kept as the regularisation test.

## References
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_dd_predictability_horizon.md` § "The predictability horizon"
- `docs/contracts/principia_integrator_contract.md` § "The profile gains a field"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_validation_orbits.md` § "4. What the suite cannot do"
- `docs/contracts/principia_integrator_contract.md` § "Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper"
- `decisions.md` § "R-159 — The prin-rs reference set is imported *(closes RQ-102 and RQ-103, with R-160 to R-167)*"
- `decisions.md` § "R-160 — The integrator equations are transcribed into integrator_contract Part 2b"
- `decisions.md` § "R-161 ✱ — Heggie's default time transformation is the measured one, Eq. 22 at n = 3/2"
- `decisions.md` § "R-162 — The reversible occupant is logH's TTL time mode"

## Deliverables
- `crates/kernel/src/regularisation/{heggie,logh}.rs` — transcribed physics layers; logH carries its TTL time mode, the reversible occupant (R-162); Heggie's time transformation defaults to Eq. 22 at n = 3/2, with Eq. 20 selectable (R-161).
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The equations and the step control (R-160, R-161)" — the Heggie, logH and TTL equations of motion, time transformations and step control, and the predictive step limit, transcribed from `docs/reference/prin-rs/src/integrate/{heggie,logh}/` and `az/driver.rs` with citations (Heggie 1974; Mikkola & Tanikawa 1999; Preto & Tremaine 1999) (REQ-INT-084).
- Default occupant config = Heggie; `none`, AZ, logH selectable.
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The profile gains a field" — where the re-registration count lives (definition, R-72); if a payload field: the ledger row in `crates/ledger`, its generated accessors and the schema-version change.
- `fixtures/gates/radial-collision/` — the two-body radial collision case.

## Acceptance tests
- `cargo test -p kernel regularisation_default` — each stepper composes with each regularisation; the default config selects Heggie, with Eq. 22 at n = 3/2 as its time transformation and Eq. 20 selectable (R-161) (REQ-INT-006).
- Part 2b gives the equations, time transformations and step control for Heggie, logH and TTL and the predictive step limit, each cited to its paper and to the prin-rs file it was transcribed from; the physics reviewer approved; the human confirmed it at the M3 gate (REQ-INT-084).
- Review (physics): a consumer can read, per method or per trajectory, how many sync-boundary re-registrations occurred (REQ-INT-018).
- Part 2b states the choice; any payload field appears in the ledger with a schema-version change; physics reviewer approved (REQ-INT-073).
- `cargo xtask gate radial-collision` — the radial collision case passes through d_min ≈ 1e-11 with bounded energy drift (the AZ validation reported 6.2e-15 at d_min = 1.35e-11), run for the default occupant and AZ (REQ-VAL-045).

## Notes
- *Was: "Gap: Heggie's and logH's equations of motion and their step control are not in the corpus; REQ-INT-016 says they are transcribed from prin-rs, which is not in this repository."* RQ-102 ruled: R-159 imports the prin-rs reference set (`docs/reference/prin-rs`, reference, not authority); R-160 has this task transcribe the equations into Part 2b (REQ-INT-084), physics-reviewed and confirmed at the M3 gate. `FINDINGS.md:96`'s Eq. 20 statement is a documentation error; the default is Eq. 22 at n = 3/2 (R-161). TTL is built, validated and loses on accuracy in prin-rs, a prior finding (R-162).
- The Heggie-vs-AZ gate (REQ-INT-051) and the logH falsification (REQ-VAL-115) are TASK-M3-32's, after the comparison harness.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-073; and, under R-160, REQ-INT-084.
