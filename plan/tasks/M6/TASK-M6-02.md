# TASK-M6-02 — The split gate: the screen floor, the sliding MAX_REL_DEPTH and the motion refinement floor

- **Milestone:** M6
- **Closes:** REQ-SCHED-057, REQ-SCHED-058, REQ-SCHED-059, REQ-SCHED-060, REQ-SCHED-061, REQ-SCHED-069, REQ-SCHED-070, REQ-REF-012, REQ-REF-020, REQ-REF-021, REQ-SCHED-089
- **Depends on:** TASK-M6-01, TASK-M5-12
- **Needs (earlier milestones):** REQ-SCHED-040, REQ-SCHED-086, REQ-SCHED-020, REQ-SCHED-075, REQ-PERF-016, REQ-SCHED-048
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-3
- **Size:** ~480 lines

## Goal
The scheduler's split predicate becomes scheduler_contract Part 3's sliding form: `split(C) ⟺ (in_view(C) ∧ tile_size(C) > pixel_size) ∨ (policy_splits(C) ∧ ℓ < camera_depth + MAX_REL_DEPTH)`, with the terminal guard checked before the policy and no absolute depth constant anywhere. The screen-space floor is evaluated live against the current zoom, never cached, never terminal; off-screen quads read adequate at query time (stored nowhere); `MAX_REL_DEPTH` caps every split beyond the screen floor (R-98), is not on the sim key (R-89) and is rejected below the screen floor. In view above the floor a quad must split at rest; during a gesture the frame budget governs (R-108), and the motion refinement-floor lever lets leaves stop one or two levels above pixel size and snap back at rest. `policy_splits` is a trait seam here, filled by TASK-M6-03.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 3 — Depth: infinite zoom by default"
- `docs/contracts/principia_scheduler_contract.md` § "The absolute floor is emergent"
- `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound"
- `docs/contracts/principia_scheduler_contract.md` § "`MAX_REL_DEPTH` (renamed from `MAX_DEPTH`)"
- `decisions.md` § "R-15 — `Policy::Tolerance` governs refinement *(closes RQ-13)*"
- `decisions.md` § "R-88 — What stops in-view refinement *(closes RQ-39)*"
- `decisions.md` § "R-98 — `MAX_REL_DEPTH` caps every split beyond the screen floor *(closes RQ-58)*"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `decisions.md` § "R-108 — Must-split above the floor is the at-rest target *(closes RQ-68)*"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/read_first/principia_INDEX.md` § "The evidence base — where settled defaults were measured"
- `docs/design/principia_dd_refinement_policy.md` § "0.1 In view, the camera decides depth and the criterion decides ORDER"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/refine/gate.rs`: `split_gate(quad, camera, policy) -> Decision` in the Part 4 order (terminal guard → screen floor → `MAX_REL_DEPTH` → policy), `tile_size_px` computed live from the camera, `in_view` from the camera, `adequate` computed at query time.
- `MAX_REL_DEPTH` as scheduler state beside `frame_budget` (not in the sim-key hash); validation rejecting `MAX_REL_DEPTH` below the screen floor.
- `crates/engine/src/refine/motion_floor.rs`: the motion refinement-floor lever (a floor offset of one or two levels while a gesture is active, zero at rest), owned by the scheduler's motion adaptation, not by `QualitySettings`.
- A `Policy` trait with a test stub, so the gate is testable before TASK-M6-03 lands.
- Tests: `crates/engine/tests/split_gate.rs` (table-driven), a proptest navigation fuzzer in `crates/engine/tests/nav_fuzz.rs`, and the scripted zoom past depth 30.

## Acceptance tests
- `cargo test -p engine infinite_zoom_no_absolute_cap` + Review checklist (code) — drive a scripted zoom to depth > 30 with MAX_REL_DEPTH fixed; assert quads keep being scheduled below every fixed depth until a floor fires; review: no MAX_DEPTH-style absolute constant exists (REQ-SCHED-057).
- `cargo test -p engine split_gate_sliding` — for camera depths 0, 10, 30 and MAX_REL_DEPTH = 4: an in-view quad above the screen floor splits whatever the policy says; below the floor a policy-splitting quad at ℓ = camera_depth + 3 splits and at ℓ = camera_depth + 4 does not; an off-screen policy-splitting quad at ℓ = camera_depth + 4 does not split (REQ-SCHED-058).
- `cargo test -p engine max_rel_depth_not_sim_key` — lower MAX_REL_DEPTH while zoomed; assert the sim-key hash is unchanged and no cached quad is evicted or invalidated (REQ-SCHED-059).
- `cargo test -p engine screen_floor_live` — screen-floor a quad, zoom in 2×; assert it is re-queued as refinable and splits with real new samples (REQ-SCHED-060).
- `cargo test -p engine stop_hierarchy_table` — table-driven test over quads in each state asserts the split/stop/switch outcome per the hierarchy; at rest an in-view quad above the screen floor splits even when the policy keeps it; an off-screen quad is capped by MAX_REL_DEPTH; a MAX_REL_DEPTH below the screen floor is rejected (REQ-SCHED-061).
- `cargo test -p engine nav_fuzz_sliding_cap` (proptest) — fuzzed navigation never produces a quad beyond the sliding cap, an unsplit in-view quad above the screen floor at rest, or an illegal transition (REQ-SCHED-069).
- `cargo test -p engine motion_refinement_floor` — during a gesture leaves stop at tile ≈ 2–4 px; after rest they refine to ≤ 1 px (REQ-SCHED-070).
- `cargo test -p engine terminal_guard_before_policy` — a terminal quad is never passed to the policy; a non-terminal in-view quad above the screen floor splits; otherwise a non-terminal quad splits iff policy_splits and the depth gate holds (REQ-REF-012).
- `cargo test -p engine in_view_stops_at_one_pixel` — an in-view quad with n_unresolved = 0 and tile_size_px = 2 splits; at tile_size_px = 1 it keeps (REQ-REF-020).
- `cargo test -p engine offscreen_adequate_at_query` — an off-screen quad with tile_size_px ≫ 1 and n_unresolved = 0 keeps; an unresolved off-screen quad at ℓ = camera_depth + MAX_REL_DEPTH does not split; no adequacy field is persisted on the quad (REQ-REF-021).
- Proposal: the motion refinement floor's offset with frame-time and live-sample evidence from scripted pans; the human confirms it at the M6 gate (REQ-SCHED-089).

## Notes
- The at-rest vs gesture regime comes from caching Part 6 and the gesture debounce calibrated in M5 (REQ-SCHED-075); this task reads it, it does not set it.
- The motion floor's offset (one or two levels) is a controller lever (memory_tiers §5); TASK-M6-17 drives it. Which of the two the scheduler uses by default is not given — see Gaps.
- Closes, for gaps the corpus leaves open: REQ-SCHED-089 (R-71 calibration) (classification accepted by R-132).
