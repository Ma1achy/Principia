# TASK-M3-24 — The ground-truth harness: Path A, Path B, the fixture set and the golden anchors

- **Milestone:** M3
- **Closes:** REQ-VAL-029, REQ-VAL-046, REQ-VAL-048, REQ-VAL-049, REQ-VAL-126, REQ-VAL-128, REQ-VAL-129
- **Depends on:** TASK-M3-19, TASK-M2-26
- **Needs (earlier milestones):** REQ-ENC-002, REQ-ENC-012, REQ-VAL-014, REQ-VAL-016, REQ-VAL-018, REQ-DEC-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~450 lines

## Goal
The ground-truth validation suite exists in `crates/validation`, separate from parity, comparing the CPU-f64 instantiation against external truth in four tiers. Known ICs enter by Path A (canonicalise → encode → CHECK 1 round-trip → the general forward pipeline → CHECK 2 behaviour) or, for exactly-degenerate analytic configs, Path B (a lower harness entry taking physical `(m, r, p)`, bypassing decode). The validation note defines Path B's entry and the fixture set (R-72); benchmark runs record the pinned, calibrated `r_coll`. The integrator unit suite uses the golden anchors z = 0 and the Burrau IC encoded from (3,4,5).

## References
- `docs/design/principia_dd_integrator.md` § "5. Unit tests"
- `docs/notes/principia_validation_ground_truth_note.md` § "Self-consistency ≠ correctness (why this is a separate suite)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "The correctness factoring: validate on CPU, inherit on GPU"
- `docs/contracts/principia_parity_contract.md` § "7. What this contract does *not* cover"
- `docs/contracts/principia_canonical_spec.md` § "7. Precision & validation model *(authoritative: `parity_contract`, `validation_ground_truth_note`, `core_design`)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Two entry paths (routed by interior-vs-degenerate)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Everything is representable in principle — so a failed round-trip is a BUG signal"
- `docs/notes/principia_validation_ground_truth_note.md` § "Open sub-questions (settle at implementation)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/validation/src/ground_truth/{mod,path_a,path_b,tiers}.rs` — the two entry paths and the four tiers, CPU-f64 only.
- Doc changes: `docs/notes/principia_validation_ground_truth_note.md` — Path B's signature and what it bypasses (REQ-VAL-129); the fixture set, each with its source and asserted quantities (REQ-VAL-126).
- `fixtures/ground_truth/` — the defined fixtures; `fixtures/anchors/` — z = 0 and Burrau (3,4,5).
- Benchmark provenance carrying `r_coll`; calibration proposal for the benchmark `r_coll` derived from the reference computation's regularisation.

## Acceptance tests
- `cargo test -p kernel golden_anchors` — both anchors (z = 0, and Burrau encoded from (3,4,5)) decode deterministically and are exercised by the integrator suite (REQ-VAL-029).
- Review (physics): the suite exists with at least one case per tier, runs on the CPU-f64 build; GPU correctness is claimed only via parity (REQ-VAL-046).
- `cargo test -p validation entry_paths` — figure-eight/Burrau via Path A report the CHECK 1 residual and CHECK 2 behaviour; Euler collinear via the Path B entry point (REQ-VAL-048).
- Review (physics): Burrau and other benchmark runs carry the pinned r_coll in provenance (REQ-VAL-049).
- The note lists each fixture with its source and the quantities asserted; physics reviewer approved (REQ-VAL-126).
- The proposal cites the reference computation's regularisation and derives the benchmark r_coll from it; the human confirms it at the M3 gate and it is recorded in decisions.md (REQ-VAL-128).
- The note gives the Path B entry's signature and what it bypasses; physics reviewer approved (REQ-VAL-129).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-128.
- Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-126, REQ-VAL-129.
