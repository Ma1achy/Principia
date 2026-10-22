# TASK-M7-65 — R-130's eight styles as the Print graph's Poster family: their definitions in the design doc, any node they need, and their goldens (R-532)

- **Milestone:** M7
- **Closes:** REQ-COL-044, REQ-COL-153
- **Depends on:** TASK-M7-58, TASK-M7-59
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~700 lines, the tests and the definitions included

## Goal
Watercolour & pencil and the seven print presets (Default, Drops, Newspaper, Vintage, Poster78, Riso, Duotone) as Print graph presets beside the lab's 103, 111 in all, in a twelfth family labelled "Poster", R-130's Default always "Poster › Default". Each is first defined in the design doc, a graph of registry nodes whose look is R-130's, transcribed from R-130's sources as render_gui_spec's *Was* note cites them; a look the lab's nodes cannot make (the halftone's per-plate misregistration and cell ceiling) is a node added to the registry with its licence. Their fidelity reference is that definition, not a lab image.

## References
- `docs/design/principia_print_graph.md` § "7.1 The Print panel (replaces Display › Style)"
- `docs/design/principia_print_graph.md` § "10. Porting from the lab"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `decisions.md` § "R-130 ✱ — The styles are the poster's *(closes RQ-107)*"
- `decisions.md` § "R-485 — Post-processing is the Print graph, a node graph after the stain and separate from it; it replaces the Display window's "Style", and R-130's v1 styles become presets of it, still available *(supersedes R-130)*"
- `decisions.md` § "R-532 — R-130's eight styles are ported as additional Print graph presets beside the lab's 103, 111 in all *(closes RQ-396; amends R-485 and R-486)*"

## Deliverables
- `docs/design/principia_print_graph.md` — a subsection defining the eight (REQ-COL-153), with a "Removed lines" note.
- `crates/render/print_presets/poster/` — the eight preset files; any added node in `crates/render/src/print/nodes/` with its licence.
- Test `print_port_poster`.

## Acceptance tests
- `cargo test -p render print_port_poster` — REQ-COL-044's verify (the eight presets are registered in the family "Poster" and each renders within REQ-COL-142's tolerance of its definition's reference render; controls: a Poster preset whose graph drops its halftone's per-plate slip fails its golden, and a preset labelled bare "Default" fails the label check).
- Definition: the design doc holds the eight definitions, each a `principia.print/1` graph with its settings, and each added node's registry entry and licence; `workbench/` is not cited as normative (R-159); the gui reviewer checks each look against R-130's description (REQ-COL-153).

## Notes
- `workbench/` is never cited as normative (R-159): the definition in the design doc is.
- The definition is reviewed by the gui reviewer, not the physics reviewer: R-485 (the human's point 5) names physics review for the field inputs only (applied per R-369).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
