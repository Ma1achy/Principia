# TASK-M2-54 — `arctan_sum_bits` confirmed in the constants register (R-699)

- **Milestone:** M2
- **Closes:** REQ-GEN-045
- **Depends on:** TASK-M2-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~20 lines

## Goal
R-699 confirms `arctan_sum_bits` at 100. The register still marks it provisional until M2, which blocks that gate. This task takes the mark off, as R-428's entries were.

## References
- `decisions.md` § "R-699 — `arctan_sum_bits` = 100 is confirmed as proposed: mid-plateau, with margin on both sides *(closes RQ-635)*"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"

## Deliverables
- `crates/ledger/src/constants.rs`: `ARCTAN_SUM_BITS` a `confirmed` entry citing R-699; its doc comment says so.
- `docs/design/principia_dd_generation_root.md` § 3.8: the entry's row moved from the provisional table to the confirmed entries, with its dated note.

## Acceptance tests
- `cargo test -p ledger --test arctan` — REQ-GEN-045's verify.
- `cargo xtask gate-provisional --milestone M2` — does not list `arctan_sum_bits` (REQ-GEN-045).

## Notes
- Added by R-699 (4 Nov 2026, RQ-635 (a)), applied per R-369: that ruling confirms the value; the register's change is code, so a task.
