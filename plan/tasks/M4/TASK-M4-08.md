# TASK-M4-08 — The sim key, the render key and resolve()

- **Milestone:** M4
- **Closes:** REQ-SCHED-005, REQ-SCHED-007, REQ-INT-068, REQ-RENDER-030, REQ-SCHED-006, REQ-SYS-021
- **Depends on:** TASK-M4-06, TASK-M2-21, TASK-M3-21
- **Needs (earlier milestones):** REQ-INT-026, REQ-GEN-008, REQ-GUI-002, REQ-GUI-007, REQ-SYS-011, REQ-SYS-013, REQ-RENDER-006, REQ-RENDER-075, REQ-SCHED-002
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The recompute rule made mechanical. The sim key holds chart id + params, the slice plane (z₀'s out-of-plane part, span{q₁, q₂}, the in-plane orientation), warps, link ids, integrator config including the bound occupant and `N_max`, T, event thresholds, the tier's sim-key components (N, FTLE, word — not depth, not E) and the ledger content hash; a change re-boots the march from t = 0. Everything on the render key only recolours: no compute dispatch, sim buffers byte-identical. `resolve(ViewState, SimKey, RenderConfig)` runs engine-side in Rust and returns the compute key and uniforms, the fragment key and uniforms, and the dispatch plan.

## References
- `docs/contracts/principia_integrator_contract.md` § "The table gains two rows"
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `decisions.md` § "R-36 — Schema version = content hash of the ledger *(PL-1)*"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "0. The ladder — the organising abstraction"
- `docs/design/principia_systems_architecture.md` § "The two keys as ladder geometry"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_core_design.md` § "4. Integrate and colour are separate passes — and now separate *mechanisms*"
- `docs/design/principia_temporal_architecture_note.md` § "Render swapping is unaffected — the struct is the boundary"
- `docs/design/principia_colour_composition.md` § "0. Scope & membrane position"
- `docs/design/principia_dd_colouring.md` § "1. What it is"
- `docs/design/principia_dd_colouring.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 1 — What lowering is"
- `docs/contracts/principia_lowering_contract.md` § "Part 5 — The resolution function (the "switch", concretely)"

## Deliverables
- `crates/engine`: `SimKey` (hashable, every component above) and the render key.
- `crates/engine/src/resolve.rs`: `resolve` with the five outputs of lowering Part 5 (flat-grid dispatch plan).
- `crates/engine`: sim-key change → re-boot to t = 0; render-key change → fragment rebind only.

## Acceptance tests
- `cargo test -p engine sim_key_components` — changing each sim-key component changes the key and re-boots the state to t = 0; changing E, `MAX_REL_DEPTH`, an in-plane pan or zoom, the lock or any render-key component does not change the sim key (REQ-SCHED-007).
- `cargo test -p engine occupant_on_sim_key` — switching occupant changes the sim key, invalidates the payload and re-integrates; the parity harness asserts both sides bind the same occupant (REQ-SCHED-005).
- `cargo test -p engine n_max_sim_key` — changing `N_max` changes the sim key; the parity harness passes `N_max` identically to both instantiations (REQ-INT-068).
- `cargo test -p engine render_swap_no_dispatch` — swapping the colour occupant mid-march issues no compute dispatch and leaves the sim buffers byte-identical (REQ-RENDER-030).
- `cargo test -p engine render_key_property` (proptest; dd_colouring test 12 / seam "render key") — with the playhead held, cycle every render mode, stain-graph node, uniform and palette: zero compute dispatches and an unchanged sim-buffer hash (REQ-SCHED-006).
- `cargo test -p engine resolve_outputs` — `resolve` on a fixture config returns all five outputs; it is Rust, not TS (REQ-SYS-021).

## Notes
- The frame loop (TASK-M4-10) consumes the re-boot; this task tests it with the playhead held, driving the march directly.
- Waits on RQ-95 (`REVIEW_QUEUE.md`): M2 requirements that need M3, M4, M5 or an artboard.
- Waits on RQ-98 (`REVIEW_QUEUE.md`): M3 and M4 requirements that name later surfaces.
