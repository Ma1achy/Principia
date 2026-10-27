# TASK-M8-69 — The Print graph editor: the Stain's node editor reused on the slots' graph, typed wire refusal, positions in `ViewUI`, unstable nodes marked, and custom WGSL nodes (R-486, R-491)

- **Milestone:** M8
- **Closes:** REQ-GUI-361, REQ-GUI-362, REQ-RENDER-116
- **Depends on:** TASK-M8-68, TASK-M8-93, TASK-M8-21
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
Edit graph… opens the Stain's node editor component on the Print graph: the same wiring, typed ports, undo and keyboard scopes, on the same graph the slots show; a mismatched wire refused with its reason; node positions, pan, zoom and selection in `ViewUI`, so a move is no undo entry; nodes that are not `temporally_stable` marked; custom nodes (user WGSL) through the Stain's occupant mechanism, tier 3.

## References
- `docs/design/principia_print_graph.md` § "7.2 The graph editor"
- `docs/design/principia_print_graph.md` § "4.2 Port types"
- `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback"
- `docs/design/principia_print_graph.md` § "9. Provenance"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-486 — The Print graph's node set is the lab's, all 78 node types and all 103 presets, ported from GLSL to WGSL; its data format is the Python prototype's, as the design doc §4 merges them *(supersedes R-130, with R-485)*"
- `decisions.md` § "R-491 — `RenderState.print` is recorded by value everywhere `RenderState` goes; a graph of built-in nodes is round-trip tier 2, custom WGSL tier 3"

## Deliverables
- `crates/gui/src/print_editor/` — the editor's Print graph adapter over the stain editor's canvas.
- Screenshot case `print_graph_editor`; tests `print_editor`, `print_custom_node`.

## Acceptance tests
- `cargo xtask screenshot print_graph_editor` and `cargo test -p gui print_editor` — REQ-GUI-361's verify (an edit in the editor shows in the slots and the reverse; an `image` → `direction` drag is refused with the reason; moving a node adds no undo entry and leaves the serialised graph unchanged; a pan of the canvas, a zoom of the canvas and a change of selection each add no undo entry, leave the serialised graph unchanged and are held in `ViewUI`; Kuwahara and stained glass are marked unstable; controls: an editor that accepts the mismatched wire, a node move recorded as an undo entry, and a pan, a zoom or a selection recorded as an undo entry or written into the serialised graph, each fail).
- `cargo test -p gui print_custom_node` — REQ-GUI-362's verify (a custom WGSL node compiles into the Print graph through the occupant path, shows its source in the editor and is recorded in tier 3; control: a custom node recorded in tier 2 fails the tier check).
- Definition, and `cargo test -p gui print_custom_node` — REQ-RENDER-116's verify (§7.2 states the rule; a custom WGSL node registers with the class and policy it gives, and the arbiter's modelled print cost includes it; control: a custom node registered with no class or policy fails REQ-RENDER-103's check, and a modelled cost that omits the custom node fails).

## Notes
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 3 of PR #281 (perf review 5608473498, F4, applied per R-369):** REQ-RENDER-116 (new, a definition, perf-reviewed) gives a custom WGSL node its cost class and motion policy, written into print_graph §7.2 by this task. *Was:* "**Closes:** REQ-GUI-361, REQ-GUI-362".
- **Fix round 8 of PR #281 (qa review 5608745757, finding 10, applied per R-369):** the acceptance lines follow REQ-GUI-361's new verify. *Was:* "`cargo xtask screenshot print_graph_editor` and `cargo test -p gui print_editor` — REQ-GUI-361's verify (an edit in the editor shows in the slots and the reverse; an `image` → `direction` drag is refused with the reason; moving a node adds no undo entry and leaves the serialised graph unchanged; Kuwahara and stained glass are marked unstable; controls: an editor that accepts the mismatched wire, and a node move recorded as an undo entry, each fail).".
- **Fix round 9 of PR #281 (qa re-check 5624154623, E, applied per R-369):** the round 8 note's golden-image clause is dropped, this task having no golden verify. The dropped clause: "a golden-image verify runs under `cargo xtask golden <suite>` with its baselines in `fixtures/golden/print/` (R-146, R-110)".
- R-627 (27 Oct 2026): this task runs on the real engine, so where it depended on a task R-627 moves to the mock it depends on that task's real-engine side, which depends on it: TASK-M8-20 by TASK-M8-93 (applied per R-369). *Was:* "- **Depends on:** TASK-M8-68, TASK-M8-20, TASK-M8-21".
