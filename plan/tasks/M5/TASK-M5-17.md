# TASK-M5-17 — Resolve stage: outcome and validity reductions, and Undetermined quads

- **Milestone:** M5
- **Closes:** REQ-REF-001, REQ-VAL-083, REQ-REF-009, REQ-VAL-080, REQ-SCHED-098, REQ-REF-053, REQ-PAY-096
- **Depends on:** TASK-M5-01, TASK-M5-16
- **Needs (earlier milestones):** REQ-PAY-004, REQ-PAY-014, REQ-PAY-046, REQ-SYS-019, REQ-TOOL-033, REQ-TOOL-011
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
`N² × (E+1) ≤ 2³² − 1` (R-410). `escape_time_min` and `escape_time_max` start from their reductions' identities, +∞ and −∞,
with no no-escape branch, so a quad none of whose copies escaped stores exactly those bits (R-418).
It also defines render_contract Part 6's quad fields sim-failed flag, which R-424 moved here from the M1 checklist,
fills it with the validity members, and adds it to the field-view catalogue (REQ-PAY-096, R-72).

## References
- `docs/design/principia_dd_generation_root.md` § "Outcome (all at joint `class ⊕ detail` grain)"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_dd_simstate_payload.md` § "VERTICAL-SLICE ADDITIONS"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `decisions.md` § "R-20 — No `majority_class` field *(closes RQ-18)*"
- `decisions.md` § "R-410 — `class_histogram`'s bins are u32: the bound `N² × (E+1) ≤ 2³² − 1` is stated and asserted at dispatch, and no setting is capped *(closes RQ-266)*"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_dd_generation_root.md` § "Validity and diagnostics"
- `decisions.md` § "R-424 — The quad fields row's flags: contains-ensemble, contains-FTLE and the schema version are checked at M1 against `has_ensemble()`, `has_ftle` and `PAYLOAD_SCHEMA_VERSION`; sim-failed moves to the `QuadReduction` task and cache-valid to the M5 cache task *(closes RQ-275)*"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-418 — `escape_time_min`, `escape_time_max` and `mean_orbit_count_spread` join REQ-PAY-093, confirmed at the M5 gate; with no escape `_min` is +∞ and `_max` −∞, each its reduction's identity, and "no escape" reads `_min > _max` *(closes RQ-268)*"

## Deliverables
- `crates/kernel/src/reduce/outcome.rs` (shared source; the CPU reference driver runs it in f64).
- `crates/render`: the impurity-mask cross-check view (debug_tooling_plan §G).
- The dispatch assertion `N² × (E+1) ≤ 2³² − 1` in `crates/engine`, where the resolve stage is dispatched, with its test
  `resolve_histogram_bound` (REQ-SCHED-098).
- Tests `crates/kernel/tests/reduce_outcome.rs` (fixture footprints, triple events at detail 3, all-failed quad);
  `cargo xtask gate undetermined-starved` on the starved near-field fixture.
- The sim-failed definition (REQ-PAY-096, R-424) in `docs/design/principia_dd_generation_root.md` §3.7 (and §3.7a if a
  `RenderQuad` member is added), with the commit's "Removed lines" note; its ledger rows, its reduction, and the quad
  fields view's sim-failed cell in the generated catalogue.

## Acceptance tests
- `cargo test -p kernel reduce_outcome_histogram` — fixture footprints: dominant and impurity agree with the histogram; triple events at detail 3 (REQ-REF-001).
- `cargo test -p render impurity_mask_mean` — mask mean equals the reduction's outcome_impurity (REQ-VAL-083).
- `cargo xtask gate undetermined-starved` — starved near-field fixture (64/64 budget, 512/512 non-finite) → Undetermined, although its spread_median reads 4.58e-4 (REQ-REF-009).
- `cargo test -p kernel reduce_all_failed_undetermined` — a quad of all-failed samples reports Undetermined, not a small spread; a grep finds no filtering in reductions (REQ-VAL-080).
- Review checklist (qa) — a quad of all-failed samples reports Undetermined, not a small spread; a grep finds no filtering in reductions (REQ-VAL-080).
- `cargo test -p kernel reduce_escape_no_escape_bits` — a quad with no escaped copy stores `0x7c00` in `escape_time_min` and `0xfc00` in `escape_time_max`, compared by bits, and reads `_min > _max`; one escaped copy gives `_min == _max ==` its time; control: a no-escape branch storing +∞ in `_max` (RQ-268's option (d)) fails (REQ-REF-053).
- `cargo test -p engine resolve_histogram_bound` — a configuration with N² × (E+1) = 2³² − 1 passes the assertion at dispatch, one with 2³² fails it, naming the bound; every tier row of memory_tiers §4 passes; control: an assertion against u8's 255 fails on Medium (16² × 2 = 512) (REQ-SCHED-098).
- Review checklist (physics) — dd_generation_root §3.7 (and §3.7a if a `RenderQuad` member is added) defines what sim-failed holds per quad, from the samples whose `state` is `sim_failed` (§3.1's code 4; `valid_sample_count` counts decode failures only), and the member that carries it or the members it derives from; the ledger, the generated structs and the reduction follow it; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-096).
- Review checklist (qa) — render_contract Part 6's quad fields row's sim-failed cell is ticked against the generated catalogue on a synthetic payload, a quad with sim_failed samples and one without drawing differently (REQ-PAY-096).

## Notes
- The "no filtering in reductions" grep (REQ-VAL-080) is a CI lint in this PR, not a one-off search.
- **The histogram's bound (R-410, RQ-266's option (a)).** `class_histogram`'s bins are u32 (TASK-M5-01 writes it into
  §3.7), so dispatch asserts `N² × (E+1) ≤ 2³² − 1`, as REQ-SCHED-010 asserts its own bound: a hard assertion, never a
  clamp or a refusal in the settings UI, since no setting that can allocate reaches it and no setting is capped. Placed
  here, the task whose dispatch fills the histogram, applied per R-369 in R-410's port.
- Fractions over N² footprints take at most N²+1 values: tests must not assert a resolution the quantity cannot have
  (pitfalls §3).
- Definitions (R-72) this task writes: REQ-PAY-096.
- **R-424 (13 Oct 2026) rules RQ-275** (TASK-M1-19's pre-flight): render_contract Part 6's quad fields row read five
  payload/status flags from `ctx.quad` that no contract defined. "sim-failed moves to the QuadReduction task": this
  task, which builds `QuadReduction`'s validity reductions (applied per R-369; TASK-M5-01, which builds the struct and
  was in review as PR #191, keeps its scope). No quad-level sim_failed flag or count exists yet, so this task defines
  one (REQ-PAY-096, R-72); a member it adds to `QuadReduction` goes into TASK-M5-01's ledger rows here.
