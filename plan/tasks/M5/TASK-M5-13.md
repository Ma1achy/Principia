# TASK-M5-13 — Navigation as chart-uniform edits: in-plane re-addressing and new identities

- **Milestone:** M5
- **Closes:** REQ-CHART-038, REQ-SCHED-025, REQ-SCHED-054
- **Depends on:** TASK-M5-04, TASK-M5-06, TASK-M5-08
- **Needs (earlier milestones):** REQ-GUI-004, REQ-GUI-005, REQ-GUI-007, REQ-CHART-007, REQ-SYS-022
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~320 lines

## Goal
Every gesture is a CPU-side diff of the one chart uniform (z₀, q₁, q₂): pan and slice edit z₀, zoom and tilt the
basis, the lock pins the centre, with non-raw directions taken as tangents at the centre. In-plane pan and zoom only
change which quad addresses are asked for; slicing out of the plane, tilting and rotating request new identities that
integrate fresh; the lock changes neither key; no navigation invalidates a cached payload. The GPU has no locked mode,
view mode, camera object or second code path.

## References
- `docs/contracts/principia_canonical_spec.md` § "4. Charts & navigation *(authoritative: `chart_decoder_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 4 — Navigation is chart construction (pan, slice, zoom, tilt, lock)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Design axioms (the six that must survive contact with a code agent)"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "The two keys as ladder geometry"
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/read_first/principia_00_philosophy.md` § "The projective microscope — addressing, not computing"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"
- `decisions.md` § "R-97 — Quad addresses live in the slice plane *(closes RQ-57)*"

## Deliverables
- `crates/engine/src/view/navigate.rs`: gesture → (z₀, q₁, q₂) diff; plane-anchor update on re-integrating events.
- Tests `crates/engine/tests/navigation.rs`: gesture diffs, pan away and back, tilt away and back, no invalidation event.
- A kernel reflection check that no mode flag exists.

## Acceptance tests
- `cargo test -p engine gesture_is_uniform_diff` — each gesture produces only a diff of (z₀, q₁, q₂); the kernel has no mode flag (REQ-CHART-038).
- `cargo test -p engine pan_tilt_away_and_back` — pan away and back: cached entries are still valid and no integration is dispatched for them; tilt away and back: the original plane's entries are still present and valid; no invalidation event fires (REQ-SCHED-025).
- `cargo test -p engine tilt_new_identities` — Tilt the plane and back; assert cached entries for the original plane are still present and valid and the tilted planes were added as new identities (REQ-SCHED-054).

## Notes
- Revealed quads catching up to the playhead off-loop (REQ-SCHED-025's last clause) uses the catch-up path of
  TASK-M5-21; this task's property test runs with the playhead at t = 0 and is re-run by TASK-M5-21 at t > 0.
