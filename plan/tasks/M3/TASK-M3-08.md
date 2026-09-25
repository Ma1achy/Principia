# TASK-M3-08 — Heggie and logH occupants, and the visible re-registration count

- **Milestone:** M3
- **Closes:** REQ-INT-006, REQ-INT-018, REQ-INT-073, REQ-VAL-045
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

## Deliverables
- `crates/kernel/src/regularisation/{heggie,logh}.rs` — transcribed physics layers.
- Default occupant config = Heggie; `none`, AZ, logH selectable.
- Doc change: `docs/contracts/principia_integrator_contract.md` § "The profile gains a field" — where the re-registration count lives (definition, R-72); if a payload field: the ledger row in `crates/ledger`, its generated accessors and the schema-version change.
- `fixtures/gates/radial-collision/` — the two-body radial collision case.

## Acceptance tests
- `cargo test -p kernel regularisation_default` — each stepper composes with each regularisation; the default config selects Heggie (REQ-INT-006).
- Review (physics): a consumer can read, per method or per trajectory, how many sync-boundary re-registrations occurred (REQ-INT-018).
- Part 2b states the choice; any payload field appears in the ledger with a schema-version change; physics reviewer approved (REQ-INT-073).
- `cargo xtask gate radial-collision` — the radial collision case passes through d_min ≈ 1e-11 with bounded energy drift (the AZ validation reported 6.2e-15 at d_min = 1.35e-11), run for the default occupant and AZ (REQ-VAL-045).

## Notes
- Gap reported: Heggie's and logH's equations of motion and their step control are not in the corpus; REQ-INT-016 says they are transcribed from prin-rs, which is not in this repository.
- The Heggie-vs-AZ gate (REQ-INT-051) and the logH falsification (REQ-VAL-115) are TASK-M3-32's, after the comparison harness.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-073.
