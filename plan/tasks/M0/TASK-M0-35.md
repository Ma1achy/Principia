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
- `decisions.md` § "R-247 — The signed-width rule waits for the first signed type *(amends R-242; closes RQ-156 in part)*"
- `decisions.md` § "R-248 — Float types at a packed location: exact width; an f16 range lies within f16's finite range *(amends R-242; closes RQ-156)*"

## Deliverables
- `crates/ledger/src/check.rs` — the static check per packed word: overlap, coverage-or-reserved, width against range, under R-242's rules as R-247 and R-248 amend them (unsigned widths; an unbounded end fits no width; `f16-pair`/`fixed16` exactly 16 bits and `f32` exactly 32 at a packed location; anything else there, a vector included, fails naming the field and its type; an `f16-pair` range within ±65504, an unbounded end only with `overflow`). The schema gains the optional `overflow: saturate | inf` key (§3.8, R-248).
- The generator driver (`crates/ledger/src/gen/mod.rs`, TASK-M0-07) runs the static check before emitting; `cargo xtask codegen` refuses to generate on a finding.
- The fixture ledger (TASK-M0-07's) extended with the words these tests need.

## Acceptance tests
- `cargo test -p ledger layout_static` — static disjointness/coverage check over the ledger: an overlapping pair, an undeclared uncovered bit and a width too narrow for its range each fail, naming the word and the bits; the fixture ledger passes (REQ-GEN-003).
- `cargo test -p ledger layout_static` — an unsigned field too narrow for its range, an unbounded range on a packed u-bits field, an `f16-pair`/`fixed16` field not exactly 16 bits, an `f32` not exactly 32, a vector at a packed location, an `f16-pair` range beyond ±65504, and an `f16-pair` unbounded end with no `overflow` each fail, naming the word or entry, the field and the bits; an unbounded end with `overflow` passes (REQ-GEN-028; R-247 defers the signed case).

## Notes
- R-240: TASK-M0-07 was built at 1003 counted lines; this part's code (~440) already exists on TASK-M0-07's local branch and is brought over once TASK-M0-07 merges. TASK-M0-09 depends on this task (applied per R-204, sequencing).
- R-247, R-248 (close RQ-156): no signed type exists yet, so the signed test waits; float types at a packed location are checked by exact width, and an `f16-pair` range must fit f16. Drop ce8e559's `DerivedFrom` check when porting: TASK-M0-07's gate does it.
