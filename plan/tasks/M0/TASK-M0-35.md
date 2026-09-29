# TASK-M0-35 — The static layout check: overlap, coverage-or-reserved, width against range

- **Milestone:** M0
- **Closes:** REQ-GEN-003, REQ-GEN-028
- **Depends on:** TASK-M0-07
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~440 counted lines (R-240), plus R-242's signed and unrecognised-type cases

## Goal
Part (b) of TASK-M0-07 (R-240). Before the generator driver emits anything, the static check runs over every packed word: no two fields overlap, every bit is covered or explicitly reserved, and every width fits its range (generation-root §5), under R-242's width rules.

## References
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `decisions.md` § "R-240 — TASK-M0-07 is split in two *(closes RQ-151)*"
- `decisions.md` § "R-242 — The layout check's range and width rules *(closes RQ-151's rules)*"

## Deliverables
- `crates/ledger/src/check.rs` — the static check per packed word: overlap, coverage-or-reserved, width against range, under R-242's rules (unsigned and signed integer widths; an unbounded end fits no width; an unrecognised field type fails, naming the field and its type).
- The generator driver (`crates/ledger/src/gen/mod.rs`, TASK-M0-07) runs the static check before emitting; `cargo xtask codegen` refuses to generate on a finding.
- The fixture ledger (TASK-M0-07's) extended with the words these tests need.

## Acceptance tests
- `cargo test -p ledger layout_static` — static disjointness/coverage check over the ledger: an overlapping pair, an undeclared uncovered bit and a width too narrow for its range each fail, naming the word and the bits; the fixture ledger passes (REQ-GEN-003).
- `cargo test -p ledger layout_static` — an unsigned field too narrow for its range, a signed field too narrow for its range, an unbounded range on a packed field, and a field of an unrecognised type each fail, naming the word, the field and the bits (REQ-GEN-028).

## Notes
- R-240: TASK-M0-07 was built at 1003 counted lines; this part's code (~440) already exists on TASK-M0-07's local branch and is brought over once TASK-M0-07 merges. TASK-M0-09 depends on this task (applied per R-204, sequencing).
