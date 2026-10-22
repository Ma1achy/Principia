# TASK-M7-59 — Print graph port, Print: 9 node types and 11 presets, each with its lab golden (R-486, R-492)

- **Milestone:** M7
- **Closes:** REQ-COL-146
- **Depends on:** TASK-M7-56, TASK-M7-57
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~1200 lines, the tests and 11 preset files included

## Goal
Port the lab's Print category to WGSL, one node at a time against the lab's own output: offset, halftone, riso, stipple, engrave, mezzotint, linocut, cyanotype, plotter. Each node's registry entry is written from `catalogue/node_catalogue.json` (its `doc`, typed inputs and parameter ranges, each length with its unit), with its provisional cost class and motion policy, `temporally_stable`, and every deviation the design requires listed, never tuned to match (R-492). With them, the presets this category completes (each lab preset is ported by the task of the last category its nodes need, applied per R-369), converted from the lab's graphs to `principia.print/1`: Offset press (NASA Facts, 1978); Halftone; Risograph; Stipple (after Luminet, 1979); Limited palette, then halftone; Engraving; Mezzotint; Linocut; Cyanotype; Pen plotter; Fluoro palette, risograph. Goldens: each node at its defaults on the bundled source image against `images/nodes/`, each preset against `images/presets/`, within REQ-COL-142's tolerance; the scale (§11 test 4) and stability (test 5) harnesses run on the category's nodes.

## References
- `docs/design/principia_print_graph.md` § "10. Porting from the lab"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/design/principia_print_graph.md` § "4.3 Node types are a registry"
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback"
- `docs/design/principia_colour_composition.md` § "7. Preset table & golden-image obligation"
- `decisions.md` § "R-486 — The Print graph's node set is the lab's, all 78 node types and all 103 presets, ported from GLSL to WGSL; its data format is the Python prototype's, as the design doc §4 merges them *(supersedes R-130, with R-485)*"
- `decisions.md` § "R-492 — Fidelity to the lab is measured per node and per preset against the bundled reference renders; a deviation the design requires (units, seeding, real fields for the lab's stand-in) is listed in the node's registry entry, never tuned to match; the design doc's §11 tests, each with an arm that can fail"

## Deliverables
- `crates/render/src/print/nodes/` — the 9 Print nodes' registry entries and WGSL passes (`shaders/wgsl/print/`).
- `crates/render/print_presets/` — the presets above as `principia.print/1` files.
- `fixtures/golden/print/` — the category's references and results; golden suite `print-print`.

## Acceptance tests
- `cargo xtask golden print-print` — REQ-COL-146's verify (against the baselines in `fixtures/golden/print/` (R-146, R-110; the M8 browser suite checks the same baselines) (the lab's references, pinned there per colour_composition §7 "The Print graph's goldens"), every node of the category, and every preset listed, renders within REQ-COL-142's median and p99 of its reference or is listed in its registry entry with the deviation's reason (§11 test 9); the category's `ref_px` and `fraction` nodes pass §11 test 4 and its `temporally_stable` nodes test 5; controls: one node rendered with a default parameter doubled exceeds the tolerance and fails, a node compared to another node's reference fails, and a registry entry whose listed deviation is removed while the port still deviates fails).

## Notes
- References are the lab's renders in `docs/gui/reference/print_graph/` `images/nodes/` and `images/presets/` (R-492; reference, not code to build on, R-485); its GLSL is translated, not redesigned (§10).
- A preset that uses a category not yet ported waits for that category's task: the dependencies above are the categories its presets use.
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 8 of PR #281 (qa review 5608745757, finding 1, applied per R-369):** the acceptance lines follow REQ-COL-146's new verify; a golden-image verify runs under `cargo xtask golden <suite>` with its baselines in `fixtures/golden/print/` (R-146, R-110). *Was:* "`fixtures/golden/print/` — the category's references and results; test `print_port_print`."; "`cargo test -p render print_port_print` — REQ-COL-146's verify (every node of the category, and every preset listed, renders within REQ-COL-142's median and p99 of its reference or is listed in its registry entry with the deviation's reason (§11 test 9); the category's `ref_px` and `fraction` nodes pass §11 test 4 and its `temporally_stable` nodes test 5; controls: one node rendered with a default parameter doubled exceeds the tolerance and fails, a node compared to another node's reference fails, and a registry entry whose listed deviation is removed while the port still deviates fails).".
