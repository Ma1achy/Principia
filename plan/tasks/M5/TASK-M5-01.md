# TASK-M5-01 — QuadReduction ledger: member order, packing, histogram and impurity grain

- **Milestone:** M5
- **Closes:** REQ-PAY-075, REQ-PAY-076, REQ-PAY-077, REQ-REF-006, REQ-REF-007, REQ-REF-008, REQ-PAY-006, REQ-PAY-089, REQ-GEN-030, REQ-REF-052, REQ-PAY-093, REQ-PAY-094
- **Depends on:** TASK-M1-08
- **Needs (earlier milestones):** REQ-PAY-001, REQ-GEN-002, REQ-GEN-003, REQ-GEN-007, REQ-GEN-008, REQ-GEN-010
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-3, PIT-9
- **Size:** ~380 lines

## Goal
`QuadReduction` exists as a generated struct, produced by the ledger generator from a completed
generation-root §3.7 row set. The three definitions §3.7 leaves to "the task that builds it" are written into the doc:
the `class_histogram[N]` bin count, with its u32 bins and the bound they hold (R-410),
which class fraction `outcome_impurity` takes, and the member order, the packing of the 2-bit and 5-bit members and the
final aligned size. The validity members carry their §3.7 types, `spread_t_end` is absent, and the CPU-side quad
metadata (`priority`, `cacheAge`, `lifecycle`, `computeCostMs`, `gentime`) lives in an engine struct, never in the GPU
struct. Nothing populates the reduction yet (TASK-M5-17 to TASK-M5-19 do); this PR fixes its shape.

## References
- `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger"
- `docs/design/principia_dd_generation_root.md` § "Outcome (all at joint `class ⊕ detail` grain)"
- `docs/design/principia_dd_generation_root.md` § "Validity and diagnostics"
- `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)"
- `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors"
- `docs/design/principia_dd_generation_root.md` § "Refinement — the scaling exponent"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `docs/design/principia_memory_tiers.md` § "4. The six quality tiers"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/design/principia_dd_generation_root.md` § "Conditional — not yet included"
- `docs/design/principia_dd_generation_root.md` § "Not reduction fields"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-142 — The latch is evaluated on the GPU; only its verdict returns *(closes RQ-72)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-306 — `QuadReduction`'s member list is ledger data at M0; the struct is built at M5 *(closes RQ-178)*"
- `decisions.md` § "R-312 — §3.8 gains an `f16` type: one half-float at a packed 16-bit location, under R-248's rules *(closes RQ-182)*"
- `decisions.md` § "R-315 — `n_unresolved` is a u16 `QuadReduction` member, like `valid_sample_count` *(closes RQ-183)*"
- `decisions.md` § "R-317 — #98's `f16` restriction stands; `f16` is storage-only"
- `decisions.md` § "R-248 — Float types at a packed location: exact width; an f16 range lies within f16's finite range *(amends R-242; closes RQ-156)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- `decisions.md` § "R-137 — The Ultra and Extreme rows are provisional *(closes RQ-115)*"
- `decisions.md` § "R-398 — `N` is a power of two at every tier and setting, so the sample coordinates are dyadic, as the quadtree's are *(closes RQ-258)*"
- `decisions.md` § "R-410 — `class_histogram`'s bins are u32: the bound `N² × (E+1) ≤ 2³² − 1` is stated and asserted at dispatch, and no setting is capped *(closes RQ-266)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `docs/design/principia_dd_generation_root.md` §3.7: the three definitions (REQ-PAY-075, REQ-PAY-076, REQ-PAY-077), and
  `first_divergence_t`'s sentinel and each member's scale (REQ-PAY-094), with the commit's "Removed lines" note.
- The REQ-PAY-093 proposal: the declared range, and `overflow` where needed, of `error_ratio`, `roundtrip_error`,
  `alpha_area`, `alpha_energy` and `worst_energy_drift`, each with its evidence, attached to the PR.
- `crates/ledger`: `QuadReduction` ledger rows (every §3.7 member, including the M6 refinement members `alpha_area`,
  `alpha_energy`, `worst_energy_drift`, laid out now and populated in M6), the generated Rust type in `crates/engine`
  and the generated WGSL struct and accessors for the render and kernel sides.
- `crates/engine/src/quad/meta.rs`: `QuadMeta` holding `priority`, `cache_age`, `lifecycle`, `compute_cost_ms`, `gentime`.
- `QuadReduction`'s measured aligned size (R-410), written into `docs/design/principia_dd_generation_root.md` §3.7's
  "Size" paragraph and where the docs cite it: `docs/design/principia_systems_architecture.md` (the Memory and
  Reduction rows and the two "big data never crosses" passages), `docs/contracts/principia_render_contract.md`
  (Part 1's opening), `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model" (the struct
  line), `plan/reviewers/perf.md` § "3. Memory tiers and budgets", and REQ-PAY-089's and REQ-SYS-036's statements
  (reqio), with the commit's "Removed lines" note.
- Tests in `crates/ledger/tests/quad_reduction.rs`: member list and types, Rust/WGSL layout agreement, histogram capacity.

## Acceptance tests
- Review checklist (physics) — the bin count (§3.7's `N` in `class_histogram[N]`, written N_bins here) matches the joint class ⊕ detail set; each bin is a u32 (R-410); §3.7 states the bound N² × (E+1) ≤ 2³² − 1, where N is the samples per quad side (memory_tiers §4; not the bin count) and E+1 the copies per footprint; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-075).
- `cargo test -p ledger quad_reduction_histogram_capacity` — the test that follows from the written definition: the bin count (§3.7's `N` in `class_histogram[N]`, written N_bins here) matches the joint class ⊕ detail set; a u32 bin counts Extreme's provisional 16² × 16 = 4096 and the bound 2³² − 1 without wrapping; control: §3.7's former u8 bin fails at Medium's 16² × 2 = 512 (REQ-PAY-075).
- Review checklist (physics) — the ledger row states the fraction; the impurity-mask cross-check (debug_tooling_plan §G) uses the same fraction; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-076).
- Review checklist (physics) — the ledger lists member order, packed-member bit positions and the aligned size; the generated Rust and WGSL layouts match it; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-077).
- `cargo test -p ledger quad_reduction_layout` — the test that follows from the written definition: the ledger lists member order, packed-member bit positions and the aligned size; the generated Rust and WGSL layouts match it (REQ-PAY-077).
- `cargo test -p ledger quad_reduction_members` — generated layout member list and types (REQ-REF-006).
- Review checklist (code) — every QuadReduction member is a fixed-size scalar; no array member grows with time or sample count; QuadReduction's size (R-410; this task measures it) is not treated as a cap (REQ-PAY-006).
- `cargo test -p ledger quad_reduction_size` — `size_of::<QuadReduction>()` equals the aligned sum of its member list as REQ-PAY-077 defines it; the generated Rust and WGSL sizes agree (REQ-PAY-089).
- Review checklist (perf) — `QuadReduction`'s aligned size with u32 bins, and its cost at memory_tiers §3–§4's quad count (render pixels / N² at each tier's `N`, at the largest display, plus their ancestors), are stated in the PR from the member list; the perf reviewer confirms the size cost (R-410; REQ-PAY-089).
- Review checklist (code) — no spread_t_end member in v1 (REQ-REF-007).
- Review checklist (code) — none of these in the GPU struct (REQ-REF-008).
- Review checklist (physics) — the PR gives the declared range of `error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy` and `worst_energy_drift`, each within ±65504 or with its `overflow` stated (R-248), with its evidence; the physics reviewer checks each; the human confirms them at the M5 gate (REQ-PAY-093).
- Review checklist (physics) — §3.7 gives `first_divergence_t`'s sentinel, distinct from every crossing time and not NaN, and each member's scale; the ledger entries carry them; the doc change is in this PR and the physics reviewer approves it before merge (REQ-PAY-094).
- `cargo test -p ledger layout_static_f16` — §3.8's `f16` type: an f16 field 15 or 17 bits wide, at a scalar index, as a vector component, with a range beyond ±65504, and with an unbounded end and no `overflow` each fail, naming the field; a 16-bit f16 field within range passes, and one with an unbounded end and `overflow` passes; the generated Rust and WGSL accessors read the same binary16 value as f32; the generated WGSL contains no `enable f16` and no `f16` type; each has a registered negative control (REQ-GEN-030).

## Notes
- Definitions (R-72) this task writes: REQ-PAY-075, REQ-PAY-076, REQ-PAY-077, REQ-PAY-094. Calibrations (R-71) it
  proposes: REQ-PAY-093, used provisionally until the human confirms them at the M5 gate (R-182).
- The histogram capacity test must be able to fail: it asserts against the largest footprint count per quad,
  N² × (E+1), not against a typical quad (pitfalls §3, "check the measurement can fire"). Two different N's meet here:
  §3.7's `class_histogram[N]` bin count, and memory_tiers' `N`, the samples per quad side; REQ-PAY-075's verify keeps
  them apart. E has no corpus bound (Extreme's E = 15 is provisional, R-137, and Custom mode sets E directly,
  unbounded), and N = 16 is R-132's cap for Ultra and Extreme only: Custom's N is bounded by
  `N² ≤ maxComputeInvocationsPerWorkgroup` (memory_tiers § "5. Controller levers, ranked by impact", REQ-PERF-011),
  so N = 32 on a 1024-invocation adapter. A bin `w` bits wide overflows when N² × (E+1) > 2^w − 1 (§3.7's former u8
  already at Medium, 16² × 2 = 512), and a bin that wraps silently corrupts `dominant_outcome` and `outcome_impurity`.
  **RQ-266 ruled: R-410, option (a), u32 bins.** §3.7 states the bound, N² × (E+1) ≤ 2³² − 1 (overflow needs more than
  about 4.2 M copies per footprint at N = 32, which can't be allocated); no setting is capped; dispatch asserts the
  bound (REQ-SCHED-098, TASK-M5-17). The capacity test runs at Extreme's provisional E = 15 and N = 16 (R-398, R-132),
  N² × (E+1) = 4096, and at the bound 2³² − 1, with §3.7's former u8 bin as its control. The width is fixed, so the
  layout does not follow the setting, and each bin is a full u32 under §3.8's vector rule. RQ-266 no longer holds this
  task or TASK-M5-17; this task is still held by the M1 gate (and the gates before M5), unless it passes R-415's test
  for merging early.
- **Perf reviews this task (R-410: "Perf to confirm the size cost to QuadReduction").** u32 bins make `QuadReduction`
  larger than §3.7's u8 bins would; the perf reviewer confirms its aligned size and its cost at
  memory_tiers §3–§4's quad count (render pixels / N² at each tier's `N`, at the largest display, plus their ancestors). The perf reviewer was added in R-410's port (applied per R-369). The
  former descriptive ~80 B and §3.7's "~4k quads … ~0.3 MB" are no longer stated; this task writes the measured size
  in their place (perf review 5481098292 of PR #189, applied per R-369).
- The impurity grain chosen here is the one the impurity-mask cross-check (TASK-M5-17, REQ-VAL-083) uses.
- The temporal-accumulator members (`running_mean_divergence`, `first_divergence_t`) are laid out here; how the
  per-footprint latch reaches the split decision is R-142's: evaluated on the GPU in the resolve pass, with only the
  unresolved-footprint count in `QuadReduction` (see TASK-M5-19).
- RQ-93 ruled: R-113, option (b) — QuadReduction's sizing is at M5: REQ-PAY-006 moves here and REQ-PAY-089 (sized from its member list, then aligned) is split from REQ-PAY-001; the member list itself stays at M0 (TASK-M0-11).
- RQ-178 ruled: R-306 — M0 (TASK-M0-11) records §3.7's member list as ledger data, not emitted; this task builds the
  generated struct from it, with the members' §3.8 entries and `class_histogram`'s placement (§3.8's vector rule reads
  a `u-bits` component at a scalar index as a full u32, so `u8 × N` needed a placement here; under R-410 each bin is a
  u32, a full u32 slot).
- RQ-182 ruled: R-312 — §3.8 gains a single `f16` type, one half-float at a packed 16-bit location, under R-248's
  rules; `f16-pair` stays two halves. This task adds `f16` to the ledger's type set, the static check and both
  emitters (REQ-GEN-030), and types `QuadReduction`'s f16 members with it. Stable Rust has no `f16` type, so the
  generated struct stores binary16 bits.
- RQ-183 ruled: R-315 — `n_unresolved` is a u16 member (§3.7's temporal-accumulators row), the latch's verdict
  (REQ-REF-052). R-317 — `f16` is storage-only: its accessors widen to f32, and no generated WGSL uses `enable f16`.
- **The members' §3.8 entries (R-306), where the corpus leaves them open** (the R-388 pre-flight, 10 Oct 2026): §3.8
  says "A field without a complete entry fails generation loudly", but the corpus gives no range for the f16 members
  `error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy` and `worst_energy_drift` (an f16 range lies within
  ±65504 or the entry states its `overflow`, R-248, R-312), no value for `first_divergence_t`'s "sentinel until
  crossed", and no member's scale. The ranges are REQ-PAY-093 (calibration, R-71: proposed with evidence, e.g. §3.7's
  measured `error_ratio` of 204.8, confirmed at the M5 gate); the sentinel and the scales are REQ-PAY-094 (definition,
  R-72: written into §3.7, the physics reviewer approving). Where §3.7 states a bound (a fraction in [0, 1],
  `spread_shape`'s chord bound, `spread_event`'s attainable maximum, `n_unresolved` at most N²), the entry transcribes
  it, citing the line.
- **REQ-REF-046** (TASK-M6-03's definition) was reworded in the pre-flight (requirements.yaml's rule: the source wins):
  §3.7's temporal-accumulators table lists `running_mean_divergence` and `first_divergence_t` as f32 `QuadReduction`
  members, and R-99 makes them diagnostics, not split inputs; R-99 does not take them out of the struct. This task lays
  both out as members, as §3.7 has them.
- **REQ-PAY-077's `rq: RQ-182`** is dropped: R-312 ruled RQ-182, and R-312 joins its rulings.
