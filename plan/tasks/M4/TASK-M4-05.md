# TASK-M4-05 — Flat-grid compute: in-kernel decode from chart uniforms, one workgroup per quad, an in-place march

- **Milestone:** M4
- **Closes:** REQ-INT-070, REQ-PERF-012, REQ-PERF-013, REQ-SCHED-012, REQ-SYS-022, REQ-SCHED-010, REQ-SCHED-011, REQ-PERF-008
- **Depends on:** TASK-M4-01, TASK-M3-06, TASK-M3-15
- **Needs (earlier milestones):** REQ-SCHED-001, REQ-SYS-012, REQ-CHART-003, REQ-INT-026, REQ-PAY-025, REQ-PAY-038, REQ-PAY-037, REQ-PAY-014, REQ-SYS-003, REQ-SCHED-003, REQ-DEC-007
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
deep_zoom layer 0's compute side: a flat grid of equal quads covering the view, each dispatched as one workgroup of N² threads (64 at N = 8), one thread per texel. The kernel takes the chart description (z₀, basis vectors, chart id + params) and `SimUniforms` as uniforms and, per sample, maps → z → decode → (m, r, p) → integrate in-kernel; there is no IC buffer and per-frame CPU work is a uniform write and a dispatch. The march updates each `SimState` in place, re-dispatch is gated on `is_finished`, the word buffer is a separable allocation, and dispatch hard-asserts the `total_substeps` bound.

## References
- `docs/design/principia_core_design.md` § "3. Kernel eats chart params, decodes in-kernel"
- `docs/design/principia_systems_architecture.md` § "5.5 THE DISPATCH SHAPE — one thread per texel, one dispatch per ensemble copy"
- `docs/design/principia_systems_architecture.md` § "The shape"
- `docs/design/principia_systems_architecture.md` § "Not doing: worker tiles"
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/contracts/principia_lowering_contract.md` § "Part 1 — What lowering is"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree"

- `decisions.md` § "R-102 — The ensemble isn't a baked variant *(closes RQ-62)*"
- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"
## Deliverables
- `crates/engine/src/flat_grid.rs`: the layer-0 grid (quad list at one depth, per-quad uniforms `c, h`), `SimState` / word buffer allocation, dispatch of one workgroup per quad.
- `crates/kernel`: the survey entry point — workgroup size N², per-thread map → z → decode → wrapper; workgroup shared memory holds reduction accumulators only.
- Dispatch guards: `horizon_steps × N_max ≤ 2³²−1` hard assert; re-dispatch gated on `sd_is_finished`.
- Word buffer as its own allocation, bound only when symbolic features are on.

## Acceptance tests
- `cargo test -p engine dispatch_shape` — shader reflection: workgroup size N²; shared-memory size independent of E; the kernel has no loop over ensemble copies — each copy is the same kernel dispatched again with `copy_index` a uniform (R-102); the resolve pass is dispatched after the E+1 copy dispatches and is the only writer of the footprint resolve and QuadReduction (R-135) (REQ-PERF-012).
- Perf reviewer: no `k` (texels-per-thread) parameter exists in the dispatch configuration (REQ-PERF-013).
- `cargo test -p engine per_frame_uniform_write` plus code review — no IC buffer exists; per-frame CPU work is a uniform write and dispatch, independent of grid size (REQ-INT-070).
- Code reviewer: no ping-pong `SimState` copy exists; the march writes each sample's own slot in place (REQ-SCHED-012).
- `cargo test -p kernel discipline_lint` (extended) plus physics review — kernel source has no runtime switch on axis-kind / link / occupant enums, except the workgroup-uniform decode-mode branch (REQ-SYS-022).
- `cargo test -p engine horizon_substep_bound` — a config violating `horizon_steps × N_max ≤ 2³²−1` fails the assertion at dispatch (REQ-SCHED-010).
- `cargo test -p engine finished_gate` — a `sim_failed` sample is not re-dispatched; the predicate truth table for codes 0–7 (`is_resolved_outcome` ≤ 2, `is_running` = 3, `is_failed` ≥ 4 incl. reserved 6–7) (REQ-SCHED-011).
- `cargo test -p engine word_buffer_separable` — symbolic features disabled: word buffer freed, `SimState` allocation and contents unchanged (REQ-PERF-008).

## Notes
- RQ-92 ruled: R-124 — systems_architecture §5.5 is conformed to R-102 (applied in step 7; the heading above is the new one): each ensemble copy is the same kernel dispatched again with `copy_index` a uniform. This task builds the per-copy dispatch shape; the copy offsets and the (E+1) dispatches are TASK-M4-06's. This settles the gap.
- R-135 settles where the across-copy reduction lives: its own resolve pass, dispatched after all E+1 copy dispatches for a quad complete, reading their SimState slices and writing the footprint resolve and the QuadReduction fields (REQ-PERF-012). This task dispatches that pass; the reductions it computes are M5's (TASK-M5-17, TASK-M5-18).
