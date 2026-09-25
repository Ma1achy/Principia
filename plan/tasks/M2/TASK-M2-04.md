# TASK-M2-04 — Energy normalisation η_E with an explicit off switch

- **Milestone:** M2
- **Closes:** REQ-DEC-024, REQ-DEC-041
- **Depends on:** TASK-M2-03
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~220 lines

## Goal
The optional post-momentum rescale pᵢ ← η_E·pᵢ, η_E = √((E* − U)/K₀), with feasibility E* ≥ U, exists in the shared decoder behind an explicit `Option` (R-25): `None` is off, and E* = 0 is a real target. Its behaviour at K₀ = 0 (the rest start) and on a failed E* ≥ U check is written into dd_decoder §3.7 and reviewed by the physics reviewer. The refusal of η_E on the invariant charts is closed with those charts (TASK-M2-12).

## References
- `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)"
- `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden on some charts"
- `docs/design/principia_dd_decoder.md` § "6. Deferred / flagged"
- `decisions.md` § "R-25 — `η_E` is kept, with an explicit off switch *(CD-5)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"
## Deliverables
- `crates/kernel/src/decode/energy_norm.rs` (`Option<Real>` target; applied only when the chart allows it).
- The dd_decoder §3.7 doc change: the outcome for K₀ = 0 and for E* < U (label or refusal), with a "Removed lines" note in the commit.
- Tests in `crates/kernel/tests/energy_norm.rs`.

## Acceptance tests
- `cargo test -p kernel energy_normalisation` — with E* set, the decoded energy equals E*; with the option `None`, momenta are untouched bit-for-bit; E* = 0 is treated as a real target (energy set to zero), not as off; the K₀ = 0 and E* < U cases produce the outcome the §3.7 definition states (REQ-DEC-024).
- Doc review (physics): dd_decoder §3.7 states the outcome for K₀ = 0 and for E* < U (label or refusal); physics reviewer approved before merge (REQ-DEC-041).

## Notes
- RQ-92 ruled: R-124 — R-25 is applied in the docs (dd_decoder §3.7, chart_reference §0.6 and §5.2, chart_decoder_contract Part 5, inverse_encode's flag table): the refusal covers every `Some(E*)`, including `Some(0)` (TASK-M2-12 tests it).
