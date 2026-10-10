# TASK-M5-17 — Resolve stage: outcome and validity reductions, and Undetermined quads

- **Milestone:** M5
- **Closes:** REQ-REF-001, REQ-VAL-083, REQ-REF-009, REQ-VAL-080, REQ-SCHED-098
- **Depends on:** TASK-M5-01, TASK-M5-16
- **Needs (earlier milestones):** REQ-PAY-004, REQ-PAY-014, REQ-PAY-046, REQ-SYS-019, REQ-TOOL-033
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~420 lines

## Goal
The data-side resolve pass fills `QuadReduction`'s outcome and validity members from a quad's samples:
`class_histogram`, `dominant_outcome` and `outcome_impurity` from the one histogram at the joint class ⊕ detail grain,
`terminated_fraction`, and the validity members. A quad whose every footprint is budget-exhausted with all copies
unusable reports `Undetermined` (`footprint_undetermined`), detected from those conditions, never from the spread
statistic; no reduction filters samples. The impurity-mask cross-check view compares each sample's class ⊕ detail with
the quad's `dominant_outcome`, and its spatial mean equals `outcome_impurity`. Dispatch asserts the histogram's bound,
`N² × (E+1) ≤ 2³² − 1` (R-410).

## References
- `docs/design/principia_dd_generation_root.md` § "Outcome (all at joint `class ⊕ detail` grain)"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_dd_simstate_payload.md` § "VERTICAL-SLICE ADDITIONS"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `decisions.md` § "R-20 — No `majority_class` field *(closes RQ-18)*"
- `decisions.md` § "R-410 — `class_histogram`'s bins are u32: the bound `N² × (E+1) ≤ 2³² − 1` is stated and asserted at dispatch, and no setting is capped *(closes RQ-266)*"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"

## Deliverables
- `crates/kernel/src/reduce/outcome.rs` (shared source; the CPU reference driver runs it in f64).
- `crates/render`: the impurity-mask cross-check view (debug_tooling_plan §G).
- The dispatch assertion `N² × (E+1) ≤ 2³² − 1` in `crates/engine`, where the resolve stage is dispatched, with its test
  `resolve_histogram_bound` (REQ-SCHED-098).
- Tests `crates/kernel/tests/reduce_outcome.rs` (fixture footprints, triple events at detail 3, all-failed quad);
  `cargo xtask gate undetermined-starved` on the starved near-field fixture.

## Acceptance tests
- `cargo test -p kernel reduce_outcome_histogram` — fixture footprints: dominant and impurity agree with the histogram; triple events at detail 3 (REQ-REF-001).
- `cargo test -p render impurity_mask_mean` — mask mean equals the reduction's outcome_impurity (REQ-VAL-083).
- `cargo xtask gate undetermined-starved` — starved near-field fixture (64/64 budget, 512/512 non-finite) → Undetermined, although its spread_median reads 4.58e-4 (REQ-REF-009).
- `cargo test -p kernel reduce_all_failed_undetermined` — a quad of all-failed samples reports Undetermined, not a small spread; a grep finds no filtering in reductions (REQ-VAL-080).
- Review checklist (qa) — a quad of all-failed samples reports Undetermined, not a small spread; a grep finds no filtering in reductions (REQ-VAL-080).
- `cargo test -p engine resolve_histogram_bound` — a configuration with N² × (E+1) = 2³² − 1 passes the assertion at dispatch, one with 2³² fails it, naming the bound; every tier row of memory_tiers §4 passes; control: an assertion against u8's 255 fails on Medium (16² × 2 = 512) (REQ-SCHED-098).

## Notes
- The "no filtering in reductions" grep (REQ-VAL-080) is a CI lint in this PR, not a one-off search.
- **The histogram's bound (R-410, RQ-266's option (a)).** `class_histogram`'s bins are u32 (TASK-M5-01 writes it into
  §3.7), so dispatch asserts `N² × (E+1) ≤ 2³² − 1`, as REQ-SCHED-010 asserts its own bound: a hard assertion, never a
  clamp or a refusal in the settings UI, since no setting that can allocate reaches it and no setting is capped. Placed
  here, the task whose dispatch fills the histogram, applied per R-369 in R-410's port.
- Fractions over N² footprints take at most N²+1 values: tests must not assert a resolution the quantity cannot have
  (pitfalls §3).
