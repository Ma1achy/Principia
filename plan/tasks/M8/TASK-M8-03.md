# TASK-M8-03 — Undo in the contract: the shared SetField history, coalescing, and per-field blast-radius metadata

- **Milestone:** M8
- **Closes:** REQ-GUI-035, REQ-GUI-071, REQ-GUI-077
- **Depends on:** TASK-M8-01
- **Needs (earlier milestones):** REQ-SCHED-017, REQ-SCHED-048, REQ-PERF-016, REQ-SCHED-059, REQ-GUI-007, REQ-GUI-009
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The contract keeps the one undo / redo history of typed SetField edits (R-52), shared by every GUI client: every SimConfig and RenderState edit is undoable, including navigation and lock / unlock (R-69); ViewUI-only state never enters it; the GUI clock's playhead writes are marked "no history" (R-101); a drag or a scrub coalesces into one entry (R-96). Each field carries its caching blast-radius entry, so a control's re-integrate warning comes from the field, not from the struct it lives in (R-89, R-92).

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "7. What a replacement GUI must honour (the teardown contract)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"
- `decisions.md` § "R-89 — Depth and E are not on the sim key *(closes RQ-40)*"
- `decisions.md` § "R-92 — What the sim key holds of navigation *(closes RQ-43)*"

## Deliverables
- `crates/engine/src/contract/history.rs` — `History` with `apply`, `undo`, `redo`, `depth`, coalescing by gesture id; `History::NoHistory` edits bypass it.
- `crates/engine/src/contract/field_meta.rs` — per-path metadata `{key: Sim|Render|ViewUi, blast_radius}` generated from caching Part 2's table: `render_scale`, `lock_to_native`, `MAX_REL_DEPTH`, E and `checkerboard_mode` warn nothing though they sit in `SimConfig.quality`; in-plane pan / zoom and lock warn nothing; slicing out of plane, tilt and rotate warn; transport warns nothing.
- `crates/engine/src/state_surface.rs` — `undo()` / `redo()` requests on the surface; the gui crate holds no stack.
- Tests: `undo_scope`, `undo_shared`, `field_warnings`.

## Acceptance tests
- `cargo test -p engine undo_scope` — pan, lock, unlock and a stain edit each add one history entry and undo restores the prior state; opening a window, moving focus, selecting, keeping an orbit, play / pause / speed / loop and ten seconds of playback add none; a scrub adds exactly one (REQ-GUI-035).
- `cargo test -p engine undo_shared` — apply edits from two GUI clients, undo from one: the contract reverts the last edit regardless of which client made it; a slider drag, a scrubber drag and a body drag each add exactly one entry; the gui crate holds no undo stack (REQ-GUI-071).
- `cargo test -p engine field_warnings` — for every exposed field the re-integrate warning equals the caching Part 2 blast-radius entry: render_scale, lock_to_native, MAX_REL_DEPTH, E and checkerboard_mode show no warning although they sit in SimConfig.quality; in-plane pan and zoom and lock show none; slicing out of the plane, tilt and rotate do; playback transport shows none (REQ-GUI-077).

## Notes
- none
