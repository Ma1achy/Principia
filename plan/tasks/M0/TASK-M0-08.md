# TASK-M0-08 — The constants register: every settled constant carries its citation and admissibility class

- **Milestone:** M0
- **Closes:** REQ-SYS-001, REQ-SYS-005, REQ-VAL-006, REQ-SYS-063
- **Depends on:** TASK-M0-07
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
`crates/ledger` holds a constants register beside the layout table. Each settled default or threshold is declared once with its value, its admissibility class (bounded by its own achievable maximum; fixed by a conservation law; expressed in canonical units — philosophy §4.2) and a citation of where it was measured or derived (a corpus section, a prin-rs FINDINGS/results path per INDEX's evidence base, or a calibration requirement id). A threshold on a quantity spanning decades also records that it is relative and the distribution or measured gap it was set from (pitfalls §3). Generation fails on an entry missing any of these, and `cargo xtask lint constants` fails on a numeric constant in the physics and engine crates that is not read from the register.

## References
- `docs/contracts/principia_canonical_spec.md` § "Principia — canonical architecture & design (the yardstick)"
- `docs/read_first/principia_INDEX.md` § "The evidence base — where settled defaults were measured"
- `docs/read_first/principia_00_philosophy.md` § "4.2 Every constant must be derived or absent"
- `docs/read_first/principia_00_philosophy.md` § "6. How to tell if a decision is drifting"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `decisions.md` § "R-36 — Schema version = content hash of the ledger *(PL-1)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/ledger/src/constants.rs` — `Constant { name, value, class, citation, relative_basis }` and the register; the generation gate.
- The register's M0 entries, the constants the payload ledger uses: the `horizon_steps` limit 65535 (R-86), the word capacity 76 and the length sentinel 127 (payload §3), the diffusion sentinel −1.0 (generation-root §3.4) and the f16 pack clamp ±65504 (payload §1), each with its class and citation.
- `xtask/src/lint_constants.rs` — `cargo xtask lint constants` over `crates/{kernel,ledger,engine}` (generated files excluded: their numbers are emitted from the ledger); registered in `cargo xtask ci`.

## Acceptance tests
- `cargo test -p ledger constants_gate` — an entry without a citation fails generation naming the constant (REQ-SYS-001); an entry without an admissibility class fails the same way (REQ-SYS-005).
- `cargo test -p ledger constants_threshold` — a threshold entry with no relative basis (distribution or measured gap) fails; fixture entries reproducing the recorded regressions — an absolute closure cutoff of 2e-3 inside the bound population's range, and tau_display at the 0.4th percentile — fail (REQ-VAL-006).
- `cargo xtask lint constants` — passes on the tree; a bare numeric `const` added to `crates/kernel` fails naming file and line (REQ-SYS-001, REQ-SYS-005).
- Definition: the constants register's entry fields and its relation to the hashed ledger written into dd_generation_root §3.8 and approved by the physics reviewer (REQ-SYS-063).

## Notes
- Constants the corpus leaves open are not entered with a value: they are calibration requirements, and the register cites the requirement id until the human confirms the value (R-71).
- See Gaps: where the register lives relative to the §3.8 schema and the R-36 hash; prin-rs citations are not resolvable from this repo.
- Closes, for gaps the corpus leaves open: REQ-SYS-063 (R-72 definition) (classification accepted by R-132).
