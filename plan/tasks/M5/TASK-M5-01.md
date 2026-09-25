# TASK-M5-01 — QuadReduction ledger: member order, packing, histogram and impurity grain

- **Milestone:** M5
- **Closes:** REQ-PAY-075, REQ-PAY-076, REQ-PAY-077, REQ-REF-006, REQ-REF-007, REQ-REF-008, REQ-PAY-006, REQ-PAY-089
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-PAY-001, REQ-GEN-002, REQ-GEN-003, REQ-GEN-007, REQ-GEN-008, REQ-GEN-010
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~380 lines

## Goal
`QuadReduction` exists as a generated struct, produced by the ledger generator from a completed
generation-root §3.7 row set. The three definitions §3.7 leaves to "the task that builds it" are written into the doc:
the `class_histogram[N]` bin count and bin width (sized so a bin cannot overflow at the largest footprint count per quad),
which class fraction `outcome_impurity` takes, and the member order, the packing of the 2-bit and 5-bit members and the
final aligned size. The validity members carry their §3.7 types, `spread_t_end` is absent, and the CPU-side quad
metadata (`priority`, `cacheAge`, `lifecycle`, `computeCostMs`, `gentime`) lives in an engine struct, never in the GPU
struct. Nothing populates the reduction yet (TASK-M5-17 to TASK-M5-19 do); this PR fixes its shape.

## References
- `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger"
- `docs/design/principia_dd_generation_root.md` § "Outcome (all at joint `class ⊕ detail` grain)"
- `docs/design/principia_dd_generation_root.md` § "Validity and diagnostics"
- `docs/design/principia_dd_generation_root.md` § "Conditional — not yet included"
- `docs/design/principia_dd_generation_root.md` § "Not reduction fields"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-142 — The latch is evaluated on the GPU; only its verdict returns *(closes RQ-72)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `docs/design/principia_dd_generation_root.md` §3.7: the three definitions (REQ-PAY-075, REQ-PAY-076, REQ-PAY-077), with
  the commit's "Removed lines" note.
- `crates/ledger`: `QuadReduction` ledger rows (every §3.7 member, including the M6 refinement members `alpha_area`,
  `alpha_energy`, `worst_energy_drift`, laid out now and populated in M6), the generated Rust type in `crates/engine`
  and the generated WGSL struct and accessors for the render and kernel sides.
- `crates/engine/src/quad/meta.rs`: `QuadMeta` holding `priority`, `cache_age`, `lifecycle`, `compute_cost_ms`, `gentime`.
- Tests in `crates/ledger/tests/quad_reduction.rs`: member list and types, Rust/WGSL layout agreement, histogram capacity.

## Acceptance tests
- Review checklist (physics) — N matches the joint class ⊕ detail set; the bin width cannot overflow at the largest footprint count per quad (N² × (E+1)); the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-075).
- `cargo test -p ledger quad_reduction_histogram_capacity` — the test that follows from the written definition: N matches the joint class ⊕ detail set; the bin width cannot overflow at the largest footprint count per quad (N² × (E+1)) (REQ-PAY-075).
- Review checklist (physics) — the ledger row states the fraction; the impurity-mask cross-check (debug_tooling_plan §G) uses the same fraction; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-076).
- Review checklist (physics) — the ledger lists member order, packed-member bit positions and the aligned size; the generated Rust and WGSL layouts match it; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-077).
- `cargo test -p ledger quad_reduction_layout` — the test that follows from the written definition: the ledger lists member order, packed-member bit positions and the aligned size; the generated Rust and WGSL layouts match it (REQ-PAY-077).
- `cargo test -p ledger quad_reduction_members` — generated layout member list and types (REQ-REF-006).
- Review checklist (code) — every QuadReduction member is a fixed-size scalar; no array member grows with time or sample count; the ~80 B figure is not treated as a cap (REQ-PAY-006).
- `cargo test -p ledger quad_reduction_size` — `size_of::<QuadReduction>()` equals the aligned sum of its member list as REQ-PAY-077 defines it; the generated Rust and WGSL sizes agree (REQ-PAY-089).
- Review checklist (code) — no spread_t_end member in v1 (REQ-REF-007).
- Review checklist (code) — none of these in the GPU struct (REQ-REF-008).

## Notes
- Definitions (R-72) this task writes: REQ-PAY-075, REQ-PAY-076, REQ-PAY-077.
- The histogram capacity test must be able to fail: it asserts against the largest footprint count per quad,
  N² × (E+1) at the tier table's largest N and E (memory_tiers §4), not against a typical quad (pitfalls §3, "check the
  measurement can fire").
- The impurity grain chosen here is the one the impurity-mask cross-check (TASK-M5-17, REQ-VAL-083) uses.
- The temporal-accumulator members (`running_mean_divergence`, `first_divergence_t`) are laid out here; how the
  per-footprint latch reaches the split decision is R-142's: evaluated on the GPU in the resolve pass, with only the
  unresolved-footprint count in `QuadReduction` (see TASK-M5-19).
- RQ-93 ruled: R-113, option (b) — QuadReduction's sizing is at M5: REQ-PAY-006 moves here and REQ-PAY-089 (sized from its member list, then aligned) is split from REQ-PAY-001; the member list itself stays at M0 (TASK-M0-11).
