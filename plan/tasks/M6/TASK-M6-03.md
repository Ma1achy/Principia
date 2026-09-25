# TASK-M6-03 — Policy::Tolerance's split rule and the per-footprint latch

- **Milestone:** M6
- **Closes:** REQ-REF-022, REQ-REF-013, REQ-REF-019, REQ-REF-030, REQ-REF-036, REQ-REF-046
- **Depends on:** TASK-M6-01, TASK-M5-19
- **Needs (earlier milestones):** REQ-REF-002, REQ-REF-004, REQ-PAY-065, REQ-PAY-077, REQ-SCHED-047, REQ-SYS-033, REQ-VAL-083, REQ-REF-045
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-5, PIT-9
- **Size:** ~450 lines

## Goal
`Policy::Tolerance` (R-15) decides splits under the one knob `eps`: a quad splits iff any footprint is unresolved, where `unresolved(f) ⟺ spread_shape(f) > eps ∨ its latched running maximum ever exceeded eps ∨ the copies disagree on event class (incl. RUNNING vs terminal) ∨ the footprint is undetermined`, evaluated live at the playhead, with no exponent in the split test and `S_word` never read. The latch `running_max_divergence` (f32) is held per footprint with the resident quad (R-99), not in `QuadReduction`; `running_mean_divergence` and `first_divergence_t` are diagnostics. The task writes the R-72 definition of where the diagnostics are held into dd_generation_root's temporal-accumulators section; the latch's storage layout is TASK-M5-19's (REQ-REF-045, R-113). Everything is scored in payload space.

## References
- `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `decisions.md` § "R-15 — `Policy::Tolerance` governs refinement *(closes RQ-13)*"
- `decisions.md` § "R-91 — The temporal accumulators feed "unresolved" *(closes RQ-42)*"
- `decisions.md` § "R-99 — The latch is per footprint and lives with the resident quad *(closes RQ-59)*"
- `docs/contracts/principia_scheduler_contract.md` § "Part 8 — Continuous refinement & the live-to-live handoff"
- `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)"
- `docs/design/principia_dd_refinement_policy.md` § "4. THE METRIC MUST BE PAYLOAD-SPACE. THIS IS NOT OPTIONAL."
- `docs/read_first/principia_01_pitfalls.md` § "5. MEASURING IN THE WRONG SPACE — `error(B)` in OKLab"
- `docs/design/principia_temporal_architecture_note.md` § "Continuous refinement — temporal accumulators + spatial coherence"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-142 — The latch is evaluated on the GPU; only its verdict returns *(closes RQ-72)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- Doc change: `docs/design/principia_dd_generation_root.md` § "Temporal accumulators (scheduler Part 8)" — where the two diagnostics live and whether/how they cross GPU→CPU (REQ-REF-046), under R-142: the latch is evaluated on the GPU and only its verdict crosses.
- `crates/engine/src/refine/tolerance.rs`: `Policy::Tolerance { eps }`, `unresolved(footprint)`, `policy_splits(quad)` plugged into TASK-M6-02's gate.
- `crates/engine/src/refine/latch.rs`: the per-footprint latch store keyed by the resident quad (fixed size per footprint, O(1) in t), max-updated from each reduction readback.
- The ledger/generated layout (`crates/ledger`) has no `divergence_trend` and `QuadReduction` has no `running_max_divergence`.
- Tests: `crates/engine/tests/tolerance.rs` (disjunct table, diverge-then-reconverge fixture, S_word invariance) and a proptest that recolouring leaves the tree bit-identical.

## Acceptance tests
- `cargo test -p engine unresolved_disjuncts` — table test over footprints hitting each disjunct individually and none; a footprint whose spread fell back below eps after exceeding it stays unresolved (REQ-REF-022).
- `cargo test -p engine live_split_at_playhead` — a quad whose samples diverge mid-flight and reconverge splits at the divergence time and stays split while resident; changing S_word alone never changes the split decision (REQ-REF-013).
- `cargo test -p engine latch_per_footprint` + `cargo test -p ledger no_divergence_trend` — diverge-then-reconverge fixture: the footprint's running max stays; first_divergence_t written once; the generated layout has no divergence_trend and QuadReduction has no running_max_divergence (REQ-REF-019).
- `cargo test -p engine metric_payload_space` (proptest) — changing the palette/colouring leaves the refined tree and its error score bit-identical (REQ-REF-030).
- Review checklist (physics, code) — the scheduler calls the refinement policy for splits; the accumulators' size is fixed in t; no trend accumulator or θ threshold exists in code or config (REQ-REF-036).
- Review checklist (physics) — the doc states where both diagnostics live and how the GPU→CPU crossing carries them, if at all; physics reviewer approved; the doc change is merged with the physics reviewer's approval (REQ-REF-046).

## Notes
- RQ-72 ruled: R-142 — the latch is evaluated on the GPU in the resolve pass (R-135), its state stays in GPU-resident per-quad memory, and `QuadReduction` carries only the verdict: the count of unresolved footprints, latched ones included (REQ-REF-019, REQ-REF-022, REQ-REF-045).
- Pitfall regression: PIT-5 (a render-space metric). The recolouring proptest is the guard.
- RQ-100 ruled: R-113 — REQ-REF-045 moved to M5 (TASK-M5-19); this task reads the latch layout written there.
